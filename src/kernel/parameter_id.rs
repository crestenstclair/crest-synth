// path: src/kernel/parameter_id.rs

//! Identifies a plugin parameter.
//!
//! `ParameterId` is a small, `Copy` newtype wrapping a `u32`. Plugin
//! parameters have stable numeric IDs across versions so that host
//! automation lanes (recorded against a numeric ID in a DAW project) keep
//! working after the plugin is updated. It carries no behavior beyond
//! identity, so it is cheap to copy across the real-time/non-real-time
//! thread boundary (e.g. inside a `SetParameter` command or a
//! `ParameterChanged` event payload).

use std::fmt;

/// Identifies a plugin parameter with a stable numeric ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParameterId(u32);

impl ParameterId {
    /// Constructs a `ParameterId` from a raw `u32` value.
    ///
    /// ```
    /// use crest_synth::kernel::parameter_id::ParameterId;
    ///
    /// let id = ParameterId::new(7);
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

impl From<u32> for ParameterId {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl From<ParameterId> for u32 {
    fn from(id: ParameterId) -> Self {
        id.0
    }
}

impl fmt::Display for ParameterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_the_provided_value() {
        let id = ParameterId::new(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn from_u32_round_trips() {
        let id: ParameterId = 5u32.into();
        assert_eq!(id.value(), 5);

        let raw: u32 = id.into();
        assert_eq!(raw, 5);
    }

    #[test]
    fn equal_values_are_equal_and_hash_consistently() {
        use std::collections::HashSet;

        let a = ParameterId::new(3);
        let b = ParameterId::new(3);
        let c = ParameterId::new(4);

        assert_eq!(a, b);
        assert_ne!(a, c);

        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
        assert!(!set.contains(&c));
    }

    #[test]
    fn ordering_follows_underlying_value() {
        assert!(ParameterId::new(1) < ParameterId::new(2));
    }

    #[test]
    fn display_renders_the_raw_value() {
        let id = ParameterId::new(99);
        assert_eq!(id.to_string(), "99");
    }

    #[test]
    fn is_copy_and_cheap_to_pass_across_thread_boundary() {
        let id = ParameterId::new(1);
        let copied = id;
        // Both usable: ParameterId is Copy, so this is not a move.
        assert_eq!(id, copied);
    }
}
