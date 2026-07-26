use crate::adapter::braids_capability::BraidsCapability;
use crate::adapter::braids_preparer::BraidsPreparer;
use crate::adapter::hidef_soundfont_capability::HiDefSoundFontCapability;
use crate::adapter::hidef_soundfont_preparer::HiDefSoundFontPreparer;
use crate::synth::instrument_capability::{CapabilityError, CapabilityRegistry};
use crate::synth::instrument_capability_provider::InstrumentCapabilityProvider;
use crate::synth::instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};

/// Builds the production providers in stable fixture/discovery order.
pub fn production_instrument_providers(
) -> Result<Vec<Box<dyn InstrumentCapabilityProvider>>, CapabilityError> {
    Ok(vec![
        Box::new(HiDefSoundFontCapability::new()?),
        Box::new(BraidsCapability::new()?),
    ])
}

/// Builds the immutable production registry in stable fixture/discovery order.
pub fn production_capability_registry() -> Result<CapabilityRegistry, CapabilityError> {
    let providers = production_instrument_providers()?;
    CapabilityRegistry::new(
        providers
            .iter()
            .map(|provider| provider.descriptor())
            .collect(),
    )
}

/// Prepares both production factories in the same exact capability order.
pub fn production_instrument_preparers(
) -> Result<Vec<Box<dyn InstrumentPreparer>>, InstrumentPreparationError> {
    Ok(vec![
        Box::new(HiDefSoundFontPreparer::new()?),
        Box::new(BraidsPreparer::new()?),
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
