// path: src/bin/synth_ui.rs

//! The standalone synthesizer application: opens the audio output and a
//! real window (via `eframe`/`egui`), renders the MIXER VIEW over 16
//! `ChannelStrip` channels, polls keyboard and gamepad for navigation, and
//! plays notes from external MIDI hardware and/or a MIDI file through the
//! full engine-to-mixer signal path.
//!
//! Modes:
//! - (no flags): opens a real audio output stream (via
//!   `port::Shell::AudioOutput`/`CpalAudioOutput`), a real window (via
//!   `eframe`), MIDI hardware input (via `port::Shell::MidiInput`/
//!   `MidirMidiInput`), and gamepad input (via `GilrsGamepadInput`), and
//!   runs the mixer-view event loop until the window is closed.
//! - `--play <FILE.mid>`: additionally loads and sequences a standard MIDI
//!   file through the engine, looping the file until quit.
//! - `--smoke`: headless self-check -- no window, no audio device, no MIDI
//!   device. Builds the full stack (dispatcher, engine, mixer, mixer view,
//!   design-system theme) and exercises it entirely in memory, printing
//!   the required stdout markers and exiting non-zero on any failure.
//! - `--autopilot [--seconds N]`: a real, non-hermetic end-to-end run that
//!   opens the real window and real audio device, drives a scripted
//!   sequence of MixerViewEvents and a built-in note sequence through the
//!   live engine, asserts in code that real audio played and that all 6
//!   channel strips fit on screen, then self-terminates.
//! - `--tour`: prints a captioned walkthrough of the mixer view's
//!   features. No validation is required for this flag in this increment.
//!
//! MIXER VIEW SCOPE (editor increment): this binary's GUI is the mixer
//! view only -- no view switching, no on-screen keyboard, no mouse/touch
//! input. All note performance comes from external MIDI hardware and/or
//! `--play`; the UI itself originates no notes.
//!
//! Several collaborators are not (yet) wired through their hexagonal
//! ports because the port's shape does not fit this binary's needs:
//! `port::Shell::AppWindow::run` takes only a plain `App{title,width,
//! height}` with no update-loop injection point, and
//! `port::Shell::GuiRenderer`'s `ViewState` models an unrelated, simpler
//! four-surface shell than the `aggregate.Mixer.MixerView` this binary
//! draws. Per this project's established convention for a port that does
//! not (yet) fit a consumer's needs, this binary drives `eframe`/`egui`
//! directly instead, while still using the real `port::Shell::AudioOutput`
//! (`CpalAudioOutput`) and `port::Shell::MidiInput` (`MidirMidiInput`)
//! adapters, whose shapes do fit.

use std::cell::UnsafeCell;
use std::env;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use eframe::egui;

use crest_synth::design_system::default_theme::DefaultTheme;
use crest_synth::design_system::rgba::Rgba;
use crest_synth::design_system::semantic_token::SemanticToken;
use crest_synth::design_system::theme::Theme;

use crest_synth::effects::chain_renderer::ChainRenderer;
use crest_synth::effects::effect_chain::EffectChain;
use crest_synth::effects::effect_processor::{AudioFrame as MixFrame, EffectProcessor};

use crest_synth::engine::engine_renderer::{EngineRenderer, VoiceRenderState};
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

use crest_synth::kernel::audio_frame::AudioFrame as KernelAudioFrame;
use crest_synth::kernel::midi_event_kind::MidiEventKind;

use crest_synth::mixer::channel_strip::{ChannelStrip, Decibel};
use crest_synth::mixer::mix_bus::MixBus;
use crest_synth::mixer::mix_engine::{Limiter, MasterSource, MixEngine, StripSource};
use crest_synth::mixer::mixer_view::{
    MixerParam, MixerView, MixerViewEvent, TOTAL_CHANNELS, VISIBLE_CHANNELS,
};

use crest_synth::patch::midi_dispatcher::{MidiAddress, MidiDispatcher, RoutablePatch};
use crest_synth::patch::patch::{ChannelMapping as PatchChannelMapping, PatchId};
use crest_synth::patch::patch_manager::PatchManager;

use crest_synth::real_time::parameter_bridge::{ParameterBridge, ParameterSnapshot};

use crest_synth::shell::audio_output::{
    AudioOutput, BufferSize, RenderCallback, SampleRate as PortSampleRate,
};
use crest_synth::shell::cpal_audio_output::{CpalAudioOutput, CpalStreamHandle};
use crest_synth::shell::gamepad_input::{GamepadAction, GamepadButton, GamepadInput};
use crest_synth::shell::gilrs_gamepad_input::GilrsGamepadInput;
use crest_synth::shell::midi_input::{
    EventCallback, MidiEvent as ShellMidiEvent, MidiEventKind as ShellMidiEventKind, MidiInput,
};
use crest_synth::shell::midi_normalizer::MidiNormalizer;
use crest_synth::shell::midir_midi_input::MidirMidiInput;

/// Fixed render sample rate used throughout this binary.
const SAMPLE_RATE_HZ: u32 = 44_100;
/// Frames rendered per block, both for the headless smoke check and for
/// the live audio stream's fixed buffer size.
const BLOCK_LEN: usize = 512;
/// Fixed voice pool size for the demo patch's engine.
const POLYPHONY: usize = 8;
/// How many seconds of audio `--smoke` renders before measuring peak.
const SMOKE_SECONDS: f64 = 2.0;

/// Fixed width, in logical pixels, of a single channel strip. Every strip
/// gets exactly this much width inside its own fixed-width sub-region --
/// never `ui.available_width()`, which would let the first strip consume
/// the whole window and push the rest off-screen.
const STRIP_WIDTH: f32 = 120.0;
/// Extra width reserved for row labels/gutter alongside the strips.
const LABEL_GUTTER: f32 = 100.0;
/// Default window width: wide enough for all 6 visible strips plus gutter.
const WINDOW_WIDTH: f32 = STRIP_WIDTH * VISIBLE_CHANNELS as f32 + LABEL_GUTTER;
/// Default window height.
const WINDOW_HEIGHT: f32 = 520.0;
/// Window during which a second Edit press counts as a double-tap.
const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(350);

/// The 10 `SemanticToken` variants this skin depends on, resolved through
/// the `DesignSystem::Theme` port. Used both by draw code (to pick
/// per-role colors) and by the `--smoke` theme self-check (to prove every
/// depended-on token resolves with no panic and no silent fallback).
const ALL_SEMANTIC_TOKENS: [SemanticToken; 10] = [
    SemanticToken::Background,
    SemanticToken::Foreground,
    SemanticToken::ForegroundMuted,
    SemanticToken::Accent,
    SemanticToken::Border,
    SemanticToken::FocusRing,
    SemanticToken::EditActive,
    SemanticToken::MeterLow,
    SemanticToken::MeterMid,
    SemanticToken::MeterHigh,
];

/// Converts a `Theme`-resolved `Rgba` to egui's native color at the single
/// point this skin touches a raw color. No other function in this binary
/// constructs a `Color32` from a literal.
fn to_color32(color: Rgba) -> egui::Color32 {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    egui::Color32::from_rgba_unmultiplied(
        channel(color.r()),
        channel(color.g()),
        channel(color.b()),
        channel(color.a()),
    )
}

// ---------------------------------------------------------------------
// A minimal, real-time-safe ADSR envelope generator.
//
// No concrete `EnvelopeGenerator` adapter exists yet in this crate's
// module tree (only the port and a private test double), so one is
// defined locally here, matching the project's established convention for
// not-yet-available collaborators (see `engine::voice`'s local
// `NoteId`/`NoteNumber`/`Velocity`).
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdsrStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// A simple sample-driven ADSR envelope. Never allocates, locks, or
/// blocks in `trigger`/`release`/`tick`, matching the `EnvelopeGenerator`
/// port's real-time safety contract.
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

/// Derives a stable engine-level `NoteId` from a raw MIDI channel/note
/// pair. This keeps the render-side voice bookkeeping free of a
/// heap-allocating lookup table: the same (channel, note) always maps to
/// the same engine `NoteId`, which is all `VoiceAllocator::release` needs
/// to find the right voice to release.
fn synthetic_engine_note_id(channel: u8, note: u8) -> EngineNoteId {
    EngineNoteId::new((u64::from(channel) << 8) | u64::from(note))
}

// ---------------------------------------------------------------------
// A minimal local lock-free, single-producer/single-consumer event ring
// for crossing the real-time boundary.
//
// `real_time::event_ring` / `real_time::rtrb_event_ring` are named in the
// project's module tree but are not yet reflected in this crate's module
// declarations (`real_time::mod` does not yet declare either), so -- per
// this project's convention for not-yet-available collaborators -- a
// minimal local ring is defined here instead of depending on an
// unpublished module path. The design mirrors the project's own
// documented approach for this exact seam: a fixed-capacity ring,
// allocated once at construction, where `push`/`pop` never allocate,
// lock, or block.
// ---------------------------------------------------------------------

