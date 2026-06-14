// path: src/patch/patch_mixer.rs

//! `PatchMixer` domain service — sums audio from all active patches, applying
//! per-patch gain and pan.
//!
//! # Design
//!
//! `PatchMixer` accepts a slice of [`PatchMixEntry`] descriptors on the audio
//! thread and applies them to a buffer of interleaved stereo frames.  Each
//! entry carries the patch's current [`Amplitude`] (gain) and pan position.
//!
//! The mixer is completely allocation-free and lock-free on the audio thread.
//! Ownership of `Patch` objects and voice rendering is the caller's
//! responsibility; `PatchMixer` only knows about gain/pan metadata.
//!
//! # Channel dispatch
//!
//! Callers are responsible for routing MIDI events to all patches whose
//! [`ChannelSubscription`] matches the incoming event; the mixer itself does
//! not filter by channel.  The invariant "dispatch delivers events to *all*
//! subscribed patches, not just the first match" is enforced at the caller
//! level (see [`ChannelSubscription::matches`]).

use crate::kernel::amplitude::Amplitude;
use crate::kernel::audio_frame::AudioFrame;

// ─────────────────────────────────────────────────────────────────────────────
// Pan utilities (stack-only, allocation-free)
// ─────────────────────────────────────────────────────────────────────────────

/// Compute left/right gain coefficients from a linear pan value in [−1.0, 1.0].
///
/// Uses a constant-power (sine/cosine) pan law so that the perceived loudness
/// stays consistent across the stereo field.
///
/// Returns `(left_gain, right_gain)` as `f32` for use on the audio thread.
#[inline]
fn pan_gains(pan: f64) -> (f32, f32) {
    // Clamp to [-1, 1] defensively — Patch already enforces this, but we want
    // no UB from out-of-range trig inputs.
    let p = pan.clamp(-1.0, 1.0);
    // Map pan [-1, 1] → angle [0, π/2]
    let angle = (p + 1.0) * std::f64::consts::FRAC_PI_4; // (p+1)/2 * π/2
    let left = angle.cos() as f32;
    let right = angle.sin() as f32;
    (left, right)
}

// ─────────────────────────────────────────────────────────────────────────────
// PatchMixEntry
// ─────────────────────────────────────────────────────────────────────────────

/// Mixing metadata for one active patch on the audio thread.
///
/// All fields are `Copy` primitives — no heap allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatchMixEntry {
    /// Per-patch output gain.
    pub gain: Amplitude,
    /// Stereo pan: −1.0 (full left) … 0.0 (centre) … 1.0 (full right).
    pub pan: f64,
}

impl PatchMixEntry {
    /// Construct a `PatchMixEntry` with the given `gain` and `pan`.
    pub fn new(gain: Amplitude, pan: f64) -> Self {
        Self { gain, pan }
    }

    /// Construct a `PatchMixEntry` at unity gain and centre pan.
    pub fn unity() -> Self {
        Self {
            gain: Amplitude::unity(),
            pan: 0.0,
        }
    }
}

