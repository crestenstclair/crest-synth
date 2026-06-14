// path: src/plugin/plugin_shell.rs
//
// PluginShell — orchestrates plugin lifecycle: init, process, param sync, and
// state persistence via a host-provided callback interface.
//
// # Design
//
// `PluginShell` is the top-level object that a DAW plugin wrapper (e.g. a
// future VST3 / CLAP adapter) instantiates once per plugin instance. It is
// entirely host-agnostic: no audio driver code, no window system, no controller
// code lives here. Those concerns belong in shell adapters.
//
// ## Real-time seam
//
// All parameter changes flow through `ParameterBridge` (triple-buffer, lock-free).
// All event-style messages (MIDI, transport) flow through `EventRingBuffer` (SPSC,
// lock-free). The audio-thread render path reads both without any allocation
// or mutex.
//
// ## State persistence
//
// `PluginShell` uses `PresetCodec` for all serialization — the same codec the
// standalone app uses, so presets are format-compatible.
//
// ## Parameters
//
// Plugin parameters are identified by stable `ParameterId` — a `u32` that
// never changes across versions, preserving host automation.
//
// ## Signal flow
//
// MIDI → voice pool → per-patch FX → mix bus → master FX → output
// (effect chains process in slot order, per the architecture invariant)

use crate::plugin::parameter_id::ParameterId;
use crate::plugin::plugin_parameter::PluginParameter;
use crate::presets::preset::Preset;
use crate::presets::preset_codec::{CodecError, PresetCodec};
use crate::presets::setup::Setup;
use crate::real_time::boundary_message::{BoundaryMessage, BoundaryMessageKind};
use crate::real_time::deferred_deallocator::{deferred_deallocator, CollectHandle, RetireHandle};
use crate::real_time::event_ring_buffer::EventRingBuffer;
use crate::real_time::parameter_bridge::{
    ParameterBridge, ParameterBridgeReader, ParameterBridgeWriter,
};
use crate::real_time::parameter_snapshot::ParameterSnapshot;

// ─── Errors ───────────────────────────────────────────────────────────────────

/// Errors that can arise during plugin shell operations.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginShellError {
    /// State bytes could not be serialized or deserialized.
    Codec(CodecError),
    /// The caller passed empty state bytes where non-empty bytes were required.
    EmptyState,
}

impl std::fmt::Display for PluginShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginShellError::Codec(e) => write!(f, "plugin shell codec error: {e}"),
            PluginShellError::EmptyState => write!(f, "plugin shell: state bytes are empty"),
        }
    }
}

impl std::error::Error for PluginShellError {}

impl From<CodecError> for PluginShellError {
    fn from(e: CodecError) -> Self {
        PluginShellError::Codec(e)
    }
}

// ─── PluginShell ─────────────────────────────────────────────────────────────

/// Capacity of the event ring buffer (MIDI + transport messages per callback).
const EVENT_BUFFER_CAPACITY: usize = 256;

/// Orchestrates plugin lifecycle: init, process, param sync, and state
/// persistence via `PresetCodec`.
///
/// # Responsibilities
///
/// - Owns the lock-free real-time seam (`ParameterBridge`, `EventRingBuffer`,
///   `DeferredDeallocator`).
/// - Routes parameter changes from the host (non-RT) to the audio thread (RT)
///   through `ParameterBridge` and `EventRingBuffer`.
/// - Routes MIDI / transport events to the audio thread through
///   `EventRingBuffer`.
/// - Saves and loads plugin state via `PresetCodec`, ensuring format
///   compatibility with the standalone app.
/// - Maintains the list of [`PluginParameter`]s with their stable
///   [`ParameterId`]s — IDs are fixed for the lifetime of the plugin format.
///
/// # Audio-thread contract
///
/// Methods documented as "RT-safe" may be called from the audio callback.
/// All other methods are control-thread only.
///
/// # Signal flow
///
/// MIDI → voice pool → per-patch FX → mix bus → master FX → output
pub struct PluginShell {
    /// Codec for preset and setup serialization.
    codec: PresetCodec,

    /// Ordered list of host-visible parameters with stable IDs.
    parameters: Vec<PluginParameter>,

    /// Writer side of the parameter bridge (control thread).
    param_writer: ParameterBridgeWriter,

    /// Reader side of the parameter bridge (audio thread).
    param_reader: ParameterBridgeReader,

    /// Event ring buffer for crossing the RT boundary.
    event_buffer: EventRingBuffer,

