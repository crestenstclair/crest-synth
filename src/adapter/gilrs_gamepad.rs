// path: src/adapter/gilrs_gamepad.rs

//! Infrastructure adapter for gamepad input using the `gilrs` crate.
//!
//! [`GilrsGamepad`] implements the [`GamepadInput`] port by delegating to
//! `gilrs` for cross-platform controller discovery and event delivery.
//!
//! # Design
//!
//! * All types exposed (`ControllerId`, `ControllerType`, `GamepadEvent`) come
//!   from the shell port — the adapter has no domain logic.
//! * Event mapping is non-blocking: `gilrs::Gilrs::next_event` never blocks.
//! * The adapter is dependency-injectable via [`GilrsGamepad::with_gilrs`],
//!   which accepts a pre-constructed `gilrs::Gilrs`; the convenience constructor
//!   [`GilrsGamepad::new`] initialises one internally.

use crate::shell::gamepad_input::{
    AxisId, ButtonId, ControllerId, ControllerType, GamepadEvent, GamepadInput,
};

// ── GilrsGamepad ──────────────────────────────────────────────────────────────

/// Infrastructure adapter that backs the [`GamepadInput`] port with `gilrs`.
///
/// `gilrs` manages platform-specific controller enumeration and HID event
/// delivery.  This adapter maps `gilrs` types to the shell's value types so
/// that higher-level code remains back-end agnostic.
///
/// # Controller IDs
///
/// Each `gilrs::GamepadId` is mapped to a [`ControllerId`] by converting the
/// underlying `usize` index to `u64`.  The mapping is stable for the lifetime
/// of this adapter.
pub struct GilrsGamepad {
    gilrs: gilrs::Gilrs,
}

impl GilrsGamepad {
    /// Create a new adapter by initialising `gilrs` internally.
    ///
    /// Returns an error string when the gamepad subsystem cannot be
    /// initialised (e.g. the host has no HID back-end).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crest_synth::adapter::gilrs_gamepad::GilrsGamepad;
    ///
    /// if let Ok(mut pad) = GilrsGamepad::new() {
    ///     // use pad …
    /// }
    /// ```
    pub fn new() -> Result<Self, String> {
        let gilrs = gilrs::Gilrs::new().map_err(|e| format!("gilrs init: {e}"))?;
        Ok(Self::with_gilrs(gilrs))
    }

    /// Create an adapter from a pre-constructed `gilrs::Gilrs` instance.
    ///
    /// Useful for injecting a specific `Gilrs` configuration in tests or
    /// specialised environments.
    pub fn with_gilrs(gilrs: gilrs::Gilrs) -> Self {
        Self { gilrs }
    }
}

impl GamepadInput for GilrsGamepad {
    /// Returns IDs for all currently connected controllers.
    fn connected_controllers(&self) -> Vec<ControllerId> {
        self.gilrs
            .gamepads()
            .map(|(id, _)| ControllerId(usize::from(id) as u64))
            .collect()
    }

    /// Returns the type of the given controller.
    ///
    /// `gilrs` 0.10 does not expose a typed gamepad-category enum, so all
    /// recognised controllers are reported as [`ControllerType::Gamepad`].
    /// Unrecognised IDs yield [`ControllerType::Unknown`].
    fn controller_type(&self, id: ControllerId) -> ControllerType {
        for (gid, _) in self.gilrs.gamepads() {
            if usize::from(gid) as u64 == id.0 {
                return ControllerType::Gamepad;
            }
        }
        ControllerType::Unknown
    }

    /// Drain all pending gilrs events since the last call.  Never blocks.
    fn poll(&mut self) -> Vec<GamepadEvent> {
        let mut events = Vec::new();
        while let Some(gilrs::Event { id, event, .. }) = self.gilrs.next_event() {
            let controller = ControllerId(usize::from(id) as u64);
            match event {
                gilrs::EventType::ButtonPressed(btn, _) => {
                    events.push(GamepadEvent::ButtonPressed {
                        controller,
                        button: map_button(btn),
                    });
                }
                gilrs::EventType::ButtonReleased(btn, _) => {
                    events.push(GamepadEvent::ButtonReleased {
                        controller,
                        button: map_button(btn),
                    });
                }
                gilrs::EventType::AxisChanged(axis, value, _) => {
                    events.push(GamepadEvent::AxisChanged {
                        controller,
                        axis: map_axis(axis),
                        value,
                    });
                }
                gilrs::EventType::Connected => {
                    events.push(GamepadEvent::ControllerConnected { controller });
                }
                gilrs::EventType::Disconnected => {
                    events.push(GamepadEvent::ControllerDisconnected { controller });
                }
                _ => {} // ButtonChanged, ButtonRepeated, Dropped — ignored
            }
        }
        events
    }
}

// ── Mapping helpers ───────────────────────────────────────────────────────────

fn map_button(btn: gilrs::Button) -> ButtonId {
    match btn {
        gilrs::Button::South => ButtonId::South,
        gilrs::Button::East => ButtonId::East,
        gilrs::Button::West => ButtonId::West,
        gilrs::Button::North => ButtonId::North,
        gilrs::Button::LeftTrigger => ButtonId::LeftBumper,
        gilrs::Button::RightTrigger => ButtonId::RightBumper,
        gilrs::Button::LeftTrigger2 => ButtonId::LeftTrigger,
        gilrs::Button::RightTrigger2 => ButtonId::RightTrigger,
        gilrs::Button::Select => ButtonId::Select,
        gilrs::Button::Start => ButtonId::Start,
        gilrs::Button::LeftThumb => ButtonId::LeftThumb,
        gilrs::Button::RightThumb => ButtonId::RightThumb,
        gilrs::Button::DPadUp => ButtonId::DPadUp,
        gilrs::Button::DPadDown => ButtonId::DPadDown,
        gilrs::Button::DPadLeft => ButtonId::DPadLeft,
        gilrs::Button::DPadRight => ButtonId::DPadRight,
        gilrs::Button::Mode => ButtonId::Mode,
        gilrs::Button::Unknown => ButtonId::Other(0),
        _ => ButtonId::Other(0),
    }
}

