// path: src/patch/channel_mapping.rs

//! `ChannelMapping` — which MIDI addresses a patch listens on.
//!
//! A `ChannelMapping` is a plain value object: an explicit list of
//! [`ChannelAddress`]es the owning patch responds to, plus an `omni` flag
//! that, when set, makes the patch respond to every address regardless of
//! the explicit list. It holds no dependencies and performs no I/O,
//! allocation-on-demand, or locking beyond the `Vec` it owns, so it is safe
//! to construct off the real-time audio thread and hand to dispatch logic
//! by reference.
//!
//! Multiple patches may hold mappings that match the same address — that
//! layering is intentional. What a `ChannelMapping` guarantees is narrower:
//! for a *single* mapping, [`ChannelMapping::matches`] answers exactly
//! whether a given address is one this patch listens on, with no leakage
//! to addresses it was not configured for.

use crate::kernel::channel_address::ChannelAddress;

/// Which MIDI addresses a patch listens on.
///
/// When `omni` is `true` the mapping matches every address, irrespective of
/// the contents of `addresses`. When `omni` is `false`, the mapping matches
/// exactly the addresses present in `addresses` (an empty, non-omni mapping
/// matches nothing).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChannelMapping {
    addresses: Vec<ChannelAddress>,
    omni: bool,
}

impl ChannelMapping {
    /// Constructs a `ChannelMapping` from an explicit set of addresses and
    /// an omni flag.
    ///
    /// No validation is performed on `addresses` beyond what
    /// [`ChannelAddress`] itself already guarantees (each address is a
    /// well-formed channel/group pair); duplicate addresses are harmless
    /// since matching is a membership test, not an enumeration.
    pub fn new(addresses: Vec<ChannelAddress>, omni: bool) -> Self {
        Self { addresses, omni }
    }

    /// A mapping that matches no address at all.
    pub fn none() -> Self {
        Self {
            addresses: Vec::new(),
            omni: false,
        }
    }

    /// A mapping that matches every address (omni mode).
    pub fn omni() -> Self {
        Self {
            addresses: Vec::new(),
            omni: true,
        }
    }

    /// A mapping that matches exactly one address.
    pub fn single(address: ChannelAddress) -> Self {
        Self {
            addresses: vec![address],
            omni: false,
        }
    }

    /// Returns `true` if this mapping is in omni mode (matches every
    /// address regardless of the explicit list).
    pub fn is_omni(&self) -> bool {
        self.omni
    }

    /// Returns the explicit addresses this mapping lists.
    ///
    /// When `is_omni()` is `true` this list is informational only —
    /// [`matches`](Self::matches) still answers `true` for every address
    /// because omni mode overrides the explicit list.
    pub fn addresses(&self) -> &[ChannelAddress] {
        &self.addresses
    }

    /// Returns `true` if a `MidiEvent` carrying `address` should be
    /// dispatched to the patch owning this mapping.
    ///
    /// This is the dispatch predicate the channel-mapping invariant relies
    /// on: a `MidiEvent` reaches exactly the set of patches whose mapping
    /// matches its address. Layering (multiple patches matching the same
    /// address) is expected and intentional; this method only answers for
    /// a single mapping.
    pub fn matches(&self, address: &ChannelAddress) -> bool {
        self.omni || self.addresses.iter().any(|mapped| mapped.matches(address))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::channel_address::{MidiChannel, MidiGroup};

    fn address(channel: u8, group: u8) -> ChannelAddress {
        ChannelAddress::new(
            MidiChannel::try_new(channel).expect("valid channel"),
            MidiGroup::try_new(group).expect("valid group"),
        )
    }

    #[test]
    fn none_matches_nothing() {
        let mapping = ChannelMapping::none();
        assert!(!mapping.is_omni());
        assert!(mapping.addresses().is_empty());
        assert!(!mapping.matches(&address(0, 0)));
        assert!(!mapping.matches(&address(15, 15)));
    }

    #[test]
    fn omni_matches_every_address_even_with_empty_list() {
        let mapping = ChannelMapping::omni();
        assert!(mapping.is_omni());
        assert!(mapping.matches(&address(0, 0)));
        assert!(mapping.matches(&address(9, 3)));
        assert!(mapping.matches(&address(15, 15)));
    }

    #[test]
    fn single_matches_only_its_own_address() {
        let target = address(3, 1);
        let mapping = ChannelMapping::single(target);

        assert!(mapping.matches(&target));
        assert!(!mapping.matches(&address(4, 1)));
        assert!(!mapping.matches(&address(3, 2)));
    }

    #[test]
    fn new_matches_any_address_in_the_explicit_list() {
        let a = address(0, 0);
        let b = address(1, 0);
        let unlisted = address(2, 0);
        let mapping = ChannelMapping::new(vec![a, b], false);

        assert!(mapping.matches(&a));
        assert!(mapping.matches(&b));
        assert!(!mapping.matches(&unlisted));
    }

    #[test]
    fn omni_flag_overrides_a_non_matching_explicit_list() {
        let listed = address(5, 0);
        let mapping = ChannelMapping::new(vec![listed], true);

        // Omni matches even addresses absent from the explicit list.
        assert!(mapping.matches(&address(9, 9)));
        assert!(mapping.matches(&listed));
    }

    #[test]
    fn multiple_mappings_may_layer_on_the_same_address() {
        let shared = address(6, 2);
        let mapping_one = ChannelMapping::single(shared);
        let mapping_two = ChannelMapping::new(vec![shared, address(7, 2)], false);

        // Layering across independent patches' mappings is intentional:
        // both mappings independently match the shared address.
        assert!(mapping_one.matches(&shared));
        assert!(mapping_two.matches(&shared));
    }

    #[test]
    fn default_mapping_matches_nothing() {
        let mapping = ChannelMapping::default();
        assert!(!mapping.is_omni());
        assert!(!mapping.matches(&address(0, 0)));
    }

    #[test]
    fn equality_is_structural() {
        let a = ChannelMapping::new(vec![address(1, 0)], false);
        let b = ChannelMapping::new(vec![address(1, 0)], false);
        let c = ChannelMapping::new(vec![address(2, 0)], false);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
