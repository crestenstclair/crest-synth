use crate::adapter::braids_capability::{
    BraidsCapability, BRAIDS_CAPABILITY_ID, BRAIDS_COLOR_PARAMETER_ID, BRAIDS_MODELS,
    BRAIDS_MODEL_PARAMETER_ID, BRAIDS_TIMBRE_PARAMETER_ID,
};
use crate::adapter::braids_native::{
    BraidsNativeError, BraidsVoiceBank, BRAIDS_INTERNAL_CHUNK_FRAMES, BRAIDS_MODEL_COUNT,
    BRAIDS_VOICE_COUNT,
};
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::patch_id::PatchId;
use crate::real_time::parameter_snapshot::RtPatchParameters;
use crate::synth::capability_id::CapabilityId;
use crate::synth::instrument_capability::ParameterValue;
use crate::synth::instrument_capability_provider::InstrumentCapabilityProvider;
use crate::synth::instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
use crate::synth::parameter_id::ParameterId;
use crate::synth::patch::Patch;
use crate::synth::prepared_instrument::{PreparedInstrument, PreparedInstrumentError};
use crate::synth::voice_envelope_state::VoiceEnvelopeState;

pub const BRAIDS_HOST_SAMPLE_RATE: f32 = 48_000.0;
pub const BRAIDS_INTERNAL_SAMPLE_RATE: f32 = 96_000.0;
const HOST_FRAMES_PER_NATIVE_CHUNK: usize = BRAIDS_INTERNAL_CHUNK_FRAMES / 2;
const NORMALIZED_I16: f32 = 1.0 / 32_768.0;
const PITCH_UNITS_PER_SEMITONE: i32 = 128;
const PITCH_BEND_RANGE_SEMITONES: i32 = 2;
const MIDI_BEND_CENTER: i32 = 8_192;

/// Control/worker-side preparer for the pinned Braids capability.
pub struct BraidsPreparer {
    capability_id: CapabilityId,
}

impl BraidsPreparer {
    pub fn new() -> Result<Self, InstrumentPreparationError> {
        let capability_id = CapabilityId::new(BRAIDS_CAPABILITY_ID)
            .map_err(|_| InstrumentPreparationError::AssetParseFailed)?;
        Ok(Self { capability_id })
    }

    fn prepare_patch(
        &self,
        patch: &Patch,
        sample_rate: f32,
        max_frames: usize,
    ) -> Result<PreparedBraidsInstrument, InstrumentPreparationError> {
        if sample_rate != BRAIDS_HOST_SAMPLE_RATE {
            return Err(InstrumentPreparationError::InvalidSampleRate);
        }
        if max_frames == 0 {
            return Err(InstrumentPreparationError::InvalidFrameCapacity);
        }
        PreparedBraidsConfig::try_from_patch(patch)?;
        let bank = BraidsVoiceBank::new().map_err(|_| {
            InstrumentPreparationError::VoiceCapacityExceeded {
                patch_id: patch.id(),
            }
        })?;
        Ok(PreparedBraidsInstrument {
            patch_id: patch.id(),
            bank,
            voices: [BraidsVoice::IDLE; BRAIDS_VOICE_COUNT],
            expression: 1.0,
            pressure: 1.0,
            pitch_bend: 0,
            next_age: 1,
            max_frames,
            native_scratch: [0; BRAIDS_INTERNAL_CHUNK_FRAMES],
            maximum_native_chunk_observed: 0,
        })
    }
}

