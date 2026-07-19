use crate::shell::audio_output::{AudioOutput, AudioOutputError, AudioRenderCallback, AudioStream};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    Device, FromSample, SampleFormat, SizedSample, Stream, StreamConfig, SupportedStreamConfig,
    I24, U24,
};

const STEREO_SCRATCH_SAMPLES: usize = 2_048;

/// Physical low-latency stereo output backed by the system's default CPAL host.
#[derive(Clone, Copy, Debug, Default)]
pub struct CpalAudioOutput;

impl CpalAudioOutput {
    /// Creates the stateless default-device adapter.
    pub const fn new() -> Self {
        Self
    }
}

impl AudioOutput for CpalAudioOutput {
    fn open(&self, render: AudioRenderCallback) -> Result<AudioStream, AudioOutputError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AudioOutputError::new("default output device is unavailable"))?;
        let supported_config = select_output_config(&device)?;
        let sample_format = supported_config.sample_format();
        let sample_rate = supported_config.sample_rate() as f32;
        let stream_config = supported_config.config();

        let stream = build_stream(&device, sample_format, stream_config, sample_rate, render)?;
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

    if default_config.channels() >= 2 && !default_config.sample_format().is_dsd() {
        return Ok(default_config);
    }

    let output_range = device
        .supported_output_configs()
        .map_err(|error| {
            AudioOutputError::new(format!(
                "failed to query PCM output configurations: {error}"
            ))
        })?
        .filter(|config| config.channels() >= 2 && !config.sample_format().is_dsd())
        .max_by(|left, right| left.cmp_default_heuristics(right))
        .ok_or_else(|| {
            AudioOutputError::new(
                "default output device has no PCM configuration with two channels",
            )
        })?;

    Ok(output_range
        .try_with_standard_sample_rate()
        .unwrap_or_else(|| output_range.with_max_sample_rate()))
}

fn build_stream(
    device: &Device,
    sample_format: SampleFormat,
    config: StreamConfig,
    sample_rate: f32,
    render: AudioRenderCallback,
) -> Result<Stream, AudioOutputError> {
    let channels = usize::from(config.channels);

    match sample_format {
        SampleFormat::F32 if channels == 2 => {
            build_native_stereo_stream(device, config, sample_rate, render)
        }
        SampleFormat::F32 => build_mapped_stream::<f32>(device, config, sample_rate, render),
        SampleFormat::F64 => build_mapped_stream::<f64>(device, config, sample_rate, render),
        SampleFormat::I8 => build_mapped_stream::<i8>(device, config, sample_rate, render),
        SampleFormat::I16 => build_mapped_stream::<i16>(device, config, sample_rate, render),
        SampleFormat::I24 => build_mapped_stream::<I24>(device, config, sample_rate, render),
        SampleFormat::I32 => build_mapped_stream::<i32>(device, config, sample_rate, render),
        SampleFormat::I64 => build_mapped_stream::<i64>(device, config, sample_rate, render),
        SampleFormat::U8 => build_mapped_stream::<u8>(device, config, sample_rate, render),
        SampleFormat::U16 => build_mapped_stream::<u16>(device, config, sample_rate, render),
        SampleFormat::U24 => build_mapped_stream::<U24>(device, config, sample_rate, render),
        SampleFormat::U32 => build_mapped_stream::<u32>(device, config, sample_rate, render),
        SampleFormat::U64 => build_mapped_stream::<u64>(device, config, sample_rate, render),
        unsupported => Err(AudioOutputError::new(format!(
            "unsupported default output sample format: {unsupported}"
        ))),
    }
}

fn build_native_stereo_stream(
    device: &Device,
    config: StreamConfig,
    sample_rate: f32,
    mut render: AudioRenderCallback,
) -> Result<Stream, AudioOutputError> {
    device
        .build_output_stream(
            config,
            move |device_buffer: &mut [f32], _| {
                render(device_buffer, sample_rate);
            },
            ignore_stream_error,
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
    sample_rate: f32,
    mut render: AudioRenderCallback,
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
                    render(render_buffer, sample_rate);
                    map_stereo_samples(render_buffer, device_chunk, channels, silence);
                }
            },
            ignore_stream_error,
            None,
        )
        .map_err(|error| {
            AudioOutputError::new(format!(
                "failed to open the default mapped PCM output stream: {error}"
            ))
        })
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

fn ignore_stream_error(_error: cpal::Error) {}

#[cfg(test)]
mod tests {
    use super::{bound_sample, map_stereo_samples, CpalAudioOutput, STEREO_SCRATCH_SAMPLES};
    use crate::shell::audio_output::AudioOutput;

    #[test]
    fn adapter_implements_the_audio_output_port() {
        fn assert_audio_output<Output: AudioOutput>() {}

        assert_audio_output::<CpalAudioOutput>();
        let _output = CpalAudioOutput::new();
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
