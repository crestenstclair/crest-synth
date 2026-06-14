// path: src/shell/controller_glyph.rs

/// Logical gamepad button identifiers, controller-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// The family of controller whose visual glyphs should be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerType {
    Xbox,
    PlayStation,
    Switch,
    Generic,
}

/// Maps a logical [`GamepadButton`] to the correct visual glyph path for the
/// connected [`ControllerType`].
///
/// # Examples
///
/// ```
/// use crest_synth::shell::controller_glyph::{ControllerGlyph, GamepadButton, ControllerType};
///
/// let glyph = ControllerGlyph::new(GamepadButton::South, ControllerType::Xbox);
/// assert!(!glyph.glyph_path().is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerGlyph {
    button: GamepadButton,
    controller_type: ControllerType,
    glyph_path: String,
}

impl ControllerGlyph {
    /// Construct a [`ControllerGlyph`], deriving `glyph_path` from
    /// `button` and `controller_type`.
    pub fn new(button: GamepadButton, controller_type: ControllerType) -> Self {
        let glyph_path = Self::resolve_path(button, controller_type);
        Self {
            button,
            controller_type,
            glyph_path,
        }
    }

    /// Construct a [`ControllerGlyph`] with an explicit `glyph_path` —
    /// useful for tests or dynamic asset overrides.
    pub fn with_path(
        button: GamepadButton,
        controller_type: ControllerType,
        glyph_path: String,
    ) -> Self {
        Self {
            button,
            controller_type,
            glyph_path,
        }
    }

    /// The logical button this glyph represents.
    pub fn button(&self) -> GamepadButton {
        self.button
    }

    /// The controller family whose art style is used.
    pub fn controller_type(&self) -> ControllerType {
        self.controller_type
    }

    /// Relative asset path to the glyph image.
    pub fn glyph_path(&self) -> &str {
        &self.glyph_path
    }

    /// Resolve the canonical glyph asset path for a button + controller combination.
    fn resolve_path(button: GamepadButton, controller_type: ControllerType) -> String {
        let family = match controller_type {
            ControllerType::Xbox => "xbox",
            ControllerType::PlayStation => "playstation",
            ControllerType::Switch => "switch",
            ControllerType::Generic => "generic",
        };

        let glyph = match button {
            GamepadButton::South => "south",
            GamepadButton::East => "east",
            GamepadButton::West => "west",
            GamepadButton::North => "north",
            GamepadButton::LeftBumper => "left_bumper",
            GamepadButton::RightBumper => "right_bumper",
            GamepadButton::LeftTrigger => "left_trigger",
            GamepadButton::RightTrigger => "right_trigger",
            GamepadButton::Select => "select",
            GamepadButton::Start => "start",
            GamepadButton::Mode => "mode",
            GamepadButton::LeftThumb => "left_thumb",
            GamepadButton::RightThumb => "right_thumb",
            GamepadButton::DPadUp => "dpad_up",
            GamepadButton::DPadDown => "dpad_down",
            GamepadButton::DPadLeft => "dpad_left",
            GamepadButton::DPadRight => "dpad_right",
        };

        format!("assets/glyphs/{}/{}.png", family, glyph)
    }
}

#[cfg(test)]
mod controller_glyph {
    use super::*;

    #[test]
    fn new_derives_glyph_path() {
        let g = ControllerGlyph::new(GamepadButton::South, ControllerType::Xbox);
        assert_eq!(g.glyph_path(), "assets/glyphs/xbox/south.png");
        assert_eq!(g.button(), GamepadButton::South);
        assert_eq!(g.controller_type(), ControllerType::Xbox);
    }

    #[test]
    fn playstation_south_path() {
        let g = ControllerGlyph::new(GamepadButton::South, ControllerType::PlayStation);
        assert_eq!(g.glyph_path(), "assets/glyphs/playstation/south.png");
    }

    #[test]
    fn switch_dpad_up_path() {
        let g = ControllerGlyph::new(GamepadButton::DPadUp, ControllerType::Switch);
        assert_eq!(g.glyph_path(), "assets/glyphs/switch/dpad_up.png");
    }

    #[test]
    fn generic_trigger_path() {
        let g = ControllerGlyph::new(GamepadButton::LeftTrigger, ControllerType::Generic);
        assert_eq!(g.glyph_path(), "assets/glyphs/generic/left_trigger.png");
    }

    #[test]
    fn with_path_override() {
        let g = ControllerGlyph::with_path(
            GamepadButton::North,
            ControllerType::Xbox,
            "custom/y_button.png".to_string(),
        );
        assert_eq!(g.glyph_path(), "custom/y_button.png");
        assert_eq!(g.button(), GamepadButton::North);
    }

    #[test]
    fn all_xbox_buttons_produce_non_empty_paths() {
        let buttons = [
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
            GamepadButton::Mode,
            GamepadButton::LeftThumb,
            GamepadButton::RightThumb,
            GamepadButton::DPadUp,
            GamepadButton::DPadDown,
            GamepadButton::DPadLeft,
            GamepadButton::DPadRight,
        ];
        for button in buttons {
            let g = ControllerGlyph::new(button, ControllerType::Xbox);
            assert!(!g.glyph_path().is_empty(), "path empty for {:?}", button);
        }
    }

    #[test]
    fn all_controller_types_produce_unique_paths() {
        let types = [
            ControllerType::Xbox,
            ControllerType::PlayStation,
            ControllerType::Switch,
            ControllerType::Generic,
        ];
        let mut paths: Vec<String> = types
            .iter()
            .map(|&ct| {
                ControllerGlyph::new(GamepadButton::South, ct)
                    .glyph_path()
                    .to_string()
            })
            .collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), types.len());
    }

    #[test]
    fn glyph_path_contains_family_and_button() {
        let g = ControllerGlyph::new(GamepadButton::RightBumper, ControllerType::PlayStation);
        assert!(g.glyph_path().contains("playstation"));
        assert!(g.glyph_path().contains("right_bumper"));
    }
}
