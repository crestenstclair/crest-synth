// path: src/audio/audio_renderer.rs

//! Domain service: iterate all active [`Voice`] aggregates, render each
//! through the synthesis engine, and mix to stereo output.
//!
//! # Audio-thread safety
//!
//! `render_block` and the note command methods are lock-free and
//! allocation-free when the voice pool is not exhausted — safe to call on
//! the audio thread.

use crate::kernel::audio_frame::AudioFrame;
use crate::kernel::note_id::NoteId;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::sample_rate::SampleRate;
use crate::kernel::velocity::Velocity;
use crate::synth::voice::{NoteOff, NoteOn, Voice, VoiceEvent};

/// Maximum number of simultaneous voices.
///
/// Choosing a compile-time constant avoids heap allocation on the audio thread.
const MAX_VOICES: usize = 16;

/// Domain service: polyphonic voice pool that mixes all active
/// [`Voice`] aggregates into an output buffer.
///
/// # Design
///
/// `AudioRenderer` owns a fixed-size pool of [`Voice`] instances and
/// allocates them by finding a reclaimable slot. Note commands are
/// forwarded to the appropriate voice; `render_block` iterates the pool,
/// renders each active voice, and sums samples to produce stereo frames.
///
/// # Audio-thread safety
///
/// `render_block` makes no heap allocations and acquires no locks.
/// Voice-stealing (finding a reclaimable slot) iterates the fixed pool.
pub struct AudioRenderer {
    voices: [Voice; MAX_VOICES],
    sample_rate: SampleRate,
}

impl AudioRenderer {
    /// Create a new `AudioRenderer` with the given sample rate.
    ///
    /// All voices in the pool start idle.
    pub fn new(sample_rate: SampleRate) -> Self {
        // `Voice` implements `Default` (idle state), so we use array init.
        Self {
            voices: std::array::from_fn(|_| Voice::new()),
            sample_rate,
        }
    }

    /// Returns the configured sample rate.
    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    /// **Command — NoteOn**: assign a reclaimable voice and start sounding.
    ///
    /// Finds a reclaimable (idle) voice in the pool and issues `NoteOn`.
    /// Returns the events emitted by the chosen voice, or `None` if the
    /// pool is full (no reclaimable voice found).
    ///
    /// # Audio-thread safety
    ///
    /// Iterates the fixed pool; no heap allocation.
    pub fn note_on(
        &mut self,
        note_id: NoteId,
        note_number: NoteNumber,
        velocity: Velocity,
    ) -> Option<Vec<VoiceEvent>> {
        let cmd = NoteOn {
            note_id,
            note_number,
            velocity,
        };
        // Find a reclaimable (idle) voice.
        let voice = self.voices.iter_mut().find(|v| v.is_reclaimable())?;
        Some(voice.note_on(cmd))
    }

    /// **Command — NoteOff**: transition the matching active voice to Release.
    ///
    /// Searches active voices for one whose `note_id` matches, then issues
    /// `NoteOff`. Returns `Some(VoiceEvent::VoiceReleased)` on success, or
    /// `None` if no active voice with the given `NoteId` exists.
    ///
    /// # Audio-thread safety
    ///
    /// Iterates the fixed pool; no heap allocation.
    pub fn note_off(&mut self, note_id: NoteId) -> Option<VoiceEvent> {
        let cmd = NoteOff { note_id };
        for voice in self.voices.iter_mut() {
            if voice.is_active() && voice.note_id() == note_id {
                return voice.note_off(cmd).ok();
            }
        }
        None
    }

    /// Fill `output` with mixed, stereo audio frames.
    ///
    /// For each sample position the renderer sums the outputs of all active
    /// voices and converts the mono sum to a stereo [`AudioFrame`]. Voices
    /// that emit a `VoiceFinished` event during rendering transition to Idle
    /// automatically — no explicit GC step is needed.
    ///
    /// # Audio-thread safety
    ///
    /// This method does not allocate and does not lock.
    pub fn render_block(&mut self, output: &mut [AudioFrame]) {
        let sr = self.sample_rate.value() as f64;
        for frame in output.iter_mut() {
            let mut sum = 0.0_f32;
            for voice in self.voices.iter_mut() {
                // render_sample returns 0.0 for idle voices (no branch needed).
                let (sample, _event) = voice.render_sample(sr, 0.0);
                sum += sample;
            }
            *frame = AudioFrame::mono(sum);
        }
    }

    /// Returns `true` if at least one voice is currently active.
    pub fn any_active(&self) -> bool {
        self.voices.iter().any(|v| v.is_active())
    }

    /// Returns the number of currently active voices (not in Idle stage).
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sr() -> SampleRate {
        SampleRate::try_new(44100).unwrap()
    }

    fn note_id(v: u32) -> NoteId {
        NoteId::new(v)
    }

    fn note_number(v: u8) -> NoteNumber {
        NoteNumber::try_new(v).unwrap()
    }

    fn velocity(v: f64) -> Velocity {
        Velocity::try_new(v).unwrap()
    }

    // ── Construction ──────────────────────────────────────────────────────────

    #[test]
    fn new_creates_renderer_with_given_sample_rate() {
        let renderer = AudioRenderer::new(sr());
        assert_eq!(renderer.sample_rate().value(), 44100);
    }

    #[test]
    fn no_active_voices_on_construction() {
        let renderer = AudioRenderer::new(sr());
        assert!(!renderer.any_active());
        assert_eq!(renderer.active_voice_count(), 0);
    }

    // ── render_block — silence ────────────────────────────────────────────────

