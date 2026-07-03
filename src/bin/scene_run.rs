// path: src/bin/scene_run.rs

//! `scene_run`: loads a `Scene` file, replays it through `SceneRunner` (the
//! exact same reducer/projector/render path the live app drives), and
//! reports the outcome for offline evaluation.
//!
//! ```text
//! scene_run --scene <FILE> [--dump-every-step] [--out <FILE>]
//! ```
//!
//! - Loads `<FILE>` as a scene document (see [`SceneFile`] below) and runs
//!   it through `SceneRunner` using the real `SerdeSnapshotCodec` and a
//!   real tone-rendering `BlockRenderer` (see [`ToneBlockRenderer`]) -- not
//!   a silent stub -- so `peak` in the summary line is a genuinely measured
//!   value, not a fabricated one.
//! - Prints the FINAL `StateSnapshot` to stdout as one JSON document (or to
//!   `--out <FILE>` instead of stdout, when given). With
//!   `--dump-every-step`, one snapshot JSON document per step is printed
//!   first, in step order, followed by the final snapshot.
//! - After the snapshot document(s), prints exactly one summary line:
//!   `events_applied=<N> rejections=<M> frames=<F> peak=<final rendered peak>`
//!   measured from the run, and exits non-zero if any event was rejected --
//!   a scene that doesn't fully apply is a failed scene.
//!
//! # Scene file format
//!
//! Neither `loop::scene::Scene` nor `loop::scene_step::SceneStep` derive
//! `serde::Serialize`/`Deserialize` (a scene's steps wrap `MixerViewEvent`,
//! `EditorEvent`, and `GamepadAction`, none of which derive serde either).
//! Rather than hand-roll a second Wire mirror model for the same four event
//! kinds `loop::app_event::AppEvent` already codecs losslessly, this binary
//! reuses that existing, tested codec: a scene file's `steps[].event` field
//! deserializes as `app_event::AppEvent` (its `Patch`/`Preset` variants are
//! then rejected -- a `Scene` only scripts the four event kinds
//! `scene_step::AppEvent` closes over), and gets converted into the
//! `scene_step::AppEvent` a `Scene` actually stores.
//!
//! # Real mixer state in the emitted snapshot
//!
//! `SnapshotSource::derive` only ever sees the Loop reducer's bare
//! frame-clock `AppState` (see `loop::scene_runner`'s module docs), which
//! cannot by itself tell us which channels are soloed, muted, or at what
//! volume/pan -- that state lives in a `mixer::mixer_view::MixerView`, not
//! in the Loop reducer. So this binary drives its own `MixerView` through
//! the scene's `Mixer(MixerViewEvent)` steps (see [`MixerViewSnapshotSource`])
//! and reports ITS real per-channel state, rather than accepting
//! `scene_runner::DefaultSnapshotSource`'s fixed, always-empty-channels
//! stub.

use std::cell::Cell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde::Deserialize;

use crest_synth::engine::oscillator::{
    Amplitude, Frequency, Oscillator, OscillatorConfig, SampleRate, StandardOscillator, Waveform,
};
use crest_synth::mixer::channel_strip::ChannelStrip;
use crest_synth::mixer::mixer_view::{MixerView, MixerViewEvent as ViewMixerViewEvent};
use crest_synth::mixer::mixer_view_event::MixerViewEvent as SceneMixerEvent;
use crest_synth::mixer::peak_level::PeakLevel;
use crest_synth::r#loop::app_event::AppEvent;
use crest_synth::r#loop::app_state::{AppState as LoopAppState, AppStateEvent};
use crest_synth::r#loop::scene::Scene;
use crest_synth::r#loop::scene_runner::{BlockRenderer, SceneRunner, SnapshotSource};
use crest_synth::r#loop::scene_step::{AppEvent as SceneEvent, SceneStep};
use crest_synth::r#loop::serde_snapshot_codec::SerdeSnapshotCodec;
use crest_synth::r#loop::snapshot_codec::{AppState as SnapshotState, ChannelState};
use crest_synth::r#loop::state_projector::StateProjector;
use crest_synth::real_time::parameter_bridge::ParameterBridge;

