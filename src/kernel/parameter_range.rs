// path: src/kernel/parameter_range.rs

//! The inclusive `[min, max]` range a plugin parameter's value may take.
//!
//! Unlike `VelocityRange` (which bounds an already-normalized `Velocity`),
//! a `ParameterRange` bounds an arbitrary host-facing parameter — e.g. a
//! filter cutoff in Hz or a mix in percent — so it is expressed directly in
//! `f64` rather than over a domain newtype.

/// Inclusive `[min, max]` bounds for a plugin parameter's value.
///
/// `ParameterRange` enforces `min <= max` and rejects `NaN` bounds at
/// construction, so every live `ParameterRange` is a well-formed interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterRange {
    min: f64,
    max: f64,
}

/// Error returned when constructing a `ParameterRange` with invalid bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterRangeError {
    min: f64,
    max: f64,
}

impl std::fmt::Display for ParameterRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ParameterRange bounds [{}, {}] must be non-NaN with min <= max",
            self.min, self.max
        )
    }
}

impl std::error::Error for ParameterRangeError {}

impl ParameterRange {
    /// Construct a `ParameterRange` from a `min` and `max` bound.
    ///
    /// Returns `Err` if either bound is `NaN` or if `min > max`.
    ///
    /// ```
    /// use crest_synth::kernel::parameter_range::ParameterRange;
    ///
    /// assert!(ParameterRange::try_new(20.0, 20_000.0).is_ok());
    /// assert!(ParameterRange::try_new(20_000.0, 20.0).is_err());
    /// ```
    pub fn try_new(min: f64, max: f64) -> Result<Self, ParameterRangeError> {
        if min.is_nan() || max.is_nan() || min > max {
            return Err(ParameterRangeError { min, max });
        }
        Ok(Self { min, max })
    }

    /// Returns the lower bound of the range.
    #[inline]
    pub fn min(self) -> f64 {
        self.min
    }

    /// Returns the upper bound of the range.
    #[inline]
    pub fn max(self) -> f64 {
        self.max
    }

    /// Returns `true` if `value` falls within this inclusive range.
    #[inline]
    pub fn contains(self, value: f64) -> bool {
        !value.is_nan() && (self.min..=self.max).contains(&value)
    }

    /// Clamps `value` into this inclusive range.
    #[inline]
    pub fn clamp(self, value: f64) -> f64 {
        value.clamp(self.min, self.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_less_than_max_is_valid() {
        assert!(ParameterRange::try_new(0.0, 1.0).is_ok());
    }

    #[test]
    fn equal_bounds_are_valid() {
        let range = ParameterRange::try_new(0.5, 0.5).unwrap();
        assert!((range.min() - 0.5).abs() < f64::EPSILON);
        assert!((range.max() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn min_greater_than_max_is_rejected() {
        assert!(ParameterRange::try_new(1.0, 0.0).is_err());
    }

    #[test]
    fn nan_bounds_are_rejected() {
        assert!(ParameterRange::try_new(f64::NAN, 1.0).is_err());
        assert!(ParameterRange::try_new(0.0, f64::NAN).is_err());
    }

    #[test]
    fn accessors_return_constructed_bounds() {
        let range = ParameterRange::try_new(20.0, 20_000.0).unwrap();
        assert!((range.min() - 20.0).abs() < f64::EPSILON);
        assert!((range.max() - 20_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn contains_within_bounds_inclusive() {
        let range = ParameterRange::try_new(0.0, 1.0).unwrap();
        assert!(range.contains(0.0));
        assert!(range.contains(1.0));
        assert!(range.contains(0.5));
    }

    #[test]
    fn contains_outside_bounds_is_false() {
        let range = ParameterRange::try_new(0.0, 1.0).unwrap();
        assert!(!range.contains(-0.1));
        assert!(!range.contains(1.1));
        assert!(!range.contains(f64::NAN));
    }

    #[test]
    fn clamp_pulls_values_inside_the_range() {
        let range = ParameterRange::try_new(0.0, 1.0).unwrap();
        assert!((range.clamp(-5.0) - 0.0).abs() < f64::EPSILON);
        assert!((range.clamp(5.0) - 1.0).abs() < f64::EPSILON);
        assert!((range.clamp(0.5) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn copy_semantics() {
        let a = ParameterRange::try_new(0.2, 0.8).unwrap();
        let b = a;
        assert_eq!(a.min(), b.min());
        assert_eq!(a.max(), b.max());
    }
}
