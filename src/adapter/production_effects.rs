use crate::adapter::chorus_capability::ChorusCapability;
use crate::adapter::chorus_preparer::ChorusPreparer;
use crate::synth::{
    compose_effect_registry, EffectCapabilityError, EffectCapabilityProvider,
    EffectCapabilityRegistry, EffectCompositionError, EffectPreparationError, EffectPreparer,
    EffectSlotId, PostEffectConfig,
};

#[derive(Clone, Debug, thiserror::Error)]
pub enum ProductionEffectCompositionError {
    #[error("failed to construct the production effect capability: {0}")]
    Capability(EffectCapabilityError),
    #[error("failed to construct the production effect preparer: {0}")]
    Preparation(EffectPreparationError),
    #[error("failed to compose the production effect registry: {0}")]
    Composition(EffectCompositionError),
}

pub fn production_effect_providers(
) -> Result<Vec<Box<dyn EffectCapabilityProvider>>, ProductionEffectCompositionError> {
    Ok(vec![Box::new(
        ChorusCapability::new().map_err(ProductionEffectCompositionError::Capability)?,
    )])
}

pub fn production_effect_preparers(
) -> Result<Vec<Box<dyn EffectPreparer>>, ProductionEffectCompositionError> {
    Ok(vec![Box::new(
        ChorusPreparer::new().map_err(ProductionEffectCompositionError::Preparation)?,
    )])
}

pub fn production_effect_registry(
) -> Result<EffectCapabilityRegistry, ProductionEffectCompositionError> {
    let providers = production_effect_providers()?;
    let preparers = production_effect_preparers()?;
    compose_effect_registry(&providers, &preparers)
        .map_err(ProductionEffectCompositionError::Composition)
}

pub fn production_chorus_config(
    slot_id: EffectSlotId,
) -> Result<PostEffectConfig, ProductionEffectCompositionError> {
    ChorusCapability::new()
        .map_err(ProductionEffectCompositionError::Capability)?
        .default_config(slot_id)
        .map_err(ProductionEffectCompositionError::Capability)
}
