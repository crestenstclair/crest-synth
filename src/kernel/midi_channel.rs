// path: src/kernel/midi_channel.rs

//! `MidiChannel` — channel within a group.
//!
//! A small newtype around `u8` that guarantees the wrapped value is a valid
//! MIDI channel number. MIDI channels are addressed 0-15 (the 16 channels of
//! a single MIDI cable); any other value is meaningless and would make
//! downstream channel-mapping and dispatch logic degenerate.

use std::fmt;

/// A single MIDI channel within a group (0-15).
///
/// Invariant: the wrapped value is always in the range `0..=15`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MidiChannel(u8);

/// Error returned when constructing a `MidiChannel` from an invalid value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiChannelError {
    /// The supplied value was outside the valid `0..=15` range.
    OutOfRange(u8),
}

impl fmt::Display for MidiChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MidiChannelError::OutOfRange(value) => {
                write!(f, "midi channel must be 0-15 (got {value})")
            }
        }
    }
}

impl std::error::Error for MidiChannelError {}

impl MidiChannel {
    /// Attempts to construct a `MidiChannel` from a raw channel number.
    ///
    /// Returns `Err(MidiChannelError::OutOfRange)` if `channel` is not in
    /// `0..=15`.
    ///
    /// ```
    /// use crest_synth::kernel::midi_channel::MidiChannel;
    ///
    /// assert!(MidiChannel::try_new(0).is_ok());
    /// assert!(MidiChannel::try_new(15).is_ok());
    /// assert!(MidiChannel::try_new(16).is_err());
    /// ```
    pub fn try_new(channel: u8) -> Result<Self, MidiChannelError> {
        if !(0..=15).contains(&channel) {
            Err(MidiChannelError::OutOfRange(channel))
        } else {
            Ok(Self(channel))
        }
    }

    /// Returns the channel number as a raw `u8`.
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for MidiChannel {
    type Error = MidiChannelError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<MidiChannel> for u8 {
    fn from(value: MidiChannel) -> Self {
        value.0
    }
}

impl fmt::Display for MidiChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_in_range_values() {
        let channel = MidiChannel::try_new(9).expect("9 is in range");
        assert_eq!(channel.value(), 9);
    }

    #[test]
    fn try_new_accepts_lower_bound() {
        let channel = MidiChannel::try_new(0).expect("0 is in range");
        assert_eq!(channel.value(), 0);
    }

    #[test]
    fn try_new_accepts_upper_bound() {
        let channel = MidiChannel::try_new(15).expect("15 is in range");
        assert_eq!(channel.value(), 15);
    }

    #[test]
    fn try_new_rejects_above_range() {
        let result = MidiChannel::try_new(16);
        assert_eq!(result, Err(MidiChannelError::OutOfRange(16)));
    }

    #[test]
    fn try_new_rejects_far_above_range() {
        let result = MidiChannel::try_new(255);
        assert_eq!(result, Err(MidiChannelError::OutOfRange(255)));
    }

    #[test]
    fn try_from_u8_matches_try_new() {
        let channel: Result<MidiChannel, _> = MidiChannel::try_from(3);
        assert_eq!(channel.expect("3 is in range").value(), 3);

        let err: Result<MidiChannel, _> = MidiChannel::try_from(16);
        assert!(err.is_err());
    }

    #[test]
    fn into_u8_round_trips() {
        let channel = MidiChannel::try_new(7).expect("7 is in range");
        let raw: u8 = channel.into();
        assert_eq!(raw, 7);
    }

    #[test]
    fn display_shows_raw_value() {
        let channel = MidiChannel::try_new(12).expect("12 is in range");
        assert_eq!(channel.to_string(), "12");
    }

    #[test]
    fn ordering_and_equality_are_derived_from_value() {
        let low = MidiChannel::try_new(1).expect("1 is in range");
        let high = MidiChannel::try_new(10).expect("10 is in range");
        assert!(low < high);
        assert_eq!(low, MidiChannel::try_new(1).expect("1 is in range"));
    }

    #[test]
    fn error_message_is_descriptive() {
        let err = MidiChannel::try_new(16).expect_err("16 is invalid");
        assert_eq!(err.to_string(), "midi channel must be 0-15 (got 16)");
    }
}
