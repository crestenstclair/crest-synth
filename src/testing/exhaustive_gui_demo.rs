use crate::control::app_event::{AppEvent, Direction};
use crate::control::app_loop::AppLoop;
use crate::control::app_state::{exercise_reducer_table_rejections, EventRejection};
use crate::control::event_log::{EventCoverage, EventLog, EventLogError};
use crate::control::event_record::{
    AudioEffect, EmittedEvent, EventDirection, EventInput, EventOutcome, EventRecord, EventSource,
    MidiKind,
};
use crate::control::state_tree::StateTree;
use crate::control::text_projection::TextProjection;
use crate::mixer::global_effects_processor::GlobalEffectsProcessor;
use crate::real_time::audio_boundary::{AudioThreadBoundary, BoundaryFull, ControlAudioBoundary};
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::audio_renderer::AudioRenderer;
use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
use crate::shell::window_input::{WindowInput, WindowInputKind, WindowKey};
use crate::synth::sound_font_engine::SoundFontEngine;
use crate::testing::demo_scene::{DemoScene, DemoSceneStep};
use crate::testing::demo_scene_report::{
    DemoCoverageGroup, DemoSceneCheckpoint, DemoSceneCheckpointError, DemoSceneCoverage,
    DemoSceneReport, DemoSceneReportError,
};
use core::fmt;
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;

const COVERAGE_GROUPS: [DemoCoverageGroup; 8] = [
    DemoCoverageGroup::Events,
    DemoCoverageGroup::Directions,
    DemoCoverageGroup::MidiKinds,
    DemoCoverageGroup::EditableParameters,
    DemoCoverageGroup::SerializedProperties,
    DemoCoverageGroup::Rejections,
    DemoCoverageGroup::Projections,
    DemoCoverageGroup::AudioEffects,
];

/// A structural or production-seam failure while running an exhaustive scene.
#[derive(Clone, Debug, PartialEq)]
pub enum ExhaustiveGuiDemoError {
    EmptyAudioBuffer,
    MissingInstalledFixtureEvent,
    SourceEventLogDropped {
        dropped: u64,
    },
    TranslationMismatch {
        input: WindowInput,
        expected: Option<AppEvent>,
        actual: Option<AppEvent>,
    },
    ExpectedRejectionAccepted {
        expected: EventRejection,
    },
    RejectionMismatch {
        expected: EventRejection,
        actual: EventRejection,
    },
    CheckpointRejectionMismatch {
        step: String,
        expected: EventRejection,
        actual: Option<EventRejection>,
    },
    AudioBoundaryFull(BoundaryFull),
    NonFiniteAudioMeasurement {
        step: String,
    },
    StateTreeSerialization,
    EventLogSerialization,
    ProjectionStateMismatch,
    EventLog(EventLogError),
    Checkpoint(DemoSceneCheckpointError),
    Report(DemoSceneReportError),
}

impl fmt::Display for ExhaustiveGuiDemoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAudioBuffer => {
                formatter.write_str("the exhaustive demo requires a nonempty prepared audio buffer")
            }
            Self::MissingInstalledFixtureEvent => formatter.write_str(
                "the exhaustive demo must begin after the fixture Patch installation event",
            ),
            Self::SourceEventLogDropped { dropped } => write!(
                formatter,
                "the production AppLoop dropped {dropped} event records before report assembly"
            ),
            Self::TranslationMismatch { .. } => formatter.write_str(
                "KeyboardInputTranslator did not emit the exact normalized GUI event",
            ),
            Self::ExpectedRejectionAccepted { expected } => write!(
                formatter,
                "an event expected to be rejected as {expected} was accepted"
            ),
            Self::RejectionMismatch { expected, actual } => write!(
                formatter,
                "event rejection mismatch: expected {expected}, got {actual}"
            ),
            Self::CheckpointRejectionMismatch {
                step,
                expected,
                actual,
            } => match actual {
                Some(actual) => write!(
                    formatter,
                    "checkpoint {step} expected rejection {expected}, got {actual}"
                ),
                None => write!(
                    formatter,
                    "checkpoint {step} expected rejection {expected}, but the last event was accepted"
                ),
            },
            Self::AudioBoundaryFull(error) => error.fmt(formatter),
            Self::NonFiniteAudioMeasurement { step } => {
                write!(formatter, "audio measurement after {step} was not finite")
            }
            Self::StateTreeSerialization => {
                formatter.write_str("the final StateTree was not valid JSON")
            }
            Self::EventLogSerialization => {
                formatter.write_str("the complete EventLog was not valid JSON")
            }
            Self::ProjectionStateMismatch => formatter.write_str(
                "the current text projection and StateTree do not identify the same accepted state",
            ),
            Self::EventLog(error) => error.fmt(formatter),
            Self::Checkpoint(error) => error.fmt(formatter),
            Self::Report(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ExhaustiveGuiDemoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AudioBoundaryFull(error) => Some(error),
            Self::EventLog(error) => Some(error),
            Self::Checkpoint(error) => Some(error),
            Self::Report(error) => Some(error),
            _ => None,
        }
    }
}

