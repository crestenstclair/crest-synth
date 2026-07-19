use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
use crate::mixer::mix_engine::MixEngine;
use crate::real_time::audio_boundary::AudioThreadBoundary;
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::patch_audio_block::{PatchAudioBlock, PatchAudioBlockError};
use crate::synth::sound_font_engine::SoundFontEngine;
use core::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioError {
    InvalidSampleRate,
    InvalidMaxFrames,
    SampleCapacityExceeded,
    StorageAllocationFailed,
    Effects(EffectError),
    PatchAudioBlock(PatchAudioBlockError),
}

impl fmt::Display for AudioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSampleRate => {
                formatter.write_str("audio sample rate must be finite and greater than zero")
            }
            Self::InvalidMaxFrames => {
                formatter.write_str("maximum audio frame count must be greater than zero")
            }
            Self::SampleCapacityExceeded => {
                formatter.write_str("maximum audio frame count exceeds addressable stereo storage")
            }
            Self::StorageAllocationFailed => {
                formatter.write_str("audio renderer scratch storage allocation failed")
            }
            Self::Effects(error) => write!(formatter, "global mixer preparation failed: {error}"),
            Self::PatchAudioBlock(error) => {
                write!(formatter, "Patch audio preparation failed: {error}")
            }
        }
    }
}

impl std::error::Error for AudioError {}

impl From<EffectError> for AudioError {
    fn from(error: EffectError) -> Self {
        Self::Effects(error)
    }
}

impl From<PatchAudioBlockError> for AudioError {
    fn from(error: PatchAudioBlockError) -> Self {
        Self::PatchAudioBlock(error)
    }
}

/// Joins the callback-side boundary, the one SoundFont engine, and the global mixer.
///
/// The engine and its patches must be loaded and configured on the control thread
/// before construction. After `prepare`, `render` uses only bounded storage and
/// non-blocking operations supplied by its dependencies.
pub struct AudioRenderer<Boundary, Engine, Effects> {
    boundary: Boundary,
    engine: Engine,
    mixer: MixEngine<Effects>,
    patch_audio: Option<PatchAudioBlock>,
    max_frames: usize,
    prepared: bool,
}

