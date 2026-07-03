// path: src/bin/midi_play.rs
//
// Offline MIDI-file player: renders a .mid file (or a built-in demo melody)
// to a 16-bit mono WAV file through the phase-1 engine's `Oscillator` port.
//
// This binary is a batch/offline tool: it has no real-time audio callback,
// so it is not the "audio thread" governed by this project's real-time
// invariants (no allocation, no locks, no blocking I/O on that thread). It
// is free to allocate and perform blocking file I/O.
//
// Usage:
//   midi_play [FILE.mid] [--out OUT.wav]
//
// If FILE is omitted, a built-in demo melody is rendered instead so no
// .mid asset needs to live in the repository.

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::exit;

use crest_synth::engine::oscillator::{
    Amplitude, Frequency, Oscillator, OscillatorConfig, SampleRate, StandardOscillator, Waveform,
};
use crest_synth::kernel::channel_address::{ChannelAddress, MidiChannel, MidiGroup};
use crest_synth::kernel::midi_event::MidiEvent;
use crest_synth::kernel::midi_event_kind::MidiEventKind;
use crest_synth::kernel::note_id::NoteId;
use crest_synth::kernel::note_number::NoteNumber;
use crest_synth::kernel::velocity::Velocity;
use crest_synth::midi_file::midi_file_reader::{MidiFileReader, Song, TimedMidiEvent};
use crest_synth::midi_file::midly_midi_file_reader::MidlyMidiFileReader;

/// Fixed sample rate used for the offline render.
const SAMPLE_RATE_HZ: f64 = 44_100.0;
/// Fixed block size the renderer steps by (mirrors a real-time callback's
/// fixed-size buffer, even though this render is offline).
const BLOCK_SIZE: usize = 512;
/// Extra tail rendered after the last note-off so releases are not cut off.
const TAIL_SECONDS: f64 = 0.3;
/// Sustain applied to a note-on that is never matched by a note-off.
const DEFAULT_SUSTAIN_SECONDS: f64 = 0.5;
/// Per-voice attack/release fade to avoid clicks at note boundaries.
const FADE_SECONDS: f64 = 0.005;
/// Headroom applied per voice before summing, to leave room for polyphony.
const VOICE_GAIN: f32 = 0.28;

fn main() {
    match run() {
        Ok(summary) => {
            print_summary(&summary);
            exit(0);
        }
        Err(message) => {
            eprintln!("midi_play: error: {message}");
            exit(1);
        }
    }
}

struct RenderSummary {
    source: String,
    total_events: usize,
    peak_voices: usize,
    rendered_seconds: f64,
    output_path: String,
}

fn print_summary(summary: &RenderSummary) {
    println!("midi_play: source={}", summary.source);
    println!(
        "midi_play: sample_rate={} channels=1 block_size={}",
        SAMPLE_RATE_HZ as u32, BLOCK_SIZE
    );
    println!("midi_play: total events={}", summary.total_events);
    println!(
        "midi_play: peak simultaneous voices={}",
        summary.peak_voices
    );
    // The literal token `rendered seconds=` must appear verbatim so a
    // validation harness can assert the offline render actually ran.
    println!(
        "midi_play: rendered seconds={:.2}",
        summary.rendered_seconds
    );
    println!("midi_play: output={}", summary.output_path);
}

fn run() -> Result<RenderSummary, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let (file_arg, out_path) = parse_args(&args)?;

    let (events, source_label): (Vec<TimedMidiEvent>, String) = match &file_arg {
        Some(path) => {
            let song =
                load_song(path).map_err(|e| format!("failed to parse MIDI file '{path}': {e}"))?;
            (song.events().to_vec(), path.clone())
        }
        None => (build_demo_events(), "built-in demo melody".to_string()),
    };

    let total_events = events.len();
    let notes = build_notes(&events);

    let rendered = render_notes_to_pcm(&notes);
    let peak_voices = compute_peak_voices(&notes);

    write_wav(&out_path, &rendered.samples)
        .map_err(|e| format!("failed to write WAV file '{out_path}': {e}"))?;

    Ok(RenderSummary {
        source: source_label,
        total_events,
        peak_voices,
        rendered_seconds: rendered.duration_seconds,
        output_path: out_path,
    })
}

