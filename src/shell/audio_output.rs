use core::any::Any;
use core::fmt;

/// The render function installed in an audio device callback.
///
/// The buffer always contains interleaved stereo `f32` frames. A native stereo
/// `f32` adapter may pass its device-owned buffer directly; adapters for other
/// formats or channel counts must use bounded storage allocated before startup.
/// The callback may run on a hard real-time thread, so callers must ensure its
/// captured state is safe to access without allocation, locking, blocking, I/O,
/// logging, or destruction.
pub type AudioRenderCallback = Box<dyn FnMut(&mut [f32], f32) + Send + 'static>;

/// Keeps an opened device stream alive on the control side.
///
/// The concrete stream is deliberately type-erased so platform-specific handles
/// do not leak through the port. Creating and dropping this value are control-side
/// operations; it must not be moved into or destroyed by the audio callback.
pub struct AudioStream {
    _handle: Box<dyn Any>,
}

impl AudioStream {
    /// Wraps the adapter's concrete stream handle.
    pub fn new<Handle>(handle: Handle) -> Self
    where
        Handle: 'static,
    {
        Self {
            _handle: Box::new(handle),
        }
    }
}

impl fmt::Debug for AudioStream {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AudioStream")
            .finish_non_exhaustive()
    }
}

/// A failure while selecting, configuring, opening, or starting audio output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioOutputError {
    message: String,
}

impl AudioOutputError {
    /// Creates an actionable control-side audio setup error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the adapter-provided failure description.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for AudioOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AudioOutputError {}

/// Outbound port for a low-latency audio device.
///
/// Implementations complete device discovery and setup in `open` before the
/// device may invoke `render`. The render buffer is always interleaved stereo
/// `f32`. A native stereo `f32` device may forward its caller-owned buffer;
/// a wider device must adapt through bounded preallocated callback storage,
/// write stereo to its first two channels, and silence every surplus channel.
pub trait AudioOutput {
    /// Opens and starts the default output device.
    fn open(&self, render: AudioRenderCallback) -> Result<AudioStream, AudioOutputError>;
}

#[cfg(test)]
mod tests {
    use super::{AudioOutput, AudioOutputError, AudioRenderCallback, AudioStream};
    use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
    use std::sync::Arc;

    struct TestAudioOutput {
        setup_complete: Arc<AtomicBool>,
        buffer_address: Arc<AtomicUsize>,
    }

    impl AudioOutput for TestAudioOutput {
        fn open(&self, mut render: AudioRenderCallback) -> Result<AudioStream, AudioOutputError> {
            self.setup_complete.store(true, Ordering::Release);
            let mut device_buffer = [0.0; 4];
            self.buffer_address
                .store(device_buffer.as_mut_ptr() as usize, Ordering::Release);
            render(&mut device_buffer, 48_000.0);
            Ok(AudioStream::new(()))
        }
    }

    #[test]
    fn port_is_object_safe_and_forwards_the_device_buffer_after_setup() {
        let setup_complete = Arc::new(AtomicBool::new(false));
        let expected_address = Arc::new(AtomicUsize::new(0));
        let observed_address = Arc::new(AtomicUsize::new(0));
        let observed_sample_rate = Arc::new(AtomicU32::new(0));

        let output = TestAudioOutput {
            setup_complete: Arc::clone(&setup_complete),
            buffer_address: Arc::clone(&expected_address),
        };
        let output: &dyn AudioOutput = &output;

        let callback_setup = Arc::clone(&setup_complete);
        let callback_address = Arc::clone(&observed_address);
        let callback_sample_rate = Arc::clone(&observed_sample_rate);
        let callback: AudioRenderCallback = Box::new(move |buffer, sample_rate| {
            assert!(callback_setup.load(Ordering::Acquire));
            assert_eq!(buffer.len() % 2, 0);
            callback_address.store(buffer.as_mut_ptr() as usize, Ordering::Release);
            callback_sample_rate.store(sample_rate.to_bits(), Ordering::Release);
        });

        output.open(callback).expect("test output should open");

        assert_eq!(
            observed_address.load(Ordering::Acquire),
            expected_address.load(Ordering::Acquire)
        );
        assert_eq!(
            f32::from_bits(observed_sample_rate.load(Ordering::Acquire)),
            48_000.0
        );
    }

    #[test]
    fn setup_error_preserves_its_actionable_message() {
        let error = AudioOutputError::new("default output device is unavailable");

        assert_eq!(error.message(), "default output device is unavailable");
        assert_eq!(error.to_string(), "default output device is unavailable");
    }
}
