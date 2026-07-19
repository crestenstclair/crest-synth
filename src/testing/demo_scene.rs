use crate::control::app_event::{AppEvent, Direction};
use crate::control::app_state::EventRejection;
use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::patch_id::PatchId;
use crate::shell::window_input::{WindowInput, WindowInputKind, WindowKey};
use crate::synth::patch::Patch;
use core::fmt;
use std::time::Duration;

const TICK_DURATION: Duration = Duration::from_millis(10);
const PATCH_PARAMETERS: [&str; 4] = ["gainDb", "pan", "reverbSend", "delaySend"];
const GLOBAL_PARAMETERS: [&str; 7] = [
    "masterGainDb",
    "reverbRoomSize",
    "reverbDamping",
    "reverbReturn",
    "delayMilliseconds",
    "delayFeedback",
    "delayReturn",
];

/// An immutable point at which a scene runner records coherent projections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemoCheckpoint {
    name: String,
    expected_last_rejection: Option<EventRejection>,
}

impl DemoCheckpoint {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            expected_last_rejection: None,
        }
    }

    pub fn after_rejection(
        name: impl Into<String>,
        expected_last_rejection: EventRejection,
    ) -> Self {
        Self {
            name: name.into(),
            expected_last_rejection: Some(expected_last_rejection),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn expected_last_rejection(&self) -> Option<EventRejection> {
        self.expected_last_rejection
    }
}

/// One semantic MIDI input and the reducer outcome expected from it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MidiProbe {
    patch_id: PatchId,
    message: MidiMessage,
    expected_rejection: Option<EventRejection>,
}

impl MidiProbe {
    pub const fn accepted(patch_id: PatchId, message: MidiMessage) -> Self {
        Self {
            patch_id,
            message,
            expected_rejection: None,
        }
    }

    pub const fn rejected(
        patch_id: PatchId,
        message: MidiMessage,
        expected_rejection: EventRejection,
    ) -> Self {
        Self {
            patch_id,
            message,
            expected_rejection: Some(expected_rejection),
        }
    }

    pub const fn patch_id(&self) -> PatchId {
        self.patch_id
    }

    pub const fn message(&self) -> MidiMessage {
        self.message
    }

    pub const fn expected_rejection(&self) -> Option<EventRejection> {
        self.expected_rejection
    }
}

/// One deterministic action in the exhaustive headless scene.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DemoSceneStep {
    WindowInput(WindowInput),
    MidiProbe(MidiProbe),
    Tick(Duration),
    Checkpoint(DemoCheckpoint),
}

/// A complete deterministic control-surface scene derived from installed Patches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemoScene {
    name: String,
    schema_version: u32,
    steps: Vec<DemoSceneStep>,
    expected_coverage: Vec<String>,
}

impl DemoScene {
    pub const NAME: &'static str = "exhaustive-gui-demo";
    pub const SCHEMA_VERSION: u32 = 1;

    /// Derives the complete current scene from the accepted fixture Patch list.
    ///
    /// At least two Patches are required because routing and mixer observations
    /// must discriminate the edited Patch from an unaffected Patch.
    pub fn exhaustive(installed_patches: &[Patch]) -> Result<Self, DemoSceneError> {
        if installed_patches.len() < 2 {
            return Err(DemoSceneError::InsufficientPatches {
                actual: installed_patches.len(),
            });
        }

        Ok(Self {
            name: Self::NAME.to_owned(),
            schema_version: Self::SCHEMA_VERSION,
            steps: build_steps(installed_patches),
            expected_coverage: build_expected_coverage(installed_patches),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn steps(&self) -> &[DemoSceneStep] {
        &self.steps
    }

    pub fn expected_coverage(&self) -> &[String] {
        &self.expected_coverage
    }

    /// A safe journal capacity for startup plus every potentially observable step.
    pub fn event_log_capacity(&self) -> usize {
        self.steps.len().saturating_add(1)
    }

    pub fn into_steps(self) -> Vec<DemoSceneStep> {
        self.steps
    }
}

/// A scene cannot prove multi-Patch isolation without two installed Patches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemoSceneError {
    InsufficientPatches { actual: usize },
}

impl fmt::Display for DemoSceneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InsufficientPatches { actual } => write!(
                formatter,
                "the exhaustive demo requires at least two installed Patches, got {actual}"
            ),
        }
    }
}

