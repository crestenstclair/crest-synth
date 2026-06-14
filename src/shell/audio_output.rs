// path: src/shell/audio_output.rs

use crate::kernel::audio_frame::AudioFrame;
use crate::kernel::sample_rate::SampleRate;

/// An opaque handle to an open audio stream.
///
/// Returned by [`AudioOutput::open_stream`]. Callers hold this handle for the
/// lifetime of the stream; dropping it closes the stream.
///
/// The handle is not cloneable — only one owner may drive the stream at a time.
pub struct AudioStream {
    pub(crate) sample_rate: SampleRate,
}

impl AudioStream {
    /// Creates a new `AudioStream` handle with the given sample rate.
    ///
    /// Intended for use by [`AudioOutput`] implementations only.
    pub fn new(sample_rate: SampleRate) -> Self {
        Self { sample_rate }
    }

    /// Returns the sample rate of this audio stream.
    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }
}

/// Port: audio output device.
///
/// Implementations wire a concrete audio back-end (e.g. `cpal`) behind this
/// interface so that higher-level code stays back-end agnostic.
///
/// # Contract
/// - `open_stream`: given a [`SampleRate`], open the hardware output and return
///   an [`AudioStream`] handle.
/// - `write_buffer`: enqueue a slice of [`AudioFrame`]s to the device ring
///   buffer.  Must be called from a non-audio thread; the implementation feeds
///   the ring buffer without blocking.
/// - `available_frames`: returns how many frames the ring buffer can currently
///   accept without blocking.  Use this to self-regulate feed pacing.
///
/// # Audio-thread constraints
/// Implementations **must not** allocate on the heap, take locks, or perform
/// blocking I/O on the audio callback thread.
pub trait AudioOutput {
    /// Opens the audio output stream at the requested sample rate.
    ///
    /// Returns an [`AudioStream`] handle that is valid until dropped.
    fn open_stream(&mut self, sample_rate: SampleRate) -> AudioStream;

    /// Writes a buffer of stereo frames to the audio output ring buffer.
    ///
    /// Frames that exceed the current ring-buffer capacity are silently
    /// discarded.  Callers should use [`available_frames`][Self::available_frames]
    /// to avoid overflow.
    fn write_buffer(&mut self, frames: &[AudioFrame]);

    /// Returns the number of frames the ring buffer can currently accept.
    ///
    /// Use this to pace calls to [`write_buffer`][Self::write_buffer] so that
    /// the audio thread is never starved or flooded.
    fn available_frames(&self) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::audio_frame::AudioFrame;
    use crate::kernel::sample_rate::SampleRate;

    // ── AudioStream ────────────────────────────────────────────────────────

    #[test]
    fn audio_stream_exposes_sample_rate() {
        let sr = SampleRate::try_new(44100).unwrap();
        let stream = AudioStream::new(sr);
        assert_eq!(stream.sample_rate().value(), 44100);
    }

    #[test]
    fn audio_stream_48k() {
        let sr = SampleRate::try_new(48000).unwrap();
        let stream = AudioStream::new(sr);
        assert_eq!(stream.sample_rate().value(), 48000);
    }

    // ── AudioOutput (stub impl) ────────────────────────────────────────────

    /// A minimal stub implementation used only within this test module.
    struct StubAudioOutput {
        buffer: Vec<AudioFrame>,
        capacity: usize,
    }

    impl StubAudioOutput {
        fn new(capacity: usize) -> Self {
            Self {
                buffer: Vec::new(),
                capacity,
            }
        }
    }

    impl AudioOutput for StubAudioOutput {
        fn open_stream(&mut self, sample_rate: SampleRate) -> AudioStream {
            AudioStream::new(sample_rate)
        }

        fn write_buffer(&mut self, frames: &[AudioFrame]) {
            let space = self.capacity.saturating_sub(self.buffer.len());
            let to_write = frames.len().min(space);
            self.buffer.extend_from_slice(&frames[..to_write]);
        }

        fn available_frames(&self) -> usize {
            self.capacity.saturating_sub(self.buffer.len())
        }
    }

    #[test]
    fn stub_open_stream_returns_correct_sample_rate() {
        let mut output = StubAudioOutput::new(1024);
        let sr = SampleRate::try_new(44100).unwrap();
        let stream = output.open_stream(sr);
        assert_eq!(stream.sample_rate().value(), 44100);
    }

    #[test]
    fn stub_write_buffer_fills_capacity() {
        let mut output = StubAudioOutput::new(4);
        assert_eq!(output.available_frames(), 4);

        let frames = [AudioFrame::mono(0.5); 3];
        output.write_buffer(&frames);

        assert_eq!(output.buffer.len(), 3);
        assert_eq!(output.available_frames(), 1);
    }

    #[test]
    fn stub_write_buffer_does_not_exceed_capacity() {
        let mut output = StubAudioOutput::new(2);
        let frames = [AudioFrame::mono(1.0); 5];
        output.write_buffer(&frames);

        // Only 2 frames should have been written (capacity limit).
        assert_eq!(output.buffer.len(), 2);
        assert_eq!(output.available_frames(), 0);
    }

    #[test]
    fn stub_write_empty_buffer_is_noop() {
        let mut output = StubAudioOutput::new(8);
        output.write_buffer(&[]);
        assert_eq!(output.available_frames(), 8);
    }

    #[test]
    fn audio_output_trait_is_object_safe() {
        let output: Box<dyn AudioOutput> = Box::new(StubAudioOutput::new(64));
        drop(output);
    }
}
