use crate::real_time::audio_boundary::{
    AudioBoundary, AudioThreadBoundary, BoundaryFull, ControlAudioBoundary, RetiredAudioState,
};
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::parameter_snapshot::ParameterSnapshot;
use basedrop::{Collector, Handle, Owned};
use core::marker::PhantomData;
use rtrb::{Consumer, Producer, PushError, RingBuffer};
use triple_buffer::{triple_buffer, Input, Output};

/// A complete lock-free control/audio seam backed by fixed-capacity primitives.
pub struct LockFreeAudioBoundary<State: Send + 'static> {
    control: LockFreeControlHandle<State>,
    audio: LockFreeAudioHandle<State>,
}

impl<State: Send + 'static> LockFreeAudioBoundary<State> {
    /// Allocates all boundary storage before either handle reaches the audio callback.
    pub fn new(command_capacity: usize, initial_parameters: ParameterSnapshot) -> Self {
        let (command_producer, command_consumer) = RingBuffer::new(command_capacity);
        let (parameter_input, parameter_output) = triple_buffer(&initial_parameters);
        let collector = Collector::new();
        let retirement_handle = collector.handle();

        Self {
            control: LockFreeControlHandle {
                commands: command_producer,
                parameters: parameter_input,
                collector: Some(collector),
                retirement_handle: Some(retirement_handle),
                state: PhantomData,
            },
            audio: LockFreeAudioHandle {
                commands: command_consumer,
                parameters: parameter_output,
                state: PhantomData,
            },
        }
    }
}

impl<State: Send + 'static> AudioBoundary for LockFreeAudioBoundary<State> {
    type RetiredState = Owned<State>;
    type ControlHandle = LockFreeControlHandle<State>;
    type AudioHandle = LockFreeAudioHandle<State>;

    fn into_handles(self) -> (Self::ControlHandle, Self::AudioHandle) {
        (self.control, self.audio)
    }
}

/// The control-thread half of a LockFreeAudioBoundary.
///
/// This handle owns every allocating and destructor-running operation.
pub struct LockFreeControlHandle<State: Send + 'static> {
    commands: Producer<AudioCommand>,
    parameters: Input<ParameterSnapshot>,
    collector: Option<Collector>,
    retirement_handle: Option<Handle>,
    state: PhantomData<State>,
}

impl<State: Send + 'static> LockFreeControlHandle<State> {
    /// Allocates engine-owned state under the deferred-drop collector.
    ///
    /// Call this on the control thread before the value can reach the callback.
    pub fn prepare_retirement(&self, state: State) -> RetiredAudioState<Owned<State>> {
        let handle = self
            .retirement_handle
            .as_ref()
            .expect("retirement handle is present until control handle drop");
        RetiredAudioState::new(Owned::new(handle, state))
    }
}

impl<State: Send + 'static> ControlAudioBoundary for LockFreeControlHandle<State> {
    fn push_command(&mut self, command: AudioCommand) -> Result<(), BoundaryFull> {
        match self.commands.push(command) {
            Ok(()) => Ok(()),
            Err(PushError::Full(command)) => Err(BoundaryFull::new(command)),
        }
    }

    fn publish_parameters(&mut self, parameters: ParameterSnapshot) {
        self.parameters.write(parameters);
    }

    fn collect(&mut self) {
        self.collector
            .as_mut()
            .expect("collector is present until control handle drop")
            .collect();
    }
}

impl<State: Send + 'static> Drop for LockFreeControlHandle<State> {
    fn drop(&mut self) {
        if let Some(collector) = self.collector.as_mut() {
            collector.collect();
        }
        self.retirement_handle.take();

        if let Some(collector) = self.collector.take() {
            let _cleanup_result = collector.try_cleanup();
        }
    }
}

/// The callback-only half of a LockFreeAudioBoundary.
///
/// Its methods only touch preallocated lock-free storage or enqueue a
/// basedrop Owned allocation for later control-thread destruction.
pub struct LockFreeAudioHandle<State: Send + 'static> {
    commands: Consumer<AudioCommand>,
    parameters: Output<ParameterSnapshot>,
    state: PhantomData<State>,
}

impl<State: Send + 'static> AudioThreadBoundary for LockFreeAudioHandle<State> {
    type RetiredState = Owned<State>;

    fn pop_command(&mut self) -> Option<AudioCommand> {
        self.commands.pop().ok()
    }

    fn read_latest_parameters(&mut self) -> ParameterSnapshot {
        *self.parameters.read()
    }

    fn retire(&mut self, state: RetiredAudioState<Self::RetiredState>) {
        drop(state.into_inner());
    }
}

#[cfg(test)]
mod tests {
    use super::{LockFreeAudioBoundary, LockFreeAudioHandle, LockFreeControlHandle};
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::audio_boundary::{
        AudioBoundary, AudioThreadBoundary, BoundaryFull, ControlAudioBoundary,
    };
    use crate::real_time::audio_command::AudioCommand;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;
    use core::sync::atomic::{AtomicUsize, Ordering};

    fn command(note: u8) -> AudioCommand {
        AudioCommand::patch_midi(
            PatchId::new(1).unwrap(),
            MidiMessage::try_new(
                MidiChannel::new(0).unwrap(),
                MidiMessageKind::NoteOn,
                note,
                100,
            )
            .unwrap(),
        )
    }

    fn parameters(generation: u64) -> ParameterSnapshot {
        ParameterSnapshot::new(
            generation,
            GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5).unwrap(),
            &[],
        )
        .unwrap()
    }

    #[test]
    fn bounded_commands_remain_fifo_and_return_rejected_values() {
        let boundary = LockFreeAudioBoundary::<DropProbe>::new(2, parameters(0));
        let (mut control, mut audio) = boundary.into_handles();

        control.push_command(command(60)).unwrap();
        control.push_command(command(62)).unwrap();
        assert_eq!(
            control.push_command(command(64)),
            Err(BoundaryFull::new(command(64)))
        );

        assert_eq!(audio.pop_command(), Some(command(60)));
        assert_eq!(audio.pop_command(), Some(command(62)));
        assert_eq!(audio.pop_command(), None);
    }

    #[test]
    fn parameter_publication_is_latest_wins_and_complete() {
        let boundary = LockFreeAudioBoundary::<DropProbe>::new(1, parameters(0));
        let (mut control, mut audio) = boundary.into_handles();

        control.publish_parameters(parameters(1));
        control.publish_parameters(parameters(2));

        assert_eq!(audio.read_latest_parameters().generation(), 2);
    }

    static DROPS: AtomicUsize = AtomicUsize::new(0);

    struct DropProbe;

    impl Drop for DropProbe {
        fn drop(&mut self) {
            DROPS.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn callback_retirement_defers_destruction_until_control_collection() {
        DROPS.store(0, Ordering::SeqCst);
        let boundary = LockFreeAudioBoundary::<DropProbe>::new(1, parameters(0));
        let (mut control, mut audio) = boundary.into_handles();
        let retired = control.prepare_retirement(DropProbe);

        audio.retire(retired);
        assert_eq!(DROPS.load(Ordering::SeqCst), 0);

        control.collect();
        assert_eq!(DROPS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn concrete_handles_are_send_but_keep_distinct_capabilities() {
        fn assert_send<T: Send>() {}

        assert_send::<LockFreeControlHandle<DropProbe>>();
        assert_send::<LockFreeAudioHandle<DropProbe>>();
    }
}
