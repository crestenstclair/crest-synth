use crate::control::app_event::Direction;
use crate::control::top_level_context::TopLevelContext;
use crate::control::{InteractionMode, SemanticAction};
use crate::shell::window_input::{WindowInput, WindowInputKind, WindowKey};

/// Translates normalized window input into the closed semantic-action vocabulary.
///
/// The translator owns only transient modifier/hold state. It never
/// owns or mutates application selection, Patch parameters, projections, or
/// audio state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyboardInputTranslator {
    k_held: bool,
    shift_held: bool,
    start_held: bool,
    start_preview_held: bool,
}

impl KeyboardInputTranslator {
    /// Creates a translator with no modifier held.
    pub const fn new() -> Self {
        Self {
            k_held: false,
            shift_held: false,
            start_held: false,
            start_preview_held: false,
        }
    }

    /// Translates one normalized window input into at most one semantic action.
    pub fn translate(&mut self, event: WindowInput) -> Option<SemanticAction> {
        match event.kind() {
            WindowInputKind::FocusLost => {
                self.k_held = false;
                self.shift_held = false;
                self.start_held = false;
                if core::mem::take(&mut self.start_preview_held) {
                    Some(SemanticAction::PreviewStop)
                } else {
                    Some(SemanticAction::SetInteractionMode(
                        InteractionMode::Navigate,
                    ))
                }
            }
            WindowInputKind::KeyUp => match event.key() {
                WindowKey::K => {
                    self.k_held = false;
                    Some(SemanticAction::SetInteractionMode(
                        InteractionMode::Navigate,
                    ))
                }
                WindowKey::Shift => {
                    self.shift_held = false;
                    None
                }
                WindowKey::Space if core::mem::take(&mut self.start_held) => {
                    core::mem::take(&mut self.start_preview_held)
                        .then_some(SemanticAction::PreviewStop)
                }
                _ => None,
            },
            WindowInputKind::KeyDown => self.translate_key_down(event.key()),
        }
    }

