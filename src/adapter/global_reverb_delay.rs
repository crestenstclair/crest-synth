use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
use crate::mixer::global_parameters::GlobalParameters;

const REVERB_LEFT_MILLISECONDS: f32 = 29.7;
const REVERB_RIGHT_MILLISECONDS: f32 = 37.1;

/// In-process implementation of the one shared stereo reverb and stereo delay.
#[derive(Debug, Default)]
pub struct GlobalReverbDelay {
    reverb: StereoReverb,
    delay: StereoFeedbackDelay,
    max_frames: usize,
    prepared: bool,
}

impl GlobalReverbDelay {
    /// Creates an unprepared processor without allocating effect storage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl GlobalEffectsProcessor for GlobalReverbDelay {
    fn prepare(
        &mut self,
        sample_rate: f32,
        max_frames: usize,
        max_delay_milliseconds: f32,
    ) -> Result<(), EffectError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(EffectError::InvalidSampleRate);
        }
        if max_frames == 0 {
            return Err(EffectError::InvalidMaxFrames);
        }
        if !max_delay_milliseconds.is_finite() || max_delay_milliseconds <= 0.0 {
            return Err(EffectError::InvalidMaxDelayMilliseconds);
        }

        let reverb = StereoReverb::prepared(sample_rate)?;
        let delay = StereoFeedbackDelay::prepared(sample_rate, max_delay_milliseconds)?;

        self.reverb = reverb;
        self.delay = delay;
        self.max_frames = max_frames;
        self.prepared = true;
        Ok(())
    }

    fn process(
        &mut self,
        reverb_input: &[f32],
        delay_input: &[f32],
        output: &mut [f32],
        parameters: &GlobalParameters,
    ) {
        if !self.prepared {
            return;
        }

        let frame_count = (reverb_input.len() / 2)
            .min(delay_input.len() / 2)
            .min(output.len() / 2)
            .min(self.max_frames);
        let mut frame = 0;

        while frame < frame_count {
            let left = frame * 2;
            let right = left + 1;

            let (reverb_left, reverb_right) = self.reverb.process(
                reverb_input[left],
                reverb_input[right],
                parameters.reverb_room_size(),
                parameters.reverb_damping(),
            );
            let (delay_left, delay_right) = self.delay.process(
                delay_input[left],
                delay_input[right],
                parameters.delay_milliseconds(),
                parameters.delay_feedback(),
            );

            output[left] +=
                reverb_left * parameters.reverb_return() + delay_left * parameters.delay_return();
            output[right] +=
                reverb_right * parameters.reverb_return() + delay_right * parameters.delay_return();

            frame += 1;
        }
    }
}

#[derive(Debug, Default)]
struct StereoReverb {
    left_buffer: Vec<f32>,
    right_buffer: Vec<f32>,
    left_index: usize,
    right_index: usize,
    left_damping_state: f32,
    right_damping_state: f32,
}

impl StereoReverb {
    fn prepared(sample_rate: f32) -> Result<Self, EffectError> {
        let left_length = milliseconds_to_samples(sample_rate, REVERB_LEFT_MILLISECONDS)?;
        let right_length = milliseconds_to_samples(sample_rate, REVERB_RIGHT_MILLISECONDS)?;

        Ok(Self {
            left_buffer: allocate_zeros(left_length)?,
            right_buffer: allocate_zeros(right_length)?,
            left_index: 0,
            right_index: 0,
            left_damping_state: 0.0,
            right_damping_state: 0.0,
        })
    }

    fn process(
        &mut self,
        left_input: f32,
        right_input: f32,
        room_size: f32,
        damping: f32,
    ) -> (f32, f32) {
        let left_delayed = self.left_buffer[self.left_index];
        let right_delayed = self.right_buffer[self.right_index];
        let undamped = 1.0 - damping;

        let left_filtered = left_delayed * undamped + self.left_damping_state * damping;
        let right_filtered = right_delayed * undamped + self.right_damping_state * damping;
        self.left_damping_state = left_filtered;
        self.right_damping_state = right_filtered;

        let feedback = 0.2 + room_size * 0.75;
        self.left_buffer[self.left_index] = left_input + left_filtered * feedback;
        self.right_buffer[self.right_index] = right_input + right_filtered * feedback;

        self.left_index += 1;
        if self.left_index == self.left_buffer.len() {
            self.left_index = 0;
        }
        self.right_index += 1;
        if self.right_index == self.right_buffer.len() {
            self.right_index = 0;
        }

        (left_filtered, right_filtered)
    }
}

