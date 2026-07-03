// path: src/design_system/default_theme.rs

use crate::design_system::rgba::Rgba;
use crate::design_system::semantic_token::SemanticToken;
use crate::design_system::theme::Theme;

/// The application's built-in dark palette.
///
/// `DefaultTheme` implements the `Theme` port by matching every
/// `SemanticToken` variant to a concrete `Rgba` literal. The match has no
/// wildcard arm, so the compiler rejects any build that adds a
/// `SemanticToken` variant without also resolving it here — "no token left
/// unresolved" is enforced at compile time, not by a runtime fallback.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultTheme;

impl DefaultTheme {
    /// Construct the default dark theme. `DefaultTheme` is zero-sized and
    /// holds no dependencies, so there is nothing to inject; this is a
    /// convenience constructor equivalent to the derived `Default`.
    pub fn new() -> Self {
        Self
    }
}

impl Theme for DefaultTheme {
    fn color(&self, token: SemanticToken) -> Rgba {
        match token {
            SemanticToken::Background => Rgba::try_new(0.07, 0.07, 0.08, 1.0),
            SemanticToken::Surface => Rgba::try_new(0.12, 0.12, 0.13, 1.0),
            SemanticToken::SurfaceVariant => Rgba::try_new(0.16, 0.16, 0.18, 1.0),
            SemanticToken::Foreground => Rgba::try_new(1.0, 1.0, 1.0, 1.0),
            SemanticToken::ForegroundMuted => Rgba::try_new(0.60, 0.60, 0.62, 1.0),
            SemanticToken::Accent => Rgba::try_new(0.30, 0.62, 1.0, 1.0),
            SemanticToken::AccentMuted => Rgba::try_new(0.20, 0.42, 0.68, 1.0),
            SemanticToken::Border => Rgba::try_new(0.24, 0.24, 0.26, 1.0),
            SemanticToken::FocusRing => Rgba::try_new(1.0, 0.80, 0.20, 1.0),
            SemanticToken::EditActive => Rgba::try_new(1.0, 0.45, 0.20, 1.0),
            SemanticToken::Danger => Rgba::try_new(0.90, 0.25, 0.25, 1.0),
            SemanticToken::Warning => Rgba::try_new(0.95, 0.70, 0.15, 1.0),
            SemanticToken::Success => Rgba::try_new(0.25, 0.80, 0.40, 1.0),
            SemanticToken::MeterLow => Rgba::try_new(0.25, 0.80, 0.40, 1.0),
            SemanticToken::MeterMid => Rgba::try_new(0.95, 0.70, 0.15, 1.0),
            SemanticToken::MeterHigh => Rgba::try_new(0.90, 0.25, 0.25, 1.0),
            SemanticToken::MuteIndicator => Rgba::try_new(0.60, 0.60, 0.62, 1.0),
            SemanticToken::SoloIndicator => Rgba::try_new(1.0, 0.80, 0.20, 1.0),
        }
        .expect("DefaultTheme literals are in-range by construction")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_tokens() -> [SemanticToken; 18] {
        [
            SemanticToken::Background,
            SemanticToken::Surface,
            SemanticToken::SurfaceVariant,
            SemanticToken::Foreground,
            SemanticToken::ForegroundMuted,
            SemanticToken::Accent,
            SemanticToken::AccentMuted,
            SemanticToken::Border,
            SemanticToken::FocusRing,
            SemanticToken::EditActive,
            SemanticToken::Danger,
            SemanticToken::Warning,
            SemanticToken::Success,
            SemanticToken::MeterLow,
            SemanticToken::MeterMid,
            SemanticToken::MeterHigh,
            SemanticToken::MuteIndicator,
            SemanticToken::SoloIndicator,
        ]
    }

    #[test]
    fn default_theme_resolves_every_semantic_token() {
        let theme = DefaultTheme::new();
        // Reaching this line for every token in the exhaustive array proves
        // every SemanticToken variant resolves without panicking.
        for token in all_tokens() {
            let _ = theme.color(token);
        }
    }

    #[test]
    fn default_theme_new_matches_derived_default() {
        let a = DefaultTheme::new();
        let b = DefaultTheme;
        assert_eq!(
            a.color(SemanticToken::Accent),
            b.color(SemanticToken::Accent)
        );
    }

    #[test]
    fn default_theme_usable_as_trait_object() {
        let theme: Box<dyn Theme> = Box::new(DefaultTheme::new());
        let _ = theme.color(SemanticToken::Foreground);
    }

    #[test]
    fn distinct_tokens_can_map_to_distinct_colors() {
        let theme = DefaultTheme::new();
        assert_ne!(
            theme.color(SemanticToken::Background),
            theme.color(SemanticToken::Foreground)
        );
    }
}
