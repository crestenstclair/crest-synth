// path: src/loop/serde_snapshot_codec.rs

use crate::r#loop::snapshot_codec::{
    AppState, ChannelSnapshot, ChannelState, CodecError, SnapshotCodec, StateSnapshot,
    CURRENT_SCHEMA_VERSION,
};

/// Serde-backed [`SnapshotCodec`] adapter.
///
/// Converts between the in-memory [`AppState`] and the plain-field
/// [`StateSnapshot`] using the same `serde`-derived types the snapshot is
/// defined with: encoding canonicalizes the snapshot through `serde_json`
/// so the value that ships on the wire is exactly what `serde_json` would
/// itself produce and re-parse, and decoding validates every field before
/// it is accepted into domain state.
///
/// `decode` never partially applies: on any validation failure the caller's
/// prior state is left untouched, matching the "atomic restore" invariant
/// for session loads.
#[derive(Debug, Clone, Copy, Default)]
pub struct SerdeSnapshotCodec;

impl SnapshotCodec for SerdeSnapshotCodec {
    fn encode(&self, state: AppState) -> StateSnapshot {
        let snapshot = StateSnapshot {
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
        };

        // Canonicalize through serde_json so the encoded snapshot is exactly
        // what a serde_json round-trip would produce -- the adapter's
        // contract with the "serde" framework named in its declaration.
        let json = serde_json::to_string(&snapshot).expect("StateSnapshot always serializes");
        serde_json::from_str(&json).expect("a just-serialized StateSnapshot always deserializes")
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
            tempo_bpm: 128.0,
            time_signature: (3, 4),
            master_volume_db: -1.5,
            master_muted: false,
            channels: vec![
                ChannelState {
                    name: "Pad".to_string(),
                    volume_db: -4.0,
                    pan: -0.2,
                    muted: false,
                    soloed: true,
                },
                ChannelState {
                    name: "Kick".to_string(),
                    volume_db: -2.0,
                    pan: 0.0,
                    muted: false,
                    soloed: false,
                },
            ],
        }
    }

    #[test]
    fn round_trip_preserves_state() {
        let codec = SerdeSnapshotCodec;
        let state = sample_state();
        let snapshot = codec.encode(state.clone());
        let decoded = codec.decode(snapshot).expect("decode should succeed");
        assert_eq!(decoded, state);
    }

    #[test]
    fn encode_stamps_current_schema_version() {
        let codec = SerdeSnapshotCodec;
        let snapshot = codec.encode(sample_state());
        assert_eq!(snapshot.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn encode_output_matches_a_serde_json_round_trip() {
        let codec = SerdeSnapshotCodec;
        let snapshot = codec.encode(sample_state());
        let json = serde_json::to_string(&snapshot).expect("serialize");
        let reparsed: StateSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(reparsed, snapshot);
    }

    #[test]
    fn decode_rejects_unsupported_schema_version() {
        let codec = SerdeSnapshotCodec;
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
        let codec = SerdeSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.time_signature_numerator = 0;
        let err = codec.decode(snapshot).unwrap_err();
        assert_eq!(err, CodecError::InvalidTimeSignatureNumerator(0));
    }

    #[test]
    fn decode_rejects_non_power_of_two_denominator() {
        let codec = SerdeSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.time_signature_denominator = 5;
        let err = codec.decode(snapshot).unwrap_err();
        assert_eq!(err, CodecError::InvalidTimeSignatureDenominator(5));
    }

    #[test]
    fn decode_rejects_non_finite_tempo() {
        let codec = SerdeSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.tempo_bpm = f64::NAN;
        let err = codec.decode(snapshot).unwrap_err();
        assert!(matches!(err, CodecError::InvalidTempo(_)));
    }

    #[test]
    fn decode_rejects_out_of_range_pan() {
        let codec = SerdeSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.channels[0].pan = -1.2;
        let err = codec.decode(snapshot).unwrap_err();
        assert_eq!(
            err,
            CodecError::PanOutOfRange {
                channel: 0,
                value: -1.2,
            }
        );
    }

    #[test]
    fn decode_rejects_non_finite_channel_volume() {
        let codec = SerdeSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.channels[1].volume_db = f64::NEG_INFINITY;
        let err = codec.decode(snapshot).unwrap_err();
        assert_eq!(err, CodecError::VolumeNotFinite { channel: Some(1) });
    }

    #[test]
    fn a_failed_decode_leaves_the_snapshot_recoverable_and_side_effect_free() {
        let codec = SerdeSnapshotCodec;
        let mut snapshot = codec.encode(sample_state());
        snapshot.master_volume_db = f64::INFINITY;
        assert!(codec.decode(snapshot.clone()).is_err());

        snapshot.master_volume_db = -1.5;
        let decoded = codec.decode(snapshot).expect("decode should now succeed");
        assert_eq!(decoded, sample_state());
    }
}
