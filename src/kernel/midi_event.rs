// path: src/kernel/midi_event.rs

use crate::kernel::channel_address::ChannelAddress;
use crate::kernel::midi_event_kind::MidiEventKind;
use crate::kernel::note_id::NoteId;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::velocity::Velocity;

/// A normalized internal MIDI event: addressed, high-resolution values,
/// `NoteId` tagged.
///
/// `MidiEvent` is the canonical representation of MIDI activity once it has
/// crossed from the outside world (raw MIDI bytes, hardware controllers)
/// into the engine's domain. Every event carries an explicit
/// `ChannelAddress` so dispatch can match it against a patch's channel
/// mapping without ambiguity (layering is intentional; leakage is not), and
/// a `NoteId` so per-note state such as envelopes and per-note expression
/// can be tracked independently of the raw note number, even when the same
/// note number is retriggered before the previous instance has finished
/// releasing.
///
/// `MidiEvent` is an immutable value object: once constructed its fields
/// never change. Producing a modified event means constructing a new one.
#[derive(Debug, Clone, PartialEq)]
pub struct MidiEvent {
    address: ChannelAddress,
    kind: MidiEventKind,
    note: NoteNumber,
    note_id: NoteId,
    velocity: Velocity,
}

impl MidiEvent {
    /// Construct a new normalized MIDI event from its constituent parts.
    pub fn new(
        address: ChannelAddress,
        kind: MidiEventKind,
        note: NoteNumber,
        note_id: NoteId,
        velocity: Velocity,
    ) -> Self {
        Self {
            address,
            kind,
            note,
            note_id,
            velocity,
        }
    }

    /// The channel address this event is targeted at.
    ///
    /// Dispatch matches this address against each patch's channel mapping;
    /// a `MidiEvent` is routed to exactly the set of patches whose mapping
    /// matches, no more and no less.
    pub fn address(&self) -> &ChannelAddress {
        &self.address
    }

    /// The kind of MIDI event this is (note on/off, control change, etc).
    pub fn kind(&self) -> &MidiEventKind {
        &self.kind
    }

    /// The note number this event pertains to.
    pub fn note(&self) -> &NoteNumber {
        &self.note
    }

    /// The stable identity of the note instance this event pertains to,
    /// distinct from its raw note number so overlapping retriggers of the
    /// same note number remain individually addressable.
    pub fn note_id(&self) -> &NoteId {
        &self.note_id
    }

    /// The high-resolution velocity carried by this event.
    pub fn velocity(&self) -> &Velocity {
        &self.velocity
    }
}
