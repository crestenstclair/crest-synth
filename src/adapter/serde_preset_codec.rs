// path: src/adapter/serde_preset_codec.rs

//! `SerdePresetCodec` — infrastructure-layer adapter that serialises and
//! deserialises [`Preset`] and [`Setup`] values.
//!
//! # Format
//!
//! Uses UTF-8 JSON for [`Preset`] (human-readable, fully round-trippable) and
//! delegates to the same codec for [`Setup`].  Both formats are provided by the
//! `PresetCodec` domain type in the `presets` module.
//!
//! # Audio-thread safety
//!
//! `SerdePresetCodec` must **never** be used on the audio thread.
//! Serialisation/deserialisation allocate heap memory. All codec operations
//! belong on the control / UI thread.

use crate::presets::preset::Preset;
pub use crate::presets::preset_codec::CodecError;
use crate::presets::preset_codec::PresetCodec;
use crate::presets::setup::Setup;

// ─────────────────────────────────────────────────────────────────────────────
// SerdePresetCodec
// ─────────────────────────────────────────────────────────────────────────────

/// Infrastructure-layer codec that serialises and deserialises [`Preset`] and
/// [`Setup`] values.
///
/// `SerdePresetCodec` is the adapter-layer wrapper over [`PresetCodec`].  It
/// fulfils the `port.Presets.PresetCodec` contract with serde_json-compatible
/// UTF-8 JSON output for presets.
///
/// | method               | contract                                          |
/// |----------------------|---------------------------------------------------|
/// | `serialize`          | `Preset → Vec<u8>`                                |
/// | `deserialize`        | `Vec<u8> → Result<Preset, CodecError>`            |
/// | `serialize_setup`    | `Setup → Vec<u8>`                                 |
/// | `deserialize_setup`  | `Vec<u8> → Result<Setup, CodecError>`             |
///
/// `SerdePresetCodec` holds no mutable state and is safe to construct once
/// and reuse across the lifetime of the application.
pub struct SerdePresetCodec {
    inner: PresetCodec,
}

impl SerdePresetCodec {
    /// Create a new `SerdePresetCodec` backed by the default [`PresetCodec`].
    pub fn new() -> Self {
        Self {
            inner: PresetCodec::new(),
        }
    }

    // ── Preset ────────────────────────────────────────────────────────────────

    /// Serialise a [`Preset`] to UTF-8 JSON bytes.
    ///
    /// The resulting bytes can be written to disk, sent over a network, or
    /// stored in a database and later restored via [`SerdePresetCodec::deserialize`].
    pub fn serialize(&self, preset: Preset) -> Vec<u8> {
        self.inner.serialize(preset)
    }

    /// Deserialise a [`Preset`] from UTF-8 JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::InvalidJson`] when `bytes` cannot be parsed as
    /// valid UTF-8 JSON or when a required field is missing.
    ///
    /// Returns [`CodecError::InvalidData`] when a field value violates an
    /// invariant (e.g. resonance out of range, unknown engine type).
    pub fn deserialize(&self, bytes: Vec<u8>) -> Result<Preset, CodecError> {
        self.inner.deserialize(bytes)
    }

    // ── Setup ─────────────────────────────────────────────────────────────────

    /// Serialise a [`Setup`] to UTF-8 JSON bytes.
    pub fn serialize_setup(&self, setup: Setup) -> Vec<u8> {
        self.inner.serialize_setup(setup)
    }

    /// Deserialise a [`Setup`] from UTF-8 JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::InvalidJson`] when `bytes` cannot be parsed as
    /// valid UTF-8 JSON or the structure does not match `Setup`.
    pub fn deserialize_setup(&self, bytes: Vec<u8>) -> Result<Setup, CodecError> {
        self.inner.deserialize_setup(bytes)
    }
}

impl Default for SerdePresetCodec {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod serde_preset_codec {
    use super::*;
    use crate::presets::preset::Preset;
    use crate::presets::setup::Setup;

    fn default_preset(id: &str) -> Preset {
        Preset::default_for(id, "Test Preset")
    }

    // ── Preset round-trip ─────────────────────────────────────────────────────

    #[test]
    fn preset_round_trip() {
        let codec = SerdePresetCodec::new();
        let original = default_preset("round-trip-test");
        let bytes = codec.serialize(original.clone());
        let restored = codec.deserialize(bytes).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn preset_output_is_valid_utf8() {
        let codec = SerdePresetCodec::new();
        let bytes = codec.serialize(default_preset("utf8-test"));
        assert!(
            std::str::from_utf8(&bytes).is_ok(),
            "serialised preset must be valid UTF-8"
        );
    }

    #[test]
    fn preset_output_contains_name() {
        let codec = SerdePresetCodec::new();
        let mut preset = default_preset("name-test");
        preset.metadata.name = "Bright Pad".to_string();
        let bytes = codec.serialize(preset);
        let json = std::str::from_utf8(&bytes).unwrap();
        assert!(
            json.contains("Bright Pad"),
            "output must include the preset name"
        );
    }

    #[test]
    fn preset_deserialize_invalid_bytes_errors() {
        let codec = SerdePresetCodec::new();
        let result = codec.deserialize(b"not valid json".to_vec());
        assert!(matches!(result, Err(CodecError::InvalidJson(_))));
    }

    #[test]
    fn preset_deserialize_empty_bytes_errors() {
        let codec = SerdePresetCodec::new();
        let result = codec.deserialize(vec![]);
        assert!(matches!(result, Err(CodecError::InvalidJson(_))));
    }

    // ── Setup round-trip ──────────────────────────────────────────────────────

    #[test]
    fn setup_round_trip() {
        let codec = SerdePresetCodec::new();
        let mut setup = Setup::new("My Session");
        setup.master_gain = 0.8;
        let bytes = codec.serialize_setup(setup.clone());
        let restored = codec.deserialize_setup(bytes).unwrap();
        assert_eq!(restored.name, "My Session");
        assert!((restored.master_gain - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn setup_deserialize_invalid_bytes_errors() {
        let codec = SerdePresetCodec::new();
        let result = codec.deserialize_setup(b"garbage".to_vec());
        assert!(matches!(result, Err(CodecError::InvalidJson(_))));
    }

    // ── Default ───────────────────────────────────────────────────────────────

    #[test]
    fn default_creates_codec() {
        let _codec = SerdePresetCodec::default();
    }
}
