use crate::adapter::hidef_soundfont_capability::{
    HIDEF_CAPABILITY_ID, HIDEF_POLYPHONY_CEILING, HIDEF_SOUNDFONT_PATH,
    SOUNDFONT_BANK_PARAMETER_ID, SOUNDFONT_FILE_PARAMETER_ID, SOUNDFONT_PERCUSSION_PARAMETER_ID,
    SOUNDFONT_PROGRAM_PARAMETER_ID,
};
use crate::adapter::soundfont_voice_engine::{PreparedSoundFontBank, SoundFontVoiceEngine};
use crate::kernel::midi_message::MidiMessage;
use crate::kernel::patch_id::PatchId;
use crate::synth::capability_id::CapabilityId;
use crate::synth::instrument_capability::{AssetKind, ParameterValue};
use crate::synth::instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
use crate::synth::parameter_id::ParameterId;
use crate::synth::patch::Patch;
use crate::synth::prepared_instrument::{PreparedInstrument, PreparedInstrumentError};
use rustysynth::SoundFont;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

const PERCUSSION_BANK_FLAG: i32 = 128;
const MELODIC_CHANNEL: i32 = 0;
const PERCUSSION_CHANNEL: i32 = 9;
const SOUNDFONT_ENGINE_VOICE_SLOTS: usize = HIDEF_POLYPHONY_CEILING as usize;

/// Control/worker-side HiDef SoundFont preparer.
///
/// Construction opens and parses the fixed bank exactly once. All prepared
/// Patch instruments share that immutable bank while owning independent voices,
/// MIDI state, and bounded render scratch.
pub struct HiDefSoundFontPreparer {
    capability_id: CapabilityId,
    prepared_bank: Arc<PreparedSoundFontBank>,
    parsed_bank_count: usize,
}

impl HiDefSoundFontPreparer {
    /// Opens and parses the one production SoundFont asset.
    pub fn new() -> Result<Self, InstrumentPreparationError> {
        Self::from_path(Path::new(HIDEF_SOUNDFONT_PATH))
    }

    fn from_path(path: &Path) -> Result<Self, InstrumentPreparationError> {
        if path != Path::new(HIDEF_SOUNDFONT_PATH) {
            return Err(InstrumentPreparationError::AssetLoadFailed);
        }
        let mut file = File::open(path).map_err(|_| InstrumentPreparationError::AssetLoadFailed)?;
        let sound_font = Arc::new(
            SoundFont::new(&mut file).map_err(|_| InstrumentPreparationError::AssetParseFailed)?,
        );
        let prepared_bank = Arc::new(
            PreparedSoundFontBank::new(Arc::clone(&sound_font))
                .map_err(|_| InstrumentPreparationError::AssetParseFailed)?,
        );
        let capability_id = CapabilityId::new(HIDEF_CAPABILITY_ID)
            .map_err(|_| InstrumentPreparationError::AssetParseFailed)?;
        Ok(Self {
            capability_id,
            prepared_bank,
            parsed_bank_count: 1,
        })
    }

    /// Returns the number of banks parsed by this preparer instance.
    pub const fn parsed_bank_count(&self) -> usize {
        self.parsed_bank_count
    }

    fn prepare_patch(
        &self,
        patch: &Patch,
        sample_rate: f32,
        max_frames: usize,
    ) -> Result<HiDefPreparedInstrument, InstrumentPreparationError> {
        let sample_rate = validated_sample_rate(sample_rate)?;
        if max_frames == 0 {
            return Err(InstrumentPreparationError::InvalidFrameCapacity);
        }

        let prepared = PreparedPatch::try_from_patch(patch)?;
        if !self
            .prepared_bank
            .has_preset(prepared.effective_bank, prepared.program as u8)
        {
            return Err(InstrumentPreparationError::PresetUnavailable {
                patch_id: patch.id(),
            });
        }
        let engine = SoundFontVoiceEngine::<SOUNDFONT_ENGINE_VOICE_SLOTS>::new(
            Arc::clone(&self.prepared_bank),
            sample_rate as f32,
            max_frames,
            prepared.effective_bank,
            prepared.program as u8,
        )
        .map_err(|_| InstrumentPreparationError::VoiceCapacityExceeded {
            patch_id: patch.id(),
        })?;

        Ok(HiDefPreparedInstrument {
            prepared,
            engine,
            max_frames,
        })
    }
}