impl From<EventLogError> for ExhaustiveGuiDemoError {
    fn from(error: EventLogError) -> Self {
        Self::EventLog(error)
    }
}

impl From<DemoSceneCheckpointError> for ExhaustiveGuiDemoError {
    fn from(error: DemoSceneCheckpointError) -> Self {
        Self::Checkpoint(error)
    }
}

impl From<DemoSceneReportError> for ExhaustiveGuiDemoError {
    fn from(error: DemoSceneReportError) -> Self {
        Self::Report(error)
    }
}

/// Runs a DemoScene through the same translator, AppLoop, boundary, renderer,
/// engine, and global mixer used by the standalone application.
///
/// The caller retains ownership of the already initialized production services.
/// The scene runner performs only control-thread work and renders into caller-
/// supplied storage that was allocated before the run.
pub struct ExhaustiveGuiDemo<'a, ControlBoundary, RenderBoundary, Engine, Effects>
where
    ControlBoundary: ControlAudioBoundary,
    RenderBoundary: AudioThreadBoundary,
    Engine: SoundFontEngine,
    Effects: GlobalEffectsProcessor,
{
    app_loop: &'a mut AppLoop<ControlBoundary>,
    renderer: &'a mut AudioRenderer<RenderBoundary, Engine, Effects>,
    audio_buffer: &'a mut [f32],
    translator: KeyboardInputTranslator,
}

impl<'a, ControlBoundary, RenderBoundary, Engine, Effects>
    ExhaustiveGuiDemo<'a, ControlBoundary, RenderBoundary, Engine, Effects>