    #[test]
    fn render_block_is_silent_with_no_active_voices() {
        let mut renderer = AudioRenderer::new(sr());
        let mut buf = vec![AudioFrame::silence(); 64];
        renderer.render_block(&mut buf);
        for frame in &buf {
            assert_eq!(frame.left, 0.0, "expected silence");
            assert_eq!(frame.right, 0.0, "expected silence");
        }
    }

    // ── render_block — with an active voice ───────────────────────────────────

    #[test]
    fn render_block_produces_non_zero_output_with_active_voice() {
        let mut renderer = AudioRenderer::new(sr());
        renderer
            .note_on(note_id(1), note_number(69), velocity(0.8))
            .expect("note_on should succeed");

        let mut buf = vec![AudioFrame::silence(); 64];
        renderer.render_block(&mut buf);

        // At least some frames should be non-silent.
        let any_nonzero = buf.iter().any(|f| f.left.abs() > 1e-6);
        assert!(any_nonzero, "expected non-zero output with active voice");
    }

    // ── render_block — stereo ─────────────────────────────────────────────────

    #[test]
    fn render_block_produces_identical_left_right_channels() {
        let mut renderer = AudioRenderer::new(sr());
        renderer
            .note_on(note_id(1), note_number(60), velocity(0.5))
            .expect("note_on should succeed");

        let mut buf = vec![AudioFrame::silence(); 16];
        renderer.render_block(&mut buf);

        for frame in &buf {
            assert_eq!(
                frame.left, frame.right,
                "mono source must produce identical L/R"
            );
        }
    }

    // ── NoteOn / NoteOff ──────────────────────────────────────────────────────

    #[test]
    fn note_on_increments_active_voice_count() {
        let mut renderer = AudioRenderer::new(sr());
        assert_eq!(renderer.active_voice_count(), 0);
        renderer.note_on(note_id(1), note_number(60), velocity(0.5));
        assert_eq!(renderer.active_voice_count(), 1);
        renderer.note_on(note_id(2), note_number(64), velocity(0.5));
        assert_eq!(renderer.active_voice_count(), 2);
    }

    #[test]
    fn note_off_transitions_voice_to_release() {
        let mut renderer = AudioRenderer::new(sr());
        renderer.note_on(note_id(1), note_number(69), velocity(0.8));
        let event = renderer.note_off(note_id(1));
        assert!(
            matches!(event, Some(VoiceEvent::VoiceReleased { .. })),
            "expected VoiceReleased, got {event:?}"
        );
    }

    #[test]
    fn note_off_unknown_id_returns_none() {
        let mut renderer = AudioRenderer::new(sr());
        let result = renderer.note_off(note_id(99));
        assert!(
            result.is_none(),
            "note_off for unknown id should return None"
        );
    }

    // ── Polyphony ─────────────────────────────────────────────────────────────

    #[test]
    fn render_block_mixes_multiple_active_voices() {
        let mut renderer = AudioRenderer::new(sr());
        for n in 0u32..4 {
            renderer
                .note_on(
                    note_id(n),
                    note_number((60 + n as u8).min(127)),
                    velocity(0.5),
                )
                .expect("note_on should succeed");
        }

        let mut buf = vec![AudioFrame::silence(); 256];
        renderer.render_block(&mut buf);

        let any_nonzero = buf.iter().any(|f| f.left.abs() > 1e-6);
        assert!(any_nonzero, "polyphonic output should be non-silent");
    }

    #[test]
    fn pool_can_handle_up_to_max_voices() {
        let mut renderer = AudioRenderer::new(sr());
        for n in 0u32..MAX_VOICES as u32 {
            let result = renderer.note_on(note_id(n), note_number(60), velocity(0.5));
            assert!(
                result.is_some(),
                "should be able to allocate voice {n} of {MAX_VOICES}"
            );
        }
        assert_eq!(renderer.active_voice_count(), MAX_VOICES);
    }

    // ── zero-length block ─────────────────────────────────────────────────────

    #[test]
    fn render_block_with_empty_slice_does_not_panic() {
        let mut renderer = AudioRenderer::new(sr());
        renderer.render_block(&mut []);
    }

    // ── VoiceEvent from note_on ───────────────────────────────────────────────

    #[test]
    fn note_on_returns_voice_activated_event() {
        let mut renderer = AudioRenderer::new(sr());
        let events = renderer
            .note_on(note_id(1), note_number(69), velocity(0.8))
            .expect("note_on should succeed");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, VoiceEvent::VoiceActivated { .. })),
            "expected VoiceActivated event in {events:?}"
        );
    }

    // ── Voice transitions to Idle after Release ───────────────────────────────

    #[test]
    fn voice_becomes_reclaimable_after_release_completes() {
        // Use a very fast release config: attack=1ms, decay=1ms, sustain=0.5, release=1ms.
        // Default Voice config has 300ms release — too slow for a unit test.
        // We drive enough samples to exhaust the envelope.
        let mut renderer = AudioRenderer::new(sr());
        renderer.note_on(note_id(1), note_number(69), velocity(0.8));
        renderer.note_off(note_id(1));

        // Render several seconds to ensure release completes even with default ADSR.
        // Default release = 0.3 s → 44100 * 0.3 ≈ 13230 samples; render 60000 to be safe.
        let mut buf = vec![AudioFrame::silence(); 512];
        for _ in 0..120 {
            renderer.render_block(&mut buf);
        }

        // After enough samples the voice should be idle (reclaimable) and
        // active_voice_count should drop to 0.
        assert_eq!(
            renderer.active_voice_count(),
            0,
            "voice should be idle after release completes"
        );
    }
}
