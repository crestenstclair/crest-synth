// path: src/bin/midi_play_live.rs

//! CLI: `midi_play_live [FILE.mid] [--seconds N] [--no-device-dry-run]`
//!
//! Streams a Standard MIDI File (loaded via the `MidiFileReader` port and
//! its `MidlyMidiFileReader` adapter -- this project's MIDI-file-loader
//! module) -- or, if no file is given, a built-in demo melody -- through
//! the default audio output device in real time, via the `CpalAudioOutput`
//! adapter (the `Shell::AudioOutput` port) and the phase-2/3 engine
//! (`Voice` aggregate, `VoiceAllocator`, `VoiceRenderer`/`EngineRenderer`
//! domain services).
//!
//! Two run modes:
//! - Live playback (default): resolves the default output device, prints a
//!   startup line (device name, event count, duration), then streams audio
//!   in real time until the timeline finishes or `--seconds` elapses.
//! - `--no-device-dry-run`: never touches `cpal`'s host/device APIs and
//!   never blocks on the wall clock. Parses the CLI and the timeline, then
//!   constructs the exact real-time pipeline the live path uses -- an
//!   `RtrbEventRing`, a `TripleBufferParameterBridge`, and `basedrop`'s
//!   deferred-deallocation plumbing -- prints a line containing
//!   `dry-run ok: pipeline constructed`, and exits 0. This makes the
//!   real-time wiring mechanically checkable on any machine, including CI,
//!   with no audio device present.
//!
//! # Real-time boundary
//!
//! The audio callback (`PlaybackCallback::render`) never allocates heap
//! memory, never acquires a lock, and never performs blocking I/O. Every
//! note-on/note-off crosses from this binary's scheduling loop (the
//! non-real-time main thread) to the audio thread through the
//! `RtrbEventRing`; every parameter read crosses through the
//! `TripleBufferParameterBridge`. No other path touches audio-thread state.
//!
//! `EnvelopeGenerator`/`Filter` adapters are not yet available as committed
//! resources in this crate beyond `StateVariableFilter`, so a minimal local
//! ADSR envelope (`SimpleAdsrEnvelope`) is defined here, following this
//! project's established convention for not-yet-available collaborators
//! (see `engine::voice`, `bin::synth_ui`).

use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait};

use crest_synth::engine::engine_renderer::{EngineRenderer, EngineRendererError, VoiceRenderState};
use crest_synth::engine::envelope_generator::EnvelopeGenerator;
use crest_synth::engine::filter::{FilterConfig, FilterKind, StateVariableFilter};
use crest_synth::engine::oscillator::{
    Amplitude as OscAmplitude, OscillatorConfig, SampleRate as EngineSampleRate,
    StandardOscillator, Waveform as EngineWaveform,
};
use crest_synth::engine::voice::{
    EnvelopeTiming, NoteId as EngineNoteId, NoteNumber as EngineNoteNumber,
    Velocity as EngineVelocity, VoiceConfig as EngineVoiceConfig, VoiceEvent,
};
use crest_synth::engine::voice_allocator::{StealPolicy, VoiceAllocator, VoiceAssignment};
use crest_synth::engine::voice_renderer::VoiceRenderer;

use crest_synth::kernel::audio_frame::AudioFrame;
use crest_synth::kernel::channel_address::{ChannelAddress, MidiChannel, MidiGroup};
use crest_synth::kernel::midi_event::MidiEvent;
use crest_synth::kernel::midi_event_kind::MidiEventKind;
use crest_synth::kernel::note_id::NoteId as KernelNoteId;
use crest_synth::kernel::note_number::NoteNumber as KernelNoteNumber;
use crest_synth::kernel::velocity::Velocity as KernelVelocity;

use crest_synth::midi_file::midi_file_reader::{MidiFileReader, TimedMidiEvent};
use crest_synth::midi_file::midly_midi_file_reader::MidlyMidiFileReader;

use crest_synth::real_time::basedrop_deferred_deallocator::{
    basedrop_deferred_deallocator, BasedropRetirer,
};
use crest_synth::real_time::deferred_deallocator::Collect;
use crest_synth::real_time::event_ring::BoundaryMessage as RtBoundaryMessage;
use crest_synth::real_time::parameter_bridge::ParameterSnapshot as BridgeParameterSnapshot;
use crest_synth::real_time::rtrb_event_ring::RtrbEventRing;
use crest_synth::real_time::triple_buffer_parameter_bridge::TripleBufferParameterBridge;

