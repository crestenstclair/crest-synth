// path: src/shell/gamepad_input.rs

//! Gamepad input port for the Shell context.
//!
//! Provides a trait ([`GamepadInput`]) that abstracts gamepad discovery and
//! event polling, plus a concrete adapter ([`GilrsGamepadInput`]) backed by
//! the `gilrs` crate, and a [`StubGamepadInput`] test double.
//!
//! # Design
//!
//! * **`GamepadInput` trait** – narrow port interface: connected_controllers,
//!   controller_type, poll.
//! * **`GilrsGamepadInput`** – implements the trait using `gilrs`; events are
//!   polled via `next_event` (non-blocking).
//! * **Value types** – `ControllerId`, `ControllerType`, `GamepadEvent` are
//!   plain data types free of driver details.

use std::collections::HashMap;

// ── Value types ───────────────────────────────────────────────────────────────

/// Opaque, stable identifier for a connected gamepad controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControllerId(pub u64);

/// Broad category of a connected controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerType {
    /// Standard gamepad (two analogue sticks, ABXY buttons, triggers).
    Gamepad,
    /// Driving wheel or similar rotary controller.
    Wheel,
    /// Arcade stick / fight-stick.
    ArcadeStick,
    /// Flightstick / joystick without pedals.
    FlightStick,
    /// Dance pad (floor buttons only).
    DancePad,
    /// A guitar, bass, or other music game controller.
    Guitar,
    /// Drum kit.
    Drums,
    /// Anything that doesn't fit the above categories.
    Unknown,
}

/// A button identifier on a standard controller layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonId {
    /// South face button (A / Cross).
    South,
    /// East face button (B / Circle).
    East,
    /// West face button (X / Square).
    West,
    /// North face button (Y / Triangle).
    North,
    /// Left bumper / L1.
    LeftBumper,
    /// Right bumper / R1.
    RightBumper,
    /// Left trigger / L2.
    LeftTrigger,
    /// Right trigger / R2.
    RightTrigger,
    /// Select / Back / Share.
    Select,
    /// Start / Options.
    Start,
    /// Left stick click / L3.
    LeftThumb,
    /// Right stick click / R3.
    RightThumb,
    /// D-pad up.
    DPadUp,
    /// D-pad down.
    DPadDown,
    /// D-pad left.
    DPadLeft,
    /// D-pad right.
    DPadRight,
    /// Home / guide / PS button.
    Mode,
    /// Other platform-specific button.
    Other(u8),
}

/// An axis identifier on a standard controller layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisId {
    /// Left stick horizontal (−1.0 = left, +1.0 = right).
    LeftStickX,
    /// Left stick vertical (−1.0 = down, +1.0 = up).
    LeftStickY,
    /// Right stick horizontal.
    RightStickX,
    /// Right stick vertical.
    RightStickY,
    /// Left trigger analogue value (0.0 … 1.0).
    LeftTrigger,
    /// Right trigger analogue value (0.0 … 1.0).
    RightTrigger,
    /// D-pad horizontal (−1.0 = left, +1.0 = right).
    DPadX,
    /// D-pad vertical (−1.0 = down, +1.0 = up).
    DPadY,
    /// Other platform-specific axis.
    Other(u8),
}

/// A discrete event produced by a gamepad.
#[derive(Debug, Clone, PartialEq)]
pub enum GamepadEvent {
    /// A button was pressed.
    ButtonPressed {
        controller: ControllerId,
        button: ButtonId,
    },
    /// A button was released.
    ButtonReleased {
        controller: ControllerId,
        button: ButtonId,
    },
    /// An analogue axis changed value.  Value is in `−1.0 … 1.0` (or `0.0 … 1.0`
    /// for triggers).
    AxisChanged {
        controller: ControllerId,
        axis: AxisId,
        value: f32,
    },
    /// A new controller was connected.
    ControllerConnected { controller: ControllerId },
    /// A controller was disconnected.
    ControllerDisconnected { controller: ControllerId },
}

// ── Port trait ────────────────────────────────────────────────────────────────

/// Narrow gamepad-input port interface.
///
/// Implementors discover connected controllers, classify their type, and
/// deliver queued events on demand.
pub trait GamepadInput {
    /// Return the IDs of all currently connected controllers.
    fn connected_controllers(&self) -> Vec<ControllerId>;

