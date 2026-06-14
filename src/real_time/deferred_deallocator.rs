// path: src/real_time/deferred_deallocator.rs
//
// DeferredDeallocator — basedrop-style deferred memory reclamation.
//
// The audio thread calls `retire` to hand ownership of an `Arc<T>` to the
// deallocator without freeing it.  A background thread calls `collect` to
// drain the queue and drop the arcs (which may free the underlying
// allocation).  This guarantees that `free()` never runs on the audio thread.
//
// Design
// ------
//   • `retire` is lock-free and allocation-free:
//       - The incoming `Arc<T>` is coerced to `Arc<dyn Any + Send + Sync>`,
//         a fat pointer (2 words) stored directly in the ring buffer slot.
//         No `Box::new` or heap allocation occurs.
//       - `rtrb::Producer::push` is a single atomic operation; it never blocks.
//   • `collect` may call `free()` (dropping the last Arc) but is only ever
//     called from a non-real-time background thread.
//   • The two halves are split by `rtrb::RingBuffer` into a `Producer`
//     (audio-thread side) and a `Consumer` (collector-thread side).

use std::{any::Any, sync::Arc};

use rtrb::{Consumer, Producer, RingBuffer};

/// Capacity of the retire queue (number of `Arc` slots).
///
/// If the audio thread retires more than this many objects between consecutive
/// `collect` calls, `retire` will silently drop the excess rather than
/// block or allocate.  Size conservatively: in practice a single audio
/// callback retires at most a handful of voices.
const QUEUE_CAPACITY: usize = 256;

/// Type-erased retired `Arc`.  Stored as a fat pointer (2 words) — no
/// additional heap allocation.
type Retired = Arc<dyn Any + Send + Sync>;

/// Audio-thread handle.  Accepts retired `Arc<T>` values without blocking
/// and without heap allocation.
pub struct RetireHandle {
    producer: Producer<Retired>,
}

impl RetireHandle {
    /// Hand an `Arc<T>` to the deallocator.
    ///
    /// This operation is **lock-free and allocation-free**:
    /// - Coercing to `Arc<dyn Any + Send + Sync>` is a fat-pointer cast (no alloc).
    /// - `push` is a single atomic store onto the ring buffer; never blocks.
    ///
    /// If the queue is full (the background thread has fallen behind by
    /// more than `QUEUE_CAPACITY` slots) the arc is silently dropped here
    /// instead — still correct, but `free()` may run on the audio thread
    /// in that rare overload case.
    pub fn retire<T: Any + Send + Sync + 'static>(&mut self, value: Arc<T>) {
        let erased: Retired = value; // fat-pointer coercion, no allocation
        let _ = self.producer.push(erased); // single atomic op
    }
}

/// Background-thread handle.  Drains and drops retired values.
pub struct CollectHandle {
    consumer: Consumer<Retired>,
}

impl CollectHandle {
    /// Drain all pending retired values and drop them.
    ///
    /// Call this periodically from a background (non-audio) thread.
    /// Dropping the `Arc<dyn …>` may call `free()` on the underlying
    /// allocation — safe here because we are not on the audio thread.
    pub fn collect(&mut self) {
        while let Ok(retired) = self.consumer.pop() {
            drop(retired); // may call free(); safe on background thread
        }
    }
}

/// Create a linked (`RetireHandle`, `CollectHandle`) pair.
///
/// `retire_handle` goes to the audio thread; `collect_handle` goes to a
/// background collector thread or is polled from a timer.
///
/// # Example
///
/// ```
/// use crest_synth::real_time::deferred_deallocator::deferred_deallocator;
/// use std::sync::Arc;
///
/// let (mut retire, mut collect) = deferred_deallocator();
/// let data: Arc<Vec<f32>> = Arc::new(vec![0.0_f32; 1024]);
/// retire.retire(data);
/// collect.collect(); // drops the Arc on the background thread
/// ```
pub fn deferred_deallocator() -> (RetireHandle, CollectHandle) {
    let (producer, consumer) = RingBuffer::<Retired>::new(QUEUE_CAPACITY);
    (RetireHandle { producer }, CollectHandle { consumer })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Helper: an object that records when it is dropped.
    struct DropRecorder {
        dropped: Arc<Mutex<bool>>,
    }

    impl Drop for DropRecorder {
        fn drop(&mut self) {
            *self.dropped.lock().unwrap() = true;
        }
    }

    #[test]
    fn collect_drops_retired_value() {
        let (mut retire, mut collect) = deferred_deallocator();

        let was_dropped = Arc::new(Mutex::new(false));
        let recorder = Arc::new(DropRecorder {
            dropped: Arc::clone(&was_dropped),
        });

        // Drop the local `Arc` — only the retired one remains.
        retire.retire(Arc::clone(&recorder));
        drop(recorder);

        // Not yet dropped — still in the queue.
        assert!(!*was_dropped.lock().unwrap());

        // collect() drains the queue; the last Arc drops here.
        collect.collect();
        assert!(*was_dropped.lock().unwrap());
    }

    #[test]
    fn multiple_retires_all_collected() {
        let (mut retire, mut collect) = deferred_deallocator();

        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        struct Counter(Arc<std::sync::atomic::AtomicUsize>);
        impl Drop for Counter {
            fn drop(&mut self) {
                self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }

        for _ in 0..8 {
            retire.retire(Arc::new(Counter(Arc::clone(&count))));
        }

        assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 0);
        collect.collect();
        assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 8);
    }

    #[test]
    fn collect_on_empty_queue_is_noop() {
        let (_retire, mut collect) = deferred_deallocator();
        collect.collect(); // must not panic
    }

    #[test]
    fn retire_returns_without_blocking() {
        // Sanity: retire 100 arcs, none should block.
        let (mut retire, mut collect) = deferred_deallocator();
        for i in 0_u64..100 {
            retire.retire(Arc::new(i));
        }
        collect.collect();
    }
}
