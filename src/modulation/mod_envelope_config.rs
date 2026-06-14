// path: src/modulation/mod_envelope_config.rs

/// ADSR envelope configuration for modulation routing: attack/decay/release times in seconds
/// and sustain level (0.0–1.0). This envelope can be routed to arbitrary modulation destinations.
///
/// All time values must be non-negative. Sustain must be in the range 0.0–1.0 inclusive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModEnvelopeConfig {
    /// Attack time in seconds (≥ 0.0).
    pub attack: f64,
    /// Decay time in seconds (≥ 0.0).
    pub decay: f64,
    /// Sustain level (0.0–1.0 inclusive).
    pub sustain: f64,
    /// Release time in seconds (≥ 0.0).
    pub release: f64,
}

/// Error returned when a `ModEnvelopeConfig` field is out of range or NaN.
#[derive(Debug, Clone, PartialEq)]
pub struct ModEnvelopeConfigError(String);

impl std::fmt::Display for ModEnvelopeConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ModEnvelopeConfig validation error: {}", self.0)
    }
}

impl std::error::Error for ModEnvelopeConfigError {}

impl ModEnvelopeConfig {
    /// Construct a `ModEnvelopeConfig` from raw field values.
    ///
    /// Returns `Err` if any of the following invariants are violated:
    /// - `attack`, `decay`, or `release` is NaN or negative
    /// - `sustain` is NaN or outside 0.0–1.0
    ///
    /// ```
    /// use crest_synth::modulation::mod_envelope_config::ModEnvelopeConfig;
    /// assert!(ModEnvelopeConfig::try_new(0.01, 0.1, 0.8, 0.3).is_ok());
    /// assert!(ModEnvelopeConfig::try_new(-0.01, 0.1, 0.8, 0.3).is_err());
    /// assert!(ModEnvelopeConfig::try_new(0.01, 0.1, 1.1, 0.3).is_err());
    /// ```
    pub fn try_new(
        attack: f64,
        decay: f64,
        sustain: f64,
        release: f64,
    ) -> Result<Self, ModEnvelopeConfigError> {
        if attack.is_nan() || attack < 0.0 {
            return Err(ModEnvelopeConfigError(format!(
                "attack must be non-negative, got {}",
                attack
            )));
        }
        if decay.is_nan() || decay < 0.0 {
            return Err(ModEnvelopeConfigError(format!(
                "decay must be non-negative, got {}",
                decay
            )));
        }
        if sustain.is_nan() || !(0.0..=1.0).contains(&sustain) {
            return Err(ModEnvelopeConfigError(format!(
                "sustain must be in 0.0-1.0, got {}",
                sustain
            )));
        }
        if release.is_nan() || release < 0.0 {
            return Err(ModEnvelopeConfigError(format!(
                "release must be non-negative, got {}",
                release
            )));
        }
        Ok(Self {
            attack,
            decay,
            sustain,
            release,
        })
    }
}

impl Default for ModEnvelopeConfig {
    /// Returns a default ADSR: 10 ms attack, 100 ms decay, 0.8 sustain, 300 ms release.
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.8,
            release: 0.3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_config_is_accepted() {
        let cfg = ModEnvelopeConfig::try_new(0.01, 0.1, 0.8, 0.3).unwrap();
        assert!((cfg.attack - 0.01).abs() < f64::EPSILON);
        assert!((cfg.decay - 0.1).abs() < f64::EPSILON);
        assert!((cfg.sustain - 0.8).abs() < f64::EPSILON);
        assert!((cfg.release - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn zero_times_are_valid() {
        assert!(ModEnvelopeConfig::try_new(0.0, 0.0, 0.0, 0.0).is_ok());
    }

    #[test]
    fn sustain_zero_and_one_are_valid() {
        assert!(ModEnvelopeConfig::try_new(0.0, 0.0, 0.0, 0.0).is_ok());
        assert!(ModEnvelopeConfig::try_new(0.0, 0.0, 1.0, 0.0).is_ok());
    }

    #[test]
    fn negative_attack_is_rejected() {
        assert!(ModEnvelopeConfig::try_new(-0.001, 0.1, 0.8, 0.3).is_err());
    }

    #[test]
    fn negative_decay_is_rejected() {
        assert!(ModEnvelopeConfig::try_new(0.01, -0.001, 0.8, 0.3).is_err());
    }

    #[test]
    fn negative_release_is_rejected() {
        assert!(ModEnvelopeConfig::try_new(0.01, 0.1, 0.8, -0.001).is_err());
    }

    #[test]
    fn sustain_above_one_is_rejected() {
        assert!(ModEnvelopeConfig::try_new(0.01, 0.1, 1.001, 0.3).is_err());
    }

    #[test]
    fn sustain_below_zero_is_rejected() {
        assert!(ModEnvelopeConfig::try_new(0.01, 0.1, -0.001, 0.3).is_err());
    }

    #[test]
    fn nan_attack_is_rejected() {
        assert!(ModEnvelopeConfig::try_new(f64::NAN, 0.1, 0.8, 0.3).is_err());
    }

    #[test]
    fn nan_decay_is_rejected() {
        assert!(ModEnvelopeConfig::try_new(0.01, f64::NAN, 0.8, 0.3).is_err());
    }

    #[test]
    fn nan_sustain_is_rejected() {
        assert!(ModEnvelopeConfig::try_new(0.01, 0.1, f64::NAN, 0.3).is_err());
    }

    #[test]
    fn nan_release_is_rejected() {
        assert!(ModEnvelopeConfig::try_new(0.01, 0.1, 0.8, f64::NAN).is_err());
    }

    #[test]
    fn default_is_valid() {
        let cfg = ModEnvelopeConfig::default();
        assert!(
            ModEnvelopeConfig::try_new(cfg.attack, cfg.decay, cfg.sustain, cfg.release).is_ok()
        );
    }

    #[test]
    fn copy_semantics() {
        let a = ModEnvelopeConfig::default();
        let b = a;
        assert_eq!(a, b);
    }
}
