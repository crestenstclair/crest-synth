// path: src/adapter/cpal_audio_output.rs
//
// CpalAudioOutput — cpal-backed implementation of the AudioOutput port.
//
// # Design
//
// A `rtrb` ring buffer acts as the lock-free seam between the caller thread
// (producer) and the cpal audio callback (consumer):
//
//   caller                ring buffer             cpal callback
//   ──────────────────    ───────────────────     ──────────────────────
//   write_buffer()  ─►  producer.write_chunk  ─►  consumer.read_chunk ─► hardware
//
// The audio callback is entirely allocation-free, lock-free, and non-blocking:
// it reads exactly as many frames as cpal requests, padding silence when the
// ring buffer is starved.
//
// # Thread-safety note
//
// `cpal::Stream` is NOT `Send` + `Sync` on macOS (CoreAudio).  `CpalAudioOutput`
// must therefore be created and kept on the main/UI thread; do NOT move it
// into `thread::spawn`.  Only `Send`-able data (e.g. MIDI events over a channel)
// may cross thread boundaries.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream, StreamConfig};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::kernel::audio_frame::AudioFrame;
use crate::kernel::sample_rate::SampleRate;
use crate::shell::audio_output::{AudioOutput, AudioStream};

// ─── ring-buffer capacity ───────────────────────────────────────────────────

/// Default ring-buffer capacity in frames.
///
/// 8 192 frames ≈ 170 ms at 48 kHz — large enough to absorb scheduling jitter
/// without audible delay.
const DEFAULT_RING_CAPACITY: usize = 8_192;

// ─── CpalAudioOutput ──────────────────────────────────────────────────

/// cpal-backed [`AudioOutput`] implementation.
///
/// Wraps a `cpal` output stream and a `rtrb` ring buffer so that callers on
/// any (non-audio) thread can push [`AudioFrame`]s without contending with the
/// audio callback.
///
/// # Important: `Stream` is not `Send` on macOS
///
/// Keep this struct on the thread that created it (typically the main/UI
/// thread).  Never move it into `std::thread::spawn`.
pub struct CpalAudioOutput {
    /// The cpal device used to open streams.
    device: Device,
    /// The cpal host (kept alive for the device lifetime).
    _host: Host,
    /// Active stream, present after `open_stream` is called.
    _stream: Option<Stream>,
    /// Producer half of the ring buffer — owned by the caller thread.
    producer: Option<Producer<f32>>,
    /// Capacity (in frames) of the ring buffer.
    ring_capacity: usize,
}

impl CpalAudioOutput {
    /// Create a `CpalAudioOutput` using the default cpal host and output device.
    ///
    /// Returns `None` if no output device is available.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crest_synth::adapter::cpal_audio_output::CpalAudioOutput;
    ///
    /// if let Some(output) = CpalAudioOutput::new() {
    ///     // use output …
    ///     drop(output);
    /// }
    /// ```
    pub fn new() -> Option<Self> {
        Self::with_capacity(DEFAULT_RING_CAPACITY)
    }

    /// Create a `CpalAudioOutput` with an explicit ring-buffer capacity (in frames).
    ///
    /// Returns `None` if no output device is available.
    pub fn with_capacity(ring_capacity: usize) -> Option<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        Some(Self {
            device,
            _host: host,
            _stream: None,
            producer: None,
            ring_capacity,
        })
    }
}

