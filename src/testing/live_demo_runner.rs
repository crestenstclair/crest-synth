use crate::control::app_event::AppEvent;
use crate::control::app_loop::AppLoop;
use crate::control::app_state::EventRejection;
use crate::control::event_record::{
    EmittedEvent, EventInput, EventOutcome, EventRecord, EventSource,
};
use crate::control::state_tree::StateTree;
use crate::control::text_projection::TextProjection;
use crate::control::{
    Direction, EngineSelectionEffectKind, EngineSelectionRequestId, EngineSelectionStatusKind,
    StructuralEditIntent, TopLevelContext,
};
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::real_time::audio_boundary::ControlAudioBoundary;
use crate::real_time::audio_observation::ControlAudioObservation;
use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
use crate::synth::{
    InstrumentConfig, ParameterDefault, ParameterKind, ParameterValue, VoiceEnvelope,
};
use crate::testing::automatic_midi_test::{AutomaticMidiTest, TestInputError};
use crate::testing::live_demo_checkpoint::{
    LiveCheckpoint, LiveDemoCheckpoint, LiveDemoCheckpointError, LiveEngineCheckpoint,
    LivePresetProjection,
};
use crate::testing::live_demo_report::{
    LiveDemoCoverage, LiveDemoReport, LiveDemoReportError, RuntimeAudioWitness,
};
use crate::testing::live_demo_scene::{
    selected_parameter_value, LiveDemoScene, LiveDemoSceneError, LiveDemoStep,
    LiveEngineTransition, LiveExpectedTransition,
};
use crate::testing::midi_event_source::MidiEventSource;
use core::fmt;
use std::time::Duration;

/// Maximum elapsed control time without a qualifying autonomous milestone.
pub const LIVE_DEMO_NO_PROGRESS_TIMEOUT: Duration = Duration::from_secs(10);
/// Maximum elapsed control time for the complete autonomous live scene.
pub const LIVE_DEMO_TOTAL_TIMEOUT: Duration = Duration::from_secs(120);

/// Control-thread state machine for the real-window live scene.
pub struct LiveDemoRunner<Source, Observation> {
    automatic_midi: AutomaticMidiTest<Source>,
    observation: Observation,
    scene: LiveDemoScene,
    step_index: usize,
    elapsed: Duration,
    last_progress_at: Duration,
    deferred_fixture_elapsed: Duration,
    tick_index: u64,
    pending_checkpoint: Option<PendingCheckpoint>,
    checkpoints: Vec<LiveCheckpoint>,
    coverage: LiveDemoCoverage,
    engine_transition_index: usize,
    engine_phase: LiveEnginePhase,
    last_ready_graph_revision: crate::real_time::GraphRevision,
    engine_envelope_baseline: Option<VoiceEnvelope>,
    structural_config_baseline: Option<InstrumentConfig>,
    cleanup_sequence_before: Option<u64>,
    completed_report: Option<LiveDemoReport>,
    runtime_audio: RuntimeAudioWitness,
    aborted: bool,
}

