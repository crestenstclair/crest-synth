// path: src/bin/voice_demo.rs

//! Renders an over-polyphonic passage through the engine's `VoiceAllocator`
//! and `EngineRenderer`, deliberately exceeding a small polyphony limit so
//! voice stealing is forced, and writes the result to a mono 16-bit WAV
//! file.
//!
//! CLI: `voice_demo [--out OUT.wav]`. Writes `voice-demo.wav` by default.
//!
//! The passage is a rolling cluster of ten overlapping sustained notes fed
//! into a `VoiceAllocator` configured with only four voices, so every note
//! past the fourth forces the allocator to steal a still-sounding voice.
//! The measured steal count is asserted to be strictly greater than zero:
//! a `steals=0` run is a stealing-logic regression and must fail the
//! process, not merely report a zero count.
//!
//! Every note is driven through the real signal path used elsewhere in
//! this crate (see `src/bin/synth_ui.rs`): `VoiceAllocator` owns note
//! assignment and stealing, `VoiceRenderer` composes the `Oscillator`,
//! `Filter`, and `EnvelopeGenerator` ports to render one voice per buffer
//! along the oscillator -> filter -> envelope path, and `EngineRenderer`
//! sums every active voice into the mix in fixed sample blocks.
//!
//! No concrete `EnvelopeGenerator` adapter exists yet in this crate's
//! module tree (only the port and a private test double), so this binary
//! defines a small ADSR generator locally, mirroring the same convention
//! `src/bin/synth_ui.rs` already established for this exact gap.

use std::env;
use std::path::{Path, PathBuf};

use crest_synth::engine::engine_renderer::{EngineRenderer, VoiceRenderState};
use crest_synth::engine::envelope_generator::EnvelopeGenerator;
use crest_synth::engine::filter::{FilterConfig, FilterKind, StateVariableFilter};
use crest_synth::engine::oscillator::{
    Amplitude, OscillatorConfig, SampleRate, StandardOscillator, Waveform,
};
use crest_synth::engine::voice::{
    AmpEnvelopeStage, EnvelopeTiming, NoteId, NoteNumber, Velocity, VoiceConfig, VoiceEvent,
};
use crest_synth::engine::voice_allocator::{StealPolicy, VoiceAllocator, VoiceAssignment};
use crest_synth::engine::voice_renderer::VoiceRenderer;
use crest_synth::kernel::audio_frame::AudioFrame;

const DEFAULT_OUTPUT_PATH: &str = "voice-demo.wav";
const SAMPLE_RATE_HZ: f64 = 44_100.0;
const BLOCK_LEN: usize = 256;
/// Deliberately small polyphony limit: the passage below holds far more
/// simultaneous notes than this, forcing the allocator to steal voices.
const MAX_VOICES: usize = 4;
const NOTE_SPACING_SECONDS: f64 = 0.10;
const NOTE_HOLD_SECONDS: f64 = 1.0;
const RELEASE_TAIL_SECONDS: f64 = 0.3;
/// MIDI note numbers for the rolling cluster (ten notes -- more than
/// double `MAX_VOICES`), a repeating C major triad plus an octave note so
/// the passage sounds like a plausible held chord rather than noise.
const NOTE_NUMBERS: [u8; 10] = [60, 64, 67, 60, 64, 67, 72, 67, 64, 60];

/// One scheduled note in the demo passage: on at `time_on`, explicitly
/// released at `time_off` (unless its voice was stolen first).
#[derive(Debug, Clone, Copy)]
struct NoteEvent {
    note_id: NoteId,
    note: NoteNumber,
    velocity: Velocity,
    time_on: f64,
    time_off: f64,
}

/// Builds the demo passage: a rolling cluster of overlapping sustained
/// notes, each starting `NOTE_SPACING_SECONDS` after the last and held for
/// `NOTE_HOLD_SECONDS`. With `NOTE_SPACING_SECONDS` well under
/// `NOTE_HOLD_SECONDS`, several notes are always overlapping at once --
/// comfortably more than `MAX_VOICES` -- which is what forces the
/// allocator in `run_passage` to steal voices.
fn build_passage() -> Vec<NoteEvent> {
    NOTE_NUMBERS
        .iter()
        .enumerate()
        .map(|(index, &note_number)| {
            let time_on = index as f64 * NOTE_SPACING_SECONDS;
            NoteEvent {
                note_id: NoteId::new(index as u64 + 1),
                note: NoteNumber::try_new(note_number).expect("note numbers are valid MIDI notes"),
                velocity: Velocity::try_new(0.85).expect("0.85 is a valid velocity"),
                time_on,
                time_off: time_on + NOTE_HOLD_SECONDS,
            }
        })
        .collect()
}