    /// Audio-thread side of the deferred deallocator.
    retire_handle: RetireHandle,

    /// Control-thread side of the deferred deallocator (collect on idle tick).
    collect_handle: CollectHandle,

    /// Current parameter snapshot (control-thread copy).
    current_snapshot: ParameterSnapshot,

    /// Monotonically increasing sequence number for boundary messages.
    sequence: u64,
}

impl PluginShell {
    /// Construct a new `PluginShell` with the given `codec` and the supplied
    /// list of `parameters`.
    ///
    /// Parameter IDs must be stable across versions. The caller is responsible
    /// for never renumbering or removing IDs from the list.
    pub fn new(codec: PresetCodec, parameters: Vec<PluginParameter>) -> Self {
        let initial = ParameterSnapshot::default();
        let (param_writer, param_reader) = ParameterBridge::split(initial);
        let event_buffer = EventRingBuffer::new(EVENT_BUFFER_CAPACITY);
        let (retire_handle, collect_handle) = deferred_deallocator();
        Self {
            codec,
            parameters,
            param_writer,
            param_reader,
            event_buffer,
            retire_handle,
            collect_handle,
            current_snapshot: initial,
            sequence: 0,
        }
    }

    /// Construct a `PluginShell` with default parameters and a fresh codec.
    ///
    /// Convenience for callers that do not need to inject dependencies.
    pub fn with_defaults() -> Self {
        Self::new(PresetCodec::new(), Vec::new())
    }

    // ── Parameter registry ────────────────────────────────────────────────────

    /// Return a slice of all registered [`PluginParameter`]s.
    ///
    /// The parameters are in registration order and their IDs are stable.
    pub fn parameters(&self) -> &[PluginParameter] {
        &self.parameters
    }

    /// Look up a parameter by its stable [`ParameterId`].
    ///
    /// Returns `None` if no parameter with that ID is registered.
    pub fn parameter(&self, id: ParameterId) -> Option<&PluginParameter> {
        self.parameters.iter().find(|p| p.id() == id)
    }

    // ── Parameter sync ────────────────────────────────────────────────────────

    /// Notify the shell that the host has set parameter `id` to `value`.
    ///
    /// The change is encoded as a `ParameterChange` boundary message (4-byte
    /// little-endian `u32` param ID followed by 4-byte little-endian `f32`
    /// value) and queued for the audio thread via `EventRingBuffer`.
    ///
    /// **Control-thread only.** Never call from the audio callback.
    pub fn set_parameter(&mut self, id: ParameterId, value: f32) {
        // Encode: [param_id: u32 LE][value: f32 LE]
        let mut payload = [0u8; 8];
        payload[0..4].copy_from_slice(&id.get().to_le_bytes());
        payload[4..8].copy_from_slice(&value.to_le_bytes());

        let seq = self.next_sequence();
        let msg = BoundaryMessage::new(BoundaryMessageKind::ParameterChange, payload.to_vec(), seq);
        // Best-effort: if the ring buffer is full the audio thread has not
        // consumed recent events. The change is dropped — not a panic.
        let _ = self.event_buffer.push(msg);
    }

    /// Publish a [`ParameterSnapshot`] to the audio thread via the lock-free
    /// triple-buffer bridge.
    ///
    /// Call this after one or more `set_parameter` calls to deliver the latest
    /// values to the audio thread on the next render callback.
    ///
    /// **Control-thread only.**
    pub fn flush_parameters(&mut self, snapshot: ParameterSnapshot) {
        self.current_snapshot = snapshot;
        self.param_writer.write(snapshot);
    }

    // ── MIDI / transport routing ──────────────────────────────────────────────

    /// Send a raw MIDI event to the audio thread.
    ///
    /// `bytes` — raw MIDI bytes (status + up to two data bytes).
    ///
    /// **Control-thread only.**
    pub fn send_midi(&mut self, bytes: &[u8]) {
        let seq = self.next_sequence();
        let msg = BoundaryMessage::new(BoundaryMessageKind::MidiEvent, bytes.to_vec(), seq);
        let _ = self.event_buffer.push(msg);
    }

    /// Send a transport command to the audio thread.
    ///
    /// `cmd` — single-byte transport command (0 = stop, 1 = play, etc.).
    ///
    /// **Control-thread only.**
    pub fn send_transport_command(&mut self, cmd: u8) {
        let seq = self.next_sequence();
        let msg = BoundaryMessage::new(BoundaryMessageKind::TransportCommand, vec![cmd], seq);
        let _ = self.event_buffer.push(msg);
    }

