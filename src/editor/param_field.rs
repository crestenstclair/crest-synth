// path: src/editor/param_field.rs

//! A single editable parameter field shown in the editor: a bounded value
//! with a name and a step size defining the "one unit" fine adjustment.

/// One editable field in the editor: a named value bounded to `[min, max]`
/// with a `step` defining the fine-adjustment unit (coarse adjustment is
/// ten times `step`).
#[derive(Debug, Clone, PartialEq)]
pub struct ParamField {
    name: String,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
}

impl ParamField {
    /// Creates a new field, clamping `value` to `[min, max]` on construction.
    ///
    /// # Panics
    ///
    /// Panics if `min > max`, `step <= 0.0`, or any of `value`/`min`/`max`/`step`
    /// is NaN.
    pub fn new(name: impl Into<String>, value: f32, min: f32, max: f32, step: f32) -> Self {
        assert!(
            !min.is_nan() && !max.is_nan() && !value.is_nan() && !step.is_nan(),
            "ParamField fields must not be NaN"
        );
        assert!(min <= max, "ParamField min must be <= max");
        assert!(step > 0.0, "ParamField step must be positive");
        let value = value.clamp(min, max);
        Self { name: name.into(), value, min, max, step }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn min(&self) -> f32 {
        self.min
    }

    pub fn max(&self) -> f32 {
        self.max
    }

    pub fn step(&self) -> f32 {
        self.step
    }

    /// Adjusts the value by `delta`, clamping the result to `[min, max]`.
    /// Crate-internal: only `EditorState::apply` performs mutation, keeping
    /// `apply` the sole entry point for editor state changes.
    pub(crate) fn adjust(&mut self, delta: f32) {
        self.value = (self.value + delta).clamp(self.min, self.max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_clamps_out_of_range_value() {
        let field = ParamField::new("gain", 999.0, 0.0, 10.0, 1.0);
        assert_eq!(field.value(), 10.0);
    }

    #[test]
    fn adjust_clamps_to_max() {
        let mut field = ParamField::new("gain", 9.5, 0.0, 10.0, 1.0);
        field.adjust(5.0);
        assert_eq!(field.value(), 10.0);
    }

    #[test]
    fn adjust_clamps_to_min() {
        let mut field = ParamField::new("gain", 0.5, 0.0, 10.0, 1.0);
        field.adjust(-5.0);
        assert_eq!(field.value(), 0.0);
    }
}
