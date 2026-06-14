// path: src/patch/channel_dispatcher.rs

//! `ChannelDispatcher` domain service — routes incoming [`MidiEvent`]s to
//! every [`Patch`] whose [`ChannelSubscription`] matches the event's group and
//! channel.
//!
//! # Guarantees
//!
//! - **All subscribers receive every matching event.**  The dispatcher
//!   iterates over all registered patches and delivers to every match, not
//!   just the first.
//! - **Independent voice pools.**  Each `Patch` owns its own `VoiceAllocator`;
//!   the dispatcher never touches voice pools directly, so the polyphony of one
//!   patch can never deplete the voices of another.
//! - **Audio-thread safe.**  The dispatcher holds a pre-allocated `Vec<Patch>`
//!   built at initialisation time.  During dispatch it iterates without
//!   allocating, locking, or performing any I/O.

use crate::kernel::midi_event::MidiEvent;
use crate::patch::patch::Patch;

// ─────────────────────────────────────────────────────────────────────────────
// ChannelDispatcher
// ─────────────────────────────────────────────────────────────────────────────

/// Routes incoming [`MidiEvent`]s to every [`Patch`] whose
/// [`crate::patch::channel_subscription::ChannelSubscription`] matches the
/// event's group and channel.
///
/// # Audio-thread contract
///
/// `dispatch` must never allocate heap memory, acquire a mutex, or perform
/// blocking I/O.  All state is pre-allocated at construction time.
///
/// # Multi-patch dispatch
///
/// When two or more patches share the same channel subscription the dispatcher
/// delivers the event to **all** of them — enabling automatic layering.
pub struct ChannelDispatcher {
    /// Pre-allocated collection of patches managed by this dispatcher.
    patches: Vec<Patch>,
}

impl ChannelDispatcher {
    /// Create a new `ChannelDispatcher` with a pre-allocated capacity.
    ///
    /// `capacity` controls the initial allocation; no further heap allocation
    /// occurs while the number of registered patches stays within it.
    pub fn new(capacity: usize) -> Self {
        Self {
            patches: Vec::with_capacity(capacity),
        }
    }

    /// Create a `ChannelDispatcher` from an existing collection of patches.
    ///
    /// The patches are moved into the dispatcher; the underlying storage is
    /// reused without any additional allocation.
    pub fn from_patches(patches: Vec<Patch>) -> Self {
        Self { patches }
    }

    /// Register a `Patch` with this dispatcher.
    ///
    /// This method **may allocate** if the internal `Vec` needs to grow.
    /// Call it during the setup phase, not from the audio thread.
    pub fn register(&mut self, patch: Patch) {
        self.patches.push(patch);
    }

    /// Dispatch `event` to every patch whose subscription matches the event's
    /// group and channel.
    ///
    /// Returns the number of patches the event was delivered to.
    ///
    /// # Audio-thread safety
    ///
    /// This method performs no heap allocation, no locking, and no I/O.  It
    /// iterates the pre-allocated slice and calls the provided `deliver`
    /// closure for each matching patch.
    pub fn dispatch<F>(&mut self, event: &MidiEvent, mut deliver: F) -> usize
    where
        F: FnMut(&mut Patch, &MidiEvent),
    {
        let group = event.group;
        let channel = event.channel;
        let mut delivered = 0usize;
        for patch in &mut self.patches {
            if patch.subscription().matches(group, channel) {
                deliver(patch, event);
                delivered += 1;
            }
        }
        delivered
    }

    /// Return an immutable slice of all registered patches.
    pub fn patches(&self) -> &[Patch] {
        &self.patches
    }

    /// Return a mutable slice of all registered patches.
    pub fn patches_mut(&mut self) -> &mut [Patch] {
        &mut self.patches
    }

    /// Return the number of registered patches.
    pub fn len(&self) -> usize {
        self.patches.len()
    }