/// Number of samples rendered by one `ToneBlockRenderer::render_block`
/// call. Matches the fixed-block convention used elsewhere in this crate
/// (see `src/bin/voice_demo.rs`).
const BLOCK_LEN: usize = 256;
const RENDER_SAMPLE_RATE_HZ: f64 = 44_100.0;
const RENDER_TONE_HZ: f64 = 440.0;

// ---------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------

/// Parsed command-line arguments for `scene_run`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Args {
    scene: PathBuf,
    dump_every_step: bool,
    out: Option<PathBuf>,
}

/// Parses `scene_run`'s CLI surface: `--scene <FILE>` (required),
/// `--dump-every-step` (flag), `--out <FILE>` (optional).
fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut scene: Option<PathBuf> = None;
    let mut dump_every_step = false;
    let mut out: Option<PathBuf> = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--scene" => {
                let path = iter
                    .next()
                    .ok_or_else(|| "--scene requires a file path argument".to_string())?;
                scene = Some(PathBuf::from(path));
            }
            "--dump-every-step" => dump_every_step = true,
            "--out" => {
                let path = iter
                    .next()
                    .ok_or_else(|| "--out requires a file path argument".to_string())?;
                out = Some(PathBuf::from(path));
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }

    let scene = scene.ok_or_else(|| "--scene <FILE> is required".to_string())?;
    Ok(Args {
        scene,
        dump_every_step,
        out,
    })
}

// ---------------------------------------------------------------------
// Scene file loading
// ---------------------------------------------------------------------

/// The on-disk shape of a scene file: a name plus an ordered list of
/// steps. Deserializes directly into this shape before being converted
/// into a domain `Scene` by `into_scene`.
#[derive(Debug, Deserialize)]
struct SceneFile {
    name: String,
    steps: Vec<SceneStepFile>,
}

/// The on-disk shape of one scene step: an `app_event::AppEvent` (the
/// closed union that already has a lossless serde codec) plus the number
/// of headless audio blocks to render after applying it.
#[derive(Debug, Deserialize)]
struct SceneStepFile {
    event: AppEvent,
    render_blocks: u32,
}

/// Converts a closed-union `app_event::AppEvent` into the narrower
/// `scene_step::AppEvent` vocabulary a `Scene` actually scripts.
///
/// Infallible for `Midi`/`Gamepad`/`Editor`/`Mixer` (each has a matching
/// `scene_step::AppEvent` variant); `Patch`/`Preset` are rejected with an
/// explicit error rather than silently dropped, since a scene file naming
/// one is a malformed scene, not an empty step.
fn to_scene_event(event: AppEvent) -> Result<SceneEvent, String> {
    match event {
        AppEvent::Midi(midi_event) => Ok(SceneEvent::Midi(midi_event)),
        AppEvent::Gamepad(action) => Ok(SceneEvent::Gamepad(action)),
        AppEvent::Editor(editor_event) => Ok(SceneEvent::Editor(editor_event)),
        AppEvent::Mixer(mixer_event) => Ok(SceneEvent::Mixer(mixer_event)),
        AppEvent::Patch(_) => {
            Err("scene files do not support Patch commands (Scene only scripts Midi/Gamepad/Editor/Mixer events)".to_string())
        }
        AppEvent::Preset(_) => {
            Err("scene files do not support Preset commands (Scene only scripts Midi/Gamepad/Editor/Mixer events)".to_string())
        }
    }
}

