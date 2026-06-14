// path: src/sample_library/sample_set.rs
//
// SampleSet — aggregate root for the SampleLibrary context.
//
// Design notes
// ------------
//   • Zones are held in a `Vec<SampleZone>`; the invariant that no two zones
//     overlap in key+velocity space is enforced on insert.
//   • All sample PCM data is stored behind `Arc<[f32]>`.  The audio thread
//     reads frames through the shared reference without any allocation or
//     lock acquisition.
//   • When a `SampleSet` is retired (see `UnloadSampleSet`), callers hand the
//     Arc to a `DeferredDeallocator::RetireHandle` — ensuring `free()` never
//     runs on the audio thread.

use crate::sample_library::sample_format::SampleFormat;
use crate::sample_library::sample_set_id::SampleSetId;
use crate::sample_library::sample_zone::SampleZone;

/// Commands handled by the SampleLibrary aggregate.
#[derive(Debug, Clone)]
pub enum SampleSetCommand {
    /// Request to load a sample set from the given filesystem path.
    LoadSampleSet {
        /// Path to the sample-set directory or file on disk.
        path: String,
        /// Expected sample format.
        format: SampleFormat,
    },
    /// Request to unload a previously loaded sample set.
    UnloadSampleSet {
        /// Id of the set to remove.
        id: SampleSetId,
    },
}

/// Events emitted by the SampleLibrary aggregate.
#[derive(Debug, Clone, PartialEq)]
pub enum SampleSetEvent {
    /// Emitted after a sample set has been successfully loaded.
    SampleSetLoaded {
        id: SampleSetId,
        name: String,
        zone_count: u32,
    },
    /// Emitted after a sample set has been unloaded.
    SampleSetUnloaded { id: SampleSetId },
}

/// Error returned when a zone addition violates the non-overlapping invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneOverlapError {
    /// Index of the existing zone that the new zone overlaps.
    pub existing_zone_index: usize,
}

impl std::fmt::Display for ZoneOverlapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "new zone overlaps with existing zone at index {}",
            self.existing_zone_index
        )
    }
}

impl std::error::Error for ZoneOverlapError {}

/// A loaded collection of samples mapped to key/velocity zones.
///
/// # Invariants
///
/// - No two zones in the same set have overlapping key+velocity ranges.
/// - Sample data is held via `Arc<[f32]>`; the audio thread reads via shared
///   reference, never loading or freeing data itself.
/// - When unloaded, the caller must retire the Arcs through a
///   `DeferredDeallocator`, never dropping them on the audio thread.
#[derive(Debug, Clone)]
pub struct SampleSet {
    /// Unique identifier for this set.
    pub id: SampleSetId,
    /// Human-readable name (typically derived from the file/directory name).
    pub name: String,
    /// Audio format of the samples in this set.
    pub format: SampleFormat,
    /// Zones, each binding a range of keys+velocities to sample data.
    zones: Vec<SampleZone>,
}

impl SampleSet {
    /// Create a new, empty `SampleSet`.
    pub fn new(id: SampleSetId, name: String, format: SampleFormat) -> Self {
        Self {
            id,
            name,
            format,
            zones: Vec::new(),
        }
    }

    /// Add a zone to the set.
    ///
    /// Returns `Err(ZoneOverlapError)` if the new zone's key+velocity range
    /// overlaps with any existing zone, which would violate the aggregate
    /// invariant.
    pub fn add_zone(&mut self, zone: SampleZone) -> Result<(), ZoneOverlapError> {
        for (index, existing) in self.zones.iter().enumerate() {
            if existing.range().overlaps(zone.range()) {
                return Err(ZoneOverlapError {
                    existing_zone_index: index,
                });
            }
        }
        self.zones.push(zone);
        Ok(())
    }

    /// Iterate over all zones in the set.
    pub fn zones(&self) -> &[SampleZone] {
        &self.zones
    }

    /// Number of zones currently in the set.
    pub fn zone_count(&self) -> u32 {
        self.zones.len() as u32
    }

    /// Find the first zone that covers the given note and velocity.
    ///
    /// The audio thread calls this during voice allocation.  The operation is
    /// allocation-free and lock-free.
    #[inline]
    pub fn find_zone(
        &self,
        note: crate::kernel::note_number::NoteNumber,
        velocity: crate::kernel::velocity::Velocity,
    ) -> Option<&SampleZone> {
        self.zones
            .iter()
            .find(|z| z.range().contains(note, velocity))
    }
}

