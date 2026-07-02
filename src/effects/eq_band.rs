// path: src/effects/eq_band.rs

use std::fmt;

/// The filter shape applied by a single parametric EQ band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EqBandType {
    LowShelf,
    HighShelf,
    Peaking,
    LowPass,
    HighPass,
    Notch,
}

/// A newtype for a frequency expressed in Hertz. Must be finite and positive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frequency(f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrequencyError;

impl fmt::Display for FrequencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "frequency must be finite and positive")
    }
}

impl std::error::Error for FrequencyError {}

impl Frequency {
    pub fn try_new(hz: f64) -> Result<Self, FrequencyError> {
        if hz.is_nan() || hz <= 0.0 {
            return Err(FrequencyError);
        }
        Ok(Self(hz))
    }

    pub fn hz(&self) -> f64 {
        self.0
    }
}

/// A newtype for a gain expressed in decibels. Must be finite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decibel(f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecibelError;

impl fmt::Display for DecibelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gain must be finite")
    }
}

impl std::error::Error for DecibelError {}

impl Decibel {
    pub fn try_new(db: f64) -> Result<Self, DecibelError> {
        if db.is_nan() || db.is_infinite() {
            return Err(DecibelError);
        }
        Ok(Self(db))
    }

    pub fn db(&self) -> f64 {
        self.0
    }
}

/// Errors that can occur when constructing or mutating an `EqBand`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqBandError {
    InvalidQ,
}

impl fmt::Display for EqBandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EqBandError::InvalidQ => write!(f, "q must be positive"),
        }
    }
}

impl std::error::Error for EqBandError {}

/// One parametric EQ band: shape, center frequency, gain, and resonance (Q).
///
/// `q` must be positive (invariant enforced at construction and on every
/// mutator that changes it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqBand {
    band_type: EqBandType,
    frequency: Frequency,
    gain_db: Decibel,
    q: f64,
}

impl EqBand {
    pub fn try_new(
        band_type: EqBandType,
        frequency: Frequency,
        gain_db: Decibel,
        q: f64,
    ) -> Result<Self, EqBandError> {
        let q = Self::validated_q(q)?;
        Ok(Self {
            band_type,
            frequency,
            gain_db,
            q,
        })
    }

    fn validated_q(q: f64) -> Result<f64, EqBandError> {
        if q.is_nan() || q <= 0.0 {
            return Err(EqBandError::InvalidQ);
        }
        Ok(q)
    }

    pub fn band_type(&self) -> EqBandType {
        self.band_type
    }

    pub fn frequency(&self) -> Frequency {
        self.frequency
    }

    pub fn gain_db(&self) -> Decibel {
        self.gain_db
    }

    pub fn q(&self) -> f64 {
        self.q
    }

    pub fn with_band_type(&self, band_type: EqBandType) -> Self {
        Self { band_type, ..*self }
    }

    pub fn with_frequency(&self, frequency: Frequency) -> Self {
        Self { frequency, ..*self }
    }

    pub fn with_gain_db(&self, gain_db: Decibel) -> Self {
        Self { gain_db, ..*self }
    }

    pub fn with_q(&self, q: f64) -> Result<Self, EqBandError> {
        let q = Self::validated_q(q)?;
        Ok(Self { q, ..*self })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_frequency() -> Frequency {
        Frequency::try_new(1000.0).unwrap()
    }

    fn sample_gain() -> Decibel {
        Decibel::try_new(0.0).unwrap()
    }

    #[test]
    fn constructs_with_valid_q() {
        let band = EqBand::try_new(EqBandType::Peaking, sample_frequency(), sample_gain(), 0.707)
            .unwrap();
        assert_eq!(band.q(), 0.707);
        assert_eq!(band.band_type(), EqBandType::Peaking);
    }

    #[test]
    fn rejects_zero_q() {
        let result = EqBand::try_new(EqBandType::LowShelf, sample_frequency(), sample_gain(), 0.0);
        assert_eq!(result, Err(EqBandError::InvalidQ));
    }

    #[test]
    fn rejects_negative_q() {
        let result =
            EqBand::try_new(EqBandType::HighShelf, sample_frequency(), sample_gain(), -1.0);
        assert_eq!(result, Err(EqBandError::InvalidQ));
    }

    #[test]
    fn rejects_nan_q() {
        let result =
            EqBand::try_new(EqBandType::Notch, sample_frequency(), sample_gain(), f64::NAN);
        assert_eq!(result, Err(EqBandError::InvalidQ));
    }

    #[test]
    fn with_q_updates_immutably_and_validates() {
        let band =
            EqBand::try_new(EqBandType::Peaking, sample_frequency(), sample_gain(), 1.0).unwrap();
        let updated = band.with_q(2.0).unwrap();
        assert_eq!(updated.q(), 2.0);
        assert_eq!(band.q(), 1.0);
        assert_eq!(band.with_q(0.0), Err(EqBandError::InvalidQ));
    }

    #[test]
    fn with_frequency_and_gain_update_immutably() {
        let band =
            EqBand::try_new(EqBandType::LowPass, sample_frequency(), sample_gain(), 1.0).unwrap();
        let new_freq = Frequency::try_new(2000.0).unwrap();
        let new_gain = Decibel::try_new(-6.0).unwrap();
        let updated = band.with_frequency(new_freq).with_gain_db(new_gain);
        assert_eq!(updated.frequency().hz(), 2000.0);
        assert_eq!(updated.gain_db().db(), -6.0);
        assert_eq!(band.frequency().hz(), 1000.0);
    }

    #[test]
    fn frequency_rejects_non_positive_or_nan() {
        assert!(Frequency::try_new(0.0).is_err());
        assert!(Frequency::try_new(-10.0).is_err());
        assert!(Frequency::try_new(f64::NAN).is_err());
    }

    #[test]
    fn decibel_rejects_non_finite() {
        assert!(Decibel::try_new(f64::NAN).is_err());
        assert!(Decibel::try_new(f64::INFINITY).is_err());
        assert!(Decibel::try_new(f64::NEG_INFINITY).is_err());
    }
}
