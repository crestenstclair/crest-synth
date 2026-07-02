// path: src/effects/delay_params.rs

//! Delay line settings.
//!
//! `DelayParams` is an immutable value object describing the configuration
//! of a delay effect: how long the delay line is, how much of the delayed
//! signal feeds back into itself, whether it alternates left/right
//! ("ping-pong"), whether the time locks to tempo, and the wet/dry mix.
//!
//! Construction is validated: `feedback` and `wetDry` must each lie in
//! `0.0..=1.0`. Feedback above unity would make the delay line
//! self-oscillate unboundedly, which is never a valid state for this type
//! to represent.

/// An error returned when constructing a [`DelayParams`] with an
/// out-of-range field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DelayParamsError {
    /// `feedback` was not within `0.0..=1.0` (or was NaN).
    FeedbackOutOfRange(f64),
    /// `wetDry` was not within `0.0..=1.0` (or was NaN).
    WetDryOutOfRange(f64),
}

impl std::fmt::Display for DelayParamsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DelayParamsError::FeedbackOutOfRange(v) => {
                write!(f, "feedback must be within 0.0..=1.0, got {v}")
            }
            DelayParamsError::WetDryOutOfRange(v) => {
                write!(f, "wetDry must be within 0.0..=1.0, got {v}")
            }
        }
    }
}

impl std::error::Error for DelayParamsError {}

/// Delay line settings: time, feedback, mix, and routing/sync flags.
///
/// # Invariants
///
/// - `feedback` is within `0.0..=1.0` (unity or less, or the line
///   self-oscillates unboundedly).
/// - `wetDry` is within `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DelayParams {
    feedback: f64,
    ping_pong: bool,
    tempo_sync: bool,
    time_ms: f64,
    wet_dry: f64,
}

impl DelayParams {
    /// The largest valid feedback amount (unity gain).
    pub const MAX_FEEDBACK: f64 = 1.0;
    /// The smallest valid feedback amount.
    pub const MIN_FEEDBACK: f64 = 0.0;
    /// The largest valid wet/dry mix (fully wet).
    pub const MAX_WET_DRY: f64 = 1.0;
    /// The smallest valid wet/dry mix (fully dry).
    pub const MIN_WET_DRY: f64 = 0.0;

    /// Constructs a new [`DelayParams`], validating `feedback` and
    /// `wet_dry`.
    ///
    /// `time_ms` is not range-checked here: it is expected to be a
    /// non-negative delay time in milliseconds, but this type does not
    /// enforce a specific upper bound since that is a property of the
    /// concrete delay line implementation, not of the parameter value
    /// object itself.
    ///
    /// # Errors
    ///
    /// Returns [`DelayParamsError::FeedbackOutOfRange`] if `feedback` is
    /// NaN or outside `0.0..=1.0`.
    /// Returns [`DelayParamsError::WetDryOutOfRange`] if `wet_dry` is NaN
    /// or outside `0.0..=1.0`.
    pub fn try_new(
        feedback: f64,
        ping_pong: bool,
        tempo_sync: bool,
        time_ms: f64,
        wet_dry: f64,
    ) -> Result<Self, DelayParamsError> {
        if feedback.is_nan() || !(Self::MIN_FEEDBACK..=Self::MAX_FEEDBACK).contains(&feedback) {
            return Err(DelayParamsError::FeedbackOutOfRange(feedback));
        }
        if wet_dry.is_nan() || !(Self::MIN_WET_DRY..=Self::MAX_WET_DRY).contains(&wet_dry) {
            return Err(DelayParamsError::WetDryOutOfRange(wet_dry));
        }

        Ok(Self {
            feedback,
            ping_pong,
            tempo_sync,
            time_ms,
            wet_dry,
        })
    }

    /// The feedback amount, in `0.0..=1.0`.
    pub fn feedback(&self) -> f64 {
        self.feedback
    }

    /// Whether the delay alternates between left and right channels.
    pub fn ping_pong(&self) -> bool {
        self.ping_pong
    }

    /// Whether the delay time locks to the host tempo.
    pub fn tempo_sync(&self) -> bool {
        self.tempo_sync
    }

    /// The delay time in milliseconds.
    pub fn time_ms(&self) -> f64 {
        self.time_ms
    }

    /// The wet/dry mix, in `0.0..=1.0` (0.0 is fully dry, 1.0 is fully
    /// wet).
    pub fn wet_dry(&self) -> f64 {
        self.wet_dry
    }

    /// Returns a copy of `self` with `feedback` replaced, re-validating
    /// the new value.
    pub fn with_feedback(&self, feedback: f64) -> Result<Self, DelayParamsError> {
        Self::try_new(
            feedback,
            self.ping_pong,
            self.tempo_sync,
            self.time_ms,
            self.wet_dry,
        )
    }

    /// Returns a copy of `self` with `ping_pong` replaced.
    pub fn with_ping_pong(&self, ping_pong: bool) -> Self {
        Self { ping_pong, ..*self }
    }

