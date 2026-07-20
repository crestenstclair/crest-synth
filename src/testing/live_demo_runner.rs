use crate::control::app_loop::AppLoop;
use crate::control::app_state::EventRejection;
use crate::control::event_record::{EventInput, EventOutcome, EventRecord, EventSource};
use crate::control::state_tree::StateTree;
use crate::control::text_projection::TextProjection;
use crate::real_time::audio_boundary::ControlAudioBoundary;
use crate::real_time::audio_observation::ControlAudioObservation;
use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
use crate::testing::automatic_midi_test::{AutomaticMidiTest, TestInputError};
use crate::testing::live_demo_checkpoint::{LiveDemoCheckpoint, LiveDemoCheckpointError};
use crate::testing::live_demo_report::{LiveDemoCoverage, LiveDemoReport, LiveDemoReportError};
use crate::testing::live_demo_scene::{
    selected_parameter_value, LiveDemoScene, LiveDemoSceneError, LiveDemoStep,
    LiveExpectedTransition,
};
use crate::testing::midi_event_source::MidiEventSource;
use core::fmt;
use std::time::Duration;

/// Control-thread state machine for the real-window live scene.
pub struct LiveDemoRunner<Source, Observation> {
    automatic_midi: AutomaticMidiTest<Source>,
    observation: Observation,
    scene: LiveDemoScene,
    step_index: usize,
    elapsed: Duration,
    deferred_fixture_elapsed: Duration,
    tick_index: u64,
    pending_checkpoint: Option<PendingCheckpoint>,
    checkpoints: Vec<LiveDemoCheckpoint>,
    coverage: LiveDemoCoverage,
    cleanup_sequence_before: Option<u64>,
    completed_report: Option<LiveDemoReport>,
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
    ) -> Self {
        let coverage = LiveDemoCoverage::new(scene.expected_editable_parameters());
        Self {
            automatic_midi,
            observation,
            scene,
            step_index: 0,
            elapsed: Duration::ZERO,
            deferred_fixture_elapsed: Duration::ZERO,
            tick_index: 0,
            pending_checkpoint: None,
            checkpoints: Vec::new(),
            coverage,
            cleanup_sequence_before: None,
            completed_report: None,
            aborted: false,
        }
    }

    /// Advances at most one scene AppEvent and never sleeps or blocks.
    pub fn advance<Boundary>(
        &mut self,
        elapsed: Duration,
        app_loop: &mut AppLoop<Boundary>,
    ) -> Result<Option<LiveDemoCheckpoint>, LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        if self.completed_report.is_some() || self.aborted {
            return Ok(None);
        }

        self.elapsed = self.elapsed.saturating_add(elapsed);
        self.deferred_fixture_elapsed = self.deferred_fixture_elapsed.saturating_add(elapsed);
        self.tick_index = self.tick_index.saturating_add(1);

        if self.pending_checkpoint.is_some() {
            return self.advance_pending(app_loop);
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
    ) -> Result<Option<LiveDemoCheckpoint>, LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        let before_tree = app_loop.current_state_tree();
        let observed_value = step
            .editable_parameter()
            .map(|parameter| selected_parameter_value(&before_tree, parameter))
            .transpose()?;
        let expected = LiveExpectedTransition::for_step(
            &step,
            before_tree.generation(),
            before_tree.selected_line(),
            observed_value,
        )?;
        let audio_sequence_before = self.observation.read_latest_on_control().sequence();
        let records_before = app_loop.event_log().total_observed();
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

        let event_log = app_loop.event_log();
        if event_log.total_observed() != records_before.saturating_add(1) {
            return Err(LiveDemoError::MissingEventRecord);
        }
        let record = event_log
            .records()
            .last()
            .ok_or(LiveDemoError::MissingEventRecord)?
            .clone();
        verify_record(&step, &expected, &record)?;

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
    ) -> Result<Option<LiveDemoCheckpoint>, LiveDemoError>
    where
        Boundary: ControlAudioBoundary,
    {
        let pending = self
            .pending_checkpoint
            .as_mut()
            .expect("pending state was checked");
        if pending.checkpoint.is_none() {
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
        self.checkpoints.push(checkpoint.clone());
        self.step_index += 1;
        Ok(Some(checkpoint))
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
        self.completed_report = Some(LiveDemoReport::new(
            self.scene.name(),
            self.checkpoints.clone(),
            app_loop.event_log(),
            tree,
            self.coverage.clone(),
            &installed,
            cleanup_sequence_before,
            observation,
        )?);
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