impl std::error::Error for DemoSceneError {}

fn build_steps(patches: &[Patch]) -> Vec<DemoSceneStep> {
    let mut steps = Vec::new();
    push_checkpoint(&mut steps, DemoCheckpoint::new("scene.start"));

    for input in complete_window_vocabulary() {
        steps.push(DemoSceneStep::WindowInput(input));
    }

    // Prove that focus loss clears modifier state. The following W/S pair is
    // navigation and returns the selection to its original parameter.
    steps.push(DemoSceneStep::WindowInput(WindowInput::key_down(
        WindowKey::K,
    )));
    steps.push(DemoSceneStep::WindowInput(WindowInput::focus_lost()));
    push_key_press(&mut steps, WindowKey::W);
    push_key_press(&mut steps, WindowKey::S);
    push_checkpoint(&mut steps, DemoCheckpoint::new("input.vocabulary"));

    push_boundary_probe(&mut steps);

    for patch in patches {
        for parameter in PATCH_PARAMETERS {
            push_reversible_adjustments(&mut steps);
            push_checkpoint(
                &mut steps,
                DemoCheckpoint::new(format!(
                    "patch.{}.parameter.{parameter}",
                    patch.id().value()
                )),
            );
            push_key_press(&mut steps, WindowKey::S);
        }
        push_key_press(&mut steps, WindowKey::D);
    }

    for parameter in GLOBAL_PARAMETERS {
        push_reversible_adjustments(&mut steps);
        push_checkpoint(
            &mut steps,
            DemoCheckpoint::new(format!("global.parameter.{parameter}")),
        );
        push_key_press(&mut steps, WindowKey::S);
    }

    // Exercise both directions at both Patch/GLOBAL section wraps, then restore
    // the initial Patch-zero, parameter-zero selection.
    push_key_press(&mut steps, WindowKey::A);
    push_key_press(&mut steps, WindowKey::D);
    push_key_press(&mut steps, WindowKey::D);
    push_key_press(&mut steps, WindowKey::A);
    push_key_press(&mut steps, WindowKey::D);
    push_checkpoint(&mut steps, DemoCheckpoint::new("selection.restored"));

    for patch in patches {
        for message in midi_messages(patch.channel()) {
            steps.push(DemoSceneStep::MidiProbe(MidiProbe::accepted(
                patch.id(),
                message,
            )));
            steps.push(DemoSceneStep::Tick(TICK_DURATION));
        }
        push_checkpoint(
            &mut steps,
            DemoCheckpoint::new(format!("midi.patch.{}", patch.id().value())),
        );
    }

    let unknown_patch = first_unknown_patch_id(patches);
    let unknown_message = midi_message(patches[0].channel(), MidiMessageKind::NoteOn, 67, 91);
    steps.push(DemoSceneStep::MidiProbe(MidiProbe::rejected(
        unknown_patch,
        unknown_message,
        EventRejection::UnknownPatch,
    )));
    push_checkpoint(
        &mut steps,
        DemoCheckpoint::after_rejection("midi.unknownPatch", EventRejection::UnknownPatch),
    );
    push_checkpoint(&mut steps, DemoCheckpoint::new("scene.complete"));

    steps
}

fn complete_window_vocabulary() -> [WindowInput; 13] {
    [
        WindowInput::key_down(WindowKey::W),
        WindowInput::key_up(WindowKey::W),
        WindowInput::key_down(WindowKey::S),
        WindowInput::key_up(WindowKey::S),
        WindowInput::key_down(WindowKey::A),
        WindowInput::key_up(WindowKey::A),
        WindowInput::key_down(WindowKey::D),
        WindowInput::key_up(WindowKey::D),
        WindowInput::key_down(WindowKey::K),
        WindowInput::key_up(WindowKey::K),
        WindowInput::key_down(WindowKey::Other),
        WindowInput::key_up(WindowKey::Other),
        WindowInput::focus_lost(),
    ]
}