/// A message crossing the real-time boundary between the UI/MIDI thread
/// (producer) and the audio thread (consumer). Plain data -- no
/// heap-owning fields -- so pushing or popping never allocates or
/// deallocates on either side.
#[derive(Debug, Clone, Copy, PartialEq)]
enum LocalBoundaryMessage {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
}

/// A fixed-capacity, single-producer/single-consumer lock-free ring
/// buffer carrying `LocalBoundaryMessage` values across the real-time
/// boundary. Exactly one thread may call `push` and exactly one thread
/// may call `pop`; both take `&self` so the ring can be shared behind an
/// `Arc`.
struct LocalEventRing {
    slots: Box<[UnsafeCell<MaybeUninit<LocalBoundaryMessage>>]>,
    mask: usize,
    /// Next index the consumer will read from. Written only by the consumer.
    head: AtomicUsize,
    /// Next index the producer will write to. Written only by the producer.
    tail: AtomicUsize,
}

// SAFETY: designed for exactly one producer thread and exactly one
// consumer thread operating concurrently. The `head`/`tail` atomics
// establish the happens-before edges that make the single `UnsafeCell`
// slot touched by each operation race-free: the producer only ever
// writes slots the consumer has already vacated (observed via an
// Acquire load of `head`), and the consumer only ever reads slots the
// producer has already published (observed via an Acquire load of
// `tail`).
unsafe impl Sync for LocalEventRing {}

impl LocalEventRing {
    /// Creates a new ring able to hold at least `capacity` messages,
    /// rounded up to the next power of two so index wrapping can use a
    /// bitmask. This is the only allocation this type ever performs.
    fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1).next_power_of_two();
        let slots = (0..capacity)
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            slots,
            mask: capacity - 1,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Publishes `message` for the consumer. Called only from the
    /// producer thread. Returns `false` without writing anything if the
    /// ring has no free slot.
    fn push(&self, message: LocalBoundaryMessage) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail.wrapping_sub(head) >= self.slots.len() {
            return false;
        }
        let index = tail & self.mask;
        // SAFETY: the capacity check above guarantees the producer is at
        // least one full lap behind the consumer's next read, and only
        // the producer ever writes slots.
        unsafe {
            (*self.slots[index].get()).write(message);
        }
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }

    /// Retrieves the next message, if one has been published. Called
    /// only from the consumer thread. Never blocks.
    fn pop(&self) -> Option<LocalBoundaryMessage> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head == tail {
            return None;
        }
        let index = head & self.mask;
        // SAFETY: `head != tail` means the producer has published this
        // slot, so the value is fully initialized. Only the consumer
        // ever reads or retires slots.
        let message = unsafe { (*self.slots[index].get()).assume_init_read() };
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(message)
    }
}

impl Drop for LocalEventRing {
    fn drop(&mut self) {
        // Drain any messages still in flight so their destructors run.
        // Safe to use plain loads: `Drop::drop` runs with exclusive
        // (`&mut self`) access, after both threads have stopped
        // touching the ring.
        let mut head = *self.head.get_mut();
        let tail = *self.tail.get_mut();
        while head != tail {
            let index = head & self.mask;
            unsafe {
                (*self.slots[index].get()).assume_init_drop();
            }
            head = head.wrapping_add(1);
        }
    }
}

// ---------------------------------------------------------------------
// Patch dispatch: a minimal `RoutablePatch` view used to drive
// `MidiDispatcher` without depending on a concrete `Patch` reference
// (dispatch only needs identity + channel mapping).
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct DispatchTarget {
    id: PatchId,
    mapping: PatchChannelMapping,
}

impl RoutablePatch for DispatchTarget {
    fn id(&self) -> PatchId {
        self.id
    }

    fn mapping(&self) -> PatchChannelMapping {
        self.mapping
    }
}

/// Creates one omni-mapped patch via `PatchManager` (the application
/// service that owns patch identity and lifecycle) and returns the
/// dispatch-routable view of it. Used by the headless engine self-checks,
/// which only need "some note reaches some patch" and are indifferent to
/// which MIDI channel carries it.
fn create_default_patch() -> DispatchTarget {
    let mut manager = PatchManager::default();
    let id = manager.create_patch();
    manager
        .set_mapping(id, PatchChannelMapping::omni())
        .expect("freshly created patch exists");
    let mapping = manager.get(id).expect("patch exists").mapping();
    DispatchTarget { id, mapping }
}

/// Creates 16 patches, one per MIDI channel 0..16, each mapped to exactly
/// its own channel address. This is the mixer wiring: mixer channel N
/// (0-indexed) addresses the same MIDI channel N the engine dispatches
/// against, so editing `MixerView` channel N's parameters and sending
/// MIDI on channel N always speak about the same channel strip.
fn create_channel_patches() -> Vec<DispatchTarget> {
    let mut manager = PatchManager::default();
    (0..TOTAL_CHANNELS)
        .filter_map(|channel| {
            let channel = u8::try_from(channel).ok()?;
            let id = manager.create_patch();
            let mapping = PatchChannelMapping::single(channel).ok()?;
            manager.set_mapping(id, mapping).ok()?;
            Some(DispatchTarget { id, mapping })
        })
        .collect()
}

// ---------------------------------------------------------------------
// MIDI injection: normalizes raw MIDI 1.0 bytes, dispatches them to the
// patches whose channel mapping matches (layering is intentional; leakage
// is not), and forwards note lifecycle events across the real-time
// boundary via the `EventRing`.
// ---------------------------------------------------------------------

struct MidiInjector {
    normalizer: MidiNormalizer,
    dispatcher: MidiDispatcher,
    dispatch_targets: Vec<DispatchTarget>,
    event_ring: Arc<LocalEventRing>,
    events_dispatched: u64,
}

impl MidiInjector {
    fn new(event_ring: Arc<LocalEventRing>, dispatch_targets: Vec<DispatchTarget>) -> Self {
        Self {
            normalizer: MidiNormalizer::new(),
            dispatcher: MidiDispatcher::new(),
            dispatch_targets,
            event_ring,
            events_dispatched: 0,
        }
    }

    /// Normalizes one raw MIDI 1.0 message, dispatches it to every
    /// matching patch, and -- if it is a note-on/off event and at least
    /// one patch matched -- forwards it across the `EventRing`.
    /// Malformed input and non-matching addresses are dropped silently,
    /// matching the ports' own no-panic contracts.
    fn inject(&mut self, bytes: &[u8]) {
        let event = match self.normalizer.normalize(bytes) {
            Ok(event) => event,
            Err(_) => return,
        };

        let channel = event.address().channel().value();
        let address = match MidiAddress::try_new(channel) {
            Ok(address) => address,
            Err(_) => return,
        };

        let matched = self.dispatcher.dispatch(address, &self.dispatch_targets);
        if matched.is_empty() {
            return;
        }
        self.events_dispatched += matched.len() as u64;

        let message = match event.kind() {
            MidiEventKind::NoteOn => LocalBoundaryMessage::NoteOn {
                channel,
                note: event.note().value(),
                velocity: event.velocity().to_midi7(),
            },
            MidiEventKind::NoteOff => LocalBoundaryMessage::NoteOff {
                channel,
                note: event.note().value(),
            },
            // This minimal composition only sequences note lifecycle
            // events through the engine; other kinds are dispatched (and
            // therefore counted above) but not forwarded further.
            _ => return,
        };

        let _ = self.event_ring.push(message);
    }
}

// ---------------------------------------------------------------------
// The synth stack: engine -> mixer render path shared identically by the
// headless `--smoke` check and the live audio render callback.
// ---------------------------------------------------------------------

struct SynthStack {
    allocator: VoiceAllocator,
    voice_renderer: VoiceRenderer,
    engine_renderer: EngineRenderer,
    oscillator: StandardOscillator,
    osc_config: OscillatorConfig,
    engine_sample_rate: EngineSampleRate,
    voice_states: Vec<VoiceRenderState<StateVariableFilter, SimpleAdsrEnvelope>>,
    envelope_timing: EnvelopeTiming,
    scratch: Vec<f64>,
    engine_out: Vec<KernelAudioFrame>,
    mix_input: Vec<MixFrame>,

    strip: ChannelStrip,
    strip_chain: EffectChain,
    strip_processors: Vec<Box<dyn EffectProcessor>>,

    master_bus: MixBus,
    master_chain: EffectChain,
    master_processors: Vec<Box<dyn EffectProcessor>>,
    limiter: Limiter,
    mix_engine: MixEngine,

    event_ring: Arc<LocalEventRing>,
    parameter_bridge: ParameterBridge,

    block_len: usize,
}

// SAFETY: every field of `SynthStack` is `Send`, except the two boxed
// `EffectProcessor` slots -- and this composition always leaves those two
// `Vec`s empty (no insert-chain processors are configured for the strip
// or the master bus). An empty `Vec` carries no data that could race
// across threads, so moving a `SynthStack` into the audio callback's
// owning thread is sound.
unsafe impl Send for SynthStack {}

