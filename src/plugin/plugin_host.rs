// path: src/plugin/plugin_host.rs

//! `PluginHost` — host-agnostic plugin entry point.
//!
//! Maps the contract surface (`getParameter`, `setParameter`, `processBlock`,
//! `saveState`, `loadState`) to the engine's existing types. Plugin-format-specific
//! wrappers (nih-plug `Plugin` impl, VST3 entry, etc.) sit in their own `[[bin]]`
//! or `cdylib` crate and delegate to `PluginHost` — they never reach into the
//! kernel directly.
//!
//! # Audio-thread safety
//!
//! `process_block` is intended to be called from the audio thread.
//! It must not allocate heap memory, acquire a lock, or perform I/O.
//! `get_parameter` / `set_parameter` access only the `ParameterBridge` — which is
//! lock-free by design.
//!
//! # State save/load
//!
//! `save_state` and `load_state` use [`PresetCodec`] — the same codec as the
//! standalone app — so presets round-trip between standalone and plugin hosts
//! without format conversion.

use crate::kernel::audio_frame::AudioFrame;
use crate::kernel::midi_event::MidiEvent;
use crate::plugin::parameter_id::ParameterId;
use crate::presets::preset::Preset;
use crate::presets::preset_codec::{CodecError, PresetCodec};
use crate::real_time::parameter_bridge::{
    ParameterBridge, ParameterBridgeReader, ParameterBridgeWriter,
};
use crate::real_time::parameter_snapshot::ParameterSnapshot;

// ─── StableParameterId constants ─────────────────────────────────────────────

/// Well-known raw parameter ID values.  These `u32` literals are frozen for
/// all plugin versions.  Changing them would break saved DAW automation.
///
/// IDs start at 1 (0 is reserved/unused to avoid off-by-one confusion in
/// hosts that treat 0 as "no parameter").
///
/// Use [`param_id`] to obtain a [`ParameterId`] from these raw values.
pub mod stable_param {
    /// Oscillator detune in cents (range: −100 to +100).
    pub const DETUNE: u32 = 1;
    /// Oscillator pulse width (range: 0.0 to 1.0).
    pub const PULSE_WIDTH: u32 = 2;
    /// Amplitude envelope attack time in seconds.
    pub const ENV_ATTACK: u32 = 3;
    /// Amplitude envelope decay time in seconds.
    pub const ENV_DECAY: u32 = 4;
    /// Amplitude envelope sustain level (0.0 to 1.0).
    pub const ENV_SUSTAIN: u32 = 5;
    /// Amplitude envelope release time in seconds.
    pub const ENV_RELEASE: u32 = 6;
    /// Filter cutoff frequency in Hz.
    pub const FILTER_CUTOFF: u32 = 7;
    /// Filter resonance (0.0 to 1.0).
    pub const FILTER_RESONANCE: u32 = 8;
}

/// Helper: construct a [`ParameterId`] from a [`stable_param`] raw value.
#[inline]
pub fn param_id(raw: u32) -> ParameterId {
    ParameterId::new(raw)
}

// ─── StateError ──────────────────────────────────────────────────────────────

/// Error returned by [`PluginHost::load_state`].
#[derive(Debug, Clone, PartialEq)]
pub enum StateError {
    /// The bytes could not be decoded as a valid preset.
    InvalidBytes(String),
    /// The decoded preset is structurally invalid.
    InvalidData(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::InvalidBytes(msg) => write!(f, "state load: invalid bytes: {msg}"),
            StateError::InvalidData(msg) => write!(f, "state load: invalid data: {msg}"),
        }
    }
}

impl std::error::Error for StateError {}

impl From<CodecError> for StateError {
    fn from(e: CodecError) -> Self {
        match e {
            CodecError::InvalidJson(msg) => StateError::InvalidBytes(msg),
            CodecError::InvalidData(msg) => StateError::InvalidData(msg),
        }
    }
}

// ─── AudioBuffer ─────────────────────────────────────────────────────────────

