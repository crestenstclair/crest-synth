// path: src/modulation/mod_destination.rs

//! `ModDestination` — a modulation destination: oscillator pitch or pulse
//! width, filter cutoff or resonance, amp level, pan, LFO rate or depth, an
//! effect parameter (slot + param index), or a send level (bus).

use std::fmt;

/// Index of an effect slot within an insert chain (0-based).
///
/// A pure newtype: it names a position, it does not itself enforce a
/// chain's maximum slot count (that belongs to the effect chain that owns
/// the slots).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EffectSlotIndex(u8);

impl EffectSlotIndex {
    pub fn new(index: u8) -> Self {
        Self(index)
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

impl fmt::Display for EffectSlotIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Index of a parameter within an effect's parameter set (0-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EffectParamIndex(u8);

impl EffectParamIndex {
    pub fn new(index: u8) -> Self {
        Self(index)
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

impl fmt::Display for EffectParamIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Index of a send/aux bus (0-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SendBusIndex(u8);

impl SendBusIndex {
    pub fn new(index: u8) -> Self {
        Self(index)
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

impl fmt::Display for SendBusIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A modulation destination: the parameter a mod-matrix row's modulation
/// is routed to.
///
/// `ModDestination` is a pure value — it names a target parameter. It does
/// not hold a modulation value and carries no behavior for applying
/// modulation; the mod matrix resolves a `ModDestination` to a concrete
/// parameter write that crosses the real-time boundary via the
/// `ParameterBridge` or the `EventRing`, never by mutating engine state
/// directly from this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModDestination {
    /// Oscillator pitch, as an offset from the oscillator's base pitch.
    OscillatorPitch,
    /// Oscillator pulse width (duty cycle), for waveforms that support it.
    OscillatorPulseWidth,
    /// Filter cutoff frequency.
    FilterCutoff,
    /// Filter resonance.
    FilterResonance,
    /// Voice amplitude level.
    AmpLevel,
    /// Voice pan position.
    Pan,
    /// LFO rate — modulating one LFO's speed from another source.
    LfoRate,
    /// LFO depth — modulating one LFO's output amplitude from another source.
    LfoDepth,
    /// A parameter of an effect in an insert chain, addressed by slot and
    /// parameter index within that effect.
    EffectParam {
        slot: EffectSlotIndex,
        param: EffectParamIndex,
    },
    /// The send level to a particular send/aux bus.
    SendLevel { bus: SendBusIndex },
}

impl fmt::Display for ModDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModDestination::OscillatorPitch => write!(f, "oscillator pitch"),
            ModDestination::OscillatorPulseWidth => write!(f, "oscillator pulse width"),
            ModDestination::FilterCutoff => write!(f, "filter cutoff"),
            ModDestination::FilterResonance => write!(f, "filter resonance"),
            ModDestination::AmpLevel => write!(f, "amp level"),
            ModDestination::Pan => write!(f, "pan"),
            ModDestination::LfoRate => write!(f, "LFO rate"),
            ModDestination::LfoDepth => write!(f, "LFO depth"),
            ModDestination::EffectParam { slot, param } => {
                write!(f, "effect param (slot {slot}, param {param})")
            }
            ModDestination::SendLevel { bus } => write!(f, "send level (bus {bus})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_variants_are_equal_to_themselves() {
        assert_eq!(ModDestination::OscillatorPitch, ModDestination::OscillatorPitch);
        assert_eq!(ModDestination::FilterCutoff, ModDestination::FilterCutoff);
        assert_ne!(ModDestination::FilterCutoff, ModDestination::FilterResonance);
    }

    #[test]
    fn effect_param_destinations_compare_by_slot_and_param() {
        let a = ModDestination::EffectParam {
            slot: EffectSlotIndex::new(1),
            param: EffectParamIndex::new(2),
        };
        let b = ModDestination::EffectParam {
            slot: EffectSlotIndex::new(1),
            param: EffectParamIndex::new(2),
        };
        let c = ModDestination::EffectParam {
            slot: EffectSlotIndex::new(1),
            param: EffectParamIndex::new(3),
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn send_level_destinations_compare_by_bus() {
        let a = ModDestination::SendLevel {
            bus: SendBusIndex::new(0),
        };
        let b = ModDestination::SendLevel {
            bus: SendBusIndex::new(0),
        };
        let c = ModDestination::SendLevel {
            bus: SendBusIndex::new(1),
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn index_newtypes_expose_their_value() {
        assert_eq!(EffectSlotIndex::new(3).value(), 3);
        assert_eq!(EffectParamIndex::new(5).value(), 5);
        assert_eq!(SendBusIndex::new(2).value(), 2);
    }

    #[test]
    fn display_formats_are_human_readable() {
        assert_eq!(ModDestination::OscillatorPitch.to_string(), "oscillator pitch");
        assert_eq!(ModDestination::LfoDepth.to_string(), "LFO depth");
        assert_eq!(
            ModDestination::EffectParam {
                slot: EffectSlotIndex::new(0),
                param: EffectParamIndex::new(1),
            }
            .to_string(),
            "effect param (slot 0, param 1)"
        );
        assert_eq!(
            ModDestination::SendLevel {
                bus: SendBusIndex::new(4)
            }
            .to_string(),
            "send level (bus 4)"
        );
    }
}
