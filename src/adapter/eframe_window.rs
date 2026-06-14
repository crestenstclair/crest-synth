// path: src/adapter/eframe_window.rs
//
// EframeWindow — eframe (winit + wgpu) backed implementation of the
// AppWindow port.
//
// # Design
//
// `EframeWindow` wraps eframe's `run_native` which drives the event loop and
// calls back into the provided `eframe::App` on each rendered frame.  The
// `FrameCallback` from the port contract is stored inside the `eframe::App`
// implementation and invoked every frame.
//
// Because eframe's `run_native` takes ownership and blocks until the window is
// closed, `run_loop` satisfies the port's "blocks until the loop exits"
// contract exactly.
//
// # Thread-safety note
//
// `eframe::run_native` must be called on the main thread on some platforms
// (particularly macOS).  Do not call `run_loop` from a background thread.

use eframe::egui;

use crate::shell::app_window::{AppWindow, FrameCallback, Window, WindowConfig};

// ─── EframeApp (internal) ──────────────────────────────────────────────────

/// Internal `eframe::App` wrapper that invokes the `FrameCallback` each frame.
struct EframeApp {
    callback: FrameCallback,
}

impl EframeApp {
    fn new(callback: FrameCallback) -> Self {
        Self { callback }
    }
}

impl eframe::App for EframeApp {
    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        (self.callback)();
    }
}

// ─── EframeWindow ──────────────────────────────────────────────────────────

/// eframe-backed [`AppWindow`] implementation.
///
/// Uses `eframe` (winit + wgpu) as the window/rendering back-end.  The
/// `create` method records window configuration; `run_loop` enters the native
/// event loop, invoking the supplied [`FrameCallback`] once per frame until
/// the user closes the window.
///
/// # Platform note
///
/// On macOS the event loop must run on the main thread.  Never call
/// `run_loop` from a spawned thread.
pub struct EframeWindow {
    /// Stored window configuration from the most recent `create` call.
    config: Option<WindowConfig>,
}

impl EframeWindow {
    /// Creates a new `EframeWindow` with no configuration yet.
    pub fn new() -> Self {
        Self { config: None }
    }
}

impl Default for EframeWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl AppWindow for EframeWindow {
    /// Records the window configuration, returning an opaque [`Window`] handle.
    ///
    /// The native window is not opened until [`run_loop`][Self::run_loop] is
    /// called, because eframe creates the window as part of starting the event
    /// loop.
    fn create(&mut self, config: WindowConfig) -> Window {
        let window = Window::new(config.clone());
        self.config = Some(config);
        window
    }

    /// Enters the eframe/winit event loop, calling `callback` on every frame.
    ///
    /// This method blocks until the native window is closed.  If `create` has
    /// not been called first, sensible defaults are used (800 × 600, title
    /// "crest-synth").
    fn run_loop(&mut self, callback: FrameCallback) {
        let cfg = self.config.take().unwrap_or_else(|| WindowConfig {
            title: "crest-synth".to_string(),
            width: 800,
            height: 600,
        });

        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title(&cfg.title)
                .with_inner_size([cfg.width as f32, cfg.height as f32]),
            ..Default::default()
        };

        let _ = eframe::run_native(
            &cfg.title,
            native_options,
            Box::new(|_cc| Ok(Box::new(EframeApp::new(callback)))),
        );
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::app_window::AppWindow;

    #[test]
    fn create_returns_window_with_correct_config() {
        let mut ew = EframeWindow::new();
        let cfg = WindowConfig::new("Test", 1024, 768);
        let win = ew.create(cfg);
        assert_eq!(win.title(), "Test");
        assert_eq!(win.width(), 1024);
        assert_eq!(win.height(), 768);
    }

    #[test]
    fn default_creates_unconfigured_window() {
        let ew = EframeWindow::default();
        // No config set yet — just verify it constructs without panic.
        assert!(ew.config.is_none());
    }

    #[test]
    fn eframe_window_implements_app_window_trait() {
        // Verify the trait can be used as a trait object.
        let window: Box<dyn AppWindow> = Box::new(EframeWindow::new());
        drop(window);
    }

    #[test]
    fn create_stores_config_for_run_loop() {
        let mut ew = EframeWindow::new();
        let cfg = WindowConfig::new("SynthUI", 1280, 720);
        let _ = ew.create(cfg);
        let stored = ew
            .config
            .as_ref()
            .expect("config must be stored after create");
        assert_eq!(stored.title, "SynthUI");
        assert_eq!(stored.width, 1280);
        assert_eq!(stored.height, 720);
    }
}
