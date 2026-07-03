// path: src/kernel/amplitude.rs

use std::fmt;

/// Linear amplitude: `0.0` is silence, `1.0` is unity gain.
///
/// An `Amplitude` is always non-negative and finite. Construct one through
/// [`Amplitude::try_new`] (or [`TryFrom<f64>`]) to validate untrusted input,
/// or use the [`Amplitude::SILENCE`] / [`Amplitude::UNITY`] constants for
/// known-good values.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Amplitude(f64);

/// Error returned when constructing an [`Amplitude`] from an invalid value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InvalidAmplitude(pub f64);

impl fmt::Display for InvalidAmplitude {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "amplitude must be non-negative and finite, got {}",
            self.0
        )
    }
}

impl std::error::Error for InvalidAmplitude {}

impl Amplitude {
    /// Silence: zero amplitude.
    pub const SILENCE: Amplitude = Amplitude(0.0);

    /// Unity gain: amplitude of `1.0`.
    pub const UNITY: Amplitude = Amplitude(1.0);

    /// Construct an `Amplitude`, validating that `value` is non-negative and finite.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidAmplitude`] if `value` is negative, `NaN`, or infinite.
    pub fn try_new(value: f64) -> Result<Self, InvalidAmplitude> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(InvalidAmplitude(value))
        }
    }

    /// Returns the underlying linear amplitude value.
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for Amplitude {
    fn default() -> Self {
        Self::SILENCE
    }
}

impl TryFrom<f64> for Amplitude {
    type Error = InvalidAmplitude;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<Amplitude> for f64 {
    fn from(amplitude: Amplitude) -> f64 {
        amplitude.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_zero_and_positive_finite_values() {
        assert!(Amplitude::try_new(0.0).is_ok());
        assert!(Amplitude::try_new(1.0).is_ok());
        assert!(Amplitude::try_new(2.5).is_ok());
    }

    #[test]
    fn rejects_negative_values() {
        assert!(Amplitude::try_new(-0.1).is_err());
    }

    #[test]
    fn rejects_non_finite_values() {
        assert!(Amplitude::try_new(f64::NAN).is_err());
        assert!(Amplitude::try_new(f64::INFINITY).is_err());
        assert!(Amplitude::try_new(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn silence_and_unity_constants_hold_expected_values() {
        assert_eq!(Amplitude::SILENCE.value(), 0.0);
        assert_eq!(Amplitude::UNITY.value(), 1.0);
    }

    #[test]
    fn default_is_silence() {
        assert_eq!(Amplitude::default(), Amplitude::SILENCE);
    }

    #[test]
    fn try_from_matches_try_new() {
        let result: Result<Amplitude, _> = Amplitude::try_from(0.5);
        assert!(result.is_ok());
    }

    #[test]
    fn converts_back_to_f64() {
        let amplitude = Amplitude::try_new(0.75).expect("valid amplitude");
        let value: f64 = amplitude.into();
        assert_eq!(value, 0.75);
    }
}
