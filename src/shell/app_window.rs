use crate::control::app_event::AppEvent;
use crate::control::text_projection::TextProjection;
use core::fmt;
use std::time::Duration;

/// Semantic input emitted by a window adapter.
pub type AppInputCallback = Box<dyn FnMut(AppEvent) + 'static>;

/// Immutable view projection requested by a window adapter.
pub type ProjectionCallback = Box<dyn Fn() -> TextProjection + 'static>;

/// Periodic control-side work requested by a window adapter.
///
/// Returning `false` asks the disposable window to close after application
/// control ownership has retained a typed runtime failure.
pub type TickCallback = Box<dyn FnMut(Duration) -> bool + 'static>;

/// A failure while creating or running the application window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowError {
    message: String,
}

impl WindowError {
    /// Creates an actionable window startup or runtime error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the adapter-provided failure description.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for WindowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WindowError {}

/// Outbound port for the disposable single-text application view.
///
/// Implementations own raw key and modifier state. They emit only semantic
/// `AppEvent` values, request complete immutable `TextProjection` values, and
/// report elapsed control-side time through `on_tick`. They never own synth
/// parameters, application selection, or accepted application state.
pub trait AppWindow {
    /// Runs the window event loop until the player closes it.
    fn run(
        &self,
        on_input: AppInputCallback,
        projection: ProjectionCallback,
        on_tick: TickCallback,
    ) -> Result<(), WindowError>;
}

#[cfg(test)]
mod tests {
    use super::{AppInputCallback, AppWindow, ProjectionCallback, TickCallback, WindowError};
    use crate::control::app_event::{AppEvent, Direction};
    use crate::control::text_projection::TextProjection;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;

    struct TestWindow;

    impl AppWindow for TestWindow {
        fn run(
            &self,
            mut on_input: AppInputCallback,
            projection: ProjectionCallback,
            mut on_tick: TickCallback,
        ) -> Result<(), WindowError> {
            on_input(AppEvent::Navigate(Direction::Down));
            let projection = projection();
            assert_eq!(projection.body(), "KEYS: test");
            assert!(on_tick(Duration::from_millis(16)));
            Ok(())
        }
    }

    #[test]
    fn port_is_object_safe_and_routes_only_callbacks() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let projection_requests = Rc::new(Cell::new(0));
        let ticks = Rc::new(RefCell::new(Vec::new()));

        let events_for_callback = Rc::clone(&events);
        let on_input: AppInputCallback = Box::new(move |event| {
            events_for_callback.borrow_mut().push(event);
        });

        let requests_for_callback = Rc::clone(&projection_requests);
        let projection: ProjectionCallback = Box::new(move || {
            requests_for_callback.set(requests_for_callback.get() + 1);
            TextProjection::new("KEYS: test".to_owned(), 0, "state-hash".to_owned())
        });

        let ticks_for_callback = Rc::clone(&ticks);
        let on_tick: TickCallback = Box::new(move |duration| {
            ticks_for_callback.borrow_mut().push(duration);
            true
        });

        let window: &dyn AppWindow = &TestWindow;
        window
            .run(on_input, projection, on_tick)
            .expect("test window should run");

        assert_eq!(
            events.borrow().as_slice(),
            &[AppEvent::Navigate(Direction::Down)]
        );
        assert_eq!(projection_requests.get(), 1);
        assert_eq!(ticks.borrow().as_slice(), &[Duration::from_millis(16)]);
    }

    #[test]
    fn window_error_preserves_its_actionable_message() {
        let error = WindowError::new("window event loop failed");

        assert_eq!(error.message(), "window event loop failed");
        assert_eq!(error.to_string(), "window event loop failed");
    }
}
