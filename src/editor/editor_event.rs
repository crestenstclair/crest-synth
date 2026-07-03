// path: src/editor/editor_event.rs

//! Semantic events the editor store reacts to. Keyboard and gamepad
//! adapters both emit these — the store itself is timing- and
//! device-free, matching the MixerViewEvent precedent.

/// Events accepted by [`crate::editor::editor_state::EditorState::apply`].
///
/// In navigate mode the four `Nav*` variants move focus by one field,
/// saturating at the ends (no wrap). In edit mode they adjust the focused
/// field's value instead: `NavRight`/`NavLeft` by one fine unit,
/// `NavUp`/`NavDown` by ten fine units (coarse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorEvent {
    NavUp,
    NavDown,
    NavLeft,
    NavRight,
    EnterEditMode,
    ExitEditMode,
}
