//! The `PresetCodec` port: the single seam through which a `Preset` crosses
//! the boundary between the in-memory domain model and a byte stream (disk,
//! network, clipboard, ...).
//!
//! `valueObject.Preset.Preset` has not been committed to this module tree
//! yet, so this port defines a narrow local `Preset` shape of its own. Once
//! the real domain value object lands, adapters can convert between the two;
//! this trait's signature (`decode`/`encode`) is what the rest of the
//! project depends on and stays stable either way.
//!
//! This module also owns the shared version-migration policy so every
//! adapter (e.g. a future `SerdePresetCodec`) gets the same behavior for
//! free instead of re-implementing it: presets always carry an explicit
//! format version, and `migrate_preset` is the one place that knows how to
//! bring an older version forward.

use std::error::Error;
use std::fmt;

/// Oldest on-disk preset format version this codec still knows how to
/// migrate. Anything older is rejected rather than guessed at.
pub const MIN_SUPPORTED_PRESET_FORMAT_VERSION: u32 = 1;

/// The format version newly-encoded presets are written at, and the version
/// every successfully decoded `Preset` ends up at after migration.
pub const CURRENT_PRESET_FORMAT_VERSION: u32 = 2;

/// A decoded, fully-migrated preset.
///
/// This is a local stand-in for the domain `Preset` value object (see the
/// module-level docs). It intentionally exposes only what a codec needs to
/// round-trip a preset: a name and an opaque patch payload, both tagged with
/// the format version they were produced at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    format_version: u32,
    name: String,
    patch_payload: Vec<u8>,
}

impl Preset {
    /// Construct a preset at the current format version. Use this for
    /// presets originating in memory (e.g. a user just saved one), not for
    /// presets coming off the wire — those go through `migrate_preset`.
    pub fn new(name: impl Into<String>, patch_payload: Vec<u8>) -> Self {
        Self {
            format_version: CURRENT_PRESET_FORMAT_VERSION,
            name: name.into(),
            patch_payload,
        }
    }

    /// The format version this preset is currently represented at. Always
    /// `CURRENT_PRESET_FORMAT_VERSION` for any `Preset` a caller can obtain
    /// through this module's public API.
    pub fn format_version(&self) -> u32 {
        self.format_version
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn patch_payload(&self) -> &[u8] {
        &self.patch_payload
    }
}

/// A preset payload exactly as read off the wire, before migration: whatever
/// format version an adapter's physical decoder found, unvalidated.
///
/// Adapters parse their physical format (JSON, bincode, a custom binary
/// layout, ...) into a `RawPreset`, then hand it to `migrate_preset` to get
/// a `Preset` guaranteed to be at `CURRENT_PRESET_FORMAT_VERSION`. Centralizing
/// migration here — rather than duplicating it in every adapter — is what
/// makes the "presets carry an explicit version and are migrated on load"
/// invariant hold project-wide instead of per-adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPreset {
    pub format_version: u32,
    pub name: String,
    pub patch_payload: Vec<u8>,
}

/// Migrate a `RawPreset` at any supported historical version up to
/// `CURRENT_PRESET_FORMAT_VERSION`.
///
/// Returns `CodecError::UnsupportedVersion` if the version is newer than
/// this build understands (a newer app wrote it) or older than
/// `MIN_SUPPORTED_PRESET_FORMAT_VERSION` (support for it was dropped).
pub fn migrate_preset(raw: RawPreset) -> Result<Preset, CodecError> {
    if raw.format_version > CURRENT_PRESET_FORMAT_VERSION
        || raw.format_version < MIN_SUPPORTED_PRESET_FORMAT_VERSION
    {
        return Err(CodecError::UnsupportedVersion(raw.format_version));
    }

    // version 1 -> 2: payload layout is unchanged; only the version tag
    // itself was introduced formally. Future structural migrations add
    // match arms here without touching any adapter or caller.
    let RawPreset {
        format_version: _,
        name,
        patch_payload,
    } = raw;

    Ok(Preset {
        format_version: CURRENT_PRESET_FORMAT_VERSION,
        name,
        patch_payload,
    })
}

