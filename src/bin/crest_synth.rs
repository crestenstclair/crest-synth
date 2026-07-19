use anyhow::{bail, Context, Result};
use crest_synth::adapter::corridors_midi_event_source::CorridorsMidiEventSource;
use crest_synth::adapter::cpal_audio_output::CpalAudioOutput;
use crest_synth::adapter::eframe_text_window::EframeTextWindow;
use crest_synth::adapter::global_reverb_delay::GlobalReverbDelay;
use crest_synth::adapter::hidef_soundfont_engine::HiDefSoundFontEngine;
use crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
use crest_synth::control::event_record::{EventInput, EventOutcome};
use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
use crest_synth::shell::standalone_application::{
    ApplicationConfig, DegenerateMode, StandaloneApplication,
};
use crest_synth::testing::demo_scene_report::{DemoCoverageGroup, DemoSceneReport};
use serde::Serialize;
use serde_json::Value;
use std::env;

const AUDIO_COMMAND_CAPACITY: usize = 1_024;

fn main() -> Result<()> {
    let options = parse_options(env::args().skip(1))?;
    run(options)
}

fn run(options: Options) -> Result<()> {
    let config = ApplicationConfig::default();
    let initial_parameters = ParameterSnapshot::new(0, config.global_parameters(), &[])
        .context("failed to construct the initial audio parameter snapshot")?;
    let boundary = LockFreeAudioBoundary::<()>::new(AUDIO_COMMAND_CAPACITY, initial_parameters);
    let engine = HiDefSoundFontEngine::new(config.sample_rate() as i32, config.max_frames());
    let application = StandaloneApplication::new(
        boundary,
        engine,
        GlobalReverbDelay::new(),
        CorridorsMidiEventSource::new(),
        EframeTextWindow::default(),
        CpalAudioOutput::new(),
        config,
    );

    if options.smoke {
        if options.demo_scene {
            let report = application
                .run_demo_scene(options.degenerate)
                .context("exhaustive demo-scene execution failed")?;
            emit_demo_scene(&report)?;
            if !report.is_complete() {
                bail!(
                    "exhaustive demo scene is incomplete: {} grouped and {} event-log coverage identifiers missing, {} records dropped",
                    report.coverage().missing_count(),
                    report.event_log().coverage().missing().len(),
                    report.event_log().dropped_records(),
                );
            }
        } else {
            let observation = application
                .run_smoke(options.degenerate)
                .context("headless smoke execution failed")?;
            if options.observe {
                let json = serde_json::to_string(&observation)
                    .context("failed to serialize the smoke observation")?;
                println!("CREST_OBSERVATION {json}");
            }
        }
    } else {
        application.run().context("standalone execution failed")?;
    }

    Ok(())
}

fn emit_demo_scene(report: &DemoSceneReport) -> Result<()> {
    let event_log = report
        .event_log()
        .to_json()
        .context("failed to serialize the exhaustive event log")?;
    let state_tree = report.final_state_tree().json();
    let observation = DemoSceneObservation::from_report(report);
    let observation = serde_json::to_string(&observation)
        .context("failed to serialize the demo-scene observation")?;

    println!("CREST_EVENT_LOG {event_log}");
    println!("CREST_STATE_TREE {state_tree}");
    println!("CREST_OBSERVATION {observation}");
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Options {
    smoke: bool,
    observe: bool,
    demo_scene: bool,
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
            "--degenerate-audio" if options.degenerate.is_none() => {
                options.degenerate = Some(DegenerateMode::Audio);
            }
            "--degenerate-control" if options.degenerate.is_none() => {
                options.degenerate = Some(DegenerateMode::Control);
            }
            "--smoke" | "--observe" | "--demo-scene" => {
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

    Ok(options)
}

#[derive(Debug, Serialize)]
struct DemoSceneObservation {
    accepted_events: usize,
    adjust_directions_exercised: usize,
    all_audio_parameter_effects_observed: bool,
    all_patch_parameter_cases_exercised: bool,
    all_serialized_properties_observed: bool,
    app_event_variants_exercised: usize,
    coverage_missing: usize,
    demo_scene_complete: bool,
    event_log_dropped: u64,
    event_log_records: usize,
    final_state_tree_matches: bool,
    generation_chain_valid: bool,
    global_parameter_cases_exercised: usize,
    gui_projection_matches_state: bool,
    midi_message_kinds_exercised: usize,
    navigate_directions_exercised: usize,
    parameter_projection_matches_state: bool,
    post_rejection_event_accepted: bool,
    rejected_events: usize,
    scene_checkpoints: usize,
    state_hash_chain_valid: bool,
    state_tree_patch_count: usize,
    state_tree_schema_version: u32,
}

impl DemoSceneObservation {
    fn from_report(report: &DemoSceneReport) -> Self {
        let records = report.event_log().records();
        let mut event_variants = [false; 4];
        let mut navigate_directions = Vec::new();
        let mut adjust_directions = Vec::new();
        let mut midi_kinds = Vec::new();

        for record in records {
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

        Self {
            accepted_events,
            adjust_directions_exercised: adjust_directions.len(),
            all_audio_parameter_effects_observed,
            all_patch_parameter_cases_exercised,
            all_serialized_properties_observed,
            app_event_variants_exercised: event_variants
                .iter()
                .filter(|exercised| **exercised)
                .count(),
            coverage_missing: report.coverage().missing_count()
                + report.event_log().coverage().missing().len(),
            demo_scene_complete: report.is_complete(),
            event_log_dropped: report.event_log().dropped_records(),
            event_log_records: records.len(),
            final_state_tree_matches,
            generation_chain_valid,
            global_parameter_cases_exercised,
            gui_projection_matches_state,
            midi_message_kinds_exercised: midi_kinds.len(),
            navigate_directions_exercised: navigate_directions.len(),
            parameter_projection_matches_state,
            post_rejection_event_accepted,
            rejected_events,
            scene_checkpoints: report.checkpoints().len(),
            state_hash_chain_valid,
            state_tree_patch_count: report.final_state_tree().patch_count(),
            state_tree_schema_version: report.final_state_tree().schema_version(),
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
                degenerate: None,
            }
        );
        assert_eq!(
            parse_options(["--observe", "--smoke"]).unwrap(),
            Options {
                smoke: true,
                observe: true,
                demo_scene: false,
                degenerate: None,
            }
        );
        assert_eq!(
            parse_options(["--demo-scene", "--observe", "--smoke"]).unwrap(),
            Options {
                smoke: true,
                observe: true,
                demo_scene: true,
                degenerate: None,
            }
        );
        assert_eq!(
            parse_options(["--smoke", "--observe", "--degenerate-audio"]).unwrap(),
            Options {
                smoke: true,
                observe: true,
                demo_scene: false,
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
                degenerate: Some(DegenerateMode::Control),
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