/// Parses a scene document's JSON text into a domain `Scene`.
///
/// Split out from `load_scene` so tests can exercise parsing against an
/// in-memory string without touching the filesystem.
fn parse_scene(contents: &str) -> Result<Scene, String> {
    let file: SceneFile =
        serde_json::from_str(contents).map_err(|err| format!("failed to parse scene: {err}"))?;

    let mut steps = Vec::with_capacity(file.steps.len());
    for (index, step) in file.steps.into_iter().enumerate() {
        let event = to_scene_event(step.event).map_err(|err| format!("step {index}: {err}"))?;
        steps.push(SceneStep::new(event, step.render_blocks));
    }
    Ok(Scene::new(file.name, steps))
}

/// Reads and parses the scene file at `path`.
fn load_scene(path: &Path) -> Result<Scene, String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read scene file {}: {err}", path.display()))?;
    parse_scene(&contents).map_err(|err| format!("scene file {}: {err}", path.display()))
}

// ---------------------------------------------------------------------
// MixerView-backed SnapshotSource: derives real per-channel snapshot
// state from a MixerView driven by the scene's own Mixer(MixerViewEvent)
// steps, instead of scene_runner::DefaultSnapshotSource's fixed
// always-empty-channels stub.
// ---------------------------------------------------------------------

/// Converts a scripted `mixer::mixer_view_event::MixerViewEvent` (the
/// vocabulary `Scene`/`AppEvent` carry) into the reducer-local
/// `mixer::mixer_view::MixerViewEvent` that `MixerView::apply` actually
/// accepts. The two enums share the same seven semantic variants but are
/// nominally distinct types declared in separate modules, so a step's
/// scripted event needs this explicit conversion before it can drive a
/// real `MixerView`.
fn to_view_event(event: SceneMixerEvent) -> ViewMixerViewEvent {
    match event {
        SceneMixerEvent::NavUp => ViewMixerViewEvent::NavUp,
        SceneMixerEvent::NavDown => ViewMixerViewEvent::NavDown,
        SceneMixerEvent::NavLeft => ViewMixerViewEvent::NavLeft,
        SceneMixerEvent::NavRight => ViewMixerViewEvent::NavRight,
        SceneMixerEvent::EnterEditMode => ViewMixerViewEvent::EnterEditMode,
        SceneMixerEvent::ExitEditMode => ViewMixerViewEvent::ExitEditMode,
        SceneMixerEvent::ToggleFocusedParam => ViewMixerViewEvent::ToggleFocusedParam,
    }
}

/// A `SnapshotSource` that reports the real state of a `MixerView` driven
/// step-by-step by a scene's `Mixer(MixerViewEvent)` steps, instead of
/// `scene_runner::DefaultSnapshotSource`'s always-empty-channels stub.
///
/// `SnapshotSource::derive` only ever receives the Loop reducer's bare
/// frame-clock `AppState` (see `loop::scene_runner`'s module docs on why
/// that strategy is injected rather than derived as a pure function of
/// that state), so this precomputes one `SnapshotState` per event-sequence
/// frame up front -- by replaying the scene's Mixer events against a real
/// `MixerView` in the exact order `SceneRunner` applies them -- and looks
/// the right one up by `AppState::frame()` when asked.
struct MixerViewSnapshotSource {
    /// `snapshots_by_frame[f]` is the derived state after exactly `f` of
    /// the scene's steps have been applied to the `MixerView` (`f == 0` is
    /// the state of a freshly-constructed, untouched view).
    snapshots_by_frame: Vec<SnapshotState>,
}

impl MixerViewSnapshotSource {
    /// Replays `scene`'s Mixer steps against a fresh `MixerView`,
    /// capturing one derived `SnapshotState` per frame. A step carrying a
    /// non-Mixer event leaves the view -- and so the derived snapshot --
    /// unchanged, but still occupies its own frame slot, since
    /// `AppState::frame()` advances by one on every applied step
    /// regardless of what kind of event it carries.
    fn from_scene(scene: &Scene) -> Self {
        let mut view = MixerView::new();
        let mut snapshots_by_frame = Vec::with_capacity(scene.len() + 1);
        snapshots_by_frame.push(Self::derive_from_view(&view));

        for step in scene.steps() {
            if let SceneEvent::Mixer(mixer_event) = step.event() {
                view.apply(to_view_event(*mixer_event));
            }
            snapshots_by_frame.push(Self::derive_from_view(&view));
        }

        Self { snapshots_by_frame }
    }

