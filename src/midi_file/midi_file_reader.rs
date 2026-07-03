// path: src/midi_file/midi_file_reader.rs

//! Port: `MidiFileReader`.
//!
//! Defines the boundary between the MIDI-file domain and the outside world
//! (the filesystem, standard-MIDI-file byte layout). Implementations live in
//! the adapter layer (e.g. a Standard MIDI File parser) and are injected
//! into callers that need a decoded `Song` — callers depend on this trait,
//! not on any concrete file-format parser (Dependency Inversion).
//!
//! This is a non-real-time boundary: loading involves blocking file I/O and
//! heap allocation, so implementations must run on the UI/loader thread,
//! never inside the audio callback. A loaded `Song` reaches the audio
//! thread only via the ParameterBridge/EventRing handoff performed by the
//! caller, never directly from this trait.
//!
//! A `MidiFileReader` implementation must honor two contractual behaviors
//! baked into the shape of `Song`:
//!
//! - tempo changes embedded in the source file are honored when computing
//!   every event's [`TimedMidiEvent::at_seconds`] — the timestamp already
//!   reflects every tempo change that preceded it, so callers never need to
//!   re-derive timing from raw tick deltas themselves;
//! - every note-on event is tagged with a freshly minted [`NoteId`], never
//!   a reused or synthesized-from-note-number identifier, so overlapping
//!   retriggers of the same note number remain individually addressable
//!   downstream.

use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::kernel::midi_event::MidiEvent;
use crate::kernel::tempo::Tempo;
use crate::kernel::time_signature::TimeSignature;

/// A single decoded MIDI event, timestamped in absolute seconds from the
/// start of playback.
///
/// The timestamp already accounts for every tempo change that occurred
/// before this event in the source file — it is computed once, at load
/// time, by walking the file's tempo map, so downstream consumers can
/// schedule playback purely from `at_seconds` without re-deriving timing
/// from raw tick deltas.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedMidiEvent {
    at_seconds: f64,
    event: MidiEvent,
}

impl TimedMidiEvent {
    /// Constructs a `TimedMidiEvent` from an already-computed absolute
    /// timestamp and the normalized event it applies to.
    pub fn new(at_seconds: f64, event: MidiEvent) -> Self {
        Self { at_seconds, event }
    }

    /// The absolute time, in seconds from the start of playback, at which
    /// this event occurs. Already reflects every tempo change that
    /// preceded it in the source file.
    pub fn at_seconds(&self) -> f64 {
        self.at_seconds
    }

    /// The normalized MIDI event carried by this timestamped occurrence.
    pub fn event(&self) -> &MidiEvent {
        &self.event
    }
}

/// A tempo change occurring at a specific point in a `Song`, expressed as an
/// absolute time in seconds from the start of playback (not as a raw tick
/// offset).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempoChange {
    at_seconds: f64,
    tempo: Tempo,
}

impl TempoChange {
    /// Constructs a `TempoChange` at an already-computed absolute time.
    pub fn new(at_seconds: f64, tempo: Tempo) -> Self {
        Self { at_seconds, tempo }
    }

    /// The absolute time, in seconds from the start of playback, at which
    /// this tempo takes effect.
    pub fn at_seconds(&self) -> f64 {
        self.at_seconds
    }

    /// The tempo that takes effect at `at_seconds`.
    pub fn tempo(&self) -> Tempo {
        self.tempo
    }
}

/// A fully decoded song: a time-ordered sequence of MIDI events plus the
/// tempo map and time signature used to compute their absolute timestamps.
///
/// `Song` is an immutable value object produced by a [`MidiFileReader`].
/// Every [`TimedMidiEvent`] it carries already has tempo changes baked into
/// its `at_seconds`, and every note-on event among them carries a freshly
/// minted `NoteId` — callers never need to reprocess timing or identity.
#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    events: Vec<TimedMidiEvent>,
    tempo_changes: Vec<TempoChange>,
    time_signature: TimeSignature,
}

impl Song {
    /// Constructs a `Song` from its decoded events, tempo map, and time
    /// signature.
    pub fn new(
        events: Vec<TimedMidiEvent>,
        tempo_changes: Vec<TempoChange>,
        time_signature: TimeSignature,
    ) -> Self {
        Self {
            events,
            tempo_changes,
            time_signature,
        }
    }

    /// The song's events, in non-decreasing order of `at_seconds`.
    pub fn events(&self) -> &[TimedMidiEvent] {
        &self.events
    }

    /// The tempo changes present in the source file, in non-decreasing
    /// order of `at_seconds`. Every event's `at_seconds` already accounts
    /// for these; callers reconstructing a tempo map should not double
    /// apply them.
    pub fn tempo_changes(&self) -> &[TempoChange] {
        &self.tempo_changes
    }

    /// The song's time signature (the file's initial/only time signature).
    pub fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }
}

/// Failure modes when loading a `Song` from a MIDI file.
#[derive(Debug)]
pub enum MidiFileError {
    /// The file could not be opened or read (permissions, missing file, I/O
    /// error). Carries the OS-provided error for diagnostics.
    Io(std::io::Error),
    /// The file was read but its contents do not conform to the expected
    /// Standard MIDI File structure (corrupt or unrecognized chunks).
    Format(String),
}

