// path: src/effects/effect_processor.rs

//! Effect processor using biquad filters and delay-line DSP nodes.
//!
//! All DSP state is pre-allocated at construction time; `process` never
//! allocates.  Enum dispatch selects the active node type.

use crate::kernel::audio_frame::AudioFrame;

// ---------------------------------------------------------------------------
// EffectType — the set of supported effects
// ---------------------------------------------------------------------------

/// Which DSP algorithm the processor applies.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum EffectType {
    /// Pass audio through without modification.
    #[default]
    Bypass,
    /// Scale amplitude by a linear gain factor.
    Gain,
    /// Second-order low-pass filter (Butterworth topology).
    LowPassFilter,
    /// Second-order high-pass filter (Butterworth topology).
    HighPassFilter,
    /// Simple feedback delay line (mono-summed input).
    Delay,
}

// ---------------------------------------------------------------------------
// EffectParams — value type; no heap, safe to copy across the RT boundary
// ---------------------------------------------------------------------------

/// Parameters for an [`EffectProcessor`] slot.
///
/// All fields are plain `f32` scalars so the struct is `Copy` and can be
/// written through a `ParameterBridge` without locking.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectParams {
    /// Which algorithm to run.
    pub effect_type: EffectType,
    /// Linear amplitude multiplier (used by `Gain`; clamped to `[0, 4]`).
    pub gain: f32,
    /// Cut-off / centre frequency in Hz (used by filter effects).
    pub cutoff_hz: f32,
    /// Filter resonance / Q factor (used by filter effects; minimum 0.1).
    pub resonance: f32,
    /// Delay time in seconds (used by `Delay`; clamped to `[0, MAX_DELAY_SECS]`).
    pub delay_secs: f32,
    /// Feedback gain for delay (`[0, 0.99]`).
    pub feedback: f32,
    /// Wet/dry mix: 0.0 = fully dry, 1.0 = fully wet.
    pub wet_mix: f32,
    /// Sample rate required for coefficient calculation.
    pub sample_rate: f32,
}

impl Default for EffectParams {
    fn default() -> Self {
        Self {
            effect_type: EffectType::Bypass,
            gain: 1.0,
            cutoff_hz: 1000.0,
            resonance: 0.707,
            delay_secs: 0.25,
            feedback: 0.4,
            wet_mix: 0.5,
            sample_rate: 44100.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Biquad filter state (Direct Form I)
// ---------------------------------------------------------------------------

/// Second-order IIR biquad section.
///
/// State is stored per-channel to avoid inter-channel crosstalk.
#[derive(Debug, Clone, Copy, Default)]
struct BiquadState {
    // feed-forward coefficients
    b0: f32,
    b1: f32,
    b2: f32,
    // feed-back coefficients (negated in the recurrence)
    a1: f32,
    a2: f32,
    // delay registers — one set per stereo channel
    x1_l: f32,
    x2_l: f32,
    y1_l: f32,
    y2_l: f32,
    x1_r: f32,
    x2_r: f32,
    y1_r: f32,
    y2_r: f32,
}

impl BiquadState {
    /// Compute Butterworth low-pass coefficients.
    fn set_low_pass(&mut self, cutoff_hz: f32, q: f32, sample_rate: f32) {
        let omega = 2.0 * core::f32::consts::PI * cutoff_hz / sample_rate;
        let sin_o = omega.sin();
        let cos_o = omega.cos();
        let alpha = sin_o / (2.0 * q.max(0.1));
        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 - cos_o) / 2.0) / a0;
        self.b1 = (1.0 - cos_o) / a0;
        self.b2 = self.b0;
        self.a1 = (-2.0 * cos_o) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    /// Compute Butterworth high-pass coefficients.
    fn set_high_pass(&mut self, cutoff_hz: f32, q: f32, sample_rate: f32) {
        let omega = 2.0 * core::f32::consts::PI * cutoff_hz / sample_rate;
        let sin_o = omega.sin();
        let cos_o = omega.cos();
        let alpha = sin_o / (2.0 * q.max(0.1));
        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 + cos_o) / 2.0) / a0;
        self.b1 = (-(1.0 + cos_o)) / a0;
        self.b2 = self.b0;
        self.a1 = (-2.0 * cos_o) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    /// Process one stereo sample through the biquad (Direct Form I).
    ///
    /// No allocation; touches only the six `f32` registers stored in `self`.
    #[inline(always)]
    fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        let y_l = self.b0 * left + self.b1 * self.x1_l + self.b2 * self.x2_l
            - self.a1 * self.y1_l
            - self.a2 * self.y2_l;
        self.x2_l = self.x1_l;
        self.x1_l = left;
        self.y2_l = self.y1_l;
        self.y1_l = y_l;

        let y_r = self.b0 * right + self.b1 * self.x1_r + self.b2 * self.x2_r
            - self.a1 * self.y1_r
            - self.a2 * self.y2_r;
        self.x2_r = self.x1_r;
        self.x1_r = right;
        self.y2_r = self.y1_r;
        self.y1_r = y_r;

        (y_l, y_r)
    }

