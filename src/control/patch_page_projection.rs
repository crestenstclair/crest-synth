use crate::control::app_state::AppState;
use crate::control::top_level_context::TopLevelContext;
use crate::control::{
    EngineSelectionFailure, EngineSelectionRequestId, EngineSelectionStatusKind, PatchControlId,
    StructuralEditIntent,
};
use crate::kernel::{MidiChannel, PatchId};
use crate::mixer::patch_output::{PatchOutputParameter, PatchOutputParameterKind};
use crate::real_time::GraphRevision;
use crate::synth::instrument_capability::{
    AssetReference, ParameterChoice, ParameterDefault, ParameterKind, ParameterRange,
    ParameterUpdate, ParameterValue, PatchInteraction,
};
use crate::synth::{
    CapabilityId, EffectCapabilityDescriptor, EffectCapabilityId, EffectSlotId, ParameterId,
    ParameterSpec, PostEffectConfig,
};
use core::fmt;
use serde::{Serialize, Serializer};
use std::sync::Arc;

/// One stable focused Patch identity copied into the host-neutral PATCH view.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPageIdentity {
    id: PatchId,
    name: String,
    midi_channel: MidiChannel,
}

impl PatchPageIdentity {
    pub const fn id(&self) -> PatchId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn midi_channel(&self) -> MidiChannel {
        self.midi_channel
    }
}

/// One installed capability available to a future structural engine selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPageEngineChoice {
    capability_id: CapabilityId,
    label: String,
}

impl PatchPageEngineChoice {
    pub const fn capability_id(&self) -> &CapabilityId {
        &self.capability_id
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Active engine identity plus the complete installed registry choice surface.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPageEngine {
    control_id: PatchControlId,
    active_capability_id: CapabilityId,
    active_label: String,
    choices: Vec<PatchPageEngineChoice>,
    status: EngineSelectionStatusKind,
    active_graph_revision: GraphRevision,
    requested_capability_id: Option<CapabilityId>,
    request_id: Option<EngineSelectionRequestId>,
    target_graph_revision: Option<GraphRevision>,
    failure: Option<EngineSelectionFailure>,
    editable: bool,
}

impl PatchPageEngine {
    pub fn control_id(&self) -> PatchControlId {
        self.control_id.clone()
    }

    pub const fn active_capability_id(&self) -> &CapabilityId {
        &self.active_capability_id
    }

    pub fn active_label(&self) -> &str {
        &self.active_label
    }

    pub fn choices(&self) -> &[PatchPageEngineChoice] {
        &self.choices
    }

    pub const fn status(&self) -> EngineSelectionStatusKind {
        self.status
    }

    pub const fn active_graph_revision(&self) -> GraphRevision {
        self.active_graph_revision
    }

    pub const fn requested_capability_id(&self) -> Option<&CapabilityId> {
        self.requested_capability_id.as_ref()
    }

    pub const fn request_id(&self) -> Option<EngineSelectionRequestId> {
        self.request_id
    }

    pub const fn target_graph_revision(&self) -> Option<GraphRevision> {
        self.target_graph_revision
    }

    pub const fn failure(&self) -> Option<EngineSelectionFailure> {
        self.failure
    }

    pub const fn editable(&self) -> bool {
        self.editable
    }
}

/// One canonical editable ADSR row projected without a second field list.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPageEnvelopeRow {
    control_id: PatchControlId,
    id: String,
    label: String,
    value: f32,
    minimum: f32,
    maximum: f32,
    fine_step: f32,
    coarse_step: f32,
    unit: Option<String>,
    editable: bool,
}

/// One canonical PATCH Utility output row.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPageOutputRow {
    control_id: PatchControlId,
    id: String,
    label: String,
    kind: String,
    scalar_value: Option<f32>,
    choice_value: Option<String>,
    minimum: Option<f32>,
    maximum: Option<f32>,
    fine_step: Option<f32>,
    coarse_step: Option<f32>,
    unit: Option<String>,
    editable: bool,
}

impl PatchPageOutputRow {
    fn for_parameter(
        parameter: PatchOutputParameter,
        output: crate::mixer::patch_output::PatchOutput,
    ) -> Self {
        let descriptor = parameter.descriptor();
        Self {
            control_id: PatchControlId::Output(parameter),
            id: descriptor.name().to_owned(),
            label: descriptor.label().to_owned(),
            kind: match descriptor.kind() {
                PatchOutputParameterKind::Continuous => "continuous",
                PatchOutputParameterKind::TrackChoice => "choice",
            }
            .to_owned(),
            scalar_value: (parameter == PatchOutputParameter::TrimGain)
                .then_some(output.trim_gain_db()),
            choice_value: (parameter == PatchOutputParameter::OutputTrack)
                .then(|| output.track_id().to_string()),
            minimum: descriptor.minimum(),
            maximum: descriptor.maximum(),
            fine_step: descriptor.fine_step(),
            coarse_step: descriptor.coarse_step(),
            unit: descriptor.unit().map(str::to_owned),
            editable: true,
        }
    }

    pub(crate) fn selected_text(
        parameter: PatchOutputParameter,
        output: crate::mixer::patch_output::PatchOutput,
    ) -> Result<String, serde_json::Error> {
        serde_json::to_string(&Self::for_parameter(parameter, output))
            .map(|row| format!("> OUTPUT {row}"))
    }

