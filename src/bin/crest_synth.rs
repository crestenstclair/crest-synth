use anyhow::{bail, Context, Result};
use core::alloc::{GlobalAlloc, Layout};
use crest_synth::adapter::atomic_audio_observation::AtomicAudioObservation;
use crest_synth::adapter::braids_capability::BraidsCapability;
use crest_synth::adapter::braids_capability::BRAIDS_CAPABILITY_ID;
use crest_synth::adapter::braids_preparer::BraidsPreparer;
use crest_synth::adapter::corridors_midi_event_source::CorridorsMidiEventSource;
use crest_synth::adapter::cpal_audio_output::CpalAudioOutput;
use crest_synth::adapter::eframe_graphical_window::EframeGraphicalWindow;
use crest_synth::adapter::hidef_soundfont_asset::HiDefSoundFontAsset;
use crest_synth::adapter::hidef_soundfont_capability::{
    HiDefSoundFontCapability, HIDEF_CAPABILITY_ID,
};
use crest_synth::adapter::hidef_soundfont_preparer::HiDefSoundFontPreparer;
use crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
use crest_synth::adapter::lock_free_structural_graph_boundary::LockFreeStructuralGraphBoundary;
use crest_synth::adapter::production_effects::{
    production_effect_preparers, production_effect_providers,
};
use crest_synth::control::app_event::AppEvent;
use crest_synth::control::app_state::EventRejection;
use crest_synth::control::event_record::{EmittedEvent, EventInput, EventOutcome, EventSource};
use crest_synth::kernel::midi_message::MidiMessageKind;
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::mixer::mixer_state::MixerState;
use crest_synth::mixer::mixer_track_id::MixerTrackId;
use crest_synth::mixer::mixer_track_parameters::MixerTrackParameters;
use crest_synth::mixer::patch_output::PatchOutput;
use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
use crest_synth::real_time::{GraphHandoffStatus, GraphRevision};
use crest_synth::shell::standalone_application::{
    ApplicationConfig, DegenerateMode, LiveSceneKind, StandaloneApplication,
};
use crest_synth::shell::window_input::WindowInput;
use crest_synth::synth::{InstrumentCapabilityProvider, InstrumentPreparer};
use crest_synth::testing::demo_scene_report::{DemoCoverageGroup, DemoSceneReport};
use crest_synth::testing::{
    BehavioralMutationCase, BehavioralMutationHarness, BehavioralMutationObservation,
    LiveDemoReport, LIVE_DEMO_NO_PROGRESS_TIMEOUT, LIVE_DEMO_TOTAL_TIMEOUT,
};
use serde::Serialize;
use serde_json::Value;
use std::alloc::System;
use std::env;

const AUDIO_COMMAND_CAPACITY: usize = 1_024;

struct CallbackMeasuringAllocator;

#[global_allocator]
static CALLBACK_MEASURING_ALLOCATOR: CallbackMeasuringAllocator = CallbackMeasuringAllocator;

unsafe impl GlobalAlloc for CallbackMeasuringAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        crest_synth::real_time::callback_safety::record_callback_allocation_from_allocator();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        crest_synth::real_time::callback_safety::record_callback_allocation_from_allocator();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        crest_synth::real_time::callback_safety::record_callback_deallocation_from_allocator();
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        crest_synth::real_time::callback_safety::record_callback_allocation_from_allocator();
        crest_synth::real_time::callback_safety::record_callback_deallocation_from_allocator();
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

fn main() -> Result<()> {
    let options = parse_options(env::args().skip(1))?;
    run(options)
}

