use crate::synth::{
    AssetAssignment, EffectCapabilityDescriptor, EffectCapabilityError, EffectSlotId,
    ParameterAssignment, PostEffectConfig,
};

/// Control-side metadata/configuration port for one installed effect.
pub trait EffectCapabilityProvider {
    fn descriptor(&self) -> EffectCapabilityDescriptor;

    fn create_config(
        &self,
        slot_id: EffectSlotId,
        values: &[ParameterAssignment],
        asset_references: &[AssetAssignment],
    ) -> Result<PostEffectConfig, EffectCapabilityError>;
}