    pub fn control_id(&self) -> PatchControlId {
        self.control_id.clone()
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn scalar_value(&self) -> Option<f32> {
        self.scalar_value
    }

    pub fn choice_value(&self) -> Option<&str> {
        self.choice_value.as_deref()
    }
}

impl PatchPageEnvelopeRow {
    pub(crate) fn for_parameter(
        parameter: crate::synth::VoiceEnvelopeParameter,
        value: f32,
    ) -> Self {
        let spec = parameter.descriptor();
        Self {
            control_id: PatchControlId::Envelope(parameter),
            id: spec.name().to_owned(),
            label: spec.label().to_owned(),
            value,
            minimum: spec.minimum(),
            maximum: spec.maximum(),
            fine_step: spec.fine_step(),
            coarse_step: spec.coarse_step(),
            unit: spec.unit().map(str::to_owned),
            editable: true,
        }
    }

    pub(crate) fn selected_text(
        parameter: crate::synth::VoiceEnvelopeParameter,
        value: f32,
    ) -> Result<String, serde_json::Error> {
        serde_json::to_string(&Self::for_parameter(parameter, value))
            .map(|row| format!("> ENVELOPE {row}"))
    }

    pub fn control_id(&self) -> PatchControlId {
        self.control_id.clone()
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn value(&self) -> f32 {
        self.value
    }

    pub const fn minimum(&self) -> f32 {
        self.minimum
    }

    pub const fn maximum(&self) -> f32 {
        self.maximum
    }

    pub const fn fine_step(&self) -> f32 {
        self.fine_step
    }

    pub const fn coarse_step(&self) -> f32 {
        self.coarse_step
    }

    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    pub const fn editable(&self) -> bool {
        self.editable
    }
}

/// The exact typed config source used by one descriptor-projected row.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "source", rename_all = "camelCase")]
pub enum PatchPageParameterValue {
    Parameter { value: ParameterValue },
    Asset { reference: AssetReference },
}

impl PatchPageParameterValue {
    pub const fn parameter(&self) -> Option<&ParameterValue> {
        match self {
            Self::Parameter { value } => Some(value),
            Self::Asset { .. } => None,
        }
    }

    pub const fn asset(&self) -> Option<&AssetReference> {
        match self {
            Self::Parameter { .. } => None,
            Self::Asset { reference } => Some(reference),
        }
    }
}

/// One exact active-capability parameter row in descriptor order.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPageParameterRow {
    control_id: Option<PatchControlId>,
    id: ParameterId,
    label: String,
    kind: ParameterKind,
    update: ParameterUpdate,
    patch_interaction: PatchInteraction,
    value: PatchPageParameterValue,
    selected_choice_id: Option<String>,
    selected_label: Option<String>,
    range: Option<ParameterRange>,
    choices: Vec<ParameterChoice>,
    fine_step: Option<f64>,
    coarse_step: Option<f64>,
    unit: Option<String>,
    formatter: String,
    requested_choice_id: Option<String>,
    requested_label: Option<String>,
    status: Option<EngineSelectionStatusKind>,
    request_id: Option<EngineSelectionRequestId>,
    active_graph_revision: Option<GraphRevision>,
    target_graph_revision: Option<GraphRevision>,
    failure: Option<EngineSelectionFailure>,
    enabled: bool,
    visible: bool,
    editable: bool,
}

impl PatchPageParameterRow {
    pub(crate) fn for_effect_parameter(
        descriptor: &EffectCapabilityDescriptor,
        config: &PostEffectConfig,
        spec: &ParameterSpec,
        active_graph_revision: GraphRevision,
    ) -> Result<Self, PatchPageProjectionError> {
        if descriptor.id() != config.capability_id() || descriptor.parameter(spec.id()).is_none() {
            return Err(PatchPageProjectionError::InvalidEffectConfig);
        }
        let value = if spec.kind() == ParameterKind::Asset {
            let reference = config
                .asset_reference(spec.id())
                .or_else(|| match spec.default_value() {
                    ParameterDefault::Asset(reference) => Some(reference),
                    ParameterDefault::Value(_) => None,
                })
                .ok_or(PatchPageProjectionError::InvalidEffectConfig)?;
            PatchPageParameterValue::Asset {
                reference: reference.clone(),
            }
        } else {
            PatchPageParameterValue::Parameter {
                value: config
                    .value(spec.id())
                    .ok_or(PatchPageProjectionError::InvalidEffectConfig)?
                    .clone(),
            }
        };
        let predicate_satisfied = |predicate: Option<&crate::synth::ParameterPredicate>| {
            predicate.is_none_or(|predicate| {
                config.value(predicate.parameter_id()) == Some(predicate.equals())
            })
        };
        let enabled = predicate_satisfied(spec.enabled_when());
        let visible = predicate_satisfied(spec.visible_when());
        let control_id =
            (spec.patch_interaction() == PatchInteraction::ScalarEdit && enabled && visible)
                .then(|| PatchControlId::Effect(config.slot_id(), spec.id().clone()));
        Ok(Self {
            control_id: control_id.clone(),
            id: spec.id().clone(),
            label: spec.label().to_owned(),
            kind: spec.kind(),
            update: spec.update(),
            patch_interaction: spec.patch_interaction(),
            value,
            selected_choice_id: None,
            selected_label: None,
            range: spec.range(),
            choices: spec.choices().to_vec(),
            fine_step: spec.fine_step(),
            coarse_step: spec.coarse_step(),
            unit: spec.unit().map(str::to_owned),
            formatter: spec.formatter().to_owned(),
            requested_choice_id: None,
            requested_label: None,
            status: None,
            request_id: None,
            active_graph_revision: control_id.as_ref().map(|_| active_graph_revision),
            target_graph_revision: None,
            failure: None,
            enabled,
            visible,
            editable: control_id.is_some(),
        })
    }