/// A registry of loaded sample sets, keyed by [`SampleSetId`].
///
/// `SampleLibrary` processes [`SampleSetCommand`]s and emits [`SampleSetEvent`]s.
/// It does **not** perform file I/O itself — callers are responsible for
/// decoding audio data and building `SampleZone`s before calling
/// [`SampleLibrary::apply_load`].
#[derive(Debug, Default)]
pub struct SampleLibrary {
    sets: Vec<SampleSet>,
    next_id: u32,
}

impl SampleLibrary {
    /// Create an empty library.
    pub fn new() -> Self {
        Self {
            sets: Vec::new(),
            next_id: 1,
        }
    }

    /// Allocate the next `SampleSetId` (monotonically increasing).
    pub fn next_id(&mut self) -> SampleSetId {
        let id = SampleSetId::new(self.next_id);
        self.next_id += 1;
        id
    }

    /// Apply a pre-decoded `SampleSet` to the library and emit the
    /// corresponding event.
    ///
    /// The caller has already loaded and decoded the audio from disk; this
    /// method only updates the in-memory state.
    pub fn apply_load(&mut self, set: SampleSet) -> SampleSetEvent {
        let event = SampleSetEvent::SampleSetLoaded {
            id: set.id,
            name: set.name.clone(),
            zone_count: set.zone_count(),
        };
        self.sets.push(set);
        event
    }

    /// Remove a sample set by id.
    ///
    /// Returns the removed `SampleSet` (so the caller can retire its Arcs
    /// through `DeferredDeallocator`), along with the corresponding event.
    /// Returns `None` if no set with the given id exists.
    pub fn apply_unload(&mut self, id: SampleSetId) -> Option<(SampleSet, SampleSetEvent)> {
        let pos = self.sets.iter().position(|s| s.id == id)?;
        let set = self.sets.remove(pos);
        let event = SampleSetEvent::SampleSetUnloaded { id };
        Some((set, event))
    }

    /// Iterate over all currently loaded sample sets.
    pub fn sets(&self) -> &[SampleSet] {
        &self.sets
    }