    /// Returns a copy of `self` with `tempo_sync` replaced.
    pub fn with_tempo_sync(&self, tempo_sync: bool) -> Self {
        Self {
            tempo_sync,
            ..*self
        }
    }

    /// Returns a copy of `self` with `time_ms` replaced.
    pub fn with_time_ms(&self, time_ms: f64) -> Self {
        Self { time_ms, ..*self }
    }

    /// Returns a copy of `self` with `wet_dry` replaced, re-validating the
    /// new value.
    pub fn with_wet_dry(&self, wet_dry: f64) -> Result<Self, DelayParamsError> {
        Self::try_new(
            self.feedback,
            self.ping_pong,
            self.tempo_sync,
            self.time_ms,
            wet_dry,
        )
    }
}

impl Default for DelayParams {
    /// A quarter-note-ish delay at moderate feedback and an even mix,
    /// with no ping-pong or tempo sync.
    fn default() -> Self {
        Self {
            feedback: 0.35,
            ping_pong: false,
            tempo_sync: false,
            time_ms: 375.0,
            wet_dry: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_valid_params() {
        let params = DelayParams::try_new(0.35, true, false, 375.0, 0.5)
            .expect("valid params should construct");

        assert_eq!(params.feedback(), 0.35);
        assert!(params.ping_pong());
        assert!(!params.tempo_sync());
        assert_eq!(params.time_ms(), 375.0);
        assert_eq!(params.wet_dry(), 0.5);
    }

    #[test]
    fn try_new_accepts_boundary_values() {
        assert!(DelayParams::try_new(0.0, false, false, 0.0, 0.0).is_ok());
        assert!(DelayParams::try_new(1.0, false, false, 0.0, 1.0).is_ok());
    }

    #[test]
    fn try_new_rejects_feedback_above_unity() {
        let err = DelayParams::try_new(1.5, false, false, 375.0, 0.5)
            .expect_err("feedback above 1.0 must be rejected");
        assert_eq!(err, DelayParamsError::FeedbackOutOfRange(1.5));
    }

    #[test]
    fn try_new_rejects_negative_feedback() {
        let err = DelayParams::try_new(-0.1, false, false, 375.0, 0.5)
            .expect_err("negative feedback must be rejected");
        assert_eq!(err, DelayParamsError::FeedbackOutOfRange(-0.1));
    }

    #[test]
    fn try_new_rejects_nan_feedback() {
        let err = DelayParams::try_new(f64::NAN, false, false, 375.0, 0.5)
            .expect_err("NaN feedback must be rejected");
        assert!(matches!(err, DelayParamsError::FeedbackOutOfRange(v) if v.is_nan()));
    }

    #[test]
    fn try_new_rejects_wet_dry_out_of_range() {
        let err = DelayParams::try_new(0.35, false, false, 375.0, 1.2)
            .expect_err("wetDry above 1.0 must be rejected");
        assert_eq!(err, DelayParamsError::WetDryOutOfRange(1.2));

        let err = DelayParams::try_new(0.35, false, false, 375.0, -0.2)
            .expect_err("wetDry below 0.0 must be rejected");
        assert_eq!(err, DelayParamsError::WetDryOutOfRange(-0.2));
    }

    #[test]
    fn try_new_rejects_nan_wet_dry() {
        let err = DelayParams::try_new(0.35, false, false, 375.0, f64::NAN)
            .expect_err("NaN wetDry must be rejected");
        assert!(matches!(err, DelayParamsError::WetDryOutOfRange(v) if v.is_nan()));
    }

    #[test]
    fn default_is_valid() {
        let defaults = DelayParams::default();
        assert!((DelayParams::MIN_FEEDBACK..=DelayParams::MAX_FEEDBACK).contains(&defaults.feedback()));
        assert!((DelayParams::MIN_WET_DRY..=DelayParams::MAX_WET_DRY).contains(&defaults.wet_dry()));
    }

    #[test]
    fn with_feedback_revalidates() {
        let params = DelayParams::default();
        assert!(params.with_feedback(0.9).is_ok());
        assert!(params.with_feedback(1.1).is_err());
    }

    #[test]
    fn with_wet_dry_revalidates() {
        let params = DelayParams::default();
        assert!(params.with_wet_dry(0.9).is_ok());
        assert!(params.with_wet_dry(-0.1).is_err());
    }

    #[test]
    fn with_ping_pong_and_tempo_sync_and_time_ms_update_in_place() {
        let params = DelayParams::default();
        let updated = params
            .with_ping_pong(true)
            .with_tempo_sync(true)
            .with_time_ms(500.0);

        assert!(updated.ping_pong());
        assert!(updated.tempo_sync());
        assert_eq!(updated.time_ms(), 500.0);
        // Untouched fields are preserved.
        assert_eq!(updated.feedback(), params.feedback());
        assert_eq!(updated.wet_dry(), params.wet_dry());
    }
}
