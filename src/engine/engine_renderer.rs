// path: src/engine/engine_renderer.rs

//! Domain service: iterates all active voices and sums their output to
//! stereo.
//!
//! `EngineRenderer` composes `VoiceAllocator` (which owns the fixed voice
//! pool and knows which slots are currently sounding) with `VoiceRenderer`
//! (which renders one voice for one buffer along the oscillator -> filter
//! -> envelope path) to produce the engine's contribution to the canonical
//! signal path: `engine output -> channel strip inserts -> volume and pan
//! -> ...`. Panning is a channel-strip concern that happens downstream of
//! this service, so each active voice's mono output is duplicated to both
//! channels (via `AudioFrame::from_mono`) rather than panned here.
//!
//! `EngineRenderer` owns neither the voice pool (that is
//! `VoiceAllocator`'s responsibility) nor the per-voice oscillator/filter/
//! envelope rendering (that is `VoiceRenderer`'s responsibility); its
//! single responsibility is iterating every managed voice slot, skipping
//! reclaimable (silent) voices, and summing each active voice's rendered
//! buffer into the shared stereo output. All collaborators are accepted as
//! parameters rather than constructed internally, so tests can substitute
//! any `Oscillator` / `Filter` / `EnvelopeGenerator` implementation.
//!
//! Real-time safety: `render` never allocates heap memory, never locks,
//! and never performs I/O. It requires the caller to supply a `scratch`
//! buffer sized to the output length and a `state` slice with exactly one
//! `VoiceRenderState` per voice managed by the allocator
//! (`allocator.polyphony()` entries) -- both constructed once, outside the
//! audio callback -- so no collection is created inside this method. The
//! oscillator is shared (`&O`) across every voice because, per
//! `VoiceRenderer`'s design, oscillators are stateless: the running phase
//! is an explicit external cursor carried in `VoiceRenderState`, not
//! internal oscillator state.

use std::fmt;

use crate::engine::envelope_generator::EnvelopeGenerator;
use crate::engine::filter::{Filter, FilterConfig};
use crate::engine::oscillator::{Oscillator, OscillatorConfig, SampleRate};
use crate::engine::voice_allocator::VoiceAllocator;
use crate::engine::voice_renderer::VoiceRenderer;
use crate::kernel::audio_frame::AudioFrame;

/// Errors returned when `EngineRenderer::render` is called with
/// mismatched buffer or state lengths.
///
/// These are reported rather than panicked on because `render` runs on
/// the real-time audio thread, where panics must never occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineRendererError {
    /// `state.len()` did not equal `allocator.polyphony()`.
    StateLengthMismatch { expected: usize, actual: usize },
    /// `scratch.len()` did not equal `output.len()`.
    ScratchLengthMismatch { expected: usize, actual: usize },
}

impl fmt::Display for EngineRendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineRendererError::StateLengthMismatch { expected, actual } => write!(
                f,
                "voice render state length {actual} does not match allocator polyphony {expected}"
            ),
            EngineRendererError::ScratchLengthMismatch { expected, actual } => write!(
                f,
                "scratch buffer length {actual} does not match output buffer length {expected}"
            ),
        }
    }
}

impl std::error::Error for EngineRendererError {}

/// Per-voice real-time state needed to render one voice across buffers.
///
/// `Voice` exposes no phase mutator (only `Trigger` / `Release` /
/// `ApplyExpression` and the amp-envelope `advance` change its state), so
/// `VoiceRenderer` treats the oscillator phase as an explicit external
/// cursor that the caller must carry forward between buffers. This type
/// bundles that phase cursor together with the voice's own filter and
/// envelope-generator instances, since both are stateful (a filter has a
/// delay line, an envelope generator tracks its stage/level) and must not
/// be shared between voices.
///
/// One `VoiceRenderState` exists per slot managed by a `VoiceAllocator`
/// (`allocator.polyphony()` entries), indexed identically to the
/// allocator's own slots, so `EngineRenderer::render` can pair each voice
/// with the exact filter/envelope state it left off at on the previous
/// buffer.
pub struct VoiceRenderState<F, E>
where
    F: Filter,
    E: EnvelopeGenerator,
{
    phase: f64,
    filter: F,
    envelope: E,
}

