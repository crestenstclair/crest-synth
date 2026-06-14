// path: src/design_system/theme.rs

use crate::design_system::rgba::Rgba;
use crate::design_system::semantic_token::SemanticToken;

/// The abstraction a skin reads tokens through.
///
/// Every skin accepts a `&dyn Theme` (or a generic `T: Theme` bound) and
/// resolves every color through [`Theme::color`].  Swapping the `Theme`
/// restyles the whole app with zero behavior change.
///
/// # Contract
///
/// `color(token) -> Rgba` — the implementing type is responsible for returning
/// a meaningful color for **every** [`SemanticToken`] variant; no fallback or
/// panic path should be required in callers.
///
/// # Example
///
/// ```rust
/// use crest_synth::design_system::theme::Theme;
/// use crest_synth::design_system::semantic_token::SemanticToken;
/// use crest_synth::design_system::rgba::Rgba;
///
/// struct MyTheme;
///
/// impl Theme for MyTheme {
///     fn color(&self, token: SemanticToken) -> Rgba {
///         match token {
///             SemanticToken::FocusRing => Rgba::new(0, 120, 215, 255),
///             _ => Rgba::new(30, 30, 30, 255),
///         }
///     }
/// }
///
/// let theme = MyTheme;
/// let c = theme.color(SemanticToken::FocusRing);
/// assert_eq!(c.r, 0);
/// ```
pub trait Theme {
    /// Resolve a [`SemanticToken`] to its concrete [`Rgba`] color.
    fn color(&self, token: SemanticToken) -> Rgba;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design_system::semantic_token::SemanticToken;

    struct TestTheme;

    impl Theme for TestTheme {
        fn color(&self, token: SemanticToken) -> Rgba {
            match token {
                SemanticToken::FocusRing => Rgba::new(0, 120, 215, 255),
                SemanticToken::EditActive => Rgba::new(0, 180, 140, 255),
                SemanticToken::ValueFill => Rgba::new(80, 80, 200, 255),
                SemanticToken::MeterPeak => Rgba::new(220, 60, 60, 255),
                SemanticToken::ToggleOn => Rgba::new(60, 200, 80, 255),
                SemanticToken::ToggleOff => Rgba::new(60, 60, 60, 255),
                SemanticToken::TextDefault => Rgba::new(230, 230, 230, 255),
                SemanticToken::TextMuted => Rgba::new(140, 140, 140, 255),
                SemanticToken::PanelBg => Rgba::new(30, 30, 30, 255),
                SemanticToken::Separator => Rgba::new(55, 55, 55, 255),
            }
        }
    }

    #[test]
    fn theme_resolves_every_token() {
        let theme = TestTheme;
        // Every SemanticToken variant must resolve without panic.
        for &token in SemanticToken::all() {
            let _rgba = theme.color(token);
        }
    }

    #[test]
    fn theme_returns_correct_color_for_focus_ring() {
        let theme = TestTheme;
        let color = theme.color(SemanticToken::FocusRing);
        assert_eq!(color, Rgba::new(0, 120, 215, 255));
    }

    #[test]
    fn theme_returns_distinct_colors_for_distinct_tokens() {
        let theme = TestTheme;
        let focus_ring = theme.color(SemanticToken::FocusRing);
        let panel_bg = theme.color(SemanticToken::PanelBg);
        // Focus ring and panel background should not share the same color.
        assert_ne!(focus_ring, panel_bg);
    }

    #[test]
    fn theme_is_object_safe() {
        // Verify that Theme can be used as a trait object.
        let theme: &dyn Theme = &TestTheme;
        let _rgba = theme.color(SemanticToken::TextDefault);
    }
}
