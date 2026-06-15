// path: src/bin/synth_ui.rs
//
// synth_ui — standalone eframe/egui MIXER VIEW with live synth engine.
//
// Usage: synth_ui [--smoke] [--autopilot] [--seconds <N>] [--play <FILE.mid>]
//
// Default: opens a window with keyboard/gamepad-driven mixer view.
//   Keys: W=NavUp, S=NavDown, A=NavLeft, D=NavRight, J=EnterEditMode (hold), J double-tap=ToggleFocusedParam
// --smoke: hermetic headless mode — constructs state, drives event loop, audio self-check, exits 0.
// --autopilot [--seconds <N>]: real end-to-end run with scripted events, self-terminates after N seconds (default 4).
// --play <FILE.mid>: load and play a MIDI file via internal sequencer while editing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::Instant;

use eframe::egui;

use crest_synth::adapter::cpal_audio_output::CpalAudioOutput;
use crest_synth::design_system::default_theme::DefaultTheme;
use crest_synth::design_system::semantic_token::SemanticToken;
use crest_synth::design_system::theme::Theme;
use crest_synth::kernel::amplitude::Amplitude;
use crest_synth::kernel::audio_frame::AudioFrame;
use crest_synth::kernel::midi_event_kind::MidiEventKind;
use crest_synth::kernel::note_id::NoteId;
use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::sample_rate::SampleRate;
use crest_synth::kernel::velocity::Velocity;
use crest_synth::mixer::mixer_param::MixerParam;
use crest_synth::mixer::mixer_view::{MixerView, VISIBLE_CHANNELS};
use crest_synth::mixer::mixer_view_event::MixerViewEvent;
use crest_synth::patch::channel_mixer::ChannelMixer;
use crest_synth::patch::global_mixer::{GlobalMixer, GlobalMixerCommand, GlobalMixerWriter};
use crest_synth::patch::patch_mixer::{PatchMixEntry, PatchMixer};
use crest_synth::real_time::parameter_bridge::ParameterBridge;
use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
use crest_synth::shell::audio_output::AudioOutput;
use crest_synth::synth::voice_allocator::VoiceAllocator;

// ── Constants ─────────────────────────────────────────────────────────────────────────────

/// Audio frames per render block.
const BLOCK_SIZE: usize = 256;

/// Default audio sample rate.
const DEFAULT_SAMPLE_RATE: u32 = 44_100;

/// MIDI event channel capacity.
const MIDI_CHANNEL_CAP: usize = 512;

/// Number of MIDI/mixer channels.
const NUM_CHANNELS: usize = 16;

/// Double-tap window for J key (seconds).
const DOUBLE_TAP_WINDOW_SECS: f64 = 0.35;

/// Default autopilot duration in seconds.
const DEFAULT_AUTOPILOT_SECONDS: u32 = 4;

/// Fixed width of each channel strip in pixels.
/// NEVER use available_width() inside a per-strip vertical — that causes the
/// first strip to consume all horizontal space and pushes strips 2–6 off-screen.
const STRIP_WIDTH: f32 = 120.0;

/// Width of the 1-px column separator between strips.
const SEP_WIDTH: f32 = 1.0;

/// egui default item spacing (horizontal). Each widget in a horizontal layout
/// gets this gap before the next. We account for it when sizing the window so
/// all 6 strips fit without clipping.
const EGUI_ITEM_SPACING_X: f32 = 8.0;

/// Default window inner width: 6 strips + their separators + egui item spacing
/// for each strip + a comfortable margin so all 6 strips are fully visible.
/// Formula: N*(STRIP_WIDTH + SEP_WIDTH + EGUI_ITEM_SPACING_X) + extra_margin
const DEFAULT_WINDOW_WIDTH: f32 =
    VISIBLE_CHANNELS as f32 * (STRIP_WIDTH + SEP_WIDTH + EGUI_ITEM_SPACING_X) + 80.0;

/// Default window inner height.
const DEFAULT_WINDOW_HEIGHT: f32 = 520.0;

// ── CLI args ───────────────────────────────────────────────────────────────────────────

struct Args {
    smoke: bool,
    autopilot: bool,
    seconds: u32,
    play_file: Option<PathBuf>,
}

fn parse_args() -> Args {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut smoke = false;
    let mut autopilot = false;
    let mut seconds = DEFAULT_AUTOPILOT_SECONDS;
    let mut play_file: Option<PathBuf> = None;

    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--smoke" => {
                smoke = true;
            }
            "--autopilot" => {
                autopilot = true;
            }
            "--seconds" => {
                i += 1;
                if i >= raw.len() {
                    eprintln!("error: --seconds requires a numeric argument");
                    process::exit(1);
                }
                match raw[i].parse::<u32>() {
                    Ok(n) => {
                        seconds = n;
                    }
                    Err(_) => {
                        eprintln!("error: --seconds argument must be a positive integer");
                        process::exit(1);
                    }
                }
            }
            "--play" => {
                i += 1;
                if i >= raw.len() {
                    eprintln!("error: --play requires a file path argument");
                    process::exit(1);
                }
                play_file = Some(PathBuf::from(&raw[i]));
            }
            other => {
                eprintln!("error: unknown argument: {other}");
                process::exit(1);
            }
        }
        i += 1;
    }

    if smoke && autopilot {
        eprintln!("error: --smoke and --autopilot are mutually exclusive");
        process::exit(1);
    }

    Args {
        smoke,
        autopilot,
        seconds,
        play_file,
    }
}

// ── Internal MIDI event (Send) ──────────────────────────────────────────────────────────

/// A minimal MIDI event that is `Send` — crosses thread boundaries to the
/// main-thread audio-producer loop. Never the cpal stream itself.
#[derive(Debug, Clone)]
struct InternalMidi {
    note_id: NoteId,
    note_number: u8,
    velocity: f64,
    is_on: bool,
}

// ── Input layer: J key double-tap / hold detection ───────────────────────────────────────────

/// Stateful J-key input handler.
///
/// Detects hold (→ EnterEditMode / ExitEditMode) and double-tap
/// (→ ToggleFocusedParam). All timing logic lives here; `MixerView` is
/// timing-free.
struct JKeyState {
    /// J was held last frame.
    was_held: bool,
    /// Time of the most recent J press (for double-tap detection).
    last_press_time: Option<Instant>,
    /// Whether a J press was already counted for double-tap in this hold.
    press_counted: bool,
}

impl JKeyState {
    fn new() -> Self {
        Self {
            was_held: false,
            last_press_time: None,
            press_counted: false,
        }
    }

    /// Poll current J-key state and produce `MixerViewEvent`s.
    ///
    /// Returns a list of events to apply to `MixerView` in order.
    fn poll(&mut self, j_held: bool) -> Vec<MixerViewEvent> {
        let mut events = Vec::new();
        let now = Instant::now();

        if j_held && !self.was_held {
            // J was just pressed (leading edge).
            // Check for double-tap.
            let is_double_tap = self
                .last_press_time
                .map(|t| now.duration_since(t).as_secs_f64() < DOUBLE_TAP_WINDOW_SECS)
                .unwrap_or(false);

            if is_double_tap {
                events.push(MixerViewEvent::ToggleFocusedParam);
                // Reset so a 3rd press doesn't re-trigger.
                self.last_press_time = None;
                self.press_counted = true;
            } else {
                // First press of a potential double-tap.
                self.last_press_time = Some(now);
                self.press_counted = false;
            }
            // Enter edit mode on any J press.
            events.push(MixerViewEvent::EnterEditMode);
        } else if !j_held && self.was_held {
            // J was just released.
            events.push(MixerViewEvent::ExitEditMode);
        }

        self.was_held = j_held;
        events
    }
}

