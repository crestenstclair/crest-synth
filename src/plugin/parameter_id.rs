// path: src/plugin/parameter_id.rs

/// Stable numeric ID for a plugin parameter, used by the host for automation.
///
/// `ParameterId` is a thin newtype around `u32`. The ID must remain stable
/// across plugin versions so that DAW projects with saved automation continue
/// to work after an update.
///
/// # Examples
///
/// ```
/// use crest_synth::plugin::parameter_id::ParameterId;
///
/// let id = ParameterId::new(1);
/// assert_eq!(id.get(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParameterId(u32);

impl ParameterId {
    /// Create a new `ParameterId` from the given raw value.
    ///
    /// There are no range constraints; every `u32` is a valid identifier.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the underlying raw `u32` value.
    pub fn get(self) -> u32 {
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
        id.get()
    }
}

impl Default for ParameterId {
    /// Returns `ParameterId(0)` as the default.
    fn default() -> Self {
        Self(0)
    }
}

impl std::fmt::Display for ParameterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParameterId({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_value() {
        let id = ParameterId::new(7);
        assert_eq!(id.get(), 7);
    }

    #[test]
    fn zero_is_valid() {
        let id = ParameterId::new(0);
        assert_eq!(id.get(), 0);
    }

    #[test]
    fn max_u32_is_valid() {
        let id = ParameterId::new(u32::MAX);
        assert_eq!(id.get(), u32::MAX);
    }

    #[test]
    fn equality() {
        assert_eq!(ParameterId::new(1), ParameterId::new(1));
        assert_ne!(ParameterId::new(1), ParameterId::new(2));
    }

    #[test]
    fn ordering() {
        assert!(ParameterId::new(1) < ParameterId::new(2));
        assert!(ParameterId::new(5) > ParameterId::new(3));
    }

    #[test]
    fn copy_semantics() {
        let a = ParameterId::new(10);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn from_u32() {
        let id: ParameterId = 99u32.into();
        assert_eq!(id.get(), 99);
    }

    #[test]
    fn into_u32() {
        let id = ParameterId::new(55);
        let raw: u32 = id.into();
        assert_eq!(raw, 55);
    }

    #[test]
    fn default_is_zero() {
        let id = ParameterId::default();
        assert_eq!(id.get(), 0);
    }

    #[test]
    fn display_format() {
        let id = ParameterId::new(3);
        assert_eq!(format!("{id}"), "ParameterId(3)");
    }

    #[test]
    fn hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ParameterId::new(1));
        set.insert(ParameterId::new(2));
        set.insert(ParameterId::new(1)); // duplicate
        assert_eq!(set.len(), 2);
    }
}
