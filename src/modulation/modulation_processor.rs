// path: src/modulation/modulation_processor.rs
//
// Domain service: evaluates all mod sources and applies routed modulation to
// destination parameters each audio block.
//
// # Audio-thread guarantees
//
// `ModulationProcessor` is designed to be used on the audio thread:
// - No heap allocation during `process`.
// - No mutex or blocking lock acquired.
// - No blocking I/O.
//
// All mutable state (LFO phases, envelope stages) is kept in fixed-capacity
// arrays that are sized at construction time and never reallocated.
//
// # Per-note vs per-patch sources
//
// Per-note expression dimensions (PerNoteBendX, PerNoteTimbreY,
// PerNotePressureZ) are *not* processed here at the patch level — they must be
// delivered directly to each voice by the engine layer so that MPE works
// correctly. This service processes only patch-level sources (LFOs, mod
// envelopes, velocity, key-track, random, macro).

use crate::modulation::lfo_config::LfoConfig;
use crate::modulation::lfo_waveform::LfoWaveform;
use crate::modulation::mod_destination_type::ModDestinationType;
use crate::modulation::mod_matrix::ModMatrix;
use crate::modulation::mod_routing::ModRouting;
use crate::modulation::mod_source_type::ModSourceType;

// ── Constants ──────────────────────────────────────────────

/// Maximum number of LFO slots processed per block.
const MAX_LFOS: usize = 8;

/// Maximum number of mod envelope slots processed per block.
const MAX_MOD_ENVELOPES: usize = 8;

/// Maximum number of routings that can be evaluated per block.
const MAX_ROUTINGS: usize = 64;

// ── LFO runtime state ───────────────────────────────────────────────

/// Per-LFO runtime state: current phase in radians.
///
/// Advance by `2π * rate / sample_rate` each sample. On the audio thread we
/// only advance once per block using a block-averaged phase advance.
#[derive(Debug, Clone, Copy)]
struct LfoState {
    /// Current phase in [0.0, 2π).
    phase: f64,
}

impl LfoState {
    fn new(initial_phase: f64) -> Self {
        Self {
            phase: initial_phase,
        }
    }

    /// Advance the phase by `delta` radians (wrapping in [0, 2π)).
    #[inline]
    fn advance(&mut self, delta: f64) {
        self.phase = (self.phase + delta) % (2.0 * std::f64::consts::PI);
    }

    /// Sample the current phase using the given waveform.
    ///
    /// Returns a value in `[-1.0, 1.0]` (bipolar).
    #[inline]
    fn sample(&self, waveform: LfoWaveform) -> f64 {
        let t = self.phase / (2.0 * std::f64::consts::PI); // 0.0 .. 1.0
        match waveform {
            LfoWaveform::Sine => (self.phase).sin(),
            LfoWaveform::Triangle => {
                if t < 0.5 {
                    4.0 * t - 1.0
                } else {
                    3.0 - 4.0 * t
                }
            }
            LfoWaveform::Square => {
                if t < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            LfoWaveform::Sawtooth => 2.0 * t - 1.0,
            LfoWaveform::ReverseSawtooth => 1.0 - 2.0 * t,
            LfoWaveform::SampleAndHold => {
                // Sample-and-hold: resampled each full cycle, approximated as
                // a stable value derived from the current phase bucket.
                // On the audio thread we cannot call rand(), so we use a
                // deterministic hash of the integer cycle count as a stand-in.
                // Real S&H would be triggered from the wrapping event; this is
                // a placeholder that is deterministic and allocation-free.
                let cycle = (self.phase / (2.0 * std::f64::consts::PI)) as u64;
                // Cheap bit-mixing (xorshift64) as a deterministic pseudo-random.
                let mut x = cycle.wrapping_add(1).wrapping_mul(0x517c_c1b7_2722_0a95);
                x ^= x >> 30;
                x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
                x ^= x >> 27;
                x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
                x ^= x >> 31;
                // Map to [-1.0, 1.0]
                (x as i64 as f64) / (i64::MAX as f64)
            }
        }
    }
}

// ── Mod envelope stage ───────────────────────────────────────────────

/// The stage an envelope is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModEnvStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Runtime state for one mod envelope.
#[derive(Debug, Clone, Copy)]
struct ModEnvState {
    stage: ModEnvStage,
    /// Current output level in [0.0, 1.0].
    level: f64,
    /// Elapsed time in current stage (seconds).
    elapsed: f64,
}

impl ModEnvState {
    const fn idle() -> Self {
        Self {
            stage: ModEnvStage::Idle,
            level: 0.0,
            elapsed: 0.0,
        }
    }

