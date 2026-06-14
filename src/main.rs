// path: src/main.rs
//
// Tone test: plays a 3-second C4-E4-G4 arpeggio and writes to tone-test.wav.
//
// Notes:
//   C4 = MIDI 60, starts at 0.0 s, ends at 0.4 s
//   E4 = MIDI 64, starts at 0.5 s, ends at 0.9 s
//   G4 = MIDI 67, starts at 1.0 s, ends at 1.4 s
//
// Audio is rendered in 256-sample blocks at 44100 Hz (mono, converted to
// stereo for WAV), then written as a 16-bit PCM WAV file.

use std::fs::File;
use std::io::{BufWriter, Write};

use crest_synth::audio::sine_voice::SineVoice;
use crest_synth::kernel::note_id::NoteId;
use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::velocity::Velocity;

const SAMPLE_RATE: u32 = 44100;
const BLOCK_SIZE: usize = 256;
const TOTAL_SECONDS: f64 = 3.0;

/// A scheduled note event.
struct NoteEvent {
    /// Sample offset (from start) when this event fires.
    sample: usize,
    kind: NoteEventKind,
    note_id: NoteId,
    note_number: Option<NoteNumber>,
    velocity: Option<Velocity>,
}

enum NoteEventKind {
    On,
    Off,
}

fn main() {
    // Build the arpeggio schedule.
    // C4=60, E4=64, G4=67; on at 0.0/0.5/1.0 s, off at 0.4/0.9/1.4 s.
    let sr = SAMPLE_RATE as f64;
    let events = build_schedule(sr);

    let total_samples = (TOTAL_SECONDS * sr) as usize;
    let mut voice = SineVoice::new();
    let mut pcm: Vec<f32> = Vec::with_capacity(total_samples);

    let mut event_idx = 0;
    let num_events = events.len();

    let mut sample_pos = 0usize;

    while sample_pos < total_samples {
        let block_end = (sample_pos + BLOCK_SIZE).min(total_samples);

        // Fire any events that fall inside this block, at the correct offset.
        while event_idx < num_events && events[event_idx].sample < block_end {
            let ev = &events[event_idx];
            match ev.kind {
                NoteEventKind::On => {
                    let _ = voice.note_on(
                        ev.note_id,
                        ev.note_number.expect("note_on requires note_number"),
                        ev.velocity.expect("note_on requires velocity"),
                    );
                }
                NoteEventKind::Off => {
                    let _ = voice.note_off(ev.note_id);
                }
            }
            event_idx += 1;
        }

        // Render samples for this block.
        for _ in sample_pos..block_end {
            let sample = voice.render_sample(sr);
            pcm.push(sample);
        }

        // GC inactive voices after each block (safe here — not on audio thread).
        voice.gc_voices();

        sample_pos = block_end;
    }

    // Write PCM to WAV.
    write_wav("tone-test.wav", &pcm, SAMPLE_RATE, 1).expect("failed to write tone-test.wav");

    println!(
        "Wrote tone-test.wav ({} samples, {:.1} s)",
        pcm.len(),
        pcm.len() as f64 / sr
    );
}

/// Build the sorted list of note events for the arpeggio.
fn build_schedule(sample_rate: f64) -> Vec<NoteEvent> {
    let vel = Velocity::try_new(0.8).expect("valid velocity");

    // (midi note, on_sec, off_sec, note_id)
    let notes: &[(u8, f64, f64, u32)] = &[
        (60, 0.0, 0.4, 1), // C4
        (64, 0.5, 0.9, 2), // E4
        (67, 1.0, 1.4, 3), // G4
    ];

    let mut events: Vec<NoteEvent> = Vec::new();

    for &(midi, on_sec, off_sec, id) in notes {
        let note_number = NoteNumber::try_new(midi).expect("valid note number");
        let note_id = NoteId::new(id);

        events.push(NoteEvent {
            sample: (on_sec * sample_rate) as usize,
            kind: NoteEventKind::On,
            note_id,
            note_number: Some(note_number),
            velocity: Some(vel),
        });

        events.push(NoteEvent {
            sample: (off_sec * sample_rate) as usize,
            kind: NoteEventKind::Off,
            note_id,
            note_number: None,
            velocity: None,
        });
    }

    // Sort events by sample position; note-offs before note-ons at the same
    // position to avoid double-on at 0.
    events.sort_by_key(|e| {
        let kind_order = match e.kind {
            NoteEventKind::Off => 0u8,
            NoteEventKind::On => 1u8,
        };
        (e.sample, kind_order)
    });

    events
}

/// Write a WAV file with 16-bit PCM samples.
///
/// This is a pure-Rust implementation — no external crates.
fn write_wav(path: &str, samples: &[f32], sample_rate: u32, channels: u16) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let bits_per_sample: u16 = 16;
    let block_align = channels * (bits_per_sample / 8);
    let byte_rate = sample_rate * u32::from(block_align);
    let data_size = (samples.len() * usize::from(bits_per_sample / 8)) as u32;
    let chunk_size = 36 + data_size;

    // RIFF header
    writer.write_all(b"RIFF")?;
    writer.write_all(&chunk_size.to_le_bytes())?;
    writer.write_all(b"WAVE")?;

    // fmt chunk
    writer.write_all(b"fmt ")?;
    writer.write_all(&16u32.to_le_bytes())?; // chunk size
    writer.write_all(&1u16.to_le_bytes())?; // PCM format
    writer.write_all(&channels.to_le_bytes())?;
    writer.write_all(&sample_rate.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&bits_per_sample.to_le_bytes())?;

    // data chunk
    writer.write_all(b"data")?;
    writer.write_all(&data_size.to_le_bytes())?;

    // Convert f32 samples to i16 and write.
    for &s in samples {
        // Clamp to [-1.0, 1.0] then scale to i16 range.
        let clamped = s.clamp(-1.0, 1.0);
        let pcm16 = (clamped * i16::MAX as f32) as i16;
        writer.write_all(&pcm16.to_le_bytes())?;
    }

    writer.flush()?;
    Ok(())
}
