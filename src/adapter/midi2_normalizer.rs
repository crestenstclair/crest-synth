// path: src/adapter/midi2_normalizer.rs

//! `Midi2Normalizer` — infrastructure adapter that implements [`MidiNormalizerPort`]
//! by upconverting MIDI 1.0 wire bytes to the internal [`MidiEvent`] model.
//!
//! This adapter delegates all byte-level parsing to the stateless
//! [`StandardMidiNormalizer`] that lives in the shell port module.  Placing the
//! concrete implementation here (infrastructure layer) keeps the shell module
//! focused on the trait contract and wire types, while this crate provides the
//! production wiring.

use crate::kernel::midi_event::MidiEvent;
use crate::shell::midi_normalizer::{MidiNormalizerPort, RawMidiMessage, StandardMidiNormalizer};

// ── Adapter ────────────────────────────────────────────────────────────────

/// Infrastructure adapter: MIDI 1.0 → internal [`MidiEvent`] upconverter.
///
/// Wraps [`StandardMidiNormalizer`] and exposes it as the canonical
/// [`MidiNormalizerPort`] implementation for production use.
///
/// # Design
/// - Stateless except for the atomic note-id counter inside the inner normalizer.
/// - All heap allocations occur on the caller's thread (not the audio thread).
/// - Constructed via [`Midi2Normalizer::new`]; no hidden dependencies.
pub struct Midi2Normalizer {
    inner: StandardMidiNormalizer,
}

impl Midi2Normalizer {
    /// Create a new [`Midi2Normalizer`].
    pub fn new() -> Self {
        Self::with_normalizer(StandardMidiNormalizer::new())
    }

    /// Create a [`Midi2Normalizer`] with an explicitly provided inner normalizer.
    ///
    /// Useful for testing scenarios that need to control note-id sequencing.
    pub fn with_normalizer(inner: StandardMidiNormalizer) -> Self {
        Self { inner }
    }
}

impl Default for Midi2Normalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiNormalizerPort for Midi2Normalizer {
    /// Translate a raw MIDI 1.0 wire message into a kernel [`MidiEvent`].
    ///
    /// Returns `None` for messages that cannot be mapped (SysEx, active-sense,
    /// truncated messages, or unrecognised status bytes).
    fn normalize(&self, raw: &RawMidiMessage) -> Option<MidiEvent> {
        self.inner.normalize(raw)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_event_kind::MidiEventKind;

    fn adapter() -> Midi2Normalizer {
        Midi2Normalizer::new()
    }

    fn raw(bytes: &[u8]) -> RawMidiMessage {
        RawMidiMessage::new(bytes.to_vec(), 0)
    }

    #[test]
    fn note_on_maps_correctly() {
        let a = adapter();
        let event = a.normalize(&raw(&[0x90, 60, 100])).unwrap();
        assert_eq!(event.kind, MidiEventKind::NoteOn);
        assert_eq!(event.note_number.value(), 60);
        assert_eq!(event.channel.value(), 0);
        assert!((event.velocity.value() - 100.0_f64 / 127.0_f64).abs() < 1e-9);
    }

    #[test]
    fn note_on_velocity_zero_becomes_note_off() {
        let a = adapter();
        let event = a.normalize(&raw(&[0x91, 64, 0])).unwrap();
        assert_eq!(event.kind, MidiEventKind::NoteOff);
        assert_eq!(event.velocity.value(), 0.0);
    }

    #[test]
    fn note_off_status_byte() {
        let a = adapter();
        let event = a.normalize(&raw(&[0x82, 48, 64])).unwrap();
        assert_eq!(event.kind, MidiEventKind::NoteOff);
        assert_eq!(event.channel.value(), 2);
        assert_eq!(event.note_number.value(), 48);
    }

    #[test]
    fn control_change_normalized_value() {
        let a = adapter();
        let event = a.normalize(&raw(&[0xB0, 7, 127])).unwrap();
        assert_eq!(event.kind, MidiEventKind::ControlChange);
        assert!((event.value - 1.0_f64).abs() < 1e-9);
    }

    #[test]
    fn program_change() {
        let a = adapter();
        let event = a.normalize(&raw(&[0xC0, 42])).unwrap();
        assert_eq!(event.kind, MidiEventKind::ProgramChange);
        assert!((event.value - 42.0_f64).abs() < 1e-9);
    }

    #[test]
    fn pitch_bend_center_is_zero() {
        let a = adapter();
        // 14-bit center = 0x2000 = lsb=0x00, msb=0x40
        let event = a.normalize(&raw(&[0xE0, 0x00, 0x40])).unwrap();
        assert_eq!(event.kind, MidiEventKind::PitchBend);
        assert!(event.value.abs() < 1e-6);
    }

    #[test]
    fn pitch_bend_max_positive() {
        let a = adapter();
        // 14-bit max = 0x3FFF = lsb=0x7F, msb=0x7F
        let event = a.normalize(&raw(&[0xE0, 0x7F, 0x7F])).unwrap();
        assert_eq!(event.kind, MidiEventKind::PitchBend);
        assert!(event.value > 0.99_f64);
    }

    #[test]
    fn sysex_returns_none() {
        let a = adapter();
        assert!(a.normalize(&raw(&[0xF0, 0x41, 0xF7])).is_none());
    }

    #[test]
    fn empty_message_returns_none() {
        let a = adapter();
        assert!(a.normalize(&raw(&[])).is_none());
    }

    #[test]
    fn note_ids_are_unique() {
        let a = adapter();
        let e1 = a.normalize(&raw(&[0x90, 60, 100])).unwrap();
        let e2 = a.normalize(&raw(&[0x90, 62, 80])).unwrap();
        assert_ne!(e1.note_id, e2.note_id);
    }

    #[test]
    fn channel_encoded_in_status_byte() {
        let a = adapter();
        // Channel 5 = 0x95 for note-on channel 5
        let event = a.normalize(&raw(&[0x95, 60, 80])).unwrap();
        assert_eq!(event.channel.value(), 5);
    }

    #[test]
    fn truncated_note_on_returns_none() {
        let a = adapter();
        assert!(a.normalize(&raw(&[0x90])).is_none());
    }

    #[test]
    fn truncated_note_off_returns_none() {
        let a = adapter();
        assert!(a.normalize(&raw(&[0x80, 60])).is_none());
    }

    #[test]
    fn default_constructs_correctly() {
        let a = Midi2Normalizer::default();
        // Confirm it normalizes a basic note-on
        let event = a
            .normalize(&RawMidiMessage::new(vec![0x90, 60, 100], 0))
            .unwrap();
        assert_eq!(event.kind, MidiEventKind::NoteOn);
    }

    #[test]
    fn implements_port_as_dyn() {
        let a: Box<dyn MidiNormalizerPort> = Box::new(Midi2Normalizer::new());
        let result = a.normalize(&RawMidiMessage::new(vec![0xF0], 0));
        assert!(result.is_none());
    }
}
