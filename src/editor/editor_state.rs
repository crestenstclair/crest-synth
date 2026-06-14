// path: src/editor/editor_state.rs

use crate::editor::editor_event::EditorEvent;
use crate::editor::param_field::ParamField;

/// Flux-style store for all editor UI state.
///
/// `apply` is the **only** mutation entry point - there are no setters and no
/// other path that changes `edit_mode`, `focus`, or any field value.
///
/// # Navigate mode (`edit_mode == false`)
/// Directional events move `focus` between fields (saturating at the ends).
/// Both vertical (NavUp/NavDown) and horizontal (NavLeft/NavRight) move focus
/// by one in the single-column MVP.
///
/// # Edit mode (`edit_mode == true`)
/// Directional events adjust the focused field's value instead:
/// - `NavLeft`  -- value -= 1 step  (fine)
/// - `NavRight` -- value += 1 step  (fine)
/// - `NavDown`  -- value -= 10 steps (coarse)
/// - `NavUp`    -- value += 10 steps (coarse)
///
/// Every adjustment is clamped to the focused field's `[min, max]`.
///
/// # Examples
///
/// ```
/// use crest_synth::editor::editor_state::EditorState;
/// use crest_synth::editor::editor_event::EditorEvent;
/// use crest_synth::editor::param_field::ParamField;
///
/// let fields = vec![
///     ParamField::new("volume", "Volume", 0.0, 100.0, 1.0, 50.0).unwrap(),
///     ParamField::new("cutoff", "Cutoff", 20.0, 20000.0, 10.0, 1000.0).unwrap(),
/// ];
/// let mut state = EditorState::new(fields);
/// assert_eq!(state.focus(), 0);
/// assert!(!state.edit_mode());
///
/// // Navigate to second field
/// state.apply(EditorEvent::NavDown);
/// assert_eq!(state.focus(), 1);
///
/// // Enter edit mode and increase value by one fine step (+10.0)
/// state.apply(EditorEvent::EnterEditMode);
/// assert!(state.edit_mode());
/// state.apply(EditorEvent::NavRight);
/// assert_eq!(state.fields()[1].value(), 1010.0);
/// ```
#[derive(Debug, Clone)]
pub struct EditorState {
    edit_mode: bool,
    fields: Vec<ParamField>,
    focus: usize,
}

impl EditorState {
    /// Construct a new `EditorState` with the given fields.
    ///
    /// `focus` starts at 0. `edit_mode` starts as `false` (navigate mode).
    pub fn new(fields: Vec<ParamField>) -> Self {
        Self {
            edit_mode: false,
            fields,
            focus: 0,
        }
    }

    /// Returns `true` when the editor is in edit mode (directional = adjust value).
    pub fn edit_mode(&self) -> bool {
        self.edit_mode
    }

    /// Returns the index of the currently focused field.
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// Returns a reference to the list of parameter fields.
    pub fn fields(&self) -> &[ParamField] {
        &self.fields
    }

    /// The only way to mutate editor state.
    ///
    /// Behaviour depends on the current mode:
    /// - `EnterEditMode` / `ExitEditMode` toggle `edit_mode`.
    /// - In navigate mode: directional events move `focus` (saturating).
    /// - In edit mode: directional events adjust the focused field's value.
    pub fn apply(&mut self, event: EditorEvent) {
        match event {
            EditorEvent::EnterEditMode => {
                self.edit_mode = true;
            }
            EditorEvent::ExitEditMode => {
                self.edit_mode = false;
            }
            EditorEvent::NavUp
            | EditorEvent::NavDown
            | EditorEvent::NavLeft
            | EditorEvent::NavRight => {
                if self.edit_mode {
                    self.apply_edit(event);
                } else {
                    self.apply_navigate(event);
                }
            }
        }
    }

    /// Handle directional input in navigate mode -- moves focus (saturating).
    fn apply_navigate(&mut self, event: EditorEvent) {
        let len = self.fields.len();
        if len == 0 {
            return;
        }
        match event {
            EditorEvent::NavUp | EditorEvent::NavLeft => {
                self.focus = self.focus.saturating_sub(1);
            }
            EditorEvent::NavDown | EditorEvent::NavRight => {
                self.focus = (self.focus + 1).min(len - 1);
            }
            _ => {}
        }
    }

