// path: src/bin/preset_demo.rs

//! `preset_demo`: builds a full `Setup` (several distinct `Patch`es captured
//! as `Preset`s plus master gain), round-trips it through a JSON
//! `PresetCodec`, and proves the round trip is faithful by rendering the
//! same fixed demo passage through the original `Setup` and the reloaded
//! `Setup'` and asserting the rendered audio is bit-identical.
//!
//! `port.Presets.PresetCodec` has not landed in the module tree as a shared
//! port yet, so — per this project's convention for referencing
//! not-yet-available types (see `src/preset/preset_codec.rs`) — the codec
//! and the `Preset`/`Setup` value objects it serializes are defined locally
//! in this binary. They mirror the real domain shapes (oscillator, filter,
//! and amp-envelope configuration, gain/pan, channel subscription, master
//! gain) closely enough to exercise a real preset-integrity round trip, and
//! delegate all actual sound generation to the engine's real ports
//! (`engine::oscillator`, `engine::filter`, `engine::envelope_config`,
//! `engine::envelope_generator`) so the render path is genuine, not a stub.
//!
//! CLI: `preset_demo [--out OUT.wav]`. Default output path `preset-demo.wav`.

use std::env;
use std::error::Error;
use std::f64::consts::FRAC_PI_4;
use std::fmt;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crest_synth::engine::envelope_config::EnvelopeConfig;
use crest_synth::engine::envelope_generator::EnvelopeGenerator;
use crest_synth::engine::filter::{Filter, FilterConfig, FilterKind, StateVariableFilter};
use crest_synth::engine::oscillator::{
    Amplitude, Frequency, Oscillator, OscillatorConfig, SampleRate, StandardOscillator, Waveform,
};

/// Fixed render sample rate for the whole demo, in samples per second.
const SAMPLE_RATE_HZ: f64 = 44_100.0;
/// Default WAV output path when `--out` is not given.
const DEFAULT_OUTPUT_PATH: &str = "preset-demo.wav";
/// Total length of the built-in demo passage, in seconds. Chosen generously
/// so every note's envelope (including release tail) completes.
const DEMO_TOTAL_SECONDS: f64 = 2.5;

/// Oldest preset format version this codec still accepts.
const MIN_SUPPORTED_PRESET_VERSION: u32 = 1;
/// The format version newly authored presets are stamped with, and the
/// version every successfully decoded preset ends up at after migration.
const CURRENT_PRESET_VERSION: u32 = 1;

// ---------------------------------------------------------------------
// Serialized preset / setup value objects.
// ---------------------------------------------------------------------

/// The waveform shape an oscillator renders, mirroring
/// `engine::oscillator::Waveform` in a serde-friendly local shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum DemoWaveform {
    Sine,
    Saw,
    Square,
    Triangle,
}

/// Oscillator settings captured in a preset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct DemoOscillatorConfig {
    waveform: DemoWaveform,
    amplitude: f64,
}

/// The filter topology applied to a voice, mirroring
/// `engine::filter::FilterKind` in a serde-friendly local shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum DemoFilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

/// Filter settings captured in a preset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct DemoFilterConfig {
    filter_type: DemoFilterType,
    cutoff_hz: f64,
    resonance: f64,
}

/// ADSR amp-envelope settings captured in a preset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct DemoAmpEnvelopeConfig {
    attack: f64,
    decay: f64,
    release: f64,
    sustain: f64,
}

/// Which MIDI channels (0-15) address this patch, as a 16-bit mask so a
/// patch can be layered across multiple channels. Matching is what makes
/// layering intentional: a `MidiEvent` is dispatched to exactly the set of
/// patches whose mapping matches its address (multiple matches allowed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct DemoChannelSubscription {
    mask: u16,
}

