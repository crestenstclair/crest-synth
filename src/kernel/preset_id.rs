// path: src/kernel/preset_id.rs

//! `PresetId` — identifies a Preset.
//!
//! A small newtype wrapper around `u32`. Presets are referenced by this
//! identifier throughout the preset library (banks, sessions, patch
//! assignments) so that storage, serialization, and lookup all share one
//! unambiguous key type instead of a bare `u32` passed around by convention.

use std::fmt;

/// Identifies a Preset.
///
/// `PresetId` is an opaque, copyable value. It carries no ordering semantics
/// beyond equality/hashing for use as a map key; construction is always
/// explicit via [`PresetId::new`] so a raw `u32` can never be mistaken for a
/// `PresetId` at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PresetId(u32);

impl PresetId {
    /// Constructs a `PresetId` from its underlying `u32` value.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the underlying `u32` value.
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl fmt::Display for PresetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for PresetId {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl From<PresetId> for u32 {
    fn from(id: PresetId) -> Self {
        id.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_value_round_trip() {
        let id = PresetId::new(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn from_u32_matches_new() {
        let via_from: PresetId = 7u32.into();
        let via_new = PresetId::new(7);
        assert_eq!(via_from, via_new);
    }

    #[test]
    fn into_u32_round_trips() {
        let id = PresetId::new(99);
        let raw: u32 = id.into();
        assert_eq!(raw, 99);
    }

    #[test]
    fn equality_and_hash_are_value_based() {
        use std::collections::HashSet;

        let a = PresetId::new(5);
        let b = PresetId::new(5);
        let c = PresetId::new(6);

        assert_eq!(a, b);
        assert_ne!(a, c);

        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
        assert!(!set.contains(&c));
    }

    #[test]
    fn ordering_matches_underlying_value() {
        let low = PresetId::new(1);
        let high = PresetId::new(2);
        assert!(low < high);
    }

    #[test]
    fn display_renders_underlying_value() {
        let id = PresetId::new(123);
        assert_eq!(id.to_string(), "123");
    }

    #[test]
    fn zero_is_a_valid_preset_id() {
        let id = PresetId::new(0);
        assert_eq!(id.value(), 0);
    }

    #[test]
    fn max_value_round_trips() {
        let id = PresetId::new(u32::MAX);
        assert_eq!(id.value(), u32::MAX);
    }
}
