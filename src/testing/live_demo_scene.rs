use crate::control::app_event::{AppEvent, Direction};
use crate::control::app_state::EventRejection;
use crate::control::event_record::{EmittedEvent, EventInput, EventOutcome};
use crate::control::state_tree::StateTree;
use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_message::MidiMessage;
use crate::kernel::patch_id::PatchId;
use crate::mixer::channel_parameters::{ChannelParameter, ChannelParameters};
use crate::mixer::global_parameters::{GlobalParameter, GlobalParameters};
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
use crate::real_time::GraphRevision;
use core::fmt;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// One canonical editable value in the installed live-demo surface.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "scope", rename_all = "camelCase")]
pub enum LiveEditableParameter {
    Patch {
        #[serde(rename = "patchId")]
        patch_id: PatchId,
        parameter: ChannelParameter,
    },
    Global {
        parameter: GlobalParameter,
    },
}

impl LiveEditableParameter {
    pub const fn patch(patch_id: PatchId, parameter: ChannelParameter) -> Self {
        Self::Patch {
            patch_id,
            parameter,
        }
    }

    pub const fn global(parameter: GlobalParameter) -> Self {
        Self::Global { parameter }
    }

    /// Returns a stable descriptor-derived coverage identifier.
    pub fn identifier(self) -> String {
        match self {
            Self::Patch {
                patch_id,
                parameter,
            } => format!("patch.{}.{parameter}", patch_id.value()),
            Self::Global { parameter } => format!("global.{parameter}"),
        }
    }

    pub const fn audio_predicate(self) -> LiveAudioPredicate {
        match self {
            Self::Patch {
                parameter: ChannelParameter::GainDb,
                ..
            }
            | Self::Global {
                parameter: GlobalParameter::MasterGainDb,
            } => LiveAudioPredicate::OutputLevel,
            Self::Patch {
                parameter: ChannelParameter::Pan,
                ..
            } => LiveAudioPredicate::StereoBalance,
            Self::Patch {
                parameter: ChannelParameter::ReverbSend,
                ..
            } => LiveAudioPredicate::ReverbInput,
            Self::Patch {
                parameter: ChannelParameter::DelaySend,
                ..
            } => LiveAudioPredicate::DelayInput,
            Self::Global { .. } => LiveAudioPredicate::WetOutput,
        }
    }

    pub const fn field_name(self) -> &'static str {
        match self {
            Self::Patch { parameter, .. } => parameter.name(),
            Self::Global { parameter } => parameter.name(),
        }
    }
}

/// The causal measurement required for one editable parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LiveAudioPredicate {
    OutputLevel,
    StereoBalance,
    ReverbInput,
    DelayInput,
    WetOutput,
}

impl LiveAudioPredicate {
    /// Tests only finite fields measured by the production render path.
    pub fn evaluate(self, observation: AudioObservationSnapshot) -> bool {
        let finite = observation.left_peak().is_finite()
            && observation.right_peak().is_finite()
            && observation.output_rms().is_finite()
            && observation.reverb_input_rms().is_finite()
            && observation.delay_input_rms().is_finite()
            && observation.wet_output_rms().is_finite()
            && observation.non_finite_samples() == 0;
        if !finite {
            return false;
        }

        match self {
            Self::OutputLevel => observation.output_rms() > 0.0,
            Self::StereoBalance => {
                observation.output_rms() > 0.0
                    && (observation.left_peak() - observation.right_peak()).abs() > f32::EPSILON
            }
            Self::ReverbInput => observation.reverb_input_rms() > 0.0,
            Self::DelayInput => observation.delay_input_rms() > 0.0,
            Self::WetOutput => observation.wet_output_rms() > 0.0,
        }
    }
}

/// A transition oracle fixed before its semantic event is dispatched.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveExpectedTransition {
    input: EventInput,
    outcome: EventOutcome,
    generation_before: u64,
    generation_after: u64,
    parameter_generation: u64,
    editable_parameter: Option<LiveEditableParameter>,
    value_before: Option<f32>,
    value_after: Option<f32>,
    selected_line: Option<usize>,
    selected_text: Option<String>,
    emitted_effects: Vec<EmittedEvent>,
    rejection: Option<String>,
}

