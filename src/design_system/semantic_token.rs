// path: src/design_system/semantic_token.rs

/// A named role a skin resolves to a concrete `Rgba` through the `Theme`
/// port. Skins never hold a literal color; they hold a `SemanticToken` and
/// ask the active `Theme` to resolve it.
///
/// The token set names *purpose*, not appearance, so a new `Theme`
/// implementation (dark, light, high-contrast, ...) can restyle every skin
/// with zero draw-code change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticToken {
    /// The application's base background.
    Background,
    /// A raised surface sitting above the background (e.g. a panel).
    Surface,
    /// A surface variant used to differentiate adjacent surfaces.
    SurfaceVariant,
    /// Primary text/foreground content.
    Foreground,
    /// De-emphasized text/foreground content.
    ForegroundMuted,
    /// The primary accent used for active/selected state.
    Accent,
    /// A muted variant of the accent, for hover/secondary emphasis.
    AccentMuted,
    /// Hairline borders and dividers.
    Border,
    /// The ring drawn around the focused primitive.
    FocusRing,
    /// The indicator drawn on a primitive while it is in edit mode.
    EditActive,
    /// Destructive or error state.
    Danger,
    /// Cautionary state.
    Warning,
    /// Confirmatory/success state.
    Success,
    /// Meter fill in its low (safe) range.
    MeterLow,
    /// Meter fill in its mid (approaching hot) range.
    MeterMid,
    /// Meter fill in its high (clipping/hot) range.
    MeterHigh,
    /// A channel's mute indicator.
    MuteIndicator,
    /// A channel's solo indicator.
    SoloIndicator,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn tokens_are_distinct_hash_keys() {
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
        let unique: HashSet<SemanticToken> = tokens.iter().copied().collect();
        assert_eq!(unique.len(), tokens.len());
    }

    #[test]
    fn token_equality_is_structural() {
        assert_eq!(SemanticToken::FocusRing, SemanticToken::FocusRing);
        assert_ne!(SemanticToken::FocusRing, SemanticToken::EditActive);
    }
}
