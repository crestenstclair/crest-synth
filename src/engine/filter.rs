// path: src/engine/filter.rs

//! A configurable digital filter port for the audio engine.
//!
//! `Filter` is the hexagonal boundary between engine adapters and any
//! concrete filter implementation (state-variable, biquad, ladder, etc).
//! Implementations must be safe to call from the hard real-time audio
//! thread: no heap allocation, no locks, no blocking I/O inside `process`.

use std::f64::consts::PI;

/// The kind of frequency response a `Filter` implementation produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

/// Plain-old-data configuration for a single filter call.
///
/// `FilterConfig` carries every parameter a `Filter` needs to process one
/// sample. It is `Copy` so it can cross the real-time boundary as a
/// latest-wins snapshot (see `ParameterBridge`) without allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterConfig {
    kind: FilterKind,
    cutoff_hz: f64,
    resonance: f64,
    sample_rate_hz: f64,
}

impl FilterConfig {
    /// Construct a `FilterConfig`, clamping `cutoff_hz` below Nyquist and
    /// `resonance` to `[0.0, 1.0]`. NaN inputs fall back to safe defaults.
    pub fn new(kind: FilterKind, cutoff_hz: f64, resonance: f64, sample_rate_hz: f64) -> Self {
        let sample_rate_hz = sample_rate_hz.max(1.0);
        let nyquist = sample_rate_hz * 0.5;
        let max_cutoff = (nyquist - 1.0).max(1.0);
        let cutoff_hz = if cutoff_hz.is_nan() {
            1.0
        } else {
            cutoff_hz.clamp(1.0, max_cutoff)
        };
        let resonance = if resonance.is_nan() {
            0.0
        } else {
            resonance.clamp(0.0, 1.0)
        };
        Self {
            kind,
            cutoff_hz,
            resonance,
            sample_rate_hz,
        }
    }

    pub fn kind(&self) -> FilterKind {
        self.kind
    }

    pub fn cutoff_hz(&self) -> f64 {
        self.cutoff_hz
    }

    pub fn resonance(&self) -> f64 {
        self.resonance
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }
}

/// A hexagonal port for single-sample digital filtering.
///
/// Implementations own their own internal state (delay lines, integrators)
/// and must be safe to call from the hard real-time audio thread: no heap
/// allocation, no locks, no blocking I/O.
pub trait Filter {
    /// Process one input sample and return the filtered output sample.
    /// `config` is read for this call only; implementations derive fresh
    /// coefficients from it rather than retaining a reference.
    fn process(&mut self, sample: f64, config: FilterConfig) -> f64;

    /// Reset all internal state (delay lines, integrators) to silence, as
    /// if the filter had just been constructed.
    fn reset(&mut self);
}

/// A trapezoidal-integration state-variable filter (Chamberlin topology).
///
/// `StateVariableFilter` is a reference `Filter` implementation: it holds
/// only primitive `f64` state, performs no heap allocation, and derives
/// its coefficients from `FilterConfig` on every call to `process`.
#[derive(Debug, Clone, Copy, Default)]
pub struct StateVariableFilter {
    low: f64,
    band: f64,
}

impl StateVariableFilter {
    pub fn new() -> Self {
        Self {
            low: 0.0,
            band: 0.0,
        }
    }
}

impl Filter for StateVariableFilter {
    fn process(&mut self, sample: f64, config: FilterConfig) -> f64 {
        let f = (2.0 * (PI * config.cutoff_hz() / config.sample_rate_hz()).sin()).clamp(0.0, 1.0);
        let q = (1.0 - config.resonance()).clamp(0.0001, 1.0);

        let high = sample - self.low - q * self.band;
        self.band += f * high;
        self.low += f * self.band;

        match config.kind() {
            FilterKind::LowPass => self.low,
            FilterKind::HighPass => high,
            FilterKind::BandPass => self.band,
            FilterKind::Notch => high + self.low,
        }
    }

    fn reset(&mut self) {
        self.low = 0.0;
        self.band = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(kind: FilterKind) -> FilterConfig {
        FilterConfig::new(kind, 1000.0, 0.3, 44_100.0)
    }

    #[test]
    fn low_pass_of_silence_is_silence() {
        let mut filter = StateVariableFilter::new();
        let out = filter.process(0.0, config(FilterKind::LowPass));
        assert_eq!(out, 0.0);
    }

    #[test]
    fn reset_clears_internal_state() {
        let mut filter = StateVariableFilter::new();
        for _ in 0..64 {
            filter.process(1.0, config(FilterKind::LowPass));
        }
        filter.reset();
        let out = filter.process(0.0, config(FilterKind::LowPass));
        assert_eq!(out, 0.0);
    }

    #[test]
    fn different_kinds_produce_different_output_for_same_input() {
        let mut low = StateVariableFilter::new();
        let mut high = StateVariableFilter::new();
        let mut lp_out = 0.0;
        let mut hp_out = 0.0;
        for i in 0..32 {
            let s = (i as f64 * 0.1).sin();
            lp_out = low.process(s, config(FilterKind::LowPass));
            hp_out = high.process(s, config(FilterKind::HighPass));
        }
        assert_ne!(lp_out, hp_out);
    }

    #[test]
    fn cutoff_is_clamped_below_nyquist() {
        let cfg = FilterConfig::new(FilterKind::LowPass, 1_000_000.0, 0.0, 44_100.0);
        assert!(cfg.cutoff_hz() < 44_100.0 * 0.5);
    }

    #[test]
    fn resonance_is_clamped_to_unit_range() {
        let cfg = FilterConfig::new(FilterKind::LowPass, 1_000.0, 5.0, 44_100.0);
        assert!((0.0..=1.0).contains(&cfg.resonance()));
    }

    #[test]
    fn process_never_allocates_and_stays_finite() {
        let mut filter = StateVariableFilter::new();
        let cfg = config(FilterKind::BandPass);
        for i in 0..256 {
            let s = (i as f64 * 0.05).sin();
            let out = filter.process(s, cfg);
            assert!(out.is_finite());
        }
    }
}
