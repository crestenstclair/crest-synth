// path: src/patch/channel_strip.rs

//! `ChannelStrip` value object — one mixer channel's settings.
//!
//! # Design
//!
//! `ChannelStrip` holds the per-channel mixing parameters: volume, reverb send,
//! echo send, pan, mute, and solo. Volume also serves as the channel's metering
//! source.
//!
//! All `f64` parameters are kept within their valid ranges by clamping on every
//! set. This avoids any possibility of an out-of-range value being stored.
//!
//! `ChannelStrip` is a pure value object — it has no behaviour beyond storing
//! and validating its parameters. It is `Copy` and allocation-free.

/// One mixer channel's settings.
///
/// # Invariants
///
/// - `volume`, `reverb_send`, `echo_send` are always within `0.0..=1.0`
///   (clamped on every set).
/// - `pan` is always within `-1.0..=1.0` (clamped on every set).
///
/// # Metering
///
/// `volume` doubles as the channel's peak meter source. Metering is independent
/// of `mute` and `solo` state — a channel silenced by another channel's solo
/// still meters its own signal level.
///
/// # Examples
///
/// ```
/// use crest_synth::patch::channel_strip::ChannelStrip;
///
/// let mut strip = ChannelStrip::default();
/// strip.set_volume(0.8);
/// strip.set_pan(-0.5);
/// assert!((strip.volume() - 0.8).abs() < f64::EPSILON);
/// assert!((strip.pan() - -0.5).abs() < f64::EPSILON);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelStrip {
    /// Output volume: 0.0 (silence) to 1.0 (unity). Also the channel's
    /// metering source.
    volume: f64,
    /// Reverb send level: 0.0 (dry) to 1.0 (full send).
    reverb_send: f64,
    /// Echo (delay) send level: 0.0 (dry) to 1.0 (full send).
    echo_send: f64,
    /// Stereo pan: -1.0 (hard left) to 0.0 (center) to +1.0 (hard right).
    pan: f64,
    /// When `true` the channel is silenced in the output mix (metering
    /// still runs).
    mute: bool,
    /// When `true` this channel is soloed — all non-solo channels are
    /// silenced in the output mix (metering still runs for silenced
    /// channels).
    solo: bool,
}

impl ChannelStrip {
    /// Construct a `ChannelStrip` with explicit initial values.
    ///
    /// All `f64` values are clamped to their valid ranges on construction.
    ///
    /// ```
    /// use crest_synth::patch::channel_strip::ChannelStrip;
    ///
    /// let strip = ChannelStrip::new(0.75, 0.2, 0.1, -0.3, false, false);
    /// assert!((strip.volume() - 0.75).abs() < f64::EPSILON);
    /// assert!((strip.reverb_send() - 0.2).abs() < f64::EPSILON);
    /// assert!((strip.echo_send() - 0.1).abs() < f64::EPSILON);
    /// assert!((strip.pan() - -0.3).abs() < f64::EPSILON);
    /// assert!(!strip.mute());
    /// assert!(!strip.solo());
    /// ```
    pub fn new(
        volume: f64,
        reverb_send: f64,
        echo_send: f64,
        pan: f64,
        mute: bool,
        solo: bool,
    ) -> Self {
        Self {
            volume: clamp_unit(volume),
            reverb_send: clamp_unit(reverb_send),
            echo_send: clamp_unit(echo_send),
            pan: clamp_pan(pan),
            mute,
            solo,
        }
    }

    // ── Getters ───────────────────────────────────────────────────────────────

    /// Returns the channel volume in `[0.0, 1.0]`.
    ///
    /// This value also serves as the channel's peak meter source.
    #[inline]
    pub fn volume(&self) -> f64 {
        self.volume
    }

    /// Returns the reverb send level in `[0.0, 1.0]`.
    #[inline]
    pub fn reverb_send(&self) -> f64 {
        self.reverb_send
    }

    /// Returns the echo send level in `[0.0, 1.0]`.
    #[inline]
    pub fn echo_send(&self) -> f64 {
        self.echo_send
    }

    /// Returns the stereo pan position in `[-1.0, 1.0]`.
    ///
    /// -1.0 = hard left, 0.0 = center, +1.0 = hard right.
    #[inline]
    pub fn pan(&self) -> f64 {
        self.pan
    }

