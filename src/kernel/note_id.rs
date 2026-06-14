/// Unique identifier for a sounding note.
///
/// `NoteId` is a thin newtype around `u32`, used to track individual
/// note-on/note-off pairs so per-note expression (e.g. pitch bend, pressure)
/// can be addressed precisely. Ids are opaque — callers allocate them and the
/// kernel never inspects the value beyond equality.
///
/// # Examples
///
/// ```
/// use crest_synth::kernel::note_id::NoteId;
///
/// let id = NoteId::new(42);
/// assert_eq!(id.value(), 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct NoteId(u32);

impl NoteId {
    /// Create a new `NoteId` from the given raw value.
    ///
    /// There are no range constraints; every `u32` is a valid identifier.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the underlying `u32` value.
    pub fn value(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for NoteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoteId({})", self.0)
    }
}

impl From<u32> for NoteId {
    fn from(v: u32) -> Self {
        Self::new(v)
    }
}

impl From<NoteId> for u32 {
    fn from(id: NoteId) -> u32 {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_value() {
        let id = NoteId::new(7);
        assert_eq!(id.value(), 7);
    }

    #[test]
    fn zero_is_valid() {
        let id = NoteId::new(0);
        assert_eq!(id.value(), 0);
    }

    #[test]
    fn max_u32_is_valid() {
        let id = NoteId::new(u32::MAX);
        assert_eq!(id.value(), u32::MAX);
    }

    #[test]
    fn equality() {
        assert_eq!(NoteId::new(1), NoteId::new(1));
        assert_ne!(NoteId::new(1), NoteId::new(2));
    }

    #[test]
    fn ordering() {
        assert!(NoteId::new(1) < NoteId::new(2));
    }

    #[test]
    fn copy_semantics() {
        let a = NoteId::new(10);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn from_u32() {
        let id: NoteId = 99u32.into();
        assert_eq!(id.value(), 99);
    }

    #[test]
    fn into_u32() {
        let id = NoteId::new(55);
        let v: u32 = id.into();
        assert_eq!(v, 55);
    }

    #[test]
    fn display() {
        let id = NoteId::new(3);
        assert_eq!(format!("{id}"), "NoteId(3)");
    }

    #[test]
    fn default_is_zero() {
        let id = NoteId::default();
        assert_eq!(id.value(), 0);
    }

    #[test]
    fn hash_same_for_equal_values() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(NoteId::new(42));
        assert!(set.contains(&NoteId::new(42)));
        assert!(!set.contains(&NoteId::new(43)));
    }
}