/// One patch's complete configuration, as captured by a `Preset`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DemoPatchConfig {
    name: String,
    oscillator: DemoOscillatorConfig,
    filter: DemoFilterConfig,
    amp_envelope: DemoAmpEnvelopeConfig,
    gain: f64,
    pan: f64,
    channels: DemoChannelSubscription,
}

/// A versioned snapshot of one patch's complete configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Preset {
    version: u32,
    name: String,
    patch: DemoPatchConfig,
}

/// A full synth setup: every patch (as a `Preset`) plus master gain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Setup {
    version: u32,
    master_gain: f64,
    presets: Vec<Preset>,
}

// ---------------------------------------------------------------------
// PresetCodec port (implemented inline; see module docs).
// ---------------------------------------------------------------------

/// Everything that can go wrong turning bytes into a `Preset`/`Setup` or
/// back.
#[derive(Debug, Clone, PartialEq)]
enum CodecError {
    /// The byte stream was not valid JSON, or did not match the expected
    /// shape.
    Malformed(String),
    /// The format version tag was outside the range this build can
    /// migrate.
    UnsupportedVersion(u32),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodecError::Malformed(reason) => write!(f, "malformed preset data: {reason}"),
            CodecError::UnsupportedVersion(version) => {
                write!(f, "unsupported preset format version: {version}")
            }
        }
    }
}

impl Error for CodecError {}

/// Migrates a decoded `Preset` at any supported historical version up to
/// `CURRENT_PRESET_VERSION`. Centralizing migration here — rather than
/// duplicating it in every adapter — is what makes the "presets carry an
/// explicit version and are migrated on load" invariant hold regardless of
/// which physical format decoded the bytes.
fn migrate_preset(preset: Preset) -> Result<Preset, CodecError> {
    if preset.version > CURRENT_PRESET_VERSION || preset.version < MIN_SUPPORTED_PRESET_VERSION {
        return Err(CodecError::UnsupportedVersion(preset.version));
    }

    // Version 1 is the only version today; no structural migration is
    // needed yet. Future format changes add match arms here so every
    // adapter shares one migration policy instead of re-implementing it.
    Ok(Preset {
        version: CURRENT_PRESET_VERSION,
        ..preset
    })
}

/// The `PresetCodec` port: the single seam through which a `Preset` or a
/// whole `Setup` crosses the boundary between the in-memory model and a
/// byte stream.
trait PresetCodec {
    /// Decode a byte stream into a single, fully-migrated `Preset`.
    fn deserialize(&self, data: &[u8]) -> Result<Preset, CodecError>;

    /// Encode a single `Preset` into bytes at `CURRENT_PRESET_VERSION`.
    fn serialize(&self, preset: &Preset) -> Result<Vec<u8>, CodecError>;

    /// Decode a byte stream into a fully-migrated `Setup` (every contained
    /// `Preset` migrated individually).
    fn deserialize_setup(&self, data: &[u8]) -> Result<Setup, CodecError>;

    /// Encode a whole `Setup` into bytes at `CURRENT_PRESET_VERSION`.
    fn serialize_setup(&self, setup: &Setup) -> Result<Vec<u8>, CodecError>;
}

/// A `PresetCodec` adapter that reads and writes presets/setups as JSON via
/// serde. Holds no state of its own.
#[derive(Debug, Default, Clone, Copy)]
struct SerdePresetCodec;

impl SerdePresetCodec {
    fn new() -> Self {
        Self
    }
}

impl PresetCodec for SerdePresetCodec {
    fn deserialize(&self, data: &[u8]) -> Result<Preset, CodecError> {
        let preset: Preset =
            serde_json::from_slice(data).map_err(|err| CodecError::Malformed(err.to_string()))?;
        migrate_preset(preset)
    }

