use crate::control::app_state::AppState;
use crate::control::top_level_context::TopLevelContext;
use crate::kernel::{MidiChannel, PatchId};
use crate::synth::instrument_capability::{
    AssetReference, ParameterChoice, ParameterDefault, ParameterKind, ParameterRange,
    ParameterUpdate, ParameterValue,
};
use crate::synth::{CapabilityId, ParameterId};
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
    active_capability_id: CapabilityId,
    active_label: String,
    choices: Vec<PatchPageEngineChoice>,
    editable: bool,
}

impl PatchPageEngine {
    pub const fn active_capability_id(&self) -> &CapabilityId {
        &self.active_capability_id
    }

    pub fn active_label(&self) -> &str {
        &self.active_label
    }

    pub fn choices(&self) -> &[PatchPageEngineChoice] {
        &self.choices
    }

    pub const fn editable(&self) -> bool {
        self.editable
    }
}

/// One canonical, read-only ADSR row projected without a second field list.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPageEnvelopeRow {
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

impl PatchPageEnvelopeRow {
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
    id: ParameterId,
    label: String,
    kind: ParameterKind,
    update: ParameterUpdate,
    value: PatchPageParameterValue,
    range: Option<ParameterRange>,
    choices: Vec<ParameterChoice>,
    fine_step: Option<f64>,
    coarse_step: Option<f64>,
    unit: Option<String>,
    formatter: String,
    enabled: bool,
    visible: bool,
    editable: bool,
}

impl PatchPageParameterRow {
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

    pub const fn value(&self) -> &PatchPageParameterValue {
        &self.value
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
    patch: PatchPageIdentity,
    engine: PatchPageEngine,
    envelope: Vec<PatchPageEnvelopeRow>,
    sections: Vec<PatchPageSection>,
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
        "engine.activeLabel",
        "engine.choices[].capabilityId",
        "engine.choices[].label",
        "engine.editable",
        "envelope[].coarseStep",
        "envelope[].editable",
        "envelope[].fineStep",
        "envelope[].id",
        "envelope[].label",
        "envelope[].maximum",
        "envelope[].minimum",
        "envelope[].unit",
        "envelope[].value",
        "patch.id",
        "patch.midiChannel",
        "patch.name",
        "sections[].id",
        "sections[].label",
        "sections[].parameters[].choices[].id",
        "sections[].parameters[].choices[].label",
        "sections[].parameters[].coarseStep",
        "sections[].parameters[].editable",
        "sections[].parameters[].enabled",
        "sections[].parameters[].fineStep",
        "sections[].parameters[].formatter",
        "sections[].parameters[].id",
        "sections[].parameters[].kind",
        "sections[].parameters[].label",
        "sections[].parameters[].range.maximum",
        "sections[].parameters[].range.minimum",
        "sections[].parameters[].range",
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

    pub fn patch(&self) -> &PatchPageIdentity {
        &self.content.patch
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

        let choices = state
            .capabilities()
            .descriptors()
            .iter()
            .map(|descriptor| PatchPageEngineChoice {
                capability_id: descriptor.id().clone(),
                label: descriptor.label().to_owned(),
            })
            .collect();
        let envelope = crate::synth::VoiceEnvelope::surface_descriptor()
            .iter()
            .map(|spec| PatchPageEnvelopeRow {
                id: spec.name().to_owned(),
                label: spec.label().to_owned(),
                value: patch.envelope().value(spec.parameter()),
                minimum: spec.minimum(),
                maximum: spec.maximum(),
                fine_step: spec.fine_step(),
                coarse_step: spec.coarse_step(),
                unit: spec.unit().map(str::to_owned),
                editable: false,
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
                        Ok(PatchPageParameterRow {
                            id: spec.id().clone(),
                            label: spec.label().to_owned(),
                            kind: spec.kind(),
                            update: spec.update(),
                            value,
                            range: spec.range(),
                            choices: spec.choices().to_vec(),
                            fine_step: spec.fine_step(),
                            coarse_step: spec.coarse_step(),
                            unit: spec.unit().map(str::to_owned),
                            formatter: spec.formatter().to_owned(),
                            enabled: predicate_satisfied(spec.enabled_when()),
                            visible: predicate_satisfied(spec.visible_when()),
                            editable: false,
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

        Ok(Self {
            content: Arc::new(PatchPageContent {
                context: TopLevelContext::Patch,
                patch: PatchPageIdentity {
                    id: patch.id(),
                    name: patch.name().to_owned(),
                    midi_channel: patch.channel(),
                },
                engine: PatchPageEngine {
                    active_capability_id: descriptor.id().clone(),
                    active_label: descriptor.label().to_owned(),
                    choices,
                    editable: false,
                },
                envelope,
                sections,
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
            patch: &'a PatchPageIdentity,
            engine: &'a PatchPageEngine,
            envelope: &'a [PatchPageEnvelopeRow],
            sections: &'a [PatchPageSection],
            state_hash: &'a str,
        }

        SerializablePatchPage {
            context: self.context(),
            patch: self.patch(),
            engine: self.engine(),
            envelope: self.envelope(),
            sections: self.sections(),
            state_hash: self.state_hash(),
        }
        .serialize(serializer)
    }
}

/// A typed invariant failure while resolving the focused schema projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PatchPageProjectionError {
    MissingPatchFocus,
    UnknownPatchFocus,
    InvalidInstrumentConfig,
}

impl fmt::Display for PatchPageProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingPatchFocus => "PATCH context has no stable Patch focus",
            Self::UnknownPatchFocus => "PATCH focus does not resolve to an installed Patch",
            Self::InvalidInstrumentConfig => {
                "focused Patch config does not resolve through its capability descriptor"
            }
        })
    }
}

impl std::error::Error for PatchPageProjectionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::braids_capability::BraidsCapability;
    use crate::adapter::hidef_soundfont_capability::HiDefSoundFontCapability;
    use crate::adapter::production_instruments::production_capability_registry;
    use crate::control::{AppEvent, AppState, StateProjector, TopLevelContext};
    use crate::kernel::{MidiChannel, PatchId};
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
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
            ChannelParameters::default(),
        )
        .with_envelope(crate::synth::VoiceEnvelope::new(12.0, 34.0, 0.56, 78.0).unwrap());
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
        assert_eq!(page.engine().active_capability_id(), descriptor.id());
        assert_eq!(page.engine().active_label(), descriptor.label());
        assert!(!page.engine().editable());
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
            assert_eq!(row.id(), spec.name());
            assert_eq!(row.label(), spec.label());
            assert_eq!(row.value(), patch.envelope().value(spec.parameter()));
            assert_eq!(row.minimum(), spec.minimum());
            assert_eq!(row.maximum(), spec.maximum());
            assert_eq!(row.unit(), spec.unit());
            assert!(!row.editable());
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
                assert!(!row.editable());
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
        let soundfont = HiDefSoundFontCapability::new().unwrap();
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
        let soundfont = HiDefSoundFontCapability::new().unwrap();
        for page in [
            project(&state_with_config(
                create_soundfont_config(
                    &soundfont,
                    SoundFontInstrument::new(128, 11, false).unwrap(),
                )
                .unwrap(),
            )),
            page,
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
