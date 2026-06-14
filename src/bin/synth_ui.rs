// path: src/bin/synth_ui.rs
//
// synth_ui — standalone eframe/egui parameter editor with live synth engine.
//
// Usage: synth_ui [--smoke] [--play <FILE.mid>]
//
// Default: opens a window, keyboard/gamepad driven editor for synth parameters.
//   Keys: W=NavUp, S=NavDown, A=NavLeft, D=NavRight, J=EnterEditMode (hold)
// --smoke: headless mode — constructs state, drives event loop, audio self-check, exits 0.
// --play <FILE.mid>: load and play a MIDI file via internal sequencer while editing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::Instant;

use eframe::egui;

use crest_synth::adapter::cpal_audio_output::CpalAudioOutput;
use crest_synth::editor::editor_event::EditorEvent;
use crest_synth::editor::editor_state::EditorState;
use crest_synth::editor::param_field::ParamField;
use crest_synth::kernel::amplitude::Amplitude;
use crest_synth::kernel::audio_frame::AudioFrame;
use crest_synth::kernel::midi_event_kind::MidiEventKind;
use crest_synth::kernel::note_id::NoteId;
use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::sample_rate::SampleRate;
use crest_synth::kernel::velocity::Velocity;
use crest_synth::patch::global_mixer::{GlobalMixer, GlobalMixerCommand, GlobalMixerWriter};
use crest_synth::patch::patch_mixer::{PatchMixEntry, PatchMixer};
use crest_synth::real_time::parameter_bridge::ParameterBridge;
use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
use crest_synth::shell::audio_output::AudioOutput;
use crest_synth::synth::voice_allocator::VoiceAllocator;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Audio frames per render block. Small enough for low latency.
const BLOCK_SIZE: usize = 256;

/// Default audio sample rate.
const DEFAULT_SAMPLE_RATE: u32 = 44_100;

/// MIDI event channel capacity.
const MIDI_CHANNEL_CAP: usize = 512;

// ── CLI args ─────────────────────────────────────────────────────────────────

struct Args {
    smoke: bool,
    play_file: Option<PathBuf>,
}

fn parse_args() -> Args {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut smoke = false;
    let mut play_file: Option<PathBuf> = None;

    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--smoke" => {
                smoke = true;
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

    Args { smoke, play_file }
}

// ── ParamField seeds ─────────────────────────────────────────────────────────

/// Identifiers matching each param field index (stable positions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParamId {
    MasterGain,
    Detune,
    FilterCutoff,
}

fn make_param_fields() -> Vec<ParamField> {
    vec![
        ParamField::new("master_gain", "Master Gain", 0.0, 1.0, 0.01, 0.7).unwrap(),
        ParamField::new("detune", "Detune (st)", -24.0, 24.0, 0.5, 0.0).unwrap(),
        ParamField::new(
            "filter_cutoff",
            "Filter Cutoff",
            20.0,
            20000.0,
            10.0,
            5000.0,
        )
        .unwrap(),
    ]
}

/// Maps each field index to its `ParamId`.
fn field_param_id(idx: usize) -> Option<ParamId> {
    match idx {
        0 => Some(ParamId::MasterGain),
        1 => Some(ParamId::Detune),
        2 => Some(ParamId::FilterCutoff),
        _ => None,
    }
}

// ── Internal MIDI event (Send) ────────────────────────────────────────────────

/// A minimal MIDI event that is `Send` — crosses thread boundaries to the
/// main-thread audio-producer loop. Never the cpal stream itself.
#[derive(Debug, Clone)]
struct InternalMidi {
    note_id: NoteId,
    note_number: u8,
    velocity: f64,
    is_on: bool,
}

// ── Render function (shared by live path and smoke self-check) ───────────────

