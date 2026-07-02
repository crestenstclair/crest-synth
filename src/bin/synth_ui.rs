// path: src/bin/synth_ui.rs

//! The standalone synthesizer application: opens the audio output and a
//! window, renders GUI views, polls the gamepad for navigation, and plays
//! notes from a MIDI file (or a synthetic note) through the full
//! engine-to-mixer signal path.
//!
//! Modes:
//! - (no flags): opens a real-time audio stream (best-effort; continues
//!   without sound if no device is available), a terminal-based window,
//!   and gamepad input, looping until the window is closed.
//! - `--play <FILE.mid>`: additionally loads and sequences a standard MIDI
//!   file through the engine, looping the file until quit.
//! - `--smoke`: headless self-check -- no window, no audio device. Builds
//!   the full stack (dispatcher, engine, mixer) and renders a few seconds
//!   of audio through the exact same render path the live app uses,
//!   measuring the peak absolute sample and the count of dispatched
//!   events.
//!
//! Several collaborators named in the project's shell module tree
//! (`CpalAudioOutput`, `EframeAppWindow`, `EguiRenderer`, `MidirMidiInput`)
//! are not yet available in this crate's module declarations. Per this
//! project's established convention for not-yet-available collaborators
//! (see `engine::voice`, `engine::voice_allocator`), this module defines
//! minimal local substitutes against the ports that *are* available
//! (`AudioOutput`... note: even the `AudioOutput`/`RenderCallback` ports
//! are bypassed here in favor of driving `cpal` directly, to sidestep the
//! non-`Send` `cpal::Stream` on some platforms without standing up a
//! dedicated host thread) and the standard MIDI file format, so this
//! binary compiles and runs against what is actually committed today.

use std::cell::UnsafeCell;
use std::env;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

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

use crest_synth::mixer::channel_strip::ChannelStrip;
use crest_synth::mixer::mix_bus::MixBus;
use crest_synth::mixer::mix_engine::{Limiter, MasterSource, MixEngine, StripSource};

use crest_synth::patch::midi_dispatcher::{MidiAddress, MidiDispatcher, RoutablePatch};
use crest_synth::patch::patch::{ChannelMapping as PatchChannelMapping, PatchId};
use crest_synth::patch::patch_manager::PatchManager;

use crest_synth::real_time::parameter_bridge::{ParameterBridge, ParameterSnapshot};

use crest_synth::shell::app_window::{App, AppWindow, WindowError};
use crest_synth::shell::gamepad_input::{GamepadAction, GamepadButton, GamepadInput};
use crest_synth::shell::gilrs_gamepad_input::GilrsGamepadInput;
use crest_synth::shell::gui_renderer::{GuiRenderer, SurfaceKind, ViewState};
use crest_synth::shell::midi_input::{
    Connection, EventCallback, MidiError, MidiInput, MidiPortInfo,
};
use crest_synth::shell::midi_normalizer::MidiNormalizer;

/// Fixed render sample rate used throughout this binary.
const SAMPLE_RATE_HZ: u32 = 44_100;
/// Frames rendered per block, both for the headless smoke check and for
/// the live `cpal` stream's fixed buffer size.
const BLOCK_LEN: usize = 512;
/// Fixed voice pool size for the demo patch's engine.
const POLYPHONY: usize = 8;
/// How many seconds of audio `--smoke` renders before measuring peak.
const SMOKE_SECONDS: f64 = 2.0;

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
/// dispatch-routable view of it.
fn create_default_patch() -> DispatchTarget {
    let mut manager = PatchManager::default();
    let id = manager.create_patch();
    manager
        .set_mapping(id, PatchChannelMapping::omni())
        .expect("freshly created patch exists");
    let mapping = manager.get(id).expect("patch exists").mapping();
    DispatchTarget { id, mapping }
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
// headless `--smoke` check and the live `cpal` render callback.
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
    /// `--smoke` check and the live `cpal` render callback.
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

    /// Renders one block and writes interleaved stereo `f32` samples into
    /// `output`, for consumption by a real audio backend. Fails safe to
    /// silence (never panics) if the host hands us an unexpected buffer
    /// length, since this runs on the real-time audio thread.
    fn render_interleaved(&mut self, output: &mut [f32]) {
        let expected_len = self.block_len * 2;
        if output.len() != expected_len {
            output.fill(0.0);
            return;
        }

        let dt_seconds = self.block_len as f64 / f64::from(SAMPLE_RATE_HZ);
        match self.render_block(dt_seconds) {
            Ok(mixed) => {
                for (i, frame) in mixed.iter().enumerate() {
                    output[i * 2] = frame.left;
                    output[i * 2 + 1] = frame.right;
                }
            }
            Err(_) => output.fill(0.0),
        }
    }
}