    fn serialize(&self, preset: &Preset) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(preset).map_err(|err| CodecError::Malformed(err.to_string()))
    }

    fn deserialize_setup(&self, data: &[u8]) -> Result<Setup, CodecError> {
        let raw: Setup =
            serde_json::from_slice(data).map_err(|err| CodecError::Malformed(err.to_string()))?;

        if raw.version > CURRENT_PRESET_VERSION || raw.version < MIN_SUPPORTED_PRESET_VERSION {
            return Err(CodecError::UnsupportedVersion(raw.version));
        }

        let migrated_presets = raw
            .presets
            .into_iter()
            .map(migrate_preset)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Setup {
            version: CURRENT_PRESET_VERSION,
            master_gain: raw.master_gain,
            presets: migrated_presets,
        })
    }

    fn serialize_setup(&self, setup: &Setup) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(setup).map_err(|err| CodecError::Malformed(err.to_string()))
    }
}

// ---------------------------------------------------------------------
// A tiny, self-contained real-time-safe ADSR envelope generator adapter.
// `engine::envelope_generator` defines only the port; no concrete adapter
// exists in the module tree yet, so one is defined locally here, driven by
// the real `engine::envelope_config::EnvelopeConfig` value object.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

struct DemoAdsrEnvelope {
    config: EnvelopeConfig,
    sample_period: f64,
    stage: EnvelopeStage,
    level: f64,
    release_start_level: f64,
}

impl DemoAdsrEnvelope {
    fn new(config: EnvelopeConfig, sample_rate: f64) -> Self {
        Self {
            config,
            sample_period: 1.0 / sample_rate,
            stage: EnvelopeStage::Idle,
            level: 0.0,
            release_start_level: 0.0,
        }
    }
}

impl EnvelopeGenerator for DemoAdsrEnvelope {
    fn trigger(&mut self) {
        self.level = 0.0;
        self.stage = if self.config.attack() > 0.0 {
            EnvelopeStage::Attack
        } else if self.config.decay() > 0.0 {
            self.level = 1.0;
            EnvelopeStage::Decay
        } else {
            self.level = self.config.sustain();
            EnvelopeStage::Sustain
        };
    }

    fn release(&mut self) {
        if self.stage == EnvelopeStage::Idle {
            return;
        }
        self.release_start_level = self.level;
        self.stage = if self.config.release() > 0.0 {
            EnvelopeStage::Release
        } else {
            self.level = 0.0;
            EnvelopeStage::Idle
        };
    }

    fn tick(&mut self) -> f64 {
        match self.stage {
            EnvelopeStage::Idle => {}
            EnvelopeStage::Attack => {
                self.level += self.sample_period / self.config.attack();
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = if self.config.decay() > 0.0 {
                        EnvelopeStage::Decay
                    } else {
                        self.level = self.config.sustain();
                        EnvelopeStage::Sustain
                    };
                }
            }
            EnvelopeStage::Decay => {
                let span = (1.0 - self.config.sustain()).max(0.0);
                self.level -= span * self.sample_period / self.config.decay();
                if self.level <= self.config.sustain() {
                    self.level = self.config.sustain();
                    self.stage = EnvelopeStage::Sustain;
                }
            }
            EnvelopeStage::Sustain => {
                self.level = self.config.sustain();
            }
            EnvelopeStage::Release => {
                self.level -= self.release_start_level * self.sample_period / self.config.release();
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                }
            }
        }
        self.level
    }
}

// ---------------------------------------------------------------------
// The fixed, built-in demo passage: a deterministic set of MIDI-style note
// events dispatched to whichever patches subscribe to their channel.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct DemoNoteEvent {
    channel: u8,
    note_number: u8,
    velocity: f64,
    start_sample: usize,
    hold_samples: usize,
}

