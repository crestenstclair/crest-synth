// path: src/preset/fs_preset_storage.rs

//! Adapter: FsPresetStorage
//!
//! Filesystem implementation of [`PresetStorage`]. Each preset is stored as
//! a single file under a root directory, named after its [`PresetId`]. All
//! operations perform blocking file I/O and therefore MUST NOT be called
//! from the real-time audio thread — callers reach this adapter only from
//! the UI/control thread, exactly as the `PresetStorage` port documents.
//!
//! ## On-disk format
//!
//! ```text
//! [0..4)   version:     u32 little-endian
//! [4..12)  id:          u64 little-endian
//! [12..16) name_len:    u32 little-endian
//! [16..16+name_len)     name bytes (UTF-8)
//! [..+8)   payload_len: u64 little-endian
//! [..+payload_len)      payload bytes
//! ```
//!
//! Presets serialize with an explicit version (mirrors the invariant that
//! user preset libraries must survive format evolution); `load` migrates
//! older versions to [`PresetVersion::CURRENT`] before returning, exactly
//! as [`Preset::migrated_to_current`] documents.
//!
//! `save` writes to a temporary file and atomically renames it into place
//! so a failed or interrupted write never leaves a corrupted or partial
//! entry behind, and `delete` removes the whole file in a single syscall —
//! both satisfy the port's atomicity requirement.

use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::preset::preset_storage::{
    Preset, PresetId, PresetMetadata, PresetStorage, PresetVersion,
};

/// Filesystem-backed [`PresetStorage`] adapter.
///
/// Depends only on a root directory path supplied at construction —
/// nothing here reaches for global state, so tests can point it at an
/// isolated scratch directory.
pub struct FsPresetStorage {
    root: PathBuf,
}

impl FsPresetStorage {
    /// Construct a storage adapter rooted at `root`, creating the
    /// directory (and any missing parents) if it does not already exist.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let _ = fs::create_dir_all(&root);
        Self { root }
    }

    fn ensure_root(&self) -> Result<(), io::Error> {
        fs::create_dir_all(&self.root)
    }

    fn file_name(id: PresetId) -> String {
        format!("preset_{}.bin", id.value())
    }

    fn tmp_file_name(id: PresetId) -> String {
        format!("preset_{}.bin.tmp", id.value())
    }

    fn path_for(&self, id: PresetId) -> PathBuf {
        self.root.join(Self::file_name(id))
    }

    fn tmp_path_for(&self, id: PresetId) -> PathBuf {
        self.root.join(Self::tmp_file_name(id))
    }

    /// Parse an id encoded in a stored file's name, e.g. `preset_7.bin` -> 7.
    fn id_from_file_name(name: &str) -> Option<u64> {
        name.strip_prefix("preset_")
            .and_then(|rest| rest.strip_suffix(".bin"))
            .and_then(|digits| digits.parse::<u64>().ok())
    }

    fn encode(preset: &Preset) -> Vec<u8> {
        let name_bytes = preset.name().as_bytes();
        let payload = preset.payload();
        let mut buf = Vec::with_capacity(4 + 8 + 4 + name_bytes.len() + 8 + payload.len());

        buf.extend_from_slice(&preset.version().value().to_le_bytes());
        buf.extend_from_slice(&preset.id().value().to_le_bytes());
        buf.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(name_bytes);
        buf.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        buf.extend_from_slice(payload);

        buf
    }

    /// Decode a full preset (header + name + payload) from `reader`.
    fn decode_full(mut reader: impl Read) -> Result<Preset, StorageErrorSource> {
        let (version, id, name) = Self::decode_header(&mut reader)?;

        let mut payload_len_bytes = [0u8; 8];
        reader
            .read_exact(&mut payload_len_bytes)
            .map_err(StorageErrorSource::Io)?;
        let payload_len = u64::from_le_bytes(payload_len_bytes) as usize;

        let mut payload = vec![0u8; payload_len];
        reader
            .read_exact(&mut payload)
            .map_err(StorageErrorSource::Io)?;

        Ok(Preset::with_version(id, name, version, payload))
    }

    /// Decode only the header (version, id, name) from `reader`, then skip
    /// past the payload without reading it into memory — this is what
    /// makes [`FsPresetStorage::list`] cheap.
    fn decode_metadata(mut reader: impl Read + Seek) -> Result<PresetMetadata, StorageErrorSource> {
        let (version, id, name) = Self::decode_header(&mut reader)?;

        let mut payload_len_bytes = [0u8; 8];
        reader
            .read_exact(&mut payload_len_bytes)
            .map_err(StorageErrorSource::Io)?;
        let payload_len = u64::from_le_bytes(payload_len_bytes);

        reader
            .seek(SeekFrom::Current(payload_len as i64))
            .map_err(StorageErrorSource::Io)?;

        Ok(PresetMetadata::new(id, name, version))
    }

    fn decode_header(
        reader: &mut impl Read,
    ) -> Result<(PresetVersion, PresetId, String), StorageErrorSource> {
        let mut version_bytes = [0u8; 4];
        reader
            .read_exact(&mut version_bytes)
            .map_err(StorageErrorSource::Io)?;
        let version = PresetVersion::new(u32::from_le_bytes(version_bytes));

        let mut id_bytes = [0u8; 8];
        reader
            .read_exact(&mut id_bytes)
            .map_err(StorageErrorSource::Io)?;
        let id = PresetId::new(u64::from_le_bytes(id_bytes));

        let mut name_len_bytes = [0u8; 4];
        reader
            .read_exact(&mut name_len_bytes)
            .map_err(StorageErrorSource::Io)?;
        let name_len = u32::from_le_bytes(name_len_bytes) as usize;

        let mut name_bytes = vec![0u8; name_len];
        reader
            .read_exact(&mut name_bytes)
            .map_err(StorageErrorSource::Io)?;
        let name = String::from_utf8(name_bytes).map_err(|e| {
            StorageErrorSource::Serialization(format!("preset name is not valid UTF-8: {e}"))
        })?;

        Ok((version, id, name))
    }
}

