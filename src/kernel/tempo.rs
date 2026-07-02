// path: src/kernel/tempo.rs

//! `Tempo` is a newtype wrapping a tempo in beats per minute (BPM).
//!
//! Invariant: the wrapped value must be positive and finite (no zero,
//! negative, NaN, or infinite tempos).

use std::fmt;

/// A tempo in beats per minute (BPM). Guaranteed positive and finite.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Tempo(f64);

/// Error returned when constructing a `Tempo` from an invalid value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempoError(f64);

impl fmt::Display for TempoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tempo must be positive and finite, got {}", self.0)
    }
}

impl std::error::Error for TempoError {}

impl Tempo {
    /// Attempts to construct a `Tempo` from a raw BPM value.
    ///
    /// Returns `Err(TempoError)` if `bpm` is not positive and finite.
    ///
    /// ```
    /// use crest_synth::kernel::tempo::Tempo;
    ///
    /// assert!(Tempo::try_new(120.0).is_ok());
    /// assert!(Tempo::try_new(0.0).is_err());
    /// assert!(Tempo::try_new(-1.0).is_err());
    /// assert!(Tempo::try_new(f64::NAN).is_err());
    /// assert!(Tempo::try_new(f64::INFINITY).is_err());
    /// ```
    pub fn try_new(bpm: f64) -> Result<Self, TempoError> {
        if bpm.is_nan() || !bpm.is_finite() || bpm <= 0.0 {
            return Err(TempoError(bpm));
        }
        Ok(Self(bpm))
    }

    /// Returns the wrapped value in beats per minute.
    pub fn bpm(&self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Tempo {
    type Error = TempoError;

    fn try_from(bpm: f64) -> Result<Self, Self::Error> {
        Self::try_new(bpm)
    }
}

impl fmt::Display for Tempo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} BPM", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_positive_finite_value() {
        let tempo = Tempo::try_new(120.0).expect("120 BPM is valid");
        assert_eq!(tempo.bpm(), 120.0);
    }

    #[test]
    fn rejects_zero() {
        assert_eq!(Tempo::try_new(0.0), Err(TempoError(0.0)));
    }

    #[test]
    fn rejects_negative() {
        assert_eq!(Tempo::try_new(-20.0), Err(TempoError(-20.0)));
    }

    #[test]
    fn rejects_nan() {
        match Tempo::try_new(f64::NAN) {
            Err(TempoError(got)) => assert!(got.is_nan()),
            other => panic!("expected TempoError, got {other:?}"),
        }
    }

    #[test]
    fn rejects_positive_infinity() {
        assert_eq!(
            Tempo::try_new(f64::INFINITY),
            Err(TempoError(f64::INFINITY))
        );
    }

    #[test]
    fn rejects_negative_infinity() {
        assert_eq!(
            Tempo::try_new(f64::NEG_INFINITY),
            Err(TempoError(f64::NEG_INFINITY))
        );
    }

    #[test]
    fn try_from_matches_try_new() {
        let via_try_from: Result<Tempo, TempoError> = 90.0.try_into();
        assert_eq!(via_try_from, Tempo::try_new(90.0));
    }

    #[test]
    fn display_formats_with_unit() {
        let tempo = Tempo::try_new(140.0).expect("140 BPM is valid");
        assert_eq!(tempo.to_string(), "140 BPM");
    }
}
