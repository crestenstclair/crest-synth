// path: src/presets/preset_browser.rs

//! PresetBrowser application service — lists, searches, and previews presets
//! from all banks.
//!
//! The `PresetBrowser` is a pure read-side application service: it holds
//! references to the registered banks and presets and provides query operations
//! for the UI layer. No audio-thread concerns, no locks on the render path.
//!
//! # Design
//!
//! - Accepts banks and presets at construction time (dependency injection).
//! - All queries return owned `Vec` results — no borrowed iterators that would
//!   force lifetime coupling to the service.
//! - Search is case-insensitive substring match across name, category, author,
//!   and tags.
//! - Preview is a pure read: returns `Option<&Preset>` for the caller to
//!   inspect without side effects.

use crate::presets::preset::Preset;
use crate::presets::preset_bank::PresetBank;
use crate::presets::preset_id::PresetId;

// ── BrowserEntry ─────────────────────────────────────────────────────────────

/// A single row in the preset browser: the preset's id, name, category, author,
/// tags, and the name of the bank it belongs to.
///
/// This is a lightweight value type used by the UI to populate browser lists
/// without handing out references to the full `Preset` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserEntry {
    /// Unique identifier for the preset.
    pub preset_id: PresetId,
    /// Human-readable preset name.
    pub name: String,
    /// Broad sound category (e.g. "Lead", "Pad", "Bass").
    pub category: String,
    /// Author / creator name.
    pub author: String,
    /// Tags for this preset.
    pub tags: Vec<String>,
    /// Name of the bank this entry came from.
    pub bank_name: String,
    /// Whether the bank is a factory (read-only) bank.
    pub is_factory: bool,
}

// ── SearchQuery ───────────────────────────────────────────────────────────────

/// Parameters for a preset search.
///
/// All fields are optional; an empty `SearchQuery` returns all presets across
/// all banks.
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    /// Case-insensitive substring matched against name, category, author, or
    /// any tag. `None` (or empty string) matches everything.
    pub text: Option<String>,
    /// If `Some`, restrict results to this category (case-insensitive).
    pub category: Option<String>,
    /// If `Some`, restrict results to this author (case-insensitive).
    pub author: Option<String>,
    /// If `Some`, restrict results to a specific bank name (case-insensitive).
    pub bank_name: Option<String>,
}

impl SearchQuery {
    /// Returns a query that matches everything.
    pub fn all() -> Self {
        Self::default()
    }

    /// Returns a query that filters by a text substring.
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    /// Returns a query that filters by category.
    pub fn with_category(category: impl Into<String>) -> Self {
        Self {
            category: Some(category.into()),
            ..Default::default()
        }
    }
}

// ── PresetBrowser ─────────────────────────────────────────────────────────────

/// Application service for listing, searching, and previewing presets across
/// all registered banks.
///
/// The browser holds a snapshot of banks and presets. In a real application the
/// caller rebuilds or updates the browser when the preset collection changes;
/// the browser itself is stateless beyond its inputs.
///
/// # Construction
///
/// ```
/// use crest_synth::presets::preset_browser::PresetBrowser;
/// use crest_synth::presets::preset_bank::PresetBank;
/// use crest_synth::presets::preset::Preset;
///
/// let browser = PresetBrowser::new(vec![], vec![]);
/// assert_eq!(browser.list_all().len(), 0);
/// ```
pub struct PresetBrowser {
    banks: Vec<PresetBank>,
    presets: Vec<Preset>,
}

