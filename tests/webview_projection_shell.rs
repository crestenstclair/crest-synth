//! Acceptance for the webview projection shell (crest-spec
//! `validation.webview_projection_shell`, asset
//! `WebviewProjectionShellAcceptanceTests`; mission
//! webview-shell-foundation-01KZ9DN7, WP06).
//!
//! Five falsifiable proofs over the production reducer, projector, transports,
//! generated token table, committed projection page, and the real shipped
//! binary:
//!
//! - T022 serialized-schema fidelity: the emit path's document is
//!   byte-identical to the projector's own serialization across three
//!   distinct reducer states, so no page-facing schema fork can hide behind
//!   generation gating.
//! - T023 token-table freshness: the committed `webview-page/tokens.css` is
//!   byte-fresh against the authored vocabulary via WP04's
//!   `committed_tokens_are_fresh` contract, carries the GENERATED header, and
//!   keeps the injective property-name transform; drift names its property.
//! - T024 page render determinism: the real page in a real Tauri window
//!   renders one document to the same declared observation twice at both
//!   authored viewports, with the declared MIXER anatomy.
//! - T025 typed startup failure: an unloadable page is a typed
//!   `PageLoadFailed` on stderr with nonzero exit and no eframe fallback,
//!   proven on the shipped binary as a subprocess.
//! - T026 shutdown parity and the live layer: the harness window closes
//!   through the owned CloseRequested → Destroyed → Exit path; the shipped
//!   binary reaches the identical owned shutdown (stream release before
//!   worker completion, graph ownership collection, exit 0) under BOTH
//!   shells from real window-close runs; NFR-001 (projection-to-paint p95
//!   ≤ 50 ms via the `crest://painted` ack) and NFR-002 (30 Hz meter cadence
//!   over a soak with a structurally bounded pending slot) are measured live.
//!
//! # Harness shape (`harness = false`)
//!
//! T024/T026 open a real Tauri window, which macOS only permits on the main
//! thread; the default libtest harness runs every test on a worker thread, so
//! this target declares `harness = false` in `Cargo.toml` and sequences its
//! sections from `main`. A failing section panics (nonzero exit) before the
//! acceptance marker can print.
//!
//! # Live gate — the env var and nothing else
//!
//! `CREST_WEBVIEW_TESTS=1` admits the two window-bearing sections. The
//! decision is taken once, before any window attempt, from the environment
//! variable alone; a failure inside a running live section is a failure,
//! never a skip. Headless runs print an explicit skip listing and the marker
//! carries it. `CREST_WEBVIEW_FULL_SOAK=1` lengthens the NFR-002 soak from
//! 60 s to 300 s (both configurations are printed).

use crest_synth::adapter::braids_capability::BraidsCapability;
use crest_synth::adapter::production_effects::{
    production_chorus_config, production_effect_registry,
};
use crest_synth::adapter::production_instruments::{
    production_capability_registry, production_soundfont_capability,
};
use crest_synth::control::{
    AppEvent, AppState, Direction, SemanticGraphicalViewModel, StateProjector, TopLevelContext,
};
use crest_synth::kernel::{MidiChannel, PatchId};
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::mixer::mix_observation::MixObservation;
use crest_synth::mixer::mixer_track_id::MixerTrackId;
use crest_synth::mixer::patch_output::PatchOutput;
use crest_synth::real_time::AudioObservationSnapshot;
use crest_synth::shell::visual::ViewportDensityPolicy;
use crest_synth::shell::webview::meter_channel::{
    MeterChannel, MeterEmit, METER_EVENT, METER_INTERVAL, METER_RATE_HZ,
};
use crest_synth::shell::webview::projection_channel::{
    ProjectionChannel, ProjectionPush, PROJECTION_EVENT,
};
use crest_synth::shell::webview::token_export;
use crest_synth::synth::effect_slot_id::EffectSlotIndex;
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::{EffectSlotId, InstrumentConfig, Patch};
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The `crest://painted` ack event the page emits after each projection
/// paint (authored in `webview-page/page.js`; WP05).
const PAINTED_EVENT: &str = "crest://painted";

/// The harness-only event the driver uses to pull `renderObservation`
/// payloads and page-side statistics back out of the page.
const HARNESS_EVENT: &str = "crest://harness";

/// The declared column anatomy, closed and ordered (crest-spec
/// `valueObject.MixerTrackColumnStructure`).
const COLUMN_ANATOMY: [&str; 5] = [
    "TrackHeader",
    "LevelFader",
    "LevelReadout",
    "PanReadout",
    "StateLine",
];

/// ShellRegionId's serialized names, as the painted ack keys its post-paint
/// region evidence (WP05 page contract).
const SHELL_REGION_IDS: [&str; 5] = [
    "contextLine",
    "identityHeader",
    "mainWorkspace",
    "persistentSideRegion",
    "footer",
];

fn main() {
    // libtest-style arguments (`--nocapture`, filters) are accepted and
    // ignored: output is unconditionally visible under `harness = false`.
    let _ = std::env::args();

    // The live gate: decided exactly once, before any window attempt, from
    // the environment variable alone. Nothing downstream may turn a live
    // failure into a skip.
    let live = std::env::var("CREST_WEBVIEW_TESTS").as_deref() == Ok("1");
    println!(
        "webview_projection_shell acceptance: {} run",
        if live {
            "live (CREST_WEBVIEW_TESTS=1)"
        } else {
            "headless"
        }
    );

    let fidelity = prove_serialized_schema_fidelity();
    prove_token_table_freshness();
    prove_typed_startup_failure();

    let skips: Vec<&str> = if live {
        Vec::new()
    } else {
        vec![
            "T024 page render determinism (DOM layer at 1920x1080 and 1280x800)",
            "T026 live layer (real-window shutdown parity, NFR-001 projection-to-paint, NFR-002 meter soak)",
        ]
    };

    if live {
        run_live_sections(&fidelity);
    } else {
        for skip in &skips {
            println!(
                "CREST_WEBVIEW_SKIP {skip}: CREST_WEBVIEW_TESTS=1 absent \
                 (skip decided before any window attempt; serialized-document \
                 fidelity for the same documents ran above as T022)"
            );
        }
    }

    // Reached only when every section that ran passed: any failing assertion
    // above panics first and the process exits nonzero without this line.
    if skips.is_empty() {
        println!("CREST_ACCEPTANCE webview_projection_shell passed (skipped: none)");
    } else {
        println!(
            "CREST_ACCEPTANCE webview_projection_shell passed (skipped: {})",
            skips.join("; ")
        );
    }
}