    fn translate_key_down(&mut self, key: WindowKey) -> Option<SemanticAction> {
        match key {
            WindowKey::Digit1 => {
                return Some(SemanticAction::SelectContext(TopLevelContext::Mixer))
            }
            WindowKey::Digit2 => {
                return Some(SemanticAction::SelectContext(TopLevelContext::Patch))
            }
            // Q/E remain direct compatibility bindings for Patch stepping;
            // the authored Shift+Left/Right gesture below reaches the same
            // semantic action. Holding K must not turn either into an edit.
            WindowKey::Q => return Some(SemanticAction::SelectPatch(Direction::Left)),
            WindowKey::E => return Some(SemanticAction::SelectPatch(Direction::Right)),
            _ => {}
        }
        if key == WindowKey::K {
            self.k_held = true;
            return Some(SemanticAction::SetInteractionMode(InteractionMode::Adjust));
        }
        if key == WindowKey::Shift {
            self.shift_held = true;
            return None;
        }
        if key == WindowKey::Return {
            return Some(SemanticAction::Activate);
        }
        if key == WindowKey::Space {
            if self.start_held {
                return None;
            }
            self.start_held = true;
            if self.shift_held {
                self.start_preview_held = false;
                return Some(SemanticAction::OpenMidiSettings);
            }
            self.start_preview_held = true;
            return Some(SemanticAction::PreviewStart);
        }

        let direction = match key {
            WindowKey::W => Direction::Up,
            WindowKey::S => Direction::Down,
            WindowKey::A => Direction::Left,
            WindowKey::D => Direction::Right,
            // `Digit3` through `Digit9`, `Digit0`, and the two bracket keys are
            // normalized at the window boundary and bound nowhere here. A scene
            // that pages a gallery binds them itself, scene-locally; routing
            // them through the translator would make paging a semantic action,
            // which it is not. An unbound key therefore produces no action at
            // all rather than an approximate one.
            WindowKey::Digit1
            | WindowKey::Digit2
            | WindowKey::Digit3
            | WindowKey::Digit4
            | WindowKey::Digit5
            | WindowKey::Digit6
            | WindowKey::Digit7
            | WindowKey::Digit8
            | WindowKey::Digit9
            | WindowKey::Digit0
            | WindowKey::BracketLeft
            | WindowKey::BracketRight
            | WindowKey::Q
            | WindowKey::E
            | WindowKey::K
            | WindowKey::Shift
            | WindowKey::Return
            | WindowKey::Space
            | WindowKey::Other => return None,
        };

        if self.shift_held {
            return match direction {
                Direction::Up => Some(SemanticAction::OpenRelated),
                Direction::Down => Some(SemanticAction::Return),
                Direction::Left => Some(SemanticAction::SelectPatch(Direction::Left)),
                Direction::Right => Some(SemanticAction::SelectPatch(Direction::Right)),
            };
        }

        Some(if self.k_held {
            SemanticAction::Adjust(direction)
        } else {
            SemanticAction::Navigate(direction)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::KeyboardInputTranslator;
    use crate::control::app_event::Direction;
    use crate::control::top_level_context::TopLevelContext;
    use crate::control::{InteractionMode, SemanticAction};
    use crate::shell::window_input::{WindowInput, WindowKey};

    const DIRECTION_CASES: [(WindowKey, Direction); 4] = [
        (WindowKey::W, Direction::Up),
        (WindowKey::S, Direction::Down),
        (WindowKey::A, Direction::Left),
        (WindowKey::D, Direction::Right),
    ];

    /// The keys the window normalizes and this translator binds to nothing.
    ///
    /// The four the gallery added — `Digit9`, `Digit0`, and the two brackets —
    /// are here for the same reason the six digits are: normalizing a key so a
    /// scene may bind it locally must not give it an application meaning.
    const UNBOUND_DIGITS: [WindowKey; 10] = [
        WindowKey::Digit3,
        WindowKey::Digit4,
        WindowKey::Digit5,
        WindowKey::Digit6,
        WindowKey::Digit7,
        WindowKey::Digit8,
        WindowKey::Digit9,
        WindowKey::Digit0,
        WindowKey::BracketLeft,
        WindowKey::BracketRight,
    ];

    #[test]
    fn keyboard_input_translator_maps_direct_context_keys_independent_of_modifier() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Digit1)),
            Some(SemanticAction::SelectContext(TopLevelContext::Mixer))
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::K)),
            Some(SemanticAction::SetInteractionMode(InteractionMode::Adjust))
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Digit2)),
            Some(SemanticAction::SelectContext(TopLevelContext::Patch))
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::Digit1)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::Digit2)),
            None
        );
    }

    #[test]
    fn keyboard_input_translator_maps_every_bare_direction_to_navigation() {
        let mut translator = KeyboardInputTranslator::new();

        for (key, direction) in DIRECTION_CASES {
            assert_eq!(
                translator.translate(WindowInput::key_down(key)),
                Some(SemanticAction::Navigate(direction))
            );
        }
    }

    #[test]
    fn keyboard_input_translator_maps_every_modified_direction_to_adjustment() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::K)),
            Some(SemanticAction::SetInteractionMode(InteractionMode::Adjust))
        );

        for (key, direction) in DIRECTION_CASES {
            assert_eq!(
                translator.translate(WindowInput::key_down(key)),
                Some(SemanticAction::Adjust(direction))
            );
        }
    }

    #[test]
    fn keyboard_input_translator_k_release_restores_navigation() {
        let mut translator = KeyboardInputTranslator::new();

        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::K)),
            Some(SemanticAction::SetInteractionMode(InteractionMode::Adjust))
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::K)),
            Some(SemanticAction::SetInteractionMode(
                InteractionMode::Navigate
            ))
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::W)),
            Some(SemanticAction::Navigate(Direction::Up))
        );
    }

    #[test]
    fn keyboard_input_translator_focus_loss_clears_the_modifier() {
        let mut translator = KeyboardInputTranslator::new();

        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::K)),
            Some(SemanticAction::SetInteractionMode(InteractionMode::Adjust))
        );
        assert_eq!(
            translator.translate(WindowInput::focus_lost()),
            Some(SemanticAction::SetInteractionMode(
                InteractionMode::Navigate
            ))
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::S)),
            Some(SemanticAction::Navigate(Direction::Down))
        );
    }

    #[test]
    fn keyboard_input_translator_ignores_direction_key_releases() {
        let mut translator = KeyboardInputTranslator::new();

        for (key, _) in DIRECTION_CASES {
            assert_eq!(translator.translate(WindowInput::key_up(key)), None);
        }

        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::D)),
            Some(SemanticAction::Navigate(Direction::Right))
        );
    }

    /// The digits the gallery pages with normalize at the window boundary and
    /// translate to nothing. If one of them ever produced a `SemanticAction`,
    /// scene-local paging would have leaked into the application's semantic
    /// vocabulary.
    #[test]
    fn keyboard_input_translator_leaves_every_unbound_digit_unbound() {
        let mut translator = KeyboardInputTranslator::new();

        for key in UNBOUND_DIGITS {
            assert_eq!(
                translator.translate(WindowInput::key_down(key)),
                None,
                "{key:?} produced a semantic action"
            );
            assert_eq!(
                translator.translate(WindowInput::key_up(key)),
                None,
                "releasing {key:?} produced a semantic action"
            );
        }
    }

    /// The unbound digits stay unbound while the adjust modifier is held, so
    /// they cannot become a modified gesture either.
    #[test]
    fn keyboard_input_translator_leaves_unbound_digits_unbound_under_the_modifier() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::K)),
            Some(SemanticAction::SetInteractionMode(InteractionMode::Adjust))
        );

        for key in UNBOUND_DIGITS {
            assert_eq!(
                translator.translate(WindowInput::key_down(key)),
                None,
                "{key:?} produced a semantic action while K was held"
            );
        }

        // The modifier survives: an unbound digit is ignored, not swallowed
        // along with the state it was pressed in.
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::W)),
            Some(SemanticAction::Adjust(Direction::Up))
        );
    }

    /// The two bound digits keep their existing context bindings.
    #[test]
    fn keyboard_input_translator_keeps_the_two_bound_context_digits() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Digit1)),
            Some(SemanticAction::SelectContext(TopLevelContext::Mixer))
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Digit2)),
            Some(SemanticAction::SelectContext(TopLevelContext::Patch))
        );
    }

    #[test]
    fn keyboard_input_translator_ignores_unrelated_keys() {
        let mut translator = KeyboardInputTranslator::new();

        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Other)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::Other)),
            None
        );
    }

    #[test]
    fn shift_vertical_maps_single_level_open_and_return_without_leaking_modifier_identity() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Shift)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::W)),
            Some(SemanticAction::OpenRelated)
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::S)),
            Some(SemanticAction::Return)
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::Shift)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::W)),
            Some(SemanticAction::Navigate(Direction::Up))
        );
    }

    #[test]
    fn shift_horizontal_maps_to_semantic_patch_navigation() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Shift)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::D)),
            Some(SemanticAction::SelectPatch(Direction::Right))
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::A)),
            Some(SemanticAction::SelectPatch(Direction::Left))
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::Shift)),
            None
        );
    }

    #[test]
    fn option_state_keyboard_grammar_maps_to_existing_semantic_actions_only() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::K)),
            Some(SemanticAction::SetInteractionMode(InteractionMode::Adjust))
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::W)),
            Some(SemanticAction::Adjust(Direction::Up)),
            "Edit+Up enters the reducer-owned option state"
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::K)),
            Some(SemanticAction::SetInteractionMode(
                InteractionMode::Navigate
            ))
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::W)),
            Some(SemanticAction::Navigate(Direction::Up))
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::S)),
            Some(SemanticAction::Navigate(Direction::Down))
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Return)),
            Some(SemanticAction::Activate),
            "Edit/Return chooses through the existing Activate action"
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Shift)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::S)),
            Some(SemanticAction::Return),
            "Shift+Down closes through the existing Return action"
        );
    }

    #[test]
    fn return_and_start_press_hold_release_have_exact_semantic_edges() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Return)),
            Some(SemanticAction::Activate)
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::Return)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Space)),
            Some(SemanticAction::PreviewStart)
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Space)),
            None,
            "key repeat must not fabricate a second preview request"
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::Space)),
            Some(SemanticAction::PreviewStop)
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::Space)),
            None
        );
    }

    #[test]
    fn shift_start_opens_settings_once_without_preview_release_and_keeps_shift_down_return() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Shift)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Space)),
            Some(SemanticAction::OpenMidiSettings)
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Space)),
            None,
            "repeat must not emit a second Settings action"
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::Space)),
            None,
            "Settings entry is not a preview hold"
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::S)),
            Some(SemanticAction::Return)
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::Shift)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Space)),
            Some(SemanticAction::PreviewStart),
            "bare Start remains Sample Browser preview"
        );
    }
}
