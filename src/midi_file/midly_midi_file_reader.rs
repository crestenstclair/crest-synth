// path: src/midi_file/midly_midi_file_reader.rs

//! Adapter: `MidlyMidiFileReader` — reads a Standard MIDI File (SMF) from
//! disk using the `midly` crate and decodes it into a `Song`.
//!
//! This is the single place in the codebase that knows about the SMF byte
//! layout (chunks, running status, variable-length quantities, meta
//! events). Everything upstream of `load` depends only on the
//! `MidiFileReader` port, never on `midly` types (Dependency Inversion).
//!
//! Loading performs blocking file I/O and heap allocation, so it must only
//! ever run on the non-real-time loader thread, never from the audio
//! callback (see the invariant documented on `MidiFileReader`).
//!
//! ## Scope
//!
//! - Note On, Note Off, and Polyphonic Key Pressure are decoded into
//!   `MidiEvent`s: these are the only MIDI channel messages whose meaning
//!   fits the kernel's note-centric `MidiEvent` shape (address + kind +
//!   note number + note id + velocity).
//! - Control Change, Program Change, Channel Pressure, and Pitch Bend carry
//!   payloads (a controller number, a program number, a 14-bit bend value)
//!   that `MidiEvent` has no field for; encoding them would either lose the
//!   payload or overload an existing field with a meaning it wasn't
//!   designed for. Rather than guess, this reader skips them. Extending
//!   `MidiEvent` to carry a generic payload is a decision for the kernel
//!   context, not this adapter.
//! - The `Set Tempo` and `Time Signature` meta events are honored: every
//!   tempo change is baked into each subsequent event's absolute
//!   timestamp, per the `MidiFileReader` contract. Other meta events
//!   (track name, lyrics, etc.) are ignored.
//! - Standard MIDI Files predate MIDI 2.0 groups; every channel is
//!   addressed within group 0.
//! - Format 0 (`SingleTrack`) and Format 1 (`Parallel`) files are merged
//!   into one tick-synchronized timeline. Format 2 (`Sequential`, each
//!   track an independent song played one after another) is decoded by
//!   concatenating each track's own timeline after the previous one's.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

use crate::kernel::channel_address::{ChannelAddress, MidiChannel, MidiGroup};
use crate::kernel::midi_event::MidiEvent;
use crate::kernel::midi_event_kind::MidiEventKind;
use crate::kernel::note_id::NoteId;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::tempo::Tempo;
use crate::kernel::time_signature::TimeSignature;
use crate::kernel::velocity::Velocity;
use crate::midi_file::midi_file_reader::{
    MidiFileError, MidiFileReader, Song, TempoChange, TimedMidiEvent,
};

/// Default tempo assumed until the first `Set Tempo` meta event: 120 BPM
/// (500,000 microseconds per quarter note), per the Standard MIDI File
/// specification.
const DEFAULT_MICROS_PER_QUARTER: u32 = 500_000;

/// The decoded contents of one timeline: its timed events, the tempo
/// changes that occurred while decoding it, and the time signature found
/// (if any).
type DecodedTimeline = (Vec<TimedMidiEvent>, Vec<TempoChange>, Option<TimeSignature>);

/// The group every SMF channel is addressed within. Standard MIDI Files
/// predate MIDI 2.0 groups, so every channel lives in group 0.
fn smf_group() -> MidiGroup {
    MidiGroup::try_new(0).expect("0 is always a valid MIDI group")
}

/// Frames per second for a `midly::Fps` code, used to convert SMPTE
/// (`Timing::Timecode`) ticks into seconds.
fn fps_as_f64(fps: midly::Fps) -> f64 {
    match fps {
        midly::Fps::Fps24 => 24.0,
        midly::Fps::Fps25 => 25.0,
        midly::Fps::Fps29 => 29.97,
        midly::Fps::Fps30 => 30.0,
    }
}

/// Decodes a `Time Signature` meta event's raw numerator and power-of-two
/// denominator exponent into a `TimeSignature`, returning `None` if the
/// exponent overflows a representable denominator or the resulting
/// numerator/denominator pair is invalid.
fn decode_time_signature(numerator: u8, denominator_pow2: u8) -> Option<TimeSignature> {
    let denominator = 2u32.checked_pow(u32::from(denominator_pow2))?;
    let denominator = u8::try_from(denominator).ok()?;
    TimeSignature::try_new(numerator, denominator).ok()
}

/// The mutable decoding state threaded through one timeline (either the
/// whole file's tick-merged events, or a single track's own timeline when
/// decoding a `Format::Sequential` file): the running tempo, the tick/second
/// conversion it implies, the next fresh `NoteId` to hand out, and the
/// stack of currently-sounding note ids per (channel, note) so a note-off
/// can be paired with the note-on that started it.
struct DecodeState {
    ticks_per_beat: Option<f64>,
    seconds_per_tick_timecode: Option<f64>,
    micros_per_quarter: u32,
    last_tick: u32,
    current_seconds: f64,
    next_note_id: u32,
    active_notes: HashMap<(u8, u8), Vec<NoteId>>,
}