    /// Handle directional input in edit mode -- adjusts the focused field's value.
    fn apply_edit(&mut self, event: EditorEvent) {
        if self.fields.is_empty() {
            return;
        }
        let field = &mut self.fields[self.focus];
        match event {
            EditorEvent::NavLeft => field.step_down(),
            EditorEvent::NavRight => field.step_up(),
            EditorEvent::NavDown => field.coarse_down(),
            EditorEvent::NavUp => field.coarse_up(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod editor_state_tests {
    use super::*;

    fn make_field(id: &str, label: &str, value: f64) -> ParamField {
        ParamField::new(id, label, 0.0, 100.0, 1.0, value).unwrap()
    }

    fn two_field_state() -> EditorState {
        EditorState::new(vec![
            make_field("volume", "Volume", 50.0),
            make_field("cutoff", "Cutoff", 50.0),
        ])
    }

    // -- mode toggle --

    #[test]
    fn editor_state_starts_in_navigate_mode() {
        let state = two_field_state();
        assert!(!state.edit_mode());
    }

    #[test]
    fn editor_state_enter_edit_mode() {
        let mut state = two_field_state();
        state.apply(EditorEvent::EnterEditMode);
        assert!(state.edit_mode());
    }

    #[test]
    fn editor_state_exit_edit_mode() {
        let mut state = two_field_state();
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::ExitEditMode);
        assert!(!state.edit_mode());
    }

    // -- navigation --

    #[test]
    fn editor_state_nav_down_moves_focus() {
        let mut state = two_field_state();
        assert_eq!(state.focus(), 0);
        state.apply(EditorEvent::NavDown);
        assert_eq!(state.focus(), 1);
    }

    #[test]
    fn editor_state_nav_up_moves_focus() {
        let mut state = two_field_state();
        state.apply(EditorEvent::NavDown);
        assert_eq!(state.focus(), 1);
        state.apply(EditorEvent::NavUp);
        assert_eq!(state.focus(), 0);
    }

    #[test]
    fn editor_state_nav_right_moves_focus() {
        let mut state = two_field_state();
        state.apply(EditorEvent::NavRight);
        assert_eq!(state.focus(), 1);
    }

    #[test]
    fn editor_state_nav_left_moves_focus() {
        let mut state = two_field_state();
        state.apply(EditorEvent::NavDown);
        state.apply(EditorEvent::NavLeft);
        assert_eq!(state.focus(), 0);
    }

    #[test]
    fn editor_state_focus_saturates_at_start() {
        let mut state = two_field_state();
        state.apply(EditorEvent::NavUp);
        assert_eq!(state.focus(), 0);
        state.apply(EditorEvent::NavLeft);
        assert_eq!(state.focus(), 0);
    }

    #[test]
    fn editor_state_focus_saturates_at_end() {
        let mut state = two_field_state();
        state.apply(EditorEvent::NavDown);
        state.apply(EditorEvent::NavDown);
        assert_eq!(state.focus(), 1);
        state.apply(EditorEvent::NavRight);
        assert_eq!(state.focus(), 1);
    }

    // -- edit mode: fine adjustments --

    #[test]
    fn editor_state_edit_mode_nav_right_fine_increase() {
        let mut state = two_field_state();
        state.apply(EditorEvent::EnterEditMode);
        let before = state.fields()[0].value();
        state.apply(EditorEvent::NavRight);
        assert_eq!(state.fields()[0].value(), before + 1.0);
    }

    #[test]
    fn editor_state_edit_mode_nav_left_fine_decrease() {
        let mut state = two_field_state();
        state.apply(EditorEvent::EnterEditMode);
        let before = state.fields()[0].value();
        state.apply(EditorEvent::NavLeft);
        assert_eq!(state.fields()[0].value(), before - 1.0);
    }

    // -- edit mode: coarse adjustments --

    #[test]
    fn editor_state_edit_mode_nav_up_coarse_increase() {
        let mut state = two_field_state();
        state.apply(EditorEvent::EnterEditMode);
        let before = state.fields()[0].value();
        state.apply(EditorEvent::NavUp);
        assert_eq!(state.fields()[0].value(), before + 10.0);
    }

    #[test]
    fn editor_state_edit_mode_nav_down_coarse_decrease() {
        let mut state = two_field_state();
        state.apply(EditorEvent::EnterEditMode);
        let before = state.fields()[0].value();
        state.apply(EditorEvent::NavDown);
        assert_eq!(state.fields()[0].value(), before - 10.0);
    }

    // -- clamping --

    #[test]
    fn editor_state_edit_clamps_at_max() {
        let mut state = EditorState::new(vec![ParamField::new(
            "vol", "Volume", 0.0, 100.0, 1.0, 98.0,
        )
        .unwrap()]);
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavUp);
        assert_eq!(state.fields()[0].value(), 100.0);
    }

    #[test]
    fn editor_state_edit_clamps_at_min() {
        let mut state =
            EditorState::new(vec![
                ParamField::new("vol", "Volume", 0.0, 100.0, 1.0, 2.0).unwrap()
            ]);
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavDown);
        assert_eq!(state.fields()[0].value(), 0.0);
    }

    #[test]
    fn editor_state_edit_fine_clamps_at_max() {
        let mut state = EditorState::new(vec![ParamField::new(
            "vol", "Volume", 0.0, 100.0, 1.0, 100.0,
        )
        .unwrap()]);
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavRight);
        assert_eq!(state.fields()[0].value(), 100.0);
    }

