use crate::kernel::patch_id::PatchId;
use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
use crate::mixer::global_parameters::GlobalParameters;
use crate::mixer::mix_engine::MixEngine;
use crate::mixer::mix_observation::MixObservation;
use crate::mixer::mixer_state::MixerState;
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::mixer_track_parameters::MixerTrackParameters;
use crate::mixer::patch_output::PatchOutput;
use crate::real_time::{GraphRevision, ParameterSnapshot, PatchAudioBlock, RtPatchParameters};
use core::cell::Cell;
use std::rc::Rc;

const SAMPLE_RATE: f32 = 48_000.0;
const FRAME_COUNT: usize = 2;
const SAMPLE_COUNT: usize = FRAME_COUNT * 2;
const HALF_GAIN_DB: f32 = -6.020_600_3;
const EPSILON: f32 = 1.0e-6;

/// Discriminating control-side measurements made by the production mixer.
///
/// The physical scene supplies generation-correlated window/device evidence;
/// these paired fixed-stem runs make the signal-order predicates falsifiable
/// without relying on oscillator phase or effect-tail timing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct LiveMixerDspEvidence {
    pub(crate) shared_track_sum_exact: bool,
    pub(crate) track_level_controls_shared_sum: bool,
    pub(crate) patch_trim_isolated: bool,
    pub(crate) patch_reroute_isolated: bool,
    pub(crate) mute_wins: bool,
    pub(crate) any_solo_exact: bool,
    pub(crate) post_gate_sends_exact: bool,
    pub(crate) pre_gate_meters_exact: bool,
}

impl LiveMixerDspEvidence {
    pub(crate) fn measure() -> Self {
        Self::try_measure().unwrap_or_default()
    }

