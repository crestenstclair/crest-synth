use crate::control::event_record::{
    EmittedEvent, EventInput, EventOutcome, EventRecord, EventSource,
};
use crate::control::state_tree::StateTree;
use crate::control::text_projection::TextProjection;
use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
use crate::real_time::GraphRevision;
use crate::synth::CapabilityId;
use crate::testing::live_demo_scene::{
    projected_parameter_values, LiveAudioPredicate, LiveDemoSceneError, LiveExpectedTransition,
};
use core::fmt;
use serde::Serialize;

use crate::control::{
    EngineSelectionRequestId, EngineSelectionStatusKind, PatchControlId, StructuralEditIntent,
};
use crate::kernel::PatchId;

/// Exact selected values copied from the canonical state, text, and RT projections.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveProjectedValue {
    selected_line: usize,
    selected_text: String,
    patch_control_id: Option<PatchControlId>,
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

    pub fn patch_control_id(&self) -> Option<PatchControlId> {
        self.patch_control_id.clone()
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
            return Err(LiveDemoCheckpointError::ExpectedProjectionMismatch {
                step,
                expected_line: expected_transition.selected_line(),
                actual_line: record.selected_line(),
                expected_text: expected_transition
                    .selected_text()
                    .unwrap_or("<missing>")
                    .to_owned(),
                actual_text: selected_text,
            });
        }
        let patch_control_id = expected_transition.patch_control_id();
        if let Some(control) = patch_control_id.as_ref() {
            let control_id = control.as_str();
            let tree: serde_json::Value = serde_json::from_str(state_tree.json())
                .map_err(|_| LiveDemoCheckpointError::CanonicalProjectionMismatch)?;
            if tree
                .pointer("/interaction/activeFocus/controlId/id")
                .and_then(serde_json::Value::as_str)
                != Some(control_id.as_ref())
                || tree
                    .pointer("/patchPage/focusedControlId")
                    .and_then(serde_json::Value::as_str)
                    != Some(control_id.as_ref())
                || !(selected_text.starts_with("> ENVELOPE ")
                    || selected_text.starts_with("> OUTPUT ")
                    || selected_text.starts_with("> EFFECT_PARAMETER "))
            {
                return Err(LiveDemoCheckpointError::CanonicalProjectionMismatch);
            }
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
                patch_control_id,
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
            && self.expected_transition.patch_control_id() == self.projected_value.patch_control_id
            && self.projected_value.state_value == self.projected_value.parameter_value
            && self.audio_observation.parameter_generation() == self.generation
            && self.audio_predicate_passed
    }
}

/// A serialized live checkpoint is either one scalar observation or one
/// engine-selection lifecycle observation. The two evidence sets remain
/// distinct even though the standalone callback presents them uniformly.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LiveCheckpoint {
    Parameter {
        checkpoint: Box<LiveDemoCheckpoint>,
    },
    Engine {
        checkpoint: Box<LiveEngineCheckpoint>,
    },
}

impl LiveCheckpoint {
    pub fn parameter(checkpoint: LiveDemoCheckpoint) -> Self {
        Self::Parameter {
            checkpoint: Box::new(checkpoint),
        }
    }

    pub fn engine(checkpoint: LiveEngineCheckpoint) -> Self {
        Self::Engine {
            checkpoint: Box::new(checkpoint),
        }
    }

    pub const fn as_parameter(&self) -> Option<&LiveDemoCheckpoint> {
        match self {
            Self::Parameter { checkpoint } => Some(checkpoint),
            Self::Engine { .. } => None,
        }
    }

    pub const fn as_engine(&self) -> Option<&LiveEngineCheckpoint> {
        match self {
            Self::Engine { checkpoint } => Some(checkpoint),
            Self::Parameter { .. } => None,
        }
    }

    pub fn agrees(&self) -> bool {
        match self {
            Self::Parameter { checkpoint } => checkpoint.agrees(),
            Self::Engine { checkpoint } => checkpoint.agrees(),
        }
    }
}

