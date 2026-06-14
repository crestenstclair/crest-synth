// path: src/effects/effect_slot.rs

use crate::effects::effect_processor::{EffectParams, EffectType};

/// A single slot in an `EffectChain`.
///
/// Each slot holds one `EffectProcessor` configured with a specific effect
/// type and parameters, plus an individual bypass flag. When `bypass` is
/// `true` audio passes through the slot unchanged.
///
/// `EffectSlot` stores only the configuration needed to (re-)create an
/// `EffectProcessor`; the processor itself lives inside the chain so that
/// audio-thread state allocation is controlled at one site.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectSlot {
    /// Which algorithm this slot runs.
    pub effect_type: EffectType,
    /// Current parameters for the algorithm.
    pub params: EffectParams,
    /// When `true` this slot is muted — audio passes through unchanged.
    pub bypass: bool,
}

impl EffectSlot {
    /// Creates a new `EffectSlot` with default parameters and bypass off.
    pub fn new(effect_type: EffectType) -> Self {
        Self {
            effect_type,
            params: EffectParams {
                effect_type,
                ..EffectParams::default()
            },
            bypass: false,
        }
    }

    /// Creates a new `EffectSlot` with explicit parameters and bypass state.
    pub fn with_params(effect_type: EffectType, params: EffectParams, bypass: bool) -> Self {
        Self {
            effect_type,
            params,
            bypass,
        }
    }

    /// Returns `true` if this slot is currently active (not bypassed).
    pub fn is_active(&self) -> bool {
        !self.bypass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_slot_is_not_bypassed() {
        let slot = EffectSlot::new(EffectType::Gain);
        assert!(!slot.bypass);
        assert!(slot.is_active());
    }

    #[test]
    fn new_slot_has_correct_effect_type() {
        let slot = EffectSlot::new(EffectType::LowPassFilter);
        assert_eq!(slot.effect_type, EffectType::LowPassFilter);
    }

    #[test]
    fn new_slot_params_match_effect_type() {
        let slot = EffectSlot::new(EffectType::Gain);
        assert_eq!(slot.params.effect_type, EffectType::Gain);
    }

    #[test]
    fn bypassed_slot_is_not_active() {
        let slot = EffectSlot::with_params(
            EffectType::Gain,
            EffectParams {
                effect_type: EffectType::Gain,
                gain: 2.0,
                ..EffectParams::default()
            },
            true,
        );
        assert!(slot.bypass);
        assert!(!slot.is_active());
    }

    #[test]
    fn copy_semantics() {
        let a = EffectSlot::new(EffectType::Delay);
        let b = a; // Copy — a still usable
        assert_eq!(a, b);
    }
}
