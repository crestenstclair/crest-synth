// path: src/shell/controller_glyph.rs

//! Maps a logical gamepad button to the correct visual glyph path for the
//! connected controller type.
//!
//! This is a pure value object: given a `GamepadButton` and a
//! `ControllerType`, it resolves the path to the glyph asset that should be
//! drawn for that button on that controller. It performs no I/O and touches
//! no rendering backend — the shell's draw code is responsible for loading
//! and rendering the asset at the resolved path.

/// A logical button on a gamepad, independent of the physical controller
/// brand that produced the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    South,
    East,
    West,
    North,
    LeftShoulder,
    RightShoulder,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// The family of physical controller connected, used to pick the correct
/// glyph art (button legends differ across controller brands).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerType {
    Xbox,
    PlayStation,
    SteamDeck,
    Generic,
}

/// Maps a logical `GamepadButton`, for a given `ControllerType`, to the
/// filesystem path of the glyph asset that represents it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerGlyph {
    button: GamepadButton,
    controller_type: ControllerType,
    glyph_path: String,
}

impl ControllerGlyph {
    /// Resolves the glyph for `button` on `controller_type`.
    ///
    /// The resulting `glyph_path` is a stable, deterministic asset path
    /// derived from the controller family and button identity, e.g.
    /// `assets/glyphs/xbox/south.svg`.
    pub fn resolve(button: GamepadButton, controller_type: ControllerType) -> Self {
        let glyph_path = Self::path_for(button, controller_type);
        Self {
            button,
            controller_type,
            glyph_path,
        }
    }

    /// Constructs a `ControllerGlyph` from an explicit, pre-resolved path.
    ///
    /// Intended for tests and for callers that source glyph paths from a
    /// data-driven asset manifest rather than the built-in convention.
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

    pub fn button(&self) -> GamepadButton {
        self.button
    }

    pub fn controller_type(&self) -> ControllerType {
        self.controller_type
    }

    pub fn glyph_path(&self) -> &str {
        &self.glyph_path
    }

    fn path_for(button: GamepadButton, controller_type: ControllerType) -> String {
        format!(
            "assets/glyphs/{}/{}.svg",
            Self::controller_dir(controller_type),
            Self::button_slug(button)
        )
    }

    fn controller_dir(controller_type: ControllerType) -> &'static str {
        match controller_type {
            ControllerType::Xbox => "xbox",
            ControllerType::PlayStation => "playstation",
            ControllerType::SteamDeck => "steam_deck",
            ControllerType::Generic => "generic",
        }
    }

    fn button_slug(button: GamepadButton) -> &'static str {
        match button {
            GamepadButton::South => "south",
            GamepadButton::East => "east",
            GamepadButton::West => "west",
            GamepadButton::North => "north",
            GamepadButton::LeftShoulder => "left_shoulder",
            GamepadButton::RightShoulder => "right_shoulder",
            GamepadButton::LeftTrigger => "left_trigger",
            GamepadButton::RightTrigger => "right_trigger",
            GamepadButton::Select => "select",
            GamepadButton::Start => "start",
            GamepadButton::LeftStick => "left_stick",
            GamepadButton::RightStick => "right_stick",
            GamepadButton::DPadUp => "dpad_up",
            GamepadButton::DPadDown => "dpad_down",
            GamepadButton::DPadLeft => "dpad_left",
            GamepadButton::DPadRight => "dpad_right",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_builds_deterministic_path_per_controller() {
        let xbox = ControllerGlyph::resolve(GamepadButton::South, ControllerType::Xbox);
        let ps = ControllerGlyph::resolve(GamepadButton::South, ControllerType::PlayStation);
        let deck = ControllerGlyph::resolve(GamepadButton::South, ControllerType::SteamDeck);

        assert_eq!(xbox.glyph_path(), "assets/glyphs/xbox/south.svg");
        assert_eq!(ps.glyph_path(), "assets/glyphs/playstation/south.svg");
        assert_eq!(deck.glyph_path(), "assets/glyphs/steam_deck/south.svg");
    }

    #[test]
    fn resolve_distinguishes_every_logical_button() {
        let buttons = [
            GamepadButton::South,
            GamepadButton::East,
            GamepadButton::West,
            GamepadButton::North,
            GamepadButton::LeftShoulder,
            GamepadButton::RightShoulder,
            GamepadButton::LeftTrigger,
            GamepadButton::RightTrigger,
            GamepadButton::Select,
            GamepadButton::Start,
            GamepadButton::LeftStick,
            GamepadButton::RightStick,
            GamepadButton::DPadUp,
            GamepadButton::DPadDown,
            GamepadButton::DPadLeft,
            GamepadButton::DPadRight,
        ];

        let mut paths: Vec<String> = buttons
            .iter()
            .map(|&button| {
                ControllerGlyph::resolve(button, ControllerType::Generic)
                    .glyph_path()
                    .to_string()
            })
            .collect();
        paths.sort();
        paths.dedup();

        assert_eq!(paths.len(), buttons.len());
    }

    #[test]
    fn accessors_expose_the_resolved_fields() {
        let glyph = ControllerGlyph::resolve(GamepadButton::Start, ControllerType::SteamDeck);

        assert_eq!(glyph.button(), GamepadButton::Start);
        assert_eq!(glyph.controller_type(), ControllerType::SteamDeck);
        assert_eq!(glyph.glyph_path(), "assets/glyphs/steam_deck/start.svg");
    }

    #[test]
    fn with_path_allows_overriding_the_convention() {
        let glyph = ControllerGlyph::with_path(
            GamepadButton::East,
            ControllerType::PlayStation,
            "assets/custom/circle.png".to_string(),
        );

        assert_eq!(glyph.glyph_path(), "assets/custom/circle.png");
        assert_eq!(glyph.button(), GamepadButton::East);
        assert_eq!(glyph.controller_type(), ControllerType::PlayStation);
    }
}
