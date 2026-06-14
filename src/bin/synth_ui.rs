// path: src/bin/synth_ui.rs
//
// synth_ui — standalone eframe/egui MIXER VIEW with live synth engine.
//
// Usage: synth_ui [--smoke] [--play <FILE.mid>]
//
// Default: opens a window with keyboard/gamepad-driven mixer view.
//   Keys: W=NavUp, S=NavDown, A=NavLeft, D=NavRight, J=EnterEditMode (hold), J double-tap=ToggleFocusedParam
// --smoke: hermetic headless mode — constructs state, drives event loop, audio self-check, exits 0.
// --play <FILE.mid>: load and play a MIDI file via internal sequencer while editing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::Instant;

use eframe::egui;

use crest_synth::adapter::cpal_audio_output::CpalAudioOutput;
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

// ── Constants ─────────────────────────────────────────────────────────────────

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

// ── Input layer: J key double-tap / hold detection ───────────────────────────

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

// ── Gamepad double-tap / hold for Edit button (South) ────────────────────────

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

// ── Render function (shared by live path and smoke self-check) ───────────────

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

// ── --play MIDI sequencer ─────────────────────────────────────────────────────

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

// ── eframe App ────────────────────────────────────────────────────────────────

struct SynthUiApp {
    // ── UI state (one-way event loop) ────────────────────────────────────────
    /// Mixer view — the only source of truth for UI mixer state.
    mixer_view: MixerView,

    // ── Keyboard input state ─────────────────────────────────────────────────
    j_key: JKeyState,
    w_was_down: bool,
    s_was_down: bool,
    a_was_down: bool,
    d_was_down: bool,

    // ── Gamepad input state ──────────────────────────────────────────────────
    gamepad_edit: GamepadEditState,
    gilrs: Option<gilrs::Gilrs>,

    // ── Audio engine (all on main thread) ────────────────────────────────────
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

    // ── MIDI sources ─────────────────────────────────────────────────────────
    /// Receiver for MIDI events from MidirInput callback and sequencer thread.
    midi_rx: Receiver<InternalMidi>,
    /// Per-note-number active NoteId (for external MIDI note-off matching).
    active_notes: HashMap<u8, NoteId>,

    // ── Parameter bridge (kept alive) ────────────────────────────────────────
    _param_bridge_writer: crest_synth::real_time::parameter_bridge::ParameterBridgeWriter,
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
    ) -> Self {
        // Try to initialise gilrs for gamepad input (non-fatal if unavailable).
        let gilrs = gilrs::Gilrs::new().ok();

        Self {
            mixer_view,
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

        self.audio_out.write_buffer(&self.render_buf);
    }
}

impl eframe::App for SynthUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Process input → MixerViewEvents → mixer_view.apply().
        //    The draw code is a PURE VIEW; it never mutates state directly.
        self.process_keyboard(ctx);
        self.process_gamepad();

        // 2. Drain MIDI from all sources and apply to voice allocator.
        self.drain_midi();

        // 3. Sync MixerView → audio ChannelMixer (one-way).
        self.sync_mixer_to_audio();

        // 4. Feed the audio ring buffer (self-regulating by available_frames).
        self.render_and_feed_audio();

        // 5. Draw the MIXER VIEW as a pure view over mixer_view state.
        egui::CentralPanel::default().show(ctx, |ui| {
            draw_mixer_view(ui, &self.mixer_view, &self.channel_mixer);
        });

        // 6. Keep the loop running fast enough to stay ahead of the audio buffer.
        ctx.request_repaint();
    }
}