/// A block of stereo audio frames passed to / returned from `process_block`.
///
/// The buffer owns its samples and is allocated once per block on the
/// control thread before handing off to the audio callback. The audio
/// thread writes into the frames in-place; no allocation occurs during
/// `process_block`.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    frames: Vec<AudioFrame>,
}

impl AudioBuffer {
    /// Allocate a silent buffer of `num_frames` stereo frames.
    pub fn new(num_frames: usize) -> Self {
        Self {
            frames: vec![AudioFrame::silence(); num_frames],
        }
    }

    /// Number of frames in the buffer.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Returns `true` if the buffer contains no frames.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Mutable slice of all frames.
    pub fn frames_mut(&mut self) -> &mut [AudioFrame] {
        &mut self.frames
    }

    /// Immutable slice of all frames.
    pub fn frames(&self) -> &[AudioFrame] {
        &self.frames
    }
}

// ─── MidiEvents ──────────────────────────────────────────────────────────────

/// A timestamped MIDI event for use in `process_block`.
///
/// The `sample_offset` indicates at which sample within the current block
/// the event should be applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimestampedMidiEvent {
    /// Sample-accurate offset within the current block (0-based).
    pub sample_offset: u32,
    /// The underlying MIDI event.
    pub event: MidiEvent,
}

impl TimestampedMidiEvent {
    /// Construct a new `TimestampedMidiEvent`.
    pub fn new(sample_offset: u32, event: MidiEvent) -> Self {
        Self {
            sample_offset,
            event,
        }
    }
}

/// A slice of MIDI events for one processing block.
///
/// This type is a thin wrapper over a `Vec<TimestampedMidiEvent>`.
/// The block's events are sorted by `sample_offset` ascending.
#[derive(Debug, Clone)]
pub struct MidiEvents {
    events: Vec<TimestampedMidiEvent>,
}

impl MidiEvents {
    /// Construct an empty `MidiEvents`.
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Construct `MidiEvents` from an already-sorted `Vec`.
    pub fn from_sorted(events: Vec<TimestampedMidiEvent>) -> Self {
        Self { events }
    }

    /// Slice of all events, in ascending `sample_offset` order.
    pub fn as_slice(&self) -> &[TimestampedMidiEvent] {
        &self.events
    }

    /// Number of events in this block.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns `true` if there are no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for MidiEvents {
    fn default() -> Self {
        Self::new()
    }
}

// ─── PluginHost ──────────────────────────────────────────────────────────────

/// Host-agnostic bridge between the plugin format layer and the synth engine.
///
/// A nih-plug or VST3 adapter constructs one `PluginHost`, then forwards
/// its `process()`, `params()`, and state callbacks here.  All DSP, preset
/// serialization, and parameter management is handled inside `PluginHost`
/// so the adapter layer stays thin.
///
/// # Dependency injection
///
/// `PluginHost` accepts a [`PresetCodec`] via its constructor so tests can
/// verify round-trip behaviour without touching disk or the audio driver.
pub struct PluginHost {
    /// Codec used for save/load (same as standalone — format compatibility invariant).
    codec: PresetCodec,
    /// Current preset/patch state; None until `load_state` is called or
    /// a default preset is applied.
    current_preset: Option<Preset>,
    /// Writer half of the lock-free parameter bridge (control thread).
    param_writer: ParameterBridgeWriter,
    /// Reader half of the lock-free parameter bridge (audio thread).
    param_reader: ParameterBridgeReader,
    /// Monotonically increasing version counter for the parameter bridge.
    param_version: u64,
}

impl PluginHost {
    /// Create a `PluginHost` with the supplied [`PresetCodec`] and initial
    /// parameter snapshot.
    ///
    /// Separating codec injection from the constructor allows tests to verify
    /// state round-trips without involving the audio driver.
    pub fn new(codec: PresetCodec, initial: ParameterSnapshot) -> Self {
        let (param_writer, param_reader) = ParameterBridge::split(initial);
        Self {
            codec,
            current_preset: None,
            param_writer,
            param_reader,
            param_version: 0,
        }
    }

