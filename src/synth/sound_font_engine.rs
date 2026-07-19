use crate::kernel::midi_message::MidiMessage;
use crate::kernel::patch_id::PatchId;
use crate::real_time::parameter_snapshot::ParameterSnapshot;
use crate::real_time::patch_audio_block::PatchAudioBlock;
use crate::synth::patch::Patch;
use core::fmt;
use std::path::Path;

/// A bounded SoundFont engine failure.
///
/// The value owns no heap storage, so dispatch can report a failure on the audio
/// thread without allocating or transferring destructor-bearing data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoundFontError {
    SoundFontFileUnavailable,
    InvalidSoundFontData,
    EngineNotLoaded,
    PatchCapacityExceeded { capacity: usize },
    PatchAlreadyConfigured { patch_id: PatchId },
    PatchConfigurationFailed { patch_id: PatchId },
    UnknownPatch { patch_id: PatchId },
    MidiDispatchFailed { patch_id: PatchId },
}

impl fmt::Display for SoundFontError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::SoundFontFileUnavailable => {
                formatter.write_str("SoundFont file is unavailable or unreadable")
            }
            Self::InvalidSoundFontData => {
                formatter.write_str("SoundFont file does not contain valid synthesizer data")
            }
            Self::EngineNotLoaded => {
                formatter.write_str("SoundFont engine must be loaded before patch configuration")
            }
            Self::PatchCapacityExceeded { capacity } => {
                write!(
                    formatter,
                    "SoundFont engine patch capacity of {capacity} was exceeded"
                )
            }
            Self::PatchAlreadyConfigured { patch_id } => {
                write!(
                    formatter,
                    "SoundFont patch {patch_id} is already configured"
                )
            }
            Self::PatchConfigurationFailed { patch_id } => {
                write!(
                    formatter,
                    "SoundFont patch {patch_id} could not be configured"
                )
            }
            Self::UnknownPatch { patch_id } => {
                write!(formatter, "SoundFont patch {patch_id} is not configured")
            }
            Self::MidiDispatchFailed { patch_id } => {
                write!(
                    formatter,
                    "MIDI could not be dispatched to SoundFont patch {patch_id}"
                )
            }
        }
    }
}

impl std::error::Error for SoundFontError {}

/// Outbound synthesis port for the application's one SoundFont engine.
///
/// The composition root constructs exactly one implementation. Load and
/// configure_patch run on the control thread before audio starts. The prepared
/// engine is then transferred to the audio side, where dispatch, all_notes_off,
/// and render_patches must use only bounded preallocated state.
pub trait SoundFontEngine: Send {
    /// Loads one SoundFont before any Patch is configured.
    ///
    /// This is a control-thread operation and may perform file I/O and allocate.
    fn load(&mut self, path: &Path) -> Result<(), SoundFontError>;

    /// Installs one Patch's bank, program, percussion, channel, and identity.
    ///
    /// This is a control-thread operation. The implementation must finish every
    /// allocation and lookup needed by that Patch before returning.
    fn configure_patch(&mut self, patch: &Patch) -> Result<(), SoundFontError>;

    /// Delivers one normalized MIDI message to its configured Patch.
    ///
    /// This audio-thread operation must be allocation-free, lock-free,
    /// non-blocking, and free of I/O, logging, formatting, and destruction.
    fn dispatch(&mut self, patch_id: PatchId, message: MidiMessage) -> Result<(), SoundFontError>;

    /// Silences all active voices without allocating or blocking.
    fn all_notes_off(&mut self);

    /// Fills every active caller-owned Patch stereo stem.
    ///
    /// Each stem must contain audio only for its matching configured Patch.
    /// The implementation must not resize or retain the block and must perform
    /// no allocation, locking, blocking, I/O, logging, formatting, or
    /// destruction.
    fn render_patches(&mut self, block: &mut PatchAudioBlock, parameters: &ParameterSnapshot);
}

#[cfg(test)]
mod tests {
    use super::{SoundFontEngine, SoundFontError};
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
    use crate::real_time::patch_audio_block::PatchAudioBlock;
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use std::path::Path;

    fn patch() -> Patch {
        Patch::new(
            PatchId::new(7).unwrap(),
            "Strings".to_owned(),
            SoundFontInstrument::new(128, 48, false).unwrap(),
            MidiChannel::new(2).unwrap(),
            ChannelParameters::new(-6.0, 0.1, 0.25, 0.15).unwrap(),
        )
    }

    fn message() -> MidiMessage {
        MidiMessage::try_new(
            MidiChannel::new(2).unwrap(),
            MidiMessageKind::NoteOn,
            60,
            100,
        )
        .unwrap()
    }