use crest_synth::shell::audio_output::{
    AudioOutput, BufferSize, RenderCallback, SampleRate as PortSampleRate,
};
use crest_synth::shell::cpal_audio_output::CpalAudioOutput;

/// Fixed render sample rate used throughout this binary.
const SAMPLE_RATE_HZ: u32 = 44_100;
/// Frames rendered per block: both the headless dry-run's would-be block
/// size and the live `cpal` stream's fixed buffer size.
const BLOCK_LEN: usize = 512;
/// Fixed voice pool size for the playback engine.
const POLYPHONY: usize = 16;
/// Capacity of the real-time event ring carrying note on/off messages from
/// the scheduling loop to the audio callback.
const EVENT_RING_CAPACITY: usize = 256;
/// Extra seconds of playback appended after the last scheduled event so a
/// note's release stage can ring out before the stream is torn down.
const RELEASE_TAIL_SECONDS: f64 = 1.0;

// ---------------------------------------------------------------------
// CLI parsing.
// ---------------------------------------------------------------------

/// Parsed command-line arguments for `midi_play_live`.
#[derive(Debug, Clone, PartialEq)]
struct CliArgs {
    file_path: Option<PathBuf>,
    seconds_cap: Option<f64>,
    dry_run: bool,
}

/// Parses `midi_play_live [FILE.mid] [--seconds N] [--no-device-dry-run]`.
///
/// Exits the process with a non-zero status and a human-readable stderr
/// message on malformed input (an unrecognized flag, or a `--seconds`
/// missing/invalid value) -- never panics.
fn parse_args(args: &[String]) -> CliArgs {
    let mut file_path: Option<PathBuf> = None;
    let mut seconds_cap: Option<f64> = None;
    let mut dry_run = false;

    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--seconds" => match iter.next() {
                Some(value) => match value.parse::<f64>() {
                    Ok(parsed) if parsed.is_finite() && parsed > 0.0 => {
                        seconds_cap = Some(parsed);
                    }
                    _ => {
                        eprintln!("midi_play_live: invalid --seconds value '{value}'");
                        std::process::exit(1);
                    }
                },
                None => {
                    eprintln!("midi_play_live: --seconds requires a value");
                    std::process::exit(1);
                }
            },
            "--no-device-dry-run" => dry_run = true,
            other if !other.starts_with("--") && file_path.is_none() => {
                file_path = Some(PathBuf::from(other));
            }
            other => {
                eprintln!("midi_play_live: unrecognized argument '{other}'");
                std::process::exit(1);
            }
        }
    }

    CliArgs {
        file_path,
        seconds_cap,
        dry_run,
    }
}

// ---------------------------------------------------------------------
// Timeline: loaded from a MIDI file via `MidiFileReader`/`MidlyMidiFileReader`,
// or a built-in demo melody when no file is given.
// ---------------------------------------------------------------------

/// The channel address every demo-melody event is addressed to (channel 0,
/// group 0).
fn demo_channel_address() -> ChannelAddress {
    ChannelAddress::new(
        MidiChannel::try_new(0).expect("0 is a valid MIDI channel"),
        MidiGroup::try_new(0).expect("0 is a valid MIDI group"),
    )
}

/// Builds one timed note-on/note-off event for the demo melody.
fn demo_note_event(
    at_seconds: f64,
    kind: MidiEventKind,
    note: u8,
    note_id: u32,
    velocity_midi7: u8,
) -> TimedMidiEvent {
    TimedMidiEvent::new(
        at_seconds,
        MidiEvent::new(
            demo_channel_address(),
            kind,
            KernelNoteNumber::try_new(note).expect("demo note numbers are valid MIDI notes"),
            KernelNoteId::new(note_id),
            KernelVelocity::from_midi7(velocity_midi7),
        ),
    )
}

/// The built-in demo melody played when no `FILE.mid` is given: a C major
/// arpeggio (C4, E4, G4, C5), each note held for 0.45s starting every 0.5s.
/// Each note gets a freshly minted `NoteId`, matching the `MidiFileReader`
/// contract even though this demo never overlaps notes.
fn demo_timeline() -> Vec<TimedMidiEvent> {
    const NOTES: [u8; 4] = [60, 64, 67, 72];
    let mut events = Vec::with_capacity(NOTES.len() * 2);
    for (index, &note) in NOTES.iter().enumerate() {
        let start = index as f64 * 0.5;
        let end = start + 0.45;
        let note_id = index as u32;
        events.push(demo_note_event(
            start,
            MidiEventKind::NoteOn,
            note,
            note_id,
            100,
        ));
        events.push(demo_note_event(
            end,
            MidiEventKind::NoteOff,
            note,
            note_id,
            0,
        ));
    }
    events
}

