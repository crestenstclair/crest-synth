use crate::adapter::wav_sample_decoder::WavSampleDecoder;
use crate::synth::{
    SampleAssetCatalogPort, SampleAssetError, SampleAssetId, SampleBrowserRow,
    SampleBrowserRowKind, SampleCatalogListing, SampleFolderId, MAX_SAMPLE_SOURCE_BYTES,
};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Configured library-root filesystem adapter with no native dialog surface.
#[derive(Clone, Debug)]
pub struct FilesystemSampleCatalog {
    root: PathBuf,
}

impl FilesystemSampleCatalog {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, SampleAssetError> {
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(|_| SampleAssetError::Unavailable)?;
        if !root.is_dir() {
            return Err(SampleAssetError::Unavailable);
        }
        Ok(Self { root })
    }

    fn resolve(&self, relative: &str) -> Result<PathBuf, SampleAssetError> {
        let joined = if relative.is_empty() {
            self.root.clone()
        } else {
            self.root.join(relative)
        };
        let canonical = joined
            .canonicalize()
            .map_err(|_| SampleAssetError::Unavailable)?;
        if !canonical.starts_with(&self.root) {
            return Err(SampleAssetError::PathEscape);
        }
        Ok(canonical)
    }

    fn child_id(folder: &SampleFolderId, name: &str) -> String {
        if folder.as_str().is_empty() {
            name.to_owned()
        } else {
            format!("{}/{}", folder.as_str(), name)
        }
    }
}