fn push_key_press(steps: &mut Vec<DemoSceneStep>, key: WindowKey) {
    steps.push(DemoSceneStep::WindowInput(WindowInput::key_down(key)));
    steps.push(DemoSceneStep::WindowInput(WindowInput::key_up(key)));
}

fn push_reversible_adjustments(steps: &mut Vec<DemoSceneStep>) {
    steps.push(DemoSceneStep::WindowInput(WindowInput::key_down(
        WindowKey::K,
    )));
    // D/A are the fine positive/negative pair; W/S are the coarse pair.
    push_key_press(steps, WindowKey::D);
    push_key_press(steps, WindowKey::A);
    push_key_press(steps, WindowKey::W);
    push_key_press(steps, WindowKey::S);
    steps.push(DemoSceneStep::WindowInput(WindowInput::key_up(
        WindowKey::K,
    )));
}

fn push_boundary_probe(steps: &mut Vec<DemoSceneStep>) {
    push_checkpoint(steps, DemoCheckpoint::new("boundary.start"));
    steps.push(DemoSceneStep::WindowInput(WindowInput::key_down(
        WindowKey::K,
    )));

    // Fixture Patch gain starts at 0 dB. One coarse increase reaches 6 dB and
    // the second proves an unchanged upper-bound rejection.
    push_key_press(steps, WindowKey::W);
    push_key_press(steps, WindowKey::W);
    push_checkpoint(
        steps,
        DemoCheckpoint::after_rejection(
            "boundary.patchGain.upper",
            EventRejection::ParameterAtBoundary,
        ),
    );

    // Return to 0 dB, reach -60 dB in ten deterministic coarse steps, reject
    // one further decrement, then return exactly to the fixture baseline.
    push_key_press(steps, WindowKey::S);
    for _ in 0..10 {
        push_key_press(steps, WindowKey::S);
    }
    push_key_press(steps, WindowKey::S);
    push_checkpoint(
        steps,
        DemoCheckpoint::after_rejection(
            "boundary.patchGain.lower",
            EventRejection::ParameterAtBoundary,
        ),
    );
    for _ in 0..10 {
        push_key_press(steps, WindowKey::W);
    }

    steps.push(DemoSceneStep::WindowInput(WindowInput::key_up(
        WindowKey::K,
    )));
    push_checkpoint(steps, DemoCheckpoint::new("boundary.restored"));
}

fn push_checkpoint(steps: &mut Vec<DemoSceneStep>, checkpoint: DemoCheckpoint) {
    steps.push(DemoSceneStep::Checkpoint(checkpoint));
}

fn midi_messages(channel: MidiChannel) -> [MidiMessage; 7] {
    [
        midi_message(channel, MidiMessageKind::NoteOn, 60, 96),
        midi_message(channel, MidiMessageKind::NoteOff, 60, 0),
        midi_message(channel, MidiMessageKind::ControlChange, 1, 64),
        midi_message(channel, MidiMessageKind::ProgramChange, 10, 0),
        midi_message(channel, MidiMessageKind::ChannelPressure, 70, 0),
        midi_message(channel, MidiMessageKind::PitchBend, 0, 64),
        MidiMessage::all_notes_off(channel),
    ]
}

fn midi_message(channel: MidiChannel, kind: MidiMessageKind, data1: u8, data2: u8) -> MidiMessage {
    MidiMessage::try_new(channel, kind, data1, data2)
        .expect("the declared demo MIDI bytes are all seven-bit values")
}

fn first_unknown_patch_id(patches: &[Patch]) -> PatchId {
    let candidate = (1..=u32::MAX)
        .find(|candidate| !patches.iter().any(|patch| patch.id().value() == *candidate))
        .expect("a bounded installed Patch list cannot occupy every PatchId");
    PatchId::new(candidate).expect("the candidate search starts at one")
}