/// Loads the timeline: from `file_path` via `MidlyMidiFileReader` if given,
/// otherwise the built-in demo melody. Returns a human-readable error
/// message (never panics) if the file cannot be loaded or decoded.
fn load_timeline(file_path: Option<&PathBuf>) -> Result<Vec<TimedMidiEvent>, String> {
    match file_path {
        Some(path) => {
            let reader = MidlyMidiFileReader::new();
            let song = reader.load(path).map_err(|err| err.to_string())?;
            Ok(song.events().to_vec())
        }
        None => Ok(demo_timeline()),
    }
}

/// The natural duration of a timeline: the latest event's timestamp plus a
/// fixed release tail so the last note's envelope can ring out. `0.0` for an
/// empty timeline (plus the tail).
fn timeline_duration_seconds(events: &[TimedMidiEvent]) -> f64 {
    events
        .iter()
        .map(TimedMidiEvent::at_seconds)
        .fold(0.0, f64::max)
        + RELEASE_TAIL_SECONDS
}

/// Translates a normalized `MidiEvent` into the real-time boundary message
/// carried by the `RtrbEventRing`, or `None` for event kinds this simple
/// player does not forward (only note lifecycle events drive playback here).
fn to_boundary_message(event: &MidiEvent) -> Option<RtBoundaryMessage> {
    let channel = event.address().channel().value();
    let note = event.note().value();
    match event.kind() {
        MidiEventKind::NoteOn => Some(RtBoundaryMessage::NoteOn {
            channel,
            note,
            velocity: event.velocity().to_midi7(),
        }),
        MidiEventKind::NoteOff => Some(RtBoundaryMessage::NoteOff { channel, note }),
        _ => None,
    }
}

/// Derives a stable engine-level `NoteId` from a raw MIDI channel/note pair.
/// The `RtrbEventRing`'s `BoundaryMessage` carries only channel/note/velocity
/// (matching the MIDI 1.0 wire shape), not a pre-minted `NoteId`, so the
/// audio-thread consumer reconstructs a stable identity itself: the same
/// (channel, note) always maps to the same engine `NoteId`, which is all
/// `VoiceAllocator::release` needs to find the right voice to release.
fn synthetic_note_id(channel: u8, note: u8) -> EngineNoteId {
    EngineNoteId::new((u64::from(channel) << 8) | u64::from(note))
}

/// Pushes `message` onto `event_ring`, retrying until it succeeds.
///
/// Only ever called from the non-real-time scheduling loop, so blocking
/// (via a yield-and-retry spin) here is legal -- the audio thread's own
/// `pop` never blocks.
fn push_blocking(event_ring: &RtrbEventRing, message: RtBoundaryMessage) {
    while event_ring.push(message).is_err() {
        thread::yield_now();
    }
}

// ---------------------------------------------------------------------
// A minimal, real-time-safe ADSR envelope generator.
//
// No concrete `EnvelopeGenerator` adapter is committed to this crate yet,
// so one is defined locally here, matching this project's established
// convention for not-yet-available collaborators (see `engine::voice`,
// `bin::synth_ui::SimpleAdsrEnvelope`).
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdsrStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// A simple sample-driven ADSR envelope. Never allocates, locks, or blocks
/// in `trigger`/`release`/`tick`, matching the `EnvelopeGenerator` port's
/// real-time safety contract.
#[derive(Debug, Clone, Copy)]
struct SimpleAdsrEnvelope {
    timing: EnvelopeTiming,
    seconds_per_sample: f64,
    stage: AdsrStage,
    level: f64,
}

impl SimpleAdsrEnvelope {
    fn new(timing: EnvelopeTiming, sample_rate_hz: f64) -> Self {
        Self {
            timing,
            seconds_per_sample: 1.0 / sample_rate_hz.max(1.0),
            stage: AdsrStage::Idle,
            level: 0.0,
        }
    }
}

impl EnvelopeGenerator for SimpleAdsrEnvelope {
    fn trigger(&mut self) {
        self.stage = AdsrStage::Attack;
    }

    fn release(&mut self) {
        if self.stage != AdsrStage::Idle {
            self.stage = AdsrStage::Release;
        }
    }

