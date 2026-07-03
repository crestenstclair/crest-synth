// path: src/loop/state_snapshot.rs

//! Deterministic serialization of `AppState`.
//!
//! `StateSnapshot` guarantees that serializing the same `AppState` twice
//! yields byte-identical output: field order is stable (fields are declared
//! in the same order every time, and `serde_json` preserves struct field
//! declaration order), map ordering is stable (all keyed collections use
//! `BTreeMap`, which serializes keys in sorted order regardless of
//! insertion order), and no wall-clock timestamp or other non-deterministic
//! value is ever included in the state.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A minimal, deterministic representation of the application's
/// control-plane state.
///
/// This is intentionally self-contained: every field is either a plain
/// value or a `BTreeMap`, so equality and serialization are both
/// independent of the order values were inserted in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppState {
    /// Named numeric parameters (e.g. mixer channel values), keyed by a
    /// stable string identifier. `BTreeMap` guarantees keys serialize in
    /// sorted order no matter the order they were inserted.
    pub parameters: BTreeMap<String, i64>,

    /// Named flags (e.g. mute/solo/bypass state), keyed by a stable string
    /// identifier.
    pub flags: BTreeMap<String, bool>,

    /// A monotonic, caller-controlled revision counter for the state. This
    /// is not a wall-clock timestamp: it only changes when the caller
    /// explicitly bumps it, so it never introduces non-determinism into a
    /// snapshot of otherwise-identical state.
    pub revision: u64,
}

impl AppState {
    /// Builds an empty state at revision zero.
    pub fn new() -> Self {
        Self {
            parameters: BTreeMap::new(),
            flags: BTreeMap::new(),
            revision: 0,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// An error produced while decoding a `StateSnapshot` back into an
/// `AppState`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSnapshotError(String);

impl std::fmt::Display for StateSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "state snapshot error: {}", self.0)
    }
}

impl std::error::Error for StateSnapshotError {}

/// A complete, deterministic serialization of `AppState`.
///
/// Two invariants hold for every `StateSnapshot`:
///
/// - Serializing the same `AppState` twice yields byte-identical output
///   (`StateSnapshot::capture` is a pure function of its input).
/// - A snapshot round-trips: `StateSnapshot::capture(&state).restore()`
///   yields a value equal to the original `state`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSnapshot {
    bytes: Vec<u8>,
}

impl StateSnapshot {
    /// Captures a deterministic byte representation of `state`.
    ///
    /// Uses `serde_json` in its default (non-pretty) mode, which emits
    /// struct fields in declaration order and never reorders map keys
    /// beyond what the underlying collection already guarantees — hence
    /// the requirement that `AppState`'s keyed collections are `BTreeMap`,
    /// not `HashMap`.
    pub fn capture(state: &AppState) -> Self {
        let bytes = serde_json::to_vec(state)
            .expect("AppState serialization is infallible by construction");
        Self { bytes }
    }

    /// Returns the serialized bytes of this snapshot.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Restores the `AppState` this snapshot was captured from.
    ///
    /// This never partially populates a state: on error, no `AppState` is
    /// produced at all, so callers cannot observe a half-restored value.
    pub fn restore(&self) -> Result<AppState, StateSnapshotError> {
        serde_json::from_slice(&self.bytes).map_err(|e| StateSnapshotError(e.to_string()))
    }
}

#[cfg(test)]
mod state_snapshot_tests {
    use super::*;

    fn sample_state() -> AppState {
        let mut parameters = BTreeMap::new();
        parameters.insert("channel.1.volume".to_string(), 100);
        parameters.insert("channel.2.volume".to_string(), 80);
        parameters.insert("master.volume".to_string(), 90);

        let mut flags = BTreeMap::new();
        flags.insert("channel.1.mute".to_string(), false);
        flags.insert("channel.2.solo".to_string(), true);

        AppState {
            parameters,
            flags,
            revision: 42,
        }
    }

    #[test]
    fn capture_is_deterministic_across_repeated_calls() {
        let state = sample_state();

        let first = StateSnapshot::capture(&state);
        let second = StateSnapshot::capture(&state);

        assert_eq!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn capture_is_deterministic_regardless_of_map_insertion_order() {
        let mut a_parameters = BTreeMap::new();
        a_parameters.insert("a".to_string(), 1);
        a_parameters.insert("b".to_string(), 2);
        let a_state = AppState {
            parameters: a_parameters,
            flags: BTreeMap::new(),
            revision: 1,
        };

        let mut b_parameters = BTreeMap::new();
        b_parameters.insert("b".to_string(), 2);
        b_parameters.insert("a".to_string(), 1);
        let b_state = AppState {
            parameters: b_parameters,
            flags: BTreeMap::new(),
            revision: 1,
        };

        assert_eq!(a_state, b_state);
        assert_eq!(
            StateSnapshot::capture(&a_state).as_bytes(),
            StateSnapshot::capture(&b_state).as_bytes()
        );
    }

    #[test]
    fn round_trip_restores_the_original_state() {
        let state = sample_state();

        let snapshot = StateSnapshot::capture(&state);
        let restored = snapshot.restore().expect("restore should succeed");

        assert_eq!(restored, state);
    }

    #[test]
    fn round_trip_preserves_empty_state() {
        let state = AppState::new();

        let snapshot = StateSnapshot::capture(&state);
        let restored = snapshot.restore().expect("restore should succeed");

        assert_eq!(restored, state);
    }

    #[test]
    fn restore_on_corrupt_bytes_is_an_error_not_a_partial_state() {
        let corrupt = StateSnapshot {
            bytes: b"not valid json".to_vec(),
        };

        assert!(corrupt.restore().is_err());
    }
}