    /// Return the type of the given controller, or [`ControllerType::Unknown`]
    /// if the ID is not recognised.
    fn controller_type(&self, id: ControllerId) -> ControllerType;

    /// Drain all pending events since the last call.  Never blocks.
    fn poll(&mut self) -> Vec<GamepadEvent>;
}

// ── gilrs adapter ─────────────────────────────────────────────────────────────

/// Concrete [`GamepadInput`] backed by the `gilrs` crate.
///
/// `gilrs` handles platform-specific gamepad enumeration and event delivery.
/// This adapter maps `gilrs` types to our value types so that higher-level
/// code remains back-end agnostic.
///
/// # Controller IDs
///
/// Each `gilrs::GamepadId` is mapped to a `ControllerId` by converting the
/// underlying `usize` index to `u64`.  This mapping is stable for the lifetime
/// of the `GilrsGamepadInput` instance.
pub struct GilrsGamepadInput {
    gilrs: gilrs::Gilrs,
}

impl GilrsGamepadInput {
    /// Create a new adapter, initialising `gilrs`.
    ///
    /// Returns an error string if `gilrs` cannot be initialised (e.g. because
    /// there is no gamepad subsystem on the host).
    pub fn new() -> Result<Self, String> {
        let gilrs = gilrs::Gilrs::new().map_err(|e| format!("gilrs init error: {e}"))?;
        Ok(Self { gilrs })
    }
}

impl GamepadInput for GilrsGamepadInput {
    fn connected_controllers(&self) -> Vec<ControllerId> {
        self.gilrs
            .gamepads()
            .map(|(id, _)| ControllerId(usize::from(id) as u64))
            .collect()
    }

    /// Returns [`ControllerType::Gamepad`] for any known connected controller.
    ///
    /// `gilrs` 0.10 does not expose a gamepad-type enum; we report `Gamepad`
    /// for all connected pads and `Unknown` for unrecognised IDs.
    fn controller_type(&self, id: ControllerId) -> ControllerType {
        // Search connected gamepads for one whose id matches.
        for (gid, _gp) in self.gilrs.gamepads() {
            if usize::from(gid) as u64 == id.0 {
                return ControllerType::Gamepad;
            }
        }
        ControllerType::Unknown
    }

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

// ── Stub implementation for tests ─────────────────────────────────────────────

/// A test-double [`GamepadInput`] that allows direct injection of events and
/// controller state.
///
/// This struct lets tests exercise consumers of [`GamepadInput`] without
/// requiring real hardware.
pub struct StubGamepadInput {
    controllers: HashMap<ControllerId, ControllerType>,
    pending: Vec<GamepadEvent>,
}

impl StubGamepadInput {
    /// Create a new, empty stub (no controllers, no pending events).
    pub fn new() -> Self {
        Self {
            controllers: HashMap::new(),
            pending: Vec::new(),
        }
    }

    /// Register a controller with the given type.
    pub fn add_controller(&mut self, id: ControllerId, kind: ControllerType) {
        self.controllers.insert(id, kind);
    }

    /// Remove a controller.
    pub fn remove_controller(&mut self, id: ControllerId) {
        self.controllers.remove(&id);
    }

    /// Enqueue an event to be returned by the next [`poll`][Self::poll] call.
    pub fn inject(&mut self, event: GamepadEvent) {
        self.pending.push(event);
    }
}

impl Default for StubGamepadInput {
    fn default() -> Self {
        Self::new()
    }
}

impl GamepadInput for StubGamepadInput {
    fn connected_controllers(&self) -> Vec<ControllerId> {
        self.controllers.keys().copied().collect()
    }

    fn controller_type(&self, id: ControllerId) -> ControllerType {
        self.controllers
            .get(&id)
            .copied()
            .unwrap_or(ControllerType::Unknown)
    }

    fn poll(&mut self) -> Vec<GamepadEvent> {
        self.pending.drain(..).collect()
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u64) -> ControllerId {
        ControllerId(n)
    }

    // ── ControllerId ──────────────────────────────────────────────────────────