/// Builds the fixed demo passage: a sustained triad on channel 0 (matches
/// the "Warm Pad" patch), a short melodic run on channel 1 (matches
/// "Bright Lead"), and punchy low notes on channel 2 (matches "Sub
/// Pluck"). Deterministic and independent of wall-clock time.
fn built_in_passage(sample_rate: f64) -> Vec<DemoNoteEvent> {
    let sec = |seconds: f64| (seconds * sample_rate).round() as usize;

    vec![
        DemoNoteEvent {
            channel: 0,
            note_number: 60,
            velocity: 0.8,
            start_sample: sec(0.0),
            hold_samples: sec(1.6),
        },
        DemoNoteEvent {
            channel: 0,
            note_number: 64,
            velocity: 0.7,
            start_sample: sec(0.0),
            hold_samples: sec(1.6),
        },
        DemoNoteEvent {
            channel: 0,
            note_number: 67,
            velocity: 0.7,
            start_sample: sec(0.0),
            hold_samples: sec(1.6),
        },
        DemoNoteEvent {
            channel: 1,
            note_number: 72,
            velocity: 0.9,
            start_sample: sec(0.2),
            hold_samples: sec(0.25),
        },
        DemoNoteEvent {
            channel: 1,
            note_number: 74,
            velocity: 0.9,
            start_sample: sec(0.5),
            hold_samples: sec(0.25),
        },
        DemoNoteEvent {
            channel: 1,
            note_number: 76,
            velocity: 0.9,
            start_sample: sec(0.8),
            hold_samples: sec(0.35),
        },
        DemoNoteEvent {
            channel: 2,
            note_number: 36,
            velocity: 1.0,
            start_sample: sec(0.0),
            hold_samples: sec(0.12),
        },
        DemoNoteEvent {
            channel: 2,
            note_number: 36,
            velocity: 1.0,
            start_sample: sec(0.6),
            hold_samples: sec(0.12),
        },
        DemoNoteEvent {
            channel: 2,
            note_number: 43,
            velocity: 0.9,
            start_sample: sec(1.2),
            hold_samples: sec(0.12),
        },
    ]
}

/// Standard equal-temperament MIDI-note-to-hertz conversion (A4 = note 69 =
/// 440 Hz).
fn midi_note_to_frequency(note_number: u8) -> f64 {
    440.0 * 2f64.powf((note_number as f64 - 69.0) / 12.0)
}

fn channel_mask(channels: &[u8]) -> u16 {
    channels
        .iter()
        .fold(0u16, |mask, &channel| mask | (1u16 << channel))
}

fn to_engine_waveform(waveform: DemoWaveform) -> Waveform {
    match waveform {
        DemoWaveform::Sine => Waveform::Sine,
        DemoWaveform::Saw => Waveform::Saw,
        DemoWaveform::Square => Waveform::Square,
        DemoWaveform::Triangle => Waveform::Triangle,
    }
}

fn to_engine_filter_kind(filter_type: DemoFilterType) -> FilterKind {
    match filter_type {
        DemoFilterType::LowPass => FilterKind::LowPass,
        DemoFilterType::HighPass => FilterKind::HighPass,
        DemoFilterType::BandPass => FilterKind::BandPass,
        DemoFilterType::Notch => FilterKind::Notch,
    }
}

// ---------------------------------------------------------------------
// Rendering: dispatcher -> per-patch voices -> per-patch mix -> master bus.
// Signal path per the project invariant: engine output -> channel strip
// volume and pan -> bus routing -> master bus inserts -> limiter -> output.
// This demo configures no sends and no extra inserts, so those stages are
// documented no-ops rather than omitted silently.
// ---------------------------------------------------------------------

