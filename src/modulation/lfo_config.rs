// path: src/modulation/lfo_config.rs

use std::error::Error;
use std::fmt;

/// The waveform shape an LFO cycles through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LfoShape {
    Sine,
    Triangle,
    Square,
    SawUp,
    SawDown,
    Random,
}

/// Reasons an `LfoConfig` cannot be constructed.
#[derive(Debug, Clone, PartialEq)]
pub enum LfoConfigError {
    /// `rateHz` was not a positive, finite number.
    NonPositiveRate(f64),
    /// `startPhase` was outside the inclusive `0.0..=1.0` range.
    StartPhaseOutOfRange(f64),
}

impl fmt::Display for LfoConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LfoConfigError::NonPositiveRate(value) => {
                write!(f, "rateHz must be positive, got {value}")
            }
            LfoConfigError::StartPhaseOutOfRange(value) => {
                write!(f, "startPhase must be within 0.0-1.0, got {value}")
            }
        }
    }
}

impl Error for LfoConfigError {}

/// Settings for one of the four LFOs available to a patch.
///
/// `LfoConfig` is an immutable value object: once constructed via
/// [`LfoConfig::try_new`] its invariants hold for the lifetime of the
/// value. There is no in-place mutation API — build a new `LfoConfig`
/// to change a setting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LfoConfig {
    depth: f64,
    rate_hz: f64,
    retrigger: bool,
    shape: LfoShape,
    start_phase: f64,
    tempo_sync: bool,
}

impl LfoConfig {
    /// Constructs an `LfoConfig`, validating all invariants.
    ///
    /// # Errors
    ///
    /// Returns [`LfoConfigError::NonPositiveRate`] if `rate_hz` is not a
    /// finite, strictly positive number, or
    /// [`LfoConfigError::StartPhaseOutOfRange`] if `start_phase` falls
    /// outside the inclusive `0.0..=1.0` range.
    ///
    /// # Examples
    ///
    /// ```
    /// use crest_synth::modulation::lfo_config::{LfoConfig, LfoShape};
    ///
    /// let lfo = LfoConfig::try_new(0.5, 2.0, true, LfoShape::Sine, 0.0, false).unwrap();
    /// assert_eq!(lfo.rate_hz(), 2.0);
    /// ```
    pub fn try_new(
        depth: f64,
        rate_hz: f64,
        retrigger: bool,
        shape: LfoShape,
        start_phase: f64,
        tempo_sync: bool,
    ) -> Result<Self, LfoConfigError> {
        if !(rate_hz > 0.0) {
            return Err(LfoConfigError::NonPositiveRate(rate_hz));
        }
        if start_phase.is_nan() || !(0.0..=1.0).contains(&start_phase) {
            return Err(LfoConfigError::StartPhaseOutOfRange(start_phase));
        }

        Ok(Self {
            depth,
            rate_hz,
            retrigger,
            shape,
            start_phase,
            tempo_sync,
        })
    }

    /// Modulation depth applied by this LFO.
    pub fn depth(&self) -> f64 {
        self.depth
    }

    /// LFO rate in Hz. Always strictly positive and finite.
    pub fn rate_hz(&self) -> f64 {
        self.rate_hz
    }

    /// Whether the LFO phase resets to `start_phase` on each note-on.
    pub fn retrigger(&self) -> bool {
        self.retrigger
    }

    /// The waveform shape this LFO cycles through.
    pub fn shape(&self) -> LfoShape {
        self.shape
    }

    /// The phase, in the inclusive range `0.0..=1.0`, the LFO starts at
    /// when retriggered (or at voice start).
    pub fn start_phase(&self) -> f64 {
        self.start_phase
    }

    /// Whether the LFO rate tracks host/session tempo instead of `rate_hz`.
    pub fn tempo_sync(&self) -> bool {
        self.tempo_sync
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_valid_config() {
        let lfo = LfoConfig::try_new(0.75, 4.0, true, LfoShape::Triangle, 0.25, false).unwrap();
        assert_eq!(lfo.depth(), 0.75);
        assert_eq!(lfo.rate_hz(), 4.0);
        assert!(lfo.retrigger());
        assert_eq!(lfo.shape(), LfoShape::Triangle);
        assert_eq!(lfo.start_phase(), 0.25);
        assert!(!lfo.tempo_sync());
    }

    #[test]
    fn try_new_accepts_boundary_start_phase_values() {
        assert!(LfoConfig::try_new(0.0, 1.0, false, LfoShape::Sine, 0.0, false).is_ok());
        assert!(LfoConfig::try_new(0.0, 1.0, false, LfoShape::Sine, 1.0, false).is_ok());
    }

    #[test]
    fn try_new_rejects_zero_rate() {
        let err = LfoConfig::try_new(0.0, 0.0, false, LfoShape::Sine, 0.0, false).unwrap_err();
        assert_eq!(err, LfoConfigError::NonPositiveRate(0.0));
    }

    #[test]
    fn try_new_rejects_negative_rate() {
        let err = LfoConfig::try_new(0.0, -1.5, false, LfoShape::Sine, 0.0, false).unwrap_err();
        assert_eq!(err, LfoConfigError::NonPositiveRate(-1.5));
    }

    #[test]
    fn try_new_rejects_nan_rate() {
        let err = LfoConfig::try_new(0.0, f64::NAN, false, LfoShape::Sine, 0.0, false).unwrap_err();
        assert!(matches!(err, LfoConfigError::NonPositiveRate(v) if v.is_nan()));
    }

    #[test]
    fn try_new_rejects_start_phase_below_zero() {
        let err = LfoConfig::try_new(0.0, 1.0, false, LfoShape::Sine, -0.01, false).unwrap_err();
        assert_eq!(err, LfoConfigError::StartPhaseOutOfRange(-0.01));
    }

    #[test]
    fn try_new_rejects_start_phase_above_one() {
        let err = LfoConfig::try_new(0.0, 1.0, false, LfoShape::Sine, 1.01, false).unwrap_err();
        assert_eq!(err, LfoConfigError::StartPhaseOutOfRange(1.01));
    }

    #[test]
    fn try_new_rejects_nan_start_phase() {
        let err =
            LfoConfig::try_new(0.0, 1.0, false, LfoShape::Sine, f64::NAN, false).unwrap_err();
        assert!(matches!(err, LfoConfigError::StartPhaseOutOfRange(v) if v.is_nan()));
    }

    #[test]
    fn error_messages_are_human_readable() {
        assert_eq!(
            LfoConfigError::NonPositiveRate(-2.0).to_string(),
            "rateHz must be positive, got -2"
        );
        assert_eq!(
            LfoConfigError::StartPhaseOutOfRange(2.0).to_string(),
            "startPhase must be within 0.0-1.0, got 2"
        );
    }
}