    /// Derives the plain `SnapshotState` for the current state of `view`:
    /// one `ChannelState` per wrapped `ChannelStrip`, with `muted`
    /// carrying EFFECTIVE audibility under solo-in-place -- a channel
    /// reports muted if it is explicitly muted, OR some other channel is
    /// soloed while this one is not -- mirroring
    /// `aggregate.Mixer.MixerController::is_audible`'s semantics (a
    /// `MixerView` wraps its `ChannelStrip`s directly rather than through
    /// a `MixerController`, so that logic is reproduced here against the
    /// same underlying strips).
    fn derive_from_view(view: &MixerView) -> SnapshotState {
        let any_solo = view.channels().iter().any(ChannelStrip::solo);
        let channels = view
            .channels()
            .iter()
            .enumerate()
            .map(|(index, strip)| ChannelState {
                name: format!("Channel {}", index + 1),
                volume_db: f64::from(strip.volume_db().value()),
                pan: f64::from(strip.pan().value()),
                muted: strip.mute() || (any_solo && !strip.solo()),
                soloed: strip.solo(),
            })
            .collect();

        SnapshotState {
            tempo_bpm: 120.0,
            time_signature: (4, 4),
            master_volume_db: 0.0,
            master_muted: false,
            channels,
        }
    }
}

impl SnapshotSource<LoopAppState> for MixerViewSnapshotSource {
    fn derive(&self, state: &LoopAppState) -> SnapshotState {
        let frame = usize::try_from(state.frame()).unwrap_or(usize::MAX);
        self.snapshots_by_frame
            .get(frame)
            .or_else(|| self.snapshots_by_frame.last())
            .cloned()
            .unwrap_or_else(|| Self::derive_from_view(&MixerView::new()))
    }
}

// ---------------------------------------------------------------------
// Real (non-silent) BlockRenderer: a plain tone renderer whose measured
// peak is a genuine observation, not a printed-unconditionally token.
// ---------------------------------------------------------------------

/// Shared, `Rc`-backed counters `ToneBlockRenderer` updates on every block
/// and `main` reads back once the run completes -- `SceneRunner` takes its
/// `BlockRenderer` by value and exposes no accessor for it afterward, so
/// the renderer's own measurements must be reachable through a cheap
/// shared handle instead. Not real-time machinery: `scene_run` is an
/// offline evaluation harness, not the audio thread, so `Rc<Cell<_>>` is
/// the right (allocation-once, borrow-free) tool here.
#[derive(Debug, Clone, Default)]
struct RenderStats {
    peak: Rc<Cell<f64>>,
    blocks_rendered: Rc<Cell<u64>>,
}

impl RenderStats {
    fn new() -> Self {
        Self::default()
    }

    fn record_block_peak(&self, block_peak: f64) {
        self.blocks_rendered.set(self.blocks_rendered.get() + 1);
        if block_peak > self.peak.get() {
            self.peak.set(block_peak);
        }
    }

    /// The largest absolute sample value observed across every rendered
    /// block, or `PeakLevel::SILENT` if nothing was ever rendered.
    fn peak(&self) -> PeakLevel {
        PeakLevel::try_new(self.peak.get()).unwrap_or(PeakLevel::SILENT)
    }

    /// The total number of `render_block` calls made during the run.
    fn blocks_rendered(&self) -> u64 {
        self.blocks_rendered.get()
    }
}