/// Renders one patch's contribution (mono, pre gain/pan) across the whole
/// passage. A `MidiEvent` (here, a `DemoNoteEvent`) is rendered by this
/// patch only if its channel is present in the patch's channel mask —
/// dispatch to exactly the matching set, layering intentional.
fn render_patch(
    patch: &DemoPatchConfig,
    events: &[DemoNoteEvent],
    sample_rate: f64,
    total_samples: usize,
) -> Vec<f64> {
    let oscillator = StandardOscillator::new();
    let osc_config = OscillatorConfig::new(
        to_engine_waveform(patch.oscillator.waveform),
        Amplitude::try_new(patch.oscillator.amplitude)
            .expect("demo oscillator amplitude is always within [0.0, 1.0]"),
    );
    let filter_kind = to_engine_filter_kind(patch.filter.filter_type);
    let envelope_config = EnvelopeConfig::try_new(
        patch.amp_envelope.attack,
        patch.amp_envelope.decay,
        patch.amp_envelope.release,
        patch.amp_envelope.sustain,
    )
    .expect("demo envelope parameters are always valid");

    let mut buffer = vec![0.0_f64; total_samples];

    for event in events
        .iter()
        .filter(|event| patch.channels.mask & (1u16 << event.channel) != 0)
    {
        let frequency = Frequency::try_new(midi_note_to_frequency(event.note_number))
            .expect("demo note numbers always produce a positive frequency");
        let sample_rate_value =
            SampleRate::try_new(sample_rate).expect("demo sample rate is always positive");
        let filter_config = FilterConfig::new(
            filter_kind,
            patch.filter.cutoff_hz,
            patch.filter.resonance,
            sample_rate,
        );

        let mut filter = StateVariableFilter::new();
        let mut envelope = DemoAdsrEnvelope::new(envelope_config, sample_rate);
        envelope.trigger();

        let mut phase = 0.0_f64;
        let voice_length = total_samples.saturating_sub(event.start_sample);

        for offset in 0..voice_length {
            if offset == event.hold_samples {
                envelope.release();
            }
            let level = envelope.tick();
            let raw = oscillator.render(phase, osc_config) * level * event.velocity;
            let filtered = filter.process(raw, filter_config);
            buffer[event.start_sample + offset] += filtered;
            phase = oscillator.advance(phase, frequency, sample_rate_value);
        }
    }

    buffer
}

/// Applies channel-strip volume then pan (constant-power law), splitting a
/// mono signal into a stereo pair. Runs after `render_patch` and before bus
/// summing, matching the canonical signal path's "volume and pan" stage.
fn apply_gain_and_pan(mono: &[f64], gain: f64, pan: f64) -> Vec<(f64, f64)> {
    let clamped_pan = pan.clamp(-1.0, 1.0);
    let angle = (clamped_pan + 1.0) * FRAC_PI_4;
    let left_gain = angle.cos();
    let right_gain = angle.sin();

    mono.iter()
        .map(|&sample| {
            let volume_applied = sample * gain;
            (volume_applied * left_gain, volume_applied * right_gain)
        })
        .collect()
}

/// Renders the full `Setup` for the given passage into 16-bit PCM mono
/// samples, following dispatcher -> per-patch voices -> per-patch volume
/// and pan -> master bus -> master gain -> limiter -> mono downmix.
/// Deterministic: identical inputs always produce identical output.
fn render_setup(
    setup: &Setup,
    events: &[DemoNoteEvent],
    sample_rate: f64,
    total_samples: usize,
) -> Vec<i16> {
    let mut master_left = vec![0.0_f64; total_samples];
    let mut master_right = vec![0.0_f64; total_samples];

    for preset in &setup.presets {
        let mono = render_patch(&preset.patch, events, sample_rate, total_samples);
        for (index, (left, right)) in apply_gain_and_pan(&mono, preset.patch.gain, preset.patch.pan)
            .into_iter()
            .enumerate()
        {
            master_left[index] += left;
            master_right[index] += right;
        }
    }

    // Master bus inserts: none configured for this demo. Limiter: a
    // deterministic soft-clip so full-scale is never exceeded before
    // quantization, matching "master bus inserts -> limiter -> output".
    let mut mono_out = Vec::with_capacity(total_samples);
    for index in 0..total_samples {
        let left = (master_left[index] * setup.master_gain).tanh();
        let right = (master_right[index] * setup.master_gain).tanh();
        mono_out.push((left + right) * 0.5);
    }

    quantize_to_i16(&mono_out)
}

