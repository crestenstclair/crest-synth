//! The `Oscillator` port: a hexagonal-architecture boundary describing how a
//! phase accumulator is advanced and how a waveform sample is rendered from
//! it. Implementations of this port live entirely on the real-time audio
//! thread: no heap allocation, no locks, no I/O — every method here is pure
//! arithmetic over `Copy` value types.

use std::error::Error;
use std::fmt;

/// A validated oscillation frequency, expressed in hertz.
///
/// Must be finite and strictly positive; zero or negative frequencies do not
/// correspond to a physical oscillation and negative values would reverse
/// phase direction in a way callers do not expect.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Frequency(f64);

impl Frequency {
    /// Constructs a `Frequency`, rejecting non-finite or non-positive values.
    pub fn try_new(hertz: f64) -> Result<Self, OscillatorError> {
        if hertz.is_nan() || !(f64::MIN_POSITIVE..=f64::MAX).contains(&hertz) {
            return Err(OscillatorError::InvalidFrequency(hertz));
        }
        Ok(Self(hertz))
    }

    /// The frequency in hertz.
    pub fn hertz(&self) -> f64 {
        self.0
    }
}

/// A validated audio sample rate, expressed in samples per second.
///
/// Must be finite and strictly positive; a zero or negative sample rate
/// makes the notion of "samples per second" meaningless and would divide by
/// zero (or invert time) in phase-advance arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SampleRate(f64);

impl SampleRate {
    /// Constructs a `SampleRate`, rejecting non-finite or non-positive values.
    pub fn try_new(samples_per_second: f64) -> Result<Self, OscillatorError> {
        if samples_per_second.is_nan()
            || !(f64::MIN_POSITIVE..=f64::MAX).contains(&samples_per_second)
        {
            return Err(OscillatorError::InvalidSampleRate(samples_per_second));
        }
        Ok(Self(samples_per_second))
    }

    /// The sample rate in samples per second.
    pub fn samples_per_second(&self) -> f64 {
        self.0
    }
}

/// A validated linear amplitude scalar in `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Amplitude(f64);

impl Amplitude {
    /// Constructs an `Amplitude`, rejecting values outside `[0.0, 1.0]` or
    /// non-finite values.
    pub fn try_new(value: f64) -> Result<Self, OscillatorError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(OscillatorError::InvalidAmplitude(value));
        }
        Ok(Self(value))
    }

    /// The amplitude as a linear scalar.
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for Amplitude {
    fn default() -> Self {
        Self(1.0)
    }
}

/// The waveform shape a renderer samples from at a given phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Waveform {
    #[default]
    Sine,
    Saw,
    Square,
    Triangle,
}

/// Configuration consumed by [`Oscillator::render`]: which waveform shape to
/// sample and at what linear amplitude.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OscillatorConfig {
    pub waveform: Waveform,
    pub amplitude: Amplitude,
}

impl OscillatorConfig {
    /// Constructs a config from a waveform and amplitude.
    pub fn new(waveform: Waveform, amplitude: Amplitude) -> Self {
        Self {
            waveform,
            amplitude,
        }
    }
}

/// Errors produced while validating oscillator inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OscillatorError {
    InvalidFrequency(f64),
    InvalidSampleRate(f64),
    InvalidAmplitude(f64),
}

impl fmt::Display for OscillatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OscillatorError::InvalidFrequency(hz) => {
                write!(f, "invalid frequency: {hz} (must be finite and > 0)")
            }
            OscillatorError::InvalidSampleRate(sr) => {
                write!(f, "invalid sample rate: {sr} (must be finite and > 0)")
            }
            OscillatorError::InvalidAmplitude(a) => {
                write!(
                    f,
                    "invalid amplitude: {a} (must be finite and within [0.0, 1.0])"
                )
            }
        }
    }
}

impl Error for OscillatorError {}

/// The `Oscillator` port: advances a phase accumulator and renders a
/// waveform sample from a phase.
///
/// Implementations must be usable from the real-time audio thread: pure,
/// non-allocating, non-blocking arithmetic only. Callers on the inner sample
/// loop should hold a concrete, statically-dispatched implementation (e.g.
/// via a generic parameter) rather than a `dyn Oscillator` trait object, to
/// avoid dynamic dispatch in the hot path.
pub trait Oscillator {
    /// Advances `phase` by one sample at `frequency` and `sample_rate`,
    /// wrapping the result into `[0.0, 1.0)`.
    fn advance(&self, phase: f64, frequency: Frequency, sample_rate: SampleRate) -> f64;

    /// Renders a waveform sample at `phase` according to `config`.
    ///
    /// `phase` is expected to lie in `[0.0, 1.0)`; callers are responsible
    /// for keeping phase in range (typically via repeated calls to
    /// [`Oscillator::advance`]).
    fn render(&self, phase: f64, config: OscillatorConfig) -> f64;
}

/// The standard, stateless oscillator implementation: a phase accumulator
/// advanced by `frequency / sample_rate` per sample, rendered through one of
/// the four canonical waveform shapes.
#[derive(Debug, Clone, Copy, Default)]
pub struct StandardOscillator;

