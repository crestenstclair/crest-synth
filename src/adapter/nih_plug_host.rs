// path: src/adapter/nih_plug_host.rs

//! `NihPlugHost` — adapter that wraps [`PluginHost`] for use with the
//! nih-plug CLAP/VST3 plugin framework.
//!
//! # Design
//!
//! `NihPlugHost` is a thin infrastructure adapter.  It owns a [`PluginHost`]
//! and translates the nih-plug callback surface (initialize, process, state,
//! parameters) into calls on the engine-library types already defined in
//! `plugin::plugin_host`.
//!
//! The adapter deliberately does **not** depend on the `nih-plug` crate
//! directly.  A real CLAP/VST3 build lives in a separate `cdylib` crate that
//! derives `nih_plug::Plugin` and delegates every callback here.  This keeps
//! the engine library host-agnostic and the adapter testable without a plugin
//! host binary.
//!
//! # Audio-thread contract
//!
//! `process_block` must not allocate heap memory, acquire a lock, or perform
//! blocking I/O — the same constraints as [`PluginHost::process_block`].
//! `NihPlugHost` adds no allocation of its own inside the hot path.
//!
//! # State persistence
//!
//! State serialization uses the same [`PresetCodec`] as the standalone app,
//! maintaining format compatibility between standalone and plugin instances.

use crate::plugin::parameter_id::ParameterId;
use crate::plugin::plugin_host::{AudioBuffer, MidiEvents, PluginHost, StateError};
use crate::presets::preset_codec::PresetCodec;
use crate::real_time::parameter_snapshot::ParameterSnapshot;

// ─── NihPlugHost ─────────────────────────────────────────────────────────────

/// Adapter that maps nih-plug callback semantics onto [`PluginHost`].
///
/// A nih-plug `Plugin` implementation in a separate cdylib crate holds one
/// `NihPlugHost` and delegates its required callbacks here.
///
/// # Dependency injection
///
/// Both [`PresetCodec`] and the initial [`ParameterSnapshot`] are injected
/// via the constructor, keeping the adapter testable without an audio driver.
/// Use [`NihPlugHost::with_defaults`] when injection is not needed.
pub struct NihPlugHost {
    inner: PluginHost,
}

impl NihPlugHost {
    /// Create a `NihPlugHost` with an injected [`PresetCodec`] and initial
    /// [`ParameterSnapshot`].
    ///
    /// Prefer this constructor in tests so that codec behaviour can be
    /// verified without touching disk.
    pub fn new(codec: PresetCodec, initial: ParameterSnapshot) -> Self {
        Self {
            inner: PluginHost::new(codec, initial),
        }
    }

    /// Create a `NihPlugHost` with default codec and default parameters.
    ///
    /// Convenience constructor for production plugin entry points.
    pub fn with_defaults() -> Self {
        Self::new(PresetCodec::new(), ParameterSnapshot::default())
    }

    // ── Contract surface ──────────────────────────────────────────────────────

    /// Process one block of audio.
    ///
    /// Delegates to [`PluginHost::process_block`].  The caller pre-allocates
    /// `output` on the control thread; the audio thread writes frames in-place.
    ///
    /// **Audio-thread contract**: no heap allocation, no lock, no blocking I/O.
    pub fn process_block(&mut self, output: AudioBuffer, midi: &MidiEvents) -> AudioBuffer {
        self.inner.process_block(output, midi)
    }

    /// Read the current normalised value of a plugin parameter.
    ///
    /// Returns `0.0` for unknown parameter IDs.
    pub fn get_parameter(&mut self, id: ParameterId) -> f64 {
        self.inner.get_parameter(id)
    }

    /// Apply a parameter change from the host.
    ///
    /// Writes through the lock-free [`ParameterBridge`] so the audio thread
    /// picks up the new value on its next block.  Must only be called from
    /// the control thread.
    pub fn set_parameter(&mut self, id: ParameterId, value: f64) {
        self.inner.set_parameter(id, value);
    }

    /// Serialize the current plugin state to bytes.
    ///
    /// Delegates to [`PluginHost::save_state`], which uses the same
    /// [`PresetCodec`] as the standalone app for format compatibility.
    pub fn save_state(&self) -> Vec<u8> {
        self.inner.save_state()
    }

