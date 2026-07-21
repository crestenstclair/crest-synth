use crate::adapter::hidef_soundfont_engine::HIDEF_SOUNDFONT_PATH;
use crate::kernel::midi_message::MidiMessageKind;
use crate::synth::capability_id::CapabilityId;
use crate::synth::instrument_capability::{
    AssetAssignment, AssetKind, AssetReference, AssetRequirement, CapabilityDescriptor,
    CapabilityError, CapabilityRegistry, CapabilitySection, InstrumentConfig, ParameterAssignment,
    ParameterDefault, ParameterKind, ParameterRange, ParameterSpec, ParameterUpdate,
    ParameterValue,
};
use crate::synth::instrument_capability_provider::InstrumentCapabilityProvider;
use crate::synth::parameter_id::ParameterId;

pub const HIDEF_CAPABILITY_ID: &str = "instrument.soundfont.hidef";
pub const SOUNDFONT_BANK_PARAMETER_ID: &str = "soundfont.bank";
pub const SOUNDFONT_PROGRAM_PARAMETER_ID: &str = "soundfont.program";
pub const SOUNDFONT_PERCUSSION_PARAMETER_ID: &str = "soundfont.percussion";
pub const SOUNDFONT_FILE_PARAMETER_ID: &str = "soundfont.file";
pub const HIDEF_VOICE_LIMIT: u16 = 64;

/// Control-side descriptor/config provider for the installed HiDef SoundFont.
#[derive(Clone, Debug)]
pub struct HiDefSoundFontCapability {
    descriptor: CapabilityDescriptor,
}

impl HiDefSoundFontCapability {
    pub fn new() -> Result<Self, CapabilityError> {
        let bank = numeric_parameter(
            SOUNDFONT_BANK_PARAMETER_ID,
            "Bank",
            0,
            0,
            u16::MAX.into(),
            1.0,
            128.0,
        )?;
        let program = numeric_parameter(
            SOUNDFONT_PROGRAM_PARAMETER_ID,
            "Program",
            0,
            0,
            127,
            1.0,
            8.0,
        )?;
        let percussion = ParameterSpec::new(
            parameter_id(SOUNDFONT_PERCUSSION_PARAMETER_ID)?,
            "Percussion",
            ParameterKind::Toggle,
            ParameterUpdate::Structural,
            ParameterDefault::Value(ParameterValue::Toggle(false)),
            None,
            Vec::new(),
            None,
            None,
            None,
            "toggle",
            None,
            None,
        )?;
        let file_id = parameter_id(SOUNDFONT_FILE_PARAMETER_ID)?;
        let file = ParameterSpec::new(
            file_id.clone(),
            "SoundFont File",
            ParameterKind::Asset,
            ParameterUpdate::Structural,
            ParameterDefault::Asset(AssetReference::new(
                AssetKind::SoundFont,
                HIDEF_SOUNDFONT_PATH,
            )?),
            None,
            Vec::new(),
            None,
            None,
            None,
            "asset",
            None,
            None,
        )?;
        let descriptor = CapabilityDescriptor::new(
            CapabilityId::new(HIDEF_CAPABILITY_ID).map_err(|_| {
                CapabilityError::InvalidMetadataIdentifier(HIDEF_CAPABILITY_ID.to_owned())
            })?,
            "HiDef SoundFont",
            "instrument.soundfont",
            vec![CapabilitySection::new(
                "soundfont",
                "SoundFont",
                vec![bank, program, percussion, file],
            )?],
            vec![AssetRequirement::new(file_id, true)],
            HIDEF_VOICE_LIMIT,
            MidiMessageKind::ALL.to_vec(),
        )?;
        Ok(Self { descriptor })
    }

    /// Builds the exact immutable registry installed by current composition roots.
    pub fn registry(&self) -> Result<CapabilityRegistry, CapabilityError> {
        CapabilityRegistry::new(vec![self.descriptor()])
    }
}

impl InstrumentCapabilityProvider for HiDefSoundFontCapability {
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }

    fn create_config(
        &self,
        values: &[ParameterAssignment],
        asset_references: &[AssetAssignment],
    ) -> Result<InstrumentConfig, CapabilityError> {
        self.descriptor.create_config(values, asset_references)
    }
}

fn numeric_parameter(
    id: &str,
    label: &str,
    default: i64,
    minimum: i64,
    maximum: i64,
    fine_step: f64,
    coarse_step: f64,
) -> Result<ParameterSpec, CapabilityError> {
    ParameterSpec::new(
        parameter_id(id)?,
        label,
        ParameterKind::Stepped,
        ParameterUpdate::Structural,
        ParameterDefault::Value(ParameterValue::Stepped(default)),
        Some(ParameterRange::new(minimum as f64, maximum as f64)?),
        Vec::new(),
        Some(fine_step),
        Some(coarse_step),
        None,
        "integer",
        None,
        None,
    )
}

