// path: src/patch/global_mixer.rs
//
// GlobalMixer aggregate — master mix bus.
//
// Sums all patch outputs and applies a master gain. Parameter changes from the
// control thread cross the RT boundary via a lock-free triple-buffer bridge;
// the audio thread reads them without ever blocking or allocating.

use crate::kernel::amplitude::Amplitude;

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

// ─── bit layout ──────────────────────────────────────────────────────────────

const NEW_FLAG: u8 = 0b0000_0100;
const SLOT_MASK: u8 = 0b0000_0011;

// ─── Commands ────────────────────────────────────────────────────────────────

/// Commands accepted by [`GlobalMixer`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlobalMixerCommand {
    /// Update the master gain to `gain`.
    SetMasterGain { gain: Amplitude },
}

// ─── Events ──────────────────────────────────────────────────────────────────

/// Events emitted by [`GlobalMixer`] in response to commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlobalMixerEvent {
    /// The master gain was successfully updated to `gain`.
    MasterGainChanged { gain: Amplitude },
}

// ─── Errors ──────────────────────────────────────────────────────────────────

/// Errors that can arise when handling a [`GlobalMixerCommand`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlobalMixerError {
    /// The requested gain value is invalid (negative or NaN).
    InvalidGain,
}

impl std::fmt::Display for GlobalMixerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidGain => f.write_str("master gain must be non-negative and not NaN"),
        }
    }
}

impl std::error::Error for GlobalMixerError {}

// ─── GlobalMixerState ────────────────────────────────────────────────────────

/// Pure, immutable state for the [`GlobalMixer`] aggregate (control-thread side).
///
/// All methods are allocation-free and blocking-free.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalMixerState {
    /// Current master gain value.
    pub master_gain: Amplitude,
}

impl GlobalMixerState {
    /// Construct with an explicit `master_gain`.
    pub fn new(master_gain: Amplitude) -> Self {
        Self { master_gain }
    }

    /// Apply a [`GlobalMixerCommand`] to produce a new state and an event.
    ///
    /// State is never mutated in place — a new value is returned.
    pub fn handle(
        &self,
        cmd: GlobalMixerCommand,
    ) -> Result<(GlobalMixerState, GlobalMixerEvent), GlobalMixerError> {
        match cmd {
            GlobalMixerCommand::SetMasterGain { gain } => {
                let new_state = GlobalMixerState { master_gain: gain };
                let event = GlobalMixerEvent::MasterGainChanged { gain };
                Ok((new_state, event))
            }
        }
    }
}

impl Default for GlobalMixerState {
    /// Unity gain by default.
    fn default() -> Self {
        Self {
            master_gain: Amplitude::unity(),
        }
    }
}

// ─── MixerParameterSnapshot (internal, stack-only) ───────────────────────────

/// Lock-free snapshot of mixer parameters crossing the RT boundary.
///
/// Must be `Copy` (no heap) to guarantee allocation-free reads on the audio
/// thread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MixerParameterSnapshot {
    /// Master gain to apply at the mix bus.
    pub master_gain: Amplitude,
}

impl Default for MixerParameterSnapshot {
    fn default() -> Self {
        Self {
            master_gain: Amplitude::unity(),
        }
    }
}

// ─── MixerTripleBuffer (internal shared state) ────────────────────────────────

struct MixerTripleBuffer {
    /// Three fixed-size slots. Only the owning half touches its private slot;
    /// the "ready" slot handoff is serialised by the atomic swap.
    slots: [UnsafeCell<MixerParameterSnapshot>; 3],
    /// Packed: bits[1:0] = ready index, bit[2] = new flag.
    state: AtomicU8,
}

// SAFETY: The triple-buffer protocol guarantees no two threads ever access the
// *same* slot concurrently (each owns one private slot; the swap serialises
// the ready-slot handoff).
unsafe impl Send for MixerTripleBuffer {}
unsafe impl Sync for MixerTripleBuffer {}

// ─── MixerBridgeWriter (control-thread half) ─────────────────────────────────

/// Control-thread writer for the mixer parameter bridge.
///
/// Obtained via [`MixerBridge::split`].
pub struct MixerBridgeWriter {
    inner: Arc<MixerTripleBuffer>,
    writer_slot: u8,
}