#[derive(Debug, Default)]
struct StereoFeedbackDelay {
    left_buffer: Vec<f32>,
    right_buffer: Vec<f32>,
    write_index: usize,
    sample_rate: f32,
}

impl StereoFeedbackDelay {
    fn prepared(sample_rate: f32, max_delay_milliseconds: f32) -> Result<Self, EffectError> {
        let max_delay_samples = milliseconds_to_samples(sample_rate, max_delay_milliseconds)?;
        let buffer_length = max_delay_samples
            .checked_add(1)
            .ok_or(EffectError::StorageAllocationFailed)?;

        Ok(Self {
            left_buffer: allocate_zeros(buffer_length)?,
            right_buffer: allocate_zeros(buffer_length)?,
            write_index: 0,
            sample_rate,
        })
    }

    fn process(
        &mut self,
        left_input: f32,
        right_input: f32,
        delay_milliseconds: f32,
        feedback: f32,
    ) -> (f32, f32) {
        let requested_delay = (delay_milliseconds * self.sample_rate / 1000.0).round() as usize;
        let delay_samples = requested_delay.clamp(1, self.left_buffer.len() - 1);
        let read_index =
            (self.write_index + self.left_buffer.len() - delay_samples) % self.left_buffer.len();

        let left_delayed = self.left_buffer[read_index];
        let right_delayed = self.right_buffer[read_index];

        self.left_buffer[self.write_index] = left_input + left_delayed * feedback;
        self.right_buffer[self.write_index] = right_input + right_delayed * feedback;

        self.write_index += 1;
        if self.write_index == self.left_buffer.len() {
            self.write_index = 0;
        }

        (left_delayed, right_delayed)
    }
}

fn milliseconds_to_samples(sample_rate: f32, milliseconds: f32) -> Result<usize, EffectError> {
    let sample_count = (f64::from(sample_rate) * f64::from(milliseconds) / 1000.0).ceil();
    if !sample_count.is_finite() || sample_count > (usize::MAX - 1) as f64 {
        return Err(EffectError::StorageAllocationFailed);
    }
    Ok((sample_count as usize).max(1))
}

fn allocate_zeros(length: usize) -> Result<Vec<f32>, EffectError> {
    let mut storage = Vec::new();
    storage
        .try_reserve_exact(length)
        .map_err(|_| EffectError::StorageAllocationFailed)?;
    storage.resize(length, 0.0);
    Ok(storage)
}

#[cfg(test)]
mod tests {
    use super::GlobalReverbDelay;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::mix_engine::MixEngine;
    use crate::mixer::mixer_state::MixerState;
    use crate::mixer::mixer_track_id::MixerTrackId;
    use crate::mixer::mixer_track_parameters::MixerTrackParameters;
    use crate::mixer::patch_output::PatchOutput;
    use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
    use crate::real_time::patch_audio_block::PatchAudioBlock;

    fn parameters(reverb_return: f32, delay_return: f32) -> GlobalParameters {
        GlobalParameters::new(0.0, 0.6, 0.25, reverb_return, 10.0, 0.5, delay_return)
            .expect("fixture satisfies the global parameter bounds")
    }

    #[test]
    fn prepare_rejects_invalid_configuration() {
        let mut effects = GlobalReverbDelay::new();

        assert_eq!(
            effects.prepare(0.0, 64, 2000.0),
            Err(EffectError::InvalidSampleRate)
        );
        assert_eq!(
            effects.prepare(48_000.0, 0, 2000.0),
            Err(EffectError::InvalidMaxFrames)
        );
        assert_eq!(
            effects.prepare(48_000.0, 64, f32::NAN),
            Err(EffectError::InvalidMaxDelayMilliseconds)
        );
    }

    #[test]
    fn process_is_a_noop_until_storage_is_prepared() {
        let mut effects = GlobalReverbDelay::new();
        let mut output = [0.25; 4];

        effects.process(&[1.0; 4], &[1.0; 4], &mut output, &parameters(1.0, 1.0));

        assert_eq!(output, [0.25; 4]);
    }

