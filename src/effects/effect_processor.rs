// path: src/effects/effect_processor.rs

//! `EffectProcessor` is the port every audio effect (chorus, EQ band,
//! compressor, delay, reverb, ...) implements. It is consumed exclusively
//! from the real-time audio thread, so every implementor's `process` must
//! be allocation-free, lock-free, and non-blocking in its steady state.
//!
//! `AudioFrame` does not yet exist elsewhere in the crate's module tree, so
//! it is defined locally here as the value type this port operates on.

/// A single frame of audio: one sample per channel, interleaved as a pair
/// for stereo processing. Effects operate on contiguous slices of frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioFrame {
    /// Left channel (or mono) sample.
    pub left: f32,
    /// Right channel sample. Mirrors `left` for mono sources.
    pub right: f32,
}

impl AudioFrame {
    /// Construct a stereo frame from explicit left/right samples.
    pub fn new(left: f32, right: f32) -> Self {
        Self { left, right }
    }

    /// Construct a mono frame (left and right carry the same sample).
    pub fn mono(sample: f32) -> Self {
        Self {
            left: sample,
            right: sample,
        }
    }

    /// The silent frame, useful for padding and default buffer fill.
    pub fn silence() -> Self {
        Self {
            left: 0.0,
            right: 0.0,
        }
    }
}

impl Default for AudioFrame {
    fn default() -> Self {
        Self::silence()
    }
}

/// Port implemented by every effect in an `EffectChain` insert slot.
///
/// # Real-time contract
///
/// Implementors are invoked from the audio callback. `process` must not
/// allocate heap memory, acquire a mutex or other blocking lock, or perform
/// blocking I/O. Any parameter change originating off the audio thread must
/// arrive through the `ParameterBridge` or the `EventRing` — never by an
/// implementor reaching out to shared mutable state directly.
pub trait EffectProcessor {
    /// Fixed processing latency introduced by this effect, in frames.
    ///
    /// Used by the host to align dry/wet paths and compensate downstream
    /// timing. Effects with no lookahead or internal buffering return `0`.
    fn latency(&self) -> u32;

    /// Process one block of audio and return the transformed frames.
    ///
    /// The returned `Vec` has the same length as `input`. Implementations
    /// intended for the audio thread must back the returned buffer with
    /// scratch space pre-allocated at construction time (growing it only
    /// up to the largest block size seen) so that, after warm-up, no
    /// further heap allocation occurs on the audio thread.
    fn process(&mut self, input: &[AudioFrame]) -> Vec<AudioFrame>;

    /// Clear all internal state (delay lines, filter memory, envelope
    /// followers, etc.) as though the effect were freshly constructed.
    ///
    /// Must not allocate or block; any buffers to clear are already owned
    /// by `self` and are reset in place.
    fn reset(&mut self);
}

/// A no-op effect that copies input straight to output unmodified.
///
/// Useful as a default/bypass slot in an `EffectChain` and as a reference
/// implementation for testing the `EffectProcessor` contract.
///
/// # Allocation behaviour
///
/// The internal scratch buffer is grown (via `Vec::resize`) only when a
/// block larger than any seen before arrives. After warm-up to the largest
/// block size in use, `process` performs no further allocation.
pub struct PassthroughEffect {
    scratch: Vec<AudioFrame>,
}

impl PassthroughEffect {
    /// Construct a new passthrough effect with an empty scratch buffer.
    /// The buffer grows lazily on the first call to `process`.
    pub fn new() -> Self {
        Self {
            scratch: Vec::new(),
        }
    }
}

impl Default for PassthroughEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectProcessor for PassthroughEffect {
    fn latency(&self) -> u32 {
        0
    }

    fn process(&mut self, input: &[AudioFrame]) -> Vec<AudioFrame> {
        if self.scratch.len() < input.len() {
            self.scratch.resize(input.len(), AudioFrame::silence());
        }
        self.scratch[..input.len()].copy_from_slice(input);
        self.scratch[..input.len()].to_vec()
    }

    fn reset(&mut self) {
        for frame in self.scratch.iter_mut() {
            *frame = AudioFrame::silence();
        }
    }
}

/// A fixed-gain effect used in tests to prove `process` actually transforms
/// samples (as opposed to merely copying them, which `PassthroughEffect`
/// already covers).
#[derive(Debug, Clone, Copy)]
pub struct GainEffect {
    gain: f32,
}

impl GainEffect {
    /// Construct a gain stage that scales every sample by `gain`.
    pub fn new(gain: f32) -> Self {
        Self { gain }
    }
}

impl EffectProcessor for GainEffect {
    fn latency(&self) -> u32 {
        0
    }

    fn process(&mut self, input: &[AudioFrame]) -> Vec<AudioFrame> {
        input
            .iter()
            .map(|frame| AudioFrame::new(frame.left * self.gain, frame.right * self.gain))
            .collect()
    }

    fn reset(&mut self) {
        // Gain has no accumulated state; nothing to clear.
    }
}