fn quantize_to_i16(samples: &[f64]) -> Vec<i16> {
    samples
        .iter()
        .map(|&sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f64).round() as i16)
        .collect()
}

// ---------------------------------------------------------------------
// WAV output: a pure-Rust, dependency-free 16-bit mono PCM writer.
// ---------------------------------------------------------------------

fn write_wav(path: &Path, samples: &[i16], sample_rate: u32) -> std::io::Result<()> {
    const CHANNELS: u16 = 1;
    const BITS_PER_SAMPLE: u16 = 16;

    let byte_rate = sample_rate * CHANNELS as u32 * (BITS_PER_SAMPLE as u32 / 8);
    let block_align = CHANNELS * (BITS_PER_SAMPLE / 8);
    let data_size = (samples.len() * 2) as u32;
    let riff_size = 36 + data_size;

    let mut writer = BufWriter::new(File::create(path)?);

    writer.write_all(b"RIFF")?;
    writer.write_all(&riff_size.to_le_bytes())?;
    writer.write_all(b"WAVE")?;

    writer.write_all(b"fmt ")?;
    writer.write_all(&16u32.to_le_bytes())?; // fmt chunk size
    writer.write_all(&1u16.to_le_bytes())?; // PCM
    writer.write_all(&CHANNELS.to_le_bytes())?;
    writer.write_all(&sample_rate.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&BITS_PER_SAMPLE.to_le_bytes())?;

    writer.write_all(b"data")?;
    writer.write_all(&data_size.to_le_bytes())?;
    for &sample in samples {
        writer.write_all(&sample.to_le_bytes())?;
    }

    writer.flush()
}

// ---------------------------------------------------------------------
// Demo setup construction and CLI entry point.
// ---------------------------------------------------------------------

fn build_demo_setup() -> Setup {
    let warm_pad = Preset {
        version: CURRENT_PRESET_VERSION,
        name: "Warm Pad".to_string(),
        patch: DemoPatchConfig {
            name: "Warm Pad".to_string(),
            oscillator: DemoOscillatorConfig {
                waveform: DemoWaveform::Sine,
                amplitude: 0.6,
            },
            filter: DemoFilterConfig {
                filter_type: DemoFilterType::LowPass,
                cutoff_hz: 1200.0,
                resonance: 0.2,
            },
            amp_envelope: DemoAmpEnvelopeConfig {
                attack: 0.4,
                decay: 0.3,
                release: 0.6,
                sustain: 0.7,
            },
            gain: 0.8,
            pan: -0.3,
            channels: DemoChannelSubscription {
                mask: channel_mask(&[0]),
            },
        },
    };

    let bright_lead = Preset {
        version: CURRENT_PRESET_VERSION,
        name: "Bright Lead".to_string(),
        patch: DemoPatchConfig {
            name: "Bright Lead".to_string(),
            oscillator: DemoOscillatorConfig {
                waveform: DemoWaveform::Saw,
                amplitude: 0.5,
            },
            filter: DemoFilterConfig {
                filter_type: DemoFilterType::BandPass,
                cutoff_hz: 2500.0,
                resonance: 0.4,
            },
            amp_envelope: DemoAmpEnvelopeConfig {
                attack: 0.01,
                decay: 0.1,
                release: 0.15,
                sustain: 0.6,
            },
            gain: 0.7,
            pan: 0.4,
            channels: DemoChannelSubscription {
                mask: channel_mask(&[1]),
            },
        },
    };

    let sub_pluck = Preset {
        version: CURRENT_PRESET_VERSION,
        name: "Sub Pluck".to_string(),
        patch: DemoPatchConfig {
            name: "Sub Pluck".to_string(),
            oscillator: DemoOscillatorConfig {
                waveform: DemoWaveform::Square,
                amplitude: 0.9,
            },
            filter: DemoFilterConfig {
                filter_type: DemoFilterType::LowPass,
                cutoff_hz: 400.0,
                resonance: 0.1,
            },
            amp_envelope: DemoAmpEnvelopeConfig {
                attack: 0.002,
                decay: 0.08,
                release: 0.05,
                sustain: 0.0,
            },
            gain: 0.9,
            pan: 0.0,
            channels: DemoChannelSubscription {
                mask: channel_mask(&[2]),
            },
        },
    };

    Setup {
        version: CURRENT_PRESET_VERSION,
        master_gain: 0.85,
        presets: vec![warm_pad, bright_lead, sub_pluck],
    }
}

