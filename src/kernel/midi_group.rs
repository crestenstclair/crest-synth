/// MIDI 2.0 group index (0–15).
///
/// `MidiGroup` wraps a `u8` and enforces the invariant that the value
/// is within the range 0 to 15 inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MidiGroup(u8);

impl MidiGroup {
    /// Construct a `MidiGroup` from a raw `u8`.
    ///
    /// Returns `Err` if the value is outside 0–15.
    ///
    /// ```
    /// use crest_synth::kernel::midi_group::MidiGroup;
    /// assert!(MidiGroup::try_new(0).is_ok());
    /// assert!(MidiGroup::try_new(15).is_ok());
    /// assert!(MidiGroup::try_new(16).is_err());
    /// ```
    pub fn try_new(value: u8) -> Result<Self, MidiGroupError> {
        if !(0..=15).contains(&value) {
            return Err(MidiGroupError::OutOfRange(value));
        }
        Ok(Self(value))
    }

    /// Return the underlying `u8` value.
    pub fn value(self) -> u8 {
        self.0
    }
}

/// Error returned when a `MidiGroup` value is out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiGroupError {
    /// The supplied value was not in 0–15.
    OutOfRange(u8),
}

impl std::fmt::Display for MidiGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MidiGroupError::OutOfRange(v) => {
                write!(f, "MidiGroup value {v} is out of range (must be 0-15)")
            }
        }
    }
}

impl std::error::Error for MidiGroupError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_lower_bound() {
        let g = MidiGroup::try_new(0).expect("0 is valid");
        assert_eq!(g.value(), 0);
    }

    #[test]
    fn valid_upper_bound() {
        let g = MidiGroup::try_new(15).expect("15 is valid");
        assert_eq!(g.value(), 15);
    }

    #[test]
    fn valid_midpoint() {
        let g = MidiGroup::try_new(7).expect("7 is valid");
        assert_eq!(g.value(), 7);
    }

    #[test]
    fn invalid_16_is_rejected() {
        assert_eq!(MidiGroup::try_new(16), Err(MidiGroupError::OutOfRange(16)));
    }

    #[test]
    fn invalid_255_is_rejected() {
        assert_eq!(
            MidiGroup::try_new(255),
            Err(MidiGroupError::OutOfRange(255))
        );
    }

    #[test]
    fn error_display() {
        let err = MidiGroupError::OutOfRange(20);
        assert!(err.to_string().contains("20"));
        assert!(err.to_string().contains("0-15"));
    }

    #[test]
    fn copy_and_equality() {
        let a = MidiGroup::try_new(5).unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn ordering() {
        let low = MidiGroup::try_new(3).unwrap();
        let high = MidiGroup::try_new(12).unwrap();
        assert!(low < high);
    }
}
