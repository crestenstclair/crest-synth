// path: src/real_time/boundary_message.rs

/// Discriminant for a [`BoundaryMessage`].
///
/// Describes what kind of message is crossing the real-time boundary so that
/// the receiver can decode the raw bytes in [`BoundaryMessage::payload`]
/// without any allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryMessageKind {
    /// A MIDI event encoded as raw bytes (status + up to two data bytes).
    MidiEvent,
    /// A parameter-change notification.  The payload encodes the parameter
    /// index (4 bytes, little-endian u32) followed by the new value (4 bytes,
    /// little-endian f32).
    ParameterChange,
    /// A transport command (play, stop, …).  The payload encodes the command
    /// as a single byte.
    TransportCommand,
}

/// A discrete message that crosses the real-time boundary via a ring buffer.
///
/// `BoundaryMessage` is a fixed-size value type that travels lock-free
/// between the non-RT and RT threads.  The payload is an owned `Vec<u8>`
/// because messages are constructed on the non-RT side (where allocation is
/// fine) and then sent through the ring buffer whole.  The RT receiver reads
/// the bytes without any further allocation.
///
/// # Examples
///
/// ```
/// use crest_synth::real_time::boundary_message::{BoundaryMessage, BoundaryMessageKind};
///
/// let msg = BoundaryMessage::new(BoundaryMessageKind::MidiEvent, vec![0x90, 0x3C, 0x7F], 1);
/// assert_eq!(msg.kind, BoundaryMessageKind::MidiEvent);
/// assert_eq!(msg.sequence_number, 1);
/// assert_eq!(msg.payload, &[0x90u8, 0x3C, 0x7F]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryMessage {
    /// What kind of message this is.
    pub kind: BoundaryMessageKind,
    /// Raw byte payload; format is determined by [`kind`](BoundaryMessage::kind).
    pub payload: Vec<u8>,
    /// Monotonically increasing counter assigned by the sender so the receiver
    /// can detect gaps or reorder.
    pub sequence_number: u64,
}

impl BoundaryMessage {
    /// Construct a new [`BoundaryMessage`].
    ///
    /// # Arguments
    ///
    /// * `kind`            – the discriminant describing how `payload` is encoded
    /// * `payload`         – raw bytes; allocated on the **non-RT** side only
    /// * `sequence_number` – monotonically increasing counter from the sender
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::real_time::boundary_message::{BoundaryMessage, BoundaryMessageKind};
    ///
    /// let msg = BoundaryMessage::new(BoundaryMessageKind::ParameterChange, vec![0, 0, 0, 1, 0, 0, 128, 63], 42);
    /// assert_eq!(msg.sequence_number, 42);
    /// ```
    pub fn new(kind: BoundaryMessageKind, payload: Vec<u8>, sequence_number: u64) -> Self {
        Self {
            kind,
            payload,
            sequence_number,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_midi_event() {
        let payload = vec![0x90u8, 0x3C, 0x64];
        let msg = BoundaryMessage::new(BoundaryMessageKind::MidiEvent, payload.clone(), 0);
        assert_eq!(msg.kind, BoundaryMessageKind::MidiEvent);
        assert_eq!(msg.payload, payload);
        assert_eq!(msg.sequence_number, 0);
    }

    #[test]
    fn round_trip_parameter_change() {
        // 4-byte index (1) + 4-byte f32 (0.5)
        let mut payload = vec![0u8; 8];
        payload[0..4].copy_from_slice(&1u32.to_le_bytes());
        payload[4..8].copy_from_slice(&0.5f32.to_le_bytes());
        let msg = BoundaryMessage::new(BoundaryMessageKind::ParameterChange, payload.clone(), 99);
        assert_eq!(msg.kind, BoundaryMessageKind::ParameterChange);
        assert_eq!(msg.payload, payload);
        assert_eq!(msg.sequence_number, 99);
    }

    #[test]
    fn round_trip_transport_command() {
        let msg = BoundaryMessage::new(BoundaryMessageKind::TransportCommand, vec![1], 7);
        assert_eq!(msg.kind, BoundaryMessageKind::TransportCommand);
        assert_eq!(msg.payload, vec![1]);
        assert_eq!(msg.sequence_number, 7);
    }

    #[test]
    fn clone_equality() {
        let original = BoundaryMessage::new(BoundaryMessageKind::MidiEvent, vec![0x80, 0x3C, 0], 3);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn sequence_numbers_are_stored() {
        let msg_a = BoundaryMessage::new(BoundaryMessageKind::MidiEvent, vec![], 1000);
        let msg_b = BoundaryMessage::new(BoundaryMessageKind::MidiEvent, vec![], 1001);
        assert!(msg_b.sequence_number > msg_a.sequence_number);
    }

    #[test]
    fn kind_copy() {
        // BoundaryMessageKind must be Copy so it can be used on the RT side
        // without cloning.
        let kind = BoundaryMessageKind::MidiEvent;
        let kind2 = kind; // copy
        assert_eq!(kind, kind2);
    }
}
