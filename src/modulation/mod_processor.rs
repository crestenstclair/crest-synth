// path: src/modulation/mod_processor.rs

//! `ModProcessor`: a domain service that evaluates every modulation source,
//! applies every route through its curve and via-depth, and produces
//! per-sample parameter offsets.
//!
//! # Audio-thread safety
//!
//! `ModProcessor::process` performs no heap allocation, acquires no lock,
//! and does no I/O — it is safe to call from the audio callback once per
//! sample. It never owns the [`ModMatrix`] mutably and never mutates it;
//! all matrix mutation happens off the audio thread and crosses the
//! real-time boundary as a read-only snapshot handed in by the caller.
//! Output is written through the caller-supplied [`ModOffsetSink`], so
//! `ModProcessor` itself never allocates a result collection.

use crate::modulation::mod_matrix::{LfoConfig, ModMatrix, ModRoute, LFO_COUNT};

// ── ModCurve ─────────────────────────────────────────────────────────────

/// Shaping curve applied to a source's raw value before scaling by a
/// route's via-depth.
///
/// `ModRoute` (as published by [`ModMatrix`]) carries only a source,
/// destination and signed depth — curve shaping is not yet a published
/// dependency of this resource, so `ModCurve` is defined locally here per
/// the "define locally if not yet available" rule. It is a plain `Copy`
/// value, safe to hold in a fixed-size table alongside a route list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModCurve {
    /// Pass the source value through unchanged.
    #[default]
    Linear,
    /// Emphasize the extremes: `sign(x) * x^2`.
    Exponential,
    /// Emphasize small values: `sign(x) * sqrt(|x|)`.
    Logarithmic,
    /// Smoothstep-style ease at both ends, sign-preserving.
    SCurve,
    /// Flip the sign of the source value before scaling.
    Inverted,
}

impl ModCurve {
    /// Apply this curve to a raw source value, expected to already be
    /// normalized to `[-1.0, 1.0]`. The result stays within the same
    /// range for every variant. NaN input maps to `0.0`.
    pub fn apply(self, x: f64) -> f64 {
        if x.is_nan() {
            return 0.0;
        }
        let clamped = x.clamp(-1.0, 1.0);
        match self {
            ModCurve::Linear => clamped,
            ModCurve::Exponential => clamped.signum() * clamped * clamped,
            ModCurve::Logarithmic => clamped.signum() * clamped.abs().sqrt(),
            ModCurve::SCurve => {
                let a = clamped.abs();
                clamped.signum() * (a * a * (3.0 - 2.0 * a))
            }
            ModCurve::Inverted => -clamped,
        }
    }
}

// ── Source evaluation ───────────────────────────────────────────────────

/// Abstraction over "evaluate a modulation source's current value".
///
/// `ModProcessor` depends on this trait rather than a concrete source
/// implementation (LFO, envelope, MPE dimension, ...) — Dependency
/// Inversion keeps the processor testable in isolation and open to new
/// source kinds without modification (Open/Closed).
///
/// Implementations must be real-time safe: no heap allocation, no locks,
/// no I/O. `ModProcessor` uses a generic type parameter (static dispatch)
/// rather than `dyn ModSourceEvaluator` so the inner per-route loop never
/// pays for dynamic dispatch.
pub trait ModSourceEvaluator {
    /// Evaluate `source_id` at `sample_index`, returning a value in
    /// `[-1.0, 1.0]`. Unknown source ids return `0.0` rather than erroring,
    /// since silently contributing nothing is always safe for a mod
    /// source that does not (yet) exist.
    fn evaluate(&self, source_id: u32, sample_index: u64, sample_rate: f64) -> f64;
}

/// A [`ModSourceEvaluator`] backed by a `ModMatrix`'s four LFOs.
///
/// Route `source_id`s `0..LFO_COUNT` address the corresponding LFO slot;
/// any other id evaluates to `0.0`. Holds an owned, `Copy` snapshot of the
/// LFO configs (not a reference into `ModMatrix`) so it can be constructed
/// once off the audio thread and handed to the audio thread by value,
/// matching this project's snapshot-based real-time boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LfoSourceEvaluator {
    lfos: [LfoConfig; LFO_COUNT],
}

