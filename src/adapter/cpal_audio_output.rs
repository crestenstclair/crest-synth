use crate::shell::audio_output::{
    AudioDeviceConfig, AudioDeviceRuntimeError, AudioDeviceStatusCallback, AudioOutput,
    AudioOutputError, AudioRenderCallback, AudioSampleFormat, AudioStream, NegotiatedAudioOutput,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    BufferSize, Device, ErrorKind, FromSample, SampleFormat, SizedSample, Stream, StreamConfig,
    SupportedBufferSize, SupportedStreamConfig, I24, U24,
};

const STEREO_SCRATCH_SAMPLES: usize = 2_048;
const PREFERRED_SAMPLE_RATE: u32 = 48_000;
const PREFERRED_CALLBACK_FRAMES: u32 = 256;
const UNKNOWN_CALLBACK_RENDER_CAPACITY: usize = STEREO_SCRATCH_SAMPLES / 2;

/// Physical low-latency stereo output backed by the system's default CPAL host.
#[derive(Clone, Copy, Debug, Default)]
pub struct CpalAudioOutput;

impl CpalAudioOutput {
    pub const fn new() -> Self {
        Self
    }
}

/// A selected CPAL device whose callback has not started yet.
pub struct CpalNegotiatedAudioOutput {
    device: Device,
    sample_format: SampleFormat,
    stream_config: StreamConfig,
    config: AudioDeviceConfig,
}

impl AudioOutput for CpalAudioOutput {
    type Negotiated = CpalNegotiatedAudioOutput;

    fn negotiate(self) -> Result<Self::Negotiated, AudioOutputError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AudioOutputError::new("default output device is unavailable"))?;
        let supported = select_output_config(&device)?;
        let sample_format = supported.sample_format();
        let (stream_config, render_capacity_frames) = configure_buffer_size(&supported);
        let canonical_format = canonical_sample_format(sample_format).ok_or_else(|| {
            AudioOutputError::new(format!(
                "unsupported default output sample format: {sample_format}"
            ))
        })?;
        let config = AudioDeviceConfig::new(
            stream_config.sample_rate as f32,
            stream_config.channels,
            canonical_format,
            render_capacity_frames,
        )
        .map_err(|error| AudioOutputError::new(error.to_string()))?;

        Ok(CpalNegotiatedAudioOutput {
            device,
            sample_format,
            stream_config,
            config,
        })
    }
}

impl NegotiatedAudioOutput for CpalNegotiatedAudioOutput {
    fn config(&self) -> AudioDeviceConfig {
        self.config
    }

    fn start(
        self,
        render: AudioRenderCallback,
        on_runtime_error: AudioDeviceStatusCallback,
    ) -> Result<AudioStream, AudioOutputError> {
        let stream = build_stream(
            &self.device,
            self.sample_format,
            self.stream_config,
            render,
            on_runtime_error,
        )?;
        stream.play().map_err(|error| {
            AudioOutputError::new(format!(
                "failed to start the default PCM output stream: {error}"
            ))
        })?;
        Ok(AudioStream::new(stream))
    }
}

fn select_output_config(device: &Device) -> Result<SupportedStreamConfig, AudioOutputError> {
    let default_config = device.default_output_config().map_err(|error| {
        AudioOutputError::new(format!(
            "failed to query the default output configuration: {error}"
        ))
    })?;
    let ranges = device.supported_output_configs().map_err(|error| {
        AudioOutputError::new(format!(
            "failed to query PCM output configurations: {error}"
        ))
    })?;
    let pcm_ranges = ranges
        .filter(|config| config.channels() >= 2 && !config.sample_format().is_dsd())
        .collect::<Vec<_>>();

    if let Some(preferred) = pcm_ranges
        .iter()
        .copied()
        .filter(|range| range.contains_rate(PREFERRED_SAMPLE_RATE))
        .max_by(|left, right| left.cmp_default_heuristics(right))
        .and_then(|range| range.try_with_sample_rate(PREFERRED_SAMPLE_RATE))
    {
        return Ok(preferred);
    }

    if default_config.channels() >= 2 && !default_config.sample_format().is_dsd() {
        return Ok(default_config);
    }

    pcm_ranges
        .into_iter()
        .max_by(|left, right| left.cmp_default_heuristics(right))
        .map(|range| {
            range
                .try_with_standard_sample_rate()
                .unwrap_or_else(|| range.with_max_sample_rate())
        })
        .ok_or_else(|| {
            AudioOutputError::new(
                "default output device has no PCM configuration with two channels",
            )
        })
}

fn configure_buffer_size(supported: &SupportedStreamConfig) -> (StreamConfig, usize) {
    let mut config = supported.config();
    match *supported.buffer_size() {
        SupportedBufferSize::Range { min, max } => {
            let frames = PREFERRED_CALLBACK_FRAMES.clamp(min, max);
            config.buffer_size = BufferSize::Fixed(frames);
            (config, frames as usize)
        }
        SupportedBufferSize::Unknown => (config, UNKNOWN_CALLBACK_RENDER_CAPACITY),
    }
}

