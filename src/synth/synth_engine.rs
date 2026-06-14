// path: src/synth/synth_engine.rs

use crate::kernel::audio_frame::AudioFrame;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::velocity::Velocity;
use crate::synth::filter_config::FilterConfig;
use crate::synth::oscillator_config::OscillatorConfig;

/// Event data carried with a `note_on` command.
///
/// Bundles the note number and velocity for a new note-on event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteOn {
    /// MIDI note number (0–127).
    pub note_number: NoteNumber,
    /// Normalized attack velocity (0.0–1.0).
    pub velocity: Velocity,
}

impl NoteOn {
    /// Construct a `NoteOn` event from a note number and velocity.
    pub fn new(note_number: NoteNumber, velocity: Velocity) -> Self {
        Self {
            note_number,
            velocity,
        }
    }
}

/// Event data carried with a `note_off` command.
///
/// Carries a release velocity that implementations may use to shape the
/// release tail.  Implementations that do not support release velocity may
/// ignore the field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteOff {
    /// Release velocity (0.0–1.0).
    pub velocity: Velocity,
}

impl NoteOff {
    /// Construct a `NoteOff` with the given release velocity.
    pub fn new(velocity: Velocity) -> Self {
        Self { velocity }
    }

    /// Construct a `NoteOff` with maximum (1.0) release velocity.
    ///
    /// Equivalent to a standard MIDI note-off with no release-velocity data.
    pub fn default_velocity() -> Self {
        Self {
            velocity: Velocity::try_new(1.0).expect("1.0 is a valid velocity"),
        }
    }
}

/// Port: a single synthesizer voice.
///
/// `SynthEngine` defines the functional interface for voice synthesis.
/// Methods take `Voice` by value and return a (possibly new) `Voice`, keeping
/// the audio pipeline free of mutable shared state.
///
/// # Contract
///
/// | Method | Signature | Semantics |
/// |--------|-----------|----------|
/// | `note_on` | `(Voice, NoteOn) -> Voice` | Transition voice to sounding state. |
/// | `note_off` | `(Voice, NoteOff) -> Voice` | Transition voice to release stage. |
/// | `is_finished` | `Voice -> bool` | `true` once release tail has completed. |
/// | `render_block` | `(Voice, OscillatorConfig, FilterConfig) -> [AudioFrame]` | Advance voice by one block and return frames. |
///
/// # Audio-thread safety
///
/// Implementations MUST NOT allocate on the heap, take locks, or perform
/// blocking I/O during `render_block`.  The other methods may be called from
/// non-realtime contexts.
pub trait SynthEngine {
    /// The concrete voice type managed by this engine.
    type Voice: Clone;

    /// Apply a note-on event, returning the updated voice in the sounding state.
    fn note_on(&self, voice: Self::Voice, event: NoteOn) -> Self::Voice;

    /// Apply a note-off event, returning the voice in its release stage.
    fn note_off(&self, voice: Self::Voice, event: NoteOff) -> Self::Voice;

    /// Return `true` if the voice has finished sounding and can be freed.
    fn is_finished(&self, voice: &Self::Voice) -> bool;

    /// Render one block of audio from the voice.
    ///
    /// Returns a `(Voice, Vec<AudioFrame>)` pair: the new voice state and
    /// exactly `block_size` frames of audio.
    ///
    /// # Parameters
    ///
    /// - `voice` — the current voice state (consumed; returned as updated state).
    /// - `osc` — per-block oscillator configuration (waveform, detune, etc.).
    /// - `filter` — per-block filter configuration (cutoff, resonance, etc.).
    /// - `block_size` — number of [`AudioFrame`]s to generate.
    fn render_block(
        &self,
        voice: Self::Voice,
        osc: OscillatorConfig,
        filter: FilterConfig,
        block_size: usize,
    ) -> (Self::Voice, Vec<AudioFrame>);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::audio_frame::AudioFrame;
    use crate::kernel::note_number::NoteNumber;
    use crate::kernel::velocity::Velocity;
    use crate::synth::filter_config::{FilterConfig, FilterType};
    use crate::synth::oscillator_config::{OscillatorConfig, Waveform};

    // ── Helpers ────────────────────────────────────────────

    fn vel(v: f64) -> Velocity {
        Velocity::try_new(v).unwrap()
    }

    fn note(n: u8) -> NoteNumber {
        NoteNumber::try_new(n).unwrap()
    }

    fn osc() -> OscillatorConfig {
        OscillatorConfig::try_new(0.0, 0.5, Waveform::Sine).unwrap()
    }

    fn filter() -> FilterConfig {
        FilterConfig::try_new(1_000.0, FilterType::LowPass, 0.0).unwrap()
    }

    // ── NoteOn ───────────────────────────────────────────────

