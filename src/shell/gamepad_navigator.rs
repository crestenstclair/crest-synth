// path: src/shell/gamepad_navigator.rs

//! `GamepadNavigator`: translates raw `GamepadInput` events into the
//! mapped `GamepadAction` vocabulary that drives the cursor/edit model.
//!
//! This lives entirely on the non-real-time UI/MIDI thread: it owns a
//! `GamepadInput` port (injected via constructor, never constructed
//! internally, per Dependency Inversion) and is free to allocate while
//! translating raw hardware events into the app's semantic vocabulary.
//! Nothing here touches the audio thread directly — any parameter change
//! that ultimately results from a mapped action still has to cross into
//! the audio thread only via the `ParameterBridge` or the `EventRing`,
//! exactly like every other UI-side action.
//!
//! ## A note on naming
//!
//! The raw hardware event and the mapped, translated event are BOTH
//! named `GamepadAction`, but they are distinct types in distinct
//! modules:
//! - [`crate::shell::gamepad_input::GamepadAction`] — raw hardware events
//!   (`ButtonPressed`, `AxisMoved`, `Connected`, ...) as reported by the
//!   `GamepadInput` port.
//! - [`crate::shell::gamepad_action::GamepadAction`] — the mapped,
//!   UI-facing vocabulary (`Navigate`, `Select`, `SwitchView`, ...) this
//!   navigator produces.
//!
//! All gamepad navigation uses the app's own cursor/edit model rather
//! than any generic focus system: this navigator's whole job is to
//! produce the semantic events that model consumes, never raw device
//! input.

use std::collections::HashMap;

use crate::shell::gamepad_action::{
    Direction, GamepadAction as MappedAction, ShoulderSide, StickAxis,
};
use crate::shell::gamepad_input::{
    GamepadAction as RawAction, GamepadAxis, GamepadButton, GamepadId, GamepadInput,
};

/// Last-known reading of both axes of one analog stick.
///
/// Hardware adapters report `x` and `y` as independent `AxisMoved`
/// events, so the navigator must remember the other component to emit a
/// complete `StickAxis` reading whenever either one changes.
#[derive(Debug, Clone, Copy, Default)]
struct StickState {
    x: f32,
    y: f32,
}

/// Per-gamepad analog stick state tracked between polls.
#[derive(Debug, Clone, Copy, Default)]
struct GamepadAxisState {
    left: StickState,
    right: StickState,
}

/// Translates raw gamepad hardware input into the app's mapped
/// `GamepadAction` vocabulary.
///
/// Depends on the `GamepadInput` port — an abstraction — never a
/// concrete adapter. The concrete adapter (e.g. `GilrsGamepadInput`) is
/// injected through the constructor, so this type is testable with a
/// stub and substitutable with any conforming implementation (Liskov).
pub struct GamepadNavigator<I: GamepadInput> {
    input: I,
    axis_state: HashMap<GamepadId, GamepadAxisState>,
}

impl<I: GamepadInput> GamepadNavigator<I> {
    /// Builds a navigator over the given `GamepadInput` port.
    pub fn new(input: I) -> Self {
        Self {
            input,
            axis_state: HashMap::new(),
        }
    }

    /// Polls the underlying `GamepadInput` port and translates every raw
    /// event observed since the last call into zero or more mapped
    /// `GamepadAction`s, preserving observation order.
    ///
    /// Not every raw event has a mapped counterpart — button releases,
    /// stick/trigger-click buttons, `Connected`/`Disconnected`, and
    /// trigger axes are silently dropped, since the mapped vocabulary
    /// only covers actions the cursor/edit model actually acts on.
    pub fn poll_actions(&mut self) -> Vec<MappedAction> {
        let raw_actions = self.input.poll();
        let mut mapped = Vec::with_capacity(raw_actions.len());
        for raw in raw_actions {
            if let Some(action) = self.translate(raw) {
                mapped.push(action);
            }
        }
        mapped
    }

    fn translate(&mut self, raw: RawAction) -> Option<MappedAction> {
        match raw {
            RawAction::ButtonPressed { button, .. } => Self::translate_button(button),
            RawAction::AxisMoved {
                gamepad,
                axis,
                value,
            } => self.translate_axis(gamepad, axis, value.value()),
            RawAction::ButtonReleased { .. }
            | RawAction::Connected { .. }
            | RawAction::Disconnected { .. } => None,
        }
    }

