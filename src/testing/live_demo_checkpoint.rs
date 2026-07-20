use crate::control::event_record::{
    EmittedEvent, EventInput, EventOutcome, EventRecord, EventSource,
};
use crate::control::state_tree::StateTree;
use crate::control::text_projection::TextProjection;
use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
use crate::testing::live_demo_scene::{
    projected_parameter_values, LiveAudioPredicate, LiveDemoSceneError, LiveExpectedTransition,
};
use core::fmt;
use serde::Serialize;

/// Exact selected values copied from the canonical state, text, and RT projections.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveProjectedValue {
    selected_line: usize,
    selected_text: String,
    state_value: f32,
    parameter_value: f32,
}

impl LiveProjectedValue {
    pub const fn selected_line(&self) -> usize {
        self.selected_line
    }

    pub fn selected_text(&self) -> &str {
        &self.selected_text
    }

    pub const fn state_value(&self) -> f32 {
        self.state_value
    }

    pub const fn parameter_value(&self) -> f32 {
        self.parameter_value
    }
}

/// One immutable production-path correlation for an accepted parameter edit.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveDemoCheckpoint {
    step: usize,
    input: EventInput,
    expected_transition: LiveExpectedTransition,
    outcome: EventOutcome,
    generation: u64,
    state_hash: String,
    projected_value: LiveProjectedValue,
    parameter_generation: u64,
    emitted_effects: Vec<EmittedEvent>,
    audio_observation: AudioObservationSnapshot,
    audio_predicate: LiveAudioPredicate,
    audio_predicate_passed: bool,
}

impl LiveDemoCheckpoint {
    /// Correlates independently frozen expectations with canonical values and
    /// a newer callback observation. No expectation is derived from `record`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        step: usize,
        expected_transition: LiveExpectedTransition,
        record: &EventRecord,
        state_tree: &StateTree,
        text: &TextProjection,
        audio_sequence_before: u64,
        audio_observation: AudioObservationSnapshot,
        audio_predicate: LiveAudioPredicate,
    ) -> Result<Self, LiveDemoCheckpointError> {
        if record.source() != EventSource::DemoScene {
            return Err(LiveDemoCheckpointError::WrongEventSource);
        }
        if record.input() != expected_transition.input()
            || record.outcome() != expected_transition.outcome()
            || record.generation_before() != expected_transition.generation_before()
            || record.generation_after() != expected_transition.generation_after()
            || record.parameter_generation() != expected_transition.parameter_generation()
            || record.emitted_events() != expected_transition.emitted_effects()
        {
            return Err(LiveDemoCheckpointError::TransitionMismatch);
        }
        if record.rejection().map(|value| value.name()) != expected_transition.rejection() {
            return Err(LiveDemoCheckpointError::TransitionMismatch);
        }
        if state_tree.generation() != record.generation_after()
            || state_tree.state_hash() != record.state_hash_after()
            || state_tree.selected_line() != record.selected_line()
            || text.state_hash() != record.projection_state_hash()
            || text.selected_line() != record.selected_line()
            || record.parameter_generation() != record.generation_after()
        {
            return Err(LiveDemoCheckpointError::CanonicalProjectionMismatch);
        }

        let parameter = expected_transition
            .editable_parameter()
            .ok_or(LiveDemoCheckpointError::MissingEditableParameter)?;
        let expected_value = expected_transition
            .value_after()
            .ok_or(LiveDemoCheckpointError::MissingEditableParameter)?;
        let (state_value, parameter_value) = projected_parameter_values(state_tree, parameter)?;
        if state_value != expected_value || parameter_value != expected_value {
            return Err(LiveDemoCheckpointError::ProjectedValueMismatch {
                expected: expected_value,
                state: state_value,
                parameters: parameter_value,
            });
        }

        if audio_observation.sequence() <= audio_sequence_before {
            return Err(LiveDemoCheckpointError::StaleAudioObservation);
        }
        if audio_observation.parameter_generation() != record.parameter_generation() {
            return Err(LiveDemoCheckpointError::AudioGenerationMismatch {
                expected: record.parameter_generation(),
                actual: audio_observation.parameter_generation(),
            });
        }
        if !audio_fields_are_finite(audio_observation) {
            return Err(LiveDemoCheckpointError::NonFiniteAudioObservation);
        }
        let audio_predicate_passed = audio_predicate.evaluate(audio_observation);
        if !audio_predicate_passed {
            return Err(LiveDemoCheckpointError::AudioPredicateFailed(
                audio_predicate,
            ));
        }

        let selected_text = text
            .body()
            .lines()
            .nth(text.selected_line())
            .ok_or(LiveDemoCheckpointError::CanonicalProjectionMismatch)?
            .to_owned();
        if expected_transition.selected_line() != Some(record.selected_line())
            || expected_transition.selected_text() != Some(selected_text.as_str())
        {
            return Err(LiveDemoCheckpointError::ExpectedProjectionMismatch);
        }

        Ok(Self {
            step,
            input: record.input().clone(),
            expected_transition,
            outcome: record.outcome(),
            generation: record.generation_after(),
            state_hash: record.state_hash_after().to_owned(),
            projected_value: LiveProjectedValue {
                selected_line: record.selected_line(),
                selected_text,
                state_value,
                parameter_value,
            },
            parameter_generation: record.parameter_generation(),
            emitted_effects: record.emitted_events().to_vec(),
            audio_observation,
            audio_predicate,
            audio_predicate_passed,
        })
    }

    pub const fn step(&self) -> usize {
        self.step
    }

    pub const fn expected_transition(&self) -> &LiveExpectedTransition {
        &self.expected_transition
    }

    pub const fn outcome(&self) -> EventOutcome {
        self.outcome
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn state_hash(&self) -> &str {
        &self.state_hash
    }

    pub const fn projected_value(&self) -> &LiveProjectedValue {
        &self.projected_value
    }

    pub const fn parameter_generation(&self) -> u64 {
        self.parameter_generation
    }

    pub fn emitted_effects(&self) -> &[EmittedEvent] {
        &self.emitted_effects
    }

    pub const fn audio_observation(&self) -> AudioObservationSnapshot {
        self.audio_observation
    }

    pub const fn audio_predicate(&self) -> LiveAudioPredicate {
        self.audio_predicate
    }

    pub fn agrees(&self) -> bool {
        self.input.eq(self.expected_transition.input())
            && self.outcome == self.expected_transition.outcome()
            && self.generation == self.expected_transition.generation_after()
            && self.parameter_generation == self.expected_transition.parameter_generation()
            && self.emitted_effects.as_slice() == self.expected_transition.emitted_effects()
            && self.expected_transition.selected_line() == Some(self.projected_value.selected_line)
            && self.expected_transition.selected_text()
                == Some(self.projected_value.selected_text.as_str())
            && self.projected_value.state_value == self.projected_value.parameter_value
            && self.audio_observation.parameter_generation() == self.generation
            && self.audio_predicate_passed
    }
}

