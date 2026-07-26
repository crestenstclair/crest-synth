use crate::adapter::braids_capability::BRAIDS_CAPABILITY_ID;
use crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID;
use crate::control::app_event::AppEvent;
use crate::control::app_loop::AppLoop;
use crate::control::app_state::{exercise_reducer_table_rejections, EventRejection};
use crate::control::event_log::{EventCoverage, EventLog, EventLogError};
use crate::control::event_record::{
    AudioEffect, EmittedEvent, EventDirection, EventInput, EventOutcome, EventRecord, EventSource,
    MidiKind,
};
use crate::control::state_tree::StateTree;
use crate::control::text_projection::TextProjection;
use crate::control::TopLevelContext;
use crate::control::{EngineSelectionFailure, EngineSelectionRequestId, EngineSelectionStatusKind};
use crate::real_time::audio_boundary::{AudioThreadBoundary, BoundaryFull, ControlAudioBoundary};
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::audio_observation::CallbackAudioObservation;
use crate::real_time::audio_renderer::AudioRenderer;
use crate::real_time::structural_graph_boundary::AudioStructuralGraphBoundary;
use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
use crate::shell::window_input::{WindowInput, WindowInputKind, WindowKey};
use crate::testing::demo_scene::{
    DemoEngineExpectation, DemoEngineProbe, DemoScene, DemoSceneStep, DemoWorkerAdvance,
};
use crate::testing::demo_scene_report::{
    DemoAudioEvidence, DemoCoverageGroup, DemoEngineCheckpoint, DemoSceneCheckpoint,
    DemoSceneCheckpointError, DemoSceneCoverage, DemoSceneReport, DemoSceneReportError,
};
use crate::testing::DeterministicGraphPreparationHandle;
use core::fmt;
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;