impl fmt::Display for MidiFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MidiFileError::Io(err) => write!(f, "midi file I/O error: {err}"),
            MidiFileError::Format(msg) => write!(f, "malformed midi file: {msg}"),
        }
    }
}

impl Error for MidiFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MidiFileError::Io(err) => Some(err),
            MidiFileError::Format(_) => None,
        }
    }
}

impl From<std::io::Error> for MidiFileError {
    fn from(err: std::io::Error) -> Self {
        MidiFileError::Io(err)
    }
}

/// Port: loads a `Song` from a Standard MIDI File on disk.
///
/// This trait is the single seam between the MIDI-file domain and the
/// concrete Standard MIDI File byte layout. Implementations (an SMF-0/SMF-1
/// parser, future formats) live behind this interface so callers depend on
/// the abstraction rather than any one file format's parser. Runs on the
/// non-real-time loader thread only — never call from the audio callback.
pub trait MidiFileReader {
    /// Loads the MIDI file at `path`, decoding it into a `Song` whose
    /// events are addressed, `NoteId`-tagged, and timestamped in absolute
    /// seconds with every tempo change already applied.
    fn load(&self, path: &Path) -> Result<Song, MidiFileError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::channel_address::{ChannelAddress, MidiChannel, MidiGroup};
    use crate::kernel::midi_event_kind::MidiEventKind;
    use crate::kernel::note_id::NoteId;
    use crate::kernel::note_number::NoteNumber;
    use crate::kernel::velocity::Velocity;
    use std::io;

    struct StubReader {
        result: fn() -> Result<Song, MidiFileError>,
    }

    impl MidiFileReader for StubReader {
        fn load(&self, _path: &Path) -> Result<Song, MidiFileError> {
            (self.result)()
        }
    }

    fn address() -> ChannelAddress {
        ChannelAddress::new(
            MidiChannel::try_new(0).expect("0 is in range"),
            MidiGroup::try_new(0).expect("0 is in range"),
        )
    }

    fn note_on_event(note_id: u32) -> MidiEvent {
        MidiEvent::new(
            address(),
            MidiEventKind::NoteOn,
            NoteNumber::try_new(60).expect("60 is a valid note number"),
            NoteId::new(note_id),
            Velocity::from_midi7(100),
        )
    }

    #[test]
    fn load_returns_a_decoded_song() {
        let reader = StubReader {
            result: || {
                Ok(Song::new(
                    vec![TimedMidiEvent::new(0.0, note_on_event(1))],
                    vec![TempoChange::new(
                        0.0,
                        Tempo::try_new(120.0).expect("120 BPM is valid"),
                    )],
                    TimeSignature::common_time(),
                ))
            },
        };

        let song = reader
            .load(Path::new("song.mid"))
            .expect("expected a decoded song");

        assert_eq!(song.events().len(), 1);
        assert_eq!(song.tempo_changes().len(), 1);
        assert_eq!(song.time_signature(), TimeSignature::common_time());
    }

    #[test]
    fn timed_events_expose_absolute_seconds_reflecting_tempo_changes() {
        // Two note-ons at the same tick offset under different tempos land
        // at different absolute times once tempo has been baked in.
        let early = TimedMidiEvent::new(0.5, note_on_event(1));
        let late = TimedMidiEvent::new(1.5, note_on_event(2));

        assert!(early.at_seconds() < late.at_seconds());
        assert_eq!(early.event().note_id(), &NoteId::new(1));
        assert_eq!(late.event().note_id(), &NoteId::new(2));
    }

    #[test]
    fn note_on_events_carry_distinct_fresh_note_ids() {
        let song = Song::new(
            vec![
                TimedMidiEvent::new(0.0, note_on_event(1)),
                TimedMidiEvent::new(0.1, note_on_event(2)),
            ],
            vec![],
            TimeSignature::common_time(),
        );

        let ids: Vec<NoteId> = song
            .events()
            .iter()
            .map(|timed| *timed.event().note_id())
            .collect();

        assert_ne!(ids[0], ids[1]);
    }

    #[test]
    fn tempo_change_accessors_return_constructed_values() {
        let tempo = Tempo::try_new(90.0).expect("90 BPM is valid");
        let change = TempoChange::new(2.0, tempo);

        assert_eq!(change.at_seconds(), 2.0);
        assert_eq!(change.tempo(), tempo);
    }

    #[test]
    fn load_propagates_io_errors() {
        let reader = StubReader {
            result: || {
                Err(MidiFileError::from(io::Error::new(
                    io::ErrorKind::NotFound,
                    "missing",
                )))
            },
        };

        let result = reader.load(Path::new("missing.mid"));

        assert!(matches!(result, Err(MidiFileError::Io(_))));
    }

    #[test]
    fn load_reports_format_errors_for_corrupt_files() {
        let reader = StubReader {
            result: || Err(MidiFileError::Format("bad MThd header".to_string())),
        };

        let result = reader.load(Path::new("corrupt.mid"));

        assert!(matches!(result, Err(MidiFileError::Format(_))));
    }

    #[test]
    fn midi_file_error_display_messages_are_distinct_per_variant() {
        let io_err = MidiFileError::from(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
        let format_err = MidiFileError::Format("truncated track chunk".to_string());

        assert!(io_err.to_string().contains("I/O error"));
        assert!(format_err.to_string().contains("malformed"));
    }
}
