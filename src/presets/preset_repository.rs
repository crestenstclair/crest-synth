// path: src/presets/preset_repository.rs

//! In-memory repository for [`Preset`] aggregates.
//!
//! `PresetRepository` is the single source of truth for all presets in the
//! control plane. It lives entirely off the audio thread — presets are created,
//! modified, and searched in the UI/control layer and never accessed by the
//! real-time renderer directly.
//!
//! # Contract
//!
//! | method              | signature                          |
//! |---------------------|------------------------------------||
//! | `find_by_category`  | `string -> Vec<Preset>`            |
//! | `find_by_id`        | `PresetId -> Option<Preset>`       |
//! | `list_all`          | `() -> Vec<Preset>`                |
//! | `save`              | `Preset -> ()`                     |
//! | `search`            | `string -> Vec<Preset>`            |

use std::collections::HashMap;

use crate::presets::preset::Preset;
use crate::presets::preset_id::PresetId;

// ─────────────────────────────────────────────────────────────────────────────
// Repository trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait interface for the preset store.
///
/// Keeping the concrete type behind a trait lets callers depend on the
/// abstraction — test code can substitute a mock or stub without touching
/// production types.
pub trait PresetRepositoryPort {
    /// Return all presets in the given category (exact, case-insensitive match).
    ///
    /// Returns an empty `Vec` when no preset matches.
    fn find_by_category(&self, category: &str) -> Vec<Preset>;

    /// Return the preset with the given id, or `None` if it does not exist.
    fn find_by_id(&self, id: &PresetId) -> Option<Preset>;

    /// Return all presets in insertion order.
    fn list_all(&self) -> Vec<Preset>;

    /// Insert or replace the preset.
    ///
    /// If a preset with the same id already exists it is overwritten; otherwise
    /// the preset is appended in insertion order.
    fn save(&mut self, preset: Preset);

    /// Search presets by name, author, category, or tags (case-insensitive substring).
    ///
    /// Returns an empty `Vec` when no preset matches the query.
    fn search(&self, query: &str) -> Vec<Preset>;
}

// ─────────────────────────────────────────────────────────────────────────────
// In-memory implementation
// ─────────────────────────────────────────────────────────────────────────────

/// In-memory implementation of the preset store.
///
/// Presets are stored in a [`HashMap`] keyed by `PresetId` for O(1) lookup,
/// and a separate `Vec<PresetId>` preserves insertion order for `list_all`.
///
/// This type lives **entirely on the control / UI thread** — it must never be
/// accessed from the audio thread.
pub struct PresetRepository {
    presets: HashMap<PresetId, Preset>,
    order: Vec<PresetId>,
}

impl PresetRepository {
    /// Create an empty repository.
    pub fn new() -> Self {
        Self {
            presets: HashMap::new(),
            order: Vec::new(),
        }
    }
}

impl Default for PresetRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl PresetRepositoryPort for PresetRepository {
    /// Return all presets whose `metadata.category` matches `category`
    /// (case-insensitive exact match).
    fn find_by_category(&self, category: &str) -> Vec<Preset> {
        let lower = category.to_lowercase();
        self.order
            .iter()
            .filter_map(|id| self.presets.get(id))
            .filter(|p| p.metadata.category.to_lowercase() == lower)
            .cloned()
            .collect()
    }

    fn find_by_id(&self, id: &PresetId) -> Option<Preset> {
        self.presets.get(id).cloned()
    }

    fn list_all(&self) -> Vec<Preset> {
        self.order
            .iter()
            .filter_map(|id| self.presets.get(id))
            .cloned()
            .collect()
    }

    fn save(&mut self, preset: Preset) {
        let id = preset.id.clone();
        if !self.presets.contains_key(&id) {
            self.order.push(id.clone());
        }
        self.presets.insert(id, preset);
    }

