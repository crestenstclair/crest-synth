// path: src/editor/editor_event.rs

/// Semantic input vocabulary for the editor UI.
///
/// Keyboard and gamepad adapters both translate their raw input into
/// `EditorEvent` values and nothing else. The editor state machine
/// consumes these events and updates `EditorState` accordingly.
///
/// # Examples
///
/// ```
/// use crest_synth::editor::editor_event::EditorEvent;
///
/// let event = EditorEvent::NavDown;
/// assert!(matches!(event, EditorEvent::NavDown));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorEvent {
    /// Move the cursor up in the editor UI.
    NavUp,
    /// Move the cursor down in the editor UI.
    NavDown,
    /// Move the cursor left in the editor UI.
    NavLeft,
    /// Move the cursor right in the editor UI.
    NavRight,
    /// Enter edit mode for the currently focused element.
    EnterEditMode,
    /// Exit edit mode and return to navigation mode.
    ExitEditMode,
}

#[cfg(test)]
mod tests {
    use super::EditorEvent;

    #[test]
    fn all_variants_are_distinct() {
        let variants = [
            EditorEvent::NavUp,
            EditorEvent::NavDown,
            EditorEvent::NavLeft,
            EditorEvent::NavRight,
            EditorEvent::EnterEditMode,
            EditorEvent::ExitEditMode,
        ];
        // Every pair must be distinct
        for i in 0..variants.len() {
            for j in 0..variants.len() {
                if i == j {
                    assert_eq!(variants[i], variants[j]);
                } else {
                    assert_ne!(variants[i], variants[j]);
                }
            }
        }
    }

    #[test]
    fn clone_produces_equal_value() {
        let original = EditorEvent::EnterEditMode;
        let cloned = original;
        assert_eq!(original, cloned);
    }

    #[test]
    fn debug_format_is_non_empty() {
        let event = EditorEvent::NavRight;
        let s = format!("{:?}", event);
        assert!(!s.is_empty());
    }

    #[test]
    fn copy_semantic() {
        let e = EditorEvent::ExitEditMode;
        let e2 = e; // Copy, not move
        assert_eq!(e, e2);
    }
}
