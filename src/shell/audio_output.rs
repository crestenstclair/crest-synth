use core::any::Any;
use core::fmt;

/// The render function installed in an audio-device callback.
///
/// The buffer always contains interleaved stereo `f32` frames. The negotiated
/// [`AudioDeviceConfig`] is known before this callback is created, so the
/// callback never receives or reinterprets device configuration at run time.
pub type AudioRenderCallback = Box<dyn FnMut(&mut [f32]) + Send + 'static>;

/// A bounded, allocation-free notification emitted by a running device.
pub type AudioDeviceStatusCallback = Box<dyn FnMut(AudioDeviceRuntimeError) + Send + 'static>;

/// The PCM representation selected by an output adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioSampleFormat {
    F32,
    F64,
    I8,
    I16,
    I24,
    I32,
    I64,
    U8,
    U16,
    U24,
    U32,
    U64,
}

/// The only device-channel mapping supported by the current stereo renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioChannelMapping {
    /// Render stereo into the first two device channels and silence the rest.
    StereoToFirstTwo,
}

/// Validated device facts that must be known before graph preparation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioDeviceConfig {
    sample_rate: f32,
    channels: u16,
    sample_format: AudioSampleFormat,
    channel_mapping: AudioChannelMapping,
    render_capacity_frames: usize,
}

impl AudioDeviceConfig {
    pub fn new(
        sample_rate: f32,
        channels: u16,
        sample_format: AudioSampleFormat,
        render_capacity_frames: usize,
    ) -> Result<Self, AudioDeviceConfigError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(AudioDeviceConfigError::InvalidSampleRate);
        }
        if channels < 2 {
            return Err(AudioDeviceConfigError::InsufficientChannels);
        }
        if render_capacity_frames == 0 {
            return Err(AudioDeviceConfigError::InvalidRenderCapacity);
        }
        Ok(Self {
            sample_rate,
            channels,
            sample_format,
            channel_mapping: AudioChannelMapping::StereoToFirstTwo,
            render_capacity_frames,
        })
    }

    pub const fn sample_rate(self) -> f32 {
        self.sample_rate
    }

    pub const fn channels(self) -> u16 {
        self.channels
    }

    pub const fn sample_format(self) -> AudioSampleFormat {
        self.sample_format
    }

    pub const fn channel_mapping(self) -> AudioChannelMapping {
        self.channel_mapping
    }

    /// Maximum number of stereo frames submitted to the prepared graph in one
    /// render operation. Larger native callbacks are split into these bounded
    /// chunks by the renderer or format adapter.
    pub const fn render_capacity_frames(self) -> usize {
        self.render_capacity_frames
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioDeviceConfigError {
    InvalidSampleRate,
    InsufficientChannels,
    InvalidRenderCapacity,
}

impl fmt::Display for AudioDeviceConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSampleRate => "audio sample rate must be finite and positive",
            Self::InsufficientChannels => "audio output requires at least two channels",
            Self::InvalidRenderCapacity => "audio render capacity must be nonzero",
        })
    }
}

impl std::error::Error for AudioDeviceConfigError {}

/// Fixed-size device failures that may arrive after a stream has started.
///
/// The adapter maps its framework error to this enum in the device callback;
/// formatting and application behavior remain control-side work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AudioDeviceRuntimeError {
    DeviceBusy = 1,
    DeviceChanged = 2,
    DeviceUnavailable = 3,
    HostUnavailable = 4,
    InvalidInput = 5,
    PermissionDenied = 6,
    RealtimeDenied = 7,
    ResourceExhausted = 8,
    StreamInvalidated = 9,
    UnsupportedConfig = 10,
    UnsupportedOperation = 11,
    Xrun = 12,
    Backend = 13,
    Other = 14,
}

impl AudioDeviceRuntimeError {
    pub(crate) const fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            1 => Self::DeviceBusy,
            2 => Self::DeviceChanged,
            3 => Self::DeviceUnavailable,
            4 => Self::HostUnavailable,
            5 => Self::InvalidInput,
            6 => Self::PermissionDenied,
            7 => Self::RealtimeDenied,
            8 => Self::ResourceExhausted,
            9 => Self::StreamInvalidated,
            10 => Self::UnsupportedConfig,
            11 => Self::UnsupportedOperation,
            12 => Self::Xrun,
            13 => Self::Backend,
            14 => Self::Other,
            _ => return None,
        })
    }
}

impl fmt::Display for AudioDeviceRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::DeviceBusy => "audio device became busy",
            Self::DeviceChanged => "audio device configuration changed",
            Self::DeviceUnavailable => "audio device became unavailable",
            Self::HostUnavailable => "audio host became unavailable",
            Self::InvalidInput => "audio device reported invalid input",
            Self::PermissionDenied => "audio device permission was denied",
            Self::RealtimeDenied => "audio device denied real-time operation",
            Self::ResourceExhausted => "audio device resources were exhausted",
            Self::StreamInvalidated => "audio stream was invalidated",
            Self::UnsupportedConfig => "audio device configuration became unsupported",
            Self::UnsupportedOperation => "audio device operation became unsupported",
            Self::Xrun => "audio device reported a buffer overrun or underrun",
            Self::Backend => "audio backend failed",
            Self::Other => "audio device failed",
        })
    }
}

