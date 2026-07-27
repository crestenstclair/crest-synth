use crate::adapter::hidef_soundfont_capability::HIDEF_SOUNDFONT_PATH;
use crate::adapter::soundfont_voice_engine::{PreparedSoundFontBank, PreparedSoundFontBankError};
use crate::synth::{SoundFontPresetCatalog, SoundFontPresetCatalogError, SoundFontPresetSource};
use core::fmt;
use rustysynth::SoundFont;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// The two immutable projections produced by the one production SF2 parse.
#[derive(Clone)]
pub struct HiDefSoundFontAsset {
    catalog: Arc<SoundFontPresetCatalog>,
    prepared_bank: Arc<PreparedSoundFontBank>,
    parse_count: usize,
}

impl HiDefSoundFontAsset {
    /// Opens the fixed production asset and parses it exactly once.
    pub fn load() -> Result<Self, HiDefSoundFontAssetError> {
        Self::load_from_path(Path::new(HIDEF_SOUNDFONT_PATH))
    }

    fn load_from_path(path: &Path) -> Result<Self, HiDefSoundFontAssetError> {
        if path != Path::new(HIDEF_SOUNDFONT_PATH) {
            return Err(HiDefSoundFontAssetError::UnexpectedPath);
        }
        let mut file = File::open(path).map_err(|_| HiDefSoundFontAssetError::FileOpen)?;
        let sound_font = SoundFont::new(&mut file).map_err(|_| HiDefSoundFontAssetError::Parse)?;
        let (prepared_bank, playable_source_ordinals) =
            PreparedSoundFontBank::from_sound_font(&sound_font)
                .map_err(HiDefSoundFontAssetError::from_prepared_bank)?;

        let mut playable = vec![false; sound_font.get_presets().len()];
        for source_ordinal in playable_source_ordinals {
            let slot = playable
                .get_mut(source_ordinal)
                .ok_or(HiDefSoundFontAssetError::Metadata)?;
            *slot = true;
        }
        let sources =
            sound_font
                .get_presets()
                .iter()
                .enumerate()
                .map(|(source_ordinal, preset)| {
                    SoundFontPresetSource::new(
                        source_ordinal,
                        preset.get_bank_number(),
                        preset.get_patch_number(),
                        preset.get_name(),
                        playable[source_ordinal],
                    )
                });
        let catalog = SoundFontPresetCatalog::from_sources(sources)
            .map_err(HiDefSoundFontAssetError::Catalog)?;

        // `sound_font` and all parser/name-bearing allocations are dropped on
        // this control-owned stack before either projection is returned.
        Ok(Self {
            catalog: Arc::new(catalog),
            prepared_bank: Arc::new(prepared_bank),
            parse_count: 1,
        })
    }

    pub fn catalog(&self) -> Arc<SoundFontPresetCatalog> {
        Arc::clone(&self.catalog)
    }

    pub(crate) fn prepared_bank(&self) -> Arc<PreparedSoundFontBank> {
        Arc::clone(&self.prepared_bank)
    }

    pub const fn parse_count(&self) -> usize {
        self.parse_count
    }

    /// Returns the audited count of forbidden metadata categories retained by
    /// callback-reachable numeric storage.
    pub fn callback_metadata_counts(&self) -> [usize; 4] {
        let counts = self.prepared_bank.callback_metadata_counts();
        [
            counts.strings,
            counts.paths,
            counts.catalog_entries,
            counts.parser_structures,
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HiDefSoundFontAssetError {
    UnexpectedPath,
    FileOpen,
    Parse,
    Metadata,
    SampleStorage,
    InvalidPresetAddress { source_ordinal: usize },
    InvalidInstrumentReference { source_ordinal: usize },
    InvalidSampleReference { source_ordinal: usize },
    InvalidRegion { source_ordinal: usize },
    Catalog(SoundFontPresetCatalogError),
}

impl HiDefSoundFontAssetError {
    fn from_prepared_bank(error: PreparedSoundFontBankError) -> Self {
        match error {
            PreparedSoundFontBankError::SampleStorage => Self::SampleStorage,
            PreparedSoundFontBankError::InvalidPresetAddress { source_ordinal } => {
                Self::InvalidPresetAddress { source_ordinal }
            }
            PreparedSoundFontBankError::InvalidInstrumentReference { source_ordinal } => {
                Self::InvalidInstrumentReference { source_ordinal }
            }
            PreparedSoundFontBankError::InvalidSampleReference { source_ordinal } => {
                Self::InvalidSampleReference { source_ordinal }
            }
            PreparedSoundFontBankError::InvalidRegion { source_ordinal } => {
                Self::InvalidRegion { source_ordinal }
            }
        }
    }
}

impl fmt::Display for HiDefSoundFontAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedPath => formatter.write_str("HiDef asset path is not the fixed asset"),
            Self::FileOpen => formatter.write_str("failed to open the fixed HiDef SoundFont"),
            Self::Parse => formatter.write_str("failed to parse the fixed HiDef SoundFont"),
            Self::Metadata => formatter.write_str("HiDef SoundFont metadata is inconsistent"),
            Self::SampleStorage => {
                formatter.write_str("failed to allocate numeric HiDef sample storage")
            }
            Self::InvalidPresetAddress { source_ordinal } => write!(
                formatter,
                "HiDef preset {source_ordinal} has an invalid numeric address"
            ),
            Self::InvalidInstrumentReference { source_ordinal } => write!(
                formatter,
                "HiDef preset {source_ordinal} references an invalid instrument"
            ),
            Self::InvalidSampleReference { source_ordinal } => write!(
                formatter,
                "HiDef preset {source_ordinal} references an invalid sample"
            ),
            Self::InvalidRegion { source_ordinal } => {
                write!(
                    formatter,
                    "HiDef preset {source_ordinal} has an invalid region"
                )
            }
            Self::Catalog(error) => write!(formatter, "invalid HiDef preset catalog: {error}"),
        }
    }
}

impl std::error::Error for HiDefSoundFontAssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_asset_is_parsed_once_into_sorted_control_and_numeric_projections() {
        let asset = HiDefSoundFontAsset::load().unwrap();
        assert_eq!(asset.parse_count(), 1);
        assert!(!asset.catalog().entries().is_empty());
        assert!(asset
            .catalog()
            .entries()
            .windows(2)
            .all(|pair| pair[0].id() < pair[1].id()));
        assert!(asset
            .catalog()
            .entries()
            .iter()
            .all(|entry| asset.prepared_bank().has_preset(entry.id())));
        assert_eq!(
            asset.prepared_bank().callback_metadata_counts(),
            crate::adapter::soundfont_voice_engine::CallbackSoundFontMetadataCounts {
                strings: 0,
                paths: 0,
                catalog_entries: 0,
                parser_structures: 0,
            }
        );
    }
}