    fn tick(&mut self) -> f64 {
        let dt = self.seconds_per_sample;
        match self.stage {
            AdsrStage::Idle => {
                self.level = 0.0;
            }
            AdsrStage::Attack => {
                let attack = self.timing.attack_seconds.max(dt);
                self.level += dt / attack;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = AdsrStage::Decay;
                }
            }
            AdsrStage::Decay => {
                let decay = self.timing.decay_seconds.max(dt);
                let target = self.timing.sustain_level;
                let span = (1.0 - target).max(0.0);
                self.level -= dt * span / decay;
                if self.level <= target {
                    self.level = target;
                    self.stage = AdsrStage::Sustain;
                }
            }
            AdsrStage::Sustain => {
                self.level = self.timing.sustain_level;
            }
            AdsrStage::Release => {
                let release = self.timing.release_seconds.max(dt);
                let span = self.timing.sustain_level.max(0.000_1);
                self.level -= dt * span / release;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = AdsrStage::Idle;
                }
            }
        }
        self.level.clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------
// The playback engine: engine -> stereo output, the phase-2/3 `Voice` +
// `EngineRenderer` render path this binary streams through `cpal`.
// ---------------------------------------------------------------------

/// Owns the fixed-size voice pool and per-voice render state, and renders
/// one block at a time along the canonical `Voice` -> `EngineRenderer`
/// path. Every collection here is preallocated once in `new`; `render_block`
/// never grows or shrinks any of them, so it is safe to call from the audio
/// thread's real-time callback.
struct PlaybackEngine {
    allocator: VoiceAllocator,
    voice_renderer: VoiceRenderer,
    engine_renderer: EngineRenderer,
    oscillator: StandardOscillator,
    osc_config: OscillatorConfig,
    engine_sample_rate: EngineSampleRate,
    voice_states: Vec<VoiceRenderState<StateVariableFilter, SimpleAdsrEnvelope>>,
    envelope_timing: EnvelopeTiming,
    scratch: Vec<f64>,
    engine_out: Vec<AudioFrame>,
}

impl PlaybackEngine {
    fn new(block_len: usize) -> Self {
        let timing = EnvelopeTiming::new(0.01, 0.08, 0.7, 0.25);
        let voice_config = EngineVoiceConfig::new(timing);
        let allocator = VoiceAllocator::new(voice_config, POLYPHONY, StealPolicy::Oldest)
            .expect("POLYPHONY is nonzero");
        let voice_states = (0..POLYPHONY)
            .map(|_| {
                VoiceRenderState::new(
                    StateVariableFilter::new(),
                    SimpleAdsrEnvelope::new(timing, f64::from(SAMPLE_RATE_HZ)),
                )
            })
            .collect();
        let osc_config = OscillatorConfig::new(
            EngineWaveform::Saw,
            OscAmplitude::try_new(0.8).expect("0.8 is a valid amplitude"),
        );
        let engine_sample_rate = EngineSampleRate::try_new(f64::from(SAMPLE_RATE_HZ))
            .expect("44_100 is a valid sample rate");

        Self {
            allocator,
            voice_renderer: VoiceRenderer::new(),
            engine_renderer: EngineRenderer::new(),
            oscillator: StandardOscillator::new(),
            osc_config,
            engine_sample_rate,
            voice_states,
            envelope_timing: timing,
            scratch: vec![0.0; block_len],
            engine_out: vec![AudioFrame::silence(); block_len],
        }
    }

    /// Replaces the voice-render state at `index` with a freshly triggered
    /// one. `VoiceRenderState` only exposes its filter/envelope by shared
    /// reference, so triggering an *existing* instance in place is not
    /// possible from outside `engine::engine_renderer`; swapping in a new,
    /// already-triggered instance is the available alternative. Both
    /// `StateVariableFilter` and `SimpleAdsrEnvelope` hold only primitive
    /// `f64` fields (no heap-owned memory), so this swap -- and the drop of
    /// the old state it performs -- never allocates or frees heap memory.
    fn retrigger_voice_state(&mut self, index: usize) {
        if let Some(state) = self.voice_states.get_mut(index) {
            let mut envelope =
                SimpleAdsrEnvelope::new(self.envelope_timing, f64::from(SAMPLE_RATE_HZ));
            envelope.trigger();
            *state = VoiceRenderState::new(StateVariableFilter::new(), envelope);
        }
    }

