// path: src/real_time/rtrb_event_ring.rs

//! Adapter: `RtrbEventRing`.
//!
//! Implements the real-time event boundary (`port.RealTime.EventRing`)
//! on top of the `rtrb` crate's single-producer/single-consumer ring
//! buffer, as an alternative backend to the hand-rolled atomics in
//! [`crate::real_time::event_ring::EventRing`]. Both types expose the
//! same `push`/`pop` surface over the same [`BoundaryMessage`] and
//! [`RingFull`] types, so they are substitutable wherever the real-time
//! event boundary is expected (Liskov substitution) — callers depend on
//! that shared shape, never on which backend is wired up.
//!
//! `rtrb::Producer::push` and `rtrb::Consumer::pop` never allocate,
//! never take a lock, and never block once the ring has been
//! constructed, so both operations remain legal to call from the audio
//! thread.

use std::cell::UnsafeCell;

use rtrb::{Consumer, Producer, PushError, RingBuffer};

pub use crate::real_time::event_ring::{BoundaryMessage, RingFull};

/// A fixed-capacity SPSC ring buffer carrying [`BoundaryMessage`] values
/// across the real-time boundary, backed by the `rtrb` crate.
///
/// Exactly one thread may call [`RtrbEventRing::push`] (the UI/MIDI
/// thread) and exactly one thread may call [`RtrbEventRing::pop`] (the
/// audio thread). Both methods take `&self` — matching the port
/// contract — so the ring can be shared between the two threads behind
/// an `Arc`. `rtrb::Producer` and `rtrb::Consumer` normally require
/// `&mut self` for `push`/`pop`, so each half is held behind its own
/// `UnsafeCell` to grant the interior mutability the port contract
/// needs; the SPSC discipline documented below is what makes that
/// sound.
pub struct RtrbEventRing {
    producer: UnsafeCell<Producer<BoundaryMessage>>,
    consumer: UnsafeCell<Consumer<BoundaryMessage>>,
    capacity: usize,
}

// SAFETY: `RtrbEventRing` is designed for exactly one producer thread
// and exactly one consumer thread operating concurrently. The producer
// `UnsafeCell` is touched only by `push` (called only from the UI/MIDI
// thread) and the consumer `UnsafeCell` is touched only by `pop`
// (called only from the audio thread), so neither cell is ever
// accessed by more than one thread. `rtrb::Producer`/`rtrb::Consumer`
// internally synchronize the shared head/tail indices with atomics, so
// there is no data race on the underlying storage either.
unsafe impl Sync for RtrbEventRing {}

impl RtrbEventRing {
    /// Creates a new ring able to hold at least `capacity` messages
    /// (minimum 1).
    ///
    /// This allocates the backing storage exactly once, at construction
    /// time, off the audio thread. After construction, `push` and `pop`
    /// never allocate, never acquire a lock, and never block.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        let (producer, consumer) = RingBuffer::<BoundaryMessage>::new(capacity);
        Self {
            producer: UnsafeCell::new(producer),
            consumer: UnsafeCell::new(consumer),
            capacity,
        }
    }

    /// The number of messages the ring can hold at once.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Publishes `message` for the consumer to observe.
    ///
    /// Called only from the producer (UI/MIDI) thread. Returns
    /// `Err(RingFull)` without writing anything if the ring has no free
    /// slot; the caller decides how to handle backpressure.
    pub fn push(&self, message: BoundaryMessage) -> Result<(), RingFull> {
        // SAFETY: only the producer thread ever dereferences this cell
        // (see the struct- and impl-level safety notes above).
        let producer = unsafe { &mut *self.producer.get() };
        match producer.push(message) {
            Ok(()) => Ok(()),
            Err(PushError::Full(_)) => Err(RingFull),
        }
    }

    /// Retrieves the next message, if one has been published.
    ///
    /// Called only from the consumer (audio) thread. Never allocates,
    /// never blocks, and returns `None` immediately if the ring is
    /// empty rather than waiting for a message to arrive.
    pub fn pop(&self) -> Option<BoundaryMessage> {
        // SAFETY: only the consumer thread ever dereferences this cell
        // (see the struct- and impl-level safety notes above).
        let consumer = unsafe { &mut *self.consumer.get() };
        consumer.pop().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn pop_on_empty_ring_returns_none() {
        let ring = RtrbEventRing::new(4);
        assert_eq!(ring.pop(), None);
    }

    #[test]
    fn push_then_pop_round_trips_a_message() {
        let ring = RtrbEventRing::new(4);
        let message = BoundaryMessage::NoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
        };
        ring.push(message).expect("ring has room");
        assert_eq!(ring.pop(), Some(message));
        assert_eq!(ring.pop(), None);
    }

    #[test]
    fn capacity_reports_the_requested_size() {
        let ring = RtrbEventRing::new(8);
        assert_eq!(ring.capacity(), 8);
    }

    #[test]
    fn push_fails_once_the_ring_is_full() {
        let ring = RtrbEventRing::new(2);
        let msg = BoundaryMessage::ParameterChange {
            parameter_id: 1,
            value: 0.5,
        };
        assert!(ring.push(msg).is_ok());
        assert!(ring.push(msg).is_ok());
        assert_eq!(ring.push(msg), Err(RingFull));
    }

    #[test]
    fn wraps_around_after_repeated_push_pop_cycles() {
        let ring = RtrbEventRing::new(2);
        for i in 0..10u8 {
            let msg = BoundaryMessage::NoteOff {
                channel: 0,
                note: i,
            };
            ring.push(msg).expect("ring has room after draining");
            assert_eq!(
                ring.pop(),
                Some(BoundaryMessage::NoteOff {
                    channel: 0,
                    note: i
                })
            );
        }
    }

    #[test]
    fn preserves_fifo_order_across_real_producer_and_consumer_threads() {
        const COUNT: u32 = 10_000;
        let ring = Arc::new(RtrbEventRing::new(64));

        let producer_ring = Arc::clone(&ring);
        let producer = thread::spawn(move || {
            let mut sent = 0u32;
            while sent < COUNT {
                let msg = BoundaryMessage::ParameterChange {
                    parameter_id: sent,
                    value: sent as f32,
                };
                if producer_ring.push(msg).is_ok() {
                    sent += 1;
                } else {
                    thread::yield_now();
                }
            }
        });

        let consumer_ring = Arc::clone(&ring);
        let consumer = thread::spawn(move || {
            let mut received = 0u32;
            while received < COUNT {
                match consumer_ring.pop() {
                    Some(BoundaryMessage::ParameterChange { parameter_id, .. }) => {
                        assert_eq!(parameter_id, received);
                        received += 1;
                    }
                    Some(_) => panic!("unexpected message variant"),
                    None => thread::yield_now(),
                }
            }
        });

        producer.join().expect("producer thread panicked");
        consumer.join().expect("consumer thread panicked");
    }
}
