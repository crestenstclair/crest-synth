// path: src/shell/midi_normalizer.rs

//! `MidiNormalizer` converts raw MIDI 1.0 bytes into normalized `MidiEvent`s.
//!
//! Raw MIDI 1.0 messages arrive as untyped bytes: a status byte encoding a
//! message type and a channel, followed by zero, one, or two data bytes.
//! `MidiNormalizer` decodes that wire format into the domain's canonical
//! `MidiEvent` representation — addressed with a `ChannelAddress`, carrying
//! high-resolution `Velocity`, and tagged with a stable `NoteId` so per-note
//! state (envelopes, per-note expression) can be tracked independently of
//! the raw note number even across retriggers of the same key.
//!
//! `MidiNormalizer` runs on the non-real-time MIDI input thread, not the
//! audio thread: it owns a `HashMap` to correlate note-on/note-off pairs,
//! which is heap-allocating bookkeeping that would be unsafe inside the
//! audio callback. Normalized events cross into the real-time world only
//! via the `EventRing`, never through this type directly — `MidiNormalizer`
//! itself has no knowledge of the thread boundary.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::kernel::channel_address::{ChannelAddress, MidiChannel, MidiChannelError, MidiGroup};
use crate::kernel::midi_event::MidiEvent;
use crate::kernel::midi_event_kind::MidiEventKind;
use crate::kernel::note_id::NoteId;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::velocity::Velocity;

/// Identifies a currently-sounding note for note-on/note-off correlation.
///
/// Scoped to (group, channel, note number): a `MidiNormalizer` serves every
/// group and channel that arrives on its input, and the same raw note
/// number may sound concurrently on different channels without ambiguity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ActiveNoteKey {
    group: u8,
    channel: u8,
    note: u8,
}

/// Errors produced when normalizing a raw MIDI 1.0 byte sequence.
#[derive(Debug, Clone, PartialEq)]
pub enum MidiNormalizerError {
    /// The byte slice was empty; there is no status byte to interpret.
    Empty,
    /// The first byte was not a valid MIDI 1.0 channel-voice status byte
    /// (its high bit must be set and its high nibble must be one of the
    /// seven recognized channel-voice message types, `0x8`-`0xE`).
    InvalidStatusByte(u8),
    /// The message was shorter than its status byte requires (e.g. a
    /// note-on with no velocity byte).
    TooShort { expected: usize, actual: usize },
    /// The channel nibble failed to construct a valid `MidiChannel`.
    ///
    /// Unreachable in practice, since a 4-bit nibble is always in `0..=15`,
    /// but surfaced as a real error rather than unwrapped so normalization
    /// never panics on malformed input.
    InvalidChannel(MidiChannelError),
    /// A data byte carried a value outside MIDI 1.0's 7-bit range (the high
    /// bit must be clear on every data byte).
    InvalidDataByte(u8),
}

impl fmt::Display for MidiNormalizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MidiNormalizerError::Empty => write!(f, "cannot normalize an empty MIDI message"),
            MidiNormalizerError::InvalidStatusByte(byte) => write!(
                f,
                "byte {byte:#04x} is not a recognized MIDI 1.0 channel-voice status byte"
            ),
            MidiNormalizerError::TooShort { expected, actual } => write!(
                f,
                "MIDI message too short: expected {expected} byte(s), got {actual}"
            ),
            MidiNormalizerError::InvalidChannel(err) => write!(f, "{err}"),
            MidiNormalizerError::InvalidDataByte(byte) => write!(
                f,
                "data byte {byte:#04x} has its high bit set (must be 0-127)"
            ),
        }
    }
}

impl Error for MidiNormalizerError {}

impl From<MidiChannelError> for MidiNormalizerError {
    fn from(err: MidiChannelError) -> Self {
        MidiNormalizerError::InvalidChannel(err)
    }
}

