// path: src/bin/mod_play.rs

//! mod_play — multi-patch MIDI player with the Modulation context active.
//!
//! Builds on the patch_play shape (2-3 `Patch` aggregates, each with its
//! own `VoiceAllocator` pool, subscribed to a distinct MIDI channel and
//! routed to via the `MidiDispatcher`), then layers a per-patch `ModMatrix`
//! on top: one LFO drives a pitch-vibrato routing and a filter-cutoff-sweep
//! routing, evaluated once per sample by `ModProcessor` before that
//! sample's voices are rendered. Each patch's dry mix feeds a
//! `ChannelStrip`; `MixEngine` sums every strip into the master `MixBus`
//! and applies the final limiter.
//!
//! Usage: `mod_play [FILE.mid] [--out OUT.wav]`
//!
//! With no FILE, a built-in multi-channel demo tune is used: sustained,
//! legato notes so the vibrato and the filter sweep are clearly audible.

use std::collections::HashMap;
use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::process;

use crest_synth::effects::effect_chain::EffectChain;
use crest_synth::effects::effect_processor::{AudioFrame, EffectProcessor};
use crest_synth::engine::filter::{Filter, FilterConfig, FilterKind, StateVariableFilter};
use crest_synth::engine::oscillator::{
    Amplitude as OscAmplitude, Frequency, Oscillator, OscillatorConfig, SampleRate,
    StandardOscillator, Waveform,
};
use crest_synth::engine::voice::{
    AmpEnvelopeStage, EnvelopeTiming, NoteId as EngineNoteId, NoteNumber as EngineNoteNumber,
    Velocity as EngineVelocity, VoiceConfig as EngineVoiceConfig, VoiceEvent,
};
use crest_synth::engine::voice_allocator::{StealPolicy, VoiceAllocator, VoiceAssignment};
use crest_synth::kernel::midi_event_kind::MidiEventKind;
use crest_synth::midi_file::midi_file_reader::MidiFileReader;
use crest_synth::midi_file::midly_midi_file_reader::MidlyMidiFileReader;
use crest_synth::mixer::channel_strip::{
    ChannelStrip, ChannelStripCommand, Decibel, Pan as StripPan,
};
use crest_synth::mixer::mix_bus::MixBus;
use crest_synth::mixer::mix_engine::{Limiter, MasterSource, MixEngine, StripSource};
use crest_synth::modulation::mod_matrix::{LfoConfig, ModMatrix, ModMatrixCommand, ModRoute};
use crest_synth::modulation::mod_processor::{FixedOffsetBuffer, LfoSourceEvaluator, ModProcessor};
use crest_synth::patch::midi_dispatcher::{MidiAddress, MidiDispatcher, RoutablePatch};
use crest_synth::patch::patch::{ChannelMapping, Patch, PatchId, VoiceConfig as PatchVoiceConfig};

// ─── Constants ──────────────────────────────────────────────────────────────

const SAMPLE_RATE: u32 = 44_100;
const SAMPLE_RATE_F64: f64 = SAMPLE_RATE as f64;
/// Tail silence appended after the last event so notes can fully release.
const TAIL_SECS: f64 = 2.0;
/// Headroom applied to each patch's dry mono sum before it reaches the mixer,
/// so several overlapping voices across several patches cannot clip before
/// the master limiter has a chance to act.
const PATCH_HEADROOM: f64 = 0.2;
/// Modulation destination id (as routed through `ModRoute`/`ModProcessor`)
/// carrying the pitch-vibrato offset, in semitones at full-scale route
/// output.
const PITCH_ROUTE_DEST: u32 = 0;
/// Modulation destination id carrying the filter-cutoff-sweep offset, in Hz
/// at full-scale route output.
const FILTER_ROUTE_DEST: u32 = 1;
/// Semitone range a fully-saturated (±1.0) pitch route can offset a voice.
const VIBRATO_SEMITONE_RANGE: f64 = 2.0;
/// Hertz range a fully-saturated (±1.0) filter route can offset the cutoff.
const SWEEP_HZ_RANGE: f64 = 3_500.0;