impl SynthStack {
    fn new(block_len: usize, event_ring: Arc<LocalEventRing>) -> Self {
        let timing = EnvelopeTiming::new(0.01, 0.05, 0.8, 0.2);
        let voice_config = EngineVoiceConfig::new(timing);
        let allocator = VoiceAllocator::new(voice_config, POLYPHONY, StealPolicy::Oldest)
            .expect("polyphony is nonzero");
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
            OscAmplitude::try_new(0.9).expect("0.9 is a valid amplitude"),
        );
        let engine_sample_rate = EngineSampleRate::try_new(f64::from(SAMPLE_RATE_HZ))
            .expect("44100 is a valid sample rate");

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
            engine_out: vec![KernelAudioFrame::silence(); block_len],
            mix_input: vec![MixFrame::silence(); block_len],
            strip: ChannelStrip::new(),
            strip_chain: EffectChain::new(0),
            strip_processors: Vec::new(),
            master_bus: MixBus::new_master(),
            master_chain: EffectChain::new(0),
            master_processors: Vec::new(),
            limiter: Limiter::unity_ceiling(),
            mix_engine: MixEngine::new(ChainRenderer::new()),
            event_ring,
            parameter_bridge: ParameterBridge::new(ParameterSnapshot::default()),
            block_len,
        }
    }

    /// Drains every message waiting on the real-time boundary and applies
    /// it to the voice allocator. This is the only place the `EventRing`
    /// is popped, matching the invariant that event/parameter changes
    /// cross the boundary through this seam alone.
    fn drain_event_ring(&mut self) {
        while let Some(message) = self.event_ring.pop() {
            match message {
                LocalBoundaryMessage::NoteOn {
                    channel,
                    note,
                    velocity,
                } => {
                    if let Ok(note_number) = EngineNoteNumber::try_new(note) {
                        let note_id = synthetic_engine_note_id(channel, note);
                        let ratio = (f64::from(velocity) / 127.0).clamp(0.0, 1.0);
                        let velocity = EngineVelocity::try_new(ratio)
                            .unwrap_or_else(|_| EngineVelocity::try_new(0.0).expect("valid"));
                        // `Voice::trigger`/`release` (reached via the
                        // allocator) govern whether a voice is reclaimable,
                        // but `VoiceRenderState` exposes its filter/envelope
                        // only by shared reference, with no way to call
                        // `EnvelopeGenerator::trigger` on the existing
                        // instance. A freshly triggered envelope is swapped
                        // in via `retrigger_voice_state` instead, which is
                        // the only avenue this port's API leaves open.
                        if let Ok(VoiceAssignment::Assigned { index }) =
                            self.allocator.allocate(note_number, note_id, velocity)
                        {
                            self.retrigger_voice_state(index);
                        }
                    }
                }
                LocalBoundaryMessage::NoteOff { channel, note } => {
                    let note_id = synthetic_engine_note_id(channel, note);
                    let _ = self.allocator.release(note_id);
                }
            }
        }
    }

    /// Replaces the voice-render state at `index` with a freshly
    /// triggered one. `VoiceRenderState` only exposes its filter/envelope
    /// by shared reference, so triggering an *existing* instance in place
    /// is not possible from outside `engine::engine_renderer`; swapping in
    /// a new, already-triggered instance is the available alternative.
    fn retrigger_voice_state(&mut self, index: usize) {
        if let Some(state) = self.voice_states.get_mut(index) {
            let mut envelope =
                SimpleAdsrEnvelope::new(self.envelope_timing, f64::from(SAMPLE_RATE_HZ));
            envelope.trigger();
            *state = VoiceRenderState::new(StateVariableFilter::new(), envelope);
        }
    }

    /// Renders exactly one block through the canonical signal path:
    /// engine output -> channel strip inserts -> volume and pan -> master
    /// bus inserts -> limiter -> output. Used identically by the headless
    /// `--smoke` check and the live render callback.
    fn render_block(&mut self, dt_seconds: f64) -> Result<Vec<MixFrame>, String> {
        self.drain_event_ring();
        let mut pending_triggers: Vec<usize> = Vec::new();
        self.allocator.advance_all(dt_seconds, |index, event| {
            // A deferred steal completes here (the victim voice finally
            // reached `Idle`, so the allocator immediately re-triggers it
            // for the queued note) -- retrigger its render state exactly
            // as a fresh `Assigned` allocation would.
            if let VoiceEvent::Triggered { .. } = event {
                pending_triggers.push(index);
            }
        });
        for index in pending_triggers {
            self.retrigger_voice_state(index);
        }

        let snapshot = self.parameter_bridge.read();
        let filter_config = FilterConfig::new(
            FilterKind::LowPass,
            4_000.0 + 4_000.0 * f64::from(snapshot.filter_cutoff.clamp(0.0, 1.0)),
            f64::from(snapshot.filter_resonance.clamp(0.0, 1.0)),
            f64::from(SAMPLE_RATE_HZ),
        );

        self.engine_renderer
            .render(
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
            .map_err(|err| err.to_string())?;

        for (dst, src) in self.mix_input.iter_mut().zip(self.engine_out.iter()) {
            *dst = MixFrame::new(src.left(), src.right());
        }

        let master_volume = snapshot.master_volume;
        let block_len = self.block_len;

        let strip_source = StripSource {
            strip: &mut self.strip,
            inserts: &self.strip_chain,
            insert_processors: &mut self.strip_processors,
            input: &self.mix_input,
        };
        let mut strips = [strip_source];
        let mut master = MasterSource {
            bus: &self.master_bus,
            inserts: &self.master_chain,
            insert_processors: &mut self.master_processors,
            limiter: &self.limiter,
        };

        let mut mixed = self
            .mix_engine
            .render(block_len, &mut strips, &mut [], &mut master)
            .map_err(|err| err.to_string())?;

        for frame in mixed.iter_mut() {
            frame.left *= master_volume;
            frame.right *= master_volume;
        }

        Ok(mixed)
    }

    /// Exposes the primary channel strip's most recently metered peak
    /// level, for the `--smoke` "channel metered" self-check.
    fn strip_peak_value(&self) -> f32 {
        self.strip.peak().value()
    }
}

// ---------------------------------------------------------------------
// Live audio output: renders `SynthStack` through the real
// `port::Shell::AudioOutput` (`CpalAudioOutput`) adapter.
// ---------------------------------------------------------------------

/// Adapts `SynthStack` to the `RenderCallback` port. `cpal`'s negotiated
/// channel count is not surfaced by `AudioOutput::open`, so this callback
/// infers it each call from `output.len() / block_len` (cpal always hands
/// back exactly one fixed-size block's worth of frames per channel) and
/// fans the stack's stereo mix out to however many channels the device
/// actually opened.
struct StackRenderCallback {
    stack: SynthStack,
    block_len: usize,
    peak_bits: Arc<AtomicU32>,
}

impl RenderCallback for StackRenderCallback {
    fn render(&mut self, output: &mut [f32]) {
        let channels = output.len().checked_div(self.block_len).unwrap_or(0);
        if channels == 0 || channels * self.block_len != output.len() {
            output.fill(0.0);
            return;
        }

        let dt_seconds = self.block_len as f64 / f64::from(SAMPLE_RATE_HZ);
        match self.stack.render_block(dt_seconds) {
            Ok(frames) => {
                let mut peak: f32 = 0.0;
                for (index, frame) in frames.iter().enumerate() {
                    peak = peak.max(frame.left.abs()).max(frame.right.abs());
                    let base = index * channels;
                    output[base] = frame.left;
                    if channels > 1 {
                        output[base + 1] = frame.right;
                    }
                    for extra in output
                        .iter_mut()
                        .skip(base + 2)
                        .take(channels.saturating_sub(2))
                    {
                        *extra = frame.right;
                    }
                }
                // Track a running maximum across the whole run (never a
                // last-block value): a scripted note can decay to silence
                // well before the run's assertion point, and only a true
                // running max reflects "did real audio ever play".
                let mut observed = self.peak_bits.load(Ordering::Relaxed);
                loop {
                    if peak <= f32::from_bits(observed) {
                        break;
                    }
                    match self.peak_bits.compare_exchange_weak(
                        observed,
                        peak.to_bits(),
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(actual) => observed = actual,
                    }
                }
            }
            Err(_) => output.fill(0.0),
        }
    }
}

/// Opens the real-time audio stream via `port::Shell::AudioOutput`
/// (`CpalAudioOutput`). `stack` is moved into the render callback, which
/// the adapter drives on its own dedicated host thread.
fn open_audio_output(
    stack: SynthStack,
    block_len: usize,
    peak_bits: Arc<AtomicU32>,
) -> Result<CpalStreamHandle, String> {
    let adapter = CpalAudioOutput::new();
    let sample_rate =
        PortSampleRate::new(SAMPLE_RATE_HZ).ok_or_else(|| "invalid sample rate".to_string())?;
    let buffer_size =
        BufferSize::new(block_len as u32).ok_or_else(|| "invalid buffer size".to_string())?;
    let callback: Box<dyn RenderCallback> = Box::new(StackRenderCallback {
        stack,
        block_len,
        peak_bits,
    });
    adapter
        .open(sample_rate, buffer_size, callback)
        .map_err(|err| err.to_string())
}