/// Converts raw MIDI 1.0 bytes into normalized, addressed `MidiEvent`s.
///
/// A `MidiNormalizer` is stateful: it remembers which `NoteId` was assigned
/// to each currently-sounding note so a later note-off (or poly-pressure)
/// on the same channel and note number correlates to the same identity
/// rather than being treated as an unrelated event. All raw MIDI 1.0 input
/// is treated as arriving on group 0 — classic MIDI 1.0 has no concept of
/// groups; that concept belongs to MIDI 2.0 Universal MIDI Packets, which
/// are out of scope for this normalizer.
pub struct MidiNormalizer {
    next_note_id: u32,
    active_notes: HashMap<ActiveNoteKey, NoteId>,
    group: MidiGroup,
}

impl MidiNormalizer {
    /// The MIDI group assigned to every event produced by this normalizer,
    /// since raw MIDI 1.0 bytes carry no group information.
    fn default_group() -> MidiGroup {
        MidiGroup::try_new(0).expect("0 is within the valid 0..=15 group range")
    }

    /// Constructs a `MidiNormalizer` with no notes yet sounding.
    pub fn new() -> Self {
        Self {
            next_note_id: 0,
            active_notes: HashMap::new(),
            group: Self::default_group(),
        }
    }

    /// Normalizes one raw MIDI 1.0 message into a `MidiEvent`.
    ///
    /// `bytes` must be a single complete channel-voice message: a status
    /// byte (`0x80..=0xEF`) followed by the data bytes its message type
    /// requires (one or two, each `0x00..=0x7F`). System messages
    /// (`0xF0..=0xFF`) are not channel-addressable and are rejected as
    /// [`MidiNormalizerError::InvalidStatusByte`].
    ///
    /// A note-on with velocity `0` is normalized as a note-off, per MIDI
    /// 1.0 convention (running-status keyboards commonly send note-offs
    /// this way to avoid a status-byte change).
    pub fn normalize(&mut self, bytes: &[u8]) -> Result<MidiEvent, MidiNormalizerError> {
        let status = *bytes.first().ok_or(MidiNormalizerError::Empty)?;
        if !(0x80..=0xEF).contains(&status) {
            return Err(MidiNormalizerError::InvalidStatusByte(status));
        }

        let message_type = status & 0xF0;
        let channel_nibble = status & 0x0F;
        let channel = MidiChannel::try_new(channel_nibble)?;
        let address = ChannelAddress::new(channel, self.group);

        let data_len: usize = match message_type {
            0xC0 | 0xD0 => 1,
            _ => 2,
        };
        if bytes.len() < 1 + data_len {
            return Err(MidiNormalizerError::TooShort {
                expected: 1 + data_len,
                actual: bytes.len(),
            });
        }
        for &b in &bytes[1..1 + data_len] {
            if b > 0x7F {
                return Err(MidiNormalizerError::InvalidDataByte(b));
            }
        }

        let data1 = bytes[1];
        let data2 = if data_len == 2 { bytes[2] } else { 0 };

        let (kind, note, raw_velocity) = match message_type {
            0x80 => (MidiEventKind::NoteOff, data1, data2),
            0x90 if data2 == 0 => (MidiEventKind::NoteOff, data1, 0),
            0x90 => (MidiEventKind::NoteOn, data1, data2),
            0xA0 => (MidiEventKind::PolyPressure, data1, data2),
            0xB0 => (MidiEventKind::ControlChange, 0, data2),
            0xC0 => (MidiEventKind::ProgramChange, 0, data1),
            0xD0 => (MidiEventKind::Aftertouch, 0, data1),
            0xE0 => (MidiEventKind::PitchBend, 0, data2),
            _ => unreachable!(
                "message_type is derived from a status byte validated to be 0x80..=0xEF"
            ),
        };

        let note_number = NoteNumber::try_new(note)
            .expect("MIDI 1.0 data bytes are validated to 0..=0x7F, within NoteNumber's range");
        let velocity = Velocity::from_midi7(raw_velocity);
        let note_id = self.resolve_note_id(kind, channel_nibble, note);

        Ok(MidiEvent::new(
            address,
            kind,
            note_number,
            note_id,
            velocity,
        ))
    }