// ─── Timeline value types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventKind {
    NoteOn,
    NoteOff,
}

/// One scheduled MIDI event: a channel-addressed note on/off at an absolute
/// time offset from the start of playback.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TimelineEvent {
    at_seconds: f64,
    channel: u8,
    kind: EventKind,
    note: u8,
    velocity: f64,
}

/// Sorts `events` by timestamp, with note-offs ordered before note-ons at
/// the same instant so a retriggered note frees its voice before the new
/// one claims it.
fn sort_timeline(events: &mut [TimelineEvent]) {
    events.sort_by(|a, b| {
        a.at_seconds
            .partial_cmp(&b.at_seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| event_rank(a).cmp(&event_rank(b)))
    });
}

fn event_rank(event: &TimelineEvent) -> u8 {
    if event.kind == EventKind::NoteOff {
        0
    } else {
        1
    }
}

/// Loads a MIDI file into a flat, channel-addressed timeline via the
/// `MidlyMidiFileReader` adapter. Only Note On / Note Off events survive;
/// everything else `MidiFileReader` does not represent is skipped.
fn load_timeline_from_file(path: &Path) -> Vec<TimelineEvent> {
    let reader = MidlyMidiFileReader::new();
    let song = reader.load(path).unwrap_or_else(|e| {
        eprintln!("error: cannot parse MIDI file '{}': {e}", path.display());
        process::exit(1);
    });

    let mut events: Vec<TimelineEvent> = song
        .events()
        .iter()
        .filter_map(|timed| {
            let kind = match timed.event().kind() {
                MidiEventKind::NoteOn => EventKind::NoteOn,
                MidiEventKind::NoteOff => EventKind::NoteOff,
                _ => return None,
            };
            Some(TimelineEvent {
                at_seconds: timed.at_seconds(),
                channel: timed.event().address().channel().value(),
                kind,
                note: timed.event().note().value(),
                velocity: timed.event().velocity().value(),
            })
        })
        .collect();

    sort_timeline(&mut events);
    events
}

fn push_note(
    events: &mut Vec<TimelineEvent>,
    at: f64,
    dur: f64,
    channel: u8,
    note: u8,
    velocity: f64,
) {
    events.push(TimelineEvent {
        at_seconds: at,
        channel,
        kind: EventKind::NoteOn,
        note,
        velocity,
    });
    events.push(TimelineEvent {
        at_seconds: at + dur,
        channel,
        kind: EventKind::NoteOff,
        note,
        velocity,
    });
}

/// Built-in multi-channel demo tune: sustained / legato notes on each of the
/// three channels the demo patches subscribe to, so the LFO vibrato and the
/// filter sweep are both clearly audible over long-held notes.
fn builtin_demo() -> Vec<TimelineEvent> {
    let mut events = Vec::new();

    // Channel 0 ("Lead"): a slow, sustained melody.
    let lead: &[(f64, u8, f64)] = &[
        (0.0, 64, 1.2),
        (1.3, 67, 1.2),
        (2.6, 69, 1.2),
        (3.9, 72, 1.8),
        (5.8, 71, 1.2),
        (7.1, 69, 2.0),
    ];
    for &(t, note, dur) in lead {
        push_note(&mut events, t, dur, 0, note, 0.75);
    }

    // Channel 1 ("Pad"): long chords, ideal for a slow filter sweep.
    let pad_chords: &[(f64, &[u8], f64)] = &[
        (0.0, &[60, 64, 67], 2.8),
        (3.0, &[57, 60, 64], 2.8),
        (6.0, &[55, 59, 62], 2.5),
    ];
    for &(t, notes, dur) in pad_chords {
        for &note in notes {
            push_note(&mut events, t, dur, 1, note, 0.55);
        }
    }

    // Channel 2 ("Bass"): sustained root notes.
    let bass: &[(f64, u8, f64)] = &[(0.0, 36, 2.8), (3.0, 33, 2.8), (6.0, 31, 2.5)];
    for &(t, note, dur) in bass {
        push_note(&mut events, t, dur, 2, note, 0.85);
    }

    sort_timeline(&mut events);
    events
}