impl LfoSourceEvaluator {
    /// Snapshot the LFO configuration out of a `ModMatrix`.
    pub fn from_matrix(matrix: &ModMatrix) -> Self {
        Self {
            lfos: *matrix.lfos(),
        }
    }

    /// Construct directly from an LFO config array (mainly for tests and
    /// callers that already hold a snapshot).
    pub fn new(lfos: [LfoConfig; LFO_COUNT]) -> Self {
        Self { lfos }
    }
}

impl ModSourceEvaluator for LfoSourceEvaluator {
    fn evaluate(&self, source_id: u32, sample_index: u64, sample_rate: f64) -> f64 {
        let idx = source_id as usize;
        if idx >= LFO_COUNT || sample_rate <= 0.0 {
            return 0.0;
        }
        let lfo = self.lfos[idx];
        let t = sample_index as f64 / sample_rate;
        let phase = lfo.phase + std::f64::consts::TAU * lfo.rate_hz * t;
        phase.sin() * lfo.depth
    }
}

// ── Output ───────────────────────────────────────────────────────────────

/// Receives per-destination modulation offsets as `ModProcessor` computes
/// them, one route at a time.
///
/// `ModProcessor::process` never builds its own output collection — every
/// output value is pushed through this sink — so the audio thread never
/// allocates just to report a result. Callers own the sink's storage
/// (typically a fixed-size array via [`FixedOffsetBuffer`]).
pub trait ModOffsetSink {
    /// Accumulate `offset` into the running total for `destination_id`.
    fn add_offset(&mut self, destination_id: u32, offset: f64);
}

/// A fixed-capacity [`ModOffsetSink`] backed by a stack-allocated array,
/// indexed directly by destination id.
///
/// Destination ids `>= N` are silently dropped rather than panicking or
/// growing — real-time code must never allocate to accommodate an
/// out-of-range id.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedOffsetBuffer<const N: usize> {
    offsets: [f64; N],
}

impl<const N: usize> FixedOffsetBuffer<N> {
    /// A buffer with every destination's offset at zero.
    pub fn new() -> Self {
        Self { offsets: [0.0; N] }
    }

    /// The accumulated offset for `destination_id`, or `0.0` if it is out
    /// of range or received no contributions.
    pub fn get(&self, destination_id: u32) -> f64 {
        self.offsets
            .get(destination_id as usize)
            .copied()
            .unwrap_or(0.0)
    }

    /// Reset every offset back to zero, for reuse across samples without
    /// reallocating.
    pub fn clear(&mut self) {
        self.offsets = [0.0; N];
    }
}

impl<const N: usize> Default for FixedOffsetBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> ModOffsetSink for FixedOffsetBuffer<N> {
    fn add_offset(&mut self, destination_id: u32, offset: f64) {
        if let Some(slot) = self.offsets.get_mut(destination_id as usize) {
            *slot += offset;
        }
    }
}

// ── ModProcessor ─────────────────────────────────────────────────────────

/// Evaluates every modulation source, applies every route through its
/// curve and via-depth, and produces per-sample parameter offsets.
///
/// Generic over its source evaluator (Dependency Inversion + static
/// dispatch): `ModProcessor<S>` depends on the `ModSourceEvaluator`
/// abstraction, not a concrete LFO/envelope implementation, and pays no
/// dynamic-dispatch cost in the per-route inner loop.
///
/// # Audio-thread safety
///
/// `process` allocates no heap memory, takes no lock, performs no I/O.
/// Routes and curves are borrowed slices already owned by the caller; the
/// computed offsets are pushed into a caller-owned `ModOffsetSink`. The
/// `ModMatrix` itself is never mutated here and never crosses into the
/// audio thread as a live, mutable aggregate — only an already-constructed,
/// `Copy` source-evaluator snapshot does, per the ParameterBridge /
/// EventRing boundary convention used throughout this project.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModProcessor<S: ModSourceEvaluator> {
    source_evaluator: S,
}

impl<S: ModSourceEvaluator> ModProcessor<S> {
    /// Construct a processor over the given source evaluator. The
    /// dependency is injected via the constructor rather than created
    /// internally, so tests can supply a stub evaluator.
    pub fn new(source_evaluator: S) -> Self {
        Self { source_evaluator }
    }

