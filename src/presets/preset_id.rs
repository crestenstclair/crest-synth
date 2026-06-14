// path: src/presets/preset_id.rs

/// Unique identifier for a preset.
///
/// A `PresetId` is a thin newtype over `String` that accepts either a UUID
/// (`"550e8400-e29b-41d4-a716-446655440000"`) or a human-readable slug
/// (`"bright-pad"`).  Using a dedicated type prevents accidental mixing of
/// raw strings with preset identifiers in function signatures.
///
/// # Examples
///
/// ```
/// use crest_synth::presets::preset_id::PresetId;
///
/// let id = PresetId::new("bright-pad");
/// assert_eq!(id.as_str(), "bright-pad");
/// assert_eq!(id.to_string(), "bright-pad");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PresetId(String);

impl PresetId {
    /// Creates a new `PresetId` from any value that converts to a `String`.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes `self` and returns the underlying `String`.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for PresetId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for PresetId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<PresetId> for String {
    fn from(id: PresetId) -> Self {
        id.into_string()
    }
}

impl std::fmt::Display for PresetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::PresetId;

    #[test]
    fn new_from_str_slice() {
        let id = PresetId::new("warm-bass");
        assert_eq!(id.as_str(), "warm-bass");
    }

    #[test]
    fn new_from_string() {
        let id = PresetId::new(String::from("pad-001"));
        assert_eq!(id.as_str(), "pad-001");
    }

    #[test]
    fn uuid_style_identifier() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let id = PresetId::new(uuid);
        assert_eq!(id.as_str(), uuid);
    }

    #[test]
    fn as_str_borrows_without_clone() {
        let id = PresetId::new("organ");
        let s: &str = id.as_str();
        assert_eq!(s, "organ");
        // id is still accessible after the borrow ends
        let _clone = id.clone();
    }

    #[test]
    fn into_string_consumes() {
        let id = PresetId::new("strings");
        let raw: String = id.into_string();
        assert_eq!(raw, "strings");
    }

    #[test]
    fn from_string() {
        let id: PresetId = String::from("choir").into();
        assert_eq!(id.as_str(), "choir");
    }

    #[test]
    fn from_str_ref() {
        let id: PresetId = "epiano".into();
        assert_eq!(id.as_str(), "epiano");
    }

    #[test]
    fn from_preset_id_into_string() {
        let id = PresetId::new("flute");
        let raw: String = id.into();
        assert_eq!(raw, "flute");
    }

    #[test]
    fn display_matches_inner_string() {
        let id = PresetId::new("bright-lead");
        assert_eq!(format!("{}", id), "bright-lead");
    }

    #[test]
    fn equality() {
        assert_eq!(PresetId::new("a"), PresetId::new("a"));
        assert_ne!(PresetId::new("a"), PresetId::new("b"));
    }

    #[test]
    fn ordering() {
        assert!(PresetId::new("aaa") < PresetId::new("bbb"));
        assert!(PresetId::new("z") > PresetId::new("a"));
    }

    #[test]
    fn clone_produces_equal_value() {
        let id = PresetId::new("cloned-preset");
        let copy = id.clone();
        assert_eq!(id, copy);
    }

    #[test]
    fn hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PresetId::new("x"));
        set.insert(PresetId::new("y"));
        set.insert(PresetId::new("x")); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn can_be_used_as_map_key() {
        use std::collections::HashMap;
        let mut map: HashMap<PresetId, u32> = HashMap::new();
        map.insert(PresetId::new("preset-a"), 1);
        map.insert(PresetId::new("preset-b"), 2);
        assert_eq!(map[&PresetId::new("preset-a")], 1);
        assert_eq!(map[&PresetId::new("preset-b")], 2);
    }
}
