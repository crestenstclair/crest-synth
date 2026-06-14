// path: src/shell/midi_normalizer.rs

//! MidiNormalizer port — translates raw MIDI bytes into kernel [`MidiEvent`] values.
//!
//! This module owns the [`RawMidiMessage`] wire type and the [`MidiNormalizerPort`]
//! trait.  The adapter [`StandardMidiNormalizer`] handles the most common MIDI 1.0
//! status bytes (note-on, note-off, control-change, program-change, pitch-bend).

use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_event::MidiEvent;
use crate::kernel::midi_event_kind::MidiEventKind;
use crate::kernel::midi_group::MidiGroup;
use crate::kernel::note_id::NoteId;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::velocity::Velocity;

// ── Wire type ──────────────────────────────────────────────────────────────

/// Raw bytes received from a MIDI port before normalization.
///
/// Typically 1-3 bytes of a standard MIDI 1.0 message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMidiMessage {
    /// The raw status + data bytes of one MIDI message.
    pub bytes: Vec<u8>,
    /// MIDI 2.0 group this message arrived on (0-15; clamped to 15 if larger).
    pub group: u8,
}

impl RawMidiMessage {
    /// Create a new [`RawMidiMessage`].
    pub fn new(bytes: Vec<u8>, group: u8) -> Self {
        Self { bytes, group }
    }
}

// ── Port trait ─────────────────────────────────────────────────────────────

/// Contract: translates a raw MIDI wire message into an internal [`MidiEvent`].
///
/// Returns `None` when the message cannot be mapped (e.g. SysEx, active-sense,
/// or truncated messages).
pub trait MidiNormalizerPort {
    fn normalize(&self, raw: &RawMidiMessage) -> Option<MidiEvent>;
}

// ── Standard MIDI 1.0 adapter ─────────────────────────────────────────────

/// Stateless adapter that normalizes common MIDI 1.0 status bytes.
///
/// # Supported messages
/// - `8x` (note-off) → [`MidiEventKind::NoteOff`]
/// - `9x` with velocity > 0 → [`MidiEventKind::NoteOn`]
/// - `9x` with velocity 0 (running-status note-off convention) → [`MidiEventKind::NoteOff`]
/// - `Bx` (control-change) → [`MidiEventKind::ControlChange`] (value normalized `[0.0, 1.0]`)
/// - `Cx` (program-change) → [`MidiEventKind::ProgramChange`]
/// - `Ex` (pitch-bend) → [`MidiEventKind::PitchBend`] (value normalized `[-1.0, 1.0]`)
/// - Everything else → `None`
///
/// [`NoteId`] values are allocated monotonically using an atomic counter, so
/// successive calls produce unique ids without shared mutable state.
pub struct StandardMidiNormalizer {
    next_id: std::sync::atomic::AtomicU32,
}

impl StandardMidiNormalizer {
    /// Create a new [`StandardMidiNormalizer`].
    pub fn new() -> Self {
        Self {
            next_id: std::sync::atomic::AtomicU32::new(1),
        }
    }

    fn alloc_id(&self) -> NoteId {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        NoteId::new(id)
    }
}

