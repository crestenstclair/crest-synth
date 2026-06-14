/// Linear amplitude (0.0 = silence, 1.0 = unity).
///
/// `Amplitude` wraps a `f64` and enforces the invariant that the value
/// is non-negative (not NaN and not less than 0.0). Values above 1.0 are
/// permitted (e.g. for gain staging above unity).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Amplitude(f64);

/// Error returned when an `Amplitude` value is negative or NaN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmplitudeError(f64);

impl std::fmt::Display for AmplitudeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Amplitude value {} is negative or NaN", self.0)
    }
}

impl std::error::Error for AmplitudeError {}

impl Amplitude {
    /// Construct an `Amplitude` from a raw `f64`.
    ///
    /// Returns `Err` if the value is NaN or negative.
    ///
    /// ```
    /// use crest_synth::kernel::amplitude::Amplitude;
    /// assert!(Amplitude::try_new(0.0).is_ok());
    /// assert!(Amplitude::try_new(1.0).is_ok());
    /// assert!(Amplitude::try_new(2.5).is_ok());
    /// assert!(Amplitude::try_new(-0.1).is_err());
    /// ```
    pub fn try_new(value: f64) -> Result<Self, AmplitudeError> {
        if value.is_nan() || value < 0.0 {
            return Err(AmplitudeError(value));
        }
        Ok(Self(value))
    }

    /// Silence amplitude (0.0).
    pub fn silence() -> Self {
        Self(0.0)
    }

    /// Unity amplitude (1.0).
    pub fn unity() -> Self {
        Self(1.0)
    }

    /// Return the underlying `f64` value.
    #[inline]
    pub fn value(self) -> f64 {
        self.0
    }
}

impl Default for Amplitude {
    /// Returns silence (0.0).
    fn default() -> Self {
        Self(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_valid() {
        assert!(Amplitude::try_new(0.0).is_ok());
    }

    #[test]
    fn unity_is_valid() {
        let a = Amplitude::try_new(1.0).unwrap();
        assert!((a.value() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn above_unity_is_valid() {
        assert!(Amplitude::try_new(2.5).is_ok());
    }

    #[test]
    fn negative_is_rejected() {
        assert!(Amplitude::try_new(-0.001).is_err());
    }

    #[test]
    fn nan_is_rejected() {
        assert!(Amplitude::try_new(f64::NAN).is_err());
    }

    #[test]
    fn silence_constructor() {
        assert!((Amplitude::silence().value()).abs() < f64::EPSILON);
    }

    #[test]
    fn unity_constructor() {
        assert!((Amplitude::unity().value() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn default_is_silence() {
        assert!((Amplitude::default().value()).abs() < f64::EPSILON);
    }

    #[test]
    fn copy_semantics() {
        let a = Amplitude::try_new(0.5).unwrap();
        let b = a;
        assert!((a.value() - b.value()).abs() < f64::EPSILON);
    }

    #[test]
    fn error_message_contains_value() {
        let err = Amplitude::try_new(-1.0).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("-1"));
    }
}