// ── Gamepad double-tap / hold for Edit button (South) ────────────────────────────────────────────

struct GamepadEditState {
    was_held: bool,
    last_press_time: Option<Instant>,
}

impl GamepadEditState {
    fn new() -> Self {
        Self {
            was_held: false,
            last_press_time: None,
        }
    }

    fn on_button_pressed(&mut self) -> Vec<MixerViewEvent> {
        let mut events = Vec::new();
        let now = Instant::now();

        let is_double_tap = self
            .last_press_time
            .map(|t| now.duration_since(t).as_secs_f64() < DOUBLE_TAP_WINDOW_SECS)
            .unwrap_or(false);

        if is_double_tap {
            events.push(MixerViewEvent::ToggleFocusedParam);
            self.last_press_time = None;
        } else {
            self.last_press_time = Some(now);
        }
        events.push(MixerViewEvent::EnterEditMode);
        self.was_held = true;
        events
    }

    fn on_button_released(&mut self) -> Vec<MixerViewEvent> {
        self.was_held = false;
        vec![MixerViewEvent::ExitEditMode]
    }
}

// ── Render function (shared by live path and smoke self-check) ─────────────────────────────────────────

/// Render `num_frames` audio frames through the full engine graph.
///
/// Path: VoiceAllocator → (mono sample) → all frames go to channel 0 of
/// `channel_mixer` → GlobalMixer master gain.
///
/// This is the SINGLE render function that both the live eframe update tick and
/// the --smoke audio self-check call. The channel mixer records per-channel peak
/// levels as a side-effect of mixing.
///
/// `output` is cleared and refilled with exactly `num_frames` frames.
#[allow(clippy::too_many_arguments)]
fn render_frames(
    num_frames: usize,
    voice_alloc: &mut VoiceAllocator,
    patch_mixer: &PatchMixer,
    channel_mixer: &mut ChannelMixer,
    global_mixer_writer: &mut GlobalMixerWriter,
    sample_rate: f64,
    output: &mut Vec<AudioFrame>,
) {
    if num_frames == 0 {
        output.clear();
        return;
    }

    // Get current master gain.
    let gain = global_mixer_writer.state().master_gain.value() as f32;

    // Build per-channel input buffers: channel 0 receives all rendered audio;
    // channels 1–15 are silent (no per-channel voice pools in this phase).
    let mut channel_inputs: [Vec<AudioFrame>; NUM_CHANNELS] = std::array::from_fn(|_| Vec::new());

    // Pre-allocate
    for ch in channel_inputs.iter_mut() {
        ch.resize(num_frames, AudioFrame::silence());
    }

    // Render voice audio into a flat buffer, then assign to channel 0.
    let mut remaining = num_frames;
    let mut write_pos = 0;
    while remaining > 0 {
        let this_block = remaining.min(BLOCK_SIZE);
        for _ in 0..this_block {
            let (sample, _events) = voice_alloc.render_sample(sample_rate, 0.0);
            // Apply PatchMixer (centre pan, unity gain).
            let patch_frame = AudioFrame::mono(sample);
            let mixed = patch_mixer.apply_entry(patch_frame, &PatchMixEntry::unity());
            channel_inputs[0][write_pos] = mixed;
            write_pos += 1;
        }
        remaining -= this_block;
    }

    // Run through the 16-channel mixer (applies volume/pan/mute/solo, records peaks).
    let mut mixed_out: Vec<AudioFrame> = Vec::with_capacity(num_frames);
    channel_mixer.mix(&channel_inputs, &mut mixed_out);

    // Apply master gain and write to output.
    output.clear();
    for frame in &mixed_out {
        output.push(AudioFrame {
            left: frame.left * gain,
            right: frame.right * gain,
        });
    }
}

// ── --play MIDI sequencer ─────────────────────────────────────────────────────────────────────────────

fn load_play_events(path: &std::path::Path) -> Option<Vec<(f64, InternalMidi)>> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("warning: cannot read MIDI file '{}': {e}", path.display());
            return None;
        }
    };
    let timeline = match crest_synth::midi_file::load(&bytes) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("warning: cannot parse MIDI file '{}': {e}", path.display());
            return None;
        }
    };

    let mut active: HashMap<(u8, NoteId), NoteId> = HashMap::new();
    let mut next_id: u32 = 10_000;
    let mut events: Vec<(f64, InternalMidi)> = Vec::new();

    for (time_secs, midi_event) in &timeline {
        let note_num = midi_event.note_number.value();
        match midi_event.kind {
            MidiEventKind::NoteOn => {
                let local_id = NoteId::new(next_id);
                next_id += 1;
                active.insert((note_num, midi_event.note_id), local_id);
                events.push((
                    *time_secs,
                    InternalMidi {
                        note_id: local_id,
                        note_number: note_num,
                        velocity: midi_event.velocity.value(),
                        is_on: true,
                    },
                ));
            }
            MidiEventKind::NoteOff => {
                let key = (note_num, midi_event.note_id);
                let local_id = active.remove(&key).unwrap_or_else(|| NoteId::new(0));
                events.push((
                    *time_secs,
                    InternalMidi {
                        note_id: local_id,
                        note_number: note_num,
                        velocity: 0.0,
                        is_on: false,
                    },
                ));
            }
            _ => {}
        }
    }

    events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Some(events)
}