    /// Trigger the envelope (note-on).
    fn trigger(&mut self) {
        self.stage = ModEnvStage::Attack;
        self.elapsed = 0.0;
        // Keep level for legato — or reset to 0 for non-legato.
    }

    /// Begin release (note-off).
    fn release(&mut self) {
        if self.stage != ModEnvStage::Idle {
            self.stage = ModEnvStage::Release;
            self.elapsed = 0.0;
        }
    }

    /// Advance envelope by `block_seconds` at sample rate `sample_rate`.
    ///
    /// Uses linear segments — no heap allocation.
    #[inline]
    fn advance(&mut self, block_seconds: f64, attack: f64, decay: f64, sustain: f64, release: f64) {
        match self.stage {
            ModEnvStage::Idle => {}
            ModEnvStage::Attack => {
                self.elapsed += block_seconds;
                let a = if attack > 0.0 { attack } else { 1e-5 };
                self.level = (self.elapsed / a).min(1.0);
                if self.level >= 1.0 {
                    self.stage = ModEnvStage::Decay;
                    self.elapsed = 0.0;
                }
            }
            ModEnvStage::Decay => {
                self.elapsed += block_seconds;
                let d = if decay > 0.0 { decay } else { 1e-5 };
                self.level = 1.0 - (1.0 - sustain) * (self.elapsed / d).min(1.0);
                if self.elapsed >= d {
                    self.level = sustain;
                    self.stage = ModEnvStage::Sustain;
                }
            }
            ModEnvStage::Sustain => {
                self.level = sustain;
            }
            ModEnvStage::Release => {
                self.elapsed += block_seconds;
                let r = if release > 0.0 { release } else { 1e-5 };
                let start_level = self.level;
                self.level = (start_level * (1.0 - (self.elapsed / r).min(1.0))).max(0.0);
                if self.elapsed >= r {
                    self.level = 0.0;
                    self.stage = ModEnvStage::Idle;
                }
            }
        }
    }

    fn current_level(self) -> f64 {
        self.level
    }
}

// ── ModulationValues ────────────────────────────────────────────────

/// The computed modulation contribution for each destination, summed across all
/// active routings for a single block.
///
/// All values are in `[-1.0, 1.0]` (bipolar). The synth engine multiplies these
/// by the base parameter value to produce the final modulated parameter.
///
/// This is a pure stack value — no heap allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModulationValues {
    /// Summed mod to oscillator pitch (semitones scaling factor).
    pub oscillator_pitch: f64,
    /// Summed mod to oscillator fine-tune.
    pub oscillator_fine_tune: f64,
    /// Summed mod to oscillator shape/pulse-width.
    pub oscillator_shape: f64,
    /// Summed mod to filter cutoff.
    pub filter_cutoff: f64,
    /// Summed mod to filter resonance.
    pub filter_resonance: f64,
    /// Summed mod to amplitude.
    pub amplitude: f64,
    /// Summed mod to panning.
    pub pan: f64,
    /// Summed mod to envelope attack.
    pub envelope_attack: f64,
    /// Summed mod to envelope decay.
    pub envelope_decay: f64,
    /// Summed mod to envelope sustain.
    pub envelope_sustain: f64,
    /// Summed mod to envelope release.
    pub envelope_release: f64,
    // Note: LfoRate/LfoDepth/EffectsSend are patch-level destinations; they are
    // represented in the patch-level values below.
    /// Summed mod to LFO rate (indexed; index 0 only — extend as needed).
    pub lfo_rate: [f64; MAX_LFOS],
    /// Summed mod to LFO depth (indexed).
    pub lfo_depth: [f64; MAX_LFOS],
}

