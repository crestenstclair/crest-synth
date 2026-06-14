// path: src/bin/midi_play_live.rs
//
// midi_play_live — live MIDI-file player through the default audio output device
//
// Usage: midi_play_live [FILE.mid] [--seconds N] [--no-device-dry-run]
//
// If FILE is omitted, a built-in demo arpeggio is played.
// --seconds N caps playback duration to N seconds.
// --no-device-dry-run constructs the full pipeline without opening audio and exits 0.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, Instant};

use crest_synth::adapter::cpal_audio_output::CpalAudioOutput;
use crest_synth::audio::audio_renderer::AudioRenderer;
use crest_synth::kernel::audio_frame::AudioFrame;
use crest_synth::kernel::midi_event_kind::MidiEventKind;
use crest_synth::kernel::note_id::NoteId;
use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::sample_rate::SampleRate;
use crest_synth::kernel::velocity::Velocity;
use crest_synth::real_time::deferred_deallocator::deferred_deallocator;
use crest_synth::real_time::event_ring_buffer::EventRingBuffer;
use crest_synth::real_time::parameter_bridge::ParameterBridge;
use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
use crest_synth::shell::audio_output::AudioOutput;

// ── Constants ──────────────────────────────────────────────────────────────────

/// Number of audio frames per render block.
const BLOCK_SIZE: usize = 256;

/// Ring-buffer capacity for MIDI events crossing the real-time boundary.
const EVENT_RING_CAPACITY: usize = 256;

/// Default playback sample rate (Hz) passed to the device.
const DEFAULT_SAMPLE_RATE: u32 = 44_100;

// ── MIDI event timeline ────────────────────────────────────────────────────────

/// A single scheduled MIDI note action (local to this binary).
#[derive(Debug, Clone)]
struct NoteEvent {
    time_secs: f64,
    note_id: NoteId,
    note: u8,
    is_on: bool,
    velocity: f64,
}

// ── CLI args ───────────────────────────────────────────────────────────────────

struct Args {
    midi_path: Option<PathBuf>,
    cap_secs: Option<f64>,
    dry_run: bool,
}

fn parse_args() -> Args {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut midi_path: Option<PathBuf> = None;
    let mut cap_secs: Option<f64> = None;
    let mut dry_run = false;

    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--seconds" => {
                i += 1;
                if i >= raw.len() {
                    eprintln!("error: --seconds requires a numeric argument");
                    process::exit(1);
                }
                match raw[i].parse::<f64>() {
                    Ok(n) if n > 0.0 => cap_secs = Some(n),
                    _ => {
                        eprintln!("error: --seconds must be a positive number");
                        process::exit(1);
                    }
                }
            }
            "--no-device-dry-run" => {
                dry_run = true;
            }
            other => {
                midi_path = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }

    Args {
        midi_path,
        cap_secs,
        dry_run,
    }
}

// ── Built-in demo ──────────────────────────────────────────────────────────────

