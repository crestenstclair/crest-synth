// path: src/shell/glyph_resolver.rs

//! Domain service: resolves the correct controller glyph for each button
//! based on the currently connected controller type.
//!
//! `GlyphResolver` tracks which `ControllerType` is currently connected
//! (updated via `set_controller_type`, driven by the UI/MIDI thread as
//! gamepad connect/disconnect events arrive) and produces a
//! `ControllerGlyph` for any logical `GamepadButton` against that
//! connected type. It performs no I/O and touches no rendering backend —
//! it purely composes the `ControllerGlyph` value object with the notion
//! of "the currently connected controller."

use crate::shell::controller_glyph::{ControllerGlyph, ControllerType, GamepadButton};

/// Resolves controller glyphs for the currently connected controller.
///
/// Holds no rendering state — it is a thin domain service over
/// `ControllerGlyph::resolve`, tracking which `ControllerType` is
/// currently connected so callers don't have to thread that value
/// through every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphResolver {
    controller_type: ControllerType,
}

impl GlyphResolver {
    /// Builds a resolver defaulting to `ControllerType::Generic` until a
    /// real controller is detected — falling back to the generic glyph
    /// set is safer than guessing a specific brand before one connects.
    pub fn new() -> Self {
        Self::with_controller_type(ControllerType::Generic)
    }

    /// Builds a resolver for an explicitly known controller type.
    pub fn with_controller_type(controller_type: ControllerType) -> Self {
        Self { controller_type }
    }

    /// Returns the controller type this resolver currently resolves
    /// glyphs against.
    pub fn controller_type(&self) -> ControllerType {
        self.controller_type
    }

    /// Updates which controller type is connected. Intended to be called
    /// by the shell in response to gamepad connect/disconnect events.
    pub fn set_controller_type(&mut self, controller_type: ControllerType) {
        self.controller_type = controller_type;
    }

    /// Resolves the glyph for `button` on the currently connected
    /// controller type.
    pub fn resolve(&self, button: GamepadButton) -> ControllerGlyph {
        ControllerGlyph::resolve(button, self.controller_type)
    }
}

impl Default for GlyphResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_to_generic_controller_type() {
        let resolver = GlyphResolver::new();
        assert_eq!(resolver.controller_type(), ControllerType::Generic);
    }

    #[test]
    fn resolve_uses_the_configured_controller_type() {
        let resolver = GlyphResolver::with_controller_type(ControllerType::Xbox);
        let glyph = resolver.resolve(GamepadButton::South);
        assert_eq!(glyph.glyph_path(), "assets/glyphs/xbox/south.svg");
    }

    #[test]
    fn set_controller_type_changes_subsequent_resolutions() {
        let mut resolver = GlyphResolver::new();
        assert_eq!(
            resolver.resolve(GamepadButton::Start).glyph_path(),
            "assets/glyphs/generic/start.svg"
        );

        resolver.set_controller_type(ControllerType::SteamDeck);
        assert_eq!(
            resolver.resolve(GamepadButton::Start).glyph_path(),
            "assets/glyphs/steam_deck/start.svg"
        );
    }

    #[test]
    fn resolve_matches_direct_controller_glyph_resolution() {
        let resolver = GlyphResolver::with_controller_type(ControllerType::PlayStation);
        let via_resolver = resolver.resolve(GamepadButton::East);
        let direct = ControllerGlyph::resolve(GamepadButton::East, ControllerType::PlayStation);
        assert_eq!(via_resolver, direct);
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(GlyphResolver::default(), GlyphResolver::new());
    }
}
