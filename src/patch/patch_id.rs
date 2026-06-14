// path: src/patch/patch_id.rs

/// Unique identifier for a patch.
///
/// A thin newtype over `u32` that prevents accidental mixing of raw integers
/// with patch identifiers in function signatures.
///
/// # Examples
///
/// ```
/// use crest_synth::patch::patch_id::PatchId;
///
/// let id = PatchId::new(42);
/// assert_eq!(id.get(), 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PatchId(u32);

impl PatchId {
    /// Creates a new `PatchId` from the given raw value.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the underlying raw identifier value.
    pub fn get(self) -> u32 {
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
        id.get()
    }
}

impl std::fmt::Display for PatchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PatchId({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_value() {
        let id = PatchId::new(7);
        assert_eq!(id.get(), 7);
    }

    #[test]
    fn zero_is_valid() {
        let id = PatchId::new(0);
        assert_eq!(id.get(), 0);
    }

    #[test]
    fn max_u32_is_valid() {
        let id = PatchId::new(u32::MAX);
        assert_eq!(id.get(), u32::MAX);
    }

    #[test]
    fn equality() {
        assert_eq!(PatchId::new(1), PatchId::new(1));
        assert_ne!(PatchId::new(1), PatchId::new(2));
    }

    #[test]
    fn ordering() {
        assert!(PatchId::new(1) < PatchId::new(2));
        assert!(PatchId::new(5) > PatchId::new(3));
    }

    #[test]
    fn copy_semantics() {
        let a = PatchId::new(10);
        let b = a; // Copy — a is still usable
        assert_eq!(a, b);
    }

    #[test]
    fn from_u32_into_patch_id() {
        let id: PatchId = 99u32.into();
        assert_eq!(id.get(), 99);
    }

    #[test]
    fn from_patch_id_into_u32() {
        let id = PatchId::new(55);
        let raw: u32 = id.into();
        assert_eq!(raw, 55);
    }

    #[test]
    fn display_format() {
        let id = PatchId::new(3);
        assert_eq!(format!("{}", id), "PatchId(3)");
    }

    #[test]
    fn hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PatchId::new(1));
        set.insert(PatchId::new(2));
        set.insert(PatchId::new(1)); // duplicate
        assert_eq!(set.len(), 2);
    }
}