    #[test]
    fn one_shared_delay_returns_stereo_feedback() {
        let mut effects = GlobalReverbDelay::new();
        effects
            .prepare(1000.0, 64, 20.0)
            .expect("test storage is small and valid");
        let mut input = [0.0; 128];
        input[0] = 1.0;
        input[1] = 0.5;
        let mut output = [0.0; 128];

        effects.process(&[0.0; 128], &input, &mut output, &parameters(0.0, 1.0));

        assert!((output[20] - 1.0).abs() < f32::EPSILON);
        assert!((output[21] - 0.5).abs() < f32::EPSILON);
        assert!((output[40] - 0.5).abs() < f32::EPSILON);
        assert!((output[41] - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn one_shared_reverb_produces_decorrelated_stereo_returns() {
        let mut effects = GlobalReverbDelay::new();
        effects
            .prepare(1000.0, 64, 20.0)
            .expect("test storage is small and valid");
        let mut input = [0.0; 128];
        input[0] = 1.0;
        input[1] = 1.0;
        let mut output = [0.0; 128];

        effects.process(&input, &[0.0; 128], &mut output, &parameters(1.0, 0.0));

        assert!(output[60].abs() > 0.0);
        assert!(output[77].abs() > 0.0);
        assert_eq!(output[61], 0.0);
        assert!(output[76].abs() < output[77].abs());
    }

    #[test]
    fn return_parameters_change_the_rendered_signal() {
        let mut muted = GlobalReverbDelay::new();
        let mut audible = GlobalReverbDelay::new();
        muted
            .prepare(1000.0, 64, 20.0)
            .expect("test storage is small and valid");
        audible
            .prepare(1000.0, 64, 20.0)
            .expect("test storage is small and valid");

        let mut input = [0.0; 128];
        input[0] = 1.0;
        let mut muted_output = [0.0; 128];
        let mut audible_output = [0.0; 128];

        muted.process(
            &[0.0; 128],
            &input,
            &mut muted_output,
            &parameters(0.0, 0.0),
        );
        audible.process(
            &[0.0; 128],
            &input,
            &mut audible_output,
            &parameters(0.0, 1.0),
        );

        assert_eq!(muted_output[20], 0.0);
        assert!(audible_output[20] > 0.0);
    }

    fn render_global_parameters(parameters: GlobalParameters) -> Vec<f32> {
        const FRAME_COUNT: usize = 256;
        let patch_id = PatchId::new(1).unwrap();
        let snapshot = ParameterSnapshot::new(
            1,
            parameters,
            MixerState::default().with_track(
                MixerTrackId::default(),
                MixerTrackParameters::new(-12.0, 0.0, false, false, 0.7, 0.6).unwrap(),
            ),
            &[RtPatchParameters::new(patch_id, PatchOutput::default())],
        )
        .unwrap();
        let mut block = PatchAudioBlock::prepare(FRAME_COUNT).unwrap();
        block.begin_render(&snapshot, FRAME_COUNT).unwrap();
        let stem = block.stem_mut(0, patch_id).unwrap();
        stem[0] = 0.2;
        stem[1] = -0.1;

        let mut mixer = MixEngine::new(GlobalReverbDelay::new());
        mixer.prepare(1_000.0, FRAME_COUNT).unwrap();
        let mut output = vec![0.0; FRAME_COUNT * 2];
        mixer.mix(&block, &snapshot, &mut output);
        output
    }

    #[test]
    fn global_effects_parameter_sensitivity() {
        let baseline = GlobalParameters::new(0.0, 0.5, 0.3, 0.6, 10.0, 0.4, 0.6).unwrap();
        let baseline_output = render_global_parameters(baseline);

        for descriptor in GlobalParameters::surface_descriptor() {
            let parameter = descriptor.parameter();
            let current = baseline.value(parameter);
            let changed = if current + descriptor.coarse_step() <= descriptor.maximum() {
                current + descriptor.coarse_step()
            } else {
                current - descriptor.coarse_step()
            };
            let variant = baseline.with_value(parameter, changed).unwrap();
            let variant_output = render_global_parameters(variant);
            assert!(
                baseline_output
                    .iter()
                    .zip(&variant_output)
                    .any(|(left, right)| (left - right).abs() > 1.0e-7),
                "{} must change measured output from identical fresh effect state",
                descriptor.name()
            );
        }
    }
}