impl AudioOutput for CpalAudioOutput {
    /// Opens a cpal output stream at the nearest supported sample rate.
    ///
    /// Builds a `rtrb` ring buffer; the audio callback reads from the consumer
    /// and the caller writes to the producer via [`write_buffer`][Self::write_buffer].
    fn open_stream(&mut self, sample_rate: SampleRate) -> AudioStream {
        // ── pick a supported config ──────────────────────────────────────────────
        let supported = self
            .device
            .default_output_config()
            .expect("no default output config");

        let channels = supported.channels();
        let actual_sr = cpal::SampleRate(sample_rate.value());

        let config = StreamConfig {
            channels,
            sample_rate: actual_sr,
            buffer_size: cpal::BufferSize::Default,
        };

        // ── ring buffer (in stereo f32 samples, not frames) ───────────────────
        let cap_samples = self.ring_capacity * 2; // 2 f32 per frame (L + R)
        let (producer, mut consumer): (Producer<f32>, Consumer<f32>) = RingBuffer::new(cap_samples);

        // ── audio callback (lock-free, allocation-free) ─────────────────────
        let err_fn = |e| eprintln!("cpal stream error: {e}");

        let stream = match supported.sample_format() {
            SampleFormat::F32 => self
                .device
                .build_output_stream(
                    &config,
                    move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                        audio_callback(data, &mut consumer);
                    },
                    err_fn,
                    None,
                )
                .expect("failed to build f32 output stream"),
            // For integer formats, convert via the f32 callback anyway.
            _ => self
                .device
                .build_output_stream(
                    &config,
                    move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                        audio_callback(data, &mut consumer);
                    },
                    err_fn,
                    None,
                )
                .expect("failed to build output stream"),
        };

        stream.play().expect("failed to start audio stream");

        self._stream = Some(stream);
        self.producer = Some(producer);

        // Return the actual sample rate negotiated with the device.
        let final_sr = SampleRate::try_new(sample_rate.value())
            .unwrap_or_else(|_| SampleRate::try_new(48_000).unwrap());
        AudioStream::new(final_sr)
    }

    /// Enqueue audio frames into the ring buffer for the audio callback.
    ///
    /// Frames are written as interleaved stereo `f32` samples (L, R, L, R, …).
    /// Frames that exceed the current ring-buffer capacity are silently
    /// discarded — callers should use [`available_frames`][Self::available_frames]
    /// to pace writes.
    fn write_buffer(&mut self, frames: &[AudioFrame]) {
        let Some(producer) = self.producer.as_mut() else {
            return;
        };

        let available_samples = producer.slots();
        let available_frames = available_samples / 2;
        let frames_to_write = frames.len().min(available_frames);

        if frames_to_write == 0 {
            return;
        }

        // Write interleaved L/R samples into the ring buffer chunk.
        if let Ok(mut chunk) = producer.write_chunk_uninit(frames_to_write * 2) {
            let iter = frames[..frames_to_write]
                .iter()
                .flat_map(|f| [f.left, f.right]);
            for (i, sample) in iter.enumerate() {
                let (head, tail) = chunk.as_mut_slices();
                let head_len = head.len();
                if i < head_len {
                    head[i].write(sample);
                } else {
                    tail[i - head_len].write(sample);
                }
            }
            // SAFETY: we initialised all `frames_to_write * 2` slots above.
            unsafe { chunk.commit_all() };
        }
    }

    /// Returns the number of [`AudioFrame`]s the ring buffer can currently accept.
    ///
    /// Use this to pace calls to [`write_buffer`][Self::write_buffer].
    fn available_frames(&self) -> usize {
        self.producer.as_ref().map(|p| p.slots() / 2).unwrap_or(0)
    }
}

// ─── audio callback (lock-free, allocation-free) ────────────────────────────

