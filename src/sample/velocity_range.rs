use crate::kernel::velocity::Velocity;

/// Inclusive velocity range a zone responds to.
///
/// `VelocityRange` pairs a lower and upper `Velocity` bound and enforces
/// the invariant that `low <= high`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityRange {
    low: Velocity,
    high: Velocity,
}

/// Error returned when constructing a `VelocityRange` with `low > high`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityRangeError {
    low: Velocity,
    high: Velocity,
}

impl std::fmt::Display for VelocityRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VelocityRange low {:?} must be <= high {:?}",
            self.low, self.high
        )
    }
}

impl std::error::Error for VelocityRangeError {}

impl VelocityRange {
    /// Construct a `VelocityRange` from a low and high `Velocity`.
    ///
    /// Returns `Err` if `low > high`.
    ///
    /// ```
    /// use crest_synth::kernel::velocity::Velocity;
    /// use crest_synth::sample::velocity_range::VelocityRange;
    ///
    /// let low = Velocity::try_new(0.2).unwrap();
    /// let high = Velocity::try_new(0.8).unwrap();
    /// assert!(VelocityRange::try_new(low, high).is_ok());
    /// assert!(VelocityRange::try_new(high, low).is_err());
    /// ```
    pub fn try_new(low: Velocity, high: Velocity) -> Result<Self, VelocityRangeError> {
        if low.value() > high.value() {
            return Err(VelocityRangeError { low, high });
        }
        Ok(Self { low, high })
    }

    /// Return the lower bound of the range.
    #[inline]
    pub fn low(self) -> Velocity {
        self.low
    }

    /// Return the upper bound of the range.
    #[inline]
    pub fn high(self) -> Velocity {
        self.high
    }

    /// Returns `true` if `velocity` falls within this inclusive range.
    #[inline]
    pub fn contains(self, velocity: Velocity) -> bool {
        (self.low.value()..=self.high.value()).contains(&velocity.value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(value: f64) -> Velocity {
        Velocity::try_new(value).unwrap()
    }

    #[test]
    fn low_less_than_high_is_valid() {
        assert!(VelocityRange::try_new(v(0.1), v(0.9)).is_ok());
    }

    #[test]
    fn equal_bounds_are_valid() {
        let range = VelocityRange::try_new(v(0.5), v(0.5)).unwrap();
        assert!((range.low().value() - 0.5).abs() < f64::EPSILON);
        assert!((range.high().value() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn low_greater_than_high_is_rejected() {
        assert!(VelocityRange::try_new(v(0.9), v(0.1)).is_err());
    }

    #[test]
    fn accessors_return_constructed_bounds() {
        let range = VelocityRange::try_new(v(0.2), v(0.8)).unwrap();
        assert!((range.low().value() - 0.2).abs() < f64::EPSILON);
        assert!((range.high().value() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn contains_within_bounds_inclusive() {
        let range = VelocityRange::try_new(v(0.2), v(0.8)).unwrap();
        assert!(range.contains(v(0.2)));
        assert!(range.contains(v(0.8)));
        assert!(range.contains(v(0.5)));
    }

    #[test]
    fn contains_outside_bounds_is_false() {
        let range = VelocityRange::try_new(v(0.2), v(0.8)).unwrap();
        assert!(!range.contains(v(0.1)));
        assert!(!range.contains(v(0.9)));
    }

    #[test]
    fn copy_semantics() {
        let a = VelocityRange::try_new(v(0.3), v(0.6)).unwrap();
        let b = a;
        assert_eq!(a.low(), b.low());
        assert_eq!(a.high(), b.high());
    }
}
