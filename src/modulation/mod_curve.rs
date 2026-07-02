// path: src/modulation/mod_curve.rs

//! Response curve applied to a modulation route: maps a normalized input
//! in `[0.0, 1.0]` to a normalized output in `[0.0, 1.0]`.
//!
//! `ModCurve` is a pure value type — evaluating it does no heap allocation,
//! locking, or I/O, so it is safe to call from the audio thread's inner
//! loop.

use std::f32::consts::E;

/// Number of discrete steps for a `Stepped` curve. Must be at least
/// `StepCount::MIN` (fewer steps cannot represent a curve).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepCount(u32);

impl StepCount {
    pub const MIN: u32 = 2;

    /// Constructs a `StepCount`, clamping any value below the minimum.
    pub fn new(steps: u32) -> Self {
        Self(steps.max(Self::MIN))
    }

    pub fn get(&self) -> u32 {
        self.0
    }
}

impl Default for StepCount {
    fn default() -> Self {
        Self::new(Self::MIN)
    }
}

/// Exponent for an `Exponential` curve. Constrained to a finite, positive
/// value so evaluation never produces NaN or infinity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveExponent(f32);

impl CurveExponent {
    pub const MIN: f32 = 0.01;
    pub const MAX: f32 = 100.0;

    /// Constructs a `CurveExponent`, clamping to the valid range and
    /// substituting the default if the input is NaN.
    pub fn new(exponent: f32) -> Self {
        if exponent.is_nan() {
            return Self::default();
        }
        Self(exponent.clamp(Self::MIN, Self::MAX))
    }

    pub fn get(&self) -> f32 {
        self.0
    }
}

impl Default for CurveExponent {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Steepness for an `SCurve`. Higher values sharpen the transition around
/// the curve's midpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveSteepness(f32);

impl CurveSteepness {
    pub const MIN: f32 = 0.1;
    pub const MAX: f32 = 20.0;

    /// Constructs a `CurveSteepness`, clamping to the valid range and
    /// substituting the default if the input is NaN.
    pub fn new(steepness: f32) -> Self {
        if steepness.is_nan() {
            return Self::default();
        }
        Self(steepness.clamp(Self::MIN, Self::MAX))
    }

    pub fn get(&self) -> f32 {
        self.0
    }
}

impl Default for CurveSteepness {
    fn default() -> Self {
        Self(6.0)
    }
}

/// The response curve applied to a modulation route: shapes how a source's
/// normalized value maps onto the amount actually applied to a destination.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModCurve {
    /// Identity mapping: output equals input.
    Linear,
    /// Power curve: `input.powf(exponent)`. An exponent of `1.0` is
    /// equivalent to `Linear`; values above `1.0` bias toward the low end,
    /// values below `1.0` bias toward the high end.
    Exponential(CurveExponent),
    /// Logistic S-curve centered at the curve's midpoint, renormalized so
    /// the endpoints still map to `0.0` and `1.0`.
    SCurve(CurveSteepness),
    /// Quantizes the input into `StepCount` equal bands, each mapped to a
    /// discrete output level.
    Stepped(StepCount),
}

impl ModCurve {
    /// Evaluates the curve at `input`, which is clamped to `[0.0, 1.0]`
    /// before shaping (NaN is treated as `0.0`). The result is always
    /// finite and within `[0.0, 1.0]`. Pure computation with no
    /// allocation, locking, or I/O — safe to call from the audio thread's
    /// inner loop.
    pub fn apply(&self, input: f32) -> f32 {
        let x = if input.is_nan() {
            0.0
        } else {
            input.clamp(0.0, 1.0)
        };

        match self {
            ModCurve::Linear => x,
            ModCurve::Exponential(exponent) => x.powf(exponent.get()),
            ModCurve::SCurve(steepness) => Self::logistic(x, steepness.get()),
            ModCurve::Stepped(steps) => Self::quantize(x, steps.get()),
        }
    }

