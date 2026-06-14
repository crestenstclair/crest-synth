// path: src/shell/app_window.rs

/// Configuration for creating an application window.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Title displayed in the window title bar.
    pub title: String,
    /// Initial width in logical pixels.
    pub width: u32,
    /// Initial height in logical pixels.
    pub height: u32,
}

impl WindowConfig {
    /// Creates a new `WindowConfig` with the given title and dimensions.
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            title: title.into(),
            width,
            height,
        }
    }
}

/// An opaque handle to an open application window.
///
/// Returned by [`AppWindow::create`]; passed back to the port implementation
/// as needed. Callers should not construct this directly.
pub struct Window {
    pub(crate) config: WindowConfig,
}

impl Window {
    /// Creates a `Window` from a [`WindowConfig`].
    ///
    /// Intended for use by [`AppWindow`] implementations only.
    pub fn new(config: WindowConfig) -> Self {
        Self { config }
    }

    /// Returns the title of this window.
    pub fn title(&self) -> &str {
        &self.config.title
    }

    /// Returns the width of this window in logical pixels.
    pub fn width(&self) -> u32 {
        self.config.width
    }

    /// Returns the height of this window in logical pixels.
    pub fn height(&self) -> u32 {
        self.config.height
    }
}

/// Callback invoked on every UI frame.
///
/// The closure receives no arguments; implementations use captured state
/// (e.g., `Arc<Mutex<AppState>>`) to drive rendering per-frame.
pub type FrameCallback = Box<dyn FnMut() + Send + 'static>;

/// Port: application window lifecycle.
///
/// Implementations wire a concrete windowing back-end (e.g. `eframe` / `winit`)
/// behind this interface so that higher-level code stays back-end agnostic.
///
/// # Contract
/// - `create`: given a [`WindowConfig`], open or configure the native window
///   and return a [`Window`] handle.
/// - `run_loop`: enter the native event loop, calling `callback` on every
///   rendered frame until the window is closed.  Blocks until the loop exits.
pub trait AppWindow {
    /// Creates a window from the given configuration, returning a handle.
    fn create(&mut self, config: WindowConfig) -> Window;

    /// Runs the native event loop, invoking `callback` each frame.
    ///
    /// This method blocks until the window is closed or the loop is otherwise
    /// terminated by the underlying back-end.
    fn run_loop(&mut self, callback: FrameCallback);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── WindowConfig ─────────────────────────────────────────────────────────

    #[test]
    fn window_config_new_stores_fields() {
        let cfg = WindowConfig::new("My Synth", 1280, 720);
        assert_eq!(cfg.title, "My Synth");
        assert_eq!(cfg.width, 1280);
        assert_eq!(cfg.height, 720);
    }

    #[test]
    fn window_config_clone_is_independent() {
        let cfg = WindowConfig::new("Original", 800, 600);
        let mut clone = cfg.clone();
        clone.title = "Clone".to_string();
        assert_eq!(cfg.title, "Original");
    }

    // ── Window ──────────────────────────────────────────────────────────────

    #[test]
    fn window_exposes_config_via_accessors() {
        let cfg = WindowConfig::new("Test Window", 640, 480);
        let win = Window::new(cfg);
        assert_eq!(win.title(), "Test Window");
        assert_eq!(win.width(), 640);
        assert_eq!(win.height(), 480);
    }

    // ── AppWindow (stub impl) ───────────────────────────────────────────────

    /// A minimal stub implementation used only within this test module.
    struct StubWindow {
        frames_rendered: u32,
    }

    impl StubWindow {
        fn new() -> Self {
            Self { frames_rendered: 0 }
        }
    }

    impl AppWindow for StubWindow {
        fn create(&mut self, config: WindowConfig) -> Window {
            Window::new(config)
        }

        fn run_loop(&mut self, mut callback: FrameCallback) {
            // Simulate a brief event loop: call the callback a fixed number
            // of times, then return (simulating window close).
            for _ in 0..3 {
                callback();
                self.frames_rendered += 1;
            }
        }
    }

    #[test]
    fn stub_create_returns_window_with_correct_config() {
        let mut stub = StubWindow::new();
        let cfg = WindowConfig::new("Stub", 320, 240);
        let win = stub.create(cfg);
        assert_eq!(win.title(), "Stub");
        assert_eq!(win.width(), 320);
        assert_eq!(win.height(), 240);
    }

    #[test]
    fn stub_run_loop_invokes_callback_each_frame() {
        let mut stub = StubWindow::new();
        stub.run_loop(Box::new(move || {}));
        // StubWindow simulates 3 frames; frames_rendered should be 3.
        assert_eq!(stub.frames_rendered, 3);
    }

    #[test]
    fn app_window_trait_is_object_safe() {
        // Verify the trait can be used as a trait object.
        let stub: Box<dyn AppWindow> = Box::new(StubWindow::new());
        // Just hold the box; no method call needed to confirm object safety.
        drop(stub);
    }
}