where
    ControlBoundary: ControlAudioBoundary,
    RenderBoundary: AudioThreadBoundary,
    Engine: SoundFontEngine,
    Effects: GlobalEffectsProcessor,
{
    /// Injects the already initialized control and callback-side production services.
    pub fn new(
        app_loop: &'a mut AppLoop<ControlBoundary>,
        renderer: &'a mut AudioRenderer<RenderBoundary, Engine, Effects>,
        audio_buffer: &'a mut [f32],
    ) -> Self {
        Self {
            app_loop,
            renderer,
            audio_buffer,
            translator: KeyboardInputTranslator::new(),
        }
    }

    /// Executes every deterministic scene step and returns a complete diagnostic report.
    pub fn run(&mut self, scene: DemoScene) -> Result<DemoSceneReport, ExhaustiveGuiDemoError> {
        if self.audio_buffer.is_empty() {
            return Err(ExhaustiveGuiDemoError::EmptyAudioBuffer);
        }

        self.translator = KeyboardInputTranslator::new();
        let startup_log = self.app_loop.event_log();
        ensure_installed_fixture(&startup_log)?;

        let initial_measurement = self.render_audio("scene.initial")?;
        let mut run = RunObservations::new(initial_measurement);

        self.dispatch_semantic(
            AppEvent::InstallPatches(Vec::new()),
            EventSource::DemoScene,
            Some(EventRejection::InstallationClosed),
            &mut run,
        )?;

        let mut oracle = TranslationOracle::default();
        for step in scene.steps() {
            match step {
                DemoSceneStep::WindowInput(input) => {
                    run.observed
                        .insert(format!("input.{}", window_input_identifier(*input)));

                    let expected = oracle.translate(*input);
                    let actual = self.translator.translate(*input);
                    if actual != expected {
                        return Err(ExhaustiveGuiDemoError::TranslationMismatch {
                            input: *input,
                            expected,
                            actual,
                        });
                    }
                    if let Some(event) = actual {
                        self.dispatch_semantic(event, EventSource::Keyboard, None, &mut run)?;
                    }
                }
                DemoSceneStep::MidiProbe(probe) => {
                    self.dispatch_semantic(
                        AppEvent::Midi {
                            patch_id: probe.patch_id(),
                            message: probe.message(),
                        },
                        EventSource::DemoScene,
                        probe.expected_rejection(),
                        &mut run,
                    )?;
                }
                DemoSceneStep::AudioCommandProbe(command) => {
                    self.app_loop
                        .push_recovery_command(*command)
                        .map_err(ExhaustiveGuiDemoError::AudioBoundaryFull)?;
                    match command {
                        AudioCommand::PatchMidi { .. } => {
                            run.observed
                                .insert("effect.emitted.audioCommand.patchMidi".to_owned());
                        }
                        AudioCommand::AllNotesOff => {
                            run.observed
                                .insert("effect.emitted.audioCommand.allNotesOff".to_owned());
                        }
                    }
                }
                DemoSceneStep::Tick(elapsed) => {
                    run.audio_measurement = self.render_audio_tick(*elapsed)?;
                }
                DemoSceneStep::Checkpoint(checkpoint) => {
                    if let Some(expected) = checkpoint.expected_last_rejection() {
                        if run.last_rejection != Some(expected) {
                            return Err(ExhaustiveGuiDemoError::CheckpointRejectionMismatch {
                                step: checkpoint.name().to_owned(),
                                expected,
                                actual: run.last_rejection,
                            });
                        }
                    }

                    run.audio_measurement = self.render_audio(checkpoint.name())?;
                    let tree = self.app_loop.current_state_tree();
                    run.checkpoints.push(DemoSceneCheckpoint::new(
                        checkpoint.name(),
                        tree.state_hash(),
                        tree.generation(),
                        tree.selected_line(),
                        tree.generation(),
                        run.audio_measurement,
                    )?);
                }
            }
        }

        let source_log = self.app_loop.event_log();
        if source_log.dropped_records() != 0 {
            return Err(ExhaustiveGuiDemoError::SourceEventLogDropped {
                dropped: source_log.dropped_records(),
            });
        }
        observe_records(source_log.records(), &mut run.observed);
        let probe_patch = self
            .app_loop
            .state()
            .patches()
            .first()
            .expect("the exhaustive scene requires an installed Patch");
        for rejection in exercise_reducer_table_rejections(
            self.app_loop.capabilities(),
            probe_patch.instrument_config(),
        ) {
            run.observed
                .insert(format!("rejection.{}", rejection.name()));
        }

        let expected = scene.expected_coverage().to_vec();
        let mut event_log = rebuild_event_log(&source_log, &expected, scene.event_log_capacity())?;
        let final_tree = self.app_loop.current_state_tree();
        let final_text = self.app_loop.current_text();
        if final_text.state_hash() != final_tree.state_hash() {
            return Err(ExhaustiveGuiDemoError::ProjectionStateMismatch);
        }

        observe_serialized_properties(
            &expected,
            &final_tree,
            &final_text,
            &event_log,
            &mut run.observed,
        )?;

        for identifier in &run.observed {
            event_log.mark_exercised(identifier.clone());
        }

        let coverage = build_coverage(&expected, &run.observed);
        DemoSceneReport::new(
            scene.name(),
            coverage,
            run.checkpoints,
            event_log,
            final_tree,
        )
        .map_err(ExhaustiveGuiDemoError::from)
    }

    fn dispatch_semantic(
        &mut self,
        event: AppEvent,
        source: EventSource,
        expected_rejection: Option<EventRejection>,
        run: &mut RunObservations,
    ) -> Result<(), ExhaustiveGuiDemoError> {
        let before_tree = self.app_loop.current_state_tree();
        let adjustment = matches!(event, AppEvent::Adjust(_));

        match self.app_loop.dispatch_from(event, source) {
            Ok(result) => {
                if let Some(expected) = expected_rejection {
                    return Err(ExhaustiveGuiDemoError::ExpectedRejectionAccepted { expected });
                }
                if let Some(error) = result.boundary_full() {
                    return Err(ExhaustiveGuiDemoError::AudioBoundaryFull(error));
                }

                run.last_rejection = None;
                let after_tree = self.app_loop.current_state_tree();
                let measurement = self.render_audio("accepted event")?;
                if adjustment {
                    if let Some(identifier) =
                        changed_parameter_identifier(&before_tree, &after_tree)
                    {
                        run.observed.insert(identifier.clone());
                        let suffix = identifier
                            .strip_prefix("parameter.")
                            .expect("parameter identifiers have a stable prefix");
                        run.observed
                            .insert(format!("effect.parameterSnapshot.{suffix}"));
                    }
                }
                run.audio_measurement = measurement;
                Ok(())
            }
            Err(actual) => {
                run.last_rejection = Some(actual);
                if let Some(expected) = expected_rejection {
                    if actual != expected {
                        return Err(ExhaustiveGuiDemoError::RejectionMismatch { expected, actual });
                    }
                }
                Ok(())
            }
        }
    }

    fn render_audio_tick(&mut self, _elapsed: Duration) -> Result<f64, ExhaustiveGuiDemoError> {
        self.render_audio("deterministic tick")
    }

    fn render_audio(&mut self, step: &str) -> Result<f64, ExhaustiveGuiDemoError> {
        self.renderer.render(self.audio_buffer);

        let mut measurement = 0.0_f64;
        for (index, sample) in self.audio_buffer.iter().copied().enumerate() {
            if !sample.is_finite() {
                return Err(ExhaustiveGuiDemoError::NonFiniteAudioMeasurement {
                    step: step.to_owned(),
                });
            }
            let channel_weight = if index % 2 == 0 {
                1.0
            } else {
                1.618_033_988_75
            };
            let position_weight = 1.0 + (index % 17) as f64 * 0.000_1;
            measurement += f64::from(sample).abs() * channel_weight * position_weight;
        }
        measurement /= self.audio_buffer.len() as f64;

        if !measurement.is_finite() {
            return Err(ExhaustiveGuiDemoError::NonFiniteAudioMeasurement {
                step: step.to_owned(),
            });
        }
        Ok(measurement)
    }
}

