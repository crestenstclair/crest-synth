use crate::control::app_event::{AppEvent, Direction};
use crate::control::app_state::{EventRejection, StateAccepted};
use crate::control::state_snapshot::StateSnapshot;
use crate::control::text_projection::TextProjection;
use crate::control::top_level_context::TopLevelContext;
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::GraphRevision;
use crate::synth::instrument_capability::InstrumentConfig;
use crate::synth::patch::Patch;
use crate::synth::voice_envelope::VoiceEnvelope;
use core::fmt;
use serde::{Serialize, Serializer};

/// The stable origin of an application event.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventSource {
    Startup,
    Keyboard,
    AutomaticMidi,
    DemoScene,
    System,
}

/// Whether the reducer accepted or rejected an input.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventOutcome {
    Accepted,
    Rejected,
}

/// A stable direction value used by the serialized input descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventDirection {
    Up,
    Down,
    Left,
    Right,
}

impl From<Direction> for EventDirection {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::Up => Self::Up,
            Direction::Down => Self::Down,
            Direction::Left => Self::Left,
            Direction::Right => Self::Right,
        }
    }
}

/// The stable kind of a normalized MIDI message.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MidiKind {
    NoteOn,
    NoteOff,
    ControlChange,
    ProgramChange,
    ChannelPressure,
    PitchBend,
    AllNotesOff,
}

impl From<MidiMessageKind> for MidiKind {
    fn from(kind: MidiMessageKind) -> Self {
        match kind {
            MidiMessageKind::NoteOn => Self::NoteOn,
            MidiMessageKind::NoteOff => Self::NoteOff,
            MidiMessageKind::ControlChange => Self::ControlChange,
            MidiMessageKind::ProgramChange => Self::ProgramChange,
            MidiMessageKind::ChannelPressure => Self::ChannelPressure,
            MidiMessageKind::PitchBend => Self::PitchBend,
            MidiMessageKind::AllNotesOff => Self::AllNotesOff,
        }
    }
}

/// A complete, bounded MIDI payload suitable for deterministic observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiInput {
    channel: u8,
    kind: MidiKind,
    data1: u8,
    data2: u8,
}

impl MidiInput {
    pub const fn channel(&self) -> u8 {
        self.channel
    }

    pub const fn kind(&self) -> MidiKind {
        self.kind
    }

    pub const fn data1(&self) -> u8 {
        self.data1
    }

    pub const fn data2(&self) -> u8 {
        self.data2
    }
}

impl From<MidiMessage> for MidiInput {
    fn from(message: MidiMessage) -> Self {
        Self {
            channel: message.channel().value(),
            kind: message.kind().into(),
            data1: message.data1(),
            data2: message.data2(),
        }
    }
}

/// One patch payload recorded from the startup installation event.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchInput {
    id: u32,
    name: String,
    channel: u8,
    instrument: InstrumentConfig,
    envelope: VoiceEnvelope,
    gain_db: f32,
    pan: f32,
    reverb_send: f32,
    delay_send: f32,
}

impl PatchInput {
    pub const fn id(&self) -> u32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn channel(&self) -> u8 {
        self.channel
    }

    pub const fn instrument_config(&self) -> &InstrumentConfig {
        &self.instrument
    }

    pub const fn envelope(&self) -> &VoiceEnvelope {
        &self.envelope
    }

    pub const fn gain_db(&self) -> f32 {
        self.gain_db
    }

    pub const fn pan(&self) -> f32 {
        self.pan
    }

    pub const fn reverb_send(&self) -> f32 {
        self.reverb_send
    }

    pub const fn delay_send(&self) -> f32 {
        self.delay_send
    }
}

impl From<&Patch> for PatchInput {
    fn from(patch: &Patch) -> Self {
        Self {
            id: patch.id().value(),
            name: patch.name().to_owned(),
            channel: patch.channel().value(),
            instrument: patch.instrument_config().clone(),
            envelope: *patch.envelope(),
            gain_db: patch.parameters().gain_db(),
            pan: patch.parameters().pan(),
            reverb_send: patch.parameters().reverb_send(),
            delay_send: patch.parameters().delay_send(),
        }
    }
}