    // ── RT-side accessors ─────────────────────────────────────────────────────

    /// Poll the latest parameter snapshot from the triple-buffer bridge.
    ///
    /// **RT-safe.** Reads lock-free without allocating.
    #[inline]
    pub fn poll_parameters(&mut self) -> &ParameterSnapshot {
        self.param_reader.read()
    }

    /// Pop the next queued boundary event, if any.
    ///
    /// Returns `None` when the queue is empty. After processing the payload,
    /// hand the message to [`retire_message`](PluginShell::retire_message) so
    /// that the heap-allocated payload bytes are freed off the audio thread.
    ///
    /// **RT-safe.**
    #[inline]
    pub fn pop_event(&mut self) -> Option<BoundaryMessage> {
        self.event_buffer.pop()
    }

    /// Hand a processed [`BoundaryMessage`] to the deferred deallocator.
    ///
    /// The payload `Vec<u8>` inside the message will be freed on the next
    /// [`collect_garbage`](PluginShell::collect_garbage) call from the control
    /// thread, not here — preserving the audio thread's heap-allocation-free
    /// contract.
    ///
    /// **RT-safe** (no allocation, no blocking, no mutex on the fast path).
    pub fn retire_message(&mut self, msg: BoundaryMessage) {
        use std::sync::Arc;
        self.retire_handle.retire(Arc::new(msg));
    }

    /// Drain the deferred-deallocator queue and free retired memory.
    ///
    /// Call this periodically from the control thread (e.g. on a 50 ms timer
    /// or after each render cycle).
    ///
    /// **Control-thread only.**
    pub fn collect_garbage(&mut self) {
        self.collect_handle.collect();
    }

    // ── State persistence ─────────────────────────────────────────────────────

    /// Serialize a [`Preset`] to bytes using the same codec as the standalone
    /// app, ensuring format compatibility.
    ///
    /// The returned bytes can be handed to the DAW host for state persistence
    /// and later restored via [`load_preset_state`].
    ///
    /// **Control-thread only** (allocates).
    ///
    /// [`load_preset_state`]: PluginShell::load_preset_state
    pub fn save_preset_state(&self, preset: Preset) -> Vec<u8> {
        self.codec.serialize(preset)
    }

    /// Deserialize a [`Preset`] from bytes previously produced by
    /// [`save_preset_state`].
    ///
    /// Returns `Err(PluginShellError::EmptyState)` if `bytes` is empty.
    /// Returns `Err(PluginShellError::Codec(_))` for malformed data.
    ///
    /// **Control-thread only** (allocates).
    ///
    /// [`save_preset_state`]: PluginShell::save_preset_state
    pub fn load_preset_state(&self, bytes: Vec<u8>) -> Result<Preset, PluginShellError> {
        if bytes.is_empty() {
            return Err(PluginShellError::EmptyState);
        }
        self.codec
            .deserialize(bytes)
            .map_err(PluginShellError::from)
    }

    /// Serialize a full [`Setup`] (all patches, subscriptions, mixer, effects)
    /// to bytes using the same codec as the standalone app.
    ///
    /// **Control-thread only** (allocates).
    pub fn save_setup_state(&self, setup: Setup) -> Vec<u8> {
        self.codec.serialize_setup(setup)
    }

