// path: src/kernel/midi_group.rs

use std::fmt;

/// MIDI 2.0 group index.
///
/// A `MidiGroup` identifies one of the 16 UMP groups (0-15) that MIDI 2.0
/// uses to address a stream of messages within a single physical or virtual
/// connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MidiGroup(u8);

/// Error returned when constructing a [`MidiGroup`] from an out-of-range value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiGroupError {
    value: u8,
}

impl fmt::Display for MidiGroupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MIDI group index {} is out of range: must be 0-15",
            self.value
        )
    }
}

impl std::error::Error for MidiGroupError {}

impl MidiGroup {
    /// The smallest valid MIDI group index.
    pub const MIN: u8 = 0;
    /// The largest valid MIDI group index.
    pub const MAX: u8 = 15;

    /// Constructs a new [`MidiGroup`], validating that `value` is within
    /// the inclusive range 0-15.
    pub fn try_new(value: u8) -> Result<Self, MidiGroupError> {
        if !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(MidiGroupError { value });
        }
        Ok(Self(value))
    }

    /// Returns the underlying group index.
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for MidiGroup {
    type Error = MidiGroupError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<MidiGroup> for u8 {
    fn from(group: MidiGroup) -> Self {
        group.0
    }
}

impl fmt::Display for MidiGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_min_boundary() {
        assert!(MidiGroup::try_new(0).is_ok());
    }

    #[test]
    fn accepts_max_boundary() {
        assert!(MidiGroup::try_new(15).is_ok());
    }

    #[test]
    fn rejects_above_max() {
        assert!(MidiGroup::try_new(16).is_err());
    }

    #[test]
    fn value_round_trips() {
        let group = MidiGroup::try_new(7).unwrap();
        assert_eq!(group.value(), 7);
        assert_eq!(u8::from(group), 7);
    }

    #[test]
    fn try_from_matches_try_new() {
        let group: MidiGroup = 3.try_into().unwrap();
        assert_eq!(group.value(), 3);
    }

    #[test]
    fn display_shows_numeric_value() {
        let group = MidiGroup::try_new(9).unwrap();
        assert_eq!(group.to_string(), "9");
    }

    #[test]
    fn error_message_mentions_range() {
        let err = MidiGroup::try_new(200).unwrap_err();
        assert!(err.to_string().contains("0-15"));
    }
}