    fn try_measure() -> Option<Self> {
        let shared_track = track(3)?;
        let reroute_track = track(4)?;
        let first_stem = [0.2, 0.4, 0.2, 0.4];
        let second_stem = [0.3, 0.1, 0.3, 0.1];
        let shared_outputs = [
            PatchOutput::to_track(shared_track),
            PatchOutput::to_track(shared_track),
        ];

        let shared = run_mix(
            &shared_outputs,
            MixerState::default(),
            &[first_stem, second_stem],
        )?;
        let shared_track_sum_exact = samples_equal(&shared.output, &[0.5; SAMPLE_COUNT])
            && approximately(shared.observation.track(shared_track).rms(), 0.5)
            && MixerTrackId::ALL
                .iter()
                .filter(|track_id| **track_id != shared_track)
                .all(|track_id| shared.observation.track(*track_id).rms() == 0.0);

        let half_level = MixerState::default().with_track(
            shared_track,
            MixerTrackParameters::new(HALF_GAIN_DB, 0.0, false, false, 0.0, 0.0).ok()?,
        );
        let shared_half = run_mix(&shared_outputs, half_level, &[first_stem, second_stem])?;
        let track_level_controls_shared_sum =
            samples_equal(&shared_half.output, &[0.25; SAMPLE_COUNT])
                && approximately(
                    shared_half.observation.track(shared_track).rms(),
                    shared.observation.track(shared_track).rms() * 0.5,
                )
                && MixerTrackId::ALL
                    .iter()
                    .filter(|track_id| **track_id != shared_track)
                    .all(|track_id| shared_half.observation.track(*track_id).rms() == 0.0);

        let trimmed_outputs = [
            PatchOutput::new(shared_track, HALF_GAIN_DB).ok()?,
            PatchOutput::to_track(shared_track),
        ];
        let trimmed = run_mix(
            &trimmed_outputs,
            MixerState::default(),
            &[first_stem, second_stem],
        )?;
        let patch_trim_isolated = samples_equal(&trimmed.output, &[0.4, 0.3, 0.4, 0.3])
            && approximately(
                trimmed.observation.track(shared_track).rms(),
                0.125_f32.sqrt(),
            )
            && MixerTrackId::ALL
                .iter()
                .filter(|track_id| **track_id != shared_track)
                .all(|track_id| trimmed.observation.track(*track_id).rms() == 0.0);

        let rerouted_outputs = [
            PatchOutput::to_track(shared_track),
            PatchOutput::to_track(reroute_track),
        ];
        let rerouted = run_mix(
            &rerouted_outputs,
            MixerState::default(),
            &[first_stem, second_stem],
        )?;
        let patch_reroute_isolated = samples_equal(&rerouted.output, &shared.output)
            && approximately(
                rerouted.observation.track(shared_track).rms(),
                0.1_f32.sqrt(),
            )
            && approximately(
                rerouted.observation.track(reroute_track).rms(),
                0.05_f32.sqrt(),
            )
            && MixerTrackId::ALL
                .iter()
                .filter(|track_id| **track_id != shared_track && **track_id != reroute_track)
                .all(|track_id| rerouted.observation.track(*track_id).rms() == 0.0);

        let send_stem = [0.4, 0.2, 0.4, 0.2];
        let send_parameters =
            MixerTrackParameters::new(HALF_GAIN_DB, 0.0, false, false, 0.25, 0.5).ok()?;
        let sends = run_mix(
            &[PatchOutput::to_track(shared_track)],
            MixerState::default().with_track(shared_track, send_parameters),
            &[send_stem],
        )?;
        let sounding_meter = sends.observation.track(shared_track);
        let post_gate_sends_exact = samples_equal(&sends.output, &[0.2, 0.1, 0.2, 0.1])
            && samples_equal(&sends.reverb_input, &[0.05, 0.025, 0.05, 0.025])
            && samples_equal(&sends.delay_input, &[0.1, 0.05, 0.1, 0.05]);
        let sounding_meter_exact = approximately(sounding_meter.left_peak(), 0.2)
            && approximately(sounding_meter.right_peak(), 0.1)
            && approximately(sounding_meter.rms(), 0.025_f32.sqrt());

        let muted_parameters =
            MixerTrackParameters::new(HALF_GAIN_DB, 0.0, true, false, 0.25, 0.5).ok()?;
        let muted = run_mix(
            &[PatchOutput::to_track(shared_track)],
            MixerState::default().with_track(shared_track, muted_parameters),
            &[send_stem],
        )?;
        let mute_wins = samples_equal(&muted.output, &[0.0; SAMPLE_COUNT])
            && samples_equal(&muted.reverb_input, &[0.0; SAMPLE_COUNT])
            && samples_equal(&muted.delay_input, &[0.0; SAMPLE_COUNT]);
        let pre_gate_meters_exact = sounding_meter_exact
            && meters_equal(muted.observation, sends.observation, shared_track);

        let solo_parameters = MixerTrackParameters::new(0.0, 0.0, false, true, 0.0, 0.0).ok()?;
        let soloed = run_mix(
            &rerouted_outputs,
            MixerState::default().with_track(shared_track, solo_parameters),
            &[first_stem, second_stem],
        )?;
        let muted_solo_parameters =
            MixerTrackParameters::new(0.0, 0.0, true, true, 0.5, 0.5).ok()?;
        let muted_solo = run_mix(
            &rerouted_outputs,
            MixerState::default().with_track(shared_track, muted_solo_parameters),
            &[first_stem, second_stem],
        )?;
        let any_solo_exact = samples_equal(&soloed.output, &first_stem)
            && samples_equal(&muted_solo.output, &[0.0; SAMPLE_COUNT])
            && muted_solo.observation.track(shared_track).rms() > 0.0
            && muted_solo.observation.track(reroute_track).rms() > 0.0;

        Some(Self {
            shared_track_sum_exact,
            track_level_controls_shared_sum,
            patch_trim_isolated,
            patch_reroute_isolated,
            mute_wins,
            any_solo_exact,
            post_gate_sends_exact,
            pre_gate_meters_exact,
        })
    }
}