fn map_axis(axis: gilrs::Axis) -> AxisId {
    match axis {
        gilrs::Axis::LeftStickX => AxisId::LeftStickX,
        gilrs::Axis::LeftStickY => AxisId::LeftStickY,
        gilrs::Axis::RightStickX => AxisId::RightStickX,
        gilrs::Axis::RightStickY => AxisId::RightStickY,
        gilrs::Axis::LeftZ => AxisId::LeftTrigger,
        gilrs::Axis::RightZ => AxisId::RightTrigger,
        gilrs::Axis::DPadX => AxisId::DPadX,
        gilrs::Axis::DPadY => AxisId::DPadY,
        _ => AxisId::Other(0),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::gamepad_input::{
        AxisId, ButtonId, ControllerId, ControllerType, GamepadEvent, GamepadInput,
        StubGamepadInput,
    };

    fn id(n: u64) -> ControllerId {
        ControllerId(n)
    }

    // ── Verify the trait is object-safe ──────────────────────────────────────

    #[test]
    fn gamepad_input_is_object_safe() {
        let stub: Box<dyn GamepadInput> = Box::new(StubGamepadInput::new());
        drop(stub);
    }

    // ── StubGamepadInput — connected_controllers ──────────────────────────────

    #[test]
    fn stub_empty_initially() {
        let stub = StubGamepadInput::new();
        assert!(stub.connected_controllers().is_empty());
    }

    #[test]
    fn stub_add_shows_in_connected() {
        let mut stub = StubGamepadInput::new();
        stub.add_controller(id(1), ControllerType::Gamepad);
        assert!(stub.connected_controllers().contains(&id(1)));
    }

    #[test]
    fn stub_remove_disappears_from_connected() {
        let mut stub = StubGamepadInput::new();
        stub.add_controller(id(2), ControllerType::Gamepad);
        stub.remove_controller(id(2));
        assert!(stub.connected_controllers().is_empty());
    }

    // ── StubGamepadInput — controller_type ───────────────────────────────────

    #[test]
    fn stub_returns_registered_type() {
        let mut stub = StubGamepadInput::new();
        stub.add_controller(id(5), ControllerType::Guitar);
        assert_eq!(stub.controller_type(id(5)), ControllerType::Guitar);
    }

    #[test]
    fn stub_unknown_for_missing_id() {
        let stub = StubGamepadInput::new();
        assert_eq!(stub.controller_type(id(99)), ControllerType::Unknown);
    }

    // ── StubGamepadInput — poll ───────────────────────────────────────────────

    #[test]
    fn stub_poll_empty_initially() {
        let mut stub = StubGamepadInput::new();
        assert!(stub.poll().is_empty());
    }

    #[test]
    fn stub_poll_drains_events_in_order() {
        let mut stub = StubGamepadInput::new();
        let e1 = GamepadEvent::ButtonPressed {
            controller: id(1),
            button: ButtonId::South,
        };
        let e2 = GamepadEvent::ButtonReleased {
            controller: id(1),
            button: ButtonId::South,
        };
        stub.inject(e1.clone());
        stub.inject(e2.clone());

        let events = stub.poll();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], e1);
        assert_eq!(events[1], e2);
    }

    #[test]
    fn stub_poll_clears_after_drain() {
        let mut stub = StubGamepadInput::new();
        stub.inject(GamepadEvent::ControllerConnected { controller: id(3) });
        stub.poll();
        assert!(stub.poll().is_empty());
    }

    #[test]
    fn stub_axis_event_roundtrip() {
        let mut stub = StubGamepadInput::new();
        stub.inject(GamepadEvent::AxisChanged {
            controller: id(0),
            axis: AxisId::LeftStickX,
            value: 0.5,
        });
        let events = stub.poll();
        assert_eq!(events.len(), 1);
        if let GamepadEvent::AxisChanged { axis, value, .. } = &events[0] {
            assert_eq!(*axis, AxisId::LeftStickX);
            assert!((value - 0.5).abs() < f32::EPSILON);
        } else {
            panic!("Expected AxisChanged");
        }
    }

    // ── map_button coverage ───────────────────────────────────────────────────

    #[test]
    fn map_button_south() {
        assert_eq!(map_button(gilrs::Button::South), ButtonId::South);
    }

    #[test]
    fn map_button_north() {
        assert_eq!(map_button(gilrs::Button::North), ButtonId::North);
    }

    #[test]
    fn map_button_unknown_maps_to_other() {
        assert_eq!(map_button(gilrs::Button::Unknown), ButtonId::Other(0));
    }

    // ── map_axis coverage ─────────────────────────────────────────────────────

    #[test]
    fn map_axis_left_stick_x() {
        assert_eq!(map_axis(gilrs::Axis::LeftStickX), AxisId::LeftStickX);
    }

    #[test]
    fn map_axis_right_z_maps_to_right_trigger() {
        assert_eq!(map_axis(gilrs::Axis::RightZ), AxisId::RightTrigger);
    }
}
