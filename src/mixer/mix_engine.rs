use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
use crate::real_time::parameter_snapshot::ParameterSnapshot;
use crate::real_time::patch_audio_block::PatchAudioBlock;

const MAX_DELAY_MILLISECONDS: f32 = 2000.0;

/// Combines identity-preserving Patch stems and the two shared global effect returns.
pub struct MixEngine<E> {
    effects: E,
    reverb_input: Vec<f32>,
    delay_input: Vec<f32>,
    max_frames: usize,
    prepared: bool,
}

impl<E> MixEngine<E>
where
    E: GlobalEffectsProcessor,
{
    /// Creates an unprepared mixer without allocating callback storage.
    #[must_use]
    pub fn new(effects: E) -> Self {
        Self {
            effects,
            reverb_input: Vec::new(),
            delay_input: Vec::new(),
            max_frames: 0,
            prepared: false,
        }
    }

    /// Allocates every scratch buffer and prepares the global effects.
    pub fn prepare(&mut self, sample_rate: f32, max_frames: usize) -> Result<(), EffectError> {
        self.prepared = false;

        let sample_capacity = max_frames
            .checked_mul(2)
            .ok_or(EffectError::StorageAllocationFailed)?;
        let reverb_input = allocate_zeros(sample_capacity)?;
        let delay_input = allocate_zeros(sample_capacity)?;

        self.effects
            .prepare(sample_rate, max_frames, MAX_DELAY_MILLISECONDS)?;

        self.reverb_input = reverb_input;
        self.delay_input = delay_input;
        self.max_frames = max_frames;
        self.prepared = true;
        Ok(())
    }

    /// Mixes one prepared stereo stem per active Patch into the stereo output.
    ///
    /// Each stem must match the Patch identity and index in `parameters`.
    /// A missing or mismatched stem silences the complete output rather than
    /// substituting another Patch's audio or parameters.
    pub fn mix(
        &mut self,
        patch_audio: &PatchAudioBlock,
        parameters: &ParameterSnapshot,
        output: &mut [f32],
    ) {
        output.fill(0.0);
        if !self.prepared {
            return;
        }

        let frame_count = (output.len() / 2).min(self.max_frames);
        let sample_count = frame_count * 2;
        if patch_audio.frame_count() != frame_count
            || patch_audio.patch_count() != parameters.patch_count()
        {
            return;
        }

        for (index, patch) in parameters.patches().iter().enumerate() {
            let Some(patch_id) = patch.patch_id() else {
                return;
            };
            let Some(stem) = patch_audio.stem(index, patch_id) else {
                return;
            };
            if stem.frame_count() != frame_count {
                return;
            }
        }

        self.reverb_input[..sample_count].fill(0.0);
        self.delay_input[..sample_count].fill(0.0);

        for (index, patch) in parameters.patches().iter().enumerate() {
            let patch_id = patch
                .patch_id()
                .expect("active ParameterSnapshot entries have Patch identities");
            let audio = patch_audio
                .stem(index, patch_id)
                .expect("Patch stems were validated before mixing")
                .samples();
            let channel = patch.parameters();
            let gain = db_to_linear(channel.gain_db());
            let (left_pan, right_pan) = pan_gains(channel.pan());
            let left_gain = gain * left_pan;
            let right_gain = gain * right_pan;
            let mut frame = 0;

            while frame < frame_count {
                let left = frame * 2;
                let right = left + 1;
                let mixed_left = audio[left] * left_gain;
                let mixed_right = audio[right] * right_gain;

                output[left] += mixed_left;
                output[right] += mixed_right;
                self.reverb_input[left] += mixed_left * channel.reverb_send();
                self.reverb_input[right] += mixed_right * channel.reverb_send();
                self.delay_input[left] += mixed_left * channel.delay_send();
                self.delay_input[right] += mixed_right * channel.delay_send();

                frame += 1;
            }
        }

        self.effects.process(
            &self.reverb_input[..sample_count],
            &self.delay_input[..sample_count],
            &mut output[..sample_count],
            parameters.global(),
        );

        let master_gain = db_to_linear(parameters.global().master_gain_db());
        for sample in &mut output[..sample_count] {
            *sample *= master_gain;
        }
    }
}

fn db_to_linear(decibels: f32) -> f32 {
    10.0_f32.powf(decibels / 20.0)
}