// ---------------------------------------------------------------------
// Live MIDI hardware input: connects every visible port via the real
// `port::Shell::MidiInput` (`MidirMidiInput`) adapter, forwarding decoded
// events to the main thread over an `mpsc` channel (only `Send` data --
// never the adapter or its native connections -- crosses this boundary).
// ---------------------------------------------------------------------

/// Re-encodes a decoded `port::Shell::MidiInput` event back into raw MIDI
/// 1.0 bytes, so it can be fed through the same `MidiInjector::inject`
/// (normalize -> dispatch -> EventRing) path as `--play` and the gamepad.
fn raw_bytes_for(event: ShellMidiEvent) -> Option<Vec<u8>> {
    let channel = event.channel.value();
    Some(match event.kind {
        ShellMidiEventKind::NoteOn { note, velocity } => vec![0x90 | channel, note, velocity],
        ShellMidiEventKind::NoteOff { note, velocity } => vec![0x80 | channel, note, velocity],
        ShellMidiEventKind::ControlChange { controller, value } => {
            vec![0xB0 | channel, controller, value]
        }
        ShellMidiEventKind::PitchBend { value } => {
            let raw = (i32::from(value) + 8192).clamp(0, 16_383) as u16;
            vec![
                0xE0 | channel,
                (raw & 0x7F) as u8,
                ((raw >> 7) & 0x7F) as u8,
            ]
        }
        ShellMidiEventKind::ChannelPressure { value } => vec![0xD0 | channel, value],
        ShellMidiEventKind::PolyPressure { note, value } => vec![0xA0 | channel, note, value],
    })
}

/// Connects every MIDI input port currently visible to the host, if any,
/// forwarding decoded events to `sender`. Returns the adapter, which must
/// be kept alive for as long as the connections should stay open (it owns
/// the native connections internally); reports an empty port list rather
/// than erroring when no MIDI subsystem is available.
fn connect_midi_hardware(sender: mpsc::Sender<Vec<u8>>) -> Option<MidirMidiInput> {
    let backend = MidirMidiInput::new("crest-synth");
    let ports = backend.list_ports();
    if ports.is_empty() {
        println!("no hardware MIDI input ports found; use the gamepad or --play a file");
        return Some(backend);
    }
    for port in &ports {
        let tx = sender.clone();
        let callback: Box<dyn EventCallback> = Box::new(move |event: ShellMidiEvent| {
            if let Some(bytes) = raw_bytes_for(event) {
                let _ = tx.send(bytes);
            }
        });
        match backend.connect(port, callback) {
            Ok(_connection) => println!("connected MIDI input: {}", port.name()),
            Err(err) => eprintln!(
                "warning: could not connect MIDI port {}: {err}",
                port.name()
            ),
        }
    }
    Some(backend)
}

// ---------------------------------------------------------------------
// Keyboard/gamepad -> MixerViewEvent translation.
//
// Double-tap detection and Edit-hold timing live entirely here, never in
// MixerView: this input layer reads raw key/button state and emits the
// same semantic MixerViewEvents regardless of which device produced them.
// ---------------------------------------------------------------------

/// Tracks one input device's Edit-modifier gesture (hold = edit mode,
/// double-tap = toggle). Kept separate per device (keyboard, gamepad) so
/// a keyboard double-tap and a gamepad double-tap never interfere.
struct EditGesture {
    held: bool,
    last_press: Option<Instant>,
}

impl EditGesture {
    fn new() -> Self {
        Self {
            held: false,
            last_press: None,
        }
    }

    fn on_press(&mut self, now: Instant, out: &mut Vec<MixerViewEvent>) {
        if self.held {
            return;
        }
        self.held = true;
        let is_double_tap = self
            .last_press
            .is_some_and(|previous| now.duration_since(previous) < DOUBLE_TAP_WINDOW);
        self.last_press = Some(now);
        out.push(MixerViewEvent::EnterEditMode);
        if is_double_tap {
            out.push(MixerViewEvent::ToggleFocusedParam);
        }
    }

    fn on_release(&mut self, out: &mut Vec<MixerViewEvent>) {
        if !self.held {
            return;
        }
        self.held = false;
        out.push(MixerViewEvent::ExitEditMode);
    }
}

/// Reads raw egui key state for this frame and translates it into
/// semantic `MixerViewEvent`s: W/A/S/D on press edges, J hold/double-tap
/// via `edit`.
fn poll_keyboard_events(ctx: &egui::Context, edit: &mut EditGesture) -> Vec<MixerViewEvent> {
    let mut events = Vec::new();
    let now = Instant::now();
    let (up, down, left, right, edit_pressed, edit_released) = ctx.input(|input| {
        (
            input.key_pressed(egui::Key::W),
            input.key_pressed(egui::Key::S),
            input.key_pressed(egui::Key::A),
            input.key_pressed(egui::Key::D),
            input.key_pressed(egui::Key::J),
            input.key_released(egui::Key::J),
        )
    });
    if up {
        events.push(MixerViewEvent::NavUp);
    }
    if down {
        events.push(MixerViewEvent::NavDown);
    }
    if left {
        events.push(MixerViewEvent::NavLeft);
    }
    if right {
        events.push(MixerViewEvent::NavRight);
    }
    if edit_pressed {
        edit.on_press(now, &mut events);
    }
    if edit_released {
        edit.on_release(&mut events);
    }
    events
}

/// Translates gamepad actions observed since the last poll into the
/// identical `MixerViewEvent` vocabulary the keyboard adapter emits:
/// d-pad -> Nav events, South -> Edit hold/double-tap.
fn poll_gamepad_events(
    gamepad: &mut GilrsGamepadInput,
    edit: &mut EditGesture,
) -> Vec<MixerViewEvent> {
    let mut events = Vec::new();
    let now = Instant::now();
    for action in gamepad.poll() {
        match action {
            GamepadAction::ButtonPressed {
                button: GamepadButton::DPadUp,
                ..
            } => events.push(MixerViewEvent::NavUp),
            GamepadAction::ButtonPressed {
                button: GamepadButton::DPadDown,
                ..
            } => events.push(MixerViewEvent::NavDown),
            GamepadAction::ButtonPressed {
                button: GamepadButton::DPadLeft,
                ..
            } => events.push(MixerViewEvent::NavLeft),
            GamepadAction::ButtonPressed {
                button: GamepadButton::DPadRight,
                ..
            } => events.push(MixerViewEvent::NavRight),
            GamepadAction::ButtonPressed {
                button: GamepadButton::South,
                ..
            } => edit.on_press(now, &mut events),
            GamepadAction::ButtonReleased {
                button: GamepadButton::South,
                ..
            } => edit.on_release(&mut events),
            _ => {}
        }
    }
    events
}

// ---------------------------------------------------------------------
// Minimal standard MIDI file (SMF) sequencing.
//
// `midi_file::midi_file_reader::MidiFileReader` (and the `Song` /
// `TimedEvent` types it would return) are named in this asset's file
// pattern but are not yet reflected in this crate's module declarations,
// so -- per this project's convention for not-yet-available
// collaborators -- a minimal local reader is defined here instead of
// guessing at an unpublished API.
// ---------------------------------------------------------------------

/// Reads a variable-length quantity starting at `pos`, returning its value
/// and the position immediately after it.
fn read_varlen(data: &[u8], mut pos: usize) -> Result<(u32, usize), String> {
    let mut value: u32 = 0;
    loop {
        let byte = *data
            .get(pos)
            .ok_or_else(|| "truncated variable-length quantity".to_string())?;
        pos += 1;
        value = (value << 7) | u32::from(byte & 0x7F);
        if byte & 0x80 == 0 {
            break;
        }
    }
    Ok((value, pos))
}

fn ticks_to_samples(
    tick: u64,
    ticks_per_quarter: u32,
    micros_per_quarter: u32,
    sample_rate_hz: u32,
) -> u64 {
    let seconds = (tick as f64 * f64::from(micros_per_quarter))
        / (f64::from(ticks_per_quarter) * 1_000_000.0);
    (seconds * f64::from(sample_rate_hz)) as u64
}

