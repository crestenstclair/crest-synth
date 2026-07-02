// path: src/preset/preset_storage.rs

//! Port: PresetStorage
//!
//! Defines the persistence boundary for presets: save, load, list, and
//! delete, independent of the underlying medium (filesystem, database,
//! cloud sync). All I/O implied by these operations happens off the
//! real-time audio thread — implementations MUST NOT be invoked from the
//! audio callback.

use std::fmt;

/// Uniquely identifies a stored preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PresetId(u64);

impl PresetId {
    /// Construct a `PresetId` from a raw identifier.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// The raw identifier value.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for PresetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The on-disk/schema version of a serialized preset.
///
/// Presets serialize with an explicit version so that older versions can
/// be migrated on load rather than silently misinterpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PresetVersion(u32);

impl PresetVersion {
    /// The current version written by this build.
    pub const CURRENT: PresetVersion = PresetVersion(1);

    /// Construct a version from a raw number (e.g. read from a file header).
    pub fn new(version: u32) -> Self {
        Self(version)
    }

    /// The raw version number.
    pub fn value(&self) -> u32 {
        self.0
    }

    /// Whether this version predates the current version and would need
    /// migration before use.
    pub fn needs_migration(&self) -> bool {
        self.0 < Self::CURRENT.0
    }
}

impl Default for PresetVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

/// Lightweight, listable summary of a preset — cheap to enumerate without
/// loading the full preset payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetMetadata {
    id: PresetId,
    name: String,
    version: PresetVersion,
}

impl PresetMetadata {
    pub fn new(id: PresetId, name: impl Into<String>, version: PresetVersion) -> Self {
        Self {
            id,
            name: name.into(),
            version,
        }
    }

    pub fn id(&self) -> PresetId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> PresetVersion {
        self.version
    }
}

/// A fully materialized preset: identity, name, explicit version, and its
/// serialized payload.
///
/// The payload is an opaque byte blob at this boundary; the storage port
/// does not interpret preset internals — only persists and retrieves them.
/// Migrating an older [`PresetVersion`] to [`PresetVersion::CURRENT`] is the
/// responsibility of a `PresetStorage` implementation's `load`, never of
/// callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    id: PresetId,
    name: String,
    version: PresetVersion,
    payload: Vec<u8>,
}

impl Preset {
    /// Construct a new preset at the current version.
    pub fn new(id: PresetId, name: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            id,
            name: name.into(),
            version: PresetVersion::CURRENT,
            payload,
        }
    }

    /// Construct a preset with an explicit (possibly older) version, e.g.
    /// while reconstructing one read from storage prior to migration.
    pub fn with_version(
        id: PresetId,
        name: impl Into<String>,
        version: PresetVersion,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            version,
            payload,
        }
    }

    pub fn id(&self) -> PresetId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> PresetVersion {
        self.version
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Metadata view of this preset, for listing without holding the full
    /// payload.
    pub fn metadata(&self) -> PresetMetadata {
        PresetMetadata::new(self.id, self.name.clone(), self.version)
    }

    /// Return a copy of this preset migrated to the current version.
    ///
    /// Implementations of [`PresetStorage::load`] must call this (or an
    /// equivalent migration path) before returning a preset whose stored
    /// version predates [`PresetVersion::CURRENT`].
    pub fn migrated_to_current(&self) -> Self {
        if !self.version.needs_migration() {
            return self.clone();
        }
        Self {
            id: self.id,
            name: self.name.clone(),
            version: PresetVersion::CURRENT,
            payload: self.payload.clone(),
        }
    }
}

/// Failure modes for preset storage operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// No preset exists for the given id.
    NotFound(PresetId),
    /// The underlying medium (filesystem, database, ...) reported a failure.
    Io(String),
    /// The stored bytes could not be decoded into a `Preset`.
    Serialization(String),
    /// The stored preset's version is newer than this build can understand.
    UnsupportedVersion(PresetVersion),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::NotFound(id) => write!(f, "preset {id} not found"),
            StorageError::Io(msg) => write!(f, "storage I/O error: {msg}"),
            StorageError::Serialization(msg) => write!(f, "preset serialization error: {msg}"),
            StorageError::UnsupportedVersion(version) => {
                write!(f, "preset version {} is not supported", version.value())
            }
        }
    }
}

impl std::error::Error for StorageError {}

