// path: src/effects/delay_config.rs

/// Error returned when `DelayConfig` construction fails.
#[derive(Debug, Clone, PartialEq)]
pub enum DelayConfigError {
    /// The `time` value was NaN, zero, or negative.
    InvalidTime(f64),
    /// The `feedback` value was NaN or outside 0.0–1.0.
    InvalidFeedback(f64),
    /// The `dry_wet` value was NaN or outside 0.0–1.0.
    InvalidDryWet(f64),
}

impl std::fmt::Display for DelayConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DelayConfigError::InvalidTime(v) => {
                write!(f, "delay time {} is not positive", v)
            }
            DelayConfigError::InvalidFeedback(v) => {
                write!(f, "feedback value {} is out of range 0.0-1.0", v)
            }
            DelayConfigError::InvalidDryWet(v) => {
                write!(f, "dry/wet value {} is out of range 0.0-1.0", v)
            }
        }
    }
}

impl std::error::Error for DelayConfigError {}

// ─────────────────────────────────────────────────────────────────────────────

/// Delay effect parameters.
///
/// Invariants:
/// - `time` is positive (> 0.0, not NaN).
/// - `feedback` is in the range 0.0–1.0 (inclusive, not NaN).
/// - `dry_wet` is in the range 0.0–1.0 (inclusive, not NaN).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DelayConfig {
    /// Delay time in seconds. Must be positive.
    time: f64,
    /// Feedback amount (0.0 = no feedback, 1.0 = infinite feedback).
    feedback: f64,
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    dry_wet: f64,
    /// When `true`, the delay time is locked to the host tempo.
    pub sync_to_tempo: bool,
}

impl DelayConfig {
    /// Construct a `DelayConfig`.
    ///
    /// Returns `Err` if `time` is NaN, zero, or negative; if `feedback` is NaN
    /// or outside 0.0–1.0; or if `dry_wet` is NaN or outside 0.0–1.0.
    ///
    /// ```
    /// use crest_synth::effects::delay_config::DelayConfig;
    /// let cfg = DelayConfig::try_new(0.5, 0.3, 0.4, false).unwrap();
    /// assert!((cfg.time() - 0.5).abs() < f64::EPSILON);
    /// assert!((cfg.feedback() - 0.3).abs() < f64::EPSILON);
    /// assert!((cfg.dry_wet() - 0.4).abs() < f64::EPSILON);
    /// assert!(!cfg.sync_to_tempo);
    /// ```
    pub fn try_new(
        time: f64,
        feedback: f64,
        dry_wet: f64,
        sync_to_tempo: bool,
    ) -> Result<Self, DelayConfigError> {
        if time.is_nan() || time <= 0.0 {
            return Err(DelayConfigError::InvalidTime(time));
        }
        if feedback.is_nan() || !(0.0..=1.0).contains(&feedback) {
            return Err(DelayConfigError::InvalidFeedback(feedback));
        }
        if dry_wet.is_nan() || !(0.0..=1.0).contains(&dry_wet) {
            return Err(DelayConfigError::InvalidDryWet(dry_wet));
        }
        Ok(Self {
            time,
            feedback,
            dry_wet,
            sync_to_tempo,
        })
    }

    /// Return the delay time in seconds (always positive).
    #[inline]
    pub fn time(self) -> f64 {
        self.time
    }

    /// Return the feedback amount (0.0–1.0).
    #[inline]
    pub fn feedback(self) -> f64 {
        self.feedback
    }

    /// Return the dry/wet mix (0.0–1.0).
    #[inline]
    pub fn dry_wet(self) -> f64 {
        self.dry_wet
    }
}