struct RunObservations {
    observed: BTreeSet<String>,
    checkpoints: Vec<DemoSceneCheckpoint>,
    audio_measurement: f64,
    last_rejection: Option<EventRejection>,
}

impl RunObservations {
    fn new(audio_measurement: f64) -> Self {
        Self {
            observed: BTreeSet::new(),
            checkpoints: Vec::new(),
            audio_measurement,
            last_rejection: None,
        }
    }
}

#[derive(Default)]
struct TranslationOracle {
    k_held: bool,
}

impl TranslationOracle {
    fn translate(&mut self, input: WindowInput) -> Option<AppEvent> {
        match input.kind() {
            WindowInputKind::FocusLost => {
                self.k_held = false;
                None
            }
            WindowInputKind::KeyUp => {
                if input.key() == WindowKey::K {
                    self.k_held = false;
                }
                None
            }
            WindowInputKind::KeyDown => {
                if input.key() == WindowKey::K {
                    self.k_held = true;
                    return None;
                }
                let direction = match input.key() {
                    WindowKey::W => Direction::Up,
                    WindowKey::S => Direction::Down,
                    WindowKey::A => Direction::Left,
                    WindowKey::D => Direction::Right,
                    WindowKey::K | WindowKey::Other => return None,
                };
                Some(if self.k_held {
                    AppEvent::Adjust(direction)
                } else {
                    AppEvent::Navigate(direction)
                })
            }
        }
    }
}

fn ensure_installed_fixture(event_log: &EventLog) -> Result<(), ExhaustiveGuiDemoError> {
    if event_log.dropped_records() != 0 {
        return Err(ExhaustiveGuiDemoError::SourceEventLogDropped {
            dropped: event_log.dropped_records(),
        });
    }

    let installed = event_log.records().iter().any(|record| {
        record.outcome() == EventOutcome::Accepted
            && matches!(
                record.input(),
                EventInput::InstallPatches { patches } if !patches.is_empty()
            )
    });
    if installed {
        Ok(())
    } else {
        Err(ExhaustiveGuiDemoError::MissingInstalledFixtureEvent)
    }
}

fn rebuild_event_log(
    source: &EventLog,
    expected: &[String],
    scene_capacity: usize,
) -> Result<EventLog, ExhaustiveGuiDemoError> {
    let capacity = scene_capacity
        .saturating_add(2)
        .max(source.records().len())
        .max(1);
    let mut event_log =
        EventLog::with_coverage(capacity, EventCoverage::new(expected.iter().cloned()))?;
    for record in source.records() {
        event_log.append(record.clone())?;
    }
    Ok(event_log)
}

fn build_coverage(expected: &[String], observed: &BTreeSet<String>) -> DemoSceneCoverage {
    let mut coverage = DemoSceneCoverage::new();
    for group in COVERAGE_GROUPS {
        coverage.declare_expected(
            group,
            expected
                .iter()
                .filter(|identifier| coverage_group(identifier) == Some(group))
                .cloned(),
        );
    }

    for identifier in observed {
        if let Some(group) = coverage_group(identifier) {
            coverage.mark_exercised(group, identifier.clone());
        }
    }
    coverage
}