    /// Returns `true` if the channel is muted.
    ///
    /// A muted channel is silenced in the output mix but metering continues.
    #[inline]
    pub fn mute(&self) -> bool {
        self.mute
    }

    /// Returns `true` if the channel is soloed.
    ///
    /// When any channel is soloed, all non-soloed channels are silenced in
    /// the output mix, but metering continues for all channels.
    #[inline]
    pub fn solo(&self) -> bool {
        self.solo
    }

    // ── Setters (clamping) ────────────────────────────────────────────────────

    /// Set the channel volume, clamping to `[0.0, 1.0]`.
    ///
    /// ```
    /// use crest_synth::patch::channel_strip::ChannelStrip;
    ///
    /// let mut strip = ChannelStrip::default();
    /// strip.set_volume(1.5); // clamped to 1.0
    /// assert!((strip.volume() - 1.0).abs() < f64::EPSILON);
    /// strip.set_volume(-0.5); // clamped to 0.0
    /// assert!(strip.volume().abs() < f64::EPSILON);
    /// ```
    #[inline]
    pub fn set_volume(&mut self, value: f64) {
        self.volume = clamp_unit(value);
    }

    /// Set the reverb send level, clamping to `[0.0, 1.0]`.
    ///
    /// ```
    /// use crest_synth::patch::channel_strip::ChannelStrip;
    ///
    /// let mut strip = ChannelStrip::default();
    /// strip.set_reverb_send(2.0); // clamped to 1.0
    /// assert!((strip.reverb_send() - 1.0).abs() < f64::EPSILON);
    /// ```
    #[inline]
    pub fn set_reverb_send(&mut self, value: f64) {
        self.reverb_send = clamp_unit(value);
    }

    /// Set the echo send level, clamping to `[0.0, 1.0]`.
    ///
    /// ```
    /// use crest_synth::patch::channel_strip::ChannelStrip;
    ///
    /// let mut strip = ChannelStrip::default();
    /// strip.set_echo_send(1.5); // clamped to 1.0
    /// assert!((strip.echo_send() - 1.0).abs() < f64::EPSILON);
    /// ```
    #[inline]
    pub fn set_echo_send(&mut self, value: f64) {
        self.echo_send = clamp_unit(value);
    }

    /// Set the stereo pan, clamping to `[-1.0, 1.0]`.
    ///
    /// ```
    /// use crest_synth::patch::channel_strip::ChannelStrip;
    ///
    /// let mut strip = ChannelStrip::default();
    /// strip.set_pan(2.0); // clamped to 1.0
    /// assert!((strip.pan() - 1.0).abs() < f64::EPSILON);
    /// strip.set_pan(-2.0); // clamped to -1.0
    /// assert!((strip.pan() - -1.0).abs() < f64::EPSILON);
    /// ```
    #[inline]
    pub fn set_pan(&mut self, value: f64) {
        self.pan = clamp_pan(value);
    }

    /// Set the mute toggle.
    ///
    /// ```
    /// use crest_synth::patch::channel_strip::ChannelStrip;
    ///
    /// let mut strip = ChannelStrip::default();
    /// strip.set_mute(true);
    /// assert!(strip.mute());
    /// strip.set_mute(false);
    /// assert!(!strip.mute());
    /// ```
    #[inline]
    pub fn set_mute(&mut self, value: bool) {
        self.mute = value;
    }

    /// Set the solo toggle.
    ///
    /// ```
    /// use crest_synth::patch::channel_strip::ChannelStrip;
    ///
    /// let mut strip = ChannelStrip::default();
    /// strip.set_solo(true);
    /// assert!(strip.solo());
    /// strip.set_solo(false);
    /// assert!(!strip.solo());
    /// ```
    #[inline]
    pub fn set_solo(&mut self, value: bool) {
        self.solo = value;
    }

    // ── Derived helpers ───────────────────────────────────────────────────────

    /// Returns `true` if this channel produces audible output in the mix,
    /// given the set of channels that are currently soloed.
    ///
    /// A channel is audible when it is not muted and either:
    /// - No channel is soloed, **or**
    /// - This channel itself is soloed.
    ///
    /// Metering is always active regardless of this value.
    ///
    /// ```
    /// use crest_synth::patch::channel_strip::ChannelStrip;
    ///
    /// let strip = ChannelStrip::default();
    /// // No solos active, not muted → audible.
    /// assert!(strip.is_audible(false));
    /// // Another channel is soloed and this one is not → silent.
    /// assert!(!strip.is_audible(true));
    /// ```
    #[inline]
    pub fn is_audible(&self, any_solo_active: bool) -> bool {
        if self.mute {
            return false;
        }
        if any_solo_active && !self.solo {
            return false;
        }
        true
    }
}