    /// Logistic function renormalized so `f(0.0) == 0.0` and `f(1.0) ==
    /// 1.0`.
    fn logistic(x: f32, k: f32) -> f32 {
        let raw = |v: f32| 1.0 / (1.0 + E.powf(-k * (v - 0.5)));
        let (lo, hi) = (raw(0.0), raw(1.0));
        let span = hi - lo;
        if span.abs() < f32::EPSILON {
            x
        } else {
            (raw(x) - lo) / span
        }
    }

    fn quantize(x: f32, steps: u32) -> f32 {
        let steps_f = steps as f32;
        let band = (x * steps_f).floor().min(steps_f - 1.0);
        band / (steps_f - 1.0)
    }
}

impl Default for ModCurve {
    fn default() -> Self {
        ModCurve::Linear
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_is_identity() {
        let curve = ModCurve::Linear;
        assert_eq!(curve.apply(0.0), 0.0);
        assert_eq!(curve.apply(0.5), 0.5);
        assert_eq!(curve.apply(1.0), 1.0);
    }

    #[test]
    fn linear_clamps_out_of_range_input() {
        let curve = ModCurve::Linear;
        assert_eq!(curve.apply(-1.0), 0.0);
        assert_eq!(curve.apply(2.0), 1.0);
    }

    #[test]
    fn linear_treats_nan_as_zero() {
        let curve = ModCurve::Linear;
        assert_eq!(curve.apply(f32::NAN), 0.0);
    }

    #[test]
    fn exponential_endpoints_are_fixed() {
        let curve = ModCurve::Exponential(CurveExponent::new(2.0));
        assert_eq!(curve.apply(0.0), 0.0);
        assert!((curve.apply(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn exponential_with_exponent_two_squares_input() {
        let curve = ModCurve::Exponential(CurveExponent::new(2.0));
        assert!((curve.apply(0.5) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn exponent_clamps_to_valid_range() {
        let high = CurveExponent::new(1000.0);
        assert_eq!(high.get(), CurveExponent::MAX);
        let low = CurveExponent::new(-5.0);
        assert_eq!(low.get(), CurveExponent::MIN);
    }

    #[test]
    fn exponent_nan_falls_back_to_default() {
        let exponent = CurveExponent::new(f32::NAN);
        assert_eq!(exponent.get(), CurveExponent::default().get());
    }

    #[test]
    fn scurve_endpoints_are_fixed() {
        let curve = ModCurve::SCurve(CurveSteepness::new(6.0));
        assert!(curve.apply(0.0).abs() < 1e-6);
        assert!((curve.apply(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn scurve_midpoint_is_half() {
        let curve = ModCurve::SCurve(CurveSteepness::new(6.0));
        assert!((curve.apply(0.5) - 0.5).abs() < 1e-3);
    }

    #[test]
    fn scurve_is_monotonic() {
        let curve = ModCurve::SCurve(CurveSteepness::new(6.0));
        let mut prev = curve.apply(0.0);
        let mut i = 1;
        while i <= 10 {
            let x = i as f32 / 10.0;
            let value = curve.apply(x);
            assert!(value >= prev);
            prev = value;
            i += 1;
        }
    }

    #[test]
    fn steepness_clamps_to_valid_range() {
        let high = CurveSteepness::new(1000.0);
        assert_eq!(high.get(), CurveSteepness::MAX);
        let low = CurveSteepness::new(-5.0);
        assert_eq!(low.get(), CurveSteepness::MIN);
    }

    #[test]
    fn stepped_quantizes_into_bands() {
        let curve = ModCurve::Stepped(StepCount::new(4));
        assert_eq!(curve.apply(0.0), 0.0);
        assert!((curve.apply(1.0) - 1.0).abs() < 1e-6);
        // Values within the same band map to the same output.
        assert_eq!(curve.apply(0.1), curve.apply(0.2));
    }

    #[test]
    fn step_count_clamps_below_minimum() {
        let one = StepCount::new(1);
        assert_eq!(one.get(), StepCount::MIN);
        let zero = StepCount::new(0);
        assert_eq!(zero.get(), StepCount::MIN);
    }

    #[test]
    fn default_curve_is_linear() {
        assert_eq!(ModCurve::default(), ModCurve::Linear);
    }
}
