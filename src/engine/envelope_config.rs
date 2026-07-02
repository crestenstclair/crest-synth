// path: src/engine/envelope_config.rs

//! ADSR envelope configuration: attack, decay, release (seconds) and sustain
//! level. This is a pure value object — it holds validated domain quantities
//! and performs no I/O, allocation, or locking of its own, so it is safe to
//! construct on either the real-time or non-real-time thread and to copy
//! across the ParameterBridge / EventRing boundary.

/// Error returned when constructing an [`EnvelopeConfig`] from out-of-range
/// values.
#[derive(Debug, Clone, PartialEq)]
pub enum EnvelopeConfigError {
    /// `attack` was negative or NaN.
    NegativeAttack(f64),
    /// `decay` was negative or NaN.
    NegativeDecay(f64),
    /// `release` was negative or NaN.
    NegativeRelease(f64),
    /// `sustain` was outside the inclusive range `0.0..=1.0`, or NaN.
    SustainOutOfRange(f64),
}

impl std::fmt::Display for EnvelopeConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvelopeConfigError::NegativeAttack(v) => {
                write!(f, "attack must be non-negative, got {v}")
            }
            EnvelopeConfigError::NegativeDecay(v) => {
                write!(f, "decay must be non-negative, got {v}")
            }
            EnvelopeConfigError::NegativeRelease(v) => {
                write!(f, "release must be non-negative, got {v}")
            }
            EnvelopeConfigError::SustainOutOfRange(v) => {
                write!(f, "sustain must be within 0.0..=1.0, got {v}")
            }
        }
    }
}

impl std::error::Error for EnvelopeConfigError {}

/// ADSR envelope times (seconds) and sustain level.
///
/// - `attack`, `decay`, and `release` must be non-negative.
/// - `sustain` must lie within `0.0..=1.0`.
///
/// Values are only ever produced through [`EnvelopeConfig::try_new`] (or the
/// infallible [`EnvelopeConfig::default`]), so any `EnvelopeConfig` in scope
/// is guaranteed to satisfy its invariants — no runtime re-validation is
/// needed once constructed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvelopeConfig {
    attack: f64,
    decay: f64,
    release: f64,
    sustain: f64,
}

impl EnvelopeConfig {
    /// Attempts to construct an [`EnvelopeConfig`], validating every field.
    ///
    /// # Errors
    ///
    /// Returns [`EnvelopeConfigError`] if `attack`, `decay`, or `release` is
    /// negative or NaN, or if `sustain` is outside `0.0..=1.0` (including
    /// NaN).
    ///
    /// ```
    /// use crest_synth::engine::envelope_config::EnvelopeConfig;
    /// assert!(EnvelopeConfig::try_new(0.01, 0.1, 0.3, 0.8).is_ok());
    /// assert!(EnvelopeConfig::try_new(-0.01, 0.1, 0.3, 0.8).is_err());
    /// assert!(EnvelopeConfig::try_new(0.01, 0.1, 0.3, 1.1).is_err());
    /// ```
    pub fn try_new(
        attack: f64,
        decay: f64,
        release: f64,
        sustain: f64,
    ) -> Result<Self, EnvelopeConfigError> {
        if attack.is_nan() || attack < 0.0 {
            return Err(EnvelopeConfigError::NegativeAttack(attack));
        }
        if decay.is_nan() || decay < 0.0 {
            return Err(EnvelopeConfigError::NegativeDecay(decay));
        }
        if release.is_nan() || release < 0.0 {
            return Err(EnvelopeConfigError::NegativeRelease(release));
        }
        if sustain.is_nan() || !(0.0..=1.0).contains(&sustain) {
            return Err(EnvelopeConfigError::SustainOutOfRange(sustain));
        }

        Ok(Self {
            attack,
            decay,
            release,
            sustain,
        })
    }

    /// Attack time in seconds.
    pub fn attack(&self) -> f64 {
        self.attack
    }

    /// Decay time in seconds.
    pub fn decay(&self) -> f64 {
        self.decay
    }

    /// Release time in seconds.
    pub fn release(&self) -> f64 {
        self.release
    }

    /// Sustain level, within `0.0..=1.0`.
    pub fn sustain(&self) -> f64 {
        self.sustain
    }

    /// Returns a copy of this config with `attack` replaced, re-validating
    /// the new value.
    pub fn with_attack(self, attack: f64) -> Result<Self, EnvelopeConfigError> {
        Self::try_new(attack, self.decay, self.release, self.sustain)
    }

