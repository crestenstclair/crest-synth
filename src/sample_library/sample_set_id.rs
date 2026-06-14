// path: src/sample_library/sample_set_id.rs

/// Unique identifier for a loaded sample set.
///
/// `SampleSetId` is a transparent newtype over `u32`. Two sample sets loaded
/// into the engine are distinguished solely by their id; the audio thread
/// never inspects the numeric value — it only compares ids for equality.
///
/// # Examples
///
/// ```
/// use crest_synth::sample_library::sample_set_id::SampleSetId;
///
/// let id = SampleSetId::new(42);
/// assert_eq!(id.as_u32(), 42);
///
/// let same = SampleSetId::new(42);
/// assert_eq!(id, same);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SampleSetId(u32);

impl SampleSetId {
    /// Creates a new `SampleSetId` from the given raw value.
    #[inline]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the underlying `u32` value.
    #[inline]
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl From<u32> for SampleSetId {
    #[inline]
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl From<SampleSetId> for u32 {
    #[inline]
    fn from(id: SampleSetId) -> Self {
        id.as_u32()
    }
}

impl std::fmt::Display for SampleSetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SampleSetId({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::SampleSetId;

    #[test]
    fn round_trip_from_u32() {
        let id = SampleSetId::from(7u32);
        assert_eq!(u32::from(id), 7);
    }

    #[test]
    fn equality_and_ordering() {
        let a = SampleSetId::new(1);
        let b = SampleSetId::new(2);
        assert!(a < b);
        assert_ne!(a, b);
        assert_eq!(a, SampleSetId::new(1));
    }

    #[test]
    fn display() {
        let id = SampleSetId::new(99);
        assert_eq!(id.to_string(), "SampleSetId(99)");
    }

    #[test]
    fn copy_semantics() {
        let id = SampleSetId::new(5);
        let copy = id;
        assert_eq!(id, copy);
    }

    #[test]
    fn hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SampleSetId::new(10));
        set.insert(SampleSetId::new(10));
        assert_eq!(set.len(), 1);
    }
}
