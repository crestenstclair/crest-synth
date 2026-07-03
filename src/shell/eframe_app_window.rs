// path: src/shell/eframe_app_window.rs

//! Adapter: EframeAppWindow
//!
//! Implements the `AppWindow` port using the `eframe` (egui) windowing
//! framework. `EframeAppWindow` owns and drives the native window's event
//! loop; everything it needs comes from the `App` bundle handed to `run`
//! (assembled by the composition root) and, optionally, from a
//! `NativeOptions` override injected at construction time. This type never
//! constructs its own `App` and never reaches into globals for window
//! configuration.

use eframe::egui;

use crate::shell::app_window::{App, AppWindow, WindowError};

/// Derives the `eframe::NativeOptions` window configuration from an `App`
/// bundle.
///
/// Pulled out as a free function so the mapping -- "how do App's
/// title/width/height become egui's viewport configuration" -- is
/// testable as plain data, without opening a real window.
fn native_options_for(app: &App) -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(app.title.clone())
            .with_inner_size([app.width as f32, app.height as f32]),
        ..Default::default()
    }
}

/// Minimal `eframe::App` implementor: renders a central panel showing the
/// application title.
///
/// Single Responsibility: this type's only job is drawing the one frame of
/// UI the window currently has to show; it owns no window lifecycle logic
/// (that belongs to `EframeAppWindow`) and no audio/MIDI/gamepad logic
/// (those belong to whatever collaborators a richer `App` bundle would
/// carry).
struct EframeUiApp {
    title: String,
}

impl eframe::App for EframeUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(&self.title);
        });
    }
}

/// Adapter: drives the top-level application window using `eframe`.
///
/// Implements the narrow `AppWindow` port (Interface Segregation): its only
/// responsibility is running the window's event loop to completion given an
/// assembled `App`. Callers depend on the `AppWindow` trait, never on this
/// concrete type (Dependency Inversion).
///
/// An explicit `NativeOptions` override can be injected via
/// [`EframeAppWindow::with_options`] -- e.g. to pin a specific window
/// configuration in a specialized caller. [`EframeAppWindow::new`] /
/// [`EframeAppWindow::default`] are convenience constructors for callers
/// who don't care and want the options derived from the `App` passed to
/// `run`.
pub struct EframeAppWindow {
    options_override: Option<eframe::NativeOptions>,
}

impl EframeAppWindow {
    /// Convenience constructor: window options are derived from the `App`
    /// passed to `run` at call time.
    pub fn new() -> Self {
        Self::with_options(None)
    }

    /// Full constructor: an explicit `NativeOptions` override, if present,
    /// takes precedence over the options this adapter would otherwise
    /// derive from `App`.
    pub fn with_options(options_override: Option<eframe::NativeOptions>) -> Self {
        Self { options_override }
    }
}

impl Default for EframeAppWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl AppWindow for EframeAppWindow {
    fn run(&mut self, app: App) -> Result<(), WindowError> {
        let options = self
            .options_override
            .clone()
            .unwrap_or_else(|| native_options_for(&app));

        let app_name = app.title.clone();
        let ui_app = EframeUiApp { title: app.title };

        eframe::run_native(&app_name, options, Box::new(|_cc| Ok(Box::new(ui_app))))
            .map_err(|err| WindowError::EventLoopFailed(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_options_for_uses_app_title_and_size() {
        let app = App::new("My Synth", 800, 600);

        let options = native_options_for(&app);

        assert_eq!(options.viewport.title.as_deref(), Some("My Synth"));
        assert_eq!(
            options.viewport.inner_size,
            Some(egui::Vec2::new(800.0, 600.0))
        );
    }

    #[test]
    fn with_options_none_defers_to_app_derived_options() {
        let window = EframeAppWindow::with_options(None);

        assert!(window.options_override.is_none());
    }

    #[test]
    fn default_constructs_without_explicit_options() {
        let window = EframeAppWindow::default();

        assert!(window.options_override.is_none());
    }

    #[test]
    fn with_options_some_stores_the_override() {
        let explicit = eframe::NativeOptions::default();

        let window = EframeAppWindow::with_options(Some(explicit));

        assert!(window.options_override.is_some());
    }
}
