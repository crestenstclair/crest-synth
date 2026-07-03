// path: src/preset/preset_browser.rs

//! Application service: `PresetBrowser`.
//!
//! Orchestrates the Preset context's aggregate and ports to support the
//! preset-library workflow: search/filter, preview with a test note, load a
//! preset into a patch, save a patch as a preset, import an SF2 payload, and
//! export a bank.
//!
//! This service never touches the real-time audio thread. Previewing a note
//! is delegated through the narrow [`NotePreviewer`] port (Dependency
//! Inversion) so whatever eventually plays the note — e.g. something that
//! pushes onto the `EventRing` — stays swappable and testable in isolation.
//!
//! `aggregate.Preset.Bank`, `port.Preset.PresetCodec`, and
//! `port.Preset.PresetStorage` were each committed independently and each
//! defines its own local `PresetId` newtype (all wrapping `u64`, none of
//! them the same Rust type). This service is the seam that reconciles them:
//! it converts by raw value at the boundary rather than assuming the types
//! are interchangeable.

use std::fmt;

use crate::preset::bank::{Bank, BankCommand, BankError, PresetId as BankPresetId};
use crate::preset::preset_codec::{CodecError, Preset as CodecPreset, PresetCodec};
use crate::preset::preset_storage::{
    Preset as StoredPreset, PresetId as StoragePresetId, PresetMetadata, PresetStorage,
    StorageError,
};

/// A single MIDI-like test note used to preview a preset.
///
/// Deliberately a plain local value type (not a kernel domain type) so this
/// application-layer module never has to guess at another module's
/// constructor API; it only needs to hand a note description to a
/// [`NotePreviewer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TestNote {
    note_number: u8,
    velocity: u8,
    duration_ms: u32,
}

impl TestNote {
    /// Constructs a test note. `note_number` and `velocity` are clamped into
    /// the valid MIDI 7-bit range (0..=127) so callers can't hand a
    /// `NotePreviewer` an out-of-range value.
    pub fn new(note_number: u8, velocity: u8, duration_ms: u32) -> Self {
        Self {
            note_number: note_number.min(127),
            velocity: velocity.min(127),
            duration_ms,
        }
    }

    pub fn note_number(&self) -> u8 {
        self.note_number
    }

    pub fn velocity(&self) -> u8 {
        self.velocity
    }

    pub fn duration_ms(&self) -> u32 {
        self.duration_ms
    }
}

impl Default for TestNote {
    /// Middle C, moderate velocity, half a second — a sensible default
    /// preview note when the caller doesn't care.
    fn default() -> Self {
        Self::new(60, 100, 500)
    }
}

/// Narrow port (Interface Segregation): the one thing `PresetBrowser` needs
/// in order to preview a preset is something that can play a test note
/// against it. Real implementations live outside this module (e.g. in
/// `shell`) and are responsible for getting the note across the real-time
/// boundary correctly; this trait says nothing about how.
pub trait NotePreviewer {
    fn play_test_note(&self, preset: &StoredPreset, note: TestNote) -> Result<(), PreviewError>;
}

/// Failure modes for previewing a preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewError {
    /// No output device/voice was available to play the note.
    DeviceUnavailable,
    /// Some other previewer-specific failure, carried as a message.
    Other(String),
}

impl fmt::Display for PreviewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreviewError::DeviceUnavailable => write!(f, "no preview device available"),
            PreviewError::Other(msg) => write!(f, "preview failed: {msg}"),
        }
    }
}

impl std::error::Error for PreviewError {}

/// Search/filter criteria over stored preset metadata.
///
/// Only a name substring filter today (all `PresetMetadata` exposes besides
/// id/version); built with a small fluent API so new criteria can be added
/// later without breaking existing callers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchCriteria {
    name_contains: Option<String>,
}

