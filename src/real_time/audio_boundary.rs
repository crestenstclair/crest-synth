use crate::real_time::audio_command::AudioCommand;
use crate::real_time::parameter_snapshot::ParameterSnapshot;
use core::fmt;
use core::mem::ManuallyDrop;

/// Returned when the bounded command queue cannot accept another command.
///
/// The rejected command is preserved so control-side code can retry or apply
/// its explicit overflow policy without reconstructing the event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundaryFull {
    command: AudioCommand,
}

impl BoundaryFull {
    pub const fn new(command: AudioCommand) -> Self {
        Self { command }
    }

    pub const fn command(&self) -> AudioCommand {
        self.command
    }

    pub const fn into_command(self) -> AudioCommand {
        self.command
    }
}

impl fmt::Display for BoundaryFull {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the bounded audio command queue is full")
    }
}

impl std::error::Error for BoundaryFull {}

/// Ownership token for engine state that must be destroyed on the control side.
///
/// Constructing this token only moves State into ManuallyDrop; it performs no
/// allocation. Dropping an uncollected token deliberately does not destroy the
/// state on the current thread. A boundary implementation must transfer the
/// token away from the callback and have its control-side collect operation call
/// into_inner before dropping the state.
#[must_use = "retired audio state must be transferred to the audio boundary"]
pub struct RetiredAudioState<State> {
    state: ManuallyDrop<State>,
}

impl<State> RetiredAudioState<State> {
    pub const fn new(state: State) -> Self {
        Self {
            state: ManuallyDrop::new(state),
        }
    }

    /// Recovers ownership for destruction by the control-side collector.
    pub fn into_inner(self) -> State {
        ManuallyDrop::into_inner(self.state)
    }
}

impl<State> fmt::Debug for RetiredAudioState<State> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RetiredAudioState")
            .finish_non_exhaustive()
    }
}

/// Control-thread operations for the real-time seam.
///
/// Implementations use the producer side of a bounded SPSC command queue, the
/// publisher side of a latest-wins snapshot transfer, and the collector for
/// retired audio state.
pub trait ControlAudioBoundary: Send {
    fn push_command(&mut self, command: AudioCommand) -> Result<(), BoundaryFull>;

    fn publish_parameters(&mut self, parameters: ParameterSnapshot);

    /// Reclaims retired state. This operation must run only on the control
    /// thread and may execute destructors.
    fn collect(&mut self);
}

/// Hard real-time operations available to the audio callback.
///
/// Implementations must make every method allocation-free, lock-free,
/// non-blocking, and free of I/O, logging, formatting, and destruction.
pub trait AudioThreadBoundary: Send {
    type RetiredState: Send + 'static;

    fn pop_command(&mut self) -> Option<AudioCommand>;

    fn read_latest_parameters(&mut self) -> ParameterSnapshot;

    /// Transfers state for later control-thread destruction.
    ///
    /// The implementation must not drop the token or its inner state before
    /// ownership has crossed to the control-side collector.
    fn retire(&mut self, state: RetiredAudioState<Self::RetiredState>);
}

/// Factory contract for one complete lock-free control/audio boundary.
///
/// Splitting consumes the factory and returns narrow handles. Callback code that
/// owns only AudioHandle cannot call publish_parameters, push_command, or
/// collect.
pub trait AudioBoundary: Send + Sized {
    type RetiredState: Send + 'static;
    type ControlHandle: ControlAudioBoundary;
    type AudioHandle: AudioThreadBoundary<RetiredState = Self::RetiredState>;

    fn into_handles(self) -> (Self::ControlHandle, Self::AudioHandle);
}

#[cfg(test)]
mod tests {
    use super::{
        AudioBoundary, AudioThreadBoundary, BoundaryFull, ControlAudioBoundary, RetiredAudioState,
    };
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::audio_command::AudioCommand;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;
    use core::sync::atomic::{AtomicUsize, Ordering};