impl LiveExpectedTransition {
    pub(crate) fn for_step(
        step: &LiveDemoStep,
        generation_before: u64,
        graph_revision: GraphRevision,
        selected_line: usize,
        observed_value: Option<f32>,
    ) -> Result<Self, LiveDemoSceneError> {
        if let Some(expected_before) = step.value_before {
            let actual = observed_value.ok_or(LiveDemoSceneError::SelectedParameterMismatch)?;
            if actual != expected_before {
                return Err(LiveDemoSceneError::ExpectedValueMismatch {
                    expected: expected_before,
                    actual,
                });
            }
        }

        let generation_after = if step.expected_outcome == EventOutcome::Accepted {
            generation_before
                .checked_add(1)
                .ok_or(LiveDemoSceneError::GenerationOverflow)?
        } else {
            generation_before
        };

        let selected_text = step
            .editable_parameter
            .zip(step.value_after)
            .map(|(parameter, value)| format!("> {}={value}", parameter.field_name()));
        let selected_line = step.editable_parameter.map(|_| selected_line);
        let mut emitted_effects = Vec::new();
        if step.expected_outcome == EventOutcome::Accepted {
            emitted_effects.push(EmittedEvent::StateAccepted {
                generation: generation_after,
            });
            emitted_effects.push(EmittedEvent::ParameterSnapshotPublished {
                generation: generation_after,
                graph_revision,
            });
            if let AppEvent::Midi { patch_id, message } = &step.event {
                emitted_effects.push(EmittedEvent::AudioCommand {
                    effect: AudioCommand::patch_midi(*patch_id, *message).into(),
                });
            }
        }

        Ok(Self {
            input: EventInput::from(&step.event),
            outcome: step.expected_outcome,
            generation_before,
            generation_after,
            parameter_generation: generation_after,
            editable_parameter: step.editable_parameter,
            value_before: step.value_before,
            value_after: step.value_after,
            selected_line,
            selected_text,
            emitted_effects,
            rejection: step.expected_rejection.map(|value| value.name().to_owned()),
        })
    }

    pub const fn input(&self) -> &EventInput {
        &self.input
    }

    pub const fn outcome(&self) -> EventOutcome {
        self.outcome
    }

    pub const fn generation_before(&self) -> u64 {
        self.generation_before
    }

    pub const fn generation_after(&self) -> u64 {
        self.generation_after
    }

    pub const fn parameter_generation(&self) -> u64 {
        self.parameter_generation
    }

    pub const fn editable_parameter(&self) -> Option<LiveEditableParameter> {
        self.editable_parameter
    }

    pub const fn value_before(&self) -> Option<f32> {
        self.value_before
    }

    pub const fn value_after(&self) -> Option<f32> {
        self.value_after
    }

    pub const fn selected_line(&self) -> Option<usize> {
        self.selected_line
    }

    pub fn selected_text(&self) -> Option<&str> {
        self.selected_text.as_deref()
    }

    pub fn emitted_effects(&self) -> &[EmittedEvent] {
        &self.emitted_effects
    }

    pub fn rejection(&self) -> Option<&str> {
        self.rejection.as_deref()
    }
}

/// One bounded semantic action in the live scene.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveDemoStep {
    event: AppEvent,
    expected_outcome: EventOutcome,
    expected_rejection: Option<EventRejection>,
    editable_parameter: Option<LiveEditableParameter>,
    value_before: Option<f32>,
    value_after: Option<f32>,
    checkpoint: bool,
    cleanup: bool,
}

impl LiveDemoStep {
    fn accepted_event(event: AppEvent) -> Self {
        Self {
            event,
            expected_outcome: EventOutcome::Accepted,
            expected_rejection: None,
            editable_parameter: None,
            value_before: None,
            value_after: None,
            checkpoint: false,
            cleanup: false,
        }
    }

    fn adjustment(
        event: AppEvent,
        parameter: LiveEditableParameter,
        value_before: f32,
        value_after: f32,
    ) -> Self {
        Self {
            event,
            expected_outcome: EventOutcome::Accepted,
            expected_rejection: None,
            editable_parameter: Some(parameter),
            value_before: Some(value_before),
            value_after: Some(value_after),
            checkpoint: true,
            cleanup: false,
        }
    }

    fn rejected_adjustment(
        event: AppEvent,
        parameter: LiveEditableParameter,
        value: f32,
        rejection: EventRejection,
    ) -> Self {
        Self {
            event,
            expected_outcome: EventOutcome::Rejected,
            expected_rejection: Some(rejection),
            editable_parameter: Some(parameter),
            value_before: Some(value),
            value_after: Some(value),
            checkpoint: false,
            cleanup: false,
        }
    }