impl<Source, Observation> LiveDemoRunner<Source, Observation>
where
    Source: MidiEventSource,
    Observation: ControlAudioObservation,
{
    /// Starts around an already initialized fixture service. Construction does
    /// not mutate AppState or open an audio/window device.
    pub fn start(
        scene: LiveDemoScene,
        automatic_midi: AutomaticMidiTest<Source>,
        observation: Observation,
        runtime_audio: RuntimeAudioWitness,
    ) -> Self {
        let coverage = LiveDemoCoverage::with_engine_transitions(
            scene.expected_editable_parameters(),
            scene.expected_engine_transitions(),
        );
        let last_ready_graph_revision = runtime_audio.active_graph_revision();
        Self {
            automatic_midi,
            observation,
            scene,
            step_index: 0,
            elapsed: Duration::ZERO,
            last_progress_at: Duration::ZERO,
            deferred_fixture_elapsed: Duration::ZERO,
            tick_index: 0,
            pending_checkpoint: None,
            checkpoints: Vec::new(),
            coverage,
            engine_transition_index: 0,
            engine_phase: LiveEnginePhase::SelectPatch,
            last_ready_graph_revision,
            engine_envelope_baseline: None,
            structural_config_baseline: None,
            cleanup_sequence_before: None,
            completed_report: None,
            runtime_audio,
            aborted: false,
        }
    }

    /// Advances at most one scene AppEvent and never sleeps or blocks.
    pub fn advance<Boundary>(
        &mut self,
        elapsed: Duration,
        app_loop: &mut AppLoop<Boundary>,
    ) -> Result<Option<LiveCheckpoint>, LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        if self.completed_report.is_some() || self.aborted {
            return Ok(None);
        }

        self.elapsed = self.elapsed.saturating_add(elapsed);
        self.deferred_fixture_elapsed = self.deferred_fixture_elapsed.saturating_add(elapsed);
        self.tick_index = self.tick_index.saturating_add(1);

        if self.elapsed >= LIVE_DEMO_TOTAL_TIMEOUT {
            return Err(LiveDemoError::TotalTimeout {
                elapsed: self.elapsed,
            });
        }

        let result = self.advance_current(app_loop);
        if result.is_ok()
            && self.completed_report.is_none()
            && !self.aborted
            && self.elapsed.saturating_sub(self.last_progress_at) >= LIVE_DEMO_NO_PROGRESS_TIMEOUT
        {
            return Err(LiveDemoError::ProgressTimedOut {
                stage: self.progress_stage(),
                stalled_for: self.elapsed.saturating_sub(self.last_progress_at),
            });
        }
        result
    }

    fn advance_current<Boundary>(
        &mut self,
        app_loop: &mut AppLoop<Boundary>,
    ) -> Result<Option<LiveCheckpoint>, LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        if self.pending_checkpoint.is_some() {
            return self.advance_pending(app_loop);
        }

        if self.step_index == self.scene.scalar_step_count()
            && self.engine_phase != LiveEnginePhase::Complete
        {
            return self.advance_engine(app_loop);
        }

        if self.step_index == self.scene.steps().len() {
            self.try_complete(app_loop)?;
            return Ok(None);
        }

        let step = self.scene.steps()[self.step_index].clone();
        if !step.is_cleanup() {
            self.tick_fixture(app_loop)?;
        }
        self.dispatch_step(step, app_loop)
    }

    fn mark_progress(&mut self) {
        self.last_progress_at = self.elapsed;
    }

    fn progress_stage(&self) -> &'static str {
        if let Some(pending) = &self.pending_checkpoint {
            return if pending.checkpoint.is_some() {
                "parameter projection dwell"
            } else {
                "parameter audio observation"
            };
        }
        if self.step_index < self.scene.scalar_step_count() {
            return "autonomous scene dispatch";
        }
        if self.engine_phase != LiveEnginePhase::Complete {
            if self
                .scene
                .expected_engine_transitions()
                .get(self.engine_transition_index)
                .is_some_and(LiveEngineTransition::is_preset)
            {
                return match self.engine_phase {
                    LiveEnginePhase::AwaitSourceAudio { .. } => "preset source audio observation",
                    LiveEnginePhase::AwaitActivating { .. } => "preset preparation",
                    LiveEnginePhase::AwaitReady { .. } => "preset graph activation",
                    LiveEnginePhase::AwaitTargetAudio { .. } => "preset target audio observation",
                    _ => self.engine_phase.stage_name(),
                };
            }
            return self.engine_phase.stage_name();
        }
        if self.step_index < self.scene.steps().len() {
            return "semantic note cleanup dispatch";
        }
        "zero-active-notes cleanup observation"
    }

    fn tick_fixture<Boundary>(
        &mut self,
        app_loop: &mut AppLoop<Boundary>,
    ) -> Result<(), LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        let elapsed = core::mem::take(&mut self.deferred_fixture_elapsed);
        self.automatic_midi.tick(elapsed, app_loop)?;
        Ok(())
    }

    fn dispatch_step<Boundary>(
        &mut self,
        step: LiveDemoStep,
        app_loop: &mut AppLoop<Boundary>,
    ) -> Result<Option<LiveCheckpoint>, LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        let before_tree = app_loop.current_state_tree();
        let observed_value = step
            .editable_parameter()
            .map(|parameter| {
                selected_parameter_value(&before_tree, parameter, step.patch_control_id())
            })
            .transpose()?;
        let expected = LiveExpectedTransition::for_step(
            &step,
            before_tree.generation(),
            before_tree.graph_revision(),
            before_tree.selected_line(),
            observed_value,
        )?;
        let audio_sequence_before = self.observation.read_latest_on_control().sequence();
        let records_before = app_loop.event_log_ref().total_observed();
        let dispatch = app_loop.dispatch_from(step.event().clone(), EventSource::DemoScene);

        match (step.expected_outcome(), dispatch) {
            (EventOutcome::Accepted, Ok(result)) => {
                if !result.audio_effects_published() {
                    return Err(LiveDemoError::AudioBoundaryFull);
                }
            }
            (EventOutcome::Rejected, Err(actual)) if Some(actual) == step.expected_rejection() => {}
            (EventOutcome::Accepted, Err(actual)) => {
                return Err(LiveDemoError::UnexpectedRejection(actual));
            }
            (EventOutcome::Rejected, Ok(_)) => {
                return Err(LiveDemoError::ExpectedRejectionWasAccepted);
            }
            (EventOutcome::Rejected, Err(actual)) => {
                return Err(LiveDemoError::WrongRejection {
                    expected: step
                        .expected_rejection()
                        .expect("rejected steps carry a rejection"),
                    actual,
                });
            }
        }

        let event_log = app_loop.event_log_ref();
        if event_log.total_observed() != records_before.saturating_add(1) {
            return Err(LiveDemoError::MissingEventRecord);
        }
        let record = event_log
            .records()
            .last()
            .ok_or(LiveDemoError::MissingEventRecord)?
            .clone();
        verify_record(&step, &expected, &record)?;
        self.mark_progress();

        if step.requires_checkpoint() {
            let parameter = step
                .editable_parameter()
                .expect("checkpoint steps carry a typed parameter");
            self.pending_checkpoint = Some(PendingCheckpoint {
                step: self.step_index,
                expected,
                record,
                state_tree: app_loop.current_state_tree(),
                text: app_loop.current_text(),
                audio_sequence_before,
                predicate: parameter.audio_predicate(),
                dispatched_at: self.elapsed,
                dispatched_tick: self.tick_index,
                checkpoint: None,
            });
        } else {
            if step.is_cleanup() {
                self.cleanup_sequence_before = Some(audio_sequence_before);
            }
            self.step_index += 1;
        }

        Ok(None)
    }

    fn advance_pending<Boundary>(
        &mut self,
        app_loop: &mut AppLoop<Boundary>,
    ) -> Result<Option<LiveCheckpoint>, LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        let pending = self
            .pending_checkpoint
            .as_mut()
            .expect("pending state was checked");
        let captured_now = if pending.checkpoint.is_none() {
            let observation = self.observation.read_latest_on_control();
            if observation.sequence() <= pending.audio_sequence_before
                || observation.parameter_generation() < pending.record.parameter_generation()
            {
                return Ok(None);
            }
            if observation.parameter_generation() > pending.record.parameter_generation() {
                return Err(LiveDemoError::MissedAudioGeneration {
                    expected: pending.record.parameter_generation(),
                    actual: observation.parameter_generation(),
                });
            }
            if !audio_is_finite(observation) {
                return Err(LiveDemoError::NonFiniteAudioObservation);
            }
            if !pending.predicate.evaluate(observation) {
                return Ok(None);
            }
            pending.checkpoint = Some(LiveDemoCheckpoint::new(
                pending.step,
                pending.expected.clone(),
                &pending.record,
                &pending.state_tree,
                &pending.text,
                pending.audio_sequence_before,
                observation,
                pending.predicate,
            )?);
            true
        } else {
            false
        };
        if captured_now {
            self.mark_progress();
        }

        // Once exact-generation audio is captured, fixture time may advance;
        // the immutable checkpoint retains the correlated production values.
        self.tick_fixture(app_loop)?;

        let pending = self
            .pending_checkpoint
            .as_ref()
            .expect("fixture ticking does not clear pending state");
        let frame_rendered = self.tick_index > pending.dispatched_tick;
        let dwell_complete = self.elapsed.saturating_sub(pending.dispatched_at)
            >= self.scene.minimum_parameter_dwell();
        if !frame_rendered || !dwell_complete {
            return Ok(None);
        }

        let pending = self
            .pending_checkpoint
            .take()
            .expect("completed pending state exists");
        let checkpoint = pending
            .checkpoint
            .expect("completion requires an exact-generation observation");
        let parameter = checkpoint
            .expected_transition()
            .editable_parameter()
            .expect("parameter checkpoints carry a typed parameter");
        self.coverage.mark_exercised(parameter);
        let checkpoint = LiveCheckpoint::parameter(checkpoint);
        self.checkpoints.push(checkpoint.clone());
        self.step_index += 1;
        self.mark_progress();
        Ok(Some(checkpoint))
    }

    fn advance_engine<Boundary>(
        &mut self,
        app_loop: &mut AppLoop<Boundary>,
    ) -> Result<Option<LiveCheckpoint>, LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        let phase = self.engine_phase.clone();
        if !matches!(
            phase,
            LiveEnginePhase::AwaitSourceAudio { .. } | LiveEnginePhase::AwaitTargetAudio { .. }
        ) {
            self.tick_fixture(app_loop)?;
        }
        match phase {
            LiveEnginePhase::SelectPatch => {
                dispatch_engine_event(app_loop, AppEvent::SelectContext(TopLevelContext::Patch))?;
                let transition = self.current_engine_transition()?;
                let patch = app_loop
                    .patches()
                    .iter()
                    .find(|patch| patch.id() == transition.patch_id())
                    .ok_or(LiveDemoError::EngineLifecycleMismatch)?;
                self.engine_envelope_baseline = Some(*patch.envelope());
                self.structural_config_baseline = Some(patch.instrument_config().clone());
                self.engine_phase = LiveEnginePhase::FocusControl;
                self.mark_progress();
                Ok(None)
            }
            LiveEnginePhase::FocusControl => {
                let transition = self.current_engine_transition()?.clone();
                let page = app_loop
                    .current_patch_page()
                    .ok_or(LiveDemoError::MissingPatchProjection)?;
                let target = transition.focused_control_id();
                if page.focused_control_id() == target {
                    self.engine_phase = LiveEnginePhase::Request;
                } else {
                    let controls = app_loop.state().focused_patch_controls()?;
                    let current = controls
                        .iter()
                        .position(|control| control == &page.focused_control_id())
                        .ok_or(LiveDemoError::EngineProjectionMismatch)?;
                    let target_index = controls
                        .iter()
                        .position(|control| control == &target)
                        .ok_or(LiveDemoError::EngineProjectionMismatch)?;
                    let direction = if current < target_index {
                        Direction::Down
                    } else {
                        Direction::Up
                    };
                    dispatch_engine_event(app_loop, AppEvent::Navigate(direction))?;
                }
                self.mark_progress();
                Ok(None)
            }
            LiveEnginePhase::Request => {
                let transition = self.current_engine_transition()?.clone();
                let page = app_loop
                    .current_patch_page()
                    .ok_or(LiveDemoError::MissingPatchProjection)?;
                if page.patch().id() != transition.patch_id()
                    || page.engine().active_capability_id() != transition.source_capability_id()
                    || app_loop.engine_selection_status().kind() != EngineSelectionStatusKind::Ready
                    || app_loop.graph_revision() != self.last_ready_graph_revision
                    || !focused_config_matches_transition_source(app_loop, &transition)?
                {
                    return Err(LiveDemoError::EngineLifecycleMismatch);
                }
                if transition.is_preset() {
                    let audio_sequence_before =
                        self.observation.read_latest_on_control().sequence();
                    let note = MidiMessage::try_new(
                        transition.channel(),
                        MidiMessageKind::NoteOn,
                        60,
                        112,
                    )
                    .expect("the frozen live source MIDI constants are valid");
                    let note_record = dispatch_engine_event(
                        app_loop,
                        AppEvent::Midi {
                            patch_id: transition.patch_id(),
                            message: note,
                        },
                    )?;
                    self.engine_phase = LiveEnginePhase::AwaitSourceAudio {
                        source_revision: app_loop.graph_revision(),
                        generation: note_record.generation_after(),
                        audio_sequence_before,
                    };
                } else {
                    self.engine_phase = LiveEnginePhase::DispatchRequest { source_audio: None };
                }
                self.mark_progress();
                Ok(None)
            }
            LiveEnginePhase::AwaitSourceAudio {
                source_revision,
                generation,
                audio_sequence_before,
            } => {
                let transition = self.current_engine_transition()?.clone();
                let observation = self.observation.read_latest_on_control();
                if observation.sequence() <= audio_sequence_before
                    || observation.parameter_generation() < generation
                {
                    return Ok(None);
                }
                if observation.parameter_generation() > generation {
                    return Err(LiveDemoError::MissedAudioGeneration {
                        expected: generation,
                        actual: observation.parameter_generation(),
                    });
                }
                if !audio_is_finite(observation)
                    || observation.active_graph_revision() != source_revision
                    || observation.primary_patch_id() != Some(transition.patch_id())
                    || observation.primary_active_notes() == 0
                    || observation.primary_patch_rms() <= 0.0
                    || observation.routing_failures() != 0
                {
                    return Ok(None);
                }
                self.engine_phase = LiveEnginePhase::DispatchRequest {
                    source_audio: Some(observation),
                };
                self.mark_progress();
                Ok(None)
            }
            LiveEnginePhase::DispatchRequest { source_audio } => {
                let transition = self.current_engine_transition()?.clone();
                let source_revision = app_loop.graph_revision();
                let request_record =
                    dispatch_engine_event(app_loop, AppEvent::Adjust(transition.direction()))?;
                let request_id = app_loop
                    .engine_selection_status()
                    .correlation()
                    .map(|correlation| correlation.request_id())
                    .ok_or(LiveDemoError::EngineLifecycleMismatch)?;
                verify_engine_effects(
                    app_loop,
                    &transition,
                    request_id,
                    source_revision,
                    None,
                    EngineSelectionStatusKind::Preparing,
                )?;
                let checkpoint = self.capture_engine_checkpoint(
                    app_loop,
                    &transition,
                    EngineSelectionStatusKind::Preparing,
                    request_id,
                    request_record.sequence(),
                    source_audio,
                    None,
                )?;
                self.engine_phase = LiveEnginePhase::AwaitActivating {
                    request_id,
                    source_revision,
                };
                self.mark_progress();
                self.push_engine_checkpoint(checkpoint)
            }
            LiveEnginePhase::AwaitActivating {
                request_id,
                source_revision,
            } => {
                let transition = self.current_engine_transition()?.clone();
                match app_loop.engine_selection_status().kind() {
                    EngineSelectionStatusKind::Preparing => Ok(None),
                    EngineSelectionStatusKind::Activating => {
                        let target_revision = app_loop
                            .engine_selection_status()
                            .correlation()
                            .and_then(|correlation| correlation.target_graph_revision())
                            .ok_or(LiveDemoError::EngineLifecycleMismatch)?;
                        if target_revision <= source_revision
                            || target_revision <= self.last_ready_graph_revision
                        {
                            return Err(LiveDemoError::EngineLifecycleMismatch);
                        }
                        let event_sequence = verify_engine_effects(
                            app_loop,
                            &transition,
                            request_id,
                            source_revision,
                            Some(target_revision),
                            EngineSelectionStatusKind::Activating,
                        )?;
                        let checkpoint = self.capture_engine_checkpoint(
                            app_loop,
                            &transition,
                            EngineSelectionStatusKind::Activating,
                            request_id,
                            event_sequence,
                            None,
                            None,
                        )?;
                        self.engine_phase = LiveEnginePhase::AwaitReady {
                            request_id,
                            source_revision,
                            target_revision,
                        };
                        self.mark_progress();
                        self.push_engine_checkpoint(checkpoint)
                    }
                    EngineSelectionStatusKind::Failed | EngineSelectionStatusKind::Ready => {
                        Err(LiveDemoError::EngineLifecycleMismatch)
                    }
                }
            }
            LiveEnginePhase::AwaitReady {
                request_id,
                source_revision,
                target_revision,
            } => {
                let transition = self.current_engine_transition()?.clone();
                match app_loop.engine_selection_status().kind() {
                    EngineSelectionStatusKind::Activating => Ok(None),
                    EngineSelectionStatusKind::Ready => {
                        verify_engine_effects(
                            app_loop,
                            &transition,
                            request_id,
                            source_revision,
                            Some(target_revision),
                            EngineSelectionStatusKind::Ready,
                        )?;
                        if transition.is_preset() {
                            dispatch_engine_event(
                                app_loop,
                                AppEvent::Midi {
                                    patch_id: transition.patch_id(),
                                    message: MidiMessage::all_notes_off(transition.channel()),
                                },
                            )?;
                        }
                        self.engine_phase = LiveEnginePhase::StartTargetNote {
                            request_id,
                            source_revision,
                            target_revision,
                        };
                        self.mark_progress();
                        Ok(None)
                    }
                    EngineSelectionStatusKind::Failed | EngineSelectionStatusKind::Preparing => {
                        Err(LiveDemoError::EngineLifecycleMismatch)
                    }
                }
            }
            LiveEnginePhase::StartTargetNote {
                request_id,
                source_revision,
                target_revision,
            } => {
                let transition = self.current_engine_transition()?.clone();
                let audio_sequence_before = self.observation.read_latest_on_control().sequence();
                let note = MidiMessage::try_new(
                    transition.channel(),
                    MidiMessageKind::NoteOn,
                    64_u8.saturating_add(self.engine_transition_index as u8),
                    112,
                )
                .expect("the frozen live target MIDI constants are valid");
                let note_record = dispatch_engine_event(
                    app_loop,
                    AppEvent::Midi {
                        patch_id: transition.patch_id(),
                        message: note,
                    },
                )?;
                self.engine_phase = LiveEnginePhase::AwaitTargetAudio {
                    request_id,
                    source_revision,
                    target_revision,
                    generation: note_record.generation_after(),
                    event_sequence: note_record.sequence(),
                    audio_sequence_before,
                };
                self.mark_progress();
                Ok(None)
            }
            LiveEnginePhase::AwaitTargetAudio {
                request_id,
                source_revision,
                target_revision,
                generation,
                event_sequence,
                audio_sequence_before,
            } => {
                let transition = self.current_engine_transition()?.clone();
                let observation = self.observation.read_latest_on_control();
                if observation.sequence() <= audio_sequence_before
                    || observation.parameter_generation() < generation
                {
                    return Ok(None);
                }
                if observation.parameter_generation() > generation {
                    return Err(LiveDemoError::MissedAudioGeneration {
                        expected: generation,
                        actual: observation.parameter_generation(),
                    });
                }
                if !audio_is_finite(observation)
                    || observation.active_graph_revision() != target_revision
                    || observation.routing_failures() != 0
                {
                    return Err(LiveDemoError::EngineTargetAudioMismatch);
                }
                if observation.primary_patch_id() != Some(transition.patch_id())
                    || observation.primary_active_notes() == 0
                    || observation.primary_patch_rms() <= 0.0
                {
                    return Ok(None);
                }
                verify_engine_effects(
                    app_loop,
                    &transition,
                    request_id,
                    source_revision,
                    Some(target_revision),
                    EngineSelectionStatusKind::Ready,
                )?;
                let checkpoint = self.capture_engine_checkpoint(
                    app_loop,
                    &transition,
                    EngineSelectionStatusKind::Ready,
                    request_id,
                    event_sequence,
                    None,
                    Some(observation),
                )?;
                if !self
                    .runtime_audio
                    .record_ready_capability(transition.target_capability_id(), target_revision)
                {
                    return Err(LiveDemoError::EngineLifecycleMismatch);
                }
                self.coverage.mark_engine_exercised(&transition);
                self.last_ready_graph_revision = target_revision;
                self.engine_phase = LiveEnginePhase::StopTargetNote;
                self.mark_progress();
                self.push_engine_checkpoint(checkpoint)
            }
            LiveEnginePhase::StopTargetNote => {
                let transition = self.current_engine_transition()?.clone();
                dispatch_engine_event(
                    app_loop,
                    AppEvent::Midi {
                        patch_id: transition.patch_id(),
                        message: MidiMessage::all_notes_off(transition.channel()),
                    },
                )?;
                self.engine_transition_index = self.engine_transition_index.saturating_add(1);
                self.engine_phase = if self.engine_transition_index
                    < self.scene.expected_engine_transitions().len()
                {
                    let next = self.current_engine_transition()?;
                    let patch = app_loop
                        .patches()
                        .iter()
                        .find(|patch| patch.id() == next.patch_id())
                        .ok_or(LiveDemoError::EngineLifecycleMismatch)?;
                    self.engine_envelope_baseline = Some(*patch.envelope());
                    self.structural_config_baseline = Some(patch.instrument_config().clone());
                    LiveEnginePhase::FocusControl
                } else {
                    LiveEnginePhase::RestoreMixer
                };
                self.mark_progress();
                Ok(None)
            }
            LiveEnginePhase::RestoreMixer => {
                dispatch_engine_event(app_loop, AppEvent::SelectContext(TopLevelContext::Mixer))?;
                self.engine_phase = LiveEnginePhase::Complete;
                self.mark_progress();
                Ok(None)
            }
            LiveEnginePhase::Complete => Ok(None),
        }
    }

    fn current_engine_transition(&self) -> Result<&LiveEngineTransition, LiveDemoError> {
        self.scene
            .expected_engine_transitions()
            .get(self.engine_transition_index)
            .ok_or(LiveDemoError::EngineLifecycleMismatch)
    }

    fn push_engine_checkpoint(
        &mut self,
        checkpoint: LiveEngineCheckpoint,
    ) -> Result<Option<LiveCheckpoint>, LiveDemoError> {
        let checkpoint = LiveCheckpoint::engine(checkpoint);
        self.checkpoints.push(checkpoint.clone());
        Ok(Some(checkpoint))
    }

    #[allow(clippy::too_many_arguments)]
    fn capture_engine_checkpoint<Boundary>(
        &self,
        app_loop: &AppLoop<Boundary>,
        transition: &LiveEngineTransition,
        status: EngineSelectionStatusKind,
        request_id: EngineSelectionRequestId,
        event_sequence: u64,
        source_audio_observation: Option<AudioObservationSnapshot>,
        audio_observation: Option<AudioObservationSnapshot>,
    ) -> Result<LiveEngineCheckpoint, LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        let page = app_loop
            .current_patch_page()
            .ok_or(LiveDemoError::MissingPatchProjection)?;
        let engine = page.engine();
        let tree = app_loop.current_state_tree();
        let text = app_loop.current_text();
        let lifecycle = app_loop.engine_selection_status();
        let handoff = app_loop
            .engine_graph_handoff_status()
            .ok_or(LiveDemoError::EngineRuntimeUnavailable)?;
        if page.patch().id() != transition.patch_id()
            || page.focused_control_id() != transition.focused_control_id()
            || page.state_hash() != tree.state_hash()
            || text.state_hash() != tree.state_hash()
            || tree.graph_revision() != app_loop.graph_revision()
            || lifecycle.kind() != status
            || lifecycle.failure().is_some()
        {
            return Err(LiveDemoError::EngineProjectionMismatch);
        }
        if status == EngineSelectionStatusKind::Ready {
            if lifecycle.correlation().is_some() {
                return Err(LiveDemoError::EngineProjectionMismatch);
            }
        } else if !lifecycle.correlation().is_some_and(|correlation| {
            correlation.request_id() == request_id
                && correlation.patch_id() == transition.patch_id()
                && correlation.intent() == transition.intent()
                && correlation.source_capability_id() == transition.source_capability_id()
                && correlation.target_capability_id() == transition.target_capability_id()
        }) {
            return Err(LiveDemoError::EngineProjectionMismatch);
        }
        let patch = app_loop
            .patches()
            .iter()
            .find(|patch| patch.id() == transition.patch_id())
            .ok_or(LiveDemoError::EngineProjectionMismatch)?;
        if self.engine_envelope_baseline.as_ref() != Some(patch.envelope()) {
            return Err(LiveDemoError::EngineProjectionMismatch);
        }
        let baseline = self
            .structural_config_baseline
            .as_ref()
            .ok_or(LiveDemoError::EngineTargetConfigMismatch)?;
        let preset = match transition.intent() {
            StructuralEditIntent::ReplaceCapability { .. } => {
                let expected_request =
                    (status != EngineSelectionStatusKind::Ready).then_some(request_id);
                let expected_requested = (status != EngineSelectionStatusKind::Ready)
                    .then_some(transition.target_capability_id());
                let expected_active = if status == EngineSelectionStatusKind::Preparing {
                    transition.source_capability_id()
                } else {
                    transition.target_capability_id()
                };
                if engine.status() != status
                    || engine.active_capability_id() != expected_active
                    || engine.request_id() != expected_request
                    || engine.requested_capability_id() != expected_requested
                    || engine.failure().is_some()
                    || (status == EngineSelectionStatusKind::Preparing
                        && patch.instrument_config() != baseline)
                    || (status != EngineSelectionStatusKind::Preparing
                        && !focused_config_is_descriptor_default(app_loop, transition)?)
                {
                    return Err(LiveDemoError::EngineTargetConfigMismatch);
                }
                None
            }
            StructuralEditIntent::ReplaceParameterChoice {
                parameter_id,
                choice_id,
                ..
            } => {
                if engine.status() != EngineSelectionStatusKind::Ready
                    || engine.active_capability_id() != transition.source_capability_id()
                    || engine.request_id().is_some()
                    || engine.requested_capability_id().is_some()
                    || engine.failure().is_some()
                {
                    return Err(LiveDemoError::EngineProjectionMismatch);
                }
                let row = page
                    .sections()
                    .iter()
                    .flat_map(|section| section.parameters())
                    .find(|row| row.id() == parameter_id)
                    .ok_or(LiveDemoError::EngineProjectionMismatch)?;
                let active_choice = if status == EngineSelectionStatusKind::Preparing {
                    transition
                        .source_choice_id()
                        .ok_or(LiveDemoError::EngineProjectionMismatch)?
                } else {
                    transition
                        .target_choice_id()
                        .ok_or(LiveDemoError::EngineProjectionMismatch)?
                };
                let active_label = if status == EngineSelectionStatusKind::Preparing {
                    transition
                        .source_label()
                        .ok_or(LiveDemoError::EngineProjectionMismatch)?
                } else {
                    transition
                        .target_label()
                        .ok_or(LiveDemoError::EngineProjectionMismatch)?
                };
                let requested_choice =
                    (status != EngineSelectionStatusKind::Ready).then_some(choice_id.as_str());
                let requested_label = (status != EngineSelectionStatusKind::Ready)
                    .then(|| transition.target_label())
                    .flatten();
                let expected_row_status =
                    (status != EngineSelectionStatusKind::Ready).then_some(status);
                if row.control_id() != Some(transition.focused_control_id())
                    || row.selected_choice_id() != Some(active_choice)
                    || row.selected_label() != Some(active_label)
                    || row.requested_choice_id() != requested_choice
                    || row.requested_label() != requested_label
                    || row.status() != expected_row_status
                    || row.failure().is_some()
                    || (status == EngineSelectionStatusKind::Preparing
                        && patch.instrument_config() != baseline)
                    || (status != EngineSelectionStatusKind::Preparing
                        && !preset_config_delta_is_exact(
                            baseline,
                            patch.instrument_config(),
                            parameter_id,
                            choice_id,
                        ))
                {
                    return Err(LiveDemoError::EngineTargetConfigMismatch);
                }
                Some(LivePresetProjection::new(
                    transition
                        .source_choice_id()
                        .ok_or(LiveDemoError::EngineProjectionMismatch)?,
                    transition
                        .source_label()
                        .ok_or(LiveDemoError::EngineProjectionMismatch)?,
                    transition
                        .target_choice_id()
                        .ok_or(LiveDemoError::EngineProjectionMismatch)?,
                    transition
                        .target_label()
                        .ok_or(LiveDemoError::EngineProjectionMismatch)?,
                    active_choice,
                    active_label,
                    requested_choice.map(str::to_owned),
                    requested_label.map(str::to_owned),
                ))
            }
        };

        LiveEngineCheckpoint::new(
            transition.identifier(),
            self.engine_transition_index,
            status,
            request_id,
            transition.patch_id(),
            page.focused_control_id(),
            transition.source_capability_id().clone(),
            transition.target_capability_id().clone(),
            transition.intent().clone(),
            preset,
            engine.active_capability_id().clone(),
            engine.requested_capability_id().cloned(),
            tree.generation(),
            tree.state_hash(),
            app_loop.graph_revision(),
            handoff.active_revision(),
            handoff.retired_revision(),
            app_loop.staged_graph_revision(),
            app_loop.in_flight_graph_revision(),
            event_sequence,
            source_audio_observation,
            audio_observation,
            self.runtime_audio.callback_allocations(),
            self.runtime_audio.callback_destructions(),
        )
        .map_err(LiveDemoError::from)
    }

    fn try_complete<Boundary>(&mut self, app_loop: &AppLoop<Boundary>) -> Result<(), LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        let cleanup_sequence_before = self
            .cleanup_sequence_before
            .ok_or(LiveDemoError::MissingCleanupObservation)?;
        let observation = self.observation.read_latest_on_control();
        let tree = app_loop.current_state_tree();
        if observation.sequence() <= cleanup_sequence_before
            || observation.parameter_generation() < tree.generation()
            || observation.active_notes() != 0
        {
            return Ok(());
        }
        if observation.parameter_generation() > tree.generation() {
            return Err(LiveDemoError::MissedAudioGeneration {
                expected: tree.generation(),
                actual: observation.parameter_generation(),
            });
        }
        if !audio_is_finite(observation) {
            return Err(LiveDemoError::NonFiniteAudioObservation);
        }

        let installed: Vec<_> = self.scene.patch_ids().collect();
        let active_graph_revision = tree.graph_revision();
        self.completed_report = Some(LiveDemoReport::new(
            self.scene.name(),
            self.checkpoints.clone(),
            app_loop.event_log(),
            tree,
            self.coverage.clone(),
            &installed,
            cleanup_sequence_before,
            observation,
            self.runtime_audio
                .with_active_graph_revision(active_graph_revision),
        )?);
        self.mark_progress();
        Ok(())
    }

    /// Best-effort semantic cleanup for a user-closed window. It never emits a
    /// success report and makes later advances inert.
    pub fn cleanup_before_close<Boundary>(
        &mut self,
        app_loop: &mut AppLoop<Boundary>,
    ) -> Result<(), LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        if self.completed_report.is_some() || self.aborted {
            return Ok(());
        }
        for step in self.scene.steps().iter().filter(|step| step.is_cleanup()) {
            let result = app_loop.dispatch_from(step.event().clone(), EventSource::DemoScene)?;
            if !result.audio_effects_published() {
                return Err(LiveDemoError::AudioBoundaryFull);
            }
        }
        self.aborted = true;
        self.pending_checkpoint = None;
        Ok(())
    }

    pub const fn completed_report(&self) -> Option<&LiveDemoReport> {
        self.completed_report.as_ref()
    }

    pub const fn is_aborted(&self) -> bool {
        self.aborted
    }

    pub const fn step_index(&self) -> usize {
        self.step_index
    }
}