/// Loads every track chunk of a standard MIDI file (format 0 or 1) and
/// returns every channel-voice message it contains as
/// `(sample_offset, raw_bytes)` pairs, sorted by sample offset. Meta and
/// sysex events are skipped, except a tempo meta event (`0xFF 0x51 03`),
/// which is honored for tick-to-sample conversion.
fn load_smf_events(path: &Path, sample_rate_hz: u32) -> Result<Vec<(u64, Vec<u8>)>, String> {
    let data = std::fs::read(path).map_err(|err| err.to_string())?;
    if data.len() < 14 || data[0..4] != b"MThd"[..] {
        return Err("not a standard MIDI file (missing MThd header)".to_string());
    }
    let header_len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
    if header_len < 6 || data.len() < 8 + header_len {
        return Err("truncated MThd header".to_string());
    }
    let division = u16::from_be_bytes([data[8 + 4], data[8 + 5]]);
    if division & 0x8000 != 0 {
        return Err("SMPTE-based division is not supported".to_string());
    }
    let ticks_per_quarter = u32::from(division.max(1));

    let mut offset = 8 + header_len;
    let mut micros_per_quarter: u32 = 500_000; // default 120 BPM
    let mut events: Vec<(u64, Vec<u8>)> = Vec::new();

    while offset + 8 <= data.len() {
        let is_track = data[offset..offset + 4] == b"MTrk"[..];
        let chunk_len = u32::from_be_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        let chunk_start = offset + 8;
        let chunk_end = (chunk_start + chunk_len).min(data.len());

        if !is_track {
            offset = chunk_end;
            continue;
        }

        let mut pos = chunk_start;
        let mut tick: u64 = 0;
        let mut running_status: Option<u8> = None;

        while pos < chunk_end {
            let (delta, next) = read_varlen(&data, pos)?;
            pos = next;
            tick += u64::from(delta);

            if pos >= chunk_end {
                break;
            }
            let mut status = data[pos];
            if status < 0x80 {
                status =
                    running_status.ok_or_else(|| "data byte with no running status".to_string())?;
            } else {
                pos += 1;
            }

            if status == 0xFF {
                let meta_type = *data
                    .get(pos)
                    .ok_or_else(|| "truncated meta event".to_string())?;
                pos += 1;
                let (len, next) = read_varlen(&data, pos)?;
                pos = next;
                let len = len as usize;
                if meta_type == 0x51 && len == 3 && pos + 3 <= chunk_end {
                    micros_per_quarter = (u32::from(data[pos]) << 16)
                        | (u32::from(data[pos + 1]) << 8)
                        | u32::from(data[pos + 2]);
                }
                pos += len;
                running_status = None;
                continue;
            }
            if status == 0xF0 || status == 0xF7 {
                let (len, next) = read_varlen(&data, pos)?;
                pos = next + len as usize;
                running_status = None;
                continue;
            }

            running_status = Some(status);
            let data_len = match status & 0xF0 {
                0xC0 | 0xD0 => 1,
                _ => 2,
            };
            if pos + data_len > chunk_end {
                break;
            }
            let mut bytes = vec![status];
            bytes.extend_from_slice(&data[pos..pos + data_len]);
            pos += data_len;

            if matches!(status & 0xF0, 0x80 | 0x90) {
                let sample =
                    ticks_to_samples(tick, ticks_per_quarter, micros_per_quarter, sample_rate_hz);
                events.push((sample, bytes));
            }
        }

        // Every track chunk is sequenced and merged into one timeline --
        // standard MIDI format 1 files commonly split the tempo/meta
        // information into track 0 and the actual channel-voice events
        // (the melody) into later tracks, so stopping after the first
        // chunk would silently drop all note data for such files.
        offset = chunk_end;
    }

    events.sort_by_key(|(sample, _)| *sample);

    // Rebase onto the first event so that any silent lead-in before the
    // first note (common in real-world song files) does not eat into a
    // caller's fixed-length "first seconds" playback window.
    if let Some(&(first_sample, _)) = events.first() {
        for event in events.iter_mut() {
            event.0 -= first_sample;
        }
    }

    Ok(events)
}

// ---------------------------------------------------------------------
// Modes.
// ---------------------------------------------------------------------

fn synthetic_note_schedule() -> Vec<(u64, Vec<u8>)> {
    vec![
        (0, vec![0x90, 60, 100]),
        (
            (f64::from(SAMPLE_RATE_HZ) * (SMOKE_SECONDS - 0.2)) as u64,
            vec![0x80, 60, 0],
        ),
    ]
}

/// Headless self-check: no window, no audio device, no MIDI device.
/// Builds the full stack (dispatcher, engine, mixer, mixer view, design
/// system) and exercises every layer entirely in memory, printing the
/// stdout markers the wave gate checks for and returning the process exit
/// code.
fn run_smoke(play_path: Option<PathBuf>) -> i32 {
    // THEME SELF-CHECK: proves the design-system seam is wired and
    // exhaustive without opening a window.
    let theme = DefaultTheme::new();
    ALL_SEMANTIC_TOKENS.iter().for_each(|&token| {
        let _ = theme.color(token);
    });
    let tokens_resolved = ALL_SEMANTIC_TOKENS.len();
    println!("theme tokens resolved: {tokens_resolved}");

    // UI-SMOKE: construct the mixer-view app state -- MixerView over its
    // seeded 16 ChannelStrip channels -- without a window, audio device,
    // or MIDI device, and drive a few events through it to confirm the
    // loop is wired end to end.
    let mut mixer_view = MixerView::new();
    mixer_view.apply(MixerViewEvent::NavRight);
    mixer_view.apply(MixerViewEvent::EnterEditMode);
    mixer_view.apply(MixerViewEvent::NavRight);
    mixer_view.apply(MixerViewEvent::ExitEditMode);
    let mixer_wired = mixer_view.channels().len() == TOTAL_CHANNELS;
    println!("ui smoke ok: app constructed");

    // AUDIO RENDER SELF-CHECK: apply a synthetic note-on (middle C, full
    // velocity) and render through the exact same render path the live
    // RenderCallback uses.
    let render_check_ring = Arc::new(LocalEventRing::new(8));
    let render_check_target = create_default_patch();
    let mut render_check_injector =
        MidiInjector::new(Arc::clone(&render_check_ring), vec![render_check_target]);
    let mut render_check_stack = SynthStack::new(BLOCK_LEN, render_check_ring);
    render_check_injector.inject(&[0x90, 60, 127]);
    let dt_seconds = BLOCK_LEN as f64 / f64::from(SAMPLE_RATE_HZ);
    let mut render_peak: f32 = 0.0;
    for _ in 0..10 {
        if let Ok(frames) = render_check_stack.render_block(dt_seconds) {
            for frame in frames {
                render_peak = render_peak.max(frame.left.abs()).max(frame.right.abs());
            }
        }
    }
    let non_silent = render_peak > 0.0;
    println!("render non-silent: {non_silent}");
    let channel_metered = render_check_stack.strip_peak_value() > 0.0;
    println!("channel metered: {channel_metered}");

    // ENGINE SELF-CHECK (pre-existing): sequence the first seconds of
    // `play_path` (or a synthetic note) through the full stack and
    // measure the peak and dispatched-event count.
    let event_ring = Arc::new(LocalEventRing::new(64));
    let dispatch_target = create_default_patch();
    let mut injector = MidiInjector::new(Arc::clone(&event_ring), vec![dispatch_target]);
    let mut stack = SynthStack::new(BLOCK_LEN, event_ring);

    let scheduled: Vec<(u64, Vec<u8>)> = match &play_path {
        Some(path) => match load_smf_events(path, SAMPLE_RATE_HZ) {
            Ok(events) if !events.is_empty() => events,
            Ok(_) => {
                eprintln!(
                    "warning: {} contained no channel-voice events; using a synthetic note",
                    path.display()
                );
                synthetic_note_schedule()
            }
            Err(err) => {
                eprintln!(
                    "warning: could not load {}: {err}; using a synthetic note",
                    path.display()
                );
                synthetic_note_schedule()
            }
        },
        None => synthetic_note_schedule(),
    };

    let total_samples = (f64::from(SAMPLE_RATE_HZ) * SMOKE_SECONDS) as u64;
    let mut sample_cursor: u64 = 0;
    let mut schedule_index = 0usize;
    let mut peak: f32 = 0.0;

    while sample_cursor < total_samples {
        while schedule_index < scheduled.len() && scheduled[schedule_index].0 <= sample_cursor {
            injector.inject(&scheduled[schedule_index].1);
            schedule_index += 1;
        }

        match stack.render_block(dt_seconds) {
            Ok(frames) => {
                for frame in frames {
                    peak = peak.max(frame.left.abs()).max(frame.right.abs());
                }
            }
            Err(err) => {
                eprintln!("render error: {err}");
                return 1;
            }
        }

        sample_cursor += BLOCK_LEN as u64;
    }

    println!("peak={peak}");
    println!("events={}", injector.events_dispatched);

    if peak > 0.05
        && peak <= 1.0
        && injector.events_dispatched > 0
        && non_silent
        && channel_metered
        && mixer_wired
        && tokens_resolved == ALL_SEMANTIC_TOKENS.len()
    {
        0
    } else {
        1
    }
}

// ---------------------------------------------------------------------
// The live mixer-view application: a real `eframe::App` driving the
// window/render/audio/MIDI/gamepad loop described above.
// ---------------------------------------------------------------------

