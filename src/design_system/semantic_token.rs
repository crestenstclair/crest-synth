// path: src/design_system/semantic_token.rs

/// Named UI intent tokens that a skin resolves through the Theme port.
///
/// A `SemanticToken` expresses *what* a color is for, never *which* color it
/// is. Skins map each token to a concrete `egui::Color32` (or size) by
/// consulting the active `Theme` implementation — no literal colour value or
/// hard-coded size ever appears in draw code.
///
/// # Variants
///
/// | Token        | Intent                                              |
/// |--------------|-----------------------------------------------------|
/// | FocusRing    | Marks the currently focused cell                   |
/// | EditActive   | Marks a cell that is focused *and* in edit mode    |
/// | ValueFill    | Value readout or progress-bar fill                  |
/// | MeterPeak    | Live peak-level overlay on the meter                |
/// | ToggleOn     | Toggle/button in the ON state                       |
/// | ToggleOff    | Toggle/button in the OFF state                      |
/// | TextDefault  | Primary body text                                   |
/// | TextMuted    | Secondary / de-emphasised text                      |
/// | PanelBg      | Panel or container background                       |
/// | Separator    | Divider lines between containers                    |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticToken {
    /// Marks the currently focused cell.
    FocusRing,
    /// Marks a cell that is focused *and* in edit mode.
    EditActive,
    /// Value readout or progress-bar fill.
    ValueFill,
    /// Live peak-level overlay on the meter.
    MeterPeak,
    /// Toggle/button in the ON state.
    ToggleOn,
    /// Toggle/button in the OFF state.
    ToggleOff,
    /// Primary body text.
    TextDefault,
    /// Secondary / de-emphasised text.
    TextMuted,
    /// Panel or container background.
    PanelBg,
    /// Divider lines between containers.
    Separator,
}

impl SemanticToken {
    /// Returns a slice containing every variant in declaration order.
    pub fn all() -> &'static [SemanticToken] {
        &[
            SemanticToken::FocusRing,
            SemanticToken::EditActive,
            SemanticToken::ValueFill,
            SemanticToken::MeterPeak,
            SemanticToken::ToggleOn,
            SemanticToken::ToggleOff,
            SemanticToken::TextDefault,
            SemanticToken::TextMuted,
            SemanticToken::PanelBg,
            SemanticToken::Separator,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::SemanticToken;

    #[test]
    fn all_returns_exactly_ten_variants() {
        assert_eq!(SemanticToken::all().len(), 10);
    }

    #[test]
    fn variant_set_is_complete() {
        let all = SemanticToken::all();
        assert!(all.contains(&SemanticToken::FocusRing));
        assert!(all.contains(&SemanticToken::EditActive));
        assert!(all.contains(&SemanticToken::ValueFill));
        assert!(all.contains(&SemanticToken::MeterPeak));
        assert!(all.contains(&SemanticToken::ToggleOn));
        assert!(all.contains(&SemanticToken::ToggleOff));
        assert!(all.contains(&SemanticToken::TextDefault));
        assert!(all.contains(&SemanticToken::TextMuted));
        assert!(all.contains(&SemanticToken::PanelBg));
        assert!(all.contains(&SemanticToken::Separator));
    }

    #[test]
    fn tokens_are_copy_and_eq() {
        let a = SemanticToken::FocusRing;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_variants_are_not_equal() {
        assert_ne!(SemanticToken::FocusRing, SemanticToken::EditActive);
        assert_ne!(SemanticToken::ToggleOn, SemanticToken::ToggleOff);
        assert_ne!(SemanticToken::TextDefault, SemanticToken::TextMuted);
    }

    #[test]
    fn token_is_hashable() {
        use std::collections::HashMap;
        let mut map: HashMap<SemanticToken, &str> = HashMap::new();
        map.insert(SemanticToken::FocusRing, "focus-ring");
        map.insert(SemanticToken::PanelBg, "panel-bg");
        assert_eq!(map[&SemanticToken::FocusRing], "focus-ring");
        assert_eq!(map[&SemanticToken::PanelBg], "panel-bg");
    }
}
