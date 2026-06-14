// path: src/presets/preset_metadata.rs

/// Metadata about a preset for browsing and search.
///
/// `PresetMetadata` is a pure value object — it carries no audio-thread data,
/// no locks, and no heap-allocated buffers that are touched at render time.
/// It is only created, cloned, and serialised on non-audio threads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetMetadata {
    /// Human-readable name shown in the preset browser.
    pub name: String,
    /// Author or creator of the preset.
    pub author: String,
    /// Broad category (e.g. "Lead", "Pad", "Bass").
    pub category: String,
    /// ISO-8601 creation timestamp (e.g. `"2025-01-15T10:30:00Z"`).
    pub created_at: String,
    /// Free-form tags for filtering (e.g. `["warm", "evolving"]`).
    pub tags: Vec<String>,
}

impl PresetMetadata {
    /// Create a new `PresetMetadata`.
    ///
    /// # Arguments
    ///
    /// * `name`       – preset display name (non-empty recommended)
    /// * `author`     – creator name
    /// * `category`   – broad sound category
    /// * `created_at` – ISO-8601 timestamp string
    /// * `tags`       – zero or more search tags
    ///
    /// # Example
    ///
    /// ```
    /// use crest_synth::presets::preset_metadata::PresetMetadata;
    ///
    /// let meta = PresetMetadata::new(
    ///     "Warm Pad",
    ///     "Alice",
    ///     "Pad",
    ///     "2025-01-15T10:30:00Z",
    ///     vec!["warm".to_string(), "evolving".to_string()],
    /// );
    /// assert_eq!(meta.name, "Warm Pad");
    /// assert_eq!(meta.tags.len(), 2);
    /// ```
    pub fn new(
        name: impl Into<String>,
        author: impl Into<String>,
        category: impl Into<String>,
        created_at: impl Into<String>,
        tags: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            author: author.into(),
            category: category.into(),
            created_at: created_at.into(),
            tags,
        }
    }

    /// Returns `true` if any tag matches `query` (case-insensitive substring).
    ///
    /// # Example
    ///
    /// ```
    /// use crest_synth::presets::preset_metadata::PresetMetadata;
    ///
    /// let meta = PresetMetadata::new(
    ///     "Bass Line",
    ///     "Bob",
    ///     "Bass",
    ///     "2025-03-01T00:00:00Z",
    ///     vec!["punchy".to_string(), "sub".to_string()],
    /// );
    /// assert!(meta.has_tag("PUNCHY"));
    /// assert!(!meta.has_tag("warm"));
    /// ```
    pub fn has_tag(&self, query: &str) -> bool {
        let lower = query.to_lowercase();
        self.tags.iter().any(|t| t.to_lowercase().contains(&lower))
    }
}

impl Default for PresetMetadata {
    fn default() -> Self {
        Self::new("Untitled", "", "Other", "", vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta() -> PresetMetadata {
        PresetMetadata::new(
            "Warm Pad",
            "Alice",
            "Pad",
            "2025-01-15T10:30:00Z",
            vec!["warm".to_string(), "evolving".to_string()],
        )
    }

    // ---- construction ----

    #[test]
    fn preset_metadata_fields_round_trip() {
        let meta = make_meta();
        assert_eq!(meta.name, "Warm Pad");
        assert_eq!(meta.author, "Alice");
        assert_eq!(meta.category, "Pad");
        assert_eq!(meta.created_at, "2025-01-15T10:30:00Z");
        assert_eq!(meta.tags, vec!["warm", "evolving"]);
    }

    #[test]
    fn preset_metadata_clone_is_equal() {
        let meta = make_meta();
        let cloned = meta.clone();
        assert_eq!(meta, cloned);
    }

    #[test]
    fn preset_metadata_empty_tags_allowed() {
        let meta = PresetMetadata::new("Init", "Dev", "Other", "", vec![]);
        assert!(meta.tags.is_empty());
    }

    // ---- default ----

    #[test]
    fn preset_metadata_default_name_is_untitled() {
        let meta = PresetMetadata::default();
        assert_eq!(meta.name, "Untitled");
    }

    // ---- has_tag ----

    #[test]
    fn preset_metadata_has_tag_case_insensitive() {
        let meta = make_meta();
        assert!(meta.has_tag("WARM"));
        assert!(meta.has_tag("warm"));
        assert!(meta.has_tag("Evolving"));
    }

    #[test]
    fn preset_metadata_has_tag_substring_match() {
        let meta = make_meta();
        assert!(meta.has_tag("vol")); // matches "evolving"
    }

    #[test]
    fn preset_metadata_has_tag_miss() {
        let meta = make_meta();
        assert!(!meta.has_tag("dark"));
    }

    #[test]
    fn preset_metadata_has_tag_empty_query_matches_any_nonempty_tag() {
        let meta = make_meta();
        // An empty query is a substring of every string.
        assert!(meta.has_tag(""));
    }

    #[test]
    fn preset_metadata_has_tag_no_tags_returns_false() {
        let meta = PresetMetadata::new("Init", "", "Other", "", vec![]);
        assert!(!meta.has_tag("anything"));
    }
}
