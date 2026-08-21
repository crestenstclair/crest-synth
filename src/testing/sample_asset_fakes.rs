use crate::synth::{
    DecodedSample, SampleAssetCatalogPort, SampleAssetError, SampleAssetId, SampleCatalogListing,
    SampleDecoderPort, SampleFolderId,
};
use std::collections::BTreeMap;

/// Deterministic control/test adapter whose complete catalog and failures are
/// supplied explicitly. It never substitutes a production asset.
#[derive(Clone, Debug, Default)]
pub struct DeterministicSampleCatalog {
    listings: BTreeMap<SampleFolderId, Result<SampleCatalogListing, SampleAssetError>>,
    assets: BTreeMap<SampleAssetId, Result<Vec<u8>, SampleAssetError>>,
}

impl DeterministicSampleCatalog {
    pub fn new(
        listings: impl IntoIterator<
            Item = (
                SampleFolderId,
                Result<SampleCatalogListing, SampleAssetError>,
            ),
        >,
        assets: impl IntoIterator<Item = (SampleAssetId, Result<Vec<u8>, SampleAssetError>)>,
    ) -> Self {
        Self {
            listings: listings.into_iter().collect(),
            assets: assets.into_iter().collect(),
        }
    }
}

impl SampleAssetCatalogPort for DeterministicSampleCatalog {
    fn list(&self, folder: &SampleFolderId) -> Result<SampleCatalogListing, SampleAssetError> {
        self.listings
            .get(folder)
            .cloned()
            .unwrap_or(Err(SampleAssetError::Unavailable))
    }

    fn read(&self, asset: &SampleAssetId) -> Result<Vec<u8>, SampleAssetError> {
        self.assets
            .get(asset)
            .cloned()
            .unwrap_or(Err(SampleAssetError::Unavailable))
    }
}

/// Deterministic decoder fake keyed by the same stable library-relative IDs
/// the production adapter receives. Input bytes remain observable and cannot
/// cause an unconfigured fixture to appear.
#[derive(Clone, Debug, Default)]
pub struct DeterministicSampleDecoder {
    decoded: BTreeMap<SampleAssetId, Result<DecodedSample, SampleAssetError>>,
}

impl DeterministicSampleDecoder {
    pub fn new(
        decoded: impl IntoIterator<Item = (SampleAssetId, Result<DecodedSample, SampleAssetError>)>,
    ) -> Self {
        Self {
            decoded: decoded.into_iter().collect(),
        }
    }
}

impl SampleDecoderPort for DeterministicSampleDecoder {
    fn decode(
        &self,
        asset: &SampleAssetId,
        bytes: &[u8],
    ) -> Result<DecodedSample, SampleAssetError> {
        if bytes.is_empty() {
            return Err(SampleAssetError::MalformedWave);
        }
        self.decoded
            .get(asset)
            .cloned()
            .unwrap_or(Err(SampleAssetError::Unavailable))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{SampleBrowserRow, SampleBrowserRowKind};

    #[test]
    fn fakes_return_only_explicit_results_and_fail_closed() {
        let asset = SampleAssetId::new("fixture.wav").unwrap();
        let folder = SampleFolderId::default();
        let listing = SampleCatalogListing::new(
            folder.clone(),
            vec![SampleBrowserRow::new(
                "cancel:",
                "CANCEL — UNCHANGED",
                SampleBrowserRowKind::Cancel,
                None,
            )
            .unwrap()],
        )
        .unwrap();
        let catalog = DeterministicSampleCatalog::new(
            [(folder.clone(), Ok(listing.clone()))],
            [(asset.clone(), Ok(vec![1]))],
        );
        assert_eq!(catalog.list(&folder), Ok(listing));
        assert_eq!(catalog.read(&asset), Ok(vec![1]));
        assert_eq!(
            catalog.read(&SampleAssetId::new("missing.wav").unwrap()),
            Err(SampleAssetError::Unavailable)
        );
        assert_eq!(
            DeterministicSampleDecoder::default().decode(&asset, &[1]),
            Err(SampleAssetError::Unavailable)
        );
    }
}