    /// Renders exactly one block of the engine's stereo output.
    ///
    /// Real-time safe: `pending_triggers` is a fixed-size stack array
    /// (never a heap-allocated `Vec`), sized to `POLYPHONY`, so completing a
    /// deferred voice steal (see `VoiceAllocator::advance_all`) never
    /// allocates on this, the audio thread's, calling path.
    fn render_block(
        &mut self,
        dt_seconds: f64,
        snapshot: BridgeParameterSnapshot,
    ) -> Result<(), EngineRendererError> {
        let mut pending_triggers = [false; POLYPHONY];
        self.allocator.advance_all(dt_seconds, |index, event| {
            if let VoiceEvent::Triggered { .. } = event {
                if index < POLYPHONY {
                    pending_triggers[index] = true;
                }
            }
        });
        for (index, &pending) in pending_triggers.iter().enumerate() {
            if pending {
                self.retrigger_voice_state(index);
            }
        }

        let filter_config = FilterConfig::new(
            FilterKind::LowPass,
            4_000.0 + 4_000.0 * f64::from(snapshot.filter_cutoff.clamp(0.0, 1.0)),
            f64::from(snapshot.filter_resonance.clamp(0.0, 1.0)),
            f64::from(SAMPLE_RATE_HZ),
        );

        self.engine_renderer.render(
            &self.allocator,
            &self.voice_renderer,
            &self.oscillator,
            self.osc_config,
            filter_config,
            self.engine_sample_rate,
            &mut self.voice_states,
            &mut self.scratch,
            &mut self.engine_out,
        )
    }

    /// Applies a note-on/note-off boundary message to the voice pool.
    fn apply_boundary_message(&mut self, message: RtBoundaryMessage) {
        match message {
            RtBoundaryMessage::NoteOn {
                channel,
                note,
                velocity,
            } => {
                if let Ok(note_number) = EngineNoteNumber::try_new(note) {
                    let note_id = synthetic_note_id(channel, note);
                    let ratio = (f64::from(velocity) / 127.0).clamp(0.0, 1.0);
                    if let Ok(engine_velocity) = EngineVelocity::try_new(ratio) {
                        if let Ok(VoiceAssignment::Assigned { index }) =
                            self.allocator
                                .allocate(note_number, note_id, engine_velocity)
                        {
                            self.retrigger_voice_state(index);
                        }
                        // `VoiceAssignment::Stolen` completes later, once the
                        // victim voice reaches `Idle` inside `render_block`'s
                        // call to `advance_all`.
                    }
                }
            }
            RtBoundaryMessage::NoteOff { channel, note } => {
                let note_id = synthetic_note_id(channel, note);
                let _ = self.allocator.release(note_id);
            }
            RtBoundaryMessage::ParameterChange { .. } => {
                // This binary's fixed playback schedule never publishes a
                // `ParameterChange` message; parameter reads flow through
                // the `ParameterBridge` instead. Present for exhaustiveness
                // only.
            }
        }
    }
}

// ---------------------------------------------------------------------
// The real-time render callback: the audio thread's only entry point.
// ---------------------------------------------------------------------

/// Drives `PlaybackEngine` from the audio thread. Owns the consumer half of
/// the real-time boundary: the `RtrbEventRing` (note on/off), the
/// `TripleBufferParameterBridge` (continuously-varying parameters), and the
/// `basedrop`-backed deferred-deallocation retirer.
struct PlaybackCallback {
    engine: PlaybackEngine,
    event_ring: Arc<RtrbEventRing>,
    parameter_bridge: Arc<TripleBufferParameterBridge>,
    /// Real-time-side handle of the deferred-deallocation pipeline. This
    /// binary's fixed playback schedule never swaps in a voice-render state
    /// that owns heap memory (see `PlaybackEngine::retrigger_voice_state`),
    /// so nothing is ever handed to `retire` on this particular path --
    /// this field exists so the audio thread genuinely owns the same
    /// real-time pipeline object the `--no-device-dry-run` mode constructs
    /// and reports on, per this asset's contract.
    #[allow(dead_code)]
    _retirer: BasedropRetirer,
    block_len: usize,
}

impl PlaybackCallback {
    fn new(
        engine: PlaybackEngine,
        event_ring: Arc<RtrbEventRing>,
        parameter_bridge: Arc<TripleBufferParameterBridge>,
        retirer: BasedropRetirer,
        block_len: usize,
    ) -> Self {
        Self {
            engine,
            event_ring,
            parameter_bridge,
            _retirer: retirer,
            block_len,
        }
    }

