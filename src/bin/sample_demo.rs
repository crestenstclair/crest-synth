// path: src/bin/sample_demo.rs
//
// sample_demo — hermetic SampleLibrary prover.
//
// HERMETIC: Synthesizes a tiny mono 16-bit WAV in code (decaying sine ~0.3s
// at a known root note), writes it to a temp file, loads it through
// SampleLoader, builds a two-zone SampleSet, plays a passage hitting both
// zones, pitch-shifts with SampleInterpolator (Linear), and writes the mix
// to a 16-bit mono WAV.
//
// Required stdout markers (exact text, parsed by harness):
//   zones loaded=N
//   zone hit: <name> (note=K vel=V)
//   distinct zones hit=K
//
// Three in-code assertions (non-zero exit on failure):
//   1. Loaded zone count >= 2
//   2. At least 2 distinct zones were hit during playback
//   3. Pitch-shifted interpolation is NOT a no-op

use std::collections::HashSet;
use std::env;
use std::io;
use std::sync::Arc;

use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::sample_rate::SampleRate;
use crest_synth::kernel::velocity::Velocity;
use crest_synth::sample_library::interpolation_mode::InterpolationMode;
use crest_synth::sample_library::key_velocity_range::KeyVelocityRange;
use crest_synth::sample_library::sample_format::SampleFormat;
use crest_synth::sample_library::sample_interpolator::SampleInterpolator;
use crest_synth::sample_library::sample_loader::{SampleLoader, WavLoadOptions};
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

/// Synthesize a short mono 16-bit WAV decaying sine and write it using hound.
fn synthesize_and_write_wav(
    path: &std::path::Path,
    sample_rate: u32,
    duration_secs: f64,
    freq_hz: f64,
) {
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let decay = 6.0 / duration_secs; // ~6 time-constants across the duration

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create temp WAV");
    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let envelope = (-decay * t).exp();
        let sample_f = (std::f64::consts::TAU * freq_hz * t).sin() * envelope * 0.8;
        let s = (sample_f * i16::MAX as f64).clamp(i16::MIN as f64, i16::MAX as f64) as i16;
        writer.write_sample(s).expect("write sample");
    }
    writer.finalize().expect("finalize WAV");
}

// ─────────────────────────────────────────────────────────────────────────────
// WAV writer (output)
// ─────────────────────────────────────────────────────────────────────────────

fn write_wav_output(path: &str, samples: &[f32], sample_rate: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create output WAV");
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(v).expect("write output sample");
    }
    writer.finalize().expect("finalize output WAV");
}

// ─────────────────────────────────────────────────────────────────────────────
// Zone name lookup helper
// ─────────────────────────────────────────────────────────────────────────────

/// Return a display name for the zone based on its key range.
fn zone_name(key_low: u8, key_high: u8) -> String {
    format!("notes-{key_low}-{key_high}")
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
}