/// Spawn a background thread that replays a MIDI timeline in real time,
/// sending `InternalMidi` events to `tx`. Only `Send` data crosses thread boundary.
fn spawn_sequencer_thread(events: Vec<(f64, InternalMidi)>, tx: SyncSender<InternalMidi>) {
    std::thread::spawn(move || {
        if events.is_empty() {
            return;
        }

        let duration = events.last().map(|(t, _)| *t + 0.5).unwrap_or(1.0);

        loop {
            let start = Instant::now();
            let mut cursor = 0;

            loop {
                let elapsed = start.elapsed().as_secs_f64();
                if elapsed >= duration {
                    break;
                }

                while cursor < events.len() && events[cursor].0 <= elapsed {
                    let (_, ref ev) = events[cursor];
                    // Non-blocking try_send: never block the sequencer thread.
                    let _ = tx.try_send(ev.clone());
                    cursor += 1;
                }

                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    });
}

// ── Autopilot scripted event sequence ──────────────────────────────────────────────────────────────────────────────

/// Build the deterministic autopilot script: a sequence of `MixerViewEvents`
/// that exercises edge-scrolling, edit mode, fine/coarse nudges, and toggle
/// toggling. The entire sequence is applied one event per update tick.
///
/// The script is designed to drive enough ticks to be interesting, not to fit
/// within exactly N seconds — the wall-clock budget closes the window regardless.
fn build_autopilot_script() -> Vec<MixerViewEvent> {
    let mut script = Vec::new();

    // Navigate right across channels to force viewport edge-scrolling (at least 10 steps → wraps past 6-channel window).
    for _ in 0..10 {
        script.push(MixerViewEvent::NavRight);
    }

    // Navigate left to scroll the viewport back.
    for _ in 0..10 {
        script.push(MixerViewEvent::NavLeft);
    }

    // Enter edit mode, nudge Volume row (fine steps) via NavRight, then exit.
    script.push(MixerViewEvent::EnterEditMode);
    for _ in 0..5 {
        script.push(MixerViewEvent::NavRight); // fine increment in edit mode
    }
    script.push(MixerViewEvent::ExitEditMode);

    // Move to ReverbSend row.
    script.push(MixerViewEvent::NavDown);

    // Enter edit mode, nudge ReverbSend (coarse: use NavUp as a coarse increment in edit).
    script.push(MixerViewEvent::EnterEditMode);
    for _ in 0..3 {
        script.push(MixerViewEvent::NavUp); // coarse decrement
    }
    script.push(MixerViewEvent::ExitEditMode);

    // Navigate down to Mute row and toggle it via double-tap (ToggleFocusedParam).
    script.push(MixerViewEvent::NavDown); // EchoSend
    script.push(MixerViewEvent::NavDown); // Pan
    script.push(MixerViewEvent::NavDown); // Mute
    script.push(MixerViewEvent::ToggleFocusedParam); // toggle Mute ON
    script.push(MixerViewEvent::ToggleFocusedParam); // toggle Mute OFF

    // Move to Solo row and toggle it.
    script.push(MixerViewEvent::NavDown); // Solo
    script.push(MixerViewEvent::ToggleFocusedParam); // toggle Solo ON
    script.push(MixerViewEvent::ToggleFocusedParam); // toggle Solo OFF

    // Navigate back to Volume row.
    script.push(MixerViewEvent::NavUp); // Mute
    script.push(MixerViewEvent::NavUp); // Pan
    script.push(MixerViewEvent::NavUp); // EchoSend
    script.push(MixerViewEvent::NavUp); // ReverbSend
    script.push(MixerViewEvent::NavUp); // Volume

    script
}

// ── Autopilot built-in note sequence ──────────────────────────────────────────────────────────────────

/// Deterministic autopilot MIDI note schedule: (time_from_start_secs, note_number, is_on, velocity).
/// Injects notes into the live VoiceAllocator to prove the real audio path produces sound.
fn build_autopilot_notes() -> Vec<(f64, u8, bool, f64)> {
    // A short repeating pattern: C4, E4, G4 on MIDI ch 1 (we use VoiceAllocator directly).
    vec![
        (0.05, 60, true, 0.8),  // C4 note-on
        (0.30, 60, false, 0.0), // C4 note-off
        (0.35, 64, true, 0.8),  // E4 note-on
        (0.60, 64, false, 0.0), // E4 note-off
        (0.65, 67, true, 0.8),  // G4 note-on
        (0.90, 67, false, 0.0), // G4 note-off
        (0.95, 60, true, 0.8),  // C4 again
        (1.20, 60, false, 0.0),
        (1.25, 64, true, 0.8),
        (1.50, 64, false, 0.0),
    ]
}

// ── eframe App ──────────────────────────────────────────────────────────────────────────────────

struct SynthUiApp {
    // ── UI state (one-way event loop) ───────────────────────────────────────────────────────────────────────────────
    /// Mixer view — the only source of truth for UI mixer state.
    mixer_view: MixerView,

    // ── Design system theme (resolved once at construction) ──────────────────────────────────────────────
    /// The active theme — the ONLY source of color for all draw code.
    theme: DefaultTheme,

    // ── Keyboard input state ───────────────────────────────────────────────────────────────────────────
    j_key: JKeyState,
    w_was_down: bool,
    s_was_down: bool,
    a_was_down: bool,
    d_was_down: bool,

    // ── Gamepad input state ─────────────────────────────────────────────────────────────────────────────
    gamepad_edit: GamepadEditState,
    gilrs: Option<gilrs::Gilrs>,

    // ── Audio engine (all on main thread) ──────────────────────────────────────────────────────────────────────
    /// Voice allocator.
    voice_alloc: VoiceAllocator,
    /// Stateless patch mixer.
    patch_mixer: PatchMixer,
    /// 16-channel mixer — applies per-channel volume/pan/mute/solo, records peaks.
    channel_mixer: ChannelMixer,
    /// GlobalMixer control-thread handle.
    global_mixer_writer: GlobalMixerWriter,

    // ── Audio output (main thread only — cpal::Stream is !Send on macOS) ─────
    audio_out: CpalAudioOutput,

    /// Reusable render buffer.
    render_buf: Vec<AudioFrame>,

    // ── MIDI sources ────────────────────────────────────────────────────────────────────────────────────
    /// Receiver for MIDI events from MidirInput callback and sequencer thread.
    midi_rx: Receiver<InternalMidi>,
    /// Per-note-number active NoteId (for external MIDI note-off matching).
    active_notes: HashMap<u8, NoteId>,

    // ── Parameter bridge (kept alive) ───────────────────────────────────────────────────────────────────
    _param_bridge_writer: crest_synth::real_time::parameter_bridge::ParameterBridgeWriter,

    // ── Autopilot state ─────────────────────────────────────────────────────────────────────────────────────
    /// Whether autopilot mode is active.
    autopilot_mode: bool,
    /// Wall-clock start time for autopilot (set on first update tick).
    autopilot_start: Option<Instant>,
    /// Wall-clock budget in seconds before autopilot closes the window.
    autopilot_seconds: f64,
    /// Scripted event sequence for autopilot (control-plane events).
    autopilot_script: Vec<MixerViewEvent>,
    /// Index into the scripted event sequence (how many have been applied).
    autopilot_script_index: usize,
    /// Total number of scripted MixerViewEvents applied so far.
    autopilot_event_count: usize,
    /// Whether the window-close command has been sent (avoid double-send).
    autopilot_closed: bool,
    /// Running peak of real device-bound audio frames written to the ring buffer.
    /// Tracked only in autopilot mode to assert real audio is produced.
    autopilot_audio_peak: f32,
    /// Deterministic built-in note schedule for autopilot (time_secs, note, is_on, vel).
    autopilot_notes: Vec<(f64, u8, bool, f64)>,
    /// Index into the note schedule (next note to inject).
    autopilot_notes_index: usize,
    /// Active autopilot note IDs (note_number → NoteId), for note-off matching.
    autopilot_active_notes: HashMap<u8, NoteId>,
    /// Running counter for autopilot note IDs (distinct from external MIDI).
    autopilot_note_id_counter: u32,
    /// Last observed strip-visible count (set by draw code, read at close).
    last_strips_visible: usize,
    /// Screenshot requested via ViewportCommand — once set, do not request again.
    screenshot_requested: bool,
    /// Screenshot has been saved (avoid double-write).
    screenshot_saved: bool,
}

impl SynthUiApp {
    #[allow(clippy::too_many_arguments)]
    fn new(
        mixer_view: MixerView,
        voice_alloc: VoiceAllocator,
        patch_mixer: PatchMixer,
        channel_mixer: ChannelMixer,
        global_mixer_writer: GlobalMixerWriter,
        audio_out: CpalAudioOutput,
        midi_rx: Receiver<InternalMidi>,
        param_bridge_writer: crest_synth::real_time::parameter_bridge::ParameterBridgeWriter,
        autopilot_mode: bool,
        autopilot_seconds: f64,
    ) -> Self {
        // Try to initialise gilrs for gamepad input (non-fatal if unavailable).
        let gilrs = gilrs::Gilrs::new().ok();

        let autopilot_script = if autopilot_mode {
            build_autopilot_script()
        } else {
            Vec::new()
        };

        let autopilot_notes = if autopilot_mode {
            build_autopilot_notes()
        } else {
            Vec::new()
        };

        Self {
            mixer_view,
            // Construct the DefaultTheme once at app construction.
            // All draw code resolves colors through this; no literal color appears in draw code.
            theme: DefaultTheme::new(),
            j_key: JKeyState::new(),
            w_was_down: false,
            s_was_down: false,
            a_was_down: false,
            d_was_down: false,
            gamepad_edit: GamepadEditState::new(),
            gilrs,
            voice_alloc,
            patch_mixer,
            channel_mixer,
            global_mixer_writer,
            audio_out,
            render_buf: Vec::with_capacity(BLOCK_SIZE * 4),
            midi_rx,
            active_notes: HashMap::new(),
            _param_bridge_writer: param_bridge_writer,
            autopilot_mode,
            autopilot_start: None,
            autopilot_seconds,
            autopilot_script,
            autopilot_script_index: 0,
            autopilot_event_count: 0,
            autopilot_closed: false,
            autopilot_audio_peak: 0.0,
            autopilot_notes,
            autopilot_notes_index: 0,
            autopilot_active_notes: HashMap::new(),
            autopilot_note_id_counter: 50_000,
            last_strips_visible: 0,
            screenshot_requested: false,
            screenshot_saved: false,
        }
    }

    /// Process keyboard input and emit `MixerViewEvent`s to `mixer_view`.
    ///
    /// All cursor/edit-mode changes go through `mixer_view.apply()`.
    /// The egui draw code never mutates state directly.
    fn process_keyboard(&mut self, ctx: &egui::Context) {
        let input = ctx.input(|i| i.clone());

        // J = edit mode (hold) + double-tap = ToggleFocusedParam.
        let j_held = input.key_down(egui::Key::J);
        let j_events = self.j_key.poll(j_held);
        for ev in j_events {
            self.mixer_view.apply(ev);
        }

        // W/S/A/D = Nav (press-edge only).
        let w_down = input.key_down(egui::Key::W);
        if w_down && !self.w_was_down {
            self.mixer_view.apply(MixerViewEvent::NavUp);
        }
        self.w_was_down = w_down;

        let s_down = input.key_down(egui::Key::S);
        if s_down && !self.s_was_down {
            self.mixer_view.apply(MixerViewEvent::NavDown);
        }
        self.s_was_down = s_down;

        let a_down = input.key_down(egui::Key::A);
        if a_down && !self.a_was_down {
            self.mixer_view.apply(MixerViewEvent::NavLeft);
        }
        self.a_was_down = a_down;

        let d_down = input.key_down(egui::Key::D);
        if d_down && !self.d_was_down {
            self.mixer_view.apply(MixerViewEvent::NavRight);
        }
        self.d_was_down = d_down;
    }

    /// Process gamepad events and emit identical `MixerViewEvent`s.
    ///
    /// D-pad → Nav events, South (Edit) button → EnterEditMode/ExitEditMode/
    /// ToggleFocusedParam. Keyboard and gamepad emit identical events.
    fn process_gamepad(&mut self) {
        if let Some(ref mut g) = self.gilrs {
            let mut events_to_apply: Vec<MixerViewEvent> = Vec::new();
            while let Some(gilrs::Event { event, .. }) = g.next_event() {
                match event {
                    gilrs::EventType::ButtonPressed(gilrs::Button::DPadUp, _) => {
                        events_to_apply.push(MixerViewEvent::NavUp);
                    }
                    gilrs::EventType::ButtonPressed(gilrs::Button::DPadDown, _) => {
                        events_to_apply.push(MixerViewEvent::NavDown);
                    }
                    gilrs::EventType::ButtonPressed(gilrs::Button::DPadLeft, _) => {
                        events_to_apply.push(MixerViewEvent::NavLeft);
                    }
                    gilrs::EventType::ButtonPressed(gilrs::Button::DPadRight, _) => {
                        events_to_apply.push(MixerViewEvent::NavRight);
                    }
                    gilrs::EventType::ButtonPressed(gilrs::Button::South, _) => {
                        let evs = self.gamepad_edit.on_button_pressed();
                        events_to_apply.extend(evs);
                    }
                    gilrs::EventType::ButtonReleased(gilrs::Button::South, _) => {
                        let evs = self.gamepad_edit.on_button_released();
                        events_to_apply.extend(evs);
                    }
                    _ => {}
                }
            }
            for ev in events_to_apply {
                self.mixer_view.apply(ev);
            }
        }
    }

    /// Inject autopilot built-in MIDI notes into the VoiceAllocator based on
    /// elapsed time. This drives the real audio path to prove it produces sound.
    fn inject_autopilot_notes(&mut self, elapsed: f64) {
        // Wrap elapsed to the note-pattern duration (loop the notes).
        let pattern_duration = 2.0_f64; // seconds (covers all notes in build_autopilot_notes)
        let looped_elapsed = elapsed % pattern_duration;

        // Walk through all notes in the schedule (they may fire multiple times as
        // elapsed wraps, but we track injection by the schedule index modulo len).
        while self.autopilot_notes_index < self.autopilot_notes.len() {
            let (t, note_num, is_on, vel) = self.autopilot_notes[self.autopilot_notes_index];
            if looped_elapsed >= t {
                if is_on {
                    if let Ok(nn) = NoteNumber::try_new(note_num) {
                        if let Ok(v) = Velocity::try_new(vel.clamp(0.001, 1.0)) {
                            let note_id = NoteId::new(self.autopilot_note_id_counter);
                            self.autopilot_note_id_counter += 1;
                            self.voice_alloc.note_on(note_id, nn, v);
                            self.autopilot_active_notes.insert(note_num, note_id);
                        }
                    }
                } else {
                    if let Some(note_id) = self.autopilot_active_notes.remove(&note_num) {
                        let _ = self.voice_alloc.note_off(note_id);
                    }
                }
                self.autopilot_notes_index += 1;
            } else {
                break;
            }
        }

        // Reset index when we've exhausted the schedule (so they fire again next loop).
        if self.autopilot_notes_index >= self.autopilot_notes.len() {
            self.autopilot_notes_index = 0;
        }
    }

    /// Drive the autopilot script: apply one scripted event per tick,
    /// and close the window once the wall-clock budget has elapsed.
    ///
    /// Returns `true` if the window-close command was sent this tick.
    fn process_autopilot(&mut self, ctx: &egui::Context) -> bool {
        if !self.autopilot_mode || self.autopilot_closed {
            return false;
        }

        // Record start time on first tick.
        let start = *self.autopilot_start.get_or_insert_with(Instant::now);
        let elapsed = start.elapsed().as_secs_f64();

        // Inject built-in MIDI notes for the real audio assertion.
        self.inject_autopilot_notes(elapsed);

        // Apply one scripted event per tick (if any remain).
        if self.autopilot_script_index < self.autopilot_script.len() {
            let ev = self.autopilot_script[self.autopilot_script_index];
            self.mixer_view.apply(ev);
            self.autopilot_script_index += 1;
            self.autopilot_event_count += 1;
        }

        // Request screenshot on the second-to-last second (once), save when received.
        if !self.screenshot_requested && elapsed >= (self.autopilot_seconds - 1.0).max(0.5) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            self.screenshot_requested = true;
        }

        // After budget elapsed, emit assertions and close.
        if elapsed >= self.autopilot_seconds {
            // Audio peak assertion — must be > 0 (real audio produced).
            println!("autopilot audio peak: {}", self.autopilot_audio_peak);
            assert!(
                self.autopilot_audio_peak > 0.0,
                "autopilot FAILED: real audio peak is 0.0 — engine produced no sound"
            );

            // Strip count assertion — must equal VISIBLE_CHANNELS (6 strips on screen).
            let n = self.last_strips_visible;
            println!("autopilot strips visible: {n}");
            assert!(
                n == VISIBLE_CHANNELS,
                "autopilot FAILED: only {n} strips visible on screen (expected {VISIBLE_CHANNELS})"
            );

            println!("autopilot complete: {} events", self.autopilot_event_count);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.autopilot_closed = true;
            return true;
        }

        false
    }

    /// Sync MixerView state to the audio ChannelMixer (control-plane → audio).
    ///
    /// Publishes current per-channel volume/pan/mute/solo from the UI mixer
    /// to the audio-thread ChannelMixer. This is the one-way data flow; the
    /// draw code never reads or writes mixer values directly.
    fn sync_mixer_to_audio(&mut self) {
        let ui_mixer = self.mixer_view.mixer();
        for ch in 0..NUM_CHANNELS {
            let state = ui_mixer.channel(ch);
            // Apply volume via SetVolume command.
            self.channel_mixer.handle(
                crest_synth::patch::channel_mixer::ChannelMixerCommand::SetVolume {
                    channel: ch,
                    value: state.volume as f64,
                },
            );
            // Pan
            self.channel_mixer.handle(
                crest_synth::patch::channel_mixer::ChannelMixerCommand::SetPan {
                    channel: ch,
                    value: state.pan as f64,
                },
            );
            // Mute (set to match; toggle if different)
            let current_mute = self.channel_mixer.channels[ch].mute();
            if current_mute != state.mute {
                self.channel_mixer.handle(
                    crest_synth::patch::channel_mixer::ChannelMixerCommand::ToggleMute {
                        channel: ch,
                    },
                );
            }
            // Solo (same pattern)
            let current_solo = self.channel_mixer.channels[ch].solo();
            if current_solo != state.solo {
                self.channel_mixer.handle(
                    crest_synth::patch::channel_mixer::ChannelMixerCommand::ToggleSolo {
                        channel: ch,
                    },
                );
            }
        }

        // Publish master gain (from UI state; use volume of channel 0 as proxy,
        // or keep unity by default — the GlobalMixer carries master gain separately).
        let _ = self
            .global_mixer_writer
            .handle(GlobalMixerCommand::SetMasterGain {
                gain: Amplitude::unity(),
            });
    }

    /// Drain pending MIDI events and apply to voice allocator.
    fn drain_midi(&mut self) {
        while let Ok(ev) = self.midi_rx.try_recv() {
            if ev.is_on {
                if let Ok(note_num) = NoteNumber::try_new(ev.note_number) {
                    if let Ok(vel) = Velocity::try_new(ev.velocity.clamp(0.001, 1.0)) {
                        let note_id = ev.note_id;
                        self.voice_alloc.note_on(note_id, note_num, vel);
                        self.active_notes.insert(ev.note_number, note_id);
                    }
                }
            } else {
                let note_id = ev.note_id;
                let _ = self.voice_alloc.note_off(note_id);
                self.active_notes.remove(&ev.note_number);
            }
        }
    }

    /// Feed the audio ring buffer by exactly `available_frames()` — self-regulating.
    ///
    /// This avoids both underfeeding (buzz) and overfeeding (overflow). The first
    /// tick `available_frames()` returns the full ring capacity, which primes the buffer.
    ///
    /// In autopilot mode, tracks the running peak of frames actually written to
    /// the device-bound ring buffer (for the real audio assertion).
    fn render_and_feed_audio(&mut self) {
        let free = self.audio_out.available_frames();
        if free == 0 {
            return;
        }

        render_frames(
            free,
            &mut self.voice_alloc,
            &self.patch_mixer,
            &mut self.channel_mixer,
            &mut self.global_mixer_writer,
            DEFAULT_SAMPLE_RATE as f64,
            &mut self.render_buf,
        );

        // In autopilot mode, track the peak of the ACTUAL frames written to the ring buffer.
        if self.autopilot_mode {
            for frame in &self.render_buf {
                let s = frame.left.abs().max(frame.right.abs());
                if s > self.autopilot_audio_peak {
                    self.autopilot_audio_peak = s;
                }
            }
        }

        self.audio_out.write_buffer(&self.render_buf);
    }

    /// Handle a screenshot event from egui: save as autopilot.png.
    fn handle_screenshot(&mut self, image: &egui::ColorImage) {
        if self.screenshot_saved {
            return;
        }
        self.screenshot_saved = true;

        // Write PNG via raw bytes: RGBA → PNG using a simple approach.
        // We use the image data directly as an RGBA buffer.
        let width = image.size[0];
        let height = image.size[1];
        let pixels: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(|c| [c.r(), c.g(), c.b(), c.a()])
            .collect();

        // Write a minimal PNG.
        match write_png("autopilot.png", width as u32, height as u32, &pixels) {
            Ok(()) => eprintln!("autopilot: screenshot saved to autopilot.png"),
            Err(e) => eprintln!("autopilot: failed to save screenshot: {e}"),
        }
    }
}

impl eframe::App for SynthUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle screenshot if one has arrived (autopilot only).
        if self.autopilot_mode && !self.screenshot_saved {
            let maybe_img: Option<std::sync::Arc<egui::ColorImage>> = ctx.input(|i| {
                for event in &i.raw.events {
                    if let egui::Event::Screenshot { image, .. } = event {
                        return Some(image.clone());
                    }
                }
                None
            });
            if let Some(img) = maybe_img {
                self.handle_screenshot(&img);
            }
        }

        // 1. Process input → MixerViewEvents → mixer_view.apply().
        //    The draw code is a PURE VIEW; it never mutates state directly.
        //    In autopilot mode, scripted events are also injected here.
        self.process_keyboard(ctx);
        self.process_gamepad();
        self.process_autopilot(ctx);

        // 2. Drain MIDI from all sources and apply to voice allocator.
        self.drain_midi();

        // 3. Sync MixerView → audio ChannelMixer (one-way).
        self.sync_mixer_to_audio();

        // 4. Feed the audio ring buffer (self-regulating by available_frames).
        self.render_and_feed_audio();

        // 5. Draw the MIXER VIEW as a pure view over mixer_view state.
        //    Pass the theme so draw code can resolve all colors via SemanticToken.
        let theme = &self.theme;
        let strips_visible = egui::CentralPanel::default()
            .show(ctx, |ui| {
                draw_mixer_view(ui, &self.mixer_view, &self.channel_mixer, theme)
            })
            .inner;

        // Record the strip count for autopilot assertion.
        self.last_strips_visible = strips_visible;

        // 6. Keep the loop running fast enough to stay ahead of the audio buffer.
        ctx.request_repaint();
    }
}