// ---------------------------------------------------------------------
// Shell adapters not yet available as committed resources
// (`MidirMidiInput`) get a minimal local substitute against the port
// that *is* available (`shell::midi_input::MidiInput`).
// ---------------------------------------------------------------------

/// Reports no hardware MIDI ports. A real `midir`-backed adapter will
/// replace this once `shell::midir_midi_input` is generated; until then
/// this keeps the `MidiInput` port wired end-to-end (gamepad and/or
/// `--play` remain fully functional without it).
struct NoMidiHardwareInput;

impl MidiInput for NoMidiHardwareInput {
    fn list_ports(&self) -> Vec<MidiPortInfo> {
        Vec::new()
    }

    fn connect(
        &self,
        port: &MidiPortInfo,
        _on_event: Box<dyn EventCallback>,
    ) -> Result<Connection, MidiError> {
        Err(MidiError::PortNotFound(port.id().to_string()))
    }
}

/// Prints the active GUI surface once at startup. A real `egui`-backed
/// renderer will replace this once `shell::egui_renderer` is generated.
struct TerminalGuiRenderer;

impl GuiRenderer for TerminalGuiRenderer {
    fn render(&mut self, view: ViewState) {
        println!(
            "[{:?}] patch: {}",
            view.active_surface, view.patch_editor.patch_name
        );
    }
}

/// A dependency-free `AppWindow` adapter: drives the gamepad-navigable
/// tick loop from a plain terminal rather than a graphical window. A real
/// `eframe`-backed adapter will replace this once
/// `shell::eframe_app_window` is generated; until then this keeps every
/// action reachable via gamepad (South triggers a note) and via `--play`
/// file sequencing, and keeps the mechanical build free of a heavy
/// windowing-toolkit dependency that may not have system libraries
/// available wherever this crate is built.
struct TerminalAppWindow {
    injector: MidiInjector,
    gamepad: Option<GilrsGamepadInput>,
    schedule: Vec<(u64, Vec<u8>)>,
    sample_rate_hz: u32,
    gui: TerminalGuiRenderer,
}

impl TerminalAppWindow {
    fn new(
        injector: MidiInjector,
        gamepad: Option<GilrsGamepadInput>,
        schedule: Vec<(u64, Vec<u8>)>,
        sample_rate_hz: u32,
    ) -> Self {
        Self {
            injector,
            gamepad,
            schedule,
            sample_rate_hz,
            gui: TerminalGuiRenderer,
        }
    }
}

impl AppWindow for TerminalAppWindow {
    fn run(&mut self, app: App) -> Result<(), WindowError> {
        let mut view = ViewState::new(SurfaceKind::PatchEditor);
        view.patch_editor.patch_name = app.title.clone();
        self.gui.render(view);
        println!(
            "{} -- gamepad South triggers a note; type 'quit' to exit",
            app.title
        );

        let (quit_tx, quit_rx) = mpsc::channel::<()>();
        thread::spawn(move || {
            let mut line = String::new();
            loop {
                line.clear();
                match std::io::stdin().read_line(&mut line) {
                    Ok(0) => {
                        let _ = quit_tx.send(());
                        return;
                    }
                    Ok(_) => {
                        if line.trim().eq_ignore_ascii_case("quit") {
                            let _ = quit_tx.send(());
                            return;
                        }
                    }
                    Err(_) => {
                        let _ = quit_tx.send(());
                        return;
                    }
                }
            }
        });

        let start = Instant::now();
        let schedule_len = self.schedule.len();
        let mut schedule_index = 0usize;
        let mut last_position: u64 = 0;
        let mut gamepad_note_on = false;

        let cycle_samples: u64 = if schedule_len > 0 {
            self.schedule[schedule_len - 1].0 + u64::from(self.sample_rate_hz)
        } else {
            1
        };

        loop {
            if quit_rx.try_recv().is_ok() {
                break;
            }

            if let Some(gamepad) = self.gamepad.as_mut() {
                for action in gamepad.poll() {
                    match action {
                        GamepadAction::ButtonPressed {
                            button: GamepadButton::South,
                            ..
                        } if !gamepad_note_on => {
                            self.injector.inject(&[0x90, 60, 100]);
                            gamepad_note_on = true;
                        }
                        GamepadAction::ButtonReleased {
                            button: GamepadButton::South,
                            ..
                        } if gamepad_note_on => {
                            self.injector.inject(&[0x80, 60, 0]);
                            gamepad_note_on = false;
                        }
                        _ => {}
                    }
                }
            }

            if schedule_len > 0 {
                let elapsed_samples =
                    (start.elapsed().as_secs_f64() * f64::from(self.sample_rate_hz)) as u64;
                let position = elapsed_samples % cycle_samples;
                if position < last_position {
                    // Wrapped around: loop the file until quit.
                    schedule_index = 0;
                }
                last_position = position;

                while schedule_index < schedule_len && self.schedule[schedule_index].0 <= position {
                    self.injector.inject(&self.schedule[schedule_index].1);
                    schedule_index += 1;
                }
            }

            thread::sleep(Duration::from_millis(16));
        }

        Ok(())
    }
}