/// Port: persistence boundary for presets.
///
/// A narrow interface (Interface Segregation) covering only the four
/// operations a caller needs to manage a preset library: save, load, list,
/// delete. Implementations (adapters) own the actual medium — filesystem,
/// database, cloud sync — and MUST perform any blocking I/O off the
/// real-time audio thread; nothing in this crate's audio callback may call
/// through this trait directly.
///
/// `save` and `delete` must be atomic from the caller's point of view: a
/// failed `save` must not leave a corrupted or partial entry behind, and a
/// failed `delete` must leave the existing entry untouched.
pub trait PresetStorage {
    /// Persist `preset`, replacing any existing entry with the same id.
    fn save(&self, preset: Preset) -> Result<(), StorageError>;

    /// Load the preset stored under `id`, migrating it to
    /// [`PresetVersion::CURRENT`] if it was written by an older version.
    fn load(&self, id: PresetId) -> Result<Preset, StorageError>;

    /// List metadata for every preset currently in storage, without
    /// loading full payloads.
    fn list(&self) -> Vec<PresetMetadata>;

    /// Remove the preset stored under `id`.
    fn delete(&self, id: PresetId) -> Result<(), StorageError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct InMemoryPresetStorage {
        entries: std::cell::RefCell<Vec<Preset>>,
    }

    impl InMemoryPresetStorage {
        fn new() -> Self {
            Self {
                entries: std::cell::RefCell::new(Vec::new()),
            }
        }
    }

    impl PresetStorage for InMemoryPresetStorage {
        fn save(&self, preset: Preset) -> Result<(), StorageError> {
            let mut entries = self.entries.borrow_mut();
            entries.retain(|p| p.id() != preset.id());
            entries.push(preset);
            Ok(())
        }

        fn load(&self, id: PresetId) -> Result<Preset, StorageError> {
            self.entries
                .borrow()
                .iter()
                .find(|p| p.id() == id)
                .map(|p| p.migrated_to_current())
                .ok_or(StorageError::NotFound(id))
        }

        fn list(&self) -> Vec<PresetMetadata> {
            self.entries.borrow().iter().map(Preset::metadata).collect()
        }

        fn delete(&self, id: PresetId) -> Result<(), StorageError> {
            let mut entries = self.entries.borrow_mut();
            let before = entries.len();
            entries.retain(|p| p.id() != id);
            if entries.len() == before {
                Err(StorageError::NotFound(id))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn save_then_load_round_trips() {
        let storage = InMemoryPresetStorage::new();
        let id = PresetId::new(1);
        let preset = Preset::new(id, "Warm Pad", vec![1, 2, 3]);
        storage.save(preset.clone()).unwrap();

        let loaded = storage.load(id).unwrap();
        assert_eq!(loaded.id(), id);
        assert_eq!(loaded.name(), "Warm Pad");
        assert_eq!(loaded.payload(), &[1, 2, 3]);
    }

    #[test]
    fn load_missing_preset_errors() {
        let storage = InMemoryPresetStorage::new();
        let result = storage.load(PresetId::new(42));
        assert_eq!(result, Err(StorageError::NotFound(PresetId::new(42))));
    }

    #[test]
    fn list_returns_metadata_for_all_saved_presets() {
        let storage = InMemoryPresetStorage::new();
        storage
            .save(Preset::new(PresetId::new(1), "A", vec![]))
            .unwrap();
        storage
            .save(Preset::new(PresetId::new(2), "B", vec![]))
            .unwrap();

        let mut names: Vec<_> = storage
            .list()
            .iter()
            .map(|m| m.name().to_string())
            .collect();
        names.sort();
        assert_eq!(names, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn delete_removes_entry_and_reports_missing_on_second_attempt() {
        let storage = InMemoryPresetStorage::new();
        let id = PresetId::new(7);
        storage.save(Preset::new(id, "Lead", vec![])).unwrap();

        storage.delete(id).unwrap();
        assert!(storage.load(id).is_err());
        assert_eq!(storage.delete(id), Err(StorageError::NotFound(id)));
    }

    #[test]
    fn save_overwrites_existing_entry_with_same_id() {
        let storage = InMemoryPresetStorage::new();
        let id = PresetId::new(9);
        storage.save(Preset::new(id, "Original", vec![0])).unwrap();
        storage.save(Preset::new(id, "Renamed", vec![1])).unwrap();

        let loaded = storage.load(id).unwrap();
        assert_eq!(loaded.name(), "Renamed");
        assert_eq!(loaded.payload(), &[1]);
        assert_eq!(storage.list().len(), 1);
    }

    #[test]
    fn older_version_is_migrated_on_load() {
        let storage = InMemoryPresetStorage::new();
        let id = PresetId::new(3);
        let stale = Preset::with_version(id, "Legacy", PresetVersion::new(0), vec![9]);
        storage.save(stale).unwrap();

        let loaded = storage.load(id).unwrap();
        assert_eq!(loaded.version(), PresetVersion::CURRENT);
    }
}
