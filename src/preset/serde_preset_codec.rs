// path: src/preset/serde_preset_codec.rs

//! `SerdePresetCodec`: a JSON on-disk implementation of the `PresetCodec`
//! port, built on `serde` + `serde_json`.
//!
//! Physical format is a small versioned JSON document:
//! `{ "format_version": u32, "name": string, "patch_payload": [u8, ...] }`.
//! Decoding always routes through `migrate_preset` so callers get the same
//! "explicit version, migrated on load" guarantee regardless of which
//! adapter produced the bytes — this codec never re-implements migration
//! itself, it only owns the physical JSON layout.

use serde::{Deserialize, Serialize};

use crate::preset::preset_codec::{
    migrate_preset, CodecError, Preset, PresetCodec, RawPreset, CURRENT_PRESET_FORMAT_VERSION,
};

/// The on-wire JSON shape. Kept separate from the domain-facing `Preset` so
/// the physical format (field names, JSON itself) is free to evolve without
/// touching the port contract.
#[derive(Debug, Serialize, Deserialize)]
struct SerdePresetDocument {
    format_version: u32,
    name: String,
    patch_payload: Vec<u8>,
}

/// A `PresetCodec` adapter that reads and writes presets as JSON via serde.
///
/// Holds no state and no dependencies of its own — it is a pure function
/// object wrapping `serde_json`. All physical-format concerns (field names,
/// JSON syntax) live here; the shared version-migration policy lives in
/// `port::Preset::PresetCodec::migrate_preset`, so every adapter gets it for
/// free instead of re-implementing it.
#[derive(Debug, Default, Clone, Copy)]
pub struct SerdePresetCodec;

impl SerdePresetCodec {
    /// Construct a new codec. Carries no configuration today; exists so
    /// call sites have a stable construction point if configuration (e.g.
    /// pretty-printing) is added later without changing every call site.
    pub fn new() -> Self {
        Self
    }
}

impl PresetCodec for SerdePresetCodec {
    fn decode(&self, data: &[u8]) -> Result<Preset, CodecError> {
        if data.is_empty() {
            return Err(CodecError::Truncated {
                expected_at_least: 1,
                actual: 0,
            });
        }

        if std::str::from_utf8(data).is_err() {
            return Err(CodecError::InvalidUtf8Name);
        }

        let document: SerdePresetDocument = serde_json::from_slice(data).map_err(|err| {
            if err.is_eof() {
                CodecError::Truncated {
                    expected_at_least: data.len() + 1,
                    actual: data.len(),
                }
            } else {
                CodecError::Malformed(err.to_string())
            }
        })?;

        migrate_preset(RawPreset {
            format_version: document.format_version,
            name: document.name,
            patch_payload: document.patch_payload,
        })
    }

    fn encode(&self, preset: Preset) -> Result<Vec<u8>, CodecError> {
        let document = SerdePresetDocument {
            format_version: CURRENT_PRESET_FORMAT_VERSION,
            name: preset.name().to_string(),
            patch_payload: preset.patch_payload().to_vec(),
        };

        serde_json::to_vec(&document).map_err(|err| CodecError::Malformed(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_preset_through_encode_then_decode() {
        let codec = SerdePresetCodec::new();
        let original = Preset::new("Warm Pad", vec![1, 2, 3, 4]);

        let bytes = codec.encode(original.clone()).expect("encode succeeds");
        let decoded = codec.decode(&bytes).expect("decode succeeds");

        assert_eq!(decoded, original);
        assert_eq!(decoded.format_version(), CURRENT_PRESET_FORMAT_VERSION);
    }

    #[test]
    fn encode_always_writes_the_current_format_version() {
        let codec = SerdePresetCodec::new();
        let preset = Preset::new("Bright Lead", vec![]);

        let bytes = codec.encode(preset).expect("encode succeeds");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");

        assert_eq!(
            value["format_version"],
            serde_json::json!(CURRENT_PRESET_FORMAT_VERSION)
        );
    }

    #[test]
    fn decode_migrates_an_old_but_supported_version_to_current() {
        let codec = SerdePresetCodec::new();
        let raw = serde_json::json!({
            "format_version": crate::preset::preset_codec::MIN_SUPPORTED_PRESET_FORMAT_VERSION,
            "name": "Legacy Pad",
            "patch_payload": [9, 9]
        });
        let bytes = serde_json::to_vec(&raw).expect("serializes");

        let decoded = codec.decode(&bytes).expect("decode succeeds");

        assert_eq!(decoded.format_version(), CURRENT_PRESET_FORMAT_VERSION);
        assert_eq!(decoded.name(), "Legacy Pad");
        assert_eq!(decoded.patch_payload(), &[9, 9]);
    }

    #[test]
    fn decode_rejects_a_version_newer_than_this_build_understands() {
        let codec = SerdePresetCodec::new();
        let raw = serde_json::json!({
            "format_version": CURRENT_PRESET_FORMAT_VERSION + 1,
            "name": "Future Patch",
            "patch_payload": []
        });
        let bytes = serde_json::to_vec(&raw).expect("serializes");

        let result = codec.decode(&bytes);

        assert_eq!(
            result,
            Err(CodecError::UnsupportedVersion(
                CURRENT_PRESET_FORMAT_VERSION + 1
            ))
        );
    }

    #[test]
    fn decode_reports_truncated_for_empty_input_instead_of_panicking() {
        let codec = SerdePresetCodec::new();

        let result = codec.decode(&[]);

        assert!(matches!(result, Err(CodecError::Truncated { .. })));
    }

    #[test]
    fn decode_reports_truncated_for_an_incomplete_json_document() {
        let codec = SerdePresetCodec::new();
        let incomplete = br#"{"format_version": 2, "name": "Oops""#;

        let result = codec.decode(incomplete);

        assert!(matches!(result, Err(CodecError::Truncated { .. })));
    }

    #[test]
    fn decode_reports_malformed_for_structurally_wrong_json() {
        let codec = SerdePresetCodec::new();
        let wrong_shape =
            br#"{"format_version": "not-a-number", "name": "X", "patch_payload": []}"#;

        let result = codec.decode(wrong_shape);

        assert!(matches!(result, Err(CodecError::Malformed(_))));
    }

    #[test]
    fn decode_reports_invalid_utf8_instead_of_panicking() {
        let codec = SerdePresetCodec::new();
        let invalid_utf8 = vec![0xFF, 0xFE, 0x00, 0x01];

        let result = codec.decode(&invalid_utf8);

        assert_eq!(result, Err(CodecError::InvalidUtf8Name));
    }
}
