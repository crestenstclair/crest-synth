pub mod capability_id;
pub mod descriptor_default_config_factory;
pub mod effect_capability;
pub mod effect_capability_id;
pub mod effect_capability_provider;
pub mod effect_composition;
pub mod effect_preparer;
pub mod effect_slot_id;
pub mod instrument_capability;
pub mod instrument_capability_provider;
pub mod instrument_composition;
pub mod instrument_preparer;
pub mod parameter_id;
pub mod patch;
pub mod prepared_engine_rack_builder;
pub mod prepared_instrument;
pub mod prepared_post_effect;
pub mod prepared_post_effect_rack_builder;
pub mod sound_font_preset;

pub use capability_id::{CapabilityId, IdentifierError};
pub use descriptor_default_config_factory::DescriptorDefaultConfigFactory;
pub use effect_capability::{
    EffectCapabilityDescriptor, EffectCapabilityError, EffectCapabilityRegistry, PostEffectConfig,
    MAX_EFFECT_SCALAR_PARAMETERS, MAX_POST_EFFECTS_PER_PATCH,
};
pub use effect_capability_id::EffectCapabilityId;
pub use effect_capability_provider::EffectCapabilityProvider;
pub use effect_composition::{compose_effect_registry, EffectCompositionError};
pub use effect_preparer::{EffectPreparationError, EffectPreparer};
pub use effect_slot_id::{EffectSlotId, EffectSlotIdError};
pub use instrument_capability::{
    AssetAssignment, AssetKind, AssetReference, AssetRequirement, CapabilityDescriptor,
    CapabilityError, CapabilityRegistry, CapabilitySection, InstrumentConfig, ParameterAdjustment,
    ParameterAssignment, ParameterChoice, ParameterDefault, ParameterKind, ParameterPredicate,
    ParameterRange, ParameterSpec, ParameterUpdate, ParameterValue, PatchInteraction, VoicePolicy,
    MAX_INSTRUMENT_SCALAR_PARAMETERS,
};
pub use instrument_capability_provider::InstrumentCapabilityProvider;
pub use instrument_composition::{compose_instrument_registry, InstrumentCompositionError};
pub use instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
pub use patch::{resolve_patch_editable_targets, Patch, PatchEditableTarget};
pub use prepared_engine_rack_builder::{PreparedEngineRackBuilder, RackPreparationError};
pub use prepared_instrument::{PreparedInstrument, PreparedInstrumentError};
pub use prepared_post_effect::{PreparedEffectError, PreparedPostEffect};
pub use prepared_post_effect_rack_builder::{
    EffectRackPreparationError, PreparedPostEffectRackBuilder,
};
pub mod sound_font_instrument;
pub mod voice_envelope;
pub mod voice_envelope_state;
pub use parameter_id::ParameterId;
pub use sound_font_preset::{
    SoundFontPresetCatalog, SoundFontPresetCatalogEntry, SoundFontPresetCatalogError,
    SoundFontPresetCollision, SoundFontPresetId, SoundFontPresetIdError, SoundFontPresetSource,
};
pub use voice_envelope::{
    VoiceEnvelope, VoiceEnvelopeError, VoiceEnvelopeParameter, VoiceEnvelopeParameterDescriptor,
};
pub use voice_envelope_state::{VoiceEnvelopeStage, VoiceEnvelopeState};
