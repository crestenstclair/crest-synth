// path: src/kernel/midi_event_kind.rs

//! `MidiEventKind` — the discriminator for the kind of a MIDI event.
//!
//! This is a pure value object: an enumeration with no behavior beyond
//! identity, equality, and lightweight classification queries. It carries
//! no payload — payload data (note number, velocity, controller value,
//! etc.) belongs on the `MidiEvent` that wraps this discriminator. Being a
//! plain `Copy` enum with no heap allocation, no locks, and no I/O, it is
//! safe to read, compare, and pass across the real-time audio boundary.

use std::fmt;

/// The kind of MIDI event being represented.
///
/// `MidiEventKind` is a closed, exhaustive enumeration. Adding a new kind
/// of MIDI event requires adding a new variant here (an explicit,
/// reviewable change) rather than encoding new event types as data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MidiEventKind {
    /// A note-on event: a key was pressed (or a note was triggered).
    NoteOn,
    /// A note-off event: a key was released (or a note was released).
    NoteOff,
    /// A control-change event: a continuous controller (CC) value changed.
    ControlChange,
    /// A pitch-bend event: the pitch wheel moved.
    PitchBend,
    /// A channel-pressure (aftertouch) event: pressure applied after the
    /// initial note-on, reported per-channel rather than per-note.
    Aftertouch,
    /// A polyphonic key-pressure event: pressure applied to an individual
    /// held note, reported per-note.
    PolyPressure,
    /// A program-change event: the active program/patch number changed.
    ProgramChange,
}

impl MidiEventKind {
    /// All variants, in a stable, canonical order.
    ///
    /// Useful for iteration in UI pickers, exhaustive test tables, and
    /// serialization round-trip checks.
    pub const ALL: [MidiEventKind; 7] = [
        MidiEventKind::NoteOn,
        MidiEventKind::NoteOff,
        MidiEventKind::ControlChange,
        MidiEventKind::PitchBend,
        MidiEventKind::Aftertouch,
        MidiEventKind::PolyPressure,
        MidiEventKind::ProgramChange,
    ];

    /// A short, human-readable, stable name for the kind.
    ///
    /// Distinct from the derived `Debug` output so display and logging can
    /// rely on it even if `Debug` formatting ever changes.
    pub const fn name(self) -> &'static str {
        match self {
            MidiEventKind::NoteOn => "NoteOn",
            MidiEventKind::NoteOff => "NoteOff",
            MidiEventKind::ControlChange => "ControlChange",
            MidiEventKind::PitchBend => "PitchBend",
            MidiEventKind::Aftertouch => "Aftertouch",
            MidiEventKind::PolyPressure => "PolyPressure",
            MidiEventKind::ProgramChange => "ProgramChange",
        }
    }

    /// True for the two variants that bracket a sounding note
    /// (`NoteOn`, `NoteOff`).
    pub const fn is_note_lifecycle(self) -> bool {
        matches!(self, MidiEventKind::NoteOn | MidiEventKind::NoteOff)
    }

    /// True for events that carry per-note identity (a specific note
    /// number), as opposed to being scoped to the whole channel.
    pub const fn is_per_note(self) -> bool {
        matches!(
            self,
            MidiEventKind::NoteOn | MidiEventKind::NoteOff | MidiEventKind::PolyPressure
        )
    }
}

impl fmt::Display for MidiEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_contains_every_variant_exactly_once() {
        let mut seen: Vec<MidiEventKind> = MidiEventKind::ALL.to_vec();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), MidiEventKind::ALL.len());
        assert_eq!(seen.len(), 7);
    }

    #[test]
    fn name_matches_display() {
        for kind in MidiEventKind::ALL {
            assert_eq!(kind.name(), kind.to_string());
        }
    }

    #[test]
    fn names_are_distinct() {
        let mut names: Vec<&str> = MidiEventKind::ALL.iter().map(|k| k.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), MidiEventKind::ALL.len());
    }

    #[test]
    fn is_note_lifecycle_true_only_for_note_on_and_note_off() {
        assert!(MidiEventKind::NoteOn.is_note_lifecycle());
        assert!(MidiEventKind::NoteOff.is_note_lifecycle());
        for kind in [
            MidiEventKind::ControlChange,
            MidiEventKind::PitchBend,
            MidiEventKind::Aftertouch,
            MidiEventKind::PolyPressure,
            MidiEventKind::ProgramChange,
        ] {
            assert!(!kind.is_note_lifecycle());
        }
    }

    #[test]
    fn is_per_note_true_for_note_on_note_off_and_poly_pressure() {
        assert!(MidiEventKind::NoteOn.is_per_note());
        assert!(MidiEventKind::NoteOff.is_per_note());
        assert!(MidiEventKind::PolyPressure.is_per_note());
        for kind in [
            MidiEventKind::ControlChange,
            MidiEventKind::PitchBend,
            MidiEventKind::Aftertouch,
            MidiEventKind::ProgramChange,
        ] {
            assert!(!kind.is_per_note());
        }
    }

    #[test]
    fn equality_and_copy_semantics() {
        let a = MidiEventKind::NoteOn;
        let b = a;
        assert_eq!(a, b);
        assert_eq!(a, MidiEventKind::NoteOn);
        assert_ne!(a, MidiEventKind::NoteOff);
    }

    #[test]
    fn ordering_is_stable_and_total() {
        let mut kinds = MidiEventKind::ALL.to_vec();
        kinds.reverse();
        kinds.sort();
        assert_eq!(kinds.to_vec(), MidiEventKind::ALL.to_vec());
    }
}
