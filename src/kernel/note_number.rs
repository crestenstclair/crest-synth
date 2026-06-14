/// MIDI note number (0-127).
///
/// `NoteNumber` wraps a `u8` and enforces the invariant that the value
/// is within the range 0 to 127 inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NoteNumber(u8);

/// Error returned when a `NoteNumber` value is out of the 0-127 range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteNumberError(u8);

impl std::fmt::Display for NoteNumberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoteNumber value {} is out of range 0-127", self.0)
    }
}

impl std::error::Error for NoteNumberError {}

impl NoteNumber {
    /// Construct a `NoteNumber` from a raw `u8`.
    ///
    /// Returns `Err` if `value` is not in the range 0-127.
    ///
    /// ```
    /// use crest_synth::kernel::note_number::NoteNumber;
    /// assert!(NoteNumber::try_new(0).is_ok());
    /// assert!(NoteNumber::try_new(127).is_ok());
    /// assert!(NoteNumber::try_new(128).is_err());
    /// ```
    pub fn try_new(value: u8) -> Result<Self, NoteNumberError> {
        if !(0..=127).contains(&value) {
            return Err(NoteNumberError(value));
        }
        Ok(Self(value))
    }

    /// Return the underlying `u8` value.
    #[inline]
    pub fn value(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_valid() {
        assert!(NoteNumber::try_new(0).is_ok());
    }

    #[test]
    fn max_valid_is_127() {
        let nn = NoteNumber::try_new(127).unwrap();
        assert_eq!(nn.value(), 127);
    }

    #[test]
    fn out_of_range_rejected() {
        assert!(NoteNumber::try_new(128).is_err());
        assert!(NoteNumber::try_new(255).is_err());
    }

    #[test]
    fn value_round_trips() {
        for v in 0u8..=127 {
            let nn = NoteNumber::try_new(v).unwrap();
            assert_eq!(nn.value(), v);
        }
    }

    #[test]
    fn ordering() {
        let lo = NoteNumber::try_new(60).unwrap();
        let hi = NoteNumber::try_new(72).unwrap();
        assert!(lo < hi);
    }

    #[test]
    fn error_message_contains_value() {
        let err = NoteNumber::try_new(200).unwrap_err();
        assert!(err.to_string().contains("200"));
    }
}
