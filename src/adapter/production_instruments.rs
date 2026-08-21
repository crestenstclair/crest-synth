use crate::adapter::braids_capability::BraidsCapability;
use crate::adapter::braids_preparer::BraidsPreparer;
use crate::adapter::hidef_soundfont_asset::HiDefSoundFontAsset;
use crate::adapter::hidef_soundfont_capability::HiDefSoundFontCapability;
use crate::adapter::hidef_soundfont_preparer::HiDefSoundFontPreparer;
use crate::adapter::sample_capability::SampleCapability;
use crate::adapter::sample_preparer::SamplePreparer;
use crate::adapter::{
    filesystem_sample_catalog::FilesystemSampleCatalog, wav_sample_decoder::WavSampleDecoder,
};
use crate::synth::instrument_capability::{CapabilityError, CapabilityRegistry};
use crate::synth::instrument_capability_provider::InstrumentCapabilityProvider;
use crate::synth::instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
use crate::synth::{SampleAssetCatalogPort, SampleAssetError, SampleAssetId, SampleDecoderPort};
use std::sync::{Arc, OnceLock};

pub const SAMPLE_LIBRARY_ROOT_ENV: &str = "CREST_SAMPLE_LIBRARY_ROOT";
pub const SAMPLE_DEFAULT_ASSET_ENV: &str = "CREST_SAMPLE_DEFAULT_ASSET";

pub type ProductionSampleRootListing = Option<(
    crate::synth::SampleFolderId,
    Result<crate::synth::SampleCatalogListing, SampleAssetError>,
)>;

static SHARED_TEST_COMPOSITION_ASSET: OnceLock<
    Result<HiDefSoundFontAsset, crate::adapter::hidef_soundfont_asset::HiDefSoundFontAssetError>,
> = OnceLock::new();
static OPTIONAL_PRODUCTION_SAMPLE: OnceLock<
    Result<Option<ProductionSamplePorts>, ProductionInstrumentCompositionError>,
> = OnceLock::new();

#[derive(Clone)]
struct ProductionSamplePorts {
    capability: SampleCapability,
    catalog: Arc<dyn SampleAssetCatalogPort>,
    decoder: Arc<dyn SampleDecoderPort>,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum ProductionInstrumentCompositionError {
    #[error("failed to load the production SoundFont asset: {0}")]
    Asset(crate::adapter::hidef_soundfont_asset::HiDefSoundFontAssetError),
    #[error("failed to construct the production capability registry: {0}")]
    Capability(CapabilityError),
    #[error("failed to construct the production instrument preparers: {0}")]
    Preparation(InstrumentPreparationError),
    #[error("invalid production Sample configuration: {0}")]
    Sample(SampleAssetError),
    #[error(
        "{SAMPLE_LIBRARY_ROOT_ENV} and {SAMPLE_DEFAULT_ASSET_ENV} must be configured together"
    )]
    IncompleteSampleConfiguration,
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

fn optional_production_sample(
) -> Result<Option<ProductionSamplePorts>, ProductionInstrumentCompositionError> {
    OPTIONAL_PRODUCTION_SAMPLE
        .get_or_init(load_optional_production_sample)
        .clone()
}

fn load_optional_production_sample(
) -> Result<Option<ProductionSamplePorts>, ProductionInstrumentCompositionError> {
    let (root, asset) = match (
        std::env::var_os(SAMPLE_LIBRARY_ROOT_ENV),
        std::env::var_os(SAMPLE_DEFAULT_ASSET_ENV),
    ) {
        (None, None) => return Ok(None),
        (Some(root), Some(asset)) => (root, asset),
        _ => return Err(ProductionInstrumentCompositionError::IncompleteSampleConfiguration),
    };
    let asset = asset.into_string().map_err(|_| {
        ProductionInstrumentCompositionError::Sample(SampleAssetError::InvalidRelativeId)
    })?;
    let asset = SampleAssetId::new(asset).map_err(ProductionInstrumentCompositionError::Sample)?;
    let catalog = Arc::new(
        FilesystemSampleCatalog::new(root).map_err(ProductionInstrumentCompositionError::Sample)?,
    );
    let decoder = Arc::new(WavSampleDecoder);
    // Registration is all-or-nothing: prove the configured default through
    // the real filesystem and WAV adapters before exposing the descriptor.
    let bytes = catalog
        .read(&asset)
        .map_err(ProductionInstrumentCompositionError::Sample)?;
    decoder
        .decode(&asset, &bytes)
        .map_err(ProductionInstrumentCompositionError::Sample)?;
    let capability =
        SampleCapability::new(asset).map_err(ProductionInstrumentCompositionError::Capability)?;
    Ok(Some(ProductionSamplePorts {
        capability,
        catalog,
        decoder,
    }))
}

/// Builds the production providers in stable fixture/discovery order.
pub fn production_instrument_providers(
) -> Result<Vec<Box<dyn InstrumentCapabilityProvider>>, ProductionInstrumentCompositionError> {
    let mut providers: Vec<Box<dyn InstrumentCapabilityProvider>> = vec![
        Box::new(production_soundfont_capability()?),
        Box::new(
            BraidsCapability::new().map_err(ProductionInstrumentCompositionError::Capability)?,
        ),
    ];
    if let Some(sample) = optional_production_sample()? {
        providers.push(Box::new(sample.capability));
    }
    Ok(providers)
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
    let mut preparers: Vec<Box<dyn InstrumentPreparer>> = vec![
        Box::new(
            HiDefSoundFontPreparer::new(production_soundfont_asset()?)
                .map_err(ProductionInstrumentCompositionError::Preparation)?,
        ),
        Box::new(BraidsPreparer::new().map_err(ProductionInstrumentCompositionError::Preparation)?),
    ];
    if let Some(sample) = optional_production_sample()? {
        preparers.push(Box::new(
            SamplePreparer::new(sample.catalog, sample.decoder)
                .map_err(ProductionInstrumentCompositionError::Preparation)?,
        ));
    }
    Ok(preparers)
}

/// Returns the configured production library's root listing for reducer-owned
/// browser composition. Absence means Sample is not installed; a configured
/// adapter failure remains typed and never fabricates browser rows.
pub fn production_sample_root_listing(
) -> Result<ProductionSampleRootListing, ProductionInstrumentCompositionError> {
    let Some(sample) = optional_production_sample()? else {
        return Ok(None);
    };
    let folder = crate::synth::SampleFolderId::default();
    Ok(Some((folder.clone(), sample.catalog.list(&folder))))
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
