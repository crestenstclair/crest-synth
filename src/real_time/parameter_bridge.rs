// path: src/real_time/parameter_bridge.rs
//
// Lock-free triple-buffer bridge for synth parameters.
//
// The UI / control thread calls `write` to publish a new `ParameterSnapshot`.
// The audio thread calls `read` to receive the latest snapshot — always
// lock-free and allocation-free.
//
// # Triple-buffer protocol
//
// Three slots (indices 0, 1, 2) are allocated once on construction.
// An `AtomicU8` packs the state:
//
//   bits [1:0] — "ready" slot index (the last slot published by the writer)
//   bit  2     — "new"   flag (set when the writer has published since the
//                              last time the reader consumed the ready slot)
//
// Writer:
//   1. Writes into its private slot.
//   2. Atomically swaps its private slot index with the ready slot, setting
//      the "new" flag.
//
// Reader:
//   1. Checks the "new" flag.
//   2. If set: swaps its private slot with the ready slot, clearing the flag.
//   3. Returns a reference to its private slot (the freshest snapshot).
//
// The writer and reader each own a private slot that the other never touches
// concurrently, so no additional synchronisation is required beyond the
// single atomic swap.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use crate::real_time::parameter_snapshot::ParameterSnapshot;

// ─── bit layout ──────────────────────────────────────────────────────────────

/// Bit mask / shift constants for the packed atomic byte.
///
/// Bits [1:0]: ready slot index (0, 1, or 2).
/// Bit    2  : "new value available" flag.
const NEW_FLAG: u8 = 0b0000_0100;
const SLOT_MASK: u8 = 0b0000_0011;

// ─── internal shared state ────────────────────────────────────────────────────

struct TripleBufferInner {
    /// Three fixed-size slots.  Only the owning half (writer or reader) plus
    /// the ready slot is ever written; those accesses never overlap.
    slots: [UnsafeCell<ParameterSnapshot>; 3],
    /// Packed: bits[1:0] = ready index, bit[2] = new flag.
    state: AtomicU8,
}

// SAFETY: `TripleBufferInner` is shared between exactly two threads — the
// writer and the reader.  The invariant maintained by the triple-buffer
// protocol is that no two threads ever access the *same* slot concurrently:
// each side holds one private slot at all times, and the "ready" slot is
// only ever touched by one party at a time (the atomic swap serialises the
// handoff).  Given that invariant, the `UnsafeCell` accesses are sound.
unsafe impl Send for TripleBufferInner {}
unsafe impl Sync for TripleBufferInner {}

// ─── ParameterBridgeWriter ────────────────────────────────────────────────────

/// The writer (UI / control) half of the [`ParameterBridge`].
///
/// Obtained via [`ParameterBridge::split`].  Publishes parameter updates to
/// the audio thread without blocking.
pub struct ParameterBridgeWriter {
    inner: Arc<TripleBufferInner>,
    /// The slot index privately owned by the writer (never touched by reader).
    writer_slot: u8,
}

impl ParameterBridgeWriter {
    /// Publish `snapshot` so the audio thread will see it on the next
    /// [`ParameterBridgeReader::read`] call.
    ///
    /// Never blocks, never allocates — safe to call from any thread.
    #[inline]
    pub fn write(&mut self, snapshot: ParameterSnapshot) {
        // Write into our private slot (only we touch this slot).
        let ws = self.writer_slot as usize;
        // SAFETY: `writer_slot` is exclusively owned by this writer.
        unsafe {
            *self.inner.slots[ws].get() = snapshot;
        }

        // Swap our private slot with the ready slot and set the NEW flag.
        let new_state = self.writer_slot | NEW_FLAG;
        let prev = self.inner.state.swap(new_state, Ordering::AcqRel);

        // The previous ready slot index becomes our new private slot.
        self.writer_slot = prev & SLOT_MASK;
    }
}

// ─── ParameterBridgeReader ────────────────────────────────────────────────────

/// The reader (audio thread) half of the [`ParameterBridge`].
///
/// Obtained via [`ParameterBridge::split`].  Reads the latest parameter
/// snapshot lock-free and allocation-free.
pub struct ParameterBridgeReader {
    inner: Arc<TripleBufferInner>,
    /// The slot index privately owned by the reader.
    reader_slot: u8,
}

impl ParameterBridgeReader {
    /// Return a reference to the latest [`ParameterSnapshot`].
    ///
    /// If the writer has published a new value since the last call, the
    /// reader swaps to the ready slot first (consuming the new flag).
    /// Otherwise it returns the previously consumed snapshot unchanged.
    ///
    /// Never blocks, never allocates — safe to call from the audio thread.
    #[inline]
    pub fn read(&mut self) -> &ParameterSnapshot {
        let state = self.inner.state.load(Ordering::Acquire);
        if state & NEW_FLAG != 0 {
            // New snapshot available: swap our private slot with the ready
            // slot, clearing the NEW flag.
            let new_state = self.reader_slot; // our old slot becomes the new ready; flag cleared
            let prev = self.inner.state.swap(new_state, Ordering::AcqRel);
            self.reader_slot = prev & SLOT_MASK;
        }
        // SAFETY: `reader_slot` is exclusively owned by this reader after the swap.
        unsafe { &*self.inner.slots[self.reader_slot as usize].get() }
    }
}

