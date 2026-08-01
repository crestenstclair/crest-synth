use crate::control::app_event::AppEvent;
use crate::control::app_state::{AppState, EventRejection, StateAccepted};
use crate::control::engine_selection::{
    EngineSelectionEffect, EngineSelectionEffectKind, EngineSelectionFailure,
    EngineSelectionRequestId, EngineSelectionStatusError, StructuralEditIntent,
};
use crate::control::event_log::EventLog;
use crate::control::event_record::{EventRecord, EventSource};
use crate::control::patch_page_projection::PatchPageProjection;
use crate::control::state_projector::{MidiProjectionSeed, StateProjectionError, StateProjector};
use crate::control::state_snapshot::StateSnapshot;
use crate::control::state_tree::StateTree;
use crate::control::text_projection::TextProjection;
use crate::control::{GraphicalShellProjection, SemanticAction};
use crate::real_time::audio_boundary::{BoundaryFull, ControlAudioBoundary};
use crate::real_time::{
    ControlStructuralGraphBoundary, GraphPreparationCorrelation, GraphPreparationRequest,
    GraphPreparationRequestError, GraphPreparationResult, GraphPreparationWorker, GraphRevision,
    GraphRevisionError, GraphStageOutcome, PreparedGraph, PreparedGraphRefreshError,
    StructuralGraphCoordinator, WorkerShutdownError,
};
use crate::shell::audio_output::AudioDeviceConfig;
use crate::synth::instrument_capability::{CapabilityError, CapabilityRegistry};
use crate::synth::DescriptorDefaultConfigFactory;
use core::fmt;

const DEFAULT_EVENT_LOG_CAPACITY: usize = 1024;

/// Observable effects of one accepted application event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchResult {
    accepted: StateAccepted,
    snapshot: StateSnapshot,
    boundary_full: Option<BoundaryFull>,
}

/// Bounded observations from one nonblocking structural control tick.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StructuralProgress {
    worker_result_polled: bool,
    failure_dispatched: bool,
    graph_stage: Option<GraphStageOutcome>,
    graph_published: Option<GraphRevision>,
    activation_acknowledged: Option<GraphRevision>,
    collected_count: u64,
    rejected_worker_event: Option<EventRejection>,
}

impl StructuralProgress {
    pub const fn worker_result_polled(self) -> bool {
        self.worker_result_polled
    }

    pub const fn failure_dispatched(self) -> bool {
        self.failure_dispatched
    }

    pub const fn graph_stage(self) -> Option<GraphStageOutcome> {
        self.graph_stage
    }

    pub const fn graph_published(self) -> Option<GraphRevision> {
        self.graph_published
    }

    pub const fn activation_acknowledged(self) -> Option<GraphRevision> {
        self.activation_acknowledged
    }

    pub const fn collected_count(self) -> u64 {
        self.collected_count
    }

    pub const fn rejected_worker_event(self) -> Option<EventRejection> {
        self.rejected_worker_event
    }
}

/// A control-side ownership or invariant failure while advancing structure.
#[derive(Debug)]
pub enum StructuralAdvanceError {
    AlreadyConfigured,
    RegistryMismatch,
    Revision(GraphRevisionError),
    Refresh(PreparedGraphRefreshError),
    Publication(crate::real_time::GraphPublicationFailure),
    EventLog(crate::control::EventLogError),
    Status(EngineSelectionStatusError),
    WorkerShutdown(WorkerShutdownError),
}

