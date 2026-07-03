// path: src/real_time/deferred_deallocator.rs

//! Deferred deallocation port for the real-time boundary.
//!
//! The audio thread must never free memory itself: freeing is allocation's
//! twin and both can block unboundedly (the allocator may take a lock,
//! coalesce free lists, or make a syscall). Instead the audio thread
//! *retires* an already-heap-allocated value into a bounded, lock-free ring
//! buffer, and a non-real-time background thread later *collects* the ring
//! and drops the retired values, actually freeing them off the RT path.
//!
//! [`Retire`] is the narrow interface the audio thread depends on (push
//! only). [`Collect`] is the narrow interface the background thread depends
//! on (drain only). Splitting them (Interface Segregation) makes it
//! impossible for RT code to accidentally call the allocating/blocking
//! `collect` path, and vice versa.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A type-erased, already-heap-allocated value waiting to be freed by a
/// background thread.
///
/// Constructing a `Retired` never allocates: it takes ownership of a `Box<T>`
/// the caller already allocated (e.g. when a voice or patch was first
/// created) and performs only pointer bookkeeping via an unsizing coercion.
pub struct Retired(Box<dyn Send + 'static>);

impl Retired {
    /// Wrap an already-boxed value for deferred freeing.
    ///
    /// This does not allocate: ownership of the existing `Box<T>` moves into
    /// the returned `Retired`.
    pub fn new<T: Send + 'static>(value: Box<T>) -> Self {
        Retired(value)
    }
}

impl Drop for Retired {
    /// Explicitly touches field 0 so the compiler recognizes it as read
    /// (silencing `dead_code`) rather than merely dropped implicitly: the
    /// whole point of this type is that the background collector's thread
    /// reaches this point and frees the boxed payload here, off the audio
    /// thread. The field then finishes dropping normally afterward.
    fn drop(&mut self) {
        let _payload: &(dyn Send + 'static) = &*self.0;
    }
}

/// Port depended on by real-time code: hand off a retired allocation.
///
/// Implementations must guarantee `retire` never allocates heap memory,
/// acquires a mutex or blocking lock, or performs blocking I/O.
pub trait Retire {
    /// Hand off `allocation` to be freed later, off the audio thread.
    fn retire(&mut self, allocation: Retired);
}

/// Port depended on by the background reclaimer: drain retired allocations.
///
/// Implementations may allocate, lock, or block internally. Never call this
/// from the audio thread.
pub trait Collect {
    /// Drop every retired allocation currently queued, actually freeing
    /// them, and return how many were freed.
    fn collect(&mut self) -> u32;
}

/// One slot in the ring buffer.
///
/// `UnsafeCell` is required for interior mutability across the
/// producer/consumer split; it unconditionally opts the slot out of `Sync`,
/// which we restore below with a manual, access-pattern-justified impl.
struct Slot(UnsafeCell<Option<Retired>>);

impl Slot {
    fn empty() -> Self {
        Slot(UnsafeCell::new(None))
    }
}

// Safety: `Slot` is only ever written by the single producer half
// (`Retirer`) and only ever read/cleared by the single consumer half
// (`Collector`) of a given ring, and the two sides never touch the same
// index concurrently (guarded by the head/tail atomics below). That
// single-producer/single-consumer discipline makes shared access to the
// `RingState` safe despite the `UnsafeCell`.
unsafe impl Sync for Slot {}

/// Shared state behind a producer/consumer pair. `capacity` slots are
/// allocated once, up front, when the ring is created (not on the audio
/// thread) - `retire` and `collect` never grow or shrink this buffer.
struct RingState {
    slots: Box<[Slot]>,
    capacity: usize,
    /// Next index the consumer will read from. Written only by the
    /// consumer, read by both sides.
    head: AtomicUsize,
    /// Next index the producer will write to. Written only by the
    /// producer, read by both sides.
    tail: AtomicUsize,
}

/// Real-time-side handle: the only operation available is [`Retire::retire`].
pub struct Retirer {
    state: Arc<RingState>,
}

/// Background-side handle: the only operation available is
/// [`Collect::collect`].
pub struct Collector {
    state: Arc<RingState>,
}

// Safety: a `Retirer` is created once (off the audio thread) and then moved
// onto the audio thread; it never shares `&Retirer` across threads, so
// `Send` is all it needs.
unsafe impl Send for Retirer {}
// Safety: symmetric argument for the background thread.
unsafe impl Send for Collector {}

/// Build a deferred-deallocation ring with room for `capacity - 1` retired
/// allocations in flight at once (one slot is always kept empty to
/// distinguish "full" from "empty" without extra bookkeeping).
///
/// `capacity` must be at least 2. This allocates the backing buffer once,
/// up front - call it during setup, never from the audio thread.
pub fn deferred_deallocator(capacity: usize) -> (Retirer, Collector) {
    assert!(
        capacity >= 2,
        "deferred_deallocator requires capacity >= 2, got {capacity}"
    );

    let mut slots = Vec::with_capacity(capacity);
    for _ in 0..capacity {
        slots.push(Slot::empty());
    }

    let state = Arc::new(RingState {
        slots: slots.into_boxed_slice(),
        capacity,
        head: AtomicUsize::new(0),
        tail: AtomicUsize::new(0),
    });

    (
        Retirer {
            state: Arc::clone(&state),
        },
        Collector { state },
    )
}

impl Retire for Retirer {
    fn retire(&mut self, allocation: Retired) {
        let tail = self.state.tail.load(Ordering::Relaxed);
        let head = self.state.head.load(Ordering::Acquire);
        let next = (tail + 1) % self.state.capacity;

        if next == head {
            // The ring is full: the background collector has fallen behind.
            // We cannot drop `allocation` here (that would run its
            // destructor, i.e. free memory, on the audio thread) and we
            // cannot block waiting for room. Deliberately leak it instead -
            // a bounded, rare memory leak is preferable to a real-time
            // deadline violation.
            std::mem::forget(allocation);
            return;
        }

        // Safety: only the producer ever writes to `slots[tail]`, and the
        // `next == head` check above guarantees the consumer has already
        // vacated this slot (or never occupied it), so there is no
        // concurrent access to this index.
        unsafe {
            *self.state.slots[tail].0.get() = Some(allocation);
        }
        self.state.tail.store(next, Ordering::Release);
    }
}

impl Collect for Collector {
    fn collect(&mut self) -> u32 {
        let mut freed: u32 = 0;
        loop {
            let head = self.state.head.load(Ordering::Relaxed);
            let tail = self.state.tail.load(Ordering::Acquire);
            if head == tail {
                break;
            }

            // Safety: only the consumer ever reads/clears `slots[head]`, and
            // `head != tail` guarantees the producer has already published a
            // value at this index.
            let item = unsafe { (*self.state.slots[head].0.get()).take() };
            drop(item); // Actually frees the retired allocation.
            freed += 1;

            let next = (head + 1) % self.state.capacity;
            self.state.head.store(next, Ordering::Release);
        }
        freed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize as StdAtomicUsize, Ordering as StdOrdering};