impl Default for StandardMidiNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiNormalizerPort for StandardMidiNormalizer {
    fn normalize(&self, raw: &RawMidiMessage) -> Option<MidiEvent> {
        let bytes = &raw.bytes;
        if bytes.is_empty() {
            return None;
        }

        let status = bytes[0];
        let nibble = status & 0xF0;
        let ch_byte = status & 0x0F;

        let group = MidiGroup::try_new(raw.group.min(15)).ok()?;
        let channel = MidiChannel::try_new(ch_byte).ok()?;

        match nibble {
            // Note-off (8x) — explicit note-off status byte
            0x80 if bytes.len() >= 3 => {
                let note_number = NoteNumber::try_new(bytes[1]).ok()?;
                Some(MidiEvent::note_off(
                    group,
                    channel,
                    self.alloc_id(),
                    note_number,
                ))
            }

            // Note-on (9x): velocity 0 maps to note-off per MIDI running-status convention
            0x90 if bytes.len() >= 3 => {
                let note_number = NoteNumber::try_new(bytes[1]).ok()?;
                let vel_byte = bytes[2];
                if vel_byte == 0 {
                    Some(MidiEvent::note_off(
                        group,
                        channel,
                        self.alloc_id(),
                        note_number,
                    ))
                } else {
                    let velocity = Velocity::try_new(vel_byte as f64 / 127.0).ok()?;
                    Some(MidiEvent::note_on(
                        group,
                        channel,
                        self.alloc_id(),
                        note_number,
                        velocity,
                    ))
                }
            }

            // Control-change (Bx) — controller number in byte[1], value normalized to [0.0, 1.0]
            0xB0 if bytes.len() >= 3 => {
                let note_number = NoteNumber::try_new(bytes[1]).ok()?;
                let normalized = bytes[2] as f64 / 127.0;
                Some(MidiEvent::control_change(
                    group,
                    channel,
                    note_number,
                    normalized,
                ))
            }

            // Program-change (Cx) — program number as the continuous value
            0xC0 if bytes.len() >= 2 => {
                let prog = bytes[1];
                let note_number = NoteNumber::try_new(prog).ok()?;
                Some(MidiEvent {
                    group,
                    channel,
                    note_id: self.alloc_id(),
                    kind: MidiEventKind::ProgramChange,
                    note_number,
                    velocity: Velocity::default(),
                    value: prog as f64,
                })
            }

            // Pitch-bend (Ex) — 14-bit value normalized to [-1.0, 1.0]
            0xE0 if bytes.len() >= 3 => {
                let lsb = bytes[1] as u16;
                let msb = bytes[2] as u16;
                let raw_14 = (msb << 7) | lsb;
                // Center is 0x2000 (8192); full range [0, 16383].
                let normalized = (raw_14 as f64 - 8192.0) / 8192.0;
                let clamped = normalized.clamp(-1.0, 1.0);
                Some(MidiEvent {
                    group,
                    channel,
                    note_id: self.alloc_id(),
                    kind: MidiEventKind::PitchBend,
                    note_number: NoteNumber::try_new(0).ok()?,
                    velocity: Velocity::default(),
                    value: clamped,
                })
            }

            _ => None,
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn normalizer() -> StandardMidiNormalizer {
        StandardMidiNormalizer::new()
    }

    fn raw(bytes: &[u8]) -> RawMidiMessage {
        RawMidiMessage::new(bytes.to_vec(), 0)
    }

    #[test]
    fn note_on_maps_correctly() {
        let n = normalizer();
        let event = n.normalize(&raw(&[0x90, 60, 100])).unwrap();
        assert_eq!(event.kind, MidiEventKind::NoteOn);
        assert_eq!(event.note_number.value(), 60);
        assert_eq!(event.channel.value(), 0);
        assert!((event.velocity.value() - 100.0 / 127.0).abs() < 1e-9);
    }

    #[test]
    fn note_on_velocity_zero_becomes_note_off() {
        let n = normalizer();
        let event = n.normalize(&raw(&[0x91, 64, 0])).unwrap();
        assert_eq!(event.kind, MidiEventKind::NoteOff);
        assert_eq!(event.velocity.value(), 0.0);
    }

    #[test]
    fn note_off_status_byte() {
        let n = normalizer();
        let event = n.normalize(&raw(&[0x82, 48, 64])).unwrap();
        assert_eq!(event.kind, MidiEventKind::NoteOff);
        assert_eq!(event.channel.value(), 2);
        assert_eq!(event.note_number.value(), 48);
    }

    #[test]
    fn control_change_normalized_value() {
        let n = normalizer();
        let event = n.normalize(&raw(&[0xB0, 7, 127])).unwrap();
        assert_eq!(event.kind, MidiEventKind::ControlChange);
        assert!((event.value - 1.0).abs() < 1e-9);
    }

    #[test]
    fn program_change() {
        let n = normalizer();
        let event = n.normalize(&raw(&[0xC0, 42])).unwrap();
        assert_eq!(event.kind, MidiEventKind::ProgramChange);
        assert!((event.value - 42.0).abs() < 1e-9);
    }

    #[test]
    fn pitch_bend_center_is_zero() {
        let n = normalizer();
        // 14-bit center = 0x2000 = lsb=0x00, msb=0x40
        let event = n.normalize(&raw(&[0xE0, 0x00, 0x40])).unwrap();
        assert_eq!(event.kind, MidiEventKind::PitchBend);
        assert!(event.value.abs() < 1e-6);
    }

    #[test]
    fn pitch_bend_max_positive() {
        let n = normalizer();
        // 14-bit max = 0x3FFF = lsb=0x7F, msb=0x7F
        let event = n.normalize(&raw(&[0xE0, 0x7F, 0x7F])).unwrap();
        assert_eq!(event.kind, MidiEventKind::PitchBend);
        assert!(event.value > 0.99);
    }

    #[test]
    fn sysex_returns_none() {
        let n = normalizer();
        assert!(n.normalize(&raw(&[0xF0, 0x41, 0xF7])).is_none());
    }

    #[test]
    fn empty_message_returns_none() {
        let n = normalizer();
        assert!(n.normalize(&raw(&[])).is_none());
    }

    #[test]
    fn note_ids_are_unique() {
        let n = normalizer();
        let e1 = n.normalize(&raw(&[0x90, 60, 100])).unwrap();
        let e2 = n.normalize(&raw(&[0x90, 62, 80])).unwrap();
        assert_ne!(e1.note_id, e2.note_id);
    }

    #[test]
    fn channel_encoded_in_status_byte() {
        let n = normalizer();
        // Channel 5 = 0x95 for note-on channel 5
        let event = n.normalize(&raw(&[0x95, 60, 80])).unwrap();
        assert_eq!(event.channel.value(), 5);
    }

    #[test]
    fn truncated_note_on_returns_none() {
        let n = normalizer();
        // note-on with only 1 byte (no note number, no velocity)
        assert!(n.normalize(&raw(&[0x90])).is_none());
    }

    #[test]
    fn truncated_note_off_returns_none() {
        let n = normalizer();
        assert!(n.normalize(&raw(&[0x80, 60])).is_none());
    }

    #[test]
    fn normalizer_is_object_safe() {
        let n: Box<dyn MidiNormalizerPort> = Box::new(StandardMidiNormalizer::new());
        // SysEx → None, just confirm the dyn dispatch works
        let result = n.normalize(&RawMidiMessage::new(vec![0xF0], 0));
        assert!(result.is_none());
    }
}
