// path: src/kernel/note_number.rs

//! `NoteNumber` is a newtype over `u8` representing a MIDI note number.
//!
//! MIDI note numbers are constrained to the range 0..=127 inclusive. This
//! type makes that constraint a compile-time-enforced invariant: the only
//! way to obtain a `NoteNumber` is through [`NoteNumber::try_new`], which
//! validates the range and rejects out-of-range values.

use std::error::Error;
use std::fmt;

/// A validated MIDI note number in the range 0..=127.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteNumber(u8);

/// Error returned when constructing a [`NoteNumber`] from an out-of-range value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteNumberError {
    value: u8,
}

impl fmt::Display for NoteNumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid MIDI note number {}: must be in range {}..={}",
            self.value,
            NoteNumber::MIN,
            NoteNumber::MAX
        )
    }
}

impl Error for NoteNumberError {}

impl NoteNumber {
    /// The inclusive lower bound of a valid MIDI note number.
    pub const MIN: u8 = 0;

    /// The inclusive upper bound of a valid MIDI note number.
    pub const MAX: u8 = 127;

    /// Attempts to construct a `NoteNumber` from a raw `u8`.
    ///
    /// Returns an error if `value` is outside the inclusive range 0..=127.
    ///
    /// ```
    /// use crest_synth::kernel::note_number::NoteNumber;
    ///
    /// assert!(NoteNumber::try_new(60).is_ok());
    /// assert!(NoteNumber::try_new(127).is_ok());
    /// assert!(NoteNumber::try_new(128).is_err());
    /// ```
    pub fn try_new(value: u8) -> Result<Self, NoteNumberError> {
        if !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(NoteNumberError { value });
        }
        Ok(Self(value))
    }

    /// Returns the raw `u8` value.
    pub fn value(self) -> u8 {
        self.0
    }

    /// The lowest valid MIDI note number (0).
    pub fn min() -> Self {
        Self(Self::MIN)
    }

    /// The highest valid MIDI note number (127).
    pub fn max() -> Self {
        Self(Self::MAX)
    }
}

impl fmt::Display for NoteNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<u8> for NoteNumber {
    type Error = NoteNumberError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<NoteNumber> for u8 {
    fn from(note: NoteNumber) -> Self {
        note.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_minimum_value() {
        let note = NoteNumber::try_new(0).expect("0 is a valid note number");
        assert_eq!(note.value(), 0);
    }

    #[test]
    fn accepts_maximum_value() {
        let note = NoteNumber::try_new(127).expect("127 is a valid note number");
        assert_eq!(note.value(), 127);
    }

    #[test]
    fn accepts_middle_value() {
        let note = NoteNumber::try_new(60).expect("60 is a valid note number");
        assert_eq!(note.value(), 60);
    }

    #[test]
    fn rejects_value_above_maximum() {
        let result = NoteNumber::try_new(128);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_value_well_above_maximum() {
        let result = NoteNumber::try_new(255);
        assert!(result.is_err());
    }

    #[test]
    fn error_message_mentions_bounds() {
        let err = NoteNumber::try_new(200).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("200"));
        assert!(message.contains("0"));
        assert!(message.contains("127"));
    }

    #[test]
    fn try_from_delegates_to_try_new() {
        let note: Result<NoteNumber, _> = NoteNumber::try_from(64);
        assert_eq!(note.unwrap().value(), 64);
    }

    #[test]
    fn into_u8_round_trips() {
        let note = NoteNumber::try_new(72).unwrap();
        let raw: u8 = note.into();
        assert_eq!(raw, 72);
    }

    #[test]
    fn min_and_max_helpers_are_valid() {
        assert_eq!(NoteNumber::min().value(), NoteNumber::MIN);
        assert_eq!(NoteNumber::max().value(), NoteNumber::MAX);
    }

    #[test]
    fn display_shows_raw_value() {
        let note = NoteNumber::try_new(60).unwrap();
        assert_eq!(format!("{}", note), "60");
    }

    #[test]
    fn ordering_reflects_underlying_value() {
        let low = NoteNumber::try_new(10).unwrap();
        let high = NoteNumber::try_new(20).unwrap();
        assert!(low < high);
    }
}