impl Default for ChannelStrip {
    /// Returns a `ChannelStrip` at unity volume, center pan, zero sends,
    /// not muted, not soloed.
    fn default() -> Self {
        Self {
            volume: 1.0,
            reverb_send: 0.0,
            echo_send: 0.0,
            pan: 0.0,
            mute: false,
            solo: false,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Clamp `value` to `[0.0, 1.0]`.
///
/// NaN is mapped to 0.0 (the minimum) via `f64::clamp` semantics.
#[inline]
fn clamp_unit(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

/// Clamp `value` to `[-1.0, 1.0]`.
///
/// NaN is mapped to -1.0 (the minimum) via `f64::clamp` semantics.
#[inline]
fn clamp_pan(value: f64) -> f64 {
    value.clamp(-1.0, 1.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Default ───────────────────────────────────────────────────────────────

    #[test]
    fn channel_strip_default_values() {
        let strip = ChannelStrip::default();
        assert!(
            (strip.volume() - 1.0).abs() < f64::EPSILON,
            "default volume is unity"
        );
        assert!(
            strip.reverb_send().abs() < f64::EPSILON,
            "default reverb_send is zero"
        );
        assert!(
            strip.echo_send().abs() < f64::EPSILON,
            "default echo_send is zero"
        );
        assert!(strip.pan().abs() < f64::EPSILON, "default pan is center");
        assert!(!strip.mute(), "default mute is false");
        assert!(!strip.solo(), "default solo is false");
    }

    // ── Constructor ───────────────────────────────────────────────────────────

    #[test]
    fn channel_strip_new_stores_values() {
        let strip = ChannelStrip::new(0.5, 0.3, 0.2, 0.7, true, false);
        assert!((strip.volume() - 0.5).abs() < f64::EPSILON);
        assert!((strip.reverb_send() - 0.3).abs() < f64::EPSILON);
        assert!((strip.echo_send() - 0.2).abs() < f64::EPSILON);
        assert!((strip.pan() - 0.7).abs() < f64::EPSILON);
        assert!(strip.mute());
        assert!(!strip.solo());
    }

    // ── Volume invariant ──────────────────────────────────────────────────────

    #[test]
    fn channel_strip_volume_clamps_above_one() {
        let mut strip = ChannelStrip::default();
        strip.set_volume(1.5);
        assert!((strip.volume() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn channel_strip_volume_clamps_below_zero() {
        let mut strip = ChannelStrip::default();
        strip.set_volume(-0.5);
        assert!(strip.volume().abs() < f64::EPSILON);
    }

    #[test]
    fn channel_strip_volume_accepts_valid_range() {
        let mut strip = ChannelStrip::default();
        for i in 0..=10 {
            let v = i as f64 / 10.0;
            strip.set_volume(v);
            assert!(
                (strip.volume() - v).abs() < f64::EPSILON,
                "volume {v} rejected"
            );
        }
    }

    #[test]
    fn channel_strip_new_clamps_volume_above_one() {
        let strip = ChannelStrip::new(2.0, 0.0, 0.0, 0.0, false, false);
        assert!((strip.volume() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn channel_strip_new_clamps_volume_below_zero() {
        let strip = ChannelStrip::new(-1.0, 0.0, 0.0, 0.0, false, false);
        assert!(strip.volume().abs() < f64::EPSILON);
    }

    // ── Reverb send invariant ─────────────────────────────────────────────────

    #[test]
    fn channel_strip_reverb_send_clamps_above_one() {
        let mut strip = ChannelStrip::default();
        strip.set_reverb_send(3.0);
        assert!((strip.reverb_send() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn channel_strip_reverb_send_clamps_below_zero() {
        let mut strip = ChannelStrip::default();
        strip.set_reverb_send(-0.1);
        assert!(strip.reverb_send().abs() < f64::EPSILON);
    }

    // ── Echo send invariant ───────────────────────────────────────────────────

    #[test]
    fn channel_strip_echo_send_clamps_above_one() {
        let mut strip = ChannelStrip::default();
        strip.set_echo_send(99.0);
        assert!((strip.echo_send() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn channel_strip_echo_send_clamps_below_zero() {
        let mut strip = ChannelStrip::default();
        strip.set_echo_send(-5.0);
        assert!(strip.echo_send().abs() < f64::EPSILON);
    }

    // ── Pan invariant ─────────────────────────────────────────────────────────

    #[test]
    fn channel_strip_pan_clamps_above_plus_one() {
        let mut strip = ChannelStrip::default();
        strip.set_pan(2.5);
        assert!((strip.pan() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn channel_strip_pan_clamps_below_minus_one() {
        let mut strip = ChannelStrip::default();
        strip.set_pan(-3.0);
        assert!((strip.pan() - -1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn channel_strip_pan_accepts_valid_range() {
        let mut strip = ChannelStrip::default();
        for i in 0..=20 {
            let v = -1.0 + (i as f64) * 0.1;
            strip.set_pan(v);
            assert!((strip.pan() - v).abs() < 1e-10, "pan {v} rejected");
        }
    }

    #[test]
    fn channel_strip_new_clamps_pan_out_of_range() {
        let strip_high = ChannelStrip::new(1.0, 0.0, 0.0, 5.0, false, false);
        assert!((strip_high.pan() - 1.0).abs() < f64::EPSILON);

        let strip_low = ChannelStrip::new(1.0, 0.0, 0.0, -5.0, false, false);
        assert!((strip_low.pan() - -1.0).abs() < f64::EPSILON);
    }

    // ── Mute and Solo toggles ─────────────────────────────────────────────────

    #[test]
    fn channel_strip_set_mute_toggle() {
        let mut strip = ChannelStrip::default();
        assert!(!strip.mute());
        strip.set_mute(true);
        assert!(strip.mute());
        strip.set_mute(false);
        assert!(!strip.mute());
    }

    #[test]
    fn channel_strip_set_solo_toggle() {
        let mut strip = ChannelStrip::default();
        assert!(!strip.solo());
        strip.set_solo(true);
        assert!(strip.solo());
        strip.set_solo(false);
        assert!(!strip.solo());
    }

    // ── is_audible ────────────────────────────────────────────────────────────

    #[test]
    fn channel_strip_audible_when_not_muted_no_solo() {
        let strip = ChannelStrip::default();
        assert!(strip.is_audible(false));
    }

    #[test]
    fn channel_strip_silent_when_muted() {
        let mut strip = ChannelStrip::default();
        strip.set_mute(true);
        assert!(!strip.is_audible(false));
        assert!(!strip.is_audible(true));
    }

    #[test]
    fn channel_strip_silent_when_other_solo_active() {
        let strip = ChannelStrip::default(); // solo=false
                                             // Another channel is soloed, this one is not → silent in output.
        assert!(!strip.is_audible(true));
    }

    #[test]
    fn channel_strip_audible_when_itself_is_solo() {
        let mut strip = ChannelStrip::default();
        strip.set_solo(true);
        assert!(strip.is_audible(true));
    }

    #[test]
    fn channel_strip_muted_solo_channel_is_still_silent_in_output() {
        // Mute takes precedence over solo in output.
        let mut strip = ChannelStrip::default();
        strip.set_mute(true);
        strip.set_solo(true);
        assert!(!strip.is_audible(true));
    }

    // ── Metering independence ─────────────────────────────────────────────────

    #[test]
    fn channel_strip_metering_independent_of_mute_and_solo() {
        // The volume value is always readable regardless of mute/solo state.
        // (Metering logic uses volume() directly, not is_audible().)
        let mut strip = ChannelStrip::new(0.6, 0.0, 0.0, 0.0, true, false);
        assert!(
            (strip.volume() - 0.6).abs() < f64::EPSILON,
            "muted channel still has readable volume for metering"
        );
        strip.set_mute(false);
        strip.set_solo(false);
        // Soloed-out scenario: another channel is soloed, this one is not.
        assert!(
            !strip.is_audible(true),
            "non-solo channel is silent when another solos"
        );
        assert!(
            (strip.volume() - 0.6).abs() < f64::EPSILON,
            "silenced channel still has readable volume for metering"
        );
    }

    // ── Copy semantics ────────────────────────────────────────────────────────

    #[test]
    fn channel_strip_is_copy() {
        let a = ChannelStrip::new(0.5, 0.1, 0.2, 0.3, false, true);
        let b = a;
        assert_eq!(a, b);
    }
}
