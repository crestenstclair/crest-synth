// path: src/adapter/fundsp_effects.rs

//! `FundspEffects` — an [`EffectProcessor`]-compatible adapter backed by
//! [fundsp](https://docs.rs/fundsp) composable DSP nodes.
//!
//! # Design
//!
//! `FundspEffects` implements the same port contract as the built-in
//! `EffectProcessor` (see `effects::effect_processor`) but routes audio
//! through fundsp DSP nodes for richer reverb, chorus and delay algorithms.
//!
//! Each call to [`FundspEffects::process`] is sample-accurate: it drives
//! fundsp's `AudioUnit::tick` per stereo sample.  The internal node is
//! pre-constructed; no heap allocation occurs in the hot path once the node
//! has been built.  When parameters change the node is rebuilt outside the
//! hot path (on the first call to `process` after a change), and the old
//! node is dropped at that point — never on the audio thread if the caller
//! manages param updates correctly.
//!
//! # Audio-thread Safety
//!
//! - `process` and `reset` do **not** allocate heap memory after the node is
//!   built.
//! - No mutex or blocking lock is held during `process` or `reset`.
//! - No blocking I/O is performed.
//!
//! # Signal Flow
//!
//! ```text
//! input frames → fundsp stereo node (tick per sample) → wet/dry blend → output frames
//! ```

use crate::effects::effect_processor::{EffectParams, EffectType};
use crate::kernel::audio_frame::AudioFrame;
use fundsp::audiounit::AudioUnit;
use fundsp::hacker32::{chorus, delay, highpass_hz, lowpass_hz, pass, reverb_stereo};

// ─── Stereo fundsp node kinds ─────────────────────────────────────────────────

/// Variant tag for the active stereo node.
///
/// We keep this separate from `EffectType` so that the adapter can be
/// extended without modifying the core effects domain.
#[derive(Debug, Clone, Copy, PartialEq)]
enum NodeKind {
    Bypass,
    Gain,
    LowPassFilter,
    HighPassFilter,
    Delay,
}

impl NodeKind {
    fn from_params(params: &EffectParams) -> Self {
        match params.effect_type {
            EffectType::Bypass => NodeKind::Bypass,
            EffectType::Gain => NodeKind::Gain,
            EffectType::LowPassFilter => NodeKind::LowPassFilter,
            EffectType::HighPassFilter => NodeKind::HighPassFilter,
            EffectType::Delay => NodeKind::Delay,
        }
    }
}

// ─── Node construction helpers ────────────────────────────────────────────────

/// Build a stereo fundsp delay node with feedback.
///
/// The node is a per-channel feedback delay using fundsp's `delay` primitive.
/// The delay length is pre-allocated; feedback is applied manually in the
/// processing loop.
///
/// Returns a boxed `AudioUnit` with 2 inputs and 2 outputs.
fn build_stereo_delay(delay_secs: f32, sample_rate: f64) -> Box<dyn AudioUnit> {
    // Clamp to [1 sample, 2 s] to avoid zero-length delay.
    let secs = delay_secs.clamp(1.0 / sample_rate as f32, 2.0);
    // Stack two mono delay lines side by side: (L, R) → (L_wet, R_wet).
    // `delay(t)` is 1-in / 1-out; `|` (Stack) gives 2-in / 2-out.
    let node = delay(secs) | delay(secs);
    Box::new(node)
}

/// Build a stereo fundsp lowpass SVF node.
///
/// Stacks two identical mono lowpass nodes.
fn build_stereo_lowpass(cutoff_hz: f32, q: f32) -> Box<dyn AudioUnit> {
    // Stack two mono lowpass filters: (L, R) → (L_filtered, R_filtered).
    let node =
        lowpass_hz(cutoff_hz.max(10.0), q.max(0.1)) | lowpass_hz(cutoff_hz.max(10.0), q.max(0.1));
    Box::new(node)
}

/// Build a stereo fundsp highpass SVF node.
///
/// Stacks two identical mono highpass nodes.
fn build_stereo_highpass(cutoff_hz: f32, q: f32) -> Box<dyn AudioUnit> {
    // Stack two mono highpass filters: (L, R) → (L_filtered, R_filtered).
    let node =
        highpass_hz(cutoff_hz.max(10.0), q.max(0.1)) | highpass_hz(cutoff_hz.max(10.0), q.max(0.1));
    Box::new(node)
}