    /// Resolves the `NoteId` to tag this event with, correlating per-note
    /// event kinds (note-on, note-off, poly-pressure) to the identity
    /// assigned when the note began sounding. Event kinds that are not
    /// per-note (control change, pitch bend, aftertouch, program change)
    /// always receive a fresh `NoteId`, since they carry no note identity
    /// to correlate.
    fn resolve_note_id(&mut self, kind: MidiEventKind, channel: u8, note: u8) -> NoteId {
        if !kind.is_per_note() {
            return self.allocate_note_id();
        }

        let key = ActiveNoteKey {
            group: self.group.value(),
            channel,
            note,
        };

        match kind {
            MidiEventKind::NoteOn => {
                let id = self.allocate_note_id();
                self.active_notes.insert(key, id);
                id
            }
            MidiEventKind::NoteOff => self
                .active_notes
                .remove(&key)
                .unwrap_or_else(|| self.allocate_note_id()),
            _ => self
                .active_notes
                .get(&key)
                .copied()
                .unwrap_or_else(|| self.allocate_note_id()),
        }
    }

    /// Issues a fresh, never-before-issued `NoteId`.
    fn allocate_note_id(&mut self) -> NoteId {
        let id = NoteId::new(self.next_note_id);
        self.next_note_id = self.next_note_id.wrapping_add(1);
        id
    }
}

impl Default for MidiNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_on_normalizes_channel_note_and_velocity() {
        let mut normalizer = MidiNormalizer::new();
        let event = normalizer.normalize(&[0x90, 60, 100]).unwrap();