/// Everything that can go wrong turning bytes into a `Preset` or back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    /// The byte stream ended before a complete preset could be read.
    Truncated {
        expected_at_least: usize,
        actual: usize,
    },
    /// The format version tag was outside the range this build can migrate.
    UnsupportedVersion(u32),
    /// A name field was not valid UTF-8.
    InvalidUtf8Name,
    /// The stream was well-formed enough to read a version but the rest of
    /// its structure did not match the expected layout.
    Malformed(String),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodecError::Truncated {
                expected_at_least,
                actual,
            } => write!(
                f,
                "truncated preset data: expected at least {expected_at_least} bytes, got {actual}"
            ),
            CodecError::UnsupportedVersion(version) => {
                write!(f, "unsupported preset format version: {version}")
            }
            CodecError::InvalidUtf8Name => write!(f, "preset name is not valid UTF-8"),
            CodecError::Malformed(reason) => write!(f, "malformed preset data: {reason}"),
        }
    }
}

impl Error for CodecError {}

/// A codec that converts a `Preset` to and from bytes.
///
/// Implementations own the physical wire format (JSON, bincode, a bespoke
/// binary layout, ...). Every implementation must decode a `RawPreset` off
/// the wire and pass it through `migrate_preset` before returning it, so the
/// "explicit version, migrated on load" invariant holds no matter which
/// physical format is in play.
pub trait PresetCodec {
    /// Decode a byte stream into a fully-migrated `Preset`.
    fn decode(&self, data: &[u8]) -> Result<Preset, CodecError>;

    /// Encode a `Preset` into bytes at `CURRENT_PRESET_FORMAT_VERSION`.
    fn encode(&self, preset: Preset) -> Result<Vec<u8>, CodecError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dependency-free binary codec used only to exercise the
    /// `PresetCodec` contract and the shared migration policy in tests.
    /// Layout: `[version: u32 LE][name_len: u32 LE][name bytes]
    /// [payload_len: u32 LE][payload bytes]`.
    struct FixedLayoutPresetCodec;

    impl PresetCodec for FixedLayoutPresetCodec {
        fn decode(&self, data: &[u8]) -> Result<Preset, CodecError> {
            let mut cursor = 0usize;

            let version = read_u32(data, &mut cursor)?;
            let name_len = read_u32(data, &mut cursor)? as usize;
            let name_bytes = read_slice(data, &mut cursor, name_len)?;
            let name =
                String::from_utf8(name_bytes.to_vec()).map_err(|_| CodecError::InvalidUtf8Name)?;
            let payload_len = read_u32(data, &mut cursor)? as usize;
            let payload = read_slice(data, &mut cursor, payload_len)?.to_vec();

            migrate_preset(RawPreset {
                format_version: version,
                name,
                patch_payload: payload,
            })
        }

        fn encode(&self, preset: Preset) -> Result<Vec<u8>, CodecError> {
            let mut out =
                Vec::with_capacity(4 + 4 + preset.name.len() + 4 + preset.patch_payload.len());
            out.extend_from_slice(&CURRENT_PRESET_FORMAT_VERSION.to_le_bytes());
            out.extend_from_slice(&(preset.name.len() as u32).to_le_bytes());
            out.extend_from_slice(preset.name.as_bytes());
            out.extend_from_slice(&(preset.patch_payload.len() as u32).to_le_bytes());
            out.extend_from_slice(&preset.patch_payload);
            Ok(out)
        }
    }

    fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, CodecError> {
        let slice = read_slice(data, cursor, 4)?;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(slice);
        Ok(u32::from_le_bytes(buf))
    }

