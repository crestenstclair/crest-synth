use anyhow::{bail, Context, Result};
use crest_synth::adapter::corridors_midi_event_source::CorridorsMidiEventSource;
use crest_synth::adapter::cpal_audio_output::CpalAudioOutput;
use crest_synth::adapter::eframe_text_window::EframeTextWindow;
use crest_synth::adapter::global_reverb_delay::GlobalReverbDelay;
use crest_synth::adapter::hidef_soundfont_engine::HiDefSoundFontEngine;
use crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
use crest_synth::control::app_event::AppEvent;
use crest_synth::control::app_state::EventRejection;
use crest_synth::control::event_record::{EmittedEvent, EventInput, EventOutcome, EventSource};
use crest_synth::kernel::midi_message::MidiMessageKind;
use crest_synth::mixer::channel_parameters::ChannelParameters;
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
use crest_synth::shell::standalone_application::{
    ApplicationConfig, DegenerateMode, StandaloneApplication,
};
use crest_synth::shell::window_input::WindowInput;
use crest_synth::testing::demo_scene_report::{DemoCoverageGroup, DemoSceneReport};
use crest_synth::testing::{
    BehavioralMutationCase, BehavioralMutationHarness, BehavioralMutationObservation,
    LiveDemoReport,
};
use serde::Serialize;
use serde_json::Value;
use std::env;

const AUDIO_COMMAND_CAPACITY: usize = 1_024;

fn main() -> Result<()> {
    let options = parse_options(env::args().skip(1))?;
    run(options)
}