fn build_expected_coverage(patches: &[Patch]) -> Vec<String> {
    let sample_message = midi_messages(patches[0].channel())[0];
    let representative_events = [
        AppEvent::InstallPatches(Vec::new()),
        AppEvent::Navigate(Direction::Up),
        AppEvent::Adjust(Direction::Up),
        AppEvent::Midi {
            patch_id: patches[0].id(),
            message: sample_message,
        },
    ];

    let mut expected = representative_events
        .iter()
        .map(|event| format!("event.{}", event_identifier(event)))
        .collect::<Vec<_>>();

    expected.extend(
        complete_window_vocabulary()
            .iter()
            .map(|input| format!("input.{}", window_input_identifier(*input))),
    );

    expected.extend(
        [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
        .into_iter()
        .map(|direction| format!("direction.{}", direction_identifier(direction))),
    );

    expected.extend(
        midi_messages(patches[0].channel())
            .into_iter()
            .map(|message| format!("midi.{}", midi_kind_identifier(message.kind()))),
    );

    for patch in patches {
        for parameter in PATCH_PARAMETERS {
            expected.push(format!(
                "parameter.patch.{}.{parameter}",
                patch.id().value()
            ));
            expected.push(format!(
                "effect.parameterSnapshot.patch.{}.{parameter}",
                patch.id().value()
            ));
            expected.push(format!(
                "property.stateTree.patch.{}.parameters.{parameter}",
                patch.id().value()
            ));
            expected.push(format!(
                "property.stateTree.parameters.patch.{}.{parameter}",
                patch.id().value()
            ));
        }
        for property in [
            "id",
            "name",
            "channel",
            "instrument.bank",
            "instrument.program",
            "instrument.percussion",
        ] {
            expected.push(format!(
                "property.stateTree.patch.{}.{property}",
                patch.id().value()
            ));
        }
    }

    for parameter in GLOBAL_PARAMETERS {
        expected.push(format!("parameter.global.{parameter}"));
        expected.push(format!("effect.parameterSnapshot.global.{parameter}"));
        expected.push(format!("property.stateTree.global.{parameter}"));
        expected.push(format!("property.stateTree.parameters.global.{parameter}"));
    }

    expected.extend(
        SERIALIZED_PROPERTIES
            .into_iter()
            .map(|property| format!("property.{property}")),
    );
    expected.extend(
        [
            EventRejection::InstallationClosed,
            EventRejection::UnknownPatch,
            EventRejection::ParameterAtBoundary,
        ]
        .into_iter()
        .map(|rejection| format!("rejection.{}", rejection_identifier(rejection))),
    );
    expected.extend(EMITTED_EFFECTS.into_iter().map(str::to_owned));

    expected.sort_unstable();
    expected.dedup();
    expected
}

const SERIALIZED_PROPERTIES: [&str; 37] = [
    "eventLog.schemaVersion",
    "eventLog.totalObserved",
    "eventLog.droppedRecords",
    "eventLog.records",
    "eventLog.coverage.expected",
    "eventLog.coverage.exercised",
    "eventLog.coverage.missing",
    "eventRecord.sequence",
    "eventRecord.source",
    "eventRecord.input",
    "eventRecord.outcome",
    "eventRecord.rejection",
    "eventRecord.generationBefore",
    "eventRecord.generationAfter",
    "eventRecord.stateHashBefore",
    "eventRecord.stateHashAfter",
    "eventRecord.emittedEvents",
    "eventRecord.parameterGeneration",
    "eventRecord.selectedLine",
    "eventRecord.projectionStateHash",
    "stateTree.schemaVersion",
    "stateTree.generation",
    "stateTree.patches",
    "stateTree.global",
    "stateTree.selection.section",
    "stateTree.selection.patchIndex",
    "stateTree.selection.parameterIndex",
    "stateTree.projection.body",
    "stateTree.projection.selectedLine",
    "stateTree.projection.stateHash",
    "stateTree.parameters.generation",
    "stateTree.parameters.patchCount",
    "stateTree.parameters.patches",
    "stateTree.parameters.global",
    "textProjection.body",
    "textProjection.selectedLine",
    "textProjection.stateHash",
];

const EMITTED_EFFECTS: [&str; 4] = [
    "effect.emitted.stateAccepted",
    "effect.emitted.parameterSnapshotPublished",
    "effect.emitted.audioCommand.patchMidi",
    "effect.emitted.audioCommand.allNotesOff",
];

fn event_identifier(event: &AppEvent) -> &'static str {
    match event {
        AppEvent::Navigate(_) => "navigate",
        AppEvent::Adjust(_) => "adjust",
        AppEvent::InstallPatches(_) => "installPatches",
        AppEvent::Midi { .. } => "midi",
    }
}