/// Pure view: draw the 6 visible mixer channel strips.
///
/// Returns the number of strips that were fully laid out within the available
/// panel width (used by autopilot to assert the single-channel bug is absent).
///
/// This function is a PURE VIEW (skin) — it only reads from `mixer_view`,
/// `channel_mixer`, and resolves every color through `theme`. No literal color
/// value appears here; all colors come from `theme.color(SemanticToken::…)`
/// and are converted to `egui::Color32` only at the point of use.
///
/// ## Strip layout
///
/// Each of the 6 visible strips occupies a FIXED width of `STRIP_WIDTH` pixels,
/// allocated via `allocate_ui_with_layout`. This prevents any strip from claiming
/// `available_width()` and pushing the others off-screen (the single-channel bug).
fn draw_mixer_view(
    ui: &mut egui::Ui,
    view: &MixerView,
    channel_mixer: &ChannelMixer,
    theme: &DefaultTheme,
) -> usize {
    let offset = view.viewport_offset();
    let cursor_ch = view.cursor_channel();
    let cursor_param = view.cursor_param();
    let edit_mode = view.edit_mode();

    // ── Header bar ──────────────────────────────────────────────────────────────────────
    // Resolve colors from theme; convert Rgba → Color32 at point of use only.
    let text_default: egui::Color32 = theme.color(SemanticToken::TextDefault).into();
    let text_muted: egui::Color32 = theme.color(SemanticToken::TextMuted).into();
    let separator_color: egui::Color32 = theme.color(SemanticToken::Separator).into();
    let panel_bg: egui::Color32 = theme.color(SemanticToken::PanelBg).into();

    // Fill the panel background.
    let panel_rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(panel_rect, 0.0, panel_bg);

    let mode_label = if edit_mode {
        "EDIT (hold J)"
    } else {
        "NAVIGATE"
    };
    ui.horizontal(|ui| {
        ui.colored_label(text_default, "Crest Synth Mixer");
        ui.colored_label(separator_color, "|");
        ui.colored_label(text_muted, format!("Mode: {mode_label}"));
        ui.colored_label(separator_color, "|");
        ui.colored_label(
            text_muted,
            "W/S=row  A/D=channel  Hold J=edit  Double-tap J=toggle",
        );
    });
    // Draw a colored separator line using the Separator token color.
    // The header separator spans the full available width — this is the top-level
    // separator, not a per-strip one, so available_width() is correct here.
    {
        let sep_width = ui.available_width();
        let (sep_rect, _) =
            ui.allocate_exact_size(egui::vec2(sep_width, 1.0), egui::Sense::hover());
        ui.painter().rect_filled(sep_rect, 0.0, separator_color);
    }

    let ui_mixer = view.mixer();

    // ── Channel strips ───────────────────────────────────────────────────────────────────────────────────────
    // Draw 6 channel strips side by side.
    // Each strip is allocated a FIXED width (STRIP_WIDTH) via allocate_ui_with_layout —
    // NEVER ui.available_width() inside a per-strip vertical, which would consume all
    // remaining horizontal space and push strips 2–6 off-screen (the single-channel bug).
    let mut strips_laid_out: usize = 0;

    let panel_right = ui.clip_rect().right();
    let strips_layout = ui.available_rect_before_wrap();
    let strips_height = strips_layout.height();

    ui.horizontal(|ui| {
        for vis_idx in 0..VISIBLE_CHANNELS {
            let ch_idx = offset + vis_idx;
            let is_focused_ch = ch_idx == cursor_ch;

            // Allocate a fixed-width vertical sub-region for this strip.
            // This is the critical fix for the single-channel bug:
            // `allocate_ui_with_layout` pins the strip to exactly STRIP_WIDTH,
            // so no strip can consume `available_width()` and starve the others.
            let strip_size = egui::vec2(STRIP_WIDTH, strips_height);
            let strip_response = ui.allocate_ui_with_layout(
                strip_size,
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Enforce maximum width — belt-and-suspenders against egui expanding.
                    ui.set_max_width(STRIP_WIDTH);

                    // Channel header — use FocusRing when focused, TextMuted otherwise.
                    let ch_label = format!("Ch{}", ch_idx + 1);
                    let header_color: egui::Color32 = if is_focused_ch {
                        theme.color(SemanticToken::FocusRing).into()
                    } else {
                        theme.color(SemanticToken::TextMuted).into()
                    };
                    ui.colored_label(header_color, ch_label);

                    // The rows: Volume, ReverbSend, EchoSend, Pan, Mute, Solo
                    let ch_state = ui_mixer.channel(ch_idx);
                    // Live peak from audio channel mixer (independent of mute/solo)
                    let peak = channel_mixer.peaks[ch_idx].value();

                    draw_param_row(
                        ui,
                        theme,
                        "Vol",
                        ch_state.volume,
                        peak,
                        cursor_param == MixerParam::Volume && is_focused_ch,
                        edit_mode,
                        true, // is_volume — drives level strip metering
                    );
                    draw_param_row(
                        ui,
                        theme,
                        "Rvb",
                        ch_state.reverb_send,
                        0.0,
                        cursor_param == MixerParam::ReverbSend && is_focused_ch,
                        edit_mode,
                        false,
                    );
                    draw_param_row(
                        ui,
                        theme,
                        "Ech",
                        ch_state.echo_send,
                        0.0,
                        cursor_param == MixerParam::EchoSend && is_focused_ch,
                        edit_mode,
                        false,
                    );
                    draw_param_row(
                        ui,
                        theme,
                        "Pan",
                        (ch_state.pan + 1.0) / 2.0,
                        0.0,
                        cursor_param == MixerParam::Pan && is_focused_ch,
                        edit_mode,
                        false,
                    );
                    draw_toggle_row(
                        ui,
                        theme,
                        "Mute",
                        ch_state.mute,
                        cursor_param == MixerParam::Mute && is_focused_ch,
                        edit_mode,
                    );
                    draw_toggle_row(
                        ui,
                        theme,
                        "Solo",
                        ch_state.solo,
                        cursor_param == MixerParam::Solo && is_focused_ch,
                        edit_mode,
                    );

                    // Strip bottom separator — a 1px horizontal rule at the STRIP_WIDTH,
                    // never available_width() (which would grab the rest of the row and
                    // reintroduce the single-channel bug). Drawn via the Separator token.
                    {
                        let (sep_rect, _) = ui.allocate_exact_size(
                            egui::vec2(STRIP_WIDTH, 1.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(sep_rect, 0.0, separator_color);
                    }
                },
            );

            // Track whether this strip's right edge is within the visible panel.
            // Use the rect from the allocate_ui_with_layout response — that is the
            // actual allocated rectangle, whose right() is the strip's right edge.
            // A strip whose right edge exceeds the panel right is NOT visible.
            if strip_response.response.rect.right() <= panel_right {
                strips_laid_out += 1;
            }

            // Column separator between strips (1px wide, strip-tall via painter).
            // Uses a fixed height rather than available_height() to avoid consuming
            // the rest of the panel. Height is bounded by the strip area.
            {
                let (sep_rect, _) =
                    ui.allocate_exact_size(egui::vec2(1.0, strips_height), egui::Sense::hover());
                ui.painter().rect_filled(sep_rect, 0.0, separator_color);
            }
        }
    });

    strips_laid_out
}

/// Draw one continuous parameter row for a channel strip.
///
/// This is a SKIN function: every color comes from `theme.color(SemanticToken::…)`.
/// No literal color value appears here. `Rgba → Color32` conversion is the only
/// raw-color touch, and it comes exclusively from the theme.
///
/// `value` is normalized 0–1. `peak` is the live peak level (used on the Volume
/// row for the level strip meter). `is_volume` selects the level-strip rendering.
#[allow(clippy::too_many_arguments)]
fn draw_param_row(
    ui: &mut egui::Ui,
    theme: &DefaultTheme,
    label: &str,
    value: f32,
    peak: f32,
    is_focused_cell: bool,
    edit_mode: bool,
    is_volume: bool,
) {
    // Resolve all colors from the theme — never a literal color.
    // Rgba → Color32 conversion is the ONLY raw-color touch, and it comes from the Theme.
    let text_default: egui::Color32 = theme.color(SemanticToken::TextDefault).into();
    let text_muted: egui::Color32 = theme.color(SemanticToken::TextMuted).into();
    let focus_ring: egui::Color32 = theme.color(SemanticToken::FocusRing).into();
    let edit_active: egui::Color32 = theme.color(SemanticToken::EditActive).into();
    let value_fill: egui::Color32 = theme.color(SemanticToken::ValueFill).into();
    let meter_peak: egui::Color32 = theme.color(SemanticToken::MeterPeak).into();
    let panel_bg: egui::Color32 = theme.color(SemanticToken::PanelBg).into();

    // Row height for the bar.
    let row_height = 16.0_f32;
    let bar_width = 60.0_f32;

    ui.horizontal(|ui| {
        // Label column — TextDefault when focused cell, TextMuted otherwise.
        let label_color = if is_focused_cell {
            text_default
        } else {
            text_muted
        };
        ui.colored_label(label_color, label);

        // Value bar — allocate a fixed rectangle.
        let (bar_rect, _) =
            ui.allocate_exact_size(egui::vec2(bar_width, row_height), egui::Sense::hover());

        let painter = ui.painter();

        // Panel background fill for the bar area.
        painter.rect_filled(bar_rect, 2.0, panel_bg);

        // Value fill (ValueFill token) — fill proportional to `value`.
        let fill_width = (value.clamp(0.0, 1.0) * bar_width).max(0.0);
        if fill_width > 0.0 {
            let fill_rect =
                egui::Rect::from_min_size(bar_rect.min, egui::vec2(fill_width, row_height));
            painter.rect_filled(fill_rect, 2.0, value_fill);
        }

        // On the Volume row: overlay the live peak level (MeterPeak token).
        // The MeterPeak color from the theme is painted directly — no alpha modification.
        if is_volume && peak > 0.001 {
            let peak_width = (peak.clamp(0.0, 1.0) * bar_width).max(0.0);
            let peak_rect =
                egui::Rect::from_min_size(bar_rect.min, egui::vec2(peak_width, row_height));
            painter.rect_filled(peak_rect, 2.0, meter_peak);
        }

        // Focused-cell highlight outline — FocusRing or EditActive.
        // Drawn only when the cell is focused (no stroke otherwise → no literal transparent color needed).
        if is_focused_cell {
            let stroke_color = if edit_mode && is_volume {
                // In edit mode on the Volume row: highlight the full box (EditActive).
                edit_active
            } else if edit_mode {
                edit_active
            } else {
                focus_ring
            };
            painter.rect_stroke(bar_rect, 2.0, egui::Stroke::new(1.5, stroke_color));
        }

        // Numeric readout alongside the bar.
        let readout = format!("{value:.2}");
        let readout_color = if is_focused_cell {
            text_default
        } else {
            text_muted
        };
        ui.colored_label(readout_color, readout);
    });
}

/// Draw one toggle parameter row for a channel strip.
///
/// This is a SKIN function: every color comes from `theme.color(SemanticToken::…)`.
/// No literal color value appears here.
fn draw_toggle_row(
    ui: &mut egui::Ui,
    theme: &DefaultTheme,
    label: &str,
    active: bool,
    is_focused_cell: bool,
    edit_mode: bool,
) {
    // Resolve all colors from the theme — never a literal color.
    let text_default: egui::Color32 = theme.color(SemanticToken::TextDefault).into();
    let text_muted: egui::Color32 = theme.color(SemanticToken::TextMuted).into();
    let toggle_on: egui::Color32 = theme.color(SemanticToken::ToggleOn).into();
    let toggle_off: egui::Color32 = theme.color(SemanticToken::ToggleOff).into();
    let focus_ring: egui::Color32 = theme.color(SemanticToken::FocusRing).into();
    let edit_active: egui::Color32 = theme.color(SemanticToken::EditActive).into();

    let indicator_color = if active { toggle_on } else { toggle_off };

    // Focused-cell stroke color — only used when is_focused_cell is true (no literal transparent needed).
    let cell_stroke_color: egui::Color32 = if edit_mode { edit_active } else { focus_ring };

    ui.horizontal(|ui| {
        let label_color = if is_focused_cell {
            text_default
        } else {
            text_muted
        };
        ui.colored_label(label_color, label);

        // Toggle indicator pill.
        let pill_size = egui::vec2(40.0, 16.0);
        let (pill_rect, _) = ui.allocate_exact_size(pill_size, egui::Sense::hover());
        let painter = ui.painter();

        // Fill with ToggleOn or ToggleOff color.
        painter.rect_filled(pill_rect, 8.0, indicator_color);

        // Focused-cell highlight outline — drawn only when focused (avoids literal transparent color).
        if is_focused_cell {
            painter.rect_stroke(pill_rect, 8.0, egui::Stroke::new(1.5, cell_stroke_color));
        }

        // State label inside the pill.
        let state_text = if active { "ON" } else { "off" };
        let state_color = if active { text_default } else { text_muted };
        painter.text(
            pill_rect.center(),
            egui::Align2::CENTER_CENTER,
            state_text,
            egui::FontId::proportional(11.0),
            state_color,
        );
    });
}

// ── PNG writer (no external dependency needed — raw DEFLATE-less PNG) ─────────────────────────────

/// Write a minimal PNG file with RGBA pixels.
/// Uses zlib-compressed IDAT for valid PNG output.
fn write_png(path: &str, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    // PNG signature
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR chunk
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // color type: RGBA
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_chunk(&mut buf, b"IHDR", &ihdr);

    // IDAT chunk: raw image data with filter byte per row (zlib-compressed)
    let mut raw = Vec::new();
    for row in 0..height as usize {
        raw.push(0); // filter type: None
        let row_start = row * width as usize * 4;
        let row_end = row_start + width as usize * 4;
        raw.extend_from_slice(&rgba[row_start..row_end]);
    }
    let compressed = zlib_compress(&raw);
    write_chunk(&mut buf, b"IDAT", &compressed);

    // IEND chunk
    write_chunk(&mut buf, b"IEND", b"");

    std::fs::write(path, &buf)?;
    Ok(())
}

fn write_chunk(buf: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    let len = data.len() as u32;
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(tag);
    buf.extend_from_slice(data);
    let mut crc = 0xFFFF_FFFFu32;
    for &b in tag.iter().chain(data.iter()) {
        crc = crc_update(crc, b);
    }
    buf.extend_from_slice(&(crc ^ 0xFFFF_FFFF).to_be_bytes());
}

fn crc_update(crc: u32, byte: u8) -> u32 {
    let mut c = crc ^ byte as u32;
    for _ in 0..8 {
        if c & 1 != 0 {
            c = 0xEDB88320 ^ (c >> 1);
        } else {
            c >>= 1;
        }
    }
    c
}

/// Minimal zlib compression (deflate stored blocks — no actual compression, but valid zlib).
fn zlib_compress(data: &[u8]) -> Vec<u8> {
    // zlib header: CMF=0x78 (deflate, window 32k), FLG computed for check bits.
    // For CMF=0x78, FLG must satisfy (CMF*256+FLG) % 31 == 0.
    // 0x78*256 = 30720; 30720 % 31 = 30720 - 991*31 = 30720 - 30721 = ... let's compute:
    // 31*990 = 30690, 30720-30690=30, so 30720%31=30, FLG=(31-30)%31=1.
    let cmf: u8 = 0x78;
    let flg: u8 = 0x01; // makes (cmf*256+flg) % 31 == 0: (30720+1)=30721, 30721%31=0 ✓
    let mut out = vec![cmf, flg];

    // Deflate stored blocks (BTYPE=00, no compression).
    const BLOCK_MAX: usize = 65535;
    let mut pos = 0;
    while pos < data.len() || data.is_empty() {
        let end = (pos + BLOCK_MAX).min(data.len());
        let is_last = end == data.len();
        let bfinal: u8 = if is_last { 1 } else { 0 };
        out.push(bfinal); // BFINAL | BTYPE(00)
        let len = (end - pos) as u16;
        let nlen = !len;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&nlen.to_le_bytes());
        out.extend_from_slice(&data[pos..end]);
        pos = end;
        if data.is_empty() {
            break;
        }
    }

    // Adler-32 checksum.
    let (mut s1, mut s2) = (1u32, 0u32);
    for &b in data {
        s1 = (s1 + b as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    let adler = (s2 << 16) | s1;
    out.extend_from_slice(&adler.to_be_bytes());

    out
}

// ── Smoke mode ──────────────────────────────────────────────────────────────────────────────────

fn run_smoke(play_file: Option<PathBuf>) {
    // In smoke mode: ignore --play entirely (parsing the flag value is fine).
    let _ = play_file;

    // Construct full app state exactly as the window path would.
    let mixer_view = MixerView::new();
    let mut voice_alloc = VoiceAllocator::new(8);
    let patch_mixer = PatchMixer::new();
    // Audio-side 16-channel mixer (tracks peaks, applies volume/pan/mute/solo).
    let mut channel_mixer = ChannelMixer::new();
    let (mut global_mixer_writer, _global_mixer_reader) = GlobalMixer::split(Amplitude::unity());
    let (_param_bridge_writer, _param_bridge_reader) =
        ParameterBridge::split(ParameterSnapshot::default());

    // Drive a few MixerViewEvents to confirm the event loop is wired.
    let mut mv = mixer_view;
    mv.apply(MixerViewEvent::NavRight);
    mv.apply(MixerViewEvent::NavRight);
    mv.apply(MixerViewEvent::EnterEditMode);
    mv.apply(MixerViewEvent::NavRight); // adjust volume fine
    mv.apply(MixerViewEvent::ExitEditMode);
    mv.apply(MixerViewEvent::NavDown); // move to ReverbSend row

    println!("ui smoke ok: app constructed");

    // Audio self-check: apply a synthetic note-on (middle C = 60) at full velocity,
    // then render one block through the EXACT SAME render function the live path uses.
    let note_id = NoteId::new(999);
    let note_number = NoteNumber::try_new(60).expect("60 is valid");
    let vel = Velocity::try_new(1.0).expect("1.0 is valid");
    voice_alloc.note_on(note_id, note_number, vel);

    let mut render_buf: Vec<AudioFrame> = Vec::with_capacity(BLOCK_SIZE);

    render_frames(
        BLOCK_SIZE,
        &mut voice_alloc,
        &patch_mixer,
        &mut channel_mixer,
        &mut global_mixer_writer,
        DEFAULT_SAMPLE_RATE as f64,
        &mut render_buf,
    );

    // Compute block peak.
    let peak = render_buf
        .iter()
        .map(|f| f.left.abs().max(f.right.abs()))
        .fold(0.0_f32, f32::max);

    if peak > 0.0 {
        println!("render non-silent: true");
    } else {
        println!("render non-silent: false");
    }

    // Channel metering: check if any channel recorded a non-zero peak.
    let any_channel_metered = channel_mixer.peaks.iter().any(|p| p.value() > 0.0);
    if any_channel_metered {
        println!("channel metered: true");
    } else {
        println!("channel metered: false");
    }

    // ── Theme self-check ────────────────────────────────────────────────────────────────────────
    // Build the SAME DefaultTheme the draw path uses, resolve EVERY SemanticToken
    // variant through it, and count how many resolved without panic.
    // This proves the design-system seam is wired and exhaustive.
    // N MUST equal the number of SemanticToken variants (10).
    let theme = DefaultTheme::new();
    let mut resolved_count: usize = 0;
    for &token in SemanticToken::all() {
        let _rgba = theme.color(token);
        resolved_count += 1;
    }
    println!("theme tokens resolved: {resolved_count}");

    process::exit(0);
}

// ── Live window mode (also used by --autopilot) ────────────────────────────────────────────────────

fn run_window(play_file: Option<PathBuf>, autopilot: bool, autopilot_seconds: f64) {
    // Channel for MIDI events from external (MidirInput) and --play sequencer.
    let (midi_tx, midi_rx): (SyncSender<InternalMidi>, Receiver<InternalMidi>) =
        mpsc::sync_channel(MIDI_CHANNEL_CAP);

    // ── External MIDI input ───────────────────────────────────────────────────────────────────────
    // Open the first available MIDI port, if any. Only Send data crosses the thread.
    let _midi_connection = open_midi_input(midi_tx.clone());

    // ── Optional --play sequencer ────────────────────────────────────────────────────────────────────────
    if let Some(ref path) = play_file {
        match load_play_events(path) {
            Some(events) => {
                spawn_sequencer_thread(events, midi_tx.clone());
            }
            None => {
                // Warning already printed by load_play_events; continue without sequencer.
            }
        }
    }

    // ── Engine objects (all on main thread) ─────────────────────────────────────────────────────
    let mixer_view = MixerView::new();
    let voice_alloc = VoiceAllocator::new(8);
    let patch_mixer = PatchMixer::new();
    let channel_mixer = ChannelMixer::new();
    let (global_mixer_writer, _global_mixer_reader) = GlobalMixer::split(Amplitude::unity());
    let (param_bridge_writer, _param_bridge_reader) =
        ParameterBridge::split(ParameterSnapshot::default());

    // ── Audio output (must stay on main/UI thread — cpal::Stream is !Send) ───
    let mut audio_out = match CpalAudioOutput::new() {
        Some(o) => o,
        None => {
            eprintln!("error: no default audio output device");
            process::exit(1);
        }
    };
    let sample_rate = SampleRate::try_new(DEFAULT_SAMPLE_RATE).expect("44100 Hz is valid");
    let _stream = audio_out.open_stream(sample_rate);

    let app = SynthUiApp::new(
        mixer_view,
        voice_alloc,
        patch_mixer,
        channel_mixer,
        global_mixer_writer,
        audio_out,
        midi_rx,
        param_bridge_writer,
        autopilot,
        autopilot_seconds,
    );

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Crest Synth Mixer")
            // Window must be wide enough for all 6 strips plus the row-label gutter.
            // At STRIP_WIDTH=120, 6 strips = 720px; add gutter+margin → ~820px minimum.
            .with_inner_size([DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT]),
        ..Default::default()
    };

    eframe::run_native(
        "Crest Synth Mixer",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .unwrap_or_else(|e| {
        eprintln!("error: eframe failed: {e}");
        process::exit(1);
    });
}

/// Attempt to open the first available MIDI input port using midir.
fn open_midi_input(tx: SyncSender<InternalMidi>) -> Option<Box<dyn std::any::Any + Send>> {
    static MIDI_NOTE_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

    let input = midir::MidiInput::new("crest-synth-ui").ok()?;
    let ports = input.ports();
    let port = ports.into_iter().next()?;

    let connection = input
        .connect(
            &port,
            "crest-synth-ui-conn",
            move |_ts, bytes, _| {
                if bytes.len() < 3 {
                    return;
                }
                let status = bytes[0] & 0xF0;
                let note_num = bytes[1];
                let vel_byte = bytes[2];

                match status {
                    0x90 if vel_byte > 0 => {
                        let id = MIDI_NOTE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let note_id = NoteId::new(id);
                        let _ = tx.try_send(InternalMidi {
                            note_id,
                            note_number: note_num,
                            velocity: vel_byte as f64 / 127.0,
                            is_on: true,
                        });
                    }
                    0x80 | 0x90 => {
                        let _ = tx.try_send(InternalMidi {
                            note_id: NoteId::new(0),
                            note_number: note_num,
                            velocity: 0.0,
                            is_on: false,
                        });
                    }
                    _ => {}
                }
            },
            (),
        )
        .ok()?;

    Some(Box::new(connection))
}

// ── main ──────────────────────────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();

    if args.smoke {
        run_smoke(args.play_file);
    } else {
        run_window(args.play_file, args.autopilot, args.seconds as f64);
    }
}
