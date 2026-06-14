/// MIDI channel within a group (0-15).
///
/// A `MidiChannel` identifies one of the 16 channels available in a MIDI
/// group. Valid values are 0–15 inclusive. Attempting to construct a value
/// outside that range returns an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MidiChannel(u8);

/// Error returned when a `MidiChannel` value is out of the 0-15 range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiChannelError(u8);

impl std::fmt::Display for MidiChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MidiChannel value {} is out of range 0-15", self.0)
    }
}

impl std::error::Error for MidiChannelError {}

impl MidiChannel {
    /// Construct a `MidiChannel` from a raw `u8`.
    ///
    /// Returns `Err` if `value` is not in the range 0-15.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::kernel::midi_channel::MidiChannel;
    ///
    /// assert!(MidiChannel::try_new(0).is_ok());
    /// assert!(MidiChannel::try_new(15).is_ok());
    /// assert!(MidiChannel::try_new(16).is_err());
    /// ```
    pub fn try_new(value: u8) -> Result<Self, MidiChannelError> {
        if !(0..=15).contains(&value) {
            return Err(MidiChannelError(value));
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
        assert!(MidiChannel::try_new(0).is_ok());
    }

    #[test]
    fn fifteen_is_valid() {
        let ch = MidiChannel::try_new(15).unwrap();
        assert_eq!(ch.value(), 15);
    }

    #[test]
    fn sixteen_is_invalid() {
        assert!(MidiChannel::try_new(16).is_err());
    }

    #[test]
    fn max_u8_is_invalid() {
        assert!(MidiChannel::try_new(255).is_err());
    }

    #[test]
    fn value_roundtrips() {
        for v in 0u8..=15 {
            let ch = MidiChannel::try_new(v).unwrap();
            assert_eq!(ch.value(), v);
        }
    }

    #[test]
    fn error_message_contains_value() {
        let err = MidiChannel::try_new(20).unwrap_err();
        assert!(err.to_string().contains("20"));
    }

    #[test]
    fn copy_semantics() {
        let a = MidiChannel::try_new(7).unwrap();
        let b = a;
        assert_eq!(a.value(), b.value());
    }

    #[test]
    fn ordering() {
        let lo = MidiChannel::try_new(3).unwrap();
        let hi = MidiChannel::try_new(12).unwrap();
        assert!(lo < hi);
    }
}