impl std::error::Error for AudioDeviceRuntimeError {}

/// Keeps an opened device stream alive on the control side.
pub struct AudioStream {
    _handle: Box<dyn Any>,
}

impl AudioStream {
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
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

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

/// A selected device/configuration that has not started its callback yet.
pub trait NegotiatedAudioOutput: Sized {
    fn config(&self) -> AudioDeviceConfig;

    /// Opens and starts the already-selected device only after the application
    /// has prepared a compatible graph and renderer.
    fn start(
        self,
        render: AudioRenderCallback,
        on_runtime_error: AudioDeviceStatusCallback,
    ) -> Result<AudioStream, AudioOutputError>;
}

/// Outbound audio port with an explicit control-side negotiation phase.
pub trait AudioOutput: Sized {
    type Negotiated: NegotiatedAudioOutput;

    /// Selects and validates a device configuration without starting a stream.
    fn negotiate(self) -> Result<Self::Negotiated, AudioOutputError>;
}

#[cfg(test)]
mod tests {
    use super::{
        AudioDeviceConfig, AudioDeviceRuntimeError, AudioOutput, AudioOutputError,
        AudioRenderCallback, AudioSampleFormat, AudioStream, NegotiatedAudioOutput,
    };
    use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    struct TestAudioOutput {
        negotiated: Arc<AtomicBool>,
        buffer_address: Arc<AtomicUsize>,
    }

    struct TestNegotiatedOutput {
        config: AudioDeviceConfig,
        negotiated: Arc<AtomicBool>,
        buffer_address: Arc<AtomicUsize>,
    }

    impl AudioOutput for TestAudioOutput {
        type Negotiated = TestNegotiatedOutput;

        fn negotiate(self) -> Result<Self::Negotiated, AudioOutputError> {
            self.negotiated.store(true, Ordering::Release);
            Ok(TestNegotiatedOutput {
                config: AudioDeviceConfig::new(44_100.0, 2, AudioSampleFormat::F32, 2).unwrap(),
                negotiated: self.negotiated,
                buffer_address: self.buffer_address,
            })
        }
    }

    impl NegotiatedAudioOutput for TestNegotiatedOutput {
        fn config(&self) -> AudioDeviceConfig {
            self.config
        }

        fn start(
            self,
            mut render: AudioRenderCallback,
            mut on_runtime_error: super::AudioDeviceStatusCallback,
        ) -> Result<AudioStream, AudioOutputError> {
            assert!(self.negotiated.load(Ordering::Acquire));
            let mut device_buffer = [0.0; 4];
            self.buffer_address
                .store(device_buffer.as_mut_ptr() as usize, Ordering::Release);
            render(&mut device_buffer);
            on_runtime_error(AudioDeviceRuntimeError::Xrun);
            Ok(AudioStream::new(()))
        }
    }

    #[test]
    fn negotiation_precedes_start_and_forwards_the_device_buffer() {
        let negotiated = Arc::new(AtomicBool::new(false));
        let expected_address = Arc::new(AtomicUsize::new(0));
        let observed_address = Arc::new(AtomicUsize::new(0));
        let runtime_error_seen = Arc::new(AtomicBool::new(false));

        let output = TestAudioOutput {
            negotiated: Arc::clone(&negotiated),
            buffer_address: Arc::clone(&expected_address),
        };
        let selected = output.negotiate().unwrap();
        assert_eq!(selected.config().sample_rate(), 44_100.0);

        let callback_address = Arc::clone(&observed_address);
        let render: AudioRenderCallback = Box::new(move |buffer| {
            callback_address.store(buffer.as_mut_ptr() as usize, Ordering::Release);
        });
        let runtime_error_seen_callback = Arc::clone(&runtime_error_seen);
        selected
            .start(
                render,
                Box::new(move |error| {
                    assert_eq!(error, AudioDeviceRuntimeError::Xrun);
                    runtime_error_seen_callback.store(true, Ordering::Release);
                }),
            )
            .unwrap();

        assert_eq!(
            observed_address.load(Ordering::Acquire),
            expected_address.load(Ordering::Acquire)
        );
        assert!(runtime_error_seen.load(Ordering::Acquire));
    }

    #[test]
    fn device_configuration_rejects_unsupported_values() {
        assert!(AudioDeviceConfig::new(0.0, 2, AudioSampleFormat::F32, 64).is_err());
        assert!(AudioDeviceConfig::new(48_000.0, 1, AudioSampleFormat::F32, 64).is_err());
        assert!(AudioDeviceConfig::new(48_000.0, 2, AudioSampleFormat::F32, 0).is_err());
    }

    #[test]
    fn setup_error_preserves_its_actionable_message() {
        let error = AudioOutputError::new("default output device is unavailable");
        assert_eq!(error.message(), "default output device is unavailable");
        assert_eq!(error.to_string(), "default output device is unavailable");
    }
}
