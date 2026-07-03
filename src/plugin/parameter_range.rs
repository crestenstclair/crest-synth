// path: src/plugin/parameter_range.rs

//! Value range and default for a host-visible plugin parameter.
//!
//! A `ParameterRange` describes the inclusive `[min, max]` bounds a
//! host-exposed parameter may take, its default value, and an optional
//! step size for quantized parameters. Constructing one is always
//! validated: a range with `min >= max` or a `defaultValue` outside
//! `[min, max]` is rejected rather than silently accepted.

use std::error::Error;
use std::fmt;

/// Error returned when constructing a [`ParameterRange`] with invalid
/// bounds or an out-of-range default value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParameterRangeError {
    /// `min` was not strictly less than `max`.
    MinNotLessThanMax { min: f64, max: f64 },
    /// `defaultValue` fell outside the inclusive `[min, max]` interval.
    DefaultOutOfRange { default_value: f64, min: f64, max: f64 },
}

impl fmt::Display for ParameterRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterRangeError::MinNotLessThanMax { min, max } => write!(
                f,
                "ParameterRange requires min < max, got min={min}, max={max}"
            ),
            ParameterRangeError::DefaultOutOfRange {
                default_value,
                min,
                max,
            } => write!(
                f,
                "ParameterRange defaultValue {default_value} is outside [{min}, {max}]"
            ),
        }
    }
}

impl Error for ParameterRangeError {}

/// Value range and default for a host-visible parameter.
///
/// Invariants (enforced by [`ParameterRange::try_new`]):
/// - `min < max`
/// - `default_value` is within `[min, max]`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterRange {
    default_value: f64,
    max: f64,
    min: f64,
    step: Option<f64>,
}

impl ParameterRange {
    /// Attempts to construct a `ParameterRange`, validating that
    /// `min < max` and that `default_value` lies within `[min, max]`.
    ///
    /// Returns `Err(ParameterRangeError)` when either invariant is
    /// violated rather than panicking or silently clamping.
    pub fn try_new(
        default_value: f64,
        min: f64,
        max: f64,
        step: Option<f64>,
    ) -> Result<Self, ParameterRangeError> {
        if !(min < max) {
            return Err(ParameterRangeError::MinNotLessThanMax { min, max });
        }
        if !(min..=max).contains(&default_value) {
            return Err(ParameterRangeError::DefaultOutOfRange {
                default_value,
                min,
                max,
            });
        }
        Ok(Self {
            default_value,
            max,
            min,
            step,
        })
    }

    /// The default value of the parameter.
    pub fn default_value(&self) -> f64 {
        self.default_value
    }

    /// The inclusive minimum of the parameter's range.
    pub fn min(&self) -> f64 {
        self.min
    }

    /// The inclusive maximum of the parameter's range.
    pub fn max(&self) -> f64 {
        self.max
    }

    /// The optional quantization step for the parameter, if any.
    pub fn step(&self) -> Option<f64> {
        self.step
    }

    /// Returns `true` if `value` lies within `[min, max]`.
    pub fn contains(&self, value: f64) -> bool {
        (self.min..=self.max).contains(&value)
    }
}

#[cfg(test)]
mod parameter_range_tests {
    use super::*;

    #[test]
    fn parameter_range_accepts_valid_bounds_and_default() {
        let range = ParameterRange::try_new(0.5, 0.0, 1.0, None)
            .expect("valid range should construct");
        assert_eq!(range.default_value(), 0.5);
        assert_eq!(range.min(), 0.0);
        assert_eq!(range.max(), 1.0);
        assert_eq!(range.step(), None);
    }

    #[test]
    fn parameter_range_accepts_default_equal_to_bounds() {
        assert!(ParameterRange::try_new(0.0, 0.0, 1.0, None).is_ok());
        assert!(ParameterRange::try_new(1.0, 0.0, 1.0, None).is_ok());
    }

    #[test]
    fn parameter_range_rejects_min_equal_to_max() {
        let result = ParameterRange::try_new(1.0, 1.0, 1.0, None);
        assert_eq!(
            result,
            Err(ParameterRangeError::MinNotLessThanMax { min: 1.0, max: 1.0 })
        );
    }

    #[test]
    fn parameter_range_rejects_min_greater_than_max() {
        let result = ParameterRange::try_new(0.5, 2.0, 1.0, None);
        assert_eq!(
            result,
            Err(ParameterRangeError::MinNotLessThanMax { min: 2.0, max: 1.0 })
        );
    }

    #[test]
    fn parameter_range_rejects_default_below_min() {
        let result = ParameterRange::try_new(-1.0, 0.0, 1.0, None);
        assert_eq!(
            result,
            Err(ParameterRangeError::DefaultOutOfRange {
                default_value: -1.0,
                min: 0.0,
                max: 1.0
            })
        );
    }

    #[test]
    fn parameter_range_rejects_default_above_max() {
        let result = ParameterRange::try_new(2.0, 0.0, 1.0, None);
        assert_eq!(
            result,
            Err(ParameterRangeError::DefaultOutOfRange {
                default_value: 2.0,
                min: 0.0,
                max: 1.0
            })
        );
    }

    #[test]
    fn parameter_range_preserves_step_when_provided() {
        let range = ParameterRange::try_new(0.0, -1.0, 1.0, Some(0.1))
            .expect("valid range should construct");
        assert_eq!(range.step(), Some(0.1));
    }

    #[test]
    fn parameter_range_contains_reports_membership() {
        let range = ParameterRange::try_new(0.0, -1.0, 1.0, None)
            .expect("valid range should construct");
        assert!(range.contains(-1.0));
        assert!(range.contains(1.0));
        assert!(range.contains(0.0));
        assert!(!range.contains(-1.5));
        assert!(!range.contains(1.5));
    }
}
