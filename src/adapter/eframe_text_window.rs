use crate::control::app_event::AppEvent;
use crate::shell::app_window::{
    AppInputCallback, AppWindow, ProjectionCallback, TickCallback, WindowError,
};
use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
use crate::shell::window_input::{WindowInput, WindowKey};
use eframe::egui;
use std::time::Instant;

/// The eframe/egui adapter for crest-synth's single keyboard-controlled text view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EframeTextWindow {
    title: String,
}

impl EframeTextWindow {
    /// Creates a window with the supplied native title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
        }
    }

    /// Returns the native window title.
    pub fn title(&self) -> &str {
        &self.title
    }
}

impl Default for EframeTextWindow {
    fn default() -> Self {
        Self::new("crest-synth")
    }
}

impl AppWindow for EframeTextWindow {
    fn run(
        &self,
        on_input: AppInputCallback,
        projection: ProjectionCallback,
        on_tick: TickCallback,
    ) -> Result<(), WindowError> {
        let title = self.title.clone();
        eframe::run_native(
            &title,
            eframe::NativeOptions::default(),
            Box::new(move |_creation_context| {
                Ok(Box::new(EframeApplication::new(
                    on_input, projection, on_tick,
                )))
            }),
        )
        .map_err(|error| WindowError::new(format!("eframe window failed: {error}")))
    }
}

struct EframeApplication {
    on_input: AppInputCallback,
    projection: ProjectionCallback,
    on_tick: TickCallback,
    translator: KeyboardInputTranslator,
    previous_tick: Instant,
}

impl EframeApplication {
    fn new(
        on_input: AppInputCallback,
        projection: ProjectionCallback,
        on_tick: TickCallback,
    ) -> Self {
        Self {
            on_input,
            projection,
            on_tick,
            translator: KeyboardInputTranslator::new(),
            previous_tick: Instant::now(),
        }
    }

    fn handle_input(&mut self, context: &egui::Context) {
        context.input(|input| {
            for event in &input.events {
                if let Some(event) = translate_egui_event(&mut self.translator, event) {
                    (self.on_input)(event);
                }
            }
        });
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.previous_tick);
        self.previous_tick = now;
        (self.on_tick)(elapsed);
    }
}

impl eframe::App for EframeApplication {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_input(context);
        self.tick();

        let projection = (self.projection)();
        egui::CentralPanel::default().show(context, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let response = ui.label(egui::RichText::new(projection.body()).monospace());
                let selected_line = projection
                    .selected_line()
                    .min(projection.body().lines().count().saturating_sub(1));
                let line_height = ui.text_style_height(&egui::TextStyle::Monospace);
                let selected_top = response.rect.top() + line_height * selected_line as f32;
                let selected_rect = egui::Rect::from_min_size(
                    egui::pos2(response.rect.left(), selected_top),
                    egui::vec2(response.rect.width(), line_height),
                );
                ui.scroll_to_rect(selected_rect, Some(egui::Align::Center));
            });
        });

        context.request_repaint();
    }
}

fn translate_egui_event(
    translator: &mut KeyboardInputTranslator,
    event: &egui::Event,
) -> Option<AppEvent> {
    normalize_egui_event(event).and_then(|input| translator.translate(input))
}

fn normalize_egui_event(event: &egui::Event) -> Option<WindowInput> {
    match event {
        egui::Event::Key { key, pressed, .. } => {
            let key = normalize_key(*key);
            Some(if *pressed {
                WindowInput::key_down(key)
            } else {
                WindowInput::key_up(key)
            })
        }
        egui::Event::WindowFocused(false) => Some(WindowInput::focus_lost()),
        _ => None,
    }
}

fn normalize_key(key: egui::Key) -> WindowKey {
    match key {
        egui::Key::W => WindowKey::W,
        egui::Key::S => WindowKey::S,
        egui::Key::A => WindowKey::A,
        egui::Key::D => WindowKey::D,
        egui::Key::K => WindowKey::K,
        _ => WindowKey::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_egui_event, translate_egui_event, EframeTextWindow};
    use crate::control::app_event::{AppEvent, Direction};
    use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
    use crate::shell::window_input::{WindowInput, WindowKey};
    use eframe::egui::{self, Key, Modifiers};

    const DIRECTION_CASES: [(Key, WindowKey, Direction); 4] = [
        (Key::W, WindowKey::W, Direction::Up),
        (Key::S, WindowKey::S, Direction::Down),
        (Key::A, WindowKey::A, Direction::Left),
        (Key::D, WindowKey::D, Direction::Right),
    ];

    fn key_event(key: Key, pressed: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::default(),
        }
    }

    #[test]
    fn eframe_text_window_default_uses_the_product_name() {
        assert_eq!(EframeTextWindow::default().title(), "crest-synth");
    }

    #[test]
    fn eframe_text_window_normalizes_the_complete_key_vocabulary() {
        let cases = [
            (Key::W, WindowKey::W),
            (Key::S, WindowKey::S),
            (Key::A, WindowKey::A),
            (Key::D, WindowKey::D),
            (Key::K, WindowKey::K),
            (Key::Enter, WindowKey::Other),
        ];

        for (egui_key, window_key) in cases {
            assert_eq!(
                normalize_egui_event(&key_event(egui_key, true)),
                Some(WindowInput::key_down(window_key))
            );
            assert_eq!(
                normalize_egui_event(&key_event(egui_key, false)),
                Some(WindowInput::key_up(window_key))
            );
        }
    }

    #[test]
    fn eframe_text_window_normalizes_focus_loss_and_ignores_other_events() {
        assert_eq!(
            normalize_egui_event(&egui::Event::WindowFocused(false)),
            Some(WindowInput::focus_lost())
        );
        assert_eq!(
            normalize_egui_event(&egui::Event::WindowFocused(true)),
            None
        );
        assert_eq!(normalize_egui_event(&egui::Event::Copy), None);
    }

    #[test]
    fn eframe_text_window_delegates_every_direction_to_the_shared_translator() {
        let mut translator = KeyboardInputTranslator::new();

        for (egui_key, _, direction) in DIRECTION_CASES {
            assert_eq!(
                translate_egui_event(&mut translator, &key_event(egui_key, true)),
                Some(AppEvent::Navigate(direction))
            );
            assert_eq!(
                translate_egui_event(&mut translator, &key_event(egui_key, false)),
                None
            );
        }

        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::K, true)),
            None
        );
        for (egui_key, _, direction) in DIRECTION_CASES {
            assert_eq!(
                translate_egui_event(&mut translator, &key_event(egui_key, true)),
                Some(AppEvent::Adjust(direction))
            );
        }
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::Enter, true)),
            None
        );
    }

    #[test]
    fn eframe_text_window_delegates_release_and_focus_reset_without_private_state() {
        let mut translator = KeyboardInputTranslator::new();

        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::K, true)),
            None
        );
        assert_eq!(
            translate_egui_event(&mut translator, &egui::Event::WindowFocused(false)),
            None
        );
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::S, true)),
            Some(AppEvent::Navigate(Direction::Down))
        );

        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::K, true)),
            None
        );
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::K, false)),
            None
        );
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::A, true)),
            Some(AppEvent::Navigate(Direction::Left))
        );
    }
}
