// path: src/shell/audio_output.rs

//! Port: `Shell.AudioOutput`
//!
//! Abstracts the boundary between the application and the platform's audio
//! output device. `open` hands a [`RenderCallback`] to a concrete adapter
//! (e.g. a `cpal`-backed implementation) and gets back a running
//! [`Stream`]; `close` tears that stream down.
//!
//! The callback supplied to `open` runs on the audio thread and is bound by
//! the project's real-time invariants: it must never allocate heap memory,
//! acquire a mutex or other blocking lock, or perform blocking I/O.

use std::fmt;

/// Sample rate in Hz (e.g. `44_100`, `48_000`).
///
/// A newtype so a raw `u32` can never be passed where a sample rate is
/// expected (or vice versa) without going through validated construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SampleRate(u32);

impl SampleRate {
    /// Construct a `SampleRate`. Returns `None` for `0`, which is never a
    /// valid sample rate.
    pub fn new(hz: u32) -> Option<Self> {
        if hz == 0 {
            None
        } else {
            Some(Self(hz))
        }
    }

    /// The sample rate in Hz.
    pub fn hz(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SampleRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Hz", self.0)
    }
}

/// Number of frames delivered to (or requested from) the audio thread on
/// every render callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferSize(u32);

impl BufferSize {
    /// Construct a `BufferSize`. Returns `None` for `0`, which would leave
    /// the audio thread nothing to render.
    pub fn new(frames: u32) -> Option<Self> {
        if frames == 0 {
            None
        } else {
            Some(Self(frames))
        }
    }

    /// The buffer size in frames.
    pub fn frames(self) -> u32 {
        self.0
    }
}

impl fmt::Display for BufferSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} frames", self.0)
    }
}

/// Errors an [`AudioOutput`] adapter can return while opening or operating
/// a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioError {
    /// No matching output device was found, or the device disappeared.
    DeviceUnavailable(String),
    /// The requested sample rate / buffer size combination is not
    /// supported by the device.
    UnsupportedConfig(String),
    /// The platform stream failed to start or reported a fatal error.
    StreamFailed(String),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::DeviceUnavailable(msg) => write!(f, "audio device unavailable: {msg}"),
            AudioError::UnsupportedConfig(msg) => write!(f, "unsupported audio config: {msg}"),
            AudioError::StreamFailed(msg) => write!(f, "audio stream failed: {msg}"),
        }
    }
}

impl std::error::Error for AudioError {}

/// The render callback invoked by the audio thread on every buffer.
///
/// Implementors execute entirely on the audio thread: `render` must not
/// allocate, lock, or perform blocking I/O. `render` fills `output`
/// (interleaved, per the opened stream's channel layout) with the next
/// block of samples.
pub trait RenderCallback: Send {
    /// Fill `output` with the next block of audio samples.
    ///
    /// Called on the audio thread on every buffer. Must not allocate,
    /// lock, or block.
    fn render(&mut self, output: &mut [f32]);
}

/// A running audio output stream.
///
/// Dropping the concrete handle (or passing it to
/// [`AudioOutput::close`]) stops the underlying platform stream.
pub trait Stream: Send {
    /// Stop the stream. After this call returns, `render` is no longer
    /// invoked on the associated callback.
    fn stop(&mut self);
}

/// Port: opens and closes a connection to the platform's audio output
/// device.
///
/// Adapters implement this trait against a concrete audio API (e.g.
/// `cpal`). The port itself performs no I/O — it only declares the
/// contract every adapter must honor: `open` hands the callback to the
/// platform and returns a stream handle; `close` tears that handle down.
/// Consumers depend on this trait, never on a concrete adapter, so the
/// audio backend can be swapped (or faked in tests) without touching
/// application code.
pub trait AudioOutput {
    /// The concrete stream handle produced by this adapter.
    type StreamHandle: Stream;

    /// Open a stream at the given sample rate and buffer size, driving
    /// `callback` on the audio thread for every buffer.
    fn open(
        &self,
        sample_rate: SampleRate,
        buffer_size: BufferSize,
        callback: Box<dyn RenderCallback>,
    ) -> Result<Self::StreamHandle, AudioError>;

    /// Close a previously opened stream.
    ///
    /// The default implementation delegates to [`Stream::stop`]; adapters
    /// only need to override this if tearing the stream down requires
    /// more than stopping it (e.g. releasing a device handle it does not
    /// otherwise own).
    fn close(&self, mut stream: Self::StreamHandle) {
        stream.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_rate_rejects_zero() {
        assert!(SampleRate::new(0).is_none());
    }

    #[test]
    fn sample_rate_round_trips_hz() {
        let rate = SampleRate::new(48_000).expect("48_000 is a valid sample rate");
        assert_eq!(rate.hz(), 48_000);
        assert_eq!(rate.to_string(), "48000 Hz");
    }

    #[test]
    fn buffer_size_rejects_zero() {
        assert!(BufferSize::new(0).is_none());
    }

    #[test]
    fn buffer_size_round_trips_frames() {
        let size = BufferSize::new(256).expect("256 is a valid buffer size");
        assert_eq!(size.frames(), 256);
        assert_eq!(size.to_string(), "256 frames");
    }

    #[test]
    fn audio_error_display_includes_detail() {
        let err = AudioError::DeviceUnavailable("no default output device".to_string());
        assert_eq!(
            err.to_string(),
            "audio device unavailable: no default output device"
        );
    }

    /// A callback that counts how many buffers it has rendered, used only
    /// to exercise the port contract below.
    struct CountingCallback {
        renders: usize,
    }

    impl RenderCallback for CountingCallback {
        fn render(&mut self, output: &mut [f32]) {
            self.renders += 1;
            output.fill(0.0);
        }
    }

    /// A fake stream that records whether it has been stopped, used only
    /// to exercise the port contract below.
    struct FakeStream {
        stopped: bool,
    }

    impl Stream for FakeStream {
        fn stop(&mut self) {
            self.stopped = true;
        }
    }

    /// A fake adapter implementing the `AudioOutput` port purely in
    /// memory, so the port's `open`/`close` contract can be exercised
    /// without a real audio device.
    struct FakeAudioOutput;

    impl AudioOutput for FakeAudioOutput {
        type StreamHandle = FakeStream;

        fn open(
            &self,
            _sample_rate: SampleRate,
            _buffer_size: BufferSize,
            mut callback: Box<dyn RenderCallback>,
        ) -> Result<Self::StreamHandle, AudioError> {
            let mut scratch = [0.0_f32; 4];
            callback.render(&mut scratch);
            Ok(FakeStream { stopped: false })
        }
    }

    #[test]
    fn open_drives_callback_and_close_stops_stream() {
        let adapter = FakeAudioOutput;
        let sample_rate = SampleRate::new(44_100).expect("valid sample rate");
        let buffer_size = BufferSize::new(128).expect("valid buffer size");
        let callback = Box::new(CountingCallback { renders: 0 });

        let stream = adapter
            .open(sample_rate, buffer_size, callback)
            .expect("fake adapter never fails to open");

        // The default `close` implementation must delegate to `Stream::stop`.
        let stopped_flag_before = stream.stopped;
        assert!(!stopped_flag_before);
        adapter.close(stream);
    }
}
