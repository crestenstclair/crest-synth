// path: src/kernel/decibel.rs

//! [`Decibel`] — a logarithmic level value.
//!
//! `0.0` dB is unity gain (no change in level). `f64::NEG_INFINITY` is
//! silence (zero linear amplitude). Positive values boost, negative values
//! attenuate.
//!
//! This is a pure value type: it holds no dependencies and performs no I/O,
//! allocation, or locking, so it is safe to construct, copy, and compare on
//! the real-time audio thread.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Sub};

/// A logarithmic level in decibels.
///
/// `Decibel(0.0)` is unity gain. `Decibel::SILENCE` (negative infinity) is
/// the representation of "no signal". NaN is never a valid value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decibel(f64);

impl Decibel {
    /// Unity gain: 0 dB, i.e. no change in level.
    pub const UNITY: Decibel = Decibel(0.0);

    /// Silence: negative infinity dB, i.e. zero linear amplitude.
    pub const SILENCE: Decibel = Decibel(f64::NEG_INFINITY);

    /// Construct a `Decibel` from a raw value.
    ///
    /// Accepts any finite value or negative infinity. Returns `None` for
    /// NaN and for positive infinity, neither of which is a meaningful
    /// level.
    ///
    /// ```
    /// use crest_synth::kernel::decibel::Decibel;
    ///
    /// assert!(Decibel::try_new(0.0).is_some());
    /// assert!(Decibel::try_new(f64::NEG_INFINITY).is_some());
    /// assert!(Decibel::try_new(f64::NAN).is_none());
    /// assert!(Decibel::try_new(f64::INFINITY).is_none());
    /// ```
    pub fn try_new(value: f64) -> Option<Decibel> {
        if value.is_nan() || value == f64::INFINITY {
            None
        } else {
            Some(Decibel(value))
        }
    }

    /// The raw decibel value.
    pub fn value(self) -> f64 {
        self.0
    }

    /// `true` if this level is silence (negative infinity).
    pub fn is_silent(self) -> bool {
        self.0 == f64::NEG_INFINITY
    }

    /// Convert a linear amplitude (voltage/sample ratio, not power) to a
    /// `Decibel`.
    ///
    /// Amplitudes at or below zero map to [`Decibel::SILENCE`] rather than
    /// producing NaN or an error, since a non-positive linear amplitude has
    /// no finite logarithm and always means "no signal" in practice.
    ///
    /// ```
    /// use crest_synth::kernel::decibel::Decibel;
    ///
    /// assert_eq!(Decibel::from_linear(1.0), Decibel::UNITY);
    /// assert!(Decibel::from_linear(0.0).is_silent());
    /// assert!(Decibel::from_linear(-1.0).is_silent());
    /// ```
    pub fn from_linear(amplitude: f64) -> Decibel {
        if amplitude <= 0.0 {
            Decibel::SILENCE
        } else {
            Decibel(20.0 * amplitude.log10())
        }
    }

    /// Convert to a linear amplitude (voltage/sample ratio, not power).
    ///
    /// [`Decibel::SILENCE`] converts to exactly `0.0`.
    ///
    /// ```
    /// use crest_synth::kernel::decibel::Decibel;
    ///
    /// assert_eq!(Decibel::UNITY.to_linear(), 1.0);
    /// assert_eq!(Decibel::SILENCE.to_linear(), 0.0);
    /// ```
    pub fn to_linear(self) -> f64 {
        if self.is_silent() {
            0.0
        } else {
            10.0f64.powf(self.0 / 20.0)
        }
    }

    /// Clamp this level to `[lo, hi]`.
    ///
    /// `lo` and `hi` are compared by their raw decibel value, so
    /// `Decibel::SILENCE` sorts below every finite level.
    pub fn clamp(self, lo: Decibel, hi: Decibel) -> Decibel {
        if self.0 < lo.0 {
            lo
        } else if self.0 > hi.0 {
            hi
        } else {
            self
        }
    }
}

impl Default for Decibel {
    /// The default level is unity gain (0 dB).
    fn default() -> Self {
        Decibel::UNITY
    }
}