fn parse_args(args: &[String]) -> Result<(Option<String>, String), String> {
    let mut file_arg: Option<String> = None;
    let mut out_path: String = "midi-play.wav".to_string();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--out" {
            let value = args
                .get(i + 1)
                .ok_or_else(|| "--out requires a path argument".to_string())?;
            out_path = value.clone();
            i += 2;
        } else if file_arg.is_none() {
            file_arg = Some(arg.clone());
            i += 1;
        } else {
            return Err(format!("unexpected argument: {arg}"));
        }
    }

    Ok((file_arg, out_path))
}

/// Loads a .mid file into a fully decoded [`Song`] via the `MidiFileReader`
/// port, using the `midly`-backed adapter. This is a non-real-time, blocking
/// path -- exactly as `MidiFileReader`'s contract requires.
fn load_song(path: &str) -> Result<Song, String> {
    let reader = MidlyMidiFileReader::new();
    reader.load(Path::new(path)).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------
// Built-in demo melody (no external asset file required).
// ---------------------------------------------------------------------

fn demo_address(channel: u8) -> ChannelAddress {
    ChannelAddress::new(
        MidiChannel::try_new(channel).expect("demo channel is within 0..=15"),
        MidiGroup::try_new(0).expect("group 0 is always valid"),
    )
}

/// Appends a matched NoteOn/NoteOff pair (sharing one freshly minted
/// `NoteId`, as a real `MidiFileReader` would) to `events`.
#[allow(clippy::too_many_arguments)]
fn push_demo_note(
    events: &mut Vec<TimedMidiEvent>,
    next_note_id: &mut u32,
    channel: u8,
    note: u8,
    velocity: u8,
    start_seconds: f64,
    end_seconds: f64,
) {
    let note_number = NoteNumber::try_new(note).expect("demo note number is in 0..=127");
    let note_id = NoteId::new(*next_note_id);
    *next_note_id += 1;
    let velocity = Velocity::from_midi7(velocity);
    let address = demo_address(channel);

    events.push(TimedMidiEvent::new(
        start_seconds,
        MidiEvent::new(
            address,
            MidiEventKind::NoteOn,
            note_number,
            note_id,
            velocity,
        ),
    ));
    events.push(TimedMidiEvent::new(
        end_seconds,
        MidiEvent::new(
            address,
            MidiEventKind::NoteOff,
            note_number,
            note_id,
            velocity,
        ),
    ));
}

/// Builds a short, recognizable arpeggio over a sustained bass note so the
/// demo exercises basic polyphony (multiple simultaneous voices). Spans
/// exactly 4.0 seconds of melody before the release tail.
fn build_demo_events() -> Vec<TimedMidiEvent> {
    let mut events = Vec::new();
    let mut next_note_id = 0u32;

    const STEP_SECONDS: f64 = 0.25;
    const MELODY: [u8; 16] = [
        60, 64, 67, 72, 67, 64, 60, 64, 67, 72, 67, 64, 60, 64, 67, 72,
    ];
    const MELODY_CHANNEL: u8 = 0;
    const MELODY_VELOCITY: u8 = 100;

    for (i, &note) in MELODY.iter().enumerate() {
        let start = i as f64 * STEP_SECONDS;
        let end = start + STEP_SECONDS;
        push_demo_note(
            &mut events,
            &mut next_note_id,
            MELODY_CHANNEL,
            note,
            MELODY_VELOCITY,
            start,
            end,
        );
    }

    // Sustained bass note under the first half of the arpeggio, giving a
    // recognizable moment of polyphony (peak simultaneous voices > 1).
    push_demo_note(&mut events, &mut next_note_id, 1, 48, 80, 0.0, 2.0);

    events.sort_by(|a, b| a.at_seconds().partial_cmp(&b.at_seconds()).unwrap());
    events
}

// ---------------------------------------------------------------------
// Note matching: pairs NoteOn/NoteOff into discrete sounding notes.
// ---------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Note {
    note_number: u8,
    /// Normalized velocity in `0.0..=1.0`.
    velocity: f64,
    start_seconds: f64,
    end_seconds: f64,
}