// ─── Runtime patch ──────────────────────────────────────────────────────────

/// Static configuration for one demo patch: everything `build_patch` needs
/// to assemble a fully wired `RuntimePatch`.
struct PatchSpec {
    id: u32,
    name: &'static str,
    channel: u8,
    waveform: Waveform,
    osc_amplitude: f64,
    filter_kind: FilterKind,
    base_cutoff_hz: f64,
    resonance: f64,
    attack: f64,
    decay: f64,
    sustain: f64,
    release: f64,
    polyphony: usize,
    steal_policy: StealPolicy,
    lfo_rate_hz: f64,
    lfo_depth: f64,
    vibrato_depth: f64,
    sweep_depth: f64,
    volume_db: f32,
    pan: f32,
}

/// One playable patch at render time: the `Patch` aggregate (identity and
/// channel mapping, used by `MidiDispatcher`), its own `VoiceAllocator`
/// pool (independent polyphony and stealing), the oscillator/filter state
/// needed to render its voices, its `ModMatrix` plus the `ModProcessor`
/// that evaluates it, and the running statistics printed at the end.
struct RuntimePatch {
    domain: Patch,
    name: &'static str,
    allocator: VoiceAllocator,
    /// Per-voice-slot oscillator phase, positionally aligned with the
    /// allocator's internal voice slots.
    phases: Vec<f64>,
    /// Per-voice-slot filter state, positionally aligned with the
    /// allocator's internal voice slots.
    filters: Vec<StateVariableFilter>,
    osc: StandardOscillator,
    osc_config: OscillatorConfig,
    filter_kind: FilterKind,
    base_cutoff_hz: f64,
    resonance: f64,
    mod_matrix: ModMatrix,
    mod_processor: ModProcessor<LfoSourceEvaluator>,
    active_notes: HashMap<u8, EngineNoteId>,
    next_note_id: u32,
    events_delivered: usize,
    voice_steals: usize,
    peak_voices: usize,
    volume_db: f32,
    pan: f32,
}

impl RoutablePatch for RuntimePatch {
    fn id(&self) -> PatchId {
        self.domain.id()
    }

    fn mapping(&self) -> ChannelMapping {
        self.domain.mapping()
    }
}

fn build_patch(spec: PatchSpec) -> RuntimePatch {
    let patch_voice_config = PatchVoiceConfig::try_new(
        spec.polyphony as u8,
        (spec.attack * 1000.0) as f32,
        (spec.decay * 1000.0) as f32,
        spec.sustain as f32,
        (spec.release * 1000.0) as f32,
    )
    .expect("valid patch voice config");

    let mut domain = Patch::new(PatchId::new(spec.id), spec.id, patch_voice_config);
    let mapping = ChannelMapping::single(spec.channel).expect("valid channel");
    let _ = domain.set_mapping(mapping);

    let engine_voice_config = EngineVoiceConfig::new(EnvelopeTiming::new(
        spec.attack,
        spec.decay,
        spec.sustain,
        spec.release,
    ));
    let allocator = VoiceAllocator::new(engine_voice_config, spec.polyphony, spec.steal_policy)
        .expect("valid voice allocator");

    let phases = vec![0.0_f64; spec.polyphony];
    let filters = vec![StateVariableFilter::new(); spec.polyphony];

    let osc_amplitude = OscAmplitude::try_new(spec.osc_amplitude).expect("valid amplitude");
    let osc_config = OscillatorConfig::new(spec.waveform, osc_amplitude);

    let mut mod_matrix = ModMatrix::new(4);
    mod_matrix
        .apply(ModMatrixCommand::SetLfo {
            index: 0,
            config: LfoConfig::new(spec.lfo_rate_hz, spec.lfo_depth, 0.0),
        })
        .expect("configure lfo");
    mod_matrix
        .apply(ModMatrixCommand::AddRoute {
            route: ModRoute::new(0, PITCH_ROUTE_DEST, spec.vibrato_depth),
        })
        .expect("add vibrato routing");
    mod_matrix
        .apply(ModMatrixCommand::AddRoute {
            route: ModRoute::new(0, FILTER_ROUTE_DEST, spec.sweep_depth),
        })
        .expect("add filter sweep routing");

    let mod_processor = ModProcessor::new(LfoSourceEvaluator::from_matrix(&mod_matrix));

    RuntimePatch {
        domain,
        name: spec.name,
        allocator,
        phases,
        filters,
        osc: StandardOscillator::new(),
        osc_config,
        filter_kind: spec.filter_kind,
        base_cutoff_hz: spec.base_cutoff_hz,
        resonance: spec.resonance,
        mod_matrix,
        mod_processor,
        active_notes: HashMap::new(),
        next_note_id: 1,
        events_delivered: 0,
        voice_steals: 0,
        peak_voices: 0,
        volume_db: spec.volume_db,
        pan: spec.pan,
    }
}