    struct DropCounter(Arc<StdAtomicUsize>);

    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.0.fetch_add(1, StdOrdering::SeqCst);
        }
    }

    #[test]
    fn collect_on_empty_ring_returns_zero() {
        let (_retirer, mut collector) = deferred_deallocator(4);
        assert_eq!(collector.collect(), 0);
    }

    #[test]
    fn retire_defers_the_drop_until_collect_runs() {
        let counter = Arc::new(StdAtomicUsize::new(0));
        let (mut retirer, mut collector) = deferred_deallocator(4);

        let item = Box::new(DropCounter(Arc::clone(&counter)));
        retirer.retire(Retired::new(item));
        assert_eq!(
            counter.load(StdOrdering::SeqCst),
            0,
            "retire must not run the destructor on the retiring thread"
        );

        let freed = collector.collect();
        assert_eq!(freed, 1);
        assert_eq!(counter.load(StdOrdering::SeqCst), 1);
    }

    #[test]
    fn collect_drains_every_retired_allocation_in_order() {
        let counter = Arc::new(StdAtomicUsize::new(0));
        let (mut retirer, mut collector) = deferred_deallocator(8);

        for _ in 0..5 {
            retirer.retire(Retired::new(Box::new(DropCounter(Arc::clone(&counter)))));
        }

        let freed = collector.collect();
        assert_eq!(freed, 5);
        assert_eq!(counter.load(StdOrdering::SeqCst), 5);

        // A second collect on a now-empty ring must be a no-op.
        assert_eq!(collector.collect(), 0);
    }

    #[test]
    fn retire_on_a_full_ring_leaks_instead_of_dropping_on_the_caller() {
        let counter = Arc::new(StdAtomicUsize::new(0));
        // capacity 2 => exactly one usable slot before "full".
        let (mut retirer, mut collector) = deferred_deallocator(2);

        retirer.retire(Retired::new(Box::new(DropCounter(Arc::clone(&counter)))));
        // Ring is now full; this second retire must not drop its payload
        // on this (simulated audio) thread.
        retirer.retire(Retired::new(Box::new(DropCounter(Arc::clone(&counter)))));
        assert_eq!(
            counter.load(StdOrdering::SeqCst),
            0,
            "a full ring must leak, not drop, on the retiring thread"
        );

        let freed = collector.collect();
        assert_eq!(freed, 1, "only the accepted allocation is collectible");
        assert_eq!(counter.load(StdOrdering::SeqCst), 1);
    }

    #[test]
    fn ring_can_be_reused_after_draining() {
        let counter = Arc::new(StdAtomicUsize::new(0));
        let (mut retirer, mut collector) = deferred_deallocator(3);

        for round in 0..3 {
            for _ in 0..2 {
                retirer.retire(Retired::new(Box::new(DropCounter(Arc::clone(&counter)))));
            }
            let freed = collector.collect();
            assert_eq!(freed, 2, "round {round} should free exactly 2");
        }

        assert_eq!(counter.load(StdOrdering::SeqCst), 6);
    }
}