impl SampleAssetCatalogPort for FilesystemSampleCatalog {
    fn list(&self, folder: &SampleFolderId) -> Result<SampleCatalogListing, SampleAssetError> {
        let path = self.resolve(folder.as_str())?;
        if !path.is_dir() {
            return Err(SampleAssetError::Unavailable);
        }
        let mut folders = Vec::new();
        let mut files = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|_| SampleAssetError::Unavailable)? {
            let entry = entry.map_err(|_| SampleAssetError::Unavailable)?;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.is_empty() || name == "." || name == ".." || name.contains('/') {
                continue;
            }
            let relative = Self::child_id(folder, &name);
            let canonical = self.resolve(&relative)?;
            if canonical.is_dir() {
                let id = SampleFolderId::new(relative)?;
                folders.push(SampleBrowserRow::new(
                    format!("folder:{}", id.as_str()),
                    name,
                    SampleBrowserRowKind::Folder(id),
                    None,
                )?);
            } else if canonical.is_file()
                && canonical
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
            {
                let bytes = canonical
                    .metadata()
                    .map_err(|_| SampleAssetError::Unavailable)?
                    .len();
                let id = SampleAssetId::new(relative)?;
                let metadata = self
                    .read(&id)
                    .and_then(|source| WavSampleDecoder::metadata(&id, &source));
                files.push(
                    SampleBrowserRow::new(
                        format!("file:{id}"),
                        name,
                        SampleBrowserRowKind::File(id),
                        Some(bytes),
                    )?
                    .with_metadata(metadata)?,
                );
            }
        }
        let key = |row: &SampleBrowserRow| (row.label().to_lowercase(), row.label().to_owned());
        folders.sort_by_key(&key);
        files.sort_by_key(&key);
        let mut rows = Vec::new();
        if let Some(parent) = folder.parent() {
            rows.push(SampleBrowserRow::new(
                format!("parent:{}", folder.as_str()),
                "..",
                SampleBrowserRowKind::Parent(parent),
                None,
            )?);
        }
        rows.extend(folders);
        rows.extend(files);
        rows.push(SampleBrowserRow::new(
            format!("cancel:{}", folder.as_str()),
            "CANCEL — UNCHANGED",
            SampleBrowserRowKind::Cancel,
            None,
        )?);
        SampleCatalogListing::new(folder.clone(), rows)
    }

    fn read(&self, asset: &SampleAssetId) -> Result<Vec<u8>, SampleAssetError> {
        let path = self.resolve(asset.as_str())?;
        if !path.is_file() {
            return Err(SampleAssetError::Unavailable);
        }
        let length = path
            .metadata()
            .map_err(|_| SampleAssetError::Unavailable)?
            .len();
        if length > MAX_SAMPLE_SOURCE_BYTES {
            return Err(SampleAssetError::SourceTooLarge);
        }
        let capacity = usize::try_from(length).map_err(|_| SampleAssetError::SourceTooLarge)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| SampleAssetError::AllocationFailed)?;
        File::open(path)
            .map_err(|_| SampleAssetError::Unavailable)?
            .take(MAX_SAMPLE_SOURCE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| SampleAssetError::Unavailable)?;
        if u64::try_from(bytes.len()).map_err(|_| SampleAssetError::SourceTooLarge)?
            > MAX_SAMPLE_SOURCE_BYTES
        {
            return Err(SampleAssetError::SourceTooLarge);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::FilesystemSampleCatalog;
    use crate::synth::{
        SampleAssetCatalogPort, SampleAssetError, SampleAssetId, SampleBrowserRowKind,
        SampleFolderId,
    };
    use std::path::PathBuf;

    struct FixtureRoot(PathBuf);

    fn mono_pcm16_wave() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&38_u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&48_000_u32.to_le_bytes());
        bytes.extend_from_slice(&96_000_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&2_u32.to_le_bytes());
        bytes.extend_from_slice(&0_i16.to_le_bytes());
        bytes
    }

    impl FixtureRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "crest-sample-catalog-{}-{}",
                std::process::id(),
                std::thread::current().name().unwrap_or("test")
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("Drums")).unwrap();
            std::fs::write(root.join("zeta.wav"), b"wave-z").unwrap();
            std::fs::write(root.join("Alpha.WAV"), mono_pcm16_wave()).unwrap();
            std::fs::write(root.join("ignore.mp3"), b"mp3").unwrap();
            std::fs::write(root.join("Drums/kick.wav"), b"kick").unwrap();
            Self(root)
        }
    }
    impl Drop for FixtureRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn listing_is_stable_folder_first_and_ids_are_library_relative() {
        let root = FixtureRoot::new();
        let catalog = FilesystemSampleCatalog::new(&root.0).unwrap();
        let listing = catalog.list(&SampleFolderId::default()).unwrap();
        let labels = listing
            .rows()
            .iter()
            .map(|row| row.label())
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            ["Drums", "Alpha.WAV", "zeta.wav", "CANCEL — UNCHANGED"]
        );
        assert!(matches!(
            listing.rows()[0].kind(),
            SampleBrowserRowKind::Folder(folder) if folder.as_str() == "Drums"
        ));
        let alpha = &listing.rows()[1];
        let metadata = alpha
            .metadata()
            .expect("eligible files carry a typed metadata state")
            .as_ref()
            .expect("the valid WAV metadata is admitted");
        assert_eq!(metadata.sample_rate(), 48_000);
        assert_eq!(metadata.channels(), 1);
        assert_eq!(metadata.bits_per_sample(), 16);
        assert_eq!(metadata.frames(), 1);
        assert!(matches!(
            listing.rows()[2].metadata(),
            Some(Err(SampleAssetError::UnsupportedContainer))
        ));
        let bytes = catalog
            .read(&SampleAssetId::new("Drums/kick.wav").unwrap())
            .unwrap();
        assert_eq!(bytes, b"kick");
    }

    #[cfg(unix)]
    #[test]
    fn traversal_and_symlink_escape_are_typed_and_never_read() {
        use std::os::unix::fs::symlink;
        let root = FixtureRoot::new();
        let outside = root.0.parent().unwrap().join("outside.wav");
        std::fs::write(&outside, b"outside").unwrap();
        symlink(&outside, root.0.join("escape.wav")).unwrap();
        let catalog = FilesystemSampleCatalog::new(&root.0).unwrap();
        assert_eq!(
            catalog.read(&SampleAssetId::new("escape.wav").unwrap()),
            Err(SampleAssetError::PathEscape)
        );
        assert_eq!(
            SampleAssetId::new("../outside.wav"),
            Err(SampleAssetError::InvalidRelativeId)
        );
        let _ = std::fs::remove_file(outside);
    }
}