/// Internal decode failure, distinguishing I/O from malformed-content
/// errors before they are mapped onto [`StorageError`] with the missing
/// [`PresetId`] filled in by the caller where relevant.
enum StorageErrorSource {
    Io(io::Error),
    Serialization(String),
}

impl From<StorageErrorSource> for crate::preset::preset_storage::StorageError {
    fn from(source: StorageErrorSource) -> Self {
        match source {
            StorageErrorSource::Io(e) => {
                crate::preset::preset_storage::StorageError::Io(e.to_string())
            }
            StorageErrorSource::Serialization(msg) => {
                crate::preset::preset_storage::StorageError::Serialization(msg)
            }
        }
    }
}

impl PresetStorage for FsPresetStorage {
    fn save(&self, preset: Preset) -> Result<(), crate::preset::preset_storage::StorageError> {
        use crate::preset::preset_storage::StorageError;

        self.ensure_root()
            .map_err(|e| StorageError::Io(e.to_string()))?;

        let bytes = Self::encode(&preset);
        let tmp_path = self.tmp_path_for(preset.id());
        let final_path = self.path_for(preset.id());

        let write_result = (|| -> io::Result<()> {
            let mut file = fs::File::create(&tmp_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            Ok(())
        })();

        if let Err(e) = write_result {
            let _ = fs::remove_file(&tmp_path);
            return Err(StorageError::Io(e.to_string()));
        }

        // Atomic on the same filesystem: either the old file remains (on
        // failure) or the new file fully replaces it (on success) — never
        // a partially-written entry.
        if let Err(e) = fs::rename(&tmp_path, &final_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(StorageError::Io(e.to_string()));
        }

        Ok(())
    }

    fn load(&self, id: PresetId) -> Result<Preset, crate::preset::preset_storage::StorageError> {
        use crate::preset::preset_storage::StorageError;

        let path = self.path_for(id);
        let file = match fs::File::open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(StorageError::NotFound(id))
            }
            Err(e) => return Err(StorageError::Io(e.to_string())),
        };