impl Default for ModulationValues {
    fn default() -> Self {
        Self {
            oscillator_pitch: 0.0,
            oscillator_fine_tune: 0.0,
            oscillator_shape: 0.0,
            filter_cutoff: 0.0,
            filter_resonance: 0.0,
            amplitude: 0.0,
            pan: 0.0,
            envelope_attack: 0.0,
            envelope_decay: 0.0,
            envelope_sustain: 0.0,
            envelope_release: 0.0,
            lfo_rate: [0.0; MAX_LFOS],
            lfo_depth: [0.0; MAX_LFOS],
        }
    }
}

// ── ModulationProcessor ───────────────────────────────────────────────

/// Domain service that evaluates all mod sources for a single patch and sums
/// the routed modulation contributions each audio block.
///
/// # Audio-thread guarantees
///
/// - `process` never allocates heap memory.
/// - `process` never acquires a mutex or blocking lock.
/// - `process` never performs blocking I/O.
///
/// # Per-note vs per-patch
///
/// Per-note expression dimensions (X/Y/Z) in the routing table are *skipped*
/// by this processor — they must be delivered directly to each voice by the
/// engine so that MPE works correctly. Only patch-level sources are evaluated
/// here.
pub struct ModulationProcessor {
    /// LFO runtime states (fixed-capacity; allocated once off audio thread).
    lfo_states: [LfoState; MAX_LFOS],
    /// Mod envelope runtime states (fixed-capacity).
    mod_env_states: [ModEnvState; MAX_MOD_ENVELOPES],
    /// Current velocity value in [0.0, 1.0] (set from note-on).
    velocity: f64,
    /// Current key-track value in [-1.0, 1.0] (set from note number / centre).
    key_track: f64,
    /// Current macro value in [-1.0, 1.0].
    macro_value: f64,
    /// Current random value in [-1.0, 1.0] (refreshed each block).
    random_value: f64,
    /// A simple 64-bit PRNG state (xorshift64) — deterministic, no allocation.
    rng_state: u64,
}

impl ModulationProcessor {
    /// Construct a `ModulationProcessor` with neutral initial state.
    ///
    /// This allocates the fixed runtime arrays once; `process` is allocation-free.
    pub fn new() -> Self {
        Self {
            lfo_states: [LfoState::new(0.0); MAX_LFOS],
            mod_env_states: [ModEnvState::idle(); MAX_MOD_ENVELOPES],
            velocity: 0.0,
            key_track: 0.0,
            macro_value: 0.0,
            random_value: 0.0,
            rng_state: 0x853c_49e6_748f_ea9b,
        }
    }

    // ── Event handlers ─────────────────────────────────────────────────

    /// Set velocity from a note-on event (0.0–1.0).
    ///
    /// Call this from the audio-thread event handling path before `process`.
    pub fn set_velocity(&mut self, velocity: f64) {
        self.velocity = velocity.clamp(0.0, 1.0);
    }

    /// Set key-track value (-1.0–1.0) derived from MIDI note number.
    ///
    /// Typical usage: `(note_number as f64 - 60.0) / 60.0` (A=440 as centre).
    pub fn set_key_track(&mut self, value: f64) {
        self.key_track = value.clamp(-1.0, 1.0);
    }

    /// Set the macro knob value (-1.0–1.0).
    pub fn set_macro(&mut self, value: f64) {
        self.macro_value = value.clamp(-1.0, 1.0);
    }

    /// Trigger all mod envelopes (note-on).
    pub fn trigger_envelopes(&mut self) {
        for env in self.mod_env_states.iter_mut() {
            env.trigger();
        }
    }

    /// Release all mod envelopes (note-off).
    pub fn release_envelopes(&mut self) {
        for env in self.mod_env_states.iter_mut() {
            env.release();
        }
    }

