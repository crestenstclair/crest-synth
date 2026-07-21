use crate::adapter::hidef_soundfont_capability::{
    HIDEF_CAPABILITY_ID, SOUNDFONT_BANK_PARAMETER_ID, SOUNDFONT_FILE_PARAMETER_ID,
    SOUNDFONT_PERCUSSION_PARAMETER_ID, SOUNDFONT_PROGRAM_PARAMETER_ID,
};
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::patch_id::PatchId;
use crate::real_time::parameter_snapshot::{ParameterSnapshot, MAX_PATCHES};
use crate::real_time::patch_audio_block::PatchAudioBlock;
use crate::synth::instrument_capability::{AssetKind, ParameterValue};
use crate::synth::parameter_id::ParameterId;
use crate::synth::patch::Patch;
use crate::synth::sound_font_engine::{SoundFontEngine, SoundFontError};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

pub const HIDEF_SOUNDFONT_PATH: &str = "./sf2/HiDef.sf2";

const DEFAULT_SAMPLE_RATE: i32 = 48_000;
const DEFAULT_MAX_FRAMES: usize = 1_024;
const SYNTHESIZER_BLOCK_SIZE: usize = 64;
const PERCUSSION_BANK_FLAG: i32 = 128;
const MELODIC_CHANNEL: i32 = 0;

/// The in-process adapter for the single shared HiDef.sf2 bank.
///
/// Loading and lane preparation happen on the control thread. Each configured
/// Patch owns one bounded rustysynth lane inside this adapter, so dispatch and
/// rendering preserve Patch identity without callback allocation or locking.
pub struct HiDefSoundFontEngine {
    sample_rate: i32,
    max_frames: usize,
    sound_font: Option<Arc<SoundFont>>,
    lanes: [Option<RenderLane>; MAX_PATCHES],
    patch_count: usize,
    left_scratch: Vec<f32>,
    right_scratch: Vec<f32>,
}

impl HiDefSoundFontEngine {
    #[must_use]
    pub fn new(sample_rate: i32, max_frames: usize) -> Self {
        let max_frames = max_frames.max(1);
        Self {
            sample_rate,
            max_frames,
            sound_font: None,
            lanes: std::array::from_fn(|_| None),
            patch_count: 0,
            left_scratch: vec![0.0; max_frames],
            right_scratch: vec![0.0; max_frames],
        }
    }

    fn prepared_patch(&self, patch_id: PatchId) -> Option<PreparedPatch> {
        self.lanes[..self.patch_count]
            .iter()
            .flatten()
            .find(|lane| lane.prepared.patch_id == patch_id)
            .map(|lane| lane.prepared)
    }

    fn render_lane_mut(&mut self, patch_id: PatchId) -> Option<&mut RenderLane> {
        self.lanes[..self.patch_count]
            .iter_mut()
            .flatten()
            .find(|lane| lane.prepared.patch_id == patch_id)
    }
}

impl Default for HiDefSoundFontEngine {
    fn default() -> Self {
        Self::new(DEFAULT_SAMPLE_RATE, DEFAULT_MAX_FRAMES)
    }
}

impl SoundFontEngine for HiDefSoundFontEngine {
    fn load(&mut self, path: &Path) -> Result<(), SoundFontError> {
        if path != Path::new(HIDEF_SOUNDFONT_PATH) {
            return Err(SoundFontError::SoundFontFileUnavailable);
        }
        if self.sound_font.is_some() {
            return Ok(());
        }
        if !(16_000..=192_000).contains(&self.sample_rate) {
            return Err(SoundFontError::InvalidSoundFontData);
        }

        let mut file = File::open(path).map_err(|_| SoundFontError::SoundFontFileUnavailable)?;
        let sound_font =
            SoundFont::new(&mut file).map_err(|_| SoundFontError::InvalidSoundFontData)?;
        self.sound_font = Some(Arc::new(sound_font));
        Ok(())
    }