fn direction_identifier(direction: Direction) -> &'static str {
    match direction {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Left => "left",
        Direction::Right => "right",
    }
}

fn midi_kind_identifier(kind: MidiMessageKind) -> &'static str {
    match kind {
        MidiMessageKind::NoteOn => "noteOn",
        MidiMessageKind::NoteOff => "noteOff",
        MidiMessageKind::ControlChange => "controlChange",
        MidiMessageKind::ProgramChange => "programChange",
        MidiMessageKind::ChannelPressure => "channelPressure",
        MidiMessageKind::PitchBend => "pitchBend",
        MidiMessageKind::AllNotesOff => "allNotesOff",
    }
}

fn window_input_identifier(input: WindowInput) -> &'static str {
    match (input.kind(), input.key()) {
        (WindowInputKind::KeyDown, WindowKey::W) => "keyDown.w",
        (WindowInputKind::KeyDown, WindowKey::S) => "keyDown.s",
        (WindowInputKind::KeyDown, WindowKey::A) => "keyDown.a",
        (WindowInputKind::KeyDown, WindowKey::D) => "keyDown.d",
        (WindowInputKind::KeyDown, WindowKey::K) => "keyDown.k",
        (WindowInputKind::KeyDown, WindowKey::Other) => "keyDown.other",
        (WindowInputKind::KeyUp, WindowKey::W) => "keyUp.w",
        (WindowInputKind::KeyUp, WindowKey::S) => "keyUp.s",
        (WindowInputKind::KeyUp, WindowKey::A) => "keyUp.a",
        (WindowInputKind::KeyUp, WindowKey::D) => "keyUp.d",
        (WindowInputKind::KeyUp, WindowKey::K) => "keyUp.k",
        (WindowInputKind::KeyUp, WindowKey::Other) => "keyUp.other",
        (WindowInputKind::FocusLost, _) => "focusLost",
    }
}