    /// Search presets by name, author, category, or tags.
    ///
    /// A preset matches if the lower-cased `query` is a substring of the
    /// lower-cased name, author, category, or any tag.
    fn search(&self, query: &str) -> Vec<Preset> {
        let lower = query.to_lowercase();
        self.order
            .iter()
            .filter_map(|id| self.presets.get(id))
            .filter(|p| {
                let meta = &p.metadata;
                meta.name.to_lowercase().contains(&lower)
                    || meta.author.to_lowercase().contains(&lower)
                    || meta.category.to_lowercase().contains(&lower)
                    || meta.tags.iter().any(|t| t.to_lowercase().contains(&lower))
            })
            .cloned()
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::preset::Preset;
    use crate::presets::preset_id::PresetId;
    use crate::presets::preset_metadata::PresetMetadata;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_preset(id: &str, name: &str, category: &str) -> Preset {
        let mut p = Preset::default_for(id, name);
        p.metadata = PresetMetadata::new(name, "Author", category, "2025-01-01T00:00:00Z", vec![]);
        p
    }

    fn make_preset_with_tags(id: &str, name: &str, category: &str, tags: Vec<&str>) -> Preset {
        let mut p = Preset::default_for(id, name);
        p.metadata = PresetMetadata::new(
            name,
            "Author",
            category,
            "2025-01-01T00:00:00Z",
            tags.into_iter().map(|t| t.to_string()).collect(),
        );
        p
    }

    fn make_preset_with_author(id: &str, name: &str, author: &str, category: &str) -> Preset {
        let mut p = Preset::default_for(id, name);
        p.metadata = PresetMetadata::new(name, author, category, "2025-01-01T00:00:00Z", vec![]);
        p
    }

    // ── new / default ─────────────────────────────────────────────────────────

    #[test]
    fn preset_repository_new_is_empty() {
        let repo = PresetRepository::new();
        assert!(repo.list_all().is_empty());
    }

    #[test]
    fn preset_repository_default_is_empty() {
        let repo = PresetRepository::default();
        assert!(repo.list_all().is_empty());
    }

    // ── save / find_by_id ─────────────────────────────────────────────────────

    #[test]
    fn preset_repository_save_and_find_by_id() {
        let mut repo = PresetRepository::new();
        let preset = make_preset("pad-001", "Warm Pad", "Pad");
        let id = preset.id.clone();
        repo.save(preset.clone());
        let found = repo.find_by_id(&id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, id);
    }

    #[test]
    fn preset_repository_find_by_id_unknown_returns_none() {
        let repo = PresetRepository::new();
        let missing = PresetId::new("does-not-exist");
        assert!(repo.find_by_id(&missing).is_none());
    }

    #[test]
    fn preset_repository_save_overwrites_existing() {
        let mut repo = PresetRepository::new();
        let preset = make_preset("bass-001", "Bass One", "Bass");
        repo.save(preset);

        let mut updated = make_preset("bass-001", "Bass One Updated", "Bass");
        updated.id = PresetId::new("bass-001");
        repo.save(updated);

        assert_eq!(repo.list_all().len(), 1);
        let found = repo.find_by_id(&PresetId::new("bass-001")).unwrap();
        assert_eq!(found.metadata.name, "Bass One Updated");
    }

    // ── list_all ─────────────────────────────────────────────────────────────

    #[test]
    fn preset_repository_list_all_empty() {
        let repo = PresetRepository::new();
        assert!(repo.list_all().is_empty());
    }

    #[test]
    fn preset_repository_list_all_returns_all_presets() {
        let mut repo = PresetRepository::new();
        repo.save(make_preset("p1", "Preset 1", "Lead"));
        repo.save(make_preset("p2", "Preset 2", "Pad"));
        assert_eq!(repo.list_all().len(), 2);
    }

    #[test]
    fn preset_repository_list_all_preserves_insertion_order() {
        let mut repo = PresetRepository::new();
        let names = ["Alpha", "Beta", "Gamma"];
        for (i, name) in names.iter().enumerate() {
            repo.save(make_preset(&format!("id-{i}"), name, "Other"));
        }
        let all = repo.list_all();
        assert_eq!(all.len(), 3);
        for (i, p) in all.iter().enumerate() {
            assert_eq!(p.metadata.name, names[i]);
        }
    }

    // ── find_by_category ─────────────────────────────────────────────────────

    #[test]
    fn preset_repository_find_by_category_returns_matching() {
        let mut repo = PresetRepository::new();
        repo.save(make_preset("p1", "Warm Pad", "Pad"));
        repo.save(make_preset("p2", "Bright Lead", "Lead"));
        repo.save(make_preset("p3", "Deep Pad", "Pad"));

        let pads = repo.find_by_category("Pad");
        assert_eq!(pads.len(), 2);
        assert!(pads.iter().all(|p| p.metadata.category == "Pad"));
    }

    #[test]
    fn preset_repository_find_by_category_case_insensitive() {
        let mut repo = PresetRepository::new();
        repo.save(make_preset("p1", "Warm Pad", "Pad"));
        let pads = repo.find_by_category("pad");
        assert_eq!(pads.len(), 1);
    }

    #[test]
    fn preset_repository_find_by_category_no_match_returns_empty() {
        let mut repo = PresetRepository::new();
        repo.save(make_preset("p1", "Warm Pad", "Pad"));
        let leads = repo.find_by_category("Lead");
        assert!(leads.is_empty());
    }

    #[test]
    fn preset_repository_find_by_category_empty_repo_returns_empty() {
        let repo = PresetRepository::new();
        assert!(repo.find_by_category("Lead").is_empty());
    }

    // ── search ────────────────────────────────────────────────────────────────

    #[test]
    fn preset_repository_search_by_name_substring() {
        let mut repo = PresetRepository::new();
        repo.save(make_preset("p1", "Warm Pad", "Pad"));
        repo.save(make_preset("p2", "Bright Lead", "Lead"));

        let results = repo.search("warm");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.name, "Warm Pad");
    }

