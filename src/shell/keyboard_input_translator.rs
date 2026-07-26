use crate::control::app_event::{AppEvent, Direction};
use crate::control::top_level_context::TopLevelContext;
use crate::shell::window_input::{WindowInput, WindowInputKind, WindowKey};

/// Translates normalized window input into the closed application-event vocabulary.
///
/// The translator owns only the transient state of the K modifier. It never
/// owns or mutates application selection, Patch parameters, projections, or
/// audio state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyboardInputTranslator {
    k_held: bool,
}

impl KeyboardInputTranslator {
    /// Creates a translator with no modifier held.
    pub const fn new() -> Self {
        Self { k_held: false }
    }

    /// Translates one normalized window input into at most one semantic event.
    pub fn translate(&mut self, event: WindowInput) -> Option<AppEvent> {
        match event.kind() {
            WindowInputKind::FocusLost => {
                self.k_held = false;
                None
            }
            WindowInputKind::KeyUp => {
                if event.key() == WindowKey::K {
                    self.k_held = false;
                }
                None
            }
            WindowInputKind::KeyDown => self.translate_key_down(event.key()),
        }
    }

    fn translate_key_down(&mut self, key: WindowKey) -> Option<AppEvent> {
        match key {
            WindowKey::Digit1 => return Some(AppEvent::SelectContext(TopLevelContext::Mixer)),
            WindowKey::Digit2 => return Some(AppEvent::SelectContext(TopLevelContext::Patch)),
            _ => {}
        }
        if key == WindowKey::K {
            self.k_held = true;
            return None;
        }

        let direction = match key {
            WindowKey::W => Direction::Up,
            WindowKey::S => Direction::Down,
            WindowKey::A => Direction::Left,
            WindowKey::D => Direction::Right,
            WindowKey::Digit1 | WindowKey::Digit2 | WindowKey::K | WindowKey::Other => return None,
        };

        Some(if self.k_held {
            AppEvent::Adjust(direction)
        } else {
            AppEvent::Navigate(direction)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::KeyboardInputTranslator;
    use crate::control::app_event::{AppEvent, Direction};
    use crate::control::top_level_context::TopLevelContext;
    use crate::shell::window_input::{WindowInput, WindowKey};

    const DIRECTION_CASES: [(WindowKey, Direction); 4] = [
        (WindowKey::W, Direction::Up),
        (WindowKey::S, Direction::Down),
        (WindowKey::A, Direction::Left),
        (WindowKey::D, Direction::Right),
    ];

    #[test]
    fn keyboard_input_translator_maps_direct_context_keys_independent_of_modifier() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Digit1)),
            Some(AppEvent::SelectContext(TopLevelContext::Mixer))
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::K)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::Digit2)),
            Some(AppEvent::SelectContext(TopLevelContext::Patch))
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
                Some(AppEvent::Navigate(direction))
            );
        }
    }

    #[test]
    fn keyboard_input_translator_maps_every_modified_direction_to_adjustment() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::K)),
            None
        );

        for (key, direction) in DIRECTION_CASES {
            assert_eq!(
                translator.translate(WindowInput::key_down(key)),
                Some(AppEvent::Adjust(direction))
            );
        }
    }

    #[test]
    fn keyboard_input_translator_k_release_restores_navigation() {
        let mut translator = KeyboardInputTranslator::new();

        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::K)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_up(WindowKey::K)),
            None
        );
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::W)),
            Some(AppEvent::Navigate(Direction::Up))
        );
    }

    #[test]
    fn keyboard_input_translator_focus_loss_clears_the_modifier() {
        let mut translator = KeyboardInputTranslator::new();

        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::K)),
            None
        );
        assert_eq!(translator.translate(WindowInput::focus_lost()), None);
        assert_eq!(
            translator.translate(WindowInput::key_down(WindowKey::S)),
            Some(AppEvent::Navigate(Direction::Down))
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
            Some(AppEvent::Navigate(Direction::Right))
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
}
