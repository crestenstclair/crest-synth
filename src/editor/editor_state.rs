// path: src/editor/editor_state.rs

//! The single editor store: owns focus, edit-mode, and the editable
//! parameter fields. `apply` is the only entry point that reacts to
//! `EditorEvent`s and mutates state.

use crate::editor::editor_event::EditorEvent;
use crate::editor::param_field::ParamField;

/// Coarse adjustment is this many times the field's fine step (10 units).
const COARSE_MULTIPLIER: f32 = 10.0;

/// The single editor store: owns focus, edit-mode, and the editable
/// parameter fields.
///
/// `apply(EditorEvent)` is the ONLY way to mutate this state; it is pure
/// and allocation-free (no I/O, rendering, or audio).
#[derive(Debug, Clone, PartialEq)]
pub struct EditorState {
    edit_mode: bool,
    fields: Vec<ParamField>,
    focus: usize,
}

impl EditorState {
    /// Creates a new editor state over `fields`, starting in navigate mode
    /// with focus on the first field (or `0` if `fields` is empty).
    pub fn new(fields: Vec<ParamField>) -> Self {
        Self { edit_mode: false, fields, focus: 0 }
    }

    pub fn edit_mode(&self) -> bool {
        self.edit_mode
    }

    pub fn fields(&self) -> &[ParamField] {
        &self.fields
    }

    pub fn focus(&self) -> usize {
        self.focus
    }

    /// The currently focused field, if any (`None` when `fields` is empty).
    pub fn focused_field(&self) -> Option<&ParamField> {
        self.fields.get(self.focus)
    }

    /// The only mutator of `EditorState`. Pure and allocation-free: no I/O,
    /// rendering, or audio.
    pub fn apply(&mut self, event: EditorEvent) {
        match event {
            EditorEvent::EnterEditMode => self.edit_mode = true,
            EditorEvent::ExitEditMode => self.edit_mode = false,
            EditorEvent::NavUp => self.on_vertical(COARSE_MULTIPLIER),
            EditorEvent::NavDown => self.on_vertical(-COARSE_MULTIPLIER),
            EditorEvent::NavLeft => self.on_horizontal(-1.0),
            EditorEvent::NavRight => self.on_horizontal(1.0),
        }
    }

    /// NavUp (positive `units`) / NavDown (negative `units`): in edit mode
    /// this is the coarse adjustment; in navigate mode it moves focus
    /// (NavUp decreases focus, NavDown increases it).
    fn on_vertical(&mut self, units: f32) {
        if self.edit_mode {
            self.adjust_focused(units);
        } else {
            self.move_focus(if units > 0.0 { -1 } else { 1 });
        }
    }

    /// NavRight (positive `units`) / NavLeft (negative `units`): in edit
    /// mode this is the fine adjustment; in navigate mode it moves focus
    /// (NavLeft decreases focus, NavRight increases it).
    fn on_horizontal(&mut self, units: f32) {
        if self.edit_mode {
            self.adjust_focused(units);
        } else {
            self.move_focus(if units > 0.0 { 1 } else { -1 });
        }
    }

    /// Moves focus by `delta` (+1 or -1), saturating at the ends (no wrap).
    fn move_focus(&mut self, delta: isize) {
        if self.fields.is_empty() {
            return;
        }
        let max_index = (self.fields.len() - 1) as isize;
        let next = self.focus as isize + delta;
        self.focus = next.clamp(0, max_index) as usize;
    }

    /// Adjusts the focused field's value by `units` fine-steps, clamping to
    /// the field's `[min, max]`.
    fn adjust_focused(&mut self, units: f32) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            let delta = units * field.step();
            field.adjust(delta);
        }
    }
}

#[cfg(test)]
mod editor_state_tests {
    use super::*;

    fn three_fields() -> Vec<ParamField> {
        vec![
            ParamField::new("a", 5.0, 0.0, 10.0, 1.0),
            ParamField::new("b", 5.0, 0.0, 10.0, 1.0),
            ParamField::new("c", 5.0, 0.0, 10.0, 1.0),
        ]
    }