    fn cleanup(event: AppEvent) -> Self {
        let mut step = Self::accepted_event(event);
        step.cleanup = true;
        step
    }

    pub fn event(&self) -> &AppEvent {
        &self.event
    }

    pub const fn expected_outcome(&self) -> EventOutcome {
        self.expected_outcome
    }

    pub const fn expected_rejection(&self) -> Option<EventRejection> {
        self.expected_rejection
    }

    pub const fn editable_parameter(&self) -> Option<LiveEditableParameter> {
        self.editable_parameter
    }

    pub const fn value_before(&self) -> Option<f32> {
        self.value_before
    }

    pub const fn value_after(&self) -> Option<f32> {
        self.value_after
    }

    pub const fn requires_checkpoint(&self) -> bool {
        self.checkpoint
    }

    pub const fn is_cleanup(&self) -> bool {
        self.cleanup
    }
}

/// A bounded descriptor-derived scene for the installed production fixture.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveDemoScene {
    name: String,
    minimum_parameter_dwell: Duration,
    steps: Vec<LiveDemoStep>,
    expected_editable_parameters: Vec<LiveEditableParameter>,
    patches: Vec<LivePatch>,
}

impl LiveDemoScene {
    pub const SCHEMA_VERSION: u32 = 1;
    pub const MINIMUM_PARAMETER_DWELL: Duration = Duration::from_millis(500);

