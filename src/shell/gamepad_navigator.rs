// path: src/shell/gamepad_navigator.rs

//! GamepadNavigator — maps raw gamepad events to [`GamepadAction`] values and
//! drives the application's own cursor/edit model.
//!
//! # Design
//!
//! * **[`CursorModel`]** — the application's own focus/edit state, completely
//!   independent of any UI framework's built-in focus system.
//! * **[`GamepadNavigator`]** — stateful service that polls a [`GamepadInput`]
//!   port, translates events to [`GamepadAction`] values, applies navigation
//!   effects to a [`CursorModel`], and returns the actions for the frame.
//!   Constructed via DI: the input port is injected.
//!
//! All gamepad navigation goes through the app's own [`CursorModel`]; this
//! module never touches egui's built-in focus.

use crate::shell::gamepad_action::GamepadAction;
use crate::shell::gamepad_input::{AxisId, ButtonId, GamepadEvent, GamepadInput};

// ── Axis dead-zone ─────────────────────────────────────────────────────────

/// Axis deflection below this absolute value is ignored.
const AXIS_DEAD_ZONE: f32 = 0.5;

// ── Cursor / edit model ────────────────────────────────────────────────────

/// The direction of a navigation gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavDirection {
    Up,
    Down,
    Left,
    Right,
}

/// The application's own cursor and edit-mode state.
///
/// Entirely independent of any UI framework's focus system.  The shell owns a
/// single [`CursorModel`] and passes a mutable reference to
/// [`GamepadNavigator::poll`] each frame.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorModel {
    /// Current row of the focused cell (0-based).
    pub row: usize,
    /// Current column of the focused cell (0-based).
    pub col: usize,
    /// Current page / tab index.
    pub page: usize,
    /// Whether the cursor is in edit / tweak mode for the focused parameter.
    pub editing: bool,
    /// Total number of rows in the active view (used for clamping).
    pub row_count: usize,
    /// Total number of columns in the active view (used for clamping).
    pub col_count: usize,
    /// Total number of pages (used for wrapping).
    pub page_count: usize,
}

impl CursorModel {
    /// Create a new `CursorModel` at position (0, 0), page 0, not editing.
    ///
    /// `row_count`, `col_count`, and `page_count` describe the grid dimensions
    /// of the active view.  Each is clamped to a minimum of 1.
    pub fn new(row_count: usize, col_count: usize, page_count: usize) -> Self {
        Self {
            row: 0,
            col: 0,
            page: 0,
            editing: false,
            row_count: row_count.max(1),
            col_count: col_count.max(1),
            page_count: page_count.max(1),
        }
    }

    /// Move the cursor by one cell in the given direction (clamped to bounds).
    pub fn move_cursor(&mut self, dir: NavDirection) {
        match dir {
            NavDirection::Up => {
                self.row = self.row.saturating_sub(1);
            }
            NavDirection::Down => {
                if self.row + 1 < self.row_count {
                    self.row += 1;
                }
            }
            NavDirection::Left => {
                self.col = self.col.saturating_sub(1);
            }
            NavDirection::Right => {
                if self.col + 1 < self.col_count {
                    self.col += 1;
                }
            }
        }
    }

    /// Advance to the next page (wraps around).
    pub fn next_page(&mut self) {
        self.page = (self.page + 1) % self.page_count;
    }

    /// Return to the previous page (wraps around).
    pub fn prev_page(&mut self) {
        if self.page == 0 {
            self.page = self.page_count - 1;
        } else {
            self.page -= 1;
        }
    }

    /// Set or clear edit mode.
    pub fn set_editing(&mut self, editing: bool) {
        self.editing = editing;
    }
}

impl Default for CursorModel {
    fn default() -> Self {
        Self::new(1, 1, 1)
    }
}

// ── GamepadNavigator ────────────────────────────────────────────────────────

/// Translates raw gamepad events into [`GamepadAction`] values and applies
/// them to a [`CursorModel`].
///
/// # Dependency Injection
///
/// The underlying [`GamepadInput`] port is provided at construction time.
/// Tests inject a [`StubGamepadInput`][crate::shell::gamepad_input::StubGamepadInput];
/// production code injects a `GilrsGamepadInput`.
///
/// ```
/// use crest_synth::shell::gamepad_navigator::{GamepadNavigator, CursorModel};
/// use crest_synth::shell::gamepad_input::StubGamepadInput;
///
/// let mut nav = GamepadNavigator::new(StubGamepadInput::new());
/// let mut cursor = CursorModel::new(4, 8, 3);
/// let actions = nav.poll(&mut cursor);
/// assert!(actions.is_empty());
/// ```
pub struct GamepadNavigator<I: GamepadInput> {
    input: I,
}