    /// Returns a copy of this config with `decay` replaced, re-validating
    /// the new value.
    pub fn with_decay(self, decay: f64) -> Result<Self, EnvelopeConfigError> {
        Self::try_new(self.attack, decay, self.release, self.sustain)
    }

    /// Returns a copy of this config with `release` replaced, re-validating
    /// the new value.
    pub fn with_release(self, release: f64) -> Result<Self, EnvelopeConfigError> {
        Self::try_new(self.attack, self.decay, release, self.sustain)
    }

    /// Returns a copy of this config with `sustain` replaced, re-validating
    /// the new value.
    pub fn with_sustain(self, sustain: f64) -> Result<Self, EnvelopeConfigError> {
        Self::try_new(self.attack, self.decay, self.release, sustain)
    }
}

impl Default for EnvelopeConfig {
    /// A reasonable default envelope: fast attack, short decay, full
    /// sustain, quick release.
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            release: 0.3,
            sustain: 0.8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_valid_values() {
        let cfg = EnvelopeConfig::try_new(0.01, 0.2, 0.3, 0.8).expect("valid config");
        assert_eq!(cfg.attack(), 0.01);
        assert_eq!(cfg.decay(), 0.2);
        assert_eq!(cfg.release(), 0.3);
        assert_eq!(cfg.sustain(), 0.8);
    }

    #[test]
    fn try_new_accepts_boundary_values() {
        let cfg = EnvelopeConfig::try_new(0.0, 0.0, 0.0, 0.0).expect("zero times are valid");
        assert_eq!(cfg.sustain(), 0.0);

        let cfg = EnvelopeConfig::try_new(0.0, 0.0, 0.0, 1.0).expect("sustain of 1.0 is valid");
        assert_eq!(cfg.sustain(), 1.0);
    }

    #[test]
    fn try_new_rejects_negative_attack() {
        let err = EnvelopeConfig::try_new(-0.001, 0.1, 0.1, 0.5).unwrap_err();
        assert_eq!(err, EnvelopeConfigError::NegativeAttack(-0.001));
    }

    #[test]
    fn try_new_rejects_negative_decay() {
        let err = EnvelopeConfig::try_new(0.1, -0.001, 0.1, 0.5).unwrap_err();
        assert_eq!(err, EnvelopeConfigError::NegativeDecay(-0.001));
    }

    #[test]
    fn try_new_rejects_negative_release() {
        let err = EnvelopeConfig::try_new(0.1, 0.1, -0.001, 0.5).unwrap_err();
        assert_eq!(err, EnvelopeConfigError::NegativeRelease(-0.001));
    }

    #[test]
    fn try_new_rejects_sustain_below_zero() {
        let err = EnvelopeConfig::try_new(0.1, 0.1, 0.1, -0.001).unwrap_err();
        assert_eq!(err, EnvelopeConfigError::SustainOutOfRange(-0.001));
    }

    #[test]
    fn try_new_rejects_sustain_above_one() {
        let err = EnvelopeConfig::try_new(0.1, 0.1, 0.1, 1.001).unwrap_err();
        assert_eq!(err, EnvelopeConfigError::SustainOutOfRange(1.001));
    }

    #[test]
    fn try_new_rejects_nan_fields() {
        assert!(EnvelopeConfig::try_new(f64::NAN, 0.1, 0.1, 0.5).is_err());
        assert!(EnvelopeConfig::try_new(0.1, f64::NAN, 0.1, 0.5).is_err());
        assert!(EnvelopeConfig::try_new(0.1, 0.1, f64::NAN, 0.5).is_err());
        assert!(EnvelopeConfig::try_new(0.1, 0.1, 0.1, f64::NAN).is_err());
    }

    #[test]
    fn default_is_valid() {
        let cfg = EnvelopeConfig::default();
        assert!(cfg.attack() >= 0.0);
        assert!(cfg.decay() >= 0.0);
        assert!(cfg.release() >= 0.0);
        assert!((0.0..=1.0).contains(&cfg.sustain()));
    }

    #[test]
    fn with_methods_revalidate() {
        let cfg = EnvelopeConfig::default();
        assert!(cfg.with_attack(-1.0).is_err());
        assert!(cfg.with_decay(-1.0).is_err());
        assert!(cfg.with_release(-1.0).is_err());
        assert!(cfg.with_sustain(2.0).is_err());

        let updated = cfg.with_sustain(0.5).expect("valid update");
        assert_eq!(updated.sustain(), 0.5);
        assert_eq!(updated.attack(), cfg.attack());
    }

    #[test]
    fn copy_semantics() {
        let a = EnvelopeConfig::default();
        let b = a;
        assert_eq!(a, b);
    }
}
