use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
use crate::mixer::mix_engine::MixEngine;
use crate::real_time::audio_boundary::AudioThreadBoundary;
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::audio_observation::{CallbackAudioObservation, DiscardAudioObservation};
use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
use crate::real_time::parameter_snapshot::MAX_PATCHES;
use crate::real_time::patch_audio_block::{PatchAudioBlock, PatchAudioBlockError};
use crate::synth::sound_font_engine::SoundFontEngine;
use crate::{kernel::midi_message::MidiMessageKind, kernel::patch_id::PatchId};
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
pub struct AudioRenderer<Boundary, Engine, Effects, Observation = DiscardAudioObservation> {
    boundary: Boundary,
    engine: Engine,
    mixer: MixEngine<Effects>,
    observation: Observation,
    patch_audio: Option<PatchAudioBlock>,
    active_notes: ActiveNoteObservation,
    rendered_blocks: u64,
    rendered_frames: u64,
    commands_consumed: u64,
    max_frames: usize,
    prepared: bool,
}

impl<Boundary, Engine, Effects> AudioRenderer<Boundary, Engine, Effects, DiscardAudioObservation>
where
    Boundary: AudioThreadBoundary,
    Engine: SoundFontEngine,
    Effects: GlobalEffectsProcessor,
{
    #[must_use]
    pub fn new(boundary: Boundary, engine: Engine, mixer: MixEngine<Effects>) -> Self {
        Self::with_observation(boundary, engine, mixer, DiscardAudioObservation)
    }
}