    fn configure_patch(&mut self, patch: &Patch) -> Result<(), SoundFontError> {
        let sound_font = self
            .sound_font
            .as_ref()
            .ok_or(SoundFontError::EngineNotLoaded)?;
        if self.prepared_patch(patch.id()).is_some() {
            return Err(SoundFontError::PatchAlreadyConfigured {
                patch_id: patch.id(),
            });
        }
        if self.patch_count == MAX_PATCHES {
            return Err(SoundFontError::PatchCapacityExceeded {
                capacity: MAX_PATCHES,
            });
        }

        let prepared = PreparedPatch::try_from_patch(patch)?;
        if self.lanes[..self.patch_count]
            .iter()
            .flatten()
            .any(|lane| lane.prepared.logical_channel == prepared.logical_channel)
        {
            return Err(SoundFontError::PatchConfigurationFailed {
                patch_id: patch.id(),
            });
        }
        if !sound_font.get_presets().iter().any(|preset| {
            preset.get_bank_number() == prepared.effective_bank
                && preset.get_patch_number() == prepared.program
        }) {
            return Err(SoundFontError::PatchConfigurationFailed {
                patch_id: patch.id(),
            });
        }

        let mut settings = SynthesizerSettings::new(self.sample_rate);
        settings.block_size = SYNTHESIZER_BLOCK_SIZE;
        settings.enable_reverb_and_chorus = false;
        let mut synthesizer = Synthesizer::new(sound_font, &settings)
            .map_err(|_| SoundFontError::InvalidSoundFontData)?;
        synthesizer.set_master_volume(1.0);
        prepared.apply_to(&mut synthesizer);

        self.lanes[self.patch_count] = Some(RenderLane {
            prepared,
            synthesizer,
        });
        self.patch_count += 1;
        Ok(())
    }

    fn dispatch(&mut self, patch_id: PatchId, message: MidiMessage) -> Result<(), SoundFontError> {
        let lane = self
            .render_lane_mut(patch_id)
            .ok_or(SoundFontError::UnknownPatch { patch_id })?;
        lane.prepared.apply_to(&mut lane.synthesizer);
        dispatch_message(
            &mut lane.synthesizer,
            lane.prepared.internal_channel,
            message,
        );
        Ok(())
    }

    fn all_notes_off(&mut self) {
        for lane in self.lanes[..self.patch_count].iter_mut().flatten() {
            lane.synthesizer.note_off_all(false);
        }
    }

    fn render_patches(&mut self, block: &mut PatchAudioBlock, parameters: &ParameterSnapshot) {
        block.clear();
        let frame_count = block.frame_count().min(self.max_frames);
        if frame_count == 0 {
            return;
        }

        let lanes = &mut self.lanes[..self.patch_count];
        let left = &mut self.left_scratch[..frame_count];
        let right = &mut self.right_scratch[..frame_count];

        for (index, patch) in parameters.patches().iter().enumerate() {
            let Some(patch_id) = patch.patch_id() else {
                continue;
            };
            let Some(lane) = lanes
                .iter_mut()
                .flatten()
                .find(|lane| lane.prepared.patch_id == patch_id)
            else {
                continue;
            };

            left.fill(0.0);
            right.fill(0.0);
            lane.synthesizer.render(left, right);

            let Some(stem) = block.stem_mut(index, patch_id) else {
                continue;
            };
            for frame in 0..frame_count {
                stem[frame * 2] = bounded_sample(left[frame]);
                stem[frame * 2 + 1] = bounded_sample(right[frame]);
            }
        }
    }
}

