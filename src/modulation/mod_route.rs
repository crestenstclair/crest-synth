// path: src/modulation/mod_route.rs

use std::fmt;

// ── Supporting types ─────────────────────────────────────────────────────────────────

/// The shape of the curve applied to a modulation route's source value
/// before it reaches the destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModCurve {
    /// Output tracks input directly.
    Linear,
    /// Small input values produce disproportionately small output.
    Exponential,
    /// Small input values produce disproportionately large output.
    Logarithmic,
    /// Eased in and out, flattest at the extremes.
    SCurve,
}

/// A modulation source: something that can produce a control signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModSource {
    Lfo1,
    Lfo2,
    EnvelopeMod1,
    EnvelopeMod2,
    Velocity,
    Aftertouch,
    ModWheel,
    PitchBend,
    NoteNumber,
    /// Per-note MPE expression: X-axis bend.
    PerNoteBendX,
    /// Per-note MPE expression: Y-axis timbre.
    PerNoteTimbreY,
    /// Per-note MPE expression: Z-axis pressure.
    PerNotePressureZ,
}

/// A modulation destination: a parameter that can be modulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModDestination {
    OscillatorPitch,
    OscillatorLevel,
    FilterCutoff,
    FilterResonance,
    Amplitude,
    Pan,
    LfoRate,
    EnvelopeAttack,
    EnvelopeDecay,
    EnvelopeSustain,
    EnvelopeRelease,
}

/// Error produced when constructing or updating a [`ModRoute`] with an
/// invalid amount.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModRouteError {
    /// The amount was NaN or outside `[-1.0, 1.0]`.
    AmountOutOfRange(f64),
}

impl fmt::Display for ModRouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModRouteError::AmountOutOfRange(v) => {
                write!(f, "mod route amount {v} out of range [-1.0, 1.0]")
            }
        }
    }
}

impl std::error::Error for ModRouteError {}

// ── ModRoute ─────────────────────────────────────────────────────────────────────────

/// One modulation route: `source` modulates `destination`, scaled by
/// `amount` and reshaped by `curve`.
///
/// The optional `via` source scales the route's depth: when present, the
/// route's effective amount at any instant is `amount` multiplied by the
/// current value of the `via` source, letting one modulation source act as
/// an amount-scaler for another route (e.g. mod-wheel scaling an LFO's
/// depth on filter cutoff).
///
/// `ModRoute` is a pure value object: it carries no behavior beyond
/// validating and exposing its own fields. Combining routes with live
/// source/via values into an audible signal is the responsibility of a
/// modulation processor that reads a `ModRoute` snapshot handed across the
/// real-time boundary — `ModRoute` itself performs no I/O and allocates no
/// heap memory, so it is safe to hold and read on the audio thread.
///
/// # Invariants
///
/// - `amount` is in `[-1.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModRoute {
    amount: f64,
    bipolar: bool,
    curve: ModCurve,
    destination: ModDestination,
    source: ModSource,
    via: Option<ModSource>,
}

impl ModRoute {
    /// Construct a new `ModRoute`.
    ///
    /// # Errors
    ///
    /// Returns [`ModRouteError::AmountOutOfRange`] if `amount` is NaN or
    /// outside `[-1.0, 1.0]`.
    pub fn try_new(
        source: ModSource,
        destination: ModDestination,
        amount: f64,
        bipolar: bool,
        curve: ModCurve,
        via: Option<ModSource>,
    ) -> Result<Self, ModRouteError> {
        Self::validate_amount(amount)?;
        Ok(Self {
            amount,
            bipolar,
            curve,
            destination,
            source,
            via,
        })
    }

    fn validate_amount(amount: f64) -> Result<(), ModRouteError> {
        if amount.is_nan() || !(-1.0..=1.0).contains(&amount) {
            return Err(ModRouteError::AmountOutOfRange(amount));
        }
        Ok(())
    }

    /// The route's depth, in `[-1.0, 1.0]`.
    pub fn amount(&self) -> f64 {
        self.amount
    }

