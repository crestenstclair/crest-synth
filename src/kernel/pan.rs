// path: src/kernel/pan.rs

//! Stereo position value object.
//!
//! `Pan` represents the stereo placement of a signal, ranging from -1.0
//! (hard left) to 1.0 (hard right), with 0.0 as center.

use std::fmt;

/// Stereo position, constrained to the inclusive range `-1.0..=1.0`.
///
/// - `-1.0` is hard left.
/// - `0.0` is center.
/// - `1.0` is hard right.
///
/// `Pan` is a validated newtype over `f64`. Values are only ever constructed
/// through [`Pan::try_new`], so any `Pan` in scope is guaranteed to lie
/// within the closed range `[-1.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Pan(f64);

/// Error returned when constructing a `Pan` from an out-of-range or
/// non-finite value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanRangeError(f64);

impl fmt::Display for PanRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pan value {} is out of range: must be -1.0 (hard left) to 1.0 (hard right)",
            self.0
        )
    }
}

impl std::error::Error for PanRangeError {}

impl Pan {
    /// Hard left.
    pub const LEFT: Pan = Pan(-1.0);
    /// Center.
    pub const CENTER: Pan = Pan(0.0);
    /// Hard right.
    pub const RIGHT: Pan = Pan(1.0);

    /// Attempts to construct a `Pan` from a raw `f64`.
    ///
    /// Returns `Err(PanRangeError)` if `value` is `NaN` or falls outside
    /// the inclusive range `-1.0..=1.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::kernel::pan::Pan;
    ///
    /// let pan = Pan::try_new(-0.5).unwrap();
    /// assert_eq!(pan.value(), -0.5);
    /// assert!(Pan::try_new(2.0).is_err());
    /// ```
    pub fn try_new(value: f64) -> Result<Self, PanRangeError> {
        if value.is_nan() || !(-1.0..=1.0).contains(&value) {
            return Err(PanRangeError(value));
        }
        Ok(Pan(value))
    }

    /// Returns the underlying `f64` value.
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for Pan {
    fn default() -> Self {
        Pan::CENTER
    }
}

impl fmt::Display for Pan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

impl TryFrom<f64> for Pan {
    type Error = PanRangeError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Pan::try_new(value)
    }
}

impl From<Pan> for f64 {
    fn from(pan: Pan) -> Self {
        pan.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_hard_left() {
        let pan = Pan::try_new(-1.0).expect("hard left is valid");
        assert_eq!(pan.value(), -1.0);
    }

    #[test]
    fn accepts_hard_right() {
        let pan = Pan::try_new(1.0).expect("hard right is valid");
        assert_eq!(pan.value(), 1.0);
    }

    #[test]
    fn accepts_center() {
        let pan = Pan::try_new(0.0).expect("center is valid");
        assert_eq!(pan.value(), 0.0);
    }

    #[test]
    fn rejects_below_hard_left() {
        assert!(Pan::try_new(-1.0001).is_err());
    }

    #[test]
    fn rejects_above_hard_right() {
        assert!(Pan::try_new(1.0001).is_err());
    }

    #[test]
    fn rejects_nan() {
        assert!(Pan::try_new(f64::NAN).is_err());
    }

    #[test]
    fn default_is_center() {
        assert_eq!(Pan::default(), Pan::CENTER);
    }

    #[test]
    fn display_formats_value() {
        let pan = Pan::try_new(0.5).expect("valid pan");
        assert_eq!(format!("{}", pan), "0.500");
    }

    #[test]
    fn error_message_mentions_range() {
        let err = Pan::try_new(2.0).expect_err("out of range");
        let message = format!("{}", err);
        assert!(message.contains("-1.0"));
        assert!(message.contains("1.0"));
    }

    #[test]
    fn try_from_matches_try_new() {
        let via_try_from: Pan = 0.25_f64.try_into().unwrap();
        assert_eq!(via_try_from, Pan::try_new(0.25).unwrap());
    }

    #[test]
    fn converts_into_f64() {
        let pan = Pan::try_new(0.5).unwrap();
        let raw: f64 = pan.into();
        assert_eq!(raw, 0.5);
    }

    #[test]
    fn constants_are_in_range() {
        assert_eq!(Pan::LEFT.value(), -1.0);
        assert_eq!(Pan::CENTER.value(), 0.0);
        assert_eq!(Pan::RIGHT.value(), 1.0);
    }
}
