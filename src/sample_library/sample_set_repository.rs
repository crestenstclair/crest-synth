// path: src/sample_library/sample_set_repository.rs
//
// SampleSetRepository — in-memory repository for the SampleSet aggregate.
//
// Design notes
// ------------
//   • The repository is purely non-realtime (UI/command thread only).
//     The audio thread accesses sample data only through Arc<[f32]> references
//     cloned out of each SampleZone — never through the repository itself.
//   • `save` replaces any existing entry with the same id, supporting both
//     insert and update semantics.
//   • All operations are O(n) over the number of loaded sets. For typical
//     sample library sizes (tens to low hundreds of sets) this is fine.

use crate::sample_library::sample_set::SampleSet;
use crate::sample_library::sample_set_id::SampleSetId;

/// A trait abstracting persistent access to [`SampleSet`] aggregates.
///
/// Implementations may be in-memory (for tests or simple use) or backed by
/// an external store. The contract mirrors the DDD repository pattern.
pub trait SampleSetRepository {
    /// Find a sample set by its unique identifier.
    ///
    /// Returns `Some(set)` if a set with the given id exists, `None`
    /// otherwise.
    fn find_by_id(&self, id: SampleSetId) -> Option<SampleSet>;

    /// Return all currently stored sample sets.
    fn list_all(&self) -> Vec<SampleSet>;

    /// Persist a sample set.
    ///
    /// If a set with the same id already exists it is replaced; otherwise
    /// the set is inserted.
    fn save(&mut self, set: SampleSet);
}

/// An in-memory [`SampleSetRepository`] backed by a `Vec`.
///
/// This implementation is intended for the single-process synthesiser where
/// all state lives in RAM. It is not thread-safe; callers must ensure
/// exclusive access (e.g. only accessed from the UI/command thread).
///
/// # Example
///
/// ```
/// use crest_synth::sample_library::sample_format::SampleFormat;
/// use crest_synth::sample_library::sample_set::SampleSet;
/// use crest_synth::sample_library::sample_set_id::SampleSetId;
/// use crest_synth::sample_library::sample_set_repository::{
///     InMemorySampleSetRepository, SampleSetRepository,
/// };
///
/// let mut repo = InMemorySampleSetRepository::new();
/// let id = SampleSetId::new(1);
/// let set = SampleSet::new(id, "Piano".to_string(), SampleFormat::Wav);
/// repo.save(set);
///
/// assert!(repo.find_by_id(id).is_some());
/// assert_eq!(repo.list_all().len(), 1);
/// ```
#[derive(Debug, Default)]
pub struct InMemorySampleSetRepository {
    sets: Vec<SampleSet>,
}

impl InMemorySampleSetRepository {
    /// Create an empty repository.
    pub fn new() -> Self {
        Self { sets: Vec::new() }
    }
}

impl SampleSetRepository for InMemorySampleSetRepository {
    fn find_by_id(&self, id: SampleSetId) -> Option<SampleSet> {
        self.sets.iter().find(|s| s.id == id).cloned()
    }

    fn list_all(&self) -> Vec<SampleSet> {
        self.sets.clone()
    }

