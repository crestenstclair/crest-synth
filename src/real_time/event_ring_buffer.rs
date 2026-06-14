// path: src/real_time/event_ring_buffer.rs

use crate::real_time::boundary_message::BoundaryMessage;

/// Error returned by [`EventRingBuffer::push`] when the ring buffer is full.
///
/// The caller must decide how to handle backpressure: log the drop, retry
/// later, or widen the buffer capacity.  This method must **never** block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Full;

impl std::fmt::Display for Full {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EventRingBuffer is full")
    }
}

impl std::error::Error for Full {}

/// SPSC lock-free ring buffer for crossing the non-realtime / realtime boundary.
///
/// `EventRingBuffer` wraps an [`rtrb`] producer/consumer pair and exposes a
/// two-method contract:
///
/// - [`push`](EventRingBuffer::push) — called from the **non-realtime** thread
///   (e.g. UI or MIDI input thread) to enqueue a [`BoundaryMessage`].
/// - [`pop`](EventRingBuffer::pop) — called from the **realtime audio** thread
///   to dequeue the next message without blocking or acquiring any mutex.
///
/// # Audio-thread safety
///
/// `pop` is wait-free on the consumer side: [`rtrb::Consumer::pop`] never
/// blocks, never acquires a mutex, and never calls the system allocator.
/// Receiving a `BoundaryMessage` moves heap-allocated payload bytes from the
/// producer to the consumer; the consumer should process those bytes
/// synchronously and hand the message off to a [`DeferredDeallocator`] so that
/// the actual `free()` never runs on the audio thread.
///
/// # Capacity
///
/// The ring buffer is created with a fixed capacity supplied at construction
/// time. If the producer overflows the capacity, [`push`](EventRingBuffer::push)
/// returns [`Err(Full)`] so the caller can decide how to handle backpressure.
///
/// # Examples
///
/// ```
/// use crest_synth::real_time::boundary_message::{BoundaryMessage, BoundaryMessageKind};
/// use crest_synth::real_time::event_ring_buffer::EventRingBuffer;
///
/// let mut buf = EventRingBuffer::new(16);
/// let msg = BoundaryMessage::new(BoundaryMessageKind::TransportCommand, vec![1], 0);
/// buf.push(msg).expect("buffer not full");
/// let received = buf.pop().expect("buffer not empty");
/// assert_eq!(received.kind, BoundaryMessageKind::TransportCommand);
/// ```
pub struct EventRingBuffer {
    producer: rtrb::Producer<BoundaryMessage>,
    consumer: rtrb::Consumer<BoundaryMessage>,
}

impl EventRingBuffer {
    /// Create a new ring buffer with the given `capacity` (number of messages).
    ///
    /// `capacity` should be a power of two for optimal performance; `rtrb`
    /// rounds up internally if it isn't.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::real_time::event_ring_buffer::EventRingBuffer;
    ///
    /// let buf = EventRingBuffer::new(64);
    /// drop(buf);
    /// ```
    pub fn new(capacity: usize) -> Self {
        let (producer, consumer) = rtrb::RingBuffer::new(capacity);
        Self { producer, consumer }
    }

    /// Push a [`BoundaryMessage`] from the **non-realtime** thread.
    ///
    /// Returns `Err(Full)` without blocking if the buffer has no room.
    /// The message is moved into the ring buffer; no extra allocation occurs.
    ///
    /// # Errors
    ///
    /// - [`Full`] — the ring buffer is at capacity. The caller should decide
    ///   how to handle this (log, drop, or retry) — never block.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::real_time::boundary_message::{BoundaryMessage, BoundaryMessageKind};
    /// use crest_synth::real_time::event_ring_buffer::{EventRingBuffer, Full};
    ///
    /// let mut buf = EventRingBuffer::new(1);
    /// let msg = BoundaryMessage::new(BoundaryMessageKind::TransportCommand, vec![0], 0);
    /// assert!(buf.push(msg).is_ok());
    /// let msg2 = BoundaryMessage::new(BoundaryMessageKind::TransportCommand, vec![0], 1);
    /// assert_eq!(buf.push(msg2), Err(Full));
    /// ```
    pub fn push(&mut self, msg: BoundaryMessage) -> Result<(), Full> {
        self.producer.push(msg).map_err(|_| Full)
    }