    #[test]
    fn preset_repository_search_case_insensitive() {
        let mut repo = PresetRepository::new();
        repo.save(make_preset("p1", "Warm Pad", "Pad"));

        let results = repo.search("WARM");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn preset_repository_search_by_author() {
        let mut repo = PresetRepository::new();
        repo.save(make_preset_with_author("p1", "Warm Pad", "Alice", "Pad"));
        repo.save(make_preset_with_author("p2", "Bass Line", "Bob", "Bass"));

        let results = repo.search("alice");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.name, "Warm Pad");
    }

    #[test]
    fn preset_repository_search_by_category() {
        let mut repo = PresetRepository::new();
        repo.save(make_preset("p1", "Warm Pad", "Pad"));
        repo.save(make_preset("p2", "Bright Lead", "Lead"));

        let results = repo.search("lead");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.name, "Bright Lead");
    }

    #[test]
    fn preset_repository_search_by_tag() {
        let mut repo = PresetRepository::new();
        repo.save(make_preset_with_tags(
            "p1",
            "Warm Pad",
            "Pad",
            vec!["ambient", "soft"],
        ));
        repo.save(make_preset_with_tags(
            "p2",
            "Bright Lead",
            "Lead",
            vec!["bright", "cutting"],
        ));

        let results = repo.search("ambient");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.name, "Warm Pad");
    }

    #[test]
    fn preset_repository_search_no_match_returns_empty() {
        let mut repo = PresetRepository::new();
        repo.save(make_preset("p1", "Warm Pad", "Pad"));

        let results = repo.search("xyzzy");
        assert!(results.is_empty());
    }

    #[test]
    fn preset_repository_search_empty_repo_returns_empty() {
        let repo = PresetRepository::new();
        assert!(repo.search("anything").is_empty());
    }

    #[test]
    fn preset_repository_search_matches_multiple_presets() {
        let mut repo = PresetRepository::new();
        repo.save(make_preset("p1", "Pad One", "Pad"));
        repo.save(make_preset("p2", "Pad Two", "Pad"));
        repo.save(make_preset("p3", "Lead Three", "Lead"));

        let results = repo.search("pad");
        assert_eq!(results.len(), 2);
    }
}
