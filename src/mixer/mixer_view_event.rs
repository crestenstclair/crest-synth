// path: src/mixer/mixer_view_event.rs

//! The semantic input vocabulary of the mixer view.
//!
//! Keyboard and gamepad adapters both emit ONLY these variants. Timing
//! concerns (Edit-hold detection, double-tap detection) live entirely in the
//! input adapter that produces these events — `MixerViewEvent` itself and the
//! `MixerView` reducer that consumes it are timing-free, which keeps the
//! store a pure reducer that unit tests can drive with plain event
//! sequences.

/// A single semantic input event understood by the mixer view's reducer.
///
/// `EnterEditMode` / `ExitEditMode` track the Edit modifier (keyboard `J` /
/// gamepad face button) being held down. `ToggleFocusedParam` is emitted by
/// the input adapter on a double-tap of Edit; the double-tap timing window
/// is entirely the adapter's concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MixerViewEvent {
    /// Move focus up (e.g. to the previous parameter row on a channel).
    NavUp,
    /// Move focus down (e.g. to the next parameter row on a channel).
    NavDown,
    /// Move focus left (e.g. to the previous channel strip).
    NavLeft,
    /// Move focus right (e.g. to the next channel strip).
    NavRight,
    /// The Edit modifier has been pressed/held down.
    EnterEditMode,
    /// The Edit modifier has been released.
    ExitEditMode,
    /// Toggle the currently focused parameter (emitted on Edit double-tap).
    ToggleFocusedParam,
}

#[cfg(test)]
mod tests {
    use super::MixerViewEvent;

    #[test]
    fn variants_are_distinct() {
        let all = [
            MixerViewEvent::NavUp,
            MixerViewEvent::NavDown,
            MixerViewEvent::NavLeft,
            MixerViewEvent::NavRight,
            MixerViewEvent::EnterEditMode,
            MixerViewEvent::ExitEditMode,
            MixerViewEvent::ToggleFocusedParam,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn event_is_copy_and_hashable() {
        use std::collections::HashSet;
        let mut set: HashSet<MixerViewEvent> = HashSet::new();
        set.insert(MixerViewEvent::NavUp);
        let copied = MixerViewEvent::NavUp;
        set.insert(copied);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn events_are_debug_formattable() {
        assert_eq!(format!("{:?}", MixerViewEvent::NavUp), "NavUp");
        assert_eq!(
            format!("{:?}", MixerViewEvent::ToggleFocusedParam),
            "ToggleFocusedParam"
        );
    }
}
