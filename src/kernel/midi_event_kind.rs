/// The kind of a normalized internal MIDI event.
///
/// `MidiEventKind` classifies what type of event occurred.
/// This enables pattern matching on `MidiEvent` without having to inspect
/// individual fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MidiEventKind {
    /// A note has been pressed (note-on with non-zero velocity).
    NoteOn,
    /// A note has been released (note-off or note-on with velocity 0).
    NoteOff,
    /// A continuous controller value has changed.
    ControlChange,
    /// Channel pressure (mono aftertouch).
    ChannelPressure,
    /// Per-note pitch bend.
    PitchBend,
    /// A program (patch) change.
    ProgramChange,
}

impl std::fmt::Display for MidiEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MidiEventKind::NoteOn => write!(f, "NoteOn"),
            MidiEventKind::NoteOff => write!(f, "NoteOff"),
            MidiEventKind::ControlChange => write!(f, "ControlChange"),
            MidiEventKind::ChannelPressure => write!(f, "ChannelPressure"),
            MidiEventKind::PitchBend => write!(f, "PitchBend"),
            MidiEventKind::ProgramChange => write!(f, "ProgramChange"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variants_are_distinct() {
        assert_ne!(MidiEventKind::NoteOn, MidiEventKind::NoteOff);
        assert_ne!(MidiEventKind::NoteOn, MidiEventKind::ControlChange);
    }

    #[test]
    fn copy_semantics() {
        let k = MidiEventKind::NoteOn;
        let k2 = k;
        assert_eq!(k, k2);
    }

    #[test]
    fn display_note_on() {
        assert_eq!(format!("{}", MidiEventKind::NoteOn), "NoteOn");
    }

    #[test]
    fn display_note_off() {
        assert_eq!(format!("{}", MidiEventKind::NoteOff), "NoteOff");
    }

    #[test]
    fn display_control_change() {
        assert_eq!(format!("{}", MidiEventKind::ControlChange), "ControlChange");
    }
}