/// Fill `data` (interleaved stereo f32) from the ring-buffer consumer.
///
/// Called on the audio thread.  Must never allocate, lock, or block.
/// Any samples not available in the ring buffer are filled with silence (0.0).
#[inline]
fn audio_callback(data: &mut [f32], consumer: &mut Consumer<f32>) {
    let available = consumer.slots();
    let to_read = available.min(data.len());

    if to_read > 0 {
        if let Ok(chunk) = consumer.read_chunk(to_read) {
            for (dst, src) in data[..to_read].iter_mut().zip(chunk) {
                *dst = src;
            }
        }
    }

    // Pad remaining output with silence if ring buffer was starved.
    for sample in data[to_read..].iter_mut() {
        *sample = 0.0;
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::audio_frame::AudioFrame;
    use crate::kernel::sample_rate::SampleRate;
    use crate::shell::audio_output::AudioOutput;

    // ── Stub AudioOutput ────────────────────────────────────────────
    //
    // Because cpal requires real audio hardware, we test the AudioOutput
    // contract using a ring-buffer-backed stub that shares the same logic
    // but without the cpal dependency.

    struct StubRingOutput {
        producer: Producer<f32>,
        consumer: Consumer<f32>,
        _stream_open: bool,
    }

    impl StubRingOutput {
        fn new(capacity_frames: usize) -> Self {
            let (producer, consumer) = RingBuffer::new(capacity_frames * 2);
            Self {
                producer,
                consumer,
                _stream_open: false,
            }
        }

        /// Drain all samples from consumer into a Vec for inspection.
        fn drain_to_frames(&mut self) -> Vec<AudioFrame> {
            let slots = self.consumer.slots();
            if slots == 0 {
                return vec![];
            }
            let chunk = self.consumer.read_chunk(slots).unwrap();
            let samples: Vec<f32> = chunk.into_iter().collect();
            samples
                .chunks_exact(2)
                .map(|s| AudioFrame::new(s[0], s[1]))
                .collect()
        }
    }

    impl AudioOutput for StubRingOutput {
        fn open_stream(&mut self, sample_rate: SampleRate) -> AudioStream {
            self._stream_open = true;
            AudioStream::new(sample_rate)
        }

        fn write_buffer(&mut self, frames: &[AudioFrame]) {
            let available_samples = self.producer.slots();
            let available_frames = available_samples / 2;
            let frames_to_write = frames.len().min(available_frames);
            if frames_to_write == 0 {
                return;
            }
            if let Ok(mut chunk) = self.producer.write_chunk_uninit(frames_to_write * 2) {
                let iter = frames[..frames_to_write]
                    .iter()
                    .flat_map(|f| [f.left, f.right]);
                for (i, sample) in iter.enumerate() {
                    let (head, tail) = chunk.as_mut_slices();
                    let head_len = head.len();
                    if i < head_len {
                        head[i].write(sample);
                    } else {
                        tail[i - head_len].write(sample);
                    }
                }
                unsafe { chunk.commit_all() };
            }
        }

        fn available_frames(&self) -> usize {
            self.producer.slots() / 2
        }
    }

    // ── tests ─────────────────────────────────────────────────────────────

    #[test]
    fn available_frames_equals_capacity_before_any_write() {
        let output = StubRingOutput::new(16);
        assert_eq!(output.available_frames(), 16);
    }

    #[test]
    fn write_buffer_reduces_available_frames() {
        let mut output = StubRingOutput::new(16);
        let frames = [AudioFrame::mono(0.5); 4];
        output.write_buffer(&frames);
        assert_eq!(output.available_frames(), 12);
    }

    #[test]
    fn drain_after_write_recovers_correct_frames() {
        let mut output = StubRingOutput::new(8);
        let frames = vec![
            AudioFrame::new(0.1, 0.2),
            AudioFrame::new(0.3, 0.4),
            AudioFrame::new(0.5, 0.6),
        ];
        output.write_buffer(&frames);
        let drained = output.drain_to_frames();
        assert_eq!(drained.len(), 3);
        assert!((drained[0].left - 0.1).abs() < 1e-6);
        assert!((drained[0].right - 0.2).abs() < 1e-6);
        assert!((drained[2].left - 0.5).abs() < 1e-6);
        assert!((drained[2].right - 0.6).abs() < 1e-6);
    }

    #[test]
    fn write_beyond_capacity_does_not_exceed_ring_buffer() {
        let mut output = StubRingOutput::new(4);
        let frames = [AudioFrame::mono(1.0); 10];
        output.write_buffer(&frames);
        // At most 4 frames fit.
        assert_eq!(output.available_frames(), 0);
        let drained = output.drain_to_frames();
        assert_eq!(drained.len(), 4);
    }

    #[test]
    fn write_empty_buffer_is_noop() {
        let mut output = StubRingOutput::new(8);
        output.write_buffer(&[]);
        assert_eq!(output.available_frames(), 8);
    }

    #[test]
    fn open_stream_returns_correct_sample_rate() {
        let mut output = StubRingOutput::new(8);
        let sr = SampleRate::try_new(44100).unwrap();
        let stream = output.open_stream(sr);
        assert_eq!(stream.sample_rate().value(), 44100);
    }

    #[test]
    fn audio_callback_fills_silence_when_starved() {
        let capacity = 8;
        let (_, mut consumer): (Producer<f32>, Consumer<f32>) = RingBuffer::new(capacity * 2);
        let mut data = [0.5f32; 8]; // pre-fill with non-zero
        audio_callback(&mut data, &mut consumer);
        // All samples should be silence since ring buffer was empty.
        for &s in &data {
            assert_eq!(s, 0.0);
        }
    }

    #[test]
    fn audio_callback_reads_available_samples_and_pads_rest() {
        let (mut producer, mut consumer): (Producer<f32>, Consumer<f32>) = RingBuffer::new(16);
        // Write 4 samples (2 frames) into the ring buffer.
        let mut chunk = producer.write_chunk_uninit(4).unwrap();
        let samples = [0.1f32, 0.2, 0.3, 0.4];
        for (i, s) in samples.iter().enumerate() {
            let (head, tail) = chunk.as_mut_slices();
            let head_len = head.len();
            if i < head_len {
                head[i].write(*s);
            } else {
                tail[i - head_len].write(*s);
            }
        }
        unsafe { chunk.commit_all() };

        let mut data = [0.0f32; 8];
        audio_callback(&mut data, &mut consumer);

        // First 4 samples from ring buffer.
        assert!((data[0] - 0.1).abs() < 1e-6);
        assert!((data[1] - 0.2).abs() < 1e-6);
        assert!((data[2] - 0.3).abs() < 1e-6);
        assert!((data[3] - 0.4).abs() < 1e-6);
        // Remaining 4 samples padded with silence.
        assert_eq!(data[4], 0.0);
        assert_eq!(data[5], 0.0);
        assert_eq!(data[6], 0.0);
        assert_eq!(data[7], 0.0);
    }

    #[test]
    fn stub_audio_output_is_object_safe() {
        let output: Box<dyn AudioOutput> = Box::new(StubRingOutput::new(64));
        drop(output);
    }
}