    #[test]
    fn note_on_stores_fields() {
        let ev = NoteOn::new(note(69), vel(0.8));
        assert_eq!(ev.note_number.value(), 69);
        assert!((ev.velocity.value() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn note_on_copy_semantics() {
        let a = NoteOn::new(note(60), vel(0.5));
        let b = a;
        assert_eq!(a, b);
    }

    // ── NoteOff ──────────────────────────────────────────────

    #[test]
    fn note_off_new_stores_velocity() {
        let ev = NoteOff::new(vel(0.6));
        assert!((ev.velocity.value() - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn note_off_default_velocity_is_one() {
        let ev = NoteOff::default_velocity();
        assert!((ev.velocity.value() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn note_off_copy_semantics() {
        let a = NoteOff::new(vel(0.3));
        let b = a;
        assert_eq!(a, b);
    }

    // ── SynthEngine (stub impl) ─────────────────────────────────────

    /// Minimal stub voice used only within this test module.
    #[derive(Clone)]
    struct StubVoice {
        active: bool,
        releasing: bool,
        release_blocks_remaining: u32,
    }

    impl StubVoice {
        fn idle() -> Self {
            Self {
                active: false,
                releasing: false,
                release_blocks_remaining: 0,
            }
        }
    }

    struct StubEngine;

    impl SynthEngine for StubEngine {
        type Voice = StubVoice;

        fn note_on(&self, _voice: StubVoice, _event: NoteOn) -> StubVoice {
            StubVoice {
                active: true,
                releasing: false,
                release_blocks_remaining: 0,
            }
        }

        fn note_off(&self, voice: StubVoice, _event: NoteOff) -> StubVoice {
            if voice.active {
                StubVoice {
                    active: true,
                    releasing: true,
                    release_blocks_remaining: 2,
                }
            } else {
                voice
            }
        }

        fn is_finished(&self, voice: &StubVoice) -> bool {
            !voice.active
        }

        fn render_block(
            &self,
            voice: StubVoice,
            _osc: OscillatorConfig,
            _filter: FilterConfig,
            block_size: usize,
        ) -> (StubVoice, Vec<AudioFrame>) {
            let frames = vec![AudioFrame::silence(); block_size];
            let next = if voice.releasing {
                let remaining = voice.release_blocks_remaining.saturating_sub(1);
                StubVoice {
                    active: remaining > 0,
                    releasing: remaining > 0,
                    release_blocks_remaining: remaining,
                }
            } else {
                voice
            };
            (next, frames)
        }
    }

    #[test]
    fn note_on_activates_voice() {
        let engine = StubEngine;
        let voice = StubVoice::idle();
        let active = engine.note_on(voice, NoteOn::new(note(60), vel(0.8)));
        assert!(active.active);
        assert!(!engine.is_finished(&active));
    }

    #[test]
    fn is_finished_true_for_idle_voice() {
        let engine = StubEngine;
        assert!(engine.is_finished(&StubVoice::idle()));
    }

    #[test]
    fn note_off_begins_release() {
        let engine = StubEngine;
        let voice = StubVoice::idle();
        let active = engine.note_on(voice, NoteOn::new(note(60), vel(0.8)));
        let releasing = engine.note_off(active, NoteOff::default_velocity());
        assert!(releasing.releasing);
        assert!(!engine.is_finished(&releasing));
    }

    #[test]
    fn voice_finishes_after_release_blocks() {
        let engine = StubEngine;
        let voice = StubVoice::idle();
        let active = engine.note_on(voice, NoteOn::new(note(60), vel(0.8)));
        let releasing = engine.note_off(active, NoteOff::default_velocity());

        let (v1, frames1) = engine.render_block(releasing, osc(), filter(), 64);
        assert_eq!(frames1.len(), 64);
        assert!(!engine.is_finished(&v1));

        let (v2, _) = engine.render_block(v1, osc(), filter(), 64);
        assert!(engine.is_finished(&v2));
    }

    #[test]
    fn render_block_returns_correct_frame_count() {
        let engine = StubEngine;
        let voice = StubVoice::idle();
        let active = engine.note_on(voice, NoteOn::new(note(60), vel(0.5)));
        let (_, frames) = engine.render_block(active, osc(), filter(), 128);
        assert_eq!(frames.len(), 128);
    }

    #[test]
    fn render_block_zero_size_returns_empty() {
        let engine = StubEngine;
        let voice = StubVoice::idle();
        let active = engine.note_on(voice, NoteOn::new(note(60), vel(0.5)));
        let (_, frames) = engine.render_block(active, osc(), filter(), 0);
        assert!(frames.is_empty());
    }

    #[test]
    fn voice_can_be_cloned() {
        let engine = StubEngine;
        let voice = StubVoice::idle();
        let active = engine.note_on(voice, NoteOn::new(note(69), vel(1.0)));
        let cloned = active.clone();
        assert_eq!(active.active, cloned.active);
    }

    #[test]
    fn synth_engine_usable_as_generic_bound() {
        fn use_engine<E: SynthEngine>(engine: &E, voice: E::Voice) -> bool {
            engine.is_finished(&voice)
        }
        let engine = StubEngine;
        let voice = StubVoice::idle();
        assert!(use_engine(&engine, voice));
    }
}
