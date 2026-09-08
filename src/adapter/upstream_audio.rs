//! Shared ownership and capability adapters for the vendored upstream DSP catalog.
//! All metadata, instances, voices, and scratch are prepared off callback.
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::PatchId;
use crate::real_time::{RtPatchParameters, RtPostEffectParameters};
use crate::synth::voice_envelope_state::VoiceEnvelopeState;
use crate::synth::*;
use std::ffi::{c_char, c_void, CStr};
use std::ptr::NonNull;
use std::sync::OnceLock;

extern "C" {
    fn crest_audio_count() -> usize;
    fn crest_audio_id(index: usize) -> *const c_char;
    fn crest_audio_name(index: usize) -> *const c_char;
    fn crest_audio_is_instrument(index: usize) -> bool;
    fn crest_audio_create(index: usize, rate: f32, frames: usize) -> *mut c_void;
    fn crest_audio_load_sample(
        handle: *mut c_void,
        bytes: *const u8,
        size: usize,
        frames: usize,
    ) -> bool;
    fn crest_audio_load_ir(
        handle: *mut c_void,
        pcm: *const f32,
        frames: usize,
        channels: usize,
        rate: f32,
    ) -> bool;
    fn crest_audio_load(handle: *mut c_void, bytes: *const u8, size: usize) -> bool;
    fn crest_audio_latency(handle: *mut c_void) -> usize;
    fn crest_audio_destroy(handle: *mut c_void);
    fn crest_audio_param_count(handle: *mut c_void) -> usize;
    fn crest_audio_param_label(handle: *mut c_void, index: usize) -> *const c_char;
    fn crest_audio_param_default(handle: *mut c_void, index: usize) -> f32;
    fn crest_audio_param_min(handle: *mut c_void, index: usize) -> f32;
    fn crest_audio_param_max(handle: *mut c_void, index: usize) -> f32;
    fn crest_audio_param_stepped(handle: *mut c_void, index: usize) -> bool;
    fn crest_audio_set(handle: *mut c_void, values: *const f32, count: usize) -> bool;
    fn crest_audio_note(handle: *mut c_void, status: i32, data1: i32, data2: i32);
    fn crest_audio_reset(handle: *mut c_void);
    fn crest_audio_process(handle: *mut c_void, stereo: *mut f32, frames: usize) -> bool;
}

pub(crate) trait VoiceProcessor: Send {
    fn latency(&self) -> usize;
    fn set(&mut self, values: &[f32]) -> bool;
    fn note(&mut self, status: i32, a: u8, b: u8);
    fn reset(&mut self);
    fn process(&mut self, stereo: &mut [f32], frames: usize) -> bool;
}
struct Processor {
    handle: NonNull<c_void>,
    max_frames: usize,
}
// Exclusive instance ownership moves to audio after preparation. No upstream
// object is shared concurrently between the preparation worker and callback.
unsafe impl Send for Processor {}
impl Processor {
    fn new(index: usize, rate: f32, max_frames: usize) -> Option<Self> {
        NonNull::new(unsafe { crest_audio_create(index, rate, max_frames) })
            .map(|handle| Self { handle, max_frames })
    }
    fn load_ir(&mut self, decoded: &DecodedSample) -> bool {
        unsafe {
            crest_audio_load_ir(
                self.handle.as_ptr(),
                decoded.interleaved().as_ptr(),
                decoded.metadata().frames() as usize,
                usize::from(decoded.metadata().channels()),
                decoded.metadata().sample_rate() as f32,
            )
        }
    }
    fn load(&mut self, bytes: &[u8]) -> bool {
        unsafe { crest_audio_load(self.handle.as_ptr(), bytes.as_ptr(), bytes.len()) }
    }
    fn latency(&self) -> usize {
        unsafe { crest_audio_latency(self.handle.as_ptr()) }
    }
    fn set(&mut self, values: &[f32]) -> bool {
        unsafe { crest_audio_set(self.handle.as_ptr(), values.as_ptr(), values.len()) }
    }
    fn note(&mut self, status: i32, a: u8, b: u8) {
        unsafe { crest_audio_note(self.handle.as_ptr(), status, i32::from(a), i32::from(b)) }
    }
    fn reset(&mut self) {
        unsafe { crest_audio_reset(self.handle.as_ptr()) }
    }
    fn process(&mut self, stereo: &mut [f32], frames: usize) -> bool {
        frames <= self.max_frames
            && stereo.len() == frames.saturating_mul(2)
            && unsafe { crest_audio_process(self.handle.as_ptr(), stereo.as_mut_ptr(), frames) }
    }
}
impl VoiceProcessor for Processor {
    fn latency(&self) -> usize {
        self.latency()
    }
    fn set(&mut self, values: &[f32]) -> bool {
        self.set(values)
    }
    fn note(&mut self, status: i32, a: u8, b: u8) {
        self.note(status, a, b)
    }
    fn reset(&mut self) {
        self.reset()
    }
    fn process(&mut self, stereo: &mut [f32], frames: usize) -> bool {
        self.process(stereo, frames)
    }
}
pub(crate) fn prepare_voice_bank<P: VoiceProcessor + 'static>(
    patch: &Patch,
    rate: f32,
    max_frames: usize,
    mut create: impl FnMut() -> Result<P, InstrumentPreparationError>,
) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
    let mut voices = Vec::new();
    voices
        .try_reserve_exact(usize::from(patch.voice_limit().value()))
        .map_err(|_| InstrumentPreparationError::StorageAllocationFailed {
            patch_id: patch.id(),
        })?;
    for _ in 0..patch.voice_limit().value() {
        voices.push(Voice {
            processor: create()?,
            envelope: VoiceEnvelopeState::IDLE,
            note: None,
            age: 0,
            held: false,
            delay: 0,
        });
    }
    Ok(Box::new(PreparedVoiceBank {
        patch_id: patch.id(),
        voices,
        scratch: vec![0.; max_frames * 2],
        rate,
        age: 0,
        sustain: false,
        bend: (0, 64),
        expression: 1.,
        pressure: 1.,
    }))
}
impl Drop for Processor {
    fn drop(&mut self) {
        unsafe { crest_audio_destroy(self.handle.as_ptr()) }
    }
}