    pub(crate) fn selected_effect_text(
        descriptor: &EffectCapabilityDescriptor,
        config: &PostEffectConfig,
        parameter_id: &ParameterId,
        active_graph_revision: GraphRevision,
    ) -> Result<String, PatchPageProjectionError> {
        let spec = descriptor
            .parameter(parameter_id)
            .ok_or(PatchPageProjectionError::InvalidEffectConfig)?;
        let row = Self::for_effect_parameter(descriptor, config, spec, active_graph_revision)?;
        serde_json::to_string(&row)
            .map(|row| format!("> EFFECT_PARAMETER {row}"))
            .map_err(|_| PatchPageProjectionError::InvalidEffectConfig)
    }

    pub fn control_id(&self) -> Option<PatchControlId> {
        self.control_id.clone()
    }

    pub const fn id(&self) -> &ParameterId {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn kind(&self) -> ParameterKind {
        self.kind
    }

    pub const fn update(&self) -> ParameterUpdate {
        self.update
    }

    pub const fn patch_interaction(&self) -> PatchInteraction {
        self.patch_interaction
    }

    pub const fn value(&self) -> &PatchPageParameterValue {
        &self.value
    }

    pub fn selected_choice_id(&self) -> Option<&str> {
        self.selected_choice_id.as_deref()
    }

    pub fn selected_label(&self) -> Option<&str> {
        self.selected_label.as_deref()
    }

    pub const fn range(&self) -> Option<ParameterRange> {
        self.range
    }

    pub fn choices(&self) -> &[ParameterChoice] {
        &self.choices
    }

    pub const fn fine_step(&self) -> Option<f64> {
        self.fine_step
    }

    pub const fn coarse_step(&self) -> Option<f64> {
        self.coarse_step
    }

    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    pub fn formatter(&self) -> &str {
        &self.formatter
    }

    pub fn requested_choice_id(&self) -> Option<&str> {
        self.requested_choice_id.as_deref()
    }

    pub fn requested_label(&self) -> Option<&str> {
        self.requested_label.as_deref()
    }

    pub const fn status(&self) -> Option<EngineSelectionStatusKind> {
        self.status
    }

    pub const fn request_id(&self) -> Option<EngineSelectionRequestId> {
        self.request_id
    }

    pub const fn active_graph_revision(&self) -> Option<GraphRevision> {
        self.active_graph_revision
    }

    pub const fn target_graph_revision(&self) -> Option<GraphRevision> {
        self.target_graph_revision
    }

    pub const fn failure(&self) -> Option<EngineSelectionFailure> {
        self.failure
    }

    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    pub const fn visible(&self) -> bool {
        self.visible
    }

    pub const fn editable(&self) -> bool {
        self.editable
    }
}

/// One active capability section and its descriptor-ordered rows.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPageSection {
    id: String,
    label: String,
    parameters: Vec<PatchPageParameterRow>,
}

/// One configured Patch effect identity and its descriptor-derived sections.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPageEffect {
    slot_id: EffectSlotId,
    capability_id: EffectCapabilityId,
    label: String,
    editable_identity: bool,
    sections: Vec<PatchPageSection>,
}

impl PatchPageEffect {
    pub const fn slot_id(&self) -> EffectSlotId {
        self.slot_id
    }

