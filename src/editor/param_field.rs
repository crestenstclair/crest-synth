// path: src/editor/param_field.rs

/// One editable parameter row: a label, current value, inclusive bounds,
/// and the fine adjustment step (coarse = 10× step).
///
/// # Invariants
///
/// - `min <= max`
/// - `value` is always within `[min, max]`
/// - `step > 0`
///
/// Construction via [`ParamField::new`] enforces all invariants and clamps
/// `value` to `[min, max]` automatically. Returns `None` when any invariant
/// would be violated.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamField {
    /// Stable identifier for this parameter (e.g. `"cutoff"`, `"volume"`).
    pub id: String,
    /// Human-readable label shown in the UI.
    pub label: String,
    /// Minimum allowed value (inclusive).
    pub min: f64,
    /// Maximum allowed value (inclusive).
    pub max: f64,
    /// Smallest unit of fine adjustment; coarse = 10× this.
    pub step: f64,
    /// Current value; always satisfies `min <= value <= max`.
    value: f64,
}

impl ParamField {
    /// Create a new [`ParamField`], clamping `value` to `[min, max]`.
    ///
    /// Returns `None` when any invariant is violated:
    /// - `min > max`
    /// - `step <= 0` or `step` is NaN
    /// - `min` or `max` is NaN
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::editor::param_field::ParamField;
    ///
    /// let pf = ParamField::new("vol", "Volume", 0.0, 100.0, 1.0, 50.0).unwrap();
    /// assert_eq!(pf.id, "vol");
    /// assert_eq!(pf.label, "Volume");
    /// assert_eq!(pf.value(), 50.0);
    /// assert!(ParamField::new("x", "X", 1.0, 0.0, 0.1, 0.5).is_none()); // min > max
    /// assert!(ParamField::new("x", "X", 0.0, 1.0, 0.0, 0.5).is_none()); // step == 0
    /// ```
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        min: f64,
        max: f64,
        step: f64,
        value: f64,
    ) -> Option<Self> {
        if min.is_nan() || max.is_nan() || min > max || step.is_nan() || step <= 0.0 {
            return None;
        }
        Some(Self {
            id: id.into(),
            label: label.into(),
            min,
            max,
            step,
            value: value.clamp(min, max),
        })
    }

    /// Returns the current value (always within `[min, max]`).
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Set the value, clamping to `[min, max]`.
    pub fn set_value(&mut self, v: f64) {
        self.value = v.clamp(self.min, self.max);
    }

    /// Adjust the value by `delta` steps (positive = increase, negative = decrease),
    /// clamping to `[min, max]`.
    pub fn adjust(&mut self, delta: f64) {
        self.set_value(self.value + delta * self.step);
    }

    /// Increment by one fine step, clamping at `max`.
    pub fn step_up(&mut self) {
        self.set_value(self.value + self.step);
    }

    /// Decrement by one fine step, clamping at `min`.
    pub fn step_down(&mut self) {
        self.set_value(self.value - self.step);
    }

    /// Increment by one coarse step (10× fine), clamping at `max`.
    pub fn coarse_up(&mut self) {
        self.set_value(self.value + self.step * 10.0);
    }

    /// Decrement by one coarse step (10× fine), clamping at `min`.
    pub fn coarse_down(&mut self) {
        self.set_value(self.value - self.step * 10.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ──────────────────────── construction ────────────────────────

    #[test]
    fn param_field_new_valid() {
        let pf = ParamField::new("vol", "Volume", 0.0, 1.0, 0.01, 0.5).unwrap();
        assert_eq!(pf.id, "vol");
        assert_eq!(pf.label, "Volume");
        assert_eq!(pf.min, 0.0);
        assert_eq!(pf.max, 1.0);
        assert_eq!(pf.step, 0.01);
        assert_eq!(pf.value(), 0.5);
    }

    #[test]
    fn param_field_rejects_min_greater_than_max() {
        assert!(ParamField::new("x", "X", 1.0, 0.0, 0.1, 0.5).is_none());
    }

    #[test]
    fn param_field_rejects_zero_step() {
        assert!(ParamField::new("x", "X", 0.0, 1.0, 0.0, 0.5).is_none());
    }

    #[test]
    fn param_field_rejects_negative_step() {
        assert!(ParamField::new("x", "X", 0.0, 1.0, -0.1, 0.5).is_none());
    }

    #[test]
    fn param_field_rejects_nan_step() {
        assert!(ParamField::new("x", "X", 0.0, 1.0, f64::NAN, 0.5).is_none());
    }

    #[test]
    fn param_field_rejects_nan_min() {
        assert!(ParamField::new("x", "X", f64::NAN, 1.0, 0.1, 0.5).is_none());
    }

    #[test]
    fn param_field_rejects_nan_max() {
        assert!(ParamField::new("x", "X", 0.0, f64::NAN, 0.1, 0.5).is_none());
    }

    #[test]
    fn param_field_min_equals_max_is_valid() {
        let pf = ParamField::new("x", "X", 0.5, 0.5, 0.1, 0.5).unwrap();
        assert_eq!(pf.value(), 0.5);
    }

    // ──────────────────────── clamping ────────────────────────

    #[test]
    fn param_field_clamps_on_construction() {
        let pf = ParamField::new("vol", "Volume", 0.0, 100.0, 1.0, 200.0).unwrap();
        assert_eq!(pf.value(), 100.0);

        let pf2 = ParamField::new("vol", "Volume", 0.0, 100.0, 1.0, -5.0).unwrap();
        assert_eq!(pf2.value(), 0.0);
    }

    #[test]
    fn param_field_set_value_clamps() {
        let mut pf = ParamField::new("x", "X", 0.0, 1.0, 0.1, 0.5).unwrap();
        pf.set_value(5.0);
        assert_eq!(pf.value(), 1.0);
        pf.set_value(-5.0);
        assert_eq!(pf.value(), 0.0);
    }

    // ──────────────────────── stepping ────────────────────────

    #[test]
    fn param_field_step_up_clamps_at_max() {
        let mut pf = ParamField::new("x", "X", 0.0, 1.0, 0.1, 0.95).unwrap();
        pf.step_up();
        assert!((pf.value() - 1.0).abs() < f64::EPSILON);
        pf.step_up();
        assert_eq!(pf.value(), 1.0);
    }

    #[test]
    fn param_field_step_down_clamps_at_min() {
        let mut pf = ParamField::new("x", "X", 0.0, 1.0, 0.1, 0.05).unwrap();
        pf.step_down();
        assert!((pf.value() - 0.0).abs() < f64::EPSILON);
        pf.step_down();
        assert_eq!(pf.value(), 0.0);
    }

    #[test]
    fn param_field_coarse_up_is_ten_times_step() {
        let mut pf = ParamField::new("x", "X", 0.0, 100.0, 1.0, 10.0).unwrap();
        pf.coarse_up();
        assert!((pf.value() - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn param_field_coarse_down_is_ten_times_step() {
        let mut pf = ParamField::new("x", "X", 0.0, 100.0, 1.0, 10.0).unwrap();
        pf.coarse_down();
        assert!((pf.value() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn param_field_adjust_clamps() {
        let mut field = ParamField::new("vol", "Volume", 0.0, 100.0, 1.0, 99.0).unwrap();
        field.adjust(5.0);
        assert_eq!(field.value(), 100.0);

        field.adjust(-200.0);
        assert_eq!(field.value(), 0.0);
    }

    #[test]
    fn param_field_adjust_fine() {
        let mut field = ParamField::new("vol", "Volume", 0.0, 100.0, 1.0, 50.0).unwrap();
        field.adjust(1.0);
        assert_eq!(field.value(), 51.0);
        field.adjust(-1.0);
        assert_eq!(field.value(), 50.0);
    }

    // ──────────────────────── invariants always hold ──────────────

    #[test]
    fn param_field_value_always_within_bounds() {
        let mut pf = ParamField::new("x", "X", 5.0, 10.0, 0.5, 7.0).unwrap();
        for _ in 0..100 {
            pf.step_up();
            assert!(pf.value() >= pf.min && pf.value() <= pf.max);
        }
        for _ in 0..100 {
            pf.step_down();
            assert!(pf.value() >= pf.min && pf.value() <= pf.max);
        }
    }
}
