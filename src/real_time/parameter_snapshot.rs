// path: src/real_time/parameter_snapshot.rs

use crate::synth::amp_envelope_config::AmpEnvelopeConfig;
use crate::synth::filter_config::FilterConfig;
use crate::synth::oscillator_config::OscillatorConfig;

/// Latest-wins snapshot of all synth parameters, readable without locking.
///
/// `ParameterSnapshot` is a plain `Copy` struct — all fields are stack-allocated
/// value types. Reading a snapshot on the audio thread is allocation-free and
/// lock-free.
///
/// The `version` counter advances monotonically every time the host writes a
/// new snapshot. Consumers compare `version` values to detect whether any
/// parameters have changed since the last observed snapshot.
///
/// ```
/// use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
/// use crest_synth::synth::amp_envelope_config::AmpEnvelopeConfig;
/// use crest_synth::synth::filter_config::FilterConfig;
/// use crest_synth::synth::oscillator_config::OscillatorConfig;
///
/// let snap = ParameterSnapshot::new(
///     AmpEnvelopeConfig::default(),
///     FilterConfig::default(),
///     OscillatorConfig::default(),
///     1,
/// );
/// assert_eq!(snap.version(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterSnapshot {
    /// Current amplitude envelope parameters.
    pub amp_envelope: AmpEnvelopeConfig,
    /// Current filter parameters.
    pub filter: FilterConfig,
    /// Current oscillator parameters.
    pub oscillator: OscillatorConfig,
    /// Monotonically increasing version counter; advances with every parameter write.
    version: u64,
}

impl ParameterSnapshot {
    /// Construct a `ParameterSnapshot` from explicit field values.
    ///
    /// All fields are taken by value — no heap allocation occurs.
    #[inline]
    pub fn new(
        amp_envelope: AmpEnvelopeConfig,
        filter: FilterConfig,
        oscillator: OscillatorConfig,
        version: u64,
    ) -> Self {
        Self {
            amp_envelope,
            filter,
            oscillator,
            version,
        }
    }

    /// Return the version counter for this snapshot.
    ///
    /// Use this to detect whether the snapshot has changed since the last
    /// read without comparing all fields individually.
    #[inline]
    pub fn version(self) -> u64 {
        self.version
    }
}

impl Default for ParameterSnapshot {
    /// Returns a snapshot with all default parameter values and version `0`.
    fn default() -> Self {
        Self {
            amp_envelope: AmpEnvelopeConfig::default(),
            filter: FilterConfig::default(),
            oscillator: OscillatorConfig::default(),
            version: 0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::filter_config::FilterType;
    use crate::synth::oscillator_config::Waveform;

    fn make_snapshot(version: u64) -> ParameterSnapshot {
        ParameterSnapshot::new(
            AmpEnvelopeConfig::default(),
            FilterConfig::default(),
            OscillatorConfig::default(),
            version,
        )
    }

    #[test]
    fn default_snapshot_has_version_zero() {
        let snap = ParameterSnapshot::default();
        assert_eq!(snap.version(), 0);
    }

    #[test]
    fn new_snapshot_stores_version() {
        let snap = make_snapshot(42);
        assert_eq!(snap.version(), 42);
    }

    #[test]
    fn version_counter_max_u64_is_representable() {
        let snap = make_snapshot(u64::MAX);
        assert_eq!(snap.version(), u64::MAX);
    }

    #[test]
    fn copy_semantics_preserve_all_fields() {
        let a = make_snapshot(7);
        let b = a;
        assert_eq!(a, b);
        assert_eq!(b.version(), 7);
    }

    #[test]
    fn snapshots_with_different_versions_are_unequal() {
        let a = make_snapshot(1);
        let b = make_snapshot(2);
        assert_ne!(a, b);
    }

    #[test]
    fn snapshots_with_same_fields_are_equal() {
        let a = make_snapshot(3);
        let b = make_snapshot(3);
        assert_eq!(a, b);
    }

    #[test]
    fn amp_envelope_field_stored_correctly() {
        let env = AmpEnvelopeConfig::try_new(0.02, 0.05, 0.7, 0.4).unwrap();
        let snap =
            ParameterSnapshot::new(env, FilterConfig::default(), OscillatorConfig::default(), 1);
        assert_eq!(snap.amp_envelope, env);
    }

    #[test]
    fn filter_field_stored_correctly() {
        let filter = FilterConfig::try_new(5_000.0, FilterType::HighPass, 0.8).unwrap();
        let snap = ParameterSnapshot::new(
            AmpEnvelopeConfig::default(),
            filter,
            OscillatorConfig::default(),
            1,
        );
        assert_eq!(snap.filter, filter);
    }

    #[test]
    fn oscillator_field_stored_correctly() {
        let osc = OscillatorConfig::try_new(100.0, 0.25, Waveform::Square).unwrap();
        let snap = ParameterSnapshot::new(
            AmpEnvelopeConfig::default(),
            FilterConfig::default(),
            osc,
            1,
        );
        assert_eq!(snap.oscillator, osc);
    }

    #[test]
    fn snapshot_is_stack_only_no_heap() {
        // Size must be a compile-time-known constant (no Box/Vec/String fields).
        // This test verifies the type is Sized and has a small, predictable size.
        let size = std::mem::size_of::<ParameterSnapshot>();
        // AmpEnvelopeConfig: 4 × f64 = 32 bytes
        // FilterConfig:  Frequency(f64) + FilterType(u8 aligned) + f64 = ~24 bytes
        // OscillatorConfig: f64 + f64 + Waveform(u8 aligned) = ~24 bytes
        // version: u64 = 8 bytes
        // Total is predictable and well under 256 bytes.
        assert!(
            size < 256,
            "ParameterSnapshot unexpectedly large: {size} bytes"
        );
    }
}