/// Matches NoteOn events to their NoteOff by shared `NoteId` (the identity a
/// `MidiFileReader` mints per sounding note), producing discrete `Note`
/// intervals. Unmatched trailing note-ons are given a default sustain so
/// they still render audibly.
fn build_notes(events: &[TimedMidiEvent]) -> Vec<Note> {
    let mut open: HashMap<NoteId, (u8, f64, f64)> = HashMap::new();
    let mut notes = Vec::new();
    let mut last_time = 0.0f64;

    for timed in events {
        last_time = last_time.max(timed.at_seconds());
        let event = timed.event();
        match event.kind() {
            MidiEventKind::NoteOn => {
                open.insert(
                    *event.note_id(),
                    (
                        event.note().value(),
                        event.velocity().value(),
                        timed.at_seconds(),
                    ),
                );
            }
            MidiEventKind::NoteOff => {
                if let Some((note_number, velocity, start)) = open.remove(event.note_id()) {
                    notes.push(Note {
                        note_number,
                        velocity,
                        start_seconds: start,
                        end_seconds: timed.at_seconds().max(start),
                    });
                }
            }
            _ => {}
        }
    }

    // Any notes still open at the end of the file get a default sustain.
    for (note_number, velocity, start) in open.into_values() {
        notes.push(Note {
            note_number,
            velocity,
            start_seconds: start,
            end_seconds: start + DEFAULT_SUSTAIN_SECONDS.max(last_time - start),
        });
    }

    notes.sort_by(|a, b| a.start_seconds.partial_cmp(&b.start_seconds).unwrap());
    notes
}

fn note_to_frequency_hz(note_number: u8) -> f64 {
    440.0 * 2f64.powf((note_number as f64 - 69.0) / 12.0)
}

// ---------------------------------------------------------------------
// Offline rendering: the phase-1 engine's `Oscillator` port, stepped in
// fixed blocks, with active voices summed for basic polyphony.
// ---------------------------------------------------------------------

struct RenderedAudio {
    samples: Vec<i16>,
    duration_seconds: f64,
}

#[derive(Clone, Copy)]
struct ActiveVoice {
    frequency: Frequency,
    velocity_gain: f32,
    phase: f64,
    start_sample: usize,
    end_sample: usize,
}

fn render_notes_to_pcm(notes: &[Note]) -> RenderedAudio {
    let oscillator = StandardOscillator::new();
    let sample_rate = SampleRate::try_new(SAMPLE_RATE_HZ).expect("44.1kHz is a valid sample rate");
    let osc_config = OscillatorConfig::new(
        Waveform::Sine,
        Amplitude::try_new(1.0).expect("1.0 is a valid amplitude"),
    );

    let last_end = notes.iter().map(|n| n.end_seconds).fold(0.0f64, f64::max);
    let duration_seconds = last_end + TAIL_SECONDS;
    let total_samples = ((duration_seconds * SAMPLE_RATE_HZ).ceil() as usize).max(1);

    let mut mix = vec![0f32; total_samples];

    // Sort notes by start sample so we can trigger them as the block
    // cursor reaches them, mirroring how a real-time engine triggers
    // note_on/note_off at the correct sample offsets.
    let mut pending: Vec<ActiveVoice> = notes
        .iter()
        .map(|n| ActiveVoice {
            frequency: Frequency::try_new(note_to_frequency_hz(n.note_number))
                .expect("a MIDI note number always yields an audible positive frequency"),
            velocity_gain: (n.velocity as f32) * VOICE_GAIN,
            phase: 0.0,
            start_sample: (n.start_seconds * SAMPLE_RATE_HZ).round() as usize,
            end_sample: (n.end_seconds * SAMPLE_RATE_HZ).round() as usize,
        })
        .collect();
    pending.sort_by_key(|v| v.start_sample);

    let fade_samples = ((FADE_SECONDS * SAMPLE_RATE_HZ) as usize).max(1);

    let mut next_pending_idx = 0usize;
    let mut active: Vec<ActiveVoice> = Vec::new();

    let mut block_start = 0usize;
    while block_start < total_samples {
        let block_end = (block_start + BLOCK_SIZE).min(total_samples);

        // Trigger note-ons whose start sample falls within this block.
        while next_pending_idx < pending.len() && pending[next_pending_idx].start_sample < block_end
        {
            active.push(pending[next_pending_idx]);
            next_pending_idx += 1;
        }

        for (sample_idx, mix_sample) in mix.iter_mut().enumerate().take(block_end).skip(block_start)
        {
            let mut sample_value = 0f32;
            for voice in active.iter_mut() {
                if sample_idx < voice.start_sample || sample_idx >= voice.end_sample {
                    continue;
                }
                let phase_into_note = sample_idx - voice.start_sample;
                let samples_remaining = voice.end_sample - sample_idx;
                let envelope = fade_gain(phase_into_note, samples_remaining, fade_samples);

                let raw = oscillator.render(voice.phase, osc_config);
                sample_value += (raw as f32) * voice.velocity_gain * envelope;
                voice.phase = oscillator.advance(voice.phase, voice.frequency, sample_rate);
            }
            *mix_sample = sample_value;
        }

        // Drop voices that have fully ended; freed here since this is an
        // offline batch process, not the real-time audio thread.
        active.retain(|voice| voice.end_sample > block_end);

        block_start = block_end;
    }

    let samples = mix.into_iter().map(clamp_to_i16).collect();

    RenderedAudio {
        samples,
        duration_seconds,
    }
}

