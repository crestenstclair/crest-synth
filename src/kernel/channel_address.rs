// path: src/kernel/channel_address.rs

use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_group::MidiGroup;

/// A (group, channel) pair that addresses one of the 256 destinations in the
/// MIDI 2.0 address space.
///
/// MIDI 2.0 extends the traditional 16-channel model by adding 16 groups, each
/// containing 16 channels, for a total of 256 independently addressable
/// destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelAddress {
    group: MidiGroup,
    channel: MidiChannel,
}

impl ChannelAddress {
    /// Construct a `ChannelAddress` from a `MidiGroup` and a `MidiChannel`.
    ///
    /// Both components must already be valid (use [`MidiGroup::try_new`] and
    /// [`MidiChannel::try_new`] to validate raw values before passing them
    /// here).
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::kernel::channel_address::ChannelAddress;
    /// use crest_synth::kernel::midi_channel::MidiChannel;
    /// use crest_synth::kernel::midi_group::MidiGroup;
    ///
    /// if let (Ok(group), Ok(channel)) = (MidiGroup::try_new(0), MidiChannel::try_new(0)) {
    ///     let addr = ChannelAddress::new(group, channel);
    ///     assert_eq!(addr.group().value(), 0);
    ///     assert_eq!(addr.channel().value(), 0);
    /// }
    /// ```
    pub fn new(group: MidiGroup, channel: MidiChannel) -> Self {
        Self { group, channel }
    }

    /// Return the `MidiGroup` component of this address.
    #[inline]
    pub fn group(self) -> MidiGroup {
        self.group
    }

    /// Return the `MidiChannel` component of this address.
    #[inline]
    pub fn channel(self) -> MidiChannel {
        self.channel
    }

    /// Return the linear index of this address in the 256-destination space
    /// (group * 16 + channel).
    ///
    /// The index is in the range 0–255.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::kernel::channel_address::ChannelAddress;
    /// use crest_synth::kernel::midi_channel::MidiChannel;
    /// use crest_synth::kernel::midi_group::MidiGroup;
    ///
    /// if let (Ok(group), Ok(channel)) = (MidiGroup::try_new(1), MidiChannel::try_new(3)) {
    ///     let addr = ChannelAddress::new(group, channel);
    ///     assert_eq!(addr.linear_index(), 1 * 16 + 3);
    /// }
    /// ```
    #[inline]
    pub fn linear_index(self) -> u8 {
        self.group.value() * 16 + self.channel.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_group::MidiGroup;

    fn make(group: u8, channel: u8) -> ChannelAddress {
        ChannelAddress::new(
            MidiGroup::try_new(group).unwrap(),
            MidiChannel::try_new(channel).unwrap(),
        )
    }

    #[test]
    fn group_zero_channel_zero() {
        let addr = make(0, 0);
        assert_eq!(addr.group().value(), 0);
        assert_eq!(addr.channel().value(), 0);
    }

    #[test]
    fn max_address() {
        let addr = make(15, 15);
        assert_eq!(addr.group().value(), 15);
        assert_eq!(addr.channel().value(), 15);
    }

    #[test]
    fn linear_index_is_group_times_16_plus_channel() {
        let addr = make(3, 7);
        assert_eq!(addr.linear_index(), 3 * 16 + 7);
    }

    #[test]
    fn linear_index_min() {
        assert_eq!(make(0, 0).linear_index(), 0);
    }

    #[test]
    fn linear_index_max() {
        assert_eq!(make(15, 15).linear_index(), 255);
    }

    #[test]
    fn equality() {
        let a = make(2, 5);
        let b = make(2, 5);
        assert_eq!(a, b);
    }

    #[test]
    fn inequality_different_group() {
        let a = make(1, 5);
        let b = make(2, 5);
        assert_ne!(a, b);
    }

    #[test]
    fn inequality_different_channel() {
        let a = make(2, 4);
        let b = make(2, 5);
        assert_ne!(a, b);
    }

    #[test]
    fn copy_semantics() {
        let a = make(7, 3);
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn hash_consistent_with_equality() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(make(0, 0));
        set.insert(make(0, 0)); // duplicate
        assert_eq!(set.len(), 1);
        set.insert(make(1, 0));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn all_256_destinations_are_distinct() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for g in 0u8..16 {
            for c in 0u8..16 {
                set.insert(make(g, c));
            }
        }
        assert_eq!(set.len(), 256);
    }

    #[test]
    fn all_256_linear_indices_are_distinct() {
        use std::collections::HashSet;
        let mut indices: HashSet<u8> = HashSet::new();
        for g in 0u8..16 {
            for c in 0u8..16 {
                indices.insert(make(g, c).linear_index());
            }
        }
        assert_eq!(indices.len(), 256);
    }
}