    pub const fn capability_id(&self) -> &EffectCapabilityId {
        &self.capability_id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn editable_identity(&self) -> bool {
        self.editable_identity
    }

    pub fn sections(&self) -> &[PatchPageSection] {
        &self.sections
    }
}

impl PatchPageSection {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn parameters(&self) -> &[PatchPageParameterRow] {
        &self.parameters
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PatchPageContent {
    context: TopLevelContext,
    focused_control_id: PatchControlId,
    patch: PatchPageIdentity,
    output: Vec<PatchPageOutputRow>,
    engine: PatchPageEngine,
    envelope: Vec<PatchPageEnvelopeRow>,
    sections: Vec<PatchPageSection>,
    effects: Vec<PatchPageEffect>,
}

/// Immutable host-neutral PATCH view model derived by one generic schema walk.
#[derive(Clone, Debug)]
pub struct PatchPageProjection {
    content: Arc<PatchPageContent>,
    state_hash: Arc<str>,
}

impl PatchPageProjection {
    pub const SERIALIZED_LEAF_DESCRIPTOR: &'static [&'static str] = &[
        "context",
        "engine.activeCapabilityId",
        "engine.activeGraphRevision",
        "engine.activeLabel",
        "engine.choices[].capabilityId",
        "engine.choices[].label",
        "engine.controlId",
        "engine.editable",
        "engine.failure",
        "engine.requestId",
        "engine.requestedCapabilityId",
        "engine.status",
        "engine.targetGraphRevision",
        "envelope[].coarseStep",
        "envelope[].controlId",
        "envelope[].editable",
        "envelope[].fineStep",
        "envelope[].id",
        "envelope[].label",
        "envelope[].maximum",
        "envelope[].minimum",
        "envelope[].unit",
        "envelope[].value",
        "focusedControlId",
        "effects[].capabilityId",
        "effects[].editableIdentity",
        "effects[].label",
        "effects[].slotId",
        "effects[].sections[].id",
        "effects[].sections[].label",
        "effects[].sections[].parameters[].coarseStep",
        "effects[].sections[].parameters[].controlId",
        "effects[].sections[].parameters[].editable",
        "effects[].sections[].parameters[].enabled",
        "effects[].sections[].parameters[].activeGraphRevision",
        "effects[].sections[].parameters[].failure",
        "effects[].sections[].parameters[].fineStep",
        "effects[].sections[].parameters[].formatter",
        "effects[].sections[].parameters[].id",
        "effects[].sections[].parameters[].kind",
        "effects[].sections[].parameters[].label",
        "effects[].sections[].parameters[].patchInteraction",
        "effects[].sections[].parameters[].requestId",
        "effects[].sections[].parameters[].range.maximum",
        "effects[].sections[].parameters[].range.minimum",
        "effects[].sections[].parameters[].requestedChoiceId",
        "effects[].sections[].parameters[].requestedLabel",
        "effects[].sections[].parameters[].selectedChoiceId",
        "effects[].sections[].parameters[].selectedLabel",
        "effects[].sections[].parameters[].status",
        "effects[].sections[].parameters[].targetGraphRevision",
        "effects[].sections[].parameters[].unit",
        "effects[].sections[].parameters[].update",
        "effects[].sections[].parameters[].value.source",
        "effects[].sections[].parameters[].value.value.kind",
        "effects[].sections[].parameters[].value.value.value",
        "effects[].sections[].parameters[].visible",
        "patch.id",
        "patch.midiChannel",
        "patch.name",
        "output[].coarseStep",
        "output[].controlId",
        "output[].editable",
        "output[].fineStep",
        "output[].id",
        "output[].kind",
        "output[].label",
        "output[].maximum",
        "output[].minimum",
        "output[].scalarValue",
        "output[].choiceValue",
        "output[].unit",
        "sections[].id",
        "sections[].label",
        "sections[].parameters[].activeGraphRevision",
        "sections[].parameters[].choices[].id",
        "sections[].parameters[].choices[].label",
        "sections[].parameters[].coarseStep",
        "sections[].parameters[].controlId",
        "sections[].parameters[].editable",
        "sections[].parameters[].enabled",
        "sections[].parameters[].failure",
        "sections[].parameters[].fineStep",
        "sections[].parameters[].formatter",
        "sections[].parameters[].id",
        "sections[].parameters[].kind",
        "sections[].parameters[].label",
        "sections[].parameters[].patchInteraction",
        "sections[].parameters[].range.maximum",
        "sections[].parameters[].range.minimum",
        "sections[].parameters[].range",
        "sections[].parameters[].requestId",
        "sections[].parameters[].requestedChoiceId",
        "sections[].parameters[].requestedLabel",
        "sections[].parameters[].selectedChoiceId",
        "sections[].parameters[].selectedLabel",
        "sections[].parameters[].status",
        "sections[].parameters[].targetGraphRevision",
        "sections[].parameters[].unit",
        "sections[].parameters[].update",
        "sections[].parameters[].value.reference.kind",
        "sections[].parameters[].value.reference.locator",
        "sections[].parameters[].value.source",
        "sections[].parameters[].value.value.kind",
        "sections[].parameters[].value.value.value",
        "sections[].parameters[].visible",
        "stateHash",
    ];

    pub const fn serialized_leaf_descriptor() -> &'static [&'static str] {
        Self::SERIALIZED_LEAF_DESCRIPTOR
    }

    pub fn context(&self) -> TopLevelContext {
        self.content.context
    }

    pub fn focused_control_id(&self) -> PatchControlId {
        self.content.focused_control_id.clone()
    }

    pub fn patch(&self) -> &PatchPageIdentity {
        &self.content.patch
    }

    pub fn output(&self) -> &[PatchPageOutputRow] {
        &self.content.output
    }

    pub fn engine(&self) -> &PatchPageEngine {
        &self.content.engine
    }

    pub fn envelope(&self) -> &[PatchPageEnvelopeRow] {
        &self.content.envelope
    }

    pub fn sections(&self) -> &[PatchPageSection] {
        &self.content.sections
    }

    pub fn effects(&self) -> &[PatchPageEffect] {
        &self.content.effects
    }

    pub fn state_hash(&self) -> &str {
        &self.state_hash
    }

    pub(crate) fn with_state_hash(&self, state_hash: String) -> Self {
        Self {
            content: Arc::clone(&self.content),
            state_hash: Arc::from(state_hash),
        }
    }