fn canonical_sample_format(sample_format: SampleFormat) -> Option<AudioSampleFormat> {
    Some(match sample_format {
        SampleFormat::F32 => AudioSampleFormat::F32,
        SampleFormat::F64 => AudioSampleFormat::F64,
        SampleFormat::I8 => AudioSampleFormat::I8,
        SampleFormat::I16 => AudioSampleFormat::I16,
        SampleFormat::I24 => AudioSampleFormat::I24,
        SampleFormat::I32 => AudioSampleFormat::I32,
        SampleFormat::I64 => AudioSampleFormat::I64,
        SampleFormat::U8 => AudioSampleFormat::U8,
        SampleFormat::U16 => AudioSampleFormat::U16,
        SampleFormat::U24 => AudioSampleFormat::U24,
        SampleFormat::U32 => AudioSampleFormat::U32,
        SampleFormat::U64 => AudioSampleFormat::U64,
        _ => return None,
    })
}

fn build_stream(
    device: &Device,
    sample_format: SampleFormat,
    config: StreamConfig,
    render: AudioRenderCallback,
    on_runtime_error: AudioDeviceStatusCallback,
) -> Result<Stream, AudioOutputError> {
    let channels = usize::from(config.channels);
    match sample_format {
        SampleFormat::F32 if channels == 2 => {
            build_native_stereo_stream(device, config, render, on_runtime_error)
        }
        SampleFormat::F32 => build_mapped_stream::<f32>(device, config, render, on_runtime_error),
        SampleFormat::F64 => build_mapped_stream::<f64>(device, config, render, on_runtime_error),
        SampleFormat::I8 => build_mapped_stream::<i8>(device, config, render, on_runtime_error),
        SampleFormat::I16 => build_mapped_stream::<i16>(device, config, render, on_runtime_error),
        SampleFormat::I24 => build_mapped_stream::<I24>(device, config, render, on_runtime_error),
        SampleFormat::I32 => build_mapped_stream::<i32>(device, config, render, on_runtime_error),
        SampleFormat::I64 => build_mapped_stream::<i64>(device, config, render, on_runtime_error),
        SampleFormat::U8 => build_mapped_stream::<u8>(device, config, render, on_runtime_error),
        SampleFormat::U16 => build_mapped_stream::<u16>(device, config, render, on_runtime_error),
        SampleFormat::U24 => build_mapped_stream::<U24>(device, config, render, on_runtime_error),
        SampleFormat::U32 => build_mapped_stream::<u32>(device, config, render, on_runtime_error),
        SampleFormat::U64 => build_mapped_stream::<u64>(device, config, render, on_runtime_error),
        unsupported => Err(AudioOutputError::new(format!(
            "unsupported default output sample format: {unsupported}"
        ))),
    }
}

fn build_native_stereo_stream(
    device: &Device,
    config: StreamConfig,
    mut render: AudioRenderCallback,
    mut on_runtime_error: AudioDeviceStatusCallback,
) -> Result<Stream, AudioOutputError> {
    device
        .build_output_stream(
            config,
            move |device_buffer: &mut [f32], _| render(device_buffer),
            move |error| on_runtime_error(map_runtime_error(error.kind())),
            None,
        )
        .map_err(|error| {
            AudioOutputError::new(format!(
                "failed to open the default native stereo f32 output stream: {error}"
            ))
        })
}

fn build_mapped_stream<Sample>(
    device: &Device,
    config: StreamConfig,
    mut render: AudioRenderCallback,
    mut on_runtime_error: AudioDeviceStatusCallback,
) -> Result<Stream, AudioOutputError>
where
    Sample: SizedSample + FromSample<f32> + Send + 'static,
{
    let channels = usize::from(config.channels);
    let device_chunk_samples = (STEREO_SCRATCH_SAMPLES / 2) * channels;
    let silence = Sample::from_sample(0.0);
    let mut stereo_buffer = [0.0; STEREO_SCRATCH_SAMPLES];

    device
        .build_output_stream(
            config,
            move |device_buffer: &mut [Sample], _| {
                for device_chunk in device_buffer.chunks_mut(device_chunk_samples) {
                    let frame_count = device_chunk.len() / channels;
                    let stereo_sample_count = frame_count * 2;
                    let render_buffer = &mut stereo_buffer[..stereo_sample_count];
                    render(render_buffer);
                    map_stereo_samples(render_buffer, device_chunk, channels, silence);
                }
            },
            move |error| on_runtime_error(map_runtime_error(error.kind())),
            None,
        )
        .map_err(|error| {
            AudioOutputError::new(format!(
                "failed to open the default mapped PCM output stream: {error}"
            ))
        })
}

