//! The Tauri v2 webview window — the explicitly selected peer of
//! `EframeGraphicalWindow` behind the `AppWindow` port.
//!
//! Composition follows the WP01 input-capture probe verdict
//! (`kitty-specs/webview-shell-foundation-01KZ9DN7/research/input-capture-probe.md`):
//!
//! - keys are captured Rust-side by [`input_capture::install`] (NSEvent local
//!   monitor, installed on the main thread before the event loop starts) and
//!   normalized through the same [`KeyboardInputTranslator`] the eframe
//!   window uses — one shared key state machine, no page key handler;
//! - the event loop runs through `run_return`, never `run`, so
//!   `AppWindow::run` returns and `StandaloneApplication::run` performs the
//!   same owned shutdown the eframe close performs (stream release before
//!   worker completion, graph ownership collection, normal exit);
//! - the tao loop waits when idle, so a detached waker pings the main thread
//!   at the 16 ms idle-frame cadence the eframe path uses and control-side
//!   ticks run on `MainEventsCleared`;
//! - `Focused(false)` feeds [`WindowInput::focus_lost`], clearing the held-K
//!   modifier exactly as the eframe adapter does;
//! - the page is served over the registered `crest://` custom protocol
//!   (`WebviewUrl::External` requires http(s) and data URLs are rejected at
//!   the config layer).
//!
//! The window serves the authored MIXER projection page (`webview-page/`,
//! WP05) with its generated token table, composition stylesheet, render
//! script, and vendored Azeret Mono faces, all embedded at compile time and
//! routed by request path through the `crest://` protocol handler. Page
//! resolution happens before any window exists, so an unloadable page is a
//! typed [`WebviewShellError::PageLoadFailed`] and the process ends with no
//! blank window. The deterministic trigger for that path is the
//! `CREST_WEBVIEW_PAGE` override documented in [`crate::shell::webview`].

use crate::shell::app_window::{
    AppInputCallback, AppWindow, AudioObservationCallback, FrameObservationCallback,
    ProjectionCallback, TickCallback, WindowError,
};
use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
use crate::shell::visual::ViewportDensityPolicy;
use crate::shell::webview::meter_channel::{MeterChannel, METER_EVENT};
use crate::shell::webview::projection_channel::{ProjectionChannel, PROJECTION_EVENT};
use crate::shell::webview::{input_capture, WebviewShellError};
use crate::shell::window_input::WindowInput;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// The idle tick cadence — the same 16 ms idle-frame convention the eframe
/// adapter declares as its repaint interval.
const TICK_INTERVAL: Duration = Duration::from_millis(16);

/// The single webview window's stable Tauri label.
const WINDOW_LABEL: &str = "main";

/// The deterministic init-failure hook consumed by the acceptance tests:
/// when set, the window serves the file at this path instead of the
/// built-in page, and an unreadable path is a typed
/// [`WebviewShellError::PageLoadFailed`] before any window opens.
const PAGE_OVERRIDE_ENV: &str = "CREST_WEBVIEW_PAGE";

/// The authored MIXER projection page (WP05, `webview-page/`), embedded at
/// compile time and served over the registered `crest://` protocol together
/// with its generated token table, composition stylesheet, render script,
/// and the vendored Azeret Mono faces. The page renders projections pushed
/// on the WP03 transports; it registers no key handler and captures no
/// input (keys stay Rust-side).
const PAGE_INDEX_HTML: &str = include_str!("../../../webview-page/index.html");
const PAGE_TOKENS_CSS: &str = include_str!("../../../webview-page/tokens.css");
const PAGE_CSS: &str = include_str!("../../../webview-page/page.css");
const PAGE_JS: &str = include_str!("../../../webview-page/page.js");
const FONT_AZERET_REGULAR: &[u8] =
    include_bytes!("../../../vendor/azeret-mono/AzeretMono-Regular.ttf");
const FONT_AZERET_MEDIUM: &[u8] =
    include_bytes!("../../../vendor/azeret-mono/AzeretMono-Medium.ttf");
const FONT_AZERET_SEMIBOLD: &[u8] =
    include_bytes!("../../../vendor/azeret-mono/AzeretMono-SemiBold.ttf");
const FONT_AZERET_BOLD: &[u8] = include_bytes!("../../../vendor/azeret-mono/AzeretMono-Bold.ttf");