    /// Freezes the installed patch and current typed descriptor surface.
    pub fn from_installed_state(tree: &StateTree) -> Result<Self, LiveDemoSceneError> {
        let state = decode_state_tree(tree)?;
        if state.patches.is_empty() {
            return Err(LiveDemoSceneError::NoInstalledPatches);
        }
        if state.selection.section != "Patch"
            || state.selection.patch_index != 0
            || state.selection.parameter_index != 0
        {
            return Err(LiveDemoSceneError::UnexpectedInitialSelection);
        }

        let mut patches = Vec::with_capacity(state.patches.len());
        let mut expected = Vec::with_capacity(
            state.patches.len() * ChannelParameters::surface_descriptor().len()
                + GlobalParameters::surface_descriptor().len(),
        );
        for patch in &state.patches {
            let patch_id =
                PatchId::new(patch.id).map_err(|_| LiveDemoSceneError::InvalidPatchId)?;
            if patches
                .iter()
                .any(|item: &LivePatch| item.patch_id == patch_id)
            {
                return Err(LiveDemoSceneError::DuplicatePatchId(patch.id));
            }
            let channel = MidiChannel::new(patch.channel)
                .map_err(|_| LiveDemoSceneError::InvalidMidiChannel(patch.channel))?;
            patches.push(LivePatch { patch_id, channel });
            expected.extend(
                ChannelParameters::surface_descriptor()
                    .iter()
                    .map(|descriptor| {
                        LiveEditableParameter::patch(patch_id, descriptor.parameter())
                    }),
            );
        }
        expected.extend(
            GlobalParameters::surface_descriptor()
                .iter()
                .map(|descriptor| LiveEditableParameter::global(descriptor.parameter())),
        );

        let mut steps = Vec::new();
        build_patch_steps(&state, &patches, &mut steps)?;
        build_global_steps(&state, &mut steps)?;
        for patch in &patches {
            steps.push(LiveDemoStep::cleanup(AppEvent::Midi {
                patch_id: patch.patch_id,
                message: MidiMessage::all_notes_off(patch.channel),
            }));
        }

        Ok(Self {
            name: "phase-1-live-observable-demo".to_owned(),
            minimum_parameter_dwell: Self::MINIMUM_PARAMETER_DWELL,
            steps,
            expected_editable_parameters: expected,
            patches,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn schema_version(&self) -> u32 {
        Self::SCHEMA_VERSION
    }

    pub const fn minimum_parameter_dwell(&self) -> Duration {
        self.minimum_parameter_dwell
    }

    pub fn steps(&self) -> &[LiveDemoStep] {
        &self.steps
    }

    pub fn expected_editable_parameters(&self) -> &[LiveEditableParameter] {
        &self.expected_editable_parameters
    }

    pub fn patch_ids(&self) -> impl ExactSizeIterator<Item = PatchId> + '_ {
        self.patches.iter().map(|patch| patch.patch_id)
    }

    pub fn required_event_log_capacity(&self, fixture_allowance: usize) -> usize {
        1usize
            .saturating_add(self.steps.len())
            .saturating_add(fixture_allowance)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LivePatch {
    patch_id: PatchId,
    channel: MidiChannel,
}

fn build_patch_steps(
    state: &DecodedStateTree,
    patches: &[LivePatch],
    steps: &mut Vec<LiveDemoStep>,
) -> Result<(), LiveDemoSceneError> {
    for (patch_index, (patch, identity)) in state.patches.iter().zip(patches).enumerate() {
        for (parameter_index, descriptor) in
            ChannelParameters::surface_descriptor().iter().enumerate()
        {
            let editable = LiveEditableParameter::patch(identity.patch_id, descriptor.parameter());
            let mut value = patch.parameters.value(descriptor.parameter());

            if patch_index == 0 && parameter_index == 0 {
                while value < descriptor.maximum() {
                    let next = adjusted_value(
                        value,
                        descriptor.minimum(),
                        descriptor.maximum(),
                        Direction::Up,
                        descriptor.fine_step(),
                        descriptor.coarse_step(),
                    )?;
                    steps.push(LiveDemoStep::adjustment(
                        AppEvent::Adjust(Direction::Up),
                        editable,
                        value,
                        next,
                    ));
                    value = next;
                }
                steps.push(LiveDemoStep::rejected_adjustment(
                    AppEvent::Adjust(Direction::Up),
                    editable,
                    value,
                    EventRejection::ParameterAtBoundary,
                ));
                let next = adjusted_value(
                    value,
                    descriptor.minimum(),
                    descriptor.maximum(),
                    Direction::Down,
                    descriptor.fine_step(),
                    descriptor.coarse_step(),
                )?;
                steps.push(LiveDemoStep::adjustment(
                    AppEvent::Adjust(Direction::Down),
                    editable,
                    value,
                    next,
                ));
            } else {
                let direction = if value < descriptor.maximum() {
                    Direction::Right
                } else {
                    Direction::Left
                };
                let next = adjusted_value(
                    value,
                    descriptor.minimum(),
                    descriptor.maximum(),
                    direction,
                    descriptor.fine_step(),
                    descriptor.coarse_step(),
                )?;
                steps.push(LiveDemoStep::adjustment(
                    AppEvent::Adjust(direction),
                    editable,
                    value,
                    next,
                ));
            }

            if parameter_index + 1 < ChannelParameters::surface_descriptor().len() {
                steps.push(LiveDemoStep::accepted_event(AppEvent::Navigate(
                    Direction::Down,
                )));
            }
        }

        // Wrap delaySend back to gainDb before moving to the next section, so
        // Patch-to-Patch and Patch-to-GLOBAL transitions preserve index zero.
        steps.push(LiveDemoStep::accepted_event(AppEvent::Navigate(
            Direction::Down,
        )));
        steps.push(LiveDemoStep::accepted_event(AppEvent::Navigate(
            Direction::Right,
        )));
    }
    Ok(())
}

fn build_global_steps(
    state: &DecodedStateTree,
    steps: &mut Vec<LiveDemoStep>,
) -> Result<(), LiveDemoSceneError> {
    for (index, descriptor) in GlobalParameters::surface_descriptor().iter().enumerate() {
        let editable = LiveEditableParameter::global(descriptor.parameter());
        let value = state.global.value(descriptor.parameter());
        let direction = if value < descriptor.maximum() {
            Direction::Right
        } else {
            Direction::Left
        };
        let next = adjusted_value(
            value,
            descriptor.minimum(),
            descriptor.maximum(),
            direction,
            descriptor.fine_step(),
            descriptor.coarse_step(),
        )?;
        steps.push(LiveDemoStep::adjustment(
            AppEvent::Adjust(direction),
            editable,
            value,
            next,
        ));
        if index + 1 < GlobalParameters::surface_descriptor().len() {
            steps.push(LiveDemoStep::accepted_event(AppEvent::Navigate(
                Direction::Down,
            )));
        }
    }
    Ok(())
}

fn adjusted_value(
    current: f32,
    minimum: f32,
    maximum: f32,
    direction: Direction,
    fine_step: f32,
    coarse_step: f32,
) -> Result<f32, LiveDemoSceneError> {
    let scale = decimal_scale(fine_step);
    let current_units = (current * scale).round();
    let fine_units = (fine_step * scale).round();
    let coarse_units = (coarse_step * scale).round();
    let delta = match direction {
        Direction::Left => -fine_units,
        Direction::Right => fine_units,
        Direction::Down => -coarse_units,
        Direction::Up => coarse_units,
    };
    let adjusted = ((current_units + delta) / scale).clamp(minimum, maximum);
    if adjusted == current {
        Err(LiveDemoSceneError::InvalidPlannedAdjustment)
    } else {
        Ok(adjusted)
    }
}

fn decimal_scale(step: f32) -> f32 {
    let mut scale = 1.0;
    while scale < 1_000_000.0 && (step * scale - (step * scale).round()).abs() > f32::EPSILON {
        scale *= 10.0;
    }
    scale
}

pub(crate) fn selected_parameter_value(
    tree: &StateTree,
    parameter: LiveEditableParameter,
) -> Result<f32, LiveDemoSceneError> {
    let state = decode_state_tree(tree)?;
    match parameter {
        LiveEditableParameter::Patch {
            patch_id,
            parameter,
        } => {
            if state.selection.section != "Patch"
                || state
                    .patches
                    .get(state.selection.patch_index)
                    .is_none_or(|patch| patch.id != patch_id.value())
                || ChannelParameters::surface_descriptor()
                    .get(state.selection.parameter_index)
                    .is_none_or(|descriptor| descriptor.parameter() != parameter)
            {
                return Err(LiveDemoSceneError::SelectedParameterMismatch);
            }
            Ok(state.patches[state.selection.patch_index]
                .parameters
                .value(parameter))
        }
        LiveEditableParameter::Global { parameter } => {
            if state.selection.section != "Global"
                || GlobalParameters::surface_descriptor()
                    .get(state.selection.parameter_index)
                    .is_none_or(|descriptor| descriptor.parameter() != parameter)
            {
                return Err(LiveDemoSceneError::SelectedParameterMismatch);
            }
            Ok(state.global.value(parameter))
        }
    }
}

pub(crate) fn projected_parameter_values(
    tree: &StateTree,
    parameter: LiveEditableParameter,
) -> Result<(f32, f32), LiveDemoSceneError> {
    let state = decode_state_tree(tree)?;
    match parameter {
        LiveEditableParameter::Patch {
            patch_id,
            parameter,
        } => {
            let patch = state
                .patches
                .iter()
                .find(|patch| patch.id == patch_id.value())
                .ok_or(LiveDemoSceneError::SelectedParameterMismatch)?;
            let projected = state
                .parameters
                .patches
                .iter()
                .find(|patch| patch.patch_id == patch_id.value())
                .ok_or(LiveDemoSceneError::SelectedParameterMismatch)?;
            Ok((
                patch.parameters.value(parameter),
                projected.parameters.value(parameter),
            ))
        }
        LiveEditableParameter::Global { parameter } => Ok((
            state.global.value(parameter),
            state.parameters.global.value(parameter),
        )),
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecodedStateTree {
    patches: Vec<DecodedPatch>,
    global: DecodedGlobal,
    selection: DecodedSelection,
    parameters: DecodedParameterSnapshot,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecodedPatch {
    id: u32,
    channel: u8,
    parameters: DecodedChannel,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecodedChannel {
    gain_db: f32,
    pan: f32,
    reverb_send: f32,
    delay_send: f32,
}

impl DecodedChannel {
    const fn value(self, parameter: ChannelParameter) -> f32 {
        match parameter {
            ChannelParameter::GainDb => self.gain_db,
            ChannelParameter::Pan => self.pan,
            ChannelParameter::ReverbSend => self.reverb_send,
            ChannelParameter::DelaySend => self.delay_send,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecodedGlobal {
    master_gain_db: f32,
    reverb_room_size: f32,
    reverb_damping: f32,
    reverb_return: f32,
    delay_milliseconds: f32,
    delay_feedback: f32,
    delay_return: f32,
}

impl DecodedGlobal {
    const fn value(self, parameter: GlobalParameter) -> f32 {
        match parameter {
            GlobalParameter::MasterGainDb => self.master_gain_db,
            GlobalParameter::ReverbRoomSize => self.reverb_room_size,
            GlobalParameter::ReverbDamping => self.reverb_damping,
            GlobalParameter::ReverbReturn => self.reverb_return,
            GlobalParameter::DelayMilliseconds => self.delay_milliseconds,
            GlobalParameter::DelayFeedback => self.delay_feedback,
            GlobalParameter::DelayReturn => self.delay_return,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecodedSelection {
    section: String,
    patch_index: usize,
    parameter_index: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecodedParameterSnapshot {
    patches: Vec<DecodedParameterPatch>,
    global: DecodedGlobal,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecodedParameterPatch {
    patch_id: u32,
    parameters: DecodedChannel,
}

fn decode_state_tree(tree: &StateTree) -> Result<DecodedStateTree, LiveDemoSceneError> {
    serde_json::from_str(tree.json()).map_err(|_| LiveDemoSceneError::StateTreeDeserialization)
}

/// A structural or expectation error in the frozen live scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LiveDemoSceneError {
    StateTreeDeserialization,
    NoInstalledPatches,
    UnexpectedInitialSelection,
    InvalidPatchId,
    DuplicatePatchId(u32),
    InvalidMidiChannel(u8),
    InvalidPlannedAdjustment,
    SelectedParameterMismatch,
    ExpectedValueMismatch { expected: f32, actual: f32 },
    GenerationOverflow,
}

impl fmt::Display for LiveDemoSceneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::StateTreeDeserialization => {
                formatter.write_str("canonical state tree could not be decoded")
            }
            Self::NoInstalledPatches => {
                formatter.write_str("live demo requires installed fixture patches")
            }
            Self::UnexpectedInitialSelection => {
                formatter.write_str("live demo must start at the first Patch gain parameter")
            }
            Self::InvalidPatchId => {
                formatter.write_str("installed state contains an invalid PatchId")
            }
            Self::DuplicatePatchId(value) => {
                write!(formatter, "installed state repeats PatchId {value}")
            }
            Self::InvalidMidiChannel(value) => write!(
                formatter,
                "installed state contains invalid MIDI channel {value}"
            ),
            Self::InvalidPlannedAdjustment => {
                formatter.write_str("descriptor-derived adjustment would not change its parameter")
            }
            Self::SelectedParameterMismatch => {
                formatter.write_str("live scene selection no longer matches its frozen plan")
            }
            Self::ExpectedValueMismatch { expected, actual } => write!(
                formatter,
                "live scene expected selected value {expected}, got {actual}"
            ),
            Self::GenerationOverflow => formatter.write_str("live scene generation cannot advance"),
        }
    }
}

impl std::error::Error for LiveDemoSceneError {}

#[cfg(test)]
mod tests {
    use super::LiveAudioPredicate;
    use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;

    #[allow(clippy::too_many_arguments)]
    fn observation(
        left: f32,
        right: f32,
        output: f32,
        reverb: f32,
        delay: f32,
        wet: f32,
        non_finite: u64,
    ) -> AudioObservationSnapshot {
        AudioObservationSnapshot::from_parts(
            2, 2, 64, 7, 1, 1, left, right, output, reverb, delay, wet, non_finite, 0,
        )
    }

    #[test]
    fn audible_predicates_are_finite_and_stage_specific() {
        assert!(
            LiveAudioPredicate::OutputLevel.evaluate(observation(0.4, 0.3, 0.2, 0.0, 0.0, 0.0, 0,))
        );
        assert!(LiveAudioPredicate::StereoBalance
            .evaluate(observation(0.4, 0.3, 0.2, 0.0, 0.0, 0.0, 0,)));
        assert!(!LiveAudioPredicate::StereoBalance
            .evaluate(observation(0.4, 0.4, 0.2, 0.0, 0.0, 0.0, 0,)));
        assert!(
            LiveAudioPredicate::ReverbInput.evaluate(observation(0.4, 0.3, 0.2, 0.1, 0.0, 0.0, 0,))
        );
        assert!(
            LiveAudioPredicate::DelayInput.evaluate(observation(0.4, 0.3, 0.2, 0.0, 0.1, 0.0, 0,))
        );
        assert!(
            LiveAudioPredicate::WetOutput.evaluate(observation(0.4, 0.3, 0.2, 0.0, 0.0, 0.1, 0,))
        );

        let non_finite = observation(f32::NAN, 0.3, 0.2, 0.1, 0.1, 0.1, 1);
        for predicate in [
            LiveAudioPredicate::OutputLevel,
            LiveAudioPredicate::StereoBalance,
            LiveAudioPredicate::ReverbInput,
            LiveAudioPredicate::DelayInput,
            LiveAudioPredicate::WetOutput,
        ] {
            assert!(!predicate.evaluate(non_finite));
        }
    }
}
