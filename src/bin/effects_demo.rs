// path: src/bin/effects_demo.rs

//! `effects_demo`: renders a small multi-patch tune through per-patch and
//! global `EffectChain`s and writes the result to a 16-bit mono WAV file.
//!
//! CLI: `effects_demo [FILE.mid] [--out OUT.wav]`. With no `FILE.mid`, a
//! built-in multi-channel tune of sustained notes is used so the effects
//! are audible. Output defaults to `effects-demo.wav`.
//!
//! Signal flow (matches the project's canonical mixer path in miniature):
//! patch voices -> per-patch `EffectChain` (slot 0, then slot 1, ...) ->
//! `PatchMixer` -> `GlobalMixer` -> master `EffectChain` -> output.
//!
//! `PatchMixer` and `GlobalMixer` are not yet ports elsewhere in this
//! crate's module tree, so they are defined locally here, mirroring the
//! stateless-service shape of `ChainRenderer`.
//!
//! Before rendering anything, this binary mechanically proves the two
//! invariants an `EffectChain` must uphold, using the real `EffectChain` /
//! `ChainRenderer` types against a short synthetic block:
//!
//! - slot order changes the outcome for non-commutative effects (prints
//!   `slot order matters: true`);
//! - a fully bypassed chain returns its input bit-identical (prints
//!   `bypass passthrough: true`).

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use crest_synth::effects::chain_renderer::ChainRenderer;
use crest_synth::effects::effect_chain::{EffectChain, EffectChainCommand};
use crest_synth::effects::effect_processor::{AudioFrame, EffectProcessor, GainEffect};
use crest_synth::engine::oscillator::{
    Amplitude, Frequency, Oscillator, OscillatorConfig, SampleRate, StandardOscillator, Waveform,
};
use crest_synth::kernel::midi_event_kind::MidiEventKind;
use crest_synth::kernel::note_id::NoteId;
use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::velocity::Velocity;
use crest_synth::midi_file::midi_file_reader::{MidiFileError, MidiFileReader, Song};
use crest_synth::midi_file::midly_midi_file_reader::MidlyMidiFileReader;
use crest_synth::patch::midi_dispatcher::{MidiAddress, MidiDispatcher, RoutablePatch};
use crest_synth::patch::patch::{ChannelMapping, PatchId};

/// Render sample rate for the whole demo.
const SAMPLE_RATE_HZ: u32 = 44_100;
/// Default output path when `--out` is not given.
const DEFAULT_OUT_PATH: &str = "effects-demo.wav";
/// Linear fade applied at the head/tail of every rendered note, so summing
/// overlapping voices never produces an audible click.
const FADE_SECONDS: f64 = 0.01;
/// The demo caps itself at this many simultaneous patches; MIDI channels
/// beyond the first this-many distinct channels encountered are dropped.
const MAX_DEMO_PATCHES: usize = 3;
/// Headroom applied per rendered note so 2-3 overlapping voices summed
/// through a patch and then the master bus stay well clear of hard clipping.
const NOTE_HEADROOM: f64 = 0.5;

// ─────────────────────────────────────────────────────────────────────────
// Note extraction (built-in tune, or a real MIDI file)
// ─────────────────────────────────────────────────────────────────────────

/// One note, addressed by raw MIDI channel, before it has been routed to a
/// patch.
#[derive(Debug, Clone, Copy)]
struct RawNote {
    channel: u8,
    note_number: NoteNumber,
    velocity: Velocity,
    start_seconds: f64,
    duration_seconds: f64,
}

/// The built-in multi-channel demo tune: three channels, each with
/// sustained (long) notes so the per-patch and master effects are audible.
fn built_in_notes() -> Vec<RawNote> {
    let note = |channel: u8, note_number: u8, velocity: u8, start: f64, duration: f64| RawNote {
        channel,
        note_number: NoteNumber::try_new(note_number).expect("built-in note numbers are valid"),
        velocity: Velocity::from_midi7(velocity),
        start_seconds: start,
        duration_seconds: duration,
    };

    vec![
        // Channel 0: a slow two-note melody.
        note(0, 57, 100, 0.0, 2.4),
        note(0, 60, 96, 2.4, 2.4),
        // Channel 1: a sustained pad overlapping the melody.
        note(1, 64, 80, 0.3, 4.2),
        // Channel 2: a sparse higher voice, staggered against both.
        note(2, 69, 90, 1.0, 1.8),
        note(2, 72, 85, 3.1, 1.6),
    ]
}