    fn translate_button(button: GamepadButton) -> Option<MappedAction> {
        match button {
            GamepadButton::DPadUp => Some(MappedAction::Navigate(Direction::Up)),
            GamepadButton::DPadDown => Some(MappedAction::Navigate(Direction::Down)),
            GamepadButton::DPadLeft => Some(MappedAction::Navigate(Direction::Left)),
            GamepadButton::DPadRight => Some(MappedAction::Navigate(Direction::Right)),
            GamepadButton::South => Some(MappedAction::Select),
            GamepadButton::East => Some(MappedAction::Back),
            GamepadButton::LeftTrigger => Some(MappedAction::SwitchView(ShoulderSide::Left)),
            GamepadButton::RightTrigger => Some(MappedAction::SwitchView(ShoulderSide::Right)),
            GamepadButton::LeftBumper => Some(MappedAction::SwitchPatch(ShoulderSide::Left)),
            GamepadButton::RightBumper => Some(MappedAction::SwitchPatch(ShoulderSide::Right)),
            GamepadButton::Start => Some(MappedAction::SaveSession),
            GamepadButton::Select => Some(MappedAction::OpenBrowser),
            GamepadButton::North
            | GamepadButton::West
            | GamepadButton::LeftStick
            | GamepadButton::RightStick
            | GamepadButton::Guide => None,
        }
    }