    fn read_slice<'a>(
        data: &'a [u8],
        cursor: &mut usize,
        len: usize,
    ) -> Result<&'a [u8], CodecError> {
        let end = *cursor + len;
        if end > data.len() {
            return Err(CodecError::Truncated {
                expected_at_least: end,
                actual: data.len(),
            });
        }
        let slice = &data[*cursor..end];
        *cursor = end;
        Ok(slice)
    }

    #[test]
    fn round_trips_a_preset_through_encode_then_decode() {
        let codec = FixedLayoutPresetCodec;
        let original = Preset::new("Warm Pad", vec![1, 2, 3, 4]);

        let bytes = codec.encode(original.clone()).expect("encode succeeds");
        let decoded = codec.decode(&bytes).expect("decode succeeds");

        assert_eq!(decoded, original);
        assert_eq!(decoded.format_version(), CURRENT_PRESET_FORMAT_VERSION);
    }

    #[test]
    fn encode_always_writes_the_current_format_version() {
        let codec = FixedLayoutPresetCodec;
        let preset = Preset::new("Bright Lead", vec![]);

        let bytes = codec.encode(preset).expect("encode succeeds");
        let version = u32::from_le_bytes(bytes[0..4].try_into().unwrap());

        assert_eq!(version, CURRENT_PRESET_FORMAT_VERSION);
    }

    #[test]
    fn decode_rejects_a_version_newer_than_this_build_understands() {
        let raw = RawPreset {
            format_version: CURRENT_PRESET_FORMAT_VERSION + 1,
            name: "Future Patch".to_string(),
            patch_payload: vec![],
        };

        let result = migrate_preset(raw);

        assert_eq!(
            result,
            Err(CodecError::UnsupportedVersion(
                CURRENT_PRESET_FORMAT_VERSION + 1
            ))
        );
    }

    #[test]
    fn decode_rejects_a_version_older_than_this_build_supports() {
        let raw = RawPreset {
            format_version: MIN_SUPPORTED_PRESET_FORMAT_VERSION - 1,
            name: "Ancient Patch".to_string(),
            patch_payload: vec![],
        };

        let result = migrate_preset(raw);

        assert_eq!(
            result,
            Err(CodecError::UnsupportedVersion(
                MIN_SUPPORTED_PRESET_FORMAT_VERSION - 1
            ))
        );
    }

    #[test]
    fn migrate_preset_carries_an_old_but_supported_version_forward_to_current() {
        let raw = RawPreset {
            format_version: MIN_SUPPORTED_PRESET_FORMAT_VERSION,
            name: "Legacy Pad".to_string(),
            patch_payload: vec![9, 9],
        };

        let migrated = migrate_preset(raw).expect("migration succeeds");

        assert_eq!(migrated.format_version(), CURRENT_PRESET_FORMAT_VERSION);
        assert_eq!(migrated.name(), "Legacy Pad");
        assert_eq!(migrated.patch_payload(), &[9, 9]);
    }

    #[test]
    fn decode_reports_truncated_data_instead_of_panicking() {
        let codec = FixedLayoutPresetCodec;
        let too_short = CURRENT_PRESET_FORMAT_VERSION.to_le_bytes().to_vec();

        let result = codec.decode(&too_short);

        assert!(matches!(result, Err(CodecError::Truncated { .. })));
    }

    #[test]
    fn decode_reports_invalid_utf8_names_instead_of_panicking() {
        let mut bytes = CURRENT_PRESET_FORMAT_VERSION.to_le_bytes().to_vec();
        let invalid_name = vec![0xFF, 0xFE];
        bytes.extend_from_slice(&(invalid_name.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&invalid_name);
        bytes.extend_from_slice(&0u32.to_le_bytes());

        let codec = FixedLayoutPresetCodec;
        let result = codec.decode(&bytes);

        assert_eq!(result, Err(CodecError::InvalidUtf8Name));
    }

    #[test]
    fn codec_error_messages_are_human_readable() {
        assert_eq!(
            CodecError::UnsupportedVersion(7).to_string(),
            "unsupported preset format version: 7"
        );
        assert_eq!(
            CodecError::InvalidUtf8Name.to_string(),
            "preset name is not valid UTF-8"
        );
    }
}
