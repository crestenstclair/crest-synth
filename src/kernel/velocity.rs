/// Normalized note velocity (0.0–1.0).
///
/// `Velocity` wraps a `f64` and enforces the invariant that the value
/// is within the range 0.0 to 1.0 inclusive (not NaN).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Velocity(f64);

/// Error returned when a `Velocity` value is out of range or NaN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityError(f64);

impl std::fmt::Display for VelocityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Velocity value {} is out of range 0.0-1.0", self.0)
    }
}

impl std::error::Error for VelocityError {}

impl Velocity {
    /// Construct a `Velocity` from a raw `f64`.
    ///
    /// Returns `Err` if the value is NaN or outside 0.0–1.0.
    ///
    /// ```
    /// use crest_synth::kernel::velocity::Velocity;
    /// assert!(Velocity::try_new(0.0).is_ok());
    /// assert!(Velocity::try_new(1.0).is_ok());
    /// assert!(Velocity::try_new(1.1).is_err());
    /// assert!(Velocity::try_new(-0.1).is_err());
    /// ```
    pub fn try_new(value: f64) -> Result<Self, VelocityError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(VelocityError(value));
        }
        Ok(Self(value))
    }

    /// Return the underlying `f64` value.
    #[inline]
    pub fn value(self) -> f64 {
        self.0
    }
}

impl Default for Velocity {
    /// Returns silence velocity (0.0).
    fn default() -> Self {
        Self(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_valid() {
        assert!(Velocity::try_new(0.0).is_ok());
    }

    #[test]
    fn one_is_valid() {
        let v = Velocity::try_new(1.0).unwrap();
        assert!((v.value() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn above_one_is_rejected() {
        assert!(Velocity::try_new(1.001).is_err());
    }

    #[test]
    fn below_zero_is_rejected() {
        assert!(Velocity::try_new(-0.001).is_err());
    }

    #[test]
    fn nan_is_rejected() {
        assert!(Velocity::try_new(f64::NAN).is_err());
    }

    #[test]
    fn midpoint_is_valid() {
        let v = Velocity::try_new(0.5).unwrap();
        assert!((v.value() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn default_is_zero() {
        assert!((Velocity::default().value()).abs() < f64::EPSILON);
    }

    #[test]
    fn copy_semantics() {
        let a = Velocity::try_new(0.7).unwrap();
        let b = a;
        assert!((a.value() - b.value()).abs() < f64::EPSILON);
    }
}
