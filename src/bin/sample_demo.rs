// path: src/bin/sample_demo.rs
//
// sample_demo — hermetic SampleLibrary prover.
//
// Synthesizes a tiny mono WAV sample in memory, writes it to a temp file,
// loads it back through the SampleLoader port, builds a two-zone SampleSet
// aggregate sharing the loaded SampleData via Arc, plays a short built-in
// passage of notes chosen to land in different zones, pitch-shifts each hit
// through linear interpolation, mixes the results in fixed-size blocks, and
// writes a 16-bit mono WAV. Asserts in code that zone routing and
// interpolation actually did something rather than trusting printed output.

use std::collections::{HashMap, HashSet};
use std::env;
use std::f32::consts::PI as PI32;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crest_synth::sample::sample_loader::{SampleData, SampleLoader};
use crest_synth::sample::sample_player::{LoopMode, PlaybackRequest, SamplePlayer};
use crest_synth::sample::sample_set::{
    InterpolationMode, KeyRange, SampleSet, VelocityRange, Zone,
};
use crest_synth::sample::symphonia_sample_loader::SymphoniaSampleLoader;

/// Fixed block size used when mixing rendered note passages into the output
/// buffer. Processing in fixed blocks (rather than one giant copy) mirrors
/// how a real-time mixer would consume rendered audio in bounded chunks.
const MIX_BLOCK_SIZE: usize = 256;

/// Sample rate for the synthesized source sample and rendered output.
const SAMPLE_RATE_HZ: u32 = 44_100;

/// MIDI note the synthesized sample is recorded at. Must match the root_key
/// `SymphoniaSampleLoader::load_wav` assigns to every loaded WAV (currently
/// hardcoded to 60 there, since plain WAV carries no root-key metadata).
const ROOT_NOTE: u8 = 60;

/// Deletes the wrapped path when dropped, so the hermetic temp sample file is
/// cleaned up on every exit path, including an early panic from a failed
/// in-code assertion.
struct TempFileGuard(PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn main() {
    let out_path = parse_out_path();

    let temp_path = synthesize_and_write_temp_wav(ROOT_NOTE, SAMPLE_RATE_HZ);
    let _temp_guard = TempFileGuard(temp_path.clone());

    let loader = SymphoniaSampleLoader::new();
    let loaded = loader
        .load_wav(&temp_path)
        .expect("hermetic temp WAV must load through SampleLoader");
    let sample_data: Arc<SampleData> = Arc::new(
        loaded
            .samples
            .into_iter()
            .next()
            .expect("synthesized WAV must decode to exactly one SampleData"),
    );

    // Two non-overlapping zones sharing the same synthesized SampleData via
    // Arc: a low-key zone (notes 0..=59) and a high-key zone (notes 60..=127).
    let mut sample_set = SampleSet::new(InterpolationMode::Linear);
    sample_set.apply_add_zone(Zone::new(
        KeyRange::try_new(0, 59).expect("valid low-key range"),
        VelocityRange::try_new(0, 127).expect("valid velocity range"),
        "low-key".to_string(),
    ));
    sample_set.apply_add_zone(Zone::new(
        KeyRange::try_new(60, 127).expect("valid high-key range"),
        VelocityRange::try_new(0, 127).expect("valid velocity range"),
        "high-key".to_string(),
    ));

    let zone_count = sample_set.zones().len();
    println!("zones loaded={zone_count}");
    assert!(
        zone_count >= 2,
        "expected at least 2 zones, found {zone_count}"
    );

    let mut sample_by_ref: HashMap<String, Arc<SampleData>> = HashMap::new();
    for zone in sample_set.zones() {
        sample_by_ref.insert(zone.sample_ref().to_string(), Arc::clone(&sample_data));
    }

    // Built-in passage: notes chosen to land in different zones, including at
    // least one note away from the sample's root key so pitch-shift
    // interpolation is actually exercised.
    let passage: [(u8, u8); 3] = [(48, 80), (72, 100), (36, 40)];

    let player = SamplePlayer::new();
    let mut distinct_zones: HashSet<String> = HashSet::new();
    let mut rendered_passages: Vec<Vec<f32>> = Vec::new();
    let mut interpolation_probe: Option<(Vec<f32>, Vec<f32>)> = None;

    for &(note, velocity) in &passage {
        let request = PlaybackRequest::new(note, velocity, LoopMode::OneShot);
        let instructions = player.play(&sample_set, request);
        assert!(
            !instructions.is_empty(),
            "note={note} vel={velocity} matched no zone"
        );

        // Layered zones can produce multiple instructions; the passage is
        // built so each note's *first* match is the zone under test.
        let instruction = &instructions[0];
        println!(
            "zone hit: {} (note={} vel={})",
            instruction.sample_ref(),
            note,
            velocity
        );
        distinct_zones.insert(instruction.sample_ref().to_string());

        let data = sample_by_ref
            .get(instruction.sample_ref())
            .expect("every zone's sample_ref must resolve to loaded SampleData");

        let rendered = render_pitch_shifted(data, note);
        if note != ROOT_NOTE && interpolation_probe.is_none() {
            let root_pitch_reference = read_same_length_at_root(data, rendered.len());
            interpolation_probe = Some((rendered.clone(), root_pitch_reference));
        }
        rendered_passages.push(rendered);
    }

    let distinct_zone_count = distinct_zones.len();
    println!("distinct zones hit={distinct_zone_count}");
    assert!(
        distinct_zone_count >= 2,
        "expected at least 2 distinct zones hit, found {distinct_zone_count}"
    );

    let (interpolated, root_reference) =
        interpolation_probe.expect("passage must include a note away from the sample's root key");
    assert_ne!(
        interpolated, root_reference,
        "pitch-shifted interpolated render must differ from a same-length root-pitch read"
    );

    let mixed = mix_in_blocks(&rendered_passages);
    write_wav_mono_16(&out_path, SAMPLE_RATE_HZ, &mixed);
    println!("wrote {} ({} frames)", out_path.display(), mixed.len());
}

/// Parses `--out OUT.wav` from argv, defaulting to `sample-demo.wav`.
fn parse_out_path() -> PathBuf {
    let mut args = env::args().skip(1);
    let mut out = PathBuf::from("sample-demo.wav");
    while let Some(arg) = args.next() {
        if arg == "--out" {
            if let Some(value) = args.next() {
                out = PathBuf::from(value);
            }
        }
    }
    out
}

/// Converts a MIDI note number to frequency in Hz using equal temperament
/// with A4 (note 69) at 440 Hz.
fn note_to_frequency(note: u8) -> f64 {
    440.0 * 2f64.powf((note as f64 - 69.0) / 12.0)
}

/// Synthesizes a short decaying sine wave at `root_note`'s frequency, writes
/// it as a 16-bit mono WAV to a uniquely-named file under the system temp
/// directory, and returns that path. This is the only "sample" this binary
/// ever touches — nothing ships in the repo.
fn synthesize_and_write_temp_wav(root_note: u8, sample_rate_hz: u32) -> PathBuf {
    let duration_secs = 0.3_f32;
    let frame_count = (duration_secs * sample_rate_hz as f32) as usize;
    let freq = note_to_frequency(root_note) as f32;

    let mut frames = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / sample_rate_hz as f32;
        let decay = (-t * 6.0).exp();
        let sample = (2.0 * PI32 * freq * t).sin() * decay;
        frames.push(sample);
    }

    let unique = std::process::id();
    let path = env::temp_dir().join(format!("crest_synth_sample_demo_{unique}.wav"));
    write_wav_mono_16(&path, sample_rate_hz, &frames);
    path
}