/// How many notes in `passage` are conceptually held (`time_on..time_off`)
/// at `time_seconds`. Used only to document/verify that the passage
/// actually asks for more simultaneous notes than `MAX_VOICES` -- the
/// stealing itself is exercised end-to-end by `run_passage`, not inferred
/// from this count.
fn simultaneous_notes_at(passage: &[NoteEvent], time_seconds: f64) -> usize {
    passage
        .iter()
        .filter(|event| event.time_on <= time_seconds && time_seconds < event.time_off)
        .count()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdsrStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// A minimal, real-time-safe ADSR envelope generator. No concrete
/// `EnvelopeGenerator` adapter exists yet in this crate's module tree, so
/// one is defined locally here, mirroring the same local adapter
/// `src/bin/synth_ui.rs` defines for this same gap. Never allocates,
/// locks, or blocks in `trigger`/`release`/`tick`.
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

/// Outcome of driving the demo passage through the real engine: the
/// rendered mono samples and the measured voice-steal count.
struct SimulationOutcome {
    samples: Vec<f32>,
    steal_count: usize,
}

/// Replaces the voice-render state at `index` with a freshly triggered
/// one. `VoiceRenderState` only exposes its filter/envelope by shared
/// reference (see `engine::engine_renderer`), so triggering an *existing*
/// instance from outside that module is not possible; swapping in a new,
/// already-triggered instance -- matching `src/bin/synth_ui.rs` -- is the
/// available alternative.
fn retrigger_voice_state(
    voice_states: &mut [VoiceRenderState<StateVariableFilter, SimpleAdsrEnvelope>],
    index: usize,
    timing: EnvelopeTiming,
) {
    if let Some(state) = voice_states.get_mut(index) {
        let mut envelope = SimpleAdsrEnvelope::new(timing, SAMPLE_RATE_HZ);
        envelope.trigger();
        *state = VoiceRenderState::new(StateVariableFilter::new(), envelope);
    }
}

/// Drives `passage` through a `VoiceAllocator` limited to `polyphony`
/// voices, rendering audio in fixed `BLOCK_LEN` blocks via `EngineRenderer`
/// and `VoiceRenderer`, exactly as the live `synth_ui` render path does.
///
/// Every voice steal (`VoiceAssignment::Stolen`) is counted. Every amp
/// envelope stage transition observed on any managed voice is printed, so
/// the envelope progression (Attack/Decay/Sustain/Release) is directly
/// observable in the demo's output, not just asserted on internally.
fn run_passage(passage: &[NoteEvent], polyphony: usize) -> SimulationOutcome {
    let timing = EnvelopeTiming::new(0.05, 0.05, 0.6, 0.15);
    let voice_config = VoiceConfig::new(timing);
    let mut allocator = VoiceAllocator::new(voice_config, polyphony, StealPolicy::Oldest)
        .expect("polyphony is nonzero");
    let mut voice_states: Vec<VoiceRenderState<StateVariableFilter, SimpleAdsrEnvelope>> = (0
        ..polyphony)
        .map(|_| {
            VoiceRenderState::new(
                StateVariableFilter::new(),
                SimpleAdsrEnvelope::new(timing, SAMPLE_RATE_HZ),
            )
        })
        .collect();

    let voice_renderer = VoiceRenderer::new();
    let engine_renderer = EngineRenderer::new();
    let oscillator = StandardOscillator::new();
    let osc_config = OscillatorConfig::new(
        Waveform::Saw,
        Amplitude::try_new(0.8).expect("0.8 is a valid amplitude"),
    );
    let filter_config = FilterConfig::new(FilterKind::LowPass, 6_000.0, 0.15, SAMPLE_RATE_HZ);
    let sample_rate = SampleRate::try_new(SAMPLE_RATE_HZ).expect("44.1kHz is a valid sample rate");

    let total_duration = passage
        .iter()
        .map(|event| event.time_off)
        .fold(0.0_f64, f64::max)
        + timing.release_seconds
        + RELEASE_TAIL_SECONDS;

    let mut fired_on = vec![false; passage.len()];
    let mut fired_off = vec![false; passage.len()];
    let mut prev_stage = vec![AmpEnvelopeStage::Idle; polyphony];
    let mut steal_count = 0usize;
    let mut samples = Vec::new();
    let mut scratch = vec![0.0_f64; BLOCK_LEN];
    let mut engine_out = vec![AudioFrame::silence(); BLOCK_LEN];
    let mut cluster_summary_printed = false;

    let dt_per_block = BLOCK_LEN as f64 / SAMPLE_RATE_HZ;
    let mut block_index: u64 = 0;

    println!(
        "voice_demo: passage of {} notes over {} voices (peak overlap {} notes)",
        passage.len(),
        polyphony,
        passage
            .iter()
            .map(|event| simultaneous_notes_at(passage, event.time_on))
            .max()
            .unwrap_or(0)
    );

    loop {
        let block_start_time = block_index as f64 * dt_per_block;
        if block_start_time >= total_duration {
            break;
        }

        for (index, event) in passage.iter().enumerate() {
            if !fired_on[index] && event.time_on <= block_start_time {
                fired_on[index] = true;
                match allocator.allocate(event.note, event.note_id, event.velocity) {
                    Ok(VoiceAssignment::Assigned { index: slot }) => {
                        retrigger_voice_state(&mut voice_states, slot, timing);
                        println!(
                            "t={:6.3}s note-on  note_id={:?} note={:?} -> assigned slot {slot}",
                            block_start_time, event.note_id, event.note
                        );
                    }
                    Ok(VoiceAssignment::Stolen {
                        index: slot,
                        stolen_note_id,
                    }) => {
                        steal_count += 1;
                        println!(
                            "t={:6.3}s note-on  note_id={:?} note={:?} -> STOLE slot {slot} from note_id={:?} (steal #{steal_count})",
                            block_start_time, event.note_id, event.note, stolen_note_id
                        );
                    }
                    Err(err) => {
                        println!("t={block_start_time:6.3}s note-on rejected: {err:?}");
                    }
                }
            }
            if !fired_off[index] && event.time_off <= block_start_time {
                fired_off[index] = true;
                if allocator.release(event.note_id).is_ok() {
                    println!(
                        "t={:6.3}s note-off note_id={:?} released",
                        block_start_time, event.note_id
                    );
                }
            }
        }

        if !cluster_summary_printed && fired_on.iter().all(|&fired| fired) {
            cluster_summary_printed = true;
            println!(
                "-- section summary: cluster build-up complete, steals so far={steal_count} --"
            );
        }

        let mut pending_triggers: Vec<usize> = Vec::new();
        allocator.advance_all(dt_per_block, |index, event| {
            if let VoiceEvent::Triggered { .. } = event {
                pending_triggers.push(index);
            }
        });
        for index in pending_triggers {
            retrigger_voice_state(&mut voice_states, index, timing);
        }

        for (index, prev) in prev_stage.iter_mut().enumerate() {
            let stage = allocator
                .voice(index)
                .map(|voice| voice.amp_stage())
                .unwrap_or(AmpEnvelopeStage::Idle);
            if stage != *prev {
                println!("t={block_start_time:6.3}s slot {index} envelope stage -> {stage:?}");
                *prev = stage;
            }
        }

        engine_renderer
            .render(
                &allocator,
                &voice_renderer,
                &oscillator,
                osc_config,
                filter_config,
                sample_rate,
                &mut voice_states,
                &mut scratch,
                &mut engine_out,
            )
            .expect("state/scratch/output are sized to match the allocator's polyphony");

        for frame in &engine_out {
            samples.push(frame.left());
        }

        block_index += 1;
    }

    println!(
        "-- section summary: passage complete, {} samples rendered, total steals={steal_count} --",
        samples.len()
    );

    SimulationOutcome {
        samples,
        steal_count,
    }
}

/// Converts a mono `f32` buffer in the (approximate) range `-1.0..=1.0`
/// into 16-bit PCM samples, clamping any excursion outside that range
/// rather than wrapping -- this is an offline demo render, not the
/// real-time path, but clipping should still be a hard ceiling rather than
/// integer overflow.
fn to_pcm16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&sample| (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16)
        .collect()
}