    // ── Core processing ─────────────────────────────────────────────────

    /// Evaluate all patch-level mod sources for one audio block and return the
    /// summed modulation contributions.
    ///
    /// # Parameters
    ///
    /// - `matrix` — the current `ModMatrix` snapshot for this patch. Read-only.
    /// - `block_size` — number of samples in the block.
    /// - `sample_rate` — audio sample rate in Hz.
    ///
    /// # Returns
    ///
    /// A [`ModulationValues`] struct with one summed contribution per
    /// destination. Values are in `[-1.0, 1.0]` (bipolar) before the engine
    /// scales them.
    ///
    /// # Audio-thread invariants
    ///
    /// This function never allocates heap memory, never acquires a lock, and
    /// never performs I/O.
    pub fn process(
        &mut self,
        matrix: &ModMatrix,
        block_size: usize,
        sample_rate: f64,
    ) -> ModulationValues {
        debug_assert!(block_size > 0, "block_size must be positive");
        debug_assert!(sample_rate > 0.0, "sample_rate must be positive");

        let block_seconds = block_size as f64 / sample_rate;
        let lfo_configs = matrix.lfo_configs();
        let mod_envelopes = matrix.mod_envelopes();
        let routings = matrix.routings();

        // ── 1. Advance LFO phases ──────────────────────────────────────────

        for (i, state) in self.lfo_states.iter_mut().enumerate() {
            if i >= lfo_configs.len() {
                break;
            }
            let cfg = &lfo_configs[i];
            let delta = 2.0 * std::f64::consts::PI * cfg.rate * block_seconds;
            state.advance(delta);
        }

        // ── 2. Advance mod envelopes ───────────────────────────────────────

        for (i, env_state) in self.mod_env_states.iter_mut().enumerate() {
            if i >= mod_envelopes.len() {
                break;
            }
            let cfg = &mod_envelopes[i];
            env_state.advance(
                block_seconds,
                cfg.attack,
                cfg.decay,
                cfg.sustain,
                cfg.release,
            );
        }

        // ── 3. Refresh random value (once per block, no allocation) ───────────

        self.random_value = self.next_random();

        // ── 4. Evaluate each routing and accumulate ──────────────────────────

        let mut values = ModulationValues::default();
        let max_routings = routings.len().min(MAX_ROUTINGS);

        for routing in &routings[..max_routings] {
            // Per-note expression sources are skipped: they must be delivered
            // directly to each voice by the engine.
            if routing.source().is_per_note_expression() {
                continue;
            }

            let source_value = self.evaluate_source(routing, lfo_configs);
            let contribution = source_value * routing.depth();
            Self::accumulate_destination(&mut values, routing, contribution);
        }

        values
    }

    // ── Private helpers ──────────────────────────────────────────────────

    /// Evaluate a single mod source to a value in [-1.0, 1.0].
    ///
    /// Never allocates. Never blocks. Never does I/O.
    #[inline]
    fn evaluate_source(&self, routing: &ModRouting, lfo_configs: &[LfoConfig]) -> f64 {
        match routing.source() {
            ModSourceType::Lfo => {
                // Use the first LFO for generic Lfo source.
                if lfo_configs.is_empty() {
                    return 0.0;
                }
                self.lfo_states[0].sample(lfo_configs[0].waveform) * lfo_configs[0].depth
            }
            ModSourceType::Envelope => {
                // Use the first mod envelope.
                self.mod_env_states[0].current_level() * 2.0 - 1.0
            }
            ModSourceType::Random => self.random_value,
            ModSourceType::Macro => self.macro_value,
            ModSourceType::Velocity => self.velocity * 2.0 - 1.0,
            ModSourceType::KeyTrack => self.key_track,
            // Per-note expression sources must never be processed at patch level.
            // They reach each voice independently via the engine.
            ModSourceType::PerNoteBendX
            | ModSourceType::PerNoteTimbreY
            | ModSourceType::PerNotePressureZ => {
                debug_assert!(
                    false,
                    "per-note expression sources must not be evaluated at patch level"
                );
                0.0
            }
        }
    }