#[derive(Default)]
struct EffectProbe {
    reverb_input: Cell<[f32; SAMPLE_COUNT]>,
    delay_input: Cell<[f32; SAMPLE_COUNT]>,
}

struct ProbeEffects {
    probe: Rc<EffectProbe>,
}

impl GlobalEffectsProcessor for ProbeEffects {
    fn prepare(
        &mut self,
        _sample_rate: f32,
        _max_frames: usize,
        _max_delay_milliseconds: f32,
    ) -> Result<(), EffectError> {
        Ok(())
    }

    fn process(
        &mut self,
        reverb_input: &[f32],
        delay_input: &[f32],
        _output: &mut [f32],
        _parameters: &GlobalParameters,
    ) {
        let mut reverb = [0.0; SAMPLE_COUNT];
        let mut delay = [0.0; SAMPLE_COUNT];
        reverb.copy_from_slice(reverb_input);
        delay.copy_from_slice(delay_input);
        self.probe.reverb_input.set(reverb);
        self.probe.delay_input.set(delay);
    }
}

struct MixRun {
    output: [f32; SAMPLE_COUNT],
    observation: MixObservation,
    reverb_input: [f32; SAMPLE_COUNT],
    delay_input: [f32; SAMPLE_COUNT],
}

fn run_mix(
    outputs: &[PatchOutput],
    mixer_state: MixerState,
    stems: &[[f32; SAMPLE_COUNT]],
) -> Option<MixRun> {
    if outputs.len() != stems.len() {
        return None;
    }
    let patches = outputs
        .iter()
        .copied()
        .enumerate()
        .map(|(index, output)| {
            PatchId::new(index as u32 + 1)
                .ok()
                .map(|patch_id| RtPatchParameters::new(patch_id, output))
        })
        .collect::<Option<Vec<_>>>()?;
    let parameters =
        ParameterSnapshot::for_graph(1, GraphRevision::INITIAL, globals()?, mixer_state, &patches)
            .ok()?;
    let mut patch_audio = PatchAudioBlock::prepare(FRAME_COUNT).ok()?;
    patch_audio.begin_render(&parameters, FRAME_COUNT).ok()?;
    for (index, (patch, stem)) in parameters.patches().iter().zip(stems).enumerate() {
        patch_audio
            .stem_mut(index, patch.patch_id()?)?
            .copy_from_slice(stem);
    }

    let probe = Rc::new(EffectProbe::default());
    let mut mixer = MixEngine::new(ProbeEffects {
        probe: Rc::clone(&probe),
    });
    mixer.prepare(SAMPLE_RATE, FRAME_COUNT).ok()?;
    let mut output = [0.0; SAMPLE_COUNT];
    let observation = mixer.mix(&patch_audio, &parameters, &mut output);
    Some(MixRun {
        output,
        observation,
        reverb_input: probe.reverb_input.get(),
        delay_input: probe.delay_input.get(),
    })
}

fn globals() -> Option<GlobalParameters> {
    GlobalParameters::new(0.0, 0.5, 0.4, 0.0, 250.0, 0.3, 0.0).ok()
}

fn track(index: u8) -> Option<MixerTrackId> {
    MixerTrackId::new(index).ok()
}

fn approximately(actual: f32, expected: f32) -> bool {
    actual.is_finite() && expected.is_finite() && (actual - expected).abs() <= EPSILON
}

fn samples_equal(actual: &[f32], expected: &[f32]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| approximately(*actual, *expected))
}

fn meters_equal(actual: MixObservation, expected: MixObservation, track_id: MixerTrackId) -> bool {
    let actual = actual.track(track_id);
    let expected = expected.track(track_id);
    approximately(actual.left_peak(), expected.left_peak())
        && approximately(actual.right_peak(), expected.right_peak())
        && approximately(actual.rms(), expected.rms())
}