/// Encodes `samples` as a mono 16-bit PCM WAV file's raw bytes, written by
/// hand (no external WAV crate) per the standard RIFF/WAVE layout.
fn encode_wav_mono_16(samples: &[i16], sample_rate_hz: u32) -> Vec<u8> {
    const NUM_CHANNELS: u16 = 1;
    const BITS_PER_SAMPLE: u16 = 16;
    let byte_rate = sample_rate_hz * u32::from(NUM_CHANNELS) * u32::from(BITS_PER_SAMPLE) / 8;
    let block_align = NUM_CHANNELS * BITS_PER_SAMPLE / 8;
    let data_len = (samples.len() * 2) as u32;
    let riff_len = 36 + data_len;

    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_len.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&NUM_CHANNELS.to_le_bytes());
    bytes.extend_from_slice(&sample_rate_hz.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

/// Parses `--out OUT.wav` from `args`, defaulting to `DEFAULT_OUTPUT_PATH`.
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

fn write_wav(path: &Path, samples: &[i16], sample_rate_hz: u32) {
    let bytes = encode_wav_mono_16(samples, sample_rate_hz);
    std::fs::write(path, bytes).unwrap_or_else(|err| {
        panic!(
            "voice_demo FAILED: could not write WAV file {}: {err}",
            path.display()
        )
    });
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let output_path = parse_output_path(&args);

    let passage = build_passage();
    let outcome = run_passage(&passage, MAX_VOICES);

    if outcome.steal_count == 0 {
        panic!("voice_demo FAILED: passage forced no voice steals");
    }

    let pcm = to_pcm16(&outcome.samples);
    write_wav(&output_path, &pcm, SAMPLE_RATE_HZ as u32);

    println!("wrote {} samples to {}", pcm.len(), output_path.display());
    println!("steals={}", outcome.steal_count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passage_holds_between_eight_and_twelve_notes() {
        let passage = build_passage();
        assert!(passage.len() >= 8 && passage.len() <= 12);
    }

    #[test]
    fn passage_exceeds_max_voices_at_peak_overlap() {
        let passage = build_passage();
        let peak = passage
            .iter()
            .map(|event| simultaneous_notes_at(&passage, event.time_on))
            .max()
            .unwrap_or(0);
        assert!(peak > MAX_VOICES);
    }

    #[test]
    fn running_the_passage_forces_at_least_one_steal() {
        let passage = build_passage();
        let outcome = run_passage(&passage, MAX_VOICES);
        assert!(outcome.steal_count > 0);
    }

    #[test]
    fn running_the_passage_renders_nonempty_nonsilent_audio() {
        let passage = build_passage();
        let outcome = run_passage(&passage, MAX_VOICES);
        assert!(!outcome.samples.is_empty());
        let peak = outcome
            .samples
            .iter()
            .fold(0.0_f32, |max, &sample| max.max(sample.abs()));
        assert!(peak > 0.0);
    }

    #[test]
    fn to_pcm16_clamps_out_of_range_values() {
        let pcm = to_pcm16(&[2.0, -2.0, 0.0]);
        assert_eq!(pcm[0], i16::MAX);
        assert_eq!(pcm[1], -i16::MAX);
        assert_eq!(pcm[2], 0);
    }

    #[test]
    fn encode_wav_mono_16_has_a_valid_riff_header() {
        let bytes = encode_wav_mono_16(&[0, 100, -100], 44_100);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(bytes.len(), 44 + 3 * 2);
    }

    #[test]
    fn parse_output_path_defaults_when_flag_absent() {
        let args: Vec<String> = vec![];
        assert_eq!(parse_output_path(&args), PathBuf::from(DEFAULT_OUTPUT_PATH));
    }

    #[test]
    fn parse_output_path_reads_out_flag() {
        let args: Vec<String> = vec!["--out".to_string(), "custom.wav".to_string()];
        assert_eq!(parse_output_path(&args), PathBuf::from("custom.wav"));
    }
}