struct PendingCheckpoint {
    step: usize,
    expected: LiveExpectedTransition,
    record: EventRecord,
    state_tree: StateTree,
    text: TextProjection,
    audio_sequence_before: u64,
    predicate: crate::testing::live_demo_scene::LiveAudioPredicate,
    dispatched_at: Duration,
    dispatched_tick: u64,
    checkpoint: Option<LiveDemoCheckpoint>,
}

#[derive(Clone, Debug, PartialEq)]
enum LiveEnginePhase {
    SelectPatch,
    FocusControl,
    Request,
    AwaitSourceAudio {
        source_revision: crate::real_time::GraphRevision,
        generation: u64,
        audio_sequence_before: u64,
    },
    DispatchRequest {
        source_audio: Option<AudioObservationSnapshot>,
    },
    AwaitActivating {
        request_id: EngineSelectionRequestId,
        source_revision: crate::real_time::GraphRevision,
    },
    AwaitReady {
        request_id: EngineSelectionRequestId,
        source_revision: crate::real_time::GraphRevision,
        target_revision: crate::real_time::GraphRevision,
    },
    StartTargetNote {
        request_id: EngineSelectionRequestId,
        source_revision: crate::real_time::GraphRevision,
        target_revision: crate::real_time::GraphRevision,
    },
    AwaitTargetAudio {
        request_id: EngineSelectionRequestId,
        source_revision: crate::real_time::GraphRevision,
        target_revision: crate::real_time::GraphRevision,
        generation: u64,
        event_sequence: u64,
        audio_sequence_before: u64,
    },
    StopTargetNote,
    RestoreMixer,
    Complete,
}