fn coverage_group(identifier: &str) -> Option<DemoCoverageGroup> {
    if identifier.starts_with("event.") || identifier.starts_with("input.") {
        Some(DemoCoverageGroup::Events)
    } else if identifier.starts_with("direction.") {
        Some(DemoCoverageGroup::Directions)
    } else if identifier.starts_with("midi.") {
        Some(DemoCoverageGroup::MidiKinds)
    } else if identifier.starts_with("parameter.") {
        Some(DemoCoverageGroup::EditableParameters)
    } else if identifier.starts_with("rejection.") {
        Some(DemoCoverageGroup::Rejections)
    } else if identifier.starts_with("effect.") {
        Some(DemoCoverageGroup::AudioEffects)
    } else if identifier.starts_with("property.stateTree.projection.")
        || identifier.starts_with("property.textProjection.")
    {
        Some(DemoCoverageGroup::Projections)
    } else if identifier.starts_with("property.") {
        Some(DemoCoverageGroup::SerializedProperties)
    } else {
        None
    }
}

fn observe_records(records: &[EventRecord], observed: &mut BTreeSet<String>) {
    for record in records {
        match record.input() {
            EventInput::Navigate { direction } => {
                observed.insert("event.navigate".to_owned());
                observed.insert(format!("direction.{}", direction_identifier(*direction)));
            }
            EventInput::Adjust { direction } => {
                observed.insert("event.adjust".to_owned());
                observed.insert(format!("direction.{}", direction_identifier(*direction)));
            }
            EventInput::InstallPatches { .. } => {
                observed.insert("event.installPatches".to_owned());
            }
            EventInput::Midi { message, .. } => {
                observed.insert("event.midi".to_owned());
                observed.insert(format!("midi.{}", midi_kind_identifier(message.kind())));
            }
        }

        if let Some(rejection) = record.rejection() {
            observed.insert(format!("rejection.{}", rejection_identifier(rejection)));
        }

        for emitted in record.emitted_events() {
            match emitted {
                EmittedEvent::StateAccepted { .. } => {
                    observed.insert("effect.emitted.stateAccepted".to_owned());
                }
                EmittedEvent::ParameterSnapshotPublished { .. } => {
                    observed.insert("effect.emitted.parameterSnapshotPublished".to_owned());
                }
                EmittedEvent::AudioCommand { effect } => match effect {
                    AudioEffect::PatchMidi { .. } => {
                        observed.insert("effect.emitted.audioCommand.patchMidi".to_owned());
                    }
                    AudioEffect::AllNotesOff => {
                        observed.insert("effect.emitted.audioCommand.allNotesOff".to_owned());
                    }
                },
            }
        }
    }
}

fn observe_serialized_properties(
    expected: &[String],
    tree: &StateTree,
    text: &TextProjection,
    event_log: &EventLog,
    observed: &mut BTreeSet<String>,
) -> Result<(), ExhaustiveGuiDemoError> {
    let tree_json: Value = serde_json::from_str(tree.json())
        .map_err(|_| ExhaustiveGuiDemoError::StateTreeSerialization)?;
    let event_log_string = event_log
        .to_json()
        .map_err(|_| ExhaustiveGuiDemoError::EventLogSerialization)?;
    let event_log_json: Value = serde_json::from_str(&event_log_string)
        .map_err(|_| ExhaustiveGuiDemoError::EventLogSerialization)?;

    for identifier in expected
        .iter()
        .filter(|identifier| identifier.starts_with("property."))
    {
        if property_present(identifier, &tree_json, text, &event_log_json) {
            observed.insert(identifier.clone());
        }
    }
    Ok(())
}

fn property_present(
    identifier: &str,
    tree: &Value,
    text: &TextProjection,
    event_log: &Value,
) -> bool {
    let Some(property) = identifier.strip_prefix("property.") else {
        return false;
    };

    if let Some(rest) = property.strip_prefix("stateTree.patch.") {
        return dynamic_patch_property(tree, rest, false);
    }
    if let Some(rest) = property.strip_prefix("stateTree.capability.") {
        return dynamic_capability_property(tree, rest);
    }
    if let Some(rest) = property.strip_prefix("stateTree.parameters.patch.") {
        return dynamic_patch_property(tree, rest, true);
    }
    if let Some(path) = property.strip_prefix("stateTree.") {
        return json_path_exists(tree, path);
    }
    if let Some(path) = property.strip_prefix("eventLog.") {
        return json_path_exists(event_log, path);
    }
    if let Some(path) = property.strip_prefix("eventRecord.") {
        let Some(records) = event_log.get("records").and_then(Value::as_array) else {
            return false;
        };
        return !records.is_empty() && records.iter().all(|record| json_path_exists(record, path));
    }
    if let Some(path) = property.strip_prefix("textProjection.") {
        return match path {
            "body" => !text.body().is_empty(),
            "selectedLine" => true,
            "stateHash" => !text.state_hash().is_empty(),
            _ => false,
        };
    }
    false
}