/// One immutable observation of a planned live engine transition.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveEngineCheckpoint {
    transition: String,
    transition_index: usize,
    status: EngineSelectionStatusKind,
    request_id: EngineSelectionRequestId,
    patch_id: PatchId,
    focused_control_id: PatchControlId,
    source_capability_id: CapabilityId,
    target_capability_id: CapabilityId,
    intent: StructuralEditIntent,
    preset: Option<LivePresetProjection>,
    active_capability_id: CapabilityId,
    requested_capability_id: Option<CapabilityId>,
    generation: u64,
    state_hash: String,
    graph_revision: GraphRevision,
    handoff_active_revision: Option<GraphRevision>,
    handoff_retired_revision: Option<GraphRevision>,
    staged_revision: Option<GraphRevision>,
    in_flight_revision: Option<GraphRevision>,
    event_sequence: u64,
    source_audio_observation: Option<AudioObservationSnapshot>,
    source_audio_nonzero: bool,
    audio_observation: Option<AudioObservationSnapshot>,
    target_audio_nonzero: bool,
    callback_allocations: usize,
    callback_destructions: usize,
}

/// Exact authored and projected choice evidence for a structural preset replacement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LivePresetProjection {
    source_choice_id: String,
    source_label: String,
    target_choice_id: String,
    target_label: String,
    active_choice_id: String,
    active_label: String,
    requested_choice_id: Option<String>,
    requested_label: Option<String>,
}

impl LivePresetProjection {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_choice_id: impl Into<String>,
        source_label: impl Into<String>,
        target_choice_id: impl Into<String>,
        target_label: impl Into<String>,
        active_choice_id: impl Into<String>,
        active_label: impl Into<String>,
        requested_choice_id: Option<String>,
        requested_label: Option<String>,
    ) -> Self {
        Self {
            source_choice_id: source_choice_id.into(),
            source_label: source_label.into(),
            target_choice_id: target_choice_id.into(),
            target_label: target_label.into(),
            active_choice_id: active_choice_id.into(),
            active_label: active_label.into(),
            requested_choice_id,
            requested_label,
        }
    }

    pub fn source_choice_id(&self) -> &str {
        &self.source_choice_id
    }

    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    pub fn target_choice_id(&self) -> &str {
        &self.target_choice_id
    }

    pub fn target_label(&self) -> &str {
        &self.target_label
    }

    pub fn active_choice_id(&self) -> &str {
        &self.active_choice_id
    }

    pub fn active_label(&self) -> &str {
        &self.active_label
    }

    pub fn requested_choice_id(&self) -> Option<&str> {
        self.requested_choice_id.as_deref()
    }

    pub fn requested_label(&self) -> Option<&str> {
        self.requested_label.as_deref()
    }
}

