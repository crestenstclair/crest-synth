use crate::adapter::hidef_soundfont_capability::HiDefSoundFontCapability;
use crate::adapter::lock_free_audio_boundary::{
    LockFreeAudioBoundary, LockFreeAudioHandle, LockFreeControlHandle,
};
use crate::control::app_event::{AppEvent, Direction};
use crate::control::app_loop::AppLoop;
use crate::control::app_state::AppState;
use crate::control::event_record::{EventInput, EventSource};
use crate::control::state_projector::StateProjector;
use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::patch_id::PatchId;
use crate::mixer::channel_parameters::ChannelParameters;
use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
use crate::mixer::global_parameters::GlobalParameters;
use crate::mixer::mix_engine::MixEngine;
use crate::real_time::audio_boundary::{AudioBoundary, AudioThreadBoundary};
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::audio_renderer::AudioRenderer;
use crate::real_time::graph_revision::GraphRevision;
use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
use crate::real_time::patch_audio_block::PatchAudioBlock;
use crate::real_time::prepared_graph_builder::PreparedGraphBuilder;
use crate::real_time::structural_graph_boundary::NoStructuralGraphChanges;
use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
use crate::shell::window_input::{WindowInput, WindowKey};
use crate::synth::patch::Patch;
use crate::synth::sound_font_instrument::SoundFontInstrument;
use crate::synth::{
    CapabilityId, InstrumentPreparationError, InstrumentPreparer, PreparedInstrument,
    PreparedInstrumentError,
};
use crate::testing::automatic_midi_test::create_soundfont_config;
use serde::Serialize;
use serde_json::Value;
use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

const FRAME_COUNT: usize = 16;
const SAMPLE_RATE: f32 = 48_000.0;
const ENERGY_EPSILON: f64 = 1.0e-12;

/// One focused healthy-or-mutant verification case.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BehavioralMutationCase {
    DroppedAdjustment,
    CrossPatchParameterLeak,
    PatchMisroute,
    OmittedStateTreeLeaf,
    DryToWetBypass,
    ZeroRenderer,
}

