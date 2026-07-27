use crate::kernel::midi_message::MidiMessageKind;
use crate::synth::{
    AssetAssignment, AssetReference, AssetRequirement, CapabilityDescriptor, CapabilityError,
    CapabilityId, CapabilitySection, EffectCapabilityId, EffectSlotId, ParameterAssignment,
    ParameterKind, ParameterSpec, ParameterUpdate, ParameterValue, VoicePolicy,
};
use core::fmt;
use serde::{Deserialize, Serialize};

pub const MAX_POST_EFFECTS_PER_PATCH: usize = 1;
pub const MAX_EFFECT_SCALAR_PARAMETERS: usize = 8;

/// Immutable ordered control-side schema for one post-effect capability.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectCapabilityDescriptor {
    id: EffectCapabilityId,
    label: String,
    semantic_accent: String,
    sections: Vec<CapabilitySection>,
    asset_requirements: Vec<AssetRequirement>,
}

impl EffectCapabilityDescriptor {
    pub fn new(
        id: EffectCapabilityId,
        label: impl Into<String>,
        semantic_accent: impl Into<String>,
        sections: Vec<CapabilitySection>,
        asset_requirements: Vec<AssetRequirement>,
    ) -> Result<Self, EffectCapabilityError> {
        let descriptor = Self {
            id,
            label: label.into(),
            semantic_accent: semantic_accent.into(),
            sections,
            asset_requirements,
        };
        descriptor.validation_descriptor()?;
        if descriptor.scalar_parameter_count() > MAX_EFFECT_SCALAR_PARAMETERS {
            return Err(EffectCapabilityError::TooManyScalarParameters {
                count: descriptor.scalar_parameter_count(),
                capacity: MAX_EFFECT_SCALAR_PARAMETERS,
            });
        }
        Ok(descriptor)
    }

    pub const fn id(&self) -> &EffectCapabilityId {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn semantic_accent(&self) -> &str {
        &self.semantic_accent
    }

    pub fn sections(&self) -> &[CapabilitySection] {
        &self.sections
    }

    pub fn asset_requirements(&self) -> &[AssetRequirement] {
        &self.asset_requirements
    }

    pub fn parameters(&self) -> impl Iterator<Item = &ParameterSpec> {
        self.sections.iter().flat_map(CapabilitySection::parameters)
    }

    pub fn parameter(&self, id: &crate::synth::ParameterId) -> Option<&ParameterSpec> {
        self.parameters().find(|parameter| parameter.id() == id)
    }

    pub fn scalar_parameters(&self) -> impl Iterator<Item = &ParameterSpec> {
        self.parameters()
            .filter(|parameter| parameter.update() == ParameterUpdate::Scalar)
    }

    pub fn scalar_parameter_count(&self) -> usize {
        self.scalar_parameters().count()
    }

    pub fn create_config(
        &self,
        slot_id: EffectSlotId,
        values: &[ParameterAssignment],
        asset_references: &[AssetAssignment],
    ) -> Result<PostEffectConfig, EffectCapabilityError> {
        let canonical = self
            .validation_descriptor()?
            .create_config(values, asset_references)?;
        Ok(PostEffectConfig {
            slot_id,
            capability_id: self.id.clone(),
            values: canonical.values().to_vec(),
            asset_references: canonical.asset_references().to_vec(),
        })
    }

    fn validation_descriptor(&self) -> Result<CapabilityDescriptor, EffectCapabilityError> {
        if self.scalar_parameter_count() > MAX_EFFECT_SCALAR_PARAMETERS {
            return Err(EffectCapabilityError::TooManyScalarParameters {
                count: self.scalar_parameter_count(),
                capacity: MAX_EFFECT_SCALAR_PARAMETERS,
            });
        }
        CapabilityDescriptor::new(
            CapabilityId::new("instrument.effect-schema")
                .expect("static validation capability id is valid"),
            self.label.clone(),
            self.semantic_accent.clone(),
            self.sections.clone(),
            self.asset_requirements.clone(),
            VoicePolicy::FixedPerPatch { voices: 1 },
            vec![MidiMessageKind::AllNotesOff],
        )
        .map_err(Into::into)
    }
}

/// One canonical Patch-owned post-effect slot configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostEffectConfig {
    slot_id: EffectSlotId,
    capability_id: EffectCapabilityId,
    values: Vec<ParameterAssignment>,
    asset_references: Vec<AssetAssignment>,
}

impl PostEffectConfig {
    pub const fn from_parts(
        slot_id: EffectSlotId,
        capability_id: EffectCapabilityId,
        values: Vec<ParameterAssignment>,
        asset_references: Vec<AssetAssignment>,
    ) -> Self {
        Self {
            slot_id,
            capability_id,
            values,
            asset_references,
        }
    }

    pub const fn slot_id(&self) -> EffectSlotId {
        self.slot_id
    }

    pub const fn capability_id(&self) -> &EffectCapabilityId {
        &self.capability_id
    }

    pub fn values(&self) -> &[ParameterAssignment] {
        &self.values
    }

    pub fn asset_references(&self) -> &[AssetAssignment] {
        &self.asset_references
    }

    pub fn value(&self, id: &crate::synth::ParameterId) -> Option<&ParameterValue> {
        self.values
            .iter()
            .find(|assignment| assignment.parameter_id() == id)
            .map(ParameterAssignment::value)
    }