impl<Boundary, Engine, Effects> AudioRenderer<Boundary, Engine, Effects>
where
    Boundary: AudioThreadBoundary,
    Engine: SoundFontEngine,
    Effects: GlobalEffectsProcessor,
{
    #[must_use]
    pub fn new(boundary: Boundary, engine: Engine, mixer: MixEngine<Effects>) -> Self {
        Self {
            boundary,
            engine,
            mixer,
            patch_audio: None,
            max_frames: 0,
            prepared: false,
        }
    }

    /// Allocates every renderer, mixer, and effect buffer on the control thread.
    pub fn prepare(&mut self, max_frames: usize, sample_rate: f32) -> Result<(), AudioError> {
        self.prepared = false;
        if max_frames == 0 {
            return Err(AudioError::InvalidMaxFrames);
        }
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(AudioError::InvalidSampleRate);
        }

        let _sample_capacity = max_frames
            .checked_mul(2)
            .ok_or(AudioError::SampleCapacityExceeded)?;
        let patch_audio = PatchAudioBlock::prepare(max_frames)?;
        self.mixer.prepare(sample_rate, max_frames)?;

        self.patch_audio = Some(patch_audio);
        self.max_frames = max_frames;
        self.prepared = true;
        Ok(())
    }

    /// Drains ready commands, consumes one latest snapshot, renders the
    /// SoundFont stream, and applies the global mixer.
    pub fn render(&mut self, interleaved_stereo: &mut [f32]) {
        interleaved_stereo.fill(0.0);
        if !self.prepared {
            return;
        }

        while let Some(command) = self.boundary.pop_command() {
            match command {
                AudioCommand::PatchMidi { patch_id, message } => {
                    if self.engine.dispatch(patch_id, message).is_err() {
                        self.engine.all_notes_off();
                    }
                }
                AudioCommand::AllNotesOff => self.engine.all_notes_off(),
            }
        }

        let parameters = self.boundary.read_latest_parameters();
        let frame_count = (interleaved_stereo.len() / 2).min(self.max_frames);
        let sample_count = frame_count * 2;
        if sample_count == 0 {
            return;
        }

        let Some(patch_audio) = self.patch_audio.as_mut() else {
            return;
        };
        if patch_audio.begin_render(&parameters, frame_count).is_err() {
            return;
        }

        self.engine.render_patches(patch_audio, &parameters);
        self.mixer.mix(patch_audio, &parameters, interleaved_stereo);
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioError, AudioRenderer};
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::mix_engine::MixEngine;
    use crate::real_time::audio_boundary::{AudioThreadBoundary, RetiredAudioState};
    use crate::real_time::audio_command::AudioCommand;
    use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
    use crate::real_time::patch_audio_block::PatchAudioBlock;
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_engine::{SoundFontEngine, SoundFontError};
    use core::alloc::{GlobalAlloc, Layout};
    use core::cell::Cell;
    use std::alloc::System;
    use std::path::Path;

    thread_local! {
        static COUNT_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
        static ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
    }

    struct TestAllocator;

    #[global_allocator]
    static TEST_ALLOCATOR: TestAllocator = TestAllocator;

    unsafe impl GlobalAlloc for TestAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record_allocation();
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record_allocation();
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            unsafe { System.dealloc(pointer, layout) }
        }

        unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            record_allocation();
            unsafe { System.realloc(pointer, layout, new_size) }
        }
    }

    fn record_allocation() {
        let _ = COUNT_ALLOCATIONS.try_with(|enabled| {
            if enabled.get() {
                let _ = ALLOCATION_COUNT.try_with(|count| count.set(count.get() + 1));
            }
        });
    }

    fn begin_allocation_count() {
        ALLOCATION_COUNT.with(|count| count.set(0));
        COUNT_ALLOCATIONS.with(|enabled| enabled.set(true));
    }

    fn finish_allocation_count() -> usize {
        COUNT_ALLOCATIONS.with(|enabled| enabled.set(false));
        ALLOCATION_COUNT.with(Cell::get)
    }

    struct TestBoundary {
        commands: [Option<AudioCommand>; 3],
        next_command: usize,
        parameters: ParameterSnapshot,
        parameter_reads: usize,
    }

    impl AudioThreadBoundary for TestBoundary {
        type RetiredState = ();

        fn pop_command(&mut self) -> Option<AudioCommand> {
            let command = self.commands.get_mut(self.next_command)?.take();
            if command.is_some() {
                self.next_command += 1;
            }
            command
        }

        fn read_latest_parameters(&mut self) -> ParameterSnapshot {
            self.parameter_reads += 1;
            self.parameters
        }

        fn retire(&mut self, _state: RetiredAudioState<Self::RetiredState>) {}
    }

    #[derive(Default)]
    struct TestEngine {
        dispatched: [Option<(PatchId, MidiMessage)>; 2],
        dispatch_count: usize,
        all_notes_off_count: usize,
        rendered_generation: Option<u64>,
    }

    impl SoundFontEngine for TestEngine {
        fn load(&mut self, _path: &Path) -> Result<(), SoundFontError> {
            Ok(())
        }

        fn configure_patch(&mut self, _patch: &Patch) -> Result<(), SoundFontError> {
            Ok(())
        }

        fn dispatch(
            &mut self,
            patch_id: PatchId,
            message: MidiMessage,
        ) -> Result<(), SoundFontError> {
            if let Some(slot) = self.dispatched.get_mut(self.dispatch_count) {
                *slot = Some((patch_id, message));
                self.dispatch_count += 1;
                Ok(())
            } else {
                Err(SoundFontError::MidiDispatchFailed { patch_id })
            }
        }

        fn all_notes_off(&mut self) {
            self.all_notes_off_count += 1;
        }

        fn render_patches(&mut self, output: &mut PatchAudioBlock, parameters: &ParameterSnapshot) {
            self.rendered_generation = Some(parameters.generation());
            for (index, patch) in parameters.patches().iter().enumerate() {
                let Some(patch_id) = patch.patch_id() else {
                    continue;
                };
                if let Some(samples) = output.stem_mut(index, patch_id) {
                    samples.fill(if index == 0 { 0.25 } else { 0.5 });
                }
            }
        }
    }

    struct TestEffects;

    impl GlobalEffectsProcessor for TestEffects {
        fn prepare(
            &mut self,
            _sample_rate: f32,
            _max_frames: usize,
            _max_delay_milliseconds: f32,
        ) -> Result<(), EffectError> {
            Ok(())
        }

        fn process(
            &mut self,
            reverb_input: &[f32],
            delay_input: &[f32],
            output: &mut [f32],
            _parameters: &GlobalParameters,
        ) {
            for ((output_sample, reverb_sample), delay_sample) in
                output.iter_mut().zip(reverb_input).zip(delay_input)
            {
                *output_sample += reverb_sample + delay_sample;
            }
        }
    }

    fn midi(kind: MidiMessageKind) -> MidiMessage {
        MidiMessage::try_new(MidiChannel::new(0).unwrap(), kind, 60, 100).unwrap()
    }

    fn parameters(first_patch_id: PatchId, second_patch_id: PatchId) -> ParameterSnapshot {
        ParameterSnapshot::new(
            9,
            GlobalParameters::new(0.0, 0.5, 0.5, 1.0, 250.0, 0.5, 1.0).unwrap(),
            &[
                RtPatchParameters::new(
                    first_patch_id,
                    ChannelParameters::new(0.0, 0.0, 0.0, 0.0).unwrap(),
                ),
                RtPatchParameters::new(
                    second_patch_id,
                    ChannelParameters::new(0.0, 0.0, 0.0, 0.0).unwrap(),
                ),
            ],
        )
        .unwrap()
    }

    fn renderer() -> AudioRenderer<TestBoundary, TestEngine, TestEffects> {
        let patch_id = PatchId::new(4).unwrap();
        let second_patch_id = PatchId::new(7).unwrap();
        let boundary = TestBoundary {
            commands: [
                Some(AudioCommand::patch_midi(
                    patch_id,
                    midi(MidiMessageKind::NoteOn),
                )),
                Some(AudioCommand::patch_midi(
                    patch_id,
                    midi(MidiMessageKind::NoteOff),
                )),
                Some(AudioCommand::all_notes_off()),
            ],
            next_command: 0,
            parameters: parameters(patch_id, second_patch_id),
            parameter_reads: 0,
        };

        AudioRenderer::new(boundary, TestEngine::default(), MixEngine::new(TestEffects))
    }

    #[test]
    fn audio_renderer_realtime_contract() {
        let mut renderer = renderer();
        renderer.prepare(4, 48_000.0).unwrap();
        let mut output = [0.0; 8];

        begin_allocation_count();
        renderer.render(&mut output);
        let callback_allocations = finish_allocation_count();

        assert_eq!(callback_allocations, 0);
        assert_eq!(renderer.boundary.parameter_reads, 1);
        assert_eq!(renderer.engine.dispatch_count, 2);
        assert_eq!(renderer.engine.all_notes_off_count, 1);
        assert_eq!(renderer.engine.rendered_generation, Some(9));
        assert!(output
            .iter()
            .all(|sample| (*sample - 0.75).abs() < 0.000_001));
    }

    #[test]
    fn prepare_rejects_invalid_callback_shapes_before_rendering() {
        let mut renderer = renderer();

        assert_eq!(
            renderer.prepare(0, 48_000.0),
            Err(AudioError::InvalidMaxFrames)
        );
        assert_eq!(
            renderer.prepare(4, f32::NAN),
            Err(AudioError::InvalidSampleRate)
        );

        let mut output = [1.0; 4];
        renderer.render(&mut output);
        assert_eq!(output, [0.0; 4]);
    }

    #[test]
    fn render_silences_samples_past_the_prepared_block() {
        let mut renderer = renderer();
        renderer.prepare(1, 48_000.0).unwrap();
        let mut output = [1.0; 5];

        renderer.render(&mut output);

        assert!(output[..2]
            .iter()
            .all(|sample| (*sample - 0.75).abs() < 0.000_001));
        assert_eq!(output[2..], [0.0; 3]);
    }
}
