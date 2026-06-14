// path: src/mixer/mixer_view_event.rs

/// Semantic input vocabulary of the mixer view.
///
/// Keyboard and gamepad adapters both emit **only** these events. The two input
/// paths are interchangeable — callers need not know the origin.
///
/// * `EnterEditMode` / `ExitEditMode` — track the Edit modifier (J key or a
///   face button) hold state. Emitted by the adapter when the modifier is
///   pressed or released.
/// * `ToggleFocusedParam` — emitted by the adapter on a **double-tap** of the
///   Edit modifier. Double-tap detection and timing logic live entirely in the
///   adapter; this store is timing-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MixerViewEvent {
    /// Move focus to the previous row / channel strip.
    NavUp,
    /// Move focus to the next row / channel strip.
    NavDown,
    /// Move focus to the previous parameter within the focused channel.
    NavLeft,
    /// Move focus to the next parameter within the focused channel.
    NavRight,
    /// The Edit modifier was pressed — enter value-editing mode.
    EnterEditMode,
    /// The Edit modifier was released — exit value-editing mode.
    ExitEditMode,
    /// Toggle the currently focused parameter (emitted on double-tap of Edit).
    ToggleFocusedParam,
}

#[cfg(test)]
mod tests {
    use super::MixerViewEvent;

    #[test]
    fn all_variants_debug() {
        let variants = [
            MixerViewEvent::NavUp,
            MixerViewEvent::NavDown,
            MixerViewEvent::NavLeft,
            MixerViewEvent::NavRight,
            MixerViewEvent::EnterEditMode,
            MixerViewEvent::ExitEditMode,
            MixerViewEvent::ToggleFocusedParam,
        ];
        for v in &variants {
            // Each variant must be representable as a debug string.
            let s = format!("{:?}", v);
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn clone_and_eq() {
        let a = MixerViewEvent::NavUp;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn copy_semantics() {
        let a = MixerViewEvent::ToggleFocusedParam;
        let b = a; // copy
        let _ = a; // still usable
        assert_eq!(a, b);
    }

    #[test]
    fn variants_are_distinct() {
        assert_ne!(MixerViewEvent::NavUp, MixerViewEvent::NavDown);
        assert_ne!(MixerViewEvent::NavLeft, MixerViewEvent::NavRight);
        assert_ne!(MixerViewEvent::EnterEditMode, MixerViewEvent::ExitEditMode);
        assert_ne!(
            MixerViewEvent::ToggleFocusedParam,
            MixerViewEvent::EnterEditMode
        );
    }
}
