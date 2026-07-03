// path: src/real_time/triple_buffer_parameter_bridge.rs

//! Adapter implementing `port.RealTime.ParameterBridge` on top of the
//! external `triple_buffer` crate (`meta.framework = "triple_buffer"`),
//! as an alternative to the hand-rolled atomic-index implementation in
//! [`crate::real_time::parameter_bridge::ParameterBridge`].
//!
//! # Design
//!
//! The `triple_buffer` crate itself provides the wait-free, allocation-
//! free triple-buffer machinery: three slots are allocated once when the
//! buffer is split into an [`triple_buffer::Input`] half and a
//! [`triple_buffer::Output`] half. Publishing and reading afterward never
//! allocate, never lock, and never block — satisfying the invariants that
//! the audio thread never allocates heap memory and never acquires a
//! blocking lock, and that all parameter changes cross the RT boundary
//! via the `ParameterBridge`.
//!
//! `triple_buffer::Input::write` and `triple_buffer::Output::read` both
//! require `&mut self`, but the port contract calls for `read`/`publish`
//! on `&self` so a single bridge instance can be shared (e.g. via `Arc`)
//! between the writer and reader threads. Each half is therefore wrapped
//! in an [`UnsafeCell`], exactly mirroring the safety argument already
//! used by [`crate::real_time::parameter_bridge::ParameterBridge`]: the
//! input half is only ever touched by `publish`, called exclusively by
//! the single non-real-time writer thread, and the output half is only
//! ever touched by `read`, called exclusively by the single real-time
//! reader thread. The two cells are disjoint, so no data race occurs
//! even though both are reached through a shared `&self`.

use std::cell::UnsafeCell;

use triple_buffer::{Input, Output, TripleBuffer};

use crate::real_time::parameter_bridge::ParameterSnapshot;

/// Lock-free, allocation-free bridge for publishing the latest
/// [`ParameterSnapshot`] across the real-time/non-real-time thread
/// boundary, built on the external `triple_buffer` crate.
///
/// One thread (the non-RT "writer") calls
/// [`TripleBufferParameterBridge::publish`]; a different thread (the RT
/// "reader") calls [`TripleBufferParameterBridge::read`]. Both methods
/// take `&self` so a single `Arc<TripleBufferParameterBridge>` can be
/// shared between the two threads without any additional
/// synchronization.
pub struct TripleBufferParameterBridge {
    input: UnsafeCell<Input<ParameterSnapshot>>,
    output: UnsafeCell<Output<ParameterSnapshot>>,
}

// SAFETY: `input` is only ever accessed from `publish`, called
// exclusively by the single writer thread; `output` is only ever
// accessed from `read`, called exclusively by the single reader thread.
// The two cells never alias the same underlying data at the same time,
// so there is no data race even though both are reached through a
// shared `&self`. This mirrors the safety argument documented on
// `crate::real_time::parameter_bridge::ParameterBridge`.
unsafe impl Sync for TripleBufferParameterBridge {}

impl TripleBufferParameterBridge {
    /// Creates a bridge pre-seeded with `initial` in every slot.
    /// Allocation happens here, at construction time, on whichever
    /// thread builds the bridge — never inside `publish` or `read`.
    pub fn new(initial: ParameterSnapshot) -> Self {
        let (input, output) = TripleBuffer::new(&initial).split();
        Self {
            input: UnsafeCell::new(input),
            output: UnsafeCell::new(output),
        }
    }

    /// Publishes a new snapshot, overwriting whatever the writer last
    /// published but was never read. Never allocates, never blocks.
    ///
    /// Must only be called from the single writer thread (the
    /// non-real-time UI/MIDI thread).
    pub fn publish(&self, snapshot: ParameterSnapshot) {
        // SAFETY: see struct-level safety comment; only the writer
        // thread ever reaches this cell.
        let input = unsafe { &mut *self.input.get() };
        input.write(snapshot);
    }

    /// Returns the most recently published snapshot without blocking.
    /// If nothing has been published yet, returns the snapshot the
    /// bridge was constructed with.
    ///
    /// Must only be called from the single reader thread (the
    /// real-time audio thread).
    pub fn read(&self) -> ParameterSnapshot {
        // SAFETY: see struct-level safety comment; only the reader
        // thread ever reaches this cell.
        let output = unsafe { &mut *self.output.get() };
        *output.read()
    }
}

impl Default for TripleBufferParameterBridge {
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
        let bridge = TripleBufferParameterBridge::default();
        assert_eq!(bridge.read(), ParameterSnapshot::default());
    }

    #[test]
    fn read_after_publish_returns_latest_snapshot() {
        let bridge = TripleBufferParameterBridge::default();
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
        let bridge = TripleBufferParameterBridge::default();
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
        let bridge = TripleBufferParameterBridge::default();

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
        let bridge = Arc::new(TripleBufferParameterBridge::default());
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
    fn default_snapshot_round_trips_through_publish_and_read() {
        let bridge = TripleBufferParameterBridge::default();
        let snapshot = ParameterSnapshot::default();

        bridge.publish(snapshot);

        assert_eq!(bridge.read(), snapshot);
    }
}