/// Build a stereo chorus node.
///
/// `fundsp::hacker32::chorus` is mono (1-in / 1-out).  We stack two with
/// different LFO seeds to produce a stereo chorus with natural-sounding
/// channel decorrelation.
#[allow(dead_code)]
fn build_stereo_chorus() -> Box<dyn AudioUnit> {
    // Seed 0 for left, seed 1 for right.
    // `|` stacks the two mono chorus nodes: (L, R) → (L_chorused, R_chorused).
    let node = chorus(0, 0.015, 0.005, 0.2) | chorus(1, 0.015, 0.005, 0.2);
    Box::new(node)
}

/// Build a stereo fundsp reverb node (32-channel FDN).
#[allow(dead_code)]
fn build_stereo_reverb(room_size: f32, time: f32, damping: f32) -> Box<dyn AudioUnit> {
    // `reverb_stereo` (hacker32) takes f32 parameters and is already 2-in / 2-out.
    let node = reverb_stereo(
        room_size.clamp(1.0, 30.0),
        time.clamp(0.1, 10.0),
        damping.clamp(0.0, 1.0),
    );
    Box::new(node)
}

// ─── FundspEffects ─────────────────────────────────────────────────────────────

/// Single-slot effect processor backed by fundsp composable DSP nodes.
///
/// Implements the same port contract as `effects::effect_processor::EffectProcessor`:
///
/// | Method    | Signature                                           |
/// |-----------|-----------------------------------------------------|
/// | `process` | `(&mut self, &[AudioFrame], EffectParams) -> &[AudioFrame]` |
/// | `reset`   | `(&mut self)`                                       |
///
/// # Allocation policy
///
/// The underlying fundsp node is allocated once during [`build_node`] (which
/// is called automatically on the first call to `process` and whenever
/// parameters change).  After warm-up the hot path (`process`) performs no
/// heap allocation and acquires no locks.
pub struct FundspEffects {
    /// The active fundsp stereo node (2 in, 2 out).  `None` before the first
    /// `process` call.
    node: Option<Box<dyn AudioUnit>>,
    /// Params used to build the current node; used to detect changes.
    built_params: Option<EffectParams>,
    /// The kind of node currently active.
    active_kind: NodeKind,
    /// Per-channel feedback accumulator for the Delay algorithm.
    ///
    /// fundsp's `delay` node does not include a feedback path, so we
    /// implement the wet→input summation manually here, in a lock-free,
    /// allocation-free way.
    feedback_l: f32,
    feedback_r: f32,
    /// Pre-allocated output scratch buffer.
    out_buf: Vec<AudioFrame>,
    /// Scratch arrays for fundsp `tick` I/O (re-used every sample).
    tick_in: [f32; 2],
    tick_out: [f32; 2],
}