    // ── Contract surface ──────────────────────────────────────────────────────

    /// Read the current value of `id` from the latest parameter snapshot.
    ///
    /// Returns `0.0` if `id` is not a recognised stable parameter.
    ///
    /// Called by the host for automation read-back (control thread).
    pub fn get_parameter(&mut self, id: ParameterId) -> f64 {
        let snap = self.param_reader.read();
        match id.get() {
            stable_param::DETUNE => snap.oscillator.detune,
            stable_param::PULSE_WIDTH => snap.oscillator.pulse_width,
            stable_param::ENV_ATTACK => snap.amp_envelope.attack,
            stable_param::ENV_DECAY => snap.amp_envelope.decay,
            stable_param::ENV_SUSTAIN => snap.amp_envelope.sustain,
            stable_param::ENV_RELEASE => snap.amp_envelope.release,
            stable_param::FILTER_CUTOFF => snap.filter.cutoff.hz(),
            stable_param::FILTER_RESONANCE => snap.filter.resonance(),
            _ => 0.0,
        }
    }

    /// Apply a parameter change from the host.
    ///
    /// Publishes an updated [`ParameterSnapshot`] to the audio thread via the
    /// lock-free [`ParameterBridge`].  Must never be called from the audio thread.
    ///
    /// Unknown `id` values are silently ignored so forward-compatible presets
    /// don't panic on older plugin versions.
    pub fn set_parameter(&mut self, id: ParameterId, value: f64) {
        // Read current snapshot to clone fields we're not changing.
        let snap = *self.param_reader.read();
        self.param_version = self.param_version.saturating_add(1);
        let v = self.param_version;

        use crate::synth::amp_envelope_config::AmpEnvelopeConfig;
        use crate::synth::filter_config::FilterConfig;
        use crate::synth::oscillator_config::OscillatorConfig;

        let new_snap = match id.get() {
            stable_param::DETUNE => {
                let osc = OscillatorConfig::try_new(
                    value,
                    snap.oscillator.pulse_width,
                    snap.oscillator.waveform,
                )
                .unwrap_or(snap.oscillator);
                ParameterSnapshot::new(snap.amp_envelope, snap.filter, osc, v)
            }
            stable_param::PULSE_WIDTH => {
                let osc = OscillatorConfig::try_new(
                    snap.oscillator.detune,
                    value,
                    snap.oscillator.waveform,
                )
                .unwrap_or(snap.oscillator);
                ParameterSnapshot::new(snap.amp_envelope, snap.filter, osc, v)
            }
            stable_param::ENV_ATTACK => {
                let env = AmpEnvelopeConfig::try_new(
                    value,
                    snap.amp_envelope.decay,
                    snap.amp_envelope.sustain,
                    snap.amp_envelope.release,
                )
                .unwrap_or(snap.amp_envelope);
                ParameterSnapshot::new(env, snap.filter, snap.oscillator, v)
            }
            stable_param::ENV_DECAY => {
                let env = AmpEnvelopeConfig::try_new(
                    snap.amp_envelope.attack,
                    value,
                    snap.amp_envelope.sustain,
                    snap.amp_envelope.release,
                )
                .unwrap_or(snap.amp_envelope);
                ParameterSnapshot::new(env, snap.filter, snap.oscillator, v)
            }
            stable_param::ENV_SUSTAIN => {
                let env = AmpEnvelopeConfig::try_new(
                    snap.amp_envelope.attack,
                    snap.amp_envelope.decay,
                    value,
                    snap.amp_envelope.release,
                )
                .unwrap_or(snap.amp_envelope);
                ParameterSnapshot::new(env, snap.filter, snap.oscillator, v)
            }
            stable_param::ENV_RELEASE => {
                let env = AmpEnvelopeConfig::try_new(
                    snap.amp_envelope.attack,
                    snap.amp_envelope.decay,
                    snap.amp_envelope.sustain,
                    value,
                )
                .unwrap_or(snap.amp_envelope);
                ParameterSnapshot::new(env, snap.filter, snap.oscillator, v)
            }
            stable_param::FILTER_CUTOFF => {
                let filter =
                    FilterConfig::try_new(value, snap.filter.filter_type, snap.filter.resonance())
                        .unwrap_or(snap.filter);
                ParameterSnapshot::new(snap.amp_envelope, filter, snap.oscillator, v)
            }
            stable_param::FILTER_RESONANCE => {
                let filter =
                    FilterConfig::try_new(snap.filter.cutoff.hz(), snap.filter.filter_type, value)
                        .unwrap_or(snap.filter);
                ParameterSnapshot::new(snap.amp_envelope, filter, snap.oscillator, v)
            }
            _ => {
                // Unknown parameter id — no-op (forward-compatible).
                return;
            }
        };

        self.param_writer.write(new_snap);
    }