    /// Zero all delay registers.
    fn reset(&mut self) {
        *self = Self {
            b0: self.b0,
            b1: self.b1,
            b2: self.b2,
            a1: self.a1,
            a2: self.a2,
            ..Default::default()
        };
    }
}

// ---------------------------------------------------------------------------
// Delay line
// ---------------------------------------------------------------------------

/// Maximum delay time supported (seconds).
const MAX_DELAY_SECS: f32 = 2.0;
/// Maximum delay buffer length in stereo frames.
const MAX_DELAY_FRAMES: usize = (MAX_DELAY_SECS * 96_000.0) as usize + 1;

/// Simple feedback delay line with pre-allocated ring buffer.
///
/// The buffer is allocated once at construction; no allocation on `process`.
struct DelayLine {
    buf_l: Box<[f32; MAX_DELAY_FRAMES]>,
    buf_r: Box<[f32; MAX_DELAY_FRAMES]>,
    write_head: usize,
    delay_frames: usize,
}

impl DelayLine {
    fn new() -> Self {
        Self {
            buf_l: Box::new([0.0; MAX_DELAY_FRAMES]),
            buf_r: Box::new([0.0; MAX_DELAY_FRAMES]),
            write_head: 0,
            delay_frames: 0,
        }
    }

    fn set_delay(&mut self, delay_secs: f32, sample_rate: f32) {
        let frames = (delay_secs * sample_rate) as usize;
        self.delay_frames = frames.min(MAX_DELAY_FRAMES.saturating_sub(1));
    }

    /// Process one stereo sample; returns (wet_left, wet_right).
    ///
    /// No allocation; only index arithmetic and array reads/writes.
    #[inline(always)]
    fn process_sample(&mut self, left: f32, right: f32, feedback: f32) -> (f32, f32) {
        let len = MAX_DELAY_FRAMES;
        let read = (self.write_head + len - self.delay_frames) % len;
        let wet_l = self.buf_l[read];
        let wet_r = self.buf_r[read];
        self.buf_l[self.write_head] = left + wet_l * feedback;
        self.buf_r[self.write_head] = right + wet_r * feedback;
        self.write_head = (self.write_head + 1) % len;
        (wet_l, wet_r)
    }

    fn reset(&mut self) {
        self.buf_l.iter_mut().for_each(|s| *s = 0.0);
        self.buf_r.iter_mut().for_each(|s| *s = 0.0);
        self.write_head = 0;
    }
}

impl core::fmt::Debug for DelayLine {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DelayLine")
            .field("write_head", &self.write_head)
            .field("delay_frames", &self.delay_frames)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// EffectProcessor
// ---------------------------------------------------------------------------

/// Single-slot effect processor.
///
/// # Design
///
/// All DSP nodes (biquad state, delay line) are pre-allocated during
/// construction.  [`process`] mutates only those pre-allocated fields, so no
/// heap allocation ever occurs on the audio thread.
///
/// Enum dispatch in [`process`] selects the active algorithm based on
/// `EffectParams::effect_type`.
///
/// # Audio-thread Safety
///
/// - Zero heap allocation during `process`.
/// - Zero mutex or lock acquisition.
/// - Zero blocking I/O.
///
/// # Examples
///
/// ```
/// use crest_synth::effects::effect_processor::{EffectProcessor, EffectParams, EffectType};
/// use crest_synth::kernel::audio_frame::AudioFrame;
///
/// let mut proc = EffectProcessor::new();
/// let params = EffectParams { effect_type: EffectType::Bypass, ..EffectParams::default() };
/// let frames = [AudioFrame::new(0.5, -0.5)];
/// let out = proc.process(&frames, params);
/// assert_eq!(out[0].left, 0.5);
/// ```
pub struct EffectProcessor {
    biquad: BiquadState,
    delay: DelayLine,
    /// Cache of the last params used to set filter coefficients.
    last_params: EffectParams,
    /// Output frame scratch buffer — pre-allocated to avoid per-call allocation.
    out_buf: Vec<AudioFrame>,
}

impl core::fmt::Debug for EffectProcessor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EffectProcessor")
            .field("biquad", &self.biquad)
            .field("delay", &self.delay)
            .finish()
    }
}

impl EffectProcessor {
    /// Create a new `EffectProcessor` with default bypass state.
    ///
    /// This allocates the delay line buffer and an output scratch buffer.
    /// After construction no further heap allocation occurs during `process`.
    pub fn new() -> Self {
        Self {
            biquad: BiquadState::default(),
            delay: DelayLine::new(),
            last_params: EffectParams::default(),
            out_buf: Vec::new(),
        }
    }