/// A stable tagged representation of every current AppEvent variant.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EventInput {
    SelectContext {
        context: TopLevelContext,
    },
    Navigate {
        direction: EventDirection,
    },
    Adjust {
        direction: EventDirection,
    },
    InstallPatches {
        patches: Vec<PatchInput>,
    },
    Midi {
        #[serde(rename = "patchId")]
        patch_id: u32,
        message: MidiInput,
    },
}

impl From<&AppEvent> for EventInput {
    fn from(event: &AppEvent) -> Self {
        match event {
            AppEvent::SelectContext(context) => Self::SelectContext { context: *context },
            AppEvent::Navigate(direction) => Self::Navigate {
                direction: (*direction).into(),
            },
            AppEvent::Adjust(direction) => Self::Adjust {
                direction: (*direction).into(),
            },
            AppEvent::InstallPatches(patches) => Self::InstallPatches {
                patches: patches.iter().map(PatchInput::from).collect(),
            },
            AppEvent::Midi { patch_id, message } => Self::Midi {
                patch_id: patch_id.value(),
                message: (*message).into(),
            },
        }
    }
}

/// A stable, allocation-free description of one command sent toward audio.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AudioEffect {
    PatchMidi {
        #[serde(rename = "patchId")]
        patch_id: u32,
        message: MidiInput,
    },
    AllNotesOff,
}

impl From<AudioCommand> for AudioEffect {
    fn from(command: AudioCommand) -> Self {
        match command {
            AudioCommand::PatchMidi { patch_id, message } => Self::PatchMidi {
                patch_id: patch_id.value(),
                message: message.into(),
            },
            AudioCommand::AllNotesOff => Self::AllNotesOff,
        }
    }
}

/// One deterministic effect derived after an input has been accepted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EmittedEvent {
    StateAccepted {
        generation: u64,
    },
    ParameterSnapshotPublished {
        generation: u64,
        #[serde(rename = "graphRevision")]
        graph_revision: GraphRevision,
    },
    AudioCommand {
        effect: AudioEffect,
    },
}

/// A coherence failure detected while assembling an EventRecord.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventRecordError {
    GenerationOverflow,
    GenerationDidNotAdvance { expected: u64, actual: u64 },
    StateHashDidNotChange,
    ParameterGenerationMismatch { expected: u64, actual: u64 },
    ProjectionStateHashMismatch { expected: String, actual: String },
}

impl fmt::Display for EventRecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GenerationOverflow => {
                formatter.write_str("event record generation cannot advance beyond u64::MAX")
            }
            Self::GenerationDidNotAdvance { expected, actual } => write!(
                formatter,
                "accepted generation must advance exactly once: expected {expected}, got {actual}"
            ),
            Self::StateHashDidNotChange => {
                formatter.write_str("accepted event must produce a new state hash")
            }
            Self::ParameterGenerationMismatch { expected, actual } => write!(
                formatter,
                "parameter generation must equal the recorded state generation: expected {expected}, got {actual}"
            ),
            Self::ProjectionStateHashMismatch { expected, actual } => write!(
                formatter,
                "projection state hash must match the recorded state hash: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for EventRecordError {}

/// One deterministic control-side record of an input and its complete outcome.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRecord {
    sequence: u64,
    source: EventSource,
    input: EventInput,
    outcome: EventOutcome,
    generation_before: u64,
    generation_after: u64,
    state_hash_before: String,
    state_hash_after: String,
    parameter_generation: u64,
    selected_line: usize,
    projection_state_hash: String,
    emitted_events: Vec<EmittedEvent>,
    #[serde(serialize_with = "serialize_rejection")]
    rejection: Option<EventRejection>,
}

