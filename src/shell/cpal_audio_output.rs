// path: src/shell/cpal_audio_output.rs

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Producer, RingBuffer};

use crate::kernel::audio_frame::AudioFrame;
use crate::kernel::sample_rate::SampleRate;
use crate::shell::audio_output::{AudioOutput, AudioStream};

/// Ring-buffer capacity in stereo frames.
const RING_FRAMES: usize = 8192;

/// cpal-backed implementation of [`AudioOutput`].
///
/// The ring buffer (rtrb) is the lock-free seam between the writing thread and
/// the cpal audio callback:
///
/// - **Producer half** lives inside `CpalAudioOutput` and is driven from the
///   calling thread (e.g. the main / UI thread) via [`write_buffer`].
/// - **Consumer half** is owned by the cpal data callback closure; it drains
///   the ring and fills any shortfall with silence so the callback never
///   underruns or blocks.
///
/// # Audio-thread safety
/// The cpal callback only uses the `Consumer` end of the rtrb ring, which is
/// `Send`, lock-free, and performs no heap allocation on the hot path.
///
/// # cpal::Stream and Send
/// `cpal::Stream` is **not** `Send` on macOS (CoreAudio). `CpalAudioOutput`
/// therefore also opts out of `Send` so callers are prevented from moving it
/// across thread boundaries at compile time. Keep this struct on the thread
/// that created it.
pub struct CpalAudioOutput {
    /// Producer end of the lock-free ring buffer.
    producer: Option<Producer<f32>>,
    /// The live cpal stream; kept alive for the duration of playback.
    _stream: Option<cpal::Stream>,
    /// Monotonically increasing count of f32 samples silently dropped because
    /// the ring was full. Never printed on the hot path.
    dropped_frames: usize,
}

// Explicitly NOT Send — cpal::Stream is !Send on macOS.
// Safety: this type must stay on the thread that created it.

impl CpalAudioOutput {
    /// Creates a new `CpalAudioOutput` without opening a stream yet.
    pub fn new() -> Self {
        Self {
            producer: None,
            _stream: None,
            dropped_frames: 0,
        }
    }

    /// Returns how many stereo frames have been silently dropped due to
    /// ring-buffer overflow. Intended for diagnostics only; never polled on
    /// the audio thread.
    pub fn dropped_frames(&self) -> usize {
        self.dropped_frames
    }
}

impl Default for CpalAudioOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioOutput for CpalAudioOutput {
    /// Opens the cpal default output device and starts the audio stream.
    ///
    /// A fresh ring buffer is created; the previous stream (if any) is
    /// dropped first, which stops the old callback.
    fn open_stream(&mut self, sample_rate: SampleRate) -> AudioStream {
        // Drop any existing stream before replacing it.
        self._stream = None;
        self.producer = None;

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");

        // Build a config at the requested sample rate, stereo, f32.
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(sample_rate.value()),
            buffer_size: cpal::BufferSize::Default,
        };

        // Allocate the ring buffer: capacity is RING_FRAMES * 2 f32 slots
        // (interleaved L, R).
        let (producer, mut consumer) = RingBuffer::<f32>::new(RING_FRAMES * 2);

        let stream = device
            .build_output_stream(
                &config,
                // Data callback — runs on the audio thread.
                move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Drain what is available, fill the rest with silence.
                    let available = consumer.slots();
                    let to_read = available.min(output.len());

                    // SAFETY: rtrb's read_chunk is infallible when `to_read
                    // <= consumer.slots()`.
                    if to_read > 0 {
                        if let Ok(chunk) = consumer.read_chunk(to_read) {
                            let (head, tail) = chunk.as_slices();
                            output[..head.len()].copy_from_slice(head);
                            output[head.len()..head.len() + tail.len()].copy_from_slice(tail);
                            chunk.commit_all();
                        }
                    }

                    // Fill any remaining slots with silence.
                    let filled = to_read;
                    for sample in &mut output[filled..] {
                        *sample = 0.0;
                    }
                },
                // Error callback — not on the hot path.
                |err| {
                    eprintln!("cpal stream error: {err}");
                },
                None,
            )
            .expect("failed to build output stream");

        stream.play().expect("failed to start output stream");

        self.producer = Some(producer);
        self._stream = Some(stream);
        self.dropped_frames = 0;

        AudioStream::new(sample_rate)
    }

    /// Pushes interleaved L,R samples into the ring buffer.
    ///
    /// Frames that do not fit because the ring is full are silently discarded;
    /// only an internal counter is incremented.  Callers that respect
    /// [`available_frames`][Self::available_frames] will never overflow.
    fn write_buffer(&mut self, frames: &[AudioFrame]) {
        let producer = match self.producer.as_mut() {
            Some(p) => p,
            None => return,
        };

        for frame in frames {
            // Two f32 slots per stereo frame.
            if producer.slots() >= 2 {
                // push is infallible when slots() >= 1.
                let _ = producer.push(frame.left);
                let _ = producer.push(frame.right);
            } else {
                self.dropped_frames += 1;
            }
        }
    }

    /// Returns the number of whole stereo frames of free space in the ring
    /// buffer (producer free f32 slots / 2).
    ///
    /// This is cheap, non-blocking, and safe to call from any thread.
    fn available_frames(&self) -> usize {
        match &self.producer {
            Some(p) => p.slots() / 2,
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_zero_available_frames() {
        let output = CpalAudioOutput::new();
        assert_eq!(output.available_frames(), 0);
    }

    #[test]
    fn new_has_zero_dropped_frames() {
        let output = CpalAudioOutput::new();
        assert_eq!(output.dropped_frames(), 0);
    }

    #[test]
    fn write_buffer_before_open_is_noop() {
        let mut output = CpalAudioOutput::new();
        // Should not panic — producer is None.
        output.write_buffer(&[AudioFrame::mono(1.0); 4]);
        assert_eq!(output.dropped_frames(), 0);
    }

    #[test]
    fn default_equals_new() {
        let a = CpalAudioOutput::new();
        let b = CpalAudioOutput::default();
        // Both have no producer and no dropped frames.
        assert_eq!(a.available_frames(), 0);
        assert_eq!(b.available_frames(), 0);
        assert_eq!(a.dropped_frames(), 0);
        assert_eq!(b.dropped_frames(), 0);
    }
}
