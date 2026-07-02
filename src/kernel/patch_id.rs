// path: src/kernel/patch_id.rs

//! Identifies a Patch.
//!
//! `PatchId` is a small, `Copy` newtype wrapping a `u32`. It carries no
//! behavior beyond identity — comparison, hashing, and construction — so it
//! is safe to pass across the real-time/non-real-time thread boundary (e.g.
//! inside a `MidiEvent` payload or a channel-mapping table entry) without
//! allocation.

use std::fmt;

/// Identifies a Patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PatchId(u32);

impl PatchId {
    /// Constructs a `PatchId` from a raw `u32` value.
    ///
    /// ```
    /// use crest_synth::kernel::patch_id::PatchId;
    ///
    /// let id = PatchId::new(7);
    /// assert_eq!(id.value(), 7);
    /// ```
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the underlying raw value.
    pub fn value(self) -> u32 {
        self.0
    }
}

impl From<u32> for PatchId {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl From<PatchId> for u32 {
    fn from(id: PatchId) -> Self {
        id.0
    }
}

impl fmt::Display for PatchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_the_provided_value() {
        let id = PatchId::new(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn from_u32_round_trips() {
        let id: PatchId = 5u32.into();
        assert_eq!(id.value(), 5);

        let raw: u32 = id.into();
        assert_eq!(raw, 5);
    }

    #[test]
    fn equal_values_are_equal_and_hash_consistently() {
        use std::collections::HashSet;

        let a = PatchId::new(3);
        let b = PatchId::new(3);
        let c = PatchId::new(4);

        assert_eq!(a, b);
        assert_ne!(a, c);

        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
        assert!(!set.contains(&c));
    }

    #[test]
    fn ordering_follows_underlying_value() {
        assert!(PatchId::new(1) < PatchId::new(2));
    }

    #[test]
    fn display_renders_the_raw_value() {
        let id = PatchId::new(99);
        assert_eq!(id.to_string(), "99");
    }

    #[test]
    fn is_copy_and_cheap_to_pass_across_thread_boundary() {
        let id = PatchId::new(1);
        let copied = id;
        // Both usable: PatchId is Copy, so this is not a move.
        assert_eq!(id, copied);
    }
}
