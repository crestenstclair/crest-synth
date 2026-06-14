// path: src/patch/patch_repository.rs

//! In-memory repository for [`Patch`] aggregates.
//!
//! `PatchRepository` is the single source of truth for all patches in the
//! system.  It is intentionally kept off the audio thread — it lives in the
//! control plane and feeds snapshots to the real-time layer via
//! `ParameterBridge` or `EventRingBuffer`.
//!
//! # Contract
//!
//! | method | signature |
//! |---|---|
//! | `find_by_channel` | `ChannelAddress -> Vec<&Patch>` |
//! | `find_by_id`      | `PatchId -> Option<&Patch>` |
//! | `list_all`        | `() -> Vec<&Patch>` |
//! | `save`            | `Patch -> ()` |

use std::collections::HashMap;

use crate::patch::channel_subscription::ChannelAddress;
use crate::patch::patch::Patch;
use crate::patch::patch_id::PatchId;

// ─────────────────────────────────────────────────────────────────────────────
// Repository trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait interface for the patch store.
///
/// Keeping the concrete type behind a trait lets callers depend on the
/// abstraction — in particular, test code can substitute a mock or stub
/// without touching production types.
pub trait PatchRepositoryPort {
    /// Return all patches whose subscription matches the given channel address.
    ///
    /// Returns an empty `Vec` when no patch matches.
    fn find_by_channel(&self, address: ChannelAddress) -> Vec<&Patch>;

    /// Return the patch with the given id, or `None` if it does not exist.
    fn find_by_id(&self, id: PatchId) -> Option<&Patch>;

    /// Return all patches in insertion order.
    fn list_all(&self) -> Vec<&Patch>;

    /// Insert or replace the patch.
    ///
    /// If a patch with the same id already exists it is overwritten; otherwise
    /// the patch is appended.  Patches without an id (not yet created via the
    /// `CreatePatch` command) are silently ignored.
    fn save(&mut self, patch: Patch);
}

// ─────────────────────────────────────────────────────────────────────────────
// In-memory implementation
// ─────────────────────────────────────────────────────────────────────────────

/// In-memory implementation of the patch store.
///
/// Patches are stored in a [`HashMap`] keyed by `PatchId` for O(1) lookup, and
/// a separate `Vec<PatchId>` preserves insertion order for `list_all`.
///
/// This type lives **entirely on the control / UI thread** — it is not `Send`
/// and must never be accessed from the audio thread.
pub struct PatchRepository {
    patches: HashMap<PatchId, Patch>,
    order: Vec<PatchId>,
}

impl PatchRepository {
    /// Create an empty repository.
    pub fn new() -> Self {
        Self {
            patches: HashMap::new(),
            order: Vec::new(),
        }
    }
}

impl Default for PatchRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl PatchRepositoryPort for PatchRepository {
    /// Return all patches subscribed to the given channel address.
    ///
    /// Iterates patches in insertion order and returns references to those
    /// whose subscription's address matches the given `ChannelAddress`.
    fn find_by_channel(&self, address: ChannelAddress) -> Vec<&Patch> {
        self.order
            .iter()
            .filter_map(|id| self.patches.get(id))
            .filter(|p| p.subscription().matches(address.group(), address.channel()))
            .collect()
    }

    fn find_by_id(&self, id: PatchId) -> Option<&Patch> {
        self.patches.get(&id)
    }

    fn list_all(&self) -> Vec<&Patch> {
        self.order
            .iter()
            .filter_map(|id| self.patches.get(id))
            .collect()
    }