fn autopilot_event_script() -> Vec<MixerViewEvent> {
    let mut script = Vec::new();
    for _ in 0..(TOTAL_CHANNELS * 2) {
        script.push(MixerViewEvent::NavRight);
    }
    for _ in 0..(TOTAL_CHANNELS * 2) {
        script.push(MixerViewEvent::NavLeft);
    }
    script.push(MixerViewEvent::EnterEditMode);
    script.push(MixerViewEvent::NavRight); // fine nudge
    script.push(MixerViewEvent::NavUp); // coarse nudge
    script.push(MixerViewEvent::ExitEditMode);
    script.push(MixerViewEvent::NavDown); // Volume -> Pan
    script.push(MixerViewEvent::NavDown); // Pan -> Mute
    script.push(MixerViewEvent::ToggleFocusedParam); // toggle Mute
    script.push(MixerViewEvent::NavDown); // Mute -> Solo
    script.push(MixerViewEvent::ToggleFocusedParam); // toggle Solo
    script
}

fn autopilot_note_schedule() -> Vec<(f64, Vec<u8>)> {
    vec![
        (0.10, vec![0x90, 60, 100]),
        (0.30, vec![0x91, 64, 100]),
        (0.60, vec![0x80, 60, 0]),
        (0.90, vec![0x92, 67, 100]),
        (1.20, vec![0x81, 64, 0]),
        (1.50, vec![0x82, 67, 0]),
    ]
}

fn print_tour_captions() {
    for caption in [
        "tour: W/A/S/D or the d-pad navigates channels and parameter rows",
        "tour: hold J (or gamepad South) to enter edit mode; nudge with A/D (fine) or W/S (coarse)",
        "tour: double-tap J (or South) to toggle the focused Mute/Solo row",
        "tour: the viewport edge-scrolls once the cursor reaches channel 6 or channel 11",
    ] {
        println!("{caption}");
    }
}

/// Owns the mixer-view application state and drives its one-way event
/// loop: input adapters emit `MixerViewEvent`s, `MixerView::apply` is the
/// only thing that mutates state, and draw code is a pure skin that reads
/// state and resolves every color through the injected `DefaultTheme`.
struct MixerUiApp {
    theme: DefaultTheme,
    mixer_view: MixerView,
    keyboard_edit: EditGesture,
    gamepad_edit: EditGesture,
    gamepad: Option<GilrsGamepadInput>,
    midi_rx: Option<mpsc::Receiver<Vec<u8>>>,
    _midi_backend: Option<MidirMidiInput>,
    injector: MidiInjector,
    channel_activity: [f32; TOTAL_CHANNELS],
    schedule: Vec<(u64, Vec<u8>)>,
    schedule_index: usize,
    schedule_last_position: u64,
    schedule_start: Instant,
    sample_rate_hz: u32,
    audio_peak_bits: Arc<AtomicU32>,
    _audio_stream: Option<CpalStreamHandle>,

    autopilot: bool,
    autopilot_seconds: f64,
    autopilot_start: Instant,
    autopilot_script_applied: bool,
    autopilot_events_applied: u64,
    autopilot_note_index: usize,
    autopilot_finished: bool,
    autopilot_strips_visible: usize,

    tour: bool,
    tour_printed: bool,
}

impl MixerUiApp {
    #[allow(clippy::too_many_arguments)]
    fn new(
        injector: MidiInjector,
        schedule: Vec<(u64, Vec<u8>)>,
        gamepad: Option<GilrsGamepadInput>,
        midi_rx: Option<mpsc::Receiver<Vec<u8>>>,
        midi_backend: Option<MidirMidiInput>,
        audio_stream: Option<CpalStreamHandle>,
        audio_peak_bits: Arc<AtomicU32>,
        autopilot: bool,
        autopilot_seconds: f64,
        tour: bool,
    ) -> Self {
        Self {
            theme: DefaultTheme::new(),
            mixer_view: MixerView::new(),
            keyboard_edit: EditGesture::new(),
            gamepad_edit: EditGesture::new(),
            gamepad,
            midi_rx,
            _midi_backend: midi_backend,
            injector,
            channel_activity: [0.0; TOTAL_CHANNELS],
            schedule,
            schedule_index: 0,
            schedule_last_position: 0,
            schedule_start: Instant::now(),
            sample_rate_hz: SAMPLE_RATE_HZ,
            audio_peak_bits,
            _audio_stream: audio_stream,
            autopilot,
            autopilot_seconds,
            autopilot_start: Instant::now(),
            autopilot_script_applied: false,
            autopilot_events_applied: 0,
            autopilot_note_index: 0,
            autopilot_finished: false,
            autopilot_strips_visible: 0,
            tour,
            tour_printed: false,
        }
    }

    fn note_activity_from_bytes(&mut self, bytes: &[u8]) {
        if let Some(&status) = bytes.first() {
            let channel = usize::from(status & 0x0F);
            let is_note_on = status & 0xF0 == 0x90 && bytes.get(2).copied().unwrap_or(0) > 0;
            if is_note_on {
                if let Some(level) = self.channel_activity.get_mut(channel) {
                    *level = 1.0;
                }
            }
        }
    }

    fn drive_schedule(&mut self) {
        if self.schedule.is_empty() {
            return;
        }
        let elapsed_samples =
            (self.schedule_start.elapsed().as_secs_f64() * f64::from(self.sample_rate_hz)) as u64;
        let cycle_samples = self
            .schedule
            .last()
            .map(|(sample, _)| sample + u64::from(self.sample_rate_hz))
            .unwrap_or(1)
            .max(1);
        let position = elapsed_samples % cycle_samples;
        if position < self.schedule_last_position {
            self.schedule_index = 0;
        }
        self.schedule_last_position = position;

        while self.schedule_index < self.schedule.len()
            && self.schedule[self.schedule_index].0 <= position
        {
            let bytes = self.schedule[self.schedule_index].1.clone();
            self.note_activity_from_bytes(&bytes);
            self.injector.inject(&bytes);
            self.schedule_index += 1;
        }
    }

    /// Paints one frame of the mixer view and returns how many of the
    /// fixed-width channel strips fit within the panel's available width
    /// -- the same measurement `--autopilot` asserts equals
    /// `VISIBLE_CHANNELS`.
    fn draw(&mut self, ctx: &egui::Context) -> usize {
        let theme = self.theme;
        let mixer_view = &self.mixer_view;
        let channel_activity = self.channel_activity;
        let mut strips_that_fit = 0usize;

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(to_color32(theme.color(SemanticToken::Background))))
            .show(ctx, |ui| {
                let panel_width = ui.available_width();
                strips_that_fit =
                    ((panel_width / STRIP_WIDTH).floor() as usize).min(VISIBLE_CHANNELS);

                ui.horizontal(|ui| {
                    for absolute_index in mixer_view.visible_range() {
                        draw_channel_strip(
                            ui,
                            &theme,
                            mixer_view,
                            absolute_index,
                            channel_activity[absolute_index],
                        );
                        ui.add(egui::Separator::default().vertical());
                    }
                });
            });

