// path: src/patch/midi_dispatcher.rs

//! MidiDispatcher domain service: routes each normalized MIDI address to
//! exactly the patches whose channel mapping matches it.
//!
//! Layering is intentional — a single address may match multiple patches'
//! `ChannelMapping`s and is dispatched to all of them. Leakage is not:
//! patches whose mapping does not match the address never receive it.
//! This type does no I/O and never touches the real-time audio thread
//! directly; it is a pure lookup used by whichever side needs to know
//! which patches a given MIDI channel reaches.

use std::fmt;

use crate::patch::patch::{ChannelMapping, PatchId};

/// A validated MIDI channel address (0-15) that an event targets.
///
/// This is the "address" referred to by the dispatch invariant: an event
/// is dispatched to exactly the set of patches whose `ChannelMapping`
/// matches this address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiAddress(u8);

impl MidiAddress {
    /// Builds a `MidiAddress` from a raw MIDI channel. `channel` must be
    /// within `0..=15`.
    pub fn try_new(channel: u8) -> Result<Self, MidiDispatchError> {
        if !(0..=15).contains(&channel) {
            return Err(MidiDispatchError::InvalidChannel(channel));
        }
        Ok(Self(channel))
    }

    /// The raw MIDI channel (0-15).
    pub fn channel(self) -> u8 {
        self.0
    }
}

/// Narrow read-only view onto a single patch's routing identity: its
/// stable id and the channel mapping used to test dispatch.
///
/// `MidiDispatcher` depends on this trait rather than on a concrete
/// `Patch` or `PatchManager` so it can be exercised against any
/// collection of patch-like things, including test fakes.
pub trait RoutablePatch {
    /// The patch's stable identity.
    fn id(&self) -> PatchId;

    /// The patch's current channel mapping.
    fn mapping(&self) -> ChannelMapping;
}

/// Errors produced while building a `MidiAddress`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiDispatchError {
    InvalidChannel(u8),
}

impl fmt::Display for MidiDispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MidiDispatchError::InvalidChannel(channel) => {
                write!(f, "invalid MIDI channel: {channel} (must be 0..=15)")
            }
        }
    }
}

impl std::error::Error for MidiDispatchError {}

/// Domain service that routes a `MidiAddress` to the patches whose
/// `ChannelMapping` matches it.
///
/// Stateless: it holds no patch collection of its own, so it has no
/// constructor dependencies to inject. Callers pass whichever slice of
/// `RoutablePatch` implementors is authoritative for that call, e.g. the
/// current set of `Patch` aggregates.
#[derive(Debug, Default, Clone, Copy)]
pub struct MidiDispatcher;

impl MidiDispatcher {
    /// Builds a dispatcher. Takes no dependencies: routing is a pure
    /// function of the address and the patches supplied at call time.
    pub fn new() -> Self {
        Self
    }

    /// Returns the ids of every patch in `patches` whose `ChannelMapping`
    /// matches `address`.
    ///
    /// Layering is intentional: zero, one, or many patches may match.
    /// Allocates only the returned `Vec` of matches, so this must be
    /// called off the real-time audio thread — patch dispatch is a
    /// non-RT concern whose result crosses the boundary like any other
    /// parameter change.
    pub fn dispatch<P>(&self, address: MidiAddress, patches: &[P]) -> Vec<PatchId>
    where
        P: RoutablePatch,
    {
        patches
            .iter()
            .filter(|patch| patch.mapping().matches(address.channel()))
            .map(RoutablePatch::id)
            .collect()
    }

    /// True if `address` matches at least one patch in `patches`.
    pub fn has_match<P>(&self, address: MidiAddress, patches: &[P]) -> bool
    where
        P: RoutablePatch,
    {
        patches
            .iter()
            .any(|patch| patch.mapping().matches(address.channel()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    struct FakePatch {
        id: PatchId,
        mapping: ChannelMapping,
    }

    impl RoutablePatch for FakePatch {
        fn id(&self) -> PatchId {
            self.id
        }

        fn mapping(&self) -> ChannelMapping {
            self.mapping
        }
    }

    fn patch(id: u32, mapping: ChannelMapping) -> FakePatch {
        FakePatch {
            id: PatchId::new(id),
            mapping,
        }
    }

    #[test]
    fn dispatches_to_single_matching_patch() {
        let dispatcher = MidiDispatcher::new();
        let address = MidiAddress::try_new(3).unwrap();
        let patches = [
            patch(1, ChannelMapping::single(3).unwrap()),
            patch(2, ChannelMapping::single(4).unwrap()),
        ];

        let matched = dispatcher.dispatch(address, &patches);

        assert_eq!(matched, vec![PatchId::new(1)]);
    }

    #[test]
    fn dispatches_to_every_layered_patch_that_matches() {
        let dispatcher = MidiDispatcher::new();
        let address = MidiAddress::try_new(5).unwrap();
        let patches = [
            patch(1, ChannelMapping::single(5).unwrap()),
            patch(2, ChannelMapping::omni()),
            patch(3, ChannelMapping::single(6).unwrap()),
        ];

        let matched = dispatcher.dispatch(address, &patches);

        assert_eq!(matched, vec![PatchId::new(1), PatchId::new(2)]);
    }

    #[test]
    fn does_not_leak_to_non_matching_patches() {
        let dispatcher = MidiDispatcher::new();
        let address = MidiAddress::try_new(7).unwrap();
        let patches = [patch(1, ChannelMapping::single(8).unwrap())];

        let matched = dispatcher.dispatch(address, &patches);

        assert!(matched.is_empty());
    }

    #[test]
    fn empty_patch_set_yields_no_dispatch() {
        let dispatcher = MidiDispatcher::new();
        let address = MidiAddress::try_new(0).unwrap();
        let patches: [FakePatch; 0] = [];

        assert!(dispatcher.dispatch(address, &patches).is_empty());
        assert!(!dispatcher.has_match(address, &patches));
    }

    #[test]
    fn rejects_channel_above_fifteen() {
        let err = MidiAddress::try_new(16).unwrap_err();
        assert_eq!(err, MidiDispatchError::InvalidChannel(16));
    }

    #[test]
    fn accepts_channel_at_upper_bound() {
        let address = MidiAddress::try_new(15).unwrap();
        assert_eq!(address.channel(), 15);
    }

    #[test]
    fn has_match_true_when_any_patch_matches() {
        let dispatcher = MidiDispatcher::new();
        let address = MidiAddress::try_new(2).unwrap();
        let patches = [
            patch(1, ChannelMapping::single(1).unwrap()),
            patch(2, ChannelMapping::single(2).unwrap()),
        ];

        assert!(dispatcher.has_match(address, &patches));
    }
}