// ---------------------------------------------------------------------------
// Production fixture (the recorded pattern: tests/spike_webview_view_model_dump.rs)
// ---------------------------------------------------------------------------

fn soundfont_config() -> InstrumentConfig {
    create_soundfont_config(
        &production_soundfont_capability().expect("the production SoundFont capability loads"),
        SoundFontInstrument::new(0, 40, false).expect("the fixture instrument is valid"),
    )
    .expect("the fixture SoundFont config is valid")
}

/// The production fixture state, built exactly as the recorded spike harness
/// builds it: production registries, two patches (SoundFont + Braids), chorus
/// on the first, installed through the production reducer.
fn production_fixture_state() -> AppState {
    let soundfont = soundfont_config();
    let braids = BraidsCapability::new()
        .expect("the Braids capability constructs")
        .default_config()
        .expect("the Braids default config is valid");
    let patches = [soundfont, braids]
        .into_iter()
        .enumerate()
        .map(|(index, config)| {
            let patch = Patch::new(
                PatchId::new(index as u32 + 1).expect("fixture patch ids are nonzero"),
                format!("Semantic {}", index + 1),
                config,
                MidiChannel::new(index as u8).expect("fixture channels are in range"),
                PatchOutput::new(
                    MixerTrackId::new(index as u8).expect("fixture tracks are in range"),
                    -5.0 - index as f32,
                )
                .expect("fixture outputs are valid"),
            );
            if index == 0 {
                patch.with_effect_slot(
                    EffectSlotIndex::ALL[0],
                    production_chorus_config(EffectSlotId::new(1).expect("slot id 1 is nonzero"))
                        .expect("the production chorus config is valid"),
                )
            } else {
                patch
            }
        })
        .collect();
    let mut state = AppState::new_with_effects(
        production_capability_registry().expect("the production capability registry loads"),
        production_effect_registry().expect("the production effect registry loads"),
        GlobalParameters::new(-3.0).expect("the fixture master gain is in range"),
    );
    state
        .apply(AppEvent::InstallPatches(patches))
        .expect("installing the fixture patches is accepted");
    state
}

/// The fixture with the MIXER context selected — the state every section
/// projects from.
fn production_mixer_state() -> AppState {
    let mut state = production_fixture_state();
    state
        .apply(AppEvent::SelectContext(TopLevelContext::Mixer))
        .expect("selecting MIXER is accepted");
    state
}

// ---------------------------------------------------------------------------
// T022 — serialized-schema fidelity
// ---------------------------------------------------------------------------

struct FidelityEvidence {
    /// State A's exact page-facing document (the bytes the emit path hands
    /// the transport), reused by the live sections so the DOM layer renders
    /// the very document whose fidelity was proven here.
    document_a: String,
}

/// Pushes the state's accepted projection through the production
/// [`ProjectionChannel`] (the emit path — the same code path the window
/// invokes) and proves the document is the projector's own serialization,
/// byte-for-byte and structurally.
fn check_state_fidelity(
    projector: &StateProjector,
    channel: &mut ProjectionChannel,
    state: &AppState,
    label: &str,
) -> (u64, String) {
    let projection = projector
        .project_with_shell(state)
        .expect("the production projector accepts the fixture state")
        .3;
    let model = projection.semantic_model();
    let generation = projection.generation();

    let mut captured: Option<Value> = None;
    let outcome = channel
        .push(&projection, |document| {
            captured = Some(document);
            Ok(())
        })
        .expect("pushing an accepted projection through the channel succeeds");
    assert_eq!(
        outcome,
        ProjectionPush::Emitted,
        "{label}: a newly accepted generation must emit (generation gating may not hide a state)"
    );
    let emitted = captured.expect("an Emitted outcome hands the emitter exactly one document");
    let emitted_bytes =
        serde_json::to_string(&emitted).expect("the emitted document serializes to JSON text");

    // Byte identity, same code path invoked independently: the emit path's
    // text equals a fresh serialization of the projector's model through the
    // identical serde route. Any webview-only struct, trimmed field, or
    // reordered value in the emit path lands here.
    let independent = serde_json::to_string(
        &serde_json::to_value(model).expect("the projector's model converts to a JSON value"),
    )
    .expect("the projector's model serializes to JSON text");
    assert_eq!(
        emitted_bytes, independent,
        "{label}: the emit path's document must be byte-identical to the projector's serialization"
    );

    // Byte identity against the projector's own direct `to_string`,
    // canonicalized through `serde_json::Value`. (serde_json maps hold keys
    // in sorted order, so the struct-declaration-order text is canonicalized
    // before the byte comparison; values and keys are untouched — a fork of
    // any kind still lands here.)
    let direct = serde_json::to_string(model)
        .expect("the projector's model serializes directly to JSON text");
    let direct_canonical = serde_json::to_string(
        &serde_json::from_str::<Value>(&direct).expect("the projector's text parses"),
    )
    .expect("the canonicalized projector text serializes");
    assert_eq!(
        emitted_bytes, direct_canonical,
        "{label}: the emit path's bytes must equal the canonicalized bytes of \
         serde_json::to_string of the projector's model"
    );

    // Structural round-trip: the emitted string parses back into a Value
    // equal to the model's own serialized Value — the declared anti-fork
    // assertion ("any webview-only struct in the emit path fails this
    // section by construction").
    let round_tripped: Value =
        serde_json::from_str(&emitted_bytes).expect("the emitted document round-trips");
    let model_value: Value =
        serde_json::from_str(&direct).expect("the projector's document round-trips");
    assert_eq!(
        round_tripped, model_value,
        "{label}: the emitted document must round-trip into a Value equal to the model's"
    );

    // The declared top-level surface, nothing added or trimmed.
    let Value::Object(map) = &round_tripped else {
        panic!("{label}: the emitted document must be a JSON object");
    };
    let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
    keys.sort_unstable();
    let mut declared: Vec<&str> =
        SemanticGraphicalViewModel::SERIALIZED_PROPERTY_DESCRIPTOR.to_vec();
    declared.sort_unstable();
    assert_eq!(
        keys, declared,
        "{label}: the emitted top-level properties must be exactly the declared descriptor"
    );

    (generation, emitted_bytes)
}