fn audio_fields_are_finite(observation: AudioObservationSnapshot) -> bool {
    observation.left_peak().is_finite()
        && observation.right_peak().is_finite()
        && observation.output_rms().is_finite()
        && observation.reverb_input_rms().is_finite()
        && observation.delay_input_rms().is_finite()
        && observation.wet_output_rms().is_finite()
        && observation.non_finite_samples() == 0
}

/// A falsifiable mismatch while constructing one live checkpoint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LiveDemoCheckpointError {
    WrongEventSource,
    TransitionMismatch,
    CanonicalProjectionMismatch,
    ExpectedProjectionMismatch,
    MissingEditableParameter,
    ProjectedValueMismatch {
        expected: f32,
        state: f32,
        parameters: f32,
    },
    StaleAudioObservation,
    AudioGenerationMismatch {
        expected: u64,
        actual: u64,
    },
    NonFiniteAudioObservation,
    AudioPredicateFailed(LiveAudioPredicate),
    Scene(LiveDemoSceneError),
}

impl From<LiveDemoSceneError> for LiveDemoCheckpointError {
    fn from(error: LiveDemoSceneError) -> Self {
        Self::Scene(error)
    }
}

impl fmt::Display for LiveDemoCheckpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::WrongEventSource => formatter.write_str("checkpoint event did not come from DemoScene"),
            Self::TransitionMismatch => formatter.write_str("actual transition differs from its pre-dispatch expectation"),
            Self::CanonicalProjectionMismatch => formatter.write_str("event record, StateTree, and TextProjection are not one canonical generation"),
            Self::ExpectedProjectionMismatch => formatter.write_str("selected TextProjection differs from its pre-dispatch expectation"),
            Self::MissingEditableParameter => formatter.write_str("parameter checkpoint has no editable parameter expectation"),
            Self::ProjectedValueMismatch { expected, state, parameters } => write!(formatter, "expected value {expected}, StateTree has {state}, ParameterSnapshot has {parameters}"),
            Self::StaleAudioObservation => formatter.write_str("audio observation did not advance after dispatch"),
            Self::AudioGenerationMismatch { expected, actual } => write!(formatter, "audio observation generation {actual} does not match checkpoint generation {expected}"),
            Self::NonFiniteAudioObservation => formatter.write_str("audio observation contains non-finite output"),
            Self::AudioPredicateFailed(predicate) => write!(formatter, "audio observation did not satisfy {predicate:?}"),
            Self::Scene(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for LiveDemoCheckpointError {}