impl LiveEnginePhase {
    const fn stage_name(&self) -> &'static str {
        match self {
            Self::SelectPatch => "engine PATCH selection",
            Self::FocusControl => "structural control focus",
            Self::Request => "structural source-note dispatch",
            Self::AwaitSourceAudio { .. } => "preset source audio observation",
            Self::DispatchRequest { .. } => "structural request dispatch",
            Self::AwaitActivating { .. } => "engine preparation",
            Self::AwaitReady { .. } => "engine graph activation",
            Self::StartTargetNote { .. } => "structural target-note dispatch",
            Self::AwaitTargetAudio { .. } => "engine target audio observation",
            Self::StopTargetNote => "structural target-note cleanup",
            Self::RestoreMixer => "MIXER restoration",
            Self::Complete => "engine transitions complete",
        }
    }
}

fn dispatch_engine_event<Boundary>(
    app_loop: &mut AppLoop<Boundary>,
    event: AppEvent,
) -> Result<EventRecord, LiveDemoError>
where
    Boundary: ControlAudioBoundary,
{
    let records_before = app_loop.event_log_ref().total_observed();
    let result = app_loop
        .dispatch_from(event.clone(), EventSource::DemoScene)
        .map_err(LiveDemoError::UnexpectedRejection)?;
    if !result.audio_effects_published() {
        return Err(LiveDemoError::AudioBoundaryFull);
    }
    let log = app_loop.event_log_ref();
    if log.total_observed() != records_before.saturating_add(1) {
        return Err(LiveDemoError::MissingEventRecord);
    }
    let record = log
        .records()
        .last()
        .ok_or(LiveDemoError::MissingEventRecord)?;
    if record.source() != EventSource::DemoScene
        || record.outcome() != EventOutcome::Accepted
        || record.input() != &EventInput::from(&event)
    {
        return Err(LiveDemoError::EventRecordMismatch);
    }
    Ok(record.clone())
}