#[derive(Clone)]
pub struct UpstreamInstrument {
    index: usize,
    descriptor: CapabilityDescriptor,
}
#[derive(Clone)]
pub struct UpstreamEffect {
    index: usize,
    descriptor: EffectCapabilityDescriptor,
}
struct Catalog {
    instruments: Vec<UpstreamInstrument>,
    effects: Vec<UpstreamEffect>,
}
#[derive(Clone, Debug, thiserror::Error)]
#[error("upstream audio catalog: {0}")]
pub struct CatalogError(String);

fn catalog() -> Result<&'static Catalog, CatalogError> {
    static CATALOG: OnceLock<Result<Catalog, CatalogError>> = OnceLock::new();
    CATALOG
        .get_or_init(build_catalog)
        .as_ref()
        .map_err(Clone::clone)
}
pub fn instrument_ports() -> Result<Vec<UpstreamInstrument>, CatalogError> {
    Ok(catalog()?.instruments.clone())
}
pub fn effect_ports() -> Result<Vec<UpstreamEffect>, CatalogError> {
    Ok(catalog()?.effects.clone())
}

fn text(pointer: *const c_char) -> Result<String, CatalogError> {
    if pointer.is_null() {
        return Err(CatalogError("missing native metadata".into()));
    }
    unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .map(str::to_owned)
        .map_err(|e| CatalogError(e.to_string()))
}
fn build_catalog() -> Result<Catalog, CatalogError> {
    let mut catalog = Catalog {
        instruments: Vec::new(),
        effects: Vec::new(),
    };
    for index in 0..unsafe { crest_audio_count() } {
        let id = text(unsafe { crest_audio_id(index) })?
            .to_lowercase()
            .replace('_', ".");
        if id == "sfizz.sample" {
            continue;
        }
        let label = text(unsafe { crest_audio_name(index) })?;
        let native = Processor::new(index, 48_000.0, 256)
            .ok_or_else(|| CatalogError(format!("could not prepare {id}")))?;
        let mut params = Vec::new();
        for i in 0..unsafe { crest_audio_param_count(native.handle.as_ptr()) } {
            let parameter = (|| -> Result<ParameterSpec, CapabilityError> {
                let label = text(unsafe { crest_audio_param_label(native.handle.as_ptr(), i) })
                    .map_err(|_| CapabilityError::EmptyLabel)?;
                let default =
                    unsafe { crest_audio_param_default(native.handle.as_ptr(), i) } as f64;
                let min = unsafe { crest_audio_param_min(native.handle.as_ptr(), i) } as f64;
                let max = unsafe { crest_audio_param_max(native.handle.as_ptr(), i) } as f64;
                let stepped = unsafe { crest_audio_param_stepped(native.handle.as_ptr(), i) };
                ParameterSpec::new_with_patch_interaction(
                    ParameterId::new(format!("{id}.parameter-{i}"))
                        .map_err(|_| CapabilityError::InvalidMetadataIdentifier(id.clone()))?,
                    label,
                    if stepped {
                        ParameterKind::Stepped
                    } else {
                        ParameterKind::Continuous
                    },
                    ParameterUpdate::Scalar,
                    PatchInteraction::ScalarEdit,
                    ParameterDefault::Value(if stepped {
                        ParameterValue::Stepped(default as i64)
                    } else {
                        ParameterValue::continuous(default)?
                    }),
                    Some(ParameterRange::new(min, max)?),
                    Vec::new(),
                    Some(if stepped { 1.0 } else { (max - min) / 100.0 }),
                    Some(if stepped { 1.0 } else { (max - min) / 10.0 }),
                    None,
                    if stepped { "integer" } else { "number" },
                    None,
                    None,
                )
            })()
            .map_err(|e| CatalogError(format!("{id} parameter {i}: {e}")))?;
            params.push(parameter);
        }
        let mut assets = Vec::new();
        if id == "nam.model" || id == "fft.convolver" {
            let (name, kind, reference) = if id == "nam.model" {
                (
                    super::model_assets::NAM_FILE,
                    AssetKind::NeuralModel,
                    super::model_assets::NAM_DEFAULT,
                )
            } else {
                (
                    super::model_assets::IR_FILE,
                    AssetKind::ImpulseResponse,
                    super::model_assets::IR_DEFAULT,
                )
            };
            params.insert(
                0,
                super::model_assets::spec(name, kind, reference)
                    .map_err(|e| CatalogError(e.to_string()))?,
            );
            assets.push(AssetRequirement::new(ParameterId::new(name).unwrap(), true));
        }
        if id == "sfizz.sampler" {
            let file = ParameterId::new(super::sfz_library::FILE).unwrap();
            params.push(
                ParameterSpec::new_with_patch_interaction(
                    file.clone(),
                    "SFZ Library",
                    ParameterKind::Asset,
                    ParameterUpdate::Structural,
                    PatchInteraction::ReadOnly,
                    ParameterDefault::Asset(
                        AssetReference::new(AssetKind::Sfz, super::sfz_library::DEFAULT)
                            .map_err(|e| CatalogError(e.to_string()))?,
                    ),
                    None,
                    Vec::new(),
                    None,
                    None,
                    None,
                    "asset",
                    None,
                    None,
                )
                .map_err(|e| CatalogError(e.to_string()))?,
            );
            assets.push(AssetRequirement::new(file, true));
        }
        let sections = if params.is_empty() {
            Vec::new()
        } else {
            vec![CapabilitySection::new("controls", "Parameters", params)
                .map_err(|e| CatalogError(e.to_string()))?]
        };
        if unsafe { crest_audio_is_instrument(index) } {
            let id = CapabilityId::new(format!("instrument.{id}"))
                .map_err(|e| CatalogError(e.to_string()))?;
            let descriptor = CapabilityDescriptor::new(
                id.clone(),
                label,
                id.as_str(),
                sections,
                assets,
                VoicePolicy::Configurable { default_voices: 16 },
                vec![
                    MidiMessageKind::NoteOn,
                    MidiMessageKind::NoteOff,
                    MidiMessageKind::ControlChange,
                    MidiMessageKind::PitchBend,
                    MidiMessageKind::ChannelPressure,
                    MidiMessageKind::AllNotesOff,
                ],
            )
            .map_err(|e| CatalogError(e.to_string()))?;
            let descriptor = if id.as_str() == super::dx7_library::CAPABILITY {
                super::dx7_library::Dx7Library::bundled()
                    .descriptor(
                        AssetReference::new(AssetKind::SysEx, super::dx7_library::BUNDLED)
                            .map_err(|e| CatalogError(e.to_string()))?,
                    )
                    .map_err(|e| CatalogError(e.to_string()))?
            } else {
                descriptor
            };
            catalog
                .instruments
                .push(UpstreamInstrument { index, descriptor });
        } else {
            let id = EffectCapabilityId::new(format!("effect.{id}"))
                .map_err(|e| CatalogError(e.to_string()))?;
            let descriptor =
                EffectCapabilityDescriptor::new(id.clone(), label, id.as_str(), sections, assets)
                    .map_err(|e| CatalogError(e.to_string()))?;
            catalog.effects.push(UpstreamEffect { index, descriptor });
        }
    }
    Ok(catalog)
}
impl InstrumentCapabilityProvider for UpstreamInstrument {
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }
    fn create_config(
        &self,
        values: &[ParameterAssignment],
        assets: &[AssetAssignment],
    ) -> Result<InstrumentConfig, CapabilityError> {
        self.descriptor.create_config(values, assets)
    }
}
impl EffectCapabilityProvider for UpstreamEffect {
    fn descriptor(&self) -> EffectCapabilityDescriptor {
        self.descriptor.clone()
    }
    fn create_config(
        &self,
        slot: EffectSlotId,
        values: &[ParameterAssignment],
        assets: &[AssetAssignment],
    ) -> Result<PostEffectConfig, EffectCapabilityError> {
        self.descriptor.create_config(slot, values, assets)
    }
}
impl EffectPreparer for UpstreamEffect {
    fn capability_id(&self) -> &EffectCapabilityId {
        self.descriptor.id()
    }
    fn prepare(
        &self,
        patch_id: PatchId,
        config: &PostEffectConfig,
        rate: f32,
        max_frames: usize,
    ) -> Result<Box<dyn PreparedPostEffect>, EffectPreparationError> {
        if !rate.is_finite() || rate < 8000.0 {
            return Err(EffectPreparationError::InvalidSampleRate);
        }
        if max_frames == 0 {
            return Err(EffectPreparationError::InvalidFrameCapacity);
        }
        if config.capability_id() != self.descriptor.id() {
            return Err(EffectPreparationError::UnsupportedCapability { patch_id });
        }
        let canonical = self
            .create_config(config.slot_id(), config.values(), config.asset_references())
            .map_err(|_| EffectPreparationError::InvalidConfiguration { patch_id })?;
        if &canonical != config {
            return Err(EffectPreparationError::InvalidConfiguration { patch_id });
        }
        let mut processor = Processor::new(self.index, rate, max_frames)
            .ok_or(EffectPreparationError::PreparationFailed { patch_id })?;
        for assignment in config.asset_references() {
            let reference = assignment.reference();
            if reference.locator() == super::model_assets::NAM_DEFAULT
                || reference.locator() == super::model_assets::IR_DEFAULT
            {
                continue;
            }
            let bytes = super::model_assets::read(reference)
                .map_err(|_| EffectPreparationError::PreparationFailed { patch_id })?;
            load_model(&mut processor, reference.kind(), &bytes)
                .map_err(|_| EffectPreparationError::PreparationFailed { patch_id })?;
        }
        Ok(Box::new(PreparedEffect {
            patch_id,
            slot_id: config.slot_id(),
            processor,
        }))
    }
}
struct PreparedEffect {
    patch_id: PatchId,
    slot_id: EffectSlotId,
    processor: Processor,
}
impl PreparedPostEffect for PreparedEffect {
    fn patch_id(&self) -> PatchId {
        self.patch_id
    }
    fn slot_id(&self) -> EffectSlotId {
        self.slot_id
    }
    fn process(
        &mut self,
        stereo: &mut [f32],
        frames: usize,
        params: &RtPostEffectParameters,
    ) -> Result<(), PreparedEffectError> {
        if params.slot_id() != Some(self.slot_id) {
            return Err(PreparedEffectError::SlotMismatch);
        }
        if !self.processor.set(params.scalars()) {
            return Err(PreparedEffectError::ScalarLayoutMismatch);
        }
        if !self.processor.process(stereo, frames) {
            return Err(PreparedEffectError::ProcessRejected);
        }
        Ok(())
    }
}
impl InstrumentPreparer for UpstreamInstrument {
    fn asset_descriptor(
        &self,
        config: &InstrumentConfig,
    ) -> Result<Option<CapabilityDescriptor>, InstrumentPreparationError> {
        if self.descriptor.id().as_str() != super::dx7_library::CAPABILITY {
            return Ok(None);
        }
        let reference = config
            .asset_reference(&ParameterId::new(super::dx7_library::FILE).unwrap())
            .ok_or(InstrumentPreparationError::AssetParseFailed)?;
        let library = super::dx7_library::load(reference)
            .map_err(|_| InstrumentPreparationError::AssetParseFailed)?;
        Ok(Some(library.descriptor(reference.clone()).map_err(
            |_| InstrumentPreparationError::AssetParseFailed,
        )?))
    }
    fn capability_id(&self) -> &CapabilityId {
        self.descriptor.id()
    }
    fn prepare(
        &self,
        patch: &Patch,
        rate: f32,
        max_frames: usize,
    ) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
        let patch_id = patch.id();
        if !rate.is_finite() || rate < 8000.0 {
            return Err(InstrumentPreparationError::InvalidSampleRate);
        }
        if max_frames == 0 {
            return Err(InstrumentPreparationError::InvalidFrameCapacity);
        }
        if patch.instrument_config().capability_id() != self.descriptor.id() {
            return Err(InstrumentPreparationError::UnsupportedCapability { patch_id });
        }
        let library = if self.descriptor.id().as_str() == super::dx7_library::CAPABILITY {
            let reference = patch
                .instrument_config()
                .asset_reference(&ParameterId::new(super::dx7_library::FILE).unwrap())
                .ok_or(InstrumentPreparationError::InvalidConfiguration { patch_id })?;
            Some((
                super::dx7_library::load(reference)
                    .map_err(|_| InstrumentPreparationError::AssetParseFailed)?,
                reference,
            ))
        } else {
            None
        };
        let descriptor = library
            .as_ref()
            .map(|(library, reference)| library.descriptor((*reference).clone()))
            .transpose()
            .map_err(|_| InstrumentPreparationError::InvalidConfiguration { patch_id })?;
        let canonical = descriptor
            .as_ref()
            .unwrap_or(&self.descriptor)
            .create_config(
                patch.instrument_config().values(),
                patch.instrument_config().asset_references(),
            )
            .map_err(|_| InstrumentPreparationError::InvalidConfiguration { patch_id })?;
        let patch_data = if let Some((library, _)) = &library {
            let Some(ParameterValue::Choice(id)) =
                canonical.value(&ParameterId::new(super::dx7_library::PRESET).unwrap())
            else {
                return Err(InstrumentPreparationError::InvalidConfiguration { patch_id });
            };
            Some(
                library
                    .presets
                    .iter()
                    .find(|p| &p.id == id)
                    .ok_or(InstrumentPreparationError::InvalidConfiguration { patch_id })?
                    .data,
            )
        } else {
            None
        };
        if &canonical != patch.instrument_config() {
            return Err(InstrumentPreparationError::InvalidConfiguration { patch_id });
        }
        let sfz = if self.descriptor.id().as_str() == super::sfz_library::CAPABILITY {
            let reference = canonical
                .asset_reference(&ParameterId::new(super::sfz_library::FILE).unwrap())
                .ok_or(InstrumentPreparationError::AssetParseFailed)?;
            Some(
                super::sfz_library::load(reference)
                    .map_err(|_| InstrumentPreparationError::AssetParseFailed)?,
            )
        } else {
            None
        };
        let mut voices = Vec::new();
        let capacity = usize::from(patch.voice_limit().value());
        voices
            .try_reserve_exact(capacity)
            .map_err(|_| InstrumentPreparationError::StorageAllocationFailed { patch_id })?;
        for _ in 0..capacity {
            let mut processor = Processor::new(self.index, rate, max_frames)
                .ok_or(InstrumentPreparationError::PreparationFailed { patch_id })?;
            if let Some(data) = &patch_data {
                if !processor.load(data) {
                    return Err(InstrumentPreparationError::AssetParseFailed);
                }
            }
            if let Some(bytes) = &sfz {
                if !processor.load(bytes) {
                    return Err(InstrumentPreparationError::AssetParseFailed);
                }
            }
            voices.push(Voice {
                processor,
                envelope: VoiceEnvelopeState::IDLE,
                note: None,
                age: 0,
                held: false,
                delay: 0,
            });
        }
        Ok(Box::new(PreparedVoiceBank {
            patch_id,
            voices,
            scratch: vec![0.0; max_frames * 2],
            rate,
            age: 0,
            sustain: false,
            bend: (0, 64),
            expression: 1.0,
            pressure: 1.0,
        }))
    }
}
struct Voice<P> {
    processor: P,
    envelope: VoiceEnvelopeState,
    note: Option<u8>,
    age: u64,
    held: bool,
    delay: usize,
}
struct PreparedVoiceBank<P> {
    patch_id: PatchId,
    voices: Vec<Voice<P>>,
    scratch: Vec<f32>,
    rate: f32,
    age: u64,
    sustain: bool,
    bend: (u8, u8),
    expression: f32,
    pressure: f32,
}
impl<P: VoiceProcessor> PreparedInstrument for PreparedVoiceBank<P> {
    fn patch_id(&self) -> PatchId {
        self.patch_id
    }
    fn dispatch(
        &mut self,
        message: MidiMessage,
        params: &RtPatchParameters,
    ) -> Result<(), PreparedInstrumentError> {
        if params.patch_id() != Some(self.patch_id) {
            return Err(PreparedInstrumentError::DispatchRejected);
        }
        let a = message.data1();
        let b = message.data2();
        match message.kind() {
            MidiMessageKind::NoteOn if b > 0 => {
                let voice = self
                    .voices
                    .iter_mut()
                    .find(|v| v.envelope.is_idle())
                    .ok_or(PreparedInstrumentError::DispatchRejected)?;
                voice.processor.reset();
                if !voice.processor.set(params.instrument().values()) {
                    return Err(PreparedInstrumentError::DispatchRejected);
                }
                voice.processor.note(0xe0, self.bend.0, self.bend.1);
                voice.processor.note(0x90, a, b);
                voice.delay = voice.processor.latency();
                voice.envelope.note_on(*params.envelope(), self.rate);
                self.age = self.age.wrapping_add(1);
                voice.age = self.age;
                voice.note = Some(a);
                voice.held = true;
            }
            MidiMessageKind::NoteOn | MidiMessageKind::NoteOff => {
                for voice in self
                    .voices
                    .iter_mut()
                    .filter(|v| v.note == Some(a) && v.held)
                {
                    voice.held = false;
                    if !self.sustain {
                        voice
                            .envelope
                            .note_off(params.envelope().release_milliseconds(), self.rate);
                        voice.processor.note(0x80, a, 0);
                    }
                }
            }
            MidiMessageKind::PitchBend => {
                self.bend = (a, b);
                for voice in &mut self.voices {
                    voice.processor.note(0xe0, a, b);
                }
            }
            MidiMessageKind::ChannelPressure => {
                self.pressure = f32::from(a) / 127.0;
            }
            MidiMessageKind::ControlChange => match a {
                64 => {
                    self.sustain = b >= 64;
                    if !self.sustain {
                        for voice in self.voices.iter_mut().filter(|v| !v.held) {
                            voice
                                .envelope
                                .note_off(params.envelope().release_milliseconds(), self.rate);
                            if let Some(note) = voice.note {
                                voice.processor.note(0x80, note, 0);
                            }
                        }
                    }
                }
                7 | 11 => {
                    self.expression = f32::from(b) / 127.0;
                }
                120 | 123 => self.all_notes_off(),
                // Native controls belong to capability parameters. Bank and
                // channel-mode CCs must not bypass asset or voice ownership.
                _ => {}
            },
            MidiMessageKind::AllNotesOff => self.all_notes_off(),
            MidiMessageKind::ProgramChange => {
                return Err(PreparedInstrumentError::DispatchRejected)
            }
        }
        Ok(())
    }
    fn render(
        &mut self,
        stereo: &mut [f32],
        frames: usize,
        params: &RtPatchParameters,
    ) -> Result<(), crate::synth::PreparedInstrumentError> {
        stereo.fill(0.0);
        if stereo.len() != frames.saturating_mul(2) || stereo.len() > self.scratch.len() {
            return Err(PreparedInstrumentError::InvalidFrameCapacity);
        }
        for voice in &mut self.voices {
            if voice.envelope.is_idle() {
                continue;
            }
            let scratch = &mut self.scratch[..stereo.len()];
            scratch.fill(0.0);
            if !voice.processor.set(params.instrument().values()) {
                stereo.fill(0.0);
                return Err(PreparedInstrumentError::ScalarLayoutMismatch);
            }
            if !voice.processor.process(scratch, frames) {
                stereo.fill(0.0);
                return Err(PreparedInstrumentError::RenderRejected);
            }
            for (input, output) in scratch.chunks_exact(2).zip(stereo.chunks_exact_mut(2)) {
                let gain = if voice.delay > 0 {
                    voice.delay -= 1;
                    0.0
                } else {
                    voice.envelope.next_gain(self.rate) * self.expression * self.pressure
                };
                output[0] += input[0] * gain;
                output[1] += input[1] * gain;
            }
        }

        Ok(())
    }
    fn all_notes_off(&mut self) {
        for voice in &mut self.voices {
            voice.envelope = VoiceEnvelopeState::IDLE;
            voice.note = None;
            voice.held = false;
            voice.processor.reset();
        }
        self.sustain = false;
    }
}

