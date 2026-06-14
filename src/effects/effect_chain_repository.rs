// path: src/effects/effect_chain_repository.rs

//! In-memory repository for [`EffectChain`] aggregates.
//!
//! `EffectChainRepository` is the single source of truth for all effect chains
//! in the control plane.  It lives off the audio thread and feeds snapshots to
//! the real-time layer via `ParameterBridge` or `EventRingBuffer`.
//!
//! # Contract
//!
//! | method        | signature                          |
//! |---------------|------------------------------------|
//! | `find_by_id`  | `EffectChainId -> Option<&EffectChain>` |
//! | `save`        | `EffectChain -> ()`                |

use std::collections::HashMap;

use crate::effects::effect_chain::EffectChain;
use crate::effects::effect_chain_id::EffectChainId;

// ─────────────────────────────────────────────────────────────────────────────
// Repository trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait interface for the effect-chain store.
///
/// Keeping the concrete type behind a trait lets callers depend on the
/// abstraction — test code can substitute a mock or stub without touching
/// production types (DIP / ISP).
pub trait EffectChainRepositoryPort {
    /// Return the effect chain with the given id, or `None` if it does not exist.
    fn find_by_id(&self, id: EffectChainId) -> Option<&EffectChain>;

    /// Insert or replace the effect chain.
    ///
    /// If a chain with the same id already exists it is overwritten; otherwise
    /// the chain is inserted.
    fn save(&mut self, chain: EffectChain);
}

// ─────────────────────────────────────────────────────────────────────────────
// In-memory implementation
// ─────────────────────────────────────────────────────────────────────────────

/// In-memory implementation of the effect-chain store.
///
/// Chains are stored in a [`HashMap`] keyed by `EffectChainId` for O(1) lookup,
/// and a separate `Vec<EffectChainId>` preserves insertion order.
///
/// This type lives **entirely on the control / UI thread** — it must never be
/// accessed from the audio thread.
pub struct EffectChainRepository {
    chains: HashMap<EffectChainId, EffectChain>,
    order: Vec<EffectChainId>,
}

impl EffectChainRepository {
    /// Create an empty repository.
    pub fn new() -> Self {
        Self {
            chains: HashMap::new(),
            order: Vec::new(),
        }
    }
}

impl Default for EffectChainRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectChainRepositoryPort for EffectChainRepository {
    fn find_by_id(&self, id: EffectChainId) -> Option<&EffectChain> {
        self.chains.get(&id)
    }

    fn save(&mut self, chain: EffectChain) {
        let id = chain.id();
        if !self.chains.contains_key(&id) {
            self.order.push(id);
        }
        self.chains.insert(id, chain);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::effect_chain::{AddEffect, BypassChain};
    use crate::effects::effect_processor::EffectType;

    fn make_chain(id: u32) -> EffectChain {
        EffectChain::new(EffectChainId::new(id))
    }

    // ── find_by_id ────────────────────────────────────────────────────────────

    #[test]
    fn effect_chain_repository_find_by_id_returns_saved_chain() {
        let mut repo = EffectChainRepository::new();
        let chain = make_chain(1);
        let id = chain.id();
        repo.save(chain);
        assert!(repo.find_by_id(id).is_some());
    }

    #[test]
    fn effect_chain_repository_find_by_id_unknown_returns_none() {
        let repo = EffectChainRepository::new();
        assert!(repo.find_by_id(EffectChainId::new(999)).is_none());
    }

    // ── save ──────────────────────────────────────────────────────────────────

    #[test]
    fn effect_chain_repository_save_overwrites_existing() {
        let mut repo = EffectChainRepository::new();
        let chain = make_chain(1);
        let id = chain.id();
        repo.save(chain);

        // Save a new chain with the same id (simulating an update).
        let mut updated = make_chain(1);
        updated.add_effect(AddEffect {
            effect_type: EffectType::Gain,
            position: 0,
        });
        repo.save(updated);

        // Only one entry in the store.
        assert_eq!(repo.chains.len(), 1);
        // The updated chain has one slot.
        assert_eq!(repo.find_by_id(id).unwrap().len(), 1);
    }

    #[test]
    fn effect_chain_repository_save_multiple_distinct_ids() {
        let mut repo = EffectChainRepository::new();
        repo.save(make_chain(1));
        repo.save(make_chain(2));
        repo.save(make_chain(3));
        assert_eq!(repo.chains.len(), 3);
    }

    // ── find_by_id reflects mutations ─────────────────────────────────────────

    #[test]
    fn effect_chain_repository_find_by_id_reflects_chain_state() {
        let mut repo = EffectChainRepository::new();
        let mut chain = make_chain(10);
        chain.add_effect(AddEffect {
            effect_type: EffectType::Delay,
            position: 0,
        });
        let id = chain.id();
        repo.save(chain);

        let stored = repo.find_by_id(id).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored.slots()[0].effect_type, EffectType::Delay);
    }

    #[test]
    fn effect_chain_repository_find_by_id_reflects_bypass_state() {
        let mut repo = EffectChainRepository::new();
        let mut chain = make_chain(5);
        chain.bypass_chain(BypassChain);
        let id = chain.id();
        repo.save(chain);

        let stored = repo.find_by_id(id).unwrap();
        assert!(stored.is_bypassed());
    }

    // ── Default ───────────────────────────────────────────────────────────────

    #[test]
    fn effect_chain_repository_default_creates_empty_repo() {
        let repo = EffectChainRepository::default();
        assert!(repo.find_by_id(EffectChainId::new(0)).is_none());
    }

    // ── trait object usage ────────────────────────────────────────────────────

    #[test]
    fn effect_chain_repository_trait_object_find_and_save() {
        let mut repo: Box<dyn EffectChainRepositoryPort> = Box::new(EffectChainRepository::new());
        let chain = make_chain(7);
        let id = chain.id();
        repo.save(chain);
        assert!(repo.find_by_id(id).is_some());
        assert!(repo.find_by_id(EffectChainId::new(0)).is_none());
    }
}
