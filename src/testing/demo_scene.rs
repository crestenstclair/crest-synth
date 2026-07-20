use crate::control::app_event::{AppEvent, Direction};
use crate::control::app_state::EventRejection;
use crate::control::event_log::EventLog;
use crate::control::event_record::EventRecord;
use crate::control::state_tree::StateTree;
use crate::control::text_projection::TextProjection;
use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::patch_id::PatchId;
use crate::mixer::channel_parameters::ChannelParameters;
use crate::mixer::global_parameters::GlobalParameters;
use crate::real_time::audio_command::AudioCommand;
use crate::shell::window_input::{WindowInput, WindowInputKind, WindowKey};
use crate::synth::patch::Patch;
use core::fmt;
use std::time::Duration;

const TICK_DURATION: Duration = Duration::from_millis(10);
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
    AudioCommandProbe(AudioCommand),
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
    pub fn exhaustive(
        installed_patches: &[Patch],
        global_parameters: &GlobalParameters,
    ) -> Result<Self, DemoSceneError> {
        if installed_patches.len() < 2 {
            return Err(DemoSceneError::InsufficientPatches {
                actual: installed_patches.len(),
            });
        }

        Ok(Self {
            name: Self::NAME.to_owned(),
            schema_version: Self::SCHEMA_VERSION,
            steps: build_steps(installed_patches, global_parameters),
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

fn build_steps(patches: &[Patch], global_parameters: &GlobalParameters) -> Vec<DemoSceneStep> {
    let mut steps = Vec::new();
    push_checkpoint(&mut steps, DemoCheckpoint::new("scene.start"));

    for input in complete_window_vocabulary() {
        steps.push(DemoSceneStep::WindowInput(*input));
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

    for (patch_index, patch) in patches.iter().enumerate() {
        for descriptor in ChannelParameters::surface_descriptor() {
            let parameter = descriptor.parameter();
            if patch_index == 0 {
                push_parameter_boundary_probe(
                    &mut steps,
                    &format!("patch.{}.{}", patch.id().value(), descriptor.name()),
                    patch.parameters().value(parameter),
                    descriptor.minimum(),
                    descriptor.maximum(),
                    descriptor.fine_step(),
                    descriptor.coarse_step(),
                );
            }
            push_reversible_adjustments(&mut steps);
            push_checkpoint(
                &mut steps,
                DemoCheckpoint::new(format!(
                    "patch.{}.parameter.{}",
                    patch.id().value(),
                    descriptor.name()
                )),
            );
            push_key_press(&mut steps, WindowKey::S);
        }
        push_key_press(&mut steps, WindowKey::D);
    }

    for descriptor in GlobalParameters::surface_descriptor() {
        let parameter = descriptor.parameter();
        push_parameter_boundary_probe(
            &mut steps,
            &format!("global.{}", descriptor.name()),
            global_parameters.value(parameter),
            descriptor.minimum(),
            descriptor.maximum(),
            descriptor.fine_step(),
            descriptor.coarse_step(),
        );
        push_reversible_adjustments(&mut steps);
        push_checkpoint(
            &mut steps,
            DemoCheckpoint::new(format!("global.parameter.{}", descriptor.name())),
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

    steps.push(DemoSceneStep::AudioCommandProbe(
        AudioCommand::all_notes_off(),
    ));
    steps.push(DemoSceneStep::Tick(TICK_DURATION));
    push_checkpoint(&mut steps, DemoCheckpoint::new("audioCommand.allNotesOff"));

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

fn complete_window_vocabulary() -> &'static [WindowInput] {
    WindowInput::surface_descriptor()
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

#[allow(clippy::too_many_arguments)]
fn push_parameter_boundary_probe(
    steps: &mut Vec<DemoSceneStep>,
    identifier: &str,
    initial: f32,
    minimum: f32,
    maximum: f32,
    fine_step: f32,
    coarse_step: f32,
) {
    push_checkpoint(
        steps,
        DemoCheckpoint::new(format!("boundary.{identifier}.start")),
    );
    steps.push(DemoSceneStep::WindowInput(WindowInput::key_down(
        WindowKey::K,
    )));

    for _ in 0..steps_to_boundary(initial, maximum, coarse_step) {
        push_key_press(steps, WindowKey::W);
    }
    push_key_press(steps, WindowKey::W);
    push_checkpoint(
        steps,
        DemoCheckpoint::after_rejection(
            format!("boundary.{identifier}.upper"),
            EventRejection::ParameterAtBoundary,
        ),
    );

    for _ in 0..steps_to_boundary(maximum, minimum, coarse_step) {
        push_key_press(steps, WindowKey::S);
    }
    push_key_press(steps, WindowKey::S);
    push_checkpoint(
        steps,
        DemoCheckpoint::after_rejection(
            format!("boundary.{identifier}.lower"),
            EventRejection::ParameterAtBoundary,
        ),
    );

    let (coarse_restoration_steps, fine_restoration_steps) =
        restoration_steps(minimum, initial, fine_step, coarse_step);
    for _ in 0..coarse_restoration_steps {
        push_key_press(steps, WindowKey::W);
    }
    for _ in 0..fine_restoration_steps {
        push_key_press(steps, WindowKey::D);
    }

    steps.push(DemoSceneStep::WindowInput(WindowInput::key_up(
        WindowKey::K,
    )));
    push_checkpoint(
        steps,
        DemoCheckpoint::new(format!("boundary.{identifier}.restored")),
    );
}

fn steps_to_boundary(from: f32, to: f32, step: f32) -> usize {
    (((to - from).abs() / step).ceil()) as usize
}

fn restoration_steps(from: f32, to: f32, fine_step: f32, coarse_step: f32) -> (usize, usize) {
    let fine_units = ((to - from).abs() / fine_step).round() as usize;
    let coarse_units = (coarse_step / fine_step).round() as usize;
    (fine_units / coarse_units, fine_units % coarse_units)
}

fn push_checkpoint(steps: &mut Vec<DemoSceneStep>, checkpoint: DemoCheckpoint) {
    steps.push(DemoSceneStep::Checkpoint(checkpoint));
}

fn midi_messages(channel: MidiChannel) -> Vec<MidiMessage> {
    MidiMessageKind::surface_descriptor()
        .iter()
        .map(|kind| match kind {
            MidiMessageKind::NoteOn => midi_message(channel, *kind, 60, 96),
            MidiMessageKind::NoteOff => midi_message(channel, *kind, 60, 0),
            MidiMessageKind::ControlChange => midi_message(channel, *kind, 1, 64),
            MidiMessageKind::ProgramChange => midi_message(channel, *kind, 10, 0),
            MidiMessageKind::ChannelPressure => midi_message(channel, *kind, 70, 0),
            MidiMessageKind::PitchBend => midi_message(channel, *kind, 0, 64),
            MidiMessageKind::AllNotesOff => MidiMessage::all_notes_off(channel),
        })
        .collect()
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
    let mut expected = Vec::new();
    for descriptor in AppEvent::surface_descriptor() {
        match descriptor {
            crate::control::app_event::AppEventSurfaceDescriptor::Navigate { direction } => {
                expected.push("event.navigate".to_owned());
                expected.push(format!("direction.{}", direction_identifier(*direction)));
            }
            crate::control::app_event::AppEventSurfaceDescriptor::Adjust { direction } => {
                expected.push("event.adjust".to_owned());
                expected.push(format!("direction.{}", direction_identifier(*direction)));
            }
            crate::control::app_event::AppEventSurfaceDescriptor::InstallPatches { .. } => {
                expected.push("event.installPatches".to_owned());
            }
            crate::control::app_event::AppEventSurfaceDescriptor::Midi { .. } => {
                expected.push("event.midi".to_owned());
            }
        }
    }

    expected.extend(
        complete_window_vocabulary()
            .iter()
            .map(|input| format!("input.{}", window_input_identifier(*input))),
    );

    expected.extend(
        Direction::ALL
            .into_iter()
            .map(|direction| format!("direction.{}", direction_identifier(direction))),
    );

    expected.extend(
        midi_messages(patches[0].channel())
            .into_iter()
            .map(|message| format!("midi.{}", midi_kind_identifier(message.kind()))),
    );

    for patch in patches {
        for descriptor in ChannelParameters::surface_descriptor() {
            let parameter = descriptor.name();
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

    for descriptor in GlobalParameters::surface_descriptor() {
        let parameter = descriptor.name();
        expected.push(format!("parameter.global.{parameter}"));
        expected.push(format!("effect.parameterSnapshot.global.{parameter}"));
        expected.push(format!("property.stateTree.global.{parameter}"));
        expected.push(format!("property.stateTree.parameters.global.{parameter}"));
    }

    expected.extend(
        EventLog::serialized_property_descriptor()
            .iter()
            .map(|property| format!("property.eventLog.{property}")),
    );
    expected.extend(
        EventRecord::serialized_property_descriptor()
            .iter()
            .map(|property| format!("property.eventRecord.{property}")),
    );
    expected.extend(
        StateTree::serialized_property_descriptor()
            .iter()
            .map(|property| format!("property.stateTree.{property}")),
    );
    expected.extend(
        TextProjection::serialized_property_descriptor()
            .iter()
            .map(|property| format!("property.textProjection.{property}")),
    );
    expected.extend(
        EventRejection::surface_descriptor()
            .iter()
            .map(|descriptor| format!("rejection.{}", descriptor.name())),
    );
    expected.extend(EMITTED_EFFECTS.into_iter().map(str::to_owned));

    expected.sort_unstable();
    expected.dedup();
    expected
}

const EMITTED_EFFECTS: [&str; 4] = [
    "effect.emitted.stateAccepted",
    "effect.emitted.parameterSnapshotPublished",
    "effect.emitted.audioCommand.patchMidi",
    "effect.emitted.audioCommand.allNotesOff",
];

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

#[cfg(test)]
mod tests {
    use super::{DemoScene, DemoSceneError, DemoSceneStep, MidiProbe};
    use crate::control::app_state::EventRejection;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::MidiMessageKind;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
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

    fn globals() -> GlobalParameters {
        GlobalParameters::new(0.0, 0.5, 0.4, 0.35, 250.0, 0.3, 0.25).unwrap()
    }

    #[test]
    fn requires_discriminating_multi_patch_input() {
        assert_eq!(
            DemoScene::exhaustive(&[], &globals()),
            Err(DemoSceneError::InsufficientPatches { actual: 0 })
        );
        assert_eq!(
            DemoScene::exhaustive(&[patch(1, 0)], &globals()),
            Err(DemoSceneError::InsufficientPatches { actual: 1 })
        );
    }

    #[test]
    fn derives_patch_specific_parameters_and_midi_from_the_fixture() {
        let patches = patches();
        let scene = DemoScene::exhaustive(&patches, &globals()).unwrap();

        for patch in &patches {
            for descriptor in ChannelParameters::surface_descriptor() {
                let identifier = format!(
                    "parameter.patch.{}.{}",
                    patch.id().value(),
                    descriptor.name()
                );
                assert!(scene.expected_coverage().contains(&identifier));
            }
        }
        for descriptor in GlobalParameters::surface_descriptor() {
            assert!(scene
                .expected_coverage()
                .contains(&format!("parameter.global.{}", descriptor.name())));
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
    fn schema_derived_current_surface() {
        let patches = patches();
        let scene = DemoScene::exhaustive(&patches, &globals()).unwrap();
        assert!(scene
            .expected_coverage()
            .windows(2)
            .all(|pair| pair[0] < pair[1]));
        assert_eq!(WindowInput::surface_descriptor().len(), 13);
        assert_eq!(
            crate::control::app_event::AppEvent::surface_descriptor().len(),
            10
        );
        assert_eq!(
            crate::kernel::midi_message::MidiMessageKind::surface_descriptor().len(),
            7
        );
        assert_eq!(ChannelParameters::surface_descriptor().len(), 4);
        assert_eq!(GlobalParameters::surface_descriptor().len(), 7);
        assert_eq!(EventRejection::surface_descriptor().len(), 9);
        for patch in &patches {
            for descriptor in ChannelParameters::surface_descriptor() {
                assert!(scene.expected_coverage().contains(&format!(
                    "parameter.patch.{}.{}",
                    patch.id().value(),
                    descriptor.name()
                )));
            }
        }
    }

    #[test]
    fn includes_every_normalized_window_input_and_focus_reset() {
        let scene = DemoScene::exhaustive(&patches(), &globals()).unwrap();
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
        let scene = DemoScene::exhaustive(&patches(), &globals()).unwrap();
        let checkpoints = scene
            .steps()
            .iter()
            .filter_map(|step| match step {
                DemoSceneStep::Checkpoint(checkpoint) => Some(checkpoint),
                _ => None,
            })
            .collect::<Vec<_>>();

        let boundary_rejections = checkpoints
            .iter()
            .filter(|checkpoint| {
                checkpoint.expected_last_rejection() == Some(EventRejection::ParameterAtBoundary)
            })
            .count();
        assert_eq!(boundary_rejections, 22);
        assert_eq!(
            checkpoints
                .iter()
                .filter(|checkpoint| checkpoint.name().starts_with("boundary.")
                    && checkpoint.name().ends_with(".restored"))
                .count(),
            11
        );
    }

    #[test]
    fn construction_and_coverage_order_are_deterministic() {
        let patches = patches();
        let first = DemoScene::exhaustive(&patches, &globals()).unwrap();
        let second = DemoScene::exhaustive(&patches, &globals()).unwrap();

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
