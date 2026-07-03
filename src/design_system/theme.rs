// path: src/design_system/theme.rs

use std::collections::HashMap;

use crate::design_system::rgba::Rgba;
use crate::design_system::semantic_token::SemanticToken;

/// The seam every skin resolves colors through.
///
/// A skin holds no literal color: it holds a `SemanticToken` and asks a
/// `Theme` to resolve it via `color`. Swapping the `Theme` implementation
/// (dark, light, high-contrast, ...) restyles every consumer with zero
/// draw-code change. Consumers depend on this trait, never on a concrete
/// theme, so tests can supply a stub `Theme` without touching real palettes.
pub trait Theme {
    /// Resolve a semantic role to a concrete color.
    ///
    /// Implementations must be total: every `SemanticToken` variant
    /// resolves to some `Rgba`, even if that means falling back to a
    /// default color for tokens the palette does not explicitly cover.
    fn color(&self, token: SemanticToken) -> Rgba;
}

/// A `Theme` whose palette is supplied by the caller rather than hard-coded,
/// so the mapping from token to color is an injected dependency, not a
/// baked-in assumption of this type.
///
/// `StaticTheme::dark()` is a convenience constructor for callers who just
/// want a sensible built-in palette; tests and alternate skins use
/// `StaticTheme::new` to inject any palette they like.
pub struct StaticTheme {
    palette: HashMap<SemanticToken, Rgba>,
    fallback: Rgba,
}

impl StaticTheme {
    /// Full constructor: inject the palette and the fallback color used for
    /// any token the palette omits.
    pub fn new(palette: HashMap<SemanticToken, Rgba>, fallback: Rgba) -> Self {
        Self { palette, fallback }
    }

    /// Convenience constructor: a complete, sensible dark palette.
    pub fn dark() -> Self {
        Self::new(Self::dark_palette(), Rgba::black())
    }

    fn dark_palette() -> HashMap<SemanticToken, Rgba> {
        let mut palette = HashMap::new();
        palette.insert(
            SemanticToken::Background,
            Rgba::try_new(0.07, 0.07, 0.08, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::Surface,
            Rgba::try_new(0.12, 0.12, 0.13, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::SurfaceVariant,
            Rgba::try_new(0.16, 0.16, 0.18, 1.0).expect("in-range literal"),
        );
        palette.insert(SemanticToken::Foreground, Rgba::white());
        palette.insert(
            SemanticToken::ForegroundMuted,
            Rgba::try_new(0.6, 0.6, 0.62, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::Accent,
            Rgba::try_new(0.30, 0.62, 1.0, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::AccentMuted,
            Rgba::try_new(0.20, 0.42, 0.68, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::Border,
            Rgba::try_new(0.24, 0.24, 0.26, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::FocusRing,
            Rgba::try_new(1.0, 0.80, 0.20, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::EditActive,
            Rgba::try_new(1.0, 0.45, 0.20, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::Danger,
            Rgba::try_new(0.90, 0.25, 0.25, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::Warning,
            Rgba::try_new(0.95, 0.70, 0.15, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::Success,
            Rgba::try_new(0.25, 0.80, 0.40, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::MeterLow,
            Rgba::try_new(0.25, 0.80, 0.40, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::MeterMid,
            Rgba::try_new(0.95, 0.70, 0.15, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::MeterHigh,
            Rgba::try_new(0.90, 0.25, 0.25, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::MuteIndicator,
            Rgba::try_new(0.60, 0.60, 0.62, 1.0).expect("in-range literal"),
        );
        palette.insert(
            SemanticToken::SoloIndicator,
            Rgba::try_new(1.0, 0.80, 0.20, 1.0).expect("in-range literal"),
        );
        palette
    }
}

impl Theme for StaticTheme {
    fn color(&self, token: SemanticToken) -> Rgba {
        self.palette.get(&token).copied().unwrap_or(self.fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_resolves_every_known_token() {
        let theme = StaticTheme::dark();
        let tokens = [
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
        ];
        for token in tokens {
            // Every known token must resolve without falling back.
            let resolved = theme.color(token);
            assert_ne!(resolved, theme.fallback_for_test());
        }
    }

    #[test]
    fn missing_token_falls_back() {
        let mut palette = HashMap::new();
        palette.insert(SemanticToken::Background, Rgba::black());
        let fallback = Rgba::white();
        let theme = StaticTheme::new(palette, fallback);

        assert_eq!(theme.color(SemanticToken::Accent), fallback);
        assert_eq!(theme.color(SemanticToken::Background), Rgba::black());
    }

    #[test]
    fn injected_palette_is_used_verbatim() {
        let mut palette = HashMap::new();
        let custom = Rgba::try_new(0.5, 0.5, 0.5, 1.0).unwrap();
        palette.insert(SemanticToken::FocusRing, custom);
        let theme = StaticTheme::new(palette, Rgba::black());

        assert_eq!(theme.color(SemanticToken::FocusRing), custom);
    }

    #[test]
    fn theme_is_usable_as_a_trait_object() {
        let theme: Box<dyn Theme> = Box::new(StaticTheme::dark());
        let _ = theme.color(SemanticToken::Foreground);
    }

    impl StaticTheme {
        /// Test-only accessor so tests can assert against the configured
        /// fallback without duplicating the literal.
        fn fallback_for_test(&self) -> Rgba {
            self.fallback
        }
    }
}
