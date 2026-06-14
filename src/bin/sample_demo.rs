// path: src/bin/sample_demo.rs
//
// sample_demo — hermetic SampleLibrary prover.
//
// Synthesizes a tiny mono WAV sample in code, loads it into a SampleSet
// with two non-overlapping zones, plays a short passage through different
// zones with linear pitch interpolation, and writes the mix to a WAV file.

use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::sync::Arc;

use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::sample_rate::SampleRate;
use crest_synth::kernel::velocity::Velocity;
use crest_synth::sample_library::interpolation_mode::InterpolationMode;
use crest_synth::sample_library::key_velocity_range::KeyVelocityRange;
use crest_synth::sample_library::sample_format::SampleFormat;
use crest_synth::sample_library::sample_metadata::SampleMetadata;
use crest_synth::sample_library::sample_set::{SampleLibrary, SampleSet};
use crest_synth::sample_library::sample_set_id::SampleSetId;
use crest_synth::sample_library::sample_zone::SampleZone;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Output sample rate for the rendered WAV.
const OUT_SAMPLE_RATE: u32 = 44_100;
/// Block size for mixing.
const BLOCK_SIZE: usize = 256;
/// Duration of the synthesized sample in seconds.
const SAMPLE_DURATION_SECS: f64 = 0.3;
/// MIDI root note for the synthesized sample (A4 = MIDI 69, 440 Hz).
const ROOT_NOTE: u8 = 69;
/// Sine frequency at root note.
const ROOT_FREQ_HZ: f64 = 440.0;

// ─────────────────────────────────────────────────────────────────────────────
// WAV synthesis — produce a short decaying sine at ROOT_NOTE
// ─────────────────────────────────────────────────────────────────────────────

/// Synthesize a short mono 16-bit WAV decaying sine and return raw bytes.
fn synthesize_wav_bytes(sample_rate: u32, duration_secs: f64, freq_hz: f64) -> Vec<u8> {
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let decay = 6.0 / duration_secs; // ~6 time-constants across the duration

    let mut pcm_i16: Vec<i16> = Vec::with_capacity(num_samples);
    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let envelope = (-decay * t).exp();
        let sample_f = (std::f64::consts::TAU * freq_hz * t).sin() * envelope * 0.8;
        let s = (sample_f * i16::MAX as f64).clamp(i16::MIN as f64, i16::MAX as f64) as i16;
        pcm_i16.push(s);
    }

    // Build minimal WAV header + data
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate: u32 = sample_rate * u32::from(num_channels) * u32::from(bits_per_sample) / 8;
    let block_align: u16 = num_channels * bits_per_sample / 8;
    let data_size: u32 = (pcm_i16.len() as u32) * u32::from(block_align);
    let riff_size: u32 = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + pcm_i16.len() * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&num_channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    for s in &pcm_i16 {
        wav.extend_from_slice(&s.to_le_bytes());
    }

    wav
}

// ─────────────────────────────────────────────────────────────────────────────
// WAV loading — parse 16-bit mono PCM from raw bytes; return f32 samples
// ─────────────────────────────────────────────────────────────────────────────