    pub fn asset_reference(&self, id: &crate::synth::ParameterId) -> Option<&AssetReference> {
        self.asset_references
            .iter()
            .find(|assignment| assignment.parameter_id() == id)
            .map(AssetAssignment::reference)
    }

    pub fn with_scalar_value(
        &self,
        descriptor: &EffectCapabilityDescriptor,
        id: &crate::synth::ParameterId,
        value: ParameterValue,
    ) -> Result<Self, EffectCapabilityError> {
        if descriptor.id() != self.capability_id() {
            return Err(EffectCapabilityError::ProviderRegistryMismatch(
                self.capability_id.clone(),
            ));
        }
        let spec = descriptor
            .parameter(id)
            .ok_or_else(|| CapabilityError::UndeclaredParameter(id.clone()))?;
        if spec.update() != ParameterUpdate::Scalar || spec.kind() == ParameterKind::Asset {
            return Err(CapabilityError::StructuralParameter(id.clone()).into());
        }
        spec.scalar_value(&value)?;
        let mut values = self.values.clone();
        let assignment = values
            .iter_mut()
            .find(|assignment| assignment.parameter_id() == id)
            .ok_or_else(|| CapabilityError::MissingParameter(id.clone()))?;
        *assignment = ParameterAssignment::new(id.clone(), value);
        descriptor.create_config(self.slot_id, &values, &self.asset_references)
    }
}

/// Immutable ordered registry of installed post-effect descriptors.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectCapabilityRegistry {
    descriptors: Vec<EffectCapabilityDescriptor>,
}

impl EffectCapabilityRegistry {
    pub fn new(
        descriptors: Vec<EffectCapabilityDescriptor>,
    ) -> Result<Self, EffectCapabilityError> {
        for (index, descriptor) in descriptors.iter().enumerate() {
            descriptor.validation_descriptor()?;
            if descriptors[..index]
                .iter()
                .any(|prior| prior.id() == descriptor.id())
            {
                return Err(EffectCapabilityError::DuplicateCapability(
                    descriptor.id().clone(),
                ));
            }
        }
        Ok(Self { descriptors })
    }

    pub fn descriptors(&self) -> &[EffectCapabilityDescriptor] {
        &self.descriptors
    }

    pub fn descriptor(&self, id: &EffectCapabilityId) -> Option<&EffectCapabilityDescriptor> {
        self.descriptors
            .iter()
            .find(|descriptor| descriptor.id() == id)
    }

    pub fn validate_config(&self, config: &PostEffectConfig) -> Result<(), EffectCapabilityError> {
        let descriptor = self.descriptor(config.capability_id()).ok_or_else(|| {
            EffectCapabilityError::UnknownCapability(config.capability_id.clone())
        })?;
        let canonical = descriptor.create_config(
            config.slot_id(),
            config.values(),
            config.asset_references(),
        )?;
        if canonical != *config {
            return Err(EffectCapabilityError::ConfigOrderMismatch(
                config.capability_id.clone(),
            ));
        }
        Ok(())
    }

    pub fn validate_patch_effects(
        &self,
        configs: &[PostEffectConfig],
    ) -> Result<(), EffectCapabilityError> {
        if configs.len() > MAX_POST_EFFECTS_PER_PATCH {
            return Err(EffectCapabilityError::TooManyEffectSlots {
                count: configs.len(),
                capacity: MAX_POST_EFFECTS_PER_PATCH,
            });
        }
        for (index, config) in configs.iter().enumerate() {
            if configs[..index]
                .iter()
                .any(|prior| prior.slot_id() == config.slot_id())
            {
                return Err(EffectCapabilityError::DuplicateSlot(config.slot_id()));
            }
            self.validate_config(config)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectCapabilityError {
    Capability(CapabilityError),
    DuplicateCapability(EffectCapabilityId),
    UnknownCapability(EffectCapabilityId),
    ProviderRegistryMismatch(EffectCapabilityId),
    ConfigOrderMismatch(EffectCapabilityId),
    DuplicateSlot(EffectSlotId),
    TooManyEffectSlots { count: usize, capacity: usize },
    TooManyScalarParameters { count: usize, capacity: usize },
}

impl From<CapabilityError> for EffectCapabilityError {
    fn from(error: CapabilityError) -> Self {
        Self::Capability(error)
    }
}

impl fmt::Display for EffectCapabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capability(error) => error.fmt(formatter),
            Self::DuplicateCapability(id) => write!(formatter, "duplicate effect capability {id}"),
            Self::UnknownCapability(id) => write!(formatter, "unknown effect capability {id}"),
            Self::ProviderRegistryMismatch(id) => {
                write!(
                    formatter,
                    "effect provider descriptor for {id} is not installed exactly"
                )
            }
            Self::ConfigOrderMismatch(id) => {
                write!(
                    formatter,
                    "effect config for {id} is not in descriptor order"
                )
            }
            Self::DuplicateSlot(id) => write!(formatter, "duplicate effect slot {id}"),
            Self::TooManyEffectSlots { count, capacity } => {
                write!(
                    formatter,
                    "Patch has {count} post effects; maximum is {capacity}"
                )
            }
            Self::TooManyScalarParameters { count, capacity } => write!(
                formatter,
                "effect declares {count} Scalar parameters but capacity is {capacity}"
            ),
        }
    }
}

impl std::error::Error for EffectCapabilityError {}