/// Builds the three demo patches: distinct engine settings, distinct
/// channel subscriptions, each with its own vibrato + filter-sweep
/// modulation routing.
fn build_patches() -> Vec<RuntimePatch> {
    vec![
        build_patch(PatchSpec {
            id: 0,
            name: "Lead",
            channel: 0,
            waveform: Waveform::Sine,
            osc_amplitude: 0.9,
            filter_kind: FilterKind::LowPass,
            base_cutoff_hz: 6_000.0,
            resonance: 0.25,
            attack: 0.03,
            decay: 0.08,
            sustain: 0.75,
            release: 0.5,
            polyphony: 4,
            steal_policy: StealPolicy::Quietest,
            lfo_rate_hz: 5.0,
            lfo_depth: 1.0,
            vibrato_depth: 0.35,
            sweep_depth: 0.2,
            volume_db: -6.0,
            pan: -0.4,
        }),
        build_patch(PatchSpec {
            id: 1,
            name: "Pad",
            channel: 1,
            waveform: Waveform::Triangle,
            osc_amplitude: 0.8,
            filter_kind: FilterKind::LowPass,
            base_cutoff_hz: 2_500.0,
            resonance: 0.4,
            attack: 0.4,
            decay: 0.3,
            sustain: 0.7,
            release: 0.9,
            polyphony: 6,
            steal_policy: StealPolicy::Oldest,
            lfo_rate_hz: 0.35,
            lfo_depth: 1.0,
            vibrato_depth: 0.15,
            sweep_depth: 0.85,
            volume_db: -8.0,
            pan: 0.0,
        }),
        build_patch(PatchSpec {
            id: 2,
            name: "Bass",
            channel: 2,
            waveform: Waveform::Saw,
            osc_amplitude: 0.85,
            filter_kind: FilterKind::LowPass,
            base_cutoff_hz: 1_200.0,
            resonance: 0.3,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.8,
            release: 0.3,
            polyphony: 3,
            steal_policy: StealPolicy::Quietest,
            lfo_rate_hz: 4.5,
            lfo_depth: 0.8,
            vibrato_depth: 0.25,
            sweep_depth: 0.5,
            volume_db: -4.0,
            pan: 0.35,
        }),
    ]
}

// ─── Channel dispatch ───────────────────────────────────────────────────────

/// Routes one timeline event through the `MidiDispatcher` to every patch
/// whose channel mapping matches, then applies the note on/off to each
/// matched patch's own voice allocator.
fn dispatch_event(
    event: &TimelineEvent,
    dispatcher: &MidiDispatcher,
    patches: &mut [RuntimePatch],
) {
    let address = match MidiAddress::try_new(event.channel) {
        Ok(address) => address,
        Err(_) => return,
    };

    let matched: Vec<PatchId> = dispatcher.dispatch(address, patches);
    for id in matched {
        if let Some(patch) = patches.iter_mut().find(|p| p.id() == id) {
            patch.events_delivered += 1;
            match event.kind {
                EventKind::NoteOn => handle_note_on(patch, event.note, event.velocity),
                EventKind::NoteOff => handle_note_off(patch, event.note),
            }
        }
    }
}

