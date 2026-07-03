// path: src/mixer/peak_level.rs

//! `PeakLevel` — the most recent peak absolute sample level for metering.
//!
//! Metering values are produced on the audio thread (from the running max of
//! `|sample|` over a block) and read on the UI thread through a snapshot —
//! never mutated in place by anyone but the meter that owns it. The type
//! itself only enforces the domain invariant: a peak level can never be
//! negative.

use std::error::Error;
use std::fmt;

/// The most recent peak absolute sample level for metering.
///
/// Always finite and non-negative. There is no upper bound: callers that
/// need to detect clipping compare the raw value against their own
/// threshold (e.g. `1.0` for a normalized float signal).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PeakLevel(f64);

/// Explains why a candidate `f64` could not become a [`PeakLevel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeakLevelError {
    /// The value was NaN, which has no defined ordering against zero.
    NotANumber,
    /// The value was negative; a peak level measures magnitude only.
    Negative,
}

impl fmt::Display for PeakLevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeakLevelError::NotANumber => write!(f, "peak level must be a real number, got NaN"),
            PeakLevelError::Negative => write!(f, "peak level must be non-negative"),
        }
    }
}

impl Error for PeakLevelError {}

impl PeakLevel {
    /// The zero (silent) peak level.
    pub const SILENT: PeakLevel = PeakLevel(0.0);

    /// Attempts to construct a `PeakLevel` from a raw sample magnitude.
    ///
    /// # Errors
    ///
    /// Returns [`PeakLevelError::NotANumber`] if `value` is NaN, or
    /// [`PeakLevelError::Negative`] if `value` is negative.
    ///
    /// ```
    /// use crest_synth::mixer::peak_level::PeakLevel;
    ///
    /// assert!(PeakLevel::try_new(0.0).is_ok());
    /// assert!(PeakLevel::try_new(1.25).is_ok());
    /// assert!(PeakLevel::try_new(-0.1).is_err());
    /// assert!(PeakLevel::try_new(f64::NAN).is_err());
    /// ```
    pub fn try_new(value: f64) -> Result<Self, PeakLevelError> {
        if value.is_nan() {
            return Err(PeakLevelError::NotANumber);
        }
        if value < 0.0 {
            return Err(PeakLevelError::Negative);
        }
        Ok(PeakLevel(value))
    }

    /// Returns the raw sample magnitude.
    pub fn raw(self) -> f64 {
        self.0
    }

    /// Combines two peak readings into the running peak, i.e. the larger of
    /// the two. Useful for folding successive block peaks into one meter
    /// reading without ever needing a lock: both operands are `Copy`.
    pub fn max(self, other: PeakLevel) -> PeakLevel {
        if self.0 >= other.0 {
            self
        } else {
            other
        }
    }

    /// Derives a peak level from a slice of samples by taking the maximum
    /// absolute value. Returns [`PeakLevel::SILENT`] for an empty slice.
    ///
    /// This performs no heap allocation and is safe to call from the audio
    /// thread's metering step.
    pub fn from_samples(samples: &[f32]) -> PeakLevel {
        let mut peak = 0.0_f64;
        for &sample in samples {
            let magnitude = f64::from(sample.abs());
            if magnitude > peak {
                peak = magnitude;
            }
        }
        PeakLevel(peak)
    }
}

impl Default for PeakLevel {
    fn default() -> Self {
        PeakLevel::SILENT
    }
}

impl fmt::Display for PeakLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6}", self.0)
    }
}

impl TryFrom<f64> for PeakLevel {
    type Error = PeakLevelError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        PeakLevel::try_new(value)
    }
}

impl From<PeakLevel> for f64 {
    fn from(level: PeakLevel) -> Self {
        level.raw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_valid() {
        let level = PeakLevel::try_new(0.0).expect("zero must be a valid peak level");
        assert_eq!(level.raw(), 0.0);
    }

    #[test]
    fn positive_value_is_valid() {
        let level = PeakLevel::try_new(0.87).expect("positive value must be valid");
        assert_eq!(level.raw(), 0.87);
    }

    #[test]
    fn value_above_unity_is_valid() {
        let level = PeakLevel::try_new(1.5).expect("values above unity are valid");
        assert_eq!(level.raw(), 1.5);
    }

    #[test]
    fn negative_value_is_rejected() {
        let err = PeakLevel::try_new(-0.001).expect_err("negative value must be rejected");
        assert_eq!(err, PeakLevelError::Negative);
    }

    #[test]
    fn nan_is_rejected() {
        let err = PeakLevel::try_new(f64::NAN).expect_err("NaN must be rejected");
        assert_eq!(err, PeakLevelError::NotANumber);
    }

    #[test]
    fn default_is_silent() {
        assert_eq!(PeakLevel::default(), PeakLevel::SILENT);
    }

    #[test]
    fn max_picks_larger_reading() {
        let a = PeakLevel::try_new(0.2).unwrap();
        let b = PeakLevel::try_new(0.5).unwrap();
        assert_eq!(a.max(b), b);
        assert_eq!(b.max(a), b);
    }

    #[test]
    fn from_samples_takes_absolute_peak() {
        let samples = [0.1_f32, -0.9, 0.4, -0.2];
        let level = PeakLevel::from_samples(&samples);
        assert!((level.raw() - 0.9).abs() < 1e-6);
    }

    #[test]
    fn from_samples_empty_slice_is_silent() {
        let level = PeakLevel::from_samples(&[]);
        assert_eq!(level, PeakLevel::SILENT);
    }

    #[test]
    fn try_from_matches_try_new() {
        let via_trait: Result<PeakLevel, _> = PeakLevel::try_from(0.42);
        let via_ctor = PeakLevel::try_new(0.42);
        assert_eq!(via_trait, via_ctor);
    }

    #[test]
    fn into_f64_round_trips() {
        let level = PeakLevel::try_new(0.63).unwrap();
        let raw: f64 = level.into();
        assert_eq!(raw, 0.63);
    }

    #[test]
    fn ordering_reflects_magnitude() {
        let low = PeakLevel::try_new(0.1).unwrap();
        let high = PeakLevel::try_new(0.9).unwrap();
        assert!(low < high);
    }

    #[test]
    fn display_formats_with_fixed_precision() {
        let level = PeakLevel::try_new(0.5).unwrap();
        assert_eq!(format!("{}", level), "0.500000");
    }
}