    /// Drains every boundary message currently waiting on the event ring
    /// and applies it to the voice pool. This is the only place the
    /// `RtrbEventRing` is popped, matching the invariant that note events
    /// cross the real-time boundary through this seam alone.
    fn drain_event_ring(&mut self) {
        while let Some(message) = self.event_ring.pop() {
            self.engine.apply_boundary_message(message);
        }
    }
}

impl RenderCallback for PlaybackCallback {
    fn render(&mut self, output: &mut [f32]) {
        let expected_len = self.block_len * 2;
        if output.len() != expected_len {
            // An unexpected buffer length is a caller/host programming
            // error, not something to panic over on the audio thread: fail
            // safe to silence.
            output.fill(0.0);
            return;
        }

        self.drain_event_ring();

        let snapshot = self.parameter_bridge.read();
        let dt_seconds = self.block_len as f64 / f64::from(SAMPLE_RATE_HZ);

        if self.engine.render_block(dt_seconds, snapshot).is_err() {
            output.fill(0.0);
            return;
        }

        for (index, frame) in self.engine.engine_out.iter().enumerate() {
            output[index * 2] = frame.left() * snapshot.master_volume;
            output[index * 2 + 1] = frame.right() * snapshot.master_volume;
        }
    }
}

// ---------------------------------------------------------------------
// Scheduling: walks the timeline in wall-clock time on the non-real-time
// main thread, pushing note on/off messages across the `RtrbEventRing`.
// ---------------------------------------------------------------------

/// Walks `events` in wall-clock time from `start`, pushing each note
/// lifecycle event onto `event_ring` at its scheduled offset, then sleeps
/// out any remaining time up to `play_duration_seconds` so a final note's
/// release stage can ring out. Events beyond `play_duration_seconds` are
/// never scheduled, honoring an optional `--seconds` cap.
fn schedule_playback(
    event_ring: &RtrbEventRing,
    events: &[TimedMidiEvent],
    play_duration_seconds: f64,
) {
    let start = Instant::now();

    for event in events {
        if event.at_seconds() > play_duration_seconds {
            break;
        }
        let target = Duration::from_secs_f64(event.at_seconds().max(0.0));
        let elapsed = start.elapsed();
        if target > elapsed {
            thread::sleep(target - elapsed);
        }
        if let Some(message) = to_boundary_message(event.event()) {
            push_blocking(event_ring, message);
        }
    }

    let total = Duration::from_secs_f64(play_duration_seconds.max(0.0));
    let elapsed = start.elapsed();
    if total > elapsed {
        thread::sleep(total - elapsed);
    }
}

// ---------------------------------------------------------------------
// Run modes.
// ---------------------------------------------------------------------

/// Constructs the exact real-time pipeline objects the live path uses --
/// the `RtrbEventRing`, the `TripleBufferParameterBridge`, and the
/// `basedrop` deferred-deallocation retirer/collector pair, wired into a
/// `PlaybackEngine`/`PlaybackCallback` exactly as `run_live` would -- without
/// ever touching `cpal`'s host/device APIs and without blocking on the wall
/// clock. Returns the process exit code (always `0`).
fn run_dry_run(event_count: usize) -> i32 {
    let event_ring = Arc::new(RtrbEventRing::new(EVENT_RING_CAPACITY));
    let parameter_bridge = Arc::new(TripleBufferParameterBridge::new(
        BridgeParameterSnapshot::default(),
    ));
    let (retirer, mut collector) = basedrop_deferred_deallocator();

    let engine = PlaybackEngine::new(BLOCK_LEN);
    let callback = PlaybackCallback::new(
        engine,
        Arc::clone(&event_ring),
        Arc::clone(&parameter_bridge),
        retirer,
        BLOCK_LEN,
    );
    // Boxing as `dyn RenderCallback` is exactly what `run_live` hands to
    // `AudioOutput::open`; doing it here too proves, at compile time, that
    // the constructed pipeline satisfies the same `Send` real-time contract
    // the live path depends on.
    let callback: Box<dyn RenderCallback> = Box::new(callback);
    drop(callback);

    // Nothing has been retired yet on this path, but draining the
    // collector once exercises the background half of the pipeline the
    // live path's cleanup also drives.
    let _ = collector.collect();
    let _ = event_ring.capacity();

    println!("dry-run ok: pipeline constructed (events={event_count})");
    0
}