/// Resolves one `crest://` request path to its embedded page asset as a
/// `(content type, body)` pair, or `None` for a path the page never
/// references (served as 404). `index_html` is passed in because the index
/// document — and only the index document — honors the deterministic
/// `CREST_WEBVIEW_PAGE` override.
fn page_asset(path: &str, index_html: &str) -> Option<(&'static str, Vec<u8>)> {
    match path {
        "/" | "/index.html" => Some(("text/html; charset=utf-8", index_html.as_bytes().to_vec())),
        "/tokens.css" => Some((
            "text/css; charset=utf-8",
            PAGE_TOKENS_CSS.as_bytes().to_vec(),
        )),
        "/page.css" => Some(("text/css; charset=utf-8", PAGE_CSS.as_bytes().to_vec())),
        "/page.js" => Some((
            "text/javascript; charset=utf-8",
            PAGE_JS.as_bytes().to_vec(),
        )),
        "/fonts/AzeretMono-Regular.ttf" => Some(("font/ttf", FONT_AZERET_REGULAR.to_vec())),
        "/fonts/AzeretMono-Medium.ttf" => Some(("font/ttf", FONT_AZERET_MEDIUM.to_vec())),
        "/fonts/AzeretMono-SemiBold.ttf" => Some(("font/ttf", FONT_AZERET_SEMIBOLD.to_vec())),
        "/fonts/AzeretMono-Bold.ttf" => Some(("font/ttf", FONT_AZERET_BOLD.to_vec())),
        _ => None,
    }
}

/// Production Tauri v2 webview window: the explicitly selected `AppWindow`
/// peer of `EframeGraphicalWindow`. Selecting it is a launch-time decision;
/// there is no fallback edge to or from the eframe shell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TauriWebviewWindow {
    title: String,
}