impl EventRecord {
    pub const SERIALIZED_PROPERTY_DESCRIPTOR: &'static [&'static str] = &[
        "sequence",
        "source",
        "input",
        "outcome",
        "rejection",
        "generationBefore",
        "generationAfter",
        "stateHashBefore",
        "stateHashAfter",
        "emittedEvents",
        "parameterGeneration",
        "selectedLine",
        "projectionStateHash",
    ];

    /// Returns the stable top-level properties of every serialized event record.
    pub const fn serialized_property_descriptor() -> &'static [&'static str] {
        Self::SERIALIZED_PROPERTY_DESCRIPTOR
    }

    /// Every normalized leaf path emitted by the EventRecord serializer.
    ///
    /// Object components are separated by `.`, and `[]` denotes any element
    /// of a serialized array. The descriptor is sorted, contains no duplicates,
    /// and must be compared with the union of discriminating accepted and
    /// rejected records rather than with one convenient record.
    pub const SERIALIZED_LEAF_DESCRIPTOR: &[&str] = &[
        "emittedEvents[].effect.kind",
        "emittedEvents[].effect.message.channel",
        "emittedEvents[].effect.message.data1",
        "emittedEvents[].effect.message.data2",
        "emittedEvents[].effect.message.kind",
        "emittedEvents[].effect.patchId",
        "emittedEvents[].generation",
        "emittedEvents[].graphRevision",
        "emittedEvents[].kind",
        "generationAfter",
        "generationBefore",
        "input.context",
        "input.direction",
        "input.kind",
        "input.message.channel",
        "input.message.data1",
        "input.message.data2",
        "input.message.kind",
        "input.patchId",
        "input.patches[].channel",
        "input.patches[].delaySend",
        "input.patches[].envelope.attackMilliseconds",
        "input.patches[].envelope.decayMilliseconds",
        "input.patches[].envelope.releaseMilliseconds",
        "input.patches[].envelope.sustain",
        "input.patches[].gainDb",
        "input.patches[].id",
        "input.patches[].instrument.assetReferences[].parameterId",
        "input.patches[].instrument.assetReferences[].reference.kind",
        "input.patches[].instrument.assetReferences[].reference.locator",
        "input.patches[].instrument.capabilityId",
        "input.patches[].instrument.values[].parameterId",
        "input.patches[].instrument.values[].value.kind",
        "input.patches[].instrument.values[].value.value",
        "input.patches[].name",
        "input.patches[].pan",
        "input.patches[].reverbSend",
        "outcome",
        "parameterGeneration",
        "projectionStateHash",
        "rejection",
        "selectedLine",
        "sequence",
        "source",
        "stateHashAfter",
        "stateHashBefore",
    ];

    /// Returns the production-owned serialized leaf descriptor.
    #[must_use]
    pub const fn serialized_leaf_descriptor() -> &'static [&'static str] {
        Self::SERIALIZED_LEAF_DESCRIPTOR
    }

    /// Records an accepted transition after all coherent projections exist.
    #[allow(clippy::too_many_arguments)]
    pub fn accepted(
        sequence: u64,
        source: EventSource,
        input: &AppEvent,
        generation_before: u64,
        state_hash_before: impl Into<String>,
        accepted: StateAccepted,
        snapshot: &StateSnapshot,
        parameter_generation: u64,
        parameter_graph_revision: GraphRevision,
        projection: &TextProjection,
        audio_command: Option<AudioCommand>,
    ) -> Result<Self, EventRecordError> {
        let generation_after = generation_before
            .checked_add(1)
            .ok_or(EventRecordError::GenerationOverflow)?;
        if accepted.generation() != generation_after {
            return Err(EventRecordError::GenerationDidNotAdvance {
                expected: generation_after,
                actual: accepted.generation(),
            });
        }

        let state_hash_before = state_hash_before.into();
        if state_hash_before == snapshot.hash() {
            return Err(EventRecordError::StateHashDidNotChange);
        }
        validate_parameter_generation(generation_after, parameter_generation)?;
        validate_projection_hash(snapshot.hash(), projection.state_hash())?;

        let mut emitted_events = vec![
            EmittedEvent::StateAccepted {
                generation: accepted.generation(),
            },
            EmittedEvent::ParameterSnapshotPublished {
                generation: parameter_generation,
                graph_revision: parameter_graph_revision,
            },
        ];
        if let Some(command) = audio_command {
            emitted_events.push(EmittedEvent::AudioCommand {
                effect: command.into(),
            });
        }

        Ok(Self {
            sequence,
            source,
            input: EventInput::from(input),
            outcome: EventOutcome::Accepted,
            generation_before,
            generation_after,
            state_hash_before,
            state_hash_after: snapshot.hash().to_owned(),
            parameter_generation,
            selected_line: projection.selected_line(),
            projection_state_hash: projection.state_hash().to_owned(),
            emitted_events,
            rejection: None,
        })
    }

    /// Records a rejected input without deriving or publishing new effects.
    #[allow(clippy::too_many_arguments)]
    pub fn rejected(
        sequence: u64,
        source: EventSource,
        input: &AppEvent,
        generation: u64,
        state_hash: impl Into<String>,
        parameter_generation: u64,
        projection: &TextProjection,
        rejection: EventRejection,
    ) -> Result<Self, EventRecordError> {
        let state_hash = state_hash.into();
        validate_parameter_generation(generation, parameter_generation)?;
        validate_projection_hash(&state_hash, projection.state_hash())?;

        Ok(Self {
            sequence,
            source,
            input: EventInput::from(input),
            outcome: EventOutcome::Rejected,
            generation_before: generation,
            generation_after: generation,
            state_hash_before: state_hash.clone(),
            state_hash_after: state_hash,
            parameter_generation,
            selected_line: projection.selected_line(),
            projection_state_hash: projection.state_hash().to_owned(),
            emitted_events: Vec::new(),
            rejection: Some(rejection),
        })
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn source(&self) -> EventSource {
        self.source
    }

    pub const fn input(&self) -> &EventInput {
        &self.input
    }

    pub const fn outcome(&self) -> EventOutcome {
        self.outcome
    }

    pub const fn generation_before(&self) -> u64 {
        self.generation_before
    }

    pub const fn generation_after(&self) -> u64 {
        self.generation_after
    }

    pub fn state_hash_before(&self) -> &str {
        &self.state_hash_before
    }

    pub fn state_hash_after(&self) -> &str {
        &self.state_hash_after
    }

    pub const fn parameter_generation(&self) -> u64 {
        self.parameter_generation
    }

    pub const fn selected_line(&self) -> usize {
        self.selected_line
    }

    pub fn projection_state_hash(&self) -> &str {
        &self.projection_state_hash
    }

    pub fn emitted_events(&self) -> &[EmittedEvent] {
        &self.emitted_events
    }

    pub const fn rejection(&self) -> Option<EventRejection> {
        self.rejection
    }

    /// Serializes the complete record with stable camelCase field names.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

fn validate_parameter_generation(expected: u64, actual: u64) -> Result<(), EventRecordError> {
    if actual == expected {
        Ok(())
    } else {
        Err(EventRecordError::ParameterGenerationMismatch { expected, actual })
    }
}

fn validate_projection_hash(expected: &str, actual: &str) -> Result<(), EventRecordError> {
    if actual == expected {
        Ok(())
    } else {
        Err(EventRecordError::ProjectionStateHashMismatch {
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        })
    }
}

fn serialize_rejection<S>(
    rejection: &Option<EventRejection>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    rejection.map(rejection_name).serialize(serializer)
}

const fn rejection_name(rejection: EventRejection) -> &'static str {
    match rejection {
        EventRejection::InstallationClosed => "installationClosed",
        EventRejection::TooManyPatches => "tooManyPatches",
        EventRejection::DuplicateMidiChannel => "duplicateMidiChannel",
        EventRejection::InvalidInstrumentConfig => "invalidInstrumentConfig",
        EventRejection::NoPatchesInstalled => "noPatchesInstalled",
        EventRejection::UnknownPatch => "unknownPatch",
        EventRejection::InvalidSelection => "invalidSelection",
        EventRejection::ParameterAtBoundary => "parameterAtBoundary",
        EventRejection::InvalidParameterValue => "invalidParameterValue",
        EventRejection::ActionUnavailableInContext => "actionUnavailableInContext",
        EventRejection::GenerationOverflow => "generationOverflow",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioEffect, EmittedEvent, EventDirection, EventInput, EventOutcome, EventRecord,
        EventRecordError, EventSource, MidiInput, MidiKind, PatchInput,
    };
    use crate::adapter::hidef_soundfont_capability::{
        HiDefSoundFontCapability, HIDEF_CAPABILITY_ID, SOUNDFONT_BANK_PARAMETER_ID,
        SOUNDFONT_PERCUSSION_PARAMETER_ID, SOUNDFONT_PROGRAM_PARAMETER_ID,
    };
    use crate::control::app_event::{AppEvent, Direction};
    use crate::control::app_state::{AppState, EventRejection};
    use crate::control::state_snapshot::StateSnapshot;
    use crate::control::text_projection::TextProjection;
    use crate::control::TopLevelContext;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::GraphRevision;
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::voice_envelope::VoiceEnvelope;
    use crate::synth::{ParameterId, ParameterValue};
    use crate::testing::automatic_midi_test::create_soundfont_config;
    use serde_json::Value;
    use std::collections::BTreeSet;

    fn patch(id: u32) -> Patch {
        let provider = HiDefSoundFontCapability::new().unwrap();
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            create_soundfont_config(
                &provider,
                SoundFontInstrument::new(128, (id - 1) as u8, false).unwrap(),
            )
            .unwrap(),
            MidiChannel::new((id - 1) as u8).unwrap(),
            ChannelParameters::new(-6.0, 0.25, 0.5, 0.75).unwrap(),
        )
    }

    fn installed_state() -> AppState {
        let provider = HiDefSoundFontCapability::new().unwrap();
        let mut state = AppState::new(
            provider.registry().unwrap(),
            GlobalParameters::new(-3.0, 0.5, 0.4, 0.25, 250.0, 0.3, 0.2).unwrap(),
        );
        state
            .apply(AppEvent::InstallPatches(vec![patch(1)]))
            .unwrap();
        state
    }

    fn snapshot_and_projection(generation: u64) -> (StateSnapshot, TextProjection) {
        let snapshot = StateSnapshot::new(format!("{{\"generation\":{generation}}}"));
        let projection = TextProjection::new(
            "GLOBAL\n> masterGainDb=-3".to_owned(),
            1,
            snapshot.hash().to_owned(),
        );
        (snapshot, projection)
    }

    fn schema_record(
        source: EventSource,
        input: EventInput,
        outcome: EventOutcome,
        emitted_events: Vec<EmittedEvent>,
        rejection: Option<EventRejection>,
    ) -> EventRecord {
        EventRecord {
            sequence: 1,
            source,
            input,
            outcome,
            generation_before: 4,
            generation_after: 5,
            state_hash_before: "before".to_owned(),
            state_hash_after: "after".to_owned(),
            parameter_generation: 5,
            selected_line: 2,
            projection_state_hash: "after".to_owned(),
            emitted_events,
            rejection,
        }
    }

    fn discriminating_schema_records() -> Vec<EventRecord> {
        let provider = HiDefSoundFontCapability::new().unwrap();
        let patch = PatchInput {
            id: 7,
            name: "Schema Patch".to_owned(),
            channel: 3,
            instrument: create_soundfont_config(
                &provider,
                SoundFontInstrument::new(128, 11, false).unwrap(),
            )
            .unwrap(),
            envelope: VoiceEnvelope::default(),
            gain_db: -4.0,
            pan: 0.25,
            reverb_send: 0.5,
            delay_send: 0.75,
        };
        let message = MidiInput {
            channel: 3,
            kind: MidiKind::PitchBend,
            data1: 1,
            data2: 65,
        };

        vec![
            schema_record(
                EventSource::Keyboard,
                EventInput::SelectContext {
                    context: TopLevelContext::Patch,
                },
                EventOutcome::Accepted,
                Vec::new(),
                None,
            ),
            schema_record(
                EventSource::Startup,
                EventInput::InstallPatches {
                    patches: vec![patch],
                },
                EventOutcome::Accepted,
                vec![
                    EmittedEvent::StateAccepted { generation: 5 },
                    EmittedEvent::ParameterSnapshotPublished {
                        generation: 5,
                        graph_revision: GraphRevision::INITIAL,
                    },
                ],
                None,
            ),
            schema_record(
                EventSource::Keyboard,
                EventInput::Navigate {
                    direction: EventDirection::Up,
                },
                EventOutcome::Rejected,
                Vec::new(),
                Some(EventRejection::ParameterAtBoundary),
            ),
            schema_record(
                EventSource::AutomaticMidi,
                EventInput::Midi {
                    patch_id: 7,
                    message,
                },
                EventOutcome::Accepted,
                vec![EmittedEvent::AudioCommand {
                    effect: AudioEffect::PatchMidi {
                        patch_id: 7,
                        message,
                    },
                }],
                None,
            ),
            schema_record(
                EventSource::DemoScene,
                EventInput::Adjust {
                    direction: EventDirection::Right,
                },
                EventOutcome::Accepted,
                vec![EmittedEvent::AudioCommand {
                    effect: AudioEffect::AllNotesOff,
                }],
                None,
            ),
            schema_record(
                EventSource::System,
                EventInput::Navigate {
                    direction: EventDirection::Down,
                },
                EventOutcome::Accepted,
                Vec::new(),
                None,
            ),
        ]
    }

    fn collect_leaf_paths(value: &Value, prefix: &str, paths: &mut BTreeSet<String>) {
        match value {
            Value::Object(fields) => {
                for (name, value) in fields {
                    let path = if prefix.is_empty() {
                        name.to_owned()
                    } else {
                        format!("{prefix}.{name}")
                    };
                    collect_leaf_paths(value, &path, paths);
                }
            }
            Value::Array(values) => {
                let path = format!("{prefix}[]");
                for value in values {
                    collect_leaf_paths(value, &path, paths);
                }
            }
            _ => {
                assert!(!prefix.is_empty());
                paths.insert(prefix.to_owned());
            }
        }
    }

    #[test]
    fn serialized_leaf_descriptor_exactly_matches_discriminating_record_union() {
        let descriptor = EventRecord::serialized_leaf_descriptor();
        assert!(descriptor.windows(2).all(|pair| pair[0] < pair[1]));

        let described = descriptor
            .iter()
            .copied()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        assert_eq!(described.len(), descriptor.len());

        let mut discovered = BTreeSet::new();
        for record in discriminating_schema_records() {
            let value = serde_json::to_value(record).expect("schema record serializes");
            collect_leaf_paths(&value, "", &mut discovered);
        }

        assert_eq!(described, discovered);
    }

    #[test]
    fn accepted_record_keeps_one_coherent_hash_generation_and_effect_chain() {
        let mut state = installed_state();
        let message = MidiMessage::try_new(
            MidiChannel::new(0).unwrap(),
            MidiMessageKind::NoteOn,
            60,
            100,
        )
        .unwrap();
        let event = AppEvent::Midi {
            patch_id: PatchId::new(1).unwrap(),
            message,
        };
        let generation_before = state.generation();
        let outcome = state.apply(event.clone()).unwrap();
        let (snapshot, projection) = snapshot_and_projection(state.generation());

        let record = EventRecord::accepted(
            7,
            EventSource::AutomaticMidi,
            &event,
            generation_before,
            "previous-state-hash",
            outcome.accepted(),
            &snapshot,
            state.generation(),
            GraphRevision::INITIAL,
            &projection,
            outcome.audio_command().copied(),
        )
        .unwrap();

        assert_eq!(record.sequence(), 7);
        assert_eq!(record.outcome(), EventOutcome::Accepted);
        assert_eq!(record.generation_before(), 1);
        assert_eq!(record.generation_after(), 2);
        assert_eq!(record.state_hash_before(), "previous-state-hash");
        assert_eq!(record.state_hash_after(), snapshot.hash());
        assert_eq!(record.projection_state_hash(), snapshot.hash());
        assert_eq!(record.parameter_generation(), 2);
        assert_eq!(record.selected_line(), 1);
        assert_eq!(record.rejection(), None);
        assert_eq!(record.emitted_events().len(), 3);
        assert_eq!(
            record.emitted_events()[0],
            EmittedEvent::StateAccepted { generation: 2 }
        );
        assert_eq!(
            record.emitted_events()[1],
            EmittedEvent::ParameterSnapshotPublished {
                generation: 2,
                graph_revision: GraphRevision::INITIAL,
            }
        );
        assert_eq!(
            record.emitted_events()[2],
            EmittedEvent::AudioCommand {
                effect: AudioEffect::PatchMidi {
                    patch_id: 1,
                    message: message.into(),
                },
            }
        );

        let json: serde_json::Value = serde_json::from_str(&record.to_json().unwrap()).unwrap();
        assert_eq!(json["source"], "automaticMidi");
        assert_eq!(json["input"]["kind"], "midi");
        assert_eq!(json["input"]["patchId"], 1);
        assert_eq!(json["input"]["message"]["kind"], "noteOn");
        assert_eq!(json["rejection"], serde_json::Value::Null);
    }

    #[test]
    fn rejected_record_preserves_state_and_emits_nothing() {
        let state = installed_state();
        let event = AppEvent::Adjust(Direction::Right);
        let (snapshot, projection) = snapshot_and_projection(state.generation());

        let record = EventRecord::rejected(
            8,
            EventSource::Keyboard,
            &event,
            state.generation(),
            snapshot.hash(),
            state.generation(),
            &projection,
            EventRejection::ParameterAtBoundary,
        )
        .unwrap();

        assert_eq!(record.outcome(), EventOutcome::Rejected);
        assert_eq!(record.generation_before(), record.generation_after());
        assert_eq!(record.state_hash_before(), record.state_hash_after());
        assert!(record.emitted_events().is_empty());
        assert_eq!(
            record.rejection(),
            Some(EventRejection::ParameterAtBoundary)
        );

        let json: serde_json::Value = serde_json::from_str(&record.to_json().unwrap()).unwrap();
        assert_eq!(json["input"]["kind"], "adjust");
        assert_eq!(json["input"]["direction"], "right");
        assert_eq!(json["rejection"], "parameterAtBoundary");
    }

    #[test]
    fn install_input_records_every_domain_payload() {
        let event = AppEvent::InstallPatches(vec![patch(3)]);
        let input = EventInput::from(&event);

        let EventInput::InstallPatches { patches } = input else {
            panic!("expected installation descriptor");
        };
        let installed = &patches[0];

        assert_eq!(installed.id(), 3);
        assert_eq!(installed.name(), "Patch 3");
        assert_eq!(installed.channel(), 2);
        assert_eq!(
            installed.instrument_config().capability_id().as_str(),
            HIDEF_CAPABILITY_ID
        );
        assert_eq!(
            installed
                .instrument_config()
                .value(&ParameterId::new(SOUNDFONT_BANK_PARAMETER_ID).unwrap()),
            Some(&ParameterValue::Stepped(128))
        );
        assert_eq!(
            installed
                .instrument_config()
                .value(&ParameterId::new(SOUNDFONT_PROGRAM_PARAMETER_ID).unwrap()),
            Some(&ParameterValue::Stepped(2))
        );
        assert_eq!(
            installed
                .instrument_config()
                .value(&ParameterId::new(SOUNDFONT_PERCUSSION_PARAMETER_ID).unwrap()),
            Some(&ParameterValue::Toggle(false))
        );
        assert_eq!(installed.gain_db(), -6.0);
        assert_eq!(installed.pan(), 0.25);
        assert_eq!(installed.reverb_send(), 0.5);
        assert_eq!(installed.delay_send(), 0.75);
    }

    #[test]
    fn accepted_record_rejects_incoherent_projection_generation() {
        let mut state = installed_state();
        let event = AppEvent::Navigate(Direction::Down);
        let generation_before = state.generation();
        let outcome = state.apply(event.clone()).unwrap();
        let (snapshot, projection) = snapshot_and_projection(state.generation());

        let error = EventRecord::accepted(
            1,
            EventSource::DemoScene,
            &event,
            generation_before,
            "previous-state-hash",
            outcome.accepted(),
            &snapshot,
            state.generation() + 1,
            GraphRevision::INITIAL,
            &projection,
            None,
        )
        .unwrap_err();

        assert_eq!(
            error,
            EventRecordError::ParameterGenerationMismatch {
                expected: state.generation(),
                actual: state.generation() + 1,
            }
        );
    }
}
