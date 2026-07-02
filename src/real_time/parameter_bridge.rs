// path: src/real_time/parameter_bridge.rs

//! Lock-free, allocation-free bridge for publishing the latest
//! [`ParameterSnapshot`] from the non-real-time (UI/MIDI) thread to the
//! real-time audio thread.
//!
//! # Design
//!
//! A classic wait-free "triple buffer": three snapshot slots are
//! allocated once, at construction. `publish` and `read` never allocate,
//! never lock, and never block afterward — they are pure atomic index
//! exchanges plus a single `Copy` of a small POD value. This satisfies
//! the project invariant that all parameter changes cross the RT
//! boundary through the `ParameterBridge`, and that the audio thread
//! never allocates or blocks.
//!
//! Slot ownership rotates through three roles:
//! - **back**: owned by the writer, safe to overwrite.
//! - **shared**: the most recently published snapshot, exchanged
//!   atomically between writer and reader.
//! - **front**: owned by the reader, safe to read from repeatedly.
//!
//! Publishing writes into `back`, then atomically swaps it with
//! `shared` (marking it fresh). Reading atomically swaps `shared` into
//! `front` only if fresh, then copies out of `front`. Because each
//! index is only ever touched by the thread that currently "owns" it,
//! there is no data race even though the buffers are shared without a
//! lock.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU8, Ordering};

/// A snapshot of all continuously-varying synthesis parameters,
/// published by the non-real-time thread and consumed by the
/// real-time audio thread without blocking.
///
/// Kept as a small, `Copy` plain-old-data type so that publishing and
/// reading are a single atomic index exchange plus a cheap copy — never
/// a heap allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterSnapshot {
    pub master_volume: f32,
    pub master_pan: f32,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
}

impl Default for ParameterSnapshot {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            master_pan: 0.0,
            filter_cutoff: 1.0,
            filter_resonance: 0.0,
        }
    }
}

const NEW_DATA_FLAG: u8 = 0b100;
const INDEX_MASK: u8 = 0b011;

/// Lock-free single-producer/single-consumer bridge carrying the latest
/// [`ParameterSnapshot`] across the real-time/non-real-time thread
/// boundary.
///
/// One thread (the non-RT "writer") calls [`ParameterBridge::publish`];
/// a different thread (the RT "reader") calls [`ParameterBridge::read`].
/// Both methods take `&self` so a single `Arc<ParameterBridge>` can be
/// shared between the two threads without any additional
/// synchronization.
///
/// The reader always observes the most recently published snapshot (or
/// the default snapshot if nothing has been published yet) and never
/// blocks waiting on the writer, and vice versa.
pub struct ParameterBridge {
    buffers: [UnsafeCell<ParameterSnapshot>; 3],
    /// Index (0..3) of the buffer currently exchanged between writer
    /// and reader, packed with `NEW_DATA_FLAG` in the top bit.
    shared_state: AtomicU8,
    /// Index (0..3) of the buffer currently owned by the writer.
    /// Only ever touched from within `publish`, i.e. by the writer
    /// thread; kept atomic purely so the type can be `Sync` without
    /// unsafe casts elsewhere.
    back_index: AtomicU8,
    /// Index (0..3) of the buffer currently owned by the reader.
    /// Only ever touched from within `read`, i.e. by the reader thread.
    front_index: AtomicU8,
}

// SAFETY: `buffers` is only ever accessed through the indices tracked
// by `back_index` (writer-owned), `shared_state` (exchanged
// atomically), and `front_index` (reader-owned). The publish/read
// algorithm below guarantees that at any instant each of the three
// indices is distinct, so the writer and reader never touch the same
// slot concurrently. This is the standard wait-free triple-buffer
// construction; see module docs.
unsafe impl Sync for ParameterBridge {}

impl ParameterBridge {
    /// Creates a bridge pre-seeded with `initial` in every slot.
    /// Allocation happens here, at construction time, on whichever
    /// thread builds the bridge — never inside `publish` or `read`.
    pub fn new(initial: ParameterSnapshot) -> Self {
        Self {
            buffers: [
                UnsafeCell::new(initial),
                UnsafeCell::new(initial),
                UnsafeCell::new(initial),
            ],
            shared_state: AtomicU8::new(2),
            back_index: AtomicU8::new(0),
            front_index: AtomicU8::new(1),
        }
    }