    /// Evaluate every route in `routes` at `sample_index`, shape each
    /// source value through the route's curve (from `curves`, matched by
    /// index; missing entries default to [`ModCurve::Linear`]), scale by
    /// the route's via-depth, and push each contribution into `sink`.
    ///
    /// Multiple routes may target the same destination — their
    /// contributions are summed by the sink (layering is intentional,
    /// mirroring this project's MIDI-dispatch semantics: multiple matches
    /// are allowed, never silently merged into one).
    pub fn process(
        &self,
        routes: &[ModRoute],
        curves: &[ModCurve],
        sample_index: u64,
        sample_rate: f64,
        sink: &mut impl ModOffsetSink,
    ) {
        for (i, route) in routes.iter().enumerate() {
            let curve = curves.get(i).copied().unwrap_or_default();
            let raw = self
                .source_evaluator
                .evaluate(route.source_id, sample_index, sample_rate);
            let shaped = curve.apply(raw);
            let offset = shaped * route.depth;
            sink.add_offset(route.destination_id, offset);
        }
    }

    /// Convenience form of [`ModProcessor::process`] that reads routes
    /// directly from a `ModMatrix` snapshot and applies [`ModCurve::Linear`]
    /// to every route (no curve table supplied).
    pub fn process_matrix(
        &self,
        matrix: &ModMatrix,
        sample_index: u64,
        sample_rate: f64,
        sink: &mut impl ModOffsetSink,
    ) {
        self.process(matrix.routes(), &[], sample_index, sample_rate, sink);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modulation::mod_matrix::{ModMatrixCommand, ModRoute};

    const EPS: f64 = 1e-9;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 || (a - b).abs() <= EPS
    }

    // ── ModCurve ────────────────────────────────────────────────────────

    #[test]
    fn linear_curve_passes_value_through() {
        assert!(approx_eq(ModCurve::Linear.apply(0.5), 0.5));
        assert!(approx_eq(ModCurve::Linear.apply(-0.25), -0.25));
    }

    #[test]
    fn linear_curve_clamps_out_of_range_input() {
        assert!(approx_eq(ModCurve::Linear.apply(2.0), 1.0));
        assert!(approx_eq(ModCurve::Linear.apply(-5.0), -1.0));
    }

    #[test]
    fn exponential_curve_preserves_sign_and_shrinks_mid_values() {
        let v = ModCurve::Exponential.apply(0.5);
        assert!(v > 0.0 && v < 0.5);
        let v = ModCurve::Exponential.apply(-0.5);
        assert!(v < 0.0 && v > -0.5);
    }

    #[test]
    fn logarithmic_curve_preserves_sign_and_grows_mid_values() {
        let v = ModCurve::Logarithmic.apply(0.25);
        assert!(v > 0.25);
        let v = ModCurve::Logarithmic.apply(-0.25);
        assert!(v < -0.25);
    }

    #[test]
    fn s_curve_maps_extremes_to_themselves() {
        assert!(approx_eq(ModCurve::SCurve.apply(1.0), 1.0));
        assert!(approx_eq(ModCurve::SCurve.apply(-1.0), -1.0));
        assert!(approx_eq(ModCurve::SCurve.apply(0.0), 0.0));
    }

    #[test]
    fn inverted_curve_flips_sign() {
        assert!(approx_eq(ModCurve::Inverted.apply(0.3), -0.3));
        assert!(approx_eq(ModCurve::Inverted.apply(-0.3), 0.3));
    }

    #[test]
    fn curve_apply_maps_nan_to_zero() {
        assert!(approx_eq(ModCurve::Linear.apply(f64::NAN), 0.0));
        assert!(approx_eq(ModCurve::Exponential.apply(f64::NAN), 0.0));
    }

    #[test]
    fn default_curve_is_linear() {
        assert_eq!(ModCurve::default(), ModCurve::Linear);
    }

    // ── LfoSourceEvaluator ──────────────────────────────────────────────

