/// Error returned when a [`PerNoteExpression`] value is out of range or NaN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerNoteExpressionError {
    field: &'static str,
    value: f64,
}

impl std::fmt::Display for PerNoteExpressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PerNoteExpression field '{}' value {} is out of range 0.0-1.0",
            self.field, self.value
        )
    }
}

impl std::error::Error for PerNoteExpressionError {}

/// Per-note expression triple: X = pitch bend, Y = timbre, Z = pressure.
///
/// All three dimensions are normalized to 0.0–1.0.
/// Pitch bend (X) is bipolar and stored with 0.5 as the center (no bend).
/// This is per-voice, not per-patch.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PerNoteExpression {
    /// Pitch bend: bipolar, 0.5 = center (no bend), 0.0 = full down, 1.0 = full up.
    bend_x: f64,
    /// Pressure: 0.0 = none, 1.0 = maximum.
    pressure_z: f64,
    /// Timbre: 0.0 = minimum, 1.0 = maximum.
    timbre_y: f64,
}

fn check_field(name: &'static str, value: f64) -> Result<(), PerNoteExpressionError> {
    if value.is_nan() || !(0.0..=1.0).contains(&value) {
        return Err(PerNoteExpressionError { field: name, value });
    }
    Ok(())
}

impl PerNoteExpression {
    /// Construct a [`PerNoteExpression`] with explicit values for all three dimensions.
    ///
    /// Returns `Err` if any value is NaN or outside `0.0..=1.0`.
    ///
    /// ```
    /// use crest_synth::modulation::per_note_expression::PerNoteExpression;
    /// assert!(PerNoteExpression::try_new(0.5, 0.0, 0.0).is_ok());
    /// assert!(PerNoteExpression::try_new(1.1, 0.0, 0.0).is_err());
    /// ```
    pub fn try_new(
        bend_x: f64,
        timbre_y: f64,
        pressure_z: f64,
    ) -> Result<Self, PerNoteExpressionError> {
        check_field("bend_x", bend_x)?;
        check_field("timbre_y", timbre_y)?;
        check_field("pressure_z", pressure_z)?;
        Ok(Self {
            bend_x,
            pressure_z,
            timbre_y,
        })
    }

    /// Return the pitch bend (X) value. 0.5 = center, 0.0 = full down, 1.0 = full up.
    #[inline]
    pub fn bend_x(self) -> f64 {
        self.bend_x
    }

    /// Return the timbre (Y) value.
    #[inline]
    pub fn timbre_y(self) -> f64 {
        self.timbre_y
    }

    /// Return the pressure (Z) value.
    #[inline]
    pub fn pressure_z(self) -> f64 {
        self.pressure_z
    }
}

impl Default for PerNoteExpression {
    /// Returns neutral expression: bend at center (0.5), timbre and pressure at 0.0.
    fn default() -> Self {
        Self {
            bend_x: 0.5,
            timbre_y: 0.0,
            pressure_z: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_valid() {
        let expr = PerNoteExpression::default();
        assert!((expr.bend_x() - 0.5).abs() < f64::EPSILON);
        assert!((expr.timbre_y()).abs() < f64::EPSILON);
        assert!((expr.pressure_z()).abs() < f64::EPSILON);
    }

    #[test]
    fn valid_construction() {
        let expr = PerNoteExpression::try_new(0.5, 0.75, 0.25).unwrap();
        assert!((expr.bend_x() - 0.5).abs() < f64::EPSILON);
        assert!((expr.timbre_y() - 0.75).abs() < f64::EPSILON);
        assert!((expr.pressure_z() - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn boundary_values_are_valid() {
        assert!(PerNoteExpression::try_new(0.0, 0.0, 0.0).is_ok());
        assert!(PerNoteExpression::try_new(1.0, 1.0, 1.0).is_ok());
    }

    #[test]
    fn bend_x_out_of_range_rejected() {
        assert!(PerNoteExpression::try_new(1.1, 0.0, 0.0).is_err());
        assert!(PerNoteExpression::try_new(-0.1, 0.0, 0.0).is_err());
    }

    #[test]
    fn timbre_y_out_of_range_rejected() {
        assert!(PerNoteExpression::try_new(0.5, 1.1, 0.0).is_err());
        assert!(PerNoteExpression::try_new(0.5, -0.1, 0.0).is_err());
    }

    #[test]
    fn pressure_z_out_of_range_rejected() {
        assert!(PerNoteExpression::try_new(0.5, 0.0, 1.1).is_err());
        assert!(PerNoteExpression::try_new(0.5, 0.0, -0.1).is_err());
    }

    #[test]
    fn nan_bend_x_rejected() {
        assert!(PerNoteExpression::try_new(f64::NAN, 0.0, 0.0).is_err());
    }

    #[test]
    fn nan_timbre_y_rejected() {
        assert!(PerNoteExpression::try_new(0.5, f64::NAN, 0.0).is_err());
    }

    #[test]
    fn nan_pressure_z_rejected() {
        assert!(PerNoteExpression::try_new(0.5, 0.0, f64::NAN).is_err());
    }

    #[test]
    fn copy_semantics() {
        let a = PerNoteExpression::try_new(0.5, 0.3, 0.7).unwrap();
        let b = a;
        assert!((a.bend_x() - b.bend_x()).abs() < f64::EPSILON);
        assert!((a.timbre_y() - b.timbre_y()).abs() < f64::EPSILON);
        assert!((a.pressure_z() - b.pressure_z()).abs() < f64::EPSILON);
    }

    #[test]
    fn error_display_mentions_field_and_value() {
        let err = PerNoteExpression::try_new(1.5, 0.0, 0.0).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("bend_x"), "expected 'bend_x' in: {}", msg);
        assert!(msg.contains("1.5"), "expected '1.5' in: {}", msg);
    }

    #[test]
    fn center_bend_is_no_bend() {
        // 0.5 is the neutral position for the bipolar bend dimension
        let expr = PerNoteExpression::try_new(0.5, 0.0, 0.0).unwrap();
        assert!((expr.bend_x() - 0.5).abs() < f64::EPSILON);
    }
}