fn parse_output_path(args: &[String]) -> PathBuf {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--out" {
            if let Some(path) = iter.next() {
                return PathBuf::from(path);
            }
        }
    }
    PathBuf::from(DEFAULT_OUTPUT_PATH)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let out_path = parse_output_path(&args);

    let setup = build_demo_setup();
    let codec = SerdePresetCodec::new();

    // Exercise the single-`Preset` half of the codec port before the
    // whole-`Setup` round trip: every preset in `setup` must survive being
    // serialized and deserialized on its own, independent of the setup it
    // lives in.
    for preset in &setup.presets {
        let preset_bytes = codec
            .serialize(preset)
            .expect("serializing a single preset should never fail");
        let reloaded_preset = codec
            .deserialize(&preset_bytes)
            .expect("deserializing a single preset should never fail");
        if &reloaded_preset != preset {
            panic!(
                "preset roundtrip mismatch for {:?}: reloaded preset does not equal the original",
                preset.name
            );
        }
    }

    let setup_bytes = codec
        .serialize_setup(&setup)
        .expect("serializing the demo setup should never fail");
    let reloaded_setup = codec
        .deserialize_setup(&setup_bytes)
        .expect("deserializing the demo setup should never fail");

    if setup != reloaded_setup {
        panic!(
            "setup roundtrip mismatch: the setup reloaded from its serialized bytes does not \
             equal the original\noriginal: {setup:?}\nreloaded: {reloaded_setup:?}"
        );
    }
    println!("setup roundtrip: equal");

    let passage = built_in_passage(SAMPLE_RATE_HZ);
    let total_samples = (DEMO_TOTAL_SECONDS * SAMPLE_RATE_HZ).round() as usize;

    let rendered_original = render_setup(&setup, &passage, SAMPLE_RATE_HZ, total_samples);
    let rendered_reloaded = render_setup(&reloaded_setup, &passage, SAMPLE_RATE_HZ, total_samples);

    if rendered_original != rendered_reloaded {
        let first_mismatch = rendered_original
            .iter()
            .zip(rendered_reloaded.iter())
            .position(|(a, b)| a != b);
        panic!(
            "render mismatch: rendering the original setup and the reloaded setup produced \
             different audio (first differing sample index: {first_mismatch:?})"
        );
    }
    println!("render identical: true");

    write_wav(&out_path, &rendered_original, SAMPLE_RATE_HZ as u32)
        .expect("writing the WAV file should never fail");

    let peak = rendered_original
        .iter()
        .fold(0i16, |max, &sample| max.max(sample.abs()));

    println!("patches: {}", setup.presets.len());
    println!("master_gain: {:.3}", setup.master_gain);
    println!("samples: {}", rendered_original.len());
    println!("peak_sample: {peak}");
    println!("output: {}", out_path.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_patch() -> DemoPatchConfig {
        DemoPatchConfig {
            name: "Test Patch".to_string(),
            oscillator: DemoOscillatorConfig {
                waveform: DemoWaveform::Sine,
                amplitude: 0.5,
            },
            filter: DemoFilterConfig {
                filter_type: DemoFilterType::LowPass,
                cutoff_hz: 1000.0,
                resonance: 0.2,
            },
            amp_envelope: DemoAmpEnvelopeConfig {
                attack: 0.01,
                decay: 0.05,
                release: 0.1,
                sustain: 0.5,
            },
            gain: 0.8,
            pan: 0.0,
            channels: DemoChannelSubscription {
                mask: channel_mask(&[0]),
            },
        }
    }

    fn sample_preset() -> Preset {
        Preset {
            version: CURRENT_PRESET_VERSION,
            name: "Test Patch".to_string(),
            patch: sample_patch(),
        }
    }

    #[test]
    fn preset_round_trips_through_codec() {
        let codec = SerdePresetCodec::new();
        let preset = sample_preset();

        let bytes = codec.serialize(&preset).expect("serialize succeeds");
        let decoded = codec.deserialize(&bytes).expect("deserialize succeeds");

        assert_eq!(decoded, preset);
    }

    #[test]
    fn setup_round_trips_through_codec() {
        let codec = SerdePresetCodec::new();
        let setup = build_demo_setup();

        let bytes = codec.serialize_setup(&setup).expect("serialize succeeds");
        let decoded = codec
            .deserialize_setup(&bytes)
            .expect("deserialize succeeds");

        assert_eq!(decoded, setup);
    }

    #[test]
    fn migrate_preset_rejects_a_version_newer_than_current() {
        let mut preset = sample_preset();
        preset.version = CURRENT_PRESET_VERSION + 1;

        let result = migrate_preset(preset);

        assert!(matches!(result, Err(CodecError::UnsupportedVersion(_))));
    }

    #[test]
    fn migrate_preset_rejects_a_version_older_than_supported() {
        let mut preset = sample_preset();
        preset.version = MIN_SUPPORTED_PRESET_VERSION.saturating_sub(1);
        // MIN_SUPPORTED_PRESET_VERSION is 1, so this underflows to 0 via
        // saturating_sub, which is still "older than supported" (0 < 1).
        let result = migrate_preset(preset);

        assert!(matches!(result, Err(CodecError::UnsupportedVersion(_))));
    }

    #[test]
    fn channel_mask_matches_only_subscribed_channels() {
        let patch = sample_patch();
        assert_eq!(patch.channels.mask & (1u16 << 0), 1u16 << 0);
        assert_eq!(patch.channels.mask & (1u16 << 1), 0);
    }

    #[test]
    fn envelope_reaches_silence_after_release_completes() {
        let config = EnvelopeConfig::try_new(0.0, 0.0, 0.01, 1.0).expect("valid envelope");
        let mut envelope = DemoAdsrEnvelope::new(config, 1000.0);
        envelope.trigger();
        assert_eq!(envelope.tick(), 1.0);
        envelope.release();
        for _ in 0..20 {
            envelope.tick();
        }
        assert_eq!(envelope.tick(), 0.0);
    }

    #[test]
    fn rendering_the_same_setup_twice_is_deterministic() {
        let setup = build_demo_setup();
        let passage = built_in_passage(1000.0);
        let total_samples = 2000;

        let first = render_setup(&setup, &passage, 1000.0, total_samples);
        let second = render_setup(&setup, &passage, 1000.0, total_samples);

        assert_eq!(first, second);
    }

    #[test]
    fn quantize_clamps_out_of_range_samples() {
        let quantized = quantize_to_i16(&[2.0, -2.0, 0.0]);
        assert_eq!(quantized[0], i16::MAX);
        assert_eq!(quantized[1], -i16::MAX);
        assert_eq!(quantized[2], 0);
    }

    #[test]
    fn parse_output_path_defaults_when_flag_absent() {
        let args: Vec<String> = vec![];
        assert_eq!(parse_output_path(&args), PathBuf::from(DEFAULT_OUTPUT_PATH));
    }

    #[test]
    fn parse_output_path_honors_out_flag() {
        let args: Vec<String> = vec!["--out".to_string(), "custom.wav".to_string()];
        assert_eq!(parse_output_path(&args), PathBuf::from("custom.wav"));
    }
}
