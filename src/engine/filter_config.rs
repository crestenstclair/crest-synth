// path: src/engine/filter_config.rs

//! Filter configuration for a single voice.

use std::fmt;

/// Minimum permissible cutoff frequency, in Hertz.
const MIN_CUTOFF_HZ: f64 = 20.0;
/// Maximum permissible cutoff frequency, in Hertz.
const MAX_CUTOFF_HZ: f64 = 20_000.0;

/// Error returned when constructing a [`Frequency`] outside the audible
/// range (20-20000 Hz).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrequencyRangeError {
    pub value: f64,
}

impl fmt::Display for FrequencyRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "frequency {} Hz is outside the valid range {}-{} Hz",
            self.value, MIN_CUTOFF_HZ, MAX_CUTOFF_HZ
        )
    }
}

impl std::error::Error for FrequencyRangeError {}

/// A validated audio frequency in Hertz, constrained to the audible range
/// used for filter cutoffs (20 Hz - 20,000 Hz).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frequency(f64);

impl Frequency {
    /// Constructs a `Frequency`, validating that `hz` is finite and within
    /// 20-20000 Hz.
    pub fn try_new(hz: f64) -> Result<Self, FrequencyRangeError> {
        if hz.is_nan() || !(MIN_CUTOFF_HZ..=MAX_CUTOFF_HZ).contains(&hz) {
            return Err(FrequencyRangeError { value: hz });
        }
        Ok(Self(hz))
    }

    /// Returns the frequency value in Hertz.
    pub fn hz(&self) -> f64 {
        self.0
    }
}

/// The kind of filter topology applied to a voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

/// Error returned when a [`FilterConfig`] is constructed with an invalid
/// field value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterConfigError {
    Cutoff(FrequencyRangeError),
}

impl fmt::Display for FilterConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterConfigError::Cutoff(e) => write!(f, "invalid filter config: {e}"),
        }
    }
}

impl std::error::Error for FilterConfigError {}

/// Filter settings for one voice.
///
/// `drive`, `envelope_amount`, `key_tracking`, and `resonance` are stored as
/// raw `f64` per the resource declaration; only `cutoff_hz` carries a
/// validated range (20-20000 Hz), enforced by [`Frequency::try_new`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterConfig {
    cutoff_hz: Frequency,
    drive: f64,
    envelope_amount: f64,
    filter_type: FilterType,
    key_tracking: f64,
    resonance: f64,
}

impl FilterConfig {
    /// Constructs a `FilterConfig`, validating `cutoff_hz` against the
    /// audible range (20-20000 Hz).
    pub fn try_new(
        cutoff_hz: f64,
        drive: f64,
        envelope_amount: f64,
        filter_type: FilterType,
        key_tracking: f64,
        resonance: f64,
    ) -> Result<Self, FilterConfigError> {
        let cutoff_hz = Frequency::try_new(cutoff_hz).map_err(FilterConfigError::Cutoff)?;
        Ok(Self {
            cutoff_hz,
            drive,
            envelope_amount,
            filter_type,
            key_tracking,
            resonance,
        })
    }

    /// Constructs a `FilterConfig` from an already-validated [`Frequency`].
    pub fn from_frequency(
        cutoff_hz: Frequency,
        drive: f64,
        envelope_amount: f64,
        filter_type: FilterType,
        key_tracking: f64,
        resonance: f64,
    ) -> Self {
        Self {
            cutoff_hz,
            drive,
            envelope_amount,
            filter_type,
            key_tracking,
            resonance,
        }
    }

    pub fn cutoff_hz(&self) -> Frequency {
        self.cutoff_hz
    }

    pub fn drive(&self) -> f64 {
        self.drive
    }

    pub fn envelope_amount(&self) -> f64 {
        self.envelope_amount
    }

    pub fn filter_type(&self) -> FilterType {
        self.filter_type
    }

    pub fn key_tracking(&self) -> f64 {
        self.key_tracking
    }

    pub fn resonance(&self) -> f64 {
        self.resonance
    }

    /// Returns a copy of this config with `cutoff_hz` replaced, validating
    /// the new value.
    pub fn with_cutoff_hz(&self, cutoff_hz: f64) -> Result<Self, FilterConfigError> {
        let cutoff_hz = Frequency::try_new(cutoff_hz).map_err(FilterConfigError::Cutoff)?;
        Ok(Self { cutoff_hz, ..*self })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_lower_bound() {
        let config = FilterConfig::try_new(20.0, 0.0, 0.0, FilterType::LowPass, 0.0, 0.0)
            .expect("20 Hz is the inclusive lower bound");
        assert_eq!(config.cutoff_hz().hz(), 20.0);
    }

    #[test]
    fn try_new_accepts_upper_bound() {
        let config = FilterConfig::try_new(20_000.0, 0.0, 0.0, FilterType::LowPass, 0.0, 0.0)
            .expect("20000 Hz is the inclusive upper bound");
        assert_eq!(config.cutoff_hz().hz(), 20_000.0);
    }

    #[test]
    fn try_new_rejects_below_range() {
        let result = FilterConfig::try_new(19.999, 0.0, 0.0, FilterType::LowPass, 0.0, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn try_new_rejects_above_range() {
        let result = FilterConfig::try_new(20_000.001, 0.0, 0.0, FilterType::LowPass, 0.0, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn try_new_rejects_nan() {
        let result = FilterConfig::try_new(f64::NAN, 0.0, 0.0, FilterType::LowPass, 0.0, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn with_cutoff_hz_updates_only_cutoff() {
        let config = FilterConfig::try_new(1000.0, 0.5, 0.25, FilterType::BandPass, 0.1, 0.2)
            .expect("valid config");
        let updated = config.with_cutoff_hz(2000.0).expect("valid new cutoff");
        assert_eq!(updated.cutoff_hz().hz(), 2000.0);
        assert_eq!(updated.drive(), 0.5);
        assert_eq!(updated.filter_type(), FilterType::BandPass);
    }

    #[test]
    fn with_cutoff_hz_rejects_invalid() {
        let config = FilterConfig::try_new(1000.0, 0.0, 0.0, FilterType::LowPass, 0.0, 0.0)
            .expect("valid config");
        assert!(config.with_cutoff_hz(0.0).is_err());
    }
}