    fn command() -> AudioCommand {
        AudioCommand::patch_midi(
            PatchId::new(1).unwrap(),
            MidiMessage::try_new(
                MidiChannel::new(0).unwrap(),
                MidiMessageKind::NoteOn,
                60,
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

    struct TestControl {
        queued: Option<AudioCommand>,
        latest: ParameterSnapshot,
        collections: usize,
    }

    impl ControlAudioBoundary for TestControl {
        fn push_command(&mut self, command: AudioCommand) -> Result<(), BoundaryFull> {
            if self.queued.is_some() {
                Err(BoundaryFull::new(command))
            } else {
                self.queued = Some(command);
                Ok(())
            }
        }

        fn publish_parameters(&mut self, parameters: ParameterSnapshot) {
            self.latest = parameters;
        }

        fn collect(&mut self) {
            self.collections += 1;
        }
    }

    struct TestAudio {
        queued: Option<AudioCommand>,
        latest: ParameterSnapshot,
        retired: Option<RetiredAudioState<DropProbe>>,
    }

    impl AudioThreadBoundary for TestAudio {
        type RetiredState = DropProbe;

        fn pop_command(&mut self) -> Option<AudioCommand> {
            self.queued.take()
        }

        fn read_latest_parameters(&mut self) -> ParameterSnapshot {
            self.latest
        }

        fn retire(&mut self, state: RetiredAudioState<Self::RetiredState>) {
            self.retired = Some(state);
        }
    }

    struct TestBoundary;

    impl AudioBoundary for TestBoundary {
        type RetiredState = DropProbe;
        type ControlHandle = TestControl;
        type AudioHandle = TestAudio;

        fn into_handles(self) -> (Self::ControlHandle, Self::AudioHandle) {
            (
                TestControl {
                    queued: None,
                    latest: parameters(0),
                    collections: 0,
                },
                TestAudio {
                    queued: Some(command()),
                    latest: parameters(1),
                    retired: None,
                },
            )
        }
    }

    static DROPS: AtomicUsize = AtomicUsize::new(0);

    struct DropProbe;

    impl Drop for DropProbe {
        fn drop(&mut self) {
            DROPS.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn boundary_full_preserves_the_rejected_command() {
        let rejected = command();
        let error = BoundaryFull::new(rejected);

        assert_eq!(error.command(), rejected);
        assert_eq!(error.into_command(), rejected);
        assert_eq!(error.to_string(), "the bounded audio command queue is full");
    }

    #[test]
    fn split_handles_expose_their_narrow_operations() {
        let (mut control, mut audio) = TestBoundary.into_handles();

        control.push_command(command()).unwrap();
        assert_eq!(
            control.push_command(AudioCommand::all_notes_off()),
            Err(BoundaryFull::new(AudioCommand::all_notes_off()))
        );
        control.publish_parameters(parameters(2));
        control.collect();

        assert_eq!(audio.pop_command(), Some(command()));
        assert_eq!(audio.pop_command(), None);
        assert_eq!(audio.read_latest_parameters().generation(), 1);
        assert_eq!(control.latest.generation(), 2);
        assert_eq!(control.collections, 1);
    }

    #[test]
    fn retired_state_drops_only_after_control_side_recovery() {
        DROPS.store(0, Ordering::SeqCst);

        let token = RetiredAudioState::new(DropProbe);
        drop(token);
        assert_eq!(DROPS.load(Ordering::SeqCst), 0);

        let token = RetiredAudioState::new(DropProbe);
        drop(token.into_inner());
        assert_eq!(DROPS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn real_time_values_are_copyable_and_need_no_destruction() {
        fn assert_copy<T: Copy>() {}

        assert_copy::<AudioCommand>();
        assert_copy::<ParameterSnapshot>();
        assert!(!core::mem::needs_drop::<AudioCommand>());
        assert!(!core::mem::needs_drop::<ParameterSnapshot>());
    }
}
