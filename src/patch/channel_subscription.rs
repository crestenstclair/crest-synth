// path: src/patch/channel_subscription.rs

use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_group::MidiGroup;
use crate::patch::mpe_zone::MpeZone;

/// Identifies a MIDI channel within a MIDI 2.0 group.
///
/// A `ChannelAddress` combines a MIDI 2.0 group (0–15) with a MIDI channel
/// (0–15), uniquely identifying a logical channel in the system.
///
/// `ChannelAddress` is a pure value object — it is `Copy` and contains no heap
/// allocations, making it safe to construct and compare on the audio thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelAddress {
    group: MidiGroup,
    channel: MidiChannel,
}

impl ChannelAddress {
    /// Construct a new `ChannelAddress`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::kernel::midi_channel::MidiChannel;
    /// use crest_synth::kernel::midi_group::MidiGroup;
    /// use crest_synth::patch::channel_subscription::ChannelAddress;
    ///
    /// let group = MidiGroup::try_new(0).unwrap();
    /// let channel = MidiChannel::try_new(1).unwrap();
    /// let addr = ChannelAddress::new(group, channel);
    /// assert_eq!(addr.group().value(), 0);
    /// assert_eq!(addr.channel().value(), 1);
    /// ```
    pub fn new(group: MidiGroup, channel: MidiChannel) -> Self {
        Self { group, channel }
    }

    /// Return the MIDI group.
    #[inline]
    pub fn group(self) -> MidiGroup {
        self.group
    }

    /// Return the MIDI channel.
    #[inline]
    pub fn channel(self) -> MidiChannel {
        self.channel
    }
}

/// Describes what MIDI address a patch listens to, and optionally which MPE
/// zone it occupies.
///
/// A patch that has no `mpe_zone` responds to plain MIDI on the given
/// `address`.  A patch with an `mpe_zone` also handles per-note expression on
/// the zone's member channels.
///
/// `ChannelSubscription` is a pure value object — it is `Copy` and contains no
/// heap allocations, making it safe to use on the audio thread.
///
/// # Examples
///
/// ```
/// use crest_synth::kernel::midi_channel::MidiChannel;
/// use crest_synth::kernel::midi_group::MidiGroup;
/// use crest_synth::patch::channel_subscription::{ChannelAddress, ChannelSubscription};
///
/// let group   = MidiGroup::try_new(0).unwrap();
/// let channel = MidiChannel::try_new(1).unwrap();
/// let address = ChannelAddress::new(group, channel);
/// let sub = ChannelSubscription::new(address, None);
/// assert_eq!(sub.address(), address);
/// assert!(sub.mpe_zone().is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelSubscription {
    address: ChannelAddress,
    mpe_zone: Option<MpeZone>,
}

impl ChannelSubscription {
    /// Construct a new `ChannelSubscription`.
    pub fn new(address: ChannelAddress, mpe_zone: Option<MpeZone>) -> Self {
        Self { address, mpe_zone }
    }

    /// Return the channel address this subscription listens to.
    #[inline]
    pub fn address(self) -> ChannelAddress {
        self.address
    }

    /// Return the MPE zone, if this patch uses MPE.
    #[inline]
    pub fn mpe_zone(self) -> Option<MpeZone> {
        self.mpe_zone
    }