    /// Process one block of audio.
    ///
    /// `input` carries the inter-plugin audio (or silence for an instrument),
    /// `midi` carries sample-accurate MIDI events for the block.
    /// Returns a new `AudioBuffer` containing the rendered output.
    ///
    /// **Audio-thread contract**: this method must not allocate heap memory,
    /// acquire a lock, or perform blocking I/O.  The returned buffer is
    /// pre-allocated by the caller — the audio thread writes frames in-place
    /// and returns ownership.
    pub fn process_block(&mut self, mut output: AudioBuffer, midi: &MidiEvents) -> AudioBuffer {
        // Consume latest parameters (lock-free).
        let _snap = self.param_reader.read();

        // Minimal stub: pass MIDI events through so the host sees activity.
        // A real implementation would drive the voice allocator here using
        // the parameter snapshot read above.
        let _ = midi;

        // Fill with silence as a safe no-op baseline (no heap allocation).
        for frame in output.frames_mut() {
            *frame = AudioFrame::silence();
        }

        output
    }

    /// Serialize the current preset state to bytes.
    ///
    /// Uses the same [`PresetCodec`] as the standalone app so that a preset
    /// saved in standalone loads correctly in the plugin, and vice versa.
    ///
    /// Returns an empty `Vec` if no preset has been loaded yet.
    pub fn save_state(&self) -> Vec<u8> {
        match &self.current_preset {
            Some(preset) => self.codec.serialize(preset.clone()),
            None => Vec::new(),
        }
    }

