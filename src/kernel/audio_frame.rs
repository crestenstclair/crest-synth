/// One stereo sample pair.
///
/// `AudioFrame` holds a left and right channel sample as 32-bit floats.
/// It is a pure value type with no heap allocation, suitable for use on
/// the audio thread.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AudioFrame {
    pub left: f32,
    pub right: f32,
}

impl AudioFrame {
    /// Create a new `AudioFrame` from left and right channel samples.
    pub fn new(left: f32, right: f32) -> Self {
        Self { left, right }
    }

    /// Create a silent (zero-amplitude) `AudioFrame`.
    pub fn silence() -> Self {
        Self::default()
    }

    /// Create a mono `AudioFrame` where both channels carry the same sample.
    pub fn mono(sample: f32) -> Self {
        Self {
            left: sample,
            right: sample,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_left_and_right() {
        let frame = AudioFrame::new(0.5, -0.5);
        assert_eq!(frame.left, 0.5);
        assert_eq!(frame.right, -0.5);
    }

    #[test]
    fn silence_is_zero() {
        let frame = AudioFrame::silence();
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }

    #[test]
    fn mono_copies_sample_to_both_channels() {
        let frame = AudioFrame::mono(0.75);
        assert_eq!(frame.left, 0.75);
        assert_eq!(frame.right, 0.75);
    }

    #[test]
    fn default_is_silence() {
        let frame = AudioFrame::default();
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }

    #[test]
    fn copy_semantics() {
        let a = AudioFrame::new(1.0, 2.0);
        let b = a;
        assert_eq!(a.left, b.left);
        assert_eq!(a.right, b.right);
    }
}
