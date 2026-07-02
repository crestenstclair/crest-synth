// path: src/kernel/channel_address.rs

//! A fully-qualified MIDI destination: a channel within a group.
//!
//! `ChannelAddress` is a plain value object — it holds no dependencies and
//! performs no I/O, allocation, or locking, so it is safe to construct,
//! copy, and compare on the real-time audio thread.

use std::error::Error;
use std::fmt;

/// A MIDI channel number.
///
/// Classic MIDI exposes 16 channels per cable (0-15, displayed to users as
/// 1-16). This newtype stores the zero-based value and enforces the valid
/// range at construction so an out-of-range channel can never exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MidiChannel(u8);

/// The valid range of MIDI channel numbers (zero-based, inclusive).
const MIDI_CHANNEL_RANGE: std::ops::RangeInclusive<u8> = 0..=15;

impl MidiChannel {
    /// Attempts to construct a `MidiChannel`, rejecting values outside the
    /// valid 0-15 range.
    pub fn try_new(value: u8) -> Result<Self, MidiChannelError> {
        if !MIDI_CHANNEL_RANGE.contains(&value) {
            return Err(MidiChannelError::OutOfRange(value));
        }
        Ok(Self(value))
    }

    /// Returns the zero-based channel number.
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl fmt::Display for MidiChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Errors produced when constructing a [`MidiChannel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiChannelError {
    /// The supplied value fell outside the valid 0-15 range.
    OutOfRange(u8),
}

impl fmt::Display for MidiChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MidiChannelError::OutOfRange(value) => {
                write!(f, "MIDI channel {value} is out of range (expected 0..=15)")
            }
        }
    }
}

impl Error for MidiChannelError {}

/// A MIDI group number.
///
/// Groups partition a MIDI 2.0 Universal MIDI Packet stream (or, in this
/// project, a logical bank of MIDI channel sets) into independently
/// addressable sets of 16 channels each. This newtype stores the zero-based
/// group index and enforces the valid range at construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MidiGroup(u8);

/// The valid range of MIDI group numbers (zero-based, inclusive).
const MIDI_GROUP_RANGE: std::ops::RangeInclusive<u8> = 0..=15;

impl MidiGroup {
    /// Attempts to construct a `MidiGroup`, rejecting values outside the
    /// valid 0-15 range.
    pub fn try_new(value: u8) -> Result<Self, MidiGroupError> {
        if !MIDI_GROUP_RANGE.contains(&value) {
            return Err(MidiGroupError::OutOfRange(value));
        }
        Ok(Self(value))
    }

    /// Returns the zero-based group index.
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl fmt::Display for MidiGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Errors produced when constructing a [`MidiGroup`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiGroupError {
    /// The supplied value fell outside the valid 0-15 range.
    OutOfRange(u8),
}

impl fmt::Display for MidiGroupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MidiGroupError::OutOfRange(value) => {
                write!(f, "MIDI group {value} is out of range (expected 0..=15)")
            }
        }
    }
}

impl Error for MidiGroupError {}

/// A fully-qualified MIDI destination: a channel within a group.
///
/// `ChannelAddress` identifies exactly one channel slot in the system. Two
/// addresses are equal only when both their group and channel match, which
/// is what channel-mapping dispatch relies on: a `MidiEvent` reaches every
/// patch whose mapping contains a `ChannelAddress` equal to the event's
/// address, and no others.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelAddress {
    channel: MidiChannel,
    group: MidiGroup,
}

impl ChannelAddress {
    /// Constructs a `ChannelAddress` from an already-validated channel and
    /// group.
    pub fn new(channel: MidiChannel, group: MidiGroup) -> Self {
        Self { channel, group }
    }

    /// Returns the channel component of this address.
    pub fn channel(&self) -> MidiChannel {
        self.channel
    }

    /// Returns the group component of this address.
    pub fn group(&self) -> MidiGroup {
        self.group
    }

    /// Returns `true` if `other` names the same channel within the same
    /// group as `self`.
    ///
    /// This is the equality check channel-mapping dispatch uses: a
    /// `MidiEvent` is routed to a patch precisely when the event's address
    /// matches an address in that patch's channel mapping.
    pub fn matches(&self, other: &ChannelAddress) -> bool {
        self == other
    }
}

impl fmt::Display for ChannelAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "group {} / channel {}", self.group, self.channel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_channel_accepts_full_valid_range() {
        for raw in 0..=15u8 {
            assert_eq!(MidiChannel::try_new(raw).unwrap().value(), raw);
        }
    }

    #[test]
    fn midi_channel_rejects_out_of_range() {
        let err = MidiChannel::try_new(16).unwrap_err();
        assert_eq!(err, MidiChannelError::OutOfRange(16));
    }

    #[test]
    fn midi_group_accepts_full_valid_range() {
        for raw in 0..=15u8 {
            assert_eq!(MidiGroup::try_new(raw).unwrap().value(), raw);
        }
    }

    #[test]
    fn midi_group_rejects_out_of_range() {
        let err = MidiGroup::try_new(200).unwrap_err();
        assert_eq!(err, MidiGroupError::OutOfRange(200));
    }

    #[test]
    fn addresses_with_same_channel_and_group_are_equal() {
        let a = ChannelAddress::new(
            MidiChannel::try_new(3).unwrap(),
            MidiGroup::try_new(1).unwrap(),
        );
        let b = ChannelAddress::new(
            MidiChannel::try_new(3).unwrap(),
            MidiGroup::try_new(1).unwrap(),
        );
        assert_eq!(a, b);
        assert!(a.matches(&b));
    }

    #[test]
    fn addresses_differing_by_channel_do_not_match() {
        let a = ChannelAddress::new(
            MidiChannel::try_new(3).unwrap(),
            MidiGroup::try_new(1).unwrap(),
        );
        let b = ChannelAddress::new(
            MidiChannel::try_new(4).unwrap(),
            MidiGroup::try_new(1).unwrap(),
        );
        assert!(!a.matches(&b));
    }

    #[test]
    fn addresses_differing_by_group_do_not_match() {
        let a = ChannelAddress::new(
            MidiChannel::try_new(3).unwrap(),
            MidiGroup::try_new(1).unwrap(),
        );
        let b = ChannelAddress::new(
            MidiChannel::try_new(3).unwrap(),
            MidiGroup::try_new(2).unwrap(),
        );
        assert!(!a.matches(&b));
    }

    #[test]
    fn accessors_return_the_constructed_components() {
        let channel = MidiChannel::try_new(9).unwrap();
        let group = MidiGroup::try_new(5).unwrap();
        let address = ChannelAddress::new(channel, group);
        assert_eq!(address.channel(), channel);
        assert_eq!(address.group(), group);
    }
}