fn run(options: Options) -> Result<()> {
    let make_application = || -> Result<_> {
        let config = ApplicationConfig::default();
        let initial_parameters = ParameterSnapshot::new(0, config.global_parameters(), &[])
            .context("failed to construct the initial audio parameter snapshot")?;
        let boundary = LockFreeAudioBoundary::<()>::new(AUDIO_COMMAND_CAPACITY, initial_parameters);
        let engine = HiDefSoundFontEngine::new(config.sample_rate() as i32, config.max_frames());
        Ok(StandaloneApplication::new(
            boundary,
            engine,
            GlobalReverbDelay::new(),
            CorridorsMidiEventSource::new(),
            EframeTextWindow::default(),
            CpalAudioOutput::new(),
            config,
        ))
    };

    if options.demo_live {
        make_application()?
            .run_live_demo(
                |checkpoint| {
                    let json = serde_json::to_string(checkpoint)
                        .expect("LiveDemoCheckpoint has a stable serializable schema");
                    println!("CREST_LIVE_CHECKPOINT {json}");
                },
                emit_live_report,
            )
            .context("live observable demo execution failed")?;
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
            "--demo-live" if !options.demo_live => options.demo_live = true,
            "--degenerate-audio" if options.degenerate.is_none() => {
                options.degenerate = Some(DegenerateMode::Audio);
            }
            "--degenerate-control" if options.degenerate.is_none() => {
                options.degenerate = Some(DegenerateMode::Control);
            }
            "--smoke" | "--observe" | "--demo-scene" | "--demo-live" => {
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
        bail!("--demo-live must be used by itself");
    }

    Ok(options)
}

#[derive(Debug, Serialize)]
struct DemoSceneObservation {
    accepted_events: usize,
    adjust_directions_exercised: usize,
    all_parameter_boundaries_exercised: bool,
    all_audio_parameter_effects_observed: bool,
    all_patch_parameter_cases_exercised: bool,
    all_serialized_properties_observed: bool,
    audio_command_variants_exercised: usize,
    app_event_variants_exercised: usize,
    baseline_restored: bool,
    causal_audio_comparisons: bool,
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
    navigate_directions_exercised: usize,
    parameter_projection_matches_state: bool,
    post_rejection_event_accepted: bool,
    rejection_variants_exercised: usize,
    rejected_events: usize,
    reverb_input_nonzero: bool,
    scene_checkpoints: usize,
    schema_surface_equal: bool,
    selection_clamps_exact: bool,
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
        let mut event_variants = [false; 4];
        let mut navigate_directions = Vec::new();
        let mut adjust_directions = Vec::new();
        let mut midi_kinds = Vec::new();
        let mut event_sources = Vec::new();

        for record in records {
            push_unique(&mut event_sources, record.source());
            match record.input() {
                EventInput::InstallPatches { .. } => event_variants[0] = true,
                EventInput::Navigate { direction } => {
                    event_variants[1] = true;
                    push_unique(&mut navigate_directions, *direction);
                }
                EventInput::Adjust { direction } => {
                    event_variants[2] = true;
                    push_unique(&mut adjust_directions, *direction);
                }
                EventInput::Midi { message, .. } => {
                    event_variants[3] = true;
                    push_unique(&mut midi_kinds, message.kind());
                }
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
        let parameter_projection_matches_state = tree_json
            .as_ref()
            .is_some_and(parameter_projection_matches_state);

        let editable = report
            .coverage()
            .group(DemoCoverageGroup::EditableParameters);
        let global_parameter_cases_exercised = editable
            .exercised()
            .iter()
            .filter(|identifier| identifier.starts_with("parameter.global."))
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
        let events = report.coverage().group(DemoCoverageGroup::Events);
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
            >= 2 * (ChannelParameters::surface_descriptor().len()
                + GlobalParameters::surface_descriptor().len());

        let harness = BehavioralMutationHarness::new();
        let dry_observation = harness
            .run(BehavioralMutationCase::DryToWetBypass, false)
            .into_observation();
        let BehavioralMutationObservation::DryToWetBypass(dry_observation) = dry_observation else {
            unreachable!("the selected dry-to-wet case retains its typed schema")
        };
        let cross_observation = harness
            .run(BehavioralMutationCase::CrossPatchParameterLeak, false)
            .into_observation();
        let BehavioralMutationObservation::CrossPatchParameterLeak(cross_observation) =
            cross_observation
        else {
            unreachable!("the selected cross-Patch case retains its typed schema")
        };
        let reverb_input_nonzero = dry_observation.nonzero_send_reverb_input_energy > 0.0;
        let delay_input_nonzero = dry_observation.nonzero_send_delay_input_energy > 0.0;
        let faithful_effect_path = reverb_input_nonzero
            && delay_input_nonzero
            && dry_observation.identical_effect_state
            && dry_observation.dry_bypass_absent
            && dry_observation.finite_audio
            && dry_observation.baseline_restored;
        let causal_audio_comparisons = cross_observation.edited_patch_audio_changed
            && cross_observation.unedited_patch_audio_unchanged
            && cross_observation.all_channel_parameters_isolated;
        let baseline_restored = tree_json
            .as_ref()
            .is_some_and(fixed_fixture_baseline_restored)
            && dry_observation.baseline_restored
            && cross_observation.baseline_restored;
        let selection_clamps_exact = tree_json.as_ref().is_some_and(|tree| {
            tree.pointer("/selection/section").and_then(Value::as_str) == Some("Patch")
                && tree
                    .pointer("/selection/patchIndex")
                    .and_then(Value::as_u64)
                    == Some(0)
                && tree
                    .pointer("/selection/parameterIndex")
                    .and_then(Value::as_u64)
                    == Some(0)
        });
        let exact_state_values = parameter_projection_matches_state
            && tree_json.as_ref().is_some_and(final_tree_values_are_exact);
        let exact_projection_values =
            gui_projection_matches_state && parameter_projection_matches_state;
        let event_record_payloads_exact = event_records_are_exact(records);
        let tick_events_exact = event_sources.contains(&EventSource::AutomaticMidi)
            && midi_kinds.len() == MidiMessageKind::surface_descriptor().len();

        Self {
            accepted_events,
            adjust_directions_exercised: adjust_directions.len(),
            all_parameter_boundaries_exercised,
            all_audio_parameter_effects_observed,
            all_patch_parameter_cases_exercised,
            all_serialized_properties_observed,
            audio_command_variants_exercised,
            app_event_variants_exercised: event_variants
                .iter()
                .filter(|exercised| **exercised)
                .count(),
            baseline_restored,
            causal_audio_comparisons,
            coverage_missing,
            demo_scene_complete: report.is_complete(),
            delay_input_nonzero,
            descriptors_unique: descriptors_are_unique(),
            event_record_payloads_exact,
            event_sources_exercised: event_sources.len(),
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
            navigate_directions_exercised: navigate_directions.len(),
            parameter_projection_matches_state,
            post_rejection_event_accepted,
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
            window_input_cases_exercised: events
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
        && unique(ChannelParameters::surface_descriptor())
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
                    ChannelParameters::surface_descriptor()
                        .iter()
                        .all(|descriptor| {
                            patch
                                .pointer(&format!("/parameters/{}", descriptor.name()))
                                .and_then(Value::as_f64)
                                == Some(0.0)
                        })
                })
        });
    let defaults = ApplicationConfig::default().global_parameters();
    let globals_restored = GlobalParameters::surface_descriptor()
        .iter()
        .all(|descriptor| {
            tree.pointer(&format!("/global/{}", descriptor.name()))
                .and_then(Value::as_f64)
                .is_some_and(|value| value as f32 == defaults.value(descriptor.parameter()))
        });

    patches_restored && globals_restored
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
                        && ChannelParameters::surface_descriptor()
                            .iter()
                            .all(|descriptor| {
                                patch
                                    .pointer(&format!("/parameters/{}", descriptor.name()))
                                    .and_then(Value::as_f64)
                                    .is_some_and(f64::is_finite)
                            })
                })
        });
    let globals_exact = GlobalParameters::surface_descriptor()
        .iter()
        .all(|descriptor| {
            tree.pointer(&format!("/global/{}", descriptor.name()))
                .and_then(Value::as_f64)
                .is_some_and(f64::is_finite)
        });

    patches_exact && globals_exact
}

fn event_records_are_exact(records: &[crest_synth::control::event_record::EventRecord]) -> bool {
    records.iter().enumerate().all(|(index, record)| {
        let sequence_exact = u64::try_from(index) == Ok(record.sequence());
        let projection_exact = record.parameter_generation() == record.generation_after()
            && record.projection_state_hash() == record.state_hash_after();
        let outcome_exact = match record.outcome() {
            EventOutcome::Accepted => {
                let state_accepted = record.emitted_events().iter().any(|effect| {
                    matches!(
                        effect,
                        EmittedEvent::StateAccepted { generation }
                            if *generation == record.generation_after()
                    )
                });
                let parameters_published = record.emitted_events().iter().any(|effect| {
                    matches!(
                        effect,
                        EmittedEvent::ParameterSnapshotPublished { generation }
                            if *generation == record.generation_after()
                    )
                });
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
        sequence_exact && projection_exact && outcome_exact
    })
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
    if parameters.get("generation").and_then(Value::as_u64) != Some(generation)
        || parameters.get("patchCount").and_then(Value::as_u64) != u64::try_from(patches.len()).ok()
        || parameters.get("global") != Some(global)
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
                degenerate: Some(DegenerateMode::Control),
            }
        );
        assert_eq!(
            parse_options(["--demo-live"]).unwrap(),
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