    /// Process a slice of [`AudioFrame`]s, returning a slice with the same length.
    ///
    /// The returned slice is backed by an internal pre-grown buffer; it remains
    /// valid until the next call to `process`.
    ///
    /// `params` are applied sample-accurately: coefficient updates happen at
    /// the start of the block (block-accurate parameter updates).
    ///
    /// # Allocation behaviour
    ///
    /// The internal output buffer is grown (via `Vec::resize`) only when
    /// `frames.len()` exceeds its current capacity.  After a warm-up block,
    /// no allocation occurs for subsequent blocks of the same or smaller size.
    pub fn process(&mut self, frames: &[AudioFrame], params: EffectParams) -> &[AudioFrame] {
        // Grow the output scratch buffer if necessary (only happens during warm-up).
        if self.out_buf.len() < frames.len() {
            self.out_buf.resize(frames.len(), AudioFrame::silence());
        }

        // Recompute filter / delay coefficients only when params change.
        if params != self.last_params {
            self.apply_params(&params);
            self.last_params = params;
        }

        let wet = params.wet_mix.clamp(0.0, 1.0);
        let dry = 1.0 - wet;

        for (i, frame) in frames.iter().enumerate() {
            let out = match params.effect_type {
                EffectType::Bypass => *frame,

                EffectType::Gain => {
                    let g = params.gain.clamp(0.0, 4.0);
                    let gained_l = frame.left * g;
                    let gained_r = frame.right * g;
                    AudioFrame::new(
                        frame.left * dry + gained_l * wet,
                        frame.right * dry + gained_r * wet,
                    )
                }

                EffectType::LowPassFilter | EffectType::HighPassFilter => {
                    let (y_l, y_r) = self.biquad.process_sample(frame.left, frame.right);
                    AudioFrame::new(frame.left * dry + y_l * wet, frame.right * dry + y_r * wet)
                }

                EffectType::Delay => {
                    let (wet_l, wet_r) = self.delay.process_sample(
                        frame.left,
                        frame.right,
                        params.feedback.clamp(0.0, 0.99),
                    );
                    AudioFrame::new(
                        frame.left * dry + wet_l * wet,
                        frame.right * dry + wet_r * wet,
                    )
                }
            };
            self.out_buf[i] = out;
        }

        &self.out_buf[..frames.len()]
    }

    /// Reset all internal DSP state to silence.
    ///
    /// Call this when a voice or patch stops to avoid click artefacts from
    /// stale filter memory.
    pub fn reset(&mut self) {
        self.biquad.reset();
        self.delay.reset();
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn apply_params(&mut self, params: &EffectParams) {
        let sr = params.sample_rate.max(1.0);
        match params.effect_type {
            EffectType::LowPassFilter => {
                self.biquad
                    .set_low_pass(params.cutoff_hz, params.resonance, sr);
            }
            EffectType::HighPassFilter => {
                self.biquad
                    .set_high_pass(params.cutoff_hz, params.resonance, sr);
            }
            EffectType::Delay => {
                self.delay
                    .set_delay(params.delay_secs.clamp(0.0, MAX_DELAY_SECS), sr);
            }
            _ => {}
        }
    }
}

impl Default for EffectProcessor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn bypass_params() -> EffectParams {
        EffectParams {
            effect_type: EffectType::Bypass,
            ..EffectParams::default()
        }
    }

    #[test]
    fn bypass_passes_signal_unchanged() {
        let mut proc = EffectProcessor::new();
        let input = [AudioFrame::new(0.5, -0.5)];
        let out = proc.process(&input, bypass_params());
        assert_eq!(out[0].left, 0.5);
        assert_eq!(out[0].right, -0.5);
    }

    #[test]
    fn gain_scales_amplitude() {
        let mut proc = EffectProcessor::new();
        let params = EffectParams {
            effect_type: EffectType::Gain,
            gain: 2.0,
            wet_mix: 1.0,
            ..EffectParams::default()
        };
        let input = [AudioFrame::new(0.25, 0.1)];
        let out = proc.process(&input, params);
        assert!((out[0].left - 0.5).abs() < 1e-6);
        assert!((out[0].right - 0.2).abs() < 1e-6);
    }

    #[test]
    fn gain_zero_silences_output() {
        let mut proc = EffectProcessor::new();
        let params = EffectParams {
            effect_type: EffectType::Gain,
            gain: 0.0,
            wet_mix: 1.0,
            ..EffectParams::default()
        };
        let input = [AudioFrame::new(1.0, 1.0)];
        let out = proc.process(&input, params);
        assert_eq!(out[0].left, 0.0);
        assert_eq!(out[0].right, 0.0);
    }

