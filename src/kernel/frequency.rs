/// Frequency in Hz.
///
/// `Frequency` wraps a `f64` and enforces the invariant that the value
/// is strictly positive (> 0.0) and not NaN or infinite.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Frequency(f64);

/// Error returned when a `Frequency` value is not strictly positive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrequencyError(f64);

impl std::fmt::Display for FrequencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Frequency value {} is not a valid positive Hz value",
            self.0
        )
    }
}

impl std::error::Error for FrequencyError {}

impl Frequency {
    /// Construct a `Frequency` from a raw `f64` in Hz.
    ///
    /// Returns `Err` if the value is NaN, infinite, or not strictly positive (≤ 0.0).
    ///
    /// ```
    /// use crest_synth::kernel::frequency::Frequency;
    /// assert!(Frequency::try_new(440.0).is_ok());
    /// assert!(Frequency::try_new(0.0).is_err());
    /// assert!(Frequency::try_new(-1.0).is_err());
    /// ```
    pub fn try_new(value: f64) -> Result<Self, FrequencyError> {
        if value.is_nan() || value.is_infinite() || value <= 0.0 {
            return Err(FrequencyError(value));
        }
        Ok(Self(value))
    }

    /// Return the underlying `f64` value in Hz.
    #[inline]
    pub fn value(self) -> f64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a440_is_valid() {
        let f = Frequency::try_new(440.0).unwrap();
        assert!((f.value() - 440.0).abs() < f64::EPSILON);
    }

    #[test]
    fn very_small_positive_is_valid() {
        assert!(Frequency::try_new(f64::MIN_POSITIVE).is_ok());
    }

    #[test]
    fn zero_is_rejected() {
        assert!(Frequency::try_new(0.0).is_err());
    }

    #[test]
    fn negative_is_rejected() {
        assert!(Frequency::try_new(-440.0).is_err());
    }

    #[test]
    fn nan_is_rejected() {
        assert!(Frequency::try_new(f64::NAN).is_err());
    }

    #[test]
    fn infinity_is_rejected() {
        assert!(Frequency::try_new(f64::INFINITY).is_err());
    }

    #[test]
    fn neg_infinity_is_rejected() {
        assert!(Frequency::try_new(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn copy_semantics() {
        let a = Frequency::try_new(880.0).unwrap();
        let b = a;
        assert!((a.value() - b.value()).abs() < f64::EPSILON);
    }

    #[test]
    fn error_message_contains_value() {
        let err = Frequency::try_new(-1.0).unwrap_err();
        assert!(err.to_string().contains("-1"));
    }
}