fn verify_engine_effects<Boundary>(
    app_loop: &AppLoop<Boundary>,
    transition: &LiveEngineTransition,
    request_id: EngineSelectionRequestId,
    source_revision: crate::real_time::GraphRevision,
    target_revision: Option<crate::real_time::GraphRevision>,
    status: EngineSelectionStatusKind,
) -> Result<u64, LiveDemoError>
where
    Boundary: ControlAudioBoundary,
{
    let expected: &[EngineSelectionEffectKind] = match status {
        EngineSelectionStatusKind::Preparing => &[EngineSelectionEffectKind::PrepareRequested],
        EngineSelectionStatusKind::Activating => &[
            EngineSelectionEffectKind::PrepareRequested,
            EngineSelectionEffectKind::CandidateCommitted,
            EngineSelectionEffectKind::GraphStaged,
            EngineSelectionEffectKind::GraphPublished,
        ],
        EngineSelectionStatusKind::Ready => &[
            EngineSelectionEffectKind::PrepareRequested,
            EngineSelectionEffectKind::CandidateCommitted,
            EngineSelectionEffectKind::GraphStaged,
            EngineSelectionEffectKind::GraphPublished,
            EngineSelectionEffectKind::ActivationAcknowledged,
        ],
        EngineSelectionStatusKind::Failed => return Err(LiveDemoError::EngineLifecycleMismatch),
    };
    let mut actual = Vec::new();
    let mut endpoint = None;
    for record in app_loop.event_log_ref().records() {
        for emitted in record.emitted_events() {
            let EmittedEvent::EngineSelection { effect } = emitted else {
                continue;
            };
            if effect.request_id() != request_id {
                continue;
            }
            let expected_target = if effect.kind() == EngineSelectionEffectKind::PrepareRequested {
                None
            } else {
                target_revision
            };
            if effect.patch_id() != transition.patch_id()
                || effect.intent() != transition.intent()
                || effect.source_capability_id() != transition.source_capability_id()
                || effect.target_capability_id() != transition.target_capability_id()
                || effect.source_graph_revision() != source_revision
                || effect.target_graph_revision() != expected_target
            {
                return Err(LiveDemoError::EngineEffectMismatch);
            }
            actual.push(effect.kind());
            endpoint = Some(record.sequence());
        }
    }
    if actual != expected {
        return Err(LiveDemoError::EngineEffectMismatch);
    }
    endpoint.ok_or(LiveDemoError::EngineEffectMismatch)
}