impl SearchCriteria {
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict matches to presets whose name contains `needle`
    /// (case-insensitive). An empty needle matches everything.
    pub fn with_name_contains(mut self, needle: impl Into<String>) -> Self {
        let needle = needle.into();
        self.name_contains = if needle.is_empty() {
            None
        } else {
            Some(needle)
        };
        self
    }

    pub fn matches(&self, metadata: &PresetMetadata) -> bool {
        match &self.name_contains {
            Some(needle) => metadata
                .name()
                .to_lowercase()
                .contains(&needle.to_lowercase()),
            None => true,
        }
    }
}

/// Everything that can go wrong running a `PresetBrowser` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetBrowserError {
    Storage(StorageError),
    Codec(CodecError),
    Bank(BankError),
    Preview(PreviewError),
}

impl fmt::Display for PresetBrowserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PresetBrowserError::Storage(e) => write!(f, "{e}"),
            PresetBrowserError::Codec(e) => write!(f, "{e}"),
            PresetBrowserError::Bank(e) => write!(f, "{e}"),
            PresetBrowserError::Preview(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PresetBrowserError {}

impl From<StorageError> for PresetBrowserError {
    fn from(e: StorageError) -> Self {
        PresetBrowserError::Storage(e)
    }
}

impl From<CodecError> for PresetBrowserError {
    fn from(e: CodecError) -> Self {
        PresetBrowserError::Codec(e)
    }
}

impl From<BankError> for PresetBrowserError {
    fn from(e: BankError) -> Self {
        PresetBrowserError::Bank(e)
    }
}

impl From<PreviewError> for PresetBrowserError {
    fn from(e: PreviewError) -> Self {
        PresetBrowserError::Preview(e)
    }
}

/// Converts a storage-side preset id into the bank aggregate's own
/// `PresetId` type by raw value. The two are independently-defined newtypes
/// over the same `u64` identity space, not the same Rust type.
fn to_bank_preset_id(id: StoragePresetId) -> BankPresetId {
    BankPresetId::new(id.value())
}

/// Application service: search/filter the preset library, preview presets
/// with a test note, load a preset for use in a patch, save a patch back as
/// a preset, import an SF2-derived payload, and export a bank's presets to
/// bytes.
///
/// Dependencies are injected via the constructor (Dependency Inversion):
/// `PresetBrowser` depends only on the `PresetStorage` and `PresetCodec`
/// port traits and the local `NotePreviewer` port, never on a concrete
/// adapter. `Bank` is not held by this service — it is data the caller
/// passes to the specific operations (`import_sf2`, `export_bank`) that
/// need to know which bank is in play, since search/preview/load/save don't
/// need one at all (Interface Segregation).
pub struct PresetBrowser<S, C, P>
where
    S: PresetStorage,
    C: PresetCodec,
    P: NotePreviewer,
{
    storage: S,
    codec: C,
    previewer: P,
}

impl<S, C, P> PresetBrowser<S, C, P>
where
    S: PresetStorage,
    C: PresetCodec,
    P: NotePreviewer,
{
    pub fn new(storage: S, codec: C, previewer: P) -> Self {
        Self {
            storage,
            codec,
            previewer,
        }
    }

    /// Search and filter the preset library.
    pub fn search(&self, criteria: &SearchCriteria) -> Vec<PresetMetadata> {
        self.storage
            .list()
            .into_iter()
            .filter(|metadata| criteria.matches(metadata))
            .collect()
    }

    /// Load a preset and play a test note through the injected previewer.
    pub fn preview(&self, id: StoragePresetId, note: TestNote) -> Result<(), PresetBrowserError> {
        let preset = self.storage.load(id)?;
        self.previewer.play_test_note(&preset, note)?;
        Ok(())
    }

    /// Load a preset by id, e.g. to apply into a patch.
    pub fn load_preset(&self, id: StoragePresetId) -> Result<StoredPreset, PresetBrowserError> {
        Ok(self.storage.load(id)?)
    }

    /// Save a patch's serialized payload as a preset under `id`, replacing
    /// any existing preset with that id.
    pub fn save_preset(
        &self,
        id: StoragePresetId,
        name: impl Into<String>,
        patch_payload: Vec<u8>,
    ) -> Result<(), PresetBrowserError> {
        let preset = StoredPreset::new(id, name, patch_payload);
        self.storage.save(preset)?;
        Ok(())
    }

    /// Import an SF2-derived (or any codec-decodable) byte payload as a new
    /// preset under `id`, registering it in `bank`.
    ///
    /// Order of operations honors the "no partial loads/imports" invariant:
    /// the bank command is validated (but not applied) before anything is
    /// persisted, so a bank-side rejection (e.g. duplicate preset) never
    /// results in a preset being written to storage that the bank refuses
    /// to register. The bank is only mutated after the storage write
    /// succeeds, using the already-validated event.
    pub fn import_sf2(
        &self,
        bank: &mut Bank,
        id: StoragePresetId,
        raw_bytes: &[u8],
    ) -> Result<(), PresetBrowserError> {
        let decoded: CodecPreset = self.codec.decode(raw_bytes)?;

        let bank_id = to_bank_preset_id(id);
        let command = BankCommand::AddPreset { preset: bank_id };
        let event = bank.handle(&command)?;

        let stored = StoredPreset::new(
            id,
            decoded.name().to_string(),
            decoded.patch_payload().to_vec(),
        );
        self.storage.save(stored)?;

        bank.apply(&event);
        Ok(())
    }

    /// Export every preset currently registered in `bank` to a single byte
    /// stream: a sequence of `[len: u32 LE][codec-encoded preset bytes]`
    /// entries, one per preset, in bank order.
    ///
    /// This performs no mutation of `bank` or `storage` — if any load or
    /// encode fails partway through, the partially-built buffer is simply
    /// discarded along with the error; nothing external is left half-written.
    pub fn export_bank(&self, bank: &Bank) -> Result<Vec<u8>, PresetBrowserError> {
        let mut out = Vec::new();
        for bank_preset_id in bank.presets() {
            let storage_id = StoragePresetId::new(bank_preset_id.value());
            let stored = self.storage.load(storage_id)?;
            let codec_preset =
                CodecPreset::new(stored.name().to_string(), stored.payload().to_vec());
            let bytes = self.codec.encode(codec_preset)?;
            out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(&bytes);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::bank::BankId;
    use std::cell::RefCell;

    struct InMemoryPresetStorage {
        entries: RefCell<Vec<StoredPreset>>,
    }

    impl InMemoryPresetStorage {
        fn new() -> Self {
            Self {
                entries: RefCell::new(Vec::new()),
            }
        }

        fn seeded(presets: Vec<StoredPreset>) -> Self {
            Self {
                entries: RefCell::new(presets),
            }
        }
    }

    impl PresetStorage for InMemoryPresetStorage {
        fn save(&self, preset: StoredPreset) -> Result<(), StorageError> {
            let mut entries = self.entries.borrow_mut();
            entries.retain(|p| p.id() != preset.id());
            entries.push(preset);
            Ok(())
        }

        fn load(&self, id: StoragePresetId) -> Result<StoredPreset, StorageError> {
            self.entries
                .borrow()
                .iter()
                .find(|p| p.id() == id)
                .cloned()
                .ok_or(StorageError::NotFound(id))
        }

        fn list(&self) -> Vec<PresetMetadata> {
            self.entries
                .borrow()
                .iter()
                .map(StoredPreset::metadata)
                .collect()
        }

        fn delete(&self, id: StoragePresetId) -> Result<(), StorageError> {
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

    /// Codec that round-trips the codec's local `Preset` shape without any
    /// physical byte layout, sufficient for exercising `PresetBrowser`.
    struct EchoCodec;

    impl PresetCodec for EchoCodec {
        fn decode(&self, data: &[u8]) -> Result<CodecPreset, CodecError> {
            if data.is_empty() {
                return Err(CodecError::Malformed("empty payload".to_string()));
            }
            Ok(CodecPreset::new("Imported", data.to_vec()))
        }

        fn encode(&self, preset: CodecPreset) -> Result<Vec<u8>, CodecError> {
            Ok(preset.patch_payload().to_vec())
        }
    }

    struct RecordingPreviewer {
        calls: RefCell<Vec<(String, TestNote)>>,
        fail: bool,
    }

    impl RecordingPreviewer {
        fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                fail: true,
            }
        }
    }

    impl NotePreviewer for RecordingPreviewer {
        fn play_test_note(
            &self,
            preset: &StoredPreset,
            note: TestNote,
        ) -> Result<(), PreviewError> {
            if self.fail {
                return Err(PreviewError::DeviceUnavailable);
            }
            self.calls
                .borrow_mut()
                .push((preset.name().to_string(), note));
            Ok(())
        }
    }

    fn browser(
        storage: InMemoryPresetStorage,
    ) -> PresetBrowser<InMemoryPresetStorage, EchoCodec, RecordingPreviewer> {
        PresetBrowser::new(storage, EchoCodec, RecordingPreviewer::new())
    }

    #[test]
    fn search_with_empty_criteria_returns_everything() {
        let storage = InMemoryPresetStorage::seeded(vec![
            StoredPreset::new(StoragePresetId::new(1), "Warm Pad", vec![]),
            StoredPreset::new(StoragePresetId::new(2), "Bright Lead", vec![]),
        ]);
        let b = browser(storage);

        let results = b.search(&SearchCriteria::new());
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_filters_by_name_substring_case_insensitively() {
        let storage = InMemoryPresetStorage::seeded(vec![
            StoredPreset::new(StoragePresetId::new(1), "Warm Pad", vec![]),
            StoredPreset::new(StoragePresetId::new(2), "Bright Lead", vec![]),
        ]);
        let b = browser(storage);

        let results = b.search(&SearchCriteria::new().with_name_contains("warm"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name(), "Warm Pad");
    }

    #[test]
    fn preview_loads_preset_and_invokes_previewer() {
        let storage = InMemoryPresetStorage::seeded(vec![StoredPreset::new(
            StoragePresetId::new(1),
            "Warm Pad",
            vec![1, 2, 3],
        )]);
        let b = PresetBrowser::new(storage, EchoCodec, RecordingPreviewer::new());

        let result = b.preview(StoragePresetId::new(1), TestNote::default());
        assert!(result.is_ok());
    }

    #[test]
    fn preview_of_missing_preset_errors_without_invoking_previewer() {
        let storage = InMemoryPresetStorage::new();
        let b = browser(storage);

        let result = b.preview(StoragePresetId::new(99), TestNote::default());
        assert_eq!(
            result,
            Err(PresetBrowserError::Storage(StorageError::NotFound(
                StoragePresetId::new(99)
            )))
        );
    }

    #[test]
    fn preview_propagates_previewer_failure() {
        let storage = InMemoryPresetStorage::seeded(vec![StoredPreset::new(
            StoragePresetId::new(1),
            "Warm Pad",
            vec![],
        )]);
        let b = PresetBrowser::new(storage, EchoCodec, RecordingPreviewer::failing());

        let result = b.preview(StoragePresetId::new(1), TestNote::default());
        assert_eq!(
            result,
            Err(PresetBrowserError::Preview(PreviewError::DeviceUnavailable))
        );
    }

    #[test]
    fn load_preset_returns_the_stored_preset() {
        let storage = InMemoryPresetStorage::seeded(vec![StoredPreset::new(
            StoragePresetId::new(1),
            "Warm Pad",
            vec![9],
        )]);
        let b = browser(storage);

        let loaded = b.load_preset(StoragePresetId::new(1)).unwrap();
        assert_eq!(loaded.name(), "Warm Pad");
        assert_eq!(loaded.payload(), &[9]);
    }

    #[test]
    fn save_preset_persists_a_new_entry() {
        let storage = InMemoryPresetStorage::new();
        let b = browser(storage);

        b.save_preset(StoragePresetId::new(5), "New Patch", vec![1, 2])
            .unwrap();

        let loaded = b.load_preset(StoragePresetId::new(5)).unwrap();
        assert_eq!(loaded.name(), "New Patch");
        assert_eq!(loaded.payload(), &[1, 2]);
    }

    #[test]
    fn import_sf2_saves_preset_and_registers_it_in_the_bank() {
        let storage = InMemoryPresetStorage::new();
        let b = browser(storage);
        let mut bank = Bank::new(BankId::new(1));

        b.import_sf2(&mut bank, StoragePresetId::new(7), &[1, 2, 3])
            .unwrap();

        let loaded = b.load_preset(StoragePresetId::new(7)).unwrap();
        assert_eq!(loaded.name(), "Imported");
        assert!(bank.contains(BankPresetId::new(7)));
    }

    #[test]
    fn import_sf2_rejects_duplicate_without_touching_storage() {
        let storage = InMemoryPresetStorage::new();
        let b = browser(storage);
        let mut bank = Bank::new(BankId::new(1));
        bank.add_preset(BankPresetId::new(7)).unwrap();

        let result = b.import_sf2(&mut bank, StoragePresetId::new(7), &[1, 2, 3]);

        assert!(matches!(
            result,
            Err(PresetBrowserError::Bank(BankError::DuplicatePreset(_)))
        ));
        // The bank rejected the command before storage was ever touched.
        assert!(b.load_preset(StoragePresetId::new(7)).is_err());
    }

    #[test]
    fn import_sf2_propagates_codec_decode_failure() {
        let storage = InMemoryPresetStorage::new();
        let b = browser(storage);
        let mut bank = Bank::new(BankId::new(1));

        let result = b.import_sf2(&mut bank, StoragePresetId::new(1), &[]);

        assert!(matches!(result, Err(PresetBrowserError::Codec(_))));
        assert!(!bank.contains(BankPresetId::new(1)));
    }

    #[test]
    fn export_bank_encodes_every_registered_preset_in_order() {
        let storage = InMemoryPresetStorage::seeded(vec![
            StoredPreset::new(StoragePresetId::new(1), "A", vec![10, 20]),
            StoredPreset::new(StoragePresetId::new(2), "B", vec![30]),
        ]);
        let b = browser(storage);
        let mut bank = Bank::new(BankId::new(1));
        bank.add_preset(BankPresetId::new(1)).unwrap();
        bank.add_preset(BankPresetId::new(2)).unwrap();

        let bytes = b.export_bank(&bank).unwrap();

        // entry 1: len=2 (LE u32) + [10, 20]
        // entry 2: len=1 (LE u32) + [30]
        let mut expected = Vec::new();
        expected.extend_from_slice(&2u32.to_le_bytes());
        expected.extend_from_slice(&[10, 20]);
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(&[30]);
        assert_eq!(bytes, expected);
    }

    #[test]
    fn export_bank_of_empty_bank_yields_empty_bytes() {
        let storage = InMemoryPresetStorage::new();
        let b = browser(storage);
        let bank = Bank::new(BankId::new(1));

        let bytes = b.export_bank(&bank).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn export_bank_errors_if_a_registered_preset_is_missing_from_storage() {
        let storage = InMemoryPresetStorage::new();
        let b = browser(storage);
        let mut bank = Bank::new(BankId::new(1));
        bank.add_preset(BankPresetId::new(42)).unwrap();

        let result = b.export_bank(&bank);
        assert!(matches!(result, Err(PresetBrowserError::Storage(_))));
    }
}
