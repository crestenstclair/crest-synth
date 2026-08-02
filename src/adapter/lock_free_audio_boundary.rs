use crate::real_time::audio_boundary::{
    AudioBoundary, AudioThreadBoundary, BoundaryFull, ControlAudioBoundary,
};
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::parameter_snapshot::ParameterSnapshot;
use rtrb::{Consumer, Producer, PushError, RingBuffer};
use triple_buffer::{triple_buffer, Input, Output};

/// A complete lock-free control/audio seam backed by fixed-capacity primitives.
pub struct LockFreeAudioBoundary {
    control: LockFreeControlHandle,
    audio: LockFreeAudioHandle,
}

impl LockFreeAudioBoundary {
    /// Allocates all boundary storage before either handle reaches the audio callback.
    pub fn new(command_capacity: usize, initial_parameters: ParameterSnapshot) -> Self {
        let (command_producer, command_consumer) = RingBuffer::new(command_capacity);
        let (parameter_input, parameter_output) = triple_buffer(&initial_parameters);

        Self {
            control: LockFreeControlHandle {
                commands: command_producer,
                parameters: parameter_input,
            },
            audio: LockFreeAudioHandle {
                commands: command_consumer,
                parameters: parameter_output,
            },
        }
    }
}

impl AudioBoundary for LockFreeAudioBoundary {
    type ControlHandle = LockFreeControlHandle;
    type AudioHandle = LockFreeAudioHandle;

    fn into_handles(self) -> (Self::ControlHandle, Self::AudioHandle) {
        (self.control, self.audio)
    }
}

/// The control-thread half of a LockFreeAudioBoundary.
///
/// This handle owns command publication and latest scalar replacement only.
pub struct LockFreeControlHandle {
    commands: Producer<AudioCommand>,
    parameters: Input<ParameterSnapshot>,
}

impl ControlAudioBoundary for LockFreeControlHandle {
    fn push_command(&mut self, command: AudioCommand) -> Result<(), BoundaryFull> {
        match self.commands.push(command) {
            Ok(()) => Ok(()),
            Err(PushError::Full(command)) => Err(BoundaryFull::new(command)),
        }
    }

    fn publish_parameters(&mut self, parameters: ParameterSnapshot) {
        self.parameters.write(parameters);
    }
}

/// The callback-only half of a LockFreeAudioBoundary.
///
/// Its methods only touch preallocated lock-free command or scalar storage.
pub struct LockFreeAudioHandle {
    commands: Consumer<AudioCommand>,
    parameters: Output<ParameterSnapshot>,
}

impl AudioThreadBoundary for LockFreeAudioHandle {
    fn pop_command(&mut self) -> Option<AudioCommand> {
        self.commands.pop().ok()
    }

    fn read_latest_parameters(&mut self) -> ParameterSnapshot {
        *self.parameters.read()
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
            GlobalParameters::new(0.0).unwrap(),
            crate::mixer::mixer_state::MixerState::default(),
            &[],
        )
        .unwrap()
    }

    #[test]
    fn bounded_commands_remain_fifo_and_return_rejected_values() {
        let boundary = LockFreeAudioBoundary::new(2, parameters(0));
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
        let boundary = LockFreeAudioBoundary::new(1, parameters(0));
        let (mut control, mut audio) = boundary.into_handles();

        control.publish_parameters(parameters(1));
        control.publish_parameters(parameters(2));

        assert_eq!(audio.read_latest_parameters().generation(), 2);
    }

    #[test]
    fn concrete_handles_are_send_but_keep_distinct_capabilities() {
        fn assert_send<T: Send>() {}

        assert_send::<LockFreeControlHandle>();
        assert_send::<LockFreeAudioHandle>();
    }
}