impl InstrumentPreparer for HiDefSoundFontPreparer {
    fn capability_id(&self) -> &CapabilityId {
        &self.capability_id
    }

    fn prepared_shared_asset_count(&self) -> usize {
        self.parsed_bank_count
    }

    fn prepare(
        &self,
        patch: &Patch,
        sample_rate: f32,
        max_frames: usize,
    ) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
        self.prepare_patch(patch, sample_rate, max_frames)
            .map(|prepared| Box::new(prepared) as Box<dyn PreparedInstrument>)
    }
}

struct HiDefPreparedInstrument {
    prepared: PreparedPatch,
    engine: SoundFontVoiceEngine<SOUNDFONT_ENGINE_VOICE_SLOTS>,
    max_frames: usize,
}

impl PreparedInstrument for HiDefPreparedInstrument {
    fn patch_id(&self) -> PatchId {
        self.prepared.patch_id
    }

    fn dispatch(
        &mut self,
        message: MidiMessage,
        parameters: &crate::real_time::RtPatchParameters,
    ) -> Result<(), PreparedInstrumentError> {
        if parameters.patch_id() != Some(self.prepared.patch_id) {
            return Err(PreparedInstrumentError::DispatchRejected);
        }
        self.engine.dispatch(message, *parameters.envelope())
    }

    fn render(
        &mut self,
        interleaved_stereo: &mut [f32],
        frame_count: usize,
        parameters: &crate::real_time::RtPatchParameters,
    ) {
        let frame_count = frame_count
            .min(self.max_frames)
            .min(interleaved_stereo.len() / 2);
        if frame_count == 0 {
            return;
        }

        let output = &mut interleaved_stereo[..frame_count * 2];
        output.fill(0.0);
        if parameters.patch_id() == Some(self.prepared.patch_id) {
            self.engine.render(output, frame_count);
        }
    }

