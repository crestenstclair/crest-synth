//! `SampleRate` — samples per second, e.g. 44100 or 48000.
//!
//! A newtype around `u32` that guarantees the wrapped value is positive.
//! Construction always goes through [`SampleRate::try_new`] (or the
//! `TryFrom<u32>` impl), so a `SampleRate` in scope is always valid.

use std::convert::TryFrom;
use std::fmt;

/// Samples per second, e.g. 44100 or 48000.
///
/// Invariant: the wrapped value must be positive (non-zero).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SampleRate(u32);

/// Error returned when constructing a [`SampleRate`] from an invalid value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleRateError {
    /// The provided value was zero, which is not a valid sample rate.
    Zero,
}

impl fmt::Display for SampleRateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SampleRateError::Zero => write!(f, "sample rate must be positive, got 0"),
        }
    }
}

impl std::error::Error for SampleRateError {}

impl SampleRate {
    /// Attempts to construct a `SampleRate` from a raw `u32`.
    ///
    /// # Errors
    ///
    /// Returns [`SampleRateError::Zero`] if `value` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::kernel::sample_rate::SampleRate;
    ///
    /// let rate = SampleRate::try_new(44_100).unwrap();
    /// assert_eq!(rate.get(), 44_100);
    ///
    /// assert!(SampleRate::try_new(0).is_err());
    /// ```
    pub fn try_new(value: u32) -> Result<Self, SampleRateError> {
        if value == 0 {
            Err(SampleRateError::Zero)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the wrapped sample-rate value as `u32`.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Returns the sample rate expressed as `f64`, useful for period and
    /// frequency calculations.
    #[must_use]
    pub fn as_f64(self) -> f64 {
        f64::from(self.0)
    }
}

impl TryFrom<u32> for SampleRate {
    type Error = SampleRateError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<SampleRate> for u32 {
    fn from(rate: SampleRate) -> Self {
        rate.0
    }
}

impl fmt::Display for SampleRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_common_rates() {
        assert_eq!(SampleRate::try_new(44_100).unwrap().get(), 44_100);
        assert_eq!(SampleRate::try_new(48_000).unwrap().get(), 48_000);
    }

    #[test]
    fn try_new_rejects_zero() {
        assert_eq!(SampleRate::try_new(0), Err(SampleRateError::Zero));
    }

    #[test]
    fn try_from_delegates_to_try_new() {
        let rate = SampleRate::try_from(96_000).unwrap();
        assert_eq!(rate.get(), 96_000);

        assert!(SampleRate::try_from(0).is_err());
    }

    #[test]
    fn into_u32_round_trips() {
        let rate = SampleRate::try_new(44_100).unwrap();
        let raw: u32 = rate.into();
        assert_eq!(raw, 44_100);
    }

    #[test]
    fn as_f64_converts_without_loss() {
        let rate = SampleRate::try_new(48_000).unwrap();
        assert_eq!(rate.as_f64(), 48_000.0_f64);
    }

    #[test]
    fn display_shows_raw_value() {
        let rate = SampleRate::try_new(44_100).unwrap();
        assert_eq!(rate.to_string(), "44100");
    }

    #[test]
    fn ordering_and_equality_are_by_value() {
        let low = SampleRate::try_new(44_100).unwrap();
        let high = SampleRate::try_new(48_000).unwrap();
        assert!(low < high);
        assert_eq!(low, SampleRate::try_new(44_100).unwrap());
    }

    #[test]
    fn error_message_is_descriptive() {
        let err = SampleRate::try_new(0).unwrap_err();
        assert_eq!(err.to_string(), "sample rate must be positive, got 0");
    }
}