/// Resolves the default output device, opens it through `CpalAudioOutput`,
/// and streams `events` in real time for `play_duration_seconds`. Returns
/// the process exit code: non-zero (with a stderr message, never a panic)
/// if no output device is available or the stream fails to open.
fn run_live(events: Vec<TimedMidiEvent>, play_duration_seconds: f64) -> i32 {
    let host = cpal::default_host();
    let device_name = match host.default_output_device() {
        Some(device) => device
            .name()
            .unwrap_or_else(|_| "<unnamed output device>".to_string()),
        None => {
            eprintln!("midi_play_live: no default output device");
            return 1;
        }
    };

    println!(
        "device: {device_name}, events: {event_count}, duration: {play_duration_seconds:.2}s",
        event_count = events.len(),
    );

    let event_ring = Arc::new(RtrbEventRing::new(EVENT_RING_CAPACITY));
    let parameter_bridge = Arc::new(TripleBufferParameterBridge::new(
        BridgeParameterSnapshot::default(),
    ));
    let (retirer, mut collector) = basedrop_deferred_deallocator();

    let engine = PlaybackEngine::new(BLOCK_LEN);
    let callback = PlaybackCallback::new(
        engine,
        Arc::clone(&event_ring),
        Arc::clone(&parameter_bridge),
        retirer,
        BLOCK_LEN,
    );

    let audio_output = CpalAudioOutput::new();
    let sample_rate = match PortSampleRate::new(SAMPLE_RATE_HZ) {
        Some(rate) => rate,
        None => {
            eprintln!("midi_play_live: invalid sample rate {SAMPLE_RATE_HZ}");
            return 1;
        }
    };
    let buffer_size = match BufferSize::new(BLOCK_LEN as u32) {
        Some(size) => size,
        None => {
            eprintln!("midi_play_live: invalid buffer size {BLOCK_LEN}");
            return 1;
        }
    };

    let stream = match audio_output.open(sample_rate, buffer_size, Box::new(callback)) {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("midi_play_live: {err}");
            return 1;
        }
    };

    schedule_playback(&event_ring, &events, play_duration_seconds);

    audio_output.close(stream);
    let _ = collector.collect();
    0
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let cli = parse_args(&args);

    let events = match load_timeline(cli.file_path.as_ref()) {
        Ok(events) => events,
        Err(err) => {
            eprintln!("midi_play_live: {err}");
            std::process::exit(1);
        }
    };

    let natural_duration = timeline_duration_seconds(&events);
    let play_duration = cli
        .seconds_cap
        .map(|cap| natural_duration.min(cap))
        .unwrap_or(natural_duration);

    if cli.dry_run {
        std::process::exit(run_dry_run(events.len()));
    }

    std::process::exit(run_live(events, play_duration));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(raw: &[&str]) -> Vec<String> {
        std::iter::once("midi_play_live".to_string())
            .chain(raw.iter().map(|s| s.to_string()))
            .collect()
    }

    #[test]
    fn parse_args_defaults_to_no_file_no_cap_live_mode() {
        let cli = parse_args(&args(&[]));
        assert_eq!(
            cli,
            CliArgs {
                file_path: None,
                seconds_cap: None,
                dry_run: false,
            }
        );
    }

    #[test]
    fn parse_args_reads_a_positional_file_path() {
        let cli = parse_args(&args(&["song.mid"]));
        assert_eq!(cli.file_path, Some(PathBuf::from("song.mid")));
    }

    #[test]
    fn parse_args_reads_seconds_cap() {
        let cli = parse_args(&args(&["--seconds", "5.5"]));
        assert_eq!(cli.seconds_cap, Some(5.5));
    }

    #[test]
    fn parse_args_reads_dry_run_flag() {
        let cli = parse_args(&args(&["--no-device-dry-run"]));
        assert!(cli.dry_run);
    }

    #[test]
    fn parse_args_combines_file_seconds_and_dry_run_in_any_order() {
        let cli = parse_args(&args(&[
            "--seconds",
            "3",
            "song.mid",
            "--no-device-dry-run",
        ]));
        assert_eq!(cli.file_path, Some(PathBuf::from("song.mid")));
        assert_eq!(cli.seconds_cap, Some(3.0));
        assert!(cli.dry_run);
    }

    #[test]
    fn synthetic_note_id_is_stable_for_the_same_channel_and_note() {
        assert_eq!(synthetic_note_id(0, 60), synthetic_note_id(0, 60));
        assert_ne!(synthetic_note_id(0, 60), synthetic_note_id(1, 60));
        assert_ne!(synthetic_note_id(0, 60), synthetic_note_id(0, 61));
    }

    #[test]
    fn to_boundary_message_maps_note_on() {
        let event = MidiEvent::new(
            demo_channel_address(),
            MidiEventKind::NoteOn,
            KernelNoteNumber::try_new(60).unwrap(),
            KernelNoteId::new(1),
            KernelVelocity::from_midi7(100),
        );
        let message = to_boundary_message(&event).expect("note-on maps to a boundary message");
        assert_eq!(
            message,
            RtBoundaryMessage::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100,
            }
        );
    }

    #[test]
    fn to_boundary_message_maps_note_off() {
        let event = MidiEvent::new(
            demo_channel_address(),
            MidiEventKind::NoteOff,
            KernelNoteNumber::try_new(60).unwrap(),
            KernelNoteId::new(1),
            KernelVelocity::from_midi7(0),
        );
        let message = to_boundary_message(&event).expect("note-off maps to a boundary message");
        assert_eq!(
            message,
            RtBoundaryMessage::NoteOff {
                channel: 0,
                note: 60,
            }
        );
    }

    #[test]
    fn to_boundary_message_ignores_other_event_kinds() {
        let event = MidiEvent::new(
            demo_channel_address(),
            MidiEventKind::PolyPressure,
            KernelNoteNumber::try_new(60).unwrap(),
            KernelNoteId::new(1),
            KernelVelocity::from_midi7(64),
        );
        assert_eq!(to_boundary_message(&event), None);
    }

    #[test]
    fn demo_timeline_has_four_notes_each_with_an_on_and_off() {
        let events = demo_timeline();
        assert_eq!(events.len(), 8);
        let note_ons = events
            .iter()
            .filter(|e| *e.event().kind() == MidiEventKind::NoteOn)
            .count();
        let note_offs = events
            .iter()
            .filter(|e| *e.event().kind() == MidiEventKind::NoteOff)
            .count();
        assert_eq!(note_ons, 4);
        assert_eq!(note_offs, 4);
    }

    #[test]
    fn demo_timeline_events_are_non_negative_and_non_decreasing() {
        let events = demo_timeline();
        let mut last = 0.0;
        for event in &events {
            assert!(event.at_seconds() >= 0.0);
            assert!(event.at_seconds() >= last);
            last = event.at_seconds();
        }
    }

    #[test]
    fn timeline_duration_adds_the_release_tail_after_the_last_event() {
        let events = demo_timeline();
        let last = events
            .iter()
            .map(TimedMidiEvent::at_seconds)
            .fold(0.0, f64::max);
        let duration = timeline_duration_seconds(&events);
        assert!((duration - (last + RELEASE_TAIL_SECONDS)).abs() < 1e-9);
    }

    #[test]
    fn timeline_duration_of_empty_timeline_is_just_the_tail() {
        assert_eq!(timeline_duration_seconds(&[]), RELEASE_TAIL_SECONDS);
    }

    #[test]
    fn playback_engine_renders_silence_with_no_notes() {
        let mut engine = PlaybackEngine::new(8);
        let snapshot = BridgeParameterSnapshot::default();
        engine
            .render_block(1.0 / f64::from(SAMPLE_RATE_HZ), snapshot)
            .unwrap();
        for frame in &engine.engine_out {
            assert_eq!(*frame, AudioFrame::silence());
        }
    }

    #[test]
    fn playback_engine_renders_audible_output_after_a_note_on() {
        let mut engine = PlaybackEngine::new(BLOCK_LEN);
        engine.apply_boundary_message(RtBoundaryMessage::NoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
        });

        let dt_seconds = BLOCK_LEN as f64 / f64::from(SAMPLE_RATE_HZ);
        let snapshot = BridgeParameterSnapshot::default();
        let mut peak: f32 = 0.0;
        for _ in 0..20 {
            engine.render_block(dt_seconds, snapshot).unwrap();
            for frame in &engine.engine_out {
                peak = peak.max(frame.left().abs());
            }
        }

        assert!(
            peak > 0.0,
            "a triggered voice should produce nonzero output"
        );
    }

    #[test]
    fn dry_run_returns_success_without_touching_cpal_or_the_wall_clock() {
        let start = Instant::now();
        let code = run_dry_run(demo_timeline().len());
        assert_eq!(code, 0);
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "dry-run must return promptly, never blocking on the wall clock"
        );
    }
}
