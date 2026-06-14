// path: src/effects/effect_chain_id.rs

/// Unique identifier for an effect chain.
///
/// A thin newtype wrapper around `u32` that is `Copy`, heap-free, and safe
/// to use on the audio thread.
///
/// # Examples
///
/// ```
/// use crest_synth::effects::effect_chain_id::EffectChainId;
///
/// let id = EffectChainId::new(42);
/// assert_eq!(id.value(), 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EffectChainId(u32);

impl EffectChainId {
    /// Creates a new `EffectChainId` from a raw `u32`.
    #[inline]
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the underlying `u32` value.
    #[inline]
    pub fn value(self) -> u32 {
        self.0
    }
}

impl From<u32> for EffectChainId {
    #[inline]
    fn from(id: u32) -> Self {
        Self::new(id)
    }
}

impl From<EffectChainId> for u32 {
    #[inline]
    fn from(id: EffectChainId) -> Self {
        id.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_value() {
        let id = EffectChainId::new(7);
        assert_eq!(id.value(), 7);
    }

    #[test]
    fn from_u32_roundtrip() {
        let id = EffectChainId::from(99u32);
        let raw: u32 = id.into();
        assert_eq!(raw, 99);
    }

    #[test]
    fn equality_and_ordering() {
        let a = EffectChainId::new(1);
        let b = EffectChainId::new(2);
        assert!(a < b);
        assert_eq!(a, EffectChainId::new(1));
    }

    #[test]
    fn copy_semantics() {
        let a = EffectChainId::new(5);
        let b = a; // copy, not move
        assert_eq!(a, b);
    }

    #[test]
    fn zero_is_valid() {
        let id = EffectChainId::new(0);
        assert_eq!(id.value(), 0);
    }

    #[test]
    fn max_value_is_valid() {
        let id = EffectChainId::new(u32::MAX);
        assert_eq!(id.value(), u32::MAX);
    }
}
