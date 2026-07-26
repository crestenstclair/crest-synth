use crate::synth::capability_id::CapabilityId;
use crate::synth::instrument_capability::{
    AssetAssignment, CapabilityError, CapabilityRegistry, InstrumentConfig, ParameterAssignment,
    ParameterDefault,
};
use crate::synth::instrument_capability_provider::InstrumentCapabilityProvider;

/// Constructs provider-validated instrument configs from descriptor defaults.
///
/// This control-side service owns no Patch, prepared engine, graph, renderer, or
/// inactive-engine cache. It only translates the selected immutable descriptor
/// into the generic provider inputs already declared by that descriptor.
pub struct DescriptorDefaultConfigFactory {
    registry: CapabilityRegistry,
    providers: Vec<Box<dyn InstrumentCapabilityProvider>>,
}

impl DescriptorDefaultConfigFactory {
    pub const fn new(
        registry: CapabilityRegistry,
        providers: Vec<Box<dyn InstrumentCapabilityProvider>>,
    ) -> Self {
        Self {
            registry,
            providers,
        }
    }

    pub const fn registry(&self) -> &CapabilityRegistry {
        &self.registry
    }

    /// Creates one exact descriptor-default config without capability-specific
    /// branching or substitution.
    pub fn create(
        &self,
        capability_id: &CapabilityId,
    ) -> Result<InstrumentConfig, CapabilityError> {
        let descriptor = self
            .registry
            .descriptor(capability_id)
            .ok_or_else(|| CapabilityError::UnknownCapability(capability_id.clone()))?;

        let mut matching = self
            .providers
            .iter()
            .filter(|provider| provider.descriptor().id() == capability_id);
        let provider = matching
            .next()
            .ok_or_else(|| CapabilityError::ProviderRegistryMismatch(capability_id.clone()))?;
        if matching.next().is_some() || provider.descriptor() != *descriptor {
            return Err(CapabilityError::ProviderRegistryMismatch(
                capability_id.clone(),
            ));
        }

        let mut values = Vec::with_capacity(descriptor.parameters().count());
        let mut assets = Vec::with_capacity(descriptor.asset_requirements().len());
        for parameter in descriptor.parameters() {
            match parameter.default_value() {
                ParameterDefault::Value(value) => values.push(ParameterAssignment::new(
                    parameter.id().clone(),
                    value.clone(),
                )),
                ParameterDefault::Asset(reference) => assets.push(AssetAssignment::new(
                    parameter.id().clone(),
                    reference.clone(),
                )),
            }
        }

        let config = provider.create_config(&values, &assets)?;
        self.registry.validate_config(&config)?;
        if config.capability_id() != capability_id {
            return Err(CapabilityError::ProviderRegistryMismatch(
                capability_id.clone(),
            ));
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::DescriptorDefaultConfigFactory;
    use crate::adapter::braids_capability::{
        BraidsCapability, BRAIDS_CAPABILITY_ID, BRAIDS_COLOR_PARAMETER_ID, BRAIDS_MODELS,
        BRAIDS_MODEL_PARAMETER_ID, BRAIDS_TIMBRE_PARAMETER_ID,
    };
    use crate::adapter::hidef_soundfont_capability::{
        HiDefSoundFontCapability, HIDEF_CAPABILITY_ID, HIDEF_SOUNDFONT_PATH,
        SOUNDFONT_BANK_PARAMETER_ID, SOUNDFONT_FILE_PARAMETER_ID,
        SOUNDFONT_PERCUSSION_PARAMETER_ID, SOUNDFONT_PROGRAM_PARAMETER_ID,
    };
    use crate::synth::instrument_capability::{
        AssetAssignment, CapabilityDescriptor, CapabilityError, CapabilityRegistry,
        InstrumentConfig, ParameterAssignment, ParameterValue,
    };
    use crate::synth::{CapabilityId, InstrumentCapabilityProvider, ParameterId};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn capability_id(value: &str) -> CapabilityId {
        CapabilityId::new(value).unwrap()
    }

    fn parameter_id(value: &str) -> ParameterId {
        ParameterId::new(value).unwrap()
    }

    fn production_factory() -> DescriptorDefaultConfigFactory {
        let providers: Vec<Box<dyn InstrumentCapabilityProvider>> = vec![
            Box::new(HiDefSoundFontCapability::new().unwrap()),
            Box::new(BraidsCapability::new().unwrap()),
        ];
        let registry = CapabilityRegistry::new(
            providers
                .iter()
                .map(|provider| provider.descriptor())
                .collect(),
        )
        .unwrap();
        DescriptorDefaultConfigFactory::new(registry, providers)
    }

    #[test]
    fn descriptor_default_config_factory_creates_both_exact_production_defaults() {
        let factory = production_factory();

        let soundfont = factory.create(&capability_id(HIDEF_CAPABILITY_ID)).unwrap();
        assert_eq!(
            soundfont.value(&parameter_id(SOUNDFONT_BANK_PARAMETER_ID)),
            Some(&ParameterValue::Stepped(0))
        );
        assert_eq!(
            soundfont.value(&parameter_id(SOUNDFONT_PROGRAM_PARAMETER_ID)),
            Some(&ParameterValue::Stepped(0))
        );
        assert_eq!(
            soundfont.value(&parameter_id(SOUNDFONT_PERCUSSION_PARAMETER_ID)),
            Some(&ParameterValue::Toggle(false))
        );
        assert_eq!(
            soundfont
                .asset_reference(&parameter_id(SOUNDFONT_FILE_PARAMETER_ID))
                .map(|reference| reference.locator()),
            Some(HIDEF_SOUNDFONT_PATH)
        );

        let braids = factory
            .create(&capability_id(BRAIDS_CAPABILITY_ID))
            .unwrap();
        assert_eq!(
            braids.value(&parameter_id(BRAIDS_MODEL_PARAMETER_ID)),
            Some(&ParameterValue::Choice(BRAIDS_MODELS[0].id.to_owned()))
        );
        assert_eq!(
            braids.value(&parameter_id(BRAIDS_TIMBRE_PARAMETER_ID)),
            Some(&ParameterValue::continuous(0.5).unwrap())
        );
        assert_eq!(
            braids.value(&parameter_id(BRAIDS_COLOR_PARAMETER_ID)),
            Some(&ParameterValue::continuous(0.5).unwrap())
        );
        assert!(braids.asset_references().is_empty());
    }

    struct NeverCreateProvider {
        descriptor: CapabilityDescriptor,
        calls: Arc<AtomicUsize>,
    }

    impl InstrumentCapabilityProvider for NeverCreateProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            self.descriptor.clone()
        }

        fn create_config(
            &self,
            _values: &[ParameterAssignment],
            _asset_references: &[AssetAssignment],
        ) -> Result<InstrumentConfig, CapabilityError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            unreachable!("a mismatched provider must not be called")
        }
    }

    struct InvalidOutputProvider {
        descriptor: CapabilityDescriptor,
    }

    impl InstrumentCapabilityProvider for InvalidOutputProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            self.descriptor.clone()
        }

        fn create_config(
            &self,
            _values: &[ParameterAssignment],
            _asset_references: &[AssetAssignment],
        ) -> Result<InstrumentConfig, CapabilityError> {
            Ok(InstrumentConfig::from_parts(
                self.descriptor.id().clone(),
                Vec::new(),
                Vec::new(),
            ))
        }
    }

    #[test]
    fn descriptor_default_config_factory_rejects_unknown_missing_duplicate_and_mismatched_providers_without_fallback(
    ) {
        let hidef = HiDefSoundFontCapability::new().unwrap();
        let braids = BraidsCapability::new().unwrap();
        let registry =
            CapabilityRegistry::new(vec![hidef.descriptor(), braids.descriptor()]).unwrap();

        let missing = DescriptorDefaultConfigFactory::new(
            registry.clone(),
            vec![Box::new(HiDefSoundFontCapability::new().unwrap())],
        );
        assert_eq!(
            missing.create(&capability_id(BRAIDS_CAPABILITY_ID)),
            Err(CapabilityError::ProviderRegistryMismatch(capability_id(
                BRAIDS_CAPABILITY_ID
            )))
        );
        let unknown = capability_id("instrument.unknown.test");
        assert_eq!(
            missing.create(&unknown),
            Err(CapabilityError::UnknownCapability(unknown))
        );

        let duplicate = DescriptorDefaultConfigFactory::new(
            registry.clone(),
            vec![
                Box::new(HiDefSoundFontCapability::new().unwrap()),
                Box::new(HiDefSoundFontCapability::new().unwrap()),
                Box::new(BraidsCapability::new().unwrap()),
            ],
        );
        assert_eq!(
            duplicate.create(&capability_id(HIDEF_CAPABILITY_ID)),
            Err(CapabilityError::ProviderRegistryMismatch(capability_id(
                HIDEF_CAPABILITY_ID
            )))
        );

        let canonical = hidef.descriptor();
        let mismatched = CapabilityDescriptor::new(
            canonical.id().clone(),
            "Wrong provider descriptor",
            canonical.semantic_accent(),
            canonical.sections().to_vec(),
            canonical.asset_requirements().to_vec(),
            canonical.voice_policy(),
            canonical.supported_midi_kinds().to_vec(),
        )
        .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let factory = DescriptorDefaultConfigFactory::new(
            registry,
            vec![Box::new(NeverCreateProvider {
                descriptor: mismatched,
                calls: Arc::clone(&calls),
            })],
        );
        assert_eq!(
            factory.create(&capability_id(HIDEF_CAPABILITY_ID)),
            Err(CapabilityError::ProviderRegistryMismatch(capability_id(
                HIDEF_CAPABILITY_ID
            )))
        );
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn descriptor_default_config_factory_revalidates_provider_output() {
        let hidef = HiDefSoundFontCapability::new().unwrap();
        let registry = CapabilityRegistry::new(vec![hidef.descriptor()]).unwrap();
        let factory = DescriptorDefaultConfigFactory::new(
            registry,
            vec![Box::new(InvalidOutputProvider {
                descriptor: hidef.descriptor(),
            })],
        );

        assert_eq!(
            factory.create(&capability_id(HIDEF_CAPABILITY_ID)),
            Err(CapabilityError::MissingParameter(parameter_id(
                SOUNDFONT_BANK_PARAMETER_ID
            )))
        );
    }
}