impl StandardOscillator {
    /// Constructs a `StandardOscillator`. Stateless, so this never fails and
    /// never allocates.
    pub fn new() -> Self {
        Self
    }
}

impl Oscillator for StandardOscillator {
    fn advance(&self, phase: f64, frequency: Frequency, sample_rate: SampleRate) -> f64 {
        let increment = frequency.hertz() / sample_rate.samples_per_second();
        (phase + increment).rem_euclid(1.0)
    }

    fn render(&self, phase: f64, config: OscillatorConfig) -> f64 {
        let wrapped = phase.rem_euclid(1.0);
        let raw = match config.waveform {
            Waveform::Sine => (wrapped * std::f64::consts::TAU).sin(),
            Waveform::Saw => 2.0 * wrapped - 1.0,
            Waveform::Square => {
                if wrapped < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Triangle => {
                // Shift phase by a quarter period so the triangle's zero
                // crossings and peaks line up with the sine's: f(0.0) = 0.0,
                // f(0.25) = 1.0, f(0.5) = 0.0, f(0.75) = -1.0.
                let shifted = (wrapped + 0.25).rem_euclid(1.0);
                4.0 * (shifted - (shifted + 0.5).floor()).abs() - 1.0
            }
        };
        raw * config.amplitude.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_rejects_zero_negative_and_non_finite() {
        assert!(Frequency::try_new(0.0).is_err());
        assert!(Frequency::try_new(-440.0).is_err());
        assert!(Frequency::try_new(f64::NAN).is_err());
        assert!(Frequency::try_new(f64::INFINITY).is_err());
        assert!(Frequency::try_new(440.0).is_ok());
    }

    #[test]
    fn sample_rate_rejects_zero_negative_and_non_finite() {
        assert!(SampleRate::try_new(0.0).is_err());
        assert!(SampleRate::try_new(-48000.0).is_err());
        assert!(SampleRate::try_new(f64::NAN).is_err());
        assert!(SampleRate::try_new(44_100.0).is_ok());
    }

    #[test]
    fn amplitude_rejects_out_of_range_and_non_finite() {
        assert!(Amplitude::try_new(-0.1).is_err());
        assert!(Amplitude::try_new(1.1).is_err());
        assert!(Amplitude::try_new(f64::NAN).is_err());
        assert!(Amplitude::try_new(0.0).is_ok());
        assert!(Amplitude::try_new(1.0).is_ok());
    }

    #[test]
    fn advance_moves_phase_by_frequency_over_sample_rate() {
        let osc = StandardOscillator::new();
        let frequency = Frequency::try_new(100.0).unwrap();
        let sample_rate = SampleRate::try_new(1000.0).unwrap();

        let next = osc.advance(0.0, frequency, sample_rate);
        assert!((next - 0.1).abs() < 1e-12);
    }

    #[test]
    fn advance_wraps_phase_into_unit_range() {
        let osc = StandardOscillator::new();
        let frequency = Frequency::try_new(100.0).unwrap();
        let sample_rate = SampleRate::try_new(1000.0).unwrap();

        let next = osc.advance(0.95, frequency, sample_rate);
        assert!((next - 0.05).abs() < 1e-9);
        assert!((0.0..1.0).contains(&next));
    }

    #[test]
    fn render_sine_matches_known_points() {
        let osc = StandardOscillator::new();
        let config = OscillatorConfig::new(Waveform::Sine, Amplitude::try_new(1.0).unwrap());

        assert!((osc.render(0.0, config) - 0.0).abs() < 1e-9);
        assert!((osc.render(0.25, config) - 1.0).abs() < 1e-9);
        assert!((osc.render(0.75, config) - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn render_saw_matches_known_points() {
        let osc = StandardOscillator::new();
        let config = OscillatorConfig::new(Waveform::Saw, Amplitude::try_new(1.0).unwrap());

        assert!((osc.render(0.0, config) - (-1.0)).abs() < 1e-9);
        assert!((osc.render(0.5, config) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn render_square_matches_known_points() {
        let osc = StandardOscillator::new();
        let config = OscillatorConfig::new(Waveform::Square, Amplitude::try_new(1.0).unwrap());

        assert!((osc.render(0.0, config) - 1.0).abs() < 1e-9);
        assert!((osc.render(0.5, config) - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn render_triangle_matches_known_points() {
        let osc = StandardOscillator::new();
        let config = OscillatorConfig::new(Waveform::Triangle, Amplitude::try_new(1.0).unwrap());

        assert!((osc.render(0.0, config) - 0.0).abs() < 1e-9);
        assert!((osc.render(0.25, config) - 1.0).abs() < 1e-9);
        assert!((osc.render(0.75, config) - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn render_scales_by_amplitude() {
        let osc = StandardOscillator::new();
        let full = OscillatorConfig::new(Waveform::Sine, Amplitude::try_new(1.0).unwrap());
        let half = OscillatorConfig::new(Waveform::Sine, Amplitude::try_new(0.5).unwrap());

        let full_sample = osc.render(0.25, full);
        let half_sample = osc.render(0.25, half);
        assert!((half_sample - full_sample * 0.5).abs() < 1e-9);
    }
}
