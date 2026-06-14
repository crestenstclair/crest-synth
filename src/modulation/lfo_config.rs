// path: src/modulation/lfo_config.rs

use crate::modulation::lfo_waveform::LfoWaveform;

/// LFO parameters.
///
/// # Invariants
/// - `rate` must be positive (> 0.0)
/// - `depth` must be in [0.0, 1.0]
#[derive(Debug, Clone, PartialEq)]
pub struct LfoConfig {
    /// Oscillation rate in Hz. Must be positive.
    pub rate: f64,
    /// Modulation depth in [0.0, 1.0].
    pub depth: f64,
    /// Initial phase in radians (arbitrary; not validated for range).
    pub phase: f64,
    /// Whether the LFO rate syncs to the host tempo.
    pub sync_to_tempo: bool,
    /// Waveform shape for the LFO.
    pub waveform: LfoWaveform,
}

/// Error returned when `LfoConfig` construction fails an invariant.
#[derive(Debug, Clone, PartialEq)]
pub enum LfoConfigError {
    /// `rate` was zero or negative.
    RateNotPositive(f64),
    /// `depth` was outside [0.0, 1.0].
    DepthOutOfRange(f64),
}

impl std::fmt::Display for LfoConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LfoConfigError::RateNotPositive(v) => {
                write!(f, "LfoConfig: rate must be positive, got {v}")
            }
            LfoConfigError::DepthOutOfRange(v) => {
                write!(f, "LfoConfig: depth must be 0.0\u{2013}1.0, got {v}")
            }
        }
    }
}

impl std::error::Error for LfoConfigError {}

impl LfoConfig {
    /// Construct a validated `LfoConfig`.
    ///
    /// Returns `Err` if `rate <= 0.0` or `depth` is outside `[0.0, 1.0]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::modulation::lfo_config::LfoConfig;
    /// use crest_synth::modulation::lfo_waveform::LfoWaveform;
    ///
    /// let cfg = LfoConfig::try_new(2.0, 0.5, 0.0, false, LfoWaveform::Sine);
    /// assert!(cfg.is_ok());
    /// ```
    pub fn try_new(
        rate: f64,
        depth: f64,
        phase: f64,
        sync_to_tempo: bool,
        waveform: LfoWaveform,
    ) -> Result<Self, LfoConfigError> {
        if rate <= 0.0 || rate.is_nan() {
            return Err(LfoConfigError::RateNotPositive(rate));
        }
        if depth.is_nan() || !(0.0..=1.0).contains(&depth) {
            return Err(LfoConfigError::DepthOutOfRange(depth));
        }
        Ok(Self {
            rate,
            depth,
            phase,
            sync_to_tempo,
            waveform,
        })
    }

    /// Construct with default values (rate = 1.0 Hz, depth = 0.0, sine waveform).
    pub fn default_sine() -> Self {
        Self {
            rate: 1.0,
            depth: 0.0,
            phase: 0.0,
            sync_to_tempo: false,
            waveform: LfoWaveform::Sine,
        }
    }
}

impl Default for LfoConfig {
    fn default() -> Self {
        Self::default_sine()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modulation::lfo_waveform::LfoWaveform;

    #[test]
    fn valid_config_is_accepted() {
        let cfg = LfoConfig::try_new(4.0, 0.75, 0.0, false, LfoWaveform::Triangle);
        assert!(cfg.is_ok());
        let cfg = cfg.unwrap();
        assert_eq!(cfg.rate, 4.0);
        assert_eq!(cfg.depth, 0.75);
        assert_eq!(cfg.waveform, LfoWaveform::Triangle);
    }

    #[test]
    fn zero_rate_is_rejected() {
        let result = LfoConfig::try_new(0.0, 0.5, 0.0, false, LfoWaveform::Sine);
        assert_eq!(result, Err(LfoConfigError::RateNotPositive(0.0)));
    }

    #[test]
    fn negative_rate_is_rejected() {
        let result = LfoConfig::try_new(-1.0, 0.5, 0.0, false, LfoWaveform::Sine);
        assert_eq!(result, Err(LfoConfigError::RateNotPositive(-1.0)));
    }

    #[test]
    fn nan_rate_is_rejected() {
        let result = LfoConfig::try_new(f64::NAN, 0.5, 0.0, false, LfoWaveform::Sine);
        assert!(matches!(result, Err(LfoConfigError::RateNotPositive(_))));
    }

    #[test]
    fn depth_below_zero_is_rejected() {
        let result = LfoConfig::try_new(1.0, -0.1, 0.0, false, LfoWaveform::Sine);
        assert_eq!(result, Err(LfoConfigError::DepthOutOfRange(-0.1)));
    }

    #[test]
    fn depth_above_one_is_rejected() {
        let result = LfoConfig::try_new(1.0, 1.1, 0.0, false, LfoWaveform::Sine);
        assert_eq!(result, Err(LfoConfigError::DepthOutOfRange(1.1)));
    }

    #[test]
    fn nan_depth_is_rejected() {
        let result = LfoConfig::try_new(1.0, f64::NAN, 0.0, false, LfoWaveform::Sine);
        assert!(matches!(result, Err(LfoConfigError::DepthOutOfRange(_))));
    }

    #[test]
    fn depth_boundary_values_accepted() {
        assert!(LfoConfig::try_new(1.0, 0.0, 0.0, false, LfoWaveform::Sine).is_ok());
        assert!(LfoConfig::try_new(1.0, 1.0, 0.0, false, LfoWaveform::Sine).is_ok());
    }

    #[test]
    fn default_sine_has_valid_invariants() {
        let cfg = LfoConfig::default_sine();
        assert!(cfg.rate > 0.0);
        assert!((0.0..=1.0).contains(&cfg.depth));
        assert_eq!(cfg.waveform, LfoWaveform::Sine);
    }

    #[test]
    fn sync_to_tempo_round_trips() {
        let cfg = LfoConfig::try_new(2.0, 0.5, 0.0, true, LfoWaveform::Square).unwrap();
        assert!(cfg.sync_to_tempo);
    }

    #[test]
    fn error_display_contains_value() {
        let e = LfoConfigError::RateNotPositive(-3.0);
        assert!(e.to_string().contains("-3"));
        let e2 = LfoConfigError::DepthOutOfRange(2.5);
        assert!(e2.to_string().contains("2.5"));
    }
}