/// A one-sample delay used in tests to prove `reset` actually clears
/// internal state rather than being a no-op stub.
#[derive(Debug, Clone)]
pub struct OneSampleDelay {
    last: AudioFrame,
}

impl OneSampleDelay {
    /// Construct a one-sample delay line, initially holding silence.
    pub fn new() -> Self {
        Self {
            last: AudioFrame::silence(),
        }
    }
}

impl Default for OneSampleDelay {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectProcessor for OneSampleDelay {
    fn latency(&self) -> u32 {
        1
    }

    fn process(&mut self, input: &[AudioFrame]) -> Vec<AudioFrame> {
        let mut out = Vec::with_capacity(input.len());
        let mut carry = self.last;
        for frame in input {
            out.push(carry);
            carry = *frame;
        }
        self.last = carry;
        out
    }

    fn reset(&mut self) {
        self.last = AudioFrame::silence();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(samples: &[(f32, f32)]) -> Vec<AudioFrame> {
        samples
            .iter()
            .map(|&(l, r)| AudioFrame::new(l, r))
            .collect()
    }

    #[test]
    fn audio_frame_mono_mirrors_channel() {
        let frame = AudioFrame::mono(0.5);
        assert_eq!(frame.left, 0.5);
        assert_eq!(frame.right, 0.5);
    }

    #[test]
    fn audio_frame_silence_is_zero() {
        let frame = AudioFrame::silence();
        assert_eq!(frame, AudioFrame::new(0.0, 0.0));
    }

    #[test]
    fn passthrough_latency_is_zero() {
        let effect = PassthroughEffect::new();
        assert_eq!(effect.latency(), 0);
    }

    #[test]
    fn passthrough_copies_input_to_output() {
        let mut effect = PassthroughEffect::new();
        let input = frames(&[(0.1, -0.1), (0.2, -0.2)]);

        let output = effect.process(&input);

        assert_eq!(output, input);
    }

    #[test]
    fn passthrough_process_empty_slice_returns_empty() {
        let mut effect = PassthroughEffect::new();
        let output = effect.process(&[]);
        assert_eq!(output.len(), 0);
    }

    #[test]
    fn passthrough_processing_smaller_block_does_not_grow_capacity() {
        let mut effect = PassthroughEffect::new();
        let big = vec![AudioFrame::new(1.0, 1.0); 8];
        effect.process(&big);
        let cap_after_big = effect.scratch.capacity();

        let small = vec![AudioFrame::new(2.0, 2.0); 3];
        let output = effect.process(&small);

        assert_eq!(output.len(), 3);
        assert_eq!(effect.scratch.capacity(), cap_after_big);
    }

    #[test]
    fn passthrough_reset_clears_scratch_and_stays_functional() {
        let mut effect = PassthroughEffect::new();
        let input = frames(&[(0.5, 0.5); 4]);
        effect.process(&input);

        effect.reset();

        for frame in &effect.scratch {
            assert_eq!(*frame, AudioFrame::silence());
        }
        let output = effect.process(&input);
        assert_eq!(output, input);
    }

    #[test]
    fn gain_effect_scales_every_sample() {
        let mut effect = GainEffect::new(2.0);
        let input = frames(&[(0.25, 0.5)]);

        let output = effect.process(&input);

        assert_eq!(output[0], AudioFrame::new(0.5, 1.0));
    }

    #[test]
    fn gain_effect_reset_is_idempotent_no_op() {
        let mut effect = GainEffect::new(3.0);
        effect.reset();
        let input = frames(&[(1.0, 1.0)]);

        let output = effect.process(&input);

        assert_eq!(output[0], AudioFrame::new(3.0, 3.0));
    }

    #[test]
    fn one_sample_delay_shifts_output_by_one_frame() {
        let mut effect = OneSampleDelay::new();
        let input = frames(&[(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]);

        let output = effect.process(&input);

        assert_eq!(output, frames(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]));
    }

    #[test]
    fn one_sample_delay_reports_latency_of_one() {
        let effect = OneSampleDelay::new();
        assert_eq!(effect.latency(), 1);
    }

    #[test]
    fn one_sample_delay_reset_clears_carried_sample() {
        let mut effect = OneSampleDelay::new();
        let warm_up = frames(&[(9.0, 9.0)]);
        effect.process(&warm_up);

        effect.reset();

        let input = frames(&[(1.0, 1.0)]);
        let output = effect.process(&input);
        assert_eq!(output[0], AudioFrame::silence());
    }

    #[test]
    fn effect_processor_trait_is_object_safe() {
        let mut boxed: Box<dyn EffectProcessor> = Box::new(PassthroughEffect::new());
        assert_eq!(boxed.latency(), 0);
        let frames = [AudioFrame::new(0.1, 0.2)];
        let out = boxed.process(&frames);
        assert_eq!(out[0], AudioFrame::new(0.1, 0.2));
        boxed.reset();
    }
}