        strips_that_fit
    }

    fn run_autopilot_tick(&mut self, ctx: &egui::Context) {
        if !self.autopilot_script_applied {
            for event in autopilot_event_script() {
                self.mixer_view.apply(event);
                self.autopilot_events_applied += 1;
            }
            self.autopilot_script_applied = true;
        }

        let elapsed = self.autopilot_start.elapsed().as_secs_f64();
        let notes = autopilot_note_schedule();
        while self.autopilot_note_index < notes.len()
            && notes[self.autopilot_note_index].0 <= elapsed
        {
            let bytes = notes[self.autopilot_note_index].1.clone();
            self.note_activity_from_bytes(&bytes);
            self.injector.inject(&bytes);
            self.autopilot_note_index += 1;
        }

        if elapsed >= self.autopilot_seconds && !self.autopilot_finished {
            self.autopilot_finished = true;

            let peak = f32::from_bits(self.audio_peak_bits.load(Ordering::Relaxed));
            println!("autopilot audio peak: {peak}");
            assert!(
                peak > 0.0,
                "autopilot: real device-bound audio peak was 0.0 (silent output)"
            );

            println!(
                "autopilot strips visible: {}",
                self.autopilot_strips_visible
            );
            assert_eq!(
                self.autopilot_strips_visible, VISIBLE_CHANNELS,
                "autopilot: only {} of {VISIBLE_CHANNELS} channel strips fit on screen",
                self.autopilot_strips_visible
            );

            println!(
                "autopilot complete: {} events",
                self.autopilot_events_applied
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for MixerUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.tour && !self.tour_printed {
            print_tour_captions();
            self.tour_printed = true;
        }

        let mut events = poll_keyboard_events(ctx, &mut self.keyboard_edit);
        if let Some(gamepad) = self.gamepad.as_mut() {
            events.extend(poll_gamepad_events(gamepad, &mut self.gamepad_edit));
        }
        for event in &events {
            self.mixer_view.apply(*event);
        }

        let mut incoming_midi = Vec::new();
        if let Some(rx) = &self.midi_rx {
            while let Ok(bytes) = rx.try_recv() {
                incoming_midi.push(bytes);
            }
        }
        for bytes in incoming_midi {
            self.note_activity_from_bytes(&bytes);
            self.injector.inject(&bytes);
        }

        self.drive_schedule();

        for level in self.channel_activity.iter_mut() {
            *level *= 0.9;
        }

        let strips_visible = self.draw(ctx);

        if self.autopilot {
            self.autopilot_strips_visible = strips_visible;
            self.run_autopilot_tick(ctx);
        }

        ctx.request_repaint();
    }
}

/// Draws one fixed-width channel strip: Volume (a level strip that both
/// sets volume and meters live peak), Reverb send, Echo send, Pan, Mute,
/// Solo. Every color is resolved through `theme`; the only raw-color
/// touch is `to_color32` converting the theme's `Rgba` at the point of
/// use.
fn draw_channel_strip(
    ui: &mut egui::Ui,
    theme: &DefaultTheme,
    mixer_view: &MixerView,
    absolute_index: usize,
    activity: f32,
) {
    let Some(strip) = mixer_view.channel(absolute_index) else {
        return;
    };
    let is_cursor_channel = absolute_index == mixer_view.cursor_channel();
    let edit_mode = mixer_view.edit_mode();
    let cursor_param = mixer_view.cursor_param();

    ui.allocate_ui_with_layout(
        egui::vec2(STRIP_WIDTH, ui.available_height()),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.set_width(STRIP_WIDTH);
            let text_default = to_color32(theme.color(SemanticToken::Foreground));
            ui.colored_label(text_default, format!("Ch {}", absolute_index + 1));

            let volume_focused = is_cursor_channel && cursor_param == MixerParam::Volume;
            let volume_frac = ((strip.volume_db().value() - Decibel::MIN)
                / (Decibel::MAX - Decibel::MIN))
                .clamp(0.0, 1.0);
            let meter_frac = (volume_frac * (0.35 + 0.65 * activity)).clamp(0.0, 1.0);

            let box_color = if volume_focused && edit_mode {
                to_color32(theme.color(SemanticToken::EditActive))
            } else if volume_focused {
                to_color32(theme.color(SemanticToken::FocusRing))
            } else {
                to_color32(theme.color(SemanticToken::Border))
            };
            // The value fill (the fader's own resting position) uses the
            // accent token; the live peak overlay on top of it escalates
            // through the meter's low/mid/high tokens as level rises, like
            // a conventional green/yellow/red level meter.
            let fill_color = to_color32(theme.color(SemanticToken::Accent));
            let meter_color = if meter_frac > 0.85 {
                to_color32(theme.color(SemanticToken::MeterHigh))
            } else if meter_frac > 0.5 {
                to_color32(theme.color(SemanticToken::MeterMid))
            } else {
                to_color32(theme.color(SemanticToken::MeterLow))
            };

            let (meter_rect_full, _response) =
                ui.allocate_exact_size(egui::vec2(24.0, 96.0), egui::Sense::hover());
            ui.painter()
                .rect_stroke(meter_rect_full, 2.0, egui::Stroke::new(2.0, box_color));
            let fill_height = meter_rect_full.height() * volume_frac;
            let fill_rect = egui::Rect::from_min_max(
                egui::pos2(meter_rect_full.min.x, meter_rect_full.max.y - fill_height),
                meter_rect_full.max,
            );
            ui.painter().rect_filled(fill_rect, 0.0, fill_color);
            let live_height = meter_rect_full.height() * meter_frac;
            let live_rect = egui::Rect::from_min_max(
                egui::pos2(meter_rect_full.min.x, meter_rect_full.max.y - live_height),
                meter_rect_full.max,
            );
            ui.painter().rect_filled(live_rect, 0.0, meter_color);
            ui.colored_label(
                text_default,
                format!("{:+.1} dB", strip.volume_db().value()),
            );

            let text_muted = to_color32(theme.color(SemanticToken::ForegroundMuted));
            let reverb_send = strip.send(0).map(|tap| tap.level.value()).unwrap_or(0.0);
            ui.colored_label(text_muted, format!("Rev {reverb_send:.2}"));
            let echo_send = strip.send(1).map(|tap| tap.level.value()).unwrap_or(0.0);
            ui.colored_label(text_muted, format!("Echo {echo_send:.2}"));

            let pan_focused = is_cursor_channel && cursor_param == MixerParam::Pan;
            let pan_color = if pan_focused && edit_mode {
                to_color32(theme.color(SemanticToken::EditActive))
            } else if pan_focused {
                to_color32(theme.color(SemanticToken::FocusRing))
            } else {
                text_default
            };
            ui.colored_label(pan_color, format!("Pan {:+.2}", strip.pan().value()));

            let off_color = to_color32(theme.color(SemanticToken::ForegroundMuted));

            let mute_focused = is_cursor_channel && cursor_param == MixerParam::Mute;
            let mute_on_color = to_color32(theme.color(SemanticToken::MeterHigh));
            let mute_color = if strip.mute() {
                mute_on_color
            } else {
                off_color
            };
            ui.colored_label(mute_color, if mute_focused { "> MUTE" } else { "MUTE" });

            let solo_focused = is_cursor_channel && cursor_param == MixerParam::Solo;
            let solo_on_color = to_color32(theme.color(SemanticToken::Accent));
            let solo_color = if strip.solo() {
                solo_on_color
            } else {
                off_color
            };
            ui.colored_label(solo_color, if solo_focused { "> SOLO" } else { "SOLO" });
        },
    );
}

/// Assembles and runs the live mixer-view application: opens the real
/// audio output, connects real MIDI hardware, opens the gamepad, and runs
/// the `eframe` window's event loop until the window is closed (or, in
/// `--autopilot` mode, until the scripted run self-terminates).
fn run_app(
    play_path: Option<PathBuf>,
    autopilot: bool,
    seconds: f64,
    tour: bool,
) -> Result<(), String> {
    let event_ring = Arc::new(LocalEventRing::new(256));
    let dispatch_targets = create_channel_patches();
    let injector = MidiInjector::new(Arc::clone(&event_ring), dispatch_targets);
    let stack = SynthStack::new(BLOCK_LEN, event_ring);

    let schedule = match &play_path {
        Some(path) => match load_smf_events(path, SAMPLE_RATE_HZ) {
            Ok(events) => events,
            Err(err) => {
                eprintln!("warning: could not load {}: {err}", path.display());
                Vec::new()
            }
        },
        None => Vec::new(),
    };

    let audio_peak_bits = Arc::new(AtomicU32::new(0));
    let audio_stream = match open_audio_output(stack, BLOCK_LEN, Arc::clone(&audio_peak_bits)) {
        Ok(stream) => Some(stream),
        Err(err) => {
            eprintln!("warning: audio output unavailable: {err} (continuing without sound)");
            None
        }
    };

    let gamepad = match GilrsGamepadInput::new() {
        Ok(gamepad) => Some(gamepad),
        Err(err) => {
            eprintln!("warning: gamepad input unavailable: {err}");
            None
        }
    };

    let (midi_tx, midi_rx) = mpsc::channel::<Vec<u8>>();
    let midi_backend = connect_midi_hardware(midi_tx);

    let app = MixerUiApp::new(
        injector,
        schedule,
        gamepad,
        Some(midi_rx),
        midi_backend,
        audio_stream,
        audio_peak_bits,
        autopilot,
        seconds,
        tour,
    );

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("crest-synth")
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT]),
        ..Default::default()
    };

    eframe::run_native(
        "crest-synth",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .map_err(|err| err.to_string())
}

fn run_live(play_path: Option<PathBuf>, tour: bool) -> Result<(), String> {
    run_app(play_path, false, 0.0, tour)
}