struct RenderLane {
    prepared: PreparedPatch,
    synthesizer: Synthesizer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreparedPatch {
    patch_id: PatchId,
    logical_channel: i32,
    internal_channel: i32,
    effective_bank: i32,
    bank_select_value: i32,
    program: i32,
}

impl PreparedPatch {
    fn try_from_patch(patch: &Patch) -> Result<Self, SoundFontError> {
        let config = patch.instrument_config();
        if config.capability_id().as_str() != HIDEF_CAPABILITY_ID {
            return Err(SoundFontError::UnsupportedCapability {
                patch_id: patch.id(),
            });
        }
        let parameter_id = |value: &str| {
            ParameterId::new(value).expect("HiDef parameter constants are valid stable ids")
        };
        let bank = match config.value(&parameter_id(SOUNDFONT_BANK_PARAMETER_ID)) {
            Some(ParameterValue::Stepped(value)) => u16::try_from(*value).ok(),
            _ => None,
        };
        let program = match config.value(&parameter_id(SOUNDFONT_PROGRAM_PARAMETER_ID)) {
            Some(ParameterValue::Stepped(value)) => u8::try_from(*value).ok(),
            _ => None,
        };
        let percussion = match config.value(&parameter_id(SOUNDFONT_PERCUSSION_PARAMETER_ID)) {
            Some(ParameterValue::Toggle(value)) => Some(*value),
            _ => None,
        };
        let file = config.asset_reference(&parameter_id(SOUNDFONT_FILE_PARAMETER_ID));
        let (Some(bank), Some(program), Some(percussion), Some(file)) =
            (bank, program, percussion, file)
        else {
            return Err(SoundFontError::InvalidInstrumentConfig {
                patch_id: patch.id(),
            });
        };
        if program > 127
            || file.kind() != AssetKind::SoundFont
            || file.locator() != HIDEF_SOUNDFONT_PATH
            || config.values().len() != 3
            || config.asset_references().len() != 1
        {
            return Err(SoundFontError::InvalidInstrumentConfig {
                patch_id: patch.id(),
            });
        }

        let numeric_bank = i32::from(bank);
        let effective_bank = if percussion {
            numeric_bank | PERCUSSION_BANK_FLAG
        } else {
            numeric_bank & !PERCUSSION_BANK_FLAG
        };
        let internal_channel = if percussion {
            Synthesizer::PERCUSSION_CHANNEL as i32
        } else {
            MELODIC_CHANNEL
        };
        let bank_select_value = if percussion {
            effective_bank - PERCUSSION_BANK_FLAG
        } else {
            effective_bank
        };

        Ok(Self {
            patch_id: patch.id(),
            logical_channel: i32::from(patch.channel().value()),
            internal_channel,
            effective_bank,
            bank_select_value,
            program: i32::from(program),
        })
    }