fn load_model(
    processor: &mut Processor,
    kind: AssetKind,
    bytes: &[u8],
) -> Result<(), SampleAssetError> {
    let loaded = match kind {
        AssetKind::NeuralModel => processor.load(bytes),
        AssetKind::ImpulseResponse => {
            let decoded = super::wav_sample_decoder::WavSampleDecoder
                .decode(&AssetFileId::new("impulse.wav")?, bytes)?;
            processor.load_ir(&decoded)
        }
        _ => false,
    };
    if loaded {
        Ok(())
    } else {
        Err(SampleAssetError::MalformedModel)
    }
}
pub(crate) fn validate_model_asset(kind: AssetKind, bytes: &[u8]) -> Result<(), SampleAssetError> {
    let id = match kind {
        AssetKind::NeuralModel => "effect.nam.model",
        AssetKind::ImpulseResponse => "effect.fft.convolver",
        _ => return Err(SampleAssetError::UnsupportedContainer),
    };
    let ports = effect_ports().map_err(|_| SampleAssetError::Unavailable)?;
    let port = ports
        .iter()
        .find(|p| p.descriptor.id().as_str() == id)
        .ok_or(SampleAssetError::Unavailable)?;
    let mut processor =
        Processor::new(port.index, 48000.0, 256).ok_or(SampleAssetError::Unavailable)?;
    load_model(&mut processor, kind, bytes)
}