impl InstrumentPreparer for BraidsPreparer {
    fn capability_id(&self) -> &CapabilityId {
        &self.capability_id
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct BraidsScalarParameters {
    model: u8,
    timbre: i16,
    color: i16,
}

impl BraidsScalarParameters {
    fn from_rt(parameters: &RtPatchParameters) -> Option<Self> {
        let values = parameters.instrument().values();
        if values.len() != 3 {
            return None;
        }
        let model = values[0];
        if model.fract() != 0.0 || !(0.0..f32::from(BRAIDS_MODEL_COUNT)).contains(&model) {
            return None;
        }
        let timbre = normalized_parameter(values[1])?;
        let color = normalized_parameter(values[2])?;
        Some(Self {
            model: model as u8,
            timbre,
            color,
        })
    }
}

fn normalized_parameter(value: f32) -> Option<i16> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Some((value * f32::from(i16::MAX)).round() as i16)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BraidsVoice {
    note: Option<u8>,
    velocity: f32,
    age: u64,
    envelope: VoiceEnvelopeState,
}

impl BraidsVoice {
    const IDLE: Self = Self {
        note: None,
        velocity: 0.0,
        age: 0,
        envelope: VoiceEnvelopeState::IDLE,
    };

    fn clear(&mut self) {
        *self = Self::IDLE;
    }
}

struct PreparedBraidsInstrument {
    patch_id: PatchId,
    bank: BraidsVoiceBank,
    voices: [BraidsVoice; BRAIDS_VOICE_COUNT],
    expression: f32,
    pressure: f32,
    pitch_bend: i16,
    next_age: u64,
    max_frames: usize,
    native_scratch: [i16; BRAIDS_INTERNAL_CHUNK_FRAMES],
    maximum_native_chunk_observed: usize,
}

impl PreparedBraidsInstrument {
    fn note_on(
        &mut self,
        note: u8,
        velocity: u8,
        parameters: &RtPatchParameters,
        scalars: BraidsScalarParameters,
    ) -> Result<(), PreparedInstrumentError> {
        let voice_index = self
            .voices
            .iter()
            .position(|voice| voice.envelope.is_idle())
            .or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .min_by_key(|(index, voice)| (voice.age, *index))
                    .map(|(index, _)| index)
            })
            .ok_or(PreparedInstrumentError::DispatchRejected)?;

        self.bank
            .reset(voice_index)
            .and_then(|()| {
                self.bank.configure(
                    voice_index,
                    scalars.model,
                    note_pitch(note, self.pitch_bend),
                    scalars.timbre,
                    scalars.color,
                )
            })
            .and_then(|()| self.bank.strike(voice_index))
            .map_err(|_| PreparedInstrumentError::DispatchRejected)?;

        let voice = &mut self.voices[voice_index];
        voice.note = Some(note);
        voice.velocity = f32::from(velocity) / 127.0;
        voice.age = self.next_age;
        voice
            .envelope
            .note_on(*parameters.envelope(), BRAIDS_HOST_SAMPLE_RATE);
        self.next_age = self.next_age.saturating_add(1);
        Ok(())
    }

    fn note_off(&mut self, note: u8, parameters: &RtPatchParameters) {
        for voice in &mut self.voices {
            if voice.note == Some(note) && !voice.envelope.is_idle() {
                voice.envelope.note_off(
                    parameters.envelope().release_milliseconds(),
                    BRAIDS_HOST_SAMPLE_RATE,
                );
            }
        }
    }

    fn render_voice_chunk(
        &mut self,
        voice_index: usize,
        output: &mut [f32],
        host_offset: usize,
        host_frames: usize,
        scalars: BraidsScalarParameters,
    ) -> Result<(), BraidsNativeError> {
        let note = match self.voices[voice_index].note {
            Some(note) => note,
            None => return Ok(()),
        };
        let native_frames = host_frames * 2;
        self.maximum_native_chunk_observed = self.maximum_native_chunk_observed.max(native_frames);
        self.bank.configure(
            voice_index,
            scalars.model,
            note_pitch(note, self.pitch_bend),
            scalars.timbre,
            scalars.color,
        )?;
        self.bank
            .render(voice_index, &mut self.native_scratch[..native_frames])?;

        let voice = &mut self.voices[voice_index];
        let expression = voice.velocity * self.expression * self.pressure;
        for host_frame in 0..host_frames {
            let native_frame = host_frame * 2;
            let decimated = (f32::from(self.native_scratch[native_frame])
                + f32::from(self.native_scratch[native_frame + 1]))
                * (0.5 * NORMALIZED_I16);
            let gain = voice.envelope.next_gain(BRAIDS_HOST_SAMPLE_RATE) * expression;
            let sample = bounded_sample(decimated * gain);
            let output_frame = (host_offset + host_frame) * 2;
            output[output_frame] += sample;
            output[output_frame + 1] += sample;
        }
        if voice.envelope.is_idle() {
            voice.clear();
            self.bank.reset(voice_index)?;
        }
        Ok(())
    }

    fn clear_all_voices(&mut self) {
        for (index, voice) in self.voices.iter_mut().enumerate() {
            voice.clear();
            let _ = self.bank.reset(index);
        }
    }