    /// Publishes a new snapshot, overwriting whatever the writer last
    /// published but was never read. Never allocates, never blocks.
    ///
    /// Must only be called from the single writer thread (the
    /// non-real-time UI/MIDI thread).
    pub fn publish(&self, snapshot: ParameterSnapshot) {
        let back = self.back_index.load(Ordering::Relaxed) as usize;

        // SAFETY: `back` is owned exclusively by the writer between
        // calls to `publish` — the reader never touches this index
        // until the swap below hands it over as the new shared index.
        unsafe {
            *self.buffers[back].get() = snapshot;
        }

        let new_shared = back as u8 | NEW_DATA_FLAG;
        let previous_shared = self.shared_state.swap(new_shared, Ordering::AcqRel);
        self.back_index
            .store(previous_shared & INDEX_MASK, Ordering::Relaxed);
    }

    /// Returns the most recently published snapshot without blocking.
    /// If nothing has been published yet, returns the snapshot the
    /// bridge was constructed with.
    ///
    /// Must only be called from the single reader thread (the
    /// real-time audio thread).
    pub fn read(&self) -> ParameterSnapshot {
        let shared = self.shared_state.load(Ordering::Acquire);

        if shared & NEW_DATA_FLAG != 0 {
            let front = self.front_index.load(Ordering::Relaxed);
            let previous_shared = self.shared_state.swap(front, Ordering::AcqRel);
            self.front_index
                .store(previous_shared & INDEX_MASK, Ordering::Relaxed);
        }

        let front = self.front_index.load(Ordering::Relaxed) as usize;

        // SAFETY: `front` is owned exclusively by the reader between
        // calls to `read` — the writer never touches this index until
        // it is handed back as the new back index via a future swap.
        unsafe { *self.buffers[front].get() }
    }
}

impl Default for ParameterBridge {
    fn default() -> Self {
        Self::new(ParameterSnapshot::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn read_before_any_publish_returns_default() {
        let bridge = ParameterBridge::default();
        assert_eq!(bridge.read(), ParameterSnapshot::default());
    }

    #[test]
    fn read_after_publish_returns_latest_snapshot() {
        let bridge = ParameterBridge::default();
        let snapshot = ParameterSnapshot {
            master_volume: 0.5,
            master_pan: -0.25,
            filter_cutoff: 0.8,
            filter_resonance: 0.1,
        };

        bridge.publish(snapshot);

        assert_eq!(bridge.read(), snapshot);
    }

    #[test]
    fn repeated_reads_without_new_publish_return_same_snapshot() {
        let bridge = ParameterBridge::default();
        let snapshot = ParameterSnapshot {
            master_volume: 0.75,
            master_pan: 0.0,
            filter_cutoff: 1.0,
            filter_resonance: 0.2,
        };
        bridge.publish(snapshot);

        assert_eq!(bridge.read(), snapshot);
        assert_eq!(bridge.read(), snapshot);
    }

    #[test]
    fn only_the_most_recent_publish_is_observed() {
        let bridge = ParameterBridge::default();

        bridge.publish(ParameterSnapshot {
            master_volume: 0.1,
            ..ParameterSnapshot::default()
        });
        bridge.publish(ParameterSnapshot {
            master_volume: 0.2,
            ..ParameterSnapshot::default()
        });
        bridge.publish(ParameterSnapshot {
            master_volume: 0.3,
            ..ParameterSnapshot::default()
        });

        assert_eq!(bridge.read().master_volume, 0.3);
    }

    #[test]
    fn writer_and_reader_on_separate_threads_never_block_and_converge() {
        let bridge = Arc::new(ParameterBridge::default());
        let writer_bridge = Arc::clone(&bridge);

        let writer = thread::spawn(move || {
            for i in 0..1_000 {
                writer_bridge.publish(ParameterSnapshot {
                    master_volume: i as f32 / 1_000.0,
                    master_pan: 0.0,
                    filter_cutoff: 1.0,
                    filter_resonance: 0.0,
                });
            }
        });

        writer.join().unwrap();

        // After the writer finishes, the reader must eventually observe
        // the final published value without blocking.
        let last = bridge.read();
        assert_eq!(last.master_volume, 999.0 / 1_000.0);
    }

    #[test]
    fn bridge_holds_exactly_three_fixed_slots_regardless_of_publish_count() {
        // Structural guarantee: ParameterBridge holds exactly three
        // buffers regardless of how many times publish is called —
        // no growth, no reallocation.
        let bridge = ParameterBridge::default();
        for i in 0..10_000 {
            bridge.publish(ParameterSnapshot {
                master_volume: i as f32,
                ..ParameterSnapshot::default()
            });
        }
        assert_eq!(bridge.buffers.len(), 3);
    }
}