    fn translate_axis(
        &mut self,
        gamepad: GamepadId,
        axis: GamepadAxis,
        value: f32,
    ) -> Option<MappedAction> {
        let state = self.axis_state.entry(gamepad).or_default();
        match axis {
            GamepadAxis::LeftStickX => {
                state.left.x = value;
                Some(MappedAction::FineAdjust(StickAxis::new(
                    state.left.x,
                    state.left.y,
                )))
            }
            GamepadAxis::LeftStickY => {
                state.left.y = value;
                Some(MappedAction::FineAdjust(StickAxis::new(
                    state.left.x,
                    state.left.y,
                )))
            }
            GamepadAxis::RightStickX => {
                state.right.x = value;
                Some(MappedAction::Scroll(StickAxis::new(
                    state.right.x,
                    state.right.y,
                )))
            }
            GamepadAxis::RightStickY => {
                state.right.y = value;
                Some(MappedAction::Scroll(StickAxis::new(
                    state.right.x,
                    state.right.y,
                )))
            }
            GamepadAxis::LeftTrigger | GamepadAxis::RightTrigger => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::gamepad_input::AxisValue;

    /// A minimal stub adapter for exercising `GamepadNavigator` without
    /// real hardware or an external crate.
    struct StubGamepadInput {
        queued: Vec<Vec<RawAction>>,
    }

    impl StubGamepadInput {
        fn new(batches: Vec<Vec<RawAction>>) -> Self {
            let mut queued = batches;
            queued.reverse();
            Self { queued }
        }
    }

    impl GamepadInput for StubGamepadInput {
        fn poll(&mut self) -> Vec<RawAction> {
            self.queued.pop().unwrap_or_default()
        }
    }

    #[test]
    fn poll_actions_with_no_events_returns_empty() {
        let mut navigator = GamepadNavigator::new(StubGamepadInput::new(vec![]));
        assert!(navigator.poll_actions().is_empty());
    }

    #[test]
    fn dpad_press_maps_to_navigate() {
        let gamepad = GamepadId(0);
        let batch = vec![RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::DPadUp,
        }];
        let mut navigator = GamepadNavigator::new(StubGamepadInput::new(vec![batch]));
        assert_eq!(
            navigator.poll_actions(),
            vec![MappedAction::Navigate(Direction::Up)]
        );
    }

    #[test]
    fn south_button_maps_to_select_and_east_maps_to_back() {
        let gamepad = GamepadId(0);
        let batch = vec![
            RawAction::ButtonPressed {
                gamepad,
                button: GamepadButton::South,
            },
            RawAction::ButtonPressed {
                gamepad,
                button: GamepadButton::East,
            },
        ];
        let mut navigator = GamepadNavigator::new(StubGamepadInput::new(vec![batch]));
        assert_eq!(
            navigator.poll_actions(),
            vec![MappedAction::Select, MappedAction::Back]
        );
    }

    #[test]
    fn triggers_map_to_switch_view_by_side() {
        let gamepad = GamepadId(0);
        let batch = vec![
            RawAction::ButtonPressed {
                gamepad,
                button: GamepadButton::LeftTrigger,
            },
            RawAction::ButtonPressed {
                gamepad,
                button: GamepadButton::RightTrigger,
            },
        ];
        let mut navigator = GamepadNavigator::new(StubGamepadInput::new(vec![batch]));
        assert_eq!(
            navigator.poll_actions(),
            vec![
                MappedAction::SwitchView(ShoulderSide::Left),
                MappedAction::SwitchView(ShoulderSide::Right),
            ]
        );
    }

    #[test]
    fn bumpers_map_to_switch_patch_by_side() {
        let gamepad = GamepadId(0);
        let batch = vec![
            RawAction::ButtonPressed {
                gamepad,
                button: GamepadButton::LeftBumper,
            },
            RawAction::ButtonPressed {
                gamepad,
                button: GamepadButton::RightBumper,
            },
        ];
        let mut navigator = GamepadNavigator::new(StubGamepadInput::new(vec![batch]));
        assert_eq!(
            navigator.poll_actions(),
            vec![
                MappedAction::SwitchPatch(ShoulderSide::Left),
                MappedAction::SwitchPatch(ShoulderSide::Right),
            ]
        );
    }

    #[test]
    fn start_maps_to_save_session_and_select_maps_to_open_browser() {
        let gamepad = GamepadId(0);
        let batch = vec![
            RawAction::ButtonPressed {
                gamepad,
                button: GamepadButton::Start,
            },
            RawAction::ButtonPressed {
                gamepad,
                button: GamepadButton::Select,
            },
        ];
        let mut navigator = GamepadNavigator::new(StubGamepadInput::new(vec![batch]));
        assert_eq!(
            navigator.poll_actions(),
            vec![MappedAction::SaveSession, MappedAction::OpenBrowser]
        );
    }

    #[test]
    fn unmapped_buttons_and_lifecycle_events_are_dropped() {
        let gamepad = GamepadId(0);
        let batch = vec![
            RawAction::ButtonPressed {
                gamepad,
                button: GamepadButton::North,
            },
            RawAction::ButtonPressed {
                gamepad,
                button: GamepadButton::Guide,
            },
            RawAction::ButtonReleased {
                gamepad,
                button: GamepadButton::South,
            },
            RawAction::Connected { gamepad },
            RawAction::Disconnected { gamepad },
        ];
        let mut navigator = GamepadNavigator::new(StubGamepadInput::new(vec![batch]));
        assert!(navigator.poll_actions().is_empty());
    }

    #[test]
    fn left_stick_axes_combine_into_fine_adjust() {
        let gamepad = GamepadId(0);
        let batch = vec![
            RawAction::AxisMoved {
                gamepad,
                axis: GamepadAxis::LeftStickX,
                value: AxisValue::new(0.5),
            },
            RawAction::AxisMoved {
                gamepad,
                axis: GamepadAxis::LeftStickY,
                value: AxisValue::new(-0.25),
            },
        ];
        let mut navigator = GamepadNavigator::new(StubGamepadInput::new(vec![batch]));
        let actions = navigator.poll_actions();
        assert_eq!(
            actions,
            vec![
                MappedAction::FineAdjust(StickAxis::new(0.5, 0.0)),
                MappedAction::FineAdjust(StickAxis::new(0.5, -0.25)),
            ]
        );
    }

    #[test]
    fn right_stick_axes_combine_into_scroll() {
        let gamepad = GamepadId(0);
        let batch = vec![
            RawAction::AxisMoved {
                gamepad,
                axis: GamepadAxis::RightStickX,
                value: AxisValue::new(-0.75),
            },
            RawAction::AxisMoved {
                gamepad,
                axis: GamepadAxis::RightStickY,
                value: AxisValue::new(0.6),
            },
        ];
        let mut navigator = GamepadNavigator::new(StubGamepadInput::new(vec![batch]));
        let actions = navigator.poll_actions();
        assert_eq!(
            actions,
            vec![
                MappedAction::Scroll(StickAxis::new(-0.75, 0.0)),
                MappedAction::Scroll(StickAxis::new(-0.75, 0.6)),
            ]
        );
    }

    #[test]
    fn trigger_axes_are_dropped() {
        let gamepad = GamepadId(0);
        let batch = vec![
            RawAction::AxisMoved {
                gamepad,
                axis: GamepadAxis::LeftTrigger,
                value: AxisValue::new(1.0),
            },
            RawAction::AxisMoved {
                gamepad,
                axis: GamepadAxis::RightTrigger,
                value: AxisValue::new(1.0),
            },
        ];
        let mut navigator = GamepadNavigator::new(StubGamepadInput::new(vec![batch]));
        assert!(navigator.poll_actions().is_empty());
    }

    #[test]
    fn axis_state_is_tracked_independently_per_gamepad() {
        let gamepad_a = GamepadId(0);
        let gamepad_b = GamepadId(1);
        let batch = vec![
            RawAction::AxisMoved {
                gamepad: gamepad_a,
                axis: GamepadAxis::LeftStickX,
                value: AxisValue::new(1.0),
            },
            RawAction::AxisMoved {
                gamepad: gamepad_b,
                axis: GamepadAxis::LeftStickY,
                value: AxisValue::new(-1.0),
            },
        ];
        let mut navigator = GamepadNavigator::new(StubGamepadInput::new(vec![batch]));
        let actions = navigator.poll_actions();
        assert_eq!(
            actions,
            vec![
                MappedAction::FineAdjust(StickAxis::new(1.0, 0.0)),
                MappedAction::FineAdjust(StickAxis::new(0.0, -1.0)),
            ]
        );
    }
}