// ─── ParameterBridge ─────────────────────────────────────────────────────────

/// Factory that creates a matched [`ParameterBridgeWriter`] /
/// [`ParameterBridgeReader`] pair sharing a lock-free triple buffer.
///
/// # Usage
///
/// ```
/// use crest_synth::real_time::parameter_bridge::ParameterBridge;
/// use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
///
/// let (mut writer, mut reader) = ParameterBridge::split(ParameterSnapshot::default());
/// writer.write(ParameterSnapshot::default());
/// let _snap = reader.read();
/// ```
pub struct ParameterBridge;

impl ParameterBridge {
    /// Create a writer/reader pair initialised with `initial`.
    ///
    /// - Slot 0 → reader's private slot (initialised to `initial`)
    /// - Slot 1 → ready slot (initialised to `initial`)
    /// - Slot 2 → writer's private slot (initialised to `initial`)
    ///
    /// Initial state: ready index = 1, NEW flag = 0.
    pub fn split(initial: ParameterSnapshot) -> (ParameterBridgeWriter, ParameterBridgeReader) {
        let inner = Arc::new(TripleBufferInner {
            slots: [
                UnsafeCell::new(initial),
                UnsafeCell::new(initial),
                UnsafeCell::new(initial),
            ],
            // ready = slot 1, no new flag
            state: AtomicU8::new(1),
        });

        let writer = ParameterBridgeWriter {
            inner: Arc::clone(&inner),
            writer_slot: 2,
        };
        let reader = ParameterBridgeReader {
            inner,
            reader_slot: 0,
        };

        (writer, reader)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::amp_envelope_config::AmpEnvelopeConfig;
    use crate::synth::filter_config::FilterConfig;
    use crate::synth::oscillator_config::{OscillatorConfig, Waveform};

    fn default_snap() -> ParameterSnapshot {
        ParameterSnapshot::default()
    }

    fn make_snap(detune: f64) -> ParameterSnapshot {
        ParameterSnapshot::new(
            AmpEnvelopeConfig::default(),
            FilterConfig::default(),
            OscillatorConfig::try_new(detune, 0.5, Waveform::Sine).unwrap(),
            0,
        )
    }

    #[test]
    fn reader_returns_initial_snapshot_before_any_write() {
        let (_writer, mut reader) = ParameterBridge::split(default_snap());
        let snap = reader.read();
        assert_eq!(*snap, default_snap());
    }

    #[test]
    fn reader_sees_latest_write() {
        let (mut writer, mut reader) = ParameterBridge::split(default_snap());
        let updated = make_snap(100.0);
        writer.write(updated);
        let snap = reader.read();
        assert_eq!(*snap, updated);
    }

    #[test]
    fn multiple_writes_reader_gets_last() {
        let (mut writer, mut reader) = ParameterBridge::split(default_snap());
        writer.write(make_snap(50.0));
        writer.write(make_snap(200.0));
        let snap = reader.read();
        // Reader gets the most-recently published snapshot.
        assert_eq!(snap.oscillator.detune, 200.0);
    }

    #[test]
    fn repeated_reads_without_write_return_same_snapshot() {
        let (mut writer, mut reader) = ParameterBridge::split(default_snap());
        let snap1 = make_snap(75.0);
        writer.write(snap1);
        let a = *reader.read();
        let b = *reader.read();
        assert_eq!(a, b);
    }

    #[test]
    fn write_then_read_then_write_then_read_cycle() {
        let initial = default_snap();
        let (mut writer, mut reader) = ParameterBridge::split(initial);

        let s1 = make_snap(10.0);
        writer.write(s1);
        let r1 = *reader.read();
        assert_eq!(r1, s1);

        let s2 = make_snap(20.0);
        writer.write(s2);
        let r2 = *reader.read();
        assert_eq!(r2, s2);

        let s3 = make_snap(30.0);
        writer.write(s3);
        let r3 = *reader.read();
        assert_eq!(r3, s3);
    }

    #[test]
    fn no_write_reader_returns_initial_on_every_read() {
        let snap = make_snap(42.0);
        let (_writer, mut reader) = ParameterBridge::split(snap);
        for _ in 0..5 {
            assert_eq!(*reader.read(), snap);
        }
    }

    #[test]
    fn snapshot_is_copy_no_heap_required() {
        // ParameterSnapshot must be Copy (stack-only); verify by copying.
        let a = ParameterSnapshot::default();
        let b = a; // copy
        assert_eq!(a, b);
    }
}