    /// Return `true` if no patches have been registered.
    pub fn is_empty(&self) -> bool {
        self.patches.is_empty()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_event::MidiEvent;
    use crate::kernel::midi_group::MidiGroup;
    use crate::kernel::note_id::NoteId;
    use crate::kernel::note_number::NoteNumber;
    use crate::kernel::velocity::Velocity;
    use crate::patch::channel_subscription::{ChannelAddress, ChannelSubscription};
    use crate::patch::patch::{EngineType, Patch, PatchCommand};
    use crate::patch::voice_pool_config::{StealingPolicy, VoicePoolConfig};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn group(v: u8) -> MidiGroup {
        MidiGroup::try_new(v).unwrap()
    }

    fn ch(v: u8) -> MidiChannel {
        MidiChannel::try_new(v).unwrap()
    }

    fn subscription(g: u8, c: u8) -> ChannelSubscription {
        ChannelSubscription::new(ChannelAddress::new(group(g), ch(c)), None)
    }

    fn make_patch(g: u8, c: u8, name: &str) -> Patch {
        let cfg = VoicePoolConfig::try_new(4, StealingPolicy::QuietestFirst).unwrap();
        let sub = subscription(g, c);
        let mut p = Patch::new(cfg, sub);
        p.handle(PatchCommand::CreatePatch {
            name: name.to_string(),
            engine_type: EngineType::Sine,
            subscription: sub,
        })
        .unwrap();
        p.handle(PatchCommand::ActivatePatch).unwrap();
        p
    }

    fn note_on_event(g: u8, c: u8) -> MidiEvent {
        MidiEvent::note_on(
            group(g),
            ch(c),
            NoteId::new(1),
            NoteNumber::try_new(60).unwrap(),
            Velocity::try_new(0.8).unwrap(),
        )
    }

    // ── Construction ──────────────────────────────────────────────────────────

    #[test]
    fn new_dispatcher_is_empty() {
        let d = ChannelDispatcher::new(8);
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
    }

    #[test]
    fn from_patches_stores_all_patches() {
        let patches = vec![make_patch(0, 0, "A"), make_patch(0, 1, "B")];
        let d = ChannelDispatcher::from_patches(patches);
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn register_increases_len() {
        let mut d = ChannelDispatcher::new(4);
        d.register(make_patch(0, 0, "Lead"));
        assert_eq!(d.len(), 1);
    }

    // ── Dispatch — basic routing ──────────────────────────────────────────────

    #[test]
    fn dispatch_delivers_to_matching_patch() {
        let mut d = ChannelDispatcher::new(4);
        d.register(make_patch(0, 3, "Lead"));
        let event = note_on_event(0, 3);
        let mut count = 0usize;
        d.dispatch(&event, |_patch, _ev| {
            count += 1;
        });
        assert_eq!(count, 1);
    }

    #[test]
    fn dispatch_does_not_deliver_to_non_matching_patch() {
        let mut d = ChannelDispatcher::new(4);
        d.register(make_patch(0, 5, "Pad")); // listening on ch 5
        let event = note_on_event(0, 3); // event on ch 3
        let mut count = 0usize;
        d.dispatch(&event, |_patch, _ev| {
            count += 1;
        });
        assert_eq!(count, 0);
    }

    #[test]
    fn dispatch_returns_zero_when_no_patches_match() {
        let mut d = ChannelDispatcher::new(4);
        d.register(make_patch(0, 7, "Bass"));
        let event = note_on_event(0, 1);
        let delivered = d.dispatch(&event, |_, _| {});
        assert_eq!(delivered, 0);
    }

    // ── Multi-patch dispatch (core invariant) ─────────────────────────────────

    #[test]
    fn dispatch_delivers_to_all_subscribed_patches_on_same_channel() {
        // Two patches on the same channel — both must receive the event.
        let mut d = ChannelDispatcher::new(4);
        d.register(make_patch(0, 4, "Layer A"));
        d.register(make_patch(0, 4, "Layer B"));
        let event = note_on_event(0, 4);
        let mut count = 0usize;
        d.dispatch(&event, |_patch, _ev| {
            count += 1;
        });
        assert_eq!(count, 2, "both patches on ch 4 must receive the event");
    }

    #[test]
    fn dispatch_delivers_to_three_patches_on_same_channel() {
        let mut d = ChannelDispatcher::new(8);
        d.register(make_patch(0, 2, "P1"));
        d.register(make_patch(0, 2, "P2"));
        d.register(make_patch(0, 2, "P3"));
        let event = note_on_event(0, 2);
        let delivered = d.dispatch(&event, |_, _| {});
        assert_eq!(delivered, 3);
    }

    #[test]
    fn dispatch_only_delivers_to_matching_patches_among_mixed_subscriptions() {
        let mut d = ChannelDispatcher::new(8);
        d.register(make_patch(0, 1, "Match A")); // matches
        d.register(make_patch(0, 2, "No Match")); // does not match
        d.register(make_patch(0, 1, "Match B")); // matches
        let event = note_on_event(0, 1);
        let delivered = d.dispatch(&event, |_, _| {});
        assert_eq!(delivered, 2);
    }

    // ── Independent voice pools ───────────────────────────────────────────────

    #[test]
    fn each_patch_has_independent_voice_pool() {
        let mut d = ChannelDispatcher::new(4);
        d.register(make_patch(0, 0, "A"));
        d.register(make_patch(0, 0, "B"));
        let patches = d.patches();
        assert_eq!(patches.len(), 2);
        // Addresses of voice pools must differ — they are separate heap allocations.
        assert_ne!(
            patches[0].voice_pool() as *const _,
            patches[1].voice_pool() as *const _,
            "each patch must own its own voice pool"
        );
    }

    // ── Group isolation ───────────────────────────────────────────────────────

    #[test]
    fn dispatch_respects_midi_group() {
        let mut d = ChannelDispatcher::new(4);
        d.register(make_patch(0, 5, "Group0")); // group 0, ch 5
                                                // event on group 1, ch 5 — different group, should not match
        let event = note_on_event(1, 5);
        let delivered = d.dispatch(&event, |_, _| {});
        assert_eq!(delivered, 0);
    }

    // ── patches() / patches_mut() accessors ──────────────────────────────────

    #[test]
    fn patches_slice_length_matches_registered_count() {
        let mut d = ChannelDispatcher::new(4);
        d.register(make_patch(0, 0, "X"));
        d.register(make_patch(0, 1, "Y"));
        assert_eq!(d.patches().len(), 2);
    }

    #[test]
    fn patches_mut_allows_mutation() {
        let mut d = ChannelDispatcher::new(4);
        d.register(make_patch(0, 0, "Mutable"));
        // Activating through patches_mut should work.
        let _ = d.patches_mut()[0].handle(PatchCommand::DeactivatePatch);
        assert!(!d.patches()[0].active());
    }
}