/// Opens a real-time audio stream via `cpal` directly (bypassing the
/// `shell::audio_output::AudioOutput` port, whose adapter,
/// `CpalAudioOutput`, is not yet a committed module in this crate). Drives
/// `stack` through the exact same `render_interleaved` path `--smoke`
/// exercises headlessly. Returns the `cpal::Stream`, which must be kept
/// alive (and never sent to another thread, since `cpal::Stream` is not
/// `Send` on every platform) for audio to keep playing.
fn open_cpal_stream(
    sample_rate_hz: u32,
    block_len: usize,
    mut stack: SynthStack,
) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no default output device".to_string())?;

    let config = cpal::StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(sample_rate_hz),
        buffer_size: cpal::BufferSize::Fixed(block_len as u32),
    };

    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                stack.render_interleaved(data);
            },
            |err| eprintln!("audio stream error: {err}"),
            None,
        )
        .map_err(|err| err.to_string())?;

    stream.play().map_err(|err| err.to_string())?;
    Ok(stream)
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

/// Loads the first track chunk of a standard MIDI file (format 0 or 1)
/// and returns every channel-voice message it contains as
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

        // Only the first track chunk is sequenced -- sufficient for a
        // single-patch demo app and for the `--smoke` check's purpose of
        // exercising the render path with real events.
        break;
    }

    events.sort_by_key(|(sample, _)| *sample);
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

/// Headless self-check: no window, no audio device. Builds the full
/// stack, sequences the first seconds of `play_path` (or a synthetic
/// note-on if none was given), renders through the same render path the
/// live app uses, and reports the measured peak and dispatched event
/// count. Returns the process exit code.
fn run_smoke(play_path: Option<PathBuf>) -> i32 {
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
    let dt_seconds = BLOCK_LEN as f64 / f64::from(SAMPLE_RATE_HZ);
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

    if peak > 0.05 && peak <= 1.0 && injector.events_dispatched > 0 {
        0
    } else {
        1
    }
}

/// The standard interactive app: opens the audio output (best-effort) and
/// a window, polls the gamepad, and (if `--play` was given) sequences a
/// MIDI file through the engine, looping until the window is closed.
fn run_live(play_path: Option<PathBuf>) -> Result<(), String> {
    let event_ring = Arc::new(LocalEventRing::new(256));
    let dispatch_target = create_default_patch();
    let injector = MidiInjector::new(Arc::clone(&event_ring), vec![dispatch_target]);
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

    let stream = match open_cpal_stream(SAMPLE_RATE_HZ, BLOCK_LEN, stack) {
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

    let midi_hardware = NoMidiHardwareInput;
    if midi_hardware.list_ports().is_empty() {
        println!("no hardware MIDI input ports found; use the gamepad or --play a file");
    }

    let mut window = TerminalAppWindow::new(injector, gamepad, schedule, SAMPLE_RATE_HZ);
    let app = App::new("crest-synth", 1280, 720);
    let result = window.run(app).map_err(|err| err.to_string());

    drop(stream);
    result
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let smoke = args.iter().any(|arg| arg == "--smoke");
    let play_path = args
        .iter()
        .position(|arg| arg == "--play")
        .and_then(|idx| args.get(idx + 1))
        .map(PathBuf::from);

    if smoke {
        let code = run_smoke(play_path);
        std::process::exit(code);
    }

    if let Err(err) = run_live(play_path) {
        eprintln!("synth_ui: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
