// path: src/loop/snapshot_codec.rs

use serde::{Deserialize, Serialize};

/// Current on-disk/on-wire schema version produced by [`DefaultSnapshotCodec::encode`].
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Plain, human- and LLM-readable snapshot of [`AppState`].
///
/// Field names are the ubiquitous language and values are plain primitives
/// (dB as numbers, booleans as booleans) rather than domain newtypes, so a
/// serialized snapshot (e.g. JSON) stays legible to a human or an LLM
/// editing a preset/session file by hand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub schema_version: u32,
    pub tempo_bpm: f64,
    pub time_signature_numerator: u8,
    pub time_signature_denominator: u8,
    pub master_volume_db: f64,
    pub master_muted: bool,
    pub channels: Vec<ChannelSnapshot>,
}

/// Plain per-channel fields inside a [`StateSnapshot`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelSnapshot {
    pub name: String,
    pub volume_db: f64,
    pub pan: f64,
    pub muted: bool,
    pub soloed: bool,
}

/// In-memory control-plane state owned by the Loop reducer.
///
/// This is the domain-side counterpart to [`StateSnapshot`]: the shape a
/// codec decodes into and encodes from. It carries no serialization
/// concerns of its own.
#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub tempo_bpm: f64,
    pub time_signature: (u8, u8),
    pub master_volume_db: f64,
    pub master_muted: bool,
    pub channels: Vec<ChannelState>,
}

/// In-memory per-channel state inside an [`AppState`].
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelState {
    pub name: String,
    pub volume_db: f64,
    pub pan: f64,
    pub muted: bool,
    pub soloed: bool,
}

/// Reasons a [`StateSnapshot`] cannot be decoded into an [`AppState`].
///
/// Decoding never partially applies: on any error the caller's prior state
/// is left untouched.
#[derive(Debug, Clone, PartialEq)]
pub enum CodecError {
    UnsupportedSchemaVersion(u32),
    InvalidTimeSignatureNumerator(u8),
    InvalidTimeSignatureDenominator(u8),
    InvalidTempo(f64),
    VolumeNotFinite { channel: Option<usize> },
    PanOutOfRange { channel: usize, value: f64 },
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported snapshot schema version: {version}")
            }
            CodecError::InvalidTimeSignatureNumerator(numerator) => {
                write!(f, "invalid time signature numerator: {numerator}")
            }
            CodecError::InvalidTimeSignatureDenominator(denominator) => {
                write!(f, "invalid time signature denominator: {denominator}")
            }
            CodecError::InvalidTempo(bpm) => {
                write!(f, "invalid tempo: {bpm}")
            }
            CodecError::VolumeNotFinite {
                channel: Some(index),
            } => {
                write!(f, "non-finite volume on channel {index}")
            }
            CodecError::VolumeNotFinite { channel: None } => {
                write!(f, "non-finite master volume")
            }
            CodecError::PanOutOfRange { channel, value } => {
                write!(f, "pan out of range on channel {channel}: {value}")
            }
        }
    }
}

impl std::error::Error for CodecError {}

/// Port: converts between the in-memory [`AppState`] and its
/// plain-text-serializable [`StateSnapshot`] representation.
///
/// `encode` is infallible (every `AppState` has a valid snapshot form);
/// `decode` is fallible because a snapshot may have been hand-edited, come
/// from a newer/older schema version, or otherwise fail validation.
pub trait SnapshotCodec {
    fn encode(&self, state: AppState) -> StateSnapshot;
    fn decode(&self, snapshot: StateSnapshot) -> Result<AppState, CodecError>;
}

/// The one supported [`SnapshotCodec`]: validates plain JSON-shaped field
/// values back into domain-shaped [`AppState`], and vice versa.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultSnapshotCodec;

impl SnapshotCodec for DefaultSnapshotCodec {
    fn encode(&self, state: AppState) -> StateSnapshot {
        StateSnapshot {
            schema_version: CURRENT_SCHEMA_VERSION,
            tempo_bpm: state.tempo_bpm,
            time_signature_numerator: state.time_signature.0,
            time_signature_denominator: state.time_signature.1,
            master_volume_db: state.master_volume_db,
            master_muted: state.master_muted,
            channels: state
                .channels
                .into_iter()
                .map(|channel| ChannelSnapshot {
                    name: channel.name,
                    volume_db: channel.volume_db,
                    pan: channel.pan,
                    muted: channel.muted,
                    soloed: channel.soloed,
                })
                .collect(),
        }
    }

    fn decode(&self, snapshot: StateSnapshot) -> Result<AppState, CodecError> {
        if snapshot.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(CodecError::UnsupportedSchemaVersion(
                snapshot.schema_version,
            ));
        }
        if snapshot.time_signature_numerator == 0 {
            return Err(CodecError::InvalidTimeSignatureNumerator(
                snapshot.time_signature_numerator,
            ));
        }
        if snapshot.time_signature_denominator == 0
            || !snapshot.time_signature_denominator.is_power_of_two()
        {
            return Err(CodecError::InvalidTimeSignatureDenominator(
                snapshot.time_signature_denominator,
            ));
        }
        if !snapshot.tempo_bpm.is_finite() || snapshot.tempo_bpm <= 0.0 {
            return Err(CodecError::InvalidTempo(snapshot.tempo_bpm));
        }
        if !snapshot.master_volume_db.is_finite() {
            return Err(CodecError::VolumeNotFinite { channel: None });
        }