/// Resamples `data`'s frames via linear interpolation so that playing them
/// back at the native sample rate sounds at `target_note`'s pitch instead of
/// the sample's recorded root key. A `target_note` equal to the sample's
/// root key produces the original frames (step == 1.0).
fn render_pitch_shifted(data: &SampleData, target_note: u8) -> Vec<f32> {
    let root_freq = note_to_frequency(data.root_key);
    let target_freq = note_to_frequency(target_note);
    let step = target_freq / root_freq;

    let source = &data.frames;
    if source.is_empty() || step <= 0.0 {
        return Vec::new();
    }

    let out_len = ((source.len() as f64) / step).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * step;
        let idx0 = src_pos.floor() as usize;
        let frac = (src_pos - idx0 as f64) as f32;
        let s0 = source.get(idx0).copied().unwrap_or(0.0);
        let s1 = source.get(idx0 + 1).copied().unwrap_or(s0);
        out.push(s0 + (s1 - s0) * frac);
    }
    out
}

/// Reads the first `len` frames of `data` at its native (root) pitch, i.e.
/// with no resampling at all. Used as the "not pitch-shifted" comparison
/// point to prove interpolation is not a no-op.
fn read_same_length_at_root(data: &SampleData, len: usize) -> Vec<f32> {
    data.frames.iter().take(len).copied().collect()
}

/// Sums `passages` end-to-end (each passage starting where the previous one
/// left off) into one output buffer, mixing `MIX_BLOCK_SIZE` frames at a
/// time rather than one giant slice copy.
fn mix_in_blocks(passages: &[Vec<f32>]) -> Vec<f32> {
    let total_len: usize = passages.iter().map(|p| p.len()).sum();
    let mut output = vec![0.0_f32; total_len];

    let mut offset = 0usize;
    for passage in passages {
        let mut cursor = 0usize;
        while cursor < passage.len() {
            let end = (cursor + MIX_BLOCK_SIZE).min(passage.len());
            for i in cursor..end {
                output[offset + i] += passage[i];
            }
            cursor = end;
        }
        offset += passage.len();
    }

    output
}

/// Writes `frames` (values expected in roughly `[-1.0, 1.0]`) as a 16-bit
/// mono PCM WAV file at `sample_rate_hz`. Pure-Rust: no WAV-writing crate.
fn write_wav_mono_16(path: &Path, sample_rate_hz: u32, frames: &[f32]) {
    const BITS_PER_SAMPLE: u16 = 16;
    const CHANNELS: u16 = 1;

    let block_align = CHANNELS * (BITS_PER_SAMPLE / 8);
    let byte_rate = sample_rate_hz * block_align as u32;
    let data_bytes = frames.len() * 2;

    let mut bytes = Vec::with_capacity(44 + data_bytes);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&((36 + data_bytes) as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");

    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&CHANNELS.to_le_bytes());
    bytes.extend_from_slice(&sample_rate_hz.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());

    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    for &sample in frames {
        let clamped = sample.clamp(-1.0, 1.0);
        let quantized = (clamped * i16::MAX as f32) as i16;
        bytes.extend_from_slice(&quantized.to_le_bytes());
    }

    fs::write(path, bytes).expect("failed to write WAV file");
}
