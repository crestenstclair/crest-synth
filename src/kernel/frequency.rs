// path: src/kernel/frequency.rs

//! `Frequency` is a newtype wrapping a frequency in Hertz.
//!
//! Invariant: the wrapped value must be positive and finite (no zero,
//! negative, NaN, or infinite frequencies).

use std::fmt;

/// A frequency in Hz. Guaranteed positive and finite.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Frequency(f64);

/// Error returned when constructing a `Frequency` from an invalid value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrequencyError(f64);

impl fmt::Display for FrequencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "frequency must be positive and finite, got {}", self.0)
    }
}

impl std::error::Error for FrequencyError {}

impl Frequency {
    /// Attempts to construct a `Frequency` from a raw Hz value.
    ///
    /// Returns `Err(FrequencyError)` if `hz` is not positive and finite.
    ///
    /// ```
    /// use crest_synth::kernel::frequency::Frequency;
    ///
    /// assert!(Frequency::try_new(440.0).is_ok());
    /// assert!(Frequency::try_new(0.0).is_err());
    /// assert!(Frequency::try_new(-1.0).is_err());
    /// assert!(Frequency::try_new(f64::NAN).is_err());
    /// assert!(Frequency::try_new(f64::INFINITY).is_err());
    /// ```
    pub fn try_new(hz: f64) -> Result<Self, FrequencyError> {
        if hz.is_nan() || !hz.is_finite() || hz <= 0.0 {
            return Err(FrequencyError(hz));
        }
        Ok(Self(hz))
    }

    /// Returns the wrapped value in Hz.
    pub fn hz(&self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Frequency {
    type Error = FrequencyError;

    fn try_from(hz: f64) -> Result<Self, Self::Error> {
        Self::try_new(hz)
    }
}

impl fmt::Display for Frequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Hz", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_positive_finite_value() {
        let freq = Frequency::try_new(440.0).expect("440 Hz is valid");
        assert_eq!(freq.hz(), 440.0);
    }

    #[test]
    fn rejects_zero() {
        assert_eq!(Frequency::try_new(0.0), Err(FrequencyError(0.0)));
    }

    #[test]
    fn rejects_negative() {
        assert_eq!(Frequency::try_new(-20.0), Err(FrequencyError(-20.0)));
    }

    #[test]
    fn rejects_nan() {
        match Frequency::try_new(f64::NAN) {
            Err(FrequencyError(got)) => assert!(got.is_nan()),
            other => panic!("expected FrequencyError, got {other:?}"),
        }
    }

    #[test]
    fn rejects_positive_infinity() {
        assert_eq!(
            Frequency::try_new(f64::INFINITY),
            Err(FrequencyError(f64::INFINITY))
        );
    }

    #[test]
    fn rejects_negative_infinity() {
        assert_eq!(
            Frequency::try_new(f64::NEG_INFINITY),
            Err(FrequencyError(f64::NEG_INFINITY))
        );
    }

    #[test]
    fn try_from_matches_try_new() {
        let via_try_from: Result<Frequency, FrequencyError> = 220.0.try_into();
        assert_eq!(via_try_from, Frequency::try_new(220.0));
    }

    #[test]
    fn display_formats_with_unit() {
        let freq = Frequency::try_new(880.0).expect("880 Hz is valid");
        assert_eq!(freq.to_string(), "880 Hz");
    }
}