    /// Accumulate `contribution` into the matching field of `values`.
    ///
    /// Never allocates. Never blocks. Never does I/O.
    #[inline]
    fn accumulate_destination(
        values: &mut ModulationValues,
        routing: &ModRouting,
        contribution: f64,
    ) {
        match routing.destination() {
            ModDestinationType::OscillatorPitch => values.oscillator_pitch += contribution,
            ModDestinationType::OscillatorFineTune => values.oscillator_fine_tune += contribution,
            ModDestinationType::OscillatorShape => values.oscillator_shape += contribution,
            ModDestinationType::FilterCutoff => values.filter_cutoff += contribution,
            ModDestinationType::FilterResonance => values.filter_resonance += contribution,
            ModDestinationType::Amplitude => values.amplitude += contribution,
            ModDestinationType::Pan => values.pan += contribution,
            ModDestinationType::EnvelopeAttack => values.envelope_attack += contribution,
            ModDestinationType::EnvelopeDecay => values.envelope_decay += contribution,
            ModDestinationType::EnvelopeSustain => values.envelope_sustain += contribution,
            ModDestinationType::EnvelopeRelease => values.envelope_release += contribution,
            ModDestinationType::LfoRate(idx) => {
                let i = idx as usize;
                if i < MAX_LFOS {
                    values.lfo_rate[i] += contribution;
                }
            }
            ModDestinationType::LfoDepth(idx) => {
                let i = idx as usize;
                if i < MAX_LFOS {
                    values.lfo_depth[i] += contribution;
                }
            }
            ModDestinationType::EffectsSend(_) => {
                // EffectsSend is a patch-level destination not tracked in
                // ModulationValues yet; silently ignored.
            }
        }
    }

    /// Advance the internal PRNG and return a value in [-1.0, 1.0].
    ///
    /// Uses xorshift64 — deterministic, lock-free, allocation-free.
    #[inline]
    fn next_random(&mut self) -> f64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        (x as i64 as f64) / (i64::MAX as f64)
    }
}

impl Default for ModulationProcessor {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modulation::lfo_config::LfoConfig;
    use crate::modulation::lfo_waveform::LfoWaveform;
    use crate::modulation::mod_destination_type::ModDestinationType;
    use crate::modulation::mod_matrix::{ModMatrix, ModMatrixCommand};
    use crate::modulation::mod_source_type::ModSourceType;
    use crate::patch::patch_id::PatchId;

    fn patch() -> PatchId {
        PatchId::new(1)
    }

    fn empty_matrix() -> ModMatrix {
        ModMatrix::new(patch())
    }

    fn processor() -> ModulationProcessor {
        ModulationProcessor::new()
    }

    const SR: f64 = 48_000.0;
    const BLOCK: usize = 128;

    // ── No-routing baseline ──────────────────────────────────────────

    #[test]
    fn empty_matrix_produces_zero_mod_values() {
        let mut p = processor();
        let m = empty_matrix();
        let v = p.process(&m, BLOCK, SR);
        assert_eq!(v, ModulationValues::default());
    }

    // ── Velocity source ────────────────────────────────────────────