    #[cfg(test)]
    pub(crate) fn shares_content_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.content, &other.content)
    }

    pub(crate) fn project(
        state: &AppState,
        state_hash: &str,
    ) -> Result<Self, PatchPageProjectionError> {
        let focus = state
            .interaction()
            .patch_focus()
            .ok_or(PatchPageProjectionError::MissingPatchFocus)?;
        let focused_control_id = state
            .interaction()
            .patch_control_focus()
            .ok_or(PatchPageProjectionError::MissingPatchControlFocus)?;
        let patch = state
            .patches()
            .iter()
            .find(|patch| patch.id() == focus)
            .ok_or(PatchPageProjectionError::UnknownPatchFocus)?;
        let descriptor = state
            .capabilities()
            .descriptor(patch.instrument_config().capability_id())
            .ok_or(PatchPageProjectionError::InvalidInstrumentConfig)?;
        state
            .capabilities()
            .validate_config(patch.instrument_config())
            .map_err(|_| PatchPageProjectionError::InvalidInstrumentConfig)?;
        state
            .effects()
            .validate_patch_effects(patch.post_effects())
            .map_err(|_| PatchPageProjectionError::InvalidEffectConfig)?;
        let resolved_controls = if matches!(focused_control_id, PatchControlId::Output(_)) {
            PatchControlId::utility_surface_descriptor().to_vec()
        } else {
            state
                .focused_patch_controls()
                .map_err(|_| PatchPageProjectionError::InvalidInstrumentConfig)?
        };
        if !resolved_controls.contains(&focused_control_id) {
            return Err(PatchPageProjectionError::InvalidInstrumentConfig);
        }

        let choices = state
            .capabilities()
            .descriptors()
            .iter()
            .map(|descriptor| PatchPageEngineChoice {
                capability_id: descriptor.id().clone(),
                label: descriptor.label().to_owned(),
            })
            .collect();
        let output = PatchOutputParameter::ALL
            .into_iter()
            .map(|parameter| PatchPageOutputRow::for_parameter(parameter, patch.output()))
            .collect();
        let engine_selection = state.engine_selection();
        let correlation = engine_selection.correlation();
        let engine_targeted = correlation.is_some_and(|correlation| {
            matches!(
                correlation.intent(),
                StructuralEditIntent::ReplaceCapability { .. }
            ) && correlation.patch_id() == patch.id()
        });
        let editable = state.capabilities().descriptors().len() >= 2
            && matches!(
                engine_selection.kind(),
                EngineSelectionStatusKind::Ready | EngineSelectionStatusKind::Failed
            );
        let envelope = crate::synth::VoiceEnvelope::surface_descriptor()
            .iter()
            .map(|spec| {
                PatchPageEnvelopeRow::for_parameter(
                    spec.parameter(),
                    patch.envelope().value(spec.parameter()),
                )
            })
            .collect();
        let sections = descriptor
            .sections()
            .iter()
            .map(|section| {
                let parameters = section
                    .parameters()
                    .iter()
                    .map(|spec| {
                        let value = if spec.kind() == ParameterKind::Asset {
                            let reference = patch
                                .instrument_config()
                                .asset_reference(spec.id())
                                .or_else(|| match spec.default_value() {
                                    ParameterDefault::Asset(reference) => Some(reference),
                                    ParameterDefault::Value(_) => None,
                                })
                                .ok_or(PatchPageProjectionError::InvalidInstrumentConfig)?;
                            PatchPageParameterValue::Asset {
                                reference: reference.clone(),
                            }
                        } else {
                            let value = patch
                                .instrument_config()
                                .value(spec.id())
                                .ok_or(PatchPageProjectionError::InvalidInstrumentConfig)?;
                            PatchPageParameterValue::Parameter {
                                value: value.clone(),
                            }
                        };
                        let predicate_satisfied =
                            |predicate: Option<&crate::synth::ParameterPredicate>| {
                                predicate.is_none_or(|predicate| {
                                    patch.instrument_config().value(predicate.parameter_id())
                                        == Some(predicate.equals())
                                })
                            };
                        let enabled = predicate_satisfied(spec.enabled_when());
                        let visible = predicate_satisfied(spec.visible_when());
                        let control_id = (spec.patch_interaction()
                            == PatchInteraction::StructuralChoice
                            && enabled
                            && visible)
                            .then(|| PatchControlId::Capability(spec.id().clone()));
                        let selected_choice_id = match &value {
                            PatchPageParameterValue::Parameter {
                                value: ParameterValue::Choice(choice_id),
                            } => Some(choice_id.clone()),
                            _ => None,
                        };
                        let selected_label = selected_choice_id.as_deref().and_then(|choice_id| {
                            spec.choices()
                                .iter()
                                .find(|choice| choice.id() == choice_id)
                                .map(|choice| choice.label().to_owned())
                        });
                        let row_correlation = correlation.filter(|correlation| {
                            correlation.patch_id() == patch.id()
                                && matches!(
                                    correlation.intent(),
                                    StructuralEditIntent::ReplaceParameterChoice {
                                        capability_id,
                                        parameter_id,
                                        ..
                                    } if capability_id == descriptor.id() && parameter_id == spec.id()
                                )
                        });
                        let requested_choice_id = row_correlation.and_then(|correlation| {
                            correlation.intent().choice_id().map(str::to_owned)
                        });
                        let requested_label = requested_choice_id.as_deref().and_then(|choice_id| {
                            spec.choices()
                                .iter()
                                .find(|choice| choice.id() == choice_id)
                                .map(|choice| choice.label().to_owned())
                        });
                        Ok(PatchPageParameterRow {
                            control_id: control_id.clone(),
                            id: spec.id().clone(),
                            label: spec.label().to_owned(),
                            kind: spec.kind(),
                            update: spec.update(),
                            patch_interaction: spec.patch_interaction(),
                            value,
                            selected_choice_id,
                            selected_label,
                            range: spec.range(),
                            choices: spec.choices().to_vec(),
                            fine_step: spec.fine_step(),
                            coarse_step: spec.coarse_step(),
                            unit: spec.unit().map(str::to_owned),
                            formatter: spec.formatter().to_owned(),
                            requested_choice_id,
                            requested_label,
                            status: row_correlation.map(|_| engine_selection.kind()),
                            request_id: row_correlation
                                .map(|correlation| correlation.request_id()),
                            active_graph_revision: control_id
                                .as_ref()
                                .map(|_| engine_selection.active_graph_revision()),
                            target_graph_revision: row_correlation.and_then(|correlation| {
                                correlation.target_graph_revision()
                            }),
                            failure: row_correlation.and_then(|_| engine_selection.failure()),
                            enabled,
                            visible,
                            editable: control_id.is_some() && !engine_selection.is_in_flight(),
                        })
                    })
                    .collect::<Result<Vec<_>, PatchPageProjectionError>>()?;
                Ok(PatchPageSection {
                    id: section.id().to_owned(),
                    label: section.label().to_owned(),
                    parameters,
                })
            })
            .collect::<Result<Vec<_>, PatchPageProjectionError>>()?;
        let effects = patch
            .post_effects()
            .iter()
            .map(|config| {
                let effect_descriptor = state
                    .effects()
                    .descriptor(config.capability_id())
                    .ok_or(PatchPageProjectionError::InvalidEffectConfig)?;
                let sections = effect_descriptor
                    .sections()
                    .iter()
                    .map(|section| {
                        let parameters = section
                            .parameters()
                            .iter()
                            .map(|spec| {
                                PatchPageParameterRow::for_effect_parameter(
                                    effect_descriptor,
                                    config,
                                    spec,
                                    engine_selection.active_graph_revision(),
                                )
                            })
                            .collect::<Result<Vec<_>, PatchPageProjectionError>>()?;
                        Ok(PatchPageSection {
                            id: section.id().to_owned(),
                            label: section.label().to_owned(),
                            parameters,
                        })
                    })
                    .collect::<Result<Vec<_>, PatchPageProjectionError>>()?;
                Ok(PatchPageEffect {
                    slot_id: config.slot_id(),
                    capability_id: config.capability_id().clone(),
                    label: effect_descriptor.label().to_owned(),
                    editable_identity: false,
                    sections,
                })
            })
            .collect::<Result<Vec<_>, PatchPageProjectionError>>()?;

        Ok(Self {
            content: Arc::new(PatchPageContent {
                context: TopLevelContext::Patch,
                focused_control_id,
                patch: PatchPageIdentity {
                    id: patch.id(),
                    name: patch.name().to_owned(),
                    midi_channel: patch.channel(),
                },
                output,
                engine: PatchPageEngine {
                    control_id: PatchControlId::Engine,
                    active_capability_id: descriptor.id().clone(),
                    active_label: descriptor.label().to_owned(),
                    choices,
                    status: if engine_targeted {
                        engine_selection.kind()
                    } else {
                        EngineSelectionStatusKind::Ready
                    },
                    active_graph_revision: engine_selection.active_graph_revision(),
                    requested_capability_id: correlation
                        .filter(|_| engine_targeted)
                        .map(|correlation| correlation.target_capability_id().clone()),
                    request_id: correlation
                        .filter(|_| engine_targeted)
                        .map(|correlation| correlation.request_id()),
                    target_graph_revision: correlation
                        .filter(|_| engine_targeted)
                        .and_then(|correlation| correlation.target_graph_revision()),
                    failure: engine_targeted
                        .then(|| engine_selection.failure())
                        .flatten(),
                    editable,
                },
                envelope,
                sections,
                effects,
            }),
            state_hash: Arc::from(state_hash),
        })
    }
}