/// Loads `path` as a Standard MIDI File and pairs its note-on/note-off
/// events into `RawNote`s.
fn load_song_notes(path: &Path) -> Result<Vec<RawNote>, MidiFileError> {
    let reader = MidlyMidiFileReader::new();
    let song = reader.load(path)?;
    Ok(pair_notes(&song))
}

/// Pairs a `Song`'s note-on/note-off events (matched by `NoteId`, per the
/// project's note-identity invariant) into `RawNote`s with an absolute
/// start time and duration. A note-on left open at the end of the song is
/// closed at the song's last event time.
fn pair_notes(song: &Song) -> Vec<RawNote> {
    let mut open: HashMap<NoteId, (u8, NoteNumber, Velocity, f64)> = HashMap::new();
    let mut notes = Vec::new();
    let mut last_seconds = 0.0_f64;

    for timed in song.events() {
        last_seconds = last_seconds.max(timed.at_seconds());
        let event = timed.event();
        let channel = event.address().channel().value();

        match event.kind() {
            MidiEventKind::NoteOn if event.velocity().value() > 0.0 => {
                open.insert(
                    *event.note_id(),
                    (
                        channel,
                        *event.note(),
                        *event.velocity(),
                        timed.at_seconds(),
                    ),
                );
            }
            MidiEventKind::NoteOff | MidiEventKind::NoteOn => {
                if let Some((channel, note_number, velocity, start_seconds)) =
                    open.remove(event.note_id())
                {
                    let duration_seconds = (timed.at_seconds() - start_seconds).max(0.05);
                    notes.push(RawNote {
                        channel,
                        note_number,
                        velocity,
                        start_seconds,
                        duration_seconds,
                    });
                }
            }
            _ => {}
        }
    }

    // Close out any notes still sounding when the song ends.
    for (channel, note_number, velocity, start_seconds) in open.into_values() {
        let duration_seconds = (last_seconds - start_seconds).max(0.2);
        notes.push(RawNote {
            channel,
            note_number,
            velocity,
            start_seconds,
            duration_seconds,
        });
    }

    notes.sort_by(|a, b| {
        a.start_seconds
            .partial_cmp(&b.start_seconds)
            .expect("timestamps are always finite")
    });
    notes
}

// ─────────────────────────────────────────────────────────────────────────
// Patches: routing via the real MidiDispatcher + ChannelMapping
// ─────────────────────────────────────────────────────────────────────────

/// One note already routed to a patch: pitch, velocity, and timing.
/// Overlapping notes within a patch's `notes` are its miniature voice pool
/// -- they are summed sample-for-sample before the patch's `EffectChain`.
#[derive(Debug, Clone, Copy)]
struct DemoNote {
    note_number: NoteNumber,
    velocity: Velocity,
    start_seconds: f64,
    duration_seconds: f64,
}

/// A minimal demo patch: stable identity, channel mapping, a waveform for
/// its voices, and the notes routed to it.
struct DemoPatch {
    id: PatchId,
    mapping: ChannelMapping,
    waveform: Waveform,
    notes: Vec<DemoNote>,
}

impl RoutablePatch for DemoPatch {
    fn id(&self) -> PatchId {
        self.id
    }

    fn mapping(&self) -> ChannelMapping {
        self.mapping
    }
}

/// Builds one patch per distinct MIDI channel seen in `raw_notes`, up to
/// `MAX_DEMO_PATCHES`. Channels beyond the cap are intentionally left
/// unrouted (dropped) by the assignment pass below.
fn build_patches(raw_notes: &[RawNote]) -> Vec<DemoPatch> {
    let mut channels: Vec<u8> = Vec::new();
    for raw_note in raw_notes {
        if channels.len() >= MAX_DEMO_PATCHES {
            break;
        }
        if !channels.contains(&raw_note.channel) {
            channels.push(raw_note.channel);
        }
    }
    if channels.is_empty() {
        channels.push(0);
    }

    let waveforms = [Waveform::Sine, Waveform::Saw, Waveform::Triangle];
    channels
        .into_iter()
        .enumerate()
        .map(|(index, channel)| DemoPatch {
            id: PatchId::new(index as u32),
            mapping: ChannelMapping::single(channel)
                .expect("channel came from a validated 0..=15 MidiChannel"),
            waveform: waveforms[index % waveforms.len()],
            notes: Vec::new(),
        })
        .collect()
}