    /// Look up a sample set by id.
    pub fn get(&self, id: SampleSetId) -> Option<&SampleSet> {
        self.sets.iter().find(|s| s.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::kernel::note_number::NoteNumber;
    use crate::kernel::sample_rate::SampleRate;
    use crate::kernel::velocity::Velocity;
    use crate::sample_library::key_velocity_range::KeyVelocityRange;
    use crate::sample_library::sample_metadata::SampleMetadata;
    use crate::sample_library::sample_zone::SampleZone;

    fn nn(n: u8) -> NoteNumber {
        NoteNumber::try_new(n).unwrap()
    }

    fn vel(v: f64) -> Velocity {
        Velocity::try_new(v).unwrap()
    }

    fn make_zone(key_lo: u8, key_hi: u8, vel_lo: f64, vel_hi: f64) -> SampleZone {
        let metadata = SampleMetadata::try_new(
            1,
            1024,
            None,
            None,
            NoteNumber::try_new(key_lo).unwrap(),
            SampleRate::try_new(44100).unwrap(),
        )
        .unwrap();
        let range =
            KeyVelocityRange::try_new(nn(key_lo), nn(key_hi), vel(vel_lo), vel(vel_hi)).unwrap();
        let data: Arc<[f32]> = vec![0.0f32; 1024].into();
        SampleZone::new(metadata, range, data)
    }

    fn make_set(id: u32) -> SampleSet {
        SampleSet::new(SampleSetId::new(id), format!("Set{id}"), SampleFormat::Wav)
    }

    // --- SampleSet ---

    #[test]
    fn sample_set_new_is_empty() {
        let set = make_set(1);
        assert_eq!(set.zone_count(), 0);
        assert!(set.zones().is_empty());
    }

    #[test]
    fn sample_set_add_non_overlapping_zones_succeeds() {
        let mut set = make_set(1);
        // Two zones: different key ranges, no overlap
        let z1 = make_zone(0, 59, 0.0, 1.0);
        let z2 = make_zone(60, 127, 0.0, 1.0);
        assert!(set.add_zone(z1).is_ok());
        assert!(set.add_zone(z2).is_ok());
        assert_eq!(set.zone_count(), 2);
    }

    #[test]
    fn sample_set_add_zone_velocity_split_succeeds() {
        let mut set = make_set(1);
        // Same key range, different velocity splits
        let z1 = make_zone(60, 72, 0.0, 0.49);
        let z2 = make_zone(60, 72, 0.5, 1.0);
        assert!(set.add_zone(z1).is_ok());
        assert!(set.add_zone(z2).is_ok());
        assert_eq!(set.zone_count(), 2);
    }

    #[test]
    fn sample_set_add_overlapping_zones_rejected() {
        let mut set = make_set(1);
        let z1 = make_zone(60, 72, 0.0, 1.0);
        let z2 = make_zone(65, 80, 0.0, 1.0); // overlaps z1 in key space
        set.add_zone(z1).unwrap();
        let err = set.add_zone(z2).unwrap_err();
        assert_eq!(err.existing_zone_index, 0);
    }

    #[test]
    fn sample_set_find_zone_returns_matching() {
        let mut set = make_set(1);
        set.add_zone(make_zone(60, 72, 0.0, 1.0)).unwrap();
        set.add_zone(make_zone(73, 84, 0.0, 1.0)).unwrap();

        let found = set.find_zone(nn(65), vel(0.5));
        assert!(found.is_some());
        assert_eq!(found.unwrap().range().key_low(), nn(60));
    }

    #[test]
    fn sample_set_find_zone_returns_none_when_no_match() {
        let mut set = make_set(1);
        set.add_zone(make_zone(60, 72, 0.0, 1.0)).unwrap();

        let found = set.find_zone(nn(80), vel(0.5));
        assert!(found.is_none());
    }

    #[test]
    fn sample_set_zone_count_matches_adds() {
        let mut set = make_set(1);
        for i in 0u8..4 {
            set.add_zone(make_zone(i * 30, i * 30 + 29, 0.0, 1.0))
                .unwrap();
        }
        assert_eq!(set.zone_count(), 4);
    }

    // --- SampleLibrary ---

    #[test]
    fn sample_library_new_is_empty() {
        let lib = SampleLibrary::new();
        assert!(lib.sets().is_empty());
    }

    #[test]
    fn sample_library_next_id_is_monotonic() {
        let mut lib = SampleLibrary::new();
        let id1 = lib.next_id();
        let id2 = lib.next_id();
        assert!(id1 < id2);
    }

    #[test]
    fn sample_library_apply_load_emits_event() {
        let mut lib = SampleLibrary::new();
        let id = lib.next_id();
        let mut set = SampleSet::new(id, "Piano".to_string(), SampleFormat::Wav);
        set.add_zone(make_zone(60, 72, 0.0, 1.0)).unwrap();
        let event = lib.apply_load(set);
        assert_eq!(
            event,
            SampleSetEvent::SampleSetLoaded {
                id,
                name: "Piano".to_string(),
                zone_count: 1,
            }
        );
    }

    #[test]
    fn sample_library_apply_load_stores_set() {
        let mut lib = SampleLibrary::new();
        let id = lib.next_id();
        let set = make_set(id.as_u32());
        lib.apply_load(set);
        assert_eq!(lib.sets().len(), 1);
        assert_eq!(lib.sets()[0].id, id);
    }

    #[test]
    fn sample_library_apply_unload_removes_set() {
        let mut lib = SampleLibrary::new();
        let id = lib.next_id();
        let set = make_set(id.as_u32());
        lib.apply_load(set);

        let result = lib.apply_unload(id);
        assert!(result.is_some());
        let (removed, event) = result.unwrap();
        assert_eq!(removed.id, id);
        assert_eq!(event, SampleSetEvent::SampleSetUnloaded { id });
        assert!(lib.sets().is_empty());
    }

    #[test]
    fn sample_library_apply_unload_unknown_id_returns_none() {
        let mut lib = SampleLibrary::new();
        let missing_id = SampleSetId::new(999);
        assert!(lib.apply_unload(missing_id).is_none());
    }

    #[test]
    fn sample_library_get_finds_set_by_id() {
        let mut lib = SampleLibrary::new();
        let id = lib.next_id();
        let set = make_set(id.as_u32());
        lib.apply_load(set);

        let found = lib.get(id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, id);
    }

    #[test]
    fn sample_library_get_unknown_id_returns_none() {
        let lib = SampleLibrary::new();
        assert!(lib.get(SampleSetId::new(42)).is_none());
    }

    #[test]
    fn sample_set_zone_overlap_error_message_contains_index() {
        let err = ZoneOverlapError {
            existing_zone_index: 3,
        };
        assert!(err.to_string().contains("3"));
    }

    #[test]
    fn sample_library_multiple_sets_independent() {
        let mut lib = SampleLibrary::new();
        let id1 = lib.next_id();
        let id2 = lib.next_id();
        lib.apply_load(make_set(id1.as_u32()));
        lib.apply_load(make_set(id2.as_u32()));
        assert_eq!(lib.sets().len(), 2);

        lib.apply_unload(id1);
        assert_eq!(lib.sets().len(), 1);
        assert_eq!(lib.sets()[0].id, id2);
    }
}
