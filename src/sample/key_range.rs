// path: src/sample/key_range.rs

//! `KeyRange` — inclusive MIDI note range a sample zone responds to.
//!
//! Pairs a lower and upper `NoteNumber` bound and enforces the invariant
//! that `low <= high`. This is a plain value object: no allocation, no
//! I/O, no locking — safe to read on the audio thread.

use std::error::Error;
use std::fmt;

use crate::kernel::note_number::NoteNumber;

/// Inclusive note range `[low, high]` that a sample zone responds to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyRange {
    low: NoteNumber,
    high: NoteNumber,
}

/// Error returned when constructing a [`KeyRange`] with `low > high`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyRangeError {
    low: NoteNumber,
    high: NoteNumber,
}

impl fmt::Display for KeyRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid key range: low ({}) must be <= high ({})",
            self.low, self.high
        )
    }
}

impl Error for KeyRangeError {}

impl KeyRange {
    /// Attempts to construct a `KeyRange` from a `low` and `high` bound.
    ///
    /// Returns an error if `low > high`.
    ///
    /// ```
    /// use crest_synth::kernel::note_number::NoteNumber;
    /// use crest_synth::sample::key_range::KeyRange;
    ///
    /// let low = NoteNumber::try_new(48).unwrap();
    /// let high = NoteNumber::try_new(72).unwrap();
    /// assert!(KeyRange::try_new(low, high).is_ok());
    /// assert!(KeyRange::try_new(high, low).is_err());
    /// ```
    pub fn try_new(low: NoteNumber, high: NoteNumber) -> Result<Self, KeyRangeError> {
        if low > high {
            return Err(KeyRangeError { low, high });
        }
        Ok(Self { low, high })
    }

    /// The inclusive lower bound.
    pub fn low(&self) -> NoteNumber {
        self.low
    }

    /// The inclusive upper bound.
    pub fn high(&self) -> NoteNumber {
        self.high
    }

    /// True when `note` falls within `[low, high]`.
    pub fn contains(&self, note: NoteNumber) -> bool {
        (self.low..=self.high).contains(&note)
    }

    /// A range spanning the full valid MIDI note space (0..=127).
    pub fn full() -> Self {
        Self {
            low: NoteNumber::min(),
            high: NoteNumber::max(),
        }
    }
}

impl fmt::Display for KeyRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.low, self.high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(value: u8) -> NoteNumber {
        NoteNumber::try_new(value).unwrap()
    }

    #[test]
    fn accepts_low_equal_to_high() {
        let range = KeyRange::try_new(note(60), note(60)).unwrap();
        assert_eq!(range.low(), note(60));
        assert_eq!(range.high(), note(60));
    }

    #[test]
    fn accepts_low_less_than_high() {
        let range = KeyRange::try_new(note(48), note(72)).unwrap();
        assert_eq!(range.low(), note(48));
        assert_eq!(range.high(), note(72));
    }

    #[test]
    fn rejects_low_greater_than_high() {
        let result = KeyRange::try_new(note(72), note(48));
        assert!(result.is_err());
    }

    #[test]
    fn error_message_mentions_bounds() {
        let err = KeyRange::try_new(note(72), note(48)).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("72"));
        assert!(message.contains("48"));
    }

    #[test]
    fn contains_reports_true_within_bounds_inclusive() {
        let range = KeyRange::try_new(note(60), note(72)).unwrap();
        assert!(range.contains(note(60)));
        assert!(range.contains(note(66)));
        assert!(range.contains(note(72)));
    }

    #[test]
    fn contains_reports_false_outside_bounds() {
        let range = KeyRange::try_new(note(60), note(72)).unwrap();
        assert!(!range.contains(note(59)));
        assert!(!range.contains(note(73)));
    }

    #[test]
    fn full_spans_entire_midi_note_space() {
        let range = KeyRange::full();
        assert_eq!(range.low(), NoteNumber::min());
        assert_eq!(range.high(), NoteNumber::max());
    }

    #[test]
    fn display_shows_bounds() {
        let range = KeyRange::try_new(note(60), note(72)).unwrap();
        assert_eq!(format!("{}", range), "[60, 72]");
    }

    #[test]
    fn ordering_reflects_low_then_high() {
        let a = KeyRange::try_new(note(10), note(20)).unwrap();
        let b = KeyRange::try_new(note(10), note(30)).unwrap();
        assert!(a < b);
    }
}