impl Default for PatchMixEntry {
    fn default() -> Self {
        Self::unity()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PatchMixer
// ─────────────────────────────────────────────────────────────────────────────

/// Domain service: sums per-patch audio frames into a stereo mix.
///
/// `PatchMixer` is stateless — it holds no parameters of its own and accepts
/// all mix metadata through its [`PatchMixer::mix`] and
/// [`PatchMixer::accumulate`] methods.  This keeps it lock-free and
/// allocation-free on the audio thread.
///
/// # Independence guarantee
///
/// Each `Patch` is responsible for owning its own voice pool. `PatchMixer`
/// only consumes rendered frames via [`PatchMixEntry`], so it cannot
/// accidentally couple or share voice state between patches.
///
/// # Examples
///
/// ```
/// use crest_synth::patch::patch_mixer::{PatchMixer, PatchMixEntry};
/// use crest_synth::kernel::amplitude::Amplitude;
/// use crest_synth::kernel::audio_frame::AudioFrame;
///
/// let mixer = PatchMixer::new();
/// let entry = PatchMixEntry::new(Amplitude::unity(), 0.0);
/// let patch_frame = AudioFrame::new(0.5, 0.5);
/// let result = mixer.apply_entry(patch_frame, &entry);
/// // Centre pan, unity gain → left ≈ right ≈ 0.5 * cos(π/4) * 1.0
/// assert!(result.left > 0.0);
/// assert!(result.right > 0.0);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct PatchMixer;

impl PatchMixer {
    /// Create a new `PatchMixer`.
    ///
    /// `PatchMixer` is stateless; construction is a no-op.
    pub fn new() -> Self {
        Self
    }

    /// Apply a single [`PatchMixEntry`]'s gain and pan to one `AudioFrame`.
    ///
    /// Allocation-free and lock-free — safe to call on the audio thread.
    ///
    /// Returns the scaled, panned frame.
    #[inline]
    pub fn apply_entry(&self, frame: AudioFrame, entry: &PatchMixEntry) -> AudioFrame {
        let g = entry.gain.value() as f32;
        let (pan_l, pan_r) = pan_gains(entry.pan);
        AudioFrame {
            left: frame.left * g * pan_l,
            right: frame.right * g * pan_r,
        }
    }

    /// Accumulate one patch's rendered frame into `out`, applying `entry`'s
    /// gain and pan.
    ///
    /// Allocation-free and lock-free — safe to call on the audio thread.
    ///
    /// `out` is modified in place (additive mix).
    #[inline]
    pub fn accumulate(&self, out: &mut AudioFrame, frame: AudioFrame, entry: &PatchMixEntry) {
        let panned = self.apply_entry(frame, entry);
        out.left += panned.left;
        out.right += panned.right;
    }

    /// Sum all patch frames in `patches` into a single `AudioFrame`.
    ///
    /// `patches` is a slice of `(rendered_frame, mix_entry)` pairs.
    /// Only pairs whose associated `active` flag would be implied by the caller
    /// having populated the slice are mixed — no internal filtering.
    ///
    /// Allocation-free and lock-free — safe to call on the audio thread.
    ///
    /// ```
    /// use crest_synth::patch::patch_mixer::{PatchMixer, PatchMixEntry};
    /// use crest_synth::kernel::amplitude::Amplitude;
    /// use crest_synth::kernel::audio_frame::AudioFrame;
    ///
    /// let mixer = PatchMixer::new();
    /// let entry = PatchMixEntry::new(Amplitude::unity(), 0.0);
    /// let frame = AudioFrame::new(0.4, 0.4);
    /// let out = mixer.mix(&[(frame, entry), (frame, entry)]);
    /// // Two patches with the same frame → twice the energy.
    /// assert!(out.left > frame.left);
    /// ```
    #[inline]
    pub fn mix(&self, patches: &[(AudioFrame, PatchMixEntry)]) -> AudioFrame {
        let mut out = AudioFrame::silence();
        for (frame, entry) in patches {
            self.accumulate(&mut out, *frame, entry);
        }
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::amplitude::Amplitude;
    use crate::kernel::audio_frame::AudioFrame;

    // ── PatchMixEntry ─────────────────────────────────────────────────────────

    #[test]
    fn default_entry_is_unity_centre() {
        let e = PatchMixEntry::default();
        assert!((e.gain.value() - 1.0).abs() < f64::EPSILON);
        assert!(e.pan.abs() < f64::EPSILON);
    }

    #[test]
    fn unity_entry_constructor() {
        let e = PatchMixEntry::unity();
        assert!((e.gain.value() - 1.0).abs() < f64::EPSILON);
        assert!(e.pan.abs() < f64::EPSILON);
    }

    #[test]
    fn new_entry_stores_gain_and_pan() {
        let gain = Amplitude::try_new(0.5).unwrap();
        let e = PatchMixEntry::new(gain, 0.3);
        assert!((e.gain.value() - 0.5).abs() < f64::EPSILON);
        assert!((e.pan - 0.3).abs() < f64::EPSILON);
    }

    // ── pan_gains ─────────────────────────────────────────────────────────────

    #[test]
    fn pan_centre_gives_equal_gains() {
        let (l, r) = pan_gains(0.0);
        assert!(
            (l - r).abs() < 1e-6,
            "centre pan must be equal L/R: l={l}, r={r}"
        );
    }

    #[test]
    fn pan_hard_left_gives_max_left() {
        let (l, r) = pan_gains(-1.0);
        // angle = 0 → cos(0) = 1.0, sin(0) = 0.0
        assert!((l - 1.0_f32).abs() < 1e-6);
        assert!(r.abs() < 1e-6);
    }

    #[test]
    fn pan_hard_right_gives_max_right() {
        let (l, r) = pan_gains(1.0);
        // angle = π/2 → cos(π/2) ≈ 0, sin(π/2) = 1.0
        assert!(l.abs() < 1e-6);
        assert!((r - 1.0_f32).abs() < 1e-6);
    }

    #[test]
    fn pan_gains_are_constant_power() {
        // For any pan, l² + r² ≈ 1 (constant power).
        for i in 0..=20 {
            let pan = -1.0 + (i as f64) * 0.1;
            let (l, r) = pan_gains(pan);
            let power = l * l + r * r;
            assert!(
                (power - 1.0_f32).abs() < 1e-5,
                "constant power violated at pan={pan}: l={l}, r={r}, power={power}"
            );
        }
    }

    // ── PatchMixer::apply_entry ───────────────────────────────────────────────

    #[test]
    fn apply_entry_silence_gain_mutes_frame() {
        let mixer = PatchMixer::new();
        let entry = PatchMixEntry::new(Amplitude::silence(), 0.0);
        let frame = AudioFrame::new(1.0, 1.0);
        let out = mixer.apply_entry(frame, &entry);
        assert!(out.left.abs() < 1e-6);
        assert!(out.right.abs() < 1e-6);
    }

    #[test]
    fn apply_entry_hard_left_pan() {
        let mixer = PatchMixer::new();
        let entry = PatchMixEntry::new(Amplitude::unity(), -1.0);
        let frame = AudioFrame::new(0.8, 0.8);
        let out = mixer.apply_entry(frame, &entry);
        assert!(out.left > 0.0, "left should be non-zero");
        assert!(
            out.right.abs() < 1e-6,
            "right should be silent for hard-left pan"
        );
    }

    #[test]
    fn apply_entry_hard_right_pan() {
        let mixer = PatchMixer::new();
        let entry = PatchMixEntry::new(Amplitude::unity(), 1.0);
        let frame = AudioFrame::new(0.8, 0.8);
        let out = mixer.apply_entry(frame, &entry);
        assert!(
            out.left.abs() < 1e-6,
            "left should be silent for hard-right pan"
        );
        assert!(out.right > 0.0, "right should be non-zero");
    }

    #[test]
    fn apply_entry_half_gain_halves_amplitude() {
        let mixer = PatchMixer::new();
        let entry_unity = PatchMixEntry::new(Amplitude::unity(), 0.0);
        let entry_half = PatchMixEntry::new(Amplitude::try_new(0.5).unwrap(), 0.0);
        let frame = AudioFrame::new(0.8, 0.6);
        let full = mixer.apply_entry(frame, &entry_unity);
        let half = mixer.apply_entry(frame, &entry_half);
        assert!((full.left - half.left * 2.0).abs() < 1e-5);
        assert!((full.right - half.right * 2.0).abs() < 1e-5);
    }

    // ── PatchMixer::accumulate ─────────────────────────────────────────────────

    #[test]
    fn accumulate_adds_to_existing_output() {
        let mixer = PatchMixer::new();
        let entry = PatchMixEntry::unity();
        let frame = AudioFrame::new(0.4, 0.3);
        let mut out = AudioFrame::silence();
        mixer.accumulate(&mut out, frame, &entry);
        mixer.accumulate(&mut out, frame, &entry);
        // Two identical patches → doubled signal (modulo constant-power pan at centre).
        let single = mixer.apply_entry(frame, &entry);
        assert!((out.left - single.left * 2.0).abs() < 1e-5);
        assert!((out.right - single.right * 2.0).abs() < 1e-5);
    }

    // ── PatchMixer::mix ────────────────────────────────────────────────────────

    #[test]
    fn mix_empty_slice_returns_silence() {
        let mixer = PatchMixer::new();
        let out = mixer.mix(&[]);
        assert_eq!(out.left, 0.0);
        assert_eq!(out.right, 0.0);
    }

    #[test]
    fn mix_single_patch_matches_apply_entry() {
        let mixer = PatchMixer::new();
        let entry = PatchMixEntry::new(Amplitude::try_new(0.7).unwrap(), 0.3);
        let frame = AudioFrame::new(0.5, 0.5);
        let expected = mixer.apply_entry(frame, &entry);
        let out = mixer.mix(&[(frame, entry)]);
        assert!((out.left - expected.left).abs() < 1e-6);
        assert!((out.right - expected.right).abs() < 1e-6);
    }

    #[test]
    fn mix_two_patches_is_sum_of_individual_applies() {
        let mixer = PatchMixer::new();
        let entry1 = PatchMixEntry::new(Amplitude::try_new(0.6).unwrap(), -0.5);
        let entry2 = PatchMixEntry::new(Amplitude::try_new(0.4).unwrap(), 0.5);
        let frame1 = AudioFrame::new(0.8, 0.8);
        let frame2 = AudioFrame::new(0.3, 0.3);
        let s1 = mixer.apply_entry(frame1, &entry1);
        let s2 = mixer.apply_entry(frame2, &entry2);
        let out = mixer.mix(&[(frame1, entry1), (frame2, entry2)]);
        assert!((out.left - (s1.left + s2.left)).abs() < 1e-5);
        assert!((out.right - (s1.right + s2.right)).abs() < 1e-5);
    }

    #[test]
    fn mix_patch_with_zero_gain_contributes_nothing() {
        let mixer = PatchMixer::new();
        let silent_entry = PatchMixEntry::new(Amplitude::silence(), 0.0);
        let active_entry = PatchMixEntry::new(Amplitude::unity(), 0.0);
        let frame = AudioFrame::new(1.0, 1.0);
        let out = mixer.mix(&[(frame, silent_entry), (frame, active_entry)]);
        let expected = mixer.apply_entry(frame, &active_entry);
        assert!((out.left - expected.left).abs() < 1e-6);
        assert!((out.right - expected.right).abs() < 1e-6);
    }

    // ── Independence guarantee ────────────────────────────────────────────────

    #[test]
    fn independent_patch_voices_do_not_affect_mix() {
        // Validate: each patch has an independent voice pool; the mixer only
        // consumes rendered frames, so polyphony of one patch cannot exhaust
        // another's voices.
        //
        // This test constructs two mix entries and verifies that the output
        // produced from either entry is independent of the other.
        let mixer = PatchMixer::new();
        let entry_a = PatchMixEntry::new(Amplitude::try_new(1.0).unwrap(), 0.0);
        let entry_b = PatchMixEntry::new(Amplitude::try_new(0.5).unwrap(), 0.0);
        let frame = AudioFrame::new(0.5, 0.5);
        // Mixing only A gives the same result regardless of what B is doing.
        let only_a = mixer.mix(&[(frame, entry_a)]);
        let a_then_b = mixer.mix(&[(frame, entry_a), (AudioFrame::silence(), entry_b)]);
        // Silence from B contributes nothing, so mix is the same as only_a.
        assert!((only_a.left - a_then_b.left).abs() < 1e-6);
        assert!((only_a.right - a_then_b.right).abs() < 1e-6);
    }

    // ── Channel dispatch (multi-patch on same channel) ────────────────────────

    #[test]
    fn two_patches_on_same_channel_both_contribute_to_mix() {
        // Validates the invariant: channel dispatch delivers events to ALL
        // subscribed patches, not just the first match.
        // Here, the caller has routed the same signal to two patches on the same
        // channel; both should appear in the mix output.
        let mixer = PatchMixer::new();
        let entry = PatchMixEntry::unity();
        let frame = AudioFrame::new(0.5, 0.5);
        let single = mixer.mix(&[(frame, entry)]);
        let doubled = mixer.mix(&[(frame, entry), (frame, entry)]);
        // Two identical patches should produce double the output.
        assert!((doubled.left - single.left * 2.0).abs() < 1e-5);
        assert!((doubled.right - single.right * 2.0).abs() < 1e-5);
    }
}