fn handle_note_on(patch: &mut RuntimePatch, note: u8, velocity: f64) {
    // A retrigger of an already-sounding key releases the old instance
    // first, in case its matching note-off was never delivered.
    if let Some(old_id) = patch.active_notes.remove(&note) {
        let _ = patch.allocator.release(old_id);
    }

    let note_number = match EngineNoteNumber::try_new(note) {
        Ok(n) => n,
        Err(_) => return,
    };
    let velocity = match EngineVelocity::try_new(velocity.clamp(0.0, 1.0)) {
        Ok(v) => v,
        Err(_) => return,
    };

    let note_id = EngineNoteId::new(u64::from(patch.next_note_id));
    patch.next_note_id = patch.next_note_id.wrapping_add(1);
    patch.active_notes.insert(note, note_id);

    match patch.allocator.allocate(note_number, note_id, velocity) {
        Ok(VoiceAssignment::Assigned { index }) => patch.phases[index] = 0.0,
        Ok(VoiceAssignment::Stolen { .. }) => patch.voice_steals += 1,
        Err(_) => {}
    }
}

fn handle_note_off(patch: &mut RuntimePatch, note: u8) {
    if let Some(note_id) = patch.active_notes.remove(&note) {
        let _ = patch.allocator.release(note_id);
    }
}

// ─── Rendering ──────────────────────────────────────────────────────────────

/// Converts a MIDI note number, offset by a modulated pitch delta in
/// semitones, to its equal-tempered frequency (A4 = 440 Hz at note 69).
fn note_frequency_with_detune(note: u8, semitone_offset: f64) -> Frequency {
    let semitones_from_a4 = f64::from(note) - 69.0 + semitone_offset;
    let hertz = (440.0 * 2f64.powf(semitones_from_a4 / 12.0)).max(1.0);
    Frequency::try_new(hertz)
        .unwrap_or_else(|_| Frequency::try_new(440.0).expect("440 Hz is valid"))
}