fn dynamic_patch_property(tree: &Value, rest: &str, parameter_projection: bool) -> bool {
    let Some((patch_id, path)) = rest.split_once('.') else {
        return false;
    };
    let Ok(patch_id) = patch_id.parse::<u64>() else {
        return false;
    };

    let patches = if parameter_projection {
        tree.get("parameters")
            .and_then(|parameters| parameters.get("patches"))
    } else {
        tree.get("patches")
    };
    let Some(patches) = patches.and_then(Value::as_array) else {
        return false;
    };

    let patch = patches.iter().find(|patch| {
        let identity = if parameter_projection {
            patch.get("patchId")
        } else {
            patch.get("id")
        };
        identity.and_then(Value::as_u64) == Some(patch_id)
    });
    let Some(patch) = patch else {
        return false;
    };

    if parameter_projection {
        json_path_exists(patch, &format!("parameters.{path}"))
    } else if let Some(rest) = path.strip_prefix("instrument.value.") {
        semantic_array_property(
            patch
                .get("instrument")
                .and_then(|instrument| instrument.get("values")),
            rest,
        )
    } else if let Some(rest) = path.strip_prefix("instrument.asset.") {
        semantic_array_property(
            patch
                .get("instrument")
                .and_then(|instrument| instrument.get("assetReferences")),
            rest,
        )
    } else {
        json_path_exists(patch, path)
    }
}

fn dynamic_capability_property(tree: &Value, rest: &str) -> bool {
    let Some(descriptors) = tree
        .get("capabilities")
        .and_then(|capabilities| capabilities.get("descriptors"))
        .and_then(Value::as_array)
    else {
        return false;
    };
    let Some((descriptor, path)) = semantic_object(descriptors, rest) else {
        return false;
    };

    if let Some(rest) = path.strip_prefix("section.") {
        let Some(sections) = descriptor.get("sections").and_then(Value::as_array) else {
            return false;
        };
        let Some((section, path)) = semantic_object(sections, rest) else {
            return false;
        };
        return json_path_exists(section, path);
    }
    if let Some(rest) = path.strip_prefix("parameter.") {
        let Some(sections) = descriptor.get("sections").and_then(Value::as_array) else {
            return false;
        };
        for section in sections {
            if semantic_array_property(section.get("parameters"), rest) {
                return true;
            }
        }
        return false;
    }
    if let Some(rest) = path.strip_prefix("asset.") {
        return semantic_array_property(descriptor.get("assetRequirements"), rest);
    }
    json_path_exists(descriptor, path)
}

fn semantic_array_property(array: Option<&Value>, rest: &str) -> bool {
    let Some(array) = array.and_then(Value::as_array) else {
        return false;
    };
    let Some((value, path)) = semantic_object(array, rest) else {
        return false;
    };
    json_path_exists(value, path)
}

fn semantic_object<'a>(array: &'a [Value], rest: &'a str) -> Option<(&'a Value, &'a str)> {
    array.iter().find_map(|value| {
        let id = value
            .get("id")
            .or_else(|| value.get("parameterId"))?
            .as_str()?;
        rest.strip_prefix(id)
            .and_then(|suffix| suffix.strip_prefix('.'))
            .map(|path| (value, path))
    })
}

fn json_path_exists(value: &Value, path: &str) -> bool {
    let mut current = value;
    for part in path.split('.') {
        let Some(next) = current.get(part) else {
            return false;
        };
        current = next;
    }
    true
}

fn changed_parameter_identifier(before: &StateTree, after: &StateTree) -> Option<String> {
    let before: Value = serde_json::from_str(before.json()).ok()?;
    let after: Value = serde_json::from_str(after.json()).ok()?;

    if before.get("selection") != after.get("selection") {
        return None;
    }

    let before_patches = before.get("patches")?.as_array()?;
    let after_patches = after.get("patches")?.as_array()?;
    if before_patches.len() != after_patches.len() {
        return None;
    }

    let mut changes = Vec::new();
    for (before_patch, after_patch) in before_patches.iter().zip(after_patches) {
        for property in ["id", "name", "channel", "instrument"] {
            if before_patch.get(property) != after_patch.get(property) {
                return None;
            }
        }
        let patch_id = after_patch.get("id")?.as_u64()?;
        for parameter in ["gainDb", "pan", "reverbSend", "delaySend"] {
            let before_value = before_patch
                .get("parameters")
                .and_then(|parameters| parameters.get(parameter))?;
            let after_value = after_patch
                .get("parameters")
                .and_then(|parameters| parameters.get(parameter))?;
            if before_value != after_value {
                changes.push(format!("parameter.patch.{patch_id}.{parameter}"));
            }
        }
    }

    for parameter in [
        "masterGainDb",
        "reverbRoomSize",
        "reverbDamping",
        "reverbReturn",
        "delayMilliseconds",
        "delayFeedback",
        "delayReturn",
    ] {
        let before_value = before
            .get("global")
            .and_then(|global| global.get(parameter))?;
        let after_value = after
            .get("global")
            .and_then(|global| global.get(parameter))?;
        if before_value != after_value {
            changes.push(format!("parameter.global.{parameter}"));
        }
    }

    if changes.len() == 1 {
        changes.pop()
    } else {
        None
    }
}

