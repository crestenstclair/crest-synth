// path: src/shell/glyph_resolver.rs

/// The type of gamepad controller connected, used to select the correct button glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerKind {
    /// Sony DualSense / DualShock family (PlayStation).
    PlayStation,
    /// Xbox controller family.
    Xbox,
    /// Nintendo Switch Pro controller.
    NintendoSwitch,
    /// Generic / unknown controller — fall back to generic labels.
    Generic,
}

/// A logical button on a standard gamepad layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    /// South face button (Cross / A / B).
    South,
    /// East face button (Circle / B / A).
    East,
    /// West face button (Square / X / Y).
    West,
    /// North face button (Triangle / Y / X).
    North,
    /// Left bumper (L1 / LB / L).
    LeftBumper,
    /// Right bumper (R1 / RB / R).
    RightBumper,
    /// Left trigger (L2 / LT / ZL).
    LeftTrigger,
    /// Right trigger (R2 / RT / ZR).
    RightTrigger,
    /// Select / Share / Back / Minus.
    Select,
    /// Start / Options / Menu / Plus.
    Start,
    /// D-pad up.
    DpadUp,
    /// D-pad down.
    DpadDown,
    /// D-pad left.
    DpadLeft,
    /// D-pad right.
    DpadRight,
    /// Left stick click (L3 / LS).
    LeftStickClick,
    /// Right stick click (R3 / RS).
    RightStickClick,
}

/// A short human-readable glyph string for a button, e.g. `"✕"`, `"A"`, `"B"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyph(pub &'static str);

/// Resolves the display glyph for a given button on a specific controller kind.
///
/// Inject a `ControllerKind` and call [`GlyphResolver::resolve`] to obtain the
/// correct label for each button. This keeps controller-specific string tables
/// out of the UI rendering loop.
///
/// # Examples
///
/// ```
/// use crest_synth::shell::glyph_resolver::{GlyphResolver, ControllerKind, GamepadButton};
///
/// let resolver = GlyphResolver::new(ControllerKind::PlayStation);
/// let glyph = resolver.resolve(GamepadButton::South);
/// assert_eq!(glyph.0, "✕");
/// ```
pub struct GlyphResolver {
    kind: ControllerKind,
}

impl GlyphResolver {
    /// Create a resolver for the given controller kind.
    pub fn new(kind: ControllerKind) -> Self {
        Self { kind }
    }

    /// Return the glyph for `button` on the controller this resolver was constructed with.
    pub fn resolve(&self, button: GamepadButton) -> Glyph {
        match self.kind {
            ControllerKind::PlayStation => Self::playstation_glyph(button),
            ControllerKind::Xbox => Self::xbox_glyph(button),
            ControllerKind::NintendoSwitch => Self::nintendo_glyph(button),
            ControllerKind::Generic => Self::generic_glyph(button),
        }
    }

    /// Return the current controller kind this resolver is configured for.
    pub fn controller_kind(&self) -> ControllerKind {
        self.kind
    }

    /// Replace the active controller kind (e.g. when user hot-swaps a controller).
    pub fn set_controller_kind(&mut self, kind: ControllerKind) {
        self.kind = kind;
    }

    // ── per-family glyph tables ─────────────────────────────────────────────

    fn playstation_glyph(button: GamepadButton) -> Glyph {
        Glyph(match button {
            GamepadButton::South => "✕",
            GamepadButton::East => "○",
            GamepadButton::West => "□",
            GamepadButton::North => "△",
            GamepadButton::LeftBumper => "L1",
            GamepadButton::RightBumper => "R1",
            GamepadButton::LeftTrigger => "L2",
            GamepadButton::RightTrigger => "R2",
            GamepadButton::Select => "Share",
            GamepadButton::Start => "Options",
            GamepadButton::DpadUp => "↑",
            GamepadButton::DpadDown => "↓",
            GamepadButton::DpadLeft => "←",
            GamepadButton::DpadRight => "→",
            GamepadButton::LeftStickClick => "L3",
            GamepadButton::RightStickClick => "R3",
        })
    }

