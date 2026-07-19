use crate::kernel::midi_message::MidiMessage;
use crate::kernel::patch_id::PatchId;

/// A bounded command transferred from the control side to the audio callback.
///
/// Commands contain only copyable domain values, so moving them through the
/// event ring does not allocate or transfer heap ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioCommand {
    /// Delivers one normalized MIDI message to its configured patch.
    PatchMidi {
        patch_id: PatchId,
        message: MidiMessage,
    },
    /// Requests bounded recovery by silencing every active note.
    AllNotesOff,
}

impl AudioCommand {
    /// Creates a patch-targeted MIDI command.
    pub const fn patch_midi(patch_id: PatchId, message: MidiMessage) -> Self {
        Self::PatchMidi { patch_id, message }
    }

    /// Creates the global all-notes-off recovery command.
    pub const fn all_notes_off() -> Self {
        Self::AllNotesOff
    }

    /// Returns the target patch for a patch MIDI command.
    pub const fn patch_id(self) -> Option<PatchId> {
        match self {
            Self::PatchMidi { patch_id, .. } => Some(patch_id),
            Self::AllNotesOff => None,
        }
    }

    /// Returns the normalized MIDI payload for a patch MIDI command.
    pub const fn message(self) -> Option<MidiMessage> {
        match self {
            Self::PatchMidi { message, .. } => Some(message),
            Self::AllNotesOff => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AudioCommand;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;

    fn example_message() -> MidiMessage {
        MidiMessage::try_new(
            MidiChannel::new(2).unwrap(),
            MidiMessageKind::NoteOn,
            60,
            100,
        )
        .unwrap()
    }

    #[test]
    fn patch_midi_keeps_the_target_and_message_together() {
        let patch_id = PatchId::new(7).unwrap();
        let message = example_message();
        let command = AudioCommand::patch_midi(patch_id, message);

        assert_eq!(command.patch_id(), Some(patch_id));
        assert_eq!(command.message(), Some(message));
        assert_eq!(command, AudioCommand::PatchMidi { patch_id, message });
    }

    #[test]
    fn all_notes_off_has_no_patch_or_message_payload() {
        let command = AudioCommand::all_notes_off();

        assert_eq!(command, AudioCommand::AllNotesOff);
        assert_eq!(command.patch_id(), None);
        assert_eq!(command.message(), None);
    }

    #[test]
    fn commands_are_copyable_and_require_no_destruction() {
        fn assert_copy<T: Copy>() {}

        assert_copy::<AudioCommand>();
        let command = AudioCommand::patch_midi(PatchId::new(1).unwrap(), example_message());
        let copied = command;

        assert_eq!(command, copied);
        assert!(!core::mem::needs_drop::<AudioCommand>());
    }
}