impl<F, E> VoiceRenderState<F, E>
where
    F: Filter,
    E: EnvelopeGenerator,
{
    /// Construct render state for one voice slot, with the oscillator
    /// phase cursor starting at zero.
    pub fn new(filter: F, envelope: E) -> Self {
        Self {
            phase: 0.0,
            filter,
            envelope,
        }
    }

    /// The oscillator phase this voice last left off at.
    pub fn phase(&self) -> f64 {
        self.phase
    }

    /// This voice's filter instance.
    pub fn filter(&self) -> &F {
        &self.filter
    }

    /// This voice's envelope-generator instance.
    pub fn envelope(&self) -> &E {
        &self.envelope
    }
}

/// Iterates all voices managed by a `VoiceAllocator`, renders every voice
/// that is not reclaimable (i.e. currently sounding) via `VoiceRenderer`,
/// and sums the results into a stereo output buffer.
///
/// Stateless orchestration service: it holds no data of its own and never
/// constructs its own collaborators. One `EngineRenderer` can drive any
/// number of `VoiceAllocator` instances without interference between them.
#[derive(Debug, Default, Clone, Copy)]
pub struct EngineRenderer;

impl EngineRenderer {
    /// Constructs an `EngineRenderer`. Stateless, so this never fails and
    /// never allocates.
    pub fn new() -> Self {
        Self
    }