impl<I: GamepadInput> GamepadNavigator<I> {
    /// Create a new `GamepadNavigator` with the given input port.
    pub fn new(input: I) -> Self {
        Self { input }
    }

    /// Poll the input port, translate all pending events to [`GamepadAction`]
    /// values, apply navigation effects to `cursor`, and return the full list
    /// of actions produced this call.
    ///
    /// Call once per UI frame.  Never blocks.
    pub fn poll(&mut self, cursor: &mut CursorModel) -> Vec<GamepadAction> {
        let events = self.input.poll();
        let mut actions = Vec::new();

        for event in &events {
            if let Some(action) = Self::translate(event) {
                Self::apply_to_cursor(action, event, cursor);
                actions.push(action);
            }
        }

        actions
    }

    /// Map a raw [`GamepadEvent`] to a [`GamepadAction`], if applicable.
    fn translate(event: &GamepadEvent) -> Option<GamepadAction> {
        match event {
            GamepadEvent::ButtonPressed { button, .. } => match button {
                ButtonId::South => Some(GamepadAction::Select),
                ButtonId::East => Some(GamepadAction::Back),
                ButtonId::North => Some(GamepadAction::AssignMod),
                ButtonId::West => None,
                ButtonId::LeftBumper => Some(GamepadAction::PreviousPage),
                ButtonId::RightBumper => Some(GamepadAction::NextPage),
                ButtonId::LeftTrigger => Some(GamepadAction::TweakDown),
                ButtonId::RightTrigger => Some(GamepadAction::TweakUp),
                ButtonId::Start => Some(GamepadAction::QuickSave),
                ButtonId::Select => None,
                ButtonId::DPadUp => Some(GamepadAction::Navigate),
                ButtonId::DPadDown => Some(GamepadAction::Navigate),
                ButtonId::DPadLeft => Some(GamepadAction::Navigate),
                ButtonId::DPadRight => Some(GamepadAction::Navigate),
                ButtonId::LeftThumb | ButtonId::RightThumb | ButtonId::Mode => None,
                ButtonId::Other(_) => None,
            },
            GamepadEvent::AxisChanged { axis, value, .. } => match axis {
                AxisId::LeftStickX | AxisId::LeftStickY => {
                    if value.abs() > AXIS_DEAD_ZONE {
                        Some(GamepadAction::Navigate)
                    } else {
                        None
                    }
                }
                // D-pad axes also drive navigation
                AxisId::DPadX | AxisId::DPadY => {
                    if value.abs() > AXIS_DEAD_ZONE {
                        Some(GamepadAction::Navigate)
                    } else {
                        None
                    }
                }
                _ => None,
            },
            GamepadEvent::ButtonReleased { .. }
            | GamepadEvent::ControllerConnected { .. }
            | GamepadEvent::ControllerDisconnected { .. } => None,
        }
    }

    /// Apply a [`GamepadAction`] (and supporting event context) to the cursor.
    fn apply_to_cursor(action: GamepadAction, event: &GamepadEvent, cursor: &mut CursorModel) {
        match action {
            GamepadAction::Navigate => {
                if let Some(dir) = Self::nav_direction(event) {
                    cursor.move_cursor(dir);
                }
            }
            GamepadAction::Select => {
                cursor.set_editing(true);
            }
            GamepadAction::Back => {
                cursor.set_editing(false);
            }
            GamepadAction::NextPage => {
                cursor.next_page();
            }
            GamepadAction::PreviousPage => {
                cursor.prev_page();
            }
            // TweakUp, TweakDown, AssignMod, QuickSave do not mutate the cursor.
            GamepadAction::TweakUp
            | GamepadAction::TweakDown
            | GamepadAction::AssignMod
            | GamepadAction::QuickSave => {}
        }
    }

