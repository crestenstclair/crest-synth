// path: src/bin/midi_play.rs
//
// midi_play — offline MIDI-file renderer to WAV
//
// Usage: midi_play [FILE.mid] [--out OUT.wav]
//
// If FILE is omitted, a built-in demo arpeggio is synthesised.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

use crest_synth::audio::sine_voice::SineVoice;
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::midi_group::MidiGroup;
use crest_synth::kernel::note_id::NoteId;
use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::velocity::Velocity;

// ── Constants ──────────────────────────────────────────────────────────────────

const SAMPLE_RATE: u32 = 44_100;
const BLOCK_SIZE: usize = 256;

// ── Entry point ────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Parse CLI: optional positional FILE, optional --out PATH.
    let mut midi_path: Option<PathBuf> = None;
    let mut out_path = PathBuf::from("midi-play.wav");
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
            other => {
                midi_path = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }

    // Build the event timeline.
    let timeline = match midi_path {
        Some(ref path) => match load_midi_file(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("error: cannot parse MIDI file '{}': {e}", path.display());
                process::exit(1);
            }
        },
        None => builtin_demo(),
    };

    // Render.
    let result = render_to_wav(&timeline, &out_path);

    println!(
        "rendered seconds={:.3}  events={}  peak_voices={}  out={}",
        result.duration_secs,
        result.total_events,
        result.peak_voices,
        out_path.display()
    );
}

// ── MIDI event timeline ────────────────────────────────────────────────────────

/// A single scheduled MIDI note action.
#[derive(Debug, Clone)]
struct NoteEvent {
    time_secs: f64,
    note: u8,
    /// `true` = note-on, `false` = note-off.
    is_on: bool,
    velocity: f64,
}

// ── MIDI file loader ───────────────────────────────────────────────────────────

/// Load a Standard MIDI File and convert it to a flat, time-ordered list of
/// [`NoteEvent`]s in seconds.
fn load_midi_file(path: &Path) -> Result<Vec<NoteEvent>, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let smf = midly::Smf::parse(&bytes)?;

    let ticks_per_beat: u64 = match smf.header.timing {
        midly::Timing::Metrical(tpb) => tpb.as_int() as u64,
        midly::Timing::Timecode(fps, sub) => {
            return Err(
                format!("SMPTE timecode ({fps:?} fps, {sub} sub-frames) not supported").into(),
            );
        }
    };

    // Default tempo: 120 BPM = 500 000 µs/beat.
    let mut microsecs_per_beat: u64 = 500_000;
    let mut events: Vec<NoteEvent> = Vec::new();

    for track in &smf.tracks {
        let mut time_secs: f64 = 0.0;

        for ev in track {
            let delta = ev.delta.as_int() as u64;
            if delta > 0 {
                let delta_us = delta * microsecs_per_beat / ticks_per_beat;
                time_secs += delta_us as f64 / 1_000_000.0;
            }

            match ev.kind {
                midly::TrackEventKind::Midi { message, .. } => match message {
                    midly::MidiMessage::NoteOn { key, vel } => {
                        let v = vel.as_int();
                        if v == 0 {
                            events.push(NoteEvent {
                                time_secs,
                                note: key.as_int(),
                                is_on: false,
                                velocity: 0.0,
                            });
                        } else {
                            events.push(NoteEvent {
                                time_secs,
                                note: key.as_int(),
                                is_on: true,
                                velocity: v as f64 / 127.0,
                            });
                        }
                    }
                    midly::MidiMessage::NoteOff { key, .. } => {
                        events.push(NoteEvent {
                            time_secs,
                            note: key.as_int(),
                            is_on: false,
                            velocity: 0.0,
                        });
                    }
                    _ => {}
                },
                midly::TrackEventKind::Meta(midly::MetaMessage::Tempo(us)) => {
                    microsecs_per_beat = us.as_int() as u64;
                }
                _ => {}
            }
        }
    }

    // Sort: time ascending; at equal time, note-offs before note-ons.
    events.sort_by(|a, b| {
        a.time_secs
            .partial_cmp(&b.time_secs)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.is_on.cmp(&b.is_on)) // false < true → offs first
    });

    Ok(events)
}

// ── Built-in demo melody ───────────────────────────────────────────────────────