impl PresetBrowser {
    /// Creates a new `PresetBrowser` from a list of banks and presets.
    ///
    /// # Arguments
    ///
    /// * `banks`   — All preset banks to expose through the browser.
    /// * `presets` — All presets to expose through the browser.
    ///
    /// The service does not take ownership of any audio primitives; both
    /// arguments are plain heap data cloned or moved by the caller.
    pub fn new(banks: Vec<PresetBank>, presets: Vec<Preset>) -> Self {
        Self { banks, presets }
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Returns all presets across all banks as `BrowserEntry` rows, in bank
    /// order then preset-id order within each bank.
    ///
    /// A preset that does not appear in any bank is not listed.
    ///
    /// # Example
    ///
    /// ```
    /// use crest_synth::presets::preset_browser::PresetBrowser;
    /// use crest_synth::presets::preset_bank::{PresetBank, CreateBank, AddPresetToBank};
    /// use crest_synth::presets::preset::Preset;
    /// use crest_synth::presets::preset_id::PresetId;
    ///
    /// let (mut bank, _) = PresetBank::create(CreateBank { name: "My Bank".to_string() }).unwrap();
    /// bank.add_preset(AddPresetToBank { preset_id: PresetId::new("pad-001") }).unwrap();
    ///
    /// let preset = Preset::default_for("pad-001", "Warm Pad");
    /// let browser = PresetBrowser::new(vec![bank], vec![preset]);
    /// assert_eq!(browser.list_all().len(), 1);
    /// ```
    pub fn list_all(&self) -> Vec<BrowserEntry> {
        self.collect_entries(&SearchQuery::all())
    }

    /// Returns all distinct category names across all presets, sorted
    /// alphabetically.
    ///
    /// Useful for populating a category filter dropdown.
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self
            .presets
            .iter()
            .map(|p| p.metadata.category.clone())
            .filter(|c| !c.is_empty())
            .collect();
        cats.sort();
        cats.dedup();
        cats
    }

    /// Returns all distinct bank names, in registration order.
    pub fn bank_names(&self) -> Vec<String> {
        self.banks.iter().map(|b| b.name.clone()).collect()
    }

    /// Searches presets according to `query` and returns matching entries.
    ///
    /// Matching rules:
    /// - `text` is a case-insensitive substring checked against name, category,
    ///   author, and any tag.
    /// - `category` is a case-insensitive exact match of
    ///   `entry.category`.
    /// - `author` is a case-insensitive exact match of `entry.author`.
    /// - `bank_name` is a case-insensitive exact match of `entry.bank_name`.
    ///
    /// All non-`None` fields of the query are ANDed together.
    ///
    /// # Example
    ///
    /// ```
    /// use crest_synth::presets::preset_browser::{PresetBrowser, SearchQuery};
    /// use crest_synth::presets::preset_bank::{PresetBank, CreateBank, AddPresetToBank};
    /// use crest_synth::presets::preset::Preset;
    /// use crest_synth::presets::preset_id::PresetId;
    /// use crest_synth::presets::preset_metadata::PresetMetadata;
    ///
    /// let (mut bank, _) = PresetBank::create(CreateBank { name: "Pads".to_string() }).unwrap();
    /// bank.add_preset(AddPresetToBank { preset_id: PresetId::new("pad-001") }).unwrap();
    ///
    /// let mut preset = Preset::default_for("pad-001", "Warm Pad");
    /// preset.metadata = PresetMetadata::new("Warm Pad", "Alice", "Pad", "", vec!["warm".to_string()]);
    ///
    /// let browser = PresetBrowser::new(vec![bank], vec![preset]);
    /// let results = browser.search(&SearchQuery::with_text("warm"));
    /// assert_eq!(results.len(), 1);
    /// assert_eq!(results[0].name, "Warm Pad");
    /// ```
    pub fn search(&self, query: &SearchQuery) -> Vec<BrowserEntry> {
        self.collect_entries(query)
    }