/// Pure view: draw the 6 visible mixer channel strips.
///
/// This function is a PURE VIEW — it only reads from `mixer_view` and
/// `channel_mixer`; it never mutates any state.
fn draw_mixer_view(ui: &mut egui::Ui, view: &MixerView, channel_mixer: &ChannelMixer) {
    let offset = view.viewport_offset();
    let cursor_ch = view.cursor_channel();
    let cursor_param = view.cursor_param();
    let edit_mode = view.edit_mode();

    let mode_label = if edit_mode {
        "EDIT (hold J)"
    } else {
        "NAVIGATE"
    };
    ui.horizontal(|ui| {
        ui.heading("Crest Synth Mixer");
        ui.separator();
        ui.label(format!("Mode: {mode_label}"));
        ui.separator();
        ui.label("W/S=row  A/D=channel  Hold J=edit  Double-tap J=toggle");
    });
    ui.separator();

    let ui_mixer = view.mixer();

    // Draw 6 channel strips side by side.
    ui.horizontal(|ui| {
        for vis_idx in 0..VISIBLE_CHANNELS {
            let ch_idx = offset + vis_idx;
            let is_focused_ch = ch_idx == cursor_ch;

            ui.vertical(|ui| {
                // Channel header
                let ch_label = format!("Ch{}", ch_idx + 1);
                if is_focused_ch {
                    ui.strong(ch_label);
                } else {
                    ui.label(ch_label);
                }

                // The rows: Volume, ReverbSend, EchoSend, Pan, Mute, Solo
                let ch_state = ui_mixer.channel(ch_idx);
                // Live peak from audio channel mixer (independent of mute/solo)
                let peak = channel_mixer.peaks[ch_idx].value();

                draw_param_row(
                    ui,
                    "Vol",
                    ch_state.volume,
                    peak,
                    is_focused_ch,
                    cursor_param == MixerParam::Volume && is_focused_ch,
                    edit_mode,
                    false,
                    false,
                );
                draw_param_row(
                    ui,
                    "Rvb",
                    ch_state.reverb_send,
                    0.0,
                    is_focused_ch,
                    cursor_param == MixerParam::ReverbSend && is_focused_ch,
                    edit_mode,
                    false,
                    false,
                );
                draw_param_row(
                    ui,
                    "Ech",
                    ch_state.echo_send,
                    0.0,
                    is_focused_ch,
                    cursor_param == MixerParam::EchoSend && is_focused_ch,
                    edit_mode,
                    false,
                    false,
                );
                draw_param_row(
                    ui,
                    "Pan",
                    (ch_state.pan + 1.0) / 2.0,
                    0.0,
                    is_focused_ch,
                    cursor_param == MixerParam::Pan && is_focused_ch,
                    edit_mode,
                    false,
                    false,
                );
                draw_toggle_row(
                    ui,
                    "Mute",
                    ch_state.mute,
                    is_focused_ch,
                    cursor_param == MixerParam::Mute && is_focused_ch,
                    edit_mode,
                );
                draw_toggle_row(
                    ui,
                    "Solo",
                    ch_state.solo,
                    is_focused_ch,
                    cursor_param == MixerParam::Solo && is_focused_ch,
                    edit_mode,
                );

                ui.separator();
            });

            ui.separator();
        }
    });
}

/// Draw one continuous parameter row for a channel strip.
///
/// `value` is normalized 0–1. `peak` is the live peak level for the Volume row.
#[allow(clippy::too_many_arguments)]
fn draw_param_row(
    ui: &mut egui::Ui,
    label: &str,
    value: f32,
    peak: f32,
    _is_focused_ch: bool,
    is_focused_cell: bool,
    edit_mode: bool,
    _show_peak: bool,
    _is_volume: bool,
) {
    let display = if is_focused_cell && edit_mode {
        format!("> {label}: {value:.2} <")
    } else if is_focused_cell {
        format!("[{label}: {value:.2}]")
    } else {
        format!(" {label}: {value:.2}")
    };
    // Show peak next to label if non-zero (for metering feedback).
    let display = if peak > 0.001 {
        format!("{display} ~{peak:.2}")
    } else {
        display
    };
    ui.label(display);
}

/// Draw one toggle parameter row for a channel strip.
fn draw_toggle_row(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    _is_focused_ch: bool,
    is_focused_cell: bool,
    edit_mode: bool,
) {
    let state = if active { "ON" } else { "off" };
    let display = if is_focused_cell && edit_mode {
        format!("> {label}:{state} <")
    } else if is_focused_cell {
        format!("[{label}:{state}]")
    } else {
        format!(" {label}:{state}")
    };
    ui.label(display);
}

// ── Smoke mode ────────────────────────────────────────────────────────────────

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

    process::exit(0);
}

// ── Live window mode ──────────────────────────────────────────────────────────

fn run_window(play_file: Option<PathBuf>) {
    // Channel for MIDI events from external (MidirInput) and --play sequencer.
    let (midi_tx, midi_rx): (SyncSender<InternalMidi>, Receiver<InternalMidi>) =
        mpsc::sync_channel(MIDI_CHANNEL_CAP);

    // ── External MIDI input ───────────────────────────────────────────────────
    // Open the first available MIDI port, if any. Only Send data crosses the thread.
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

    // ── Engine objects (all on main thread) ───────────────────────────────────
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
    );

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Crest Synth Mixer")
            .with_inner_size([720.0, 420.0]),
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

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();

    if args.smoke {
        run_smoke(args.play_file);
    } else {
        run_window(args.play_file);
    }
}