    #[cfg(test)]
    fn active_voice_count(&self) -> usize {
        self.voices
            .iter()
            .filter(|voice| !voice.envelope.is_idle())
            .count()
    }
}

impl PreparedInstrument for PreparedBraidsInstrument {
    fn patch_id(&self) -> PatchId {
        self.patch_id
    }

    fn dispatch(
        &mut self,
        message: MidiMessage,
        parameters: &RtPatchParameters,
    ) -> Result<(), PreparedInstrumentError> {
        if parameters.patch_id() != Some(self.patch_id) {
            return Err(PreparedInstrumentError::DispatchRejected);
        }
        let scalars = BraidsScalarParameters::from_rt(parameters)
            .ok_or(PreparedInstrumentError::DispatchRejected)?;
        match message.kind() {
            MidiMessageKind::NoteOn if message.data2() > 0 => {
                self.note_on(message.data1(), message.data2(), parameters, scalars)
            }
            MidiMessageKind::NoteOn | MidiMessageKind::NoteOff => {
                self.note_off(message.data1(), parameters);
                Ok(())
            }
            MidiMessageKind::ControlChange => {
                if matches!(message.data1(), 7 | 11) {
                    self.expression = f32::from(message.data2()) / 127.0;
                }
                Ok(())
            }
            MidiMessageKind::ChannelPressure => {
                self.pressure = f32::from(message.data1()) / 127.0;
                Ok(())
            }
            MidiMessageKind::PitchBend => {
                self.pitch_bend = pitch_bend(message.data1(), message.data2());
                Ok(())
            }
            MidiMessageKind::AllNotesOff => {
                self.clear_all_voices();
                Ok(())
            }
            MidiMessageKind::ProgramChange => Err(PreparedInstrumentError::UnsupportedMidiKind {
                kind: MidiMessageKind::ProgramChange,
            }),
        }
    }

    fn render(
        &mut self,
        interleaved_stereo: &mut [f32],
        frame_count: usize,
        parameters: &RtPatchParameters,
    ) {
        let frame_count = frame_count
            .min(self.max_frames)
            .min(interleaved_stereo.len() / 2);
        interleaved_stereo[..frame_count * 2].fill(0.0);
        let Some(scalars) = BraidsScalarParameters::from_rt(parameters) else {
            return;
        };
        if parameters.patch_id() != Some(self.patch_id) {
            return;
        }

        let mut host_offset = 0;
        while host_offset < frame_count {
            let host_frames = (frame_count - host_offset).min(HOST_FRAMES_PER_NATIVE_CHUNK);
            for voice_index in 0..BRAIDS_VOICE_COUNT {
                if self
                    .render_voice_chunk(
                        voice_index,
                        interleaved_stereo,
                        host_offset,
                        host_frames,
                        scalars,
                    )
                    .is_err()
                {
                    self.voices[voice_index].clear();
                }
            }
            host_offset += host_frames;
        }
    }