    /// Return `true` if this subscription matches the given group and channel.
    ///
    /// For a plain subscription, returns `true` only when `group` and `channel`
    /// both match the address exactly.
    ///
    /// For an MPE subscription, also returns `true` for any member channel in
    /// the zone (via [`MpeZone::is_member_channel`]).
    #[inline]
    pub fn matches(&self, group: MidiGroup, channel: MidiChannel) -> bool {
        if self.address.group() != group {
            return false;
        }
        if self.address.channel() == channel {
            return true;
        }
        if let Some(zone) = self.mpe_zone {
            return zone.is_member_channel(channel);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_group::MidiGroup;
    use crate::patch::mpe_zone::MpeZone;

    fn group(v: u8) -> MidiGroup {
        MidiGroup::try_new(v).unwrap()
    }

    fn ch(v: u8) -> MidiChannel {
        MidiChannel::try_new(v).unwrap()
    }

    fn addr(g: u8, c: u8) -> ChannelAddress {
        ChannelAddress::new(group(g), ch(c))
    }

    // ── ChannelAddress ──────────────────────────────────────────────────────

    #[test]
    fn channel_address_stores_group_and_channel() {
        let a = addr(3, 7);
        assert_eq!(a.group().value(), 3);
        assert_eq!(a.channel().value(), 7);
    }

    #[test]
    fn channel_address_copy_semantics() {
        let a = addr(0, 0);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn channel_address_equality() {
        assert_eq!(addr(1, 2), addr(1, 2));
        assert_ne!(addr(1, 2), addr(1, 3));
        assert_ne!(addr(1, 2), addr(2, 2));
    }

    // ── ChannelSubscription — plain (no MPE) ────────────────────────────────

    #[test]
    fn plain_subscription_stores_address() {
        let a = addr(0, 5);
        let sub = ChannelSubscription::new(a, None);
        assert_eq!(sub.address(), a);
        assert!(sub.mpe_zone().is_none());
    }

    #[test]
    fn plain_subscription_matches_exact_address() {
        let sub = ChannelSubscription::new(addr(0, 3), None);
        assert!(sub.matches(group(0), ch(3)));
    }

    #[test]
    fn plain_subscription_does_not_match_wrong_channel() {
        let sub = ChannelSubscription::new(addr(0, 3), None);
        assert!(!sub.matches(group(0), ch(4)));
    }

    #[test]
    fn plain_subscription_does_not_match_wrong_group() {
        let sub = ChannelSubscription::new(addr(0, 3), None);
        assert!(!sub.matches(group(1), ch(3)));
    }

    // ── ChannelSubscription — MPE ───────────────────────────────────────────

    #[test]
    fn mpe_subscription_stores_zone() {
        let zone = MpeZone::try_new(ch(0), ch(1), ch(8)).unwrap();
        let sub = ChannelSubscription::new(addr(0, 0), Some(zone));
        assert_eq!(sub.mpe_zone(), Some(zone));
    }

    #[test]
    fn mpe_subscription_matches_manager_channel() {
        // address channel is the manager — direct match via address
        let zone = MpeZone::try_new(ch(0), ch(1), ch(8)).unwrap();
        let sub = ChannelSubscription::new(addr(0, 0), Some(zone));
        assert!(sub.matches(group(0), ch(0)));
    }

    #[test]
    fn mpe_subscription_matches_member_channels() {
        // Members are ch 1..=8
        let zone = MpeZone::try_new(ch(0), ch(1), ch(8)).unwrap();
        let sub = ChannelSubscription::new(addr(0, 0), Some(zone));
        for m in 1u8..=8 {
            assert!(sub.matches(group(0), ch(m)), "should match member ch {m}");
        }
    }

    #[test]
    fn mpe_subscription_does_not_match_outside_member_channels() {
        // ch 9..=15 are outside zone
        let zone = MpeZone::try_new(ch(0), ch(1), ch(8)).unwrap();
        let sub = ChannelSubscription::new(addr(0, 0), Some(zone));
        for outside in 9u8..=15 {
            assert!(
                !sub.matches(group(0), ch(outside)),
                "should not match ch {outside}"
            );
        }
    }

    #[test]
    fn mpe_subscription_wrong_group_does_not_match() {
        let zone = MpeZone::try_new(ch(0), ch(1), ch(8)).unwrap();
        let sub = ChannelSubscription::new(addr(0, 0), Some(zone));
        // same channel but different group
        assert!(!sub.matches(group(1), ch(1)));
    }

    // ── Multi-patch dispatch ─────────────────────────────────────────────────

    #[test]
    fn two_patches_on_same_channel_both_match() {
        // Validates: channel dispatch delivers events to ALL subscribed patches
        let sub1 = ChannelSubscription::new(addr(0, 4), None);
        let sub2 = ChannelSubscription::new(addr(0, 4), None);
        let g = group(0);
        let c = ch(4);
        assert!(sub1.matches(g, c));
        assert!(sub2.matches(g, c));
    }

    // ── Copy / Clone ─────────────────────────────────────────────────────────

    #[test]
    fn copy_semantics_plain() {
        let sub = ChannelSubscription::new(addr(2, 5), None);
        let sub2 = sub;
        assert_eq!(sub, sub2);
    }

    #[test]
    fn copy_semantics_mpe() {
        let zone = MpeZone::try_new(ch(0), ch(1), ch(8)).unwrap();
        let sub = ChannelSubscription::new(addr(0, 0), Some(zone));
        let sub2 = sub;
        assert_eq!(sub, sub2);
    }
}