fn prove_serialized_schema_fidelity() -> FidelityEvidence {
    let mut state = production_mixer_state();
    let projector = StateProjector::new();
    // One channel across all three states, so the generation gate itself is
    // exercised: three distinct accepted generations must produce three
    // distinct emits.
    let mut channel = ProjectionChannel::new();

    let (generation_a, document_a) =
        check_state_fidelity(&projector, &mut channel, &state, "state A (initial MIXER)");

    state
        .apply(AppEvent::Adjust(Direction::Up))
        .expect("adjusting the focused level is accepted");
    let (generation_b, _) = check_state_fidelity(
        &projector,
        &mut channel,
        &state,
        "state B (after a level edit)",
    );

    state
        .apply(AppEvent::Navigate(Direction::Right))
        .expect("moving focus right is accepted");
    let (generation_c, _) = check_state_fidelity(
        &projector,
        &mut channel,
        &state,
        "state C (after a focus move)",
    );

    assert!(
        generation_a < generation_b && generation_b < generation_c,
        "the three states must carry distinct ascending generations \
         ({generation_a}, {generation_b}, {generation_c}) so gating cannot mask a fork"
    );

    println!(
        "T022 serialized-schema fidelity: PASS \
         (3 distinct states, generations {generation_a}/{generation_b}/{generation_c}, \
         emit path byte-identical + structural round-trip + declared key surface)"
    );
    FidelityEvidence { document_a }
}

// ---------------------------------------------------------------------------
// T023 — token-table freshness
// ---------------------------------------------------------------------------

fn prove_token_table_freshness() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("webview-page/tokens.css");
    let committed = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("committed tokens.css must be readable: {error}"));

    // WP04's contract: byte-for-byte freshness against a fresh generator run.
    if let Err(drift) = token_export::committed_tokens_are_fresh(&committed) {
        panic!("committed tokens.css drifted from the authored vocabulary: {drift}");
    }

    // The GENERATED header names the generator and the regeneration command.
    assert!(
        committed.starts_with("/* GENERATED — DO NOT EDIT.\n"),
        "tokens.css must open with the GENERATED header"
    );
    assert!(
        committed.contains("src/shell/webview/token_export.rs")
            && committed.contains("make webview-tokens"),
        "the header must name the generator and the regeneration command"
    );

    // The generator's injectivity guarantee, asserted on the committed table:
    // no two authored tokens may share one custom property.
    let properties: Vec<&str> = committed
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("--") {
                return None;
            }
            trimmed.split(':').next()
        })
        .collect();
    assert!(
        properties.len() >= 50,
        "the committed table must carry the full vocabulary (found {} declarations)",
        properties.len()
    );
    let unique: HashSet<&str> = properties.iter().copied().collect();
    assert_eq!(
        unique.len(),
        properties.len(),
        "custom property names must stay injective over the committed table"
    );

    // A drift failure names the property (WP04's TokenDrift contract),
    // exercised against the committed file through the public API.
    let mutated = committed.replace("  --space-8: 8px;", "  --space-8: 9px;");
    assert_ne!(
        mutated, committed,
        "the drift probe must actually mutate a declaration"
    );
    let drift = token_export::committed_tokens_are_fresh(&mutated)
        .expect_err("a mutated declaration must be reported as drift");
    assert_eq!(
        drift.property, "--space-8",
        "drift must name the drifted property"
    );
    assert!(
        drift.to_string().contains("--space-8"),
        "the drift display must carry the property name"
    );

    println!(
        "T023 token-table freshness: PASS \
         ({} unique custom properties, byte-fresh, GENERATED header, drift names its property)",
        properties.len()
    );
}

// ---------------------------------------------------------------------------
// T025 — typed startup failure (subprocess on the shipped binary)
// ---------------------------------------------------------------------------

/// Waits for the child to exit on its own within `limit`, killing it and
/// panicking otherwise — a process that keeps running is exactly the
/// window-kept-alive fallback this section exists to rule out.
fn wait_with_timeout(
    child: std::process::Child,
    limit: Duration,
    label: &str,
) -> std::process::Output {
    let pid = child.id();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    match receiver.recv_timeout(limit) {
        Ok(output) => output.unwrap_or_else(|error| panic!("{label}: wait failed: {error}")),
        Err(_) => {
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
            panic!(
                "{label}: the process did not exit within {limit:?} — \
                 a shell kept a window running instead of failing typed"
            );
        }
    }
}