    /// Renders every active (non-reclaimable) voice managed by
    /// `allocator` and sums their output into `output`, one `AudioFrame`
    /// per element.
    ///
    /// `output` is fully overwritten: it is first cleared to silence, then
    /// each active voice's mono buffer (rendered via `voice_renderer` into
    /// `scratch`) is duplicated to both channels and mixed in. A
    /// reclaimable voice contributes nothing and is skipped entirely
    /// (matching `VoiceRenderer`'s own silent-idle-voice behavior, but
    /// without paying the cost of invoking the oscillator, filter, or
    /// envelope generator for it).
    ///
    /// Returns `Err` without mutating `output` if `state` is not sized to
    /// `allocator.polyphony()`, or if `scratch` is not sized to
    /// `output.len()` -- both are caller programming errors, reported
    /// rather than panicked on because this runs on the real-time audio
    /// thread.
    #[allow(clippy::too_many_arguments)]
    pub fn render<O, F, E>(
        &self,
        allocator: &VoiceAllocator,
        voice_renderer: &VoiceRenderer,
        oscillator: &O,
        oscillator_config: OscillatorConfig,
        filter_config: FilterConfig,
        sample_rate: SampleRate,
        state: &mut [VoiceRenderState<F, E>],
        scratch: &mut [f64],
        output: &mut [AudioFrame],
    ) -> Result<(), EngineRendererError>
    where
        O: Oscillator,
        F: Filter,
        E: EnvelopeGenerator,
    {
        let polyphony = allocator.polyphony();
        if state.len() != polyphony {
            return Err(EngineRendererError::StateLengthMismatch {
                expected: polyphony,
                actual: state.len(),
            });
        }
        if scratch.len() != output.len() {
            return Err(EngineRendererError::ScratchLengthMismatch {
                expected: output.len(),
                actual: scratch.len(),
            });
        }

        output.fill(AudioFrame::silence());

        for (index, slot_state) in state.iter_mut().enumerate().take(polyphony) {
            let voice = match allocator.voice(index) {
                Some(voice) => voice,
                None => continue,
            };
            if voice.is_reclaimable() {
                continue;
            }

            let next_phase = voice_renderer.render(
                voice,
                slot_state.phase,
                oscillator,
                oscillator_config,
                &mut slot_state.filter,
                filter_config,
                &mut slot_state.envelope,
                sample_rate,
                scratch,
            );
            slot_state.phase = next_phase;

            for (frame, &sample) in output.iter_mut().zip(scratch.iter()) {
                *frame += AudioFrame::from_mono(sample as f32);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::filter::FilterKind;
    use crate::engine::oscillator::{Amplitude, Frequency, Waveform};
    use crate::engine::voice::{EnvelopeTiming, NoteId, NoteNumber, Velocity, VoiceConfig};
    use crate::engine::voice_allocator::StealPolicy;

    /// A test double that always renders a fixed sample regardless of
    /// phase, isolating summing behavior from any particular waveform.
    struct ConstantOscillator(f64);

    impl Oscillator for ConstantOscillator {
        fn advance(&self, phase: f64, _frequency: Frequency, _sample_rate: SampleRate) -> f64 {
            phase
        }

        fn render(&self, _phase: f64, _config: OscillatorConfig) -> f64 {
            self.0
        }
    }

    /// A test double that advances phase by a fixed step per sample, so
    /// tests can assert the phase cursor persists across `render` calls.
    struct IncrementingOscillator;

    impl Oscillator for IncrementingOscillator {
        fn advance(&self, phase: f64, _frequency: Frequency, _sample_rate: SampleRate) -> f64 {
            phase + 1.0
        }

        fn render(&self, phase: f64, _config: OscillatorConfig) -> f64 {
            phase
        }
    }

    /// A test double that passes samples through unchanged, isolating
    /// summing behavior from any particular filter topology.
    struct PassthroughFilter;

    impl Filter for PassthroughFilter {
        fn process(&mut self, sample: f64, _config: FilterConfig) -> f64 {
            sample
        }

        fn reset(&mut self) {}
    }

    /// A deterministic test double that always reports a fixed envelope
    /// level, isolating summing behavior from any particular envelope
    /// shape.
    struct FixedLevelEnvelope(f64);

    impl EnvelopeGenerator for FixedLevelEnvelope {
        fn trigger(&mut self) {}
        fn release(&mut self) {}
        fn tick(&mut self) -> f64 {
            self.0
        }
    }

    fn osc_config() -> OscillatorConfig {
        OscillatorConfig::new(Waveform::Sine, Amplitude::try_new(1.0).unwrap())
    }

    fn filter_config() -> FilterConfig {
        FilterConfig::new(FilterKind::LowPass, 20_000.0, 0.0, 48_000.0)
    }

    fn timing() -> EnvelopeTiming {
        EnvelopeTiming::new(0.1, 0.1, 0.5, 0.1)
    }

    fn state_for(polyphony: usize) -> Vec<VoiceRenderState<PassthroughFilter, FixedLevelEnvelope>> {
        (0..polyphony)
            .map(|_| VoiceRenderState::new(PassthroughFilter, FixedLevelEnvelope(1.0)))
            .collect()
    }

    #[test]
    fn sums_all_active_voices_into_output() {
        let mut allocator =
            VoiceAllocator::new(VoiceConfig::new(timing()), 2, StealPolicy::Oldest).unwrap();
        allocator
            .allocate(
                NoteNumber::try_new(60).unwrap(),
                NoteId::new(1),
                Velocity::try_new(1.0).unwrap(),
            )
            .unwrap();
        allocator
            .allocate(
                NoteNumber::try_new(64).unwrap(),
                NoteId::new(2),
                Velocity::try_new(0.5).unwrap(),
            )
            .unwrap();

        let voice_renderer = VoiceRenderer::new();
        let engine_renderer = EngineRenderer::new();
        let oscillator = ConstantOscillator(1.0);
        let sample_rate = SampleRate::try_new(48_000.0).unwrap();

        let mut state = state_for(2);
        let mut scratch = [0.0_f64; 4];
        let mut output = [AudioFrame::silence(); 4];

        engine_renderer
            .render(
                &allocator,
                &voice_renderer,
                &oscillator,
                osc_config(),
                filter_config(),
                sample_rate,
                &mut state,
                &mut scratch,
                &mut output,
            )
            .unwrap();

        // Each active voice contributes oscillator(1.0) * envelope(1.0) *
        // velocity: voice 0 (velocity 1.0) contributes 1.0, voice 1
        // (velocity 0.5) contributes 0.5, summing to 1.5 on both channels.
        let expected_mono = 1.5_f32;
        for frame in output {
            assert!((frame.left() - expected_mono).abs() < 1e-5);
            assert!((frame.right() - expected_mono).abs() < 1e-5);
        }
    }

    #[test]
    fn skips_reclaimable_voices() {
        let allocator =
            VoiceAllocator::new(VoiceConfig::new(timing()), 2, StealPolicy::Oldest).unwrap();
        // No notes allocated: every voice starts out idle/reclaimable.
        let voice_renderer = VoiceRenderer::new();
        let engine_renderer = EngineRenderer::new();
        let oscillator = ConstantOscillator(1.0);
        let sample_rate = SampleRate::try_new(48_000.0).unwrap();

        let mut state = state_for(2);
        let mut scratch = [0.0_f64; 4];
        let mut output = [AudioFrame::new(9.0, 9.0); 4];

        engine_renderer
            .render(
                &allocator,
                &voice_renderer,
                &oscillator,
                osc_config(),
                filter_config(),
                sample_rate,
                &mut state,
                &mut scratch,
                &mut output,
            )
            .unwrap();

        for frame in output {
            assert_eq!(frame, AudioFrame::silence());
        }
    }

    #[test]
    fn errors_on_state_length_mismatch() {
        let allocator =
            VoiceAllocator::new(VoiceConfig::new(timing()), 2, StealPolicy::Oldest).unwrap();
        let voice_renderer = VoiceRenderer::new();
        let engine_renderer = EngineRenderer::new();
        let oscillator = ConstantOscillator(1.0);
        let sample_rate = SampleRate::try_new(48_000.0).unwrap();

        let mut state = state_for(1); // wrong length: allocator manages 2 voices
        let mut scratch = [0.0_f64; 4];
        let mut output = [AudioFrame::silence(); 4];

        let result = engine_renderer.render(
            &allocator,
            &voice_renderer,
            &oscillator,
            osc_config(),
            filter_config(),
            sample_rate,
            &mut state,
            &mut scratch,
            &mut output,
        );

        assert_eq!(
            result,
            Err(EngineRendererError::StateLengthMismatch {
                expected: 2,
                actual: 1
            })
        );
    }

    #[test]
    fn errors_on_scratch_length_mismatch() {
        let allocator =
            VoiceAllocator::new(VoiceConfig::new(timing()), 1, StealPolicy::Oldest).unwrap();
        let voice_renderer = VoiceRenderer::new();
        let engine_renderer = EngineRenderer::new();
        let oscillator = ConstantOscillator(1.0);
        let sample_rate = SampleRate::try_new(48_000.0).unwrap();

        let mut state = state_for(1);
        let mut scratch = [0.0_f64; 3]; // mismatched vs output length 4
        let mut output = [AudioFrame::silence(); 4];

        let result = engine_renderer.render(
            &allocator,
            &voice_renderer,
            &oscillator,
            osc_config(),
            filter_config(),
            sample_rate,
            &mut state,
            &mut scratch,
            &mut output,
        );

        assert_eq!(
            result,
            Err(EngineRendererError::ScratchLengthMismatch {
                expected: 4,
                actual: 3
            })
        );
    }

    #[test]
    fn phase_persists_in_state_across_render_calls() {
        let mut allocator =
            VoiceAllocator::new(VoiceConfig::new(timing()), 1, StealPolicy::Oldest).unwrap();
        allocator
            .allocate(
                NoteNumber::try_new(60).unwrap(),
                NoteId::new(1),
                Velocity::try_new(1.0).unwrap(),
            )
            .unwrap();

        let voice_renderer = VoiceRenderer::new();
        let engine_renderer = EngineRenderer::new();
        let oscillator = IncrementingOscillator;
        let sample_rate = SampleRate::try_new(48_000.0).unwrap();

        let mut state = state_for(1);
        let mut scratch = [0.0_f64; 2];
        let mut output = [AudioFrame::silence(); 2];

        engine_renderer
            .render(
                &allocator,
                &voice_renderer,
                &oscillator,
                osc_config(),
                filter_config(),
                sample_rate,
                &mut state,
                &mut scratch,
                &mut output,
            )
            .unwrap();
        assert_eq!(state[0].phase(), 2.0);

        engine_renderer
            .render(
                &allocator,
                &voice_renderer,
                &oscillator,
                osc_config(),
                filter_config(),
                sample_rate,
                &mut state,
                &mut scratch,
                &mut output,
            )
            .unwrap();
        assert_eq!(state[0].phase(), 4.0);
    }

    #[test]
    fn output_duplicates_mono_sum_to_both_channels_unpanned() {
        // Panning happens downstream at the channel strip, so a single
        // active voice's mono contribution must appear identically on
        // both output channels here.
        let mut allocator =
            VoiceAllocator::new(VoiceConfig::new(timing()), 1, StealPolicy::Oldest).unwrap();
        allocator
            .allocate(
                NoteNumber::try_new(60).unwrap(),
                NoteId::new(1),
                Velocity::try_new(1.0).unwrap(),
            )
            .unwrap();

        let voice_renderer = VoiceRenderer::new();
        let engine_renderer = EngineRenderer::new();
        let oscillator = ConstantOscillator(0.75);
        let sample_rate = SampleRate::try_new(48_000.0).unwrap();

        let mut state = state_for(1);
        let mut scratch = [0.0_f64; 1];
        let mut output = [AudioFrame::silence(); 1];

        engine_renderer
            .render(
                &allocator,
                &voice_renderer,
                &oscillator,
                osc_config(),
                filter_config(),
                sample_rate,
                &mut state,
                &mut scratch,
                &mut output,
            )
            .unwrap();

        assert_eq!(output[0].left(), output[0].right());
    }
}
