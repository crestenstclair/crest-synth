// path: src/real_time/basedrop_deferred_deallocator.rs

//! Adapter: `basedrop`-backed implementation of
//! `port.RealTime.DeferredDeallocator`.
//!
//! [`crate::real_time::deferred_deallocator`] already defines the port
//! (the [`Retire`]/[`Collect`] traits and the [`Retired`] domain type) plus
//! a hand-rolled SPSC-ring reference implementation. This module provides
//! an alternative adapter that delegates the actual deferred-free
//! bookkeeping to the [`basedrop`] crate's [`basedrop::Collector`], which
//! is purpose-built for exactly this real-time-audio reclamation problem.
//!
//! Split responsibilities (Interface Segregation), same as the reference
//! adapter:
//! - [`BasedropRetirer`] implements only [`Retire`] and is the handle given
//!   to the real-time audio thread. `retire` hands the already-boxed
//!   [`Retired`] value to `basedrop`'s collector handle; `basedrop` enqueues
//!   it onto its internal garbage list without running its destructor,
//!   so no memory is freed on the calling thread.
//! - [`BasedropCollector`] implements only [`Collect`] and is the handle
//!   given to a background thread. `collect` asks the underlying
//!   `basedrop::Collector` to actually drop every queued item (this is
//!   where allocation/locking/blocking may occur) and reports how many
//!   allocations were reclaimed.
//!
//! Both handles are constructed together by [`basedrop_deferred_deallocator`]
//! (dependency injection over `new`-ing collaborators internally) so callers
//! can hand `BasedropRetirer` to the audio thread and `BasedropCollector` to
//! the housekeeping thread without either side depending on a concrete type
//! it doesn't need.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use basedrop::{Collector, Handle, Owned};

use crate::real_time::deferred_deallocator::{Collect, Retire, Retired};

/// Real-time-side handle: the only operation available is [`Retire::retire`].
///
/// Cloning the underlying `basedrop::Handle` and calling `add_to_collector`
/// does not free the retired value's memory - `basedrop` only links it into
/// a pending list for a later, off-thread `collect`. This mirrors the
/// architectural invariant that memory retired by the audio thread is freed
/// via the deferred deallocator, never on the audio thread itself.
pub struct BasedropRetirer {
    handle: Handle,
    pending: Arc<AtomicU32>,
}

/// Background-side handle: the only operation available is
/// [`Collect::collect`].
pub struct BasedropCollector {
    collector: Collector,
    pending: Arc<AtomicU32>,
}

/// Build a `basedrop`-backed deferred-deallocation pair.
///
/// Constructs the shared `basedrop::Collector` once, up front - call this
/// during setup, never from the audio thread. The returned [`BasedropRetirer`]
/// is intended for the real-time audio thread; the returned
/// [`BasedropCollector`] is intended for a non-real-time background thread.
pub fn basedrop_deferred_deallocator() -> (BasedropRetirer, BasedropCollector) {
    let collector = Collector::new();
    let handle = collector.handle();
    let pending = Arc::new(AtomicU32::new(0));

    (
        BasedropRetirer {
            handle,
            pending: Arc::clone(&pending),
        },
        BasedropCollector { collector, pending },
    )
}

impl Retire for BasedropRetirer {
    fn retire(&mut self, allocation: Retired) {
        // `Retired` is `Send + 'static` (its only field is a
        // `Box<dyn Send + 'static>`, and `Send` is an auto trait), so it can
        // be handed straight to `basedrop` without unwrapping it - we never
        // need to reach inside the opaque `Retired` value ourselves.
        //
        // `basedrop`'s public API only exposes hand-off through
        // `Owned::new`, which wraps `allocation` in a `Node` and links it
        // onto the collector's queue when the `Owned` is dropped
        // (`Node::queue_drop` is a lock-free pointer swap, not a free).
        // Dropping `owned` here enqueues `allocation`'s destructor for the
        // later, off-thread `collect()` call rather than running it now.
        let owned = Owned::new(&self.handle, allocation);
        drop(owned);
        self.pending.fetch_add(1, Ordering::Relaxed);
    }
}

impl Collect for BasedropCollector {
    fn collect(&mut self) -> u32 {
        // Actually drops every item `basedrop` has queued, freeing their
        // memory. This is only safe to call off the audio thread.
        self.collector.collect();
        self.pending.swap(0, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as StdOrdering};

    struct DropCounter(Arc<AtomicUsize>);

    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.0.fetch_add(1, StdOrdering::SeqCst);
        }
    }

    #[test]
    fn collect_on_empty_queue_returns_zero() {
        let (_retirer, mut collector) = basedrop_deferred_deallocator();
        assert_eq!(collector.collect(), 0);
    }

    #[test]
    fn retire_defers_the_drop_until_collect_runs() {
        let counter = Arc::new(AtomicUsize::new(0));
        let (mut retirer, mut collector) = basedrop_deferred_deallocator();

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
    fn collect_drains_every_retired_allocation() {
        let counter = Arc::new(AtomicUsize::new(0));
        let (mut retirer, mut collector) = basedrop_deferred_deallocator();

        for _ in 0..5 {
            retirer.retire(Retired::new(Box::new(DropCounter(Arc::clone(&counter)))));
        }

        let freed = collector.collect();
        assert_eq!(freed, 5);
        assert_eq!(counter.load(StdOrdering::SeqCst), 5);

        // A second collect with nothing newly retired must be a no-op.
        assert_eq!(collector.collect(), 0);
    }

    #[test]
    fn retirer_and_collector_can_be_used_across_multiple_rounds() {
        let counter = Arc::new(AtomicUsize::new(0));
        let (mut retirer, mut collector) = basedrop_deferred_deallocator();

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
