// path: src/engine/voice_renderer.rs

//! Domain service: renders one voice for one buffer.
//!
//! `VoiceRenderer` is the domain service that composes the `Oscillator`,
//! `Filter`, and `EnvelopeGenerator` ports to turn one `Voice`'s current
//! state into a buffer of audio samples, following the canonical per-voice
//! signal path: oscillator, then filter, then envelope shaping.
//!
//! `VoiceRenderer` holds no state of its own and never constructs its own
//! collaborators -- every dependency (the port implementations, their
//! configuration, and the phase/filter/envelope state that must persist
//! across buffers) is passed in by the caller. This keeps the service
//! real-time safe (no allocation, no locking, no I/O) and lets callers
//! substitute any `Oscillator` / `Filter` / `EnvelopeGenerator`
//! implementation without modifying this service (dependency inversion).
//! `render` is generic over the three port traits rather than accepting
//! trait objects, so the inner sample loop uses static dispatch only.
//!
//! `Voice` does not expose a way to mutate its phase (only the
//! `Trigger` / `Release` / `ApplyExpression` commands and the amp-envelope
//! `advance` defined on the aggregate can change its state), so this
//! service treats the running oscillator phase as an explicit external
//! cursor: callers pass in the phase a voice last left off at and receive
//! back the phase to resume from on the next buffer, rather than this
//! service inventing a phase mutator the aggregate does not define.

use crate::engine::envelope_generator::EnvelopeGenerator;
use crate::engine::filter::{Filter, FilterConfig};
use crate::engine::oscillator::{Frequency, Oscillator, OscillatorConfig, SampleRate};
use crate::engine::voice::{AmpEnvelopeStage, Voice};

/// Converts a MIDI note number to its equal-tempered frequency, using A4
/// (MIDI note 69) as the 440 Hz reference pitch.
fn note_number_to_frequency(note: u8) -> Frequency {
    let semitones_from_a4 = f64::from(note) - 69.0;
    let hertz = 440.0 * 2f64.powf(semitones_from_a4 / 12.0);
    // Every MIDI note number in `0..=127` produces a finite, strictly
    // positive hertz value, so this conversion cannot fail.
    Frequency::try_new(hertz).expect("MIDI note numbers always yield a valid frequency")
}

/// Renders one `Voice` for one output buffer by composing the `Oscillator`,
/// `Filter`, and `EnvelopeGenerator` ports along the canonical per-voice
/// signal path: oscillator, then filter, then envelope.
///
/// Stateless orchestration service: all real-time-relevant state (the
/// running phase cursor, the filter's delay-line state, the envelope
/// generator's stage/level state) is owned by the caller and passed in by
/// reference, so one `VoiceRenderer` can render any number of
/// concurrently-active voices without interference between them.
#[derive(Debug, Default, Clone, Copy)]
pub struct VoiceRenderer;

impl VoiceRenderer {
    /// Constructs a `VoiceRenderer`. Stateless, so this never fails and
    /// never allocates.
    pub fn new() -> Self {
        Self
    }

