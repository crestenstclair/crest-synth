// path: src/shell/app_window.rs

use std::fmt;

/// Errors that can occur while running the application window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowError {
    /// The windowing backend failed to initialize (e.g. no display server, no GPU surface).
    InitializationFailed(String),
    /// The window's event loop terminated with a failure.
    EventLoopFailed(String),
    /// Audio device setup required by the window's owning App failed.
    AudioDeviceUnavailable(String),
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowError::InitializationFailed(reason) => {
                write!(f, "window initialization failed: {reason}")
            }
            WindowError::EventLoopFailed(reason) => {
                write!(f, "window event loop failed: {reason}")
            }
            WindowError::AudioDeviceUnavailable(reason) => {
                write!(f, "audio device unavailable: {reason}")
            }
        }
    }
}

impl std::error::Error for WindowError {}

/// The application handle passed into the window's `run` entry point.
///
/// `App` is a plain data bundle assembled by composition-root code (e.g.
/// `main`) via dependency injection: every collaborator the window's UI/MIDI
/// thread might need is injected here rather than constructed inside an
/// `AppWindow` implementation. `AppWindow` implementations must not construct
/// their own `App` -- they only ever receive one.
pub struct App {
    /// Human-readable title shown in the window chrome.
    pub title: String,
    /// Requested window width in logical pixels.
    pub width: u32,
    /// Requested window height in logical pixels.
    pub height: u32,
}

impl App {
    /// Full constructor -- callers (composition root, tests) supply every field.
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            title: title.into(),
            width,
            height,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new("crest-synth", 1280, 720)
    }
}

/// Port: the entry point that owns and drives the top-level application
/// window and its event loop.
///
/// `AppWindow` is a narrow, single-method interface (Interface Segregation):
/// its only responsibility is "given an assembled `App`, run the window
/// until the user closes it or a fatal error occurs." It knows nothing about
/// how the window is drawn, how MIDI or gamepad input is polled, or how the
/// audio thread is wired -- those are the responsibilities of whatever `App`
/// was assembled to hold. Concrete adapters (winit, SDL, headless test
/// double, ...) implement this trait; callers depend only on the trait
/// (Dependency Inversion), never on a concrete windowing backend.
pub trait AppWindow {
    /// Run the window's event loop to completion.
    ///
    /// Takes ownership of `app` because the window becomes the sole owner of
    /// the top-level application state for the duration of the run -- no
    /// other code may mutate it concurrently. Returns `Ok(())` on a clean
    /// user-initiated shutdown, or `Err(WindowError)` if the window could not
    /// be created or its event loop failed.
    fn run(&mut self, app: App) -> Result<(), WindowError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test double: never touches a real display server, just records
    /// whether it was invoked and with what `App`, then returns a
    /// preconfigured result. This lets tests exercise the `AppWindow`
    /// contract without pulling in a real windowing backend.
    struct FakeAppWindow {
        result: Result<(), WindowError>,
        received_title: Option<String>,
    }

    impl FakeAppWindow {
        fn new(result: Result<(), WindowError>) -> Self {
            Self {
                result,
                received_title: None,
            }
        }
    }

    impl AppWindow for FakeAppWindow {
        fn run(&mut self, app: App) -> Result<(), WindowError> {
            self.received_title = Some(app.title);
            self.result.clone()
        }
    }

    #[test]
    fn run_returns_ok_on_clean_shutdown() {
        let mut window = FakeAppWindow::new(Ok(()));
        let app = App::new("test-app", 640, 480);

        let outcome = window.run(app);

        assert!(outcome.is_ok());
        assert_eq!(window.received_title.as_deref(), Some("test-app"));
    }

    #[test]
    fn run_propagates_initialization_failure() {
        let mut window = FakeAppWindow::new(Err(WindowError::InitializationFailed(
            "no display".to_string(),
        )));
        let app = App::default();

        let outcome = window.run(app);

        assert_eq!(
            outcome,
            Err(WindowError::InitializationFailed("no display".to_string()))
        );
    }

    #[test]
    fn app_default_provides_sensible_window_size() {
        let app = App::default();

        assert_eq!(app.title, "crest-synth");
        assert!(app.width > 0);
        assert!(app.height > 0);
    }

    #[test]
    fn window_error_display_includes_reason() {
        let error = WindowError::EventLoopFailed("panic in redraw".to_string());

        assert_eq!(
            error.to_string(),
            "window event loop failed: panic in redraw"
        );
    }
}
