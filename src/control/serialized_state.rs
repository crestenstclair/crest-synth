use crate::control::app_state::AppState;
use crate::control::interaction_state::{Selection, SelectionSection};
use crate::control::top_level_context::TopLevelContext;
use crate::control::{EngineSelectionStatus, PatchControlId};
use crate::mixer::global_parameters::GlobalParameters;
use crate::synth::instrument_capability::{CapabilityRegistry, InstrumentConfig};
use crate::synth::patch::Patch;
use crate::synth::voice_envelope::VoiceEnvelope;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Canonical serde representation shared by snapshots, text, and observation trees.
///
/// Production projection borrows immutable registry, Patch, and config storage.
/// Deserialization owns the same shape for round-trip and external-snapshot tests.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SerializedState<'a> {
    pub(crate) generation: u64,
    #[serde(borrow)]
    pub(crate) capabilities: Cow<'a, CapabilityRegistry>,
    #[serde(borrow)]
    pub(crate) patches: Vec<SerializedPatch<'a>>,
    pub(crate) global: SerializedGlobalParameters,
    #[serde(default)]
    pub(crate) interaction: SerializedInteractionState,
    pub(crate) engine_selection: EngineSelectionStatus,
}

impl<'a> From<&'a AppState> for SerializedState<'a> {
    fn from(state: &'a AppState) -> Self {
        Self {
            generation: state.generation(),
            capabilities: Cow::Borrowed(state.capabilities()),
            patches: state.patches().iter().map(SerializedPatch::from).collect(),
            global: SerializedGlobalParameters::from(state.global()),
            interaction: SerializedInteractionState::from(state.interaction()),
            engine_selection: state.engine_selection().clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SerializedInteractionState {
    pub(crate) context: TopLevelContext,
    pub(crate) mixer_selection: SerializedSelection,
    pub(crate) patch_focus: Option<u32>,
    pub(crate) patch_control_focus: Option<PatchControlId>,
}

impl Default for SerializedInteractionState {
    fn default() -> Self {
        Self {
            context: TopLevelContext::Mixer,
            mixer_selection: SerializedSelection::from(Selection::global()),
            patch_focus: None,
            patch_control_focus: None,
        }
    }
}

impl From<&crate::control::interaction_state::InteractionState> for SerializedInteractionState {
    fn from(interaction: &crate::control::interaction_state::InteractionState) -> Self {
        Self {
            context: interaction.context(),
            mixer_selection: SerializedSelection::from(interaction.mixer_selection()),
            patch_focus: interaction.patch_focus().map(|patch_id| patch_id.value()),
            patch_control_focus: interaction.patch_control_focus(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SerializedPatch<'a> {
    pub(crate) id: u32,
    #[serde(borrow)]
    pub(crate) name: Cow<'a, str>,
    pub(crate) channel: u8,
    #[serde(borrow)]
    pub(crate) instrument: Cow<'a, InstrumentConfig>,
    #[serde(default)]
    pub(crate) envelope: VoiceEnvelope,
    pub(crate) gain_db: f32,
    pub(crate) pan: f32,
    pub(crate) reverb_send: f32,
    pub(crate) delay_send: f32,
}

impl<'a> From<&'a Patch> for SerializedPatch<'a> {
    fn from(patch: &'a Patch) -> Self {
        Self {
            id: patch.id().value(),
            name: Cow::Borrowed(patch.name()),
            channel: patch.channel().value(),
            instrument: Cow::Borrowed(patch.instrument_config()),
            envelope: *patch.envelope(),
            gain_db: patch.parameters().gain_db(),
            pan: patch.parameters().pan(),
            reverb_send: patch.parameters().reverb_send(),
            delay_send: patch.parameters().delay_send(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SerializedGlobalParameters {
    pub(crate) master_gain_db: f32,
    pub(crate) reverb_room_size: f32,
    pub(crate) reverb_damping: f32,
    pub(crate) reverb_return: f32,
    pub(crate) delay_milliseconds: f32,
    pub(crate) delay_feedback: f32,
    pub(crate) delay_return: f32,
}

impl From<&GlobalParameters> for SerializedGlobalParameters {
    fn from(parameters: &GlobalParameters) -> Self {
        Self {
            master_gain_db: parameters.master_gain_db(),
            reverb_room_size: parameters.reverb_room_size(),
            reverb_damping: parameters.reverb_damping(),
            reverb_return: parameters.reverb_return(),
            delay_milliseconds: parameters.delay_milliseconds(),
            delay_feedback: parameters.delay_feedback(),
            delay_return: parameters.delay_return(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SerializedSelection {
    pub(crate) section: SerializedSelectionSection,
    pub(crate) patch_index: usize,
    pub(crate) parameter_index: usize,
}

impl SerializedSelection {
    pub(crate) fn matches(self, selection: Selection) -> bool {
        self == Self::from(selection)
    }
}

impl From<Selection> for SerializedSelection {
    fn from(selection: Selection) -> Self {
        Self {
            section: SerializedSelectionSection::from(selection.section()),
            patch_index: selection.patch_index(),
            parameter_index: selection.parameter_index(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum SerializedSelectionSection {
    Patch,
    Global,
}

impl From<SelectionSection> for SerializedSelectionSection {
    fn from(section: SelectionSection) -> Self {
        match section {
            SelectionSection::Patch => Self::Patch,
            SelectionSection::Global => Self::Global,
        }
    }
}