fn parameter_id(value: &str) -> Result<ParameterId, CapabilityError> {
    ParameterId::new(value)
        .map_err(|_| CapabilityError::InvalidMetadataIdentifier(value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(
        provider: &HiDefSoundFontCapability,
        bank: i64,
        program: i64,
        percussion: bool,
    ) -> Result<InstrumentConfig, CapabilityError> {
        provider.create_config(
            &[
                ParameterAssignment::new(
                    parameter_id(SOUNDFONT_BANK_PARAMETER_ID).unwrap(),
                    ParameterValue::Stepped(bank),
                ),
                ParameterAssignment::new(
                    parameter_id(SOUNDFONT_PROGRAM_PARAMETER_ID).unwrap(),
                    ParameterValue::Stepped(program),
                ),
                ParameterAssignment::new(
                    parameter_id(SOUNDFONT_PERCUSSION_PARAMETER_ID).unwrap(),
                    ParameterValue::Toggle(percussion),
                ),
            ],
            &[AssetAssignment::new(
                parameter_id(SOUNDFONT_FILE_PARAMETER_ID).unwrap(),
                AssetReference::new(AssetKind::SoundFont, HIDEF_SOUNDFONT_PATH).unwrap(),
            )],
        )
    }

    #[test]
    fn hidef_soundfont_capability_declares_the_exact_ordered_schema() {
        let provider = HiDefSoundFontCapability::new().unwrap();
        let descriptor = provider.descriptor();

        assert_eq!(descriptor.id().as_str(), HIDEF_CAPABILITY_ID);
        assert_eq!(descriptor.label(), "HiDef SoundFont");
        assert_eq!(descriptor.semantic_accent(), "instrument.soundfont");
        assert_eq!(descriptor.voice_limit(), HIDEF_VOICE_LIMIT);
        assert_eq!(descriptor.supported_midi_kinds(), MidiMessageKind::ALL);
        assert_eq!(descriptor.sections().len(), 1);
        let parameters = descriptor.sections()[0].parameters();
        assert_eq!(
            parameters
                .iter()
                .map(|parameter| parameter.id().as_str())
                .collect::<Vec<_>>(),
            [
                SOUNDFONT_BANK_PARAMETER_ID,
                SOUNDFONT_PROGRAM_PARAMETER_ID,
                SOUNDFONT_PERCUSSION_PARAMETER_ID,
                SOUNDFONT_FILE_PARAMETER_ID,
            ]
        );
        assert!(parameters
            .iter()
            .all(|parameter| parameter.update() == ParameterUpdate::Structural));
        assert_eq!(parameters[0].kind(), ParameterKind::Stepped);
        assert_eq!(parameters[1].range().unwrap().maximum(), 127.0);
        assert_eq!(parameters[2].kind(), ParameterKind::Toggle);
        assert_eq!(parameters[3].kind(), ParameterKind::Asset);
        assert_eq!(descriptor.asset_requirements().len(), 1);
        assert!(descriptor.asset_requirements()[0].required());
    }

    #[test]
    fn hidef_soundfont_capability_creates_exact_configs_without_fallback() {
        let provider = HiDefSoundFontCapability::new().unwrap();
        let instrument_config = config(&provider, 128, 42, true).unwrap();
        provider
            .registry()
            .unwrap()
            .validate_config(&instrument_config)
            .unwrap();

        assert_eq!(
            instrument_config.capability_id().as_str(),
            HIDEF_CAPABILITY_ID
        );
        assert_eq!(
            instrument_config.value(&parameter_id(SOUNDFONT_BANK_PARAMETER_ID).unwrap()),
            Some(&ParameterValue::Stepped(128))
        );
        assert_eq!(
            instrument_config.value(&parameter_id(SOUNDFONT_PROGRAM_PARAMETER_ID).unwrap()),
            Some(&ParameterValue::Stepped(42))
        );
        assert_eq!(
            instrument_config.value(&parameter_id(SOUNDFONT_PERCUSSION_PARAMETER_ID).unwrap()),
            Some(&ParameterValue::Toggle(true))
        );
        assert_eq!(
            instrument_config
                .asset_reference(&parameter_id(SOUNDFONT_FILE_PARAMETER_ID).unwrap())
                .unwrap()
                .locator(),
            HIDEF_SOUNDFONT_PATH
        );

        assert!(matches!(
            config(&provider, 0, 128, false),
            Err(CapabilityError::ValueOutOfRange(_))
        ));
        assert!(matches!(
            provider.create_config(
                &[
                    ParameterAssignment::new(
                        parameter_id(SOUNDFONT_BANK_PARAMETER_ID).unwrap(),
                        ParameterValue::Stepped(0),
                    ),
                    ParameterAssignment::new(
                        parameter_id(SOUNDFONT_PROGRAM_PARAMETER_ID).unwrap(),
                        ParameterValue::Stepped(0),
                    ),
                    ParameterAssignment::new(
                        parameter_id(SOUNDFONT_PERCUSSION_PARAMETER_ID).unwrap(),
                        ParameterValue::Toggle(false),
                    ),
                ],
                &[AssetAssignment::new(
                    parameter_id(SOUNDFONT_FILE_PARAMETER_ID).unwrap(),
                    AssetReference::new(AssetKind::SoundFont, "./sf2/Other.sf2").unwrap(),
                )],
            ),
            Err(CapabilityError::AssetDoesNotMatch(_))
        ));
    }
}