/// A short C-major arpeggio spanning ~4 seconds.
fn builtin_demo() -> Vec<NoteEvent> {
    let notes: &[u8] = &[60, 64, 67, 72, 67, 64, 60, 55, 60, 64, 67, 72];
    let note_dur = 0.3_f64;
    let step = 0.35_f64;

    let mut events: Vec<NoteEvent> = Vec::new();
    for (i, &note) in notes.iter().enumerate() {
        let t_on = i as f64 * step;
        let t_off = t_on + note_dur;
        events.push(NoteEvent {
            time_secs: t_on,
            note,
            is_on: true,
            velocity: 0.8,
        });
        events.push(NoteEvent {
            time_secs: t_off,
            note,
            is_on: false,
            velocity: 0.0,
        });
    }

    events.sort_by(|a, b| {
        a.time_secs
            .partial_cmp(&b.time_secs)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.is_on.cmp(&b.is_on))
    });
    events
}

// ── Renderer ───────────────────────────────────────────────────────────────────

struct RenderResult {
    duration_secs: f64,
    total_events: usize,
    peak_voices: usize,
}

/// Render the note-event timeline through the SineVoice engine and write a
/// 16-bit mono WAV file.
fn render_to_wav(events: &[NoteEvent], out_path: &Path) -> RenderResult {
    let sr = SAMPLE_RATE as f64;

    let last_t = events.iter().map(|e| e.time_secs).fold(0.0_f64, f64::max);
    let total_secs = last_t + 0.5;
    let total_samples = (total_secs * sr).ceil() as usize;

    // Clamp amplitude to avoid WAV clipping on polyphony.
    let amplitude = 0.25_f32;

    // Map: note_number -> NoteId (most recently activated).
    let mut active: HashMap<u8, NoteId> = HashMap::new();
    let mut next_id: u32 = 1;

    let default_group = MidiGroup::try_new(0).expect("group 0 is valid");
    let default_channel = MidiChannel::try_new(0).expect("channel 0 is valid");

    // Silence the compiler about unused fields — these are here for
    // structural clarity (a real engine would route events via group/channel).
    let _group = default_group;
    let _channel = default_channel;

    let mut voice = SineVoice::new();

    let mut samples: Vec<i16> = Vec::with_capacity(total_samples);

    let mut event_cursor = 0;
    let mut peak_voices: usize = 0;
    let mut active_voice_count: usize = 0;
    let mut events_fired: usize = 0;

    for sample_idx in 0..total_samples {
        let t = sample_idx as f64 / sr;

        // Dispatch all events whose time has been reached.
        while event_cursor < events.len() && events[event_cursor].time_secs <= t {
            let ev = &events[event_cursor];
            if ev.is_on {
                if let (Ok(note_number), Ok(velocity)) =
                    (NoteNumber::try_new(ev.note), Velocity::try_new(ev.velocity))
                {
                    // If there is already an active voice for this pitch, stop it first.
                    if let Some(old_id) = active.remove(&ev.note) {
                        let _ = voice.note_off(old_id);
                        active_voice_count = active_voice_count.saturating_sub(1);
                    }
                    let note_id = NoteId::new(next_id);
                    next_id += 1;
                    let _ = voice.note_on(note_id, note_number, velocity);
                    active.insert(ev.note, note_id);
                    active_voice_count += 1;
                    if active_voice_count > peak_voices {
                        peak_voices = active_voice_count;
                    }
                    events_fired += 1;
                }
            } else {
                if let Some(note_id) = active.remove(&ev.note) {
                    let _ = voice.note_off(note_id);
                    active_voice_count = active_voice_count.saturating_sub(1);
                    events_fired += 1;
                }
            }
            event_cursor += 1;
        }

        let raw = voice.render_sample(sr) * amplitude;
        let clamped = raw.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32) as i16;
        samples.push(pcm);

        if sample_idx % BLOCK_SIZE == 0 {
            voice.gc_voices();
        }
    }

    write_wav(out_path, &samples, SAMPLE_RATE);

    RenderResult {
        duration_secs: total_secs,
        total_events: events_fired,
        peak_voices,
    }
}

// ── Pure-Rust WAV writer ───────────────────────────────────────────────────────

fn write_wav(path: &Path, samples: &[i16], sample_rate: u32) {
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_chunk_size = (samples.len() * 2) as u32; // 2 bytes per i16

    // Total RIFF size: 4 (WAVE id) + 24 (fmt chunk) + 8 (data header) + data
    let riff_size = 4 + 24 + 8 + data_chunk_size;

    let mut buf: Vec<u8> = Vec::with_capacity(12 + 24 + 8 + data_chunk_size as usize);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt  chunk (PCM = audio format 1)
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_chunk_size.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
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