    /// Returns an immutable reference to a preset by id for previewing.
    ///
    /// Returns `None` if no preset with that id is registered.
    ///
    /// This is a pure read; no side effects occur.
    ///
    /// # Example
    ///
    /// ```
    /// use crest_synth::presets::preset_browser::PresetBrowser;
    /// use crest_synth::presets::preset::Preset;
    /// use crest_synth::presets::preset_id::PresetId;
    ///
    /// let preset = Preset::default_for("lead-001", "Bright Lead");
    /// let browser = PresetBrowser::new(vec![], vec![preset]);
    /// assert!(browser.preview(&PresetId::new("lead-001")).is_some());
    /// assert!(browser.preview(&PresetId::new("no-such")).is_none());
    /// ```
    pub fn preview(&self, preset_id: &PresetId) -> Option<&Preset> {
        self.presets.iter().find(|p| &p.id == preset_id)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn collect_entries(&self, query: &SearchQuery) -> Vec<BrowserEntry> {
        let mut entries = Vec::new();

        for bank in &self.banks {
            for pid in &bank.preset_ids {
                if let Some(preset) = self.presets.iter().find(|p| &p.id == pid) {
                    let entry = BrowserEntry {
                        preset_id: preset.id.clone(),
                        name: preset.metadata.name.clone(),
                        category: preset.metadata.category.clone(),
                        author: preset.metadata.author.clone(),
                        tags: preset.metadata.tags.clone(),
                        bank_name: bank.name.clone(),
                        is_factory: bank.is_factory,
                    };
                    if self.matches_query(&entry, query) {
                        entries.push(entry);
                    }
                }
            }
        }

        entries
    }

    fn matches_query(&self, entry: &BrowserEntry, query: &SearchQuery) -> bool {
        // text: case-insensitive substring across name, category, author, tags
        if let Some(text) = &query.text {
            if !text.is_empty() {
                let lower = text.to_lowercase();
                let in_name = entry.name.to_lowercase().contains(&lower);
                let in_category = entry.category.to_lowercase().contains(&lower);
                let in_author = entry.author.to_lowercase().contains(&lower);
                let in_tags = entry.tags.iter().any(|t| t.to_lowercase().contains(&lower));
                if !(in_name || in_category || in_author || in_tags) {
                    return false;
                }
            }
        }

        // category: case-insensitive exact match
        if let Some(cat) = &query.category {
            if !cat.is_empty() && entry.category.to_lowercase() != cat.to_lowercase() {
                return false;
            }
        }

        // author: case-insensitive exact match
        if let Some(author) = &query.author {
            if !author.is_empty() && entry.author.to_lowercase() != author.to_lowercase() {
                return false;
            }
        }

        // bank_name: case-insensitive exact match
        if let Some(bank_name) = &query.bank_name {
            if !bank_name.is_empty() && entry.bank_name.to_lowercase() != bank_name.to_lowercase() {
                return false;
            }
        }

        true
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod preset_browser_tests {
    use super::*;
    use crate::presets::preset::Preset;
    use crate::presets::preset_bank::{AddPresetToBank, CreateBank, PresetBank};
    use crate::presets::preset_id::PresetId;
    use crate::presets::preset_metadata::PresetMetadata;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn make_preset(id: &str, name: &str, category: &str, author: &str, tags: &[&str]) -> Preset {
        let mut p = Preset::default_for(id, name);
        p.metadata = PresetMetadata::new(
            name,
            author,
            category,
            "",
            tags.iter().map(|t| t.to_string()).collect(),
        );
        p
    }

    fn user_bank_with(name: &str, ids: &[&str]) -> PresetBank {
        let (mut bank, _) = PresetBank::create(CreateBank {
            name: name.to_string(),
        })
        .unwrap();
        for id in ids {
            bank.add_preset(AddPresetToBank {
                preset_id: PresetId::new(*id),
            })
            .unwrap();
        }
        bank
    }

    fn factory_bank_with(name: &str, ids: &[&str]) -> PresetBank {
        PresetBank::create_factory(name, ids.iter().map(|id| PresetId::new(*id)).collect())
    }

    // ── PresetBrowser::new ────────────────────────────────────────────────────

    #[test]
    fn new_with_empty_inputs() {
        let b = PresetBrowser::new(vec![], vec![]);
        assert!(b.list_all().is_empty());
    }

    // ── list_all ──────────────────────────────────────────────────────────────

    #[test]
    fn list_all_returns_entries_for_bank_presets() {
        let bank = user_bank_with("My Bank", &["pad-001", "pad-002"]);
        let presets = vec![
            make_preset("pad-001", "Warm Pad", "Pad", "Alice", &["warm"]),
            make_preset("pad-002", "Dark Pad", "Pad", "Bob", &["dark"]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        let entries = b.list_all();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].preset_id, PresetId::new("pad-001"));
        assert_eq!(entries[1].preset_id, PresetId::new("pad-002"));
    }

    #[test]
    fn list_all_omits_presets_not_in_any_bank() {
        let bank = user_bank_with("My Bank", &["pad-001"]);
        let presets = vec![
            make_preset("pad-001", "Warm Pad", "Pad", "Alice", &[]),
            make_preset("pad-999", "Orphan", "Other", "Nobody", &[]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        let entries = b.list_all();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].preset_id, PresetId::new("pad-001"));
    }

    #[test]
    fn list_all_preserves_bank_order_then_preset_order() {
        let bank_a = user_bank_with("A", &["p1", "p2"]);
        let bank_b = user_bank_with("B", &["p3"]);
        let presets = vec![
            make_preset("p1", "P1", "X", "A", &[]),
            make_preset("p2", "P2", "X", "A", &[]),
            make_preset("p3", "P3", "X", "B", &[]),
        ];
        let b = PresetBrowser::new(vec![bank_a, bank_b], presets);
        let entries = b.list_all();
        assert_eq!(entries[0].bank_name, "A");
        assert_eq!(entries[1].bank_name, "A");
        assert_eq!(entries[2].bank_name, "B");
    }

    #[test]
    fn list_all_marks_factory_bank_entries() {
        let bank = factory_bank_with("Factory", &["p1"]);
        let presets = vec![make_preset("p1", "Factory Sound", "Lead", "Factory", &[])];
        let b = PresetBrowser::new(vec![bank], presets);
        let entries = b.list_all();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_factory);
    }

    #[test]
    fn list_all_user_bank_entries_not_factory() {
        let bank = user_bank_with("User", &["p1"]);
        let presets = vec![make_preset("p1", "My Sound", "Lead", "Me", &[])];
        let b = PresetBrowser::new(vec![bank], presets);
        let entries = b.list_all();
        assert!(!entries[0].is_factory);
    }

    // ── categories ───────────────────────────────────────────────────────────

    #[test]
    fn categories_returns_sorted_unique_categories() {
        let bank = user_bank_with("B", &["p1", "p2", "p3"]);
        let presets = vec![
            make_preset("p1", "A", "Pad", "X", &[]),
            make_preset("p2", "B", "Bass", "X", &[]),
            make_preset("p3", "C", "Pad", "X", &[]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        let cats = b.categories();
        // sorted and deduped
        assert_eq!(cats, vec!["Bass", "Pad"]);
    }

    #[test]
    fn categories_excludes_empty_category_strings() {
        let bank = user_bank_with("B", &["p1"]);
        let presets = vec![make_preset("p1", "A", "", "X", &[])];
        let b = PresetBrowser::new(vec![bank], presets);
        assert!(b.categories().is_empty());
    }

    // ── bank_names ───────────────────────────────────────────────────────────

    #[test]
    fn bank_names_returns_in_registration_order() {
        let b = PresetBrowser::new(
            vec![
                user_bank_with("Pads", &[]),
                user_bank_with("Leads", &[]),
                factory_bank_with("Factory", &[]),
            ],
            vec![],
        );
        assert_eq!(b.bank_names(), vec!["Pads", "Leads", "Factory"]);
    }

    // ── search ────────────────────────────────────────────────────────────────

    #[test]
    fn search_all_query_returns_all_entries() {
        let bank = user_bank_with("B", &["p1", "p2"]);
        let presets = vec![
            make_preset("p1", "A", "Lead", "X", &[]),
            make_preset("p2", "B", "Pad", "Y", &[]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        assert_eq!(b.search(&SearchQuery::all()).len(), 2);
    }

    #[test]
    fn search_by_text_matches_name() {
        let bank = user_bank_with("B", &["p1", "p2"]);
        let presets = vec![
            make_preset("p1", "Warm Lead", "Lead", "X", &[]),
            make_preset("p2", "Cold Pad", "Pad", "Y", &[]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        let results = b.search(&SearchQuery::with_text("warm"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Warm Lead");
    }

    #[test]
    fn search_by_text_is_case_insensitive() {
        let bank = user_bank_with("B", &["p1"]);
        let presets = vec![make_preset("p1", "Bright Lead", "Lead", "X", &[])];
        let b = PresetBrowser::new(vec![bank], presets);
        // Search uppercase
        assert_eq!(b.search(&SearchQuery::with_text("BRIGHT")).len(), 1);
        // Search mixed case
        assert_eq!(b.search(&SearchQuery::with_text("bRiGhT")).len(), 1);
    }

    #[test]
    fn search_by_text_matches_category() {
        let bank = user_bank_with("B", &["p1", "p2"]);
        let presets = vec![
            make_preset("p1", "A", "Synth Pad", "X", &[]),
            make_preset("p2", "B", "Bass", "Y", &[]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        let results = b.search(&SearchQuery::with_text("synth"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].category, "Synth Pad");
    }

    #[test]
    fn search_by_text_matches_author() {
        let bank = user_bank_with("B", &["p1", "p2"]);
        let presets = vec![
            make_preset("p1", "A", "Lead", "Alice", &[]),
            make_preset("p2", "B", "Pad", "Bob", &[]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        let results = b.search(&SearchQuery::with_text("alice"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].author, "Alice");
    }

    #[test]
    fn search_by_text_matches_tag() {
        let bank = user_bank_with("B", &["p1", "p2"]);
        let presets = vec![
            make_preset("p1", "A", "Lead", "X", &["bright", "punchy"]),
            make_preset("p2", "B", "Pad", "Y", &["warm", "lush"]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        let results = b.search(&SearchQuery::with_text("punchy"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].preset_id, PresetId::new("p1"));
    }

    #[test]
    fn search_by_text_no_match_returns_empty() {
        let bank = user_bank_with("B", &["p1"]);
        let presets = vec![make_preset("p1", "Warm Pad", "Pad", "Alice", &["warm"])];
        let b = PresetBrowser::new(vec![bank], presets);
        let results = b.search(&SearchQuery::with_text("zzznomatch"));
        assert!(results.is_empty());
    }

    #[test]
    fn search_by_category_filter() {
        let bank = user_bank_with("B", &["p1", "p2", "p3"]);
        let presets = vec![
            make_preset("p1", "A", "Lead", "X", &[]),
            make_preset("p2", "B", "Pad", "Y", &[]),
            make_preset("p3", "C", "Lead", "Z", &[]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        let results = b.search(&SearchQuery::with_category("Lead"));
        assert_eq!(results.len(), 2);
        for r in &results {
            assert_eq!(r.category, "Lead");
        }
    }

    #[test]
    fn search_by_category_is_case_insensitive() {
        let bank = user_bank_with("B", &["p1"]);
        let presets = vec![make_preset("p1", "A", "Pad", "X", &[])];
        let b = PresetBrowser::new(vec![bank], presets);
        assert_eq!(b.search(&SearchQuery::with_category("PAD")).len(), 1);
        assert_eq!(b.search(&SearchQuery::with_category("pad")).len(), 1);
    }

    #[test]
    fn search_by_author_filter() {
        let bank = user_bank_with("B", &["p1", "p2"]);
        let presets = vec![
            make_preset("p1", "A", "Lead", "Alice", &[]),
            make_preset("p2", "B", "Pad", "Bob", &[]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        let results = b.search(&SearchQuery {
            author: Some("alice".to_string()),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].author, "Alice");
    }

    #[test]
    fn search_by_bank_name_filter() {
        let bank_a = user_bank_with("Pads", &["p1"]);
        let bank_b = user_bank_with("Leads", &["p2"]);
        let presets = vec![
            make_preset("p1", "A", "Pad", "X", &[]),
            make_preset("p2", "B", "Lead", "Y", &[]),
        ];
        let b = PresetBrowser::new(vec![bank_a, bank_b], presets);
        let results = b.search(&SearchQuery {
            bank_name: Some("Leads".to_string()),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].bank_name, "Leads");
    }

    #[test]
    fn search_combines_text_and_category_filters() {
        let bank = user_bank_with("B", &["p1", "p2", "p3"]);
        let presets = vec![
            make_preset("p1", "Warm Lead", "Lead", "X", &[]),
            make_preset("p2", "Warm Pad", "Pad", "Y", &[]),
            make_preset("p3", "Cold Lead", "Lead", "Z", &[]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        let results = b.search(&SearchQuery {
            text: Some("warm".to_string()),
            category: Some("Lead".to_string()),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Warm Lead");
    }

    #[test]
    fn search_empty_text_matches_all() {
        let bank = user_bank_with("B", &["p1", "p2"]);
        let presets = vec![
            make_preset("p1", "A", "Lead", "X", &[]),
            make_preset("p2", "B", "Pad", "Y", &[]),
        ];
        let b = PresetBrowser::new(vec![bank], presets);
        let results = b.search(&SearchQuery {
            text: Some(String::new()),
            ..Default::default()
        });
        assert_eq!(results.len(), 2);
    }

    // ── preview ───────────────────────────────────────────────────────────────

    #[test]
    fn preview_returns_preset_for_known_id() {
        let presets = vec![make_preset("lead-001", "Bright Lead", "Lead", "Alice", &[])];
        let b = PresetBrowser::new(vec![], presets);
        let result = b.preview(&PresetId::new("lead-001"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().metadata.name, "Bright Lead");
    }

    #[test]
    fn preview_returns_none_for_unknown_id() {
        let b = PresetBrowser::new(vec![], vec![]);
        assert!(b.preview(&PresetId::new("nonexistent")).is_none());
    }

    #[test]
    fn preview_does_not_require_bank_membership() {
        // Preview works on any registered preset even if it's not in a bank
        let presets = vec![make_preset("orphan", "Orphan", "Other", "X", &[])];
        let b = PresetBrowser::new(vec![], presets);
        assert!(b.preview(&PresetId::new("orphan")).is_some());
    }

    #[test]
    fn preview_returns_reference_with_all_fields() {
        let presets = vec![make_preset("p1", "My Preset", "Bass", "Dave", &["deep"])];
        let b = PresetBrowser::new(vec![], presets);
        let p = b.preview(&PresetId::new("p1")).unwrap();
        assert_eq!(p.metadata.category, "Bass");
        assert_eq!(p.metadata.author, "Dave");
        assert_eq!(p.metadata.tags, vec!["deep"]);
    }

    // ── BrowserEntry fields ───────────────────────────────────────────────────

    #[test]
    fn browser_entry_fields_match_preset_and_bank() {
        let bank = user_bank_with("My Bank", &["p1"]);
        let presets = vec![make_preset("p1", "Warm Pad", "Pad", "Alice", &["warm"])];
        let b = PresetBrowser::new(vec![bank], presets);
        let entries = b.list_all();
        let e = &entries[0];
        assert_eq!(e.preset_id, PresetId::new("p1"));
        assert_eq!(e.name, "Warm Pad");
        assert_eq!(e.category, "Pad");
        assert_eq!(e.author, "Alice");
        assert_eq!(e.tags, vec!["warm"]);
        assert_eq!(e.bank_name, "My Bank");
        assert!(!e.is_factory);
    }

    // ── multiple banks, same preset id ───────────────────────────────────────

    #[test]
    fn same_preset_in_two_banks_appears_twice() {
        let bank_a = user_bank_with("A", &["shared"]);
        let bank_b = user_bank_with("B", &["shared"]);
        let presets = vec![make_preset("shared", "Shared Sound", "Lead", "X", &[])];
        let b = PresetBrowser::new(vec![bank_a, bank_b], presets);
        // The preset is referenced by both banks so it appears twice in list_all
        let entries = b.list_all();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.bank_name == "A"));
        assert!(entries.iter().any(|e| e.bank_name == "B"));
    }
}