    /// Renders `voice` into `output`, one sample per element, starting from
    /// `phase` and returning the phase to resume from on the next call.
    ///
    /// Each sample follows the canonical path: `oscillator.render` produces
    /// a raw waveform sample at the voice's pitch, `filter.process` shapes
    /// its timbre, and `envelope.tick` scales the result by the current
    /// envelope level and the voice's velocity. `oscillator.advance` moves
    /// the phase cursor forward for the next sample.
    ///
    /// A voice whose amp envelope has reached `Idle` (see
    /// `Voice::is_reclaimable`) is silent: `output` is filled with zeros,
    /// the phase cursor is returned unchanged, and neither the oscillator,
    /// the filter, nor the envelope generator are invoked -- an idle voice
    /// contributes nothing to the mix and must not perturb any downstream
    /// state.
    #[allow(clippy::too_many_arguments)]
    pub fn render<O, F, E>(
        &self,
        voice: &Voice,
        phase: f64,
        oscillator: &O,
        oscillator_config: OscillatorConfig,
        filter: &mut F,
        filter_config: FilterConfig,
        envelope: &mut E,
        sample_rate: SampleRate,
        output: &mut [f64],
    ) -> f64
    where
        O: Oscillator,
        F: Filter,
        E: EnvelopeGenerator,
    {
        if voice.amp_stage() == AmpEnvelopeStage::Idle {
            output.fill(0.0);
            return phase;
        }

        let frequency = note_number_to_frequency(voice.note().value());
        let velocity = voice.velocity().value();
        let mut current_phase = phase;

        for sample in output.iter_mut() {
            let raw = oscillator.render(current_phase, oscillator_config);
            let filtered = filter.process(raw, filter_config);
            let envelope_level = envelope.tick();
            *sample = filtered * envelope_level * velocity;
            current_phase = oscillator.advance(current_phase, frequency, sample_rate);
        }

        current_phase
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::filter::FilterKind;
    use crate::engine::oscillator::{Amplitude, Waveform};
    use crate::engine::voice::{EnvelopeTiming, NoteId, NoteNumber, Velocity, VoiceConfig};

    /// A test double that renders its input phase directly as the sample
    /// value and advances phase by `frequency / sample_rate`, so tests can
    /// assert on exact arithmetic without depending on any particular
    /// waveform shape.
    struct IdentityOscillator;

    impl Oscillator for IdentityOscillator {
        fn advance(&self, phase: f64, frequency: Frequency, sample_rate: SampleRate) -> f64 {
            (phase + frequency.hertz() / sample_rate.samples_per_second()).rem_euclid(1.0)
        }

        fn render(&self, phase: f64, _config: OscillatorConfig) -> f64 {
            phase
        }
    }

    /// A test double that passes samples through unchanged, isolating
    /// renderer composition logic from any particular filter topology.
    struct IdentityFilter;

    impl Filter for IdentityFilter {
        fn process(&mut self, sample: f64, _config: FilterConfig) -> f64 {
            sample
        }

        fn reset(&mut self) {}
    }

    /// A deterministic test double that always reports a fixed envelope
    /// level and counts how many times it was ticked, so tests can isolate
    /// the renderer's composition logic from any particular envelope shape.
    struct FixedEnvelope {
        level: f64,
        ticks: usize,
    }

    impl EnvelopeGenerator for FixedEnvelope {
        fn trigger(&mut self) {}
        fn release(&mut self) {}
        fn tick(&mut self) -> f64 {
            self.ticks += 1;
            self.level
        }
    }

    fn envelope_timing() -> EnvelopeTiming {
        EnvelopeTiming::new(0.1, 0.1, 0.5, 0.1)
    }

    fn triggered_voice(note: u8, velocity: f64) -> Voice {
        let config = VoiceConfig::new(envelope_timing());
        let mut voice = Voice::new(
            config,
            NoteNumber::try_new(note).unwrap(),
            NoteId::new(1),
            Velocity::try_new(velocity).unwrap(),
        );
        voice
            .trigger(
                NoteNumber::try_new(note).unwrap(),
                NoteId::new(1),
                Velocity::try_new(velocity).unwrap(),
            )
            .unwrap();
        voice
    }

    fn osc_config() -> OscillatorConfig {
        OscillatorConfig::new(Waveform::Sine, Amplitude::try_new(1.0).unwrap())
    }

    fn filter_config() -> FilterConfig {
        FilterConfig::new(FilterKind::LowPass, 20_000.0, 0.0, 48_000.0)
    }

    #[test]
    fn renders_exactly_one_sample_per_output_slot() {
        let renderer = VoiceRenderer::new();
        let voice = triggered_voice(69, 1.0);
        let osc = IdentityOscillator;
        let mut filter = IdentityFilter;
        let mut envelope = FixedEnvelope {
            level: 1.0,
            ticks: 0,
        };
        let sample_rate = SampleRate::try_new(48_000.0).unwrap();
        let mut output = [0.0; 8];

        renderer.render(
            &voice,
            0.0,
            &osc,
            osc_config(),
            &mut filter,
            filter_config(),
            &mut envelope,
            sample_rate,
            &mut output,
        );

        assert_eq!(envelope.ticks, 8);
    }

    #[test]
    fn idle_voice_renders_silence_without_advancing_phase_or_touching_ports() {
        let renderer = VoiceRenderer::new();
        let voice = Voice::new(
            VoiceConfig::new(envelope_timing()),
            NoteNumber::try_new(60).unwrap(),
            NoteId::new(1),
            Velocity::try_new(1.0).unwrap(),
        );
        let osc = IdentityOscillator;
        let mut filter = IdentityFilter;
        let mut envelope = FixedEnvelope {
            level: 1.0,
            ticks: 0,
        };
        let sample_rate = SampleRate::try_new(48_000.0).unwrap();
        let mut output = [1.0; 4];

        let returned_phase = renderer.render(
            &voice,
            0.25,
            &osc,
            osc_config(),
            &mut filter,
            filter_config(),
            &mut envelope,
            sample_rate,
            &mut output,
        );

        assert_eq!(output, [0.0; 4]);
        assert_eq!(returned_phase, 0.25);
        assert_eq!(envelope.ticks, 0);
    }

    #[test]
    fn output_is_scaled_by_envelope_level_and_velocity() {
        let renderer = VoiceRenderer::new();
        let voice = triggered_voice(69, 0.5);
        let osc = IdentityOscillator;
        let mut filter = IdentityFilter;
        let mut envelope = FixedEnvelope {
            level: 0.5,
            ticks: 0,
        };
        let sample_rate = SampleRate::try_new(48_000.0).unwrap();
        let mut output = [0.0; 1];

        renderer.render(
            &voice,
            0.3,
            &osc,
            osc_config(),
            &mut filter,
            filter_config(),
            &mut envelope,
            sample_rate,
            &mut output,
        );

        // IdentityOscillator renders the phase itself (0.3); IdentityFilter
        // passes it through unchanged; the envelope level (0.5) and
        // velocity (0.5) each scale the result multiplicatively.
        assert!((output[0] - (0.3 * 0.5 * 0.5)).abs() < 1e-12);
    }

    #[test]
    fn phase_advances_according_to_the_oscillator_across_the_whole_buffer() {
        let renderer = VoiceRenderer::new();
        let voice = triggered_voice(69, 1.0);
        let osc = IdentityOscillator;
        let mut filter = IdentityFilter;
        let mut envelope = FixedEnvelope {
            level: 1.0,
            ticks: 0,
        };
        let sample_rate = SampleRate::try_new(1000.0).unwrap();
        let mut output = [0.0; 4];

        let frequency = note_number_to_frequency(voice.note().value());
        let mut expected_phase = 0.0;
        for _ in 0..4 {
            expected_phase = osc.advance(expected_phase, frequency, sample_rate);
        }

        let returned_phase = renderer.render(
            &voice,
            0.0,
            &osc,
            osc_config(),
            &mut filter,
            filter_config(),
            &mut envelope,
            sample_rate,
            &mut output,
        );

        assert!((returned_phase - expected_phase).abs() < 1e-12);
    }

    #[test]
    fn note_69_maps_to_440_hertz_a4() {
        let frequency = note_number_to_frequency(69);
        assert!((frequency.hertz() - 440.0).abs() < 1e-9);
    }
}