/// Minimal WAV loader: reads raw bytes and returns (sample_rate, mono f32 samples).
///
/// Handles only 16-bit mono PCM (which is what we write above).
fn load_wav_f32(bytes: &[u8]) -> Result<(u32, Vec<f32>), String> {
    if bytes.len() < 44 {
        return Err("WAV too small".to_string());
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Not a RIFF/WAVE file".to_string());
    }
    if &bytes[12..16] != b"fmt " {
        return Err("Expected fmt chunk".to_string());
    }
    let audio_format = u16::from_le_bytes([bytes[20], bytes[21]]);
    if audio_format != 1 {
        return Err("Only PCM supported".to_string());
    }
    let num_channels = u16::from_le_bytes([bytes[22], bytes[23]]);
    let sample_rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
    let bits_per_sample = u16::from_le_bytes([bytes[34], bytes[35]]);

    if num_channels != 1 || bits_per_sample != 16 {
        return Err("Only 16-bit mono PCM supported".to_string());
    }

    if &bytes[36..40] != b"data" {
        return Err("Expected data chunk after fmt".to_string());
    }
    let data_size = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]) as usize;
    let data_start = 44;
    let data_end = data_start + data_size;
    if bytes.len() < data_end {
        return Err("WAV data truncated".to_string());
    }

    let num_samples = data_size / 2;
    let mut f32_samples = Vec::with_capacity(num_samples);
    for i in 0..num_samples {
        let offset = data_start + i * 2;
        let raw = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        f32_samples.push(raw as f32 / i16::MAX as f32);
    }

    Ok((sample_rate, f32_samples))
}

// ─────────────────────────────────────────────────────────────────────────────
// Linear interpolation — read a sample buffer at a fractional position
// ─────────────────────────────────────────────────────────────────────────────