pub(crate) fn validate_sfz(bytes: &[u8]) -> Result<(), SampleAssetError> {
    let ports = instrument_ports().map_err(|_| SampleAssetError::Unavailable)?;
    let port = ports
        .iter()
        .find(|p| p.descriptor.id().as_str() == super::sfz_library::CAPABILITY)
        .ok_or(SampleAssetError::Unavailable)?;
    let mut processor =
        Processor::new(port.index, 48000.0, 256).ok_or(SampleAssetError::Unavailable)?;
    if processor.load(bytes) {
        Ok(())
    } else {
        Err(SampleAssetError::MalformedModel)
    }
}

pub(crate) fn prepare_sample_bank(
    patch: &Patch,
    rate: f32,
    max_frames: usize,
    pcm: &PreparedSamplePcm,
) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
    use base64::Engine;
    let failure = InstrumentPreparationError::PreparationFailed {
        patch_id: patch.id(),
    };
    let mut bytes = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(
            &mut bytes,
            hound::WavSpec {
                channels: pcm.channels(),
                sample_rate: pcm.sample_rate(),
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .map_err(|_| failure)?;
        for value in pcm.interleaved() {
            writer.write_sample(*value).map_err(|_| failure)?;
        }
        writer.finalize().map_err(|_| failure)?;
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes.into_inner());
    let sfz=format!("<sample> name=crest.wav base64data={encoded}\n<region> sample=crest.wav pitch_keycenter=60 ampeg_attack=0 ampeg_decay=0 ampeg_sustain=100 ampeg_release=100\n");
    let index = (0..unsafe { crest_audio_count() })
        .find(|&i| text(unsafe { crest_audio_id(i) }).is_ok_and(|id| id == "sfizz_Sample"))
        .ok_or(failure)?;
    prepare_voice_bank(patch, rate, max_frames, || {
        let processor = Processor::new(index, rate, max_frames).ok_or(failure)?;
        if !unsafe {
            crest_audio_load_sample(
                processor.handle.as_ptr(),
                sfz.as_ptr(),
                sfz.len(),
                pcm.frames(),
            )
        } {
            return Err(failure);
        }
        Ok(processor)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::mixer_state::MixerState;
    use crate::mixer::patch_output::PatchOutput;
    use crate::real_time::{GraphRevision, ParameterSnapshot};

    #[test]
    fn every_upstream_effect_prepares_and_renders_through_its_public_port() {
        let ports = effect_ports().unwrap();
        assert!(ports.len() >= 21);
        for port in ports {
            let descriptor = port.descriptor();
            let config = descriptor
                .default_config(EffectSlotId::new(1).unwrap())
                .unwrap();
            let scalars = descriptor
                .scalar_parameters()
                .map(|p| p.scalar_value(config.value(p.id()).unwrap()).unwrap())
                .collect::<Vec<_>>();
            let parameters = RtPostEffectParameters::new(config.slot_id(), &scalars).unwrap();
            let mut effect = port
                .prepare(PatchId::new(1).unwrap(), &config, 48_000.0, 256)
                .unwrap();
            let mut energy = 0.0f64;
            for block in 0..128 {
                let mut audio = [0.0; 512];
                for (i, frame) in audio.chunks_exact_mut(2).enumerate() {
                    frame[0] = (((block * 256 + i) as f32) * 0.03).sin() * 0.1;
                    frame[1] = (((block * 256 + i) as f32) * 0.07).sin() * 0.1;
                }
                effect
                    .process(&mut audio, 256, &parameters)
                    .unwrap_or_else(|e| panic!("{}: {e}", descriptor.id()));
                assert!(audio.iter().all(|x| x.is_finite()), "{}", descriptor.id());
                energy += audio.iter().map(|&x| f64::from(x * x)).sum::<f64>();
            }
            assert!(energy > 1e-10, "{} produced silence", descriptor.id());
        }
    }

    #[test]
    fn elements_modulation_offset_rejects_unsupported_values_and_renders_a_full_lfo_cycle() {
        let port = instrument_ports()
            .unwrap()
            .into_iter()
            .find(|port| port.descriptor().id().as_str() == "instrument.mutable.elements")
            .unwrap();
        let descriptor = port.descriptor();
        let id = ParameterId::new("mutable.elements.parameter-16").unwrap();
        let values = descriptor
            .parameters()
            .filter_map(|spec| match spec.default_value() {
                ParameterDefault::Value(value) => {
                    Some(ParameterAssignment::new(spec.id().clone(), value.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let default = port
            .create_config(&values, &descriptor.default_assets())
            .unwrap();
        assert!(default
            .with_scalar_value(&descriptor, &id, ParameterValue::Continuous(1.0))
            .is_err());
        let range = descriptor.parameter(&id).unwrap().range().unwrap();
        for offset in [range.minimum(), range.maximum()] {
            let config = default
                .with_scalar_value(&descriptor, &id, ParameterValue::Continuous(offset))
                .unwrap();
            let mut patch = Patch::new(
                PatchId::new(1).unwrap(),
                "Elements".into(),
                config,
                MidiChannel::new(0).unwrap(),
                PatchOutput::default(),
            );
            patch.set_voice_limit(1).unwrap();
            let registry = CapabilityRegistry::new(vec![descriptor.clone()]).unwrap();
            let snapshot = ParameterSnapshot::project_patches(
                1,
                GraphRevision::INITIAL,
                GlobalParameters::new(0.0).unwrap(),
                MixerState::default(),
                &[patch.clone()],
                &registry,
            )
            .unwrap();
            let params = &snapshot.patches()[0];
            let mut engine = port.prepare(&patch, 48_000.0, 256).unwrap();
            engine
                .dispatch(
                    MidiMessage::try_new(
                        MidiChannel::new(0).unwrap(),
                        MidiMessageKind::NoteOn,
                        60,
                        100,
                    )
                    .unwrap(),
                    params,
                )
                .unwrap();
            let mut energy = 0.0f64;
            // More than the default 0.5 Hz LFO's complete two-second cycle.
            for _ in 0..512 {
                let mut output = [0.0f32; 512];
                engine.render(&mut output, 256, params).unwrap();
                assert!(output.iter().all(|value| value.is_finite()));
                energy += output
                    .iter()
                    .map(|value| f64::from(value * value))
                    .sum::<f64>();
            }
            assert!(energy > 1e-8, "Elements must sound at offset {offset}");
        }
    }

    #[test]
    fn every_upstream_instrument_plays_and_obeys_patch_release() {
        for port in instrument_ports().unwrap() {
            let descriptor = port.descriptor();
            eprintln!("rendering {}", descriptor.id());
            let values = descriptor
                .parameters()
                .filter_map(|p| match p.default_value() {
                    ParameterDefault::Value(v) => {
                        Some(ParameterAssignment::new(p.id().clone(), v.clone()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            let config = port
                .create_config(&values, &descriptor.default_assets())
                .unwrap();
            let mut patch = Patch::new(
                PatchId::new(1).unwrap(),
                "Upstream".into(),
                config,
                MidiChannel::new(0).unwrap(),
                PatchOutput::default(),
            );
            patch.set_voice_limit(2).unwrap();
            let registry = CapabilityRegistry::new(vec![descriptor.clone()]).unwrap();
            let snapshot = ParameterSnapshot::project_patches(
                1,
                GraphRevision::INITIAL,
                GlobalParameters::new(0.0).unwrap(),
                MixerState::default(),
                &[patch.clone()],
                &registry,
            )
            .unwrap();
            let params = &snapshot.patches()[0];
            let mut engine = port.prepare(&patch, 48_000.0, 256).unwrap();
            engine
                .dispatch(
                    MidiMessage::try_new(
                        MidiChannel::new(0).unwrap(),
                        MidiMessageKind::NoteOn,
                        60,
                        100,
                    )
                    .unwrap(),
                    params,
                )
                .unwrap();
            let mut energy = 0.0f64;
            let mut audio = [0.0; 512];
            for _ in 0..128 {
                engine.render(&mut audio, 256, params).unwrap();
                assert!(audio.iter().all(|x| x.is_finite()), "{}", descriptor.id());
                energy += audio.iter().map(|&x| f64::from(x * x)).sum::<f64>();
            }
            assert!(energy > 1e-10, "{} produced silence", descriptor.id());
            engine
                .dispatch(
                    MidiMessage::try_new(
                        MidiChannel::new(0).unwrap(),
                        MidiMessageKind::NoteOff,
                        60,
                        0,
                    )
                    .unwrap(),
                    params,
                )
                .unwrap();
            engine.render(&mut audio, 256, params).unwrap();
            assert!(
                audio.iter().all(|x| *x == 0.0),
                "{} ignored neutral release",
                descriptor.id()
            );
        }
    }
}