impl<Boundary, Engine, Effects, Observation> AudioRenderer<Boundary, Engine, Effects, Observation>
where
    Boundary: AudioThreadBoundary,
    Engine: SoundFontEngine,
    Effects: GlobalEffectsProcessor,
    Observation: CallbackAudioObservation,
{
    #[must_use]
    pub fn with_observation(
        boundary: Boundary,
        engine: Engine,
        mixer: MixEngine<Effects>,
        observation: Observation,
    ) -> Self {
        Self {
            boundary,
            engine,
            mixer,
            observation,
            patch_audio: None,
            active_notes: ActiveNoteObservation::new(),
            rendered_blocks: 0,
            rendered_frames: 0,
            commands_consumed: 0,
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
            self.commands_consumed = self.commands_consumed.saturating_add(1);
            match command {
                AudioCommand::PatchMidi { patch_id, message } => {
                    if self.engine.dispatch(patch_id, message).is_ok() {
                        self.active_notes.observe_patch_message(patch_id, message);
                    } else {
                        self.engine.all_notes_off();
                        self.active_notes.clear_all();
                    }
                }
                AudioCommand::AllNotesOff => {
                    self.engine.all_notes_off();
                    self.active_notes.clear_all();
                }
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
        let mix = self.mixer.mix(patch_audio, &parameters, interleaved_stereo);
        self.rendered_blocks = self.rendered_blocks.saturating_add(1);
        self.rendered_frames = self.rendered_frames.saturating_add(frame_count as u64);
        self.observation
            .publish_from_callback(AudioObservationSnapshot::from_mix(
                self.rendered_blocks,
                self.rendered_blocks,
                self.rendered_frames,
                parameters.generation(),
                self.commands_consumed,
                self.active_notes.count(),
                mix,
            ));
    }
}

#[derive(Clone, Copy)]
struct PatchNoteBits {
    patch_id: Option<PatchId>,
    low: u64,
    high: u64,
}

impl PatchNoteBits {
    const EMPTY: Self = Self {
        patch_id: None,
        low: 0,
        high: 0,
    };

    fn set(&mut self, note: u8) {
        if note < 64 {
            self.low |= 1_u64 << note;
        } else {
            self.high |= 1_u64 << (note - 64);
        }
    }

    fn clear(&mut self, note: u8) {
        if note < 64 {
            self.low &= !(1_u64 << note);
        } else {
            self.high &= !(1_u64 << (note - 64));
        }
    }

    fn clear_all(&mut self) {
        self.low = 0;
        self.high = 0;
    }

    fn count(self) -> u32 {
        self.low.count_ones().saturating_add(self.high.count_ones())
    }
}

struct ActiveNoteObservation {
    patches: [PatchNoteBits; MAX_PATCHES],
}

impl ActiveNoteObservation {
    const fn new() -> Self {
        Self {
            patches: [PatchNoteBits::EMPTY; MAX_PATCHES],
        }
    }

    fn observe_patch_message(
        &mut self,
        patch_id: PatchId,
        message: crate::kernel::midi_message::MidiMessage,
    ) {
        if message.kind() == MidiMessageKind::AllNotesOff {
            if let Some(patch) = self.patch_mut(patch_id, false) {
                patch.clear_all();
            }
            return;
        }

        let note = message.data1();
        match message.kind() {
            MidiMessageKind::NoteOn if message.data2() > 0 => {
                if let Some(patch) = self.patch_mut(patch_id, true) {
                    patch.set(note);
                }
            }
            MidiMessageKind::NoteOn | MidiMessageKind::NoteOff => {
                if let Some(patch) = self.patch_mut(patch_id, false) {
                    patch.clear(note);
                }
            }
            MidiMessageKind::ControlChange
            | MidiMessageKind::ProgramChange
            | MidiMessageKind::ChannelPressure
            | MidiMessageKind::PitchBend
            | MidiMessageKind::AllNotesOff => {}
        }
    }

    fn patch_mut(&mut self, patch_id: PatchId, create: bool) -> Option<&mut PatchNoteBits> {
        if let Some(index) = self
            .patches
            .iter()
            .position(|patch| patch.patch_id == Some(patch_id))
        {
            return self.patches.get_mut(index);
        }
        if !create {
            return None;
        }
        let slot = self
            .patches
            .iter_mut()
            .find(|patch| patch.patch_id.is_none())?;
        slot.patch_id = Some(patch_id);
        Some(slot)
    }

    fn clear_all(&mut self) {
        for patch in &mut self.patches {
            patch.clear_all();
        }
    }

    fn count(&self) -> u32 {
        self.patches
            .iter()
            .fold(0_u32, |count, patch| count.saturating_add(patch.count()))
    }
}

#[cfg(test)]
mod tests {
    use super::{ActiveNoteObservation, AudioError, AudioRenderer};
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::mix_engine::MixEngine;
    use crate::real_time::audio_boundary::{AudioThreadBoundary, RetiredAudioState};
    use crate::real_time::audio_command::AudioCommand;
    use crate::real_time::audio_observation::CallbackAudioObservation;
    use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
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

    #[derive(Default)]
    struct TestObservation {
        latest: AudioObservationSnapshot,
        publications: usize,
    }

    impl CallbackAudioObservation for TestObservation {
        fn publish_from_callback(&mut self, snapshot: AudioObservationSnapshot) {
            self.latest = snapshot;
            self.publications += 1;
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

    fn observing_renderer() -> AudioRenderer<TestBoundary, TestEngine, TestEffects, TestObservation>
    {
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

        AudioRenderer::with_observation(
            boundary,
            TestEngine::default(),
            MixEngine::new(TestEffects),
            TestObservation::default(),
        )
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

    #[test]
    fn audio_observation_realtime_contract() {
        let mut renderer = observing_renderer();
        renderer.prepare(4, 48_000.0).unwrap();
        let mut output = [0.0; 8];

        begin_allocation_count();
        renderer.render(&mut output);
        let callback_allocations = finish_allocation_count();

        let observation = renderer.observation.latest;
        assert_eq!(callback_allocations, 0);
        assert_eq!(renderer.observation.publications, 1);
        assert_eq!(observation.sequence(), 1);
        assert_eq!(observation.rendered_blocks(), 1);
        assert_eq!(observation.rendered_frames(), 4);
        assert_eq!(observation.parameter_generation(), 9);
        assert_eq!(observation.commands_consumed(), 3);
        assert_eq!(observation.active_notes(), 0);
        assert!(observation.output_rms() > 0.0);
        assert!(observation.output_rms().is_finite());
    }

    #[test]
    fn active_note_observation_tracks_patch_lifecycle_with_bounded_bits() {
        let first = PatchId::new(4).unwrap();
        let second = PatchId::new(7).unwrap();
        let mut notes = ActiveNoteObservation::new();

        notes.observe_patch_message(first, midi(MidiMessageKind::NoteOn));
        notes.observe_patch_message(second, midi(MidiMessageKind::NoteOn));
        assert_eq!(notes.count(), 2);

        notes.observe_patch_message(first, midi(MidiMessageKind::NoteOff));
        assert_eq!(notes.count(), 1);
        notes.observe_patch_message(second, midi(MidiMessageKind::AllNotesOff));
        assert_eq!(notes.count(), 0);

        notes.observe_patch_message(first, midi(MidiMessageKind::NoteOn));
        notes.clear_all();
        assert_eq!(notes.count(), 0);
    }
}
