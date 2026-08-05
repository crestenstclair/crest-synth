//! Rust-side native key capture for the webview shell.
//!
//! This is the winning path from the WP01 input-capture probe
//! (`kitty-specs/webview-shell-foundation-01KZ9DN7/research/input-capture-probe.md`):
//! an `NSEvent` local monitor installed from the Rust side at window setup.
//! The monitor observes every key event delivered to this process *before*
//! dispatch to the responder chain, so capture is independent of which view
//! (in practice the focused WKWebView) is first responder. The tao/Tauri
//! window-event path lost: `tauri::WindowEvent` carries no keyboard variant
//! at all, so window-level key capture through `on_window_event` is
//! structurally impossible under Tauri v2. The losing path is deliberately
//! absent from this module.
//!
//! The module emits [`RawKeyEvent`] values only. It owns no translator, no
//! modifier state, and no application state: the sink is expected to
//! normalize into [`crate::shell::window_input::WindowInput`] and feed the
//! shared [`crate::shell::KeyboardInputTranslator`], exactly as the eframe
//! adapter does — that wiring belongs to the webview window composition
//! (WP02), not here.
//!
//! Threading contract (macOS): [`install`] must be called on the main thread,
//! and the sink is invoked on the main thread — the same thread that runs the
//! Tauri event loop — so a `KeyboardInputTranslator` living in main-thread
//! state needs no synchronization. Dropping the returned handle removes the
//! monitor.

use crate::shell::window_input::WindowKey;
use core::fmt;

/// One key transition observed Rust-side before webview dispatch.
///
/// `key` is normalized to the canonical window-boundary vocabulary
/// ([`WindowKey`]) here, inside the platform module, so no platform key code
/// leaks past this boundary. Shift chords need no separate representation:
/// the macOS virtual key code identifies the physical key regardless of held
/// modifiers, so Shift+W arrives as `W` — the same normalization the eframe
/// path performs.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RawKeyEvent {
    key: WindowKey,
    pressed: bool,
    repeat: bool,
}

impl RawKeyEvent {
    /// Creates a raw key transition.
    pub const fn new(key: WindowKey, pressed: bool, repeat: bool) -> Self {
        Self {
            key,
            pressed,
            repeat,
        }
    }

    /// The normalized key identity.
    pub const fn key(&self) -> WindowKey {
        self.key
    }

    /// `true` for a press, `false` for a release.
    pub const fn pressed(&self) -> bool {
        self.pressed
    }

    /// `true` when this press is an OS auto-repeat of a held key.
    ///
    /// Releases are never repeats. The eframe path receives auto-repeats too
    /// (egui forwards them); the consumer decides what repeats mean.
    pub const fn repeat(&self) -> bool {
        self.repeat
    }
}

/// A failure installing the native key monitor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputCaptureError {
    /// [`install`] was called off the main thread.
    NotMainThread,
    /// The platform rejected the monitor installation.
    MonitorRejected,
    /// This platform has no native capture implementation yet.
    UnsupportedPlatform,
}

impl fmt::Display for InputCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMainThread => {
                formatter.write_str("input capture must be installed on the main thread")
            }
            Self::MonitorRejected => {
                formatter.write_str("the platform rejected the local key-event monitor")
            }
            Self::UnsupportedPlatform => formatter
                .write_str("native webview input capture is not implemented on this platform"),
        }
    }
}

impl std::error::Error for InputCaptureError {}