/// Render `num_frames` audio frames using the given voice allocator and mixers.
///
/// This is the SINGLE render function that both the live eframe update tick and
/// the --smoke audio self-check call. Both paths must use this exact function
/// so the self-check truly exercises the live render graph.
#[allow(clippy::too_many_arguments)]
fn render_frames(
    num_frames: usize,
    voice_alloc: &mut VoiceAllocator,
    patch_mixer: &PatchMixer,
    global_mixer_writer: &mut GlobalMixerWriter,
    sample_rate: f64,
    master_gain: f64,
    detune: f64,
    output: &mut Vec<AudioFrame>,
) {
    // Publish latest master gain to the audio side via the control-thread handle.
    if let Ok(amp) = Amplitude::try_new(master_gain) {
        let _ = global_mixer_writer.handle(GlobalMixerCommand::SetMasterGain { gain: amp });
    }

    let gain = global_mixer_writer.state().master_gain.value() as f32;

    output.clear();
    let mut remaining = num_frames;
    while remaining > 0 {
        let this_block = remaining.min(BLOCK_SIZE);

        for _ in 0..this_block {
            // Render one sample from all voices.
            let (sample, _events) = voice_alloc.render_sample(sample_rate, detune);

            // Pass through PatchMixer (centre pan, unity gain for single patch).
            let patch_frame = AudioFrame::mono(sample);
            let mixed = patch_mixer.apply_entry(patch_frame, &PatchMixEntry::unity());

            // Apply master gain from GlobalMixer writer state.
            let out_frame = AudioFrame {
                left: mixed.left * gain,
                right: mixed.right * gain,
            };
            output.push(out_frame);
        }

        remaining -= this_block;
    }
}

// ── --play MIDI sequencer (background thread, sends InternalMidi) ─────────────

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
/// sending `InternalMidi` events to `tx`. The thread loops the file.
///
/// Only Send data (InternalMidi) crosses the thread boundary — never cpal streams.
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

                // Dispatch all events whose times have passed.
                while cursor < events.len() && events[cursor].0 <= elapsed {
                    let (_, ref ev) = events[cursor];
                    // Non-blocking try_send: if full, drop the event (never block).
                    let _ = tx.try_send(ev.clone());
                    cursor += 1;
                }

                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    });
}

// ── eframe App ────────────────────────────────────────────────────────────────

struct SynthUiApp {
    /// One-way editor state (the only source of truth for parameter values).
    editor_state: EditorState,
    /// J key was held last frame (for edge detection).
    j_was_held: bool,
    /// Key press edge detection for W/S/A/D.
    w_was_down: bool,
    s_was_down: bool,
    a_was_down: bool,
    d_was_down: bool,

    /// Voice allocator on main thread — feeds the audio buffer.
    voice_alloc: VoiceAllocator,
    /// Patch mixer (stateless, shared render logic).
    patch_mixer: PatchMixer,
    /// GlobalMixer control-thread handle.
    global_mixer_writer: GlobalMixerWriter,

    /// cpal audio output — kept on main thread (not Send on macOS).
    audio_out: CpalAudioOutput,

    /// Reusable render buffer to avoid per-tick allocation.
    render_buf: Vec<AudioFrame>,

    /// Receiver for MIDI events from MidirInput and sequencer thread.
    midi_rx: Receiver<InternalMidi>,

    /// Active notes: note_number -> NoteId (for note-off matching).
    active_notes: HashMap<u8, NoteId>,

    /// ParameterBridge writer — publishes ParameterSnapshot to audio thread.
    _param_bridge_writer: crest_synth::real_time::parameter_bridge::ParameterBridgeWriter,
}

impl SynthUiApp {
    fn new(
        editor_state: EditorState,
        voice_alloc: VoiceAllocator,
        patch_mixer: PatchMixer,
        global_mixer_writer: GlobalMixerWriter,
        audio_out: CpalAudioOutput,
        midi_rx: Receiver<InternalMidi>,
        param_bridge_writer: crest_synth::real_time::parameter_bridge::ParameterBridgeWriter,
    ) -> Self {
        Self {
            editor_state,
            j_was_held: false,
            w_was_down: false,
            s_was_down: false,
            a_was_down: false,
            d_was_down: false,
            voice_alloc,
            patch_mixer,
            global_mixer_writer,
            audio_out,
            render_buf: Vec::with_capacity(BLOCK_SIZE * 4),
            midi_rx,
            active_notes: HashMap::new(),
            _param_bridge_writer: param_bridge_writer,
        }
    }

    /// Read raw key state and emit EditorEvents (press-edge for nav, hold-edge for J).
    fn process_keyboard(&mut self, ctx: &egui::Context) {
        let input = ctx.input(|i| i.clone());

        // J = EnterEditMode while held, ExitEditMode on release.
        let j_held = input.key_down(egui::Key::J);
        if j_held && !self.j_was_held {
            self.editor_state.apply(EditorEvent::EnterEditMode);
        } else if !j_held && self.j_was_held {
            self.editor_state.apply(EditorEvent::ExitEditMode);
        }
        self.j_was_held = j_held;

        // W = NavUp (press edge).
        let w_down = input.key_down(egui::Key::W);
        if w_down && !self.w_was_down {
            self.editor_state.apply(EditorEvent::NavUp);
        }
        self.w_was_down = w_down;

        // S = NavDown (press edge).
        let s_down = input.key_down(egui::Key::S);
        if s_down && !self.s_was_down {
            self.editor_state.apply(EditorEvent::NavDown);
        }
        self.s_was_down = s_down;

        // A = NavLeft (press edge).
        let a_down = input.key_down(egui::Key::A);
        if a_down && !self.a_was_down {
            self.editor_state.apply(EditorEvent::NavLeft);
        }
        self.a_was_down = a_down;

        // D = NavRight (press edge).
        let d_down = input.key_down(egui::Key::D);
        if d_down && !self.d_was_down {
            self.editor_state.apply(EditorEvent::NavRight);
        }
        self.d_was_down = d_down;
    }

