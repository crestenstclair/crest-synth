use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_event_kind::MidiEventKind;
use crate::kernel::midi_group::MidiGroup;
use crate::kernel::note_id::NoteId;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::velocity::Velocity;

/// Normalized internal MIDI event: (group, channel) addressed, high-res values, note-id tagged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidiEvent {
    /// MIDI 2.0 group index (0-15).
    pub group: MidiGroup,
    /// MIDI channel within the group (0-15).
    pub channel: MidiChannel,
    /// Unique identifier for the sounding note.
    pub note_id: NoteId,
    /// The kind of MIDI event (NoteOn, NoteOff, ControlChange, etc.).
    pub kind: MidiEventKind,
    /// MIDI note number (0-127).
    pub note_number: NoteNumber,
    /// Normalized velocity (0.0-1.0).
    pub velocity: Velocity,
    /// High-resolution control value (e.g. pitch bend, controller value).
    pub value: f64,
}

impl MidiEvent {
    /// Construct a NoteOn event.
    pub fn note_on(
        group: MidiGroup,
        channel: MidiChannel,
        note_id: NoteId,
        note_number: NoteNumber,
        velocity: Velocity,
    ) -> Self {
        Self {
            group,
            channel,
            note_id,
            kind: MidiEventKind::NoteOn,
            note_number,
            velocity,
            value: 0.0,
        }
    }

    /// Construct a NoteOff event.
    pub fn note_off(
        group: MidiGroup,
        channel: MidiChannel,
        note_id: NoteId,
        note_number: NoteNumber,
    ) -> Self {
        Self {
            group,
            channel,
            note_id,
            kind: MidiEventKind::NoteOff,
            note_number,
            velocity: Velocity::default(),
            value: 0.0,
        }
    }

    /// Construct a ControlChange event.
    pub fn control_change(
        group: MidiGroup,
        channel: MidiChannel,
        note_number: NoteNumber,
        value: f64,
    ) -> Self {
        Self {
            group,
            channel,
            note_id: NoteId::default(),
            kind: MidiEventKind::ControlChange,
            note_number,
            velocity: Velocity::default(),
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group() -> MidiGroup {
        MidiGroup::try_new(0).unwrap()
    }

    fn channel() -> MidiChannel {
        MidiChannel::try_new(0).unwrap()
    }

    fn note_id() -> NoteId {
        NoteId::new(1)
    }

    fn note_number() -> NoteNumber {
        NoteNumber::try_new(60).unwrap()
    }

    fn velocity() -> Velocity {
        Velocity::try_new(0.8).unwrap()
    }

    #[test]
    fn note_on_has_correct_kind() {
        let event = MidiEvent::note_on(group(), channel(), note_id(), note_number(), velocity());
        assert_eq!(event.kind, MidiEventKind::NoteOn);
        assert_eq!(event.note_number, note_number());
        assert_eq!(event.velocity, velocity());
    }

    #[test]
    fn note_off_has_correct_kind() {
        let event = MidiEvent::note_off(group(), channel(), note_id(), note_number());
        assert_eq!(event.kind, MidiEventKind::NoteOff);
        assert_eq!(event.note_number, note_number());
    }

    #[test]
    fn control_change_has_correct_kind_and_value() {
        let event = MidiEvent::control_change(group(), channel(), note_number(), 0.5);
        assert_eq!(event.kind, MidiEventKind::ControlChange);
        assert!((event.value - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn note_on_fields_round_trip() {
        let g = MidiGroup::try_new(1).unwrap();
        let ch = MidiChannel::try_new(3).unwrap();
        let id = NoteId::new(42);
        let nn = NoteNumber::try_new(72).unwrap();
        let vel = Velocity::try_new(1.0).unwrap();
        let event = MidiEvent::note_on(g, ch, id, nn, vel);
        assert_eq!(event.group, g);
        assert_eq!(event.channel, ch);
        assert_eq!(event.note_id, id);
        assert_eq!(event.kind, MidiEventKind::NoteOn);
        assert_eq!(event.note_number, nn);
        assert_eq!(event.velocity, vel);
        assert!((event.value).abs() < f64::EPSILON);
    }
}