    #[test]
    fn starts_in_navigate_mode_focused_on_first_field() {
        let state = EditorState::new(three_fields());
        assert!(!state.edit_mode());
        assert_eq!(state.focus(), 0);
    }

    #[test]
    fn navigate_mode_right_and_down_move_focus_forward() {
        let mut state = EditorState::new(three_fields());
        state.apply(EditorEvent::NavRight);
        assert_eq!(state.focus(), 1);
        state.apply(EditorEvent::NavDown);
        assert_eq!(state.focus(), 2);
    }

    #[test]
    fn navigate_mode_left_and_up_move_focus_backward() {
        let mut state = EditorState::new(three_fields());
        state.apply(EditorEvent::NavRight);
        state.apply(EditorEvent::NavRight);
        assert_eq!(state.focus(), 2);
        state.apply(EditorEvent::NavLeft);
        assert_eq!(state.focus(), 1);
        state.apply(EditorEvent::NavUp);
        assert_eq!(state.focus(), 0);
    }

    #[test]
    fn navigate_mode_saturates_at_ends_without_wrapping() {
        let mut state = EditorState::new(three_fields());
        state.apply(EditorEvent::NavLeft);
        assert_eq!(state.focus(), 0, "must saturate at 0, not wrap to last index");

        state.apply(EditorEvent::NavRight);
        state.apply(EditorEvent::NavRight);
        state.apply(EditorEvent::NavRight);
        state.apply(EditorEvent::NavRight);
        assert_eq!(state.focus(), 2, "must saturate at last index, not wrap to 0");
    }

    #[test]
    fn enter_and_exit_edit_mode_toggle_flag() {
        let mut state = EditorState::new(three_fields());
        state.apply(EditorEvent::EnterEditMode);
        assert!(state.edit_mode());
        state.apply(EditorEvent::ExitEditMode);
        assert!(!state.edit_mode());
    }

    #[test]
    fn edit_mode_right_and_left_adjust_by_fine_unit() {
        let mut state = EditorState::new(three_fields());
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavRight);
        assert_eq!(state.focused_field().unwrap().value(), 6.0);
        state.apply(EditorEvent::NavLeft);
        state.apply(EditorEvent::NavLeft);
        assert_eq!(state.focused_field().unwrap().value(), 4.0);
    }

    #[test]
    fn edit_mode_up_and_down_adjust_by_coarse_unit_ten_times_fine() {
        let mut state = EditorState::new(three_fields());
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavUp);
        assert_eq!(state.focused_field().unwrap().value(), 10.0, "NavUp is +10x fine, clamped to max");

        state.apply(EditorEvent::NavDown);
        state.apply(EditorEvent::NavDown);
        assert_eq!(state.focused_field().unwrap().value(), 0.0, "two NavDown (-10 each) clamps to min");
    }

    #[test]
    fn edit_mode_directional_events_do_not_move_focus() {
        let mut state = EditorState::new(three_fields());
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavRight);
        state.apply(EditorEvent::NavUp);
        assert_eq!(state.focus(), 0, "edit mode adjusts the value, not focus");
    }

    #[test]
    fn every_adjustment_clamps_to_focused_field_min_max() {
        let fields = vec![ParamField::new("narrow", 0.0, 0.0, 1.0, 1.0)];
        let mut state = EditorState::new(fields);
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavUp);
        assert_eq!(state.focused_field().unwrap().value(), 1.0);
        state.apply(EditorEvent::NavUp);
        assert_eq!(state.focused_field().unwrap().value(), 1.0, "stays clamped at max");
        state.apply(EditorEvent::NavDown);
        state.apply(EditorEvent::NavDown);
        assert_eq!(state.focused_field().unwrap().value(), 0.0, "stays clamped at min");
    }

    #[test]
    fn empty_fields_apply_is_a_safe_no_op() {
        let mut state = EditorState::new(Vec::new());
        state.apply(EditorEvent::NavRight);
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavUp);
        assert_eq!(state.focus(), 0);
        assert!(state.focused_field().is_none());
    }
}
