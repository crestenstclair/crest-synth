// path: src/kernel/audio_frame.rs

//! `AudioFrame` — a single stereo sample pair.
//!
//! This is a plain value type: two `f32` samples, one per channel. It carries
//! no allocation, no locks, and no I/O, so it is safe to construct, copy, and
//! mix directly on the real-time audio thread.

use std::ops::{Add, AddAssign, Mul, MulAssign};

/// A single stereo sample pair (left, right).
///
/// `AudioFrame` is `Copy` because it is exactly two `f32`s -- cheap to pass by
/// value in the inner sample loop where dynamic dispatch and heap allocation
/// are forbidden.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AudioFrame {
    left: f32,
    right: f32,
}

impl AudioFrame {
    /// Construct a frame from explicit left/right samples.
    #[must_use]
    pub const fn new(left: f32, right: f32) -> Self {
        Self { left, right }
    }

    /// A frame with both channels at digital silence (0.0).
    #[must_use]
    pub const fn silence() -> Self {
        Self {
            left: 0.0,
            right: 0.0,
        }
    }

    /// Construct a frame from a single mono sample, duplicated to both
    /// channels.
    #[must_use]
    pub const fn from_mono(sample: f32) -> Self {
        Self {
            left: sample,
            right: sample,
        }
    }

    /// The left channel sample.
    #[must_use]
    pub const fn left(&self) -> f32 {
        self.left
    }

    /// The right channel sample.
    #[must_use]
    pub const fn right(&self) -> f32 {
        self.right
    }

    /// The arithmetic mean of the two channels.
    ///
    /// Callers that need equal-power mono downmixing should apply their own
    /// scaling; this method only performs the mean.
    #[must_use]
    pub fn mono(&self) -> f32 {
        (self.left + self.right) * 0.5
    }

    /// Return a new frame with both channels scaled by `gain`.
    ///
    /// This is a pure, allocation-free transform suitable for the audio
    /// thread (e.g. applying volume or send-level gain).
    #[must_use]
    pub fn scaled(&self, gain: f32) -> Self {
        Self {
            left: self.left * gain,
            right: self.right * gain,
        }
    }

    /// Return a new frame that is the sample-wise sum of `self` and `other`.
    ///
    /// Used to mix multiple sources into a single frame (e.g. summing voices
    /// or summing send taps into a bus) without any allocation.
    #[must_use]
    pub fn mixed_with(&self, other: Self) -> Self {
        Self {
            left: self.left + other.left,
            right: self.right + other.right,
        }
    }

    /// Return true if both channels are exactly digital silence (0.0).
    #[must_use]
    pub fn is_silent(&self) -> bool {
        self.left == 0.0 && self.right == 0.0
    }

    /// Apply an equal-power (constant-power) pan law, returning a new frame.
    ///
    /// `pan` is expected in `-1.0..=1.0` (full left to full right). Values
    /// outside that range are clamped rather than causing a panic, since this
    /// runs on the audio thread where panics must never occur.
    #[must_use]
    pub fn panned(&self, pan: f32) -> Self {
        let clamped = pan.clamp(-1.0, 1.0);
        // Constant-power pan law: angle sweeps 0..=PI/2 as pan sweeps -1..=1.
        let angle = (clamped + 1.0) * std::f32::consts::FRAC_PI_4;
        let left_gain = angle.cos();
        let right_gain = angle.sin();
        Self {
            left: self.left * left_gain,
            right: self.right * right_gain,
        }
    }
}

impl Add for AudioFrame {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.mixed_with(rhs)
    }
}

impl AddAssign for AudioFrame {
    fn add_assign(&mut self, rhs: Self) {
        self.left += rhs.left;
        self.right += rhs.right;
    }
}

impl Mul<f32> for AudioFrame {
    type Output = Self;

    fn mul(self, gain: f32) -> Self::Output {
        self.scaled(gain)
    }
}

impl MulAssign<f32> for AudioFrame {
    fn mul_assign(&mut self, gain: f32) {
        self.left *= gain;
        self.right *= gain;
    }
}

