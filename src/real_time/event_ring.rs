// path: src/real_time/event_ring.rs

use std::cell::UnsafeCell;
use std::error::Error;
use std::fmt;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A message that crosses the real-time boundary between the UI/MIDI
/// thread (producer) and the audio thread (consumer).
///
/// `BoundaryMessage` variants are plain data — no heap-owning fields —
/// so pushing or popping a message never triggers an allocation or a
/// deallocation on either side of the boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoundaryMessage {
    /// A note-on event addressed to a MIDI channel.
    NoteOn { channel: u8, note: u8, velocity: u8 },
    /// A note-off event addressed to a MIDI channel.
    NoteOff { channel: u8, note: u8 },
    /// A discrete parameter change, identified by a stable numeric id.
    ParameterChange { parameter_id: u32, value: f32 },
}

/// Returned by [`EventRing::push`] when the ring has no free slot.
///
/// The producer is expected to treat this as backpressure: drop the
/// message, coalesce it with a pending one, or retry on the next UI
/// tick. The audio thread is never involved in resolving this error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingFull;

impl fmt::Display for RingFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "event ring is full")
    }
}

impl Error for RingFull {}

/// A fixed-capacity, single-producer/single-consumer lock-free ring
/// buffer carrying [`BoundaryMessage`] values across the real-time
/// boundary.
///
/// Exactly one thread may call [`EventRing::push`] (the UI/MIDI
/// thread) and exactly one thread may call [`EventRing::pop`] (the
/// audio thread). Both methods take `&self` so the ring can be shared
/// between the two threads behind an `Arc`, with each side calling
/// only its own half of the interface.
///
/// The backing storage is allocated exactly once, at construction
/// time, off the audio thread. After construction, `push` and `pop`
/// never allocate, never acquire a lock, and never block — they only
/// perform atomic loads/stores and a single element read or write.
pub struct EventRing {
    slots: Box<[UnsafeCell<MaybeUninit<BoundaryMessage>>]>,
    mask: usize,
    /// Next index the consumer will read from. Written only by the consumer.
    head: AtomicUsize,
    /// Next index the producer will write to. Written only by the producer.
    tail: AtomicUsize,
}

// SAFETY: `EventRing` is designed for exactly one producer thread and
// exactly one consumer thread operating concurrently. The `head`/`tail`
// atomics establish the happens-before edges that make the single
// `UnsafeCell` slot touched by each operation race-free: the producer
// only ever writes slots the consumer has already vacated (as observed
// via an Acquire load of `head`), and the consumer only ever reads
// slots the producer has already published (as observed via an
// Acquire load of `tail`).
unsafe impl Sync for EventRing {}

impl EventRing {
    /// Creates a new ring able to hold at least `capacity` messages.
    ///
    /// The requested capacity is rounded up to the next power of two
    /// (minimum 1) so that index wrapping can use a bitmask instead of
    /// a modulo. This is the only allocation `EventRing` ever performs;
    /// it must happen before the audio thread starts pulling from the
    /// ring.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1).next_power_of_two();
        let slots = (0..capacity)
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            slots,
            mask: capacity - 1,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// The number of messages the ring can hold at once.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Publishes `message` for the consumer to observe.
    ///
    /// Called only from the producer (UI/MIDI) thread. Returns
    /// `Err(RingFull)` without writing anything if the ring has no
    /// free slot; the caller decides how to handle backpressure.
    pub fn push(&self, message: BoundaryMessage) -> Result<(), RingFull> {
        let tail = self.tail.load(Ordering::Relaxed);
        // Acquire: synchronizes with the consumer's Release store to
        // `head`, so we see every slot it has already vacated.
        let head = self.head.load(Ordering::Acquire);

        if tail.wrapping_sub(head) >= self.slots.len() {
            return Err(RingFull);
        }

        let index = tail & self.mask;
        // SAFETY: this slot is not the one the consumer is currently
        // reading — the capacity check above guarantees the producer
        // is at least one full lap behind the consumer's next read —
        // and only the producer ever writes slots, so no other thread
        // touches this `UnsafeCell` concurrently.
        unsafe {
            (*self.slots[index].get()).write(message);
        }

        // Release: publishes the write above to the consumer's next
        // Acquire load of `tail`.
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Retrieves the next message, if one has been published.
    ///
    /// Called only from the consumer (audio) thread. Never allocates,
    /// never blocks, and returns `None` immediately if the ring is
    /// empty rather than waiting for a message to arrive.
    pub fn pop(&self) -> Option<BoundaryMessage> {
        let head = self.head.load(Ordering::Relaxed);
        // Acquire: synchronizes with the producer's Release store to
        // `tail`, so we see the fully-written slot, not a torn write.
        let tail = self.tail.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        let index = head & self.mask;
        // SAFETY: `head != tail` means the producer has published this
        // slot (its Release store to `tail` happened-before this
        // Acquire load), so the value is fully initialized. Only the
        // consumer ever reads or retires slots, so no other thread
        // touches this `UnsafeCell` concurrently.
        let message = unsafe { (*self.slots[index].get()).assume_init_read() };

        // Release: tells the producer this slot is free again.
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(message)
    }
}

impl Drop for EventRing {
    fn drop(&mut self) {
        // Drain any messages still in flight so their destructors run.
        // Safe to use plain loads here: `Drop::drop` runs with
        // exclusive (`&mut self`) access, after both threads have
        // stopped touching the ring.
        let mut head = *self.head.get_mut();
        let tail = *self.tail.get_mut();
        while head != tail {
            let index = head & self.mask;
            unsafe {
                (*self.slots[index].get()).assume_init_drop();
            }
            head = head.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn pop_on_empty_ring_returns_none() {
        let ring = EventRing::new(4);
        assert_eq!(ring.pop(), None);
    }

    #[test]
    fn push_then_pop_round_trips_a_message() {
        let ring = EventRing::new(4);
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
    fn capacity_rounds_up_to_a_power_of_two() {
        let ring = EventRing::new(5);
        assert_eq!(ring.capacity(), 8);
    }

    #[test]
    fn push_fails_once_the_ring_is_full() {
        let ring = EventRing::new(2);
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
        let ring = EventRing::new(2);
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
        let ring = Arc::new(EventRing::new(64));

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
