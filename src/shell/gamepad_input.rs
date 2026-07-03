// path: src/shell/gamepad_input.rs

//! Port: `GamepadInput`.
//!
//! Hexagonal port for reading gamepad hardware from the UI/MIDI thread.
//! This module defines the abstraction only — no adapter (e.g. a `gilrs`
//! backed implementation) lives here. Concrete adapters implement
//! [`GamepadInput`] and are injected into whatever consumes gamepad
//! actions (constructor injection, never `new`'d internally by a
//! consumer).
//!
//! `poll` is called from the UI/MIDI thread, never from the real-time
//! audio callback. It is free to allocate (the returned `Vec` is heap
//! memory) because this boundary is explicitly outside the audio
//! thread's hard-real-time constraints. Any resulting parameter change
//! must still cross into the audio thread only via the `ParameterBridge`
//! or the `EventRing` — this port never touches audio state directly.

/// A single physical button on a gamepad.
///
/// Every UI action in this project must be reachable via gamepad, so this
/// set intentionally covers the full range of a typical dual-stick
/// controller (e.g. Steam Deck / Xbox-style layout) rather than only the
/// buttons a particular screen happens to use today.
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
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Guide,
}

/// A single analog axis on a gamepad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

/// Newtype for a normalized axis reading.
///
/// Always clamped to `[-1.0, 1.0]` at construction so downstream code
/// never has to re-validate range. Triggers are conventionally reported
/// in `[0.0, 1.0]` by adapters, sticks in `[-1.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisValue(f32);

impl AxisValue {
    /// Constructs an `AxisValue`, clamping out-of-range input rather than
    /// failing — raw hardware reports occasionally jitter past the
    /// nominal range and a dropped input frame is worse than a clamp.
    pub fn new(value: f32) -> Self {
        let clamped = if value.is_nan() {
            0.0
        } else {
            value.clamp(-1.0, 1.0)
        };
        Self(clamped)
    }

    pub fn value(self) -> f32 {
        self.0
    }
}

/// Identifies which physical gamepad an action came from, for the
/// (future) multi-controller case. Adapters that only ever see one
/// gamepad may always report `GamepadId(0)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GamepadId(pub u32);

/// A single discrete gamepad input event, as observed since the last
/// `poll`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GamepadAction {
    ButtonPressed {
        gamepad: GamepadId,
        button: GamepadButton,
    },
    ButtonReleased {
        gamepad: GamepadId,
        button: GamepadButton,
    },
    AxisMoved {
        gamepad: GamepadId,
        axis: GamepadAxis,
        value: AxisValue,
    },
    Connected {
        gamepad: GamepadId,
    },
    Disconnected {
        gamepad: GamepadId,
    },
}

/// Port for polling gamepad hardware.
///
/// Implementations live in an adapter crate/module (e.g. wrapping
/// `gilrs`) and are handed to consumers via constructor injection —
/// consumers depend on this trait, never on a concrete adapter type, so
/// they remain testable with a stub implementation and substitutable per
/// Liskov (any conforming implementation must be usable wherever
/// `GamepadInput` is expected).
///
/// `poll` must be non-blocking: it drains whatever input events have
/// already arrived and returns immediately, it never waits on hardware.
pub trait GamepadInput {
    /// Returns every gamepad action observed since the previous call.
    /// Returns an empty vector when nothing has changed. Order reflects
    /// the order actions were observed.
    fn poll(&mut self) -> Vec<GamepadAction>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal stub adapter for exercising consumers of
    /// `GamepadInput` without real hardware or an external crate.
    struct StubGamepadInput {
        queued: Vec<Vec<GamepadAction>>,
    }

    impl StubGamepadInput {
        fn new(batches: Vec<Vec<GamepadAction>>) -> Self {
            // reverse so we can `pop` in FIFO order
            let mut queued = batches;
            queued.reverse();
            Self { queued }
        }
    }

    impl GamepadInput for StubGamepadInput {
        fn poll(&mut self) -> Vec<GamepadAction> {
            self.queued.pop().unwrap_or_default()
        }
    }

    #[test]
    fn poll_with_no_events_returns_empty() {
        let mut stub = StubGamepadInput::new(vec![]);
        assert!(stub.poll().is_empty());
    }

    #[test]
    fn poll_returns_queued_button_events_in_order() {
        let gamepad = GamepadId(0);
        let batch = vec![
            GamepadAction::ButtonPressed {
                gamepad,
                button: GamepadButton::South,
            },
            GamepadAction::ButtonReleased {
                gamepad,
                button: GamepadButton::South,
            },
        ];
        let mut stub = StubGamepadInput::new(vec![batch.clone()]);
        assert_eq!(stub.poll(), batch);
        assert!(stub.poll().is_empty());
    }

    #[test]
    fn poll_drains_successive_batches_independently() {
        let gamepad = GamepadId(1);
        let first = vec![GamepadAction::Connected { gamepad }];
        let second = vec![GamepadAction::AxisMoved {
            gamepad,
            axis: GamepadAxis::LeftStickX,
            value: AxisValue::new(0.5),
        }];
        let mut stub = StubGamepadInput::new(vec![first.clone(), second.clone()]);
        assert_eq!(stub.poll(), first);
        assert_eq!(stub.poll(), second);
        assert!(stub.poll().is_empty());
    }

    #[test]
    fn axis_value_clamps_out_of_range_input() {
        assert_eq!(AxisValue::new(2.0).value(), 1.0);
        assert_eq!(AxisValue::new(-2.0).value(), -1.0);
        assert_eq!(AxisValue::new(0.25).value(), 0.25);
    }

    #[test]
    fn axis_value_treats_nan_as_neutral() {
        assert_eq!(AxisValue::new(f32::NAN).value(), 0.0);
    }
}
