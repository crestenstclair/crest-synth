// path: src/preset/session_manager.rs

//! Application service: SessionManager
//!
//! Save, load, list, and delete full sessions. A session's state (tempo,
//! time signature) is serialized to bytes and persisted as a [`Preset`]
//! through the [`PresetStorage`] port. Restoring a loaded session goes
//! through the `Session` aggregate's stage-then-restore protocol so a
//! failed load — a storage error, or a corrupted/invalid payload — can
//! never leave the aggregate's prior state partially replaced.

use std::error::Error;
use std::fmt;

use crate::preset::preset_storage::{Preset, PresetId, PresetMetadata, PresetStorage, StorageError};
use crate::preset::session::{
    Session, SessionCommand, SessionError, SessionSnapshot, Tempo, TimeSignature,
};

/// Errors that can arise while saving, loading, listing, or deleting a
/// session through [`SessionManager`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionManagerError {
    /// The underlying [`PresetStorage`] reported a failure.
    Storage(StorageError),
    /// The `Session` aggregate rejected staged state or the restore
    /// command.
    Session(SessionError),
    /// The stored bytes could not be decoded into valid session state.
    Codec(String),
}

impl fmt::Display for SessionManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionManagerError::Storage(err) => write!(f, "session storage error: {err}"),
            SessionManagerError::Session(err) => write!(f, "session error: {err}"),
            SessionManagerError::Codec(msg) => write!(f, "session codec error: {msg}"),
        }
    }
}

impl Error for SessionManagerError {}

impl From<StorageError> for SessionManagerError {
    fn from(err: StorageError) -> Self {
        SessionManagerError::Storage(err)
    }
}

impl From<SessionError> for SessionManagerError {
    fn from(err: SessionError) -> Self {
        SessionManagerError::Session(err)
    }
}

/// Narrow abstraction (Interface Segregation) over encoding/decoding a
/// session's state to/from the opaque byte payload persisted by
/// [`PresetStorage`]. Injected into [`SessionManager`] via the constructor
/// so tests can substitute a deterministic or failing codec without
/// depending on a particular wire format.
pub trait SessionCodec {
    /// Encode `tempo` and `time_signature` into an opaque byte payload.
    fn encode(&self, tempo: Tempo, time_signature: TimeSignature) -> Vec<u8>;

    /// Decode a previously-encoded payload back into `(Tempo,
    /// TimeSignature)`, validating both fields as if freshly constructed.
    fn decode(&self, bytes: &[u8]) -> Result<(Tempo, TimeSignature), SessionManagerError>;
}

/// Default [`SessionCodec`]: a tiny fixed-layout binary format — 4 bytes
/// little-endian tempo (BPM as `f32`), 1 byte numerator, 1 byte
/// denominator.
#[derive(Debug, Clone, Copy, Default)]
pub struct BinarySessionCodec;

impl SessionCodec for BinarySessionCodec {
    fn encode(&self, tempo: Tempo, time_signature: TimeSignature) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(6);
        bytes.extend_from_slice(&tempo.bpm().to_le_bytes());
        bytes.push(time_signature.numerator());
        bytes.push(time_signature.denominator());
        bytes
    }

    fn decode(&self, bytes: &[u8]) -> Result<(Tempo, TimeSignature), SessionManagerError> {
        if bytes.len() != 6 {
            return Err(SessionManagerError::Codec(format!(
                "expected a 6-byte session payload, got {} bytes",
                bytes.len()
            )));
        }

        let mut bpm_bytes = [0u8; 4];
        bpm_bytes.copy_from_slice(&bytes[0..4]);
        let bpm = f32::from_le_bytes(bpm_bytes);
        let numerator = bytes[4];
        let denominator = bytes[5];

        let tempo = Tempo::try_new(bpm)?;
        let time_signature = TimeSignature::try_new(numerator, denominator)?;
        Ok((tempo, time_signature))
    }
}

/// Application service: save, load, list, and delete full sessions.
///
/// Depends only on abstractions (Dependency Inversion): the
/// [`PresetStorage`] port for persistence and a [`SessionCodec`] for the
/// wire format, both injected via the constructor so tests can substitute
/// in-memory or failing implementations without touching this type.
pub struct SessionManager<S: PresetStorage, C: SessionCodec> {
    storage: S,
    codec: C,
}

impl<S: PresetStorage> SessionManager<S, BinarySessionCodec> {
    /// Convenience constructor using the default [`BinarySessionCodec`].
    pub fn new(storage: S) -> Self {
        Self::with_codec(storage, BinarySessionCodec)
    }
}

impl<S: PresetStorage, C: SessionCodec> SessionManager<S, C> {
    /// Full constructor accepting both the storage port and a codec —
    /// used by tests that need to inject a fake or spy codec.
    pub fn with_codec(storage: S, codec: C) -> Self {
        Self { storage, codec }
    }

    /// Persist `session`'s current state under `id`, replacing any prior
    /// session stored at that id.
    pub fn save(
        &self,
        id: PresetId,
        name: impl Into<String>,
        session: &Session,
    ) -> Result<(), SessionManagerError> {
        let payload = self.codec.encode(session.tempo(), session.time_signature());
        let preset = Preset::new(id, name, payload);
        self.storage.save(preset)?;
        Ok(())
    }