/// Maps a macOS virtual key code (ANSI layout positions) to the canonical
/// window-boundary vocabulary.
///
/// Virtual key codes name physical key positions, so this matches the
/// `kVK_ANSI_*` constants; on non-ANSI hardware layouts the physical position
/// wins, which is the standard behavior for position-based bindings (WASD).
/// Every unmapped code normalizes to [`WindowKey::Other`], mirroring the
/// eframe adapter's `normalize_key`.
pub const fn window_key_from_macos_key_code(key_code: u16) -> WindowKey {
    match key_code {
        18 => WindowKey::Digit1,
        19 => WindowKey::Digit2,
        20 => WindowKey::Digit3,
        21 => WindowKey::Digit4,
        23 => WindowKey::Digit5,
        22 => WindowKey::Digit6,
        26 => WindowKey::Digit7,
        28 => WindowKey::Digit8,
        25 => WindowKey::Digit9,
        29 => WindowKey::Digit0,
        33 => WindowKey::BracketLeft,
        30 => WindowKey::BracketRight,
        12 => WindowKey::Q,
        14 => WindowKey::E,
        13 => WindowKey::W,
        1 => WindowKey::S,
        0 => WindowKey::A,
        2 => WindowKey::D,
        40 => WindowKey::K,
        _ => WindowKey::Other,
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{window_key_from_macos_key_code, InputCaptureError, RawKeyEvent};
    use block2::RcBlock;
    use core::ptr::NonNull;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSEventMask, NSEventType};
    use std::cell::RefCell;

    /// Owns the installed local monitor; dropping it removes the monitor.
    pub struct InputCaptureHandle {
        monitor: Retained<AnyObject>,
        // The monitor block holds the sink; keep the handle main-thread-bound
        // so removal happens where installation did.
        _not_send: core::marker::PhantomData<*const ()>,
    }

    impl Drop for InputCaptureHandle {
        fn drop(&mut self) {
            // SAFETY: `monitor` is exactly the token returned by
            // `addLocalMonitorForEventsMatchingMask:handler:`.
            unsafe { NSEvent::removeMonitor(&self.monitor) };
        }
    }

    /// Installs the `NSEvent` local key monitor and feeds every key
    /// transition to `sink`. Events are returned to AppKit unchanged, so the
    /// webview still receives them; whether the product shell swallows
    /// vocabulary keys is the window composition's decision (WP02).
    pub fn install(
        sink: impl FnMut(RawKeyEvent) + 'static,
    ) -> Result<InputCaptureHandle, InputCaptureError> {
        if MainThreadMarker::new().is_none() {
            return Err(InputCaptureError::NotMainThread);
        }
        let sink = RefCell::new(sink);
        let handler = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
            // SAFETY: AppKit hands the monitor a valid event for the duration
            // of the call; we only read from it.
            let observed = unsafe { event.as_ref() };
            let event_type = observed.r#type();
            let pressed = match event_type {
                NSEventType::KeyDown => true,
                NSEventType::KeyUp => false,
                _ => return event.as_ptr(),
            };
            let raw = RawKeyEvent::new(
                window_key_from_macos_key_code(observed.keyCode()),
                pressed,
                pressed && observed.isARepeat(),
            );
            (sink.borrow_mut())(raw);
            // Pass the event through untouched.
            event.as_ptr()
        });
        // SAFETY: the block returns the pointer it was given (a valid event).
        let monitor = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                NSEventMask::KeyDown | NSEventMask::KeyUp,
                &handler,
            )
        };
        monitor
            .map(|monitor| InputCaptureHandle {
                monitor,
                _not_send: core::marker::PhantomData,
            })
            .ok_or(InputCaptureError::MonitorRejected)
    }
}

#[cfg(target_os = "macos")]
pub use platform::{install, InputCaptureHandle};

#[cfg(not(target_os = "macos"))]
/// Native capture is macOS-first (WKWebView); other platforms are declared
/// unsupported rather than silently degraded.
pub fn install(_sink: impl FnMut(RawKeyEvent) + 'static) -> Result<(), InputCaptureError> {
    Err(InputCaptureError::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use super::{window_key_from_macos_key_code, RawKeyEvent};
    use crate::shell::window_input::{WindowKey, ALL_WINDOW_KEYS};

    /// Every key in the normalized window vocabulary except `Other` is
    /// reachable from exactly one macOS virtual key code, so the capture path
    /// covers the full MIXER vocabulary the translator consumes.
    #[test]
    fn every_normalized_key_is_reachable_from_one_macos_key_code() {
        for key in ALL_WINDOW_KEYS {
            if key == WindowKey::Other {
                continue;
            }
            let codes = (0_u16..=127)
                .filter(|code| window_key_from_macos_key_code(*code) == key)
                .count();
            assert_eq!(codes, 1, "{key:?} must map from exactly one key code");
        }
    }

    #[test]
    fn unmapped_key_codes_normalize_to_other() {
        // kVK_ANSI_G (5) and kVK_Space (49) are outside the vocabulary.
        assert_eq!(window_key_from_macos_key_code(5), WindowKey::Other);
        assert_eq!(window_key_from_macos_key_code(49), WindowKey::Other);
    }

    #[test]
    fn raw_key_events_are_copyable_shell_data() {
        let event = RawKeyEvent::new(WindowKey::W, true, false);
        let copied = event;
        assert_eq!(copied, event);
        assert_eq!(event.key(), WindowKey::W);
        assert!(event.pressed());
        assert!(!event.repeat());
    }
}