const fn direction_identifier(direction: EventDirection) -> &'static str {
    match direction {
        EventDirection::Up => "up",
        EventDirection::Down => "down",
        EventDirection::Left => "left",
        EventDirection::Right => "right",
    }
}

const fn midi_kind_identifier(kind: MidiKind) -> &'static str {
    match kind {
        MidiKind::NoteOn => "noteOn",
        MidiKind::NoteOff => "noteOff",
        MidiKind::ControlChange => "controlChange",
        MidiKind::ProgramChange => "programChange",
        MidiKind::ChannelPressure => "channelPressure",
        MidiKind::PitchBend => "pitchBend",
        MidiKind::AllNotesOff => "allNotesOff",
    }
}

const fn rejection_identifier(rejection: EventRejection) -> &'static str {
    match rejection {
        EventRejection::InstallationClosed => "installationClosed",
        EventRejection::TooManyPatches => "tooManyPatches",
        EventRejection::DuplicateMidiChannel => "duplicateMidiChannel",
        EventRejection::InvalidInstrumentConfig => "invalidInstrumentConfig",
        EventRejection::NoPatchesInstalled => "noPatchesInstalled",
        EventRejection::UnknownPatch => "unknownPatch",
        EventRejection::InvalidSelection => "invalidSelection",
        EventRejection::ParameterAtBoundary => "parameterAtBoundary",
        EventRejection::InvalidParameterValue => "invalidParameterValue",
        EventRejection::GenerationOverflow => "generationOverflow",
    }
}

const fn window_input_identifier(input: WindowInput) -> &'static str {
    match (input.kind(), input.key()) {
        (WindowInputKind::KeyDown, WindowKey::W) => "keyDown.w",
        (WindowInputKind::KeyDown, WindowKey::S) => "keyDown.s",
        (WindowInputKind::KeyDown, WindowKey::A) => "keyDown.a",
        (WindowInputKind::KeyDown, WindowKey::D) => "keyDown.d",
        (WindowInputKind::KeyDown, WindowKey::K) => "keyDown.k",
        (WindowInputKind::KeyDown, WindowKey::Other) => "keyDown.other",
        (WindowInputKind::KeyUp, WindowKey::W) => "keyUp.w",
        (WindowInputKind::KeyUp, WindowKey::S) => "keyUp.s",
        (WindowInputKind::KeyUp, WindowKey::A) => "keyUp.a",
        (WindowInputKind::KeyUp, WindowKey::D) => "keyUp.d",
        (WindowInputKind::KeyUp, WindowKey::K) => "keyUp.k",
        (WindowInputKind::KeyUp, WindowKey::Other) => "keyUp.other",
        (WindowInputKind::FocusLost, _) => "focusLost",
    }
}

#[cfg(test)]
mod tests {
    use super::ExhaustiveGuiDemo;
    use crate::adapter::hidef_soundfont_capability::HiDefSoundFontCapability;
    use crate::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
    use crate::control::app_event::AppEvent;
    use crate::control::app_loop::AppLoop;
    use crate::control::app_state::AppState;
    use crate::control::event_log::EventLog;
    use crate::control::event_record::{EventOutcome, EventSource};
    use crate::control::state_projector::StateProjector;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::MidiMessage;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::mix_engine::MixEngine;
    use crate::real_time::audio_boundary::AudioBoundary;
    use crate::real_time::audio_renderer::AudioRenderer;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;
    use crate::real_time::patch_audio_block::PatchAudioBlock;
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_engine::{SoundFontEngine, SoundFontError};
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::testing::automatic_midi_test::create_soundfont_config;
    use crate::testing::demo_scene::DemoScene;
    use crate::testing::demo_scene_report::DemoCoverageGroup;
    use std::path::Path;

    fn globals() -> GlobalParameters {
        GlobalParameters::new(0.0, 0.5, 0.4, 0.35, 250.0, 0.3, 0.25).unwrap()
    }

