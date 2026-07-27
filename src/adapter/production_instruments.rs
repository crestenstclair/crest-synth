use crate::adapter::braids_capability::BraidsCapability;
use crate::adapter::braids_preparer::BraidsPreparer;
use crate::adapter::hidef_soundfont_asset::HiDefSoundFontAsset;
use crate::adapter::hidef_soundfont_capability::HiDefSoundFontCapability;
use crate::adapter::hidef_soundfont_preparer::HiDefSoundFontPreparer;
use crate::synth::instrument_capability::{CapabilityError, CapabilityRegistry};
use crate::synth::instrument_capability_provider::InstrumentCapabilityProvider;
use crate::synth::instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
use std::sync::OnceLock;

static SHARED_TEST_COMPOSITION_ASSET: OnceLock<
    Result<HiDefSoundFontAsset, crate::adapter::hidef_soundfont_asset::HiDefSoundFontAssetError>,
> = OnceLock::new();

#[derive(Clone, Debug, thiserror::Error)]
pub enum ProductionInstrumentCompositionError {
    #[error("failed to load the production SoundFont asset: {0}")]
    Asset(crate::adapter::hidef_soundfont_asset::HiDefSoundFontAssetError),
    #[error("failed to construct the production capability registry: {0}")]
    Capability(CapabilityError),
    #[error("failed to construct the production instrument preparers: {0}")]
    Preparation(InstrumentPreparationError),
}

/// Shared process-local asset used by test composition helpers. The standalone
/// composition root loads and owns its explicit asset instead.
pub fn production_soundfont_asset(
) -> Result<&'static HiDefSoundFontAsset, ProductionInstrumentCompositionError> {
    SHARED_TEST_COMPOSITION_ASSET
        .get_or_init(HiDefSoundFontAsset::load)
        .as_ref()
        .map_err(|error| ProductionInstrumentCompositionError::Asset(error.clone()))
}

pub fn production_soundfont_capability(
) -> Result<HiDefSoundFontCapability, ProductionInstrumentCompositionError> {
    HiDefSoundFontCapability::new(production_soundfont_asset()?.catalog())
        .map_err(ProductionInstrumentCompositionError::Capability)
}

/// Builds the production providers in stable fixture/discovery order.
pub fn production_instrument_providers(
) -> Result<Vec<Box<dyn InstrumentCapabilityProvider>>, ProductionInstrumentCompositionError> {
    Ok(vec![
        Box::new(production_soundfont_capability()?),
        Box::new(
            BraidsCapability::new().map_err(ProductionInstrumentCompositionError::Capability)?,
        ),
    ])
}

/// Builds the immutable production registry in stable fixture/discovery order.
pub fn production_capability_registry(
) -> Result<CapabilityRegistry, ProductionInstrumentCompositionError> {
    let providers = production_instrument_providers()?;
    CapabilityRegistry::new(
        providers
            .iter()
            .map(|provider| provider.descriptor())
            .collect(),
    )
    .map_err(ProductionInstrumentCompositionError::Capability)
}

/// Prepares both production factories in the same exact capability order.
pub fn production_instrument_preparers(
) -> Result<Vec<Box<dyn InstrumentPreparer>>, ProductionInstrumentCompositionError> {
    Ok(vec![
        Box::new(
            HiDefSoundFontPreparer::new(production_soundfont_asset()?)
                .map_err(ProductionInstrumentCompositionError::Preparation)?,
        ),
        Box::new(BraidsPreparer::new().map_err(ProductionInstrumentCompositionError::Preparation)?),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::braids_capability::BRAIDS_CAPABILITY_ID;
    use crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID;

    #[test]
    fn production_composition_installs_exactly_both_matching_engine_ports() {
        let providers = production_instrument_providers().unwrap();
        let registry = production_capability_registry().unwrap();
        let preparers = production_instrument_preparers().unwrap();
        assert_eq!(providers.len(), registry.descriptors().len());
        assert_eq!(
            registry
                .descriptors()
                .iter()
                .map(|descriptor| descriptor.id().as_str())
                .collect::<Vec<_>>(),
            [HIDEF_CAPABILITY_ID, BRAIDS_CAPABILITY_ID]
        );
        assert_eq!(
            preparers
                .iter()
                .map(|preparer| preparer.capability_id().as_str())
                .collect::<Vec<_>>(),
            [HIDEF_CAPABILITY_ID, BRAIDS_CAPABILITY_ID]
        );
    }
}