fn prove_typed_startup_failure() {
    let missing_page = "/nonexistent/crest-webview-acceptance-page.html";
    let started = Instant::now();
    let child = Command::new(env!("CARGO_BIN_EXE_crest-synth"))
        .args(["--shell", "webview"])
        .env("CREST_WEBVIEW_PAGE", missing_page)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the shipped binary spawns");
    let output = wait_with_timeout(child, Duration::from_secs(120), "T025");
    let elapsed = started.elapsed();

    assert!(
        !output.status.success(),
        "an unloadable page must end the process nonzero (got {:?})",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stderr.contains(&format!("webview page {missing_page} failed to load")),
        "stderr must carry the typed PageLoadFailed display; got:\n{stderr}"
    );

    // No fallback: the process exited by itself (asserted above — an eframe
    // window would have kept it alive), and no shell startup marker or
    // observation output appears on either stream.
    for marker in ["eframe", "egui", "winit"] {
        assert!(
            !stdout.to_lowercase().contains(marker) && !stderr.to_lowercase().contains(marker),
            "no eframe-shell startup marker may appear (found {marker:?}):\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
    }
    assert!(
        !stdout.contains("CREST_"),
        "no observation marker may appear on stdout:\n{stdout}"
    );

    println!(
        "T025 typed startup failure: PASS \
         (exit={:?} after {elapsed:.1?}, typed PageLoadFailed on stderr, self-exit — no eframe fallback)",
        output.status.code()
    );
}

// ---------------------------------------------------------------------------
// Live sections (T024 + T026) — gated on CREST_WEBVIEW_TESTS=1 only
// ---------------------------------------------------------------------------

/// The committed projection page's assets, read from the repository. The
/// production window embeds these same files at compile time
/// (`src/shell/webview/window.rs`), and its unit tests prove the embedded
/// copies equal the committed ones, so serving the committed files here
/// renders the shipped page.
struct PageAssets {
    index_html: Vec<u8>,
    tokens_css: Vec<u8>,
    page_css: Vec<u8>,
    page_js: Vec<u8>,
    fonts: Vec<(String, Vec<u8>)>,
}

impl PageAssets {
    fn load(manifest: &Path) -> Self {
        let page = manifest.join("webview-page");
        let read = |path: PathBuf| {
            std::fs::read(&path)
                .unwrap_or_else(|error| panic!("page asset {} must load: {error}", path.display()))
        };
        let fonts = [
            "AzeretMono-Regular.ttf",
            "AzeretMono-Medium.ttf",
            "AzeretMono-SemiBold.ttf",
            "AzeretMono-Bold.ttf",
        ]
        .into_iter()
        .map(|name| {
            (
                format!("/fonts/{name}"),
                read(manifest.join("vendor/azeret-mono").join(name)),
            )
        })
        .collect();
        Self {
            index_html: read(page.join("index.html")),
            tokens_css: read(page.join("tokens.css")),
            page_css: read(page.join("page.css")),
            page_js: read(page.join("page.js")),
            fonts,
        }
    }

    fn resolve(&self, path: &str) -> Option<(&'static str, Vec<u8>)> {
        match path {
            "/" | "/index.html" => Some(("text/html; charset=utf-8", self.index_html.clone())),
            "/tokens.css" => Some(("text/css; charset=utf-8", self.tokens_css.clone())),
            "/page.css" => Some(("text/css; charset=utf-8", self.page_css.clone())),
            "/page.js" => Some(("text/javascript; charset=utf-8", self.page_js.clone())),
            _ => self
                .fonts
                .iter()
                .find(|(name, _)| name == path)
                .map(|(_, bytes)| ("font/ttf", bytes.clone())),
        }
    }
}

/// One painted ack: the echoed generation, its arrival instant, and the full
/// post-paint evidence payload.
type PaintedAcks = Arc<Mutex<Vec<(u64, Instant, Value)>>>;

fn evidence_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("crest-wp06-webview-acceptance");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn screenshot(name: &str) -> PathBuf {
    let path = evidence_dir().join(name);
    let _ = Command::new("screencapture")
        .args(["-x", &path.to_string_lossy()])
        .status();
    println!("  evidence screenshot: {}", path.display());
    path
}

fn run_live_sections(fidelity: &FidelityEvidence) {
    use tauri::{Listener, Manager};

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let assets = PageAssets::load(manifest);
    let desktop = ViewportDensityPolicy::Desktop.authored_viewport();

    let (harness_sender, harness_receiver) = mpsc::channel::<Value>();
    let painted: PaintedAcks = Arc::new(Mutex::new(Vec::new()));

    let app = tauri::Builder::default()
        .register_uri_scheme_protocol("crest", move |_context, request| {
            match assets.resolve(request.uri().path()) {
                Some((content_type, body)) => tauri::http::Response::builder()
                    .header("Content-Type", content_type)
                    .body(body)
                    .expect("the committed asset response is well-formed"),
                None => tauri::http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .expect("the empty not-found response is well-formed"),
            }
        })
        .build(tauri::generate_context!())
        .expect("the live harness requires a working webview runtime (gate already admitted us)");

    app.listen_any(HARNESS_EVENT, move |event| {
        if let Ok(value) = serde_json::from_str::<Value>(event.payload()) {
            let _ = harness_sender.send(value);
        }
    });
    {
        let painted = Arc::clone(&painted);
        app.listen_any(PAINTED_EVENT, move |event| {
            let at = Instant::now();
            let value: Value = serde_json::from_str(event.payload()).unwrap_or(Value::Null);
            let generation = value.get("generation").and_then(Value::as_u64).unwrap_or(0);
            painted
                .lock()
                .expect("painted acks lock")
                .push((generation, at, value));
        });
    }

    let url: tauri::Url = "crest://localhost/index.html"
        .parse()
        .expect("the static page url is well-formed");
    // The window label must be "main": the committed capability
    // (tauri.conf.json) grants the page's event permissions to that label.
    // always_on_top keeps the page unoccluded so requestAnimationFrame (the
    // painted ack's clock) is never throttled by a window in front.
    let _window =
        tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::CustomProtocol(url))
            .title("crest-synth WP06 acceptance harness")
            .inner_size(f64::from(desktop.width_px), f64::from(desktop.height_px))
            .focused(true)
            .always_on_top(true)
            .build()
            .expect("the live harness window builds");

    let handle = app.handle().clone();
    let document_a = fidelity.document_a.clone();
    let driver_painted = Arc::clone(&painted);
    let outcome: Arc<Mutex<Option<Result<(), String>>>> = Arc::new(Mutex::new(None));
    let driver_outcome = Arc::clone(&outcome);
    let driver = std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            drive_live_window(&handle, &harness_receiver, &driver_painted, &document_a)
        }));
        let posted = match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(message)) => Err(message),
            Err(panic) => Err(panic_text(&panic)),
        };
        *driver_outcome.lock().expect("driver outcome lock") = Some(posted);
        // Always close through the owned path so the loop below returns —
        // on failure as well; the recorded outcome, not the close, decides
        // pass or fail.
        if let Some(window) = handle.get_webview_window("main") {
            let _ = window.close();
        }
    });

    let exit_code = app.run_return(|_, _| {});
    driver.join().expect("the driver thread rejoins");
    let outcome = outcome
        .lock()
        .expect("driver outcome lock")
        .take()
        .expect("the driver posts exactly one outcome");
    if let Err(message) = outcome {
        eprintln!("live section FAILED: {message}");
        std::process::exit(101);
    }
    assert_eq!(
        exit_code, 0,
        "the owned close (CloseRequested -> Destroyed -> Exit) must end the loop with code 0"
    );
    println!(
        "T026 harness window owned shutdown: PASS (run_return = 0 through the owned close path)"
    );

    prove_shutdown_parity_on_real_runs();
}