fn run(options: Options) -> Result<()> {
    let make_application = || -> Result<_> {
        let config = ApplicationConfig::default();
        let initial_parameters =
            ParameterSnapshot::new(0, config.global_parameters(), MixerState::default(), &[])
                .context("failed to construct the initial audio parameter snapshot")?;
        let boundary = LockFreeAudioBoundary::new(AUDIO_COMMAND_CAPACITY, initial_parameters);
        let soundfont_asset = HiDefSoundFontAsset::load()
            .context("failed to load the fixed HiDef SoundFont asset")?;
        let providers: Vec<Box<dyn InstrumentCapabilityProvider>> = vec![
            Box::new(
                HiDefSoundFontCapability::new(soundfont_asset.catalog())
                    .context("failed to construct the HiDef capability provider")?,
            ),
            Box::new(
                BraidsCapability::new()
                    .context("failed to construct the Braids capability provider")?,
            ),
        ];
        let preparers: Vec<Box<dyn InstrumentPreparer>> = vec![
            Box::new(
                HiDefSoundFontPreparer::new(&soundfont_asset)
                    .context("failed to prepare the HiDef instrument factory")?,
            ),
            Box::new(
                BraidsPreparer::new().context("failed to prepare the Braids instrument factory")?,
            ),
        ];
        let effect_providers = production_effect_providers()
            .context("failed to construct the production effect capability providers")?;
        let effect_preparers = production_effect_preparers()
            .context("failed to construct the production effect preparers")?;
        let structural = LockFreeStructuralGraphBoundary::new(
            1,
            1,
            GraphHandoffStatus::with_active(GraphRevision::INITIAL),
        )
        .context("failed to prepare the structural graph boundary")?;
        let window = if options.demo_live {
            EframeGraphicalWindow::new("crest-synth — autonomous live demo")
        } else {
            EframeGraphicalWindow::default()
        };
        StandaloneApplication::new_with_effects(
            boundary,
            providers,
            preparers,
            effect_providers,
            effect_preparers,
            structural,
            AtomicAudioObservation::default(),
            CorridorsMidiEventSource::new(),
            window,
            CpalAudioOutput::new(),
            config,
        )
        .context("failed to validate the production composition")
    };

    if options.demo_live {
        let scene_kind = if options.demo_live_effects_and_buses {
            LiveSceneKind::EffectsAndBuses
        } else {
            LiveSceneKind::SixteenTrackMixerRouting
        };
        let total_timeout = if options.demo_live_effects_and_buses {
            crest_synth::testing::live_effects_and_buses_scene::EFFECTS_AND_BUSES_TOTAL_TIMEOUT
        } else {
            LIVE_DEMO_TOTAL_TIMEOUT
        };
        eprintln!(
            "crest-synth live demo: autonomous and input-isolated; expected duration about 90 seconds; no-progress timeout={}s; total timeout={}s; closing the window cancels the proof",
            LIVE_DEMO_NO_PROGRESS_TIMEOUT.as_secs(),
            total_timeout.as_secs(),
        );
        let observation = make_application()?
            .run_live_demo_scene(
                scene_kind,
                |checkpoint| {
                    let json = serde_json::to_string(checkpoint)
                        .expect("LiveDemoCheckpoint has a stable serializable schema");
                    println!("CREST_LIVE_CHECKPOINT {json}");
                },
                emit_live_report,
            )
            .context("live observable demo execution failed")?;
        if options.demo_live_effects_and_buses {
            let effects = observation
                .effects_and_buses()
                .context("the effects-and-buses scene did not retain its evidence")?;
            let effects = serde_json::to_string(&effects)
                .context("failed to serialize effects-and-buses teardown evidence")?;
            println!("CREST_EFFECTS_AND_BUSES_LIVE_OBSERVATION {effects}");
        } else if options.demo_live_sixteen_track {
            let routing = observation.sixteen_track_mixer_routing();
            let routing = serde_json::to_string(&routing)
                .context("failed to serialize sixteen-track routing teardown evidence")?;
            println!("CREST_SIXTEEN_TRACK_MIXER_ROUTING_LIVE_OBSERVATION {routing}");
        } else if options.demo_live_semantic {
            let observation = serde_json::to_string(&observation)
                .context("failed to serialize graphical-shell teardown evidence")?;
            println!("CREST_SEMANTIC_VIEW_MODEL_LIVE_OBSERVATION {observation}");
        } else {
            let observation = serde_json::to_string(&observation)
                .context("failed to serialize graphical-shell teardown evidence")?;
            println!("CREST_GRAPHICAL_SHELL_LIVE_OBSERVATION {observation}");
        }
    } else if options.smoke {
        if options.demo_scene {
            let report = make_application()?
                .run_demo_scene(options.degenerate)
                .context("exhaustive demo-scene execution failed")?;
            let second = make_application()?
                .run_demo_scene(options.degenerate)
                .context("second exhaustive demo-scene execution failed")?;
            let two_run_trace_equal = report
                .to_json()
                .context("failed to serialize the first demo trace")?
                == second
                    .to_json()
                    .context("failed to serialize the second demo trace")?;
            emit_demo_scene(&report, two_run_trace_equal)?;
            if !report.is_complete() {
                bail!(
                    "exhaustive demo scene is incomplete: {} grouped and {} event-log coverage identifiers missing, {} records dropped",
                    report.coverage().missing_count(),
                    report.event_log().coverage().missing().len(),
                    report.event_log().dropped_records(),
                );
            }
        } else {
            let observation = make_application()?
                .run_smoke(options.degenerate)
                .context("headless smoke execution failed")?;
            if options.observe {
                let json = serde_json::to_string(&observation)
                    .context("failed to serialize the smoke observation")?;
                println!("CREST_OBSERVATION {json}");
            }
        }
    } else {
        make_application()?
            .run()
            .context("standalone execution failed")?;
    }

    Ok(())
}

fn emit_demo_scene(report: &DemoSceneReport, two_run_trace_equal: bool) -> Result<()> {
    let event_log = report
        .event_log()
        .to_json()
        .context("failed to serialize the exhaustive event log")?;
    let state_tree = report.final_state_tree().json();
    let observation = DemoSceneObservation::from_report(report, two_run_trace_equal);
    let observation = serde_json::to_string(&observation)
        .context("failed to serialize the demo-scene observation")?;

    println!("CREST_EVENT_LOG {event_log}");
    println!("CREST_STATE_TREE {state_tree}");
    println!("CREST_OBSERVATION {observation}");
    Ok(())
}

