// path: src/effects/compressor_params.rs

//! Dynamics compressor settings.

use std::error::Error;
use std::fmt;

/// A gain value expressed in decibels.
///
/// Newtype wrapper to prevent accidental mixing of decibel values with
/// plain linear gain or other unrelated `f64` quantities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decibel(f64);

impl Decibel {
    /// Creates a new decibel value.
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    /// Returns the underlying value in decibels.
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl fmt::Display for Decibel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} dB", self.0)
    }
}

impl From<f64> for Decibel {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

/// Error returned when constructing a [`CompressorParams`] with an invalid
/// combination of values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressorParamsError {
    /// The compression ratio must be at least `1.0` (1:1, i.e. no
    /// compression).
    RatioTooLow { ratio: f64 },
}

impl fmt::Display for CompressorParamsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompressorParamsError::RatioTooLow { ratio } => {
                write!(f, "compressor ratio must be >= 1.0, got {ratio}")
            }
        }
    }
}

impl Error for CompressorParamsError {}

/// Dynamics compressor settings.
///
/// Invariant: `ratio >= 1.0`. Enforced at construction time via [`CompressorParams::new`];
/// there is no way to obtain an instance that violates it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressorParams {
    attack_ms: f64,
    knee_db: f64,
    makeup_db: Decibel,
    ratio: f64,
    release_ms: f64,
    threshold_db: Decibel,
}

impl CompressorParams {
    /// Creates a new `CompressorParams`, validating the ratio invariant.
    ///
    /// Returns `Err` if `ratio < 1.0`.
    pub fn new(
        attack_ms: f64,
        knee_db: f64,
        makeup_db: Decibel,
        ratio: f64,
        release_ms: f64,
        threshold_db: Decibel,
    ) -> Result<Self, CompressorParamsError> {
        if ratio < 1.0 {
            return Err(CompressorParamsError::RatioTooLow { ratio });
        }
        Ok(Self {
            attack_ms,
            knee_db,
            makeup_db,
            ratio,
            release_ms,
            threshold_db,
        })
    }

    /// Attack time in milliseconds.
    pub fn attack_ms(&self) -> f64 {
        self.attack_ms
    }

    /// Knee width in decibels.
    pub fn knee_db(&self) -> f64 {
        self.knee_db
    }

    /// Makeup gain applied after compression.
    pub fn makeup_db(&self) -> Decibel {
        self.makeup_db
    }

    /// Compression ratio; guaranteed `>= 1.0`.
    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    /// Release time in milliseconds.
    pub fn release_ms(&self) -> f64 {
        self.release_ms
    }

    /// Threshold above which compression is applied.
    pub fn threshold_db(&self) -> Decibel {
        self.threshold_db
    }
}

impl Default for CompressorParams {
    fn default() -> Self {
        Self {
            attack_ms: 10.0,
            knee_db: 0.0,
            makeup_db: Decibel::new(0.0),
            ratio: 4.0,
            release_ms: 100.0,
            threshold_db: Decibel::new(-24.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ratio_of_exactly_one() {
        let params = CompressorParams::new(
            10.0,
            2.0,
            Decibel::new(0.0),
            1.0,
            100.0,
            Decibel::new(-20.0),
        );
        assert!(params.is_ok());
    }

    #[test]
    fn accepts_ratio_above_one() {
        let params = CompressorParams::new(
            5.0,
            1.0,
            Decibel::new(3.0),
            8.0,
            150.0,
            Decibel::new(-18.0),
        );
        assert!(params.is_ok());
    }

    #[test]
    fn rejects_ratio_below_one() {
        let result = CompressorParams::new(
            10.0,
            2.0,
            Decibel::new(0.0),
            0.5,
            100.0,
            Decibel::new(-20.0),
        );
        assert_eq!(
            result,
            Err(CompressorParamsError::RatioTooLow { ratio: 0.5 })
        );
    }

    #[test]
    fn default_satisfies_ratio_invariant() {
        assert!(CompressorParams::default().ratio() >= 1.0);
    }

    #[test]
    fn accessors_round_trip_constructor_values() {
        let params = CompressorParams::new(
            12.5,
            3.0,
            Decibel::new(6.0),
            4.0,
            250.0,
            Decibel::new(-15.0),
        )
        .expect("valid ratio");

        assert_eq!(params.attack_ms(), 12.5);
        assert_eq!(params.knee_db(), 3.0);
        assert_eq!(params.makeup_db().value(), 6.0);
        assert_eq!(params.ratio(), 4.0);
        assert_eq!(params.release_ms(), 250.0);
        assert_eq!(params.threshold_db().value(), -15.0);
    }
}