    #[test]
    fn lfo_evaluator_at_zero_phase_and_sample_matches_sine_of_phase() {
        let cfg = LfoConfig::new(1.0, 1.0, 0.0);
        let mut lfos = [LfoConfig::default(); LFO_COUNT];
        lfos[0] = cfg;
        let eval = LfoSourceEvaluator::new(lfos);
        // sample_index = 0 => t = 0 => phase = 0 => sin(0) * depth = 0
        assert!(approx_eq(eval.evaluate(0, 0, 48_000.0), 0.0));
    }

    #[test]
    fn lfo_evaluator_scales_by_depth() {
        let cfg = LfoConfig::new(1.0, 0.5, std::f64::consts::FRAC_PI_2);
        let mut lfos = [LfoConfig::default(); LFO_COUNT];
        lfos[0] = cfg;
        let eval = LfoSourceEvaluator::new(lfos);
        // phase = pi/2 => sin = 1.0 => value = depth
        let v = eval.evaluate(0, 0, 48_000.0);
        assert!(approx_eq(v, 0.5));
    }

    #[test]
    fn lfo_evaluator_unknown_source_id_is_zero() {
        let eval = LfoSourceEvaluator::new([LfoConfig::default(); LFO_COUNT]);
        assert!(approx_eq(
            eval.evaluate(LFO_COUNT as u32, 10, 48_000.0),
            0.0
        ));
        assert!(approx_eq(eval.evaluate(999, 10, 48_000.0), 0.0));
    }

    #[test]
    fn lfo_evaluator_non_positive_sample_rate_is_zero() {
        let eval = LfoSourceEvaluator::new([LfoConfig::new(1.0, 1.0, 0.0); LFO_COUNT]);
        assert!(approx_eq(eval.evaluate(0, 5, 0.0), 0.0));
        assert!(approx_eq(eval.evaluate(0, 5, -100.0), 0.0));
    }

    #[test]
    fn lfo_evaluator_from_matrix_snapshots_configured_lfos() {
        let mut matrix = ModMatrix::new(4);
        let cfg = LfoConfig::new(2.0, 0.25, 0.0);
        matrix
            .apply(ModMatrixCommand::SetLfo {
                index: 1,
                config: cfg,
            })
            .unwrap();
        let eval = LfoSourceEvaluator::from_matrix(&matrix);
        assert_eq!(eval.lfos[1], cfg);
    }

    // ── FixedOffsetBuffer ───────────────────────────────────────────────

    #[test]
    fn fixed_offset_buffer_starts_at_zero() {
        let buf: FixedOffsetBuffer<4> = FixedOffsetBuffer::new();
        assert!(approx_eq(buf.get(0), 0.0));
        assert!(approx_eq(buf.get(3), 0.0));
    }

    #[test]
    fn fixed_offset_buffer_accumulates_multiple_adds() {
        let mut buf: FixedOffsetBuffer<4> = FixedOffsetBuffer::new();
        buf.add_offset(2, 0.5);
        buf.add_offset(2, 0.25);
        assert!(approx_eq(buf.get(2), 0.75));
    }

    #[test]
    fn fixed_offset_buffer_ignores_out_of_range_destination() {
        let mut buf: FixedOffsetBuffer<2> = FixedOffsetBuffer::new();
        buf.add_offset(50, 1.0);
        assert!(approx_eq(buf.get(50), 0.0));
    }

    #[test]
    fn fixed_offset_buffer_clear_resets_all_slots() {
        let mut buf: FixedOffsetBuffer<3> = FixedOffsetBuffer::new();
        buf.add_offset(0, 1.0);
        buf.add_offset(1, 2.0);
        buf.clear();
        assert!(approx_eq(buf.get(0), 0.0));
        assert!(approx_eq(buf.get(1), 0.0));
    }

    // ── ModProcessor ────────────────────────────────────────────────────

    struct ConstEvaluator(f64);

    impl ModSourceEvaluator for ConstEvaluator {
        fn evaluate(&self, _source_id: u32, _sample_index: u64, _sample_rate: f64) -> f64 {
            self.0
        }
    }