impl DecodeState {
    fn new(timing: Timing, start_seconds: f64, next_note_id: u32) -> Self {
        let (ticks_per_beat, seconds_per_tick_timecode) = match timing {
            Timing::Metrical(ticks) => (Some(f64::from(ticks.as_int())), None),
            Timing::Timecode(fps, subframe) => {
                let frames_per_second = fps_as_f64(fps);
                (None, Some(1.0 / (frames_per_second * f64::from(subframe))))
            }
        };
        Self {
            ticks_per_beat,
            seconds_per_tick_timecode,
            micros_per_quarter: DEFAULT_MICROS_PER_QUARTER,
            last_tick: 0,
            current_seconds: start_seconds,
            next_note_id,
            active_notes: HashMap::new(),
        }
    }

    fn seconds_per_tick(&self) -> f64 {
        match (self.ticks_per_beat, self.seconds_per_tick_timecode) {
            (Some(ticks_per_beat), _) => {
                (f64::from(self.micros_per_quarter) / 1_000_000.0) / ticks_per_beat
            }
            (None, Some(seconds_per_tick)) => seconds_per_tick,
            (None, None) => unreachable!("exactly one timing mode is always set"),
        }
    }

    fn advance_to(&mut self, abs_tick: u32) {
        let elapsed_ticks = abs_tick.saturating_sub(self.last_tick);
        self.current_seconds += f64::from(elapsed_ticks) * self.seconds_per_tick();
        self.last_tick = abs_tick;
    }

    fn fresh_note_id(&mut self) -> NoteId {
        let id = NoteId::new(self.next_note_id);
        self.next_note_id += 1;
        id
    }
}

/// Reads Standard MIDI Files from disk via the `midly` crate.
///
/// This adapter owns no state and no dependencies of its own: it is a pure
/// translation from SMF bytes to the domain's `Song` value object, so a
/// single shared instance is safe to reuse across loads.
#[derive(Debug, Default, Clone, Copy)]
pub struct MidlyMidiFileReader;

impl MidlyMidiFileReader {
    /// Constructs a new `MidlyMidiFileReader`.
    pub fn new() -> Self {
        Self
    }

