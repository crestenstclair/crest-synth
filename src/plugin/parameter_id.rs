// path: src/plugin/parameter_id.rs

//! A stable numeric identifier for a plugin parameter.
//!
//! Plugin parameters must have stable numeric IDs across versions so that a
//! host's automation lanes (recorded against a specific ID) keep working
//! after the plugin is updated. `ParameterId` is a thin newtype over `u32`
//! whose only job is to prevent accidental mixing with other numeric
//! quantities (indices, counts, etc.) at compile time.

/// Stable numeric ID for a plugin parameter, used by the host for automation.
///
/// The value itself carries no semantics beyond identity: two `ParameterId`s
/// are equal if and only if they refer to the same host-automatable
/// parameter. Assigning and retiring IDs is the responsibility of whichever
/// component maps parameters to their stable identity (out of scope for this
/// type); once assigned, an ID must never be reused for a different
/// parameter across plugin versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterId(u32);

impl ParameterId {
    /// Constructs a `ParameterId` from a raw numeric value.
    ///
    /// Every `u32` is a valid ID; there is no reserved or invalid range.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw numeric value of this ID.
    #[must_use]
    pub const fn value(self) -> u32 {
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

impl std::fmt::Display for ParameterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_value_round_trip() {
        let id = ParameterId::new(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn from_u32_round_trips_through_value() {
        let id: ParameterId = 7u32.into();
        assert_eq!(id.value(), 7);
    }

    #[test]
    fn into_u32_round_trips() {
        let id = ParameterId::new(99);
        let raw: u32 = id.into();
        assert_eq!(raw, 99);
    }

    #[test]
    fn equality_is_by_value() {
        assert_eq!(ParameterId::new(5), ParameterId::new(5));
        assert_ne!(ParameterId::new(5), ParameterId::new(6));
    }

    #[test]
    fn ordering_is_by_value() {
        assert!(ParameterId::new(1) < ParameterId::new(2));
    }

    #[test]
    fn display_shows_raw_value() {
        let id = ParameterId::new(123);
        assert_eq!(format!("{id}"), "123");
    }

    #[test]
    fn zero_is_a_valid_id() {
        let id = ParameterId::new(0);
        assert_eq!(id.value(), 0);
    }

    #[test]
    fn max_value_is_a_valid_id() {
        let id = ParameterId::new(u32::MAX);
        assert_eq!(id.value(), u32::MAX);
    }
}