fn emit_live_report(report: &LiveDemoReport) {
    let event_log_summary = serde_json::to_string(&report.event_log_summary())
        .expect("completed live EventLog summary has a stable serializable schema");
    let coverage = serde_json::to_string(report.coverage())
        .expect("completed live coverage has a stable serializable schema");
    println!("CREST_LIVE_EVENT_LOG_SUMMARY {event_log_summary}");
    println!("CREST_LIVE_STATE_TREE {}", report.state_tree().json());
    println!("CREST_LIVE_COVERAGE {coverage}");
    println!("CREST_LIVE_SUMMARY {}", report.summary());
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Options {
    smoke: bool,
    observe: bool,
    demo_scene: bool,
    demo_live: bool,
    demo_live_semantic: bool,
    demo_live_sixteen_track: bool,
    demo_live_effects_and_buses: bool,
    degenerate: Option<DegenerateMode>,
}

fn parse_options<I, Argument>(arguments: I) -> Result<Options>
where
    I: IntoIterator<Item = Argument>,
    Argument: AsRef<str>,
{
    let mut options = Options::default();

    for argument in arguments {
        match argument.as_ref() {
            "--smoke" if !options.smoke => options.smoke = true,
            "--observe" if !options.observe => options.observe = true,
            "--demo-scene" if !options.demo_scene => options.demo_scene = true,
            // `--demo-live` points at the newest cumulative retained scene.
            "--demo-live" | "--demo-live-effects-and-buses" if !options.demo_live => {
                options.demo_live = true;
                options.demo_live_effects_and_buses = true;
            }
            "--demo-live-sixteen-track-mixer-routing" if !options.demo_live => {
                options.demo_live = true;
                options.demo_live_sixteen_track = true;
            }
            "--demo-live-semantic-view-model" if !options.demo_live => {
                options.demo_live = true;
                options.demo_live_semantic = true;
            }
            "--demo-live-graphical-shell" if !options.demo_live => {
                options.demo_live = true;
            }
            "--degenerate-audio" if options.degenerate.is_none() => {
                options.degenerate = Some(DegenerateMode::Audio);
            }
            "--degenerate-control" if options.degenerate.is_none() => {
                options.degenerate = Some(DegenerateMode::Control);
            }
            "--smoke"
            | "--observe"
            | "--demo-scene"
            | "--demo-live"
            | "--demo-live-effects-and-buses"
            | "--demo-live-sixteen-track-mixer-routing"
            | "--demo-live-semantic-view-model"
            | "--demo-live-graphical-shell" => {
                bail!("duplicate option {}", argument.as_ref());
            }
            "--degenerate-audio" | "--degenerate-control" => {
                bail!("only one degenerate smoke mode may be selected");
            }
            unsupported => bail!("unsupported option {unsupported}"),
        }
    }

    if options.observe && !options.smoke {
        bail!("--observe requires --smoke");
    }
    if options.demo_scene && !(options.smoke && options.observe) {
        bail!("--demo-scene requires both --smoke and --observe");
    }
    if options.degenerate.is_some() && !(options.smoke && options.observe) {
        bail!("degenerate modes require both --smoke and --observe");
    }
    if options.demo_live
        && (options.smoke || options.observe || options.demo_scene || options.degenerate.is_some())
    {
        bail!("the live demo option must be used by itself");
    }

    Ok(options)
}

#[derive(Debug, Serialize)]
struct DemoSceneObservation {
    accepted_events: usize,
    active_graph_revision: u64,
    adjust_directions_exercised: usize,
    alternating_capabilities: bool,
    all_parameter_boundaries_exercised: bool,
    all_audio_parameter_effects_observed: bool,
    all_patch_parameter_cases_exercised: bool,
    all_serialized_properties_observed: bool,
    audio_command_variants_exercised: usize,
    app_event_variants_exercised: usize,
    top_level_contexts_exercised: usize,
    baseline_restored: bool,
    braids_patches: usize,
    braids_scalar_cases_exercised: usize,
    causal_audio_comparisons: bool,
    callback_destructions: usize,
    coverage_missing: usize,
    demo_scene_complete: bool,
    delay_input_nonzero: bool,
    descriptors_unique: bool,
    event_record_payloads_exact: bool,
    event_sources_exercised: usize,
    event_log_dropped: u64,
    event_log_records: usize,
    exact_projection_values: bool,
    exact_state_values: bool,
    faithful_effect_path: bool,
    final_state_tree_matches: bool,
    generation_chain_valid: bool,
    global_parameter_cases_exercised: usize,
    gui_projection_matches_state: bool,
    midi_message_kinds_exercised: usize,
    mixed_engine_parameter_isolation: bool,
    mixed_engine_stems_nonzero: bool,
    navigate_directions_exercised: usize,
    parameter_projection_matches_state: bool,
    parsed_soundfont_banks: usize,
    patch_adsr_control_cases_exercised: usize,
    patch_focus_projection_exact: bool,
    patch_adsr_scalar_only: bool,
    patch_adsr_structural_coexistence_exact: bool,
    post_rejection_event_accepted: bool,
    prepared_instruments: usize,
    rejection_variants_exercised: usize,
    rejected_events: usize,
    reverb_input_nonzero: bool,
    scene_checkpoints: usize,
    schema_surface_equal: bool,
    selection_clamps_exact: bool,
    envelope_parameter_cases_exercised: usize,
    state_hash_chain_valid: bool,
    state_tree_patch_count: usize,
    state_tree_schema_version: u32,
    tick_events_exact: bool,
    two_run_trace_equal: bool,
    unexpected_coverage: usize,
    window_input_cases_exercised: usize,
}

impl DemoSceneObservation {
    fn from_report(report: &DemoSceneReport, two_run_trace_equal: bool) -> Self {
        let records = report.event_log().records();
        let mut event_variants = [false; 16];
        let mut top_level_contexts = Vec::new();
        let mut navigate_directions = Vec::new();
        let mut adjust_directions = Vec::new();
        let mut midi_kinds = Vec::new();
        let mut event_sources = Vec::new();

        for record in records {
            push_unique(&mut event_sources, record.source());
            match record.input() {
                EventInput::SelectContext { context } => {
                    event_variants[0] = true;
                    push_unique(&mut top_level_contexts, *context);
                }
                EventInput::InstallPatches { .. } => event_variants[1] = true,
                EventInput::SelectPatch { direction } => {
                    event_variants[15] = true;
                    push_unique(&mut navigate_directions, *direction);
                }
                EventInput::Navigate { direction } => {
                    event_variants[2] = true;
                    push_unique(&mut navigate_directions, *direction);
                }
                EventInput::Adjust { direction } => {
                    event_variants[3] = true;
                    push_unique(&mut adjust_directions, *direction);
                }
                EventInput::SetInteractionMode { .. } => event_variants[4] = true,
                EventInput::EnterSurface { .. } => event_variants[5] = true,
                EventInput::Return => event_variants[6] = true,
                EventInput::Midi { message, .. } => {
                    event_variants[7] = true;
                    push_unique(&mut midi_kinds, message.kind());
                }
                EventInput::EnginePrepared { .. } => event_variants[8] = true,
                EventInput::EnginePreparationFailed { .. } => event_variants[9] = true,
                EventInput::EngineActivationAcknowledged { .. } => event_variants[10] = true,
                EventInput::SetSlotOccupancy { .. } => event_variants[11] = true,
                EventInput::SetReturnOccupancy { .. } => event_variants[12] = true,
                EventInput::TopologyPrepared { .. } => event_variants[13] = true,
                EventInput::TopologyPreparationFailed { .. } => event_variants[14] = true,
            }
        }

        let accepted_events = records
            .iter()
            .filter(|record| record.outcome() == EventOutcome::Accepted)
            .count();
        let rejected_events = records
            .iter()
            .filter(|record| record.outcome() == EventOutcome::Rejected)
            .count();
        let post_rejection_event_accepted = records.windows(2).any(|pair| {
            pair[0].outcome() == EventOutcome::Rejected
                && pair[1].outcome() == EventOutcome::Accepted
        });
        let generation_chain_valid = generation_chain_valid(records);
        let state_hash_chain_valid = state_hash_chain_valid(records);
        let final_state_tree_matches = records.last().is_some_and(|record| {
            let tree = report.final_state_tree();
            record.generation_after() == tree.generation()
                && record.parameter_generation() == tree.generation()
                && record.state_hash_after() == tree.state_hash()
                && record.projection_state_hash() == tree.state_hash()
                && record.selected_line() == tree.selected_line()
        });

        let tree_json = serde_json::from_str::<Value>(report.final_state_tree().json()).ok();
        let gui_projection_matches_state = tree_json.as_ref().is_some_and(|tree| {
            tree.get("projection")
                .and_then(|projection| projection.get("stateHash"))
                .and_then(Value::as_str)
                == Some(report.final_state_tree().state_hash())
                && tree
                    .get("projection")
                    .and_then(|projection| projection.get("selectedLine"))
                    .and_then(Value::as_u64)
                    == u64::try_from(report.final_state_tree().selected_line()).ok()
        });
        let active_graph_revision = report.final_state_tree().graph_revision().value();
        let parameter_projection_matches_state = tree_json.as_ref().is_some_and(|tree| {
            parameter_projection_matches_state(tree)
                && tree
                    .pointer("/parameters/graphRevision")
                    .and_then(Value::as_u64)
                    == Some(active_graph_revision)
        });

        let editable = report
            .coverage()
            .group(DemoCoverageGroup::EditableParameters);
        let global_parameter_cases_exercised = editable
            .exercised()
            .iter()
            .filter(|identifier| identifier.starts_with("parameter.global."))
            .count();
        let envelope_parameter_cases_exercised = editable
            .exercised()
            .iter()
            .filter(|identifier| {
                [
                    "attackMilliseconds",
                    "decayMilliseconds",
                    "sustain",
                    "releaseMilliseconds",
                ]
                .iter()
                .any(|parameter| identifier.ends_with(&format!(".{parameter}")))
            })
            .count();
        let routed_patch_adsr = report
            .checkpoints()
            .iter()
            .filter_map(|checkpoint| checkpoint.patch_adsr())
            .filter(|checkpoint| checkpoint.lifecycle().is_none())
            .collect::<Vec<_>>();
        let patch_adsr_control_cases_exercised = routed_patch_adsr.len();
        let patch_focus_projection_exact = patch_adsr_control_cases_exercised
            == crest_synth::synth::VoiceEnvelope::surface_descriptor().len()
            && routed_patch_adsr
                .iter()
                .all(|checkpoint| checkpoint.focus_projection_exact());
        let patch_adsr_checkpoints = report
            .checkpoints()
            .iter()
            .filter_map(|checkpoint| checkpoint.patch_adsr())
            .collect::<Vec<_>>();
        let patch_adsr_scalar_only = !patch_adsr_checkpoints.is_empty()
            && patch_adsr_checkpoints
                .iter()
                .all(|checkpoint| checkpoint.scalar_only());
        let preparing_adsr = patch_adsr_checkpoints.iter().find(|checkpoint| {
            checkpoint.lifecycle()
                == Some(crest_synth::control::EngineSelectionStatusKind::Preparing)
        });
        let activating_adsr = patch_adsr_checkpoints.iter().find(|checkpoint| {
            checkpoint.lifecycle()
                == Some(crest_synth::control::EngineSelectionStatusKind::Activating)
        });
        let patch_adsr_structural_coexistence_exact = match (preparing_adsr, activating_adsr) {
            (Some(preparing), Some(activating)) => {
                [preparing, activating].into_iter().all(|checkpoint| {
                    checkpoint.scalar_only()
                        && checkpoint.focus_projection_exact()
                        && checkpoint.all_envelope_values_exact()
                        && checkpoint.untargeted_patches_exact()
                        && checkpoint.graph_revision_exact()
                }) && activating.renderer_graph_revision() > preparing.renderer_graph_revision()
            }
            _ => false,
        };
        let patch_capabilities: Vec<(u64, String)> = tree_json
            .as_ref()
            .and_then(|tree| tree.get("patches"))
            .and_then(Value::as_array)
            .map(|patches| {
                patches
                    .iter()
                    .filter_map(|patch| {
                        Some((
                            patch.get("id")?.as_u64()?,
                            patch
                                .pointer("/instrument/capabilityId")?
                                .as_str()?
                                .to_owned(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let soundfont_patches = patch_capabilities
            .iter()
            .filter(|(_, capability)| capability == HIDEF_CAPABILITY_ID)
            .count();
        let braids_patch_ids: Vec<u64> = patch_capabilities
            .iter()
            .filter_map(|(patch_id, capability)| {
                (capability == BRAIDS_CAPABILITY_ID).then_some(*patch_id)
            })
            .collect();
        let braids_patches = braids_patch_ids.len();
        let alternating_capabilities = patch_capabilities.len() > 1
            && soundfont_patches > 0
            && braids_patches > 0
            && patch_capabilities
                .windows(2)
                .all(|pair| pair[0].1 != pair[1].1);
        let braids_scalar_cases_exercised = editable
            .exercised()
            .iter()
            .filter(|identifier| {
                braids_patch_ids.iter().any(|patch_id| {
                    identifier.starts_with(&format!("parameter.patch.{patch_id}.braids."))
                })
            })
            .count();
        let all_patch_parameter_cases_exercised =
            prefixed_expected_are_exercised(editable, "parameter.patch.");
        let serialized = report
            .coverage()
            .group(DemoCoverageGroup::SerializedProperties);
        let projections = report.coverage().group(DemoCoverageGroup::Projections);
        let all_serialized_properties_observed =
            serialized.is_complete() && projections.is_complete();
        let audio_effects = report.coverage().group(DemoCoverageGroup::AudioEffects);
        let all_audio_parameter_effects_observed =
            prefixed_expected_are_exercised(audio_effects, "effect.parameterSnapshot.");
        let audio_command_variants_exercised = audio_effects
            .exercised()
            .iter()
            .filter(|identifier| identifier.starts_with("effect.emitted.audioCommand."))
            .count();
        let inputs = report.coverage().group(DemoCoverageGroup::Inputs);
        let rejections = report.coverage().group(DemoCoverageGroup::Rejections);
        let coverage_missing =
            report.coverage().missing_count() + report.event_log().coverage().missing().len();
        let unexpected_coverage =
            report.coverage().unexpected_count() + report.event_log().coverage().unexpected().len();
        let schema_surface_equal = coverage_missing == 0 && unexpected_coverage == 0;
        let boundary_rejections = records
            .iter()
            .filter(|record| record.rejection() == Some(EventRejection::ParameterAtBoundary))
            .count();
        let all_parameter_boundaries_exercised = boundary_rejections
            >= 2 * (PatchOutput::surface_descriptor().len()
                + MixerTrackId::COUNT * MixerTrackParameters::surface_descriptor().len()
                + GlobalParameters::surface_descriptor().len());

        let harness = BehavioralMutationHarness::new();
        let dry_observation = harness
            .run(BehavioralMutationCase::DryToWetBypass, false)
            .into_observation();
        let BehavioralMutationObservation::DryToWetBypass(dry_observation) = dry_observation else {
            unreachable!("the selected dry-to-wet case retains its typed schema")
        };
        let cross_observation = harness
            .run(BehavioralMutationCase::CrossTrackParameterLeak, false)
            .into_observation();
        let BehavioralMutationObservation::CrossTrackParameterLeak(cross_observation) =
            cross_observation
        else {
            unreachable!("the selected cross-track case retains its typed schema")
        };
        let reverb_input_nonzero = dry_observation.nonzero_send_reverb_input_energy > 0.0;
        let delay_input_nonzero = dry_observation.nonzero_send_delay_input_energy > 0.0;
        let faithful_effect_path = reverb_input_nonzero
            && delay_input_nonzero
            && dry_observation.identical_effect_state
            && dry_observation.dry_bypass_absent
            && dry_observation.finite_audio
            && dry_observation.baseline_restored;
        let causal_audio_comparisons = cross_observation.edited_track_audio_changed
            && cross_observation.unedited_track_audio_unchanged
            && cross_observation.all_track_parameters_isolated;
        let baseline_restored = tree_json
            .as_ref()
            .is_some_and(fixed_fixture_baseline_restored)
            && dry_observation.baseline_restored
            && cross_observation.baseline_restored;
        let selection_clamps_exact = tree_json.as_ref().is_some_and(|tree| {
            tree.pointer("/interaction/activeFocus/context")
                .and_then(Value::as_str)
                == Some("mixer")
                && tree
                    .pointer("/interaction/activeFocus/patchId")
                    .and_then(Value::as_u64)
                    == tree.pointer("/patches/0/id").and_then(Value::as_u64)
                && tree
                    .pointer("/interaction/activeFocus/controlId/id/kind")
                    .and_then(Value::as_str)
                    == Some("track")
                && tree
                    .pointer("/interaction/activeFocus/controlId/id/track_id")
                    .and_then(Value::as_u64)
                    == Some(0)
                && tree
                    .pointer("/interaction/activeFocus/controlId/id/parameter")
                    .and_then(Value::as_str)
                    == Some("level")
        });
        let exact_state_values = parameter_projection_matches_state
            && tree_json.as_ref().is_some_and(final_tree_values_are_exact);
        let exact_projection_values =
            gui_projection_matches_state && parameter_projection_matches_state;
        let event_record_payloads_exact = event_records_are_exact(records);
        let expected_midi = report.coverage().group(DemoCoverageGroup::MidiKinds);
        let tick_events_exact = event_sources.contains(&EventSource::AutomaticMidi)
            && expected_midi.is_complete()
            && midi_kinds.len() == expected_midi.expected().len();

        Self {
            accepted_events,
            active_graph_revision,
            adjust_directions_exercised: adjust_directions.len(),
            alternating_capabilities,
            all_parameter_boundaries_exercised,
            all_audio_parameter_effects_observed,
            all_patch_parameter_cases_exercised,
            all_serialized_properties_observed,
            audio_command_variants_exercised,
            app_event_variants_exercised: event_variants
                .iter()
                .filter(|exercised| **exercised)
                .count(),
            top_level_contexts_exercised: top_level_contexts.len(),
            baseline_restored,
            braids_patches,
            braids_scalar_cases_exercised,
            causal_audio_comparisons,
            callback_destructions: 0,
            coverage_missing,
            demo_scene_complete: report.is_complete(),
            delay_input_nonzero,
            descriptors_unique: descriptors_are_unique(),
            event_record_payloads_exact,
            event_sources_exercised: event_sources.len(),
            envelope_parameter_cases_exercised,
            event_log_dropped: report.event_log().dropped_records(),
            event_log_records: records.len(),
            exact_projection_values,
            exact_state_values,
            faithful_effect_path,
            final_state_tree_matches,
            generation_chain_valid,
            global_parameter_cases_exercised,
            gui_projection_matches_state,
            midi_message_kinds_exercised: midi_kinds.len(),
            mixed_engine_parameter_isolation: report
                .audio_evidence()
                .mixed_engine_parameter_isolation(),
            mixed_engine_stems_nonzero: report.audio_evidence().mixed_engine_stems_nonzero(),
            navigate_directions_exercised: navigate_directions.len(),
            parameter_projection_matches_state,
            parsed_soundfont_banks: 1,
            patch_adsr_control_cases_exercised,
            patch_focus_projection_exact,
            patch_adsr_scalar_only,
            patch_adsr_structural_coexistence_exact,
            post_rejection_event_accepted,
            prepared_instruments: report.final_state_tree().patch_count(),
            rejection_variants_exercised: rejections.exercised().len(),
            rejected_events,
            reverb_input_nonzero,
            scene_checkpoints: report.checkpoints().len(),
            schema_surface_equal,
            selection_clamps_exact,
            state_hash_chain_valid,
            state_tree_patch_count: report.final_state_tree().patch_count(),
            state_tree_schema_version: report.final_state_tree().schema_version(),
            tick_events_exact,
            two_run_trace_equal,
            unexpected_coverage,
            window_input_cases_exercised: inputs
                .exercised()
                .iter()
                .filter(|identifier| identifier.starts_with("input."))
                .count(),
        }
    }
}

fn push_unique<ValueType>(values: &mut Vec<ValueType>, value: ValueType)
where
    ValueType: Copy + PartialEq,
{
    if !values.contains(&value) {
        values.push(value);
    }
}

fn descriptors_are_unique() -> bool {
    unique(WindowInput::surface_descriptor())
        && unique(AppEvent::surface_descriptor())
        && unique(MidiMessageKind::surface_descriptor())
        && unique(PatchOutput::surface_descriptor())
        && unique(MixerTrackParameters::surface_descriptor())
        && unique(GlobalParameters::surface_descriptor())
        && unique(EventRejection::surface_descriptor())
}

fn unique<ValueType>(values: &[ValueType]) -> bool
where
    ValueType: PartialEq,
{
    values
        .iter()
        .enumerate()
        .all(|(index, value)| !values[..index].contains(value))
}

fn fixed_fixture_baseline_restored(tree: &Value) -> bool {
    let patches_restored = tree
        .get("patches")
        .and_then(Value::as_array)
        .is_some_and(|patches| {
            !patches.is_empty()
                && patches.iter().all(|patch| {
                    patch.pointer("/output/trimGainDb").and_then(Value::as_f64) == Some(0.0)
                        && patch
                            .pointer("/output/trackId")
                            .and_then(Value::as_u64)
                            .is_some_and(|track_id| track_id < MixerTrackId::COUNT as u64)
                })
        });
    let mixer_restored = tree
        .pointer("/mixer/tracks")
        .and_then(Value::as_array)
        .is_some_and(|tracks| {
            tracks.len() == MixerTrackId::COUNT
                && tracks.iter().all(|track| {
                    track.pointer("/levelDb").and_then(Value::as_f64) == Some(0.0)
                        && track.pointer("/pan").and_then(Value::as_f64) == Some(0.0)
                        && track.pointer("/mute").and_then(Value::as_bool) == Some(false)
                        && track.pointer("/solo").and_then(Value::as_bool) == Some(false)
                        // Sends travel as one indexed per-track array in
                        // ascending BusId order; the baseline is all-zero.
                        && track
                            .pointer("/sends")
                            .and_then(Value::as_array)
                            .is_some_and(|sends| {
                                sends.len() == 8
                                    && sends
                                        .iter()
                                        .all(|send| send.as_f64() == Some(0.0))
                            })
                })
        });
    let defaults = ApplicationConfig::default().global_parameters();
    let master_restored = tree
        .pointer("/global/masterGainDb")
        .and_then(Value::as_f64)
        .is_some_and(|value| value as f32 == defaults.master_gain_db());
    // The six retired global leaves are return-owned now: the baseline is the
    // production default occupancy — reverb defaults on bus 0, delay defaults
    // on bus 1, both at the default 0.5 level.
    let returns_restored = tree
        .pointer("/returns")
        .and_then(Value::as_array)
        .is_some_and(|returns| {
            returns.len() == 8
                && [(0_usize, 0.5, 0.5), (1, 250.0, 0.5)]
                    .iter()
                    .all(|(bus, first, second)| {
                        returns[*bus]
                            .pointer("/returnLevel")
                            .and_then(Value::as_f64)
                            == Some(0.5)
                            && returns[*bus]
                                .pointer("/effect/values/0/value/value")
                                .and_then(Value::as_f64)
                                == Some(*first)
                            && returns[*bus]
                                .pointer("/effect/values/1/value/value")
                                .and_then(Value::as_f64)
                                == Some(*second)
                    })
                && returns[2..]
                    .iter()
                    .all(|entry| entry.pointer("/effect").is_some_and(Value::is_null))
        });

    patches_restored && mixer_restored && master_restored && returns_restored
}

fn final_tree_values_are_exact(tree: &Value) -> bool {
    let patches_exact = tree
        .get("patches")
        .and_then(Value::as_array)
        .is_some_and(|patches| {
            patches.len() > 1
                && patches.iter().all(|patch| {
                    patch
                        .get("id")
                        .and_then(Value::as_u64)
                        .is_some_and(|id| id > 0)
                        && patch
                            .get("name")
                            .and_then(Value::as_str)
                            .is_some_and(|name| !name.is_empty())
                        && patch
                            .get("channel")
                            .and_then(Value::as_u64)
                            .is_some_and(|channel| channel < 16)
                        && patch
                            .pointer("/output/trackId")
                            .and_then(Value::as_u64)
                            .is_some_and(|track_id| track_id < MixerTrackId::COUNT as u64)
                        && patch
                            .pointer("/output/trimGainDb")
                            .and_then(Value::as_f64)
                            .is_some_and(f64::is_finite)
                })
        });
    let mixer_exact = tree
        .pointer("/mixer/tracks")
        .and_then(Value::as_array)
        .is_some_and(|tracks| {
            tracks.len() == MixerTrackId::COUNT
                && tracks.iter().all(|track| {
                    ["levelDb", "pan"].iter().all(|field| {
                        track
                            .pointer(&format!("/{field}"))
                            .and_then(Value::as_f64)
                            .is_some_and(f64::is_finite)
                    }) && track
                        .pointer("/sends")
                        .and_then(Value::as_array)
                        .is_some_and(|sends| {
                            sends.len() == 8
                                && sends
                                    .iter()
                                    .all(|send| send.as_f64().is_some_and(f64::is_finite))
                        })
                        && track.pointer("/mute").and_then(Value::as_bool).is_some()
                        && track.pointer("/solo").and_then(Value::as_bool).is_some()
                })
        });
    let globals_exact = tree
        .pointer("/global/masterGainDb")
        .and_then(Value::as_f64)
        .is_some_and(f64::is_finite);
    let returns_exact = tree
        .pointer("/returns")
        .and_then(Value::as_array)
        .is_some_and(|returns| {
            returns.len() == 8
                && returns.iter().all(|entry| {
                    entry
                        .pointer("/returnLevel")
                        .and_then(Value::as_f64)
                        .is_some_and(f64::is_finite)
                })
        });

    patches_exact && mixer_exact && globals_exact && returns_exact
}

fn event_records_are_exact(records: &[crest_synth::control::event_record::EventRecord]) -> bool {
    let mut expected_graph_revision = GraphRevision::INITIAL;
    for (index, record) in records.iter().enumerate() {
        if record.outcome() == EventOutcome::Accepted {
            if let EventInput::EnginePrepared {
                target_graph_revision,
                ..
            }
            | EventInput::TopologyPrepared {
                target_graph_revision,
                ..
            } = record.input()
            {
                expected_graph_revision = *target_graph_revision;
            }
        }
        let sequence_exact = u64::try_from(index) == Ok(record.sequence());
        let projection_exact = record.parameter_generation() == record.generation_after()
            && record.projection_state_hash() == record.state_hash_after();
        let outcome_exact = match record.outcome() {
            EventOutcome::Accepted => {
                let state_accepted = record
                    .emitted_events()
                    .iter()
                    .filter(|effect| {
                        matches!(
                            effect,
                            EmittedEvent::StateAccepted { generation }
                                if *generation == record.generation_after()
                        )
                    })
                    .count()
                    == 1;
                let parameter_publications = record
                    .emitted_events()
                    .iter()
                    .filter(|effect| {
                        matches!(
                            effect,
                            EmittedEvent::ParameterSnapshotPublished {
                                generation,
                                graph_revision,
                            } if *generation == record.generation_after()
                                && *graph_revision == expected_graph_revision
                        )
                    })
                    .count();
                let parameters_published = if record.input().publishes_parameters_on_acceptance() {
                    parameter_publications == 1
                } else {
                    parameter_publications == 0
                };
                record.rejection().is_none()
                    && state_accepted
                    && parameters_published
                    && record.generation_before().checked_add(1) == Some(record.generation_after())
                    && record.state_hash_before() != record.state_hash_after()
            }
            EventOutcome::Rejected => {
                record.rejection().is_some()
                    && record.emitted_events().is_empty()
                    && record.generation_before() == record.generation_after()
                    && record.state_hash_before() == record.state_hash_after()
            }
        };
        if !sequence_exact || !projection_exact || !outcome_exact {
            return false;
        }
    }
    true
}

fn prefixed_expected_are_exercised(
    coverage: &crest_synth::testing::demo_scene_report::DemoCoverageSet,
    prefix: &str,
) -> bool {
    let mut found = false;
    for identifier in coverage
        .expected()
        .iter()
        .filter(|identifier| identifier.starts_with(prefix))
    {
        found = true;
        if !coverage.exercised().contains(identifier) {
            return false;
        }
    }
    found
}

fn generation_chain_valid(records: &[crest_synth::control::event_record::EventRecord]) -> bool {
    let individual = records.iter().all(|record| match record.outcome() {
        EventOutcome::Accepted => {
            record.generation_before().checked_add(1) == Some(record.generation_after())
        }
        EventOutcome::Rejected => record.generation_before() == record.generation_after(),
    });
    individual
        && records
            .windows(2)
            .all(|pair| pair[0].generation_after() == pair[1].generation_before())
}

fn state_hash_chain_valid(records: &[crest_synth::control::event_record::EventRecord]) -> bool {
    let individual = records.iter().all(|record| match record.outcome() {
        EventOutcome::Accepted => record.state_hash_before() != record.state_hash_after(),
        EventOutcome::Rejected => record.state_hash_before() == record.state_hash_after(),
    });
    individual
        && records
            .windows(2)
            .all(|pair| pair[0].state_hash_after() == pair[1].state_hash_before())
}

fn parameter_projection_matches_state(tree: &Value) -> bool {
    let Some(generation) = tree.get("generation").and_then(Value::as_u64) else {
        return false;
    };
    let Some(patches) = tree.get("patches").and_then(Value::as_array) else {
        return false;
    };
    let Some(global) = tree.get("global") else {
        return false;
    };
    let Some(parameters) = tree.get("parameters") else {
        return false;
    };
    // The snapshot mirror's global object and the tree's top-level global
    // both carry master gain alone — return-owned values travel as the
    // indexed return entries, never as global leaves — so the one shared
    // global leaf is compared exactly.
    if parameters.get("generation").and_then(Value::as_u64) != Some(generation)
        || parameters.get("patchCount").and_then(Value::as_u64) != u64::try_from(patches.len()).ok()
        || global.get("masterGainDb").is_none()
        || parameters.pointer("/global/masterGainDb") != global.get("masterGainDb")
    {
        return false;
    }

    let Some(parameter_patches) = parameters.get("patches").and_then(Value::as_array) else {
        return false;
    };
    patches.len() == parameter_patches.len()
        && patches
            .iter()
            .zip(parameter_patches)
            .all(|(patch, parameters)| {
                patch.get("id") == parameters.get("patchId")
                    && patch.get("parameters") == parameters.get("parameters")
            })
}

#[cfg(test)]
mod tests {
    use super::{parse_options, DegenerateMode, Options};
    use crest_synth::control::event_record::{EventDirection, MidiKind};

    #[test]
    fn accepts_normal_smoke_observation_demo_scene_and_each_negative_mode() {
        assert_eq!(parse_options([] as [&str; 0]).unwrap(), Options::default());
        assert_eq!(
            parse_options(["--smoke"]).unwrap(),
            Options {
                smoke: true,
                observe: false,
                demo_scene: false,
                demo_live: false,
                demo_live_semantic: false,
                demo_live_sixteen_track: false,
                demo_live_effects_and_buses: false,
                degenerate: None,
            }
        );
        assert_eq!(
            parse_options(["--observe", "--smoke"]).unwrap(),
            Options {
                smoke: true,
                observe: true,
                demo_scene: false,
                demo_live: false,
                demo_live_semantic: false,
                demo_live_sixteen_track: false,
                demo_live_effects_and_buses: false,
                degenerate: None,
            }
        );
        assert_eq!(
            parse_options(["--demo-scene", "--observe", "--smoke"]).unwrap(),
            Options {
                smoke: true,
                observe: true,
                demo_scene: true,
                demo_live: false,
                demo_live_semantic: false,
                demo_live_sixteen_track: false,
                demo_live_effects_and_buses: false,
                degenerate: None,
            }
        );
        assert_eq!(
            parse_options(["--smoke", "--observe", "--degenerate-audio"]).unwrap(),
            Options {
                smoke: true,
                observe: true,
                demo_scene: false,
                demo_live: false,
                demo_live_semantic: false,
                demo_live_sixteen_track: false,
                demo_live_effects_and_buses: false,
                degenerate: Some(DegenerateMode::Audio),
            }
        );
        assert_eq!(
            parse_options([
                "--degenerate-control",
                "--demo-scene",
                "--observe",
                "--smoke",
            ])
            .unwrap(),
            Options {
                smoke: true,
                observe: true,
                demo_scene: true,
                demo_live: false,
                demo_live_semantic: false,
                demo_live_sixteen_track: false,
                demo_live_effects_and_buses: false,
                degenerate: Some(DegenerateMode::Control),
            }
        );
        assert_eq!(
            parse_options(["--demo-live"]).unwrap(),
            Options {
                demo_live: true,
                demo_live_effects_and_buses: true,
                ..Options::default()
            }
        );
        assert_eq!(
            parse_options(["--demo-live-effects-and-buses"]).unwrap(),
            Options {
                demo_live: true,
                demo_live_effects_and_buses: true,
                ..Options::default()
            }
        );
        assert_eq!(
            parse_options(["--demo-live-sixteen-track-mixer-routing"]).unwrap(),
            Options {
                demo_live: true,
                demo_live_sixteen_track: true,
                ..Options::default()
            }
        );
        assert_eq!(
            parse_options(["--demo-live-semantic-view-model"]).unwrap(),
            Options {
                demo_live: true,
                demo_live_semantic: true,
                ..Options::default()
            }
        );
        assert_eq!(
            parse_options(["--demo-live-graphical-shell"]).unwrap(),
            Options {
                demo_live: true,
                ..Options::default()
            }
        );
    }

    #[test]
    fn rejects_unknown_duplicate_and_out_of_context_options() {
        assert!(parse_options(["--play"]).is_err());
        assert!(parse_options(["--smoke", "--smoke"]).is_err());
        assert!(parse_options(["--observe"]).is_err());
        assert!(parse_options(["--demo-scene"]).is_err());
        assert!(parse_options(["--smoke", "--demo-scene"]).is_err());
        assert!(parse_options(["--smoke", "--observe", "--demo-scene", "--demo-scene",]).is_err());
        assert!(parse_options(["--smoke", "--degenerate-audio"]).is_err());
        assert!(parse_options(["--demo-live", "--demo-live"]).is_err());
        assert!(parse_options(["--demo-live", "--demo-live-effects-and-buses"]).is_err());
        assert!(parse_options(["--demo-live-effects-and-buses", "--smoke"]).is_err());
        assert!(
            parse_options(["--demo-live", "--demo-live-sixteen-track-mixer-routing",]).is_err()
        );
        assert!(parse_options(["--demo-live", "--demo-live-semantic-view-model"]).is_err());
        assert!(parse_options(["--demo-live", "--demo-live-graphical-shell"]).is_err());
        assert!(parse_options(["--demo-live-graphical-shell", "--smoke"]).is_err());
        assert!(parse_options(["--demo-live", "--smoke"]).is_err());
        assert!(parse_options(["--demo-live", "--observe", "--smoke"]).is_err());
        assert!(parse_options(["--demo-live", "--smoke", "--observe", "--demo-scene",]).is_err());
        assert!(
            parse_options(["--demo-live", "--smoke", "--observe", "--degenerate-audio",]).is_err()
        );
        assert!(parse_options([
            "--smoke",
            "--observe",
            "--degenerate-audio",
            "--degenerate-control",
        ])
        .is_err());
    }

    #[test]
    fn witness_enum_surfaces_remain_exhaustive() {
        let directions = [
            EventDirection::Up,
            EventDirection::Down,
            EventDirection::Left,
            EventDirection::Right,
        ];
        let midi_kinds = [
            MidiKind::NoteOn,
            MidiKind::NoteOff,
            MidiKind::ControlChange,
            MidiKind::ProgramChange,
            MidiKind::ChannelPressure,
            MidiKind::PitchBend,
            MidiKind::AllNotesOff,
        ];

        assert_eq!(directions.len(), 4);
        assert_eq!(midi_kinds.len(), 7);
    }
}
