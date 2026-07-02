// path: src/sample/sample_store.rs
//
// SampleStore — port for persisting and retrieving `SampleSet` aggregates.
//
// Design notes
// ------------
//   • This is a pure interface (trait): callers depend on this abstraction,
//     never on a concrete storage mechanism (Dependency Inversion). Adapters
//     — an in-memory registry, a file-backed cache, a database-backed store
//     — implement `SampleStore` in the infrastructure layer and are
//     injected into whatever collaborator needs sample persistence.
//   • `get`/`put` return and accept owned `SampleSet` values and are free
//     to allocate and perform blocking I/O. This port is consumed only from
//     the control/UI thread — the audio callback must never call through
//     it directly. Sample data reaches the real-time thread only via the
//     ParameterBridge/EventRing seam, never through a direct `SampleStore`
//     call.

use crate::sample::sample_set::SampleSet;

/// A port for storing and retrieving [`SampleSet`] aggregates by an opaque
/// numeric identifier.
///
/// Implementations live behind this trait so callers depend on an
/// abstraction, never a concrete adapter (Dependency Inversion). This port
/// is intended for the control/UI thread only — it is not part of the
/// real-time audio boundary and implementations are free to block.
pub trait SampleStore {
    /// Look up a previously stored `SampleSet` by id.
    ///
    /// Returns `None` if no set has been stored under `id`.
    fn get(&self, id: u32) -> Option<SampleSet>;

    /// Store (or replace) the `SampleSet` for the given id.
    fn put(&mut self, id: u32, set: SampleSet);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::sample_set::InterpolationMode;
    use std::collections::HashMap;

    /// A minimal in-memory `SampleStore` used only to exercise the trait's
    /// contract in tests. Real adapters (filesystem, database, etc.) live in
    /// the infrastructure layer and implement the same trait.
    #[derive(Default)]
    struct InMemorySampleStore {
        entries: HashMap<u32, SampleSet>,
    }

    impl SampleStore for InMemorySampleStore {
        fn get(&self, id: u32) -> Option<SampleSet> {
            self.entries.get(&id).cloned()
        }

        fn put(&mut self, id: u32, set: SampleSet) {
            self.entries.insert(id, set);
        }
    }

    #[test]
    fn get_returns_none_when_absent() {
        let store = InMemorySampleStore::default();
        assert!(store.get(1).is_none());
    }

    #[test]
    fn put_then_get_returns_the_stored_set() {
        let mut store = InMemorySampleStore::default();
        store.put(1, SampleSet::new(InterpolationMode::Linear));

        let found = store.get(1).unwrap();
        assert_eq!(found.interpolation(), InterpolationMode::Linear);
    }

    #[test]
    fn put_overwrites_the_previous_value_for_the_same_id() {
        let mut store = InMemorySampleStore::default();
        store.put(1, SampleSet::new(InterpolationMode::None));
        store.put(1, SampleSet::new(InterpolationMode::Cubic));

        let found = store.get(1).unwrap();
        assert_eq!(found.interpolation(), InterpolationMode::Cubic);
    }

    #[test]
    fn get_for_unrelated_id_returns_none() {
        let mut store = InMemorySampleStore::default();
        store.put(1, SampleSet::new(InterpolationMode::Linear));

        assert!(store.get(2).is_none());
    }

    #[test]
    fn entries_for_distinct_ids_are_independent() {
        let mut store = InMemorySampleStore::default();
        store.put(1, SampleSet::new(InterpolationMode::Linear));
        store.put(2, SampleSet::new(InterpolationMode::Cubic));

        assert_eq!(
            store.get(1).unwrap().interpolation(),
            InterpolationMode::Linear
        );
        assert_eq!(
            store.get(2).unwrap().interpolation(),
            InterpolationMode::Cubic
        );
    }
}
