// path: src/midi_file/loader.rs

use std::collections::HashMap;

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_event::MidiEvent;
use crate::kernel::midi_group::MidiGroup;
use crate::kernel::note_id::NoteId;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::velocity::Velocity;

/// Default tempo: 120 BPM = 500 000 microseconds per quarter note.
const DEFAULT_TEMPO_MICROS: u32 = 500_000;

/// An error produced when loading or parsing a Standard MIDI File.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiLoadError {
    /// The raw bytes could not be parsed as a valid SMF.
    ParseError(String),
    /// The SMF header uses SMPTE timing, which is not yet supported.
    UnsupportedTiming,
}

impl std::fmt::Display for MidiLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MidiLoadError::ParseError(msg) => write!(f, "MIDI parse error: {msg}"),
            MidiLoadError::UnsupportedTiming => {
                write!(f, "SMPTE timecode timing is not supported")
            }
        }
    }
}

impl std::error::Error for MidiLoadError {}

/// Key for tracking active notes: (channel, note_number).
type NoteKey = (u8, u8);

/// Loads a Standard MIDI File from its raw bytes and returns a time-ordered
/// list of `(timestamp_seconds, MidiEvent)` pairs.
///
/// Delta ticks are converted to absolute seconds using the header's
/// ticks-per-quarter-note value and any `Set Tempo` meta-events found in the
/// tracks (defaulting to 120 BPM = 500 000 µs/beat until the first such event).
///
/// SMF events that have no kernel representation (text meta-events, etc.) are
/// silently ignored.
pub fn load(bytes: &[u8]) -> Result<Vec<(f64, MidiEvent)>, MidiLoadError> {
    let smf = Smf::parse(bytes).map_err(|e| MidiLoadError::ParseError(e.to_string()))?;

    let ticks_per_beat = match smf.header.timing {
        Timing::Metrical(tpb) => tpb.as_int() as u64,
        Timing::Timecode(_, _) => return Err(MidiLoadError::UnsupportedTiming),
    };

    // MIDI group 0 is used for all SMF events (no MIDI 2.0 group concept in SMF).
    let group = MidiGroup::try_new(0).expect("group 0 is always valid");

    // Collect all events from all tracks, tagging each with the track index.
    // We flatten into a single timeline (suitable for Format 0 and Format 1).
    let mut raw_events: Vec<(u64, u8, MidiMessage)> = Vec::new();

    // Build a tempo map: (absolute_tick, micros_per_beat).
    // The map is collected by scanning all tracks for Set Tempo meta-events.
    let mut tempo_map: Vec<(u64, u32)> = Vec::new();

    for track in &smf.tracks {
        let mut abs_tick: u64 = 0;
        for event in track.iter() {
            abs_tick += event.delta.as_int() as u64;
            match &event.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(micros)) => {
                    tempo_map.push((abs_tick, micros.as_int()));
                }
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    match message {
                        MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. } => {
                            raw_events.push((abs_tick, ch, *message));
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    // Sort tempo map by tick ascending; de-duplicate by keeping last value at
    // each tick (last write wins in case of conflicts).
    tempo_map.sort_by_key(|(tick, _)| *tick);

    // Sort raw note events by tick ascending (stable sort preserves track order
    // for events at the same tick).
    raw_events.sort_by_key(|(tick, _, _)| *tick);

    // Convert absolute ticks to seconds using the tempo map.
    let tick_to_secs = |abs_tick: u64| -> f64 {
        let mut secs = 0.0_f64;
        let mut last_tick: u64 = 0;
        let mut cur_micros_per_beat = DEFAULT_TEMPO_MICROS;

        for &(tempo_tick, new_micros) in &tempo_map {
            if tempo_tick >= abs_tick {
                break;
            }
            let ticks_in_segment = tempo_tick - last_tick;
            secs += ticks_in_segment as f64 * cur_micros_per_beat as f64
                / (ticks_per_beat as f64 * 1_000_000.0);
            last_tick = tempo_tick;
            cur_micros_per_beat = new_micros;
        }

        // Final segment from last_tick to abs_tick.
        let ticks_remaining = abs_tick - last_tick;
        secs += ticks_remaining as f64 * cur_micros_per_beat as f64
            / (ticks_per_beat as f64 * 1_000_000.0);
        secs
    };

    // Assign NoteIds: allocate a fresh id per sounding note; reuse on note-off.
    let mut next_note_id: u32 = 1;
    // Maps (channel, note_number) -> NoteId for currently sounding notes.
    let mut active_notes: HashMap<NoteKey, NoteId> = HashMap::new();

    let mut result: Vec<(f64, MidiEvent)> = Vec::with_capacity(raw_events.len());

    for (abs_tick, ch, message) in raw_events {
        let timestamp = tick_to_secs(abs_tick);

        let channel = match MidiChannel::try_new(ch) {
            Ok(c) => c,
            Err(_) => continue, // unreachable for valid SMF, skip gracefully
        };

        match message {
            MidiMessage::NoteOn { key, vel } => {
                let note_raw = key.as_int();
                let vel_raw = vel.as_int();

                let note_number = match NoteNumber::try_new(note_raw) {
                    Ok(n) => n,
                    Err(_) => continue,
                };

                let key_pair: NoteKey = (ch, note_raw);

                if vel_raw == 0 {
                    // Running-status convention: NoteOn with vel=0 → NoteOff.
                    let note_id = active_notes
                        .remove(&key_pair)
                        .unwrap_or_else(|| NoteId::new(0));
                    let event = MidiEvent::note_off(group, channel, note_id, note_number);
                    result.push((timestamp, event));
                } else {
                    let velocity = match Velocity::try_new(vel_raw as f64 / 127.0) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let note_id = NoteId::new(next_note_id);
                    next_note_id = next_note_id.wrapping_add(1);
                    active_notes.insert(key_pair, note_id);
                    let event = MidiEvent::note_on(group, channel, note_id, note_number, velocity);
                    result.push((timestamp, event));
                }
            }
            MidiMessage::NoteOff { key, .. } => {
                let note_raw = key.as_int();
                let note_number = match NoteNumber::try_new(note_raw) {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let key_pair: NoteKey = (ch, note_raw);
                let note_id = active_notes
                    .remove(&key_pair)
                    .unwrap_or_else(|| NoteId::new(0));
                let event = MidiEvent::note_off(group, channel, note_id, note_number);
                result.push((timestamp, event));
            }
            _ => {}
        }
    }

    // Sort final output by timestamp ascending.
    result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_event_kind::MidiEventKind;
    use midly::{num::u15, MidiMessage};
    use midly::{Format, Header, Smf, Timing, TrackEvent, TrackEventKind};

    /// Build a tiny in-memory SMF byte buffer (Format 0, single track).
    ///
    /// Events:
    ///   tick 0   - NoteOn ch0 key=60 vel=80
    ///   tick 480 - NoteOn ch0 key=60 vel=0   (running-status NoteOff convention)
    ///   tick 480 - NoteOn ch1 key=72 vel=100
    ///   tick 960 - NoteOff ch1 key=72 vel=0
    fn build_smf_bytes() -> Vec<u8> {
        let header = Header::new(Format::SingleTrack, Timing::Metrical(u15::new(480)));
        let mut smf = Smf::new(header);

        let track: Vec<TrackEvent<'static>> = vec![
            TrackEvent {
                delta: 0u32.into(),
                kind: TrackEventKind::Midi {
                    channel: 0u8.into(),
                    message: MidiMessage::NoteOn {
                        key: 60u8.into(),
                        vel: 80u8.into(),
                    },
                },
            },
            TrackEvent {
                delta: 480u32.into(),
                kind: TrackEventKind::Midi {
                    channel: 0u8.into(),
                    message: MidiMessage::NoteOn {
                        key: 60u8.into(),
                        vel: 0u8.into(), // vel=0 → NoteOff convention
                    },
                },
            },
            TrackEvent {
                delta: 0u32.into(),
                kind: TrackEventKind::Midi {
                    channel: 1u8.into(),
                    message: MidiMessage::NoteOn {
                        key: 72u8.into(),
                        vel: 100u8.into(),
                    },
                },
            },
            TrackEvent {
                delta: 480u32.into(),
                kind: TrackEventKind::Midi {
                    channel: 1u8.into(),
                    message: MidiMessage::NoteOff {
                        key: 72u8.into(),
                        vel: 0u8.into(),
                    },
                },
            },
            TrackEvent {
                delta: 0u32.into(),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            },
        ];

        smf.tracks.push(track);

        let mut buf = Vec::new();
        smf.write(&mut buf).expect("write SMF to Vec");
        buf
    }

    #[test]
    fn round_trip_event_count() {
        let bytes = build_smf_bytes();
        let events = load(&bytes).expect("load should succeed");
        // 4 events: NoteOn ch0, NoteOff ch0 (vel=0 convention), NoteOn ch1, NoteOff ch1
        assert_eq!(events.len(), 4, "expected 4 events, got {}", events.len());
    }

    #[test]
    fn events_are_sorted_ascending() {
        let bytes = build_smf_bytes();
        let events = load(&bytes).expect("load should succeed");
        for window in events.windows(2) {
            assert!(
                window[0].0 <= window[1].0,
                "events not sorted: {} > {}",
                window[0].0,
                window[1].0
            );
        }
    }

    #[test]
    fn note_on_vel_zero_becomes_note_off() {
        let bytes = build_smf_bytes();
        let events = load(&bytes).expect("load should succeed");
        // Second event (index 1) should be the NoteOff converted from vel=0
        let (_, ev) = &events[1];
        assert_eq!(
            ev.kind,
            MidiEventKind::NoteOff,
            "NoteOn vel=0 should produce NoteOff"
        );
    }

    #[test]
    fn channel_mapping_correct() {
        let bytes = build_smf_bytes();
        let events = load(&bytes).expect("load should succeed");
        // First event: ch0
        assert_eq!(events[0].1.channel.value(), 0, "first event should be ch0");
        // Third event: ch1 NoteOn
        assert_eq!(events[2].1.channel.value(), 1, "third event should be ch1");
    }

    #[test]
    fn note_on_note_off_share_note_id() {
        let bytes = build_smf_bytes();
        let events = load(&bytes).expect("load should succeed");
        // events[0] = NoteOn ch0, events[1] = NoteOff ch0 — same NoteId
        let on_id = events[0].1.note_id;
        let off_id = events[1].1.note_id;
        assert_eq!(
            on_id, off_id,
            "NoteOn and its NoteOff should share the same NoteId"
        );
    }

    #[test]
    fn velocity_normalized() {
        let bytes = build_smf_bytes();
        let events = load(&bytes).expect("load should succeed");
        let vel = events[0].1.velocity.value();
        let expected = 80.0 / 127.0;
        assert!(
            (vel - expected).abs() < 1e-9,
            "velocity should be normalized: expected {expected}, got {vel}"
        );
    }

    #[test]
    fn malformed_bytes_return_error() {
        let bad = b"this is not a midi file";
        assert!(load(bad).is_err(), "malformed bytes should return Err");
    }

    #[test]
    fn empty_smf_returns_empty_list() {
        let header = Header::new(Format::SingleTrack, Timing::Metrical(u15::new(480)));
        let mut smf = Smf::new(header);
        let track: Vec<TrackEvent<'static>> = vec![TrackEvent {
            delta: 0u32.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        }];
        smf.tracks.push(track);
        let mut buf = Vec::new();
        smf.write(&mut buf).expect("write");
        let events = load(&buf).expect("load should succeed");
        assert!(events.is_empty(), "empty track should yield no events");
    }
}