fn focused_config_matches_transition_source<Boundary>(
    app_loop: &AppLoop<Boundary>,
    transition: &LiveEngineTransition,
) -> Result<bool, LiveDemoError>
where
    Boundary: ControlAudioBoundary,
{
    let patch = app_loop
        .patches()
        .iter()
        .find(|patch| patch.id() == transition.patch_id())
        .ok_or(LiveDemoError::EngineTargetConfigMismatch)?;
    let config = patch.instrument_config();
    if config.capability_id() != transition.source_capability_id() {
        return Ok(false);
    }
    match transition.intent() {
        StructuralEditIntent::ReplaceCapability { .. } => Ok(true),
        StructuralEditIntent::ReplaceParameterChoice { parameter_id, .. } => Ok(matches!(
            config.value(parameter_id),
            Some(ParameterValue::Choice(choice_id))
                if Some(choice_id.as_str()) == transition.source_choice_id()
        )),
    }
}

fn preset_config_delta_is_exact(
    source: &InstrumentConfig,
    candidate: &InstrumentConfig,
    parameter_id: &crate::synth::ParameterId,
    target_choice_id: &str,
) -> bool {
    if source.capability_id() != candidate.capability_id()
        || source.asset_references() != candidate.asset_references()
        || source.values().len() != candidate.values().len()
        || !matches!(
            candidate.value(parameter_id),
            Some(ParameterValue::Choice(choice_id)) if choice_id == target_choice_id
        )
    {
        return false;
    }
    let mut changed = 0usize;
    for (before, after) in source.values().iter().zip(candidate.values()) {
        if before.parameter_id() != after.parameter_id() {
            return false;
        }
        if before != after {
            if before.parameter_id() != parameter_id {
                return false;
            }
            changed = changed.saturating_add(1);
        }
    }
    changed == 1
}