impl LiveEngineCheckpoint {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        transition: impl Into<String>,
        transition_index: usize,
        status: EngineSelectionStatusKind,
        request_id: EngineSelectionRequestId,
        patch_id: PatchId,
        focused_control_id: PatchControlId,
        source_capability_id: CapabilityId,
        target_capability_id: CapabilityId,
        intent: StructuralEditIntent,
        preset: Option<LivePresetProjection>,
        active_capability_id: CapabilityId,
        requested_capability_id: Option<CapabilityId>,
        generation: u64,
        state_hash: impl Into<String>,
        graph_revision: GraphRevision,
        handoff_active_revision: Option<GraphRevision>,
        handoff_retired_revision: Option<GraphRevision>,
        staged_revision: Option<GraphRevision>,
        in_flight_revision: Option<GraphRevision>,
        event_sequence: u64,
        source_audio_observation: Option<AudioObservationSnapshot>,
        audio_observation: Option<AudioObservationSnapshot>,
        callback_allocations: usize,
        callback_destructions: usize,
    ) -> Result<Self, LiveDemoCheckpointError> {
        let source_audio_nonzero = source_audio_observation.is_some_and(|observation| {
            let effect = observation.patch_effect();
            audio_fields_are_finite(observation)
                && observation.primary_patch_id() == Some(patch_id)
                && observation.primary_active_notes() > 0
                && observation.primary_patch_rms() > 0.0
                && effect.patch_id() == Some(patch_id)
                && effect.input_rms() > 0.0
                && effect.output_rms() > 0.0
                && effect.difference_rms() > 0.0
                && effect.side_rms() > 0.0
        });
        let target_audio_nonzero = audio_observation.is_some_and(|observation| {
            let effect = observation.patch_effect();
            audio_fields_are_finite(observation)
                && observation.active_graph_revision() == graph_revision
                && observation.parameter_generation() == generation
                && observation.active_notes() > 0
                && observation.primary_patch_id() == Some(patch_id)
                && observation.primary_active_notes() > 0
                && observation.primary_patch_rms() > 0.0
                && effect.patch_id() == Some(patch_id)
                && effect.input_rms() > 0.0
                && effect.output_rms() > 0.0
                && effect.difference_rms() > 0.0
                && effect.side_rms() > 0.0
        });
        let checkpoint = Self {
            transition: transition.into(),
            transition_index,
            status,
            request_id,
            patch_id,
            focused_control_id,
            source_capability_id,
            target_capability_id,
            intent,
            preset,
            active_capability_id,
            requested_capability_id,
            generation,
            state_hash: state_hash.into(),
            graph_revision,
            handoff_active_revision,
            handoff_retired_revision,
            staged_revision,
            in_flight_revision,
            event_sequence,
            source_audio_observation,
            source_audio_nonzero,
            audio_observation,
            target_audio_nonzero,
            callback_allocations,
            callback_destructions,
        };
        if !checkpoint.agrees() {
            return Err(LiveDemoCheckpointError::EngineLifecycleMismatch);
        }
        Ok(checkpoint)
    }

    pub fn transition(&self) -> &str {
        &self.transition
    }

    pub const fn transition_index(&self) -> usize {
        self.transition_index
    }

    pub const fn status(&self) -> EngineSelectionStatusKind {
        self.status
    }

    pub const fn request_id(&self) -> EngineSelectionRequestId {
        self.request_id
    }

    pub const fn patch_id(&self) -> PatchId {
        self.patch_id
    }

    pub fn focused_control_id(&self) -> PatchControlId {
        self.focused_control_id.clone()
    }

    pub const fn source_capability_id(&self) -> &CapabilityId {
        &self.source_capability_id
    }

    pub const fn target_capability_id(&self) -> &CapabilityId {
        &self.target_capability_id
    }

    pub const fn intent(&self) -> &StructuralEditIntent {
        &self.intent
    }

    pub const fn preset(&self) -> Option<&LivePresetProjection> {
        self.preset.as_ref()
    }

    pub const fn graph_revision(&self) -> GraphRevision {
        self.graph_revision
    }

    pub const fn audio_observation(&self) -> Option<AudioObservationSnapshot> {
        self.audio_observation
    }

    pub const fn source_audio_observation(&self) -> Option<AudioObservationSnapshot> {
        self.source_audio_observation
    }

    pub const fn source_audio_nonzero(&self) -> bool {
        self.source_audio_nonzero
    }

    pub const fn target_audio_nonzero(&self) -> bool {
        self.target_audio_nonzero
    }

    pub const fn callback_allocations(&self) -> usize {
        self.callback_allocations
    }

    pub const fn callback_destructions(&self) -> usize {
        self.callback_destructions
    }

    pub fn agrees(&self) -> bool {
        let expected_control = match &self.intent {
            StructuralEditIntent::ReplaceCapability { .. } => PatchControlId::Engine,
            StructuralEditIntent::ReplaceParameterChoice { parameter_id, .. } => {
                PatchControlId::Capability(parameter_id.clone())
            }
        };
        if self.transition.trim().is_empty()
            || self.request_id.is_none()
            || self.state_hash.is_empty()
            || self.focused_control_id != expected_control
            || self.callback_allocations != 0
            || self.callback_destructions != 0
        {
            return false;
        }
        let target_pending = self.staged_revision == Some(self.graph_revision)
            || self.in_flight_revision == Some(self.graph_revision);
        let capability_projection_exact = match &self.intent {
            StructuralEditIntent::ReplaceCapability {
                target_capability_id,
            } => {
                target_capability_id == &self.target_capability_id
                    && self.source_capability_id != self.target_capability_id
                    && self.preset.is_none()
                    && self.source_audio_observation.is_none()
                    && !self.source_audio_nonzero
                    && self.active_capability_id
                        == if self.status == EngineSelectionStatusKind::Preparing {
                            self.source_capability_id.clone()
                        } else {
                            self.target_capability_id.clone()
                        }
                    && self.requested_capability_id
                        == (self.status != EngineSelectionStatusKind::Ready)
                            .then_some(self.target_capability_id.clone())
            }
            StructuralEditIntent::ReplaceParameterChoice {
                capability_id,
                parameter_id: _,
                choice_id,
            } => {
                let Some(preset) = &self.preset else {
                    return false;
                };
                capability_id == &self.source_capability_id
                    && self.source_capability_id == self.target_capability_id
                    && self.active_capability_id == self.source_capability_id
                    && self.requested_capability_id.is_none()
                    && choice_id == preset.target_choice_id()
                    && preset.active_choice_id()
                        == if self.status == EngineSelectionStatusKind::Preparing {
                            preset.source_choice_id()
                        } else {
                            preset.target_choice_id()
                        }
                    && preset.requested_choice_id()
                        == (self.status != EngineSelectionStatusKind::Ready)
                            .then_some(preset.target_choice_id())
                    && preset.requested_label()
                        == (self.status != EngineSelectionStatusKind::Ready)
                            .then_some(preset.target_label())
                    && !preset.source_label().is_empty()
                    && !preset.target_label().is_empty()
                    && !preset.active_label().is_empty()
                    && (self.status != EngineSelectionStatusKind::Preparing
                        || self.source_audio_nonzero)
            }
        };
        capability_projection_exact
            && match self.status {
                EngineSelectionStatusKind::Preparing => {
                    self.handoff_active_revision == Some(self.graph_revision)
                        && self.staged_revision.is_none()
                        && self.in_flight_revision.is_none()
                        && self.audio_observation.is_none()
                        && !self.target_audio_nonzero
                }
                EngineSelectionStatusKind::Activating => {
                    self.handoff_active_revision.is_some()
                        && target_pending
                        && self.audio_observation.is_none()
                        && !self.target_audio_nonzero
                }
                EngineSelectionStatusKind::Ready => {
                    self.handoff_active_revision == Some(self.graph_revision)
                        && self.staged_revision.is_none()
                        && self.in_flight_revision.is_none()
                        && self.audio_observation.is_some()
                        && self.target_audio_nonzero
                }
                EngineSelectionStatusKind::Failed => false,
            }
    }
}

