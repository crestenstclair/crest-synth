/// Waveform shape for an oscillator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Waveform {
    /// Sine wave.
    #[default]
    Sine,
    /// Square wave.
    Square,
    /// Sawtooth wave (rising).
    Saw,
    /// Triangle wave.
    Triangle,
    /// Pulse wave (width controlled by `OscillatorConfig::pulse_width`).
    Pulse,
}

/// Oscillator parameters: waveform, detune offset, and pulse width.
///
/// `detune` is in cents (±100 = ±1 semitone).
/// `pulse_width` is the duty cycle for pulse/square waves, in the range 0.0–1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OscillatorConfig {
    /// Detune offset in cents.
    pub detune: f64,
    /// Duty cycle for pulse waveforms (0.0–1.0).
    pub pulse_width: f64,
    /// Waveform shape.
    pub waveform: Waveform,
}

/// Error returned when `OscillatorConfig` fields are out of valid range.
#[derive(Debug, Clone, PartialEq)]
pub struct OscillatorConfigError(pub String);

impl std::fmt::Display for OscillatorConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OscillatorConfig error: {}", self.0)
    }
}

impl std::error::Error for OscillatorConfigError {}

impl OscillatorConfig {
    /// Maximum detune magnitude in cents (±9600 cents = ±8 octaves).
    pub const MAX_DETUNE_CENTS: f64 = 9_600.0;

    /// Construct an `OscillatorConfig`, validating all fields.
    ///
    /// Returns `Err` if:
    /// - `detune` is NaN or outside `±MAX_DETUNE_CENTS`
    /// - `pulse_width` is NaN or outside 0.0–1.0
    ///
    /// ```
    /// use crest_synth::synth::oscillator_config::{OscillatorConfig, Waveform};
    /// let cfg = OscillatorConfig::try_new(0.0, 0.5, Waveform::Sine).unwrap();
    /// assert_eq!(cfg.waveform, Waveform::Sine);
    /// ```
    pub fn try_new(
        detune: f64,
        pulse_width: f64,
        waveform: Waveform,
    ) -> Result<Self, OscillatorConfigError> {
        if detune.is_nan() || !(-Self::MAX_DETUNE_CENTS..=Self::MAX_DETUNE_CENTS).contains(&detune)
        {
            return Err(OscillatorConfigError(format!(
                "detune {} is out of range ±{}",
                detune,
                Self::MAX_DETUNE_CENTS
            )));
        }
        if pulse_width.is_nan() || !(0.0..=1.0).contains(&pulse_width) {
            return Err(OscillatorConfigError(format!(
                "pulse_width {} is out of range 0.0–1.0",
                pulse_width
            )));
        }
        Ok(Self {
            detune,
            pulse_width,
            waveform,
        })
    }
}

impl Default for OscillatorConfig {
    /// Returns a sine wave with no detune and 50% pulse width.
    fn default() -> Self {
        Self {
            detune: 0.0,
            pulse_width: 0.5,
            waveform: Waveform::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid_sine() {
        let cfg = OscillatorConfig::default();
        assert_eq!(cfg.waveform, Waveform::Sine);
        assert!((cfg.detune).abs() < f64::EPSILON);
        assert!((cfg.pulse_width - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn try_new_valid_params() {
        let cfg = OscillatorConfig::try_new(50.0, 0.25, Waveform::Square).unwrap();
        assert_eq!(cfg.waveform, Waveform::Square);
        assert!((cfg.detune - 50.0).abs() < f64::EPSILON);
        assert!((cfg.pulse_width - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn detune_nan_rejected() {
        assert!(OscillatorConfig::try_new(f64::NAN, 0.5, Waveform::Sine).is_err());
    }

    #[test]
    fn detune_out_of_range_rejected() {
        assert!(OscillatorConfig::try_new(
            OscillatorConfig::MAX_DETUNE_CENTS + 1.0,
            0.5,
            Waveform::Sine
        )
        .is_err());
        assert!(OscillatorConfig::try_new(
            -(OscillatorConfig::MAX_DETUNE_CENTS + 1.0),
            0.5,
            Waveform::Sine
        )
        .is_err());
    }

    #[test]
    fn detune_at_max_bounds_ok() {
        assert!(
            OscillatorConfig::try_new(OscillatorConfig::MAX_DETUNE_CENTS, 0.5, Waveform::Saw)
                .is_ok()
        );
        assert!(
            OscillatorConfig::try_new(-OscillatorConfig::MAX_DETUNE_CENTS, 0.5, Waveform::Saw)
                .is_ok()
        );
    }

    #[test]
    fn pulse_width_nan_rejected() {
        assert!(OscillatorConfig::try_new(0.0, f64::NAN, Waveform::Pulse).is_err());
    }

    #[test]
    fn pulse_width_out_of_range_rejected() {
        assert!(OscillatorConfig::try_new(0.0, -0.1, Waveform::Pulse).is_err());
        assert!(OscillatorConfig::try_new(0.0, 1.1, Waveform::Pulse).is_err());
    }

    #[test]
    fn pulse_width_at_bounds_ok() {
        assert!(OscillatorConfig::try_new(0.0, 0.0, Waveform::Pulse).is_ok());
        assert!(OscillatorConfig::try_new(0.0, 1.0, Waveform::Pulse).is_ok());
    }

    #[test]
    fn copy_semantics() {
        let a = OscillatorConfig::default();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn all_waveforms_constructible() {
        for wf in [
            Waveform::Sine,
            Waveform::Square,
            Waveform::Saw,
            Waveform::Triangle,
            Waveform::Pulse,
        ] {
            assert!(OscillatorConfig::try_new(0.0, 0.5, wf).is_ok());
        }
    }

    #[test]
    fn error_display() {
        let err = OscillatorConfig::try_new(f64::NAN, 0.5, Waveform::Sine).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("OscillatorConfig error"));
    }
}