    fn parameters(patch: &Patch) -> ParameterSnapshot {
        ParameterSnapshot::new(
            3,
            GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5).unwrap(),
            &[RtPatchParameters::new(patch.id(), *patch.parameters())],
        )
        .unwrap()
    }

    #[derive(Default)]
    struct TestEngine {
        loaded: bool,
        configured: Option<PatchId>,
        dispatched: Option<(PatchId, MidiMessage)>,
        all_notes_off_count: usize,
    }

    impl SoundFontEngine for TestEngine {
        fn load(&mut self, path: &Path) -> Result<(), SoundFontError> {
            if path != Path::new("./sf2/HiDef.sf2") {
                return Err(SoundFontError::SoundFontFileUnavailable);
            }
            self.loaded = true;
            Ok(())
        }

        fn configure_patch(&mut self, patch: &Patch) -> Result<(), SoundFontError> {
            if !self.loaded {
                return Err(SoundFontError::EngineNotLoaded);
            }
            if self.configured == Some(patch.id()) {
                return Err(SoundFontError::PatchAlreadyConfigured {
                    patch_id: patch.id(),
                });
            }
            self.configured = Some(patch.id());
            Ok(())
        }

        fn dispatch(
            &mut self,
            patch_id: PatchId,
            message: MidiMessage,
        ) -> Result<(), SoundFontError> {
            if self.configured != Some(patch_id) {
                return Err(SoundFontError::UnknownPatch { patch_id });
            }
            self.dispatched = Some((patch_id, message));
            Ok(())
        }

        fn all_notes_off(&mut self) {
            self.dispatched = None;
            self.all_notes_off_count += 1;
        }

        fn render_patches(&mut self, block: &mut PatchAudioBlock, parameters: &ParameterSnapshot) {
            let Some(patch_id) = self.configured else {
                return;
            };
            let Some(index) = parameters
                .patches()
                .iter()
                .position(|patch| patch.patch_id() == Some(patch_id))
            else {
                return;
            };
            if let Some(samples) = block.stem_mut(index, patch_id) {
                samples.fill(0.25);
            }
        }
    }

    #[test]
    fn one_engine_loads_configures_dispatches_and_fills_caller_owned_patch_stem() {
        let patch = patch();
        let snapshot = parameters(&patch);
        let mut engine = TestEngine::default();
        let engine_port: &mut dyn SoundFontEngine = &mut engine;

        engine_port.load(Path::new("./sf2/HiDef.sf2")).unwrap();
        engine_port.configure_patch(&patch).unwrap();
        engine_port.dispatch(patch.id(), message()).unwrap();

        let mut output = PatchAudioBlock::prepare(4).unwrap();
        output.begin_render(&snapshot, 4).unwrap();
        engine_port.render_patches(&mut output, &snapshot);

        assert_eq!(output.stem(0, patch.id()).unwrap().samples(), [0.25; 8]);
        assert_eq!(output.patch_count(), 1);
        assert_eq!(engine.dispatched, Some((patch.id(), message())));
    }

    #[test]
    fn configuration_requires_a_loaded_soundfont() {
        let patch = patch();
        let mut engine = TestEngine::default();

        assert_eq!(
            engine.configure_patch(&patch),
            Err(SoundFontError::EngineNotLoaded)
        );
    }

    #[test]
    fn dispatch_rejects_an_unconfigured_patch_without_heap_state() {
        let mut engine = TestEngine::default();
        let patch_id = PatchId::new(99).unwrap();

        assert_eq!(
            engine.dispatch(patch_id, message()),
            Err(SoundFontError::UnknownPatch { patch_id })
        );
        assert!(!core::mem::needs_drop::<SoundFontError>());
    }

    #[test]
    fn all_notes_off_is_a_bounded_audio_operation() {
        let patch = patch();
        let mut engine = TestEngine::default();
        engine.load(Path::new("./sf2/HiDef.sf2")).unwrap();
        engine.configure_patch(&patch).unwrap();
        engine.dispatch(patch.id(), message()).unwrap();

        engine.all_notes_off();

        assert_eq!(engine.dispatched, None);
        assert_eq!(engine.all_notes_off_count, 1);
    }

    #[test]
    fn errors_are_copyable_and_actionable() {
        fn assert_copy<T: Copy>() {}

        assert_copy::<SoundFontError>();
        assert_eq!(
            SoundFontError::PatchCapacityExceeded { capacity: 16 }.to_string(),
            "SoundFont engine patch capacity of 16 was exceeded"
        );
    }
}