const COVERAGE_GROUPS: [DemoCoverageGroup; 10] = [
    DemoCoverageGroup::Inputs,
    DemoCoverageGroup::Events,
    DemoCoverageGroup::Contexts,
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
    MissingDeterministicWorker,
    WorkerDidNotAdvance,
    Structural(String),
    EngineCheckpoint {
        step: String,
        reason: String,
    },
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
            Self::MissingDeterministicWorker => formatter.write_str(
                "the exhaustive engine scene requires its injected deterministic worker handle",
            ),
            Self::WorkerDidNotAdvance => {
                formatter.write_str("the deterministic graph worker had no pending request")
            }
            Self::Structural(error) => write!(formatter, "structural advancement failed: {error}"),
            Self::EngineCheckpoint { step, reason } => {
                write!(formatter, "engine checkpoint {step} failed: {reason}")
            }
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
pub struct ExhaustiveGuiDemo<'a, ControlBoundary, RenderBoundary, Structural, Observation>
where
    ControlBoundary: ControlAudioBoundary,
    RenderBoundary: AudioThreadBoundary,
    Structural: AudioStructuralGraphBoundary,
    Observation: CallbackAudioObservation,
{
    app_loop: &'a mut AppLoop<ControlBoundary>,
    renderer: &'a mut AudioRenderer<RenderBoundary, Structural, Observation>,
    audio_buffer: &'a mut [f32],
    translator: KeyboardInputTranslator,
    worker: Option<DeterministicGraphPreparationHandle>,
}

impl<'a, ControlBoundary, RenderBoundary, Structural, Observation>
    ExhaustiveGuiDemo<'a, ControlBoundary, RenderBoundary, Structural, Observation>
where
    ControlBoundary: ControlAudioBoundary,
    RenderBoundary: AudioThreadBoundary,
    Structural: AudioStructuralGraphBoundary,
    Observation: CallbackAudioObservation,
{
    /// Injects the already initialized control and callback-side production services.
    pub fn new(
        app_loop: &'a mut AppLoop<ControlBoundary>,
        renderer: &'a mut AudioRenderer<RenderBoundary, Structural, Observation>,
        audio_buffer: &'a mut [f32],
    ) -> Self {
        Self {
            app_loop,
            renderer,
            audio_buffer,
            translator: KeyboardInputTranslator::new(),
            worker: None,
        }
    }

    pub fn new_with_worker(
        app_loop: &'a mut AppLoop<ControlBoundary>,
        renderer: &'a mut AudioRenderer<RenderBoundary, Structural, Observation>,
        audio_buffer: &'a mut [f32],
        worker: DeterministicGraphPreparationHandle,
    ) -> Self {
        Self {
            app_loop,
            renderer,
            audio_buffer,
            translator: KeyboardInputTranslator::new(),
            worker: Some(worker),
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
        let mut run = RunObservations::new(
            initial_measurement,
            self.mixed_engine_stems_are_nonzero(),
            self.renderer.active_revision(),
        );

        self.dispatch_semantic(
            AppEvent::InstallPatches(Vec::new()),
            EventSource::DemoScene,
            Some(EventRejection::InstallationClosed),
            &mut run,
        )?;

        for step in scene.steps() {
            match step {
                DemoSceneStep::WindowInput(input) => {
                    run.observed
                        .insert(format!("input.{}", window_input_identifier(*input)));

                    let actual = self.translator.translate(*input);
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
                    run.mixed_engine_stems_nonzero |= self.mixed_engine_stems_are_nonzero();
                }
                DemoSceneStep::AdvanceWorker(advance) => {
                    let worker = self
                        .worker
                        .as_ref()
                        .ok_or(ExhaustiveGuiDemoError::MissingDeterministicWorker)?;
                    if let DemoWorkerAdvance::Fail(failure) = advance {
                        worker.fail_next(*failure);
                    }
                    if !worker.advance() {
                        return Err(ExhaustiveGuiDemoError::WorkerDidNotAdvance);
                    }
                }
                DemoSceneStep::AdvanceStructural => {
                    let progress = self
                        .app_loop
                        .advance_structural()
                        .map_err(|error| ExhaustiveGuiDemoError::Structural(error.to_string()))?;
                    run.last_rejection = progress.rejected_worker_event();
                }
                DemoSceneStep::EngineProbe(probe) => {
                    let (event, rejection) = self.engine_probe(*probe)?;
                    self.dispatch_semantic(event, EventSource::Worker, Some(rejection), &mut run)?;
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
                    run.mixed_engine_stems_nonzero |= self.mixed_engine_stems_are_nonzero();
                    let tree = self.app_loop.current_state_tree();
                    let mut observation = DemoSceneCheckpoint::new(
                        checkpoint.name(),
                        tree.state_hash(),
                        tree.generation(),
                        tree.selected_line(),
                        tree.generation(),
                        run.audio_measurement,
                    )?;
                    if let Some(expectation) = checkpoint.engine_expectation() {
                        observation =
                            observation.with_engine_selection(self.verify_engine_checkpoint(
                                checkpoint.name(),
                                expectation,
                                &mut run,
                            )?);
                    }
                    run.checkpoints.push(observation);
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
        let audio_evidence = DemoAudioEvidence::new(
            run.mixed_engine_stems_nonzero,
            run.mixed_engine_stems_nonzero && run.all_accepted_adjustments_isolated,
        );
        DemoSceneReport::new(
            scene.name(),
            coverage,
            run.checkpoints,
            event_log,
            final_tree,
        )
        .map(|report| report.with_audio_evidence(audio_evidence))
        .map_err(ExhaustiveGuiDemoError::from)
    }

    fn engine_probe(
        &self,
        probe: DemoEngineProbe,
    ) -> Result<(AppEvent, EventRejection), ExhaustiveGuiDemoError> {
        let correlation = self
            .app_loop
            .state()
            .engine_selection()
            .correlation()
            .ok_or_else(|| ExhaustiveGuiDemoError::EngineCheckpoint {
                step: "engine.probe".to_owned(),
                reason: "canonical state has no pending correlation".to_owned(),
            })?;
        let target_revision = correlation
            .source_graph_revision()
            .checked_next()
            .map_err(|error| ExhaustiveGuiDemoError::Structural(error.to_string()))?;
        Ok(match probe {
            DemoEngineProbe::StaleWorkerFailure => {
                let stale_request = EngineSelectionRequestId::new(
                    correlation.request_id().value().saturating_add(1),
                )
                .map_err(|error| ExhaustiveGuiDemoError::Structural(error.to_string()))?;
                (
                    AppEvent::EnginePreparationFailed {
                        request_id: stale_request,
                        patch_id: correlation.patch_id(),
                        source_capability_id: correlation.source_capability_id().clone(),
                        target_capability_id: correlation.target_capability_id().clone(),
                        source_graph_revision: correlation.source_graph_revision(),
                        target_graph_revision: target_revision,
                        failure: EngineSelectionFailure::PreparationFailed,
                    },
                    EventRejection::StaleEngineSelection,
                )
            }
            DemoEngineProbe::EarlyAcknowledgement => (
                AppEvent::EngineActivationAcknowledged {
                    request_id: correlation.request_id(),
                    target_graph_revision: target_revision,
                    retired_graph_revision: correlation.source_graph_revision(),
                    collected: true,
                },
                EventRejection::StaleEngineSelection,
            ),
            DemoEngineProbe::MismatchedAcknowledgement => (
                AppEvent::EngineActivationAcknowledged {
                    request_id: correlation.request_id(),
                    target_graph_revision: target_revision,
                    retired_graph_revision: target_revision,
                    collected: true,
                },
                EventRejection::MismatchedEngineSelection,
            ),
        })
    }

    fn verify_engine_checkpoint(
        &self,
        step: &str,
        expected: &DemoEngineExpectation,
        run: &mut RunObservations,
    ) -> Result<DemoEngineCheckpoint, ExhaustiveGuiDemoError> {
        let page = self.app_loop.current_patch_page().ok_or_else(|| {
            ExhaustiveGuiDemoError::EngineCheckpoint {
                step: step.to_owned(),
                reason: "PATCH projection is unavailable".to_owned(),
            }
        })?;
        let engine = page.engine();
        let mismatch = engine.status() != expected.status()
            || engine.active_capability_id() != expected.active_capability_id()
            || engine.requested_capability_id() != expected.requested_capability_id()
            || engine.failure() != expected.failure();
        if mismatch {
            return Err(ExhaustiveGuiDemoError::EngineCheckpoint {
                step: step.to_owned(),
                reason: format!(
                    "expected {:?}/{}, requested {:?}, failure {:?}; got {:?}/{}, requested {:?}, failure {:?}",
                    expected.status(),
                    expected.active_capability_id(),
                    expected.requested_capability_id(),
                    expected.failure(),
                    engine.status(),
                    engine.active_capability_id(),
                    engine.requested_capability_id(),
                    engine.failure(),
                ),
            });
        }
        if engine.request_id().is_some() {
            for property in [
                "patchId",
                "requestId",
                "sourceCapabilityId",
                "sourceGraphRevision",
                "targetCapabilityId",
                "targetGraphRevision",
            ] {
                run.observed.insert(format!(
                    "property.stateTree.engineSelection.correlation.{property}"
                ));
            }
        }

        let state_revision = self.app_loop.graph_revision();
        let renderer_revision = self.renderer.active_revision();
        if state_revision != renderer_revision {
            return Err(ExhaustiveGuiDemoError::EngineCheckpoint {
                step: step.to_owned(),
                reason: format!(
                    "state targets graph {state_revision} while renderer owns {renderer_revision}"
                ),
            });
        }
        match expected.status() {
            EngineSelectionStatusKind::Preparing | EngineSelectionStatusKind::Failed
                if state_revision != run.last_ready_graph_revision =>
            {
                return Err(ExhaustiveGuiDemoError::EngineCheckpoint {
                    step: step.to_owned(),
                    reason: "preparation or failure changed the active graph revision".to_owned(),
                });
            }
            EngineSelectionStatusKind::Activating
                if state_revision <= run.last_ready_graph_revision =>
            {
                return Err(ExhaustiveGuiDemoError::EngineCheckpoint {
                    step: step.to_owned(),
                    reason: "activating graph revision did not advance".to_owned(),
                });
            }
            EngineSelectionStatusKind::Ready => {
                if state_revision <= run.last_ready_graph_revision {
                    return Err(ExhaustiveGuiDemoError::EngineCheckpoint {
                        step: step.to_owned(),
                        reason: "acknowledged graph revision did not advance".to_owned(),
                    });
                }
                run.last_ready_graph_revision = state_revision;
            }
            _ => {}
        }

        let target_index = self
            .app_loop
            .patches()
            .iter()
            .position(|patch| patch.id() == page.patch().id())
            .ok_or_else(|| ExhaustiveGuiDemoError::EngineCheckpoint {
                step: step.to_owned(),
                reason: "focused Patch identity is absent from canonical state".to_owned(),
            })?;
        let target_peak = self
            .renderer
            .active_patch_audio()
            .stem(target_index, page.patch().id())
            .map(|stem| {
                stem.samples()
                    .iter()
                    .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
            })
            .unwrap_or(0.0);
        if expected.require_target_audio() && target_peak <= 1.0e-6 {
            return Err(ExhaustiveGuiDemoError::EngineCheckpoint {
                step: step.to_owned(),
                reason: "the selected target Patch stem is silent".to_owned(),
            });
        }

        DemoEngineCheckpoint::new(
            engine.status(),
            engine.active_capability_id().clone(),
            engine.requested_capability_id().cloned(),
            engine.request_id(),
            state_revision,
            renderer_revision,
            engine.failure(),
            target_peak,
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
        let adjustment = matches!(event, AppEvent::Adjust(_))
            && self.app_loop.state().context() == TopLevelContext::Mixer;

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
                run.mixed_engine_stems_nonzero |= self.mixed_engine_stems_are_nonzero();
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
                    } else {
                        run.all_accepted_adjustments_isolated = false;
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

    fn mixed_engine_stems_are_nonzero(&self) -> bool {
        let stems = self.renderer.active_patch_audio();
        let mut soundfont = false;
        let mut braids = false;
        for (index, patch) in self.app_loop.patches().iter().enumerate() {
            let sounding = stems
                .stem(index, patch.id())
                .is_some_and(|stem| stem.samples().iter().any(|sample| sample.abs() > 1.0e-6));
            if !sounding {
                continue;
            }
            match patch.instrument_config().capability_id().as_str() {
                HIDEF_CAPABILITY_ID => soundfont = true,
                BRAIDS_CAPABILITY_ID => braids = true,
                _ => {}
            }
        }
        soundfont && braids
    }
}

struct RunObservations {
    observed: BTreeSet<String>,
    checkpoints: Vec<DemoSceneCheckpoint>,
    audio_measurement: f64,
    last_rejection: Option<EventRejection>,
    mixed_engine_stems_nonzero: bool,
    all_accepted_adjustments_isolated: bool,
    last_ready_graph_revision: crate::real_time::GraphRevision,
}

impl RunObservations {
    fn new(
        audio_measurement: f64,
        mixed_engine_stems_nonzero: bool,
        last_ready_graph_revision: crate::real_time::GraphRevision,
    ) -> Self {
        Self {
            observed: BTreeSet::new(),
            checkpoints: Vec::new(),
            audio_measurement,
            last_rejection: None,
            mixed_engine_stems_nonzero,
            all_accepted_adjustments_isolated: true,
            last_ready_graph_revision,
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
    if identifier.starts_with("input.") {
        Some(DemoCoverageGroup::Inputs)
    } else if identifier.starts_with("event.") {
        Some(DemoCoverageGroup::Events)
    } else if identifier.starts_with("context.") {
        Some(DemoCoverageGroup::Contexts)
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
            EventInput::SelectContext { context } => {
                observed.insert("event.selectContext".to_owned());
                observed.insert(format!("context.{}", context.label().to_ascii_lowercase()));
            }
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
            EventInput::EnginePrepared { .. } => {
                observed.insert("event.enginePrepared".to_owned());
            }
            EventInput::EnginePreparationFailed { .. } => {
                observed.insert("event.enginePreparationFailed".to_owned());
            }
            EventInput::EngineActivationAcknowledged { .. } => {
                observed.insert("event.engineActivationAcknowledged".to_owned());
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
                EmittedEvent::EngineSelection { effect } => {
                    observed.insert(format!(
                        "effect.emitted.engineSelection.{}",
                        effect.kind().name()
                    ));
                }
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
            "context" => TopLevelContext::surface_descriptor().contains(&text.context()),
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
        if path.starts_with("envelope.") || path.starts_with("instrument.") {
            json_path_exists(patch, path)
        } else {
            json_path_exists(patch, &format!("parameters.{path}"))
        }
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

    if before.pointer("/interaction/mixerSelection") != after.pointer("/interaction/mixerSelection")
    {
        return None;
    }

    let before_patches = before.get("patches")?.as_array()?;
    let after_patches = after.get("patches")?.as_array()?;
    if before_patches.len() != after_patches.len() {
        return None;
    }

    let mut changes = Vec::new();
    for (before_patch, after_patch) in before_patches.iter().zip(after_patches) {
        for property in ["id", "name", "channel"] {
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
        for parameter in [
            "attackMilliseconds",
            "decayMilliseconds",
            "sustain",
            "releaseMilliseconds",
        ] {
            let before_value = before_patch
                .get("envelope")
                .and_then(|envelope| envelope.get(parameter))?;
            let after_value = after_patch
                .get("envelope")
                .and_then(|envelope| envelope.get(parameter))?;
            if before_value != after_value {
                changes.push(format!("parameter.patch.{patch_id}.{parameter}"));
            }
        }

        let before_instrument = before_patch.get("instrument")?;
        let after_instrument = after_patch.get("instrument")?;
        for property in ["capabilityId", "assetReferences"] {
            if before_instrument.get(property) != after_instrument.get(property) {
                return None;
            }
        }
        let before_values = before_instrument.get("values")?.as_array()?;
        let after_values = after_instrument.get("values")?.as_array()?;
        if before_values.len() != after_values.len() {
            return None;
        }
        for (before_value, after_value) in before_values.iter().zip(after_values) {
            let parameter_id = after_value.get("parameterId")?.as_str()?;
            if before_value.get("parameterId") != after_value.get("parameterId") {
                return None;
            }
            if before_value.get("value") != after_value.get("value") {
                changes.push(format!("parameter.patch.{patch_id}.{parameter_id}"));
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
        EventRejection::ActionUnavailableInContext => "actionUnavailableInContext",
        EventRejection::EngineSelectionUnavailable => "engineSelectionUnavailable",
        EventRejection::StructuralEditBusy => "structuralEditBusy",
        EventRejection::StaleEngineSelection => "staleEngineSelection",
        EventRejection::MismatchedEngineSelection => "mismatchedEngineSelection",
        EventRejection::RequestIdOverflow => "requestIdOverflow",
        EventRejection::GenerationOverflow => "generationOverflow",
    }
}

const fn window_input_identifier(input: WindowInput) -> &'static str {
    match (input.kind(), input.key()) {
        (WindowInputKind::KeyDown, WindowKey::Digit1) => "keyDown.digit1",
        (WindowInputKind::KeyDown, WindowKey::Digit2) => "keyDown.digit2",
        (WindowInputKind::KeyDown, WindowKey::W) => "keyDown.w",
        (WindowInputKind::KeyDown, WindowKey::S) => "keyDown.s",
        (WindowInputKind::KeyDown, WindowKey::A) => "keyDown.a",
        (WindowInputKind::KeyDown, WindowKey::D) => "keyDown.d",
        (WindowInputKind::KeyDown, WindowKey::K) => "keyDown.k",
        (WindowInputKind::KeyDown, WindowKey::Other) => "keyDown.other",
        (WindowInputKind::KeyUp, WindowKey::Digit1) => "keyUp.digit1",
        (WindowInputKind::KeyUp, WindowKey::Digit2) => "keyUp.digit2",
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
    use crate::adapter::braids_capability::BRAIDS_CAPABILITY_ID;
    use crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID;
    use crate::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
    use crate::adapter::lock_free_structural_graph_boundary::LockFreeStructuralGraphBoundary;
    use crate::adapter::production_instruments::{
        production_capability_registry, production_instrument_preparers,
        production_instrument_providers,
    };
    use crate::control::app_event::AppEvent;
    use crate::control::app_loop::AppLoop;
    use crate::control::app_state::AppState;
    use crate::control::event_log::EventLog;
    use crate::control::event_record::{EventOutcome, EventSource};
    use crate::control::state_projector::StateProjector;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::audio_boundary::AudioBoundary;
    use crate::real_time::audio_renderer::AudioRenderer;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;
    use crate::real_time::prepared_graph_builder::PreparedGraphBuilder;
    use crate::real_time::{GraphHandoffStatus, GraphRevision, StructuralGraphBoundary};
    use crate::shell::audio_output::{AudioDeviceConfig, AudioSampleFormat};
    use crate::synth::{CapabilityId, DescriptorDefaultConfigFactory, Patch};
    use crate::testing::demo_scene::DemoScene;
    use crate::testing::demo_scene_report::DemoCoverageGroup;
    use crate::testing::DeterministicGraphPreparationWorker;

    fn globals() -> GlobalParameters {
        GlobalParameters::new(0.0, 0.5, 0.4, 0.35, 250.0, 0.3, 0.25).unwrap()
    }

    #[test]
    fn exhaustive_gui_demo_scene_uses_production_seams_and_has_no_coverage_gaps() {
        let registry = production_capability_registry().unwrap();
        let factory = DescriptorDefaultConfigFactory::new(
            registry.clone(),
            production_instrument_providers().unwrap(),
        );
        let patches = vec![
            Patch::new(
                PatchId::new(3).unwrap(),
                "Fixture 3".to_owned(),
                factory
                    .create(&CapabilityId::new(HIDEF_CAPABILITY_ID).unwrap())
                    .unwrap(),
                MidiChannel::new(1).unwrap(),
                ChannelParameters::new(0.0, 0.0, 0.3, 0.2).unwrap(),
            ),
            Patch::new(
                PatchId::new(11).unwrap(),
                "Fixture 11".to_owned(),
                factory
                    .create(&CapabilityId::new(BRAIDS_CAPABILITY_ID).unwrap())
                    .unwrap(),
                MidiChannel::new(9).unwrap(),
                ChannelParameters::new(0.0, 0.0, 0.3, 0.2).unwrap(),
            ),
        ];
        let scene = DemoScene::exhaustive(&registry, &patches, &globals()).unwrap();
        let initial_parameters = ParameterSnapshot::new(0, globals(), &[]).unwrap();
        let boundary = LockFreeAudioBoundary::new(64, initial_parameters);
        let (control, audio) = boundary.into_handles();

        let event_log = EventLog::new(scene.event_log_capacity().saturating_add(2)).unwrap();
        let mut app_loop = AppLoop::with_event_log(
            AppState::new(registry.clone(), globals()),
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

        let preparers = production_instrument_preparers().unwrap();
        let graph = PreparedGraphBuilder::new(app_loop.capabilities(), &preparers)
            .build(
                crate::real_time::GraphRevision::INITIAL,
                app_loop.patches(),
                *app_loop.current_parameters(),
                48_000.0,
                512,
            )
            .unwrap();
        let structural = LockFreeStructuralGraphBoundary::new(
            1,
            1,
            GraphHandoffStatus::with_active(GraphRevision::INITIAL),
        )
        .unwrap();
        let (structural_control, structural_audio) = structural.into_handles();
        let audio_config =
            AudioDeviceConfig::new(48_000.0, 2, AudioSampleFormat::F32, 512).unwrap();
        let worker = DeterministicGraphPreparationWorker::new(
            registry.clone(),
            production_instrument_preparers().unwrap(),
            audio_config,
        );
        let worker_handle = worker.advance_handle();
        app_loop
            .configure_engine_selection(
                DescriptorDefaultConfigFactory::new(
                    registry,
                    production_instrument_providers().unwrap(),
                ),
                worker,
                structural_control,
                &graph,
                audio_config,
            )
            .unwrap();
        let mut renderer = AudioRenderer::new(audio, structural_audio, graph);
        let mut audio_buffer = vec![0.0; 1_024];

        let mut demo = ExhaustiveGuiDemo::new_with_worker(
            &mut app_loop,
            &mut renderer,
            &mut audio_buffer,
            worker_handle,
        );
        let report = demo.run(scene).unwrap();

        assert!(
            report.is_complete(),
            "coverage={:?}; eventCoverageMissing={:?}; eventCoverageUnexpected={:?}",
            report.coverage(),
            report.event_log().coverage().missing(),
            report.event_log().coverage().unexpected()
        );
        assert_eq!(report.coverage().missing_count(), 0);
        assert_eq!(report.event_log().dropped_records(), 0);
        assert_eq!(report.final_state_tree().patch_count(), 2);
        assert!(report
            .checkpoints()
            .iter()
            .all(|checkpoint| checkpoint.audio_measurement().is_finite()));

        for group in [
            DemoCoverageGroup::Inputs,
            DemoCoverageGroup::Events,
            DemoCoverageGroup::Contexts,
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
