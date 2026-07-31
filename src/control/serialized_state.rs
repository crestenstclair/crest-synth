use crate::control::app_state::AppState;
use crate::control::{EngineSelectionStatus, FocusPath, InteractionMode, ReturnPath};
use crate::mixer::bus_id::MAX_BUS_RETURNS;
use crate::mixer::bus_return::BusReturnBank;
use crate::mixer::global_parameters::GlobalParameters;
use crate::mixer::mixer_state::MixerState;
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::mixer_track_parameters::MixerTrackParameter;
use crate::mixer::patch_output::PatchOutput;
use crate::synth::instrument_capability::{CapabilityRegistry, InstrumentConfig};
use crate::synth::patch::Patch;
use crate::synth::voice_envelope::VoiceEnvelope;
use crate::synth::{EffectCapabilityRegistry, PostEffectConfig};
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
    #[serde(borrow, default)]
    pub(crate) effects: Cow<'a, EffectCapabilityRegistry>,
    #[serde(borrow)]
    pub(crate) patches: Vec<SerializedPatch<'a>>,
    pub(crate) mixer: MixerState,
    pub(crate) global: SerializedGlobalParameters,
    #[serde(default)]
    pub(crate) returns: SerializedBusReturns,
    #[serde(default)]
    pub(crate) interaction: SerializedInteractionState,
    pub(crate) engine_selection: EngineSelectionStatus,
}

impl<'a> From<&'a AppState> for SerializedState<'a> {
    fn from(state: &'a AppState) -> Self {
        Self {
            generation: state.generation(),
            capabilities: Cow::Borrowed(state.capabilities()),
            effects: Cow::Borrowed(state.effects()),
            patches: state.patches().iter().map(SerializedPatch::from).collect(),
            mixer: *state.mixer(),
            global: SerializedGlobalParameters::from(state.global()),
            returns: SerializedBusReturns::from(state.bus_returns()),
            interaction: SerializedInteractionState::from_state(state),
            engine_selection: state.engine_selection().clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SerializedInteractionState {
    #[serde(default = "default_mixer_focus")]
    pub(crate) active_focus: FocusPath,
    #[serde(default)]
    pub(crate) remembered_patch_main: Option<FocusPath>,
    #[serde(default = "default_mixer_focus")]
    pub(crate) remembered_mixer_main: FocusPath,
    #[serde(default)]
    pub(crate) mode: InteractionMode,
    #[serde(default)]
    pub(crate) return_path: Option<ReturnPath>,
}

impl Default for SerializedInteractionState {
    fn default() -> Self {
        Self {
            active_focus: default_mixer_focus(),
            remembered_patch_main: None,
            remembered_mixer_main: default_mixer_focus(),
            mode: InteractionMode::Navigate,
            return_path: None,
        }
    }
}

impl SerializedInteractionState {
    fn from_state(state: &AppState) -> Self {
        let interaction = state.interaction();
        Self {
            active_focus: interaction.focus_path().clone(),
            remembered_patch_main: interaction.remembered_patch_main().cloned(),
            remembered_mixer_main: interaction.remembered_mixer_main().clone(),
            mode: interaction.mode(),
            return_path: interaction.return_path().cloned(),
        }
    }
}

fn default_mixer_focus() -> FocusPath {
    FocusPath::mixer_track(MixerTrackId::default(), MixerTrackParameter::Level)
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
    #[serde(borrow, default)]
    pub(crate) post_effects: Cow<'a, [PostEffectConfig]>,
    #[serde(default)]
    pub(crate) envelope: VoiceEnvelope,
    pub(crate) output: PatchOutput,
}

impl<'a> From<&'a Patch> for SerializedPatch<'a> {
    fn from(patch: &'a Patch) -> Self {
        Self {
            id: patch.id().value(),
            name: Cow::Borrowed(patch.name()),
            channel: patch.channel().value(),
            instrument: Cow::Borrowed(patch.instrument_config()),
            post_effects: Cow::Borrowed(patch.post_effects()),
            envelope: *patch.envelope(),
            output: patch.output(),
        }
    }
}

/// The one genuinely global serialized value.
///
/// The retired reverb/delay fields are gone from this shape: their state is
/// return-owned and travels as the indexed `returns` section.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SerializedGlobalParameters {
    pub(crate) master_gain_db: f32,
}

impl From<&GlobalParameters> for SerializedGlobalParameters {
    fn from(parameters: &GlobalParameters) -> Self {
        Self {
            master_gain_db: parameters.master_gain_db(),
        }
    }
}

/// One serialized bus return: zero-or-one occupying registry configuration
/// plus the return-owned output level. Array position is the `BusId`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SerializedBusReturn {
    #[serde(default)]
    pub(crate) effect: Option<PostEffectConfig>,
    pub(crate) return_level: f32,
}

/// The canonical serialized eight-return bank in ascending `BusId` order.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct SerializedBusReturns(pub(crate) Vec<SerializedBusReturn>);

impl Default for SerializedBusReturns {
    fn default() -> Self {
        Self::from(&BusReturnBank::default())
    }
}

impl From<&BusReturnBank> for SerializedBusReturns {
    fn from(bank: &BusReturnBank) -> Self {
        Self(
            bank.returns()
                .iter()
                .map(|bus_return| SerializedBusReturn {
                    effect: bus_return.effect().cloned(),
                    return_level: bus_return.return_level(),
                })
                .collect(),
        )
    }
}

impl SerializedBusReturns {
    /// Returns the serialized entries in ascending `BusId` order.
    pub(crate) fn entries(&self) -> &[SerializedBusReturn] {
        &self.0
    }

    /// Reports whether the serialized bank carries exactly eight entries.
    pub(crate) fn is_complete(&self) -> bool {
        self.0.len() == MAX_BUS_RETURNS
    }
}
