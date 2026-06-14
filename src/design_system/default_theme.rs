// path: src/design_system/default_theme.rs

use crate::design_system::rgba::Rgba;
use crate::design_system::semantic_token::SemanticToken;
use crate::design_system::theme::Theme;

/// The default dark palette — the one concretion of [`Theme`] shipped with the
/// app.
///
/// Maps every [`SemanticToken`] to a concrete [`Rgba`] using a dense, low-glare
/// dark palette suited to a synthesizer editor. The mapping is exhaustive: every
/// variant is handled by an explicit `match` arm so that adding a new
/// `SemanticToken` variant causes a compile error until `DefaultTheme` is
/// updated.
///
/// `DefaultTheme` is a zero-size struct; it allocates nothing and can be
/// constructed as a constant.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultTheme;

impl DefaultTheme {
    /// Construct a new `DefaultTheme`.
    pub fn new() -> Self {
        Self
    }
}

impl Theme for DefaultTheme {
    /// Resolve a [`SemanticToken`] to its dark-palette [`Rgba`].
    ///
    /// Every variant is handled exhaustively — no fallback / wildcard arm.
    fn color(&self, token: SemanticToken) -> Rgba {
        match token {
            // Bright cyan-ish focus ring — visible against all panel backgrounds.
            SemanticToken::FocusRing => Rgba::new(0, 188, 212, 255),
            // Amber / orange edit-active state — clearly distinct from FocusRing.
            SemanticToken::EditActive => Rgba::new(255, 167, 38, 255),
            // Mid-blue value bar — reads as "data" against the dark panel.
            SemanticToken::ValueFill => Rgba::new(66, 133, 244, 255),
            // Vivid red peak indicator — draws the eye for danger-level signals.
            SemanticToken::MeterPeak => Rgba::new(229, 57, 53, 255),
            // Soft green for an engaged toggle.
            SemanticToken::ToggleOn => Rgba::new(67, 160, 71, 255),
            // Dark grey for a disengaged toggle.
            SemanticToken::ToggleOff => Rgba::new(66, 66, 66, 255),
            // Near-white for primary body text.
            SemanticToken::TextDefault => Rgba::new(224, 224, 224, 255),
            // Medium grey for secondary / label text.
            SemanticToken::TextMuted => Rgba::new(117, 117, 117, 255),
            // Very dark background for panels and containers.
            SemanticToken::PanelBg => Rgba::new(22, 22, 22, 255),
            // Subtle mid-dark line between sections.
            SemanticToken::Separator => Rgba::new(48, 48, 48, 255),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every [`SemanticToken`] variant must resolve to a color without panicking.
    #[test]
    fn default_theme_resolves_every_token() {
        let theme = DefaultTheme::new();
        let tokens = [
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
        ];
        for token in tokens {
            let _color = theme.color(token);
        }
    }

    #[test]
    fn default_theme_tokens_are_not_all_the_same_color() {
        // A trivially wrong implementation could return black for everything.
        // Verify that at least two distinct tokens produce distinct colors.
        let theme = DefaultTheme::new();
        let focus_ring = theme.color(SemanticToken::FocusRing);
        let panel_bg = theme.color(SemanticToken::PanelBg);
        assert_ne!(focus_ring, panel_bg, "FocusRing and PanelBg must differ");
    }

    #[test]
    fn default_theme_toggle_on_differs_from_toggle_off() {
        let theme = DefaultTheme::new();
        assert_ne!(
            theme.color(SemanticToken::ToggleOn),
            theme.color(SemanticToken::ToggleOff),
            "ToggleOn and ToggleOff must be visually distinct"
        );
    }

    #[test]
    fn default_theme_focus_ring_differs_from_edit_active() {
        let theme = DefaultTheme::new();
        assert_ne!(
            theme.color(SemanticToken::FocusRing),
            theme.color(SemanticToken::EditActive),
            "FocusRing and EditActive must be visually distinct"
        );
    }

    #[test]
    fn default_theme_text_default_differs_from_text_muted() {
        let theme = DefaultTheme::new();
        assert_ne!(
            theme.color(SemanticToken::TextDefault),
            theme.color(SemanticToken::TextMuted),
            "TextDefault and TextMuted must be visually distinct"
        );
    }

    #[test]
    fn default_theme_is_copy() {
        let a = DefaultTheme::new();
        let b = a;
        // If DefaultTheme is Copy, this must compile and both remain usable.
        let _ = a.color(SemanticToken::FocusRing);
        let _ = b.color(SemanticToken::PanelBg);
    }
}