    fn all_notes_off(&mut self) {
        self.engine.all_notes_off();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreparedPatch {
    patch_id: PatchId,
    internal_channel: i32,
    effective_bank: i32,
    bank_select_value: i32,
    program: i32,
}

impl PreparedPatch {
    fn try_from_patch(patch: &Patch) -> Result<Self, InstrumentPreparationError> {
        let config = patch.instrument_config();
        if config.capability_id().as_str() != HIDEF_CAPABILITY_ID {
            return Err(InstrumentPreparationError::UnsupportedCapability {
                patch_id: patch.id(),
            });
        }
        let parameter_id = |value: &str| {
            ParameterId::new(value).map_err(|_| InstrumentPreparationError::InvalidConfiguration {
                patch_id: patch.id(),
            })
        };
        let bank = match config.value(&parameter_id(SOUNDFONT_BANK_PARAMETER_ID)?) {
            Some(ParameterValue::Stepped(value)) => u16::try_from(*value).ok(),
            _ => None,
        };
        let program = match config.value(&parameter_id(SOUNDFONT_PROGRAM_PARAMETER_ID)?) {
            Some(ParameterValue::Stepped(value)) => u8::try_from(*value).ok(),
            _ => None,
        };
        let percussion = match config.value(&parameter_id(SOUNDFONT_PERCUSSION_PARAMETER_ID)?) {
            Some(ParameterValue::Toggle(value)) => Some(*value),
            _ => None,
        };
        let file = config.asset_reference(&parameter_id(SOUNDFONT_FILE_PARAMETER_ID)?);
        let (Some(bank), Some(program), Some(percussion), Some(file)) =
            (bank, program, percussion, file)
        else {
            return Err(InstrumentPreparationError::InvalidConfiguration {
                patch_id: patch.id(),
            });
        };
        if program > 127
            || file.kind() != AssetKind::SoundFont
            || file.locator() != HIDEF_SOUNDFONT_PATH
            || config.values().len() != 3
            || config.asset_references().len() != 1
        {
            return Err(InstrumentPreparationError::InvalidConfiguration {
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
            PERCUSSION_CHANNEL
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
            internal_channel,
            effective_bank,
            bank_select_value,
            program: i32::from(program),
        })
    }
}

fn validated_sample_rate(sample_rate: f32) -> Result<i32, InstrumentPreparationError> {
    if !sample_rate.is_finite()
        || sample_rate.fract() != 0.0
        || !(16_000.0..=192_000.0).contains(&sample_rate)
    {
        return Err(InstrumentPreparationError::InvalidSampleRate);
    }
    Ok(sample_rate as i32)
}

#[cfg(test)]
mod tests {
    use super::{
        HiDefSoundFontPreparer, PreparedPatch, HIDEF_SOUNDFONT_PATH, MELODIC_CHANNEL,
        PERCUSSION_CHANNEL,
    };
    use crate::adapter::hidef_soundfont_capability::{
        HiDefSoundFontCapability, HIDEF_CAPABILITY_ID,
    };
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::real_time::parameter_snapshot::RtInstrumentParameters;
    use crate::synth::capability_id::CapabilityId;
    use crate::synth::instrument_capability::{
        AssetAssignment, AssetKind, AssetReference, InstrumentConfig,
    };
    use crate::synth::instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
    use crate::synth::parameter_id::ParameterId;
    use crate::synth::patch::Patch;
    use crate::synth::prepared_instrument::PreparedInstrument;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::voice_envelope::VoiceEnvelope;
    use crate::testing::automatic_midi_test::create_soundfont_config;
    use std::path::Path;
    use std::sync::Arc;

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

    #[test]
    fn one_parse_supplies_independent_melodic_and_percussion_instruments() {
        let preparer = HiDefSoundFontPreparer::new().unwrap();
        let melodic_patch = patch(1, 3, 0, 0, false);
        let percussion_patch = patch(2, 4, 0, 0, true);
        let shared_count = Arc::strong_count(&preparer.prepared_bank);
        let mut melodic = preparer
            .prepare_patch(&melodic_patch, 48_000.0, 512)
            .unwrap();
        let percussion = preparer
            .prepare_patch(&percussion_patch, 48_000.0, 512)
            .unwrap();

        assert_eq!(preparer.parsed_bank_count(), 1);
        assert_eq!(Arc::strong_count(&preparer.prepared_bank), shared_count + 2);
        assert_ne!(melodic.prepared, percussion.prepared);
        assert_eq!(melodic.prepared.internal_channel, MELODIC_CHANNEL);
        assert_eq!(percussion.prepared.internal_channel, PERCUSSION_CHANNEL);
        let parameters = crate::real_time::RtPatchParameters::new(
            melodic_patch.id(),
            *melodic_patch.parameters(),
        );

        melodic
            .dispatch(
                MidiMessage::try_new(melodic_patch.channel(), MidiMessageKind::NoteOn, 60, 110)
                    .unwrap(),
                &parameters,
            )
            .unwrap();
        let mut output = [0.0; 1_024];
        melodic.render(&mut output, 512, &parameters);

        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().any(|sample| sample.abs() > 0.000_001));
        assert_eq!(percussion.engine.active_note_voice_count(), 0);
    }

    #[test]
    fn preparer_rejects_invalid_assets_configs_presets_and_audio_limits() {
        assert!(matches!(
            HiDefSoundFontPreparer::from_path(Path::new("./sf2/Other.sf2")),
            Err(InstrumentPreparationError::AssetLoadFailed)
        ));

        let preparer = HiDefSoundFontPreparer::new().unwrap();
        let valid = patch(1, 0, 0, 0, false);
        assert!(matches!(
            preparer.prepare(&valid, f32::NAN, 512),
            Err(InstrumentPreparationError::InvalidSampleRate)
        ));
        assert!(matches!(
            preparer.prepare(&valid, 48_000.0, 0),
            Err(InstrumentPreparationError::InvalidFrameCapacity)
        ));

        let unsupported = Patch::new(
            PatchId::new(2).unwrap(),
            "Unsupported".to_owned(),
            InstrumentConfig::from_parts(
                CapabilityId::new("instrument.test.other").unwrap(),
                Vec::new(),
                Vec::new(),
            ),
            MidiChannel::new(0).unwrap(),
            ChannelParameters::default(),
        );
        assert!(matches!(
            preparer.prepare(&unsupported, 48_000.0, 512),
            Err(InstrumentPreparationError::UnsupportedCapability { .. })
        ));

        let malformed = Patch::new(
            PatchId::new(3).unwrap(),
            "Malformed".to_owned(),
            InstrumentConfig::from_parts(
                CapabilityId::new(HIDEF_CAPABILITY_ID).unwrap(),
                Vec::new(),
                Vec::new(),
            ),
            MidiChannel::new(1).unwrap(),
            ChannelParameters::default(),
        );
        assert!(matches!(
            preparer.prepare(&malformed, 48_000.0, 512),
            Err(InstrumentPreparationError::InvalidConfiguration { .. })
        ));

        let provider = HiDefSoundFontCapability::new().unwrap();
        let mut wrong_asset =
            create_soundfont_config(&provider, SoundFontInstrument::new(0, 0, false).unwrap())
                .unwrap();
        let file_id = ParameterId::new("soundfont.file").unwrap();
        wrong_asset = InstrumentConfig::from_parts(
            wrong_asset.capability_id().clone(),
            wrong_asset.values().to_vec(),
            vec![AssetAssignment::new(
                file_id,
                AssetReference::new(AssetKind::SoundFont, "./sf2/Other.sf2").unwrap(),
            )],
        );
        let wrong_asset = Patch::new(
            PatchId::new(4).unwrap(),
            "Wrong asset".to_owned(),
            wrong_asset,
            MidiChannel::new(2).unwrap(),
            ChannelParameters::default(),
        );
        assert!(matches!(
            preparer.prepare(&wrong_asset, 48_000.0, 512),
            Err(InstrumentPreparationError::InvalidConfiguration { .. })
        ));

        let unavailable_preset = patch(5, 3, u16::MAX, 127, false);
        assert!(matches!(
            preparer.prepare(&unavailable_preset, 48_000.0, 512),
            Err(InstrumentPreparationError::PresetUnavailable { .. })
        ));
    }

    #[test]
    fn prepared_mapping_is_patch_scoped_and_uses_the_fixed_asset() {
        let melodic = PreparedPatch::try_from_patch(&patch(7, 12, 128, 42, false)).unwrap();
        let percussion = PreparedPatch::try_from_patch(&patch(8, 13, 128, 42, true)).unwrap();

        assert_eq!(HIDEF_SOUNDFONT_PATH, "./sf2/HiDef.sf2");
        assert_eq!(melodic.patch_id, PatchId::new(7).unwrap());
        assert_eq!(melodic.effective_bank, 0);
        assert_eq!(percussion.effective_bank, 128);
        assert_ne!(melodic.internal_channel, percussion.internal_channel);
    }

    fn projected_parameters(
        patch: &Patch,
        envelope: VoiceEnvelope,
    ) -> crate::real_time::RtPatchParameters {
        crate::real_time::RtPatchParameters::projected(
            patch.id(),
            *patch.parameters(),
            envelope,
            RtInstrumentParameters::EMPTY,
        )
    }

    fn note(channel: MidiChannel, kind: MidiMessageKind, key: u8, velocity: u8) -> MidiMessage {
        MidiMessage::try_new(channel, kind, key, velocity).unwrap()
    }

    fn energy(output: &[f32]) -> f32 {
        output.iter().map(|sample| sample.abs()).sum()
    }

    #[test]
    fn common_adsr_is_owned_by_each_overlapping_soundfont_note_voice() {
        let preparer = HiDefSoundFontPreparer::new().unwrap();
        let patch = patch(1, 1, 0, 0, false);
        let envelope = VoiceEnvelope::new(0.0, 0.0, 1.0, 100.0).unwrap();
        let parameters = projected_parameters(&patch, envelope);
        let mut prepared = preparer.prepare_patch(&patch, 48_000.0, 256).unwrap();

        prepared
            .dispatch(
                note(patch.channel(), MidiMessageKind::NoteOn, 60, 110),
                &parameters,
            )
            .unwrap();
        prepared
            .dispatch(
                note(patch.channel(), MidiMessageKind::NoteOn, 64, 110),
                &parameters,
            )
            .unwrap();
        let first = prepared.engine.note_voice_counts(60);
        let second = prepared.engine.note_voice_counts(64);
        assert!(first.0 > 0 && second.0 > 0);

        prepared
            .dispatch(
                note(patch.channel(), MidiMessageKind::NoteOff, 60, 0),
                &parameters,
            )
            .unwrap();
        let first = prepared.engine.note_voice_counts(60);
        let second = prepared.engine.note_voice_counts(64);
        assert_eq!(first.1, first.0);
        assert_eq!(second.1, 0);

        let mut output = [0.0; 512];
        prepared.render(&mut output, 256, &parameters);
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(energy(&output) > 0.0);
        prepared.all_notes_off();
        assert_eq!(prepared.engine.active_note_voice_count(), 0);
    }

    #[test]
    fn every_common_adsr_field_changes_soundfont_audio_before_the_patch_stem() {
        let preparer = HiDefSoundFontPreparer::new().unwrap();
        let patch = patch(1, 1, 0, 0, false);

        let render_onset = |envelope: VoiceEnvelope| {
            let parameters = projected_parameters(&patch, envelope);
            let mut prepared = preparer.prepare_patch(&patch, 48_000.0, 256).unwrap();
            prepared
                .dispatch(
                    note(patch.channel(), MidiMessageKind::NoteOn, 60, 127),
                    &parameters,
                )
                .unwrap();
            let mut output = [0.0; 512];
            prepared.render(&mut output, 256, &parameters);
            energy(&output)
        };

        let immediate = render_onset(VoiceEnvelope::DEFAULT);
        let attacked = render_onset(VoiceEnvelope::new(100.0, 0.0, 1.0, 0.0).unwrap());
        let decayed = render_onset(VoiceEnvelope::new(0.0, 100.0, 0.0, 0.0).unwrap());
        let sustained = render_onset(VoiceEnvelope::new(0.0, 0.0, 0.25, 0.0).unwrap());
        assert!(immediate > attacked);
        assert!(immediate > decayed);
        assert!(immediate > sustained);

        let release_energy = |release_milliseconds: f32| {
            let envelope = VoiceEnvelope::new(0.0, 0.0, 1.0, release_milliseconds).unwrap();
            let parameters = projected_parameters(&patch, envelope);
            let mut prepared = preparer.prepare_patch(&patch, 48_000.0, 256).unwrap();
            prepared
                .dispatch(
                    note(patch.channel(), MidiMessageKind::NoteOn, 60, 127),
                    &parameters,
                )
                .unwrap();
            let mut onset = [0.0; 512];
            prepared.render(&mut onset, 256, &parameters);
            prepared
                .dispatch(
                    note(patch.channel(), MidiMessageKind::NoteOff, 60, 0),
                    &parameters,
                )
                .unwrap();
            let mut released = [0.0; 512];
            prepared.render(&mut released, 256, &parameters);
            energy(&released)
        };
        assert_eq!(release_energy(0.0), 0.0);
        assert!(release_energy(100.0) > 0.0);
    }
}