impl MixerBridgeWriter {
    /// Publish `snapshot` to the audio thread.  Lock-free, allocation-free.
    #[inline]
    pub fn write(&mut self, snapshot: MixerParameterSnapshot) {
        let ws = self.writer_slot as usize;
        // SAFETY: `writer_slot` is exclusively owned by this writer.
        unsafe {
            *self.inner.slots[ws].get() = snapshot;
        }
        let new_state = self.writer_slot | NEW_FLAG;
        let prev = self.inner.state.swap(new_state, Ordering::AcqRel);
        self.writer_slot = prev & SLOT_MASK;
    }
}

// ─── MixerBridgeReader (audio-thread half) ────────────────────────────────────

/// Audio-thread reader for the mixer parameter bridge.
///
/// Obtained via [`MixerBridge::split`].
pub struct MixerBridgeReader {
    inner: Arc<MixerTripleBuffer>,
    reader_slot: u8,
}

impl MixerBridgeReader {
    /// Return the latest snapshot.  Lock-free, allocation-free.
    #[inline]
    pub fn read(&mut self) -> &MixerParameterSnapshot {
        let state = self.inner.state.load(Ordering::Acquire);
        if state & NEW_FLAG != 0 {
            let new_state = self.reader_slot; // our old slot → new ready; clears flag
            let prev = self.inner.state.swap(new_state, Ordering::AcqRel);
            self.reader_slot = prev & SLOT_MASK;
        }
        // SAFETY: `reader_slot` is exclusively owned by this reader after the swap.
        unsafe { &*self.inner.slots[self.reader_slot as usize].get() }
    }
}

// ─── MixerBridge ─────────────────────────────────────────────────────────────

/// Factory that creates a matched [`MixerBridgeWriter`] / [`MixerBridgeReader`]
/// pair sharing a lock-free triple buffer.
pub struct MixerBridge;

impl MixerBridge {
    /// Create a writer/reader pair initialised with `initial`.
    pub fn split(initial: MixerParameterSnapshot) -> (MixerBridgeWriter, MixerBridgeReader) {
        let inner = Arc::new(MixerTripleBuffer {
            slots: [
                UnsafeCell::new(initial),
                UnsafeCell::new(initial),
                UnsafeCell::new(initial),
            ],
            // ready = slot 1, no new flag
            state: AtomicU8::new(1),
        });
        let writer = MixerBridgeWriter {
            inner: Arc::clone(&inner),
            writer_slot: 2,
        };
        let reader = MixerBridgeReader {
            inner,
            reader_slot: 0,
        };
        (writer, reader)
    }
}

// ─── GlobalMixerWriter (control-thread aggregate handle) ─────────────────────

/// Control-thread handle for the [`GlobalMixer`] aggregate.
///
/// Holds the authoritative state and publishes parameter updates to the audio
/// thread through the lock-free [`MixerBridgeWriter`].
///
/// # Dependency Injection
///
/// Both `state` and `bridge_writer` are accepted via constructor, allowing
/// tests to supply a paired reader without going through a real audio callback.
pub struct GlobalMixerWriter {
    state: GlobalMixerState,
    bridge_writer: MixerBridgeWriter,
}

impl GlobalMixerWriter {
    /// Construct with an explicit `state` and `bridge_writer`.
    ///
    /// Prefer [`GlobalMixer::split`] to obtain a matched writer/reader pair.
    pub fn new(state: GlobalMixerState, bridge_writer: MixerBridgeWriter) -> Self {
        Self {
            state,
            bridge_writer,
        }
    }

    /// Return the current state (read-only).
    pub fn state(&self) -> &GlobalMixerState {
        &self.state
    }

    /// Handle a [`GlobalMixerCommand`].
    ///
    /// On success: updates internal state, publishes new parameters to the
    /// audio thread via the bridge, and returns the emitted event.
    ///
    /// On error: state and bridge are unchanged.
    pub fn handle(
        &mut self,
        cmd: GlobalMixerCommand,
    ) -> Result<GlobalMixerEvent, GlobalMixerError> {
        let (new_state, event) = self.state.handle(cmd)?;
        self.state = new_state;
        // Publish the updated snapshot to the audio thread (lock-free).
        self.bridge_writer.write(MixerParameterSnapshot {
            master_gain: new_state.master_gain,
        });
        Ok(event)
    }
}

// ─── GlobalMixerReader (audio-thread aggregate handle) ───────────────────────