    #[test]
    fn velocity_modulates_amplitude() {
        let mut p = processor();
        p.set_velocity(1.0); // maximum velocity

        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Velocity,
            destination: ModDestinationType::Amplitude,
            depth: 1.0,
        })
        .unwrap();

        let v = p.process(&m, BLOCK, SR);
        // velocity 1.0 → (1.0 * 2 - 1) = 1.0, depth = 1.0, contribution = 1.0
        assert!(
            (v.amplitude - 1.0).abs() < 1e-10,
            "amplitude: {}",
            v.amplitude
        );
    }

    #[test]
    fn velocity_zero_produces_negative_one_contribution() {
        let mut p = processor();
        p.set_velocity(0.0);

        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Velocity,
            destination: ModDestinationType::Amplitude,
            depth: 1.0,
        })
        .unwrap();

        let v = p.process(&m, BLOCK, SR);
        // velocity 0 → 0*2 - 1 = -1, depth 1.0, contribution = -1.0
        assert!((v.amplitude - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn velocity_clamped_above_one() {
        let mut p = processor();
        p.set_velocity(2.0); // should be clamped to 1.0
        assert!((p.velocity - 1.0).abs() < f64::EPSILON);
    }

    // ── Key-track source ───────────────────────────────────────────

    #[test]
    fn key_track_modulates_filter_cutoff() {
        let mut p = processor();
        p.set_key_track(0.5); // half-way up

        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::KeyTrack,
            destination: ModDestinationType::FilterCutoff,
            depth: 1.0,
        })
        .unwrap();

        let v = p.process(&m, BLOCK, SR);
        assert!((v.filter_cutoff - 0.5).abs() < 1e-10);
    }

    // ── Macro source ────────────────────────────────────────────

    #[test]
    fn macro_modulates_oscillator_pitch() {
        let mut p = processor();
        p.set_macro(-0.5);

        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Macro,
            destination: ModDestinationType::OscillatorPitch,
            depth: 1.0,
        })
        .unwrap();

        let v = p.process(&m, BLOCK, SR);
        assert!((v.oscillator_pitch - (-0.5)).abs() < 1e-10);
    }

    // ── Per-note expression skipped at patch level ─────────────────────────

    #[test]
    fn per_note_expression_sources_are_skipped() {
        let mut p = processor();

        let mut m = empty_matrix();
        // Add per-note expression routings — they must be skipped.
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::PerNoteBendX,
            destination: ModDestinationType::OscillatorPitch,
            depth: 1.0,
        })
        .unwrap();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::PerNoteTimbreY,
            destination: ModDestinationType::FilterCutoff,
            depth: 1.0,
        })
        .unwrap();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::PerNotePressureZ,
            destination: ModDestinationType::Amplitude,
            depth: 1.0,
        })
        .unwrap();

        let v = p.process(&m, BLOCK, SR);
        // All contributions should be zero — per-note expression is not
        // processed at patch level.
        assert!(
            v.oscillator_pitch.abs() < 1e-10,
            "PerNoteBendX must not contribute at patch level"
        );
        assert!(
            v.filter_cutoff.abs() < 1e-10,
            "PerNoteTimbreY must not contribute at patch level"
        );
        assert!(
            v.amplitude.abs() < 1e-10,
            "PerNotePressureZ must not contribute at patch level"
        );
    }

    // ── Depth scaling ────────────────────────────────────────────────

    #[test]
    fn routing_depth_scales_contribution() {
        let mut p = processor();
        p.set_macro(1.0);

        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Macro,
            destination: ModDestinationType::Pan,
            depth: 0.5,
        })
        .unwrap();

        let v = p.process(&m, BLOCK, SR);
        // macro 1.0 * depth 0.5 = 0.5
        assert!((v.pan - 0.5).abs() < 1e-10);
    }

    #[test]
    fn negative_depth_inverts_contribution() {
        let mut p = processor();
        p.set_macro(1.0);

        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Macro,
            destination: ModDestinationType::Pan,
            depth: -1.0,
        })
        .unwrap();

        let v = p.process(&m, BLOCK, SR);
        // macro 1.0 * depth -1.0 = -1.0
        assert!((v.pan - (-1.0)).abs() < 1e-10);
    }

    // ── Multiple routings accumulate ──────────────────────────────────────

    #[test]
    fn multiple_routings_to_same_destination_accumulate() {
        let mut p = processor();
        p.set_macro(1.0);
        p.set_key_track(0.25);

        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Macro,
            destination: ModDestinationType::FilterCutoff,
            depth: 0.5,
        })
        .unwrap();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::KeyTrack,
            destination: ModDestinationType::FilterCutoff,
            depth: 1.0,
        })
        .unwrap();

        let v = p.process(&m, BLOCK, SR);
        // macro 1.0 * 0.5 + key_track 0.25 * 1.0 = 0.5 + 0.25 = 0.75
        assert!((v.filter_cutoff - 0.75).abs() < 1e-10);
    }

    // ── Envelope trigger/release ──────────────────────────────────────

    #[test]
    fn envelope_level_zero_before_trigger() {
        let p = processor();
        assert!((p.mod_env_states[0].current_level()).abs() < f64::EPSILON);
    }

    #[test]
    fn envelope_trigger_starts_attack() {
        let mut p = processor();
        p.trigger_envelopes();
        assert_eq!(p.mod_env_states[0].stage, ModEnvStage::Attack);
    }

    #[test]
    fn envelope_release_after_sustain_starts_release() {
        let mut p = processor();
        // Jump to sustain by assigning state directly.
        p.mod_env_states[0].stage = ModEnvStage::Sustain;
        p.mod_env_states[0].level = 0.8;
        p.release_envelopes();
        assert_eq!(p.mod_env_states[0].stage, ModEnvStage::Release);
    }

    // ── LFO routing ──────────────────────────────────────────────

    #[test]
    fn lfo_routing_produces_non_zero_contribution_after_advance() {
        let mut p = processor();
        // Set LFO phase to π/2 so sine = 1.0.
        p.lfo_states[0].phase = std::f64::consts::FRAC_PI_2;

        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::ConfigureLfo {
            lfo_index: 0,
            config: LfoConfig::try_new(1.0, 1.0, 0.0, false, LfoWaveform::Sine).unwrap(),
        })
        .unwrap();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Lfo,
            destination: ModDestinationType::FilterCutoff,
            depth: 1.0,
        })
        .unwrap();

        let v = p.process(&m, BLOCK, SR);
        // After the block the LFO phase advances slightly from π/2.
        // The value should be close to sine(≈π/2) * depth 1.0, so near 1.0.
        assert!(
            v.filter_cutoff > 0.9,
            "expected near-max LFO contribution, got {}",
            v.filter_cutoff
        );
    }

    // ── LFO Rate destination ──────────────────────────────────────────

    #[test]
    fn lfo_rate_destination_accumulates() {
        let mut p = processor();
        p.set_macro(1.0);

        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Macro,
            destination: ModDestinationType::LfoRate(0),
            depth: 0.8,
        })
        .unwrap();

        let v = p.process(&m, BLOCK, SR);
        assert!((v.lfo_rate[0] - 0.8).abs() < 1e-10);
    }

    // ── Default / constructor ──────────────────────────────────────────

    #[test]
    fn default_and_new_produce_same_initial_state() {
        let a = ModulationProcessor::new();
        let b = ModulationProcessor::default();
        // Both should produce the same output for an empty matrix.
        let m = empty_matrix();
        let mut pa = a;
        let mut pb = b;
        let va = pa.process(&m, BLOCK, SR);
        let vb = pb.process(&m, BLOCK, SR);
        assert_eq!(va, vb);
    }

    // ── Random source ────────────────────────────────────────────────

    #[test]
    fn random_source_produces_bounded_value() {
        let mut p = processor();

        let mut m = empty_matrix();
        m.apply(ModMatrixCommand::AddRouting {
            source: ModSourceType::Random,
            destination: ModDestinationType::OscillatorShape,
            depth: 1.0,
        })
        .unwrap();

        let v = p.process(&m, BLOCK, SR);
        assert!(
            (-1.0..=1.0).contains(&v.oscillator_shape),
            "random out of bounds: {}",
            v.oscillator_shape
        );
    }

    // ── Modulation values default ─────────────────────────────────────────

    #[test]
    fn modulation_values_default_all_zero() {
        let v = ModulationValues::default();
        assert_eq!(v.oscillator_pitch, 0.0);
        assert_eq!(v.filter_cutoff, 0.0);
        assert_eq!(v.amplitude, 0.0);
        assert_eq!(v.lfo_rate, [0.0; MAX_LFOS]);
        assert_eq!(v.lfo_depth, [0.0; MAX_LFOS]);
    }
}