    /// Derive a [`NavDirection`] from the raw event that produced a Navigate action.
    fn nav_direction(event: &GamepadEvent) -> Option<NavDirection> {
        match event {
            GamepadEvent::ButtonPressed {
                button: ButtonId::DPadUp,
                ..
            } => Some(NavDirection::Up),
            GamepadEvent::ButtonPressed {
                button: ButtonId::DPadDown,
                ..
            } => Some(NavDirection::Down),
            GamepadEvent::ButtonPressed {
                button: ButtonId::DPadLeft,
                ..
            } => Some(NavDirection::Left),
            GamepadEvent::ButtonPressed {
                button: ButtonId::DPadRight,
                ..
            } => Some(NavDirection::Right),
            GamepadEvent::AxisChanged {
                axis: AxisId::LeftStickX,
                value,
                ..
            }
            | GamepadEvent::AxisChanged {
                axis: AxisId::DPadX,
                value,
                ..
            } => {
                if *value > 0.0 {
                    Some(NavDirection::Right)
                } else {
                    Some(NavDirection::Left)
                }
            }
            GamepadEvent::AxisChanged {
                axis: AxisId::LeftStickY,
                value,
                ..
            }
            | GamepadEvent::AxisChanged {
                axis: AxisId::DPadY,
                value,
                ..
            } => {
                // Positive Y = up (per gilrs convention)
                if *value > 0.0 {
                    Some(NavDirection::Up)
                } else {
                    Some(NavDirection::Down)
                }
            }
            _ => None,
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::gamepad_input::{ControllerId, StubGamepadInput};

    fn c0() -> ControllerId {
        ControllerId(0)
    }

    fn btn_pressed(button: ButtonId) -> GamepadEvent {
        GamepadEvent::ButtonPressed {
            controller: c0(),
            button,
        }
    }

    fn btn_released(button: ButtonId) -> GamepadEvent {
        GamepadEvent::ButtonReleased {
            controller: c0(),
            button,
        }
    }

    fn axis(axis: AxisId, value: f32) -> GamepadEvent {
        GamepadEvent::AxisChanged {
            controller: c0(),
            axis,
            value,
        }
    }

    fn navigator_with(
        events: impl IntoIterator<Item = GamepadEvent>,
    ) -> (GamepadNavigator<StubGamepadInput>, CursorModel) {
        let mut input = StubGamepadInput::new();
        for e in events {
            input.inject(e);
        }
        let nav = GamepadNavigator::new(input);
        let cursor = CursorModel::new(4, 8, 3);
        (nav, cursor)
    }

    // ── CursorModel ──────────────────────────────────────────────────────────

    #[test]
    fn cursor_model_starts_at_origin() {
        let c = CursorModel::new(4, 8, 3);
        assert_eq!(c.row, 0);
        assert_eq!(c.col, 0);
        assert_eq!(c.page, 0);
        assert!(!c.editing);
    }

    #[test]
    fn cursor_move_down_then_up() {
        let mut c = CursorModel::new(4, 8, 3);
        c.move_cursor(NavDirection::Down);
        assert_eq!(c.row, 1);
        c.move_cursor(NavDirection::Up);
        assert_eq!(c.row, 0);
    }

    #[test]
    fn cursor_clamps_at_top() {
        let mut c = CursorModel::new(4, 8, 3);
        c.move_cursor(NavDirection::Up);
        assert_eq!(c.row, 0);
    }

    #[test]
    fn cursor_clamps_at_bottom() {
        let mut c = CursorModel::new(2, 8, 3);
        for _ in 0..5 {
            c.move_cursor(NavDirection::Down);
        }
        assert_eq!(c.row, 1); // clamped at row_count - 1
    }

    #[test]
    fn cursor_move_right_then_left() {
        let mut c = CursorModel::new(4, 8, 3);
        c.move_cursor(NavDirection::Right);
        assert_eq!(c.col, 1);
        c.move_cursor(NavDirection::Left);
        assert_eq!(c.col, 0);
    }

    #[test]
    fn cursor_clamps_at_right_edge() {
        let mut c = CursorModel::new(4, 2, 3);
        for _ in 0..5 {
            c.move_cursor(NavDirection::Right);
        }
        assert_eq!(c.col, 1); // clamped at col_count - 1
    }

    #[test]
    fn cursor_page_wraps_forward() {
        let mut c = CursorModel::new(4, 8, 3);
        c.next_page();
        c.next_page();
        c.next_page(); // wraps
        assert_eq!(c.page, 0);
    }

    #[test]
    fn cursor_page_wraps_backward() {
        let mut c = CursorModel::new(4, 8, 3);
        c.prev_page(); // wraps to 2
        assert_eq!(c.page, 2);
    }

    #[test]
    fn cursor_editing_flag() {
        let mut c = CursorModel::new(4, 8, 3);
        assert!(!c.editing);
        c.set_editing(true);
        assert!(c.editing);
        c.set_editing(false);
        assert!(!c.editing);
    }

    #[test]
    fn cursor_default_is_one_by_one_by_one() {
        let c = CursorModel::default();
        assert_eq!(c.row_count, 1);
        assert_eq!(c.col_count, 1);
        assert_eq!(c.page_count, 1);
    }

    // ── GamepadNavigator — mapping ────────────────────────────────────────────

    #[test]
    fn no_events_returns_empty_actions() {
        let (mut nav, mut cursor) = navigator_with([]);
        let actions = nav.poll(&mut cursor);
        assert!(actions.is_empty());
    }

    #[test]
    fn south_button_maps_to_select_and_enters_editing() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::South)]);
        let actions = nav.poll(&mut cursor);
        assert_eq!(actions, vec![GamepadAction::Select]);
        assert!(cursor.editing);
    }

    #[test]
    fn east_button_maps_to_back_and_exits_editing() {
        let mut cursor = CursorModel::new(4, 8, 3);
        cursor.set_editing(true);
        let mut input = StubGamepadInput::new();
        input.inject(btn_pressed(ButtonId::East));
        let mut nav = GamepadNavigator::new(input);
        let actions = nav.poll(&mut cursor);
        assert_eq!(actions, vec![GamepadAction::Back]);
        assert!(!cursor.editing);
    }

    #[test]
    fn dpad_down_moves_cursor_down() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::DPadDown)]);
        nav.poll(&mut cursor);
        assert_eq!(cursor.row, 1);
    }

    #[test]
    fn dpad_up_moves_cursor_up() {
        let mut cursor = CursorModel::new(4, 8, 3);
        cursor.row = 2;
        let mut input = StubGamepadInput::new();
        input.inject(btn_pressed(ButtonId::DPadUp));
        let mut nav = GamepadNavigator::new(input);
        nav.poll(&mut cursor);
        assert_eq!(cursor.row, 1);
    }

    #[test]
    fn dpad_left_moves_cursor_left() {
        let mut cursor = CursorModel::new(4, 8, 3);
        cursor.col = 3;
        let mut input = StubGamepadInput::new();
        input.inject(btn_pressed(ButtonId::DPadLeft));
        let mut nav = GamepadNavigator::new(input);
        nav.poll(&mut cursor);
        assert_eq!(cursor.col, 2);
    }

    #[test]
    fn dpad_right_moves_cursor_right() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::DPadRight)]);
        nav.poll(&mut cursor);
        assert_eq!(cursor.col, 1);
    }

    #[test]
    fn left_bumper_maps_to_prev_page() {
        let mut cursor = CursorModel::new(4, 8, 3);
        cursor.page = 2;
        let mut input = StubGamepadInput::new();
        input.inject(btn_pressed(ButtonId::LeftBumper));
        let mut nav = GamepadNavigator::new(input);
        let actions = nav.poll(&mut cursor);
        assert_eq!(actions, vec![GamepadAction::PreviousPage]);
        assert_eq!(cursor.page, 1);
    }

    #[test]
    fn right_bumper_maps_to_next_page() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::RightBumper)]);
        let actions = nav.poll(&mut cursor);
        assert_eq!(actions, vec![GamepadAction::NextPage]);
        assert_eq!(cursor.page, 1);
    }

    #[test]
    fn right_trigger_maps_to_tweak_up() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::RightTrigger)]);
        let actions = nav.poll(&mut cursor);
        assert_eq!(actions, vec![GamepadAction::TweakUp]);
    }

    #[test]
    fn left_trigger_maps_to_tweak_down() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::LeftTrigger)]);
        let actions = nav.poll(&mut cursor);
        assert_eq!(actions, vec![GamepadAction::TweakDown]);
    }

    #[test]
    fn north_button_maps_to_assign_mod() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::North)]);
        let actions = nav.poll(&mut cursor);
        assert_eq!(actions, vec![GamepadAction::AssignMod]);
    }

    #[test]
    fn start_button_maps_to_quick_save() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::Start)]);
        let actions = nav.poll(&mut cursor);
        assert_eq!(actions, vec![GamepadAction::QuickSave]);
    }

    #[test]
    fn button_released_produces_no_action() {
        let (mut nav, mut cursor) = navigator_with([btn_released(ButtonId::South)]);
        let actions = nav.poll(&mut cursor);
        assert!(actions.is_empty());
    }

    #[test]
    fn controller_connected_produces_no_action() {
        let (mut nav, mut cursor) =
            navigator_with([GamepadEvent::ControllerConnected { controller: c0() }]);
        let actions = nav.poll(&mut cursor);
        assert!(actions.is_empty());
    }

    #[test]
    fn left_stick_x_above_dead_zone_navigates_right() {
        let (mut nav, mut cursor) = navigator_with([axis(AxisId::LeftStickX, 0.8)]);
        nav.poll(&mut cursor);
        assert_eq!(cursor.col, 1);
    }

    #[test]
    fn left_stick_x_negative_above_dead_zone_navigates_left() {
        let mut cursor = CursorModel::new(4, 8, 3);
        cursor.col = 3;
        let mut input = StubGamepadInput::new();
        input.inject(axis(AxisId::LeftStickX, -0.8));
        let mut nav = GamepadNavigator::new(input);
        nav.poll(&mut cursor);
        assert_eq!(cursor.col, 2);
    }

    #[test]
    fn left_stick_y_positive_above_dead_zone_navigates_up() {
        let mut cursor = CursorModel::new(4, 8, 3);
        cursor.row = 2;
        let mut input = StubGamepadInput::new();
        input.inject(axis(AxisId::LeftStickY, 0.9));
        let mut nav = GamepadNavigator::new(input);
        nav.poll(&mut cursor);
        assert_eq!(cursor.row, 1);
    }

    #[test]
    fn left_stick_y_negative_above_dead_zone_navigates_down() {
        let (mut nav, mut cursor) = navigator_with([axis(AxisId::LeftStickY, -0.9)]);
        nav.poll(&mut cursor);
        assert_eq!(cursor.row, 1);
    }

    #[test]
    fn stick_below_dead_zone_produces_no_action() {
        let (mut nav, mut cursor) = navigator_with([axis(AxisId::LeftStickX, 0.1)]);
        let actions = nav.poll(&mut cursor);
        assert!(actions.is_empty());
        assert_eq!(cursor.col, 0); // unchanged
    }

    #[test]
    fn multiple_events_in_one_poll() {
        let (mut nav, mut cursor) = navigator_with([
            btn_pressed(ButtonId::DPadDown),
            btn_pressed(ButtonId::DPadRight),
            btn_pressed(ButtonId::South),
        ]);
        let actions = nav.poll(&mut cursor);
        assert_eq!(
            actions,
            vec![
                GamepadAction::Navigate,
                GamepadAction::Navigate,
                GamepadAction::Select,
            ]
        );
        assert_eq!(cursor.row, 1);
        assert_eq!(cursor.col, 1);
        assert!(cursor.editing);
    }

    #[test]
    fn navigator_is_generic_over_stub_input() {
        let input = StubGamepadInput::new();
        let _nav: GamepadNavigator<StubGamepadInput> = GamepadNavigator::new(input);
    }

    #[test]
    fn tweak_up_does_not_mutate_cursor() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::RightTrigger)]);
        let before = cursor.clone();
        nav.poll(&mut cursor);
        assert_eq!(cursor, before);
    }

    #[test]
    fn tweak_down_does_not_mutate_cursor() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::LeftTrigger)]);
        let before = cursor.clone();
        nav.poll(&mut cursor);
        assert_eq!(cursor, before);
    }

    #[test]
    fn assign_mod_does_not_mutate_cursor() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::North)]);
        let before = cursor.clone();
        nav.poll(&mut cursor);
        assert_eq!(cursor, before);
    }

    #[test]
    fn quick_save_does_not_mutate_cursor() {
        let (mut nav, mut cursor) = navigator_with([btn_pressed(ButtonId::Start)]);
        let before = cursor.clone();
        nav.poll(&mut cursor);
        assert_eq!(cursor, before);
    }

    #[test]
    fn dpad_axis_x_positive_navigates_right() {
        let (mut nav, mut cursor) = navigator_with([axis(AxisId::DPadX, 0.9)]);
        nav.poll(&mut cursor);
        assert_eq!(cursor.col, 1);
    }

    #[test]
    fn dpad_axis_y_negative_navigates_down() {
        let (mut nav, mut cursor) = navigator_with([axis(AxisId::DPadY, -0.9)]);
        nav.poll(&mut cursor);
        assert_eq!(cursor.row, 1);
    }
}