    fn xbox_glyph(button: GamepadButton) -> Glyph {
        Glyph(match button {
            GamepadButton::South => "A",
            GamepadButton::East => "B",
            GamepadButton::West => "X",
            GamepadButton::North => "Y",
            GamepadButton::LeftBumper => "LB",
            GamepadButton::RightBumper => "RB",
            GamepadButton::LeftTrigger => "LT",
            GamepadButton::RightTrigger => "RT",
            GamepadButton::Select => "Back",
            GamepadButton::Start => "Menu",
            GamepadButton::DpadUp => "↑",
            GamepadButton::DpadDown => "↓",
            GamepadButton::DpadLeft => "←",
            GamepadButton::DpadRight => "→",
            GamepadButton::LeftStickClick => "LS",
            GamepadButton::RightStickClick => "RS",
        })
    }

    fn nintendo_glyph(button: GamepadButton) -> Glyph {
        Glyph(match button {
            GamepadButton::South => "B",
            GamepadButton::East => "A",
            GamepadButton::West => "Y",
            GamepadButton::North => "X",
            GamepadButton::LeftBumper => "L",
            GamepadButton::RightBumper => "R",
            GamepadButton::LeftTrigger => "ZL",
            GamepadButton::RightTrigger => "ZR",
            GamepadButton::Select => "−",
            GamepadButton::Start => "+",
            GamepadButton::DpadUp => "↑",
            GamepadButton::DpadDown => "↓",
            GamepadButton::DpadLeft => "←",
            GamepadButton::DpadRight => "→",
            GamepadButton::LeftStickClick => "L↓",
            GamepadButton::RightStickClick => "R↓",
        })
    }