impl BehavioralMutationCase {
    pub const ALL: [Self; 6] = [
        Self::DroppedAdjustment,
        Self::CrossPatchParameterLeak,
        Self::PatchMisroute,
        Self::OmittedStateTreeLeaf,
        Self::DryToWetBypass,
        Self::ZeroRenderer,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DroppedAdjustment => "dropped-adjustment",
            Self::CrossPatchParameterLeak => "cross-patch-parameter-leak",
            Self::PatchMisroute => "patch-misroute",
            Self::OmittedStateTreeLeaf => "omitted-state-tree-leaf",
            Self::DryToWetBypass => "dry-to-wet-bypass",
            Self::ZeroRenderer => "zero-renderer",
        }
    }

    pub fn from_cli(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == value)
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::from_cli(value)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DroppedAdjustmentObservation {
    pub case: String,
    pub adjustment_dispatched: bool,
    pub adjust_event_recorded: bool,
    pub selected_value_exact: bool,
    pub unrelated_values_unchanged: bool,
    pub projection_values_exact: bool,
    pub baseline_restored: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CrossPatchParameterLeakObservation {
    pub case: String,
    pub edited_patch_id: u32,
    pub comparison_patch_id: u32,
    pub patch_ids_distinct: bool,
    pub parameter: String,
    pub parameter_cases_exercised: usize,
    pub edited_value_before: f64,
    pub edited_value_after: f64,
    pub comparison_value_before: f64,
    pub comparison_value_after: f64,
    pub published_edited_value: f64,
    pub published_comparison_value: f64,
    pub edited_stem_energy_before: f64,
    pub edited_stem_energy_after: f64,
    pub comparison_stem_energy_before: f64,
    pub comparison_stem_energy_after: f64,
    pub edited_value_changed: bool,
    pub comparison_value_unchanged: bool,
    pub state_values_exact: bool,
    pub published_values_exact: bool,
    pub edited_patch_audio_changed: bool,
    pub unedited_patch_audio_unchanged: bool,
    pub all_channel_parameters_isolated: bool,
    pub dry_path_isolated: bool,
    pub reverb_path_isolated: bool,
    pub delay_path_isolated: bool,
    pub baseline_restored: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PatchMisrouteObservation {
    pub case: String,
    pub command_patch_matches_event: bool,
    pub target_patch_received_command: bool,
    pub target_stem_changed: bool,
    pub untargeted_stems_unchanged: bool,
    pub patch_routing_exact: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OmittedStateTreeLeafObservation {
    pub case: String,
    pub schema_surface_equal: bool,
    pub required_leaf_count: usize,
    pub missing_leaf_count: usize,
    pub unexpected_leaf_count: usize,
    pub state_values_exact: bool,
    pub projection_values_exact: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DryToWetBypassObservation {
    pub case: String,
    pub dry_input_energy: f64,
    pub zero_send_reverb_input_energy: f64,
    pub zero_send_delay_input_energy: f64,
    pub zero_send_wet_output_energy: f64,
    pub nonzero_send_reverb_input_energy: f64,
    pub nonzero_send_delay_input_energy: f64,
    pub nonzero_send_wet_output_energy: f64,
    pub identical_effect_state: bool,
    pub dry_bypass_absent: bool,
    pub finite_audio: bool,
    pub baseline_restored: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ZeroRendererObservation {
    pub case: String,
    pub control_trace_complete: bool,
    pub renderer_called: bool,
    pub renderer_nonzero: bool,
    pub render_peak: f64,
    pub finite_audio: bool,
}

/// The exact schema selected by one mutation case.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BehavioralMutationObservation {
    DroppedAdjustment(DroppedAdjustmentObservation),
    CrossPatchParameterLeak(CrossPatchParameterLeakObservation),
    PatchMisroute(PatchMisrouteObservation),
    OmittedStateTreeLeaf(OmittedStateTreeLeafObservation),
    DryToWetBypass(DryToWetBypassObservation),
    ZeroRenderer(ZeroRendererObservation),
}

impl BehavioralMutationObservation {
    pub fn case(&self) -> BehavioralMutationCase {
        match self {
            Self::DroppedAdjustment(_) => BehavioralMutationCase::DroppedAdjustment,
            Self::CrossPatchParameterLeak(_) => BehavioralMutationCase::CrossPatchParameterLeak,
            Self::PatchMisroute(_) => BehavioralMutationCase::PatchMisroute,
            Self::OmittedStateTreeLeaf(_) => BehavioralMutationCase::OmittedStateTreeLeaf,
            Self::DryToWetBypass(_) => BehavioralMutationCase::DryToWetBypass,
            Self::ZeroRenderer(_) => BehavioralMutationCase::ZeroRenderer,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Evaluates the same positive predicates declared by the behavioral witnesses.
    pub fn satisfies_witness(&self) -> bool {
        match self {
            Self::DroppedAdjustment(value) => {
                value.adjustment_dispatched
                    && value.adjust_event_recorded
                    && value.selected_value_exact
                    && value.unrelated_values_unchanged
                    && value.projection_values_exact
                    && value.baseline_restored
            }
            Self::CrossPatchParameterLeak(value) => {
                value.edited_patch_id > 0
                    && value.comparison_patch_id > 0
                    && value.patch_ids_distinct
                    && value.parameter_cases_exercised == 4
                    && value.edited_stem_energy_before > 0.0
                    && value.comparison_stem_energy_before > 0.0
                    && value.edited_value_changed
                    && value.comparison_value_unchanged
                    && value.state_values_exact
                    && value.published_values_exact
                    && value.edited_patch_audio_changed
                    && value.unedited_patch_audio_unchanged
                    && value.all_channel_parameters_isolated
                    && value.dry_path_isolated
                    && value.reverb_path_isolated
                    && value.delay_path_isolated
                    && value.baseline_restored
            }
            Self::PatchMisroute(value) => {
                value.command_patch_matches_event
                    && value.target_patch_received_command
                    && value.target_stem_changed
                    && value.untargeted_stems_unchanged
                    && value.patch_routing_exact
            }
            Self::OmittedStateTreeLeaf(value) => {
                value.schema_surface_equal
                    && value.required_leaf_count > 0
                    && value.missing_leaf_count == 0
                    && value.unexpected_leaf_count == 0
                    && value.state_values_exact
                    && value.projection_values_exact
            }
            Self::DryToWetBypass(value) => {
                value.dry_input_energy > 0.0
                    && approximately_zero(value.zero_send_reverb_input_energy)
                    && approximately_zero(value.zero_send_delay_input_energy)
                    && approximately_zero(value.zero_send_wet_output_energy)
                    && value.nonzero_send_reverb_input_energy > 0.0
                    && value.nonzero_send_delay_input_energy > 0.0
                    && value.nonzero_send_wet_output_energy > 0.0
                    && value.identical_effect_state
                    && value.dry_bypass_absent
                    && value.finite_audio
                    && value.baseline_restored
            }
            Self::ZeroRenderer(value) => {
                value.control_trace_complete
                    && value.renderer_called
                    && value.renderer_nonzero
                    && value.render_peak > 0.001
                    && value.finite_audio
            }
        }
    }

    pub fn falsifies_witness(&self) -> bool {
        !self.satisfies_witness()
    }
}

/// One completed harness execution.
#[derive(Clone, Debug, PartialEq)]
pub struct BehavioralMutationRun {
    exit_code: i32,
    observation: BehavioralMutationObservation,
}

impl BehavioralMutationRun {
    pub const fn exit_code(&self) -> i32 {
        self.exit_code
    }

    pub const fn observation(&self) -> &BehavioralMutationObservation {
        &self.observation
    }

    pub fn into_observation(self) -> BehavioralMutationObservation {
        self.observation
    }

    pub fn observation_json(&self) -> Result<String, serde_json::Error> {
        self.observation.to_json()
    }
}

/// Fast verification-only runner for six isolated production seams.
#[derive(Clone, Copy, Debug, Default)]
pub struct BehavioralMutationHarness;

impl BehavioralMutationHarness {
    pub const fn new() -> Self {
        Self
    }

    pub fn run(&self, case: BehavioralMutationCase, mutant_enabled: bool) -> BehavioralMutationRun {
        let observation = match case {
            BehavioralMutationCase::DroppedAdjustment => {
                BehavioralMutationObservation::DroppedAdjustment(run_dropped_adjustment(
                    mutant_enabled,
                ))
            }
            BehavioralMutationCase::CrossPatchParameterLeak => {
                BehavioralMutationObservation::CrossPatchParameterLeak(
                    run_cross_patch_parameter_leak(mutant_enabled),
                )
            }
            BehavioralMutationCase::PatchMisroute => {
                BehavioralMutationObservation::PatchMisroute(run_patch_misroute(mutant_enabled))
            }
            BehavioralMutationCase::OmittedStateTreeLeaf => {
                BehavioralMutationObservation::OmittedStateTreeLeaf(run_omitted_state_tree_leaf(
                    mutant_enabled,
                ))
            }
            BehavioralMutationCase::DryToWetBypass => {
                BehavioralMutationObservation::DryToWetBypass(run_dry_to_wet_bypass(mutant_enabled))
            }
            BehavioralMutationCase::ZeroRenderer => {
                BehavioralMutationObservation::ZeroRenderer(run_zero_renderer(mutant_enabled))
            }
        };

        let exit_code = if observation.satisfies_witness() {
            0
        } else {
            1
        };
        BehavioralMutationRun {
            exit_code,
            observation,
        }
    }
}

type FixtureLoop = AppLoop<LockFreeControlHandle>;

fn fixture_globals() -> GlobalParameters {
    GlobalParameters::new(0.0, 0.6, 0.4, 0.5, 250.0, 0.35, 0.5)
        .expect("fixture global parameters are valid")
}

fn fixture_patch(
    provider: &HiDefSoundFontCapability,
    id: u32,
    parameters: ChannelParameters,
) -> Patch {
    Patch::new(
        PatchId::new(id).expect("fixture PatchId is nonzero"),
        format!("Fixture {id}"),
        create_soundfont_config(
            provider,
            SoundFontInstrument::new(0, (id * 8) as u8, false)
                .expect("fixture SoundFont instrument is valid"),
        )
        .expect("fixture config matches the production descriptor"),
        MidiChannel::new((id - 1) as u8).expect("fixture MIDI channel is valid"),
        parameters,
    )
}

fn fixture_state() -> AppState {
    state_with_parameters(
        ChannelParameters::new(-12.0, -0.35, 0.2, 0.1)
            .expect("fixture channel parameters are valid"),
        ChannelParameters::new(-6.0, 0.35, 0.4, 0.3).expect("fixture channel parameters are valid"),
    )
}

fn route_state() -> AppState {
    state_with_parameters(
        ChannelParameters::new(0.0, -1.0, 0.0, 0.0).expect("route channel parameters are valid"),
        ChannelParameters::new(0.0, 1.0, 0.0, 0.0).expect("route channel parameters are valid"),
    )
}

fn state_with_parameters(first: ChannelParameters, second: ChannelParameters) -> AppState {
    let asset = crate::adapter::hidef_soundfont_asset::HiDefSoundFontAsset::load()
        .expect("fixture SoundFont asset is valid");
    let provider =
        HiDefSoundFontCapability::new(asset.catalog()).expect("fixture capability is valid");
    let mut state = AppState::new(
        provider.registry().expect("fixture registry is valid"),
        fixture_globals(),
    );
    state
        .apply(AppEvent::InstallPatches(vec![
            fixture_patch(&provider, 1, first),
            fixture_patch(&provider, 2, second),
        ]))
        .expect("fixture installation is accepted");
    state
}

fn installed_loop(state: AppState) -> (FixtureLoop, LockFreeAudioHandle) {
    let projector = StateProjector::new();
    let initial = projector
        .parameter_snapshot(&state)
        .expect("fixture parameters project");
    let boundary = LockFreeAudioBoundary::new(16, initial);
    let (control, audio) = boundary.into_handles();
    let app_loop =
        AppLoop::new(state, projector, control).expect("fixture state has coherent projections");
    (app_loop, audio)
}

fn tree_value(app_loop: &FixtureLoop) -> Value {
    serde_json::from_str(app_loop.current_state_tree().json())
        .expect("production StateTree is valid JSON")
}

fn value_at(tree: &Value, pointer: &str) -> f64 {
    tree.pointer(pointer)
        .and_then(Value::as_f64)
        .expect("fixture numeric leaf exists")
}

fn stable_control_values_equal(left: &Value, right: &Value) -> bool {
    ["/patches", "/global", "/selection"]
        .into_iter()
        .all(|pointer| left.pointer(pointer) == right.pointer(pointer))
}

fn tree_parameter_projection_exact(tree: &Value) -> bool {
    let Some(patches) = tree.pointer("/patches").and_then(Value::as_array) else {
        return false;
    };
    let Some(projected) = tree
        .pointer("/parameters/patches")
        .and_then(Value::as_array)
    else {
        return false;
    };
    if patches.len() != projected.len() {
        return false;
    }

    let patch_values_match = patches
        .iter()
        .zip(projected)
        .all(|(state_patch, parameter_patch)| {
            state_patch.pointer("/id") == parameter_patch.pointer("/patchId")
                && state_patch.pointer("/parameters") == parameter_patch.pointer("/parameters")
        });

    patch_values_match
        && tree.pointer("/global") == tree.pointer("/parameters/global")
        && tree.pointer("/generation") == tree.pointer("/parameters/generation")
}

fn app_projection_exact(app_loop: &FixtureLoop) -> bool {
    let tree = app_loop.current_state_tree();
    let text = app_loop.current_text();
    let value: Value =
        serde_json::from_str(tree.json()).expect("production StateTree is valid JSON");

    tree.state_hash() == text.state_hash()
        && value.pointer("/projection/body").and_then(Value::as_str) == Some(text.body())
        && value
            .pointer("/projection/selectedLine")
            .and_then(Value::as_u64)
            == Some(text.selected_line() as u64)
        && tree_parameter_projection_exact(&value)
}

fn published_matches_tree(snapshot: &ParameterSnapshot, tree: &Value) -> bool {
    if tree
        .pointer("/parameters/graphRevision")
        .and_then(Value::as_u64)
        != Some(snapshot.graph_revision().value())
        || snapshot.patch_count()
            != tree
                .pointer("/parameters/patchCount")
                .and_then(Value::as_u64)
                .unwrap_or(u64::MAX) as usize
    {
        return false;
    }

    let patches_match = snapshot.patches().iter().enumerate().all(|(index, patch)| {
        let prefix = format!("/parameters/patches/{index}");
        let id_matches = patch.patch_id().map(PatchId::value)
            == tree
                .pointer(&format!("{prefix}/patchId"))
                .and_then(Value::as_u64)
                .map(|value| value as u32);
        let parameters = patch.parameters();
        id_matches
            && numeric_leaf_matches(
                tree,
                &format!("{prefix}/parameters/gainDb"),
                parameters.gain_db(),
            )
            && numeric_leaf_matches(tree, &format!("{prefix}/parameters/pan"), parameters.pan())
            && numeric_leaf_matches(
                tree,
                &format!("{prefix}/parameters/reverbSend"),
                parameters.reverb_send(),
            )
            && numeric_leaf_matches(
                tree,
                &format!("{prefix}/parameters/delaySend"),
                parameters.delay_send(),
            )
    });

    patches_match
        && numeric_leaf_matches(
            tree,
            "/parameters/global/masterGainDb",
            snapshot.global().master_gain_db(),
        )
        && numeric_leaf_matches(
            tree,
            "/parameters/global/reverbRoomSize",
            snapshot.global().reverb_room_size(),
        )
        && numeric_leaf_matches(
            tree,
            "/parameters/global/reverbDamping",
            snapshot.global().reverb_damping(),
        )
        && numeric_leaf_matches(
            tree,
            "/parameters/global/reverbReturn",
            snapshot.global().reverb_return(),
        )
        && numeric_leaf_matches(
            tree,
            "/parameters/global/delayMilliseconds",
            snapshot.global().delay_milliseconds(),
        )
        && numeric_leaf_matches(
            tree,
            "/parameters/global/delayFeedback",
            snapshot.global().delay_feedback(),
        )
        && numeric_leaf_matches(
            tree,
            "/parameters/global/delayReturn",
            snapshot.global().delay_return(),
        )
}

fn numeric_leaf_matches(tree: &Value, pointer: &str, expected: f32) -> bool {
    tree.pointer(pointer)
        .and_then(Value::as_f64)
        .is_some_and(|actual| actual as f32 == expected)
}

fn run_dropped_adjustment(mutant_enabled: bool) -> DroppedAdjustmentObservation {
    let (mut app_loop, _audio) = installed_loop(fixture_state());
    let baseline = tree_value(&app_loop);
    let before_gain = value_at(&baseline, "/patches/0/parameters/gainDb");
    let expected_gain = before_gain + 1.0;
    let mut translator = KeyboardInputTranslator::new();

    assert_eq!(
        translator.translate(WindowInput::key_down(WindowKey::K)),
        None
    );
    let translated = translator
        .translate(WindowInput::key_down(WindowKey::D))
        .expect("K+D translates to an adjustment");

    let adjustment_dispatched = if mutant_enabled {
        false
    } else {
        app_loop
            .dispatch_from(translated, EventSource::Keyboard)
            .expect("fixture adjustment is accepted");
        true
    };

    let after_forward = tree_value(&app_loop);
    let adjust_event_recorded = app_loop
        .event_log()
        .records()
        .iter()
        .any(|record| matches!(record.input(), EventInput::Adjust { .. }));
    let selected_value_exact = approximately_equal(
        value_at(&after_forward, "/patches/0/parameters/gainDb"),
        expected_gain,
    );
    let unrelated_values_unchanged = [
        "/patches/0/parameters/pan",
        "/patches/0/parameters/reverbSend",
        "/patches/0/parameters/delaySend",
        "/patches/1",
        "/global",
    ]
    .into_iter()
    .all(|pointer| after_forward.pointer(pointer) == baseline.pointer(pointer));
    let projection_values_exact = app_projection_exact(&app_loop)
        && app_loop
            .current_text()
            .body()
            .contains(&format!("> gainDb={expected_gain}"));

    let reverse = translator
        .translate(WindowInput::key_down(WindowKey::A))
        .expect("K+A translates to an inverse adjustment");
    app_loop
        .dispatch_from(reverse, EventSource::Keyboard)
        .expect("fixture inverse adjustment is accepted");
    let restored = tree_value(&app_loop);

    DroppedAdjustmentObservation {
        case: BehavioralMutationCase::DroppedAdjustment
            .as_str()
            .to_owned(),
        adjustment_dispatched,
        adjust_event_recorded,
        selected_value_exact,
        unrelated_values_unchanged,
        projection_values_exact,
        baseline_restored: stable_control_values_equal(&baseline, &restored),
    }
}

#[derive(Clone, Copy)]
enum ChannelField {
    GainDb,
    Pan,
    ReverbSend,
    DelaySend,
}

impl ChannelField {
    const ALL: [Self; 4] = [Self::GainDb, Self::Pan, Self::ReverbSend, Self::DelaySend];

    const fn name(self) -> &'static str {
        match self {
            Self::GainDb => "gainDb",
            Self::Pan => "pan",
            Self::ReverbSend => "reverbSend",
            Self::DelaySend => "delaySend",
        }
    }

    const fn selection_index(self) -> usize {
        match self {
            Self::GainDb => 0,
            Self::Pan => 1,
            Self::ReverbSend => 2,
            Self::DelaySend => 3,
        }
    }

    fn value(self, parameters: &ChannelParameters) -> f64 {
        f64::from(match self {
            Self::GainDb => parameters.gain_db(),
            Self::Pan => parameters.pan(),
            Self::ReverbSend => parameters.reverb_send(),
            Self::DelaySend => parameters.delay_send(),
        })
    }
}

#[derive(Default)]
struct MixProbe {
    dry_energy: Cell<f64>,
    reverb_energy: Cell<f64>,
    delay_energy: Cell<f64>,
}

struct MeteringEffects {
    probe: Rc<MixProbe>,
}

impl GlobalEffectsProcessor for MeteringEffects {
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
        output: &mut [f32],
        parameters: &GlobalParameters,
    ) {
        self.probe.dry_energy.set(energy(output));
        self.probe.reverb_energy.set(energy(reverb_input));
        self.probe.delay_energy.set(energy(delay_input));

        for ((sample, reverb), delay) in output
            .iter_mut()
            .zip(reverb_input.iter())
            .zip(delay_input.iter())
        {
            *sample += reverb * parameters.reverb_return() + delay * parameters.delay_return();
        }
    }
}

#[derive(Clone, Copy, Default)]
struct MixMeasurement {
    output: f64,
    dry: f64,
    reverb: f64,
    delay: f64,
}

fn measure_patch(snapshot: &ParameterSnapshot, patch_index: usize) -> MixMeasurement {
    let mut block =
        PatchAudioBlock::prepare(FRAME_COUNT).expect("verification PatchAudioBlock prepares");
    block
        .begin_render(snapshot, FRAME_COUNT)
        .expect("verification snapshot fits PatchAudioBlock");

    for (index, patch) in snapshot.patches().iter().enumerate() {
        let stem = block
            .stem_mut(
                index,
                patch
                    .patch_id()
                    .expect("active parameters carry Patch identity"),
            )
            .expect("active Patch stem exists");
        let amplitude = if index == patch_index {
            0.2 + index as f32 * 0.15
        } else {
            0.0
        };
        stem.fill(amplitude);
    }

    let probe = Rc::new(MixProbe::default());
    let effects = MeteringEffects {
        probe: Rc::clone(&probe),
    };
    let mut mixer = MixEngine::new(effects);
    mixer
        .prepare(SAMPLE_RATE, FRAME_COUNT)
        .expect("verification mixer prepares");
    let mut output = [0.0_f32; FRAME_COUNT * 2];
    mixer.mix(&block, snapshot, &mut output);

    MixMeasurement {
        output: energy(&output),
        dry: probe.dry_energy.get(),
        reverb: probe.reverb_energy.get(),
        delay: probe.delay_energy.get(),
    }
}

fn relevant_energy(field: ChannelField, measurement: MixMeasurement) -> f64 {
    match field {
        ChannelField::GainDb | ChannelField::Pan => measurement.dry,
        ChannelField::ReverbSend => measurement.reverb,
        ChannelField::DelaySend => measurement.delay,
    }
}

fn snapshot_with_cross_patch_leak(
    published: ParameterSnapshot,
    mutant_enabled: bool,
) -> ParameterSnapshot {
    if !mutant_enabled {
        return published;
    }

    let mut patches: Vec<RtPatchParameters> = published.patches().to_vec();
    let edited_parameters = *patches[1].parameters();
    let comparison_id = patches[0]
        .patch_id()
        .expect("comparison parameters are active");
    patches[0] = RtPatchParameters::new(comparison_id, edited_parameters);
    ParameterSnapshot::new(published.generation(), *published.global(), &patches)
        .expect("mutated ownership seam preserves bounded identities")
}

fn run_cross_patch_parameter_leak(mutant_enabled: bool) -> CrossPatchParameterLeakObservation {
    let mut representative = None;
    let mut parameter_cases_exercised = 0;
    let mut state_values_exact = true;
    let mut published_values_exact = true;
    let mut edited_patch_audio_changed = true;
    let mut unedited_patch_audio_unchanged = true;
    let mut dry_path_isolated = true;
    let mut reverb_path_isolated = true;
    let mut delay_path_isolated = true;
    let mut baseline_restored = true;

    for field in ChannelField::ALL {
        let (mut app_loop, mut audio) = installed_loop(fixture_state());
        app_loop
            .dispatch(AppEvent::Navigate(Direction::Right))
            .expect("fixture selects the edited Patch");
        for _ in 0..field.selection_index() {
            app_loop
                .dispatch(AppEvent::Navigate(Direction::Down))
                .expect("fixture selects the requested parameter");
        }

        let baseline = tree_value(&app_loop);
        let before_snapshot = audio.read_latest_parameters();
        let before_edited = field.value(before_snapshot.patches()[1].parameters());
        let before_comparison = field.value(before_snapshot.patches()[0].parameters());
        let edited_before_mix = measure_patch(&before_snapshot, 1);
        let comparison_before_mix = measure_patch(&before_snapshot, 0);

        app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .expect("fixture edit is accepted");
        let after_tree = tree_value(&app_loop);
        let published = audio.read_latest_parameters();
        let after_edited = field.value(published.patches()[1].parameters());
        let after_comparison = field.value(published.patches()[0].parameters());
        let mix_snapshot = snapshot_with_cross_patch_leak(published, mutant_enabled);
        let edited_after_mix = measure_patch(&mix_snapshot, 1);
        let comparison_after_mix = measure_patch(&mix_snapshot, 0);

        let edited_changed = !approximately_equal(before_edited, after_edited);
        let comparison_unchanged = approximately_equal(before_comparison, after_comparison);
        let edited_audio_changed = !approximately_equal(
            relevant_energy(field, edited_before_mix),
            relevant_energy(field, edited_after_mix),
        );
        let comparison_audio_unchanged = approximately_equal(
            relevant_energy(field, comparison_before_mix),
            relevant_energy(field, comparison_after_mix),
        );

        parameter_cases_exercised += 1;
        state_values_exact &= tree_parameter_projection_exact(&after_tree);
        published_values_exact &= published_matches_tree(&published, &after_tree);
        edited_patch_audio_changed &= edited_changed && edited_audio_changed;
        unedited_patch_audio_unchanged &= comparison_unchanged && comparison_audio_unchanged;

        match field {
            ChannelField::GainDb | ChannelField::Pan => {
                dry_path_isolated &= edited_audio_changed && comparison_audio_unchanged;
            }
            ChannelField::ReverbSend => {
                reverb_path_isolated &= edited_audio_changed && comparison_audio_unchanged;
            }
            ChannelField::DelaySend => {
                delay_path_isolated &= edited_audio_changed && comparison_audio_unchanged;
            }
        }

        if matches!(field, ChannelField::GainDb) {
            representative = Some((
                before_edited,
                after_edited,
                before_comparison,
                after_comparison,
                field.value(published.patches()[1].parameters()),
                field.value(published.patches()[0].parameters()),
                edited_before_mix.output,
                edited_after_mix.output,
                comparison_before_mix.output,
                comparison_after_mix.output,
                edited_changed,
                comparison_unchanged,
            ));
        }

        app_loop
            .dispatch(AppEvent::Adjust(Direction::Left))
            .expect("fixture inverse edit is accepted");
        baseline_restored &= stable_control_values_equal(&baseline, &tree_value(&app_loop));
    }

    let (
        edited_value_before,
        edited_value_after,
        comparison_value_before,
        comparison_value_after,
        published_edited_value,
        published_comparison_value,
        edited_stem_energy_before,
        edited_stem_energy_after,
        comparison_stem_energy_before,
        comparison_stem_energy_after,
        edited_value_changed,
        comparison_value_unchanged,
    ) = representative.expect("gain case is always exercised");

    let all_channel_parameters_isolated = edited_patch_audio_changed
        && unedited_patch_audio_unchanged
        && dry_path_isolated
        && reverb_path_isolated
        && delay_path_isolated;

    CrossPatchParameterLeakObservation {
        case: BehavioralMutationCase::CrossPatchParameterLeak
            .as_str()
            .to_owned(),
        edited_patch_id: 2,
        comparison_patch_id: 1,
        patch_ids_distinct: true,
        parameter: ChannelField::GainDb.name().to_owned(),
        parameter_cases_exercised,
        edited_value_before,
        edited_value_after,
        comparison_value_before,
        comparison_value_after,
        published_edited_value,
        published_comparison_value,
        edited_stem_energy_before,
        edited_stem_energy_after,
        comparison_stem_energy_before,
        comparison_stem_energy_after,
        edited_value_changed,
        comparison_value_unchanged,
        state_values_exact,
        published_values_exact,
        edited_patch_audio_changed,
        unedited_patch_audio_unchanged,
        all_channel_parameters_isolated,
        dry_path_isolated,
        reverb_path_isolated,
        delay_path_isolated,
        baseline_restored,
    }
}

struct MisrouteBoundary<Boundary> {
    inner: Boundary,
    mutant_enabled: bool,
    alternate_patch_id: PatchId,
    rewritten: bool,
}

impl<Boundary> AudioThreadBoundary for MisrouteBoundary<Boundary>
where
    Boundary: AudioThreadBoundary,
{
    fn pop_command(&mut self) -> Option<AudioCommand> {
        let command = self.inner.pop_command()?;
        if !self.mutant_enabled || self.rewritten {
            return Some(command);
        }

        match command {
            AudioCommand::PatchMidi { message, .. } => {
                self.rewritten = true;
                Some(AudioCommand::PatchMidi {
                    patch_id: self.alternate_patch_id,
                    message,
                })
            }
            AudioCommand::AllNotesOff => Some(AudioCommand::AllNotesOff),
        }
    }

    fn read_latest_parameters(&mut self) -> ParameterSnapshot {
        self.inner.read_latest_parameters()
    }
}

#[derive(Default)]
struct EngineProbe {
    dispatched_patch: AtomicU32,
}

struct RoutedVerificationPreparer {
    probe: Arc<EngineProbe>,
    capability_id: CapabilityId,
}

impl InstrumentPreparer for RoutedVerificationPreparer {
    fn capability_id(&self) -> &CapabilityId {
        &self.capability_id
    }

    fn prepare(
        &self,
        patch: &Patch,
        _sample_rate: f32,
        _max_frames: usize,
    ) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
        Ok(Box::new(RoutedVerificationInstrument {
            patch_id: patch.id(),
            probe: Arc::clone(&self.probe),
        }))
    }
}

struct RoutedVerificationInstrument {
    patch_id: PatchId,
    probe: Arc<EngineProbe>,
}

impl PreparedInstrument for RoutedVerificationInstrument {
    fn patch_id(&self) -> PatchId {
        self.patch_id
    }

    fn dispatch(
        &mut self,
        _message: MidiMessage,
        _parameters: &crate::real_time::RtPatchParameters,
    ) -> Result<(), PreparedInstrumentError> {
        self.probe
            .dispatched_patch
            .store(self.patch_id.value(), Ordering::Release);
        Ok(())
    }

    fn render(
        &mut self,
        output: &mut [f32],
        _frame_count: usize,
        _parameters: &crate::real_time::RtPatchParameters,
    ) {
        if self.probe.dispatched_patch.load(Ordering::Acquire) == self.patch_id.value() {
            output.fill(0.5);
        }
    }

    fn all_notes_off(&mut self) {
        self.probe.dispatched_patch.store(0, Ordering::Release);
    }
}

struct RouteRender {
    app_loop: FixtureLoop,
    probe: Arc<EngineProbe>,
    output: [f32; FRAME_COUNT * 2],
}

fn render_routed_command(mutant_enabled: bool) -> RouteRender {
    let target_id = PatchId::new(2).expect("target PatchId is valid");
    let alternate_id = PatchId::new(1).expect("alternate PatchId is valid");
    let (mut app_loop, audio) = installed_loop(route_state());
    let message = MidiMessage::try_new(
        MidiChannel::new(1).expect("fixture MIDI channel is valid"),
        MidiMessageKind::NoteOn,
        60,
        100,
    )
    .expect("fixture MIDI message is valid");
    app_loop
        .dispatch_from(
            AppEvent::Midi {
                patch_id: target_id,
                message,
            },
            EventSource::AutomaticMidi,
        )
        .expect("fixture MIDI event is accepted");

    let boundary = MisrouteBoundary {
        inner: audio,
        mutant_enabled,
        alternate_patch_id: alternate_id,
        rewritten: false,
    };
    let probe = Arc::new(EngineProbe::default());
    let preparers: Vec<Box<dyn InstrumentPreparer>> = vec![Box::new(RoutedVerificationPreparer {
        probe: Arc::clone(&probe),
        capability_id: CapabilityId::new("instrument.soundfont.hidef").unwrap(),
    })];
    let graph = PreparedGraphBuilder::new(app_loop.capabilities(), &preparers)
        .build(
            GraphRevision::INITIAL,
            app_loop.patches(),
            *app_loop.current_parameters(),
            SAMPLE_RATE,
            FRAME_COUNT,
        )
        .expect("verification graph prepares");
    let mut renderer = AudioRenderer::new(boundary, NoStructuralGraphChanges::new(), graph);
    let mut output = [0.0_f32; FRAME_COUNT * 2];
    renderer.render(&mut output);

    RouteRender {
        app_loop,
        probe,
        output,
    }
}

fn run_patch_misroute(mutant_enabled: bool) -> PatchMisrouteObservation {
    let result = render_routed_command(mutant_enabled);
    let target_id = 2;
    let dispatched_id = result.probe.dispatched_patch.load(Ordering::Acquire);
    let (left_energy, right_energy) = stereo_energy(&result.output);
    let command_patch_matches_event = dispatched_id == target_id;
    let target_patch_received_command = dispatched_id == target_id;
    let target_stem_changed = right_energy > ENERGY_EPSILON;
    let untargeted_stems_unchanged = approximately_zero(left_energy);
    let patch_routing_exact = command_patch_matches_event
        && target_patch_received_command
        && target_stem_changed
        && untargeted_stems_unchanged;

    PatchMisrouteObservation {
        case: BehavioralMutationCase::PatchMisroute.as_str().to_owned(),
        command_patch_matches_event,
        target_patch_received_command,
        target_stem_changed,
        untargeted_stems_unchanged,
        patch_routing_exact,
    }
}

fn collect_leaf_paths(value: &Value) -> BTreeSet<String> {
    fn visit(value: &Value, path: &str, leaves: &mut BTreeSet<String>) {
        match value {
            Value::Object(object) => {
                for (name, child) in object {
                    visit(child, &format!("{path}/{name}"), leaves);
                }
            }
            Value::Array(array) => {
                for (index, child) in array.iter().enumerate() {
                    visit(child, &format!("{path}/{index}"), leaves);
                }
            }
            _ => {
                leaves.insert(path.to_owned());
            }
        }
    }

    let mut leaves = BTreeSet::new();
    visit(value, "", &mut leaves);
    leaves
}

fn run_omitted_state_tree_leaf(mutant_enabled: bool) -> OmittedStateTreeLeafObservation {
    let (app_loop, _audio) = installed_loop(fixture_state());
    let healthy_tree = tree_value(&app_loop);
    let required = collect_leaf_paths(&healthy_tree);
    let mut observed_tree = healthy_tree.clone();

    if mutant_enabled {
        observed_tree
            .pointer_mut("/patches/0/parameters")
            .and_then(Value::as_object_mut)
            .expect("typed Patch parameter object exists")
            .remove("gainDb")
            .expect("the one omitted typed leaf exists");
    }

    let observed = collect_leaf_paths(&observed_tree);
    let missing_leaf_count = required.difference(&observed).count();
    let unexpected_leaf_count = observed.difference(&required).count();
    let schema_surface_equal = missing_leaf_count == 0 && unexpected_leaf_count == 0;
    let projection_values_exact =
        observed_tree.pointer("/projection") == healthy_tree.pointer("/projection");

    OmittedStateTreeLeafObservation {
        case: BehavioralMutationCase::OmittedStateTreeLeaf
            .as_str()
            .to_owned(),
        schema_surface_equal,
        required_leaf_count: required.len(),
        missing_leaf_count,
        unexpected_leaf_count,
        state_values_exact: tree_parameter_projection_exact(&observed_tree),
        projection_values_exact,
    }
}

#[derive(Default)]
struct DryWetProbe {
    dry_input_energy: Cell<f64>,
    reverb_input_energy: Cell<f64>,
    delay_input_energy: Cell<f64>,
    wet_output_energy: Cell<f64>,
    initial_state: Cell<u64>,
}

struct DryWetEffects {
    mutant_enabled: bool,
    state: u64,
    scratch: Vec<f32>,
    probe: Rc<DryWetProbe>,
}

impl GlobalEffectsProcessor for DryWetEffects {
    fn prepare(
        &mut self,
        _sample_rate: f32,
        max_frames: usize,
        _max_delay_milliseconds: f32,
    ) -> Result<(), EffectError> {
        self.scratch.resize(max_frames * 2, 0.0);
        Ok(())
    }

    fn process(
        &mut self,
        reverb_input: &[f32],
        delay_input: &[f32],
        output: &mut [f32],
        parameters: &GlobalParameters,
    ) {
        let sample_count = output
            .len()
            .min(reverb_input.len())
            .min(delay_input.len())
            .min(self.scratch.len());
        self.probe.initial_state.set(self.state);
        self.scratch[..sample_count].copy_from_slice(&output[..sample_count]);
        self.probe
            .dry_input_energy
            .set(energy(&self.scratch[..sample_count]));
        let reverb_energy = energy(&reverb_input[..sample_count]);
        let delay_energy = energy(&delay_input[..sample_count]);
        self.probe.reverb_input_energy.set(reverb_energy);
        self.probe.delay_input_energy.set(delay_energy);

        if self.mutant_enabled
            && approximately_zero(reverb_energy)
            && approximately_zero(delay_energy)
        {
            for (sample, dry) in output[..sample_count]
                .iter_mut()
                .zip(self.scratch[..sample_count].iter())
            {
                *sample += dry * 0.25;
            }
        } else {
            for ((sample, reverb), delay) in output[..sample_count]
                .iter_mut()
                .zip(reverb_input[..sample_count].iter())
                .zip(delay_input[..sample_count].iter())
            {
                *sample += reverb * parameters.reverb_return() + delay * parameters.delay_return();
            }
        }

        let wet_output_energy = output[..sample_count]
            .iter()
            .zip(self.scratch[..sample_count].iter())
            .map(|(after, dry)| {
                let delta = f64::from(*after - *dry);
                delta * delta
            })
            .sum();
        self.probe.wet_output_energy.set(wet_output_energy);
        self.state = self.state.saturating_add(1);
    }
}

#[derive(Clone, Copy)]
struct DryWetMeasurement {
    dry: f64,
    reverb: f64,
    delay: f64,
    wet: f64,
    initial_state: u64,
    finite: bool,
}

fn render_dry_wet(parameters: ParameterSnapshot, mutant_enabled: bool) -> DryWetMeasurement {
    let probe = Rc::new(DryWetProbe::default());
    let effects = DryWetEffects {
        mutant_enabled,
        state: 0,
        scratch: Vec::new(),
        probe: Rc::clone(&probe),
    };
    let mut mixer = MixEngine::new(effects);
    mixer
        .prepare(SAMPLE_RATE, FRAME_COUNT)
        .expect("verification mixer prepares");
    let mut patch_audio = PatchAudioBlock::prepare(FRAME_COUNT).unwrap();
    patch_audio.begin_render(&parameters, FRAME_COUNT).unwrap();
    for (index, patch) in parameters.patches().iter().enumerate() {
        let Some(patch_id) = patch.patch_id() else {
            continue;
        };
        if let Some(stem) = patch_audio.stem_mut(index, patch_id) {
            stem.fill(0.2 + index as f32 * 0.1);
        }
    }
    let mut output = [0.0_f32; FRAME_COUNT * 2];
    mixer.mix(&patch_audio, &parameters, &mut output);

    DryWetMeasurement {
        dry: probe.dry_input_energy.get(),
        reverb: probe.reverb_input_energy.get(),
        delay: probe.delay_input_energy.get(),
        wet: probe.wet_output_energy.get(),
        initial_state: probe.initial_state.get(),
        finite: output.iter().all(|sample| sample.is_finite()),
    }
}

fn parameter_snapshot_with_sends(reverb_send: f32, delay_send: f32) -> ParameterSnapshot {
    let channels = [
        ChannelParameters::new(0.0, -0.25, reverb_send, delay_send)
            .expect("verification sends are valid"),
        ChannelParameters::new(-3.0, 0.25, reverb_send, delay_send)
            .expect("verification sends are valid"),
    ];
    let patches = [
        RtPatchParameters::new(
            PatchId::new(1).expect("fixture PatchId is valid"),
            channels[0],
        ),
        RtPatchParameters::new(
            PatchId::new(2).expect("fixture PatchId is valid"),
            channels[1],
        ),
    ];
    ParameterSnapshot::new(1, fixture_globals(), &patches).expect("verification snapshot is valid")
}

fn run_dry_to_wet_bypass(mutant_enabled: bool) -> DryToWetBypassObservation {
    let zero_send = parameter_snapshot_with_sends(0.0, 0.0);
    let nonzero_send = parameter_snapshot_with_sends(0.4, 0.3);
    let zero_measurement = render_dry_wet(zero_send, mutant_enabled);
    let nonzero_measurement = render_dry_wet(nonzero_send, mutant_enabled);
    let finite_audio = zero_measurement.finite && nonzero_measurement.finite;
    let baseline_restored = zero_send == parameter_snapshot_with_sends(0.0, 0.0)
        && nonzero_send == parameter_snapshot_with_sends(0.4, 0.3);

    DryToWetBypassObservation {
        case: BehavioralMutationCase::DryToWetBypass.as_str().to_owned(),
        dry_input_energy: zero_measurement.dry,
        zero_send_reverb_input_energy: zero_measurement.reverb,
        zero_send_delay_input_energy: zero_measurement.delay,
        zero_send_wet_output_energy: zero_measurement.wet,
        nonzero_send_reverb_input_energy: nonzero_measurement.reverb,
        nonzero_send_delay_input_energy: nonzero_measurement.delay,
        nonzero_send_wet_output_energy: nonzero_measurement.wet,
        identical_effect_state: zero_measurement.initial_state == nonzero_measurement.initial_state,
        dry_bypass_absent: approximately_zero(zero_measurement.wet),
        finite_audio,
        baseline_restored,
    }
}

fn run_zero_renderer(mutant_enabled: bool) -> ZeroRendererObservation {
    let mut result = render_routed_command(false);
    let renderer_called = true;
    let target_received = result.probe.dispatched_patch.load(Ordering::Acquire) == 2;
    let control_trace_complete = result.app_loop.event_log().records().iter().any(|record| {
        matches!(
            record.input(),
            EventInput::Midi { patch_id, .. } if *patch_id == 2
        )
    }) && target_received;

    if mutant_enabled {
        result.output.fill(0.0);
    }

    let render_peak = result
        .output
        .iter()
        .map(|sample| f64::from(sample.abs()))
        .fold(0.0_f64, f64::max);
    let finite_audio = result.output.iter().all(|sample| sample.is_finite());

    ZeroRendererObservation {
        case: BehavioralMutationCase::ZeroRenderer.as_str().to_owned(),
        control_trace_complete,
        renderer_called,
        renderer_nonzero: render_peak > 0.0,
        render_peak,
        finite_audio,
    }
}

fn energy(samples: &[f32]) -> f64 {
    samples
        .iter()
        .map(|sample| {
            let sample = f64::from(*sample);
            sample * sample
        })
        .sum()
}

fn stereo_energy(samples: &[f32]) -> (f64, f64) {
    samples
        .chunks_exact(2)
        .fold((0.0, 0.0), |(left, right), frame| {
            (
                left + f64::from(frame[0]) * f64::from(frame[0]),
                right + f64::from(frame[1]) * f64::from(frame[1]),
            )
        })
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= ENERGY_EPSILON
}

fn approximately_zero(value: f64) -> bool {
    value.abs() <= ENERGY_EPSILON
}

#[cfg(test)]
mod tests {
    use super::{BehavioralMutationCase, BehavioralMutationHarness, BehavioralMutationObservation};
    use serde_json::Value;
    use std::collections::BTreeSet;

    fn keys(observation: &BehavioralMutationObservation) -> BTreeSet<String> {
        serde_json::to_value(observation)
            .expect("observation serializes")
            .as_object()
            .expect("observation is a JSON object")
            .keys()
            .cloned()
            .collect()
    }

    fn expected(fields: &[&str]) -> BTreeSet<String> {
        fields.iter().map(|field| (*field).to_owned()).collect()
    }

    #[test]
    fn healthy_cases_satisfy_and_matching_mutants_falsify_the_same_predicates() {
        let harness = BehavioralMutationHarness::new();

        for case in BehavioralMutationCase::ALL {
            let healthy = harness.run(case, false);
            let mutant = harness.run(case, true);
            assert_eq!(healthy.exit_code(), 0, "{case:?}");
            assert!(healthy.observation().satisfies_witness(), "{case:?}");
            assert_eq!(mutant.exit_code(), 1, "{case:?}");
            assert!(mutant.observation().falsifies_witness(), "{case:?}");
            assert_eq!(keys(healthy.observation()), keys(mutant.observation()));
        }
    }

    #[test]
    fn faithful_effects_nonzero_sends_and_baseline_restoration() {
        let observation = BehavioralMutationHarness::new()
            .run(BehavioralMutationCase::DryToWetBypass, false)
            .into_observation();
        let BehavioralMutationObservation::DryToWetBypass(observation) = observation else {
            panic!("the dry-to-wet case retains its typed schema");
        };

        assert!(observation.dry_input_energy > 0.0);
        assert!(super::approximately_zero(
            observation.zero_send_reverb_input_energy
        ));
        assert!(super::approximately_zero(
            observation.zero_send_delay_input_energy
        ));
        assert!(observation.nonzero_send_reverb_input_energy > 0.0);
        assert!(observation.nonzero_send_delay_input_energy > 0.0);
        assert!(observation.nonzero_send_wet_output_energy > 0.0);
        assert!(observation.identical_effect_state);
        assert!(observation.dry_bypass_absent);
        assert!(observation.baseline_restored);
    }

    #[test]
    fn every_case_emits_its_exact_declared_schema() {
        let harness = BehavioralMutationHarness::new();
        let cases = [
            (
                BehavioralMutationCase::DroppedAdjustment,
                expected(&[
                    "case",
                    "adjustment_dispatched",
                    "adjust_event_recorded",
                    "selected_value_exact",
                    "unrelated_values_unchanged",
                    "projection_values_exact",
                    "baseline_restored",
                ]),
            ),
            (
                BehavioralMutationCase::CrossPatchParameterLeak,
                expected(&[
                    "case",
                    "edited_patch_id",
                    "comparison_patch_id",
                    "patch_ids_distinct",
                    "parameter",
                    "parameter_cases_exercised",
                    "edited_value_before",
                    "edited_value_after",
                    "comparison_value_before",
                    "comparison_value_after",
                    "published_edited_value",
                    "published_comparison_value",
                    "edited_stem_energy_before",
                    "edited_stem_energy_after",
                    "comparison_stem_energy_before",
                    "comparison_stem_energy_after",
                    "edited_value_changed",
                    "comparison_value_unchanged",
                    "state_values_exact",
                    "published_values_exact",
                    "edited_patch_audio_changed",
                    "unedited_patch_audio_unchanged",
                    "all_channel_parameters_isolated",
                    "dry_path_isolated",
                    "reverb_path_isolated",
                    "delay_path_isolated",
                    "baseline_restored",
                ]),
            ),
            (
                BehavioralMutationCase::PatchMisroute,
                expected(&[
                    "case",
                    "command_patch_matches_event",
                    "target_patch_received_command",
                    "target_stem_changed",
                    "untargeted_stems_unchanged",
                    "patch_routing_exact",
                ]),
            ),
            (
                BehavioralMutationCase::OmittedStateTreeLeaf,
                expected(&[
                    "case",
                    "schema_surface_equal",
                    "required_leaf_count",
                    "missing_leaf_count",
                    "unexpected_leaf_count",
                    "state_values_exact",
                    "projection_values_exact",
                ]),
            ),
            (
                BehavioralMutationCase::DryToWetBypass,
                expected(&[
                    "case",
                    "dry_input_energy",
                    "zero_send_reverb_input_energy",
                    "zero_send_delay_input_energy",
                    "zero_send_wet_output_energy",
                    "nonzero_send_reverb_input_energy",
                    "nonzero_send_delay_input_energy",
                    "nonzero_send_wet_output_energy",
                    "identical_effect_state",
                    "dry_bypass_absent",
                    "finite_audio",
                    "baseline_restored",
                ]),
            ),
            (
                BehavioralMutationCase::ZeroRenderer,
                expected(&[
                    "case",
                    "control_trace_complete",
                    "renderer_called",
                    "renderer_nonzero",
                    "render_peak",
                    "finite_audio",
                ]),
            ),
        ];

        for (case, schema) in cases {
            let observation = harness.run(case, false);
            assert_eq!(keys(observation.observation()), schema, "{case:?}");
            let json: Value = serde_json::from_str(
                &observation
                    .observation_json()
                    .expect("observation serializes"),
            )
            .expect("observation JSON parses");
            assert_eq!(
                json["case"],
                Value::String(case.as_str().to_owned()),
                "{case:?}"
            );
        }
    }
}