    /// Whether the route modulates symmetrically around center (bipolar)
    /// or only in one direction from a resting value (unipolar).
    pub fn bipolar(&self) -> bool {
        self.bipolar
    }

    /// The curve reshaping the source value before it is scaled by `amount`.
    pub fn curve(&self) -> ModCurve {
        self.curve
    }

    /// The parameter this route modulates.
    pub fn destination(&self) -> ModDestination {
        self.destination
    }

    /// The signal driving this route.
    pub fn source(&self) -> ModSource {
        self.source
    }

    /// The optional source that scales this route's effective depth.
    pub fn via(&self) -> Option<ModSource> {
        self.via
    }

    /// True when this route's depth is scaled by a `via` source.
    pub fn is_scaled(&self) -> bool {
        self.via.is_some()
    }

    /// Returns a copy of this route with `amount` replaced, validated.
    ///
    /// # Errors
    ///
    /// Returns [`ModRouteError::AmountOutOfRange`] if `amount` is NaN or
    /// outside `[-1.0, 1.0]`.
    pub fn with_amount(&self, amount: f64) -> Result<Self, ModRouteError> {
        Self::validate_amount(amount)?;
        Ok(Self { amount, ..*self })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn route(amount: f64) -> Result<ModRoute, ModRouteError> {
        ModRoute::try_new(
            ModSource::Lfo1,
            ModDestination::FilterCutoff,
            amount,
            true,
            ModCurve::Linear,
            None,
        )
    }

    #[test]
    fn try_new_accepts_amount_within_range() {
        let r = route(0.5).unwrap();
        assert!((r.amount() - 0.5).abs() < f64::EPSILON);
        assert_eq!(r.source(), ModSource::Lfo1);
        assert_eq!(r.destination(), ModDestination::FilterCutoff);
        assert!(r.bipolar());
        assert_eq!(r.curve(), ModCurve::Linear);
        assert_eq!(r.via(), None);
        assert!(!r.is_scaled());
    }

    #[test]
    fn try_new_accepts_boundary_values() {
        assert!(route(1.0).is_ok());
        assert!(route(-1.0).is_ok());
    }

    #[test]
    fn try_new_rejects_amount_above_range() {
        let err = route(1.0001).unwrap_err();
        assert_eq!(err, ModRouteError::AmountOutOfRange(1.0001));
    }

    #[test]
    fn try_new_rejects_amount_below_range() {
        assert!(route(-1.0001).is_err());
    }

    #[test]
    fn try_new_rejects_nan_amount() {
        assert!(route(f64::NAN).is_err());
    }

    #[test]
    fn with_via_source_marks_route_as_scaled() {
        let r = ModRoute::try_new(
            ModSource::ModWheel,
            ModDestination::Amplitude,
            0.3,
            false,
            ModCurve::Exponential,
            Some(ModSource::Aftertouch),
        )
        .unwrap();
        assert!(r.is_scaled());
        assert_eq!(r.via(), Some(ModSource::Aftertouch));
    }

    #[test]
    fn with_amount_updates_and_validates() {
        let r = route(0.2).unwrap();
        let updated = r.with_amount(-0.4).unwrap();
        assert!((updated.amount() - (-0.4)).abs() < f64::EPSILON);
        // Other fields unchanged.
        assert_eq!(updated.source(), r.source());
        assert_eq!(updated.destination(), r.destination());
    }

    #[test]
    fn with_amount_rejects_out_of_range() {
        let r = route(0.0).unwrap();
        assert!(r.with_amount(2.0).is_err());
    }

    #[test]
    fn error_display_contains_offending_value() {
        let err = ModRouteError::AmountOutOfRange(2.5);
        assert!(err.to_string().contains("2.5"));
    }

    #[test]
    fn routes_with_same_fields_are_equal() {
        let a = route(0.5).unwrap();
        let b = route(0.5).unwrap();
        assert_eq!(a, b);
    }
}