    fn generic_glyph(button: GamepadButton) -> Glyph {
        Glyph(match button {
            GamepadButton::South => "Btn1",
            GamepadButton::East => "Btn2",
            GamepadButton::West => "Btn3",
            GamepadButton::North => "Btn4",
            GamepadButton::LeftBumper => "LB",
            GamepadButton::RightBumper => "RB",
            GamepadButton::LeftTrigger => "LT",
            GamepadButton::RightTrigger => "RT",
            GamepadButton::Select => "Sel",
            GamepadButton::Start => "Strt",
            GamepadButton::DpadUp => "↑",
            GamepadButton::DpadDown => "↓",
            GamepadButton::DpadLeft => "←",
            GamepadButton::DpadRight => "→",
            GamepadButton::LeftStickClick => "LS",
            GamepadButton::RightStickClick => "RS",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_resolver_playstation_south() {
        let resolver = GlyphResolver::new(ControllerKind::PlayStation);
        assert_eq!(resolver.resolve(GamepadButton::South).0, "✕");
    }

    #[test]
    fn glyph_resolver_xbox_south() {
        let resolver = GlyphResolver::new(ControllerKind::Xbox);
        assert_eq!(resolver.resolve(GamepadButton::South).0, "A");
    }

    #[test]
    fn glyph_resolver_nintendo_south() {
        let resolver = GlyphResolver::new(ControllerKind::NintendoSwitch);
        // Nintendo South = B
        assert_eq!(resolver.resolve(GamepadButton::South).0, "B");
    }

    #[test]
    fn glyph_resolver_generic_south() {
        let resolver = GlyphResolver::new(ControllerKind::Generic);
        assert_eq!(resolver.resolve(GamepadButton::South).0, "Btn1");
    }

    #[test]
    fn glyph_resolver_xbox_triggers() {
        let resolver = GlyphResolver::new(ControllerKind::Xbox);
        assert_eq!(resolver.resolve(GamepadButton::LeftTrigger).0, "LT");
        assert_eq!(resolver.resolve(GamepadButton::RightTrigger).0, "RT");
    }

    #[test]
    fn glyph_resolver_playstation_bumpers() {
        let resolver = GlyphResolver::new(ControllerKind::PlayStation);
        assert_eq!(resolver.resolve(GamepadButton::LeftBumper).0, "L1");
        assert_eq!(resolver.resolve(GamepadButton::RightBumper).0, "R1");
    }

    #[test]
    fn glyph_resolver_nintendo_triggers() {
        let resolver = GlyphResolver::new(ControllerKind::NintendoSwitch);
        assert_eq!(resolver.resolve(GamepadButton::LeftTrigger).0, "ZL");
        assert_eq!(resolver.resolve(GamepadButton::RightTrigger).0, "ZR");
    }

    #[test]
    fn glyph_resolver_dpad_consistent_across_controllers() {
        let buttons = [
            GamepadButton::DpadUp,
            GamepadButton::DpadDown,
            GamepadButton::DpadLeft,
            GamepadButton::DpadRight,
        ];
        let glyphs = ["↑", "↓", "←", "→"];
        let kinds = [
            ControllerKind::PlayStation,
            ControllerKind::Xbox,
            ControllerKind::NintendoSwitch,
            ControllerKind::Generic,
        ];
        for kind in kinds {
            let resolver = GlyphResolver::new(kind);
            for (button, expected) in buttons.iter().zip(glyphs.iter()) {
                assert_eq!(
                    resolver.resolve(*button).0,
                    *expected,
                    "D-pad glyph mismatch for {kind:?} / {button:?}"
                );
            }
        }
    }

    #[test]
    fn glyph_resolver_set_controller_kind() {
        let mut resolver = GlyphResolver::new(ControllerKind::PlayStation);
        assert_eq!(resolver.controller_kind(), ControllerKind::PlayStation);
        resolver.set_controller_kind(ControllerKind::Xbox);
        assert_eq!(resolver.controller_kind(), ControllerKind::Xbox);
        assert_eq!(resolver.resolve(GamepadButton::South).0, "A");
    }

    #[test]
    fn glyph_resolver_all_buttons_covered_for_each_kind() {
        let all_buttons = [
            GamepadButton::South,
            GamepadButton::East,
            GamepadButton::West,
            GamepadButton::North,
            GamepadButton::LeftBumper,
            GamepadButton::RightBumper,
            GamepadButton::LeftTrigger,
            GamepadButton::RightTrigger,
            GamepadButton::Select,
            GamepadButton::Start,
            GamepadButton::DpadUp,
            GamepadButton::DpadDown,
            GamepadButton::DpadLeft,
            GamepadButton::DpadRight,
            GamepadButton::LeftStickClick,
            GamepadButton::RightStickClick,
        ];
        let kinds = [
            ControllerKind::PlayStation,
            ControllerKind::Xbox,
            ControllerKind::NintendoSwitch,
            ControllerKind::Generic,
        ];
        for kind in kinds {
            let resolver = GlyphResolver::new(kind);
            for button in all_buttons {
                let glyph = resolver.resolve(button);
                assert!(!glyph.0.is_empty(), "Empty glyph for {kind:?} / {button:?}");
            }
        }
    }

    #[test]
    fn glyph_resolver_start_select_per_family() {
        let cases: &[(ControllerKind, &str, &str)] = &[
            (ControllerKind::PlayStation, "Share", "Options"),
            (ControllerKind::Xbox, "Back", "Menu"),
            (ControllerKind::NintendoSwitch, "−", "+"),
            (ControllerKind::Generic, "Sel", "Strt"),
        ];
        for &(kind, sel, start) in cases {
            let resolver = GlyphResolver::new(kind);
            assert_eq!(
                resolver.resolve(GamepadButton::Select).0,
                sel,
                "Select mismatch for {kind:?}"
            );
            assert_eq!(
                resolver.resolve(GamepadButton::Start).0,
                start,
                "Start mismatch for {kind:?}"
            );
        }
    }
}
