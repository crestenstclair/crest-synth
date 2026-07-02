// path: src/bin/tone_test.rs

//! Renders one second of A440 through the engine's `Oscillator` port and
//! measures the peak absolute sample value of the rendered buffer.
//!
//! Prints exactly one line, `peak=<value>`, then exits non-zero unless the
//! measured peak strictly exceeds 0.1 and is at most 1.0 -- a silent render
//! (peak too low) and a clipping render (peak too high) both fail the run.

use crest_synth::engine::oscillator::{
    Amplitude, Frequency, Oscillator, OscillatorConfig, SampleRate, StandardOscillator, Waveform,
};

const A440_HERTZ: f64 = 440.0;
const RENDER_SAMPLE_RATE_HZ: f64 = 44_100.0;
const RENDER_SECONDS: f64 = 1.0;
const MIN_AUDIBLE_PEAK: f64 = 0.1;
const MAX_PEAK: f64 = 1.0;

/// Renders `seconds` worth of audio at `sample_rate`, playing a single note
/// at `frequency` through `oscillator` shaped by `config`. Returns the
/// rendered samples in playback order.
///
/// `oscillator` is taken as `&impl Oscillator` (static dispatch) rather than
/// `&dyn Oscillator`, matching the port's guidance to avoid dynamic dispatch
/// in the inner sample loop.
fn render_note(
    oscillator: &impl Oscillator,
    frequency: Frequency,
    sample_rate: SampleRate,
    config: OscillatorConfig,
    seconds: f64,
) -> Vec<f64> {
    let sample_count = (sample_rate.samples_per_second() * seconds).round() as usize;
    let mut phase = 0.0_f64;
    let mut buffer = Vec::with_capacity(sample_count);

    for _ in 0..sample_count {
        buffer.push(oscillator.render(phase, config));
        phase = oscillator.advance(phase, frequency, sample_rate);
    }

    buffer
}

/// Returns the largest absolute sample value in `buffer`, or `0.0` for an
/// empty buffer.
fn peak_absolute(buffer: &[f64]) -> f64 {
    buffer
        .iter()
        .fold(0.0_f64, |max, &sample| max.max(sample.abs()))
}

fn main() {
    let frequency = Frequency::try_new(A440_HERTZ).expect("A440 is a valid frequency");
    let sample_rate =
        SampleRate::try_new(RENDER_SAMPLE_RATE_HZ).expect("44.1kHz is a valid sample rate");
    let amplitude = Amplitude::try_new(1.0).expect("1.0 is a valid amplitude");
    let config = OscillatorConfig::new(Waveform::Sine, amplitude);
    let oscillator = StandardOscillator::new();

    let buffer = render_note(&oscillator, frequency, sample_rate, config, RENDER_SECONDS);
    let peak = peak_absolute(&buffer);

    println!("peak={peak}");

    if !(peak > MIN_AUDIBLE_PEAK && peak <= MAX_PEAK) {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a440_config() -> (Frequency, SampleRate, OscillatorConfig) {
        (
            Frequency::try_new(A440_HERTZ).unwrap(),
            SampleRate::try_new(RENDER_SAMPLE_RATE_HZ).unwrap(),
            OscillatorConfig::new(Waveform::Sine, Amplitude::try_new(1.0).unwrap()),
        )
    }

    #[test]
    fn render_note_produces_one_second_of_samples() {
        let oscillator = StandardOscillator::new();
        let (frequency, sample_rate, config) = a440_config();

        let buffer = render_note(&oscillator, frequency, sample_rate, config, RENDER_SECONDS);

        assert_eq!(buffer.len(), RENDER_SAMPLE_RATE_HZ as usize);
    }

    #[test]
    fn render_note_a440_peak_is_audible_and_not_clipping() {
        let oscillator = StandardOscillator::new();
        let (frequency, sample_rate, config) = a440_config();

        let buffer = render_note(&oscillator, frequency, sample_rate, config, RENDER_SECONDS);
        let peak = peak_absolute(&buffer);

        assert!(peak > MIN_AUDIBLE_PEAK);
        assert!(peak <= MAX_PEAK);
    }

    #[test]
    fn peak_absolute_finds_largest_magnitude_regardless_of_sign() {
        let buffer = vec![0.1, -0.9, 0.5, -0.2];
        assert!((peak_absolute(&buffer) - 0.9).abs() < 1e-12);
    }

    #[test]
    fn peak_absolute_of_empty_buffer_is_zero() {
        let buffer: Vec<f64> = Vec::new();
        assert_eq!(peak_absolute(&buffer), 0.0);
    }
}