    fn all_notes_off(&mut self) {
        self.clear_all_voices();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PreparedBraidsConfig;

impl PreparedBraidsConfig {
    fn try_from_patch(patch: &Patch) -> Result<Self, InstrumentPreparationError> {
        let config = patch.instrument_config();
        if config.capability_id().as_str() != BRAIDS_CAPABILITY_ID
            || !config.asset_references().is_empty()
            || config.values().len() != 3
        {
            return Err(invalid_config(patch.id()));
        }
        let provider = BraidsCapability::new().map_err(|_| invalid_config(patch.id()))?;
        let canonical = provider
            .create_config(config.values(), config.asset_references())
            .map_err(|_| invalid_config(patch.id()))?;
        if canonical != *config {
            return Err(invalid_config(patch.id()));
        }

        let model_id = parameter_id(BRAIDS_MODEL_PARAMETER_ID, patch.id())?;
        let timbre_id = parameter_id(BRAIDS_TIMBRE_PARAMETER_ID, patch.id())?;
        let color_id = parameter_id(BRAIDS_COLOR_PARAMETER_ID, patch.id())?;
        let model_valid = matches!(
            config.value(&model_id),
            Some(ParameterValue::Choice(model)) if BRAIDS_MODELS.iter().any(|candidate| candidate.id == model)
        );
        let normalized = |id: &ParameterId| {
            matches!(
                config.value(id),
                Some(ParameterValue::Continuous(value)) if value.is_finite() && (0.0..=1.0).contains(value)
            )
        };
        if !model_valid || !normalized(&timbre_id) || !normalized(&color_id) {
            return Err(invalid_config(patch.id()));
        }
        Ok(Self)
    }
}

fn parameter_id(value: &str, patch_id: PatchId) -> Result<ParameterId, InstrumentPreparationError> {
    ParameterId::new(value).map_err(|_| invalid_config(patch_id))
}

const fn invalid_config(patch_id: PatchId) -> InstrumentPreparationError {
    InstrumentPreparationError::InvalidConfiguration { patch_id }
}

fn pitch_bend(lsb: u8, msb: u8) -> i16 {
    let value = i32::from(lsb) | (i32::from(msb) << 7);
    let units = (value - MIDI_BEND_CENTER)
        * (PITCH_UNITS_PER_SEMITONE * PITCH_BEND_RANGE_SEMITONES)
        / MIDI_BEND_CENTER;
    units.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

fn note_pitch(note: u8, bend: i16) -> i16 {
    (i32::from(note) * PITCH_UNITS_PER_SEMITONE + i32::from(bend)).clamp(0, i32::from(i16::MAX))
        as i16
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
    use super::*;
    use crate::adapter::braids_capability::BraidsCapability;
    use crate::adapter::braids_native::{braids_lifecycle_counts, BRAIDS_INTERNAL_CHUNK_FRAMES};
    use crate::kernel::midi_channel::MidiChannel;
    use crate::mixer::patch_output::PatchOutput;
    use crate::real_time::parameter_snapshot::RtInstrumentParameters;
    use crate::synth::voice_envelope::VoiceEnvelope;

    fn patch(id: u32) -> Patch {
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Braids {id}"),
            BraidsCapability::new().unwrap().default_config().unwrap(),
            MidiChannel::new((id % 16) as u8).unwrap(),
            PatchOutput::default(),
        )
    }

    fn parameters(
        patch_id: PatchId,
        envelope: VoiceEnvelope,
        scalars: [f32; 3],
    ) -> RtPatchParameters {
        RtPatchParameters::projected(
            patch_id,
            PatchOutput::default(),
            envelope,
            RtInstrumentParameters::new(&scalars).unwrap(),
        )
    }

    fn message(kind: MidiMessageKind, data1: u8, data2: u8) -> MidiMessage {
        MidiMessage::try_new(MidiChannel::new(1).unwrap(), kind, data1, data2).unwrap()
    }

    #[test]
    fn braids_preparer_requires_exact_rate_config_and_frame_capacity() {
        let preparer = BraidsPreparer::new().unwrap();
        let patch = patch(1);
        assert!(matches!(
            preparer.prepare(&patch, 44_100.0, 64),
            Err(InstrumentPreparationError::InvalidSampleRate)
        ));
        assert!(matches!(
            preparer.prepare(&patch, BRAIDS_HOST_SAMPLE_RATE, 0),
            Err(InstrumentPreparationError::InvalidFrameCapacity)
        ));

        let wrong = Patch::new(
            PatchId::new(2).unwrap(),
            "Wrong".to_owned(),
            crate::synth::instrument_capability::InstrumentConfig::from_parts(
                CapabilityId::new(BRAIDS_CAPABILITY_ID).unwrap(),
                patch.instrument_config().values()[..2].to_vec(),
                Vec::new(),
            ),
            MidiChannel::new(2).unwrap(),
            PatchOutput::default(),
        );
        assert!(matches!(
            preparer.prepare(&wrong, BRAIDS_HOST_SAMPLE_RATE, 64),
            Err(InstrumentPreparationError::InvalidConfiguration { .. })
        ));
    }

    #[test]
    fn braids_preparer_owns_one_bank_and_renders_finite_nonzero_audio_in_24_sample_chunks() {
        let before = braids_lifecycle_counts();
        let patch = patch(1);
        let mut prepared = BraidsPreparer::new()
            .unwrap()
            .prepare_patch(&patch, BRAIDS_HOST_SAMPLE_RATE, 64)
            .unwrap();
        let during = braids_lifecycle_counts();
        assert!(during.created > before.created);

        let parameters = parameters(patch.id(), VoiceEnvelope::DEFAULT, [0.0, 0.5, 0.5]);
        prepared
            .dispatch(message(MidiMessageKind::NoteOn, 60, 127), &parameters)
            .unwrap();
        let mut output = [0.0_f32; 128];
        prepared.render(&mut output, 64, &parameters);
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().any(|sample| sample.abs() > 0.000_001));
        assert_eq!(
            prepared.maximum_native_chunk_observed,
            BRAIDS_INTERNAL_CHUNK_FRAMES
        );
        drop(prepared);
        assert!(braids_lifecycle_counts().destroyed > before.destroyed);
    }

    #[test]
    fn idle_first_then_oldest_stealing_is_patch_local_and_all_notes_off_is_bounded() {
        let patch = patch(1);
        let mut prepared = BraidsPreparer::new()
            .unwrap()
            .prepare_patch(&patch, BRAIDS_HOST_SAMPLE_RATE, 64)
            .unwrap();
        let parameters = parameters(patch.id(), VoiceEnvelope::DEFAULT, [0.0, 0.5, 0.5]);
        for note in 48..64 {
            prepared
                .dispatch(message(MidiMessageKind::NoteOn, note, 100), &parameters)
                .unwrap();
        }
        assert_eq!(prepared.active_voice_count(), 16);
        assert_eq!(prepared.voices[0].note, Some(48));

        prepared
            .dispatch(message(MidiMessageKind::NoteOn, 80, 100), &parameters)
            .unwrap();
        assert_eq!(prepared.active_voice_count(), 16);
        assert_eq!(prepared.voices[0].note, Some(80));
        assert!(prepared.voices.iter().all(|voice| voice.note != Some(48)));

        prepared
            .dispatch(message(MidiMessageKind::AllNotesOff, 0, 0), &parameters)
            .unwrap();
        assert_eq!(prepared.active_voice_count(), 0);
    }

    #[test]
    fn overlapping_notes_release_independently_and_midi_expression_is_local() {
        let patch = patch(1);
        let envelope = VoiceEnvelope::new(0.0, 0.0, 1.0, 10.0).unwrap();
        let parameters = parameters(patch.id(), envelope, [0.0, 0.5, 0.5]);
        let mut prepared = BraidsPreparer::new()
            .unwrap()
            .prepare_patch(&patch, BRAIDS_HOST_SAMPLE_RATE, 64)
            .unwrap();
        prepared
            .dispatch(message(MidiMessageKind::NoteOn, 60, 127), &parameters)
            .unwrap();
        prepared
            .dispatch(message(MidiMessageKind::NoteOn, 64, 127), &parameters)
            .unwrap();
        prepared
            .dispatch(message(MidiMessageKind::NoteOff, 60, 0), &parameters)
            .unwrap();
        assert!(prepared.voices[0].envelope.is_releasing());
        assert!(!prepared.voices[1].envelope.is_releasing());

        prepared
            .dispatch(message(MidiMessageKind::ControlChange, 11, 64), &parameters)
            .unwrap();
        prepared
            .dispatch(message(MidiMessageKind::PitchBend, 127, 127), &parameters)
            .unwrap();
        assert!((prepared.expression - (64.0 / 127.0)).abs() < f32::EPSILON);
        assert!(prepared.pitch_bend > 0);
        assert!(matches!(
            prepared.dispatch(message(MidiMessageKind::ProgramChange, 1, 0), &parameters),
            Err(PreparedInstrumentError::UnsupportedMidiKind {
                kind: MidiMessageKind::ProgramChange
            })
        ));
    }

    #[test]
    fn model_timbre_and_color_each_change_the_native_render() {
        fn render(scalars: [f32; 3]) -> [f32; 128] {
            let patch = patch(1);
            let parameters = parameters(patch.id(), VoiceEnvelope::DEFAULT, scalars);
            let mut prepared = BraidsPreparer::new()
                .unwrap()
                .prepare_patch(&patch, BRAIDS_HOST_SAMPLE_RATE, 64)
                .unwrap();
            prepared
                .dispatch(message(MidiMessageKind::NoteOn, 60, 127), &parameters)
                .unwrap();
            let mut output = [0.0; 128];
            prepared.render(&mut output, 64, &parameters);
            output
        }

        let baseline = render([0.0, 0.5, 0.5]);
        assert_ne!(baseline, render([1.0, 0.5, 0.5]));
        assert_ne!(baseline, render([0.0, 0.75, 0.5]));
        assert_ne!(baseline, render([0.0, 0.5, 0.75]));
    }
}
