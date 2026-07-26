pub mod capability_id;
pub mod instrument_capability;
pub mod instrument_capability_provider;
pub mod instrument_composition;
pub mod instrument_preparer;
pub mod parameter_id;
pub mod patch;
pub mod prepared_engine_rack_builder;
pub mod prepared_instrument;

pub use capability_id::{CapabilityId, IdentifierError};
pub use instrument_capability::{
    AssetAssignment, AssetKind, AssetReference, AssetRequirement, CapabilityDescriptor,
    CapabilityError, CapabilityRegistry, CapabilitySection, InstrumentConfig, ParameterAdjustment,
    ParameterAssignment, ParameterChoice, ParameterDefault, ParameterKind, ParameterPredicate,
    ParameterRange, ParameterSpec, ParameterUpdate, ParameterValue, VoicePolicy,
    MAX_INSTRUMENT_SCALAR_PARAMETERS,
};
pub use instrument_capability_provider::InstrumentCapabilityProvider;
pub use instrument_composition::{compose_instrument_registry, InstrumentCompositionError};
pub use instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
pub use patch::{resolve_patch_editable_targets, Patch, PatchEditableTarget};
pub use prepared_engine_rack_builder::{PreparedEngineRackBuilder, RackPreparationError};
pub use prepared_instrument::{PreparedInstrument, PreparedInstrumentError};
pub mod sound_font_instrument;
pub mod voice_envelope;
pub mod voice_envelope_state;
pub use parameter_id::ParameterId;
pub use voice_envelope::{
    VoiceEnvelope, VoiceEnvelopeError, VoiceEnvelopeParameter, VoiceEnvelopeParameterDescriptor,
};
pub use voice_envelope_state::{VoiceEnvelopeStage, VoiceEnvelopeState};