/// Audio-thread handle for the [`GlobalMixer`] aggregate.
///
/// Reads the latest master gain from the lock-free bridge and applies it when
/// summing audio frames. Never blocks, never allocates.
///
/// # Dependency Injection
///
/// `bridge_reader` is accepted via constructor so tests can supply a paired
/// writer without involving real audio hardware.
pub struct GlobalMixerReader {
    bridge_reader: MixerBridgeReader,
    /// Cached gain used until a new value arrives from the bridge.
    cached_gain: Amplitude,
}

impl GlobalMixerReader {
    /// Construct with a `bridge_reader` and an `initial_gain`.
    ///
    /// Prefer [`GlobalMixer::split`] to obtain a matched writer/reader pair.
    pub fn new(bridge_reader: MixerBridgeReader, initial_gain: Amplitude) -> Self {
        Self {
            bridge_reader,
            cached_gain: initial_gain,
        }
    }

    /// Poll the bridge for the latest gain value and return it.
    ///
    /// Lock-free and allocation-free — safe to call on the audio thread.
    #[inline]
    pub fn master_gain(&mut self) -> Amplitude {
        let snap = self.bridge_reader.read();
        self.cached_gain = snap.master_gain;
        self.cached_gain
    }

    /// Apply the current master gain to a stereo frame `[left, right]`.
    ///
    /// Lock-free and allocation-free — safe to call on the audio thread.
    #[inline]
    pub fn apply(&mut self, frame: [f32; 2]) -> [f32; 2] {
        let g = self.master_gain().value() as f32;
        [frame[0] * g, frame[1] * g]
    }
}

// ─── GlobalMixer (factory / entry point) ─────────────────────────────────────

/// Factory that creates a matched [`GlobalMixerWriter`] / [`GlobalMixerReader`]
/// pair for the master mix bus.
///
/// The writer lives on the control thread; the reader lives on the audio thread.
///
/// # Examples
///
/// ```
/// use crest_synth::patch::global_mixer::{GlobalMixer, GlobalMixerCommand};
/// use crest_synth::kernel::amplitude::Amplitude;
///
/// let (mut writer, mut reader) = GlobalMixer::split(Amplitude::unity());
/// // Control thread: change master gain.
/// writer.handle(GlobalMixerCommand::SetMasterGain {
///     gain: Amplitude::try_new(0.5).unwrap(),
/// })
/// .unwrap();
/// // Audio thread: read current gain (lock-free).
/// let gain = reader.master_gain();
/// assert!((gain.value() - 0.5).abs() < 1e-9);
/// ```
pub struct GlobalMixer;