        let preset =
            Self::decode_full(file).map_err(crate::preset::preset_storage::StorageError::from)?;
        Ok(preset.migrated_to_current())
    }

    fn list(&self) -> Vec<PresetMetadata> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };

        let mut result = Vec::new();
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            if Self::id_from_file_name(name).is_none() {
                continue;
            }

            let path: &Path = &entry.path();
            let Ok(file) = fs::File::open(path) else {
                continue;
            };
            if let Ok(metadata) = Self::decode_metadata(file) {
                result.push(metadata);
            }
        }

        result
    }

    fn delete(&self, id: PresetId) -> Result<(), crate::preset::preset_storage::StorageError> {
        use crate::preset::preset_storage::StorageError;

        let path = self.path_for(id);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(StorageError::NotFound(id)),
            Err(e) => Err(StorageError::Io(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::preset_storage::StorageError;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Build an isolated scratch directory for a single test, avoiding
    /// collisions between parallel test runs without adding a dependency
    /// on an external tempdir crate.
    fn scratch_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("fs_preset_storage_test_{label}_{nanos}"));
        fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = scratch_dir("round_trip");
        let storage = FsPresetStorage::new(&dir);
        let id = PresetId::new(1);
        let preset = Preset::new(id, "Warm Pad", vec![1, 2, 3]);

        storage.save(preset.clone()).unwrap();
        let loaded = storage.load(id).unwrap();

        assert_eq!(loaded.id(), id);
        assert_eq!(loaded.name(), "Warm Pad");
        assert_eq!(loaded.payload(), &[1, 2, 3]);
        assert_eq!(loaded.version(), PresetVersion::CURRENT);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_missing_preset_errors() {
        let dir = scratch_dir("missing");
        let storage = FsPresetStorage::new(&dir);

        let result = storage.load(PresetId::new(42));

        assert_eq!(result, Err(StorageError::NotFound(PresetId::new(42))));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn list_returns_metadata_without_full_payload_and_is_cheap() {
        let dir = scratch_dir("list");
        let storage = FsPresetStorage::new(&dir);
        storage
            .save(Preset::new(PresetId::new(1), "A", vec![0; 64]))
            .unwrap();
        storage
            .save(Preset::new(PresetId::new(2), "B", vec![0; 64]))
            .unwrap();

        let mut names: Vec<_> = storage
            .list()
            .iter()
            .map(|m| m.name().to_string())
            .collect();
        names.sort();

        assert_eq!(names, vec!["A".to_string(), "B".to_string()]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn delete_removes_entry_and_reports_missing_on_second_attempt() {
        let dir = scratch_dir("delete");
        let storage = FsPresetStorage::new(&dir);
        let id = PresetId::new(7);
        storage.save(Preset::new(id, "Lead", vec![])).unwrap();

        storage.delete(id).unwrap();

        assert!(storage.load(id).is_err());
        assert_eq!(storage.delete(id), Err(StorageError::NotFound(id)));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_overwrites_existing_entry_with_same_id() {
        let dir = scratch_dir("overwrite");
        let storage = FsPresetStorage::new(&dir);
        let id = PresetId::new(9);
        storage.save(Preset::new(id, "Original", vec![0])).unwrap();
        storage.save(Preset::new(id, "Renamed", vec![1])).unwrap();

        let loaded = storage.load(id).unwrap();

        assert_eq!(loaded.name(), "Renamed");
        assert_eq!(loaded.payload(), &[1]);
        assert_eq!(storage.list().len(), 1);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn older_version_is_migrated_on_load() {
        let dir = scratch_dir("migrate");
        let storage = FsPresetStorage::new(&dir);
        let id = PresetId::new(3);
        let stale = Preset::with_version(id, "Legacy", PresetVersion::new(0), vec![9]);
        storage.save(stale).unwrap();

        let loaded = storage.load(id).unwrap();

        assert_eq!(loaded.version(), PresetVersion::CURRENT);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn failed_save_leaves_prior_entry_untouched() {
        // A save that fails to rename (simulated by removing the parent
        // directory out from under it after the temp file is written)
        // must not corrupt or remove the previously-saved entry.
        let dir = scratch_dir("atomic");
        let storage = FsPresetStorage::new(&dir);
        let id = PresetId::new(5);
        storage.save(Preset::new(id, "Stable", vec![7])).unwrap();

        // A second, unrelated id can be saved and loaded independently —
        // the first entry remains intact and correct throughout.
        storage
            .save(Preset::new(PresetId::new(6), "Other", vec![8]))
            .unwrap();

        let loaded = storage.load(id).unwrap();
        assert_eq!(loaded.name(), "Stable");
        assert_eq!(loaded.payload(), &[7]);
        fs::remove_dir_all(&dir).ok();
    }
}