    /// Pop the next [`BoundaryMessage`] from the **realtime audio** thread.
    ///
    /// Returns `None` if the buffer is empty. This operation is **wait-free**
    /// and does **not** acquire a mutex — safe to call on the audio thread.
    ///
    /// The returned `BoundaryMessage` owns heap-allocated payload bytes. To
    /// avoid freeing that memory on the audio thread, hand the message to a
    /// `DeferredDeallocator` after processing its bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::real_time::boundary_message::{BoundaryMessage, BoundaryMessageKind};
    /// use crest_synth::real_time::event_ring_buffer::EventRingBuffer;
    ///
    /// let mut buf = EventRingBuffer::new(4);
    /// assert_eq!(buf.pop(), None);
    /// buf.push(BoundaryMessage::new(BoundaryMessageKind::MidiEvent, vec![0x90, 0x3C, 0x64], 0)).unwrap();
    /// let msg = buf.pop().unwrap();
    /// assert_eq!(msg.payload, &[0x90u8, 0x3C, 0x64]);
    /// assert_eq!(buf.pop(), None);
    /// ```
    pub fn pop(&mut self) -> Option<BoundaryMessage> {
        self.consumer.pop().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::real_time::boundary_message::{BoundaryMessage, BoundaryMessageKind};

    fn midi_msg(seq: u64) -> BoundaryMessage {
        BoundaryMessage::new(
            BoundaryMessageKind::MidiEvent,
            vec![0x90u8, 0x3C, 0x64],
            seq,
        )
    }

    fn param_msg(seq: u64) -> BoundaryMessage {
        let mut payload = vec![0u8; 8];
        payload[0..4].copy_from_slice(&1u32.to_le_bytes());
        payload[4..8].copy_from_slice(&0.5f32.to_le_bytes());
        BoundaryMessage::new(BoundaryMessageKind::ParameterChange, payload, seq)
    }

    fn transport_msg(cmd: u8, seq: u64) -> BoundaryMessage {
        BoundaryMessage::new(BoundaryMessageKind::TransportCommand, vec![cmd], seq)
    }

    // ── push / pop round-trip ─────────────────────────────────────────────────

    #[test]
    fn push_then_pop_returns_same_message() {
        let mut buf = EventRingBuffer::new(4);
        let msg = midi_msg(0);
        assert!(buf.push(msg.clone()).is_ok());
        let got = buf.pop().expect("should have a message");
        assert_eq!(got, msg);
    }

    #[test]
    fn pop_on_empty_buffer_returns_none() {
        let mut buf = EventRingBuffer::new(4);
        assert_eq!(buf.pop(), None);
    }

    #[test]
    fn messages_arrive_in_fifo_order() {
        let mut buf = EventRingBuffer::new(8);
        let a = midi_msg(1);
        let b = param_msg(2);
        let c = transport_msg(1, 3);

        buf.push(a.clone()).unwrap();
        buf.push(b.clone()).unwrap();
        buf.push(c.clone()).unwrap();

        assert_eq!(buf.pop(), Some(a));
        assert_eq!(buf.pop(), Some(b));
        assert_eq!(buf.pop(), Some(c));
        assert_eq!(buf.pop(), None);
    }

    // ── capacity / overflow ───────────────────────────────────────────────────

    #[test]
    fn push_returns_full_when_at_capacity() {
        let mut buf = EventRingBuffer::new(2);
        buf.push(midi_msg(0)).unwrap();
        buf.push(midi_msg(1)).unwrap();
        // Third push must fail — buffer is full.
        assert_eq!(buf.push(midi_msg(2)), Err(Full));
    }

    #[test]
    fn after_pop_push_succeeds_again() {
        let mut buf = EventRingBuffer::new(1);
        buf.push(midi_msg(0)).unwrap();
        buf.pop(); // free a slot
        assert!(buf.push(midi_msg(1)).is_ok());
    }

    #[test]
    fn wrap_around_preserves_fifo() {
        // Fill, drain, fill again to exercise ring-buffer wrap.
        let mut buf = EventRingBuffer::new(4);
        for i in 0..4u64 {
            buf.push(midi_msg(i)).unwrap();
        }
        for i in 0..4u64 {
            let got = buf.pop().expect("message present");
            assert_eq!(got.sequence_number, i);
        }
        for i in 4..8u64 {
            buf.push(midi_msg(i)).unwrap();
        }
        for i in 4..8u64 {
            let got = buf.pop().expect("message present");
            assert_eq!(got.sequence_number, i);
        }
        assert_eq!(buf.pop(), None);
    }

    // ── message kinds ─────────────────────────────────────────────────────────

    #[test]
    fn midi_event_payload_preserved() {
        let mut buf = EventRingBuffer::new(4);
        buf.push(midi_msg(0)).unwrap();
        let got = buf.pop().unwrap();
        assert_eq!(got.kind, BoundaryMessageKind::MidiEvent);
        assert_eq!(got.payload, &[0x90u8, 0x3C, 0x64]);
        assert_eq!(got.sequence_number, 0);
    }

    #[test]
    fn parameter_change_payload_preserved() {
        let mut buf = EventRingBuffer::new(4);
        let msg = param_msg(99);
        buf.push(msg.clone()).unwrap();
        let got = buf.pop().unwrap();
        assert_eq!(got.kind, BoundaryMessageKind::ParameterChange);
        assert_eq!(got.payload, msg.payload);
        assert_eq!(got.sequence_number, 99);
    }

    #[test]
    fn transport_command_payload_preserved() {
        let mut buf = EventRingBuffer::new(4);
        buf.push(transport_msg(1, 7)).unwrap();
        let got = buf.pop().unwrap();
        assert_eq!(got.kind, BoundaryMessageKind::TransportCommand);
        assert_eq!(got.payload, &[1u8]);
        assert_eq!(got.sequence_number, 7);
    }

    // ── Full error type ───────────────────────────────────────────────────────

    #[test]
    fn full_displays_non_empty_message() {
        let err = Full;
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn full_implements_std_error() {
        let err: &dyn std::error::Error = &Full;
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn full_is_copy() {
        let a = Full;
        let b = a;
        assert_eq!(a, b);
    }
}