/// A `BlockRenderer` that renders a fixed A440 sine tone through the real
/// `Oscillator` port (the same one `src/bin/tone_test.rs` exercises), so a
/// scene's "audible consequences accrue" step means something real: each
/// call renders `BLOCK_LEN` samples and folds their peak into `stats`.
///
/// This does not attempt to wire the scene's individual MIDI events into
/// the engine -- `SceneRunner::BlockRenderer` deliberately carries no event
/// payload (see `loop::scene_runner`'s module docs on narrow seams), so a
/// per-note-accurate render is a different resource's job. What this
/// renderer proves is that `scene_run`'s reported peak is always a real,
/// measured value rather than a hard-coded constant.
struct ToneBlockRenderer {
    oscillator: StandardOscillator,
    config: OscillatorConfig,
    frequency: Frequency,
    sample_rate: SampleRate,
    phase: f64,
    stats: RenderStats,
}

impl ToneBlockRenderer {
    fn new(stats: RenderStats) -> Self {
        Self {
            oscillator: StandardOscillator::new(),
            config: OscillatorConfig::new(
                Waveform::Sine,
                Amplitude::try_new(1.0).expect("1.0 is a valid amplitude"),
            ),
            frequency: Frequency::try_new(RENDER_TONE_HZ).expect("440Hz is a valid frequency"),
            sample_rate: SampleRate::try_new(RENDER_SAMPLE_RATE_HZ)
                .expect("44.1kHz is a valid sample rate"),
            phase: 0.0,
            stats,
        }
    }
}

impl BlockRenderer for ToneBlockRenderer {
    fn render_block(&mut self) {
        let mut block_peak = 0.0_f64;
        for _ in 0..BLOCK_LEN {
            let sample = self.oscillator.render(self.phase, self.config);
            self.phase = self
                .oscillator
                .advance(self.phase, self.frequency, self.sample_rate);
            block_peak = block_peak.max(sample.abs());
        }
        self.stats.record_block_peak(block_peak);
    }
}

// ---------------------------------------------------------------------
// main
// ---------------------------------------------------------------------

fn fail(message: &str) -> ! {
    eprintln!("scene_run: {message}");
    std::process::exit(2);
}