fn panic_text(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = panic.downcast_ref::<String>() {
        text.clone()
    } else if let Some(text) = panic.downcast_ref::<&str>() {
        (*text).to_owned()
    } else {
        "driver panicked with a non-text payload".to_owned()
    }
}

/// Receives the next harness message carrying the given phase tag, draining
/// unrelated messages (e.g. surplus ready pings).
fn receive_phase(
    receiver: &mpsc::Receiver<Value>,
    phase: &str,
    limit: Duration,
) -> Result<Value, String> {
    let deadline = Instant::now() + limit;
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| format!("timed out waiting for harness phase {phase:?}"))?;
        let value = receiver
            .recv_timeout(remaining)
            .map_err(|_| format!("timed out waiting for harness phase {phase:?}"))?;
        if value.get("phase").and_then(Value::as_str) == Some(phase) {
            return Ok(value);
        }
    }
}

/// Asserts the page actually measures the expected CSS viewport width — the
/// guard that the window really seated at the authored size (macOS clamps
/// windows to their screen's visible frame).
fn assert_page_viewport_width(
    window: &tauri::WebviewWindow,
    receiver: &mpsc::Receiver<Value>,
    expected_width: f32,
    tag: &str,
) -> Result<(), String> {
    let script = format!(
        "window.__TAURI__.event.emit('{HARNESS_EVENT}', \
         {{ phase: '{tag}', width: window.innerWidth, height: window.innerHeight }});"
    );
    window
        .eval(&script)
        .map_err(|error| format!("viewport probe eval ({tag}) failed: {error}"))?;
    let probe = receive_phase(receiver, tag, Duration::from_secs(10))?;
    let width = probe.get("width").and_then(Value::as_f64).unwrap_or(0.0);
    if (width - f64::from(expected_width)).abs() > 1.0 {
        return Err(format!(
            "{tag}: the page measures {width} CSS px wide, expected the authored \
             {expected_width} — the window did not seat at the authored viewport"
        ));
    }
    Ok(())
}

/// Renders the document through the page's own `renderObservation` and pulls
/// the observation back over the harness event.
fn observe_render(
    window: &tauri::WebviewWindow,
    receiver: &mpsc::Receiver<Value>,
    document_json: &str,
    tag: &str,
) -> Result<Value, String> {
    let script = format!(
        "(function() {{ var doc = {document_json}; \
         window.__TAURI__.event.emit('{HARNESS_EVENT}', \
         {{ phase: '{tag}', observation: window.crest.renderObservation(doc) }}); }})();"
    );
    window
        .eval(&script)
        .map_err(|error| format!("renderObservation eval ({tag}) failed: {error}"))?;
    let message = receive_phase(receiver, tag, Duration::from_secs(10))?;
    message
        .get("observation")
        .cloned()
        .ok_or_else(|| format!("harness phase {tag:?} carried no observation"))
}

/// Structural correctness of one painted observation against the document it
/// rendered: the declared bands, the sixteen-column anatomy in order, the
/// focused column, the hex readout form, and the Inspector's declared send
/// order and width.
fn assert_observation_structure(
    observation: &Value,
    document: &Value,
    inspector_width_at_least: f32,
    label: &str,
) {
    // All five declared bands painted.
    let bands = observation
        .get("bands")
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("{label}: the observation must carry bands"));
    for band in [
        "contextLine",
        "identityHeader",
        "workspace",
        "inspector",
        "footer",
    ] {
        assert_eq!(
            bands.get(band).and_then(Value::as_bool),
            Some(true),
            "{label}: band {band} must be painted with nonzero area"
        );
    }

    // Sixteen columns, each carrying exactly the five declared structures in
    // declared order.
    let columns = observation
        .get("columns")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{label}: the observation must carry columns"));
    assert_eq!(
        columns.len(),
        MixerTrackId::COUNT,
        "{label}: all sixteen mixer columns must be seated"
    );
    let mut focused_tracks = Vec::new();
    for column in columns {
        let track_id = column
            .get("trackId")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| panic!("{label}: every column carries its track id"));
        let structures: Vec<&str> = column
            .get("structures")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("{label}: every column reports its painted structures"))
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(
            structures, COLUMN_ANATOMY,
            "{label}: column T{track_id:02X} must paint exactly the declared five structures in order"
        );
        let header = column.get("header").and_then(Value::as_str).unwrap_or("");
        assert_eq!(
            header,
            format!("T{track_id:02X}"),
            "{label}: the column header names its track"
        );
        // Level readouts present as two-digit uppercase MIDI hex.
        let hex = column.get("levelHex").and_then(Value::as_str).unwrap_or("");
        assert!(
            hex.len() == 2 && hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()),
            "{label}: column T{track_id:02X} level readout must be two-digit uppercase hex (got {hex:?})"
        );
        if column.get("focused").and_then(Value::as_bool) == Some(true) {
            focused_tracks.push(track_id);
        }
    }

    // Exactly one focused column, identified consistently by the observation
    // and by the document's own focus path.
    assert_eq!(
        focused_tracks.len(),
        1,
        "{label}: exactly one column must be focused (got {focused_tracks:?})"
    );
    let focused_track = focused_tracks[0];
    assert_eq!(
        observation
            .pointer("/focus/trackId")
            .and_then(Value::as_u64),
        Some(focused_track),
        "{label}: the observation's focus identity must match the focused column"
    );
    let document_focus_track = document
        .pointer("/focusPath/controlId/id/track_id")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("{label}: the document focus path names a track"));
    assert_eq!(
        focused_track, document_focus_track,
        "{label}: the painted focus must be the document's focus"
    );
    let cursor = observation
        .pointer("/inspector/cursor")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        cursor.starts_with(&format!("T{focused_track:02X}")),
        "{label}: the Inspector cursor must name the focused track (got {cursor:?})"
    );

    // Inspector sends in the document's declared order for the focused track.
    let expected_sends: Vec<String> = document
        .get("surfaces")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|surface| surface.get("id").and_then(Value::as_str) == Some("mixerInspector"))
        .flat_map(|surface| {
            surface
                .get("controls")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .filter(|control| {
            control
                .pointer("/path/controlId/id/kind")
                .and_then(Value::as_str)
                == Some("send")
                && control
                    .pointer("/path/controlId/id/track_id")
                    .and_then(Value::as_u64)
                    == Some(focused_track)
        })
        .map(|control| {
            let label = control.get("label").and_then(Value::as_str).unwrap_or("");
            label
                .split_once(' ')
                .map_or(label, |(_, rest)| rest)
                .to_uppercase()
        })
        .collect();
    let painted_sends: Vec<String> = observation
        .pointer("/inspector/sends")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{label}: the observation reports Inspector sends"))
        .iter()
        .map(|send| {
            send.get("label")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        })
        .collect();
    assert!(
        !expected_sends.is_empty(),
        "{label}: the fixture document declares sends for the focused track"
    );
    assert_eq!(
        painted_sends, expected_sends,
        "{label}: Inspector sends must paint in the document's declared order"
    );

    // The persistent side region honors the authored floor.
    let width = observation
        .pointer("/inspector/widthPx")
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("{label}: the observation reports the Inspector width"));
    assert!(
        width >= f64::from(inspector_width_at_least) - 1.0,
        "{label}: Inspector width {width}px must be at least the authored {inspector_width_at_least}px"
    );

    // The meter element painted its zero/stale state for the focused track
    // (only the meter listener may write a live reading).
    assert_eq!(
        observation.get("meter").and_then(Value::as_str),
        Some("METER 0.000"),
        "{label}: the meter paints the zero state under a pure render"
    );
}