fn run_autopilot(play_path: Option<PathBuf>, seconds: f64, tour: bool) -> i32 {
    match run_app(play_path, true, seconds, tour) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("synth_ui: autopilot failed: {err}");
            1
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut smoke = false;
    let mut autopilot = false;
    let mut tour = false;
    let mut seconds: f64 = 4.0;
    let mut play_path: Option<PathBuf> = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--smoke" => {
                smoke = true;
                index += 1;
            }
            "--autopilot" => {
                autopilot = true;
                index += 1;
            }
            "--tour" => {
                tour = true;
                index += 1;
            }
            "--seconds" => {
                index += 1;
                match args.get(index).and_then(|value| value.parse::<f64>().ok()) {
                    Some(value) if value > 0.0 => seconds = value,
                    _ => {
                        eprintln!("synth_ui: --seconds requires a positive number");
                        std::process::exit(1);
                    }
                }
                index += 1;
            }
            "--play" => {
                index += 1;
                match args.get(index) {
                    Some(value) => play_path = Some(PathBuf::from(value)),
                    None => {
                        eprintln!("synth_ui: --play requires a file path");
                        std::process::exit(1);
                    }
                }
                index += 1;
            }
            other => {
                eprintln!("synth_ui: unrecognized argument '{other}'");
                std::process::exit(1);
            }
        }
    }

    if smoke && autopilot {
        eprintln!("synth_ui: --smoke and --autopilot are mutually exclusive");
        std::process::exit(1);
    }

    if smoke {
        std::process::exit(run_smoke(play_path));
    }

    if autopilot {
        std::process::exit(run_autopilot(play_path, seconds, tour));
    }

    if let Err(err) = run_live(play_path, tour) {
        eprintln!("synth_ui: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn synthetic_engine_note_id_is_stable_for_the_same_channel_and_note() {
        assert_eq!(
            synthetic_engine_note_id(0, 60),
            synthetic_engine_note_id(0, 60)
        );
        assert_ne!(
            synthetic_engine_note_id(0, 60),
            synthetic_engine_note_id(1, 60)
        );
        assert_ne!(
            synthetic_engine_note_id(0, 60),
            synthetic_engine_note_id(0, 61)
        );
    }

    #[test]
    fn read_varlen_decodes_single_byte_values() {
        let data = [0x40, 0x00];
        let (value, next) = read_varlen(&data, 0).unwrap();
        assert_eq!(value, 0x40);
        assert_eq!(next, 1);
    }

    #[test]
    fn read_varlen_decodes_multi_byte_values() {
        // 0x81 0x00 encodes 128 in MIDI variable-length quantity form.
        let data = [0x81, 0x00];
        let (value, next) = read_varlen(&data, 0).unwrap();
        assert_eq!(value, 128);
        assert_eq!(next, 2);
    }

    #[test]
    fn read_varlen_rejects_truncated_input() {
        let data = [0x81];
        assert!(read_varlen(&data, 0).is_err());
    }

    #[test]
    fn ticks_to_samples_converts_one_quarter_note_at_120_bpm() {
        // At 120 BPM (500,000 microseconds/quarter) with 480 ticks per
        // quarter, one full quarter note is exactly 0.5 seconds.
        let samples = ticks_to_samples(480, 480, 500_000, 44_100);
        assert_eq!(samples, 22_050);
    }

    #[test]
    fn simple_adsr_envelope_rises_during_attack_and_reaches_sustain() {
        let timing = EnvelopeTiming::new(0.01, 0.01, 0.5, 0.01);
        let mut envelope = SimpleAdsrEnvelope::new(timing, 1_000.0);

        assert_eq!(envelope.tick(), 0.0, "idle before trigger");

        envelope.trigger();
        // Attack lasts 0.01s at 1000Hz == 10 samples; the level must rise
        // monotonically until it peaks at 1.0.
        let mut last = 0.0;
        for _ in 0..10 {
            let level = envelope.tick();
            assert!(level >= last - 1e-9);
            last = level;
        }
        assert!((last - 1.0).abs() < 1e-6, "attack should peak at unity");

        // Decay then brings it down to the sustain level, where it settles.
        for _ in 0..100 {
            envelope.tick();
        }
        let settled = envelope.tick();
        assert!(
            (settled - 0.5).abs() < 0.05,
            "should have settled near sustain"
        );
    }

    #[test]
    fn simple_adsr_envelope_falls_to_zero_after_release() {
        let timing = EnvelopeTiming::new(0.001, 0.001, 0.8, 0.01);
        let mut envelope = SimpleAdsrEnvelope::new(timing, 1_000.0);
        envelope.trigger();
        for _ in 0..20 {
            envelope.tick();
        }
        envelope.release();
        let mut level = 1.0;
        for _ in 0..50 {
            level = envelope.tick();
        }
        assert!(level <= 1e-6, "should have decayed to silence");
    }

    #[test]
    fn dispatch_target_reports_its_own_id_and_mapping() {
        let mapping = PatchChannelMapping::omni();
        let target = DispatchTarget {
            id: PatchId::new(7),
            mapping,
        };
        assert_eq!(target.id(), PatchId::new(7));
        assert_eq!(target.mapping(), mapping);
    }

    #[test]
    fn midi_injector_counts_dispatched_note_on_and_forwards_it() {
        let event_ring = Arc::new(LocalEventRing::new(4));
        let target = create_default_patch();
        let mut injector = MidiInjector::new(Arc::clone(&event_ring), vec![target]);

        injector.inject(&[0x90, 60, 100]);

        assert_eq!(injector.events_dispatched, 1);
        assert!(matches!(
            event_ring.pop(),
            Some(LocalBoundaryMessage::NoteOn {
                channel: 0,
                note: 60,
                ..
            })
        ));
    }

    #[test]
    fn midi_injector_drops_malformed_bytes_without_panicking() {
        let event_ring = Arc::new(LocalEventRing::new(4));
        let target = create_default_patch();
        let mut injector = MidiInjector::new(event_ring, vec![target]);

        injector.inject(&[]);
        injector.inject(&[0x90]);

        assert_eq!(injector.events_dispatched, 0);
    }

    #[test]
    fn synth_stack_renders_silence_with_no_input() {
        let event_ring = Arc::new(LocalEventRing::new(4));
        let mut stack = SynthStack::new(8, event_ring);

        let frames = stack.render_block(1.0 / 44_100.0).unwrap();

        assert_eq!(frames.len(), 8);
        for frame in frames {
            assert_eq!(frame.left, 0.0);
            assert_eq!(frame.right, 0.0);
        }
    }

    #[test]
    fn synth_stack_renders_audible_output_after_a_note_on() {
        let event_ring = Arc::new(LocalEventRing::new(4));
        let target = create_default_patch();
        let mut injector = MidiInjector::new(Arc::clone(&event_ring), vec![target]);
        let mut stack = SynthStack::new(BLOCK_LEN, event_ring);

        injector.inject(&[0x90, 60, 100]);

        let dt_seconds = BLOCK_LEN as f64 / f64::from(SAMPLE_RATE_HZ);
        let mut peak: f32 = 0.0;
        for _ in 0..20 {
            let frames = stack.render_block(dt_seconds).unwrap();
            for frame in frames {
                peak = peak.max(frame.left.abs());
            }
        }

        assert!(
            peak > 0.0,
            "a triggered voice should produce nonzero output"
        );
    }

    #[test]
    fn create_channel_patches_returns_sixteen_targets_with_distinct_ids() {
        let targets = create_channel_patches();
        assert_eq!(targets.len(), TOTAL_CHANNELS);
        let ids: HashSet<PatchId> = targets.iter().map(|target| target.id()).collect();
        assert_eq!(ids.len(), TOTAL_CHANNELS);
    }

    #[test]
    fn raw_bytes_for_encodes_note_on_and_note_off() {
        let note_on = ShellMidiEvent {
            channel: crest_synth::shell::midi_input::MidiChannel::try_new(3).unwrap(),
            kind: ShellMidiEventKind::NoteOn {
                note: 60,
                velocity: 100,
            },
            timestamp_micros: 0,
        };
        assert_eq!(raw_bytes_for(note_on), Some(vec![0x93, 60, 100]));

        let note_off = ShellMidiEvent {
            channel: crest_synth::shell::midi_input::MidiChannel::try_new(0).unwrap(),
            kind: ShellMidiEventKind::NoteOff {
                note: 64,
                velocity: 0,
            },
            timestamp_micros: 0,
        };
        assert_eq!(raw_bytes_for(note_off), Some(vec![0x80, 64, 0]));
    }

    #[test]
    fn all_semantic_tokens_has_exactly_ten_entries() {
        assert_eq!(ALL_SEMANTIC_TOKENS.len(), 10);
    }

    #[test]
    fn to_color32_round_trips_white() {
        let white = Rgba::white();
        let color = to_color32(white);
        assert_eq!(
            color,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255)
        );
    }

    #[test]
    fn edit_gesture_emits_toggle_on_double_tap_within_window() {
        let mut gesture = EditGesture::new();
        let mut events = Vec::new();
        let now = Instant::now();

        gesture.on_press(now, &mut events);
        gesture.on_release(&mut events);
        gesture.on_press(now + Duration::from_millis(50), &mut events);

        assert_eq!(
            events,
            vec![
                MixerViewEvent::EnterEditMode,
                MixerViewEvent::ExitEditMode,
                MixerViewEvent::EnterEditMode,
                MixerViewEvent::ToggleFocusedParam,
            ]
        );
    }

    #[test]
    fn edit_gesture_does_not_toggle_on_slow_second_press() {
        let mut gesture = EditGesture::new();
        let mut events = Vec::new();
        let now = Instant::now();

        gesture.on_press(now, &mut events);
        gesture.on_release(&mut events);
        gesture.on_press(now + Duration::from_millis(500), &mut events);

        assert_eq!(
            events,
            vec![
                MixerViewEvent::EnterEditMode,
                MixerViewEvent::ExitEditMode,
                MixerViewEvent::EnterEditMode,
            ]
        );
    }

    #[test]
    fn autopilot_event_script_visits_edit_and_toggle_events() {
        let script = autopilot_event_script();
        assert!(script.contains(&MixerViewEvent::EnterEditMode));
        assert!(script.contains(&MixerViewEvent::ExitEditMode));
        assert!(script.contains(&MixerViewEvent::ToggleFocusedParam));
    }
}