impl PartialEq for PatchPageProjection {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content && self.state_hash == other.state_hash
    }
}

impl Serialize for PatchPageProjection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SerializablePatchPage<'a> {
            context: TopLevelContext,
            focused_control_id: PatchControlId,
            patch: &'a PatchPageIdentity,
            output: &'a [PatchPageOutputRow],
            engine: &'a PatchPageEngine,
            envelope: &'a [PatchPageEnvelopeRow],
            sections: &'a [PatchPageSection],
            effects: &'a [PatchPageEffect],
            state_hash: &'a str,
        }

        SerializablePatchPage {
            context: self.context(),
            focused_control_id: self.focused_control_id(),
            patch: self.patch(),
            output: self.output(),
            engine: self.engine(),
            envelope: self.envelope(),
            sections: self.sections(),
            effects: self.effects(),
            state_hash: self.state_hash(),
        }
        .serialize(serializer)
    }
}

/// A typed invariant failure while resolving the focused schema projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PatchPageProjectionError {
    MissingPatchFocus,
    MissingPatchControlFocus,
    UnknownPatchFocus,
    InvalidInstrumentConfig,
    InvalidEffectConfig,
}

impl fmt::Display for PatchPageProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingPatchFocus => "PATCH context has no stable Patch focus",
            Self::MissingPatchControlFocus => "PATCH context has no stable control focus",
            Self::UnknownPatchFocus => "PATCH focus does not resolve to an installed Patch",
            Self::InvalidInstrumentConfig => {
                "focused Patch config does not resolve through its capability descriptor"
            }
            Self::InvalidEffectConfig => {
                "focused Patch effect config does not resolve through its capability descriptor"
            }
        })
    }
}