        let mut channels = Vec::with_capacity(snapshot.channels.len());
        for (index, channel) in snapshot.channels.into_iter().enumerate() {
            if !channel.volume_db.is_finite() {
                return Err(CodecError::VolumeNotFinite {
                    channel: Some(index),
                });
            }
            if !(-1.0..=1.0).contains(&channel.pan) {
                return Err(CodecError::PanOutOfRange {
                    channel: index,
                    value: channel.pan,
                });
            }
            channels.push(ChannelState {
                name: channel.name,
                volume_db: channel.volume_db,
                pan: channel.pan,
                muted: channel.muted,
                soloed: channel.soloed,
            });
        }

        Ok(AppState {
            tempo_bpm: snapshot.tempo_bpm,
            time_signature: (
                snapshot.time_signature_numerator,
                snapshot.time_signature_denominator,
            ),
            master_volume_db: snapshot.master_volume_db,
            master_muted: snapshot.master_muted,
            channels,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> AppState {
        AppState {
            tempo_bpm: 120.0,
            time_signature: (4, 4),
            master_volume_db: -3.0,
            master_muted: false,
            channels: vec![
                ChannelState {
                    name: "Lead".to_string(),
                    volume_db: -6.0,
                    pan: 0.25,
                    muted: false,
                    soloed: false,
                },
                ChannelState {
                    name: "Bass".to_string(),
                    volume_db: -9.5,
                    pan: -0.5,
                    muted: true,
                    soloed: false,
                },
            ],
        }
    }

    #[test]
    fn round_trip_preserves_state() {
        let codec = DefaultSnapshotCodec;
        let state = sample_state();
        let snapshot = codec.encode(state.clone());
        let decoded = codec.decode(snapshot).expect("decode should succeed");
        assert_eq!(decoded, state);
    }

    #[test]
    fn encode_stamps_current_schema_version() {
        let codec = DefaultSnapshotCodec;
        let snapshot = codec.encode(sample_state());
        assert_eq!(snapshot.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn decode_rejects_unsupported_schema_version() {
        let codec = DefaultSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.schema_version = CURRENT_SCHEMA_VERSION + 1;
        let err = codec.decode(snapshot).unwrap_err();
        assert_eq!(
            err,
            CodecError::UnsupportedSchemaVersion(CURRENT_SCHEMA_VERSION + 1)
        );
    }

    #[test]
    fn decode_rejects_zero_time_signature_numerator() {
        let codec = DefaultSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.time_signature_numerator = 0;
        let err = codec.decode(snapshot).unwrap_err();
        assert_eq!(err, CodecError::InvalidTimeSignatureNumerator(0));
    }

    #[test]
    fn decode_rejects_non_power_of_two_denominator() {
        let codec = DefaultSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.time_signature_denominator = 3;
        let err = codec.decode(snapshot).unwrap_err();
        assert_eq!(err, CodecError::InvalidTimeSignatureDenominator(3));
    }

    #[test]
    fn decode_rejects_non_finite_tempo() {
        let codec = DefaultSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.tempo_bpm = f64::NAN;
        let err = codec.decode(snapshot).unwrap_err();
        assert!(matches!(err, CodecError::InvalidTempo(_)));
    }

    #[test]
    fn decode_rejects_non_positive_tempo() {
        let codec = DefaultSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.tempo_bpm = 0.0;
        let err = codec.decode(snapshot).unwrap_err();
        assert_eq!(err, CodecError::InvalidTempo(0.0));
    }

    #[test]
    fn decode_rejects_out_of_range_pan() {
        let codec = DefaultSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.channels[0].pan = 1.5;
        let err = codec.decode(snapshot).unwrap_err();
        assert_eq!(
            err,
            CodecError::PanOutOfRange {
                channel: 0,
                value: 1.5,
            }
        );
    }

    #[test]
    fn decode_rejects_non_finite_channel_volume() {
        let codec = DefaultSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.channels[1].volume_db = f64::INFINITY;
        let err = codec.decode(snapshot).unwrap_err();
        assert_eq!(err, CodecError::VolumeNotFinite { channel: Some(1) });
    }

    #[test]
    fn a_failed_decode_leaves_the_snapshot_recoverable_and_side_effect_free() {
        let codec = DefaultSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.master_volume_db = f64::NEG_INFINITY;
        assert!(codec.decode(snapshot.clone()).is_err());

        snapshot.master_volume_db = -3.0;
        let decoded = codec.decode(snapshot).expect("decode should now succeed");
        assert_eq!(decoded, sample_state());
    }

    #[test]
    fn json_round_trip_uses_plain_field_values() {
        let codec = DefaultSnapshotCodec;
        let snapshot = codec.encode(sample_state());
        let json = serde_json::to_string(&snapshot).expect("serialize");
        assert!(json.contains("\"tempo_bpm\":120.0"));
        assert!(json.contains("\"master_muted\":false"));
        assert!(json.contains("\"muted\":true"));
        let parsed: StateSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, snapshot);
    }
}