fn rejection_identifier(rejection: EventRejection) -> &'static str {
    match rejection {
        EventRejection::InstallationClosed => "installationClosed",
        EventRejection::TooManyPatches => "tooManyPatches",
        EventRejection::DuplicateMidiChannel => "duplicateMidiChannel",
        EventRejection::NoPatchesInstalled => "noPatchesInstalled",
        EventRejection::UnknownPatch => "unknownPatch",
        EventRejection::InvalidSelection => "invalidSelection",
        EventRejection::ParameterAtBoundary => "parameterAtBoundary",
        EventRejection::InvalidParameterValue => "invalidParameterValue",
        EventRejection::GenerationOverflow => "generationOverflow",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DemoScene, DemoSceneError, DemoSceneStep, MidiProbe, GLOBAL_PARAMETERS, PATCH_PARAMETERS,
    };
    use crate::control::app_state::EventRejection;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::MidiMessageKind;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::shell::window_input::{WindowInput, WindowInputKind, WindowKey};
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_instrument::SoundFontInstrument;

    fn patch(id: u32, channel: u8) -> Patch {
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            SoundFontInstrument::new(0, id as u8, false).unwrap(),
            MidiChannel::new(channel).unwrap(),
            ChannelParameters::default(),
        )
    }

    fn patches() -> Vec<Patch> {
        vec![patch(1, 2), patch(7, 9)]
    }

    #[test]
    fn requires_discriminating_multi_patch_input() {
        assert_eq!(
            DemoScene::exhaustive(&[]),
            Err(DemoSceneError::InsufficientPatches { actual: 0 })
        );
        assert_eq!(
            DemoScene::exhaustive(&[patch(1, 0)]),
            Err(DemoSceneError::InsufficientPatches { actual: 1 })
        );
    }

    #[test]
    fn derives_patch_specific_parameters_and_midi_from_the_fixture() {
        let patches = patches();
        let scene = DemoScene::exhaustive(&patches).unwrap();

        for patch in &patches {
            for parameter in PATCH_PARAMETERS {
                let identifier = format!("parameter.patch.{}.{parameter}", patch.id().value());
                assert!(scene.expected_coverage().contains(&identifier));
            }
        }
        for parameter in GLOBAL_PARAMETERS {
            assert!(scene
                .expected_coverage()
                .contains(&format!("parameter.global.{parameter}")));
        }

        let probes = scene
            .steps()
            .iter()
            .filter_map(|step| match step {
                DemoSceneStep::MidiProbe(probe) => Some(*probe),
                _ => None,
            })
            .collect::<Vec<MidiProbe>>();
        for patch in &patches {
            let kinds = probes
                .iter()
                .filter(|probe| probe.patch_id() == patch.id())
                .map(|probe| probe.message().kind())
                .collect::<Vec<_>>();
            assert_eq!(
                kinds,
                vec![
                    MidiMessageKind::NoteOn,
                    MidiMessageKind::NoteOff,
                    MidiMessageKind::ControlChange,
                    MidiMessageKind::ProgramChange,
                    MidiMessageKind::ChannelPressure,
                    MidiMessageKind::PitchBend,
                    MidiMessageKind::AllNotesOff,
                ]
            );
        }
        assert_eq!(
            probes
                .iter()
                .filter(|probe| {
                    probe.expected_rejection() == Some(EventRejection::UnknownPatch)
                })
                .count(),
            1
        );
    }

    #[test]
    fn includes_every_normalized_window_input_and_focus_reset() {
        let scene = DemoScene::exhaustive(&patches()).unwrap();
        let inputs = scene
            .steps()
            .iter()
            .filter_map(|step| match step {
                DemoSceneStep::WindowInput(input) => Some(*input),
                _ => None,
            })
            .collect::<Vec<WindowInput>>();

        for key in [
            WindowKey::W,
            WindowKey::S,
            WindowKey::A,
            WindowKey::D,
            WindowKey::K,
            WindowKey::Other,
        ] {
            assert!(inputs.contains(&WindowInput::key_down(key)));
            assert!(inputs.contains(&WindowInput::key_up(key)));
        }
        assert!(inputs.contains(&WindowInput::focus_lost()));
        assert!(inputs.iter().any(|input| {
            input.kind() == WindowInputKind::FocusLost && input.key() == WindowKey::Other
        }));
    }

    #[test]
    fn boundary_checkpoints_name_nonfatal_rejections_and_restoration() {
        let scene = DemoScene::exhaustive(&patches()).unwrap();
        let checkpoints = scene
            .steps()
            .iter()
            .filter_map(|step| match step {
                DemoSceneStep::Checkpoint(checkpoint) => Some(checkpoint),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(checkpoints.iter().any(|checkpoint| {
            checkpoint.name() == "boundary.patchGain.upper"
                && checkpoint.expected_last_rejection() == Some(EventRejection::ParameterAtBoundary)
        }));
        assert!(checkpoints.iter().any(|checkpoint| {
            checkpoint.name() == "boundary.patchGain.lower"
                && checkpoint.expected_last_rejection() == Some(EventRejection::ParameterAtBoundary)
        }));
        assert!(checkpoints
            .iter()
            .any(|checkpoint| checkpoint.name() == "boundary.restored"));
    }

    #[test]
    fn construction_and_coverage_order_are_deterministic() {
        let patches = patches();
        let first = DemoScene::exhaustive(&patches).unwrap();
        let second = DemoScene::exhaustive(&patches).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.name(), DemoScene::NAME);
        assert_eq!(first.schema_version(), DemoScene::SCHEMA_VERSION);
        assert!(first.event_log_capacity() > first.steps().len());
        assert!(first
            .expected_coverage()
            .windows(2)
            .all(|pair| pair[0] < pair[1]));
        assert!(first
            .steps()
            .iter()
            .any(|step| matches!(step, DemoSceneStep::Tick(duration) if !duration.is_zero())));
    }
}