impl std::error::Error for PatchPageProjectionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::braids_capability::BraidsCapability;
    use crate::adapter::production_effects::{
        production_chorus_config, production_effect_registry,
    };
    use crate::adapter::production_instruments::production_capability_registry;
    use crate::control::{AppEvent, AppState, StateProjector, TopLevelContext};
    use crate::kernel::{MidiChannel, PatchId};
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::mixer_track_id::MixerTrackId;
    use crate::mixer::patch_output::PatchOutput;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::Patch;
    use crate::testing::automatic_midi_test::create_soundfont_config;
    use serde_json::Value;
    use std::collections::BTreeSet;

    fn state_with_config(config: crate::synth::InstrumentConfig) -> AppState {
        let mut state = AppState::new(
            production_capability_registry().unwrap(),
            GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5).unwrap(),
        );
        let patch = Patch::new(
            PatchId::new(7).unwrap(),
            "Focused".to_owned(),
            config,
            MidiChannel::new(3).unwrap(),
            PatchOutput::to_track(MixerTrackId::new(3).unwrap()),
        )
        .with_envelope(crate::synth::VoiceEnvelope::new(12.0, 34.0, 0.56, 78.0).unwrap());
        state.apply(AppEvent::InstallPatches(vec![patch])).unwrap();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        state
    }

    fn state_with_configured_effect(config: crate::synth::InstrumentConfig) -> AppState {
        let mut state = AppState::new_with_effects(
            production_capability_registry().unwrap(),
            production_effect_registry().unwrap(),
            GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5).unwrap(),
        );
        let patch = Patch::new(
            PatchId::new(7).unwrap(),
            "Focused".to_owned(),
            config,
            MidiChannel::new(3).unwrap(),
            PatchOutput::to_track(MixerTrackId::new(3).unwrap()),
        )
        .with_envelope(crate::synth::VoiceEnvelope::new(12.0, 34.0, 0.56, 78.0).unwrap())
        .with_post_effects(vec![production_chorus_config(
            EffectSlotId::new(1).unwrap(),
        )
        .unwrap()]);
        state.apply(AppEvent::InstallPatches(vec![patch])).unwrap();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        state
    }

    fn project(state: &AppState) -> PatchPageProjection {
        let projector = StateProjector::new();
        let snapshot = projector.state_snapshot(state).unwrap();
        projector
            .patch_page_projection(state, snapshot.hash())
            .unwrap()
            .unwrap()
    }

    fn assert_descriptor_walk(state: &AppState, page: &PatchPageProjection) {
        let patch = &state.patches()[0];
        let descriptor = state
            .capabilities()
            .descriptor(patch.instrument_config().capability_id())
            .unwrap();
        assert_eq!(page.patch().id(), patch.id());
        assert_eq!(page.patch().name(), patch.name());
        assert_eq!(page.patch().midi_channel(), patch.channel());
        assert_eq!(
            page.focused_control_id(),
            state.interaction().patch_control_focus().unwrap()
        );
        assert_eq!(page.engine().active_capability_id(), descriptor.id());
        assert_eq!(page.engine().active_label(), descriptor.label());
        assert_eq!(page.engine().control_id(), PatchControlId::Engine);
        assert_eq!(page.engine().status(), EngineSelectionStatusKind::Ready);
        assert_eq!(
            page.engine().active_graph_revision(),
            GraphRevision::INITIAL
        );
        assert_eq!(page.engine().requested_capability_id(), None);
        assert_eq!(page.engine().request_id(), None);
        assert_eq!(page.engine().target_graph_revision(), None);
        assert_eq!(page.engine().failure(), None);
        assert!(page.engine().editable());
        assert_eq!(
            page.engine().choices().len(),
            state.capabilities().descriptors().len()
        );
        for (choice, installed) in page
            .engine()
            .choices()
            .iter()
            .zip(state.capabilities().descriptors())
        {
            assert_eq!(choice.capability_id(), installed.id());
            assert_eq!(choice.label(), installed.label());
        }
        for (row, spec) in page
            .envelope()
            .iter()
            .zip(crate::synth::VoiceEnvelope::surface_descriptor())
        {
            assert_eq!(row.control_id(), PatchControlId::Envelope(spec.parameter()));
            assert_eq!(row.id(), spec.name());
            assert_eq!(row.label(), spec.label());
            assert_eq!(row.value(), patch.envelope().value(spec.parameter()));
            assert_eq!(row.minimum(), spec.minimum());
            assert_eq!(row.maximum(), spec.maximum());
            assert_eq!(row.unit(), spec.unit());
            assert!(row.editable());
        }
        assert_eq!(page.sections().len(), descriptor.sections().len());
        for (section, declared) in page.sections().iter().zip(descriptor.sections()) {
            assert_eq!(section.id(), declared.id());
            assert_eq!(section.label(), declared.label());
            assert_eq!(section.parameters().len(), declared.parameters().len());
            for (row, spec) in section.parameters().iter().zip(declared.parameters()) {
                assert_eq!(row.id(), spec.id());
                assert_eq!(row.label(), spec.label());
                assert_eq!(row.kind(), spec.kind());
                assert_eq!(row.update(), spec.update());
                assert_eq!(row.range(), spec.range());
                assert_eq!(row.choices(), spec.choices());
                assert_eq!(row.unit(), spec.unit());
                assert!(row.enabled());
                assert!(row.visible());
                assert_eq!(row.patch_interaction(), spec.patch_interaction());
                let structural =
                    spec.patch_interaction() == crate::synth::PatchInteraction::StructuralChoice;
                assert_eq!(row.editable(), structural);
                assert_eq!(
                    row.control_id(),
                    structural.then(|| PatchControlId::Capability(spec.id().clone()))
                );
                match row.value() {
                    PatchPageParameterValue::Parameter { value } => {
                        assert_eq!(Some(value), patch.instrument_config().value(spec.id()));
                    }
                    PatchPageParameterValue::Asset { reference } => {
                        assert_eq!(
                            Some(reference),
                            patch.instrument_config().asset_reference(spec.id())
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn both_production_capabilities_share_one_exact_generic_projection_walk() {
        let soundfont =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        let soundfont_state = state_with_config(
            create_soundfont_config(
                &soundfont,
                SoundFontInstrument::new(128, 11, false).unwrap(),
            )
            .unwrap(),
        );
        let braids_state =
            state_with_config(BraidsCapability::new().unwrap().default_config().unwrap());

        assert_descriptor_walk(&soundfont_state, &project(&soundfont_state));
        assert_descriptor_walk(&braids_state, &project(&braids_state));
    }

    #[test]
    fn focus_identity_is_independent_from_engine_action_availability() {
        fn selected_row_count(page: &PatchPageProjection) -> usize {
            usize::from(page.engine().control_id() == page.focused_control_id())
                + page
                    .envelope()
                    .iter()
                    .filter(|row| row.control_id() == page.focused_control_id())
                    .count()
        }

        let soundfont =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        let mut state = state_with_config(
            create_soundfont_config(
                &soundfont,
                SoundFontInstrument::new(128, 11, false).unwrap(),
            )
            .unwrap(),
        );

        let engine_ready = project(&state);
        assert_eq!(engine_ready.focused_control_id(), PatchControlId::Engine);
        assert!(engine_ready.engine().editable());
        assert_eq!(selected_row_count(&engine_ready), 1);

        state
            .apply(AppEvent::Navigate(crate::control::Direction::Down))
            .unwrap();
        let attack_ready = project(&state);
        assert_eq!(
            attack_ready.focused_control_id(),
            PatchControlId::Envelope(crate::synth::VoiceEnvelopeParameter::AttackMilliseconds)
        );
        assert!(attack_ready.engine().editable());
        assert_eq!(selected_row_count(&attack_ready), 1);

        state
            .apply(AppEvent::Navigate(crate::control::Direction::Up))
            .unwrap();
        state
            .apply(AppEvent::Adjust(crate::control::Direction::Right))
            .unwrap();
        let engine_preparing = project(&state);
        assert_eq!(
            engine_preparing.focused_control_id(),
            PatchControlId::Engine
        );
        assert!(!engine_preparing.engine().editable());
        assert_eq!(selected_row_count(&engine_preparing), 1);

        state
            .apply(AppEvent::Navigate(crate::control::Direction::Down))
            .unwrap();
        let attack_preparing = project(&state);
        assert_eq!(
            attack_preparing.focused_control_id(),
            PatchControlId::Envelope(crate::synth::VoiceEnvelopeParameter::AttackMilliseconds)
        );
        assert!(!attack_preparing.engine().editable());
        assert_eq!(selected_row_count(&attack_preparing), 1);
    }

    #[test]
    fn projection_serialization_and_midi_hash_update_keep_owned_schema_content() {
        let state = state_with_config(BraidsCapability::new().unwrap().default_config().unwrap());
        let page = project(&state);
        let advanced = page.with_state_hash("advanced-hash".to_owned());
        assert!(page.shares_content_with(&advanced));
        assert_eq!(advanced.state_hash(), "advanced-hash");

        fn leaves(value: &Value, prefix: &str, output: &mut BTreeSet<String>) {
            match value {
                Value::Object(object) => {
                    for (name, child) in object {
                        let path = if prefix.is_empty() {
                            name.to_owned()
                        } else {
                            format!("{prefix}.{name}")
                        };
                        leaves(child, &path, output);
                    }
                }
                Value::Array(array) => {
                    for child in array {
                        leaves(child, &format!("{prefix}[]"), output);
                    }
                }
                _ => {
                    output.insert(prefix.to_owned());
                }
            }
        }

        let mut discovered = BTreeSet::new();
        let soundfont =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        for page in [
            project(&state_with_config(
                create_soundfont_config(
                    &soundfont,
                    SoundFontInstrument::new(128, 11, false).unwrap(),
                )
                .unwrap(),
            )),
            page,
            project(&state_with_configured_effect(
                BraidsCapability::new().unwrap().default_config().unwrap(),
            )),
        ] {
            leaves(&serde_json::to_value(page).unwrap(), "", &mut discovered);
        }
        let described = PatchPageProjection::serialized_leaf_descriptor()
            .iter()
            .map(|path| (*path).to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(described, discovered);
    }
}