    /// Decodes one timeline's events (already delta-accumulated into
    /// absolute ticks, in non-decreasing tick order) into timed events,
    /// tempo changes, and an optional time signature, threading `state` for
    /// tempo/note-id continuity.
    fn decode_events<'a>(
        events: &[(u32, TrackEventKind<'a>)],
        state: &mut DecodeState,
    ) -> Result<DecodedTimeline, MidiFileError> {
        let mut timed_events = Vec::new();
        let mut tempo_changes = Vec::new();
        let mut time_signature = None;

        for (abs_tick, kind) in events {
            state.advance_to(*abs_tick);

            match kind {
                TrackEventKind::Midi { channel, message } => {
                    let midi_channel = MidiChannel::try_new(channel.as_int())
                        .map_err(|err| MidiFileError::Format(err.to_string()))?;
                    let address = ChannelAddress::new(midi_channel, smf_group());

                    match message {
                        MidiMessage::NoteOn { key: note, vel } if vel.as_int() > 0 => {
                            let note_number = NoteNumber::try_new(note.as_int())
                                .map_err(|err| MidiFileError::Format(err.to_string()))?;
                            let note_id = state.fresh_note_id();
                            state
                                .active_notes
                                .entry((channel.as_int(), note.as_int()))
                                .or_default()
                                .push(note_id);
                            timed_events.push(TimedMidiEvent::new(
                                state.current_seconds,
                                MidiEvent::new(
                                    address,
                                    MidiEventKind::NoteOn,
                                    note_number,
                                    note_id,
                                    Velocity::from_midi7(vel.as_int()),
                                ),
                            ));
                        }
                        MidiMessage::NoteOn { key: note, vel }
                        | MidiMessage::NoteOff { key: note, vel } => {
                            // A NoteOn with velocity 0 is the running-status
                            // convention for NoteOff; both arrive here.
                            let note_number = NoteNumber::try_new(note.as_int())
                                .map_err(|err| MidiFileError::Format(err.to_string()))?;
                            let note_id = state
                                .active_notes
                                .get_mut(&(channel.as_int(), note.as_int()))
                                .filter(|stack| !stack.is_empty())
                                .map(|stack| stack.remove(stack.len() - 1))
                                .unwrap_or_else(|| state.fresh_note_id());
                            timed_events.push(TimedMidiEvent::new(
                                state.current_seconds,
                                MidiEvent::new(
                                    address,
                                    MidiEventKind::NoteOff,
                                    note_number,
                                    note_id,
                                    Velocity::from_midi7(vel.as_int()),
                                ),
                            ));
                        }
                        MidiMessage::Aftertouch { key: note, vel } => {
                            let note_number = NoteNumber::try_new(note.as_int())
                                .map_err(|err| MidiFileError::Format(err.to_string()))?;
                            let note_id = state
                                .active_notes
                                .get(&(channel.as_int(), note.as_int()))
                                .and_then(|stack| stack.last().copied())
                                .unwrap_or_else(|| state.fresh_note_id());
                            timed_events.push(TimedMidiEvent::new(
                                state.current_seconds,
                                MidiEvent::new(
                                    address,
                                    MidiEventKind::PolyPressure,
                                    note_number,
                                    note_id,
                                    Velocity::from_midi7(vel.as_int()),
                                ),
                            ));
                        }
                        MidiMessage::Controller { .. }
                        | MidiMessage::ProgramChange { .. }
                        | MidiMessage::ChannelAftertouch { .. }
                        | MidiMessage::PitchBend { .. } => {
                            // Not representable in MidiEvent's note-centric
                            // shape (see module docs); intentionally
                            // dropped.
                        }
                    }
                }
                TrackEventKind::Meta(MetaMessage::Tempo(micros_per_quarter)) => {
                    if state.ticks_per_beat.is_some() {
                        let micros = micros_per_quarter.as_int();
                        let bpm = 60_000_000.0 / f64::from(micros);
                        let tempo = Tempo::try_new(bpm)
                            .map_err(|err| MidiFileError::Format(err.to_string()))?;
                        tempo_changes.push(TempoChange::new(state.current_seconds, tempo));
                        state.micros_per_quarter = micros;
                    }
                    // Under SMPTE (Timecode) division, tempo meta events do
                    // not affect wall-clock timing and are ignored.
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(
                    numerator,
                    denominator_pow2,
                    ..,
                )) if time_signature.is_none() => {
                    time_signature = decode_time_signature(*numerator, *denominator_pow2);
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(..)) => {
                    // A time signature was already captured; the SMF's
                    // initial signature wins, per the `Song` contract.
                }
                _ => {}
            }
        }

        Ok((timed_events, tempo_changes, time_signature))
    }
}

impl MidiFileReader for MidlyMidiFileReader {
    fn load(&self, path: &Path) -> Result<Song, MidiFileError> {
        let bytes = fs::read(path)?;
        let smf = Smf::parse(&bytes).map_err(|err| MidiFileError::Format(err.to_string()))?;
        let Header { format, timing } = smf.header;

        let mut per_track_events: Vec<Vec<(u32, TrackEventKind<'_>)>> =
            Vec::with_capacity(smf.tracks.len());
        for track in &smf.tracks {
            let mut abs_tick: u32 = 0;
            let mut track_events = Vec::with_capacity(track.len());
            for event in track {
                abs_tick = abs_tick.saturating_add(event.delta.as_int());
                track_events.push((abs_tick, event.kind));
            }
            per_track_events.push(track_events);
        }

        let mut all_timed_events = Vec::new();
        let mut all_tempo_changes = Vec::new();
        let mut time_signature = None;

        match format {
            Format::SingleTrack | Format::Parallel => {
                let mut combined: Vec<(u32, TrackEventKind<'_>)> =
                    per_track_events.into_iter().flatten().collect();
                combined.sort_by_key(|(tick, _)| *tick);

                let mut state = DecodeState::new(timing, 0.0, 0);
                let (events, tempos, sig) = Self::decode_events(&combined, &mut state)?;
                all_timed_events = events;
                all_tempo_changes = tempos;
                time_signature = sig;
            }
            Format::Sequential => {
                let mut offset_seconds = 0.0;
                let mut next_note_id = 0u32;
                for track_events in &per_track_events {
                    let mut state = DecodeState::new(timing, offset_seconds, next_note_id);
                    let (events, tempos, sig) = Self::decode_events(track_events, &mut state)?;
                    next_note_id = state.next_note_id;
                    if let Some(last_event) = events.last() {
                        offset_seconds = last_event.at_seconds();
                    }
                    all_timed_events.extend(events);
                    all_tempo_changes.extend(tempos);
                    if time_signature.is_none() {
                        time_signature = sig;
                    }
                }
            }
        }

        Ok(Song::new(
            all_timed_events,
            all_tempo_changes,
            time_signature.unwrap_or_else(TimeSignature::common_time),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_event_kind::MidiEventKind as Kind;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_FILE_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Writes `bytes` to a fresh temp file and returns its path.
    fn write_temp_midi(bytes: &[u8]) -> std::path::PathBuf {
        let id = TEST_FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "crest_synth_midly_reader_test_{}_{id}.mid",
            std::process::id()
        ));
        fs::write(&path, bytes).expect("failed to write temp midi file");
        path
    }

    /// Builds a minimal single-track, format-0 Standard MIDI File: a tempo
    /// of 120 BPM, a 4/4 time signature, a note-on for note 60 (velocity
    /// 100), a note-off 96 ticks later (one quarter note, at 96 ticks per
    /// quarter note), then end-of-track.
    fn minimal_smf_bytes() -> Vec<u8> {
        let mut track_data = Vec::new();
        // Tempo meta: delta 0, FF 51 03 <500000 as 3 bytes> (120 BPM).
        track_data.extend_from_slice(&[0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]);
        // Time signature meta: delta 0, FF 58 04 04 02 18 08 (4/4).
        track_data.extend_from_slice(&[0x00, 0xFF, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08]);
        // Note On channel 0, note 60, velocity 100: delta 0.
        track_data.extend_from_slice(&[0x00, 0x90, 0x3C, 0x64]);
        // Note Off channel 0, note 60, velocity 64: delta 96 (one quarter).
        track_data.extend_from_slice(&[0x60, 0x80, 0x3C, 0x40]);
        // End of track: delta 0.
        track_data.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"MThd");
        bytes.extend_from_slice(&6u32.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes()); // format 0
        bytes.extend_from_slice(&1u16.to_be_bytes()); // 1 track
        bytes.extend_from_slice(&96u16.to_be_bytes()); // 96 ticks/quarter
        bytes.extend_from_slice(b"MTrk");
        bytes.extend_from_slice(&(track_data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&track_data);
        bytes
    }

    #[test]
    fn load_decodes_note_on_and_note_off_with_tempo_baked_timestamps() {
        let path = write_temp_midi(&minimal_smf_bytes());
        let reader = MidlyMidiFileReader::new();

        let song = reader.load(&path).expect("expected a decoded song");
        let _ = fs::remove_file(&path);

        assert_eq!(song.events().len(), 2);
        assert_eq!(song.tempo_changes().len(), 1);
        assert_eq!(song.time_signature(), TimeSignature::try_new(4, 4).unwrap());

        let note_on = &song.events()[0];
        assert_eq!(note_on.at_seconds(), 0.0);
        assert_eq!(*note_on.event().kind(), Kind::NoteOn);
        assert_eq!(note_on.event().note().value(), 60);

        let note_off = &song.events()[1];
        assert!((note_off.at_seconds() - 0.5).abs() < 1e-9);
        assert_eq!(*note_off.event().kind(), Kind::NoteOff);
    }

    #[test]
    fn load_pairs_note_off_with_the_note_id_its_note_on_minted() {
        let path = write_temp_midi(&minimal_smf_bytes());
        let reader = MidlyMidiFileReader::new();

        let song = reader.load(&path).expect("expected a decoded song");
        let _ = fs::remove_file(&path);

        let note_on_id = *song.events()[0].event().note_id();
        let note_off_id = *song.events()[1].event().note_id();
        assert_eq!(note_on_id, note_off_id);
    }

    #[test]
    fn load_reports_io_errors_for_missing_files() {
        let reader = MidlyMidiFileReader::new();
        let missing = std::env::temp_dir().join("crest_synth_definitely_missing_file.mid");

        let result = reader.load(&missing);

        assert!(matches!(result, Err(MidiFileError::Io(_))));
    }

    #[test]
    fn load_reports_format_errors_for_non_midi_bytes() {
        let path = write_temp_midi(b"this is not a standard midi file");
        let reader = MidlyMidiFileReader::new();

        let result = reader.load(&path);
        let _ = fs::remove_file(&path);

        assert!(matches!(result, Err(MidiFileError::Format(_))));
    }

    #[test]
    fn load_assigns_distinct_note_ids_to_two_note_ons() {
        let mut track_data = Vec::new();
        track_data.extend_from_slice(&[0x00, 0x90, 0x3C, 0x64]); // note on 60
        track_data.extend_from_slice(&[0x00, 0x90, 0x40, 0x64]); // note on 64, same tick
        track_data.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]); // end of track

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"MThd");
        bytes.extend_from_slice(&6u32.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.extend_from_slice(&96u16.to_be_bytes());
        bytes.extend_from_slice(b"MTrk");
        bytes.extend_from_slice(&(track_data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&track_data);

        let path = write_temp_midi(&bytes);
        let reader = MidlyMidiFileReader::new();

        let song = reader.load(&path).expect("expected a decoded song");
        let _ = fs::remove_file(&path);

        assert_eq!(song.events().len(), 2);
        assert_ne!(
            song.events()[0].event().note_id(),
            song.events()[1].event().note_id()
        );
    }
}