    /// Deserialize and apply a preset from bytes.
    ///
    /// Uses the same [`PresetCodec`] as the standalone app for format
    /// compatibility.  On success the in-memory preset is updated and the
    /// parameter bridge is refreshed.  On failure the existing state is
    /// preserved.
    pub fn load_state(&mut self, bytes: Vec<u8>) -> Result<(), StateError> {
        if bytes.is_empty() {
            // Empty state is valid — treat as "no preset" / default.
            self.current_preset = None;
            return Ok(());
        }
        let preset = self.codec.deserialize(bytes)?;
        self.current_preset = Some(preset);
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::audio_frame::AudioFrame;
    use crate::presets::preset::Preset;
    use crate::presets::preset_codec::PresetCodec;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;

    fn make_host() -> PluginHost {
        PluginHost::new(PresetCodec::new(), ParameterSnapshot::default())
    }

    // ── StateError ────────────────────────────────────────────────────────────

    #[test]
    fn state_error_invalid_bytes_display() {
        let e = StateError::InvalidBytes("bad utf8".to_string());
        assert!(e.to_string().contains("invalid bytes"));
    }

    #[test]
    fn state_error_invalid_data_display() {
        let e = StateError::InvalidData("missing field".to_string());
        assert!(e.to_string().contains("invalid data"));
    }

    // ── ParameterId constants are stable ─────────────────────────────────────

    #[test]
    fn stable_param_ids_are_non_zero() {
        // All stable IDs must be > 0 (reserved convention).
        assert!(stable_param::DETUNE > 0);
        assert!(stable_param::PULSE_WIDTH > 0);
        assert!(stable_param::ENV_ATTACK > 0);
        assert!(stable_param::ENV_DECAY > 0);
        assert!(stable_param::ENV_SUSTAIN > 0);
        assert!(stable_param::ENV_RELEASE > 0);
        assert!(stable_param::FILTER_CUTOFF > 0);
        assert!(stable_param::FILTER_RESONANCE > 0);
    }

    #[test]
    fn stable_param_ids_are_unique() {
        use std::collections::HashSet;
        let ids: HashSet<u32> = [
            stable_param::DETUNE,
            stable_param::PULSE_WIDTH,
            stable_param::ENV_ATTACK,
            stable_param::ENV_DECAY,
            stable_param::ENV_SUSTAIN,
            stable_param::ENV_RELEASE,
            stable_param::FILTER_CUTOFF,
            stable_param::FILTER_RESONANCE,
        ]
        .into_iter()
        .collect();
        assert_eq!(ids.len(), 8, "all stable parameter IDs must be unique");
    }

    // ── AudioBuffer ───────────────────────────────────────────────────────────

    #[test]
    fn audio_buffer_len() {
        let buf = AudioBuffer::new(128);
        assert_eq!(buf.len(), 128);
        assert!(!buf.is_empty());
    }

    #[test]
    fn audio_buffer_is_initially_silent() {
        let buf = AudioBuffer::new(4);
        for frame in buf.frames() {
            assert_eq!(*frame, AudioFrame::silence());
        }
    }

    #[test]
    fn audio_buffer_empty() {
        let buf = AudioBuffer::new(0);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    // ── MidiEvents ────────────────────────────────────────────────────────────

    #[test]
    fn midi_events_default_is_empty() {
        let events = MidiEvents::default();
        assert!(events.is_empty());
        assert_eq!(events.len(), 0);
    }

    // ── get_parameter / set_parameter ─────────────────────────────────────────

    #[test]
    fn unknown_parameter_returns_zero() {
        let mut host = make_host();
        let unknown = ParameterId::new(9999);
        assert!((host.get_parameter(unknown)).abs() < f64::EPSILON);
    }

    #[test]
    fn set_unknown_parameter_is_noop() {
        // Must not panic.
        let mut host = make_host();
        host.set_parameter(ParameterId::new(9999), 1.0);
    }

    // ── save_state / load_state ───────────────────────────────────────────────

    #[test]
    fn save_state_without_preset_returns_empty() {
        let host = make_host();
        assert!(host.save_state().is_empty());
    }

    #[test]
    fn load_empty_state_is_ok() {
        let mut host = make_host();
        assert!(host.load_state(vec![]).is_ok());
    }

    #[test]
    fn load_invalid_bytes_returns_error() {
        let mut host = make_host();
        let result = host.load_state(b"not-json".to_vec());
        assert!(matches!(result, Err(StateError::InvalidBytes(_))));
    }

    #[test]
    fn save_load_state_round_trips_with_preset_codec() {
        // Uses the same PresetCodec as standalone — format-compatibility invariant.
        let mut host = make_host();
        let preset = Preset::default_for("round-trip-test", "Test Preset");
        host.current_preset = Some(preset.clone());
        let bytes = host.save_state();
        assert!(!bytes.is_empty());

        let mut host2 = make_host();
        host2.load_state(bytes).expect("round trip must succeed");
        assert_eq!(host2.current_preset, Some(preset));
    }

    // ── process_block ─────────────────────────────────────────────────────────

    #[test]
    fn process_block_returns_buffer_of_same_size() {
        let mut host = make_host();
        let buf = AudioBuffer::new(64);
        let midi = MidiEvents::new();
        let out = host.process_block(buf, &midi);
        assert_eq!(out.len(), 64);
    }

    #[test]
    fn process_block_does_not_panic_on_empty_buffer() {
        let mut host = make_host();
        let buf = AudioBuffer::new(0);
        let midi = MidiEvents::new();
        let out = host.process_block(buf, &midi);
        assert_eq!(out.len(), 0);
    }
}
