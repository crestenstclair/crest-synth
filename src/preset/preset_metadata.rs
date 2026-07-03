// path: src/preset/preset_metadata.rs

use std::fmt;

/// Human-authored, searchable metadata carried by every preset.
///
/// `PresetMetadata` is a plain value object: two instances with equal
/// field values are interchangeable. Construction is validated so a
/// `PresetMetadata` in hand always has a non-empty `name` — the field
/// the rest of the system relies on for identification and display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetMetadata {
    name: String,
    author: String,
    category: String,
    description: String,
    tags: Vec<String>,
}

/// Reasons `PresetMetadata::try_new` can reject its inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetMetadataError {
    /// The preset name was empty (or whitespace-only) after trimming.
    EmptyName,
}

impl fmt::Display for PresetMetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PresetMetadataError::EmptyName => {
                write!(f, "preset metadata name must not be empty")
            }
        }
    }
}

impl std::error::Error for PresetMetadataError {}

impl PresetMetadata {
    /// Builds validated preset metadata.
    ///
    /// `name` must contain at least one non-whitespace character; all
    /// other fields are free-form and may be empty. Tags are trimmed,
    /// empty tags are dropped, and duplicate tags (case-sensitive) are
    /// removed while preserving first-seen order.
    pub fn try_new(
        name: impl Into<String>,
        author: impl Into<String>,
        category: impl Into<String>,
        description: impl Into<String>,
        tags: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, PresetMetadataError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(PresetMetadataError::EmptyName);
        }

        Ok(Self {
            name,
            author: author.into(),
            category: category.into(),
            description: description.into(),
            tags: normalize_tags(tags),
        })
    }

    /// The preset's display name. Never empty.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Free-form author/creator string; may be empty.
    pub fn author(&self) -> &str {
        &self.author
    }

    /// Free-form category label (e.g. "Bass", "Pad"); may be empty.
    pub fn category(&self) -> &str {
        &self.category
    }

    /// Free-form human-readable description; may be empty.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Normalized, de-duplicated, order-preserving tag list.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Returns true if `query` case-insensitively matches the name,
    /// author, category, description, or any tag. An empty query
    /// matches everything.
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let needle = query.to_lowercase();
        self.name.to_lowercase().contains(&needle)
            || self.author.to_lowercase().contains(&needle)
            || self.category.to_lowercase().contains(&needle)
            || self.description.to_lowercase().contains(&needle)
            || self
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&needle))
    }

    /// Returns a copy of this metadata with the tag list replaced,
    /// applying the same trim/dedup normalization as `try_new`.
    pub fn with_tags(&self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            tags: normalize_tags(tags),
            ..self.clone()
        }
    }
}

fn normalize_tags(tags: impl IntoIterator<Item = impl Into<String>>) -> Vec<String> {
    let mut normalized: Vec<String> = Vec::new();
    for raw in tags {
        let owned = raw.into();
        let trimmed = owned.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !normalized.iter().any(|existing| existing == trimmed) {
            normalized.push(trimmed.to_string());
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_a_non_empty_name() {
        let metadata = PresetMetadata::try_new(
            "Warm Pad",
            "cresten",
            "Pad",
            "a warm evolving pad",
            ["warm", "pad"],
        )
        .expect("valid metadata should construct");

        assert_eq!(metadata.name(), "Warm Pad");
        assert_eq!(metadata.author(), "cresten");
        assert_eq!(metadata.category(), "Pad");
        assert_eq!(metadata.description(), "a warm evolving pad");
        assert_eq!(metadata.tags(), &["warm".to_string(), "pad".to_string()]);
    }

    #[test]
    fn try_new_rejects_an_empty_name() {
        let result = PresetMetadata::try_new("   ", "cresten", "", "", Vec::<String>::new());
        assert_eq!(result, Err(PresetMetadataError::EmptyName));
    }

    #[test]
    fn try_new_trims_and_dedupes_tags_preserving_order() {
        let metadata = PresetMetadata::try_new(
            "Lead",
            "",
            "",
            "",
            vec![" lead ", "bright", "lead", "", "  "],
        )
        .expect("valid metadata should construct");

        assert_eq!(metadata.tags(), &["lead".to_string(), "bright".to_string()]);
    }

    #[test]
    fn matches_is_case_insensitive_across_all_fields() {
        let metadata = PresetMetadata::try_new(
            "Deep Bass",
            "Cresten",
            "Bass",
            "A deep sub bass",
            vec!["sub", "low"],
        )
        .expect("valid metadata should construct");

        assert!(metadata.matches("deep"));
        assert!(metadata.matches("CRESTEN"));
        assert!(metadata.matches("bass"));
        assert!(metadata.matches("sub"));
        assert!(!metadata.matches("lead"));
    }

    #[test]
    fn matches_empty_query_matches_everything() {
        let metadata = PresetMetadata::try_new("Anything", "", "", "", Vec::<String>::new())
            .expect("valid metadata should construct");
        assert!(metadata.matches(""));
    }

    #[test]
    fn with_tags_returns_a_new_normalized_instance_without_mutating_original() {
        let original = PresetMetadata::try_new("Pluck", "", "", "", vec!["old"])
            .expect("valid metadata should construct");

        let updated = original.with_tags(vec!["new", "new", " fresh "]);

        assert_eq!(original.tags(), &["old".to_string()]);
        assert_eq!(updated.tags(), &["new".to_string(), "fresh".to_string()]);
        assert_eq!(updated.name(), "Pluck");
    }

    #[test]
    fn equal_field_values_are_equal_instances() {
        let a = PresetMetadata::try_new("A", "b", "c", "d", vec!["e"])
            .expect("valid metadata should construct");
        let b = PresetMetadata::try_new("A", "b", "c", "d", vec!["e"])
            .expect("valid metadata should construct");
        assert_eq!(a, b);
    }
}