fn pan_gains(pan: f32) -> (f32, f32) {
    if pan < 0.0 {
        (1.0, 1.0 + pan)
    } else {
        (1.0 - pan, 1.0)
    }
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
    use super::MixEngine;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
    use crate::real_time::patch_audio_block::PatchAudioBlock;

    #[derive(Default)]
    struct TestEffects {
        prepared: bool,
    }

    impl GlobalEffectsProcessor for TestEffects {
        fn prepare(
            &mut self,
            _sample_rate: f32,
            _max_frames: usize,
            _max_delay_milliseconds: f32,
        ) -> Result<(), EffectError> {
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
            assert!(self.prepared);
            for ((output_sample, reverb_sample), delay_sample) in
                output.iter_mut().zip(reverb_input).zip(delay_input)
            {
                *output_sample += reverb_sample * parameters.reverb_return()
                    + delay_sample * parameters.delay_return();
            }
        }
    }

    fn channel(gain_db: f32, pan: f32, reverb_send: f32, delay_send: f32) -> ChannelParameters {
        ChannelParameters::new(gain_db, pan, reverb_send, delay_send)
            .expect("test channel values satisfy their bounds")
    }

    fn globals(master_gain_db: f32, reverb_return: f32, delay_return: f32) -> GlobalParameters {
        GlobalParameters::new(
            master_gain_db,
            0.5,
            0.5,
            reverb_return,
            250.0,
            0.5,
            delay_return,
        )
        .expect("test global values satisfy their bounds")
    }

    fn snapshot(
        ids_and_channels: &[(u32, ChannelParameters)],
        global: GlobalParameters,
    ) -> ParameterSnapshot {
        let mut patches =
            [RtPatchParameters::new(PatchId::new(1).unwrap(), ChannelParameters::default()); 16];
        for (slot, (id, channel)) in patches.iter_mut().zip(ids_and_channels) {
            *slot = RtPatchParameters::new(PatchId::new(*id).unwrap(), *channel);
        }
        ParameterSnapshot::new(1, global, &patches[..ids_and_channels.len()]).unwrap()
    }

    fn audio_block(parameters: &ParameterSnapshot, stems: &[&[f32]]) -> PatchAudioBlock {
        let frame_count = stems.first().map_or(0, |stem| stem.len() / 2);
        let mut block = PatchAudioBlock::prepare(frame_count.max(1)).unwrap();
        block.begin_render(parameters, frame_count).unwrap();

        for (index, (patch, samples)) in parameters.patches().iter().zip(stems).enumerate() {
            block
                .stem_mut(index, patch.patch_id().unwrap())
                .unwrap()
                .copy_from_slice(samples);
        }

        block
    }

    #[test]
    fn global_mix_applies_gain_and_pan_independently() {
        let mut mixer = MixEngine::new(TestEffects::default());
        mixer.prepare(48_000.0, 1).expect("mixer prepares");
        let parameters = snapshot(
            &[
                (11, channel(-6.020_600_3, -1.0, 0.0, 0.0)),
                (22, channel(-6.020_600_3, 1.0, 0.0, 0.0)),
            ],
            globals(0.0, 0.0, 0.0),
        );
        let left_patch = [1.0, 1.0];
        let right_patch = [1.0, 1.0];
        let block = audio_block(&parameters, &[&left_patch, &right_patch]);
        let mut output = [0.0; 2];

        mixer.mix(&block, &parameters, &mut output);

        assert!((output[0] - 0.5).abs() < 0.000_001);
        assert!((output[1] - 0.5).abs() < 0.000_001);
    }

    #[test]
    fn global_mix_routes_both_sends_through_both_returns() {
        let mut mixer = MixEngine::new(TestEffects::default());
        mixer.prepare(48_000.0, 1).expect("mixer prepares");
        let parameters = snapshot(&[(7, channel(0.0, 0.0, 0.25, 0.5))], globals(0.0, 0.4, 0.6));
        let patch = [1.0, 1.0];
        let block = audio_block(&parameters, &[&patch]);
        let mut output = [0.0; 2];

        mixer.mix(&block, &parameters, &mut output);

        assert!((output[0] - 1.4).abs() < 0.000_001);
        assert!((output[1] - 1.4).abs() < 0.000_001);
    }

    #[test]
    fn global_mix_applies_master_gain_after_effect_returns() {
        let mut mixer = MixEngine::new(TestEffects::default());
        mixer.prepare(48_000.0, 1).expect("mixer prepares");
        let parameters = snapshot(
            &[(3, channel(0.0, 0.0, 1.0, 0.0))],
            globals(-6.020_600_3, 1.0, 0.0),
        );
        let patch = [1.0, 1.0];
        let block = audio_block(&parameters, &[&patch]);
        let mut output = [0.0; 2];

        mixer.mix(&block, &parameters, &mut output);

        assert!((output[0] - 1.0).abs() < 0.000_001);
        assert!((output[1] - 1.0).abs() < 0.000_001);
    }

    #[test]
    fn global_mix_silences_mismatched_patch_identity() {
        let mut mixer = MixEngine::new(TestEffects::default());
        mixer.prepare(48_000.0, 1).expect("mixer prepares");
        let rendered_parameters = snapshot(
            &[
                (1, channel(0.0, 0.0, 0.0, 0.0)),
                (2, channel(0.0, 0.0, 0.0, 0.0)),
            ],
            globals(0.0, 0.0, 0.0),
        );
        let current_parameters = snapshot(
            &[
                (1, channel(0.0, 0.0, 0.0, 0.0)),
                (3, channel(0.0, 0.0, 0.0, 0.0)),
            ],
            globals(0.0, 0.0, 0.0),
        );
        let first = [1.0, 0.0];
        let second = [0.0, 1.0];
        let block = audio_block(&rendered_parameters, &[&first, &second]);
        let mut output = [1.0; 2];

        mixer.mix(&block, &current_parameters, &mut output);

        assert_eq!(output, [0.0; 2]);
    }

    #[test]
    fn global_mix_uses_only_preallocated_bounded_storage() {
        let mut mixer = MixEngine::new(TestEffects::default());
        mixer.prepare(48_000.0, 2).expect("mixer prepares");
        let reverb_capacity = mixer.reverb_input.capacity();
        let delay_capacity = mixer.delay_input.capacity();
        let parameters = snapshot(&[(9, channel(0.0, 0.0, 0.5, 0.5))], globals(0.0, 1.0, 1.0));
        let patch = [1.0; 4];
        let block = audio_block(&parameters, &[&patch]);
        let mut output = [1.0; 8];

        mixer.mix(&block, &parameters, &mut output);

        assert_eq!(mixer.reverb_input.capacity(), reverb_capacity);
        assert_eq!(mixer.delay_input.capacity(), delay_capacity);
        assert_eq!(output[4..], [0.0; 4]);
    }
}