impl fmt::Display for StructuralAdvanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyConfigured => {
                formatter.write_str("engine-selection orchestration is already configured")
            }
            Self::RegistryMismatch => formatter.write_str(
                "engine-selection factory registry does not match canonical application state",
            ),
            Self::Revision(error) => error.fmt(formatter),
            Self::Refresh(error) => error.fmt(formatter),
            Self::Publication(failure) => write!(
                formatter,
                "structural graph publication failed: {failure:?}"
            ),
            Self::EventLog(error) => error.fmt(formatter),
            Self::Status(error) => error.fmt(formatter),
            Self::WorkerShutdown(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for StructuralAdvanceError {}

struct EngineSelectionRuntime {
    factory: DescriptorDefaultConfigFactory,
    worker: Box<dyn GraphPreparationWorker>,
    coordinator: StructuralGraphCoordinator<Box<dyn ControlStructuralGraphBoundary>>,
    audio_config: AudioDeviceConfig,
    activation_record_sequence: Option<(EngineSelectionRequestId, u64)>,
}

impl DispatchResult {
    /// Returns the reducer event identifying the accepted generation.
    pub const fn accepted(&self) -> StateAccepted {
        self.accepted
    }

    /// Returns the canonical serialization of the accepted state.
    pub const fn snapshot(&self) -> &StateSnapshot {
        &self.snapshot
    }

    /// Returns a rejected audio command when the bounded queue was full.
    ///
    /// Queue saturation is not an EventRejection because AppState was already
    /// accepted and its complete parameter projection was already published.
    pub const fn boundary_full(&self) -> Option<BoundaryFull> {
        self.boundary_full
    }

    /// Reports whether every accepted real-time effect was transferred.
    pub const fn audio_effects_published(&self) -> bool {
        self.boundary_full.is_none()
    }
}

/// The one-way control application service.
///
/// The loop is the only owner that exposes mutation of AppState. Input and view
/// adapters receive only dispatch and immutable projection and observation
/// operations.
pub struct AppLoop<Boundary>
where
    Boundary: ControlAudioBoundary,
{
    state: AppState,
    projector: StateProjector,
    boundary: Boundary,
    current_snapshot: StateSnapshot,
    current_patch_page: Option<PatchPageProjection>,
    current_text: TextProjection,
    current_graphical_shell: GraphicalShellProjection,
    current_parameters: crate::real_time::parameter_snapshot::ParameterSnapshot,
    current_state_tree: StateTree,
    event_log: EventLog,
    engine_selection_runtime: Option<EngineSelectionRuntime>,
    deferred_engine_failure: Option<AppEvent>,
    deferred_revision_error: Option<GraphRevisionError>,
}

impl<Boundary> AppLoop<Boundary>
where
    Boundary: ControlAudioBoundary,
{
    /// Creates the loop with a bounded interactive event journal and publishes
    /// the complete startup parameter state.
    pub fn new(
        state: AppState,
        projector: StateProjector,
        boundary: Boundary,
    ) -> Result<Self, StateProjectionError> {
        let event_log = EventLog::new(DEFAULT_EVENT_LOG_CAPACITY)
            .expect("the default event-log capacity is nonzero");
        Self::with_event_log(state, projector, boundary, event_log)
    }

    /// Creates the loop with a caller-sized event journal.
    ///
    /// Deterministic scenes can supply an EventLog large enough for their full
    /// trace, while interactive composition roots use `new`'s bounded default.
    pub fn with_event_log(
        state: AppState,
        projector: StateProjector,
        mut boundary: Boundary,
        event_log: EventLog,
    ) -> Result<Self, StateProjectionError> {
        let (
            current_snapshot,
            current_patch_page,
            current_text,
            current_graphical_shell,
            parameters,
            current_state_tree,
        ) = projector.project_with_shell_tree(&state)?;
        if parameters.graph_revision() != projector.graph_revision() {
            return Err(StateProjectionError::StateTree(
                crate::control::StateTreeError::GraphRevisionMismatch,
            ));
        }
        boundary.publish_parameters(parameters);

        Ok(Self {
            state,
            projector,
            boundary,
            current_snapshot,
            current_patch_page,
            current_text,
            current_graphical_shell,
            current_parameters: parameters,
            current_state_tree,
            event_log,
            engine_selection_runtime: None,
            deferred_engine_failure: None,
            deferred_revision_error: None,
        })
    }

    /// Installs the one worker and one structural-control owner used by engine selection.
    pub fn configure_engine_selection<Worker, Structural>(
        &mut self,
        factory: DescriptorDefaultConfigFactory,
        worker: Worker,
        structural: Structural,
        initial_graph: &PreparedGraph,
        audio_config: AudioDeviceConfig,
    ) -> Result<(), StructuralAdvanceError>
    where
        Worker: GraphPreparationWorker + 'static,
        Structural: ControlStructuralGraphBoundary + 'static,
    {
        if self.engine_selection_runtime.is_some() {
            return Err(StructuralAdvanceError::AlreadyConfigured);
        }
        if factory.registry() != self.state.capabilities() {
            return Err(StructuralAdvanceError::RegistryMismatch);
        }
        let structural: Box<dyn ControlStructuralGraphBoundary> = Box::new(structural);
        self.engine_selection_runtime = Some(EngineSelectionRuntime {
            factory,
            worker: Box::new(worker),
            coordinator: StructuralGraphCoordinator::new(structural, initial_graph),
            audio_config,
            activation_record_sequence: None,
        });
        Ok(())
    }

    /// Applies one event using the stable source for legacy callers.
    pub fn dispatch(&mut self, event: AppEvent) -> Result<DispatchResult, EventRejection> {
        self.dispatch_from(event, EventSource::System)
    }

    /// Maps one normalized user intent to exactly one AppEvent before the
    /// canonical reducer and existing commit-before-project pipeline.
    pub fn dispatch_action(
        &mut self,
        action: SemanticAction,
    ) -> Result<DispatchResult, EventRejection> {
        self.dispatch_action_from(action, EventSource::Keyboard)
    }

    pub fn dispatch_action_from(
        &mut self,
        action: SemanticAction,
        source: EventSource,
    ) -> Result<DispatchResult, EventRejection> {
        let event = AppEvent::from_semantic_action(action.clone());
        self.dispatch_internal(event, source, Some(action))
    }

    /// Applies one sourced event and publishes effects only after complete acceptance.
    pub fn dispatch_from(
        &mut self,
        event: AppEvent,
        source: EventSource,
    ) -> Result<DispatchResult, EventRejection> {
        self.dispatch_internal(event, source, None)
    }

    fn dispatch_internal(
        &mut self,
        event: AppEvent,
        source: EventSource,
        semantic_action: Option<SemanticAction>,
    ) -> Result<DispatchResult, EventRejection> {
        let generation_before = self.state.generation();
        let state_hash_before = self.current_state_tree.state_hash().to_owned();
        let midi_generation_only = matches!(event, AppEvent::Midi { .. });
        let parameters_published = event.publishes_parameters_on_acceptance();

        let reduction = match semantic_action {
            Some(action) => self.state.apply_semantic_action(action),
            None => self.state.apply(event.clone()),
        };
        let outcome = match reduction {
            Ok(outcome) => outcome,
            Err(rejection) => {
                let record = EventRecord::rejected(
                    self.event_log.next_sequence(),
                    source,
                    &event,
                    generation_before,
                    state_hash_before,
                    generation_before,
                    &self.current_text,
                    rejection,
                )
                .expect("cached projections must describe the current rejected state");
                self.event_log
                    .append(record)
                    .expect("AppLoop must append a contiguous rejected event record");
                return Err(rejection);
            }
        };

        let (snapshot, patch_page, text, graphical_shell, parameters, state_tree) =
            if midi_generation_only {
                self.projector
                    .project_midi_generation(
                        &self.state,
                        MidiProjectionSeed::new(
                            &self.current_snapshot,
                            self.current_patch_page.as_ref(),
                            &self.current_text,
                            &self.current_graphical_shell,
                            self.current_parameters,
                            &self.current_state_tree,
                        ),
                    )
                    .expect("accepted MIDI must advance coherent generation-only projections")
            } else {
                self.projector
                    .project_with_shell_tree(&self.state)
                    .expect("an accepted AppState must produce coherent projections")
            };
        let accepted = outcome.accepted();
        let audio_command = outcome.audio_command().copied();
        let engine_selection_effect = outcome.engine_selection_effect().cloned();
        let record_sequence = self.event_log.next_sequence();
        let record = EventRecord::accepted(
            record_sequence,
            source,
            &event,
            generation_before,
            state_hash_before,
            accepted,
            &snapshot,
            parameters.generation(),
            parameters.graph_revision(),
            parameters_published,
            &text,
            audio_command,
            engine_selection_effect.clone(),
        )
        .expect("accepted reducer output and projections must form one coherent record");

        if parameters_published {
            self.boundary.publish_parameters(parameters);
        }
        let boundary_full =
            audio_command.and_then(|command| self.boundary.push_command(command).err());
        self.current_snapshot = snapshot.clone();
        self.current_patch_page = patch_page;
        self.current_text = text;
        self.current_graphical_shell = graphical_shell;
        self.current_parameters = parameters;
        self.current_state_tree = state_tree;
        self.event_log
            .append(record)
            .expect("AppLoop must append a contiguous accepted event record");
        if engine_selection_effect
            .as_ref()
            .is_some_and(|effect| effect.kind() == EngineSelectionEffectKind::PrepareRequested)
        {
            self.submit_engine_selection_request(
                engine_selection_effect
                    .as_ref()
                    .expect("the PrepareRequested effect was just matched"),
            );
        }

        Ok(DispatchResult {
            accepted,
            snapshot,
            boundary_full,
        })
    }

    fn submit_engine_selection_request(&mut self, effect: &EngineSelectionEffect) {
        let target_graph_revision = match effect.source_graph_revision().checked_next() {
            Ok(revision) => revision,
            Err(error) => {
                self.deferred_revision_error = Some(error);
                return;
            }
        };
        let failure_event =
            |failure| structural_preparation_failed_event(effect, target_graph_revision, failure);
        let Some(runtime) = self.engine_selection_runtime.as_mut() else {
            self.deferred_engine_failure =
                Some(failure_event(EngineSelectionFailure::WorkerUnavailable));
            return;
        };

        // Occupancy intents carry the complete topology delta themselves;
        // the request applies it to the active Patch set and return bank.
        if effect.intent().is_occupancy() {
            let correlation = match GraphPreparationCorrelation::for_occupancy(
                effect.request_id(),
                effect.intent().clone(),
                effect.source_graph_revision(),
                target_graph_revision,
            ) {
                Ok(correlation) => correlation,
                Err(error) => {
                    self.deferred_engine_failure = Some(failure_event(map_request_failure(error)));
                    return;
                }
            };
            let request = match GraphPreparationRequest::occupancy(
                correlation,
                self.state.patches(),
                self.state.bus_returns(),
                self.state.generation(),
                *self.state.global(),
                *self.state.mixer(),
                runtime.audio_config,
                runtime.factory.registry(),
                self.state.effects(),
            ) {
                Ok(request) => request,
                Err(error) => {
                    self.deferred_engine_failure = Some(failure_event(map_request_failure(error)));
                    return;
                }
            };
            if runtime.worker.try_submit(request).is_err() {
                self.deferred_engine_failure =
                    Some(failure_event(EngineSelectionFailure::WorkerUnavailable));
            }
            return;
        }

        let (Some(patch_id), Some(source_capability_id), Some(target_capability_id)) = (
            effect.patch_id(),
            effect.source_capability_id(),
            effect.target_capability_id(),
        ) else {
            self.deferred_engine_failure =
                Some(failure_event(EngineSelectionFailure::GraphIncompatible));
            return;
        };
        let source_config = self
            .state
            .patches()
            .iter()
            .find(|patch| patch.id() == patch_id)
            .map(|patch| patch.instrument_config());
        let candidate_result = match effect.intent() {
            StructuralEditIntent::ReplaceCapability { .. } => {
                runtime.factory.create(target_capability_id)
            }
            StructuralEditIntent::ReplaceParameterChoice {
                parameter_id,
                choice_id,
                ..
            } => source_config
                .ok_or_else(|| CapabilityError::UnknownCapability(source_capability_id.clone()))
                .and_then(|source| {
                    runtime
                        .factory
                        .replace_structural_choice(source, parameter_id, choice_id)
                }),
            StructuralEditIntent::SetSlotOccupancy { .. }
            | StructuralEditIntent::SetReturnOccupancy { .. } => {
                unreachable!("occupancy intents were submitted above")
            }
        };
        let candidate_config = match candidate_result {
            Ok(config) => config,
            Err(error) => {
                self.deferred_engine_failure = Some(failure_event(
                    map_structural_capability_failure(effect.intent(), &error),
                ));
                return;
            }
        };
        let correlation = match GraphPreparationCorrelation::new_with_intent(
            effect.request_id(),
            patch_id,
            effect.intent().clone(),
            source_capability_id.clone(),
            target_capability_id.clone(),
            effect.source_graph_revision(),
            target_graph_revision,
        ) {
            Ok(correlation) => correlation,
            Err(error) => {
                self.deferred_engine_failure = Some(failure_event(map_request_failure(error)));
                return;
            }
        };
        let request = match GraphPreparationRequest::replacement_with_effects(
            correlation,
            self.state.patches(),
            candidate_config,
            self.state.generation(),
            *self.state.global(),
            *self.state.mixer(),
            runtime.audio_config,
            runtime.factory.registry(),
            self.state.effects(),
            self.state.bus_returns(),
        ) {
            Ok(request) => request,
            Err(error) => {
                self.deferred_engine_failure = Some(failure_event(map_request_failure(error)));
                return;
            }
        };
        if runtime.worker.try_submit(request).is_err() {
            self.deferred_engine_failure =
                Some(failure_event(EngineSelectionFailure::WorkerUnavailable));
        }
    }

    /// Advances at most one worker result and one structural handoff observation.
    ///
    /// The method never waits. Every lifecycle mutation is routed back through
    /// `dispatch_from` and the production reducer with `EventSource::Worker`.
    pub fn advance_structural(&mut self) -> Result<StructuralProgress, StructuralAdvanceError> {
        if let Some(error) = self.deferred_revision_error.take() {
            return Err(StructuralAdvanceError::Revision(error));
        }
        let mut progress = StructuralProgress::default();

        if let Some(event) = self.deferred_engine_failure.take() {
            match self.dispatch_from(event, EventSource::Worker) {
                Ok(_) => progress.failure_dispatched = true,
                Err(rejection) => progress.rejected_worker_event = Some(rejection),
            }
        }

        let worker_result = self
            .engine_selection_runtime
            .as_mut()
            .and_then(|runtime| runtime.worker.try_poll());
        if let Some(result) = worker_result {
            progress.worker_result_polled = true;
            self.handle_graph_preparation_result(result, &mut progress)?;
        }

        let coordinator_progress = self
            .engine_selection_runtime
            .as_mut()
            .map(|runtime| runtime.coordinator.poll());
        let Some(coordinator_progress) = coordinator_progress else {
            return Ok(progress);
        };
        progress.collected_count = coordinator_progress.collected_count();

        if let Some(revision) = coordinator_progress.published_revision() {
            let effect = self.structural_effect(EngineSelectionEffectKind::GraphPublished)?;
            let sequence = self
                .engine_selection_runtime
                .as_ref()
                .and_then(|runtime| runtime.activation_record_sequence)
                .filter(|(request_id, _)| *request_id == effect.request_id())
                .map(|(_, sequence)| sequence)
                .ok_or(StructuralAdvanceError::Status(
                    EngineSelectionStatusError::MissingCorrelation,
                ))?;
            self.event_log
                .append_engine_selection_effect(sequence, effect)
                .map_err(StructuralAdvanceError::EventLog)?;
            progress.graph_published = Some(revision);
        }

        if let Some(target_graph_revision) = coordinator_progress.completed_revision() {
            let correlation = self.state.engine_selection().correlation().cloned().ok_or(
                StructuralAdvanceError::Status(EngineSelectionStatusError::MissingCorrelation),
            )?;
            if correlation.target_graph_revision() != Some(target_graph_revision) {
                return Err(StructuralAdvanceError::Status(
                    EngineSelectionStatusError::InvalidTransition,
                ));
            }
            let event = AppEvent::EngineActivationAcknowledged {
                request_id: correlation.request_id(),
                intent: correlation.intent().clone(),
                target_graph_revision,
                retired_graph_revision: correlation.source_graph_revision(),
                collected: true,
            };
            match self.dispatch_from(event, EventSource::Worker) {
                Ok(_) => {
                    progress.activation_acknowledged = Some(target_graph_revision);
                    if let Some(runtime) = self.engine_selection_runtime.as_mut() {
                        runtime.activation_record_sequence = None;
                    }
                }
                Err(rejection) => progress.rejected_worker_event = Some(rejection),
            }
        }

        Ok(progress)
    }

    fn handle_graph_preparation_result(
        &mut self,
        result: GraphPreparationResult,
        progress: &mut StructuralProgress,
    ) -> Result<(), StructuralAdvanceError> {
        match result {
            GraphPreparationResult::Failed {
                correlation,
                failure,
            } => {
                let event = if correlation.intent().is_occupancy() {
                    AppEvent::TopologyPreparationFailed {
                        request_id: correlation.request_id(),
                        intent: correlation.intent().clone(),
                        source_graph_revision: correlation.source_graph_revision(),
                        target_graph_revision: correlation.target_graph_revision(),
                        failure,
                    }
                } else {
                    AppEvent::EnginePreparationFailed {
                        request_id: correlation.request_id(),
                        patch_id: correlation
                            .patch_id()
                            .ok_or(StructuralAdvanceError::Status(
                                EngineSelectionStatusError::MissingCorrelation,
                            ))?,
                        intent: correlation.intent().clone(),
                        source_capability_id: correlation
                            .source_capability_id()
                            .ok_or(StructuralAdvanceError::Status(
                                EngineSelectionStatusError::MissingCorrelation,
                            ))?
                            .clone(),
                        target_capability_id: correlation
                            .target_capability_id()
                            .ok_or(StructuralAdvanceError::Status(
                                EngineSelectionStatusError::MissingCorrelation,
                            ))?
                            .clone(),
                        source_graph_revision: correlation.source_graph_revision(),
                        target_graph_revision: correlation.target_graph_revision(),
                        failure,
                    }
                };
                match self.dispatch_from(event, EventSource::Worker) {
                    Ok(_) => progress.failure_dispatched = true,
                    Err(rejection) => progress.rejected_worker_event = Some(rejection),
                }
            }
            GraphPreparationResult::Prepared {
                correlation,
                candidate_config,
                mut prepared_graph,
            } => {
                let event = if correlation.intent().is_occupancy() {
                    AppEvent::TopologyPrepared {
                        request_id: correlation.request_id(),
                        intent: correlation.intent().clone(),
                        source_graph_revision: correlation.source_graph_revision(),
                        target_graph_revision: correlation.target_graph_revision(),
                    }
                } else {
                    AppEvent::EnginePrepared {
                        request_id: correlation.request_id(),
                        patch_id: correlation
                            .patch_id()
                            .ok_or(StructuralAdvanceError::Status(
                                EngineSelectionStatusError::MissingCorrelation,
                            ))?,
                        intent: correlation.intent().clone(),
                        source_capability_id: correlation
                            .source_capability_id()
                            .ok_or(StructuralAdvanceError::Status(
                                EngineSelectionStatusError::MissingCorrelation,
                            ))?
                            .clone(),
                        target_capability_id: correlation
                            .target_capability_id()
                            .ok_or(StructuralAdvanceError::Status(
                                EngineSelectionStatusError::MissingCorrelation,
                            ))?
                            .clone(),
                        source_graph_revision: correlation.source_graph_revision(),
                        target_graph_revision: correlation.target_graph_revision(),
                        candidate_config: candidate_config.ok_or(
                            StructuralAdvanceError::Status(
                                EngineSelectionStatusError::MissingCorrelation,
                            ),
                        )?,
                    }
                };
                let record_sequence = self.event_log.next_sequence();
                if let Err(rejection) = self.dispatch_from(event, EventSource::Worker) {
                    progress.rejected_worker_event = Some(rejection);
                    return Ok(());
                }

                let scope = replacement_scope(&correlation).ok_or(
                    StructuralAdvanceError::Status(EngineSelectionStatusError::MissingCorrelation),
                )?;
                prepared_graph
                    .refresh_initial_parameters(self.current_parameters)
                    .map_err(StructuralAdvanceError::Refresh)?;
                let outcome = self
                    .engine_selection_runtime
                    .as_mut()
                    .expect("a polled worker result retains its configured runtime")
                    .coordinator
                    .stage_replacement(prepared_graph, scope)
                    .map_err(|error| {
                        let failure = error.reason();
                        drop(error.into_graph());
                        StructuralAdvanceError::Publication(failure)
                    })?;
                let effect_kind = match outcome {
                    GraphStageOutcome::Published => EngineSelectionEffectKind::GraphPublished,
                    GraphStageOutcome::Staged => EngineSelectionEffectKind::GraphStaged,
                };
                let effect = self.structural_effect(effect_kind)?;
                self.event_log
                    .append_engine_selection_effect(record_sequence, effect)
                    .map_err(StructuralAdvanceError::EventLog)?;
                let runtime = self
                    .engine_selection_runtime
                    .as_mut()
                    .expect("a staged graph retains its configured runtime");
                runtime.activation_record_sequence =
                    Some((correlation.request_id(), record_sequence));
                progress.graph_stage = Some(outcome);
                if outcome == GraphStageOutcome::Published {
                    progress.graph_published = Some(correlation.target_graph_revision());
                }
            }
        }
        Ok(())
    }

    fn structural_effect(
        &self,
        kind: EngineSelectionEffectKind,
    ) -> Result<EngineSelectionEffect, StructuralAdvanceError> {
        let correlation =
            self.state
                .engine_selection()
                .correlation()
                .ok_or(StructuralAdvanceError::Status(
                    EngineSelectionStatusError::MissingCorrelation,
                ))?;
        EngineSelectionEffect::from_correlation(kind, correlation)
            .map_err(StructuralAdvanceError::Status)
    }

    /// Joins and drains worker ownership on the calling control thread.
    pub fn shutdown_engine_selection_on_control(&mut self) -> Result<(), StructuralAdvanceError> {
        if let Some(runtime) = self.engine_selection_runtime.as_mut() {
            let _ = runtime.coordinator.poll();
            runtime
                .worker
                .shutdown_on_control()
                .map_err(StructuralAdvanceError::WorkerShutdown)?;
        }
        Ok(())
    }

    /// Counts structural graph obligations still retained by control-side
    /// orchestration. A completed teardown must report zero.
    pub fn owned_structural_graphs_on_control(&self) -> usize {
        self.engine_selection_runtime.as_ref().map_or(0, |runtime| {
            usize::from(runtime.coordinator.staged_revision().is_some())
                + usize::from(runtime.coordinator.in_flight_revision().is_some())
        })
    }

    pub const fn engine_selection_configured(&self) -> bool {
        self.engine_selection_runtime.is_some()
    }

    /// Returns the newest complete immutable text projection.
    pub fn current_text(&self) -> TextProjection {
        self.current_text.clone()
    }

    /// Returns the newest immutable graphical projection consumed by the
    /// production window.
    pub fn current_graphical_shell(&self) -> GraphicalShellProjection {
        self.current_graphical_shell.clone()
    }

    /// Returns the canonical semantic model embedded in the newest shell.
    pub fn current_semantic_model(&self) -> crate::control::SemanticGraphicalViewModel {
        self.current_graphical_shell.semantic_model().clone()
    }

    /// Returns the newest host-neutral PATCH page exactly when PATCH is active.
    pub fn current_patch_page(&self) -> Option<PatchPageProjection> {
        self.current_patch_page.clone()
    }

    /// Returns the newest canonical state and projection tree.
    pub fn current_state_tree(&self) -> StateTree {
        self.current_state_tree.clone()
    }

    /// Returns the immutable capability metadata installed in canonical state.
    pub const fn capabilities(&self) -> &CapabilityRegistry {
        self.state.capabilities()
    }

    pub const fn effects(&self) -> &crate::synth::EffectCapabilityRegistry {
        self.state.effects()
    }

    /// Returns the immutable accepted Patch set used to prepare audio graphs.
    /// Returns the canonical eight-return bank owned by accepted state.
    pub const fn bus_returns(&self) -> &crate::mixer::bus_return::BusReturnBank {
        self.state.bus_returns()
    }

    pub fn patches(&self) -> &[crate::synth::patch::Patch] {
        self.state.patches()
    }

    /// Returns the latest complete scalar projection published to audio.
    pub const fn current_parameters(
        &self,
    ) -> &crate::real_time::parameter_snapshot::ParameterSnapshot {
        &self.current_parameters
    }

    /// Returns the prepared graph revision targeted by every runtime projection.
    pub const fn graph_revision(&self) -> crate::real_time::GraphRevision {
        self.current_parameters.graph_revision()
    }

    /// Borrows the canonical one-in-flight structural lifecycle.
    pub const fn engine_selection_status(&self) -> &crate::control::EngineSelectionStatus {
        self.state.engine_selection()
    }

    /// Returns the callback-published structural status when engine selection
    /// is configured. Live observers use this read-only view; they never own
    /// or advance the coordinator.
    pub fn engine_graph_handoff_status(&self) -> Option<crate::real_time::GraphHandoffStatus> {
        self.engine_selection_runtime
            .as_ref()
            .map(|runtime| runtime.coordinator.status())
    }

    pub fn staged_graph_revision(&self) -> Option<crate::real_time::GraphRevision> {
        self.engine_selection_runtime
            .as_ref()
            .and_then(|runtime| runtime.coordinator.staged_revision())
    }

    pub fn in_flight_graph_revision(&self) -> Option<crate::real_time::GraphRevision> {
        self.engine_selection_runtime
            .as_ref()
            .and_then(|runtime| runtime.coordinator.in_flight_revision())
    }

    pub(crate) const fn state(&self) -> &AppState {
        &self.state
    }

    /// Returns an immutable snapshot of the bounded control event journal.
    pub fn event_log(&self) -> EventLog {
        self.event_log.clone()
    }

    /// Borrows the immutable journal for control-side verification without
    /// cloning retained history.
    pub const fn event_log_ref(&self) -> &EventLog {
        &self.event_log
    }

    /// Enqueues a bounded system-recovery command without inventing a state
    /// transition. Deterministic verification uses this for the renderer-wide
    /// all-notes-off command, which is distinct from Patch-scoped MIDI.
    pub(crate) fn push_recovery_command(
        &mut self,
        command: crate::real_time::audio_command::AudioCommand,
    ) -> Result<(), BoundaryFull> {
        self.boundary.push_command(command)
    }
}

/// Derives the layout-admission scope for one correlated replacement.
///
/// Delegated to the correlation's canonical derivation so publication
/// admission and the renderer's voice carry-over share one scope vocabulary.
fn replacement_scope(
    correlation: &crate::real_time::GraphPreparationCorrelation,
) -> Option<crate::real_time::GraphReplacementScope> {
    correlation.replacement_scope()
}

fn structural_preparation_failed_event(
    effect: &EngineSelectionEffect,
    target_graph_revision: GraphRevision,
    failure: EngineSelectionFailure,
) -> AppEvent {
    if effect.intent().is_occupancy() {
        return AppEvent::TopologyPreparationFailed {
            request_id: effect.request_id(),
            intent: effect.intent().clone(),
            source_graph_revision: effect.source_graph_revision(),
            target_graph_revision,
            failure,
        };
    }
    AppEvent::EnginePreparationFailed {
        request_id: effect.request_id(),
        patch_id: effect
            .patch_id()
            .expect("instrument intents carry their Patch identity"),
        intent: effect.intent().clone(),
        source_capability_id: effect
            .source_capability_id()
            .expect("instrument intents carry their source capability")
            .clone(),
        target_capability_id: effect
            .target_capability_id()
            .expect("instrument intents carry their target capability")
            .clone(),
        source_graph_revision: effect.source_graph_revision(),
        target_graph_revision,
        failure,
    }
}

fn map_structural_capability_failure(
    intent: &StructuralEditIntent,
    error: &CapabilityError,
) -> EngineSelectionFailure {
    if matches!(intent, StructuralEditIntent::ReplaceParameterChoice { .. })
        && matches!(
            error,
            CapabilityError::UnknownChoice(_)
                | CapabilityError::MissingParameter(_)
                | CapabilityError::UndeclaredParameter(_)
                | CapabilityError::WrongValueKind(_)
                | CapabilityError::StructuralParameter(_)
        )
    {
        EngineSelectionFailure::PresetUnavailable
    } else {
        map_capability_failure(error)
    }
}

fn map_capability_failure(error: &CapabilityError) -> EngineSelectionFailure {
    match error {
        CapabilityError::UnknownCapability(_) => EngineSelectionFailure::UnknownCapability,
        CapabilityError::MissingParameter(_) | CapabilityError::MissingAsset(_) => {
            EngineSelectionFailure::MissingDefault
        }
        CapabilityError::ProviderRegistryMismatch(_) => EngineSelectionFailure::ProviderMismatch,
        _ => EngineSelectionFailure::InvalidDefaultConfig,
    }
}

fn map_request_failure(error: GraphPreparationRequestError) -> EngineSelectionFailure {
    match error {
        GraphPreparationRequestError::SourceCapabilityMismatch
        | GraphPreparationRequestError::TargetCapabilityMismatch => {
            EngineSelectionFailure::ProviderMismatch
        }
        GraphPreparationRequestError::InvalidActiveConfig
        | GraphPreparationRequestError::InvalidActiveEffectConfig
        | GraphPreparationRequestError::InvalidCandidateConfig
        | GraphPreparationRequestError::InvalidOccupancy => {
            EngineSelectionFailure::InvalidDefaultConfig
        }
        GraphPreparationRequestError::UnknownEffectEntry => {
            EngineSelectionFailure::UnknownCapability
        }
        GraphPreparationRequestError::MissingRequestIdentity
        | GraphPreparationRequestError::CapabilityUnchanged
        | GraphPreparationRequestError::IntentMismatch
        | GraphPreparationRequestError::ConfigDeltaMismatch
        | GraphPreparationRequestError::TargetRevisionNotNewer
        | GraphPreparationRequestError::PatchCapacityExceeded
        | GraphPreparationRequestError::DuplicatePatchId
        | GraphPreparationRequestError::UnknownPatch => EngineSelectionFailure::GraphIncompatible,
    }
}

#[cfg(test)]
mod tests {
    use super::{AppLoop, DispatchResult};
    use crate::adapter::braids_capability::BRAIDS_CAPABILITY_ID;
    use crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID;
    use crate::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
    use crate::adapter::lock_free_structural_graph_boundary::{
        LockFreeStructuralControlHandle, LockFreeStructuralGraphBoundary,
    };
    use crate::adapter::production_instruments::{
        production_capability_registry, production_instrument_preparers,
        production_instrument_providers,
    };
    use crate::control::app_event::{AppEvent, Direction};
    use crate::control::app_state::{AppState, EventRejection};
    use crate::control::engine_selection::{
        EngineSelectionEffectKind, EngineSelectionFailure, EngineSelectionStatusKind,
    };
    use crate::control::event_record::{EmittedEvent, EventOutcome, EventSource};
    use crate::control::state_projector::StateProjector;
    use crate::control::TopLevelContext;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::mixer_state::MixerState;
    use crate::mixer::mixer_track_id::MixerTrackId;
    use crate::mixer::mixer_track_parameters::{MixerTrackParameter, MixerTrackParameters};
    use crate::mixer::patch_output::PatchOutput;
    use crate::real_time::audio_boundary::{BoundaryFull, ControlAudioBoundary};
    use crate::real_time::audio_command::AudioCommand;
    use crate::real_time::audio_renderer::AudioRenderer;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;
    use crate::real_time::{
        AudioBoundary, ControlStructuralGraphBoundary, GraphHandoffStatus, GraphRevision,
        GraphStageOutcome, NoStructuralGraphChanges, PreparedGraph, PreparedGraphBuilder,
        StructuralBoundaryFull, StructuralGraphBoundary,
    };
    use crate::shell::audio_output::{AudioDeviceConfig, AudioSampleFormat};
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::{
        CapabilityId, DescriptorDefaultConfigFactory, VoiceEnvelope, VoiceEnvelopeParameter,
    };
    use crate::testing::automatic_midi_test::create_soundfont_config;
    use crate::testing::DeterministicGraphPreparationWorker;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Debug, Default, PartialEq)]
    struct BoundaryObservations {
        parameters: Vec<ParameterSnapshot>,
        commands: Vec<AudioCommand>,
        order: Vec<&'static str>,
        reject_commands: bool,
    }

    #[derive(Clone, Debug)]
    struct TestBoundary {
        observations: Arc<Mutex<BoundaryObservations>>,
    }

    impl TestBoundary {
        fn new(observations: Arc<Mutex<BoundaryObservations>>) -> Self {
            Self { observations }
        }
    }

    impl ControlAudioBoundary for TestBoundary {
        fn push_command(&mut self, command: AudioCommand) -> Result<(), BoundaryFull> {
            let mut observations = self.observations.lock().unwrap();
            observations.order.push("command");
            if observations.reject_commands {
                Err(BoundaryFull::new(command))
            } else {
                observations.commands.push(command);
                Ok(())
            }
        }

        fn publish_parameters(&mut self, parameters: ParameterSnapshot) {
            let mut observations = self.observations.lock().unwrap();
            observations.order.push("parameters");
            observations.parameters.push(parameters);
        }
    }

    struct ObservedStructuralControl {
        inner: LockFreeStructuralControlHandle,
        blocked: Arc<AtomicBool>,
        attempted_parameters: Arc<Mutex<Vec<ParameterSnapshot>>>,
    }

    impl ControlStructuralGraphBoundary for ObservedStructuralControl {
        fn publish_prepared_on_control(
            &mut self,
            graph: PreparedGraph,
        ) -> Result<(), StructuralBoundaryFull> {
            self.attempted_parameters
                .lock()
                .unwrap()
                .push(*graph.initial_parameters());
            if self.blocked.load(Ordering::SeqCst) {
                Err(StructuralBoundaryFull::new(graph))
            } else {
                self.inner.publish_prepared_on_control(graph)
            }
        }

        fn collect_retired_on_control(&mut self) -> Option<GraphRevision> {
            self.inner.collect_retired_on_control()
        }

        fn read_status_on_control(&self) -> GraphHandoffStatus {
            self.inner.read_status_on_control()
        }
    }

    fn global_parameters() -> GlobalParameters {
        GlobalParameters::new(0.0).unwrap()
    }

    fn patch(id: u32, _gain_db: f32) -> Patch {
        let provider =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            create_soundfont_config(
                &provider,
                SoundFontInstrument::new(0, (id - 1) as u8, false).unwrap(),
            )
            .unwrap(),
            MidiChannel::new(((id - 1) % 16) as u8).unwrap(),
            PatchOutput::to_track(MixerTrackId::new(((id - 1) % 16) as u8).unwrap()),
        )
    }

    fn installed_state_with_gains(gains: &[f32]) -> AppState {
        let provider =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        let mut mixer = MixerState::default();
        for (index, gain_db) in gains.iter().copied().enumerate() {
            mixer.set_track(
                MixerTrackId::new(index as u8).unwrap(),
                MixerTrackParameters::default()
                    .with_scalar_value(MixerTrackParameter::Level, gain_db)
                    .unwrap(),
            );
        }
        let mut state = AppState::new(provider.registry().unwrap(), global_parameters())
            .with_initial_mixer(mixer);
        let patches = gains
            .iter()
            .enumerate()
            .map(|(index, gain_db)| patch(index as u32 + 1, *gain_db))
            .collect();
        state.apply(AppEvent::InstallPatches(patches)).unwrap();
        state
    }

    fn installed_state() -> AppState {
        installed_state_with_gains(&[0.0])
    }

    fn loop_with_state(
        state: AppState,
    ) -> (AppLoop<TestBoundary>, Arc<Mutex<BoundaryObservations>>) {
        let observations = Arc::new(Mutex::new(BoundaryObservations::default()));
        let app_loop = AppLoop::new(
            state,
            StateProjector::new(),
            TestBoundary::new(Arc::clone(&observations)),
        )
        .unwrap();
        (app_loop, observations)
    }

    fn loop_with_observations() -> (AppLoop<TestBoundary>, Arc<Mutex<BoundaryObservations>>) {
        loop_with_state(installed_state())
    }

    #[test]
    fn one_way_control_loop_publishes_one_coherent_edit() {
        let (mut app_loop, observations) = loop_with_observations();
        let initial_text = app_loop.current_text();
        let initial_shell = app_loop.current_graphical_shell();

        let result = app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        let current_text = app_loop.current_text();
        let current_shell = app_loop.current_graphical_shell();
        let observations = observations.lock().unwrap();
        let published = observations.parameters.last().unwrap();

        assert_eq!(result.accepted().generation(), 2);
        assert_eq!(published.generation(), result.accepted().generation());
        assert_eq!(
            published.mixer_track(MixerTrackId::default()).level_db(),
            1.0
        );
        assert!(result.snapshot().json().contains("\"levelDb\":1.0"));
        assert_eq!(current_text.state_hash(), result.snapshot().hash());
        assert_ne!(current_text, initial_text);
        assert_ne!(current_shell, initial_shell);
        assert_eq!(current_shell.generation(), result.accepted().generation());
        assert_eq!(current_shell.state_hash(), result.snapshot().hash());
        assert_eq!(current_shell.workspace().diagnostic(), &current_text);
        assert!(current_text.body().contains("> levelDb=1"));
        assert!(observations.commands.is_empty());
        assert!(result.audio_effects_published());
    }

    #[test]
    fn context_dispatch_projects_page_and_publishes_same_values_without_audio_command() {
        let (mut app_loop, observations) = loop_with_observations();
        app_loop
            .dispatch(AppEvent::Navigate(Direction::Down))
            .unwrap();
        let retained_selection = app_loop.state().selection();
        let before_parameters = *app_loop.current_parameters();
        let before_patches = app_loop.patches().to_vec();
        let before_global = *app_loop.state().global();
        let command_count = observations.lock().unwrap().commands.len();

        let result = app_loop
            .dispatch_from(
                AppEvent::SelectContext(crate::control::TopLevelContext::Patch),
                EventSource::Keyboard,
            )
            .unwrap();
        let page = app_loop.current_patch_page().unwrap();
        let text = app_loop.current_text();
        let shell = app_loop.current_graphical_shell();
        let tree: serde_json::Value =
            serde_json::from_str(app_loop.current_state_tree().json()).unwrap();
        let after_parameters = *app_loop.current_parameters();

        assert_eq!(page.patch().id(), PatchId::new(1).unwrap());
        assert_eq!(page.state_hash(), result.snapshot().hash());
        assert_eq!(text.context(), crate::control::TopLevelContext::Patch);
        assert_eq!(text.state_hash(), result.snapshot().hash());
        assert_eq!(shell.context(), crate::control::TopLevelContext::Patch);
        assert_eq!(shell.generation(), result.accepted().generation());
        assert_eq!(shell.state_hash(), result.snapshot().hash());
        assert_eq!(shell.workspace().diagnostic(), &text);
        assert_eq!(tree["interaction"]["activeFocus"]["context"], "patch");
        assert_eq!(tree["patchPage"]["stateHash"], result.snapshot().hash());
        assert_eq!(
            tree["graphicalShell"],
            serde_json::to_value(&shell).unwrap()
        );
        assert_eq!(before_patches, app_loop.patches());
        assert_eq!(before_global, *app_loop.state().global());
        assert_eq!(retained_selection, app_loop.state().selection());
        assert_eq!(
            before_parameters.graph_revision(),
            after_parameters.graph_revision()
        );
        assert_eq!(before_parameters.patches(), after_parameters.patches());
        assert_eq!(before_parameters.global(), after_parameters.global());
        assert_eq!(observations.lock().unwrap().commands.len(), command_count);

        let rejected_tree = app_loop.current_state_tree();
        let rejected_shell = app_loop.current_graphical_shell();
        assert_eq!(
            app_loop.dispatch(AppEvent::Adjust(Direction::Up)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(app_loop.current_state_tree(), rejected_tree);
        assert_eq!(app_loop.current_graphical_shell(), rejected_shell);
        app_loop
            .dispatch(AppEvent::SelectContext(
                crate::control::TopLevelContext::Mixer,
            ))
            .unwrap();
        assert!(app_loop.current_patch_page().is_none());
        assert_eq!(
            app_loop.current_text().context(),
            crate::control::TopLevelContext::Mixer
        );
        assert_eq!(app_loop.state().selection(), retained_selection);
    }

    #[test]
    fn ready_and_recoverable_failed_patch_adsr_edits_publish_source_revision_to_renderer() {
        let registry = production_capability_registry().unwrap();
        let provider =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        let patch_id = PatchId::new(1).unwrap();
        let patch = Patch::new(
            patch_id,
            "Envelope lifecycle".to_owned(),
            create_soundfont_config(&provider, SoundFontInstrument::new(0, 8, false).unwrap())
                .unwrap(),
            MidiChannel::new(0).unwrap(),
            PatchOutput::default(),
        )
        .with_envelope(VoiceEnvelope::new(500.0, 600.0, 0.5, 700.0).unwrap());
        let mut state = AppState::for_graph(
            registry.clone(),
            global_parameters(),
            GraphRevision::INITIAL,
        );
        state.apply(AppEvent::InstallPatches(vec![patch])).unwrap();

        let initial_transport =
            ParameterSnapshot::new(0, global_parameters(), MixerState::default(), &[]).unwrap();
        let boundary = LockFreeAudioBoundary::new(64, initial_transport);
        let (audio_control, audio_handle) = boundary.into_handles();
        let mut app_loop = AppLoop::new(
            state,
            StateProjector::for_graph(GraphRevision::INITIAL),
            audio_control,
        )
        .unwrap();
        let graph =
            PreparedGraphBuilder::new(&registry, &production_instrument_preparers().unwrap())
                .build(
                    GraphRevision::INITIAL,
                    app_loop.patches(),
                    *app_loop.current_parameters(),
                    48_000.0,
                    512,
                )
                .unwrap();
        let mut renderer = AudioRenderer::new(audio_handle, NoStructuralGraphChanges::new(), graph);
        let mut output = [0.0_f32; 1_024];

        app_loop
            .dispatch(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        app_loop
            .dispatch(AppEvent::Navigate(Direction::Down))
            .unwrap();
        app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        let ready_edit = app_loop.event_log_ref().records().last().unwrap();
        assert_eq!(
            ready_edit.emitted_events(),
            &[
                EmittedEvent::StateAccepted {
                    generation: ready_edit.generation_after()
                },
                EmittedEvent::ParameterSnapshotPublished {
                    generation: ready_edit.generation_after(),
                    graph_revision: GraphRevision::INITIAL
                }
            ]
        );
        assert_eq!(
            app_loop
                .current_parameters()
                .patch(patch_id)
                .unwrap()
                .envelope()
                .attack_milliseconds(),
            501.0
        );
        let note = MidiMessage::try_new(
            MidiChannel::new(0).unwrap(),
            MidiMessageKind::NoteOn,
            60,
            110,
        )
        .unwrap();
        app_loop
            .dispatch(AppEvent::Midi {
                patch_id,
                message: note,
            })
            .unwrap();
        renderer.render(&mut output);
        assert_eq!(renderer.active_revision(), GraphRevision::INITIAL);
        assert_eq!(
            renderer
                .parameters()
                .patch(patch_id)
                .unwrap()
                .envelope()
                .attack_milliseconds(),
            501.0
        );
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().any(|sample| sample.abs() > f32::EPSILON));

        app_loop
            .dispatch(AppEvent::Navigate(Direction::Up))
            .unwrap();
        app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        let correlation = app_loop
            .state()
            .engine_selection()
            .correlation()
            .unwrap()
            .clone();
        let failure_focus = app_loop.state().interaction().patch_control_focus();
        let failure_envelope = *app_loop.patches()[0].envelope();
        app_loop
            .dispatch_from(
                AppEvent::EnginePreparationFailed {
                    request_id: correlation.request_id(),
                    patch_id: correlation.patch_id().unwrap(),
                    intent: correlation.intent().clone(),
                    source_capability_id: correlation.source_capability_id().unwrap().clone(),
                    target_capability_id: correlation.target_capability_id().unwrap().clone(),
                    source_graph_revision: correlation.source_graph_revision(),
                    target_graph_revision: GraphRevision::new(2).unwrap(),
                    failure: EngineSelectionFailure::AssetUnavailable,
                },
                EventSource::Worker,
            )
            .unwrap();
        assert_eq!(
            app_loop.state().engine_selection().kind(),
            EngineSelectionStatusKind::Failed
        );
        assert_eq!(
            app_loop.state().interaction().patch_control_focus(),
            failure_focus
        );
        assert_eq!(*app_loop.patches()[0].envelope(), failure_envelope);
        assert_eq!(
            app_loop.current_patch_page().unwrap().focused_control_id(),
            crate::control::PatchControlId::Engine
        );

        app_loop
            .dispatch(AppEvent::Navigate(Direction::Down))
            .unwrap();
        app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        let failed_edit = app_loop.event_log_ref().records().last().unwrap();
        assert!(failed_edit
            .emitted_events()
            .iter()
            .all(|event| !matches!(event, EmittedEvent::EngineSelection { .. })));
        assert_eq!(
            app_loop.current_parameters().graph_revision(),
            GraphRevision::INITIAL
        );
        assert_eq!(
            app_loop
                .current_parameters()
                .patch(patch_id)
                .unwrap()
                .envelope()
                .attack_milliseconds(),
            502.0
        );
        renderer.render(&mut output);
        assert_eq!(renderer.active_revision(), GraphRevision::INITIAL);
        assert_eq!(
            renderer
                .parameters()
                .patch(patch_id)
                .unwrap()
                .envelope()
                .attack_milliseconds(),
            502.0
        );
        assert!(output.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn one_way_control_loop_edits_only_the_selected_non_first_patch() {
        let (mut app_loop, observations) =
            loop_with_state(installed_state_with_gains(&[0.0, -12.0]));

        app_loop
            .dispatch(AppEvent::Navigate(Direction::Right))
            .unwrap();
        let result = app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        let observations = observations.lock().unwrap();
        let published = observations.parameters.last().unwrap();

        assert_eq!(
            published.mixer_track(MixerTrackId::default()).level_db(),
            0.0
        );
        assert_eq!(
            published
                .mixer_track(MixerTrackId::new(1).unwrap())
                .level_db(),
            -11.0
        );
        assert!(result.snapshot().json().contains("\"levelDb\":-11.0"));
    }

    #[test]
    fn one_way_control_loop_publishes_parameters_before_midi_command() {
        let (mut app_loop, observations) = loop_with_observations();
        let message = MidiMessage::try_new(
            MidiChannel::new(0).unwrap(),
            MidiMessageKind::NoteOn,
            60,
            100,
        )
        .unwrap();
        let command = AudioCommand::patch_midi(PatchId::new(1).unwrap(), message);

        let result = app_loop
            .dispatch(AppEvent::Midi {
                patch_id: PatchId::new(1).unwrap(),
                message,
            })
            .unwrap();
        let observations = observations.lock().unwrap();

        assert_eq!(
            &observations.order[observations.order.len() - 2..],
            &["parameters", "command"]
        );
        assert_eq!(observations.commands.last(), Some(&command));
        assert!(result.audio_effects_published());
    }

    #[test]
    fn one_way_control_loop_rejection_has_no_effects_or_view_change() {
        let (mut app_loop, observations) = loop_with_observations();
        let initial_text = app_loop.current_text();
        let initial_shell = app_loop.current_graphical_shell();
        let initial_observations = observations.lock().unwrap().clone();

        let result = app_loop.dispatch(AppEvent::InstallPatches(Vec::new()));

        assert_eq!(result, Err(EventRejection::InstallationClosed));
        assert_eq!(app_loop.current_text(), initial_text);
        assert_eq!(app_loop.current_graphical_shell(), initial_shell);
        assert_eq!(*observations.lock().unwrap(), initial_observations);
        assert_eq!(app_loop.event_log().len(), 1);
    }

    #[test]
    fn one_way_control_loop_boundary_rejection_is_nonfatal() {
        let (mut app_loop, observations) =
            loop_with_state(installed_state_with_gains(&[MixerTrackParameter::Level
                .descriptor()
                .maximum()]));
        let initial_observations = observations.lock().unwrap().clone();

        assert_eq!(
            app_loop.dispatch(AppEvent::Adjust(Direction::Right)),
            Err(EventRejection::ParameterAtBoundary)
        );
        assert_eq!(*observations.lock().unwrap(), initial_observations);

        let result = app_loop
            .dispatch(AppEvent::Adjust(Direction::Left))
            .unwrap();
        let log = app_loop.event_log();

        assert_eq!(result.accepted().generation(), 2);
        assert_eq!(
            observations
                .lock()
                .unwrap()
                .parameters
                .last()
                .unwrap()
                .mixer_track(MixerTrackId::default())
                .level_db(),
            5.0
        );
        assert_eq!(log.len(), 2);
        assert_eq!(log.records()[0].outcome(), EventOutcome::Rejected);
        assert_eq!(log.records()[1].outcome(), EventOutcome::Accepted);
        assert_eq!(
            log.records()[0].state_hash_after(),
            log.records()[1].state_hash_before()
        );
    }

    #[test]
    fn one_way_control_loop_reports_queue_saturation_after_acceptance() {
        let (mut app_loop, observations) = loop_with_observations();
        observations.lock().unwrap().reject_commands = true;
        let message = MidiMessage::try_new(
            MidiChannel::new(0).unwrap(),
            MidiMessageKind::NoteOn,
            64,
            90,
        )
        .unwrap();
        let command = AudioCommand::patch_midi(PatchId::new(1).unwrap(), message);

        let result: DispatchResult = app_loop
            .dispatch(AppEvent::Midi {
                patch_id: PatchId::new(1).unwrap(),
                message,
            })
            .unwrap();

        assert_eq!(result.boundary_full(), Some(BoundaryFull::new(command)));
        assert!(!result.audio_effects_published());
        assert_eq!(
            observations
                .lock()
                .unwrap()
                .parameters
                .last()
                .unwrap()
                .generation(),
            result.accepted().generation()
        );
    }

    #[test]
    fn control_observation_trace_records_exact_sources_hashes_and_generations() {
        let (mut app_loop, _) = loop_with_observations();
        let initial_tree = app_loop.current_state_tree();

        assert_eq!(
            app_loop.dispatch_from(AppEvent::InstallPatches(Vec::new()), EventSource::DemoScene,),
            Err(EventRejection::InstallationClosed)
        );
        let accepted = app_loop
            .dispatch_from(AppEvent::Adjust(Direction::Right), EventSource::Keyboard)
            .unwrap();

        let log = app_loop.event_log();
        let tree = app_loop.current_state_tree();
        assert_eq!(log.records().len(), 2);
        assert_eq!(log.records()[0].sequence(), 0);
        assert_eq!(log.records()[0].source(), EventSource::DemoScene);
        assert_eq!(log.records()[0].outcome(), EventOutcome::Rejected);
        assert_eq!(
            log.records()[0].generation_before(),
            initial_tree.generation()
        );
        assert_eq!(
            log.records()[0].generation_after(),
            initial_tree.generation()
        );
        assert_eq!(
            log.records()[0].state_hash_before(),
            log.records()[0].state_hash_after()
        );
        assert_eq!(log.records()[1].sequence(), 1);
        assert_eq!(log.records()[1].source(), EventSource::Keyboard);
        assert_eq!(log.records()[1].outcome(), EventOutcome::Accepted);
        assert_eq!(
            log.records()[0].generation_after(),
            log.records()[1].generation_before()
        );
        assert_eq!(
            log.records()[0].state_hash_after(),
            log.records()[1].state_hash_before()
        );
        assert_eq!(tree.generation(), accepted.accepted().generation());
        assert_eq!(tree.state_hash(), accepted.snapshot().hash());
        assert_eq!(
            log.records()[1].parameter_generation(),
            accepted.accepted().generation()
        );
        assert_eq!(
            log.records()[1].projection_state_hash(),
            accepted.snapshot().hash()
        );

        for property in [
            "\"patches\"",
            "\"global\"",
            "\"interaction\"",
            "\"patchPage\"",
            "\"projection\"",
            "\"parameters\"",
        ] {
            assert!(tree.json().contains(property));
        }
    }

    #[test]
    fn patch_adsr_coexists_with_preparing_staged_activation_and_latest_target_snapshot() {
        let registry = production_capability_registry().unwrap();
        let config_factory = DescriptorDefaultConfigFactory::new(
            registry.clone(),
            production_instrument_providers().unwrap(),
        );
        let patch_id = PatchId::new(1).unwrap();
        let soundfont_config = config_factory
            .create(&CapabilityId::new(HIDEF_CAPABILITY_ID).unwrap())
            .unwrap();
        let mut state = AppState::for_graph(
            registry.clone(),
            global_parameters(),
            GraphRevision::INITIAL,
        );
        state
            .apply(AppEvent::InstallPatches(vec![Patch::new(
                patch_id,
                "Lifecycle envelope".to_owned(),
                soundfont_config,
                MidiChannel::new(0).unwrap(),
                PatchOutput::default(),
            )
            .with_envelope(
                VoiceEnvelope::new(500.0, 600.0, 0.5, 700.0).unwrap(),
            )]))
            .unwrap();

        let initial_transport =
            ParameterSnapshot::new(0, global_parameters(), MixerState::default(), &[]).unwrap();
        let audio_boundary = LockFreeAudioBoundary::new(64, initial_transport);
        let (audio_control, audio_handle) = audio_boundary.into_handles();
        let mut app_loop = AppLoop::new(
            state,
            StateProjector::for_graph(GraphRevision::INITIAL),
            audio_control,
        )
        .unwrap();
        let audio_config =
            AudioDeviceConfig::new(48_000.0, 2, AudioSampleFormat::F32, 512).unwrap();
        let initial_graph =
            PreparedGraphBuilder::new(&registry, &production_instrument_preparers().unwrap())
                .build(
                    GraphRevision::INITIAL,
                    app_loop.patches(),
                    *app_loop.current_parameters(),
                    audio_config.sample_rate(),
                    audio_config.render_capacity_frames(),
                )
                .unwrap();
        let structural = LockFreeStructuralGraphBoundary::new(
            1,
            1,
            GraphHandoffStatus::with_active(GraphRevision::INITIAL),
        )
        .unwrap();
        let (structural_control, structural_audio) = structural.into_handles();
        let blocked = Arc::new(AtomicBool::new(true));
        let attempted_parameters = Arc::new(Mutex::new(Vec::new()));
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
                ObservedStructuralControl {
                    inner: structural_control,
                    blocked: Arc::clone(&blocked),
                    attempted_parameters: Arc::clone(&attempted_parameters),
                },
                &initial_graph,
                audio_config,
            )
            .unwrap();
        let mut renderer = AudioRenderer::new(audio_handle, structural_audio, initial_graph);
        let mut output = [0.0_f32; 1_024];
        let note = MidiMessage::try_new(
            MidiChannel::new(0).unwrap(),
            MidiMessageKind::NoteOn,
            60,
            110,
        )
        .unwrap();

        app_loop
            .dispatch(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        assert_eq!(
            app_loop.state().engine_selection().kind(),
            EngineSelectionStatusKind::Preparing
        );
        app_loop
            .dispatch(AppEvent::Navigate(Direction::Down))
            .unwrap();
        app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        assert_eq!(
            app_loop.current_patch_page().unwrap().focused_control_id(),
            crate::control::PatchControlId::Envelope(VoiceEnvelopeParameter::AttackMilliseconds)
        );
        assert_eq!(
            app_loop.current_parameters().graph_revision(),
            GraphRevision::INITIAL
        );
        assert_eq!(
            app_loop
                .current_parameters()
                .patch(patch_id)
                .unwrap()
                .envelope()
                .attack_milliseconds(),
            501.0
        );

        app_loop
            .dispatch(AppEvent::Navigate(Direction::Up))
            .unwrap();
        let busy_state = app_loop.current_state_tree();
        assert_eq!(
            app_loop.dispatch(AppEvent::Adjust(Direction::Right)),
            Err(EventRejection::StructuralEditBusy)
        );
        assert_eq!(app_loop.current_state_tree(), busy_state);
        app_loop
            .dispatch(AppEvent::Navigate(Direction::Down))
            .unwrap();

        app_loop
            .dispatch(AppEvent::Midi {
                patch_id,
                message: note,
            })
            .unwrap();
        renderer.render(&mut output);
        assert_eq!(renderer.active_revision(), GraphRevision::INITIAL);
        assert_eq!(
            renderer
                .parameters()
                .patch(patch_id)
                .unwrap()
                .envelope()
                .attack_milliseconds(),
            501.0
        );
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().any(|sample| sample.abs() > f32::EPSILON));

        assert!(worker_handle.advance());
        let prepared = app_loop.advance_structural().unwrap();
        let target_revision = GraphRevision::new(2).unwrap();
        assert!(prepared.worker_result_polled());
        assert_eq!(prepared.graph_stage(), Some(GraphStageOutcome::Staged));
        assert_eq!(prepared.graph_published(), None);
        assert_eq!(app_loop.staged_graph_revision(), Some(target_revision));
        assert_eq!(app_loop.in_flight_graph_revision(), None);
        assert_eq!(
            app_loop.state().engine_selection().kind(),
            EngineSelectionStatusKind::Activating
        );
        assert_eq!(
            app_loop.current_parameters().graph_revision(),
            target_revision
        );
        assert_eq!(
            app_loop.current_patch_page().unwrap().focused_control_id(),
            crate::control::PatchControlId::Envelope(VoiceEnvelopeParameter::AttackMilliseconds)
        );
        let first_attempt = attempted_parameters.lock().unwrap()[0];
        assert_eq!(first_attempt.graph_revision(), target_revision);
        assert_eq!(
            first_attempt
                .patch(patch_id)
                .unwrap()
                .envelope()
                .attack_milliseconds(),
            501.0,
            "candidate activation fallback refreshes from the latest Preparing edit"
        );

        let activating_edit = app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        assert_eq!(activating_edit.boundary_full(), None);
        assert_eq!(
            app_loop
                .current_parameters()
                .patch(patch_id)
                .unwrap()
                .envelope()
                .attack_milliseconds(),
            502.0
        );
        assert_eq!(
            app_loop.current_parameters().graph_revision(),
            target_revision
        );
        assert!(app_loop
            .event_log_ref()
            .records()
            .last()
            .unwrap()
            .emitted_events()
            .iter()
            .all(|event| !matches!(event, EmittedEvent::EngineSelection { .. })));

        app_loop
            .dispatch(AppEvent::Navigate(Direction::Up))
            .unwrap();
        assert_eq!(
            app_loop.dispatch(AppEvent::Adjust(Direction::Left)),
            Err(EventRejection::StructuralEditBusy)
        );
        app_loop
            .dispatch(AppEvent::Navigate(Direction::Down))
            .unwrap();

        output.fill(0.0);
        renderer.render(&mut output);
        assert_eq!(renderer.active_revision(), GraphRevision::INITIAL);
        assert_eq!(
            renderer
                .parameters()
                .patch(patch_id)
                .unwrap()
                .envelope()
                .attack_milliseconds(),
            501.0,
            "the source holds its last compatible source-revision snapshot"
        );
        assert!(renderer.handoff_status().incompatible_snapshots() > 0);
        assert!(output.iter().all(|sample| sample.is_finite()));

        blocked.store(false, Ordering::SeqCst);
        let published = app_loop.advance_structural().unwrap();
        assert_eq!(published.graph_published(), Some(target_revision));
        assert_eq!(app_loop.staged_graph_revision(), None);
        assert_eq!(app_loop.in_flight_graph_revision(), Some(target_revision));
        let attempts = attempted_parameters.lock().unwrap();
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0], attempts[1]);
        drop(attempts);

        app_loop
            .dispatch(AppEvent::Midi {
                patch_id,
                message: note,
            })
            .unwrap();
        output.fill(0.0);
        renderer.render(&mut output);
        assert_eq!(renderer.active_revision(), target_revision);
        assert_eq!(
            renderer
                .parameters()
                .patch(patch_id)
                .unwrap()
                .envelope()
                .attack_milliseconds(),
            502.0,
            "the activated graph consumes the latest target-revision edit"
        );
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().any(|sample| sample.abs() > f32::EPSILON));

        let acknowledged = app_loop.advance_structural().unwrap();
        assert_eq!(
            acknowledged.activation_acknowledged(),
            Some(target_revision)
        );
        assert_eq!(
            app_loop.state().engine_selection().kind(),
            EngineSelectionStatusKind::Ready
        );
        assert_eq!(
            app_loop.current_patch_page().unwrap().focused_control_id(),
            crate::control::PatchControlId::Envelope(VoiceEnvelopeParameter::AttackMilliseconds)
        );
        assert_eq!(
            app_loop.patches()[0].envelope().attack_milliseconds(),
            502.0
        );

        drop(renderer);
        app_loop.shutdown_engine_selection_on_control().unwrap();
    }

    #[test]
    fn app_loop_coordinates_real_bidirectional_graph_work_and_recoverable_failure() {
        let registry = production_capability_registry().unwrap();
        let config_factory = DescriptorDefaultConfigFactory::new(
            registry.clone(),
            production_instrument_providers().unwrap(),
        );
        let patch_id = PatchId::new(1).unwrap();
        let soundfont_config = config_factory
            .create(&CapabilityId::new(HIDEF_CAPABILITY_ID).unwrap())
            .unwrap();
        let mut state = AppState::for_graph(
            registry.clone(),
            global_parameters(),
            GraphRevision::INITIAL,
        );
        state
            .apply(AppEvent::InstallPatches(vec![Patch::new(
                patch_id,
                "Orchestrated Patch".to_owned(),
                soundfont_config.clone(),
                MidiChannel::new(0).unwrap(),
                PatchOutput::default(),
            )]))
            .unwrap();

        let initial_transport =
            ParameterSnapshot::new(0, global_parameters(), MixerState::default(), &[]).unwrap();
        let audio_boundary = LockFreeAudioBoundary::new(64, initial_transport);
        let (audio_control, audio_handle) = audio_boundary.into_handles();
        let mut app_loop = AppLoop::new(
            state,
            StateProjector::for_graph(GraphRevision::INITIAL),
            audio_control,
        )
        .unwrap();
        let audio_config =
            AudioDeviceConfig::new(48_000.0, 2, AudioSampleFormat::F32, 512).unwrap();
        let initial_preparers = production_instrument_preparers().unwrap();
        let initial_graph = PreparedGraphBuilder::new(&registry, &initial_preparers)
            .build(
                GraphRevision::INITIAL,
                app_loop.patches(),
                *app_loop.current_parameters(),
                audio_config.sample_rate(),
                audio_config.render_capacity_frames(),
            )
            .unwrap();
        let structural = LockFreeStructuralGraphBoundary::new(
            1,
            1,
            GraphHandoffStatus::with_active(GraphRevision::INITIAL),
        )
        .unwrap();
        let (structural_control, structural_audio) = structural.into_handles();
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
                &initial_graph,
                audio_config,
            )
            .unwrap();
        let mut renderer = AudioRenderer::new(audio_handle, structural_audio, initial_graph);
        let mut output = [0.0; 1_024];

        app_loop
            .dispatch_from(
                AppEvent::SelectContext(TopLevelContext::Patch),
                EventSource::Keyboard,
            )
            .unwrap();
        app_loop
            .dispatch_from(AppEvent::Adjust(Direction::Right), EventSource::Keyboard)
            .unwrap();
        assert_eq!(
            app_loop.state().engine_selection().kind(),
            EngineSelectionStatusKind::Preparing
        );
        assert!(worker_handle.is_pending());
        assert_eq!(
            app_loop.dispatch_from(AppEvent::Adjust(Direction::Right), EventSource::Keyboard),
            Err(EventRejection::StructuralEditBusy)
        );

        app_loop
            .dispatch(AppEvent::SelectContext(TopLevelContext::Mixer))
            .unwrap();
        app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        let edited_gain = app_loop
            .state()
            .mixer()
            .track(MixerTrackId::default())
            .level_db();
        let note = MidiMessage::try_new(
            MidiChannel::new(0).unwrap(),
            MidiMessageKind::NoteOn,
            60,
            100,
        )
        .unwrap();
        app_loop
            .dispatch(AppEvent::Midi {
                patch_id,
                message: note,
            })
            .unwrap();

        assert!(worker_handle.advance());
        let prepared = app_loop.advance_structural().unwrap();
        assert!(prepared.worker_result_polled());
        assert_eq!(prepared.graph_stage(), Some(GraphStageOutcome::Staged));
        assert_eq!(
            prepared.graph_published(),
            Some(GraphRevision::new(2).unwrap())
        );
        assert_eq!(
            app_loop.state().engine_selection().kind(),
            EngineSelectionStatusKind::Activating
        );
        assert_eq!(app_loop.graph_revision(), GraphRevision::new(2).unwrap());
        assert_eq!(
            app_loop
                .current_parameters()
                .mixer_track(MixerTrackId::default())
                .level_db(),
            edited_gain
        );
        renderer.render(&mut output);
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().any(|sample| sample.abs() > f32::EPSILON));
        let acknowledged = app_loop.advance_structural().unwrap();
        assert_eq!(
            acknowledged.activation_acknowledged(),
            Some(GraphRevision::new(2).unwrap())
        );
        assert_eq!(
            app_loop.state().engine_selection().kind(),
            EngineSelectionStatusKind::Ready
        );
        assert_eq!(
            app_loop.patches()[0]
                .instrument_config()
                .capability_id()
                .as_str(),
            BRAIDS_CAPABILITY_ID
        );

        app_loop
            .dispatch(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        app_loop
            .dispatch(AppEvent::Adjust(Direction::Left))
            .unwrap();
        assert!(worker_handle.advance());
        let reverse = app_loop.advance_structural().unwrap();
        assert_eq!(reverse.graph_stage(), Some(GraphStageOutcome::Staged));
        assert_eq!(
            reverse.graph_published(),
            Some(GraphRevision::new(3).unwrap())
        );
        app_loop
            .dispatch(AppEvent::Midi {
                patch_id,
                message: note,
            })
            .unwrap();
        renderer.render(&mut output);
        assert_eq!(
            app_loop
                .advance_structural()
                .unwrap()
                .activation_acknowledged(),
            Some(GraphRevision::new(3).unwrap())
        );
        assert_eq!(app_loop.patches()[0].instrument_config(), &soundfont_config);

        app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        worker_handle.fail_next(EngineSelectionFailure::AssetUnavailable);
        assert!(worker_handle.advance());
        let failed = app_loop.advance_structural().unwrap();
        assert!(failed.failure_dispatched());
        assert_eq!(
            app_loop.state().engine_selection().kind(),
            EngineSelectionStatusKind::Failed
        );
        assert_eq!(
            app_loop.state().engine_selection().failure(),
            Some(EngineSelectionFailure::AssetUnavailable)
        );
        assert_eq!(app_loop.graph_revision(), GraphRevision::new(3).unwrap());
        assert_eq!(app_loop.patches()[0].instrument_config(), &soundfont_config);

        app_loop
            .dispatch(AppEvent::Adjust(Direction::Right))
            .unwrap();
        assert!(worker_handle.advance());
        app_loop.advance_structural().unwrap();
        renderer.render(&mut output);
        app_loop.advance_structural().unwrap();
        assert_eq!(
            app_loop.state().engine_selection().kind(),
            EngineSelectionStatusKind::Ready
        );

        let effects = app_loop
            .event_log_ref()
            .records()
            .iter()
            .flat_map(|record| record.emitted_events())
            .filter_map(|event| match event {
                EmittedEvent::EngineSelection { effect } => Some(effect.kind()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(effects.contains(&EngineSelectionEffectKind::PrepareRequested));
        assert!(effects.contains(&EngineSelectionEffectKind::CandidateCommitted));
        assert!(effects.contains(&EngineSelectionEffectKind::GraphPublished));
        assert!(effects.contains(&EngineSelectionEffectKind::ActivationAcknowledged));

        drop(renderer);
        app_loop.shutdown_engine_selection_on_control().unwrap();
    }
}