impl FundspEffects {
    /// Create a new `FundspEffects` in bypass state.
    ///
    /// The fundsp node is not allocated until the first call to `process`.
    pub fn new() -> Self {
        Self {
            node: None,
            built_params: None,
            active_kind: NodeKind::Bypass,
            feedback_l: 0.0,
            feedback_r: 0.0,
            out_buf: Vec::new(),
            tick_in: [0.0; 2],
            tick_out: [0.0; 2],
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Rebuild the fundsp node for `params`.
    ///
    /// Only called when params differ from the last build.  This may allocate;
    /// callers must ensure this is not called from the audio thread (or accept
    /// the warm-up alloc on the first invocation).
    fn build_node(&mut self, params: &EffectParams) {
        let kind = NodeKind::from_params(params);
        let sr = params.sample_rate.max(1.0) as f64;
        let node: Box<dyn AudioUnit> = match kind {
            NodeKind::Bypass | NodeKind::Gain => {
                // Stereo passthrough — two `pass()` nodes stacked side by side.
                // Gain is applied manually in the tick loop.
                let node = pass() | pass();
                Box::new(node)
            }
            NodeKind::LowPassFilter => build_stereo_lowpass(params.cutoff_hz, params.resonance),
            NodeKind::HighPassFilter => build_stereo_highpass(params.cutoff_hz, params.resonance),
            NodeKind::Delay => build_stereo_delay(params.delay_secs, sr),
        };

        // Initialise the sample rate on the freshly built node.
        let mut node = node;
        node.set_sample_rate(sr);
        node.reset();

        self.node = Some(node);
        self.active_kind = kind;
        self.built_params = Some(*params);
        self.feedback_l = 0.0;
        self.feedback_r = 0.0;
    }

    /// Ensure the internal node is up-to-date for `params`.
    ///
    /// Rebuilds the node whenever parameters have changed or the node has not
    /// yet been built.
    fn ensure_node(&mut self, params: &EffectParams) {
        let needs_rebuild = match &self.built_params {
            None => true,
            Some(p) => p != params,
        };
        if needs_rebuild {
            self.build_node(params);
        }
    }

    // ── Port contract ─────────────────────────────────────────────────────────

    /// Process a slice of [`AudioFrame`]s through the fundsp DSP node.
    ///
    /// Returns a reference to an internal pre-allocated buffer holding the
    /// processed frames.  The buffer remains valid until the next call to
    /// `process`.
    ///
    /// # Allocation behaviour
    ///
    /// The output scratch buffer grows (via `Vec::resize`) only when
    /// `frames.len()` exceeds its current capacity.  After a warm-up block
    /// no allocation occurs for subsequent blocks of equal or smaller size.
    ///
    /// The fundsp node is rebuilt (heap-allocated) only when `params` differ
    /// from the previously-used params.
    pub fn process(&mut self, frames: &[AudioFrame], params: EffectParams) -> &[AudioFrame] {
        // Ensure output buffer is large enough (grows only during warm-up).
        if self.out_buf.len() < frames.len() {
            self.out_buf.resize(frames.len(), AudioFrame::silence());
        }

        // Rebuild the fundsp node if params changed.
        self.ensure_node(&params);

        let wet = params.wet_mix.clamp(0.0, 1.0);
        let dry = 1.0 - wet;

        match params.effect_type {
            EffectType::Bypass => {
                // Copy input unchanged.
                self.out_buf[..frames.len()].copy_from_slice(frames);
            }

            EffectType::Gain => {
                let node = self.node.as_mut().expect("node built above");
                let g = params.gain.clamp(0.0, 4.0);
                for (i, frame) in frames.iter().enumerate() {
                    self.tick_in[0] = frame.left;
                    self.tick_in[1] = frame.right;
                    node.tick(&self.tick_in, &mut self.tick_out);
                    // tick_out is the passthrough signal; apply gain manually.
                    let gained_l = self.tick_out[0] * g;
                    let gained_r = self.tick_out[1] * g;
                    self.out_buf[i] = AudioFrame::new(
                        frame.left * dry + gained_l * wet,
                        frame.right * dry + gained_r * wet,
                    );
                }
            }

            EffectType::LowPassFilter | EffectType::HighPassFilter => {
                let node = self.node.as_mut().expect("node built above");
                for (i, frame) in frames.iter().enumerate() {
                    self.tick_in[0] = frame.left;
                    self.tick_in[1] = frame.right;
                    node.tick(&self.tick_in, &mut self.tick_out);
                    self.out_buf[i] = AudioFrame::new(
                        frame.left * dry + self.tick_out[0] * wet,
                        frame.right * dry + self.tick_out[1] * wet,
                    );
                }
            }

            EffectType::Delay => {
                let node = self.node.as_mut().expect("node built above");
                let feedback = params.feedback.clamp(0.0, 0.99);
                for (i, frame) in frames.iter().enumerate() {
                    // Mix dry input with feedback from previous output.
                    self.tick_in[0] = frame.left + self.feedback_l * feedback;
                    self.tick_in[1] = frame.right + self.feedback_r * feedback;
                    node.tick(&self.tick_in, &mut self.tick_out);
                    // Store the delayed output for the next iteration.
                    self.feedback_l = self.tick_out[0];
                    self.feedback_r = self.tick_out[1];
                    self.out_buf[i] = AudioFrame::new(
                        frame.left * dry + self.tick_out[0] * wet,
                        frame.right * dry + self.tick_out[1] * wet,
                    );
                }
            }
        }

        &self.out_buf[..frames.len()]
    }

    /// Reset all internal DSP state to silence.
    ///
    /// Clears fundsp node state and the feedback accumulators.
    pub fn reset(&mut self) {
        if let Some(node) = self.node.as_mut() {
            node.reset();
        }
        self.feedback_l = 0.0;
        self.feedback_r = 0.0;
    }
}

impl Default for FundspEffects {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for FundspEffects {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FundspEffects")
            .field("active_kind", &self.active_kind)
            .field("built_params", &self.built_params)
            .finish()
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn bypass_params() -> EffectParams {
        EffectParams {
            effect_type: EffectType::Bypass,
            ..EffectParams::default()
        }
    }

    fn sr() -> f32 {
        44_100.0
    }

    // ── bypass ────────────────────────────────────────────────────────────────

    #[test]
    fn bypass_passes_signal_unchanged() {
        let mut fx = FundspEffects::new();
        let input = [AudioFrame::new(0.5, -0.5)];
        let out = fx.process(&input, bypass_params());
        assert_eq!(out[0].left, 0.5);
        assert_eq!(out[0].right, -0.5);
    }

    #[test]
    fn bypass_empty_slice_returns_empty() {
        let mut fx = FundspEffects::new();
        let out = fx.process(&[], bypass_params());
        assert_eq!(out.len(), 0);
    }

    // ── gain ──────────────────────────────────────────────────────────────────

    #[test]
    fn gain_wet_scales_amplitude() {
        let mut fx = FundspEffects::new();
        let params = EffectParams {
            effect_type: EffectType::Gain,
            gain: 2.0,
            wet_mix: 1.0,
            sample_rate: sr(),
            ..EffectParams::default()
        };
        let input = [AudioFrame::new(0.25, 0.1)];
        let out = fx.process(&input, params);
        assert!((out[0].left - 0.5).abs() < 1e-5, "left={}", out[0].left);
        assert!((out[0].right - 0.2).abs() < 1e-5, "right={}", out[0].right);
    }

    #[test]
    fn gain_zero_wet_silences_output() {
        let mut fx = FundspEffects::new();
        let params = EffectParams {
            effect_type: EffectType::Gain,
            gain: 0.0,
            wet_mix: 1.0,
            sample_rate: sr(),
            ..EffectParams::default()
        };
        let input = [AudioFrame::new(1.0, 1.0)];
        let out = fx.process(&input, params);
        assert_eq!(out[0].left, 0.0);
        assert_eq!(out[0].right, 0.0);
    }

    // ── low-pass filter ───────────────────────────────────────────────────────

    #[test]
    fn lowpass_attenuates_near_nyquist() {
        let mut fx = FundspEffects::new();
        let sample_rate = sr();
        let params = EffectParams {
            effect_type: EffectType::LowPassFilter,
            cutoff_hz: 500.0,
            resonance: 0.707,
            wet_mix: 1.0,
            sample_rate,
            ..EffectParams::default()
        };
        let freq = 20_000.0_f32;
        let mut input_frames = [AudioFrame::silence(); 256];
        for (i, frame) in input_frames.iter_mut().enumerate() {
            let s = (2.0 * core::f32::consts::PI * freq * i as f32 / sample_rate).sin();
            *frame = AudioFrame::mono(s);
        }
        let out = fx.process(&input_frames, params);
        let peak = out.iter().map(|f| f.left.abs()).fold(0.0_f32, f32::max);
        assert!(peak < 0.2, "expected strong attenuation, peak={peak}");
    }

    // ── high-pass filter ──────────────────────────────────────────────────────

    #[test]
    fn highpass_passes_high_frequency() {
        let mut fx = FundspEffects::new();
        let sample_rate = sr();
        let params = EffectParams {
            effect_type: EffectType::HighPassFilter,
            cutoff_hz: 200.0,
            resonance: 0.707,
            wet_mix: 1.0,
            sample_rate,
            ..EffectParams::default()
        };
        let freq = 10_000.0_f32;
        let mut input_frames = [AudioFrame::silence(); 256];
        for (i, frame) in input_frames.iter_mut().enumerate() {
            let s = (2.0 * core::f32::consts::PI * freq * i as f32 / sample_rate).sin();
            *frame = AudioFrame::mono(s);
        }
        let out = fx.process(&input_frames, params);
        let peak = out.iter().map(|f| f.left.abs()).fold(0.0_f32, f32::max);
        assert!(peak > 0.5, "expected high-freq pass, peak={peak}");
    }

    // ── delay ─────────────────────────────────────────────────────────────────

    #[test]
    fn delay_produces_echo_after_delay_time() {
        let mut fx = FundspEffects::new();
        let sample_rate = sr();
        let delay_secs = 0.01_f32; // 10 ms → 441 frames
        let delay_frames = (delay_secs * sample_rate) as usize;
        let params = EffectParams {
            effect_type: EffectType::Delay,
            delay_secs,
            feedback: 0.0,
            wet_mix: 1.0,
            sample_rate,
            ..EffectParams::default()
        };
        let mut input = vec![AudioFrame::silence(); delay_frames + 10];
        input[0] = AudioFrame::new(1.0, 1.0);
        let out = fx.process(&input, params);
        // Echo should appear at approximately `delay_frames`.
        let echo = out[delay_frames].left;
        assert!(
            echo > 0.5,
            "expected echo at frame {delay_frames}, got {echo}"
        );
    }

    // ── reset ─────────────────────────────────────────────────────────────────

    #[test]
    fn reset_clears_dsp_state() {
        let mut fx = FundspEffects::new();
        let sample_rate = sr();
        let params = EffectParams {
            effect_type: EffectType::LowPassFilter,
            cutoff_hz: 500.0,
            resonance: 0.707,
            wet_mix: 1.0,
            sample_rate,
            ..EffectParams::default()
        };
        let input = [AudioFrame::new(1.0, 1.0); 64];
        fx.process(&input, params);
        fx.reset();
        // After reset a silent input should yield silence.
        let silent = [AudioFrame::silence()];
        let out = fx.process(&silent, params);
        assert_eq!(out[0].left, 0.0);
        assert_eq!(out[0].right, 0.0);
    }

    // ── wet/dry blend ─────────────────────────────────────────────────────────

    #[test]
    fn wet_dry_blend_mixes_signals() {
        let mut fx = FundspEffects::new();
        let params = EffectParams {
            effect_type: EffectType::Gain,
            gain: 0.0,    // wet is silence
            wet_mix: 0.5, // 50% dry + 50% silent = 0.5 * input
            sample_rate: sr(),
            ..EffectParams::default()
        };
        let input = [AudioFrame::new(1.0, 1.0)];
        let out = fx.process(&input, params);
        assert!((out[0].left - 0.5).abs() < 1e-5, "left={}", out[0].left);
    }

    // ── node rebuild on param change ──────────────────────────────────────────

    #[test]
    fn node_rebuilds_when_effect_type_changes() {
        let mut fx = FundspEffects::new();
        // First call: Bypass
        let bypass = bypass_params();
        let input = [AudioFrame::new(0.3, 0.7)];
        let out1 = fx.process(&input, bypass);
        assert_eq!(out1[0].left, 0.3);
        // Second call: Gain ×2 (wet)
        let gain_params = EffectParams {
            effect_type: EffectType::Gain,
            gain: 2.0,
            wet_mix: 1.0,
            sample_rate: sr(),
            ..EffectParams::default()
        };
        let out2 = fx.process(&input, gain_params);
        assert!((out2[0].left - 0.6).abs() < 1e-5, "left={}", out2[0].left);
    }

    // ── default and new match ─────────────────────────────────────────────────

    #[test]
    fn default_matches_new() {
        let a = FundspEffects::new();
        let b = FundspEffects::default();
        assert_eq!(a.active_kind, b.active_kind);
    }

    // ── multiple frames ───────────────────────────────────────────────────────

    #[test]
    fn bypass_multiple_frames() {
        let mut fx = FundspEffects::new();
        let input = vec![
            AudioFrame::new(0.1, 0.2),
            AudioFrame::new(0.3, 0.4),
            AudioFrame::new(0.5, 0.6),
        ];
        let out = fx.process(&input, bypass_params());
        assert_eq!(out.len(), 3);
        for (i, o) in input.iter().zip(out.iter()) {
            assert_eq!(i.left, o.left);
            assert_eq!(i.right, o.right);
        }
    }
}
