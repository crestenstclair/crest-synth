// path: src/plugin/parameter_range.rs

/// Value range and default for a host-visible parameter.
///
/// # Invariants
///
/// - `min < max`
/// - `default_value` is within `[min, max]`
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterRange {
    /// Minimum allowed value.
    pub min: f64,
    /// Maximum allowed value.
    pub max: f64,
    /// Default value (must lie within `[min, max]`).
    pub default_value: f64,
    /// Optional discrete step size. `None` means continuous.
    pub step: Option<f64>,
}

/// Error returned when `ParameterRange` construction fails an invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterRangeError {
    /// `min` is not strictly less than `max`.
    InvalidBounds,
    /// `default_value` is outside `[min, max]`.
    DefaultOutOfRange,
}

impl std::fmt::Display for ParameterRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParameterRangeError::InvalidBounds => {
                write!(f, "min must be strictly less than max")
            }
            ParameterRangeError::DefaultOutOfRange => {
                write!(f, "default_value must be within [min, max]")
            }
        }
    }
}

impl std::error::Error for ParameterRangeError {}

impl ParameterRange {
    /// Construct a new [`ParameterRange`], validating all invariants.
    ///
    /// Returns [`Err`] if `min >= max` or if `default_value` is outside
    /// `[min, max]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::plugin::parameter_range::ParameterRange;
    ///
    /// let range = ParameterRange::try_new(0.0, 1.0, 0.5, None).unwrap();
    /// assert_eq!(range.min, 0.0);
    /// assert_eq!(range.max, 1.0);
    /// assert_eq!(range.default_value, 0.5);
    /// assert_eq!(range.step, None);
    /// ```
    pub fn try_new(
        min: f64,
        max: f64,
        default_value: f64,
        step: Option<f64>,
    ) -> Result<Self, ParameterRangeError> {
        if min >= max {
            return Err(ParameterRangeError::InvalidBounds);
        }
        if default_value.is_nan() || !(min..=max).contains(&default_value) {
            return Err(ParameterRangeError::DefaultOutOfRange);
        }
        Ok(Self {
            min,
            max,
            default_value,
            step,
        })
    }

    /// Returns `true` if `value` lies within `[min, max]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::plugin::parameter_range::ParameterRange;
    ///
    /// let range = ParameterRange::try_new(0.0, 1.0, 0.5, None).unwrap();
    /// assert!(range.contains(0.5));
    /// assert!(!range.contains(1.5));
    /// ```
    #[inline]
    pub fn contains(&self, value: f64) -> bool {
        !value.is_nan() && (self.min..=self.max).contains(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_range_accepted() {
        let r = ParameterRange::try_new(0.0, 1.0, 0.5, None).unwrap();
        assert_eq!(r.min, 0.0);
        assert_eq!(r.max, 1.0);
        assert_eq!(r.default_value, 0.5);
        assert_eq!(r.step, None);
    }

    #[test]
    fn min_equal_to_max_rejected() {
        let err = ParameterRange::try_new(1.0, 1.0, 1.0, None).unwrap_err();
        assert_eq!(err, ParameterRangeError::InvalidBounds);
    }

    #[test]
    fn min_greater_than_max_rejected() {
        let err = ParameterRange::try_new(2.0, 1.0, 1.5, None).unwrap_err();
        assert_eq!(err, ParameterRangeError::InvalidBounds);
    }

    #[test]
    fn default_below_min_rejected() {
        let err = ParameterRange::try_new(0.0, 1.0, -0.1, None).unwrap_err();
        assert_eq!(err, ParameterRangeError::DefaultOutOfRange);
    }

    #[test]
    fn default_above_max_rejected() {
        let err = ParameterRange::try_new(0.0, 1.0, 1.1, None).unwrap_err();
        assert_eq!(err, ParameterRangeError::DefaultOutOfRange);
    }

    #[test]
    fn default_at_min_accepted() {
        let r = ParameterRange::try_new(0.0, 1.0, 0.0, None).unwrap();
        assert_eq!(r.default_value, 0.0);
    }

    #[test]
    fn default_at_max_accepted() {
        let r = ParameterRange::try_new(0.0, 1.0, 1.0, None).unwrap();
        assert_eq!(r.default_value, 1.0);
    }

    #[test]
    fn default_nan_rejected() {
        let err = ParameterRange::try_new(0.0, 1.0, f64::NAN, None).unwrap_err();
        assert_eq!(err, ParameterRangeError::DefaultOutOfRange);
    }

    #[test]
    fn step_stored() {
        let r = ParameterRange::try_new(0.0, 10.0, 5.0, Some(1.0)).unwrap();
        assert_eq!(r.step, Some(1.0));
    }

    #[test]
    fn contains_in_range() {
        let r = ParameterRange::try_new(0.0, 1.0, 0.5, None).unwrap();
        assert!(r.contains(0.0));
        assert!(r.contains(0.5));
        assert!(r.contains(1.0));
    }

    #[test]
    fn contains_out_of_range() {
        let r = ParameterRange::try_new(0.0, 1.0, 0.5, None).unwrap();
        assert!(!r.contains(-0.1));
        assert!(!r.contains(1.1));
    }

    #[test]
    fn contains_nan_returns_false() {
        let r = ParameterRange::try_new(0.0, 1.0, 0.5, None).unwrap();
        assert!(!r.contains(f64::NAN));
    }

    #[test]
    fn negative_range_valid() {
        let r = ParameterRange::try_new(-100.0, -10.0, -50.0, None).unwrap();
        assert_eq!(r.min, -100.0);
        assert_eq!(r.max, -10.0);
    }

    #[test]
    fn display_error_messages() {
        assert!(!ParameterRangeError::InvalidBounds.to_string().is_empty());
        assert!(!ParameterRangeError::DefaultOutOfRange
            .to_string()
            .is_empty());
    }
}
