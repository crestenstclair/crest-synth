// path: src/modulation/mod_source.rs

//! `ModSource` — a modulation source: LFO 1-4, amp/filter/pitch/mod envelope,
//! velocity, key tracking, aftertouch, pitch bend, mod wheel, expression,
//! MPE X/Y/Z, or any MIDI CC 0-127.
//!
//! This is a pure value object: no I/O, no allocation, no interior mutability.
//! It is `Copy` so it can be moved across the real-time boundary (e.g. as a
//! key inside a `ModMatrix` routing table) without heap allocation.

use std::fmt;

/// One of the four LFO slots available as a modulation source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LfoIndex {
    One,
    Two,
    Three,
    Four,
}

impl LfoIndex {
    /// Construct an `LfoIndex` from its 1-based slot number.
    ///
    /// Returns `None` for any value outside `1..=4`.
    pub fn try_from_slot(slot: u8) -> Option<Self> {
        match slot {
            1 => Some(Self::One),
            2 => Some(Self::Two),
            3 => Some(Self::Three),
            4 => Some(Self::Four),
            _ => None,
        }
    }

    /// The 1-based slot number this index represents.
    pub fn slot(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
        }
    }
}

impl fmt::Display for LfoIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LFO {}", self.slot())
    }
}

/// Which envelope generator is acting as the modulation source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnvelopeSource {
    Amp,
    Filter,
    Pitch,
    Mod,
}

impl fmt::Display for EnvelopeSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Amp => "amp",
            Self::Filter => "filter",
            Self::Pitch => "pitch",
            Self::Mod => "mod",
        };
        write!(f, "{name} envelope")
    }
}

/// A MIDI Control Change number, constrained to the valid `0..=127` range.
///
/// Constructed only through [`CcNumber::try_new`] so no `CcNumber` can ever
/// hold an out-of-range value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CcNumber(u8);

impl CcNumber {
    /// Construct a `CcNumber`, validating that `value` is within `0..=127`.
    ///
    /// Returns `None` when `value` is out of range.
    pub fn try_new(value: u8) -> Option<Self> {
        if !(0..=127).contains(&value) {
            return None;
        }
        Some(Self(value))
    }

    /// The raw MIDI CC number.
    pub fn value(self) -> u8 {
        self.0
    }
}

impl fmt::Display for CcNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CC{}", self.0)
    }
}

/// A modulation source: anything that can drive an entry in the mod matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModSource {
    Lfo(LfoIndex),
    Envelope(EnvelopeSource),
    Velocity,
    KeyTracking,
    Aftertouch,
    PitchBend,
    ModWheel,
    Expression,
    MpeX,
    MpeY,
    MpeZ,
    ControlChange(CcNumber),
}

impl fmt::Display for ModSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lfo(index) => write!(f, "{index}"),
            Self::Envelope(source) => write!(f, "{source}"),
            Self::Velocity => write!(f, "velocity"),
            Self::KeyTracking => write!(f, "key tracking"),
            Self::Aftertouch => write!(f, "aftertouch"),
            Self::PitchBend => write!(f, "pitch bend"),
            Self::ModWheel => write!(f, "mod wheel"),
            Self::Expression => write!(f, "expression"),
            Self::MpeX => write!(f, "MPE X"),
            Self::MpeY => write!(f, "MPE Y"),
            Self::MpeZ => write!(f, "MPE Z"),
            Self::ControlChange(cc) => write!(f, "{cc}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lfo_index_round_trips_valid_slots() {
        assert_eq!(LfoIndex::try_from_slot(1), Some(LfoIndex::One));
        assert_eq!(LfoIndex::try_from_slot(2), Some(LfoIndex::Two));
        assert_eq!(LfoIndex::try_from_slot(3), Some(LfoIndex::Three));
        assert_eq!(LfoIndex::try_from_slot(4), Some(LfoIndex::Four));
        assert_eq!(LfoIndex::One.slot(), 1);
        assert_eq!(LfoIndex::Four.slot(), 4);
    }

    #[test]
    fn lfo_index_rejects_out_of_range_slots() {
        assert_eq!(LfoIndex::try_from_slot(0), None);
        assert_eq!(LfoIndex::try_from_slot(5), None);
    }

    #[test]
    fn cc_number_accepts_full_valid_range() {
        assert!(CcNumber::try_new(0).is_some());
        assert!(CcNumber::try_new(127).is_some());
        assert_eq!(CcNumber::try_new(64).map(CcNumber::value), Some(64));
    }

    #[test]
    fn cc_number_rejects_out_of_range() {
        assert_eq!(CcNumber::try_new(128), None);
        assert_eq!(CcNumber::try_new(255), None);
    }

    #[test]
    fn mod_source_variants_are_distinct() {
        assert_ne!(ModSource::Velocity, ModSource::KeyTracking);
        assert_ne!(
            ModSource::Lfo(LfoIndex::One),
            ModSource::Lfo(LfoIndex::Two)
        );
        assert_eq!(
            ModSource::Envelope(EnvelopeSource::Amp),
            ModSource::Envelope(EnvelopeSource::Amp)
        );
    }

    #[test]
    fn mod_source_control_change_wraps_validated_cc() {
        let cc = CcNumber::try_new(74).expect("74 is a valid CC number");
        let source = ModSource::ControlChange(cc);
        assert_eq!(source.to_string(), "CC74");
    }

    #[test]
    fn mod_source_display_covers_named_sources() {
        assert_eq!(ModSource::Velocity.to_string(), "velocity");
        assert_eq!(ModSource::MpeX.to_string(), "MPE X");
        assert_eq!(ModSource::Lfo(LfoIndex::Three).to_string(), "LFO 3");
        assert_eq!(
            ModSource::Envelope(EnvelopeSource::Filter).to_string(),
            "filter envelope"
        );
    }

    #[test]
    fn mod_source_is_copy_and_hashable() {
        use std::collections::HashSet;

        let a = ModSource::PitchBend;
        let b = a;
        assert_eq!(a, b);

        let mut set = HashSet::new();
        set.insert(ModSource::ModWheel);
        set.insert(ModSource::Expression);
        assert!(set.contains(&ModSource::ModWheel));
    }
}
