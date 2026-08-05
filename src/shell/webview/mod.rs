//! Webview shell modules (mission webview-shell-foundation-01KZ9DN7).
//!
//! WP01 contributed the native input-capture path; WP02 adds launch-time
//! shell selection ([`ShellSelection`]), the typed webview startup error
//! ([`WebviewShellError`]), and the Tauri v2 window composition
//! ([`TauriWebviewWindow`]). WP03 adds the two page-bound transports:
//! [`projection_channel`] (generation-gated serialized view models) and
//! [`meter_channel`] (30 Hz latest-value meter frames). The real projection
//! page arrives with later work packages.
//!
//! # Selection is a launch-time decision
//!
//! Which shell runs is decided exactly once, at launch, by parsing
//! `--shell <egui|webview>` at the composition root (default `egui`). There
//! is no fallback edge from one shell to the other in any code path: a
//! webview startup failure is a typed [`WebviewShellError`] that ends the
//! process through the same fatal-error path every other startup failure
//! uses — it never opens the eframe window instead, retries silently, or
//! leaves a blank window behind.
//!
//! # Deterministic init-failure hook
//!
//! For the acceptance test that proves the typed failure path (WP06, T025):
//! setting the `CREST_WEBVIEW_PAGE` environment variable overrides the page
//! the window serves with the file at that path. An unreadable path is
//! [`WebviewShellError::PageLoadFailed`], raised before any window is
//! created, so the process exits nonzero with no blank window lingering.
//! The hook is an internal test seam, not an operator surface.

pub mod input_capture;
pub mod meter_channel;
pub mod projection_channel;
pub mod token_export;
pub mod window;

pub use window::TauriWebviewWindow;

use crate::shell::app_window::WindowError;
use core::fmt;
use input_capture::InputCaptureError;

/// The launch-time shell decision parsed from `--shell <egui|webview>`.
///
/// Egui is the default. The selection is made once at the composition root
/// and never revisited: no code path falls back from one shell to the other
/// after a failure (crest-spec `adapter.TauriWebviewWindow` rule).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShellSelection {
    /// The default eframe/egui window (`EframeGraphicalWindow`).
    #[default]
    Egui,
    /// The explicitly selected Tauri v2 webview window
    /// ([`TauriWebviewWindow`]).
    Webview,
}

/// A typed webview shell startup failure.
///
/// Every variant carries its underlying cause. The error surfaces through
/// the `AppWindow` port's declared `WindowError` into
/// `ApplicationError::Window` — the same top-level fatal path every other
/// startup failure uses — and ends the process nonzero. There is no
/// fallback to the eframe shell.
#[derive(Debug)]
pub enum WebviewShellError {
    /// The Tauri runtime (system webview) failed to initialize.
    RuntimeUnavailable(tauri::Error),
    /// The page the window must serve could not be loaded.
    PageLoadFailed {
        /// The page source that failed to load.
        page: String,
        /// The underlying read failure.
        source: std::io::Error,
    },
    /// The webview window itself could not be created.
    WindowCreation(tauri::Error),
    /// The native Rust-side key monitor could not be installed.
    InputCapture(InputCaptureError),
}

impl fmt::Display for WebviewShellError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeUnavailable(error) => {
                write!(formatter, "webview runtime unavailable: {error}")
            }
            Self::PageLoadFailed { page, source } => {
                write!(formatter, "webview page {page} failed to load: {source}")
            }
            Self::WindowCreation(error) => {
                write!(formatter, "webview window creation failed: {error}")
            }
            Self::InputCapture(error) => {
                write!(formatter, "webview input capture failed: {error}")
            }
        }
    }
}

impl std::error::Error for WebviewShellError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RuntimeUnavailable(error) | Self::WindowCreation(error) => Some(error),
            Self::PageLoadFailed { source, .. } => Some(source),
            Self::InputCapture(error) => Some(error),
        }
    }
}

/// Joins the typed webview failure onto the `AppWindow` port's declared
/// error, preserving the cause text, so it travels the same
/// `ApplicationError::Window` fatal path as every other window failure.
impl From<WebviewShellError> for WindowError {
    fn from(error: WebviewShellError) -> Self {
        WindowError::new(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{InputCaptureError, ShellSelection, WebviewShellError};
    use crate::shell::app_window::WindowError;
    use std::error::Error;

    #[test]
    fn shell_selection_defaults_to_egui() {
        assert_eq!(ShellSelection::default(), ShellSelection::Egui);
    }

    #[test]
    fn webview_shell_error_preserves_each_typed_cause() {
        let page_error = WebviewShellError::PageLoadFailed {
            page: "/missing/page.html".to_owned(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        };
        assert_eq!(
            page_error.to_string(),
            "webview page /missing/page.html failed to load: no such file"
        );
        assert!(page_error.source().is_some());

        let capture_error = WebviewShellError::InputCapture(InputCaptureError::NotMainThread);
        assert_eq!(
            capture_error.to_string(),
            "webview input capture failed: input capture must be installed on the main thread"
        );
        assert!(capture_error.source().is_some());
    }

    #[test]
    fn webview_shell_error_joins_the_window_error_path_with_its_cause_text() {
        let error = WebviewShellError::PageLoadFailed {
            page: "/missing/page.html".to_owned(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        };
        let window_error = WindowError::from(error);
        assert_eq!(
            window_error.message(),
            "webview page /missing/page.html failed to load: no such file"
        );
    }
}