    fn save(&mut self, patch: Patch) {
        // Only persist patches that have been created (have an id).
        if let Some(id) = patch.id() {
            if !self.patches.contains_key(&id) {
                self.order.push(id);
            }
            self.patches.insert(id, patch);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_group::MidiGroup;
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

    fn addr(g: u8, c: u8) -> ChannelAddress {
        ChannelAddress::new(group(g), ch(c))
    }

    fn default_sub(g: u8, c: u8) -> ChannelSubscription {
        ChannelSubscription::new(addr(g, c), None)
    }

    fn default_vpc() -> VoicePoolConfig {
        VoicePoolConfig::try_new(4, StealingPolicy::QuietestFirst).unwrap()
    }

    /// Build and create a patch on channel (g, c) with the given name.
    fn make_patch(name: &str, g: u8, c: u8) -> Patch {
        let sub = default_sub(g, c);
        let mut p = Patch::new(default_vpc(), sub);
        p.handle(PatchCommand::CreatePatch {
            name: name.to_string(),
            engine_type: EngineType::Sine,
            subscription: sub,
        })
        .unwrap();
        p
    }

    // ── save / find_by_id ─────────────────────────────────────────────────────

    #[test]
    fn save_and_find_by_id() {
        let mut repo = PatchRepository::new();
        let patch = make_patch("Lead", 0, 1);
        let id = patch.id().unwrap();
        repo.save(patch);
        assert!(repo.find_by_id(id).is_some());
    }

    #[test]
    fn find_by_id_unknown_returns_none() {
        let repo = PatchRepository::new();
        assert!(repo.find_by_id(PatchId::new(9999)).is_none());
    }

    #[test]
    fn save_overwrites_existing() {
        let mut repo = PatchRepository::new();
        let patch = make_patch("Lead", 0, 1);
        let id = patch.id().unwrap();
        repo.save(patch);

        // Save a second patch that has the same id (simulating an update).
        let mut updated = make_patch("Lead", 0, 1); // same name → same id
        updated.handle(PatchCommand::ActivatePatch).unwrap();
        repo.save(updated);

        // Still only one entry in the store.
        assert_eq!(repo.list_all().len(), 1);
        assert!(repo.find_by_id(id).unwrap().active());
    }

    #[test]
    fn save_patch_without_id_is_ignored() {
        let mut repo = PatchRepository::new();
        // A patch constructed but not yet created has no id.
        let uninitialised = Patch::new(default_vpc(), default_sub(0, 0));
        repo.save(uninitialised);
        assert!(repo.list_all().is_empty());
    }

    // ── list_all ─────────────────────────────────────────────────────────────

    #[test]
    fn list_all_empty_repo() {
        let repo = PatchRepository::new();
        assert!(repo.list_all().is_empty());
    }

    #[test]
    fn list_all_returns_all_patches() {
        let mut repo = PatchRepository::new();
        repo.save(make_patch("Bass", 0, 0));
        repo.save(make_patch("Pad", 0, 1));
        assert_eq!(repo.list_all().len(), 2);
    }

    #[test]
    fn list_all_preserves_insertion_order() {
        let mut repo = PatchRepository::new();
        // Names must have distinct lengths so they get distinct PatchIds
        // (the aggregate computes id = name.len() ^ 0xDEAD_BEEF).
        let names = ["A", "AB", "ABC"];
        for (i, name) in names.iter().enumerate() {
            repo.save(make_patch(name, 0, i as u8));
        }
        let all = repo.list_all();
        assert_eq!(all.len(), 3);
        for (i, p) in all.iter().enumerate() {
            assert_eq!(p.name(), names[i]);
        }
    }

    // ── find_by_channel ───────────────────────────────────────────────────────

    #[test]
    fn find_by_channel_returns_matching_patches() {
        let mut repo = PatchRepository::new();
        // Use distinct-length names so the deterministic id (name.len() ^ constant)
        // differs between patches.
        repo.save(make_patch("Lead", 0, 3)); // len=4
        repo.save(make_patch("Melody", 0, 4)); // len=6

        let matches = repo.find_by_channel(addr(0, 3));
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name(), "Lead");
    }

    #[test]
    fn find_by_channel_no_match_returns_empty() {
        let mut repo = PatchRepository::new();
        repo.save(make_patch("Lead", 0, 1));

        let matches = repo.find_by_channel(addr(0, 9));
        assert!(matches.is_empty());
    }

    #[test]
    fn find_by_channel_returns_all_patches_on_same_channel() {
        // Two patches on the same channel address — both must be returned.
        // This verifies the invariant: channel dispatch delivers events to ALL
        // subscribed patches, not just the first match.
        let mut repo = PatchRepository::new();
        repo.save(make_patch("Layer1", 0, 2));
        repo.save(make_patch("Layer2x", 0, 2)); // different name → different id
        let matches = repo.find_by_channel(addr(0, 2));
        assert_eq!(matches.len(), 2, "both patches on ch 2 must be returned");
    }

    // ── Default ───────────────────────────────────────────────────────────────

    #[test]
    fn default_creates_empty_repo() {
        let repo = PatchRepository::default();
        assert!(repo.list_all().is_empty());
    }
}