impl GlobalMixer {
    /// Create a matched writer/reader pair with `initial_gain` as the starting value.
    pub fn split(initial_gain: Amplitude) -> (GlobalMixerWriter, GlobalMixerReader) {
        let initial_snap = MixerParameterSnapshot {
            master_gain: initial_gain,
        };
        let (bridge_w, bridge_r) = MixerBridge::split(initial_snap);
        let state = GlobalMixerState::new(initial_gain);
        let writer = GlobalMixerWriter::new(state, bridge_w);
        let reader = GlobalMixerReader::new(bridge_r, initial_gain);
        (writer, reader)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── GlobalMixerState ─────────────────────────────────────────────────────

    #[test]
    fn default_state_is_unity_gain() {
        let state = GlobalMixerState::default();
        assert!((state.master_gain.value() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn set_master_gain_updates_state_and_emits_event() {
        let state = GlobalMixerState::default();
        let gain = Amplitude::try_new(0.5).unwrap();
        let (new_state, event) = state
            .handle(GlobalMixerCommand::SetMasterGain { gain })
            .unwrap();
        assert!((new_state.master_gain.value() - 0.5).abs() < f64::EPSILON);
        assert_eq!(event, GlobalMixerEvent::MasterGainChanged { gain });
    }

    #[test]
    fn set_master_gain_to_silence_is_valid() {
        let state = GlobalMixerState::default();
        let (new_state, _) = state
            .handle(GlobalMixerCommand::SetMasterGain {
                gain: Amplitude::silence(),
            })
            .unwrap();
        assert!(new_state.master_gain.value().abs() < f64::EPSILON);
    }

    #[test]
    fn set_master_gain_above_unity_is_valid() {
        let state = GlobalMixerState::default();
        let gain = Amplitude::try_new(2.0).unwrap();
        assert!(state
            .handle(GlobalMixerCommand::SetMasterGain { gain })
            .is_ok());
    }

    #[test]
    fn state_handle_is_pure_original_unchanged() {
        let state = GlobalMixerState::default();
        let gain = Amplitude::try_new(0.3).unwrap();
        let (new_state, _) = state
            .handle(GlobalMixerCommand::SetMasterGain { gain })
            .unwrap();
        // Original unchanged.
        assert!((state.master_gain.value() - 1.0).abs() < f64::EPSILON);
        assert!((new_state.master_gain.value() - 0.3).abs() < 1e-9);
    }

    // ── MixerBridge ───────────────────────────────────────────────────────────

    #[test]
    fn bridge_reader_returns_initial_gain_before_any_write() {
        let initial = MixerParameterSnapshot {
            master_gain: Amplitude::unity(),
        };
        let (_w, mut r) = MixerBridge::split(initial);
        assert!((r.read().master_gain.value() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn bridge_reader_sees_written_gain() {
        let initial = MixerParameterSnapshot::default();
        let (mut w, mut r) = MixerBridge::split(initial);
        w.write(MixerParameterSnapshot {
            master_gain: Amplitude::try_new(0.3).unwrap(),
        });
        assert!((r.read().master_gain.value() - 0.3).abs() < 1e-9);
    }

    #[test]
    fn bridge_multiple_writes_reader_gets_last() {
        let initial = MixerParameterSnapshot::default();
        let (mut w, mut r) = MixerBridge::split(initial);
        w.write(MixerParameterSnapshot {
            master_gain: Amplitude::try_new(0.1).unwrap(),
        });
        w.write(MixerParameterSnapshot {
            master_gain: Amplitude::try_new(0.9).unwrap(),
        });
        assert!((r.read().master_gain.value() - 0.9).abs() < 1e-9);
    }

    // ── GlobalMixer factory ───────────────────────────────────────────────────

    #[test]
    fn split_reader_returns_initial_gain() {
        let (_writer, mut reader) = GlobalMixer::split(Amplitude::unity());
        assert!((reader.master_gain().value() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn split_writer_propagates_gain_to_reader() {
        let (mut writer, mut reader) = GlobalMixer::split(Amplitude::unity());
        let new_gain = Amplitude::try_new(0.25).unwrap();
        writer
            .handle(GlobalMixerCommand::SetMasterGain { gain: new_gain })
            .unwrap();
        assert!((reader.master_gain().value() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn writer_state_reflects_command() {
        let (mut writer, _reader) = GlobalMixer::split(Amplitude::unity());
        let gain = Amplitude::try_new(0.7).unwrap();
        writer
            .handle(GlobalMixerCommand::SetMasterGain { gain })
            .unwrap();
        assert!((writer.state().master_gain.value() - 0.7).abs() < 1e-9);
    }

    // ── GlobalMixerReader::apply ───────────────────────────────────────────────

    #[test]
    fn apply_scales_frame_by_gain() {
        let gain = Amplitude::try_new(0.5).unwrap();
        let (_writer, mut reader) = GlobalMixer::split(gain);
        let result = reader.apply([1.0_f32, 0.8_f32]);
        assert!((result[0] - 0.5_f32).abs() < 1e-6_f32);
        assert!((result[1] - 0.4_f32).abs() < 1e-6_f32);
    }

    #[test]
    fn apply_silence_gain_mutes_frame() {
        let (_writer, mut reader) = GlobalMixer::split(Amplitude::silence());
        let result = reader.apply([1.0_f32, 1.0_f32]);
        assert!(result[0].abs() < 1e-6_f32);
        assert!(result[1].abs() < 1e-6_f32);
    }

    #[test]
    fn apply_unity_gain_passes_frame_unchanged() {
        let (_writer, mut reader) = GlobalMixer::split(Amplitude::unity());
        let result = reader.apply([0.6_f32, 0.9_f32]);
        assert!((result[0] - 0.6_f32).abs() < 1e-6_f32);
        assert!((result[1] - 0.9_f32).abs() < 1e-6_f32);
    }

    // ── Error display ─────────────────────────────────────────────────────────

    #[test]
    fn error_invalid_gain_displays_non_empty_message() {
        let err = GlobalMixerError::InvalidGain;
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn error_implements_std_error() {
        let err: &dyn std::error::Error = &GlobalMixerError::InvalidGain;
        assert!(!err.to_string().is_empty());
    }
}