impl From<(f32, f32)> for AudioFrame {
    fn from(pair: (f32, f32)) -> Self {
        Self::new(pair.0, pair.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_left_and_right() {
        let frame = AudioFrame::new(0.25, -0.5);
        assert_eq!(frame.left(), 0.25);
        assert_eq!(frame.right(), -0.5);
    }

    #[test]
    fn default_and_silence_are_zeroed() {
        assert_eq!(AudioFrame::default(), AudioFrame::silence());
        assert!(AudioFrame::silence().is_silent());
    }

    #[test]
    fn from_mono_duplicates_sample_to_both_channels() {
        let frame = AudioFrame::from_mono(0.4);
        assert_eq!(frame.left(), 0.4);
        assert_eq!(frame.right(), 0.4);
    }

    #[test]
    fn mono_is_arithmetic_mean_of_channels() {
        let frame = AudioFrame::new(1.0, -1.0);
        assert_eq!(frame.mono(), 0.0);

        let frame = AudioFrame::new(0.5, 0.5);
        assert_eq!(frame.mono(), 0.5);
    }

    #[test]
    fn scaled_multiplies_both_channels() {
        let frame = AudioFrame::new(0.5, -0.5).scaled(2.0);
        assert_eq!(frame, AudioFrame::new(1.0, -1.0));
    }

    #[test]
    fn mixed_with_sums_channels() {
        let a = AudioFrame::new(0.1, 0.2);
        let b = AudioFrame::new(0.3, 0.4);
        assert_eq!(a.mixed_with(b), AudioFrame::new(0.4, 0.6));
    }

    #[test]
    fn add_operator_matches_mixed_with() {
        let a = AudioFrame::new(0.1, 0.2);
        let b = AudioFrame::new(0.3, 0.4);
        assert_eq!(a + b, a.mixed_with(b));
    }

    #[test]
    fn add_assign_accumulates_in_place() {
        let mut frame = AudioFrame::new(0.1, 0.1);
        frame += AudioFrame::new(0.2, 0.3);
        assert_eq!(frame, AudioFrame::new(0.3, 0.4));
    }

    #[test]
    fn mul_operator_scales_both_channels() {
        let frame = AudioFrame::new(0.5, -0.5) * 2.0;
        assert_eq!(frame, AudioFrame::new(1.0, -1.0));
    }

    #[test]
    fn mul_assign_scales_in_place() {
        let mut frame = AudioFrame::new(0.5, -0.5);
        frame *= 2.0;
        assert_eq!(frame, AudioFrame::new(1.0, -1.0));
    }

    #[test]
    fn is_silent_true_only_when_both_channels_zero() {
        assert!(AudioFrame::new(0.0, 0.0).is_silent());
        assert!(!AudioFrame::new(0.0001, 0.0).is_silent());
        assert!(!AudioFrame::new(0.0, -0.0001).is_silent());
    }

    #[test]
    fn panned_hard_left_silences_right_channel() {
        let frame = AudioFrame::new(1.0, 1.0).panned(-1.0);
        assert!((frame.left() - 1.0).abs() < 1e-6);
        assert!(frame.right().abs() < 1e-6);
    }

    #[test]
    fn panned_hard_right_silences_left_channel() {
        let frame = AudioFrame::new(1.0, 1.0).panned(1.0);
        assert!(frame.left().abs() < 1e-6);
        assert!((frame.right() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn panned_center_applies_equal_power_gain_to_both_channels() {
        let frame = AudioFrame::new(1.0, 1.0).panned(0.0);
        let expected = std::f32::consts::FRAC_1_SQRT_2;
        assert!((frame.left() - expected).abs() < 1e-6);
        assert!((frame.right() - expected).abs() < 1e-6);
    }

    #[test]
    fn panned_clamps_out_of_range_values() {
        let hard_left = AudioFrame::new(1.0, 1.0).panned(-5.0);
        let clamped_left = AudioFrame::new(1.0, 1.0).panned(-1.0);
        assert_eq!(hard_left, clamped_left);

        let hard_right = AudioFrame::new(1.0, 1.0).panned(5.0);
        let clamped_right = AudioFrame::new(1.0, 1.0).panned(1.0);
        assert_eq!(hard_right, clamped_right);
    }

    #[test]
    fn from_tuple_conversion() {
        let frame: AudioFrame = (0.2, 0.8).into();
        assert_eq!(frame, AudioFrame::new(0.2, 0.8));
    }
}