fn audio_fields_are_finite(observation: AudioObservationSnapshot) -> bool {
    let effect = observation.patch_effect();
    observation.left_peak().is_finite()
        && observation.right_peak().is_finite()
        && observation.output_rms().is_finite()
        && observation.reverb_input_rms().is_finite()
        && observation.delay_input_rms().is_finite()
        && observation.wet_output_rms().is_finite()
        && effect.input_rms().is_finite()
        && effect.output_rms().is_finite()
        && effect.difference_rms().is_finite()
        && effect.side_rms().is_finite()
        && observation.non_finite_samples() == 0
}

/// A falsifiable mismatch while constructing one live checkpoint.
#[derive(Clone, Debug, PartialEq)]
pub enum LiveDemoCheckpointError {
    WrongEventSource,
    TransitionMismatch,
    CanonicalProjectionMismatch,
    ExpectedProjectionMismatch {
        step: usize,
        expected_line: Option<usize>,
        actual_line: usize,
        expected_text: String,
        actual_text: String,
    },
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
    EngineLifecycleMismatch,
    Scene(LiveDemoSceneError),
}

impl From<LiveDemoSceneError> for LiveDemoCheckpointError {
    fn from(error: LiveDemoSceneError) -> Self {
        Self::Scene(error)
    }
}

impl fmt::Display for LiveDemoCheckpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongEventSource => formatter.write_str("checkpoint event did not come from DemoScene"),
            Self::TransitionMismatch => formatter.write_str("actual transition differs from its pre-dispatch expectation"),
            Self::CanonicalProjectionMismatch => formatter.write_str("event record, StateTree, and TextProjection are not one canonical generation"),
            Self::ExpectedProjectionMismatch {
                step,
                expected_line,
                actual_line,
                expected_text,
                actual_text,
            } => write!(
                formatter,
                "selected TextProjection at live step {step} differs from its pre-dispatch expectation: line {actual_line} (expected {expected_line:?}), text {actual_text:?} (expected {expected_text:?})"
            ),
            Self::MissingEditableParameter => formatter.write_str("parameter checkpoint has no editable parameter expectation"),
            Self::ProjectedValueMismatch { expected, state, parameters } => write!(formatter, "expected value {expected}, StateTree has {state}, ParameterSnapshot has {parameters}"),
            Self::StaleAudioObservation => formatter.write_str("audio observation did not advance after dispatch"),
            Self::AudioGenerationMismatch { expected, actual } => write!(formatter, "audio observation generation {actual} does not match checkpoint generation {expected}"),
            Self::NonFiniteAudioObservation => formatter.write_str("audio observation contains non-finite output"),
            Self::AudioPredicateFailed(predicate) => write!(formatter, "audio observation did not satisfy {predicate:?}"),
            Self::EngineLifecycleMismatch => formatter.write_str(
                "engine checkpoint does not match its lifecycle, revision, or target-audio contract",
            ),
            Self::Scene(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for LiveDemoCheckpointError {}