fn fade_gain(phase_into_note: usize, samples_remaining: usize, fade_samples: usize) -> f32 {
    let attack = if phase_into_note < fade_samples {
        phase_into_note as f32 / fade_samples as f32
    } else {
        1.0
    };
    let release = if samples_remaining < fade_samples {
        samples_remaining as f32 / fade_samples as f32
    } else {
        1.0
    };
    attack.min(release).clamp(0.0, 1.0)
}

/// Simple limiter: clamps the mixed signal to `[-1.0, 1.0]` before
/// quantizing, matching this project's canonical signal path (a limiter
/// sits before output).
fn clamp_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32) as i16
}

fn compute_peak_voices(notes: &[Note]) -> usize {
    let mut edges: Vec<(usize, i32)> = Vec::with_capacity(notes.len() * 2);
    for note in notes {
        let start = (note.start_seconds * SAMPLE_RATE_HZ).round() as usize;
        let end = (note.end_seconds * SAMPLE_RATE_HZ).round() as usize;
        edges.push((start, 1));
        edges.push((end, -1));
    }
    // At equal sample offsets, process note-offs (-1) before note-ons (+1)
    // so a note ending exactly when the next begins is not double-counted
    // as an overlap.
    edges.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let mut running = 0i32;
    let mut peak = 0i32;
    for (_, delta) in edges {
        running += delta;
        peak = peak.max(running);
    }
    peak.max(0) as usize
}

// ---------------------------------------------------------------------
// Pure-Rust 16-bit mono WAV writer (no external WAV crate).
// ---------------------------------------------------------------------

