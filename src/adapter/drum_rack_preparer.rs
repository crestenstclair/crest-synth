use super::drum_rack_capability::{
    DrumRackCapability, DRUM_RACK_CAPABILITY_ID, DRUM_RACK_FIRST_NOTE, DRUM_RACK_PADS,
};
use super::sample_preparer::SamplePreparer;
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::PatchId;
use crate::real_time::parameter_snapshot::{RtInstrumentParameters, RtPatchParameters};
use crate::synth::*;
use std::sync::Arc;

/// Composition of existing Sample engines; all assets and storage prepare off callback.
pub struct DrumRackPreparer {
    capability: DrumRackCapability,
    sample: SamplePreparer,
}
impl DrumRackPreparer {
    pub fn new(
        initial_asset: AssetFileId,
        catalog: Arc<dyn SampleAssetCatalogPort>,
        decoder: Arc<dyn SampleDecoderPort>,
    ) -> Result<Self, InstrumentPreparationError> {
        Self::with_sample(initial_asset, SamplePreparer::new(catalog, decoder)?)
    }
    pub(crate) fn with_sample(
        initial_asset: AssetFileId,
        sample: SamplePreparer,
    ) -> Result<Self, InstrumentPreparationError> {
        Ok(Self {
            capability: DrumRackCapability::new(initial_asset)
                .map_err(|_| InstrumentPreparationError::AssetParseFailed)?,
            sample,
        })
    }
}
impl InstrumentPreparer for DrumRackPreparer {
    fn capability_id(&self) -> &CapabilityId {
        self.capability.id()
    }
    fn prepared_shared_asset_count(&self) -> usize {
        self.sample.prepared_shared_asset_count()
    }
    fn prepare(
        &self,
        patch: &Patch,
        rate: f32,
        frames: usize,
    ) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
        let invalid = InstrumentPreparationError::InvalidConfiguration {
            patch_id: patch.id(),
        };
        if !rate.is_finite()
            || rate.fract() != 0.0
            || !(MIN_SAMPLE_RATE as f32..=MAX_SAMPLE_RATE as f32).contains(&rate)
        {
            return Err(InstrumentPreparationError::InvalidSampleRate);
        }
        if frames == 0 || frames.checked_mul(2).is_none() {
            return Err(InstrumentPreparationError::InvalidFrameCapacity);
        }
        let descriptor = self.capability.descriptor();
        if patch.instrument_config().capability_id().as_str() != DRUM_RACK_CAPABILITY_ID
            || descriptor
                .create_config(
                    patch.instrument_config().values(),
                    patch.instrument_config().asset_references(),
                )
                .map_err(|_| invalid)?
                != *patch.instrument_config()
        {
            return Err(invalid);
        }
        let mut pads = Vec::with_capacity(DRUM_RACK_PADS.len());
        let mut footprints = Vec::new();
        let mut visualizations = Vec::new();
        let mut bytes = 0usize;
        let mut shared = std::collections::HashSet::new();
        let sample_schema = self.capability.sample_descriptor();
        let mut scalar_offset = 1; // The descriptor's leading pad selector has no audio effect.
        for pad in 0..DRUM_RACK_PADS.len() {
            let config = self
                .capability
                .sample_config(patch.instrument_config(), pad)
                .map_err(|_| invalid)?;
            let count = sample_schema.scalar_parameter_count();
            let prepared = if let Some(config) = config {
                let sample_patch = Patch::new(
                    patch.id(),
                    patch.name().into(),
                    config,
                    patch.channel(),
                    patch.output(),
                )
                .with_envelope(*patch.envelope())
                .with_voice_limit(patch.voice_limit().value())
                .map_err(|_| invalid)?;
                let instrument = self.sample.prepare(&sample_patch, rate, frames)?;
                for footprint in instrument.prepared_asset_footprints() {
                    let unique =
                        shared.insert((footprint.reference().clone(), footprint.preparation_key()));
                    bytes = bytes
                        .checked_add(if unique { footprint.bytes() } else { 0 })
                        .and_then(|v| v.checked_add(footprint.private_bytes()))
                        .filter(|v| *v <= MAX_SAMPLE_GRAPH_PCM_BYTES)
                        .ok_or(InstrumentPreparationError::SampleAsset {
                            patch_id: patch.id(),
                            cause: SampleAssetError::GraphPcmCapacityExceeded,
                        })?;
                    footprints.push(footprint.clone());
                }
                visualizations.extend(instrument.prepared_sample_visualizations());
                let values = sample_schema
                    .scalar_parameters()
                    .map(|spec| {
                        spec.scalar_value(
                            sample_patch
                                .instrument_config()
                                .value(spec.id())
                                .ok_or(invalid)?,
                        )
                        .map_err(|_| invalid)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Some(PreparedPad {
                    instrument,
                    parameters: RtPatchParameters::projected(
                        patch.id(),
                        patch.output(),
                        *patch.envelope(),
                        RtInstrumentParameters::new(&values).map_err(|_| invalid)?,
                    ),
                    scalar_offset,
                    scalar_count: count,
                })
            } else {
                None
            };
            pads.push(prepared);
            scalar_offset += count;
        }
        Ok(Box::new(PreparedDrumRack {
            patch_id: patch.id(),
            pads,
            scratch: vec![0.0; frames * 2],
            scalar_count: scalar_offset,
            footprints,
            visualizations,
        }))
    }
    fn prepare_audition(
        &self,
        patch: PatchId,
        candidate: &InstrumentConfig,
        rate: f32,
        frames: usize,
    ) -> Result<Box<dyn PreparedAudition>, InstrumentPreparationError> {
        let invalid = InstrumentPreparationError::InvalidConfiguration { patch_id: patch };
        let pad = DrumRackCapability::selected_pad(candidate).ok_or(invalid)?;
        let config = self
            .capability
            .sample_config(candidate, pad)
            .map_err(|_| invalid)?
            .ok_or(invalid)?;
        self.sample.prepare_audition(patch, &config, rate, frames)
    }
}
struct PreparedPad {
    instrument: Box<dyn PreparedInstrument>,
    parameters: RtPatchParameters,
    scalar_offset: usize,
    scalar_count: usize,
}
impl PreparedPad {
    fn sync(&mut self, source: &RtPatchParameters) -> Result<(), PreparedInstrumentError> {
        let values = source
            .instrument()
            .values()
            .get(self.scalar_offset..self.scalar_offset + self.scalar_count)
            .ok_or(PreparedInstrumentError::ScalarLayoutMismatch)?;
        self.parameters.copy_instrument_controls(source, values)
    }
}
struct PreparedDrumRack {
    patch_id: PatchId,
    pads: Vec<Option<PreparedPad>>,
    scratch: Vec<f32>,
    scalar_count: usize,
    footprints: Vec<PreparedAssetFootprint>,
    visualizations: Vec<PreparedSampleVisualization>,
}
impl PreparedInstrument for PreparedDrumRack {
    fn patch_id(&self) -> PatchId {
        self.patch_id
    }
    fn accepts_note(&self, note: u8) -> bool {
        note.checked_sub(DRUM_RACK_FIRST_NOTE)
            .and_then(|pad| self.pads.get(usize::from(pad)))
            .is_some_and(Option::is_some)
    }
    fn dispatch(
        &mut self,
        message: MidiMessage,
        params: &RtPatchParameters,
    ) -> Result<(), PreparedInstrumentError> {
        if params.patch_id() != Some(self.patch_id) {
            return Err(PreparedInstrumentError::DispatchRejected);
        }
        if params.instrument().count() != self.scalar_count {
            return Err(PreparedInstrumentError::ScalarLayoutMismatch);
        }
        if matches!(
            message.kind(),
            MidiMessageKind::NoteOn | MidiMessageKind::NoteOff
        ) {
            let Some(pad) = message
                .data1()
                .checked_sub(DRUM_RACK_FIRST_NOTE)
                .and_then(|pad| self.pads.get_mut(usize::from(pad)))
                .and_then(Option::as_mut)
            else {
                return Ok(());
            };
            pad.sync(params)?;
            // Each pad plays at Sample's reference MIDI note 60. Root Note remains independent tuning.
            let mapped =
                MidiMessage::try_new(message.channel(), message.kind(), 60, message.data2())
                    .map_err(|_| PreparedInstrumentError::DispatchRejected)?;
            pad.instrument.dispatch(mapped, &pad.parameters)
        } else {
            for pad in self.pads.iter_mut().flatten() {
                pad.sync(params)?;
                pad.instrument.dispatch(message, &pad.parameters)?;
            }
            Ok(())
        }
    }
    fn render(
        &mut self,
        audio: &mut [f32],
        frames: usize,
        params: &RtPatchParameters,
    ) -> Result<(), PreparedInstrumentError> {
        audio.fill(0.0);
        if params.patch_id() != Some(self.patch_id)
            || params.instrument().count() != self.scalar_count
        {
            return Err(PreparedInstrumentError::ScalarLayoutMismatch);
        }
        if audio.len() != frames.saturating_mul(2) || audio.len() > self.scratch.len() {
            return Err(PreparedInstrumentError::InvalidFrameCapacity);
        }
        for pad in self.pads.iter_mut().flatten() {
            pad.sync(params)?;
            let scratch = &mut self.scratch[..audio.len()];
            pad.instrument.render(scratch, frames, &pad.parameters)?;
            for (output, input) in audio.iter_mut().zip(scratch) {
                *output += *input;
            }
        }
        Ok(())
    }
    fn all_notes_off(&mut self) {
        for pad in self.pads.iter_mut().flatten() {
            pad.instrument.all_notes_off();
        }
    }
    fn prepared_asset_footprints(&self) -> Vec<&PreparedAssetFootprint> {
        self.footprints.iter().collect()
    }
    fn prepared_sample_visualizations(&self) -> Vec<PreparedSampleVisualization> {
        self.visualizations.clone()
    }
}