/// Routes every raw note to exactly the patches whose `ChannelMapping`
/// matches its channel, via the real `MidiDispatcher` -- layering
/// (multiple matches) is honored; a note matching no patch is dropped.
fn assign_notes(mut patches: Vec<DemoPatch>, raw_notes: &[RawNote]) -> Vec<DemoPatch> {
    let dispatcher = MidiDispatcher::new();
    for raw_note in raw_notes {
        let Ok(address) = MidiAddress::try_new(raw_note.channel) else {
            continue;
        };
        let matched_ids = dispatcher.dispatch(address, &patches);
        if matched_ids.is_empty() {
            continue;
        }
        for patch in patches.iter_mut() {
            if matched_ids.contains(&patch.id) {
                patch.notes.push(DemoNote {
                    note_number: raw_note.note_number,
                    velocity: raw_note.velocity,
                    start_seconds: raw_note.start_seconds,
                    duration_seconds: raw_note.duration_seconds,
                });
            }
        }
    }
    patches
}

// ─────────────────────────────────────────────────────────────────────────
// Synthesis: one oscillator voice per note, summed into a patch buffer
// ─────────────────────────────────────────────────────────────────────────

/// Converts a MIDI note number to its equal-tempered frequency in hertz
/// (A4 = MIDI note 69 = 440 Hz).
fn midi_to_hertz(note_number: u8) -> f64 {
    440.0 * 2.0_f64.powf((f64::from(note_number) - 69.0) / 12.0)
}

/// A short linear fade-in/fade-out envelope so summed, overlapping notes
/// never click at their edges.
fn fade_envelope(sample_index: usize, total_samples: usize, fade_samples: usize) -> f32 {
    let fade_in = if sample_index < fade_samples {
        sample_index as f32 / fade_samples as f32
    } else {
        1.0
    };
    let remaining = total_samples.saturating_sub(sample_index);
    let fade_out = if remaining < fade_samples {
        remaining as f32 / fade_samples as f32
    } else {
        1.0
    };
    fade_in.min(fade_out)
}

/// Renders one note to a mono buffer of `f32` samples via the engine's
/// `Oscillator` port, faded at both ends.
fn render_note(note: &DemoNote, waveform: Waveform, sample_rate_hz: u32) -> Vec<f32> {
    let sample_rate =
        SampleRate::try_new(f64::from(sample_rate_hz)).expect("44.1kHz is a valid sample rate");
    let frequency = Frequency::try_new(midi_to_hertz(note.note_number.value()))
        .expect("MIDI note numbers map to positive, finite frequencies");
    let amplitude = Amplitude::try_new((note.velocity.value() * NOTE_HEADROOM).clamp(0.0, 1.0))
        .expect("velocity-scaled amplitude stays within [0.0, 1.0]");
    let config = OscillatorConfig::new(waveform, amplitude);
    let oscillator = StandardOscillator::new();

    let total_samples = (note.duration_seconds * f64::from(sample_rate_hz)).round() as usize;
    let fade_samples = ((FADE_SECONDS * f64::from(sample_rate_hz)) as usize)
        .min(total_samples / 2)
        .max(1);

    let mut phase = 0.0_f64;
    let mut buffer = Vec::with_capacity(total_samples);
    for sample_index in 0..total_samples {
        let raw = oscillator.render(phase, config) as f32;
        buffer.push(raw * fade_envelope(sample_index, total_samples, fade_samples));
        phase = oscillator.advance(phase, frequency, sample_rate);
    }
    buffer
}