    /// Load the session stored under `id` and atomically restore it into
    /// `session`.
    ///
    /// The loaded bytes are fetched and decoded into a fully-validated
    /// [`SessionSnapshot`] *before* anything is staged onto `session` — a
    /// storage failure or a corrupted/invalid payload returns an error and
    /// leaves `session` byte-for-byte untouched. Only once a valid
    /// snapshot exists is it staged and [`SessionCommand::Restore`]
    /// issued; the `Session` aggregate itself guarantees that command
    /// replaces all state as a single atomic swap.
    pub fn load(&self, id: PresetId, session: &mut Session) -> Result<(), SessionManagerError> {
        let preset = self.storage.load(id)?;
        let (tempo, time_signature) = self.codec.decode(preset.payload())?;
        let snapshot = SessionSnapshot::new(tempo, time_signature);

        session.stage_restore(snapshot);
        session.handle(SessionCommand::Restore)?;
        Ok(())
    }

    /// List metadata for every session currently in storage, without
    /// loading full payloads.
    pub fn list(&self) -> Vec<PresetMetadata> {
        self.storage.list()
    }

    /// Remove the session stored under `id`.
    pub fn delete(&self, id: PresetId) -> Result<(), SessionManagerError> {
        self.storage.delete(id)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeStorage {
        entries: RefCell<Vec<Preset>>,
    }

    impl PresetStorage for FakeStorage {
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
                .cloned()
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

    fn valid_tempo() -> Tempo {
        Tempo::try_new(120.0).expect("120 bpm is within range")
    }

    fn other_tempo() -> Tempo {
        Tempo::try_new(90.0).expect("90 bpm is within range")
    }

    fn valid_time_signature() -> TimeSignature {
        TimeSignature::try_new(4, 4).expect("4/4 is a valid time signature")
    }

    fn other_time_signature() -> TimeSignature {
        TimeSignature::try_new(6, 8).expect("6/8 is a valid time signature")
    }

    #[test]
    fn save_then_load_round_trips_session_state() {
        let manager = SessionManager::new(FakeStorage::default());
        let id = PresetId::new(1);
        let source = Session::new(other_tempo(), other_time_signature());

        manager.save(id, "My Session", &source).expect("save succeeds");

        let mut target = Session::new(valid_tempo(), valid_time_signature());
        manager.load(id, &mut target).expect("load succeeds");

        assert_eq!(target.tempo(), other_tempo());
        assert_eq!(target.time_signature(), other_time_signature());
    }

    #[test]
    fn load_missing_session_errors_and_leaves_state_untouched() {
        let manager = SessionManager::new(FakeStorage::default());
        let mut session = Session::new(valid_tempo(), valid_time_signature());
        let before = session.clone();

        let result = manager.load(PresetId::new(99), &mut session);

        assert!(result.is_err());
        assert_eq!(session, before);
    }

    #[test]
    fn load_with_corrupted_payload_errors_and_leaves_state_untouched() {
        let storage = FakeStorage::default();
        let id = PresetId::new(2);
        storage
            .save(Preset::new(id, "Bad", vec![1, 2, 3]))
            .expect("fake storage save cannot fail");
        let manager = SessionManager::new(storage);

        let mut session = Session::new(valid_tempo(), valid_time_signature());
        let before = session.clone();

        let result = manager.load(id, &mut session);

        assert!(result.is_err());
        assert_eq!(session, before);
    }

    #[test]
    fn list_returns_metadata_for_all_saved_sessions() {
        let manager = SessionManager::new(FakeStorage::default());
        manager
            .save(PresetId::new(1), "A", &Session::new(valid_tempo(), valid_time_signature()))
            .expect("save succeeds");
        manager
            .save(PresetId::new(2), "B", &Session::new(valid_tempo(), valid_time_signature()))
            .expect("save succeeds");

        let mut names: Vec<_> = manager.list().iter().map(|m| m.name().to_string()).collect();
        names.sort();
        assert_eq!(names, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn delete_removes_session_and_reports_missing_on_second_attempt() {
        let manager = SessionManager::new(FakeStorage::default());
        let id = PresetId::new(3);
        manager
            .save(id, "ToDelete", &Session::new(valid_tempo(), valid_time_signature()))
            .expect("save succeeds");

        manager.delete(id).expect("delete succeeds");

        let mut session = Session::new(valid_tempo(), valid_time_signature());
        assert!(manager.load(id, &mut session).is_err());
        assert!(manager.delete(id).is_err());
    }

    #[test]
    fn save_overwrites_existing_session_with_same_id() {
        let manager = SessionManager::new(FakeStorage::default());
        let id = PresetId::new(4);
        manager
            .save(id, "Original", &Session::new(valid_tempo(), valid_time_signature()))
            .expect("save succeeds");
        manager
            .save(id, "Renamed", &Session::new(other_tempo(), other_time_signature()))
            .expect("save succeeds");

        let mut session = Session::new(valid_tempo(), valid_time_signature());
        manager.load(id, &mut session).expect("load succeeds");

        assert_eq!(session.tempo(), other_tempo());
        let names: Vec<_> = manager.list().iter().map(|m| m.name().to_string()).collect();
        assert_eq!(names, vec!["Renamed".to_string()]);
    }

    #[test]
    fn binary_codec_round_trips() {
        let codec = BinarySessionCodec;
        let tempo = other_tempo();
        let time_signature = other_time_signature();

        let bytes = codec.encode(tempo, time_signature);
        let (decoded_tempo, decoded_time_signature) =
            codec.decode(&bytes).expect("decode succeeds");

        assert_eq!(decoded_tempo, tempo);
        assert_eq!(decoded_time_signature, time_signature);
    }
}