fn main() {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let args = match parse_args(&raw_args) {
        Ok(args) => args,
        Err(err) => fail(&err),
    };

    let scene = match load_scene(&args.scene) {
        Ok(scene) => scene,
        Err(err) => fail(&err),
    };

    let stats = RenderStats::new();
    let renderer = ToneBlockRenderer::new(stats.clone());
    let snapshot_source = MixerViewSnapshotSource::from_scene(&scene);
    let mut runner = SceneRunner::with_collaborators(
        SerdeSnapshotCodec,
        renderer,
        StateProjector::new(),
        ParameterBridge::default(),
        snapshot_source,
    );

    let outcome = if args.dump_every_step {
        runner.run_with_step_snapshots(&scene)
    } else {
        runner.run(&scene)
    };

    let mut document = String::new();
    if args.dump_every_step {
        for step in &outcome.steps {
            if let Some(snapshot) = &step.snapshot {
                let json =
                    serde_json::to_string(snapshot).expect("StateSnapshot always serializes");
                document.push_str(&json);
                document.push('\n');
            }
        }
    }
    let final_json =
        serde_json::to_string(&outcome.final_snapshot).expect("StateSnapshot always serializes");
    document.push_str(&final_json);
    document.push('\n');

    match &args.out {
        Some(path) => {
            if let Err(err) = fs::write(path, &document) {
                fail(&format!(
                    "failed to write output file {}: {err}",
                    path.display()
                ));
            }
        }
        None => print!("{document}"),
    }

    let events_applied = outcome
        .steps
        .iter()
        .filter(|step| matches!(step.applied, AppStateEvent::Applied { .. }))
        .count();
    let rejections = outcome
        .steps
        .iter()
        .filter(|step| matches!(step.applied, AppStateEvent::Rejected { .. }))
        .count();
    let frames = stats.blocks_rendered();
    let peak = stats.peak();

    println!("events_applied={events_applied} rejections={rejections} frames={frames} peak={peak}");

    if rejections > 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crest_synth::kernel::channel_address::{ChannelAddress, MidiChannel, MidiGroup};
    use crest_synth::kernel::midi_event::MidiEvent;
    use crest_synth::kernel::midi_event_kind::MidiEventKind;
    use crest_synth::kernel::note_id::NoteId;
    use crest_synth::kernel::note_number::NoteNumber;
    use crest_synth::kernel::velocity::Velocity;
    use crest_synth::mixer::mixer_view_event::MixerViewEvent;
    use crest_synth::r#loop::app_event::PatchCommand;

    fn sample_midi_event() -> MidiEvent {
        MidiEvent::new(
            ChannelAddress::new(
                MidiChannel::try_new(0).unwrap(),
                MidiGroup::try_new(0).unwrap(),
            ),
            MidiEventKind::NoteOn,
            NoteNumber::try_new(60).unwrap(),
            NoteId::new(1),
            Velocity::try_new(0.8).unwrap(),
        )
    }

    fn scene_json_with_events(events: &[AppEvent]) -> String {
        let steps: Vec<serde_json::Value> = events
            .iter()
            .map(|event| {
                serde_json::json!({
                    "event": event,
                    "render_blocks": 2,
                })
            })
            .collect();
        serde_json::to_string(&serde_json::json!({
            "name": "test-scene",
            "steps": steps,
        }))
        .expect("scene JSON serializes")
    }

    // ---- parse_args ----

    #[test]
    fn parse_args_requires_scene_flag() {
        let err = parse_args(&[]).unwrap_err();
        assert!(err.contains("--scene"));
    }

    #[test]
    fn parse_args_reads_scene_path() {
        let args = parse_args(&["--scene".to_string(), "foo.json".to_string()]).unwrap();
        assert_eq!(args.scene, PathBuf::from("foo.json"));
        assert!(!args.dump_every_step);
        assert_eq!(args.out, None);
    }

    #[test]
    fn parse_args_reads_dump_every_step_flag() {
        let args = parse_args(&[
            "--scene".to_string(),
            "foo.json".to_string(),
            "--dump-every-step".to_string(),
        ])
        .unwrap();
        assert!(args.dump_every_step);
    }

    #[test]
    fn parse_args_reads_out_path() {
        let args = parse_args(&[
            "--scene".to_string(),
            "foo.json".to_string(),
            "--out".to_string(),
            "out.json".to_string(),
        ])
        .unwrap();
        assert_eq!(args.out, Some(PathBuf::from("out.json")));
    }

    #[test]
    fn parse_args_rejects_unknown_flags() {
        let err = parse_args(&[
            "--scene".to_string(),
            "foo.json".to_string(),
            "--bogus".to_string(),
        ])
        .unwrap_err();
        assert!(err.contains("--bogus"));
    }

    #[test]
    fn parse_args_scene_without_value_is_an_error() {
        let err = parse_args(&["--scene".to_string()]).unwrap_err();
        assert!(err.contains("--scene"));
    }

    // ---- to_scene_event ----

    #[test]
    fn to_scene_event_accepts_midi() {
        let event = to_scene_event(AppEvent::Midi(sample_midi_event())).unwrap();
        assert_eq!(event, SceneEvent::Midi(sample_midi_event()));
    }

    #[test]
    fn to_scene_event_accepts_mixer() {
        let event = to_scene_event(AppEvent::Mixer(MixerViewEvent::NavUp)).unwrap();
        assert_eq!(event, SceneEvent::Mixer(MixerViewEvent::NavUp));
    }

    #[test]
    fn to_scene_event_rejects_patch_commands() {
        let err = to_scene_event(AppEvent::Patch(PatchCommand::Create)).unwrap_err();
        assert!(err.contains("Patch"));
    }

    // ---- parse_scene ----

    #[test]
    fn parse_scene_round_trips_a_mixer_event() {
        let json = scene_json_with_events(&[AppEvent::Mixer(MixerViewEvent::NavUp)]);
        let scene = parse_scene(&json).expect("scene parses");
        assert_eq!(scene.name(), "test-scene");
        assert_eq!(scene.len(), 1);
        assert_eq!(
            scene.steps()[0].event(),
            &SceneEvent::Mixer(MixerViewEvent::NavUp)
        );
        assert_eq!(scene.steps()[0].render_blocks(), 2);
    }

    #[test]
    fn parse_scene_round_trips_a_midi_event() {
        let json = scene_json_with_events(&[AppEvent::Midi(sample_midi_event())]);
        let scene = parse_scene(&json).expect("scene parses");
        assert_eq!(
            scene.steps()[0].event(),
            &SceneEvent::Midi(sample_midi_event())
        );
    }

    #[test]
    fn parse_scene_preserves_step_order() {
        let json = scene_json_with_events(&[
            AppEvent::Mixer(MixerViewEvent::NavUp),
            AppEvent::Mixer(MixerViewEvent::NavDown),
        ]);
        let scene = parse_scene(&json).expect("scene parses");
        assert_eq!(
            scene.steps()[0].event(),
            &SceneEvent::Mixer(MixerViewEvent::NavUp)
        );
        assert_eq!(
            scene.steps()[1].event(),
            &SceneEvent::Mixer(MixerViewEvent::NavDown)
        );
    }

    #[test]
    fn parse_scene_rejects_patch_commands_with_step_index() {
        let json = scene_json_with_events(&[AppEvent::Patch(PatchCommand::Create)]);
        let err = parse_scene(&json).unwrap_err();
        assert!(err.contains("step 0"));
    }

    #[test]
    fn parse_scene_rejects_invalid_json() {
        let err = parse_scene("not json").unwrap_err();
        assert!(err.contains("failed to parse scene"));
    }

    // ---- load_scene ----

    #[test]
    fn load_scene_reads_a_file_from_disk() {
        let json = scene_json_with_events(&[AppEvent::Mixer(MixerViewEvent::NavUp)]);
        let mut path = std::env::temp_dir();
        path.push(format!("scene_run_test_{}.json", std::process::id()));
        fs::write(&path, json).expect("write temp scene file");

        let scene = load_scene(&path).expect("scene loads");
        assert_eq!(scene.len(), 1);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_scene_reports_missing_file() {
        let err = load_scene(Path::new("/does/not/exist/scene.json")).unwrap_err();
        assert!(err.contains("failed to read scene file"));
    }

    // ---- RenderStats / ToneBlockRenderer ----

    #[test]
    fn render_stats_starts_silent_with_no_blocks() {
        let stats = RenderStats::new();
        assert_eq!(stats.peak(), PeakLevel::SILENT);
        assert_eq!(stats.blocks_rendered(), 0);
    }

    #[test]
    fn tone_block_renderer_measures_a_real_nonzero_peak() {
        let stats = RenderStats::new();
        let mut renderer = ToneBlockRenderer::new(stats.clone());

        renderer.render_block();

        assert_eq!(stats.blocks_rendered(), 1);
        assert!(stats.peak().raw() > 0.0);
    }

    #[test]
    fn tone_block_renderer_counts_every_call() {
        let stats = RenderStats::new();
        let mut renderer = ToneBlockRenderer::new(stats.clone());

        renderer.render_block();
        renderer.render_block();
        renderer.render_block();

        assert_eq!(stats.blocks_rendered(), 3);
    }

    #[test]
    fn render_stats_peak_is_the_running_maximum() {
        let stats = RenderStats::new();
        stats.record_block_peak(0.2);
        stats.record_block_peak(0.9);
        stats.record_block_peak(0.5);
        assert!((stats.peak().raw() - 0.9).abs() < 1e-9);
    }

    // ---- end-to-end: running a full scene through SceneRunner ----

    #[test]
    fn running_a_scene_reports_all_events_applied_and_no_rejections() {
        let json = scene_json_with_events(&[
            AppEvent::Mixer(MixerViewEvent::NavUp),
            AppEvent::Mixer(MixerViewEvent::NavDown),
        ]);
        let scene = parse_scene(&json).expect("scene parses");

        let stats = RenderStats::new();
        let renderer = ToneBlockRenderer::new(stats.clone());
        let mut runner = SceneRunner::new(SerdeSnapshotCodec, renderer);
        let outcome = runner.run(&scene);

        let events_applied = outcome
            .steps
            .iter()
            .filter(|step| matches!(step.applied, AppStateEvent::Applied { .. }))
            .count();
        let rejections = outcome
            .steps
            .iter()
            .filter(|step| matches!(step.applied, AppStateEvent::Rejected { .. }))
            .count();

        assert_eq!(events_applied, 2);
        assert_eq!(rejections, 0);
        // Each step in this scene requests 2 render blocks (see
        // `scene_json_with_events`).
        assert_eq!(stats.blocks_rendered(), 4);
        assert!(stats.peak().raw() > 0.0);
    }

    // ---- end-to-end: MixerViewSnapshotSource reports real channel state ----

    #[test]
    fn mixer_solo_scene_reports_real_soloed_and_effective_mute_state() {
        // NavRight then NavDown x3 lands the cursor on channel 1 / Solo
        // (MixerView's row order is [Volume, Pan, Mute, Solo]);
        // ToggleFocusedParam then solos channel 1.
        let json = scene_json_with_events(&[
            AppEvent::Mixer(MixerViewEvent::NavRight),
            AppEvent::Mixer(MixerViewEvent::NavDown),
            AppEvent::Mixer(MixerViewEvent::NavDown),
            AppEvent::Mixer(MixerViewEvent::NavDown),
            AppEvent::Mixer(MixerViewEvent::ToggleFocusedParam),
        ]);
        let scene = parse_scene(&json).expect("scene parses");

        let stats = RenderStats::new();
        let renderer = ToneBlockRenderer::new(stats.clone());
        let snapshot_source = MixerViewSnapshotSource::from_scene(&scene);
        let mut runner = SceneRunner::with_collaborators(
            SerdeSnapshotCodec,
            renderer,
            StateProjector::new(),
            ParameterBridge::default(),
            snapshot_source,
        );

        let outcome = runner.run(&scene);
        let snapshot = outcome.final_snapshot;

        assert_eq!(snapshot.channels.len(), 16);
        assert!(snapshot.channels[1].soloed);
        assert!(!snapshot.channels[1].muted);
        assert!(snapshot.channels[0].muted);
        assert!(snapshot.channels[2].muted);
    }

    #[test]
    fn volume_edit_scene_reports_the_real_adjusted_volume() {
        // NavRight x2 moves to channel 2; EnterEditMode then NavDown
        // (coarse -5.0 dB) then NavLeft x2 (fine -0.5 dB each) yields a
        // net -6.0 dB change from the 0.0 dB default.
        let json = scene_json_with_events(&[
            AppEvent::Mixer(MixerViewEvent::NavRight),
            AppEvent::Mixer(MixerViewEvent::NavRight),
            AppEvent::Mixer(MixerViewEvent::EnterEditMode),
            AppEvent::Mixer(MixerViewEvent::NavDown),
            AppEvent::Mixer(MixerViewEvent::NavLeft),
            AppEvent::Mixer(MixerViewEvent::NavLeft),
        ]);
        let scene = parse_scene(&json).expect("scene parses");

        let stats = RenderStats::new();
        let renderer = ToneBlockRenderer::new(stats.clone());
        let snapshot_source = MixerViewSnapshotSource::from_scene(&scene);
        let mut runner = SceneRunner::with_collaborators(
            SerdeSnapshotCodec,
            renderer,
            StateProjector::new(),
            ParameterBridge::default(),
            snapshot_source,
        );

        let outcome = runner.run(&scene);
        let snapshot = outcome.final_snapshot;

        assert!((snapshot.channels[2].volume_db - (-6.0)).abs() < 1e-6);
    }
}
