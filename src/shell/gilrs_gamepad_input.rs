// path: src/shell/gilrs_gamepad_input.rs

//! Adapter: `GilrsGamepadInput`.
//!
//! Concrete implementation of `port::Shell::GamepadInput` backed by the
//! `gilrs` crate. This adapter is constructed and polled exclusively on
//! the UI/MIDI thread; the real-time audio thread never touches it
//! directly. `poll` translates buffered gilrs events into
//! `GamepadAction`s and is non-blocking: gilrs queues pending events
//! internally, and draining them via `next_event` never waits on
//! hardware.

use crate::shell::gamepad_input::{
    AxisValue, GamepadAction, GamepadAxis, GamepadButton, GamepadId, GamepadInput,
};

/// Gilrs-backed implementation of the `GamepadInput` port.
///
/// Holds the `gilrs::Gilrs` context as its sole dependency. There is
/// nothing meaningful to inject in its place — gilrs itself is the
/// boundary to the OS gamepad subsystem — so `new` owns creating that
/// context, matching the "convenience constructor" shape used elsewhere:
/// callers that need a substitute (e.g. tests) depend on the
/// `GamepadInput` trait and inject a stub instead of this adapter.
pub struct GilrsGamepadInput {
    gilrs: gilrs::Gilrs,
}

impl GilrsGamepadInput {
    /// Constructs a new gilrs-backed gamepad input adapter.
    ///
    /// Fails if gilrs cannot initialize its connection to the platform
    /// gamepad subsystem. The error is boxed to keep `Result`'s stack
    /// footprint small — `gilrs::Error` itself is large enough to trip
    /// `clippy::result_large_err` when returned unboxed.
    pub fn new() -> Result<Self, Box<gilrs::Error>> {
        let gilrs = gilrs::Gilrs::new().map_err(Box::new)?;
        Ok(Self { gilrs })
    }

    fn gamepad_id(id: gilrs::GamepadId) -> GamepadId {
        let index: usize = id.into();
        GamepadId(index as u32)
    }

    /// Maps a gilrs button to our domain `GamepadButton`. Buttons gilrs
    /// reports that have no domain meaning here (e.g. `Unknown`) are
    /// dropped rather than surfaced.
    fn map_button(button: gilrs::Button) -> Option<GamepadButton> {
        match button {
            gilrs::Button::South => Some(GamepadButton::South),
            gilrs::Button::East => Some(GamepadButton::East),
            gilrs::Button::North => Some(GamepadButton::North),
            gilrs::Button::West => Some(GamepadButton::West),
            gilrs::Button::LeftTrigger => Some(GamepadButton::LeftBumper),
            gilrs::Button::LeftTrigger2 => Some(GamepadButton::LeftTrigger),
            gilrs::Button::RightTrigger => Some(GamepadButton::RightBumper),
            gilrs::Button::RightTrigger2 => Some(GamepadButton::RightTrigger),
            gilrs::Button::Select => Some(GamepadButton::Select),
            gilrs::Button::Start => Some(GamepadButton::Start),
            gilrs::Button::Mode => Some(GamepadButton::Guide),
            gilrs::Button::LeftThumb => Some(GamepadButton::LeftStick),
            gilrs::Button::RightThumb => Some(GamepadButton::RightStick),
            gilrs::Button::DPadUp => Some(GamepadButton::DPadUp),
            gilrs::Button::DPadDown => Some(GamepadButton::DPadDown),
            gilrs::Button::DPadLeft => Some(GamepadButton::DPadLeft),
            gilrs::Button::DPadRight => Some(GamepadButton::DPadRight),
            gilrs::Button::C | gilrs::Button::Z | gilrs::Button::Unknown => None,
        }
    }

    /// Maps a gilrs axis to our domain `GamepadAxis`. `LeftZ`/`RightZ`
    /// are the analog-trigger axes on gamepads that report triggers as
    /// axes rather than as analog buttons.
    fn map_axis(axis: gilrs::Axis) -> Option<GamepadAxis> {
        match axis {
            gilrs::Axis::LeftStickX => Some(GamepadAxis::LeftStickX),
            gilrs::Axis::LeftStickY => Some(GamepadAxis::LeftStickY),
            gilrs::Axis::RightStickX => Some(GamepadAxis::RightStickX),
            gilrs::Axis::RightStickY => Some(GamepadAxis::RightStickY),
            gilrs::Axis::LeftZ => Some(GamepadAxis::LeftTrigger),
            gilrs::Axis::RightZ => Some(GamepadAxis::RightTrigger),
            gilrs::Axis::DPadX | gilrs::Axis::DPadY | gilrs::Axis::Unknown => None,
        }
    }

