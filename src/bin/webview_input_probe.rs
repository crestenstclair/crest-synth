//! Disposable WP01 evidence probe: Rust-side key capture under a focused
//! WKWebView plus cpal/production-audio coexistence inside a Tauri v2 process.
//!
//! This binary is evidence, not product code
//! (kitty-specs/webview-shell-foundation-01KZ9DN7, plan IC-01, research R-02).
//! It composes the exact production application the eframe shell uses —
//! production registries, fixture patches, real cpal stream, threaded
//! graph-preparation worker — via `StandaloneApplication::run`, substituting a
//! Tauri window for the eframe window behind the same `AppWindow` port. The
//! probe page registers no key handler: that is the contract under test.
//!
//! Log vocabulary (stdout):
//! - `PROBE_KEY seq=<n> key=<WindowKey> state=<down|up> repeat=<bool>` — one
//!   line per key transition observed Rust-side by
//!   `crest_synth::shell::webview::input_capture` (NSEvent local monitor).
//! - `PROBE_ACTION <SemanticAction>` — the production translator's output for
//!   the transition above, proving the eframe-shared semantic path.
//! - `PROBE_TAO <event>` — every window-level event Tauri surfaces, showing
//!   that no keyboard event ever arrives through `on_window_event`'s
//!   vocabulary (`tauri::WindowEvent` has no keyboard variant).
//! - `PROBE_RT ...` — real-time health counters from the production
//!   `AudioObservationSnapshot`, sampled periodically and at exit.
//!
//! Modes: default keeps the window open for a physical key sweep;
//! `--close-after <secs>` closes through the normal path for automated runs;
//! `--synthetic-sweep` posts in-process synthetic `NSEvent`s through the same
//! queue hardware events use (fallback evidence when no injection permission
//! exists; synthetic events skip window-server routing, so they prove the
//! monitor path, not focus routing).

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("the webview input probe is macOS-only evidence (WKWebView)");
    std::process::exit(1);
}

#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    probe::run()
}

#[cfg(target_os = "macos")]
mod probe {
    use anyhow::{bail, Context, Result};
    use crest_synth::adapter::atomic_audio_observation::AtomicAudioObservation;
    use crest_synth::adapter::braids_capability::BraidsCapability;
    use crest_synth::adapter::braids_preparer::BraidsPreparer;
    use crest_synth::adapter::corridors_midi_event_source::CorridorsMidiEventSource;
    use crest_synth::adapter::cpal_audio_output::CpalAudioOutput;
    use crest_synth::adapter::hidef_soundfont_asset::HiDefSoundFontAsset;
    use crest_synth::adapter::hidef_soundfont_capability::HiDefSoundFontCapability;
    use crest_synth::adapter::hidef_soundfont_preparer::HiDefSoundFontPreparer;
    use crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
    use crest_synth::adapter::lock_free_structural_graph_boundary::LockFreeStructuralGraphBoundary;
    use crest_synth::adapter::production_effects::{
        production_effect_preparers, production_effect_providers,
    };
    use crest_synth::mixer::mixer_state::MixerState;
    use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
    use crest_synth::real_time::{AudioObservationSnapshot, GraphHandoffStatus, GraphRevision};
    use crest_synth::shell::app_window::{
        AppInputCallback, AppWindow, AudioObservationCallback, FrameObservationCallback,
        ProjectionCallback, TickCallback, WindowError,
    };
    use crest_synth::shell::standalone_application::{ApplicationConfig, StandaloneApplication};
    use crest_synth::shell::webview::input_capture;
    use crest_synth::shell::window_input::{WindowInput, WindowKey};
    use crest_synth::shell::KeyboardInputTranslator;
    use crest_synth::synth::{InstrumentCapabilityProvider, InstrumentPreparer};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};

    const AUDIO_COMMAND_CAPACITY: usize = 1_024;
    const RT_LOG_INTERVAL: Duration = Duration::from_secs(2);
    const SYNTHETIC_SWEEP_DELAY: Duration = Duration::from_secs(2);
    const WAKE_INTERVAL: Duration = Duration::from_millis(16);

    /// The probe page: instructions and a focused `<input>` so the WKWebView
    /// both has focus and demonstrably consumes typing. Deliberately no
    /// `keydown`/`keyup`/`keypress` handler anywhere — the crest-spec adapter
    /// rule under test.
    const PAGE_HTML: &str = r#"<!doctype html>