    fn patch(id: u32, channel: u8) -> Patch {
        let provider = HiDefSoundFontCapability::new().unwrap();
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Fixture {id}"),
            create_soundfont_config(
                &provider,
                SoundFontInstrument::new(0, id as u8, false).unwrap(),
            )
            .unwrap(),
            MidiChannel::new(channel).unwrap(),
            ChannelParameters::new(0.0, 0.0, 0.3, 0.2).unwrap(),
        )
    }

    #[derive(Default)]
    struct TestEngine;

    impl SoundFontEngine for TestEngine {
        fn load(&mut self, _path: &Path) -> Result<(), SoundFontError> {
            Ok(())
        }

        fn configure_patch(&mut self, _patch: &Patch) -> Result<(), SoundFontError> {
            Ok(())
        }

        fn dispatch(
            &mut self,
            _patch_id: PatchId,
            _message: MidiMessage,
        ) -> Result<(), SoundFontError> {
            Ok(())
        }

        fn all_notes_off(&mut self) {}

        fn render_patches(&mut self, output: &mut PatchAudioBlock, parameters: &ParameterSnapshot) {
            for (index, patch) in parameters.patches().iter().enumerate() {
                let Some(patch_id) = patch.patch_id() else {
                    continue;
                };
                let Some(stem) = output.stem_mut(index, patch_id) else {
                    continue;
                };
                let amplitude = 0.15 + index as f32 * 0.11;
                for frame in stem.chunks_exact_mut(2) {
                    frame[0] = amplitude;
                    frame[1] = amplitude * (1.0 + index as f32 * 0.07);
                }
            }
        }
    }

    struct TestEffects;

    impl GlobalEffectsProcessor for TestEffects {
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
            let reverb_shape =
                1.0 + parameters.reverb_room_size() * 0.31 + parameters.reverb_damping() * 0.17;
            let delay_shape = 1.0
                + parameters.delay_milliseconds() * 0.000_7
                + parameters.delay_feedback() * 0.23;
            for ((sample, reverb), delay) in output.iter_mut().zip(reverb_input).zip(delay_input) {
                *sample += reverb * parameters.reverb_return() * reverb_shape
                    + delay * parameters.delay_return() * delay_shape;
            }
        }
    }

    #[test]
    fn exhaustive_gui_demo_scene_uses_production_seams_and_has_no_coverage_gaps() {
        let patches = vec![patch(3, 1), patch(11, 9)];
        let provider = HiDefSoundFontCapability::new().unwrap();
        let scene =
            DemoScene::exhaustive(&provider.registry().unwrap(), &patches, &globals()).unwrap();
        let initial_parameters = ParameterSnapshot::new(0, globals(), &[]).unwrap();
        let boundary = LockFreeAudioBoundary::<()>::new(4, initial_parameters);
        let (control, audio) = boundary.into_handles();

        let event_log = EventLog::new(scene.event_log_capacity().saturating_add(2)).unwrap();
        let mut app_loop = AppLoop::with_event_log(
            AppState::new(provider.registry().unwrap(), globals()),
            StateProjector::new(),
            control,
            event_log,
        )
        .unwrap();
        app_loop
            .dispatch_from(
                AppEvent::InstallPatches(patches),
                EventSource::AutomaticMidi,
            )
            .unwrap();

        let mut renderer = AudioRenderer::new(audio, TestEngine, MixEngine::new(TestEffects));
        renderer.prepare(32, 48_000.0).unwrap();
        let mut audio_buffer = vec![0.0; 64];

        let mut demo = ExhaustiveGuiDemo::new(&mut app_loop, &mut renderer, &mut audio_buffer);
        let report = demo.run(scene).unwrap();

        assert!(report.is_complete());
        assert_eq!(report.coverage().missing_count(), 0);
        assert_eq!(report.event_log().dropped_records(), 0);
        assert_eq!(report.final_state_tree().patch_count(), 2);
        assert!(report
            .checkpoints()
            .iter()
            .all(|checkpoint| checkpoint.audio_measurement().is_finite()));

        for group in [
            DemoCoverageGroup::Events,
            DemoCoverageGroup::Directions,
            DemoCoverageGroup::MidiKinds,
            DemoCoverageGroup::EditableParameters,
            DemoCoverageGroup::SerializedProperties,
            DemoCoverageGroup::Rejections,
            DemoCoverageGroup::Projections,
            DemoCoverageGroup::AudioEffects,
        ] {
            assert!(report.coverage().group(group).is_complete());
        }

        let accepted = report
            .event_log()
            .records()
            .iter()
            .filter(|record| record.outcome() == EventOutcome::Accepted)
            .count();
        let rejected = report
            .event_log()
            .records()
            .iter()
            .filter(|record| record.outcome() == EventOutcome::Rejected)
            .count();
        assert!(accepted > 0);
        assert!(rejected >= 4);
    }
}