    /// Translates a single gilrs event into zero or one `GamepadAction`.
    /// Events this adapter has no domain mapping for (an unmapped
    /// button/axis, or gilrs's own bookkeeping events such as `Dropped`)
    /// are dropped rather than surfaced, keeping downstream consumers
    /// free of adapter-specific noise.
    fn translate(gamepad: GamepadId, event: gilrs::EventType) -> Option<GamepadAction> {
        match event {
            gilrs::EventType::ButtonPressed(button, _) => Self::map_button(button)
                .map(|button| GamepadAction::ButtonPressed { gamepad, button }),
            gilrs::EventType::ButtonReleased(button, _) => Self::map_button(button)
                .map(|button| GamepadAction::ButtonReleased { gamepad, button }),
            gilrs::EventType::ButtonChanged(button, value, _) => {
                Self::map_button(button).and_then(|mapped| match mapped {
                    GamepadButton::LeftTrigger => Some(GamepadAction::AxisMoved {
                        gamepad,
                        axis: GamepadAxis::LeftTrigger,
                        value: AxisValue::new(value),
                    }),
                    GamepadButton::RightTrigger => Some(GamepadAction::AxisMoved {
                        gamepad,
                        axis: GamepadAxis::RightTrigger,
                        value: AxisValue::new(value),
                    }),
                    _ => None,
                })
            }
            gilrs::EventType::AxisChanged(axis, value, _) => {
                Self::map_axis(axis).map(|axis| GamepadAction::AxisMoved {
                    gamepad,
                    axis,
                    value: AxisValue::new(value),
                })
            }
            gilrs::EventType::Connected => Some(GamepadAction::Connected { gamepad }),
            gilrs::EventType::Disconnected => Some(GamepadAction::Disconnected { gamepad }),
            _ => None,
        }
    }
}

impl GamepadInput for GilrsGamepadInput {
    fn poll(&mut self) -> Vec<GamepadAction> {
        let mut actions = Vec::new();
        while let Some(event) = self.gilrs.next_event() {
            let gamepad = Self::gamepad_id(event.id);
            if let Some(action) = Self::translate(gamepad, event.event) {
                actions.push(action);
            }
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_button_covers_known_buttons() {
        assert_eq!(
            GilrsGamepadInput::map_button(gilrs::Button::South),
            Some(GamepadButton::South)
        );
        assert_eq!(
            GilrsGamepadInput::map_button(gilrs::Button::LeftTrigger2),
            Some(GamepadButton::LeftTrigger)
        );
        assert_eq!(
            GilrsGamepadInput::map_button(gilrs::Button::RightTrigger),
            Some(GamepadButton::RightBumper)
        );
        assert_eq!(
            GilrsGamepadInput::map_button(gilrs::Button::Mode),
            Some(GamepadButton::Guide)
        );
        assert_eq!(
            GilrsGamepadInput::map_button(gilrs::Button::DPadUp),
            Some(GamepadButton::DPadUp)
        );
    }

    #[test]
    fn map_button_returns_none_for_unmapped() {
        assert_eq!(GilrsGamepadInput::map_button(gilrs::Button::Unknown), None);
        assert_eq!(GilrsGamepadInput::map_button(gilrs::Button::C), None);
    }

    #[test]
    fn map_axis_covers_known_axes() {
        assert_eq!(
            GilrsGamepadInput::map_axis(gilrs::Axis::LeftStickX),
            Some(GamepadAxis::LeftStickX)
        );
        assert_eq!(
            GilrsGamepadInput::map_axis(gilrs::Axis::RightZ),
            Some(GamepadAxis::RightTrigger)
        );
        assert_eq!(
            GilrsGamepadInput::map_axis(gilrs::Axis::LeftZ),
            Some(GamepadAxis::LeftTrigger)
        );
    }

    #[test]
    fn map_axis_returns_none_for_unmapped() {
        assert_eq!(GilrsGamepadInput::map_axis(gilrs::Axis::Unknown), None);
        assert_eq!(GilrsGamepadInput::map_axis(gilrs::Axis::DPadX), None);
    }

    #[test]
    fn translate_connected_event_produces_connected_action() {
        let gamepad = GamepadId(3);
        let action = GilrsGamepadInput::translate(gamepad, gilrs::EventType::Connected);
        assert_eq!(action, Some(GamepadAction::Connected { gamepad }));
    }

    #[test]
    fn translate_disconnected_event_produces_disconnected_action() {
        let gamepad = GamepadId(2);
        let action = GilrsGamepadInput::translate(gamepad, gilrs::EventType::Disconnected);
        assert_eq!(action, Some(GamepadAction::Disconnected { gamepad }));
    }
}
