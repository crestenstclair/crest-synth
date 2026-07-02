// path: src/engine/oscillator_config.rs

//! Oscillator settings for one voice.
//!
//! `OscillatorConfig` is a plain value object: it holds validated oscillator
//! parameters and enforces its own invariants on construction and on every
//! mutation. It has no dependencies on other subsystems, so it needs no
//! injected collaborators — value types are data, not services.

use std::fmt;

/// The waveform shape produced by an oscillator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Waveform {
    #[default]
    Sine,
    Saw,
    Square,
    Triangle,
    Pulse,
    Noise,
}

/// Error returned when an `OscillatorConfig` field would violate an
/// invariant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OscillatorConfigError {
    /// `unisonVoices` must be in `1..=16`.
    UnisonVoicesOutOfRange(u8),
    /// `pulseWidth` must be in `0.0..=1.0`.
    PulseWidthOutOfRange(f64),
}

impl fmt::Display for OscillatorConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OscillatorConfigError::UnisonVoicesOutOfRange(v) => {
                write!(f, "unisonVoices must be 1-16, got {v}")
            }
            OscillatorConfigError::PulseWidthOutOfRange(v) => {
                write!(f, "pulseWidth must be 0.0-1.0, got {v}")
            }
        }
    }
}

impl std::error::Error for OscillatorConfigError {}

const MIN_UNISON_VOICES: u8 = 1;
const MAX_UNISON_VOICES: u8 = 16;
const PULSE_WIDTH_RANGE: std::ops::RangeInclusive<f64> = 0.0..=1.0;

/// Oscillator settings for one voice.
///
/// Fields are kept private so every mutation goes through a validating
/// method, guaranteeing the type can never observe an invalid state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OscillatorConfig {
    detune_cents: f64,
    pulse_width: f64,
    unison_spread: f64,
    unison_voices: u8,
    waveform: Waveform,
}

impl OscillatorConfig {
    /// Builds a new `OscillatorConfig`, validating `unison_voices` and
    /// `pulse_width` against their invariants.
    pub fn new(
        detune_cents: f64,
        pulse_width: f64,
        unison_spread: f64,
        unison_voices: u8,
        waveform: Waveform,
    ) -> Result<Self, OscillatorConfigError> {
        Self::validate_unison_voices(unison_voices)?;
        Self::validate_pulse_width(pulse_width)?;

        Ok(Self {
            detune_cents,
            pulse_width,
            unison_spread,
            unison_voices,
            waveform,
        })
    }

    fn validate_unison_voices(unison_voices: u8) -> Result<(), OscillatorConfigError> {
        if !(MIN_UNISON_VOICES..=MAX_UNISON_VOICES).contains(&unison_voices) {
            return Err(OscillatorConfigError::UnisonVoicesOutOfRange(unison_voices));
        }
        Ok(())
    }

    fn validate_pulse_width(pulse_width: f64) -> Result<(), OscillatorConfigError> {
        if pulse_width.is_nan() || !PULSE_WIDTH_RANGE.contains(&pulse_width) {
            return Err(OscillatorConfigError::PulseWidthOutOfRange(pulse_width));
        }
        Ok(())
    }

    pub fn detune_cents(&self) -> f64 {
        self.detune_cents
    }

    pub fn pulse_width(&self) -> f64 {
        self.pulse_width
    }

    pub fn unison_spread(&self) -> f64 {
        self.unison_spread
    }

    pub fn unison_voices(&self) -> u8 {
        self.unison_voices
    }

    pub fn waveform(&self) -> Waveform {
        self.waveform
    }

    /// Returns a copy of this config with `detune_cents` replaced.
    pub fn with_detune_cents(&self, detune_cents: f64) -> Self {
        Self {
            detune_cents,
            ..*self
        }
    }

    /// Returns a copy of this config with `pulse_width` replaced, validating
    /// the new value.
    pub fn with_pulse_width(&self, pulse_width: f64) -> Result<Self, OscillatorConfigError> {
        Self::validate_pulse_width(pulse_width)?;
        Ok(Self {
            pulse_width,
            ..*self
        })
    }

    /// Returns a copy of this config with `unison_spread` replaced.
    pub fn with_unison_spread(&self, unison_spread: f64) -> Self {
        Self {
            unison_spread,
            ..*self
        }
    }

    /// Returns a copy of this config with `unison_voices` replaced,
    /// validating the new value.
    pub fn with_unison_voices(&self, unison_voices: u8) -> Result<Self, OscillatorConfigError> {
        Self::validate_unison_voices(unison_voices)?;
        Ok(Self {
            unison_voices,
            ..*self
        })
    }