impl TauriWebviewWindow {
    /// Creates a webview window with the supplied native title.
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

impl Default for TauriWebviewWindow {
    fn default() -> Self {
        Self::new("crest-synth")
    }
}

/// Resolves the page the window serves, honoring the deterministic
/// `CREST_WEBVIEW_PAGE` override. Called before any window exists so a
/// failure is a typed startup error with no blank window.
fn resolve_page_source(override_path: Option<&Path>) -> Result<String, WebviewShellError> {
    match override_path {
        Some(path) => {
            std::fs::read_to_string(path).map_err(|source| WebviewShellError::PageLoadFailed {
                page: path.display().to_string(),
                source,
            })
        }
        None => Ok(PAGE_INDEX_HTML.to_owned()),
    }
}

/// Feeds captured raw keys through the shared production translator into the
/// application input callback. This is plumbing only: the one key state
/// machine is [`KeyboardInputTranslator`], shared with the eframe adapter.
struct KeyPipeline {
    translator: KeyboardInputTranslator,
    on_input: AppInputCallback,
}

impl KeyPipeline {
    fn feed(&mut self, input: WindowInput) {
        if let Some(action) = self.translator.translate(input) {
            (self.on_input)(action);
        }
    }
}

impl AppWindow for TauriWebviewWindow {
    fn run(
        &self,
        on_input: AppInputCallback,
        projection: ProjectionCallback,
        audio_observation: AudioObservationCallback,
        mut on_tick: TickCallback,
        on_frame: FrameObservationCallback,
    ) -> Result<(), WindowError> {
        // Page resolution comes first: an unloadable page is a typed error
        // before any runtime, monitor, or window exists (FR-007).
        let page = resolve_page_source(
            std::env::var_os(PAGE_OVERRIDE_ENV)
                .map(std::path::PathBuf::from)
                .as_deref(),
        )
        .map_err(WindowError::from)?;

        // `ShellFrameObservation` is post-paint evidence: it demands all five
        // declared shell regions with painted bounds and visible labels. The
        // page now paints the five regions and echoes each rendered
        // generation on the `crest://painted` event with post-paint region
        // evidence (bounds and visible labels keyed by ShellRegionId's
        // serialized names), so forwarding honest observations through this
        // callback is a listener away. That forwarding belongs to the
        // acceptance work package that measures it (WP06 T026); inventing
        // pre-paint region evidence here is what the port invariant forbids.
        let _on_frame = on_frame;

        let pipeline = Rc::new(RefCell::new(KeyPipeline {
            translator: KeyboardInputTranslator::new(),
            on_input,
        }));

        // The winning WP01 path: the NSEvent local monitor, installed from
        // the Rust side on the main thread before the event loop starts. The
        // sink runs on the main thread; events pass through unchanged so the
        // webview still receives them (the page registers no key handler).
        let capture_pipeline = Rc::clone(&pipeline);
        let _capture_handle = input_capture::install(move |raw| {
            let input = if raw.pressed() {
                WindowInput::key_down(raw.key())
            } else {
                WindowInput::key_up(raw.key())
            };
            capture_pipeline.borrow_mut().feed(input);
        })
        .map_err(|error| WindowError::from(WebviewShellError::InputCapture(error)))?;

        let app = tauri::Builder::default()
            .register_uri_scheme_protocol("crest", move |_context, request| {
                match page_asset(request.uri().path(), &page) {
                    Some((content_type, body)) => tauri::http::Response::builder()
                        .header("Content-Type", content_type)
                        .body(body)
                        .expect("the embedded asset response is well-formed"),
                    None => tauri::http::Response::builder()
                        .status(404)
                        .body(Vec::new())
                        .expect("the empty not-found response is well-formed"),
                }
            })
            .build(tauri::generate_context!())
            .map_err(|error| WindowError::from(WebviewShellError::RuntimeUnavailable(error)))?;

        let url: tauri::Url = "crest://localhost/index.html"
            .parse()
            .expect("the static page url is well-formed");
        let authored = ViewportDensityPolicy::Desktop.authored_viewport();
        let smallest = ViewportDensityPolicy::SteamDeck.authored_viewport();
        let window = WebviewWindowBuilder::new(&app, WINDOW_LABEL, WebviewUrl::CustomProtocol(url))
            .title(&self.title)
            .inner_size(f64::from(authored.width_px), f64::from(authored.height_px))
            .min_inner_size(f64::from(smallest.width_px), f64::from(smallest.height_px))
            .focused(true)
            .build()
            .map_err(|error| WindowError::from(WebviewShellError::WindowCreation(error)))?;
        let _ = window.set_focus();

        // The tao loop waits when idle; a detached waker keeps control-side
        // ticks flowing (fixture MIDI, structural advance, device status) at
        // the eframe idle cadence.
        let stop_waker = Arc::new(AtomicBool::new(false));
        let waker_stop = Arc::clone(&stop_waker);
        let waker_handle = app.handle().clone();
        let waker = std::thread::spawn(move || {
            while !waker_stop.load(Ordering::Relaxed) {
                std::thread::sleep(TICK_INTERVAL);
                if waker_handle.run_on_main_thread(|| {}).is_err() {
                    break;
                }
            }
        });

        let loop_pipeline = Rc::clone(&pipeline);
        let mut last_tick = Instant::now();
        let mut close_requested = false;

        // The first transport failure while the window still lives, surfaced
        // after the loop through the window's declared error path — the same
        // record-first-error-and-close-once treatment the eframe adapter
        // gives an invalid late frame.
        let transport_error: Rc<RefCell<Option<WindowError>>> = Rc::new(RefCell::new(None));
        let loop_transport_error = Rc::clone(&transport_error);
        let mut projection_channel = ProjectionChannel::new();
        let mut meter_channel = MeterChannel::new();

        let exit_code = app.run_return(move |handle, event| match event {
            RunEvent::WindowEvent {
                event: WindowEvent::Focused(false),
                ..
            } => {
                loop_pipeline.borrow_mut().feed(WindowInput::focus_lost());
            }
            RunEvent::WindowEvent {
                event: WindowEvent::Destroyed,
                ..
            } => {
                // Teardown: the webview is gone. Stop ticking and pushing so
                // a late transport emit cannot turn a clean shutdown into a
                // fatal error (WP03 tolerated-teardown rule).
                close_requested = true;
            }
            RunEvent::MainEventsCleared => {
                if close_requested {
                    return;
                }
                let now = Instant::now();
                let elapsed = now.duration_since(last_tick);
                last_tick = now;
                if on_tick(elapsed) {
                    // Port invariant: each interactive frame advances the
                    // injected control-side tick and then requests the
                    // current immutable projection.
                    let projection_for_page = projection();
                    // WP03 projection transport: generation-gated push of
                    // the serde serialization of this projection's embedded
                    // SemanticGraphicalViewModel. A failure while the window
                    // lives is typed: record the first, close once, surface
                    // it after the loop.
                    if let Err(error) = projection_channel.push(&projection_for_page, |document| {
                        handle.emit(PROJECTION_EVENT, document)
                    }) {
                        loop_transport_error
                            .borrow_mut()
                            .get_or_insert(WindowError::from(error));
                        close_requested = true;
                        if let Some(window) = handle.get_webview_window(WINDOW_LABEL) {
                            let _ = window.close();
                        }
                        return;
                    }
                    // WP03 meter transport: latest-value snapshot, coalesced
                    // to the declared 30 Hz pace; a lost frame is display-only
                    // degradation (meter_channel module docs), never fatal.
                    meter_channel.observe(audio_observation());
                    let _ = meter_channel.emit_if_due(now, |frame| handle.emit(METER_EVENT, frame));
                } else {
                    // Port invariant, verbatim: "a false tick result closes
                    // the disposable window only after application control
                    // ownership has retained a terminal outcome: either the
                    // completed live report or a typed fatal runtime error."
                    // The tick callback is that control ownership; on false
                    // this window only closes, mechanically, exactly as the
                    // eframe adapter's request_close_once does.
                    close_requested = true;
                    if let Some(window) = handle.get_webview_window(WINDOW_LABEL) {
                        // The normal owned path:
                        // CloseRequested -> Destroyed -> ExitRequested.
                        let _ = window.close();
                    }
                }
            }
            _ => {}
        });

        stop_waker.store(true, Ordering::Relaxed);
        let _ = waker.join();
        drop(pipeline);
        if let Some(error) = transport_error.borrow_mut().take() {
            return Err(error);
        }
        if exit_code == 0 {
            Ok(())
        } else {
            Err(WindowError::new(format!(
                "tauri event loop exited with code {exit_code}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        page_asset, resolve_page_source, TauriWebviewWindow, PAGE_INDEX_HTML, PAGE_JS,
        PAGE_TOKENS_CSS,
    };
    use crate::shell::webview::WebviewShellError;
    use std::path::Path;

    #[test]
    fn webview_window_default_uses_the_product_name() {
        assert_eq!(TauriWebviewWindow::default().title(), "crest-synth");
    }

    #[test]
    fn page_resolution_serves_the_projection_page_without_an_override() {
        let page = resolve_page_source(None).expect("the built-in page always resolves");
        assert_eq!(page, PAGE_INDEX_HTML);
        // The five declared shell bands as semantic containers, with the
        // generated token table and the split page assets linked in.
        for marker in [
            "data-band=\"contextLine\"",
            "data-band=\"identityHeader\"",
            "data-band=\"workspace\"",
            "data-band=\"inspector\"",
            "data-band=\"footer\"",
            "tokens.css",
            "page.css",
            "page.js",
        ] {
            assert!(page.contains(marker), "index.html must carry {marker}");
        }
        // The page registers no key handler in the document or the render
        // script; keys are captured Rust-side (WP01/WP02 boundary).
        for source in [page.as_str(), PAGE_JS] {
            assert!(
                !source.contains("keydown")
                    && !source.contains("keyup")
                    && !source.contains("keypress"),
                "the page must register no key handler"
            );
        }
    }

    /// Every asset the page references resolves over the protocol with its
    /// declared content type; anything else is a 404, not a fallback page.
    #[test]
    fn the_protocol_serves_each_declared_page_asset_and_nothing_else() {
        let index = resolve_page_source(None).expect("the built-in page always resolves");
        for (path, expected_type) in [
            ("/", "text/html; charset=utf-8"),
            ("/index.html", "text/html; charset=utf-8"),
            ("/tokens.css", "text/css; charset=utf-8"),
            ("/page.css", "text/css; charset=utf-8"),
            ("/page.js", "text/javascript; charset=utf-8"),
            ("/fonts/AzeretMono-Regular.ttf", "font/ttf"),
            ("/fonts/AzeretMono-Medium.ttf", "font/ttf"),
            ("/fonts/AzeretMono-SemiBold.ttf", "font/ttf"),
            ("/fonts/AzeretMono-Bold.ttf", "font/ttf"),
        ] {
            let (content_type, body) =
                page_asset(path, &index).unwrap_or_else(|| panic!("{path} must resolve"));
            assert_eq!(content_type, expected_type, "{path}");
            assert!(!body.is_empty(), "{path} must carry its embedded bytes");
        }
        assert!(page_asset("/unknown.css", &index).is_none());
        assert!(page_asset("/../Cargo.toml", &index).is_none());
    }

    /// The embedded token table is the committed generated file, so the page
    /// styles from the authored vocabulary rather than a drifted copy.
    #[test]
    fn the_embedded_token_table_matches_the_generated_vocabulary() {
        crate::shell::webview::token_export::committed_tokens_are_fresh(PAGE_TOKENS_CSS)
            .expect("the embedded tokens.css must match the authored vocabulary");
    }

    /// The deterministic init-failure trigger: an unloadable override page
    /// is a typed `PageLoadFailed` carrying the path and the io cause,
    /// raised before any window exists.
    #[test]
    fn an_unloadable_override_page_is_a_typed_page_load_failure() {
        let missing = Path::new("/nonexistent/crest-webview-page.html");
        let error = resolve_page_source(Some(missing))
            .expect_err("a missing override page must fail typed");
        match error {
            WebviewShellError::PageLoadFailed { page, source } => {
                assert_eq!(page, "/nonexistent/crest-webview-page.html");
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("expected PageLoadFailed, got {other:?}"),
        }
    }
}