/// Renders every patch's dry mono buffer across the whole timeline, running
/// the `ModProcessor` once per sample to derive that sample's pitch-vibrato
/// and filter-cutoff-sweep offsets before rendering the patch's active
/// voices, then mixes every patch through `MixEngine` into the master bus.
fn render(
    timeline: &[TimelineEvent],
    patches: &mut [RuntimePatch],
    sample_rate: f64,
) -> Vec<AudioFrame> {
    let dispatcher = MidiDispatcher::new();
    let last_t = timeline
        .iter()
        .map(|e| e.at_seconds)
        .fold(0.0_f64, f64::max);
    let total_secs = last_t + TAIL_SECS;
    let total_samples = (total_secs * sample_rate).ceil() as usize;
    let sample_rate_typed = SampleRate::try_new(sample_rate).expect("valid sample rate");
    let dt = 1.0 / sample_rate;

    let mut patch_buffers: Vec<Vec<AudioFrame>> = patches
        .iter()
        .map(|_| vec![AudioFrame::silence(); total_samples])
        .collect();

    let mut cursor = 0usize;

    // `sample_idx` drives the playback clock (`t`), the mod-processor's
    // sample-index argument, and indexing into two independent collections
    // (the shared `timeline` cursor and each patch's own output buffer), so
    // it cannot be replaced by a single `.enumerate()` over one of them.
    #[allow(clippy::needless_range_loop)]
    for sample_idx in 0..total_samples {
        let t = sample_idx as f64 / sample_rate;

        while cursor < timeline.len() && timeline[cursor].at_seconds <= t {
            dispatch_event(&timeline[cursor], &dispatcher, patches);
            cursor += 1;
        }

        for (patch_idx, patch) in patches.iter_mut().enumerate() {
            // Advance every voice's amp envelope by one sample period,
            // completing any pending steal whose victim just reached Idle.
            let phases = &mut patch.phases;
            patch.allocator.advance_all(dt, |slot, event| {
                if let VoiceEvent::Triggered { .. } = event {
                    phases[slot] = 0.0;
                }
            });

            // Evaluate this patch's modulation matrix for this sample.
            let mut sink: FixedOffsetBuffer<2> = FixedOffsetBuffer::new();
            patch.mod_processor.process(
                patch.mod_matrix.routes(),
                &[],
                sample_idx as u64,
                sample_rate,
                &mut sink,
            );
            let pitch_offset_semitones = sink.get(PITCH_ROUTE_DEST) * VIBRATO_SEMITONE_RANGE;
            let cutoff_offset_hz = sink.get(FILTER_ROUTE_DEST) * SWEEP_HZ_RANGE;

            let mut sum = 0.0_f64;
            for voice_index in 0..patch.allocator.polyphony() {
                let (note, velocity, amp_level) = match patch.allocator.voice(voice_index) {
                    Some(voice) if voice.amp_stage() != AmpEnvelopeStage::Idle => (
                        voice.note().value(),
                        voice.velocity().value(),
                        voice.amp_level(),
                    ),
                    _ => continue,
                };

                let phase = patch.phases[voice_index];
                let raw = patch.osc.render(phase, patch.osc_config);

                let filter_config = FilterConfig::new(
                    patch.filter_kind,
                    patch.base_cutoff_hz + cutoff_offset_hz,
                    patch.resonance,
                    sample_rate,
                );
                let filtered = patch.filters[voice_index].process(raw, filter_config);

                sum += filtered * amp_level * velocity;

                let frequency = note_frequency_with_detune(note, pitch_offset_semitones);
                patch.phases[voice_index] = patch.osc.advance(phase, frequency, sample_rate_typed);
            }

            let active = patch.allocator.active_count();
            if active > patch.peak_voices {
                patch.peak_voices = active;
            }

            patch_buffers[patch_idx][sample_idx] = AudioFrame::mono((sum * PATCH_HEADROOM) as f32);
        }
    }

    mix_down(patches, &patch_buffers, total_samples)
}

/// Sums every patch's dry buffer through a `ChannelStrip` (per-patch gain
/// and pan) into the master `MixBus` via `MixEngine`, applying the final
/// limiter. No effect chains are used here, so every strip's insert chain
/// is empty (zero slots, zero processors).
fn mix_down(
    patches: &[RuntimePatch],
    patch_buffers: &[Vec<AudioFrame>],
    total_samples: usize,
) -> Vec<AudioFrame> {
    let mix_engine = MixEngine::default();
    let empty_chain = EffectChain::new(0);

    let mut strips: Vec<ChannelStrip> = patches
        .iter()
        .map(|patch| {
            let mut strip = ChannelStrip::new();
            strip
                .handle(ChannelStripCommand::SetVolume {
                    volume_db: Decibel::try_new(patch.volume_db).expect("valid volume"),
                })
                .expect("set volume");
            strip
                .handle(ChannelStripCommand::SetPan {
                    pan: StripPan::try_new(patch.pan).expect("valid pan"),
                })
                .expect("set pan");
            strip
        })
        .collect();

    let mut processors_per_strip: Vec<Vec<Box<dyn EffectProcessor>>> =
        patches.iter().map(|_| Vec::new()).collect();

    let mut strip_sources: Vec<StripSource<'_>> = strips
        .iter_mut()
        .zip(processors_per_strip.iter_mut())
        .zip(patch_buffers.iter())
        .map(|((strip, processors), input)| StripSource {
            strip,
            inserts: &empty_chain,
            insert_processors: processors,
            input,
        })
        .collect();

    let master_bus = MixBus::new_master();
    let master_chain = EffectChain::new(0);
    let mut master_processors: Vec<Box<dyn EffectProcessor>> = Vec::new();
    let limiter = Limiter::unity_ceiling();
    let mut master = MasterSource {
        bus: &master_bus,
        inserts: &master_chain,
        insert_processors: &mut master_processors,
        limiter: &limiter,
    };

    mix_engine
        .render(total_samples, &mut strip_sources, &mut [], &mut master)
        .expect("mix pass succeeds")
}

