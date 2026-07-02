// path: src/shell/gamepad_action.rs

//! Mapped gamepad input actions used by the UI/MIDI thread.
//!
//! A `GamepadAction` is the outcome of translating raw gamepad hardware
//! input (buttons, sticks, triggers) into a single, UI-agnostic action.
//! This value object never touches the audio thread directly; it flows
//! through the non-real-time UI/MIDI thread only. Any parameter it
//! ultimately affects must still cross into the audio thread through the
//! ParameterBridge or the EventRing.

use std::fmt;

/// Cardinal direction produced by the d-pad, or by discretizing an analog
/// stick's dominant axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// A single analog stick axis reading, normalized to `[-1.0, 1.0]`.
///
/// Positive `x` is right, positive `y` is up. Values are clamped on
/// construction so downstream consumers never have to re-validate range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StickAxis {
    x: f32,
    y: f32,
}

impl StickAxis {
    /// Builds a stick axis reading, clamping both components to
    /// `[-1.0, 1.0]`. NaN inputs are treated as `0.0` (center/rest
    /// position).
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x: Self::clamp_component(x),
            y: Self::clamp_component(y),
        }
    }

    fn clamp_component(v: f32) -> f32 {
        if v.is_nan() {
            0.0
        } else {
            v.clamp(-1.0, 1.0)
        }
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }
}

/// Which shoulder/trigger pair produced a switch-view or switch-patch
/// action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShoulderSide {
    Left,
    Right,
}

/// A mapped gamepad action: the semantic result of translating raw
/// controller input into an intent the shell UI understands.
///
/// Every UI action in this project must be reachable via gamepad, so this
/// enum is the exhaustive vocabulary of gamepad-driven intents:
/// navigate (d-pad), fine-adjust (left stick), scroll (right stick),
/// select (A), back (B), switch view (triggers), switch patch (bumpers),
/// save session (start), open browser (select).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GamepadAction {
    /// D-pad press: coarse navigation between UI elements.
    Navigate(Direction),
    /// Left stick: fine-adjust the currently focused parameter.
    FineAdjust(StickAxis),
    /// Right stick: scroll a list or view.
    Scroll(StickAxis),
    /// A button: select/confirm the focused element.
    Select,
    /// B button: back/cancel out of the current view.
    Back,
    /// Trigger press: switch the active view.
    SwitchView(ShoulderSide),
    /// Bumper press: switch the active patch.
    SwitchPatch(ShoulderSide),
    /// Start button: save the current session.
    SaveSession,
    /// Select/back button: open the preset browser.
    OpenBrowser,
}

impl fmt::Display for GamepadAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GamepadAction::Navigate(dir) => write!(f, "Navigate({dir:?})"),
            GamepadAction::FineAdjust(axis) => {
                write!(f, "FineAdjust({:.3}, {:.3})", axis.x(), axis.y())
            }
            GamepadAction::Scroll(axis) => write!(f, "Scroll({:.3}, {:.3})", axis.x(), axis.y()),
            GamepadAction::Select => write!(f, "Select"),
            GamepadAction::Back => write!(f, "Back"),
            GamepadAction::SwitchView(side) => write!(f, "SwitchView({side:?})"),
            GamepadAction::SwitchPatch(side) => write!(f, "SwitchPatch({side:?})"),
            GamepadAction::SaveSession => write!(f, "SaveSession"),
            GamepadAction::OpenBrowser => write!(f, "OpenBrowser"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_axis_clamps_out_of_range_values() {
        let axis = StickAxis::new(2.0, -3.0);
        assert_eq!(axis.x(), 1.0);
        assert_eq!(axis.y(), -1.0);
    }

    #[test]
    fn stick_axis_treats_nan_as_center() {
        let axis = StickAxis::new(f32::NAN, f32::NAN);
        assert_eq!(axis.x(), 0.0);
        assert_eq!(axis.y(), 0.0);
    }

    #[test]
    fn stick_axis_preserves_in_range_values() {
        let axis = StickAxis::new(0.42, -0.17);
        assert_eq!(axis.x(), 0.42);
        assert_eq!(axis.y(), -0.17);
    }

    #[test]
    fn navigate_actions_carry_direction() {
        let action = GamepadAction::Navigate(Direction::Up);
        assert_eq!(action, GamepadAction::Navigate(Direction::Up));
        assert_ne!(action, GamepadAction::Navigate(Direction::Down));
    }

    #[test]
    fn display_formats_simple_variants() {
        assert_eq!(GamepadAction::Select.to_string(), "Select");
        assert_eq!(GamepadAction::Back.to_string(), "Back");
        assert_eq!(GamepadAction::SaveSession.to_string(), "SaveSession");
        assert_eq!(GamepadAction::OpenBrowser.to_string(), "OpenBrowser");
    }

    #[test]
    fn display_formats_axis_variants_with_values() {
        let action = GamepadAction::FineAdjust(StickAxis::new(0.5, -0.5));
        assert_eq!(action.to_string(), "FineAdjust(0.500, -0.500)");
    }

    #[test]
    fn distinct_shoulder_sides_are_not_equal() {
        assert_ne!(
            GamepadAction::SwitchView(ShoulderSide::Left),
            GamepadAction::SwitchView(ShoulderSide::Right)
        );
    }
}