    /// Deserialize and apply plugin state from bytes.
    ///
    /// Delegates to [`PluginHost::load_state`].  On failure the existing state
    /// is preserved and a [`StateError`] is returned.
    pub fn load_state(&mut self, bytes: Vec<u8>) -> Result<(), StateError> {
        self.inner.load_state(bytes)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::audio_frame::AudioFrame;
    use crate::plugin::plugin_host::{stable_param, AudioBuffer, MidiEvents};
    use crate::presets::preset::Preset;
    use crate::presets::preset_codec::PresetCodec;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;

    fn make_host() -> NihPlugHost {
        NihPlugHost::new(PresetCodec::new(), ParameterSnapshot::default())
    }

    // ── Construction ──────────────────────────────────────────────────────────

    #[test]
    fn with_defaults_constructs_without_panic() {
        let _host = NihPlugHost::with_defaults();
    }

    #[test]
    fn new_with_injected_codec_constructs_without_panic() {
        let _host = make_host();
    }

    // ── get_parameter ─────────────────────────────────────────────────────────

    #[test]
    fn unknown_parameter_returns_zero() {
        let mut host = make_host();
        let id = ParameterId::new(9999);
        assert!((host.get_parameter(id)).abs() < f64::EPSILON);
    }

    #[test]
    fn get_known_parameter_returns_non_zero_for_filter_cutoff() {
        let mut host = make_host();
        // Filter cutoff has a sensible non-zero default.
        let id = ParameterId::new(stable_param::FILTER_CUTOFF);
        let value = host.get_parameter(id);
        // Default cutoff is > 0 Hz.
        assert!(value > 0.0, "filter cutoff must be positive, got {value}");
    }

    // ── set_parameter ─────────────────────────────────────────────────────────

    #[test]
    fn set_unknown_parameter_is_noop() {
        let mut host = make_host();
        // Must not panic.
        host.set_parameter(ParameterId::new(9999), 1.0);
    }

    #[test]
    fn set_and_get_detune_round_trips() {
        let mut host = make_host();
        let id = ParameterId::new(stable_param::DETUNE);
        host.set_parameter(id, 50.0);
        let got = host.get_parameter(id);
        assert!(
            (got - 50.0).abs() < f64::EPSILON,
            "expected detune=50.0, got {got}"
        );
    }

    // ── save_state / load_state ───────────────────────────────────────────────

    #[test]
    fn save_state_without_preset_is_empty() {
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
    fn save_load_round_trips_preset() {
        // Verify format-compatibility invariant: same codec as standalone.
        // Serialize a preset using the same PresetCodec, then load it.
        let codec = PresetCodec::new();
        let preset = Preset::default_for("nih-plug-test", "NihPlug Test Preset");
        let bytes = codec.serialize(preset);

        let mut host = make_host();
        host.load_state(bytes.clone()).expect("load must succeed");

        // save_state must return non-empty bytes after a successful load.
        let saved = host.save_state();
        assert!(!saved.is_empty(), "save_state must be non-empty after load");

        // And the saved bytes must load again without error.
        let mut host2 = make_host();
        host2
            .load_state(saved)
            .expect("second round trip must succeed");
        let saved2 = host2.save_state();
        assert!(!saved2.is_empty());
    }

    // ── process_block ─────────────────────────────────────────────────────────

    #[test]
    fn process_block_returns_same_size_buffer() {
        let mut host = make_host();
        let buf = AudioBuffer::new(128);
        let midi = MidiEvents::new();
        let out = host.process_block(buf, &midi);
        assert_eq!(out.len(), 128);
    }

    #[test]
    fn process_block_does_not_panic_on_empty_buffer() {
        let mut host = make_host();
        let buf = AudioBuffer::new(0);
        let midi = MidiEvents::new();
        let out = host.process_block(buf, &midi);
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn process_block_output_is_silent_baseline() {
        let mut host = make_host();
        let buf = AudioBuffer::new(4);
        let midi = MidiEvents::new();
        let out = host.process_block(buf, &midi);
        for frame in out.frames() {
            assert_eq!(*frame, AudioFrame::silence());
        }
    }
}