<html>
<head><meta charset="utf-8"><title>crest-synth input probe</title>
<style>
  body { font-family: monospace; background: #101418; color: #d8e0e8; margin: 2rem; }
  input { font: inherit; width: 100%; padding: .5rem; background: #1a2027; color: inherit; border: 1px solid #3a4450; }
</style></head>
<body>
  <h1>crest-synth webview input probe</h1>
  <p>The Rust side is listening. Focus the field below, then press the MIXER
  vocabulary: 1 2 W S A D Q E K (hold K while pressing W/S/A/D), plus
  Shift chords. This page registers no key handler.</p>
  <input id="probe-input" autofocus placeholder="focus here and type">
</body>
</html>
"#;

    struct Options {
        close_after: Option<Duration>,
        synthetic_sweep: bool,
    }

    fn parse_options() -> Result<Options> {
        let mut close_after = None;
        let mut synthetic_sweep = false;
        let mut arguments = std::env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--close-after" => {
                    let seconds: u64 = arguments
                        .next()
                        .context("--close-after requires a whole number of seconds")?
                        .parse()
                        .context("--close-after requires a whole number of seconds")?;
                    close_after = Some(Duration::from_secs(seconds));
                }
                "--synthetic-sweep" => synthetic_sweep = true,
                unsupported => bail!("unsupported probe option {unsupported}"),
            }
        }
        Ok(Options {
            close_after,
            synthetic_sweep,
        })
    }

    pub fn run() -> Result<()> {
        let options = parse_options()?;
        let config = ApplicationConfig::default();
        let initial_parameters =
            ParameterSnapshot::new(0, config.global_parameters(), MixerState::default(), &[])
                .context("failed to construct the initial audio parameter snapshot")?;
        let boundary = LockFreeAudioBoundary::new(AUDIO_COMMAND_CAPACITY, initial_parameters);
        let soundfont_asset = HiDefSoundFontAsset::load()
            .context("failed to load the fixed HiDef SoundFont asset")?;
        let providers: Vec<Box<dyn InstrumentCapabilityProvider>> = vec![
            Box::new(
                HiDefSoundFontCapability::new(soundfont_asset.catalog())
                    .context("failed to construct the HiDef capability provider")?,
            ),
            Box::new(
                BraidsCapability::new()
                    .context("failed to construct the Braids capability provider")?,
            ),
        ];
        let preparers: Vec<Box<dyn InstrumentPreparer>> = vec![
            Box::new(
                HiDefSoundFontPreparer::new(&soundfont_asset)
                    .context("failed to prepare the HiDef instrument factory")?,
            ),
            Box::new(
                BraidsPreparer::new().context("failed to prepare the Braids instrument factory")?,
            ),
        ];
        let effect_providers = production_effect_providers()
            .context("failed to construct the production effect capability providers")?;
        let effect_preparers = production_effect_preparers()
            .context("failed to construct the production effect preparers")?;
        let structural = LockFreeStructuralGraphBoundary::new(
            1,
            1,
            GraphHandoffStatus::with_active(GraphRevision::INITIAL),
        )
        .context("failed to prepare the structural graph boundary")?;

        let application = StandaloneApplication::new_with_effects(
            boundary,
            providers,
            preparers,
            effect_providers,
            effect_preparers,
            structural,
            AtomicAudioObservation::default(),
            CorridorsMidiEventSource::new(),
            TauriProbeWindow {
                close_after: options.close_after,
                synthetic_sweep: options.synthetic_sweep,
            },
            CpalAudioOutput::new(),
            config,
        )
        .context("failed to validate the production composition")?;

        println!("PROBE_START production composition validated; starting audio then window");
        application
            .run()
            .context("probe standalone execution failed")?;
        println!("PROBE_EXIT clean: stream released before worker collection, no runtime error");
        Ok(())
    }

    /// Feeds captured raw keys through the production translator into the
    /// application input callback, logging both layers.
    struct KeyPipeline {
        translator: KeyboardInputTranslator,
        on_input: AppInputCallback,
        sequence: u64,
    }

    impl KeyPipeline {
        fn feed(&mut self, input: WindowInput) {
            if let Some(action) = self.translator.translate(input) {
                println!("PROBE_ACTION {action:?}");
                (self.on_input)(action);
            }
        }
    }

    struct TauriProbeWindow {
        close_after: Option<Duration>,
        synthetic_sweep: bool,
    }

    impl AppWindow for TauriProbeWindow {
        fn run(
            &self,
            on_input: AppInputCallback,
            _projection: ProjectionCallback,
            audio_observation: AudioObservationCallback,
            mut on_tick: TickCallback,
            _on_frame: FrameObservationCallback,
        ) -> Result<(), WindowError> {
            let pipeline = Rc::new(RefCell::new(KeyPipeline {
                translator: KeyboardInputTranslator::new(),
                on_input,
                sequence: 0,
            }));

            // T003 path under test: the NSEvent local monitor, installed from
            // the Rust side on the main thread before the event loop starts.
            let capture_pipeline = Rc::clone(&pipeline);
            let _capture = input_capture::install(move |raw| {
                let mut pipeline = capture_pipeline.borrow_mut();
                pipeline.sequence += 1;
                println!(
                    "PROBE_KEY seq={} key={:?} state={} repeat={}",
                    pipeline.sequence,
                    raw.key(),
                    if raw.pressed() { "down" } else { "up" },
                    raw.repeat(),
                );
                let input = if raw.pressed() {
                    WindowInput::key_down(raw.key())
                } else {
                    WindowInput::key_up(raw.key())
                };
                pipeline.feed(input);
            })
            .map_err(|error| WindowError::new(format!("input capture install failed: {error}")))?;

            let app = tauri::Builder::default()
                .register_uri_scheme_protocol("probe", |_context, _request| {
                    tauri::http::Response::builder()
                        .header("Content-Type", "text/html; charset=utf-8")
                        .body(PAGE_HTML.as_bytes().to_vec())
                        .expect("static probe page response is well-formed")
                })
                .setup(|app| {
                    let url: tauri::Url = "probe://localhost/index.html"
                        .parse()
                        .expect("static probe url is well-formed");
                    let window =
                        WebviewWindowBuilder::new(app, "probe", WebviewUrl::CustomProtocol(url))
                            .title("crest-synth webview input probe")
                            .inner_size(760.0, 520.0)
                            .focused(true)
                            .build()?;
                    let _ = window.set_focus();
                    Ok(())
                })
                .build(tauri::generate_context!())
                .map_err(|error| WindowError::new(format!("tauri build failed: {error}")))?;

            // The tao loop waits when idle; a detached waker keeps control-side
            // ticks flowing (fixture MIDI, structural advance, device status)
            // at roughly frame rate, mirroring eframe's repaint-driven ticks.
            let stop_waker = Arc::new(AtomicBool::new(false));
            let waker_stop = Arc::clone(&stop_waker);
            let waker_handle = app.handle().clone();
            let waker = std::thread::spawn(move || {
                while !waker_stop.load(Ordering::Relaxed) {
                    std::thread::sleep(WAKE_INTERVAL);
                    if waker_handle.run_on_main_thread(|| {}).is_err() {
                        break;
                    }
                }
            });

            let close_after = self.close_after;
            let synthetic_sweep = self.synthetic_sweep;
            let loop_pipeline = Rc::clone(&pipeline);
            let started = Instant::now();
            let mut last_tick = started;
            let mut last_rt_log: Option<Instant> = None;
            let mut sweep_done = !synthetic_sweep;
            let mut close_requested = false;
            let mut tick_failed = false;

            let exit_code = app.run_return(move |handle, event| match event {
                RunEvent::Ready => {
                    log_rt_counters("ready", started, &audio_observation());
                    last_rt_log = Some(Instant::now());
                }
                RunEvent::WindowEvent { label, event, .. } => {
                    println!("PROBE_TAO window={label} event={event:?}");
                    if matches!(event, WindowEvent::Focused(false)) {
                        println!("PROBE_FOCUS lost");
                        loop_pipeline.borrow_mut().feed(WindowInput::focus_lost());
                    }
                }
                RunEvent::MainEventsCleared => {
                    let now = Instant::now();
                    let elapsed = now.duration_since(last_tick);
                    last_tick = now;
                    if !on_tick(elapsed) && !tick_failed {
                        tick_failed = true;
                        println!("PROBE_RT tick=false (control runtime recorded a failure)");
                        request_close(handle, &mut close_requested);
                    }
                    if last_rt_log.is_none_or(|last| now.duration_since(last) >= RT_LOG_INTERVAL) {
                        log_rt_counters("sample", started, &audio_observation());
                        last_rt_log = Some(now);
                    }
                    if !sweep_done && now.duration_since(started) >= SYNTHETIC_SWEEP_DELAY {
                        sweep_done = true;
                        post_synthetic_sweep(handle);
                    }
                    if let Some(limit) = close_after {
                        if now.duration_since(started) >= limit && !close_requested {
                            log_rt_counters("closing", started, &audio_observation());
                            request_close(handle, &mut close_requested);
                        }
                    }
                }
                RunEvent::Exit => {
                    log_rt_counters("exit", started, &audio_observation());
                }
                _ => {}
            });

            stop_waker.store(true, Ordering::Relaxed);
            let _ = waker.join();
            drop(pipeline);
            if exit_code == 0 {
                Ok(())
            } else {
                Err(WindowError::new(format!(
                    "tauri event loop exited with code {exit_code}"
                )))
            }
        }
    }

    fn request_close(handle: &tauri::AppHandle, close_requested: &mut bool) {
        if *close_requested {
            return;
        }
        *close_requested = true;
        if let Some(window) = handle.get_webview_window("probe") {
            // The normal path: CloseRequested -> Destroyed -> ExitRequested.
            let _ = window.close();
        }
    }

    fn log_rt_counters(stage: &str, started: Instant, snapshot: &AudioObservationSnapshot) {
        println!(
            "PROBE_RT stage={stage} t_ms={} sequence={} blocks={} frames={} commands={} \
             active_notes={} primary_rms={:.6} output_rms={:.6} non_finite={} clipped={} \
             routing_failures={}",
            started.elapsed().as_millis(),
            snapshot.sequence(),
            snapshot.rendered_blocks(),
            snapshot.rendered_frames(),
            snapshot.commands_consumed(),
            snapshot.active_notes(),
            snapshot.primary_patch_rms(),
            snapshot.output_rms(),
            snapshot.non_finite_samples(),
            snapshot.clipped_samples(),
            snapshot.routing_failures(),
        );
    }

    /// Fallback evidence: posts synthetic key events into this process's own
    /// AppKit event queue (`NSApplication postEvent:atStart:`), the same queue
    /// hardware key events are dequeued from, so they cross the local-monitor
    /// intercept exactly as physical keys do. They do NOT traverse the window
    /// server or first-responder focus routing — which is why the physical
    /// sweep remains the primary evidence and this is the fallback.
    fn post_synthetic_sweep(handle: &tauri::AppHandle) {
        use objc2::MainThreadMarker;
        use objc2_app_kit::NSApplication;

        let Some(main_thread) = MainThreadMarker::new() else {
            println!("PROBE_SWEEP skipped: not on the main thread");
            return;
        };
        let window_number = handle
            .get_webview_window("probe")
            .and_then(|window| window.ns_window().ok())
            .map_or(0, |pointer| {
                // SAFETY: tauri hands back the live NSWindow for this window.
                let window: &objc2_app_kit::NSWindow =
                    unsafe { &*pointer.cast::<objc2_app_kit::NSWindow>() };
                window.windowNumber()
            });
        println!("PROBE_SWEEP posting synthetic NSEvents (window_number={window_number})");
        let application = NSApplication::sharedApplication(main_thread);

        // The MIXER vocabulary with distinct press/release, the K Edit-hold
        // chord, a Shift chord, and one unbound key (G).
        let taps: &[(u16, &str, WindowKey)] = &[
            (18, "1", WindowKey::Digit1),
            (19, "2", WindowKey::Digit2),
            (13, "w", WindowKey::W),
            (1, "s", WindowKey::S),
            (0, "a", WindowKey::A),
            (2, "d", WindowKey::D),
            (12, "q", WindowKey::Q),
            (14, "e", WindowKey::E),
        ];
        for (code, characters, _) in taps {
            post_key(&application, window_number, *code, characters, true, false);
            post_key(&application, window_number, *code, characters, false, false);
        }
        // Edit-hold: K down, W tapped (Adjust while held), K up.
        post_key(&application, window_number, 40, "k", true, false);
        post_key(&application, window_number, 13, "w", true, false);
        post_key(&application, window_number, 13, "w", false, false);
        post_key(&application, window_number, 40, "k", false, false);
        // Shift chord: Shift+W identifies the same physical key.
        post_key(&application, window_number, 13, "W", true, true);
        post_key(&application, window_number, 13, "W", false, true);
        // Unbound key normalizes to Other and binds to nothing.
        post_key(&application, window_number, 5, "g", true, false);
        post_key(&application, window_number, 5, "g", false, false);
    }

    #[allow(non_snake_case)]
    fn post_key(
        application: &objc2_app_kit::NSApplication,
        window_number: isize,
        key_code: u16,
        characters: &str,
        pressed: bool,
        shift: bool,
    ) {
        use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
        use objc2_foundation::{NSPoint, NSString};

        let event_type = if pressed {
            NSEventType::KeyDown
        } else {
            NSEventType::KeyUp
        };
        let flags = if shift {
            NSEventModifierFlags::Shift
        } else {
            NSEventModifierFlags::empty()
        };
        let characters = NSString::from_str(characters);
        let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            event_type,
            NSPoint::new(0.0, 0.0),
            flags,
            0.0,
            window_number,
            None,
            &characters,
            &characters,
            false,
            key_code,
        );
        match event {
            Some(event) => application.postEvent_atStart(&event, false),
            None => println!("PROBE_SWEEP failed to construct key event code={key_code}"),
        }
    }
}