impl fmt::Display for Decibel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_silent() {
            write!(f, "-inf dB")
        } else {
            write!(f, "{:.2} dB", self.0)
        }
    }
}

impl PartialOrd for Decibel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

/// Combine two levels by summing their decibel values.
///
/// This is the logarithmic equivalent of multiplying linear gains, which is
/// the standard way to cascade levels (e.g. a fader level plus a trim
/// level). Adding anything to [`Decibel::SILENCE`] stays silent.
impl Add for Decibel {
    type Output = Decibel;

    fn add(self, rhs: Decibel) -> Decibel {
        Decibel(self.0 + rhs.0)
    }
}

/// Subtract one level from another by subtracting their decibel values.
///
/// This is the logarithmic equivalent of dividing linear gains.
impl Sub for Decibel {
    type Output = Decibel;

    fn sub(self, rhs: Decibel) -> Decibel {
        Decibel(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_is_zero_db_and_linear_one() {
        assert_eq!(Decibel::UNITY.value(), 0.0);
        assert_eq!(Decibel::UNITY.to_linear(), 1.0);
    }

    #[test]
    fn silence_is_negative_infinity_and_linear_zero() {
        assert!(Decibel::SILENCE.is_silent());
        assert_eq!(Decibel::SILENCE.value(), f64::NEG_INFINITY);
        assert_eq!(Decibel::SILENCE.to_linear(), 0.0);
    }

    #[test]
    fn try_new_rejects_nan_and_positive_infinity() {
        assert!(Decibel::try_new(f64::NAN).is_none());
        assert!(Decibel::try_new(f64::INFINITY).is_none());
    }

    #[test]
    fn try_new_accepts_finite_and_negative_infinity() {
        assert!(Decibel::try_new(-6.0).is_some());
        assert!(Decibel::try_new(f64::NEG_INFINITY).is_some());
    }

    #[test]
    fn from_linear_round_trips_through_to_linear() {
        let level = Decibel::from_linear(0.5);
        let linear = level.to_linear();
        assert!((linear - 0.5).abs() < 1e-9);
    }

    #[test]
    fn from_linear_non_positive_is_silence() {
        assert!(Decibel::from_linear(0.0).is_silent());
        assert!(Decibel::from_linear(-2.0).is_silent());
    }

    #[test]
    fn default_is_unity() {
        assert_eq!(Decibel::default(), Decibel::UNITY);
    }

    #[test]
    fn addition_sums_decibel_values() {
        let a = Decibel::try_new(3.0).unwrap();
        let b = Decibel::try_new(4.0).unwrap();
        assert_eq!((a + b).value(), 7.0);
    }

    #[test]
    fn subtraction_differences_decibel_values() {
        let a = Decibel::try_new(10.0).unwrap();
        let b = Decibel::try_new(4.0).unwrap();
        assert_eq!((a - b).value(), 6.0);
    }

    #[test]
    fn adding_to_silence_stays_silent() {
        let a = Decibel::try_new(6.0).unwrap();
        assert!((Decibel::SILENCE + a).is_silent());
    }

    #[test]
    fn ordering_places_silence_below_finite_levels() {
        let a = Decibel::try_new(-100.0).unwrap();
        assert!(Decibel::SILENCE < a);
        assert!(a < Decibel::UNITY);
    }

    #[test]
    fn clamp_bounds_within_range() {
        let lo = Decibel::try_new(-12.0).unwrap();
        let hi = Decibel::try_new(0.0).unwrap();
        let too_low = Decibel::try_new(-20.0).unwrap();
        let too_high = Decibel::try_new(6.0).unwrap();
        let in_range = Decibel::try_new(-6.0).unwrap();

        assert_eq!(too_low.clamp(lo, hi), lo);
        assert_eq!(too_high.clamp(lo, hi), hi);
        assert_eq!(in_range.clamp(lo, hi), in_range);
    }

    #[test]
    fn display_formats_finite_and_silent_levels() {
        let level = Decibel::try_new(-3.5).unwrap();
        assert_eq!(format!("{}", level), "-3.50 dB");
        assert_eq!(format!("{}", Decibel::SILENCE), "-inf dB");
    }
}