fn write_wav(path: &str, samples: &[i16]) -> Result<(), String> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);

    const BITS_PER_SAMPLE: u16 = 16;
    const NUM_CHANNELS: u16 = 1;
    let sample_rate = SAMPLE_RATE_HZ as u32;
    let byte_rate = sample_rate * NUM_CHANNELS as u32 * (BITS_PER_SAMPLE as u32 / 8);
    let block_align = NUM_CHANNELS * (BITS_PER_SAMPLE / 8);
    let data_len = (samples.len() * 2) as u32;
    let riff_len = 36 + data_len;

    writer.write_all(b"RIFF").map_err(|e| e.to_string())?;
    writer
        .write_all(&riff_len.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer.write_all(b"WAVE").map_err(|e| e.to_string())?;

    writer.write_all(b"fmt ").map_err(|e| e.to_string())?;
    writer
        .write_all(&16u32.to_le_bytes())
        .map_err(|e| e.to_string())?; // fmt chunk size
    writer
        .write_all(&1u16.to_le_bytes())
        .map_err(|e| e.to_string())?; // PCM format
    writer
        .write_all(&NUM_CHANNELS.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&sample_rate.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&byte_rate.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&block_align.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&BITS_PER_SAMPLE.to_le_bytes())
        .map_err(|e| e.to_string())?;

    writer.write_all(b"data").map_err(|e| e.to_string())?;
    writer
        .write_all(&data_len.to_le_bytes())
        .map_err(|e| e.to_string())?;
    for sample in samples {
        writer
            .write_all(&sample.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }

    writer.flush().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn address() -> ChannelAddress {
        demo_address(0)
    }

    fn note_event(note_id: u32, kind: MidiEventKind, note: u8, velocity: u8) -> MidiEvent {
        MidiEvent::new(
            address(),
            kind,
            NoteNumber::try_new(note).unwrap(),
            NoteId::new(note_id),
            Velocity::from_midi7(velocity),
        )
    }

    #[test]
    fn a4_frequency_is_440_hz() {
        let freq = note_to_frequency_hz(69);
        assert!((freq - 440.0).abs() < 0.001);
    }

    #[test]
    fn one_octave_up_doubles_frequency() {
        let a4 = note_to_frequency_hz(69);
        let a5 = note_to_frequency_hz(81);
        assert!((a5 - 2.0 * a4).abs() < 0.001);
    }

    #[test]
    fn build_notes_matches_on_off_pairs_by_note_id() {
        let events = vec![
            TimedMidiEvent::new(0.0, note_event(1, MidiEventKind::NoteOn, 60, 100)),
            TimedMidiEvent::new(1.0, note_event(1, MidiEventKind::NoteOff, 60, 0)),
        ];
        let notes = build_notes(&events);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].note_number, 60);
        assert_eq!(notes[0].start_seconds, 0.0);
        assert_eq!(notes[0].end_seconds, 1.0);
    }

    #[test]
    fn build_notes_gives_unmatched_note_on_a_default_sustain() {
        let events = vec![TimedMidiEvent::new(
            0.0,
            note_event(1, MidiEventKind::NoteOn, 60, 100),
        )];
        let notes = build_notes(&events);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].end_seconds > notes[0].start_seconds);
    }

    #[test]
    fn build_notes_ignores_non_lifecycle_events() {
        let events = vec![
            TimedMidiEvent::new(0.0, note_event(1, MidiEventKind::NoteOn, 60, 100)),
            TimedMidiEvent::new(0.5, note_event(1, MidiEventKind::PolyPressure, 60, 80)),
            TimedMidiEvent::new(1.0, note_event(1, MidiEventKind::NoteOff, 60, 0)),
        ];
        let notes = build_notes(&events);
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn peak_voices_counts_overlap() {
        let notes = vec![
            Note {
                note_number: 60,
                velocity: 1.0,
                start_seconds: 0.0,
                end_seconds: 1.0,
            },
            Note {
                note_number: 64,
                velocity: 1.0,
                start_seconds: 0.5,
                end_seconds: 1.5,
            },
        ];
        assert_eq!(compute_peak_voices(&notes), 2);
    }

    #[test]
    fn peak_voices_is_one_when_sequential() {
        let notes = vec![
            Note {
                note_number: 60,
                velocity: 1.0,
                start_seconds: 0.0,
                end_seconds: 1.0,
            },
            Note {
                note_number: 64,
                velocity: 1.0,
                start_seconds: 1.0,
                end_seconds: 2.0,
            },
        ];
        assert_eq!(compute_peak_voices(&notes), 1);
    }

    #[test]
    fn demo_events_are_time_ordered_and_nonempty() {
        let events = build_demo_events();
        assert!(!events.is_empty());
        for pair in events.windows(2) {
            assert!(pair[0].at_seconds() <= pair[1].at_seconds());
        }
    }

    #[test]
    fn demo_notes_include_a_moment_of_polyphony() {
        let events = build_demo_events();
        let notes = build_notes(&events);
        assert!(compute_peak_voices(&notes) >= 2);
    }

    #[test]
    fn render_produces_pcm_covering_the_full_duration_plus_tail() {
        let notes = vec![Note {
            note_number: 69,
            velocity: 1.0,
            start_seconds: 0.0,
            end_seconds: 1.0,
        }];
        let rendered = render_notes_to_pcm(&notes);
        assert!(rendered.duration_seconds >= 1.0);
        let expected_samples = (rendered.duration_seconds * SAMPLE_RATE_HZ).ceil() as usize;
        assert_eq!(rendered.samples.len(), expected_samples);
    }

    #[test]
    fn render_does_not_clip_a_single_voice() {
        let notes = vec![Note {
            note_number: 69,
            velocity: 1.0,
            start_seconds: 0.0,
            end_seconds: 0.5,
        }];
        let rendered = render_notes_to_pcm(&notes);
        let peak = rendered
            .samples
            .iter()
            .map(|s| s.unsigned_abs())
            .max()
            .unwrap_or(0);
        assert!(peak < i16::MAX as u16);
    }

    #[test]
    fn parse_args_defaults_to_demo_and_default_out_path() {
        let (file_arg, out_path) = parse_args(&[]).unwrap();
        assert_eq!(file_arg, None);
        assert_eq!(out_path, "midi-play.wav");
    }

    #[test]
    fn parse_args_reads_file_and_out_flag() {
        let args: Vec<String> = vec![
            "song.mid".to_string(),
            "--out".to_string(),
            "out.wav".to_string(),
        ];
        let (file_arg, out_path) = parse_args(&args).unwrap();
        assert_eq!(file_arg, Some("song.mid".to_string()));
        assert_eq!(out_path, "out.wav");
    }

    #[test]
    fn parse_args_rejects_out_flag_without_value() {
        let args: Vec<String> = vec!["--out".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn load_song_reports_a_clear_error_for_a_missing_file() {
        let result = load_song("/definitely/does/not/exist.mid");
        assert!(result.is_err());
    }
}