fn focused_config_is_descriptor_default<Boundary>(
    app_loop: &AppLoop<Boundary>,
    transition: &LiveEngineTransition,
) -> Result<bool, LiveDemoError>
where
    Boundary: ControlAudioBoundary,
{
    let patch = app_loop
        .patches()
        .iter()
        .find(|patch| patch.id() == transition.patch_id())
        .ok_or(LiveDemoError::EngineTargetConfigMismatch)?;
    let config = patch.instrument_config();
    let descriptor = app_loop
        .capabilities()
        .descriptor(transition.target_capability_id())
        .ok_or(LiveDemoError::EngineTargetConfigMismatch)?;
    if config.capability_id() != transition.target_capability_id()
        || app_loop.capabilities().validate_config(config).is_err()
    {
        return Ok(false);
    }
    let expected_values = descriptor
        .parameters()
        .filter(|parameter| parameter.kind() != ParameterKind::Asset)
        .count();
    let expected_assets = descriptor
        .parameters()
        .filter(|parameter| parameter.kind() == ParameterKind::Asset)
        .count();
    if config.values().len() != expected_values
        || config.asset_references().len() != expected_assets
    {
        return Ok(false);
    }
    Ok(descriptor
        .parameters()
        .all(|parameter| match parameter.default_value() {
            ParameterDefault::Value(value) => config.value(parameter.id()) == Some(value),
            ParameterDefault::Asset(reference) => {
                config.asset_reference(parameter.id()) == Some(reference)
            }
        }))
}