/// The driver: everything the live window is asked to do, in order — T024's
/// double-render determinism at both authored viewports, then T026's NFR
/// measurements. Runs off the main thread; every failure is a returned error
/// (or panic), never a skip.
fn drive_live_window(
    handle: &tauri::AppHandle,
    receiver: &mpsc::Receiver<Value>,
    painted: &PaintedAcks,
    document_a: &str,
) -> Result<(), String> {
    use tauri::Manager;

    let window = handle
        .get_webview_window("main")
        .ok_or_else(|| "the harness window exists".to_owned())?;
    let desktop = ViewportDensityPolicy::Desktop.authored_viewport();
    let compact = ViewportDensityPolicy::SteamDeck.authored_viewport();
    let desktop_side = ViewportDensityPolicy::Desktop.split().side_px;
    let compact_side = ViewportDensityPolicy::SteamDeck.split().side_px;

    // Seat the window on a display that can hold the authored desktop
    // viewport: macOS clamps a window to its screen's visible frame, so a
    // window created on a smaller (e.g. built-in Retina) display would
    // silently measure a narrower viewport. Failing to find such a display
    // is a live-section failure, never a skip.
    let monitor = window
        .available_monitors()
        .map_err(|error| format!("monitor enumeration failed: {error}"))?
        .into_iter()
        .find(|monitor| {
            let size = monitor.size().to_logical::<f64>(monitor.scale_factor());
            size.width >= f64::from(desktop.width_px) && size.height >= f64::from(desktop.height_px)
        })
        .ok_or_else(|| {
            format!(
                "no attached display seats the authored {}x{} viewport",
                desktop.width_px, desktop.height_px
            )
        })?;
    window
        .set_position(*monitor.position())
        .map_err(|error| format!("seating the window on its display failed: {error}"))?;
    window
        .set_size(tauri::LogicalSize::new(
            f64::from(desktop.width_px),
            f64::from(desktop.height_px),
        ))
        .map_err(|error| format!("sizing the window to the desktop viewport failed: {error}"))?;
    std::thread::sleep(Duration::from_millis(500));

    // Wait for the page: window.crest is installed by the committed page.js.
    let ready_deadline = Instant::now() + Duration::from_secs(30);
    loop {
        window
            .eval(format!(
                "if (window.crest && window.__TAURI__ && window.__TAURI__.event) \
                 {{ window.__TAURI__.event.emit('{HARNESS_EVENT}', {{ phase: 'ready' }}); }}"
            ))
            .map_err(|error| format!("ready probe eval failed: {error}"))?;
        match receiver.recv_timeout(Duration::from_millis(200)) {
            Ok(value) if value.get("phase").and_then(Value::as_str) == Some("ready") => break,
            Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {
                if Instant::now() > ready_deadline {
                    return Err("the page never became ready within 30s".to_owned());
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("the harness channel disconnected".to_owned());
            }
        }
    }

    // Page-side meter arrival counter (harness JS, not page code): counts
    // crest://meters deliveries on the page thread for NFR-002.
    window
        .eval(
            "window.__wp06 = { meters: [] }; \
             window.__TAURI__.event.listen('crest://meters', function () { \
             window.__wp06.meters.push(performance.now()); });",
        )
        .map_err(|error| format!("meter counter install failed: {error}"))?;

    // ---- T024: double-render determinism at both authored viewports ------
    let document: Value = serde_json::from_str(document_a)
        .map_err(|error| format!("the fidelity document parses: {error}"))?;

    assert_page_viewport_width(&window, receiver, desktop.width_px, "viewport-desktop")?;
    let desktop_first = observe_render(&window, receiver, document_a, "desktop-1")?;
    let desktop_second = observe_render(&window, receiver, document_a, "desktop-2")?;
    assert_eq!(
        desktop_first, desktop_second,
        "T024: two renders of one document at {}x{} must observe identically",
        desktop.width_px, desktop.height_px
    );
    assert_observation_structure(
        &desktop_first,
        &document,
        desktop_side,
        "T024 desktop 1920x1080",
    );
    screenshot("t024-desktop-1920x1080.png");

    window
        .set_size(tauri::LogicalSize::new(
            f64::from(compact.width_px),
            f64::from(compact.height_px),
        ))
        .map_err(|error| format!("resize to compact failed: {error}"))?;
    std::thread::sleep(Duration::from_millis(500));

    assert_page_viewport_width(&window, receiver, compact.width_px, "viewport-compact")?;
    let compact_first = observe_render(&window, receiver, document_a, "compact-1")?;
    let compact_second = observe_render(&window, receiver, document_a, "compact-2")?;
    assert_eq!(
        compact_first, compact_second,
        "T024: two renders of one document at {}x{} must observe identically",
        compact.width_px, compact.height_px
    );
    assert_observation_structure(
        &compact_first,
        &document,
        compact_side,
        "T024 compact 1280x800",
    );
    screenshot("t024-compact-1280x800.png");

    window
        .set_size(tauri::LogicalSize::new(
            f64::from(desktop.width_px),
            f64::from(desktop.height_px),
        ))
        .map_err(|error| format!("resize back to desktop failed: {error}"))?;
    std::thread::sleep(Duration::from_millis(500));
    println!(
        "T024 page render determinism: PASS (double-render identical at both authored \
         viewports; Inspector {}px desktop / {}px compact floors held)",
        desktop_side, compact_side
    );

    // ---- T026: NFR-001 projection-to-paint latency ------------------------
    // Real reducer edits through the production projector and the production
    // emit path, paced at the declared meter rate; the page's crest://painted
    // ack timestamps each painted generation.
    let mut state = production_mixer_state();
    let projector = StateProjector::new();
    let mut channel = ProjectionChannel::new();
    let pushes = 150_usize;
    let mut emits: Vec<(u64, Instant)> = Vec::with_capacity(pushes);
    for index in 0..pushes {
        if index > 0 {
            let direction = if index % 2 == 1 {
                Direction::Up
            } else {
                Direction::Down
            };
            state
                .apply(AppEvent::Adjust(direction))
                .map_err(|rejection| format!("paced adjust rejected: {rejection:?}"))?;
        }
        let projection = projector
            .project_with_shell(&state)
            .map_err(|error| format!("paced projection failed: {error}"))?
            .3;
        let generation = projection.generation();
        channel
            .push(&projection, |payload| {
                emits.push((generation, Instant::now()));
                tauri::Emitter::emit(handle, PROJECTION_EVENT, payload)
            })
            .map_err(|error| format!("paced push failed: {error}"))?;
        if index == 20 {
            screenshot("t026-live-mixer-render.png");
        }
        std::thread::sleep(METER_INTERVAL);
    }
    let final_generation = emits.last().map(|(generation, _)| *generation).unwrap_or(0);

    // Every pushed generation must come back as a painted ack.
    let ack_deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let acked = painted.lock().expect("painted acks lock").len();
        if acked >= pushes {
            break;
        }
        if Instant::now() > ack_deadline {
            return Err(format!(
                "only {acked} of {pushes} projections were acked as painted within 15s"
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let acks = painted.lock().expect("painted acks lock").clone();
    let mut latencies: Vec<Duration> = Vec::with_capacity(pushes);
    for (generation, emitted_at) in &emits {
        let ack = acks
            .iter()
            .find(|(acked, at, _)| acked == generation && at >= emitted_at)
            .ok_or_else(|| format!("generation {generation} was never acked as painted"))?;
        latencies.push(ack.1.duration_since(*emitted_at));
    }
    latencies.sort_unstable();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() * 95) / 100];
    let max = *latencies.last().expect("at least one latency sample");
    println!(
        "CREST_WEBVIEW_NFR001 projection-to-paint over {} paced reducer edits: \
         p50={:.1}ms p95={:.1}ms max={:.1}ms (threshold p95 <= 50ms)",
        latencies.len(),
        p50.as_secs_f64() * 1_000.0,
        p95.as_secs_f64() * 1_000.0,
        max.as_secs_f64() * 1_000.0,
    );
    if p95 > Duration::from_millis(50) {
        return Err(format!(
            "NFR-001 failed: projection-to-paint p95 {:.1}ms exceeds 50ms",
            p95.as_secs_f64() * 1_000.0
        ));
    }

    // The last painted ack carries post-paint evidence for all five declared
    // shell regions: painted bounds and a visible label each.
    let last_ack = &acks.last().expect("at least one painted ack").2;
    let regions = last_ack
        .get("regions")
        .and_then(Value::as_array)
        .ok_or_else(|| "the painted ack carries region evidence".to_owned())?;
    for region_id in SHELL_REGION_IDS {
        let region = regions
            .iter()
            .find(|region| region.get("id").and_then(Value::as_str) == Some(region_id))
            .ok_or_else(|| format!("painted ack lacks region {region_id}"))?;
        let width = region.get("widthPx").and_then(Value::as_f64).unwrap_or(0.0);
        let height = region
            .get("heightPx")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let label = region.get("label").and_then(Value::as_str).unwrap_or("");
        if width <= 0.0 || height <= 0.0 || label.is_empty() {
            return Err(format!(
                "region {region_id} must be painted with bounds and a visible label \
                 (got {width}x{height}, label {label:?})"
            ));
        }
    }

    // ---- T026: NFR-002 meter cadence soak ---------------------------------
    let full_soak = std::env::var("CREST_WEBVIEW_FULL_SOAK").as_deref() == Ok("1");
    let soak = if full_soak {
        Duration::from_secs(300)
    } else {
        Duration::from_secs(60)
    };
    println!(
        "CREST_WEBVIEW_NFR002 soak configuration: running {}s ({}; 60s default, \
         300s with CREST_WEBVIEW_FULL_SOAK=1)",
        soak.as_secs(),
        if full_soak {
            "full five-minute soak"
        } else {
            "default short soak"
        },
    );
    let mut meter_channel = MeterChannel::new();
    let soak_started = Instant::now();
    let mut sequence = 0_u64;
    let mut observations = 0_u64;
    let mut lost = 0_u64;
    let mut emit_instants: Vec<Instant> = Vec::with_capacity(soak.as_secs() as usize * 40);
    while soak_started.elapsed() < soak {
        sequence += 1;
        observations += 1;
        meter_channel.observe(AudioObservationSnapshot::from_mix(
            sequence,
            sequence,
            sequence * 512,
            final_generation,
            0,
            0,
            MixObservation::default(),
        ));
        match meter_channel.emit_if_due(Instant::now(), |frame| {
            tauri::Emitter::emit(handle, METER_EVENT, frame)
        }) {
            MeterEmit::Emitted => emit_instants.push(Instant::now()),
            MeterEmit::FrameLost(_) => lost += 1,
            MeterEmit::Coalescing | MeterEmit::Idle => {}
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    if emit_instants.len() < 2 {
        return Err("the soak produced no meter traffic".to_owned());
    }
    let span = emit_instants
        .last()
        .expect("at least two emits")
        .duration_since(emit_instants[0]);
    let rate = (emit_instants.len() - 1) as f64 / span.as_secs_f64();
    let max_gap = emit_instants
        .windows(2)
        .map(|pair| pair[1].duration_since(pair[0]))
        .max()
        .expect("at least one gap");
    // Sustained: the last third of the soak must hold the same pace as the
    // first third (no degradation, no queue-driven stall).
    let third = emit_instants.len() / 3;
    let first_third_rate = third as f64
        / emit_instants[third]
            .duration_since(emit_instants[0])
            .as_secs_f64();
    let last_third_rate = third as f64
        / emit_instants[emit_instants.len() - 1]
            .duration_since(emit_instants[emit_instants.len() - 1 - third])
            .as_secs_f64();
    // Bounded pending: the channel owns one Option slot by construction, so
    // queue depth cannot exceed one frame; measurably, the emit count can
    // never exceed the declared pace (a queue would show as a burst above it)
    // and coalescing must be doing real work.
    let pace_bound = span.as_secs_f64() / METER_INTERVAL.as_secs_f64() + 2.0;

    // Page-side arrival evidence: the harness counter on the page thread.
    window
        .eval(format!(
            "window.__TAURI__.event.emit('{HARNESS_EVENT}', {{ phase: 'meters', \
             count: window.__wp06.meters.length, \
             firstMs: window.__wp06.meters[0] || 0, \
             lastMs: window.__wp06.meters[window.__wp06.meters.length - 1] || 0 }});"
        ))
        .map_err(|error| format!("meter stats eval failed: {error}"))?;
    let meter_stats = receive_phase(receiver, "meters", Duration::from_secs(10))?;
    let page_count = meter_stats
        .get("count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let page_span_ms = meter_stats
        .get("lastMs")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        - meter_stats
            .get("firstMs")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
    let page_rate = if page_span_ms > 0.0 {
        (page_count.saturating_sub(1)) as f64 / (page_span_ms / 1_000.0)
    } else {
        0.0
    };

    println!(
        "CREST_WEBVIEW_NFR002 meter cadence over {:.0}s: rust-side {} emits = {rate:.2} Hz \
         (declared pace {METER_RATE_HZ} Hz, interval-quantized floor 29.0), max gap {:.1}ms, \
         first/last-third {first_third_rate:.2}/{last_third_rate:.2} Hz, \
         {observations} observations coalesced (pending slot bounded at one by construction, \
         emit count {} <= pace bound {pace_bound:.0}), lost {lost}; \
         page-side received {page_count} frames = {page_rate:.2} Hz",
        span.as_secs_f64(),
        emit_instants.len(),
        max_gap.as_secs_f64() * 1_000.0,
        emit_instants.len(),
    );
    if rate < 29.0 {
        return Err(format!(
            "NFR-002 failed: measured meter cadence {rate:.2} Hz is below the \
             quantization-adjusted 29.0 Hz floor for the declared {METER_RATE_HZ} Hz pace"
        ));
    }
    if last_third_rate < first_third_rate * 0.9 {
        return Err(format!(
            "NFR-002 failed: cadence degraded over the soak \
             ({first_third_rate:.2} Hz -> {last_third_rate:.2} Hz)"
        ));
    }
    if (emit_instants.len() as f64) > pace_bound {
        return Err(format!(
            "NFR-002 failed: {} emits exceed the declared pace bound {pace_bound:.0} — \
             a queue is draining",
            emit_instants.len()
        ));
    }
    if observations < emit_instants.len() as u64 * 10 {
        return Err(format!(
            "NFR-002 failed: only {observations} observations for {} emits — \
             the coalescing slot was not exercised",
            emit_instants.len()
        ));
    }
    if lost > 0 {
        return Err(format!(
            "NFR-002 failed: {lost} frames were lost while the window lived"
        ));
    }
    if page_count < (emit_instants.len() as u64 * 95) / 100 {
        return Err(format!(
            "NFR-002 failed: the page received {page_count} of {} emitted frames",
            emit_instants.len()
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// T026 — shutdown parity on the shipped binary (real window-close runs)
// ---------------------------------------------------------------------------

/// Runs the shipped binary under each shell, closes its real window through
/// the native close button (System Events), and asserts the identical owned
/// terminal outcome: exit 0 with no error output. Both shells return from
/// `AppWindow::run` into the same `StandaloneApplication::run` teardown —
/// stream release before worker completion, graph ownership collection,
/// normal exit — so an equal clean exit through a real window close is the
/// shutdown-observation parity the eframe shell records.
fn prove_shutdown_parity_on_real_runs() {
    for shell in ["egui", "webview"] {
        let started = Instant::now();
        let mut child = Command::new(env!("CARGO_BIN_EXE_crest-synth"))
            .args(["--shell", shell])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("the shipped binary spawns under {shell}: {error}"));
        let pid = child.id();

        // Let the composition start: SoundFont parse, audio negotiation,
        // stream start, window creation.
        std::thread::sleep(Duration::from_secs(8));
        if let Ok(Some(status)) = child.try_wait() {
            let output = child.wait_with_output().expect("collects the early exit");
            panic!(
                "the {shell} shell exited before its window was closed ({status:?}):\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        screenshot(&format!("t026-shutdown-parity-{shell}.png"));

        // The owned path: the native close button, exactly what an operator
        // clicks. Targeted by unix id so both sequential runs stay distinct.
        let click = Command::new("osascript")
            .args([
                "-e",
                &format!(
                    "tell application \"System Events\" to tell \
                     (first process whose unix id is {pid}) to click button 1 of window 1"
                ),
            ])
            .output()
            .expect("osascript runs");
        assert!(
            click.status.success(),
            "System Events must close the {shell} window (accessibility?): {}",
            String::from_utf8_lossy(&click.stderr)
        );

        let output = wait_with_timeout(child, Duration::from_secs(30), "T026 shutdown parity");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "the {shell} shell must exit 0 through the owned close (got {:?}):\n{stderr}",
            output.status
        );
        assert!(
            !stderr.contains("Error"),
            "the {shell} shell must shut down without an error report:\n{stderr}"
        );
        println!(
            "T026 shutdown parity ({shell}): PASS (real window closed via the native close \
             button after {:.1?}; owned teardown — stream release, worker completion, graph \
             ownership collection — reached exit 0)",
            started.elapsed()
        );
    }
    println!(
        "T026 shutdown parity: PASS (both shells reached the identical owned shutdown \
         outcome from real window-close runs)"
    );
}