impl Default for DelayConfig {
    /// Returns a sensible default: 250 ms delay, 0.3 feedback, 0.5 dry/wet,
    /// tempo sync off.
    fn default() -> Self {
        Self {
            time: 0.25,
            feedback: 0.3,
            dry_wet: 0.5,
            sync_to_tempo: false,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── time invariant ───────────────────────────────────────────────────────────────────

    #[test]
    fn delay_config_accepts_positive_time() {
        assert!(DelayConfig::try_new(0.001, 0.0, 0.0, false).is_ok());
        assert!(DelayConfig::try_new(1.0, 0.0, 0.0, false).is_ok());
        assert!(DelayConfig::try_new(10.0, 0.5, 0.5, true).is_ok());
    }

    #[test]
    fn delay_config_rejects_zero_time() {
        let err = DelayConfig::try_new(0.0, 0.5, 0.5, false).unwrap_err();
        assert!(matches!(err, DelayConfigError::InvalidTime(_)));
    }

    #[test]
    fn delay_config_rejects_negative_time() {
        let err = DelayConfig::try_new(-0.1, 0.5, 0.5, false).unwrap_err();
        assert!(matches!(err, DelayConfigError::InvalidTime(_)));
    }

    #[test]
    fn delay_config_rejects_nan_time() {
        let err = DelayConfig::try_new(f64::NAN, 0.5, 0.5, false).unwrap_err();
        assert!(matches!(err, DelayConfigError::InvalidTime(_)));
    }

    // ── feedback invariant ─────────────────────────────────────────────────────────────

    #[test]
    fn delay_config_accepts_feedback_boundaries() {
        assert!(DelayConfig::try_new(0.5, 0.0, 0.5, false).is_ok());
        assert!(DelayConfig::try_new(0.5, 1.0, 0.5, false).is_ok());
    }

    #[test]
    fn delay_config_rejects_feedback_above_one() {
        let err = DelayConfig::try_new(0.5, 1.001, 0.5, false).unwrap_err();
        assert!(matches!(err, DelayConfigError::InvalidFeedback(_)));
    }

    #[test]
    fn delay_config_rejects_feedback_below_zero() {
        let err = DelayConfig::try_new(0.5, -0.001, 0.5, false).unwrap_err();
        assert!(matches!(err, DelayConfigError::InvalidFeedback(_)));
    }

    #[test]
    fn delay_config_rejects_nan_feedback() {
        let err = DelayConfig::try_new(0.5, f64::NAN, 0.5, false).unwrap_err();
        assert!(matches!(err, DelayConfigError::InvalidFeedback(_)));
    }

    // ── dry_wet invariant ─────────────────────────────────────────────────────────────

    #[test]
    fn delay_config_accepts_dry_wet_boundaries() {
        assert!(DelayConfig::try_new(0.5, 0.5, 0.0, false).is_ok());
        assert!(DelayConfig::try_new(0.5, 0.5, 1.0, false).is_ok());
    }

    #[test]
    fn delay_config_rejects_dry_wet_above_one() {
        let err = DelayConfig::try_new(0.5, 0.5, 1.001, false).unwrap_err();
        assert!(matches!(err, DelayConfigError::InvalidDryWet(_)));
    }

    #[test]
    fn delay_config_rejects_dry_wet_below_zero() {
        let err = DelayConfig::try_new(0.5, 0.5, -0.001, false).unwrap_err();
        assert!(matches!(err, DelayConfigError::InvalidDryWet(_)));
    }

    #[test]
    fn delay_config_rejects_nan_dry_wet() {
        let err = DelayConfig::try_new(0.5, 0.5, f64::NAN, false).unwrap_err();
        assert!(matches!(err, DelayConfigError::InvalidDryWet(_)));
    }

    // ── accessors ────────────────────────────────────────────────────────────────────

    #[test]
    fn delay_config_accessors_round_trip() {
        let cfg = DelayConfig::try_new(0.25, 0.3, 0.7, true).unwrap();
        assert!((cfg.time() - 0.25).abs() < f64::EPSILON);
        assert!((cfg.feedback() - 0.3).abs() < f64::EPSILON);
        assert!((cfg.dry_wet() - 0.7).abs() < f64::EPSILON);
        assert!(cfg.sync_to_tempo);
    }

    // ── sync_to_tempo ──────────────────────────────────────────────────────────────

    #[test]
    fn delay_config_sync_to_tempo_false() {
        let cfg = DelayConfig::try_new(0.5, 0.5, 0.5, false).unwrap();
        assert!(!cfg.sync_to_tempo);
    }

    #[test]
    fn delay_config_sync_to_tempo_true() {
        let cfg = DelayConfig::try_new(0.5, 0.5, 0.5, true).unwrap();
        assert!(cfg.sync_to_tempo);
    }

    // ── default ─────────────────────────────────────────────────────────────────────

    #[test]
    fn delay_config_default_satisfies_invariants() {
        let cfg = DelayConfig::default();
        assert!(cfg.time() > 0.0);
        assert!((0.0..=1.0).contains(&cfg.feedback()));
        assert!((0.0..=1.0).contains(&cfg.dry_wet()));
    }

    // ── copy semantics ─────────────────────────────────────────────────────────────

    #[test]
    fn delay_config_copy_semantics() {
        let a = DelayConfig::try_new(0.5, 0.3, 0.4, false).unwrap();
        let b = a;
        assert_eq!(a, b);
    }

    // ── error display ─────────────────────────────────────────────────────────────

    #[test]
    fn delay_config_error_display_time() {
        let err = DelayConfigError::InvalidTime(-1.0);
        assert!(err.to_string().contains("-1"));
    }

    #[test]
    fn delay_config_error_display_feedback() {
        let err = DelayConfigError::InvalidFeedback(2.5);
        assert!(err.to_string().contains("2.5"));
    }

    #[test]
    fn delay_config_error_display_dry_wet() {
        let err = DelayConfigError::InvalidDryWet(-0.5);
        assert!(err.to_string().contains("-0.5"));
    }
}