    /// Returns a copy of this config with `waveform` replaced.
    pub fn with_waveform(&self, waveform: Waveform) -> Self {
        Self { waveform, ..*self }
    }
}

impl Default for OscillatorConfig {
    fn default() -> Self {
        Self {
            detune_cents: 0.0,
            pulse_width: 0.5,
            unison_spread: 0.0,
            unison_voices: 1,
            waveform: Waveform::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_accepts_valid_values() {
        let config = OscillatorConfig::new(-12.0, 0.5, 0.25, 4, Waveform::Saw)
            .expect("valid config should construct");

        assert_eq!(config.detune_cents(), -12.0);
        assert_eq!(config.pulse_width(), 0.5);
        assert_eq!(config.unison_spread(), 0.25);
        assert_eq!(config.unison_voices(), 4);
        assert_eq!(config.waveform(), Waveform::Saw);
    }

    #[test]
    fn new_rejects_zero_unison_voices() {
        let result = OscillatorConfig::new(0.0, 0.5, 0.0, 0, Waveform::Sine);
        assert_eq!(
            result,
            Err(OscillatorConfigError::UnisonVoicesOutOfRange(0))
        );
    }

    #[test]
    fn new_rejects_unison_voices_above_sixteen() {
        let result = OscillatorConfig::new(0.0, 0.5, 0.0, 17, Waveform::Sine);
        assert_eq!(
            result,
            Err(OscillatorConfigError::UnisonVoicesOutOfRange(17))
        );
    }

    #[test]
    fn new_accepts_boundary_unison_voices() {
        assert!(OscillatorConfig::new(0.0, 0.5, 0.0, 1, Waveform::Sine).is_ok());
        assert!(OscillatorConfig::new(0.0, 0.5, 0.0, 16, Waveform::Sine).is_ok());
    }

    #[test]
    fn new_rejects_pulse_width_below_zero() {
        let result = OscillatorConfig::new(0.0, -0.1, 0.0, 1, Waveform::Square);
        assert_eq!(
            result,
            Err(OscillatorConfigError::PulseWidthOutOfRange(-0.1))
        );
    }

    #[test]
    fn new_rejects_pulse_width_above_one() {
        let result = OscillatorConfig::new(0.0, 1.1, 0.0, 1, Waveform::Square);
        assert_eq!(
            result,
            Err(OscillatorConfigError::PulseWidthOutOfRange(1.1))
        );
    }

    #[test]
    fn new_rejects_nan_pulse_width() {
        let result = OscillatorConfig::new(0.0, f64::NAN, 0.0, 1, Waveform::Square);
        assert!(matches!(
            result,
            Err(OscillatorConfigError::PulseWidthOutOfRange(_))
        ));
    }

    #[test]
    fn new_accepts_boundary_pulse_width() {
        assert!(OscillatorConfig::new(0.0, 0.0, 0.0, 1, Waveform::Square).is_ok());
        assert!(OscillatorConfig::new(0.0, 1.0, 0.0, 1, Waveform::Square).is_ok());
    }

    #[test]
    fn default_is_valid() {
        let config = OscillatorConfig::default();
        assert_eq!(config.unison_voices(), 1);
        assert_eq!(config.pulse_width(), 0.5);
        assert_eq!(config.waveform(), Waveform::Sine);
    }

    #[test]
    fn with_pulse_width_rejects_invalid_value() {
        let config = OscillatorConfig::default();
        let result = config.with_pulse_width(2.0);
        assert_eq!(
            result,
            Err(OscillatorConfigError::PulseWidthOutOfRange(2.0))
        );
    }

    #[test]
    fn with_unison_voices_rejects_invalid_value() {
        let config = OscillatorConfig::default();
        let result = config.with_unison_voices(0);
        assert_eq!(
            result,
            Err(OscillatorConfigError::UnisonVoicesOutOfRange(0))
        );
    }

    #[test]
    fn with_methods_preserve_other_fields() {
        let config = OscillatorConfig::new(5.0, 0.4, 0.1, 2, Waveform::Triangle)
            .expect("valid config should construct");

        let updated = config
            .with_detune_cents(10.0)
            .with_unison_spread(0.9)
            .with_waveform(Waveform::Noise);

        assert_eq!(updated.detune_cents(), 10.0);
        assert_eq!(updated.unison_spread(), 0.9);
        assert_eq!(updated.waveform(), Waveform::Noise);
        // untouched fields survive the chained copies
        assert_eq!(updated.pulse_width(), 0.4);
        assert_eq!(updated.unison_voices(), 2);
    }
}
