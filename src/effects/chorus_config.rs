// path: src/effects/chorus_config.rs

/// Error returned when [`ChorusConfig`] invariants are violated.
#[derive(Debug, Clone, PartialEq)]
pub enum ChorusConfigError {
    /// `rate` must be strictly positive (> 0.0 and not NaN).
    RateNotPositive,
    /// `depth` must be in the range `[0.0, 1.0]`.
    DepthOutOfRange,
    /// `dry_wet` must be in the range `[0.0, 1.0]`.
    DryWetOutOfRange,
    /// `voices` must be at least 1.
    VoicesTooFew,
}

impl core::fmt::Display for ChorusConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RateNotPositive => write!(f, "rate must be positive (> 0.0)"),
            Self::DepthOutOfRange => write!(f, "depth must be in [0.0, 1.0]"),
            Self::DryWetOutOfRange => write!(f, "dry_wet must be in [0.0, 1.0]"),
            Self::VoicesTooFew => write!(f, "voices must be at least 1"),
        }
    }
}

/// Chorus effect parameters.
///
/// # Invariants
///
/// - `rate` is strictly positive (> 0.0).
/// - `depth` is in `[0.0, 1.0]`.
/// - `dry_wet` is in `[0.0, 1.0]`.
/// - `voices` is at least 1.
#[derive(Debug, Clone, PartialEq)]
pub struct ChorusConfig {
    /// LFO rate in Hz.
    rate: f64,
    /// Modulation depth (0.0 = none, 1.0 = maximum).
    depth: f64,
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    dry_wet: f64,
    /// Number of chorus voices (>= 1).
    voices: u8,
}

impl ChorusConfig {
    /// Construct a new [`ChorusConfig`], validating all invariants.
    ///
    /// # Errors
    ///
    /// Returns `Err` if any invariant is violated:
    /// - `rate` must be > 0.0 and not NaN.
    /// - `depth` and `dry_wet` must each be in `[0.0, 1.0]`.
    /// - `voices` must be >= 1.
    pub fn try_new(
        rate: f64,
        depth: f64,
        dry_wet: f64,
        voices: u8,
    ) -> Result<Self, ChorusConfigError> {
        if rate.is_nan() || rate <= 0.0 {
            return Err(ChorusConfigError::RateNotPositive);
        }
        if depth.is_nan() || !(0.0..=1.0).contains(&depth) {
            return Err(ChorusConfigError::DepthOutOfRange);
        }
        if dry_wet.is_nan() || !(0.0..=1.0).contains(&dry_wet) {
            return Err(ChorusConfigError::DryWetOutOfRange);
        }
        if voices < 1 {
            return Err(ChorusConfigError::VoicesTooFew);
        }
        Ok(Self {
            rate,
            depth,
            dry_wet,
            voices,
        })
    }

    /// LFO rate in Hz.
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Modulation depth in `[0.0, 1.0]`.
    pub fn depth(&self) -> f64 {
        self.depth
    }

    /// Dry/wet mix in `[0.0, 1.0]`.
    pub fn dry_wet(&self) -> f64 {
        self.dry_wet
    }

    /// Number of chorus voices (>= 1).
    pub fn voices(&self) -> u8 {
        self.voices
    }
}

#[cfg(test)]
mod chorus_config_tests {
    use super::*;

    #[test]
    fn valid_config_constructs() {
        let cfg = ChorusConfig::try_new(1.5, 0.5, 0.7, 3).unwrap();
        assert_eq!(cfg.rate(), 1.5);
        assert_eq!(cfg.depth(), 0.5);
        assert_eq!(cfg.dry_wet(), 0.7);
        assert_eq!(cfg.voices(), 3);
    }

    #[test]
    fn rate_zero_is_rejected() {
        assert_eq!(
            ChorusConfig::try_new(0.0, 0.5, 0.5, 2),
            Err(ChorusConfigError::RateNotPositive)
        );
    }

    #[test]
    fn rate_negative_is_rejected() {
        assert_eq!(
            ChorusConfig::try_new(-1.0, 0.5, 0.5, 2),
            Err(ChorusConfigError::RateNotPositive)
        );
    }

    #[test]
    fn rate_nan_is_rejected() {
        assert_eq!(
            ChorusConfig::try_new(f64::NAN, 0.5, 0.5, 2),
            Err(ChorusConfigError::RateNotPositive)
        );
    }

    #[test]
    fn depth_below_zero_is_rejected() {
        assert_eq!(
            ChorusConfig::try_new(1.0, -0.1, 0.5, 2),
            Err(ChorusConfigError::DepthOutOfRange)
        );
    }

    #[test]
    fn depth_above_one_is_rejected() {
        assert_eq!(
            ChorusConfig::try_new(1.0, 1.1, 0.5, 2),
            Err(ChorusConfigError::DepthOutOfRange)
        );
    }

    #[test]
    fn depth_nan_is_rejected() {
        assert_eq!(
            ChorusConfig::try_new(1.0, f64::NAN, 0.5, 2),
            Err(ChorusConfigError::DepthOutOfRange)
        );
    }

    #[test]
    fn dry_wet_below_zero_is_rejected() {
        assert_eq!(
            ChorusConfig::try_new(1.0, 0.5, -0.1, 2),
            Err(ChorusConfigError::DryWetOutOfRange)
        );
    }

    #[test]
    fn dry_wet_above_one_is_rejected() {
        assert_eq!(
            ChorusConfig::try_new(1.0, 0.5, 1.1, 2),
            Err(ChorusConfigError::DryWetOutOfRange)
        );
    }

    #[test]
    fn dry_wet_nan_is_rejected() {
        assert_eq!(
            ChorusConfig::try_new(1.0, 0.5, f64::NAN, 2),
            Err(ChorusConfigError::DryWetOutOfRange)
        );
    }

    #[test]
    fn voices_zero_is_rejected() {
        assert_eq!(
            ChorusConfig::try_new(1.0, 0.5, 0.5, 0),
            Err(ChorusConfigError::VoicesTooFew)
        );
    }

    #[test]
    fn voices_one_is_accepted() {
        let cfg = ChorusConfig::try_new(1.0, 0.5, 0.5, 1).unwrap();
        assert_eq!(cfg.voices(), 1);
    }

    #[test]
    fn boundary_values_accepted() {
        // depth = 0.0
        assert!(ChorusConfig::try_new(0.001, 0.0, 1.0, 4).is_ok());
        // depth = 1.0
        assert!(ChorusConfig::try_new(0.001, 1.0, 0.0, 4).is_ok());
        // dry_wet = 0.0
        assert!(ChorusConfig::try_new(100.0, 1.0, 0.0, 1).is_ok());
        // dry_wet = 1.0
        assert!(ChorusConfig::try_new(100.0, 0.0, 1.0, 1).is_ok());
    }
}