fn verify_record(
    step: &LiveDemoStep,
    expected: &LiveExpectedTransition,
    record: &EventRecord,
) -> Result<(), LiveDemoError> {
    if record.source() != EventSource::DemoScene
        || record.input() != expected.input()
        || record.input() != &EventInput::from(step.event())
        || record.outcome() != expected.outcome()
        || record.generation_before() != expected.generation_before()
        || record.generation_after() != expected.generation_after()
        || record.parameter_generation() != expected.parameter_generation()
        || record.emitted_events() != expected.emitted_effects()
        || record.rejection().map(|value| value.name()) != expected.rejection()
    {
        return Err(LiveDemoError::EventRecordMismatch);
    }
    if record.outcome() == EventOutcome::Rejected
        && (record.state_hash_before() != record.state_hash_after()
            || !record.emitted_events().is_empty())
    {
        return Err(LiveDemoError::EventRecordMismatch);
    }
    Ok(())
}

fn audio_is_finite(observation: AudioObservationSnapshot) -> bool {
    observation.left_peak().is_finite()
        && observation.right_peak().is_finite()
        && observation.output_rms().is_finite()
        && observation.reverb_input_rms().is_finite()
        && observation.delay_input_rms().is_finite()
        && observation.wet_output_rms().is_finite()
        && observation.primary_patch_rms().is_finite()
        && observation.non_finite_samples() == 0
}

/// A typed live orchestration failure suitable for visible standalone output.
#[derive(Debug)]
pub enum LiveDemoError {
    Scene(LiveDemoSceneError),
    Fixture(TestInputError),
    Checkpoint(LiveDemoCheckpointError),
    Report(LiveDemoReportError),
    UnexpectedRejection(EventRejection),
    WrongRejection {
        expected: EventRejection,
        actual: EventRejection,
    },
    ExpectedRejectionWasAccepted,
    AudioBoundaryFull,
    MissingEventRecord,
    EventRecordMismatch,
    MissedAudioGeneration {
        expected: u64,
        actual: u64,
    },
    NonFiniteAudioObservation,
    MissingCleanupObservation,
    MissingPatchProjection,
    EngineRuntimeUnavailable,
    EngineLifecycleMismatch,
    EngineProjectionMismatch,
    EngineEffectMismatch,
    EngineTargetConfigMismatch,
    EngineTargetAudioMismatch,
    ProgressTimedOut {
        stage: &'static str,
        stalled_for: Duration,
    },
    TotalTimeout {
        elapsed: Duration,
    },
}

impl From<LiveDemoSceneError> for LiveDemoError {
    fn from(error: LiveDemoSceneError) -> Self {
        Self::Scene(error)
    }
}

impl From<TestInputError> for LiveDemoError {
    fn from(error: TestInputError) -> Self {
        Self::Fixture(error)
    }
}

impl From<LiveDemoCheckpointError> for LiveDemoError {
    fn from(error: LiveDemoCheckpointError) -> Self {
        Self::Checkpoint(error)
    }
}

impl From<LiveDemoReportError> for LiveDemoError {
    fn from(error: LiveDemoReportError) -> Self {
        Self::Report(error)
    }
}

impl From<EventRejection> for LiveDemoError {
    fn from(error: EventRejection) -> Self {
        Self::UnexpectedRejection(error)
    }
}

impl fmt::Display for LiveDemoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scene(error) => error.fmt(formatter),
            Self::Fixture(error) => error.fmt(formatter),
            Self::Checkpoint(error) => error.fmt(formatter),
            Self::Report(error) => error.fmt(formatter),
            Self::UnexpectedRejection(error) => {
                write!(formatter, "live event was unexpectedly rejected: {error}")
            }
            Self::WrongRejection { expected, actual } => {
                write!(
                    formatter,
                    "live event expected {expected:?}, got {actual:?}"
                )
            }
            Self::ExpectedRejectionWasAccepted => {
                formatter.write_str("live boundary probe was unexpectedly accepted")
            }
            Self::AudioBoundaryFull => {
                formatter.write_str("live event audio command did not cross the bounded boundary")
            }
            Self::MissingEventRecord => {
                formatter.write_str("live event did not append exactly one EventRecord")
            }
            Self::EventRecordMismatch => {
                formatter.write_str("live EventRecord differs from its pre-dispatch expectation")
            }
            Self::MissedAudioGeneration { expected, actual } => write!(
                formatter,
                "latest audio observation generation {actual} passed required generation {expected}"
            ),
            Self::NonFiniteAudioObservation => {
                formatter.write_str("live audio observation contains non-finite output")
            }
            Self::MissingCleanupObservation => {
                formatter.write_str("live cleanup did not establish an observation sequence")
            }
            Self::MissingPatchProjection => {
                formatter.write_str("live engine transition has no canonical PATCH projection")
            }
            Self::EngineRuntimeUnavailable => formatter.write_str(
                "live engine transition has no configured structural runtime observation",
            ),
            Self::EngineLifecycleMismatch => formatter
                .write_str("live engine transition skipped or contradicted its ordered lifecycle"),
            Self::EngineProjectionMismatch => formatter.write_str(
                "live engine state, PATCH, text, parameter, and tree projections disagree",
            ),
            Self::EngineEffectMismatch => formatter.write_str(
                "live engine EventLog effects do not match the correlated structural lifecycle",
            ),
            Self::EngineTargetConfigMismatch => formatter
                .write_str("live engine target is not the exact descriptor-default configuration"),
            Self::EngineTargetAudioMismatch => formatter.write_str(
                "live engine target did not produce finite nonzero generation-tagged output",
            ),
            Self::ProgressTimedOut { stage, stalled_for } => write!(
                formatter,
                "live demo made no progress while awaiting {stage} for {:.1} seconds",
                stalled_for.as_secs_f32()
            ),
            Self::TotalTimeout { elapsed } => write!(
                formatter,
                "live demo exceeded its {}-second total bound after {:.1} seconds",
                LIVE_DEMO_TOTAL_TIMEOUT.as_secs(),
                elapsed.as_secs_f32()
            ),
        }
    }
}

impl std::error::Error for LiveDemoError {}

#[cfg(test)]
mod tests {
    use super::audio_is_finite;
    use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;

    #[test]
    fn non_finite_callback_measurements_are_rejected_before_checkpointing() {
        let observation = AudioObservationSnapshot::from_parts(
            1,
            1,
            64,
            7,
            0,
            1,
            f32::NAN,
            0.2,
            0.1,
            0.1,
            0.1,
            0.1,
            1,
            0,
        );

        assert!(!audio_is_finite(observation));
    }
}