fn build_passage() -> Vec<Note> {
    vec![
        // Low-key zone: notes 36–59, any velocity
        Note {
            number: 48,
            velocity: 0.3,
        },
        Note {
            number: 52,
            velocity: 0.6,
        },
        // High-key zone: notes 60–84, any velocity
        Note {
            number: 64,
            velocity: 0.9,
        },
        Note {
            number: 72,
            velocity: 0.5,
        },
        // Another low-key note
        Note {
            number: 36,
            velocity: 0.7,
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

    // ── Step 1: Synthesize a tiny sample in code (HERMETIC — no sample file in repo) ──
    let temp_path = {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "crest_synth_sample_demo_{}.wav",
            std::process::id()
        ));
        p
    };
    synthesize_and_write_wav(
        &temp_path,
        OUT_SAMPLE_RATE,
        SAMPLE_DURATION_SECS,
        ROOT_FREQ_HZ,
    );

    // ── Step 2: Load the temp WAV through SampleLoader ───────────────────────
    let loader = SampleLoader::new();
    let seed_set = loader
        .load_wav(&temp_path, SampleSetId::new(999), WavLoadOptions::default())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // Extract decoded PCM; share it across zones via Arc.
    let shared_data: Arc<[f32]> = seed_set.zones()[0].sample_data_ref();
    let num_frames = seed_set.zones()[0].frame_count() as u64;
    if num_frames == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "loaded 0 frames",
        ));
    }

    let sample_rate_obj = SampleRate::try_new(OUT_SAMPLE_RATE)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let root_note_obj = NoteNumber::try_new(ROOT_NOTE)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // ── Step 3: Build SampleSet with TWO non-overlapping zones ───────────────
    //
    // Zone A "low-key":  notes 36–59, full velocity range
    // Zone B "high-key": notes 60–84, full velocity range
    //
    // Both zones share the same Arc<[f32]> sample data.
    let vel_lo = Velocity::try_new(0.0)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let vel_hi = Velocity::try_new(1.0)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let metadata =
        SampleMetadata::try_new(1, num_frames, None, None, root_note_obj, sample_rate_obj)
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

    // Build the SampleSet aggregate via SampleLibrary.
    let mut library = SampleLibrary::new();
    let set_id: SampleSetId = library.next_id();

    let mut sample_set = SampleSet::new(set_id, "demo-set".to_string(), SampleFormat::Wav);
    sample_set
        .add_zone(zone_low)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    sample_set
        .add_zone(zone_high)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let zone_count = sample_set.zone_count() as usize;
    library.apply_load(sample_set);

    // Required output marker: zone count
    println!("zones loaded={zone_count}");

    // ── ASSERTION 1: zone count >= 2 ─────────────────────────────────────────
    assert!(
        zone_count >= 2,
        "ASSERT FAILED: zone_count {zone_count} < 2"
    );

    // ── Step 4: Drive the passage — look up zone, interpolate, render ─────────
    let set_ref = library.get(set_id).expect("just loaded; must be present");

    let passage = build_passage();
    // Each note sounds for 400 ms; notes rendered sequentially (no overlap).
    let note_duration_samples = (OUT_SAMPLE_RATE as f64 * 0.4) as usize;
    let total_samples = passage.len() * note_duration_samples;

    let mut mix_buf: Vec<f32> = vec![0.0; total_samples];
    let mut distinct_zones_hit: HashSet<String> = HashSet::new();

    // For assertion 3: collect root-pitch and pitch-shifted renders of one note
    // whose pitch_ratio != 1.0 (note 48 vs root 69 → big pitch shift).
    let mut assertion3_root_buf: Vec<f32> = Vec::new();
    let mut assertion3_shifted_buf: Vec<f32> = Vec::new();

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

        // Compute pitch ratio from zone root and target note.
        let root_midi = zone.metadata().root_note.value() as i32;
        let target_midi = note_num.value() as i32;
        let semitones = (target_midi - root_midi) as f64;
        let pitch_ratio = 2.0_f64.powf(semitones / 12.0);

        // Identify zone by key range for display + assertion 2.
        let zone_key_lo = zone.range().key_low().value();
        let zone_key_hi = zone.range().key_high().value();
        let zname = zone_name(zone_key_lo, zone_key_hi);
        distinct_zones_hit.insert(zname.clone());

        // Required output marker: zone hit
        println!(
            "zone hit: {zname} (note={} vel={:.1})",
            note.number, note.velocity
        );

        // For assertion 3: capture root-pitch and pitch-shifted renders of
        // the first note (note 48, pitch_ratio != 1.0 vs root 69).
        if assertion3_root_buf.is_empty() && (pitch_ratio - 1.0).abs() > 1e-4 {
            let zone_data = zone.sample_data_ref();
            let capture_len = zone_data.len();

            // Root-pitch render (pitch_ratio = 1.0).
            let mut root_interp = SampleInterpolator::new(InterpolationMode::Linear, 1.0);
            assertion3_root_buf = (0..capture_len)
                .map(|_| root_interp.next_frame(&zone_data))
                .collect();

            // Pitch-shifted render.
            let mut shifted_interp =
                SampleInterpolator::new(InterpolationMode::Linear, pitch_ratio);
            assertion3_shifted_buf = (0..capture_len)
                .map(|_| shifted_interp.next_frame(&zone_data))
                .collect();
        }

        // Render this note with the SampleInterpolator (Linear), in fixed blocks.
        let amplitude = note.velocity as f32 * 0.6; // scale to avoid clipping
        let zone_data = zone.sample_data_ref();
        let mut interp = SampleInterpolator::new(InterpolationMode::Linear, pitch_ratio);

        let note_start = note_idx * note_duration_samples;
        let mut block = [0.0f32; BLOCK_SIZE];
        let mut rendered = 0usize;

        while rendered < note_duration_samples {
            let to_render = BLOCK_SIZE.min(note_duration_samples - rendered);
            for slot in block[..to_render].iter_mut() {
                *slot = if interp.is_finished(zone_data.len()) {
                    0.0
                } else {
                    interp.next_frame(&zone_data) * amplitude
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

    // Required output marker: distinct zones hit count
    let distinct_count = distinct_zones_hit.len();
    println!("distinct zones hit={distinct_count}");

    // ── ASSERTION 2: at least 2 distinct zones hit ────────────────────────────
    assert!(
        distinct_count >= 2,
        "ASSERT FAILED: only {distinct_count} distinct zone(s) hit; expected >= 2"
    );

    // ── ASSERTION 3: pitch-shifted render differs from root-pitch render ───────
    assert!(
        !assertion3_root_buf.is_empty(),
        "ASSERT FAILED: no pitch-shifted note was played (all notes at root pitch?)"
    );
    let differ = assertion3_root_buf
        .iter()
        .zip(assertion3_shifted_buf.iter())
        .any(|(a, b)| (a - b).abs() > 1e-6);
    assert!(
        differ,
        "ASSERT FAILED: pitch-shifted render is identical to root-pitch render — interpolation is a no-op"
    );

    // ── Step 5: Write output WAV ──────────────────────────────────────────────
    write_wav_output(&out_path, &mix_buf, OUT_SAMPLE_RATE);
    println!("wrote {} samples to {out_path}", mix_buf.len());

    // ── Step 6: Clean up temp WAV file ────────────────────────────────────────
    if let Err(e) = std::fs::remove_file(&temp_path) {
        eprintln!(
            "warning: failed to remove temp file {}: {e}",
            temp_path.display()
        );
    }

    Ok(())
}