/// Read one mono sample from `data` at fractional `pos` using linear interpolation.
///
/// Returns 0.0 when `pos` is beyond the buffer.
fn interpolate_linear(data: &[f32], pos: f64) -> f32 {
    if pos < 0.0 {
        return 0.0;
    }
    let i = pos as usize;
    let frac = (pos - i as f64) as f32;
    if i + 1 < data.len() {
        data[i] * (1.0 - frac) + data[i + 1] * frac
    } else if i < data.len() {
        data[i] * (1.0 - frac)
    } else {
        0.0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SamplePlayer — renders a SampleZone pitch-shifted to a target note
// ─────────────────────────────────────────────────────────────────────────────

/// Renders sample data from a zone pitch-shifted to match `target_note`.
///
/// Pitch ratio = 2^((target_midi - root_midi) / 12).
/// Uses linear interpolation (or nearest for `InterpolationMode::Nearest`).
struct SamplePlayer {
    data: Arc<[f32]>,
    pos: f64,
    pitch_ratio: f64,
    mode: InterpolationMode,
}

impl SamplePlayer {
    fn new(zone: &SampleZone, target_note: NoteNumber, mode: InterpolationMode) -> Self {
        let root_midi = zone.metadata().root_note.value() as i32;
        let target_midi = target_note.value() as i32;
        let semitones = (target_midi - root_midi) as f64;
        let pitch_ratio = 2.0_f64.powf(semitones / 12.0);
        Self {
            data: zone.sample_data_ref(),
            pos: 0.0,
            pitch_ratio,
            mode,
        }
    }

    /// Returns true if playback has reached the end of the sample.
    fn is_done(&self) -> bool {
        self.pos as usize >= self.data.len()
    }

    /// Render the next sample.
    fn next_sample(&mut self) -> f32 {
        if self.is_done() {
            return 0.0;
        }
        let sample = match self.mode {
            InterpolationMode::Nearest => {
                let i = self.pos as usize;
                if i < self.data.len() {
                    self.data[i]
                } else {
                    0.0
                }
            }
            // Linear, Cubic, Sinc all use linear here (demo uses Linear)
            _ => interpolate_linear(&self.data, self.pos),
        };
        self.pos += self.pitch_ratio;
        sample
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// WAV writer
// ─────────────────────────────────────────────────────────────────────────────

fn write_wav(path: &str, samples: &[i16], sample_rate: u32) -> io::Result<()> {
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate: u32 = sample_rate * u32::from(num_channels) * u32::from(bits_per_sample) / 8;
    let block_align: u16 = num_channels * bits_per_sample / 8;
    let data_size: u32 = (samples.len() as u32) * u32::from(block_align);
    let riff_size: u32 = 36 + data_size;

    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    w.write_all(b"RIFF")?;
    w.write_all(&riff_size.to_le_bytes())?;
    w.write_all(b"WAVE")?;
    w.write_all(b"fmt ")?;
    w.write_all(&16u32.to_le_bytes())?;
    w.write_all(&1u16.to_le_bytes())?; // PCM
    w.write_all(&num_channels.to_le_bytes())?;
    w.write_all(&sample_rate.to_le_bytes())?;
    w.write_all(&byte_rate.to_le_bytes())?;
    w.write_all(&block_align.to_le_bytes())?;
    w.write_all(&bits_per_sample.to_le_bytes())?;
    w.write_all(b"data")?;
    w.write_all(&data_size.to_le_bytes())?;
    for s in samples {
        w.write_all(&s.to_le_bytes())?;
    }
    w.flush()?;
    Ok(())
}

fn f32_to_i16(v: f32) -> i16 {
    (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

// ─────────────────────────────────────────────────────────────────────────────
// Note passage
// ─────────────────────────────────────────────────────────────────────────────

/// A note to play in the demo passage.
struct Note {
    /// MIDI note number.
    number: u8,
    /// Velocity (0.0–1.0).
    velocity: f64,
    /// Name of expected zone for logging.
    zone_label: &'static str,
}

fn build_passage() -> Vec<Note> {
    vec![
        // Low-key zone: notes 36–59, any velocity
        Note {
            number: 48,
            velocity: 0.3,
            zone_label: "low-key",
        },
        Note {
            number: 52,
            velocity: 0.6,
            zone_label: "low-key",
        },
        // High-key zone: notes 60–84, any velocity
        Note {
            number: 64,
            velocity: 0.9,
            zone_label: "high-key",
        },
        Note {
            number: 72,
            velocity: 0.5,
            zone_label: "high-key",
        },
        // Another low-key note
        Note {
            number: 36,
            velocity: 0.7,
            zone_label: "low-key",
        },
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// main
// ─────────────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    // ── Parse CLI args ────────────────────────────────────────────────────────
    let args: Vec<String> = env::args().collect();
    let mut out_path = String::from("sample-demo.wav");
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--out" && i + 1 < args.len() {
            out_path = args[i + 1].clone();
            i += 2;
        } else {
            i += 1;
        }
    }

    println!("sample_demo: output={out_path}");

    // ── Step 1: Synthesize a tiny sample in code (HERMETIC — no sample file in repo) ──
    let wav_bytes = synthesize_wav_bytes(OUT_SAMPLE_RATE, SAMPLE_DURATION_SECS, ROOT_FREQ_HZ);

    // Write to a unique temp file, clean up at the end.
    let temp_path = {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "crest_synth_sample_demo_{}.wav",
            std::process::id()
        ));
        p
    };
    {
        let mut f = File::create(&temp_path)?;
        f.write_all(&wav_bytes)?;
    }
    println!("synthesized sample written to {}", temp_path.display());

    // ── Step 2: Load the temp WAV through our inline SampleLoader ────────────
    let (wav_sample_rate, f32_samples) =
        load_wav_f32(&wav_bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let num_frames = f32_samples.len() as u64;
    let sample_rate_obj = SampleRate::try_new(wav_sample_rate)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let root_note_obj = NoteNumber::try_new(ROOT_NOTE)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // Store PCM data behind an Arc so both zones share the same backing buffer.
    let shared_data: Arc<[f32]> = f32_samples.into();

    // ── Step 3: Build SampleSet with TWO non-overlapping zones ───────────────
    //
    // Zone 1 "low-key":  notes 36–59, full velocity range
    // Zone 2 "high-key": notes 60–84, full velocity range
    //
    // Both zones share the same Arc<[f32]> sample data (HERMETIC: no extra files).
    let vel_lo = Velocity::try_new(0.0)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let vel_hi = Velocity::try_new(1.0)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let metadata = SampleMetadata::try_new(
        1, // mono
        num_frames,
        None, // no loop
        None,
        root_note_obj,
        sample_rate_obj,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // Low-key zone: notes 36–59
    let low_key_lo = NoteNumber::try_new(36)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let low_key_hi = NoteNumber::try_new(59)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let low_range = KeyVelocityRange::try_new(low_key_lo, low_key_hi, vel_lo, vel_hi)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let zone_low = SampleZone::new(metadata, low_range, Arc::clone(&shared_data));

    // High-key zone: notes 60–84
    let high_key_lo = NoteNumber::try_new(60)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let high_key_hi = NoteNumber::try_new(84)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let high_range = KeyVelocityRange::try_new(high_key_lo, high_key_hi, vel_lo, vel_hi)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let zone_high = SampleZone::new(metadata, high_range, Arc::clone(&shared_data));

    // Build the SampleSet aggregate via SampleLibrary (the application service).
    let mut library = SampleLibrary::new();
    let set_id: SampleSetId = library.next_id();

    let mut sample_set = SampleSet::new(set_id, "demo-set".to_string(), SampleFormat::Wav);
    sample_set
        .add_zone(zone_low)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    sample_set
        .add_zone(zone_high)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let zone_count = sample_set.zone_count();
    library.apply_load(sample_set);

    // Required output marker: zone count
    println!("zones loaded={zone_count}");

    // ── Step 4: Drive the passage — look up zone, interpolate, render ─────────
    let set_ref = library.get(set_id).expect("just loaded; must be present");

    let passage = build_passage();
    // Each note sounds for 400 ms; notes are rendered sequentially (no overlap).
    let note_duration_samples = (OUT_SAMPLE_RATE as f64 * 0.4) as usize;
    let total_samples = passage.len() * note_duration_samples;

    let mut mix_buf: Vec<f32> = vec![0.0; total_samples];

    for (note_idx, note) in passage.iter().enumerate() {
        let note_num = NoteNumber::try_new(note.number)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let vel = Velocity::try_new(note.velocity)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        // Zone lookup via SampleSet::find_zone — the audio thread path.
        let zone = set_ref.find_zone(note_num, vel).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no zone for note={} vel={:.1}", note.number, note.velocity),
            )
        })?;

        // Required output marker: zone hit
        println!(
            "zone hit: {} (note={} vel={:.1})",
            note.zone_label, note.number, note.velocity
        );

        // Render this note with linear pitch interpolation.
        let mut player = SamplePlayer::new(zone, note_num, InterpolationMode::Linear);
        let amplitude = note.velocity as f32 * 0.6; // scale to avoid clipping

        let note_start = note_idx * note_duration_samples;
        let mut block = [0.0f32; BLOCK_SIZE];
        let mut rendered = 0usize;

        while rendered < note_duration_samples {
            let to_render = BLOCK_SIZE.min(note_duration_samples - rendered);
            for slot in block[..to_render].iter_mut() {
                *slot = if player.is_done() {
                    0.0
                } else {
                    player.next_sample() * amplitude
                };
            }
            let mix_start = note_start + rendered;
            let mix_end = mix_start + to_render;
            if mix_end <= mix_buf.len() {
                for (out, &s) in mix_buf[mix_start..mix_end]
                    .iter_mut()
                    .zip(&block[..to_render])
                {
                    *out += s;
                }
            }
            rendered += to_render;
        }
    }

    // ── Step 5: Convert to 16-bit PCM and write output WAV ───────────────────
    let pcm: Vec<i16> = mix_buf.iter().map(|&s| f32_to_i16(s)).collect();
    write_wav(&out_path, &pcm, OUT_SAMPLE_RATE)?;
    println!("wrote {} samples to {out_path}", pcm.len());

    // ── Step 6: Clean up temp WAV file ────────────────────────────────────────
    if let Err(e) = std::fs::remove_file(&temp_path) {
        eprintln!(
            "warning: failed to remove temp file {}: {e}",
            temp_path.display()
        );
    }

    Ok(())
}