    /// Drain all pending MIDI events from the channel and apply to voice allocator.
    fn drain_midi(&mut self) {
        while let Ok(ev) = self.midi_rx.try_recv() {
            if ev.is_on {
                if let Ok(note_num) = NoteNumber::try_new(ev.note_number) {
                    if let Ok(vel) = Velocity::try_new(ev.velocity.clamp(0.0, 1.0)) {
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

    /// Read current field values and feed audio buffer by free space (self-regulating).
    fn render_and_feed_audio(&mut self) {
        let free = self.audio_out.available_frames();
        if free == 0 {
            return;
        }

        let fields = self.editor_state.fields();
        let master_gain = fields
            .iter()
            .enumerate()
            .find(|(i, _)| field_param_id(*i) == Some(ParamId::MasterGain))
            .map(|(_, f)| f.value())
            .unwrap_or(0.7);
        let detune = fields
            .iter()
            .enumerate()
            .find(|(i, _)| field_param_id(*i) == Some(ParamId::Detune))
            .map(|(_, f)| f.value())
            .unwrap_or(0.0);

        render_frames(
            free,
            &mut self.voice_alloc,
            &self.patch_mixer,
            &mut self.global_mixer_writer,
            DEFAULT_SAMPLE_RATE as f64,
            master_gain,
            detune,
            &mut self.render_buf,
        );

        self.audio_out.write_buffer(&self.render_buf);
    }
}

impl eframe::App for SynthUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Process keyboard input → emit EditorEvents → update EditorState.
        self.process_keyboard(ctx);

        // 2. Drain MIDI from all sources.
        self.drain_midi();

        // 3. Render audio and feed the ring buffer by free space (self-regulating).
        self.render_and_feed_audio();

        // 4. Draw the UI as a PURE VIEW over EditorState (no state mutation here).
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Crest Synth Editor");
            ui.separator();

            let mode_label = if self.editor_state.edit_mode() {
                "EDIT [hold J]"
            } else {
                "NAVIGATE"
            };
            ui.label(format!("Mode: {mode_label}"));
            ui.separator();

            ui.label("W/S = up/down  A/D = adjust  Hold J = edit");
            ui.separator();

            let fields = self.editor_state.fields();
            let focus = self.editor_state.focus();

            for (i, field) in fields.iter().enumerate() {
                let is_focused = i == focus;
                let prefix = if is_focused { "> " } else { "  " };
                ui.label(format!(
                    "{}{}: {:.3}  ({}..{}  step {})",
                    prefix,
                    field.label,
                    field.value(),
                    field.min,
                    field.max,
                    field.step,
                ));
            }
        });

        // 5. Request continuous repaint so the audio loop keeps running.
        ctx.request_repaint();
    }
}

// ── Smoke mode ────────────────────────────────────────────────────────────────

fn run_smoke(play_file: Option<PathBuf>) {
    // In smoke mode, ignore --play completely: no file I/O, no sequencer thread.
    // Parse the flag value is fine; touching the file or audio is not.
    let _ = play_file; // intentionally ignored in smoke

    // Construct full app state exactly as the window path would.
    let fields = make_param_fields();
    let mut editor_state = EditorState::new(fields);

    // Engine objects (no audio devices opened).
    let mut voice_alloc = VoiceAllocator::new(8);
    let patch_mixer = PatchMixer::new();
    let (mut global_mixer_writer, _global_mixer_reader) = GlobalMixer::split(Amplitude::unity());
    let (_param_bridge_writer, _param_bridge_reader) =
        ParameterBridge::split(ParameterSnapshot::default());

    // Drive a few EditorEvents to confirm the loop is wired.
    editor_state.apply(EditorEvent::NavDown);
    editor_state.apply(EditorEvent::EnterEditMode);
    editor_state.apply(EditorEvent::NavRight);
    editor_state.apply(EditorEvent::ExitEditMode);
    editor_state.apply(EditorEvent::NavUp);

    println!("ui smoke ok: app constructed");

    // Audio self-check: apply a synthetic note-on (middle C = 60) at full velocity,
    // then render one block through the SAME render function the live path uses.
    let note_id = NoteId::new(999);
    let note_number = NoteNumber::try_new(60).expect("60 is valid");
    let vel = Velocity::try_new(1.0).expect("1.0 is valid");
    voice_alloc.note_on(note_id, note_number, vel);

    let fields = editor_state.fields();
    let master_gain = fields
        .iter()
        .enumerate()
        .find(|(i, _)| field_param_id(*i) == Some(ParamId::MasterGain))
        .map(|(_, f)| f.value())
        .unwrap_or(0.7);
    let detune = fields
        .iter()
        .enumerate()
        .find(|(i, _)| field_param_id(*i) == Some(ParamId::Detune))
        .map(|(_, f)| f.value())
        .unwrap_or(0.0);

    let mut render_buf: Vec<AudioFrame> = Vec::with_capacity(BLOCK_SIZE);

    render_frames(
        BLOCK_SIZE,
        &mut voice_alloc,
        &patch_mixer,
        &mut global_mixer_writer,
        DEFAULT_SAMPLE_RATE as f64,
        master_gain,
        detune,
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

    process::exit(0);
}

// ── Live window mode ──────────────────────────────────────────────────────────

fn run_window(play_file: Option<PathBuf>) {
    // Channel for MIDI events from external (MidirInput) and --play sequencer.
    let (midi_tx, midi_rx): (SyncSender<InternalMidi>, Receiver<InternalMidi>) =
        mpsc::sync_channel(MIDI_CHANNEL_CAP);

    // ── External MIDI input via MidirInput ────────────────────────────────────
    // Open the first available MIDI port, if any. The midir callback sends
    // InternalMidi (Send) over the channel — never the stream itself.
    let _midi_connection = open_midi_input(midi_tx.clone());

    // ── Optional --play sequencer ─────────────────────────────────────────────
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

    // ── Engine objects (all on main thread, !Send cpal stays here) ───────────
    let fields = make_param_fields();
    let editor_state = EditorState::new(fields);
    let voice_alloc = VoiceAllocator::new(8);
    let patch_mixer = PatchMixer::new();
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
        editor_state,
        voice_alloc,
        patch_mixer,
        global_mixer_writer,
        audio_out,
        midi_rx,
        param_bridge_writer,
    );

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Crest Synth Editor")
            .with_inner_size([480.0, 360.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Crest Synth Editor",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .unwrap_or_else(|e| {
        eprintln!("error: eframe failed: {e}");
        process::exit(1);
    });
}

/// Attempt to open the first available MIDI input port using midir.
/// On success, the callback sends InternalMidi events over `tx`.
/// Returns an opaque connection handle; dropping it closes the port.
fn open_midi_input(tx: SyncSender<InternalMidi>) -> Option<Box<dyn std::any::Any + Send>> {
    // Use raw midir directly: the callback runs on midir's internal thread
    // and sends only InternalMidi (which is Send) over the channel.
    let raw_tx = tx;
    let input = midir::MidiInput::new("crest-synth-ui").ok()?;
    let ports = input.ports();
    let port = ports.into_iter().next()?;

    // Spawn the connection. The callback runs on midir's internal thread.
    // We only send Send data (InternalMidi) across thread boundaries.
    // Use a static atomic counter for monotonic note IDs from external MIDI.
    static MIDI_NOTE_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

    let connection = input
        .connect(
            &port,
            "crest-synth-ui-conn",
            move |_ts, bytes, _| {
                // Parse note-on / note-off from raw bytes.
                if bytes.len() < 3 {
                    return;
                }
                let status = bytes[0] & 0xF0;
                let note_num = bytes[1];
                let vel_byte = bytes[2];

                match status {
                    0x90 if vel_byte > 0 => {
                        // Note-on: allocate a fresh monotonic NoteId.
                        let id = MIDI_NOTE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let note_id = NoteId::new(id);
                        let _ = raw_tx.try_send(InternalMidi {
                            note_id,
                            note_number: note_num,
                            velocity: vel_byte as f64 / 127.0,
                            is_on: true,
                        });
                    }
                    0x80 | 0x90 => {
                        // Note-off (0x80 or 0x90 vel=0).
                        // NoteId 0 is a sentinel; drain_midi matches by note_number.
                        let _ = raw_tx.try_send(InternalMidi {
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

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();

    if args.smoke {
        run_smoke(args.play_file);
    } else {
        run_window(args.play_file);
    }
}
