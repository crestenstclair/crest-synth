// path: src/shell/gamepad_action.rs

/// Actions produced by gamepad input and consumed by the shell's cursor/edit model.
///
/// Every navigation or editing gesture the controller can perform is represented
/// as a variant here.  The shell maps raw gilrs / HID events into these actions;
/// the rest of the UI never sees raw controller state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAction {
    /// Move the UI cursor (direction is context-dependent).
    Navigate,
    /// Confirm the current selection / enter a sub-menu.
    Select,
    /// Cancel or go up one level in the navigation hierarchy.
    Back,
    /// Increment the focused parameter value by one step.
    TweakUp,
    /// Decrement the focused parameter value by one step.
    TweakDown,
    /// Assign a modulation source to the focused parameter.
    AssignMod,
    /// Advance to the next page / tab in the current view.
    NextPage,
    /// Return to the previous page / tab in the current view.
    PreviousPage,
    /// Trigger an immediate save of the current session.
    QuickSave,
}

#[cfg(test)]
mod tests {
    use super::GamepadAction;

    #[test]
    fn all_variants_are_copy() {
        let action = GamepadAction::Navigate;
        let _copy = action; // copy semantics — original still usable
        let _also = action;
    }

    #[test]
    fn debug_format_is_not_empty() {
        let variants = [
            GamepadAction::Navigate,
            GamepadAction::Select,
            GamepadAction::Back,
            GamepadAction::TweakUp,
            GamepadAction::TweakDown,
            GamepadAction::AssignMod,
            GamepadAction::NextPage,
            GamepadAction::PreviousPage,
            GamepadAction::QuickSave,
        ];
        for v in &variants {
            assert!(!format!("{v:?}").is_empty());
        }
    }

    #[test]
    fn equality_holds() {
        assert_eq!(GamepadAction::Select, GamepadAction::Select);
        assert_ne!(GamepadAction::Select, GamepadAction::Back);
    }

    #[test]
    fn nine_distinct_variants() {
        use std::collections::HashSet;
        let set: HashSet<GamepadAction> = [
            GamepadAction::Navigate,
            GamepadAction::Select,
            GamepadAction::Back,
            GamepadAction::TweakUp,
            GamepadAction::TweakDown,
            GamepadAction::AssignMod,
            GamepadAction::NextPage,
            GamepadAction::PreviousPage,
            GamepadAction::QuickSave,
        ]
        .into_iter()
        .collect();
        assert_eq!(set.len(), 9);
    }
}