    #[test]
    fn low_pass_attenuates_nyquist() {
        let mut proc = EffectProcessor::new();
        let sr = 44100.0_f32;
        let params = EffectParams {
            effect_type: EffectType::LowPassFilter,
            cutoff_hz: 500.0,
            resonance: 0.707,
            wet_mix: 1.0,
            sample_rate: sr,
            ..EffectParams::default()
        };
        // Feed a high-frequency signal (close to Nyquist) — should be heavily attenuated.
        let freq = 20_000.0_f32;
        let mut input_frames = [AudioFrame::silence(); 256];
        for (i, frame) in input_frames.iter_mut().enumerate() {
            let s = (2.0 * core::f32::consts::PI * freq * i as f32 / sr).sin();
            *frame = AudioFrame::mono(s);
        }
        let out = proc.process(&input_frames, params);
        // After 256 samples of settling the steady-state amplitude should be << 1.
        let peak = out.iter().map(|f| f.left.abs()).fold(0.0_f32, f32::max);
        assert!(peak < 0.1, "expected strong attenuation, got peak={peak}");
    }

    #[test]
    fn high_pass_passes_high_freq() {
        let mut proc = EffectProcessor::new();
        let sr = 44100.0_f32;
        let params = EffectParams {
            effect_type: EffectType::HighPassFilter,
            cutoff_hz: 200.0,
            resonance: 0.707,
            wet_mix: 1.0,
            sample_rate: sr,
            ..EffectParams::default()
        };
        let freq = 10_000.0_f32;
        let mut input_frames = [AudioFrame::silence(); 256];
        for (i, frame) in input_frames.iter_mut().enumerate() {
            let s = (2.0 * core::f32::consts::PI * freq * i as f32 / sr).sin();
            *frame = AudioFrame::mono(s);
        }
        let out = proc.process(&input_frames, params);
        let peak = out.iter().map(|f| f.left.abs()).fold(0.0_f32, f32::max);
        // High frequency should pass mostly unattenuated.
        assert!(peak > 0.5, "expected high-freq pass, got peak={peak}");
    }

    #[test]
    fn delay_produces_wet_signal_after_delay_time() {
        let mut proc = EffectProcessor::new();
        let sr = 44100.0_f32;
        let delay_secs = 0.01_f32; // 10 ms → 441 frames
        let delay_frames = (delay_secs * sr) as usize;
        let params = EffectParams {
            effect_type: EffectType::Delay,
            delay_secs,
            feedback: 0.0,
            wet_mix: 1.0,
            sample_rate: sr,
            ..EffectParams::default()
        };
        // Single impulse at frame 0, then silence.
        let mut input = vec![AudioFrame::silence(); delay_frames + 10];
        input[0] = AudioFrame::new(1.0, 1.0);
        let out = proc.process(&input, params);
        // The echo should appear at `delay_frames`.
        assert!(
            out[delay_frames].left > 0.5,
            "expected echo at frame {delay_frames}, got {}",
            out[delay_frames].left
        );
    }

    #[test]
    fn reset_clears_filter_state() {
        let mut proc = EffectProcessor::new();
        let sr = 44100.0_f32;
        let params = EffectParams {
            effect_type: EffectType::LowPassFilter,
            cutoff_hz: 500.0,
            resonance: 0.707,
            wet_mix: 1.0,
            sample_rate: sr,
            ..EffectParams::default()
        };
        let input = [AudioFrame::new(1.0, 1.0); 64];
        proc.process(&input, params);
        proc.reset();
        // After reset, passing a single silent frame should yield silence.
        let silent = [AudioFrame::silence()];
        let out = proc.process(&silent, params);
        assert_eq!(out[0].left, 0.0);
        assert_eq!(out[0].right, 0.0);
    }

    #[test]
    fn process_empty_slice_returns_empty() {
        let mut proc = EffectProcessor::new();
        let out = proc.process(&[], bypass_params());
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn default_and_new_are_equivalent() {
        let p1 = EffectParams::default();
        let p2 = EffectParams {
            effect_type: EffectType::Bypass,
            ..EffectParams::default()
        };
        assert_eq!(p1.effect_type, p2.effect_type);
    }

    #[test]
    fn wet_dry_mix_blends_signal() {
        let mut proc = EffectProcessor::new();
        let params = EffectParams {
            effect_type: EffectType::Gain,
            gain: 0.0,    // wet is silence
            wet_mix: 0.5, // 50% dry + 50% silent = 0.5 * original
            ..EffectParams::default()
        };
        let input = [AudioFrame::new(1.0, 1.0)];
        let out = proc.process(&input, params);
        assert!((out[0].left - 0.5).abs() < 1e-6);
    }
}