        assert_eq!(*event.kind(), MidiEventKind::NoteOn);
        assert_eq!(event.note().value(), 60);
        assert_eq!(event.address().channel().value(), 0);
        assert_eq!(event.address().group().value(), 0);
        assert_eq!(event.velocity(), &Velocity::from_midi7(100));
    }

    #[test]
    fn note_on_uses_the_channel_nibble() {
        let mut normalizer = MidiNormalizer::new();
        let event = normalizer.normalize(&[0x93, 10, 50]).unwrap();
        assert_eq!(event.address().channel().value(), 3);
    }

    #[test]
    fn note_on_with_zero_velocity_is_normalized_as_note_off() {
        let mut normalizer = MidiNormalizer::new();
        let event = normalizer.normalize(&[0x90, 60, 0]).unwrap();
        assert_eq!(*event.kind(), MidiEventKind::NoteOff);
    }

    #[test]
    fn note_on_then_note_off_share_the_same_note_id() {
        let mut normalizer = MidiNormalizer::new();
        let on = normalizer.normalize(&[0x90, 60, 100]).unwrap();
        let off = normalizer.normalize(&[0x80, 60, 0]).unwrap();
        assert_eq!(on.note_id(), off.note_id());
    }

    #[test]
    fn retriggering_a_note_before_release_assigns_a_new_note_id() {
        let mut normalizer = MidiNormalizer::new();
        let first_on = normalizer.normalize(&[0x90, 60, 100]).unwrap();
        let second_on = normalizer.normalize(&[0x90, 60, 90]).unwrap();
        assert_ne!(first_on.note_id(), second_on.note_id());
    }

    #[test]
    fn concurrent_notes_on_different_channels_get_independent_ids() {
        let mut normalizer = MidiNormalizer::new();
        let a = normalizer.normalize(&[0x90, 60, 100]).unwrap();
        let b = normalizer.normalize(&[0x91, 60, 100]).unwrap();
        assert_ne!(a.note_id(), b.note_id());

        let a_off = normalizer.normalize(&[0x80, 60, 0]).unwrap();
        let b_off = normalizer.normalize(&[0x81, 60, 0]).unwrap();
        assert_eq!(a.note_id(), a_off.note_id());
        assert_eq!(b.note_id(), b_off.note_id());
    }

    #[test]
    fn note_off_with_no_prior_note_on_still_produces_an_event() {
        let mut normalizer = MidiNormalizer::new();
        let off = normalizer.normalize(&[0x80, 72, 0]).unwrap();
        assert_eq!(*off.kind(), MidiEventKind::NoteOff);
        assert_eq!(off.note().value(), 72);
    }

    #[test]
    fn poly_pressure_correlates_to_the_sounding_note_id() {
        let mut normalizer = MidiNormalizer::new();
        let on = normalizer.normalize(&[0x90, 60, 100]).unwrap();
        let pressure = normalizer.normalize(&[0xA0, 60, 80]).unwrap();
        assert_eq!(*pressure.kind(), MidiEventKind::PolyPressure);
        assert_eq!(on.note_id(), pressure.note_id());
    }

    #[test]
    fn control_change_decodes_value_into_velocity_with_zero_note() {
        let mut normalizer = MidiNormalizer::new();
        let event = normalizer.normalize(&[0xB0, 7, 127]).unwrap();
        assert_eq!(*event.kind(), MidiEventKind::ControlChange);
        assert_eq!(event.note().value(), 0);
        assert_eq!(event.velocity(), &Velocity::from_midi7(127));
    }

    #[test]
    fn non_per_note_kinds_each_receive_a_fresh_note_id() {
        let mut normalizer = MidiNormalizer::new();
        let a = normalizer.normalize(&[0xB0, 7, 100]).unwrap();
        let b = normalizer.normalize(&[0xB0, 7, 100]).unwrap();
        assert_ne!(a.note_id(), b.note_id());
    }

    #[test]
    fn program_change_reads_its_single_data_byte() {
        let mut normalizer = MidiNormalizer::new();
        let event = normalizer.normalize(&[0xC0, 42]).unwrap();
        assert_eq!(*event.kind(), MidiEventKind::ProgramChange);
        assert_eq!(event.velocity(), &Velocity::from_midi7(42));
    }

    #[test]
    fn channel_pressure_reads_its_single_data_byte() {
        let mut normalizer = MidiNormalizer::new();
        let event = normalizer.normalize(&[0xD0, 64]).unwrap();
        assert_eq!(*event.kind(), MidiEventKind::Aftertouch);
        assert_eq!(event.velocity(), &Velocity::from_midi7(64));
    }

    #[test]
    fn pitch_bend_decodes_as_a_two_data_byte_message() {
        let mut normalizer = MidiNormalizer::new();
        let event = normalizer.normalize(&[0xE0, 0, 64]).unwrap();
        assert_eq!(*event.kind(), MidiEventKind::PitchBend);
    }

    #[test]
    fn empty_input_is_rejected() {
        let mut normalizer = MidiNormalizer::new();
        assert_eq!(normalizer.normalize(&[]), Err(MidiNormalizerError::Empty));
    }

    #[test]
    fn a_data_byte_as_the_first_byte_is_rejected() {
        let mut normalizer = MidiNormalizer::new();
        let err = normalizer.normalize(&[0x45, 60, 100]).unwrap_err();
        assert_eq!(err, MidiNormalizerError::InvalidStatusByte(0x45));
    }

    #[test]
    fn a_system_message_status_byte_is_rejected() {
        let mut normalizer = MidiNormalizer::new();
        let err = normalizer.normalize(&[0xF8]).unwrap_err();
        assert_eq!(err, MidiNormalizerError::InvalidStatusByte(0xF8));
    }

    #[test]
    fn a_truncated_message_is_rejected() {
        let mut normalizer = MidiNormalizer::new();
        let err = normalizer.normalize(&[0x90, 60]).unwrap_err();
        assert_eq!(
            err,
            MidiNormalizerError::TooShort {
                expected: 3,
                actual: 2
            }
        );
    }

    #[test]
    fn a_data_byte_with_its_high_bit_set_is_rejected() {
        let mut normalizer = MidiNormalizer::new();
        let err = normalizer.normalize(&[0x90, 200, 100]).unwrap_err();
        assert_eq!(err, MidiNormalizerError::InvalidDataByte(200));
    }

    #[test]
    fn default_constructs_a_usable_normalizer() {
        let mut normalizer = MidiNormalizer::default();
        assert!(normalizer.normalize(&[0x90, 60, 100]).is_ok());
    }
}