    #[test]
    fn controller_id_equality() {
        assert_eq!(id(0), id(0));
        assert_ne!(id(0), id(1));
    }

    // ── StubGamepadInput — connected_controllers ──────────────────────────────

    #[test]
    fn stub_no_controllers_initially() {
        let stub = StubGamepadInput::new();
        assert!(stub.connected_controllers().is_empty());
    }

    #[test]
    fn stub_add_controller_appears_in_connected() {
        let mut stub = StubGamepadInput::new();
        stub.add_controller(id(1), ControllerType::Gamepad);
        let ids = stub.connected_controllers();
        assert_eq!(ids, vec![id(1)]);
    }

    #[test]
    fn stub_remove_controller_disappears_from_connected() {
        let mut stub = StubGamepadInput::new();
        stub.add_controller(id(2), ControllerType::Gamepad);
        stub.remove_controller(id(2));
        assert!(stub.connected_controllers().is_empty());
    }

    // ── StubGamepadInput — controller_type ───────────────────────────────────

    #[test]
    fn stub_controller_type_known() {
        let mut stub = StubGamepadInput::new();
        stub.add_controller(id(5), ControllerType::Guitar);
        assert_eq!(stub.controller_type(id(5)), ControllerType::Guitar);
    }

    #[test]
    fn stub_controller_type_unknown_for_missing_id() {
        let stub = StubGamepadInput::new();
        assert_eq!(stub.controller_type(id(99)), ControllerType::Unknown);
    }

    // ── StubGamepadInput — poll ───────────────────────────────────────────────

    #[test]
    fn stub_poll_returns_empty_when_no_events() {
        let mut stub = StubGamepadInput::new();
        assert!(stub.poll().is_empty());
    }

    #[test]
    fn stub_poll_drains_injected_events_in_order() {
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
    fn stub_poll_clears_events_after_drain() {
        let mut stub = StubGamepadInput::new();
        stub.inject(GamepadEvent::ControllerConnected { controller: id(3) });
        stub.poll(); // drain
        assert!(stub.poll().is_empty());
    }

    #[test]
    fn stub_axis_event_roundtrip() {
        let mut stub = StubGamepadInput::new();
        stub.inject(GamepadEvent::AxisChanged {
            controller: id(0),
            axis: AxisId::LeftStickX,
            value: 0.75,
        });
        let events = stub.poll();
        assert_eq!(events.len(), 1);
        if let GamepadEvent::AxisChanged {
            controller,
            axis,
            value,
        } = &events[0]
        {
            assert_eq!(*controller, id(0));
            assert_eq!(*axis, AxisId::LeftStickX);
            assert!((value - 0.75).abs() < f32::EPSILON);
        } else {
            panic!("Expected AxisChanged event");
        }
    }

    #[test]
    fn gamepad_input_trait_is_object_safe() {
        let stub: Box<dyn GamepadInput> = Box::new(StubGamepadInput::new());
        drop(stub);
    }

    // ── ControllerType coverage ───────────────────────────────────────────────

    #[test]
    fn all_controller_types_are_distinct() {
        let types = [
            ControllerType::Gamepad,
            ControllerType::Wheel,
            ControllerType::ArcadeStick,
            ControllerType::FlightStick,
            ControllerType::DancePad,
            ControllerType::Guitar,
            ControllerType::Drums,
            ControllerType::Unknown,
        ];
        // Each type must equal itself and differ from the others (spot-check a few).
        assert_eq!(types[0], ControllerType::Gamepad);
        assert_ne!(types[0], ControllerType::Wheel);
        assert_ne!(types[1], ControllerType::ArcadeStick);
    }

    // ── ButtonId coverage ─────────────────────────────────────────────────────

    #[test]
    fn button_id_other_variants_equal_by_inner_value() {
        assert_eq!(ButtonId::Other(3), ButtonId::Other(3));
        assert_ne!(ButtonId::Other(1), ButtonId::Other(2));
    }

    // ── AxisId coverage ───────────────────────────────────────────────────────

    #[test]
    fn axis_id_other_variants_equal_by_inner_value() {
        assert_eq!(AxisId::Other(7), AxisId::Other(7));
        assert_ne!(AxisId::Other(0), AxisId::Other(1));
    }
}
