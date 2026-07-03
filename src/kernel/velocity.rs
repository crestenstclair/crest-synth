// path: src/kernel/velocity.rs

//! Note velocity, upconverted from MIDI 1.0's 7-bit resolution (0-127) to a
//! normalized high-resolution `f64` in `0.0..=1.0`.
//!
//! `Velocity` is a value object: it is immutable once constructed and every
//! instance in existence satisfies its invariant. There is no way to obtain
//! a `Velocity` outside of `0.0..=1.0`.

use std::fmt;

/// The error returned when a raw value fails the `Velocity` invariant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityError {
    value: f64,
}

impl fmt::Display for VelocityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "velocity {} out of range: must be within 0.0..=1.0",
            self.value
        )
    }
}

impl std::error::Error for VelocityError {}

/// Note velocity, normalized to `0.0..=1.0`.
///
/// MIDI 1.0 conveys velocity as a 7-bit integer (0-127). `Velocity`
/// represents the upconverted, high-resolution form used internally so the
/// kernel is not coupled to any single wire protocol's resolution.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Velocity(f64);

impl Velocity {
    /// The minimum valid velocity (silent / note-off velocity floor).
    pub const MIN: Velocity = Velocity(0.0);

    /// The maximum valid velocity (full intensity).
    pub const MAX: Velocity = Velocity(1.0);

    /// Constructs a `Velocity` from a normalized `f64`.
    ///
    /// # Errors
    ///
    /// Returns [`VelocityError`] if `value` is `NaN` or outside `0.0..=1.0`.
    ///
    /// ```
    /// use crest_synth::kernel::velocity::Velocity;
    ///
    /// let v = Velocity::try_new(0.75).unwrap();
    /// assert!((v.value() - 0.75).abs() < f64::EPSILON);
    ///
    /// assert!(Velocity::try_new(1.5).is_err());
    /// assert!(Velocity::try_new(f64::NAN).is_err());
    /// ```
    pub fn try_new(value: f64) -> Result<Self, VelocityError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(VelocityError { value });
        }
        Ok(Velocity(value))
    }

    /// Constructs a `Velocity` from a MIDI 1.0 7-bit velocity value
    /// (`0..=127`), upconverting it to the normalized high-resolution range.
    ///
    /// Values greater than `127` are clamped to `127` (MIDI 1.0 only uses
    /// the low 7 bits; a caller passing a raw byte with the high bit set has
    /// already violated the wire format, and clamping keeps this
    /// constructor infallible).
    ///
    /// ```
    /// use crest_synth::kernel::velocity::Velocity;
    ///
    /// let v = Velocity::from_midi7(127);
    /// assert_eq!(v, Velocity::MAX);
    ///
    /// let silent = Velocity::from_midi7(0);
    /// assert_eq!(silent, Velocity::MIN);
    /// ```
    pub fn from_midi7(raw: u8) -> Self {
        let clamped = raw.min(127);
        Velocity(f64::from(clamped) / 127.0)
    }

    /// Returns the underlying normalized `f64` value.
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Downconverts back to a MIDI 1.0 7-bit velocity (`0..=127`), rounding
    /// to the nearest integer.
    ///
    /// ```
    /// use crest_synth::kernel::velocity::Velocity;
    ///
    /// let v = Velocity::try_new(1.0).unwrap();
    /// assert_eq!(v.to_midi7(), 127);
    /// ```
    pub fn to_midi7(&self) -> u8 {
        (self.0 * 127.0).round() as u8
    }
}

impl Default for Velocity {
    /// The default velocity is silence (`0.0`), matching `Velocity::MIN`.
    fn default() -> Self {
        Velocity::MIN
    }
}

impl TryFrom<f64> for Velocity {
    type Error = VelocityError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Velocity::try_new(value)
    }
}

impl From<Velocity> for f64 {
    fn from(velocity: Velocity) -> Self {
        velocity.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_boundaries() {
        assert!(Velocity::try_new(0.0).is_ok());
        assert!(Velocity::try_new(1.0).is_ok());
    }

    #[test]
    fn accepts_mid_range_value() {
        let v = Velocity::try_new(0.42).unwrap();
        assert!((v.value() - 0.42).abs() < f64::EPSILON);
    }

    #[test]
    fn rejects_below_min() {
        assert!(Velocity::try_new(-0.0001).is_err());
    }

    #[test]
    fn rejects_above_max() {
        assert!(Velocity::try_new(1.0001).is_err());
    }

    #[test]
    fn rejects_nan() {
        assert!(Velocity::try_new(f64::NAN).is_err());
    }

    #[test]
    fn error_message_includes_offending_value() {
        let err = Velocity::try_new(2.0).unwrap_err();
        let message = err.to_string();
        assert!(message.contains('2'));
    }

    #[test]
    fn default_is_min() {
        assert_eq!(Velocity::default(), Velocity::MIN);
    }

    #[test]
    fn from_midi7_zero_is_min() {
        assert_eq!(Velocity::from_midi7(0), Velocity::MIN);
    }

    #[test]
    fn from_midi7_max_is_max() {
        assert_eq!(Velocity::from_midi7(127), Velocity::MAX);
    }

    #[test]
    fn from_midi7_clamps_out_of_range_input() {
        assert_eq!(Velocity::from_midi7(200), Velocity::MAX);
    }

    #[test]
    fn round_trip_through_midi7_is_stable_at_boundaries() {
        assert_eq!(Velocity::from_midi7(0).to_midi7(), 0);
        assert_eq!(Velocity::from_midi7(127).to_midi7(), 127);
    }

    #[test]
    fn try_from_matches_try_new() {
        let a = Velocity::try_from(0.5).unwrap();
        let b = Velocity::try_new(0.5).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn into_f64_returns_underlying_value() {
        let v = Velocity::try_new(0.6).unwrap();
        let raw: f64 = v.into();
        assert!((raw - 0.6).abs() < f64::EPSILON);
    }
}