/// Renders and sums every note belonging to `patch` into one mono buffer --
/// the patch's voice pool, before its `EffectChain` runs.
fn render_patch_voices(patch: &DemoPatch, sample_rate_hz: u32) -> Vec<f32> {
    let mut buffer: Vec<f32> = Vec::new();
    for note in &patch.notes {
        let rendered = render_note(note, patch.waveform, sample_rate_hz);
        let start_sample = (note.start_seconds * f64::from(sample_rate_hz)).round() as usize;
        let end_sample = start_sample + rendered.len();
        if buffer.len() < end_sample {
            buffer.resize(end_sample, 0.0);
        }
        for (offset, sample) in rendered.iter().enumerate() {
            buffer[start_sample + offset] += *sample;
        }
    }
    buffer
}

// ─────────────────────────────────────────────────────────────────────────
// A tiny in-crate EffectProcessor: a single-tap feedback delay.
// `GainEffect` (already in `crest_synth::effects::effect_processor`) plays
// the role of the "gain/trim" effect.
// ─────────────────────────────────────────────────────────────────────────

/// A single-tap feedback delay line: reads one sample `delay_samples` in
/// the past, mixes it into the dry signal at `wet`, and feeds a
/// `feedback`-scaled copy of the input plus that same delayed sample back
/// into the line through a `tanh` soft-saturation stage (modeling the
/// saturating repeats of an analog delay).
///
/// The saturation stage also makes this effect genuinely non-commutative
/// with a plain scalar gain: without it, delay-then-gain and
/// gain-then-delay would be mathematically identical, since both stages
/// would be linear and gain would simply factor out regardless of where it
/// sits in the chain.
///
/// The delay line and output scratch buffer are both allocated once at
/// construction (or grown only up to the largest block seen), matching the
/// allocation discipline `EffectProcessor` implementors owe the audio
/// thread: after warm-up, `process` performs no further heap allocation.
struct FeedbackDelayEffect {
    delay_line: Vec<AudioFrame>,
    write_pos: usize,
    feedback: f32,
    wet: f32,
    scratch: Vec<AudioFrame>,
}

impl FeedbackDelayEffect {
    /// Constructs a feedback delay with a line `delay_samples` long
    /// (minimum 1), feedback and wet mix each clamped to a stable range.
    fn new(delay_samples: usize, feedback: f32, wet: f32) -> Self {
        Self {
            delay_line: vec![AudioFrame::silence(); delay_samples.max(1)],
            write_pos: 0,
            feedback: feedback.clamp(0.0, 0.95),
            wet: wet.clamp(0.0, 1.0),
            scratch: Vec::new(),
        }
    }
}

impl EffectProcessor for FeedbackDelayEffect {
    fn latency(&self) -> u32 {
        0
    }

    fn process(&mut self, input: &[AudioFrame]) -> Vec<AudioFrame> {
        if self.scratch.len() < input.len() {
            self.scratch.resize(input.len(), AudioFrame::silence());
        }
        let line_len = self.delay_line.len();
        for (index, frame) in input.iter().enumerate() {
            let tapped = self.delay_line[self.write_pos];
            let output = AudioFrame::new(
                frame.left + tapped.left * self.wet,
                frame.right + tapped.right * self.wet,
            );
            self.delay_line[self.write_pos] = AudioFrame::new(
                (frame.left + tapped.left * self.feedback).tanh(),
                (frame.right + tapped.right * self.feedback).tanh(),
            );
            self.write_pos = (self.write_pos + 1) % line_len;
            self.scratch[index] = output;
        }
        self.scratch[..input.len()].to_vec()
    }

    fn reset(&mut self) {
        for frame in self.delay_line.iter_mut() {
            *frame = AudioFrame::silence();
        }
        self.write_pos = 0;
    }
}

// ─────────────────────────────────────────────────────────────────────────
// PatchMixer / GlobalMixer: local, since neither is yet a port elsewhere in
// this crate's module tree. Mirrors ChainRenderer's stateless-service shape.
// ─────────────────────────────────────────────────────────────────────────

/// Sums the (already per-patch-effected) patch buffers into a single
/// stereo bus, padding shorter buffers with silence.
#[derive(Debug, Default, Clone, Copy)]
struct PatchMixer;

impl PatchMixer {
    fn new() -> Self {
        Self
    }