// ─── Pure-Rust WAV writer (16-bit mono) ─────────────────────────────────────

fn write_wav(path: &Path, frames: &[AudioFrame], sample_rate: u32) {
    let samples: Vec<i16> = frames
        .iter()
        .map(|frame| {
            let mono = (frame.left + frame.right) * 0.5;
            let clamped = mono.clamp(-1.0, 1.0);
            (clamped * i16::MAX as f32) as i16
        })
        .collect();

    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * u32::from(num_channels) * u32::from(bits_per_sample) / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_chunk_size = (samples.len() * 2) as u32;
    let riff_size = 4 + 24 + 8 + data_chunk_size;

    let mut buf: Vec<u8> = Vec::with_capacity((12 + 24 + 8 + data_chunk_size) as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());

    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_chunk_size.to_le_bytes());
    for &sample in &samples {
        buf.extend_from_slice(&sample.to_le_bytes());
    }

    let mut file = fs::File::create(path).unwrap_or_else(|e| {
        eprintln!("error: cannot create '{}': {e}", path.display());
        process::exit(1);
    });
    file.write_all(&buf).unwrap_or_else(|e| {
        eprintln!("error: cannot write '{}': {e}", path.display());
        process::exit(1);
    });
}

// ─── Entry point ────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut midi_path: Option<PathBuf> = None;
    let mut out_path = PathBuf::from("mod-play.wav");
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --out requires a path argument");
                    process::exit(1);
                }
                out_path = PathBuf::from(&args[i]);
            }
            other => midi_path = Some(PathBuf::from(other)),
        }
        i += 1;
    }

    let timeline = match midi_path {
        Some(path) => load_timeline_from_file(&path),
        None => builtin_demo(),
    };

    let mut patches = build_patches();

    // Verbatim modulation-routing markers: every patch is configured with
    // both a vibrato routing (LFO -> pitch) and a filter-sweep routing
    // (LFO -> filter cutoff), so these two lines describe every patch's
    // ModMatrix.
    println!("mod routing: LFO vibrato -> pitch");
    println!("mod routing: sweep -> filter cutoff");

    let master = render(&timeline, &mut patches, SAMPLE_RATE_F64);
    write_wav(&out_path, &master, SAMPLE_RATE);

    for patch in &patches {
        println!(
            "Patch \"{}\": events_delivered={} peak_voices={} voice_steals={}",
            patch.name, patch.events_delivered, patch.peak_voices, patch.voice_steals
        );
    }
    println!(
        "total_samples={} duration={:.3}s out={}",
        master.len(),
        master.len() as f64 / SAMPLE_RATE_F64,
        out_path.display()
    );
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_demo_covers_all_three_channels() {
        let events = builtin_demo();
        assert!(events.iter().any(|e| e.channel == 0));
        assert!(events.iter().any(|e| e.channel == 1));
        assert!(events.iter().any(|e| e.channel == 2));
    }

    #[test]
    fn builtin_demo_events_are_sorted_with_offs_before_ons_at_same_instant() {
        let events = builtin_demo();
        for pair in events.windows(2) {
            assert!(pair[0].at_seconds <= pair[1].at_seconds);
        }
    }

    #[test]
    fn patches_have_independent_voice_pools() {
        let patches = build_patches();
        assert_eq!(patches[0].allocator.polyphony(), 4);
        assert_eq!(patches[1].allocator.polyphony(), 6);
        assert_eq!(patches[2].allocator.polyphony(), 3);
    }

    #[test]
    fn patches_subscribe_to_distinct_channels() {
        let patches = build_patches();
        assert!(patches[0].mapping().matches(0));
        assert!(!patches[0].mapping().matches(1));
        assert!(patches[1].mapping().matches(1));
        assert!(patches[2].mapping().matches(2));
    }

    #[test]
    fn dispatcher_delivers_only_to_matching_patch() {
        let dispatcher = MidiDispatcher::new();
        let mut patches = build_patches();

        let event = TimelineEvent {
            at_seconds: 0.0,
            channel: 1,
            kind: EventKind::NoteOn,
            note: 60,
            velocity: 0.8,
        };
        dispatch_event(&event, &dispatcher, &mut patches);

        assert_eq!(patches[0].events_delivered, 0);
        assert_eq!(patches[1].events_delivered, 1);
        assert_eq!(patches[2].events_delivered, 0);
        assert_eq!(patches[1].allocator.active_count(), 1);
    }

    #[test]
    fn each_patch_mod_matrix_has_vibrato_and_sweep_routings() {
        let patches = build_patches();
        for patch in &patches {
            let routes = patch.mod_matrix.routes();
            assert_eq!(routes.len(), 2);
            assert!(routes.iter().any(|r| r.destination_id == PITCH_ROUTE_DEST));
            assert!(routes.iter().any(|r| r.destination_id == FILTER_ROUTE_DEST));
        }
    }

    #[test]
    fn modulation_processor_produces_nonzero_pitch_and_filter_offsets_over_time() {
        let patches = build_patches();
        let patch = &patches[0];

        let mut saw_nonzero_pitch = false;
        let mut saw_nonzero_filter = false;
        for sample_idx in 0..2_000u64 {
            let mut sink: FixedOffsetBuffer<2> = FixedOffsetBuffer::new();
            patch.mod_processor.process(
                patch.mod_matrix.routes(),
                &[],
                sample_idx,
                SAMPLE_RATE_F64,
                &mut sink,
            );
            if sink.get(PITCH_ROUTE_DEST).abs() > 1e-6 {
                saw_nonzero_pitch = true;
            }
            if sink.get(FILTER_ROUTE_DEST).abs() > 1e-6 {
                saw_nonzero_filter = true;
            }
        }

        assert!(saw_nonzero_pitch, "expected a nonzero vibrato pitch offset");
        assert!(saw_nonzero_filter, "expected a nonzero filter sweep offset");
    }

    #[test]
    fn note_frequency_with_detune_matches_a4_with_zero_offset() {
        let frequency = note_frequency_with_detune(69, 0.0);
        assert!((frequency.hertz() - 440.0).abs() < 1e-6);
    }

    #[test]
    fn note_frequency_with_detune_shifts_up_by_one_semitone() {
        let base = note_frequency_with_detune(69, 0.0).hertz();
        let shifted = note_frequency_with_detune(69, 1.0).hertz();
        let expected_ratio = 2f64.powf(1.0 / 12.0);
        assert!(((shifted / base) - expected_ratio).abs() < 1e-9);
    }

    #[test]
    fn render_produces_the_expected_tail_padded_length() {
        let timeline = vec![TimelineEvent {
            at_seconds: 0.0,
            channel: 0,
            kind: EventKind::NoteOn,
            note: 60,
            velocity: 0.8,
        }];
        let mut patches = build_patches();
        let frames = render(&timeline, &mut patches, SAMPLE_RATE_F64);

        let expected = ((0.0 + TAIL_SECS) * SAMPLE_RATE_F64).ceil() as usize;
        assert_eq!(frames.len(), expected);
    }

    #[test]
    fn render_delivers_events_and_reports_peak_voices_per_patch() {
        let timeline = builtin_demo();
        let mut patches = build_patches();
        let _ = render(&timeline, &mut patches, SAMPLE_RATE_F64);

        for patch in &patches {
            assert!(
                patch.events_delivered > 0,
                "{} received no events",
                patch.name
            );
            assert!(
                patch.peak_voices > 0,
                "{} never sounded a voice",
                patch.name
            );
        }
    }
}