    /// Deserialize a [`Setup`] from bytes previously produced by
    /// [`save_setup_state`].
    ///
    /// Returns `Err(PluginShellError::EmptyState)` if `bytes` is empty.
    /// Returns `Err(PluginShellError::Codec(_))` for malformed data.
    ///
    /// **Control-thread only** (allocates).
    ///
    /// [`save_setup_state`]: PluginShell::save_setup_state
    pub fn load_setup_state(&self, bytes: Vec<u8>) -> Result<Setup, PluginShellError> {
        if bytes.is_empty() {
            return Err(PluginShellError::EmptyState);
        }
        self.codec
            .deserialize_setup(bytes)
            .map_err(PluginShellError::from)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn next_sequence(&mut self) -> u64 {
        let s = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);
        s
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::parameter_range::ParameterRange;
    use crate::presets::preset::Preset;
    use crate::presets::preset_metadata::PresetMetadata;
    use crate::presets::setup::Setup;
    use crate::real_time::boundary_message::BoundaryMessageKind;

    fn make_range() -> ParameterRange {
        ParameterRange::try_new(0.0, 1.0, 0.5, None).unwrap()
    }

    fn make_param(id: u32, mapping: &str) -> PluginParameter {
        PluginParameter::new(
            ParameterId::new(id),
            format!("Param {id}"),
            mapping.to_string(),
            make_range(),
        )
    }

    fn make_shell() -> PluginShell {
        let params = vec![
            make_param(0, "master.gain"),
            make_param(1, "osc.detune"),
            make_param(2, "osc.pulse_width"),
            make_param(3, "env.attack"),
            make_param(4, "env.decay"),
            make_param(5, "env.sustain"),
            make_param(6, "env.release"),
            make_param(7, "filter.cutoff_hz"),
            make_param(8, "filter.resonance"),
        ];
        PluginShell::new(PresetCodec::new(), params)
    }

    fn default_preset(id: &str) -> Preset {
        Preset::default_for(id, "Test Preset")
    }

    // ── Construction ──────────────────────────────────────────────────────────

    #[test]
    fn new_shell_constructs_successfully() {
        let _shell = make_shell();
    }

    #[test]
    fn with_defaults_constructs_successfully() {
        let _shell = PluginShell::with_defaults();
    }

    // ── Parameter registry ────────────────────────────────────────────────────

    #[test]
    fn parameters_returns_all_registered_params() {
        let shell = make_shell();
        assert_eq!(shell.parameters().len(), 9);
    }

    #[test]
    fn parameter_ids_are_unique_and_stable() {
        use std::collections::HashSet;
        let shell = make_shell();
        let ids: HashSet<u32> = shell.parameters().iter().map(|p| p.id().get()).collect();
        assert_eq!(
            ids.len(),
            shell.parameters().len(),
            "every parameter must have a unique stable ID"
        );
    }

    #[test]
    fn parameter_lookup_by_id_succeeds() {
        let shell = make_shell();
        let p = shell
            .parameter(ParameterId::new(7))
            .expect("param 7 should exist");
        assert_eq!(p.engine_mapping(), "filter.cutoff_hz");
    }

    #[test]
    fn parameter_lookup_missing_id_returns_none() {
        let shell = make_shell();
        assert!(shell.parameter(ParameterId::new(99)).is_none());
    }

    // ── Parameter routing ─────────────────────────────────────────────────────

    #[test]
    fn set_parameter_enqueues_parameter_change_event() {
        let mut shell = make_shell();
        shell.set_parameter(ParameterId::new(0), 0.75);
        let msg = shell.pop_event().expect("event should be queued");
        assert_eq!(msg.kind, BoundaryMessageKind::ParameterChange);
        assert_eq!(msg.payload.len(), 8);
        let id = u32::from_le_bytes(msg.payload[0..4].try_into().unwrap());
        let val = f32::from_le_bytes(msg.payload[4..8].try_into().unwrap());
        assert_eq!(id, 0);
        assert!((val - 0.75_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn multiple_parameter_changes_arrive_in_fifo_order() {
        let mut shell = make_shell();
        shell.set_parameter(ParameterId::new(3), 0.1); // env.attack
        shell.set_parameter(ParameterId::new(6), 0.9); // env.release

        let msg1 = shell.pop_event().unwrap();
        let msg2 = shell.pop_event().unwrap();

        let id1 = u32::from_le_bytes(msg1.payload[0..4].try_into().unwrap());
        let id2 = u32::from_le_bytes(msg2.payload[0..4].try_into().unwrap());

        assert_eq!(id1, 3);
        assert_eq!(id2, 6);
    }

    #[test]
    fn flush_parameters_publishes_snapshot_to_audio_thread() {
        let mut shell = make_shell();
        let snap = ParameterSnapshot::default();
        shell.flush_parameters(snap);
        let read_snap = shell.poll_parameters();
        assert_eq!(*read_snap, snap);
    }

    // ── MIDI routing ──────────────────────────────────────────────────────────

    #[test]
    fn send_midi_enqueues_midi_event() {
        let mut shell = make_shell();
        shell.send_midi(&[0x90, 0x3C, 0x7F]); // note-on C4 vel=127
        let msg = shell.pop_event().expect("midi event should be queued");
        assert_eq!(msg.kind, BoundaryMessageKind::MidiEvent);
        assert_eq!(msg.payload, &[0x90u8, 0x3C, 0x7F]);
    }

    #[test]
    fn send_transport_command_enqueues_transport_event() {
        let mut shell = make_shell();
        shell.send_transport_command(1); // play
        let msg = shell.pop_event().unwrap();
        assert_eq!(msg.kind, BoundaryMessageKind::TransportCommand);
        assert_eq!(msg.payload, &[1u8]);
    }

    #[test]
    fn pop_event_returns_none_when_queue_empty() {
        let mut shell = make_shell();
        assert_eq!(shell.pop_event(), None);
    }

    // ── Sequence numbers ──────────────────────────────────────────────────────

    #[test]
    fn sequence_numbers_increase_across_events() {
        let mut shell = make_shell();
        shell.send_midi(&[0x80, 0x3C, 0]);
        shell.set_parameter(ParameterId::new(0), 1.0);
        shell.send_transport_command(0);

        let msgs: Vec<_> = (0..3).filter_map(|_| shell.pop_event()).collect();
        assert_eq!(msgs.len(), 3);
        assert!(msgs[0].sequence_number < msgs[1].sequence_number);
        assert!(msgs[1].sequence_number < msgs[2].sequence_number);
    }

    // ── Retire / collect ──────────────────────────────────────────────────────

    #[test]
    fn retire_and_collect_does_not_panic() {
        let mut shell = make_shell();
        shell.send_midi(&[0x90, 0x40, 0x64]);
        let msg = shell.pop_event().unwrap();
        shell.retire_message(msg);
        shell.collect_garbage();
    }

    // ── Preset state persistence ──────────────────────────────────────────────

    #[test]
    fn save_and_load_preset_state_round_trip() {
        let shell = make_shell();
        let original = default_preset("test-preset");
        let bytes = shell.save_preset_state(original.clone());
        let restored = shell.load_preset_state(bytes).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn load_preset_state_empty_bytes_returns_error() {
        let shell = make_shell();
        let result = shell.load_preset_state(vec![]);
        assert!(matches!(result, Err(PluginShellError::EmptyState)));
    }

    #[test]
    fn load_preset_state_invalid_bytes_returns_codec_error() {
        let shell = make_shell();
        let result = shell.load_preset_state(b"not json".to_vec());
        assert!(matches!(result, Err(PluginShellError::Codec(_))));
    }

    #[test]
    fn preset_state_bytes_are_valid_utf8() {
        let shell = make_shell();
        let bytes = shell.save_preset_state(default_preset("utf8-test"));
        assert!(std::str::from_utf8(&bytes).is_ok());
    }

    #[test]
    fn preset_state_captures_metadata_name() {
        let shell = make_shell();
        let mut preset = default_preset("name-test");
        preset.metadata =
            PresetMetadata::new("Warm Pad", "Alice", "Pad", "2025-01-01T00:00:00Z", vec![]);
        let bytes = shell.save_preset_state(preset);
        let json = std::str::from_utf8(&bytes).unwrap();
        assert!(json.contains("Warm Pad"));
    }

    // ── Setup state persistence ───────────────────────────────────────────────

    #[test]
    fn save_and_load_setup_state_round_trip() {
        let shell = make_shell();
        let mut setup = Setup::new("My Session");
        setup.master_gain = 0.8;
        let bytes = shell.save_setup_state(setup.clone());
        let restored = shell.load_setup_state(bytes).unwrap();
        assert_eq!(restored.name, "My Session");
        assert!((restored.master_gain - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn load_setup_state_empty_bytes_returns_error() {
        let shell = make_shell();
        let result = shell.load_setup_state(vec![]);
        assert!(matches!(result, Err(PluginShellError::EmptyState)));
    }

    #[test]
    fn load_setup_state_invalid_bytes_returns_codec_error() {
        let shell = make_shell();
        let result = shell.load_setup_state(b"bad".to_vec());
        assert!(matches!(result, Err(PluginShellError::Codec(_))));
    }

    #[test]
    fn setup_state_captures_master_gain_zero() {
        let shell = make_shell();
        let mut setup = Setup::new("Muted");
        setup.master_gain = 0.0;
        let bytes = shell.save_setup_state(setup);
        let restored = shell.load_setup_state(bytes).unwrap();
        assert!((restored.master_gain - 0.0).abs() < f64::EPSILON);
    }

    // ── PluginShellError display ───────────────────────────────────────────────

    #[test]
    fn empty_state_error_display() {
        let e = PluginShellError::EmptyState;
        assert!(!e.to_string().is_empty());
        assert!(e.to_string().contains("empty"));
    }

    #[test]
    fn codec_error_display_propagates() {
        let inner = CodecError::InvalidJson("eof".to_string());
        let e = PluginShellError::Codec(inner);
        assert!(e.to_string().contains("codec"));
    }
}