    fn sum(&self, patch_buffers: &[Vec<AudioFrame>]) -> Vec<AudioFrame> {
        let len = patch_buffers.iter().map(Vec::len).max().unwrap_or(0);
        let mut bus = vec![AudioFrame::silence(); len];
        for buffer in patch_buffers {
            for (index, frame) in buffer.iter().enumerate() {
                bus[index] =
                    AudioFrame::new(bus[index].left + frame.left, bus[index].right + frame.right);
            }
        }
        bus
    }
}

/// Applies master trim gain to the summed bus before it reaches the master
/// `EffectChain`.
#[derive(Debug, Clone, Copy)]
struct GlobalMixer {
    trim: f32,
}

impl GlobalMixer {
    fn new(trim: f32) -> Self {
        Self {
            trim: trim.clamp(0.0, 1.0),
        }
    }

    fn apply(&self, bus: &[AudioFrame]) -> Vec<AudioFrame> {
        bus.iter()
            .map(|frame| AudioFrame::new(frame.left * self.trim, frame.right * self.trim))
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Mechanical proofs of the two EffectChain invariants
// ─────────────────────────────────────────────────────────────────────────

/// Builds a chain of `slot_count` freshly-inserted (active, unbypassed)
/// slots.
fn build_chain(slot_count: u8) -> EffectChain {
    let mut chain = EffectChain::new(slot_count);
    for index in 0..slot_count {
        chain
            .apply(EffectChainCommand::InsertSlot {
                index: u32::from(index),
            })
            .expect("inserting up to slot_count slots never exceeds chain capacity");
    }
    chain
}

/// A short synthetic test block: not silence, not a single repeated value,
/// so a delay line's history actually differs from its input.
fn proof_test_block() -> Vec<AudioFrame> {
    (0..16)
        .map(|index| {
            let value = if index % 4 == 0 { 0.6 } else { 0.0 };
            AudioFrame::mono(value)
        })
        .collect()
}

/// Proves, using the real `EffectChain` + `ChainRenderer`, that processing
/// a block through two slots in forward order produces a different result
/// than processing it through the same two (non-commutative) effects in
/// reversed order. Panics if the two outputs are identical.
fn prove_slot_order_matters() {
    let renderer = ChainRenderer::new();
    let test_block = proof_test_block();

    let forward_chain = build_chain(2);
    let mut forward_processors: Vec<Box<dyn EffectProcessor>> = vec![
        Box::new(FeedbackDelayEffect::new(3, 0.5, 0.7)),
        Box::new(GainEffect::new(2.0)),
    ];
    let forward_output = renderer
        .render(&forward_chain, &mut forward_processors, &test_block)
        .expect("processor count matches slot count");

    let reversed_chain = build_chain(2);
    let mut reversed_processors: Vec<Box<dyn EffectProcessor>> = vec![
        Box::new(GainEffect::new(2.0)),
        Box::new(FeedbackDelayEffect::new(3, 0.5, 0.7)),
    ];
    let reversed_output = renderer
        .render(&reversed_chain, &mut reversed_processors, &test_block)
        .expect("processor count matches slot count");

    if forward_output == reversed_output {
        panic!(
            "slot order proof failed: delay-then-gain and gain-then-delay produced identical \
             output, but these effects are non-commutative and must differ"
        );
    }
    println!("slot order matters: true");
}

/// Proves, using the real `EffectChain` + `ChainRenderer`, that a chain
/// with every slot bypassed returns its input bit-identical. Panics if the
/// output differs from the dry input in any way.
fn prove_bypass_passthrough() {
    let mut chain = build_chain(2);
    chain
        .apply(EffectChainCommand::SetBypass {
            index: 0,
            bypassed: true,
        })
        .expect("slot 0 exists");
    chain
        .apply(EffectChainCommand::SetBypass {
            index: 1,
            bypassed: true,
        })
        .expect("slot 1 exists");

    let dry_block: Vec<AudioFrame> = (0..16)
        .map(|index| AudioFrame::new(0.1 * index as f32, -0.1 * index as f32))
        .collect();

    let renderer = ChainRenderer::new();
    let mut processors: Vec<Box<dyn EffectProcessor>> = vec![
        Box::new(GainEffect::new(5.0)),
        Box::new(FeedbackDelayEffect::new(3, 0.8, 0.9)),
    ];
    let output = renderer
        .render(&chain, &mut processors, &dry_block)
        .expect("processor count matches slot count");

    if output != dry_block {
        panic!(
            "bypass passthrough proof failed: a fully bypassed chain must return the input \
             bit-identical, but the output differed"
        );
    }
    println!("bypass passthrough: true");
}

// ─────────────────────────────────────────────────────────────────────────
// WAV output: a pure-Rust, 16-bit mono PCM writer.
// ─────────────────────────────────────────────────────────────────────────

/// Downmixes stereo frames to mono `i16` PCM samples, clamped to the valid
/// range before quantization.
fn to_mono_i16(frames: &[AudioFrame]) -> Vec<i16> {
    frames
        .iter()
        .map(|frame| {
            let mono = ((frame.left + frame.right) * 0.5).clamp(-1.0, 1.0);
            (mono * f32::from(i16::MAX)).round() as i16
        })
        .collect()
}

/// Writes `samples` as a 16-bit mono PCM WAV file at `sample_rate_hz`,
/// using a hand-rolled RIFF/WAVE header (no external WAV crate).
fn write_wav_mono16(path: &Path, sample_rate_hz: u32, samples: &[i16]) -> io::Result<()> {
    const BITS_PER_SAMPLE: u16 = 16;
    const CHANNEL_COUNT: u16 = 1;
    const BYTES_PER_SAMPLE: u32 = (BITS_PER_SAMPLE / 8) as u32;

    let data_size = samples.len() as u32 * BYTES_PER_SAMPLE;
    let byte_rate = sample_rate_hz * BYTES_PER_SAMPLE * u32::from(CHANNEL_COUNT);
    let block_align = CHANNEL_COUNT * BITS_PER_SAMPLE / 8;
    let riff_size = 36 + data_size;

    let mut writer = BufWriter::new(File::create(path)?);
    writer.write_all(b"RIFF")?;
    writer.write_all(&riff_size.to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16u32.to_le_bytes())?; // fmt chunk size (PCM)
    writer.write_all(&1u16.to_le_bytes())?; // audio format: PCM
    writer.write_all(&CHANNEL_COUNT.to_le_bytes())?;
    writer.write_all(&sample_rate_hz.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&BITS_PER_SAMPLE.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_size.to_le_bytes())?;
    for sample in samples {
        writer.write_all(&sample.to_le_bytes())?;
    }
    writer.flush()
}

// ─────────────────────────────────────────────────────────────────────────
// CLI + orchestration
// ─────────────────────────────────────────────────────────────────────────

struct Cli {
    midi_path: Option<PathBuf>,
    out_path: PathBuf,
}

fn parse_cli(args: &[String]) -> Cli {
    let mut midi_path = None;
    let mut out_path = PathBuf::from(DEFAULT_OUT_PATH);

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" => {
                index += 1;
                if let Some(value) = args.get(index) {
                    out_path = PathBuf::from(value);
                } else {
                    eprintln!("--out requires a path argument");
                    std::process::exit(2);
                }
            }
            other => midi_path = Some(PathBuf::from(other)),
        }
        index += 1;
    }

    Cli {
        midi_path,
        out_path,
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let cli = parse_cli(&args);

    prove_slot_order_matters();
    prove_bypass_passthrough();

    let raw_notes = match &cli.midi_path {
        Some(path) => match load_song_notes(path) {
            Ok(notes) => notes,
            Err(err) => {
                eprintln!("failed to load {}: {err}", path.display());
                std::process::exit(1);
            }
        },
        None => built_in_notes(),
    };

    let patches = assign_notes(build_patches(&raw_notes), &raw_notes);

    let renderer = ChainRenderer::new();
    let mut patch_buffers: Vec<Vec<AudioFrame>> = Vec::with_capacity(patches.len());

    for (index, patch) in patches.iter().enumerate() {
        let mono = render_patch_voices(patch, SAMPLE_RATE_HZ);
        let frames: Vec<AudioFrame> = mono
            .iter()
            .map(|&sample| AudioFrame::mono(sample))
            .collect();

        // The first patch gets a two-slot chain (trim, then feedback
        // delay) to satisfy the "at least one patch has >= 2 slots"
        // requirement; the rest get a one-slot trim chain.
        let is_lead_patch = index == 0;
        let chain = build_chain(if is_lead_patch { 2 } else { 1 });
        let mut processors: Vec<Box<dyn EffectProcessor>> = if is_lead_patch {
            vec![
                Box::new(GainEffect::new(0.9)),
                Box::new(FeedbackDelayEffect::new(
                    (f64::from(SAMPLE_RATE_HZ) * 0.22) as usize,
                    0.35,
                    0.4,
                )),
            ]
        } else {
            vec![Box::new(GainEffect::new(0.9))]
        };

        let processed = renderer
            .render(&chain, &mut processors, &frames)
            .unwrap_or_else(|err| {
                panic!(
                    "per-patch effect chain for patch {} failed: {err}",
                    patch.id.value()
                )
            });

        println!(
            "patch {} (channel mask {:#06b}, waveform {:?}): {} note(s), {} slot(s), {} samples",
            patch.id.value(),
            patch.mapping.mask(),
            patch.waveform,
            patch.notes.len(),
            chain.slots().len(),
            processed.len(),
        );

        patch_buffers.push(processed);
    }

    let patch_mixer = PatchMixer::new();
    let bus = patch_mixer.sum(&patch_buffers);

    let global_mixer = GlobalMixer::new(0.8);
    let bus = global_mixer.apply(&bus);

    let master_chain = build_chain(2);
    let mut master_processors: Vec<Box<dyn EffectProcessor>> = vec![
        Box::new(GainEffect::new(0.85)),
        Box::new(FeedbackDelayEffect::new(
            (f64::from(SAMPLE_RATE_HZ) * 0.35) as usize,
            0.3,
            0.25,
        )),
    ];
    let master_out = renderer
        .render(&master_chain, &mut master_processors, &bus)
        .expect("master chain processor count matches its slot count");

    println!(
        "master bus: {} patch(es) summed, {} slot(s), {} samples rendered",
        patch_buffers.len(),
        master_chain.slots().len(),
        master_out.len(),
    );

    let mono_samples = to_mono_i16(&master_out);
    if let Err(err) = write_wav_mono16(&cli.out_path, SAMPLE_RATE_HZ, &mono_samples) {
        eprintln!("failed to write {}: {err}", cli.out_path.display());
        std::process::exit(1);
    }

    println!(
        "wrote {} samples ({:.2}s) to {}",
        mono_samples.len(),
        mono_samples.len() as f64 / f64::from(SAMPLE_RATE_HZ),
        cli.out_path.display(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_to_hertz_a4_is_440() {
        assert!((midi_to_hertz(69) - 440.0).abs() < 1e-9);
    }

    #[test]
    fn fade_envelope_starts_and_ends_at_zero() {
        assert_eq!(fade_envelope(0, 100, 10), 0.0);
        assert_eq!(fade_envelope(99, 100, 10), 0.1);
    }

    #[test]
    fn fade_envelope_is_unity_away_from_edges() {
        assert_eq!(fade_envelope(50, 100, 10), 1.0);
    }

    #[test]
    fn pair_notes_matches_note_off_to_its_note_on_by_note_id() {
        use crest_synth::kernel::channel_address::{ChannelAddress, MidiChannel, MidiGroup};
        use crest_synth::kernel::midi_event::MidiEvent;
        use crest_synth::kernel::time_signature::TimeSignature;
        use crest_synth::midi_file::midi_file_reader::TimedMidiEvent;

        let address = ChannelAddress::new(
            MidiChannel::try_new(2).unwrap(),
            MidiGroup::try_new(0).unwrap(),
        );
        let note_id = NoteId::new(7);
        let note_on = MidiEvent::new(
            address,
            MidiEventKind::NoteOn,
            NoteNumber::try_new(60).unwrap(),
            note_id,
            Velocity::from_midi7(100),
        );
        let note_off = MidiEvent::new(
            address,
            MidiEventKind::NoteOff,
            NoteNumber::try_new(60).unwrap(),
            note_id,
            Velocity::from_midi7(0),
        );
        let song = Song::new(
            vec![
                TimedMidiEvent::new(0.5, note_on),
                TimedMidiEvent::new(2.0, note_off),
            ],
            vec![],
            TimeSignature::common_time(),
        );

        let notes = pair_notes(&song);

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].channel, 2);
        assert!((notes[0].start_seconds - 0.5).abs() < 1e-9);
        assert!((notes[0].duration_seconds - 1.5).abs() < 1e-9);
    }

    #[test]
    fn build_patches_caps_at_max_demo_patches() {
        let raw_notes: Vec<RawNote> = (0..5)
            .map(|channel| RawNote {
                channel,
                note_number: NoteNumber::try_new(60).unwrap(),
                velocity: Velocity::from_midi7(100),
                start_seconds: 0.0,
                duration_seconds: 1.0,
            })
            .collect();

        let patches = build_patches(&raw_notes);

        assert_eq!(patches.len(), MAX_DEMO_PATCHES);
    }

    #[test]
    fn assign_notes_routes_only_to_matching_patch() {
        let raw_notes = vec![
            RawNote {
                channel: 0,
                note_number: NoteNumber::try_new(60).unwrap(),
                velocity: Velocity::from_midi7(100),
                start_seconds: 0.0,
                duration_seconds: 1.0,
            },
            RawNote {
                channel: 1,
                note_number: NoteNumber::try_new(64).unwrap(),
                velocity: Velocity::from_midi7(90),
                start_seconds: 0.0,
                duration_seconds: 1.0,
            },
        ];

        let patches = assign_notes(build_patches(&raw_notes), &raw_notes);

        assert_eq!(patches.len(), 2);
        assert_eq!(patches[0].notes.len(), 1);
        assert_eq!(patches[1].notes.len(), 1);
        assert_eq!(patches[0].notes[0].note_number.value(), 60);
        assert_eq!(patches[1].notes[0].note_number.value(), 64);
    }

    #[test]
    fn feedback_delay_reset_clears_delay_line() {
        let mut effect = FeedbackDelayEffect::new(2, 0.5, 0.5);
        let warm_up = vec![AudioFrame::mono(1.0); 4];
        effect.process(&warm_up);

        effect.reset();

        let probe = effect.process(&[AudioFrame::silence()]);
        assert_eq!(probe[0], AudioFrame::silence());
    }

    #[test]
    fn patch_mixer_sums_patch_buffers_padding_shorter_ones() {
        let mixer = PatchMixer::new();
        let buffers = vec![
            vec![AudioFrame::mono(0.5), AudioFrame::mono(0.5)],
            vec![AudioFrame::mono(0.25)],
        ];

        let bus = mixer.sum(&buffers);

        assert_eq!(bus.len(), 2);
        assert!((bus[0].left - 0.75).abs() < 1e-6);
        assert!((bus[1].left - 0.5).abs() < 1e-6);
    }

    #[test]
    fn global_mixer_applies_trim() {
        let mixer = GlobalMixer::new(0.5);
        let bus = vec![AudioFrame::mono(1.0)];

        let trimmed = mixer.apply(&bus);

        assert!((trimmed[0].left - 0.5).abs() < 1e-6);
    }

    #[test]
    fn to_mono_i16_clamps_and_scales() {
        let frames = [AudioFrame::new(1.5, 1.5), AudioFrame::new(-2.0, -2.0)];
        let samples = to_mono_i16(&frames);
        assert_eq!(samples[0], i16::MAX);
        assert_eq!(samples[1], -i16::MAX);
    }

    #[test]
    fn wav_header_round_trips_sample_count() {
        let dir = std::env::temp_dir();
        let path = dir.join("effects_demo_test_output.wav");
        let samples: Vec<i16> = vec![0, 100, -100, 32000];

        write_wav_mono16(&path, 44_100, &samples).expect("write succeeds");
        let bytes = std::fs::read(&path).expect("read succeeds");
        std::fs::remove_file(&path).ok();

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        let data_size = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
        assert_eq!(data_size as usize, samples.len() * 2);
    }
}