const fn map_runtime_error(kind: ErrorKind) -> AudioDeviceRuntimeError {
    match kind {
        ErrorKind::DeviceBusy => AudioDeviceRuntimeError::DeviceBusy,
        ErrorKind::DeviceChanged => AudioDeviceRuntimeError::DeviceChanged,
        ErrorKind::DeviceNotAvailable => AudioDeviceRuntimeError::DeviceUnavailable,
        ErrorKind::HostUnavailable => AudioDeviceRuntimeError::HostUnavailable,
        ErrorKind::InvalidInput => AudioDeviceRuntimeError::InvalidInput,
        ErrorKind::PermissionDenied => AudioDeviceRuntimeError::PermissionDenied,
        ErrorKind::RealtimeDenied => AudioDeviceRuntimeError::RealtimeDenied,
        ErrorKind::ResourceExhausted => AudioDeviceRuntimeError::ResourceExhausted,
        ErrorKind::StreamInvalidated => AudioDeviceRuntimeError::StreamInvalidated,
        ErrorKind::UnsupportedConfig => AudioDeviceRuntimeError::UnsupportedConfig,
        ErrorKind::UnsupportedOperation => AudioDeviceRuntimeError::UnsupportedOperation,
        ErrorKind::Xrun => AudioDeviceRuntimeError::Xrun,
        ErrorKind::BackendError => AudioDeviceRuntimeError::Backend,
        ErrorKind::Other => AudioDeviceRuntimeError::Other,
        _ => AudioDeviceRuntimeError::Other,
    }
}

fn map_stereo_samples<Sample>(
    source: &[f32],
    destination: &mut [Sample],
    channels: usize,
    silence: Sample,
) where
    Sample: cpal::Sample + FromSample<f32>,
{
    destination.fill(silence);
    for (source_frame, destination_frame) in source
        .chunks_exact(2)
        .zip(destination.chunks_exact_mut(channels))
    {
        destination_frame[0] = Sample::from_sample(bound_sample(source_frame[0]));
        destination_frame[1] = Sample::from_sample(bound_sample(source_frame[1]));
    }
}

fn bound_sample(sample: f32) -> f32 {
    if sample.is_nan() {
        0.0
    } else {
        sample.clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bound_sample, canonical_sample_format, map_runtime_error, map_stereo_samples,
        CpalAudioOutput, STEREO_SCRATCH_SAMPLES,
    };
    use crate::shell::audio_output::{AudioDeviceRuntimeError, AudioOutput, AudioSampleFormat};
    use cpal::{ErrorKind, SampleFormat};

    #[test]
    fn adapter_implements_the_audio_output_port() {
        fn assert_audio_output<Output: AudioOutput>() {}
        assert_audio_output::<CpalAudioOutput>();
        let _output = CpalAudioOutput::new();
    }

    #[test]
    fn every_supported_pcm_format_has_a_canonical_configuration_value() {
        assert_eq!(
            canonical_sample_format(SampleFormat::F32),
            Some(AudioSampleFormat::F32)
        );
        assert_eq!(
            canonical_sample_format(SampleFormat::I16),
            Some(AudioSampleFormat::I16)
        );
        assert_eq!(
            canonical_sample_format(SampleFormat::U24),
            Some(AudioSampleFormat::U24)
        );
    }

    #[test]
    fn cpal_runtime_errors_map_without_formatting_or_allocation() {
        assert_eq!(
            map_runtime_error(ErrorKind::DeviceNotAvailable),
            AudioDeviceRuntimeError::DeviceUnavailable
        );
        assert_eq!(
            map_runtime_error(ErrorKind::Xrun),
            AudioDeviceRuntimeError::Xrun
        );
    }

    #[test]
    fn non_f32_conversion_is_finite_and_bounded() {
        let source = [f32::NEG_INFINITY, -1.0, 0.0, 1.0, f32::INFINITY, f32::NAN];
        let mut destination = [0_i16; 6];
        map_stereo_samples(&source, &mut destination, 2, 0);
        assert_eq!(destination[0], destination[1]);
        assert!(destination[1] < destination[2]);
        assert_eq!(destination[2], 0);
        assert!(destination[2] < destination[3]);
        assert_eq!(destination[3], destination[4]);
        assert_eq!(destination[5], 0);
    }

    #[test]
    fn stereo_samples_fill_first_two_channels_and_silence_surplus_channels() {
        let source = [0.25, -0.5, 0.75, -1.0];
        let mut destination = [9.0_f32; 9];
        map_stereo_samples(&source, &mut destination, 4, 0.0);
        assert_eq!(
            destination,
            [0.25, -0.5, 0.0, 0.0, 0.75, -1.0, 0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn conversion_chunks_are_bounded_stereo_sample_storage() {
        const { assert!(STEREO_SCRATCH_SAMPLES > 0) };
        assert_eq!(STEREO_SCRATCH_SAMPLES % 2, 0);
        assert_eq!(bound_sample(f32::NAN), 0.0);
        assert_eq!(bound_sample(-2.0), -1.0);
        assert_eq!(bound_sample(2.0), 1.0);
    }
}