    fn apply_to(self, synthesizer: &mut Synthesizer) {
        synthesizer.process_midi_message(self.internal_channel, 0xB0, 0, self.bank_select_value);
        synthesizer.process_midi_message(self.internal_channel, 0xC0, self.program, 0);
    }
}

fn dispatch_message(synthesizer: &mut Synthesizer, channel: i32, message: MidiMessage) {
    let (command, data1, data2) = midi_data(message);
    synthesizer.process_midi_message(channel, command, data1, data2);
}

fn midi_data(message: MidiMessage) -> (i32, i32, i32) {
    let data1 = i32::from(message.data1());
    let data2 = i32::from(message.data2());
    match message.kind() {
        MidiMessageKind::NoteOff => (0x80, data1, data2),
        MidiMessageKind::NoteOn => (0x90, data1, data2),
        MidiMessageKind::ControlChange => (0xB0, data1, data2),
        MidiMessageKind::ProgramChange => (0xC0, data1, data2),
        MidiMessageKind::ChannelPressure => (0xD0, data1, data2),
        MidiMessageKind::PitchBend => (0xE0, data1, data2),
        MidiMessageKind::AllNotesOff => (0xB0, 0x7B, 0),
    }
}

fn bounded_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bounded_sample, midi_data, HiDefSoundFontEngine, PreparedPatch, HIDEF_SOUNDFONT_PATH,
        MELODIC_CHANNEL,
    };
    use crate::adapter::hidef_soundfont_capability::{
        HiDefSoundFontCapability, HIDEF_CAPABILITY_ID,
    };
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
    use crate::real_time::patch_audio_block::PatchAudioBlock;
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_engine::{SoundFontEngine, SoundFontError};
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::{CapabilityId, InstrumentConfig};
    use crate::testing::automatic_midi_test::create_soundfont_config;
    use rustysynth::Synthesizer;
    use std::path::Path;

    fn patch(id: u32, channel: u8, bank: u16, program: u8, percussion: bool) -> Patch {
        let provider = HiDefSoundFontCapability::new().unwrap();
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            create_soundfont_config(
                &provider,
                SoundFontInstrument::new(bank, program, percussion).unwrap(),
            )
            .unwrap(),
            MidiChannel::new(channel).unwrap(),
            ChannelParameters::default(),
        )
    }

    fn parameters(patch_id: PatchId) -> ParameterSnapshot {
        ParameterSnapshot::new(
            0,
            GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5).unwrap(),
            &[RtPatchParameters::new(
                patch_id,
                ChannelParameters::default(),
            )],
        )
        .unwrap()
    }

    #[test]
    fn hidef_soundfont_engine_accepts_only_the_fixed_soundfont_path() {
        let mut engine = HiDefSoundFontEngine::default();
        assert_eq!(
            engine.load(Path::new("./sf2/Other.sf2")),
            Err(SoundFontError::SoundFontFileUnavailable)
        );
        assert_eq!(HIDEF_SOUNDFONT_PATH, "./sf2/HiDef.sf2");
    }

    #[test]
    fn hidef_soundfont_engine_requires_loading_before_configuration() {
        let mut engine = HiDefSoundFontEngine::default();
        let patch = patch(1, 0, 0, 1, false);
        assert_eq!(
            engine.configure_patch(&patch),
            Err(SoundFontError::EngineNotLoaded)
        );
    }

    #[test]
    fn hidef_soundfont_engine_uses_independent_melodic_and_percussion_channels() {
        let melodic = PreparedPatch::try_from_patch(&patch(1, 3, 128, 42, false)).unwrap();
        let percussion = PreparedPatch::try_from_patch(&patch(2, 4, 128, 42, true)).unwrap();
        assert_eq!(melodic.effective_bank, 0);
        assert_eq!(melodic.internal_channel, MELODIC_CHANNEL);
        assert_eq!(percussion.effective_bank, 128);
        assert_eq!(percussion.bank_select_value, 0);
        assert_eq!(
            percussion.internal_channel,
            Synthesizer::PERCUSSION_CHANNEL as i32
        );
        assert_ne!(melodic, percussion);
    }

    #[test]
    fn hidef_soundfont_engine_rejects_other_and_malformed_capability_configs_without_fallback() {
        let unsupported = Patch::new(
            PatchId::new(7).unwrap(),
            "Unsupported".to_owned(),
            InstrumentConfig::from_parts(
                CapabilityId::new("instrument.other").unwrap(),
                Vec::new(),
                Vec::new(),
            ),
            MidiChannel::new(0).unwrap(),
            ChannelParameters::default(),
        );
        assert_eq!(
            PreparedPatch::try_from_patch(&unsupported),
            Err(SoundFontError::UnsupportedCapability {
                patch_id: unsupported.id()
            })
        );

        let malformed = Patch::new(
            PatchId::new(8).unwrap(),
            "Malformed".to_owned(),
            InstrumentConfig::from_parts(
                CapabilityId::new(HIDEF_CAPABILITY_ID).unwrap(),
                Vec::new(),
                Vec::new(),
            ),
            MidiChannel::new(1).unwrap(),
            ChannelParameters::default(),
        );
        assert_eq!(
            PreparedPatch::try_from_patch(&malformed),
            Err(SoundFontError::InvalidInstrumentConfig {
                patch_id: malformed.id()
            })
        );
    }

    #[test]
    fn hidef_soundfont_engine_maps_every_normalized_midi_kind() {
        let channel = MidiChannel::new(3).unwrap();
        let cases = [
            (MidiMessageKind::NoteOff, 0x80, 60, 0),
            (MidiMessageKind::NoteOn, 0x90, 60, 100),
            (MidiMessageKind::ControlChange, 0xB0, 7, 110),
            (MidiMessageKind::ProgramChange, 0xC0, 12, 0),
            (MidiMessageKind::ChannelPressure, 0xD0, 64, 0),
            (MidiMessageKind::PitchBend, 0xE0, 0, 64),
            (MidiMessageKind::AllNotesOff, 0xB0, 0x7B, 0),
        ];
        for (kind, command, data1, data2) in cases {
            let message = MidiMessage::try_new(channel, kind, data1 as u8, data2 as u8).unwrap();
            assert_eq!(midi_data(message), (command, data1, data2));
        }
    }

    #[test]
    fn hidef_soundfont_engine_clears_unloaded_patch_stems() {
        let patch_id = PatchId::new(1).unwrap();
        let snapshot = parameters(patch_id);
        let mut engine = HiDefSoundFontEngine::new(48_000, 2);
        let mut output = PatchAudioBlock::prepare(3).unwrap();
        output.begin_render(&snapshot, 3).unwrap();
        output.stem_mut(0, patch_id).unwrap().fill(0.5);
        engine.render_patches(&mut output, &snapshot);
        assert_eq!(output.stem(0, patch_id).unwrap().samples(), [0.0; 6]);
    }

    #[test]
    fn hidef_soundfont_engine_bounds_non_finite_and_hot_samples() {
        assert_eq!(bounded_sample(f32::NAN), 0.0);
        assert_eq!(bounded_sample(f32::INFINITY), 0.0);
        assert_eq!(bounded_sample(-2.0), -1.0);
        assert_eq!(bounded_sample(2.0), 1.0);
        assert_eq!(bounded_sample(0.25), 0.25);
    }
}