    fn save(&mut self, set: SampleSet) {
        if let Some(pos) = self.sets.iter().position(|s| s.id == set.id) {
            self.sets[pos] = set;
        } else {
            self.sets.push(set);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_library::sample_format::SampleFormat;
    use crate::sample_library::sample_set::SampleSet;
    use crate::sample_library::sample_set_id::SampleSetId;

    fn make_set(id: u32) -> SampleSet {
        SampleSet::new(SampleSetId::new(id), format!("Set{id}"), SampleFormat::Wav)
    }

    // --- find_by_id ---

    #[test]
    fn sample_set_repository_find_by_id_returns_none_when_empty() {
        let repo = InMemorySampleSetRepository::new();
        assert!(repo.find_by_id(SampleSetId::new(1)).is_none());
    }

    #[test]
    fn sample_set_repository_find_by_id_returns_saved_set() {
        let mut repo = InMemorySampleSetRepository::new();
        let id = SampleSetId::new(1);
        repo.save(make_set(1));
        let found = repo.find_by_id(id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, id);
    }

    #[test]
    fn sample_set_repository_find_by_id_returns_none_for_unknown_id() {
        let mut repo = InMemorySampleSetRepository::new();
        repo.save(make_set(1));
        assert!(repo.find_by_id(SampleSetId::new(99)).is_none());
    }

    #[test]
    fn sample_set_repository_find_by_id_distinguishes_multiple_sets() {
        let mut repo = InMemorySampleSetRepository::new();
        repo.save(make_set(1));
        repo.save(make_set(2));
        repo.save(make_set(3));

        let found = repo.find_by_id(SampleSetId::new(2));
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Set2");
    }

    // --- list_all ---

    #[test]
    fn sample_set_repository_list_all_empty() {
        let repo = InMemorySampleSetRepository::new();
        assert!(repo.list_all().is_empty());
    }

    #[test]
    fn sample_set_repository_list_all_returns_all_sets() {
        let mut repo = InMemorySampleSetRepository::new();
        repo.save(make_set(1));
        repo.save(make_set(2));
        repo.save(make_set(3));

        let all = repo.list_all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn sample_set_repository_list_all_returns_clone_not_reference() {
        let mut repo = InMemorySampleSetRepository::new();
        repo.save(make_set(1));

        let mut all = repo.list_all();
        // Mutating the returned clone must not affect the repository.
        all.clear();
        assert_eq!(repo.list_all().len(), 1);
    }

    // --- save (insert) ---

    #[test]
    fn sample_set_repository_save_inserts_new_set() {
        let mut repo = InMemorySampleSetRepository::new();
        repo.save(make_set(1));
        assert_eq!(repo.list_all().len(), 1);
    }

    #[test]
    fn sample_set_repository_save_multiple_distinct_sets() {
        let mut repo = InMemorySampleSetRepository::new();
        repo.save(make_set(10));
        repo.save(make_set(20));
        assert_eq!(repo.list_all().len(), 2);
    }

    // --- save (update / replace) ---

    #[test]
    fn sample_set_repository_save_replaces_existing_set() {
        let mut repo = InMemorySampleSetRepository::new();
        let id = SampleSetId::new(7);

        let original = SampleSet::new(id, "Original".to_string(), SampleFormat::Wav);
        repo.save(original);

        let updated = SampleSet::new(id, "Updated".to_string(), SampleFormat::Wav);
        repo.save(updated);

        // Still only one entry.
        let all = repo.list_all();
        assert_eq!(all.len(), 1);

        // Entry reflects the updated name.
        let found = repo.find_by_id(id).unwrap();
        assert_eq!(found.name, "Updated");
    }

    #[test]
    fn sample_set_repository_save_replace_does_not_grow_list() {
        let mut repo = InMemorySampleSetRepository::new();
        let id = SampleSetId::new(1);

        for _ in 0..5 {
            repo.save(SampleSet::new(id, "Same".to_string(), SampleFormat::Wav));
        }
        assert_eq!(repo.list_all().len(), 1);
    }

    // --- combined ---

    #[test]
    fn sample_set_repository_save_and_find_round_trip() {
        let mut repo = InMemorySampleSetRepository::new();
        let id = SampleSetId::new(42);
        let name = "Strings".to_string();

        repo.save(SampleSet::new(id, name.clone(), SampleFormat::Wav));

        let retrieved = repo.find_by_id(id).expect("should be present");
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.name, name);
    }

    #[test]
    fn sample_set_repository_independent_sets_do_not_collide() {
        let mut repo = InMemorySampleSetRepository::new();
        repo.save(make_set(1));
        repo.save(make_set(2));

        // Overwrite set 1 only.
        repo.save(SampleSet::new(
            SampleSetId::new(1),
            "Replaced".to_string(),
            SampleFormat::Wav,
        ));

        assert_eq!(repo.list_all().len(), 2);
        assert_eq!(
            repo.find_by_id(SampleSetId::new(1)).unwrap().name,
            "Replaced"
        );
        assert_eq!(repo.find_by_id(SampleSetId::new(2)).unwrap().name, "Set2");
    }
}