    #[test]
    fn process_applies_linear_default_curve_and_depth() {
        let processor = ModProcessor::new(ConstEvaluator(1.0));
        let routes = [ModRoute::new(0, 7, 0.5)];
        let mut sink: FixedOffsetBuffer<16> = FixedOffsetBuffer::new();
        processor.process(&routes, &[], 0, 48_000.0, &mut sink);
        assert!(approx_eq(sink.get(7), 0.5));
    }

    #[test]
    fn process_applies_curve_before_depth() {
        let processor = ModProcessor::new(ConstEvaluator(0.5));
        let routes = [ModRoute::new(0, 1, 2.0)];
        let curves = [ModCurve::Inverted];
        let mut sink: FixedOffsetBuffer<8> = FixedOffsetBuffer::new();
        processor.process(&routes, &curves, 0, 48_000.0, &mut sink);
        // inverted(0.5) = -0.5, * depth 2.0 = -1.0
        assert!(approx_eq(sink.get(1), -1.0));
    }

    #[test]
    fn process_missing_curve_entry_defaults_to_linear() {
        let processor = ModProcessor::new(ConstEvaluator(0.4));
        let routes = [ModRoute::new(0, 3, 1.0), ModRoute::new(0, 3, 1.0)];
        // Only one curve supplied for two routes.
        let curves = [ModCurve::Linear];
        let mut sink: FixedOffsetBuffer<8> = FixedOffsetBuffer::new();
        processor.process(&routes, &curves, 0, 48_000.0, &mut sink);
        assert!(approx_eq(sink.get(3), 0.8));
    }

    #[test]
    fn process_sums_multiple_routes_to_same_destination() {
        let processor = ModProcessor::new(ConstEvaluator(1.0));
        let routes = [ModRoute::new(0, 9, 0.2), ModRoute::new(1, 9, 0.3)];
        let mut sink: FixedOffsetBuffer<16> = FixedOffsetBuffer::new();
        processor.process(&routes, &[], 0, 48_000.0, &mut sink);
        assert!(approx_eq(sink.get(9), 0.5));
    }

    #[test]
    fn process_keeps_distinct_destinations_separate() {
        let processor = ModProcessor::new(ConstEvaluator(1.0));
        let routes = [ModRoute::new(0, 1, 0.5), ModRoute::new(0, 2, 0.25)];
        let mut sink: FixedOffsetBuffer<8> = FixedOffsetBuffer::new();
        processor.process(&routes, &[], 0, 48_000.0, &mut sink);
        assert!(approx_eq(sink.get(1), 0.5));
        assert!(approx_eq(sink.get(2), 0.25));
    }

    #[test]
    fn process_empty_routes_leaves_sink_untouched() {
        let processor = ModProcessor::new(ConstEvaluator(1.0));
        let mut sink: FixedOffsetBuffer<4> = FixedOffsetBuffer::new();
        processor.process(&[], &[], 0, 48_000.0, &mut sink);
        assert!(approx_eq(sink.get(0), 0.0));
    }

    #[test]
    fn process_matrix_reads_routes_from_matrix_snapshot() {
        let processor = ModProcessor::new(ConstEvaluator(1.0));
        let mut matrix = ModMatrix::new(4);
        matrix
            .apply(ModMatrixCommand::AddRoute {
                route: ModRoute::new(0, 5, 0.6),
            })
            .unwrap();
        let mut sink: FixedOffsetBuffer<8> = FixedOffsetBuffer::new();
        processor.process_matrix(&matrix, 0, 48_000.0, &mut sink);
        assert!(approx_eq(sink.get(5), 0.6));
    }

    #[test]
    fn process_with_lfo_evaluator_end_to_end() {
        let cfg = LfoConfig::new(1.0, 1.0, std::f64::consts::FRAC_PI_2);
        let mut lfos = [LfoConfig::default(); LFO_COUNT];
        lfos[0] = cfg;
        let processor = ModProcessor::new(LfoSourceEvaluator::new(lfos));
        let routes = [ModRoute::new(0, 4, 1.0)];
        let mut sink: FixedOffsetBuffer<8> = FixedOffsetBuffer::new();
        processor.process(&routes, &[], 0, 48_000.0, &mut sink);
        // sin(pi/2) * depth(1.0) * route depth(1.0) == 1.0
        assert!(approx_eq(sink.get(4), 1.0));
    }
}