    #[test]
    fn editor_state_edit_fine_clamps_at_min() {
        let mut state =
            EditorState::new(vec![
                ParamField::new("vol", "Volume", 0.0, 100.0, 1.0, 0.0).unwrap()
            ]);
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavLeft);
        assert_eq!(state.fields()[0].value(), 0.0);
    }

    // -- in edit mode, focus does NOT move --

    #[test]
    fn editor_state_edit_mode_does_not_move_focus() {
        let mut state = two_field_state();
        state.apply(EditorEvent::EnterEditMode);
        let focus_before = state.focus();
        state.apply(EditorEvent::NavDown);
        state.apply(EditorEvent::NavUp);
        state.apply(EditorEvent::NavLeft);
        state.apply(EditorEvent::NavRight);
        assert_eq!(state.focus(), focus_before);
    }

    // -- empty fields edge case --

    #[test]
    fn editor_state_empty_fields_navigate_is_noop() {
        let mut state = EditorState::new(vec![]);
        state.apply(EditorEvent::NavDown);
        state.apply(EditorEvent::NavUp);
        assert_eq!(state.focus(), 0);
    }

    #[test]
    fn editor_state_empty_fields_edit_is_noop() {
        let mut state = EditorState::new(vec![]);
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavRight);
        state.apply(EditorEvent::NavLeft);
        state.apply(EditorEvent::NavUp);
        state.apply(EditorEvent::NavDown);
    }

    // -- focused field is adjusted, not others --

    #[test]
    fn editor_state_edit_only_adjusts_focused_field() {
        let mut state = two_field_state();
        let v0_before = state.fields()[0].value();
        let v1_before = state.fields()[1].value();

        state.apply(EditorEvent::NavDown);
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavRight);

        assert_eq!(state.fields()[0].value(), v0_before);
        assert_eq!(state.fields()[1].value(), v1_before + 1.0);
    }

    // -- step size respected --

    #[test]
    fn editor_state_step_size_respected() {
        let mut state = EditorState::new(vec![ParamField::new(
            "cutoff", "Cutoff", 0.0, 20000.0, 10.0, 500.0,
        )
        .unwrap()]);
        state.apply(EditorEvent::EnterEditMode);
        state.apply(EditorEvent::NavRight); // fine: +1 step = +10
        assert_eq!(state.fields()[0].value(), 510.0);
        state.apply(EditorEvent::NavUp); // coarse: +10 steps = +100
        assert_eq!(state.fields()[0].value(), 610.0);
    }
}