fn builtin_demo() -> Vec<NoteEvent> {
    let notes: &[u8] = &[60, 64, 67, 72, 67, 64, 60, 55, 60, 64, 67, 72];
    let note_dur = 0.3_f64;
    let step = 0.35_f64;

    let mut events: Vec<NoteEvent> = Vec::new();
    for (i, &note) in notes.iter().enumerate() {
        let t_on = i as f64 * step;
        let t_off = t_on + note_dur;
        let note_id = NoteId::new((i + 1) as u32);
        events.push(NoteEvent {
            time_secs: t_on,
            note_id,
            note,
            is_on: true,
            velocity: 0.8,
        });
        events.push(NoteEvent {
            time_secs: t_off,
            note_id,
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

// ── Load from MidiFileLoader ───────────────────────────────────────────────────

fn load_from_midi_file(
    path: &std::path::Path,
) -> Result<Vec<NoteEvent>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let timeline = crest_synth::midi_file::load(&bytes)?;

    // Re-assign monotonic NoteIds and track active notes for note-off matching.
    let mut active: HashMap<(u8, NoteId), NoteId> = HashMap::new();
    let mut next_id: u32 = 1;
    let mut events: Vec<NoteEvent> = Vec::new();

    for (time_secs, midi_event) in &timeline {
        let note_num = midi_event.note_number.value();
        match midi_event.kind {
            MidiEventKind::NoteOn => {
                let local_id = NoteId::new(next_id);
                next_id += 1;
                let key = (note_num, midi_event.note_id);
                active.insert(key, local_id);
                events.push(NoteEvent {
                    time_secs: *time_secs,
                    note_id: local_id,
                    note: note_num,
                    is_on: true,
                    velocity: midi_event.velocity.value(),
                });
            }
            MidiEventKind::NoteOff => {
                let key = (note_num, midi_event.note_id);
                let local_id = active.remove(&key).unwrap_or_else(|| NoteId::new(0));
                events.push(NoteEvent {
                    time_secs: *time_secs,
                    note_id: local_id,
                    note: note_num,
                    is_on: false,
                    velocity: 0.0,
                });
            }
            _ => {}
        }
    }

    events.sort_by(|a, b| {
        a.time_secs
            .partial_cmp(&b.time_secs)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.is_on.cmp(&b.is_on))
    });

    Ok(events)
}

// ── Entry point ────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();

    // Build the event timeline.
    let timeline: Vec<NoteEvent> = match &args.midi_path {
        Some(path) => match load_from_midi_file(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("error: cannot parse MIDI file '{}': {e}", path.display());
                process::exit(1);
            }
        },
        None => builtin_demo(),
    };

    // ── Dry-run mode ───────────────────────────────────────────────────────────
    //
    // Construct all real-time pipeline objects without opening any audio device.
    // Never touches cpal; returns 0 immediately and deterministically.
    if args.dry_run {
        // Construct the rtrb MIDI event ring buffer.
        let _event_ring = EventRingBuffer::new(EVENT_RING_CAPACITY);

        // Construct the triple-buffer ParameterBridge.
        let (_param_writer, _param_reader) = ParameterBridge::split(ParameterSnapshot::default());

        // Construct the DeferredDeallocator (basedrop-style deferred free).
        let (_retire_handle, _collect_handle) = deferred_deallocator();

        println!("dry-run ok: pipeline constructed");
        return;
    }

    // ── Live playback ──────────────────────────────────────────────────────────

    // `CpalAudioOutput` is !Send on macOS — keep it on this (main) thread.
    // Also construct the full pipeline objects used in the live path.
    let _event_ring = EventRingBuffer::new(EVENT_RING_CAPACITY);
    let (_param_writer, _param_reader) = ParameterBridge::split(ParameterSnapshot::default());
    let (_retire_handle, mut collect_handle) = deferred_deallocator();

    let sample_rate =
        SampleRate::try_new(DEFAULT_SAMPLE_RATE).expect("44100 Hz is a valid sample rate");

    // Open the default output device. CpalAudioOutput::new() returns None if
    // no default device is available.
    let mut audio_output = match CpalAudioOutput::new() {
        Some(o) => o,
        None => {
            eprintln!("error: no default output device");
            process::exit(1);
        }
    };

    // Open the audio stream. This returns the actual sample rate the device
    // will run at (may differ from requested; we use it for rendering).
    let stream = audio_output.open_stream(sample_rate);
    let sr_value = stream.sample_rate().value();
    let sr = sr_value as f64;

    // Compute timeline duration.
    let last_t = timeline.iter().map(|e| e.time_secs).fold(0.0_f64, f64::max);
    let timeline_duration = last_t + 0.5; // trailing silence tail
    let effective_duration = match args.cap_secs {
        Some(cap) => cap.min(timeline_duration),
        None => timeline_duration,
    };

    let event_count = timeline.iter().filter(|e| e.is_on).count();

    println!(
        "device: default  events: {}  duration: {:.2}s  sample_rate: {} Hz",
        event_count, effective_duration, sr_value
    );

    // Create the audio renderer (phase-2/3 Voice + AudioRenderer engine).
    let stream_sample_rate = stream.sample_rate();
    let mut renderer = AudioRenderer::new(stream_sample_rate);

    // Render block buffer.
    let mut block = vec![AudioFrame::silence(); BLOCK_SIZE];

    let start_time = Instant::now();
    let mut event_cursor = 0usize;
    let mut samples_rendered: u64 = 0;

    loop {
        let elapsed = start_time.elapsed().as_secs_f64();
        if elapsed >= effective_duration {
            break;
        }

        // Pace by ring-buffer free space: only write what fits.
        let available = audio_output.available_frames();
        if available == 0 {
            // Ring buffer is full — brief yield avoids busy-spinning.
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }

        let to_render = available.min(BLOCK_SIZE);

        // Dispatch MIDI events whose times fall within the upcoming block.
        let block_end_secs = (samples_rendered + to_render as u64) as f64 / sr;

        while event_cursor < timeline.len() && timeline[event_cursor].time_secs < block_end_secs {
            let ev = &timeline[event_cursor];
            if ev.time_secs < effective_duration {
                if ev.is_on {
                    if let Ok(note_number) = NoteNumber::try_new(ev.note) {
                        if let Ok(velocity) = Velocity::try_new(ev.velocity) {
                            renderer.note_on(ev.note_id, note_number, velocity);
                        }
                    }
                } else {
                    renderer.note_off(ev.note_id);
                }
            }
            event_cursor += 1;
        }

        // Render the block.
        renderer.render_block(&mut block[..to_render]);

        // Write to the audio output ring buffer (never blocks — frames that
        // don't fit are discarded, which is the correct real-time behaviour).
        audio_output.write_buffer(&block[..to_render]);

        samples_rendered += to_render as u64;

        // Periodic GC: drain the deferred deallocator off the audio thread.
        if samples_rendered % (sr as u64) < to_render as u64 {
            collect_handle.collect();
        }

        // Break if we've rendered enough samples.
        let samples_needed = (effective_duration * sr) as u64;
        if samples_rendered >= samples_needed {
            break;
        }
    }

    // Let the audio thread drain the remaining buffered frames before exiting.
    let wall_elapsed = start_time.elapsed().as_secs_f64();
    let audio_head_secs = samples_rendered as f64 / sr;
    let remaining = (audio_head_secs - wall_elapsed).max(0.0);
    if remaining > 0.0 {
        std::thread::sleep(Duration::from_secs_f64(remaining + 0.1));
    }

    // Final GC pass.
    collect_handle.collect();
}
