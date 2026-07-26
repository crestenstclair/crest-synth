use crate::control::app_event::{AppEvent, Direction};
use crate::control::app_state::EventRejection;
use crate::control::event_log::EventLog;
use crate::control::event_record::EventRecord;
use crate::control::state_tree::StateTree;
use crate::control::text_projection::TextProjection;
use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::patch_id::PatchId;
use crate::mixer::global_parameters::GlobalParameters;
use crate::real_time::audio_command::AudioCommand;
use crate::shell::window_input::{WindowInput, WindowInputKind, WindowKey};
use crate::synth::instrument_capability::{
    CapabilityDescriptor, CapabilityRegistry, ParameterKind,
};
use crate::synth::patch::{Patch, PatchEditableTarget};
use core::fmt;
use std::collections::BTreeSet;
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
    pub const SCHEMA_VERSION: u32 = 2;

    /// Derives the complete current scene from the accepted fixture Patch list.
    ///
    /// At least two Patches are required because routing and mixer observations
    /// must discriminate the edited Patch from an unaffected Patch.
    pub fn exhaustive(
        capabilities: &CapabilityRegistry,
        installed_patches: &[Patch],
        global_parameters: &GlobalParameters,
    ) -> Result<Self, DemoSceneError> {
        if installed_patches.len() < 2 {
            return Err(DemoSceneError::InsufficientPatches {
                actual: installed_patches.len(),
            });
        }
        if installed_patches.iter().any(|patch| {
            capabilities
                .validate_config(patch.instrument_config())
                .is_err()
        }) {
            return Err(DemoSceneError::InvalidInstrumentConfig);
        }

        Ok(Self {
            name: Self::NAME.to_owned(),
            schema_version: Self::SCHEMA_VERSION,
            steps: build_steps(capabilities, installed_patches, global_parameters),
            expected_coverage: build_expected_coverage(capabilities, installed_patches),
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
    InvalidInstrumentConfig,
}

impl fmt::Display for DemoSceneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InsufficientPatches { actual } => write!(
                formatter,
                "the exhaustive demo requires at least two installed Patches, got {actual}"
            ),
            Self::InvalidInstrumentConfig => {
                formatter.write_str("installed Patch config does not match the capability registry")
            }
        }
    }
}

impl std::error::Error for DemoSceneError {}

fn build_steps(
    capabilities: &CapabilityRegistry,
    patches: &[Patch],
    global_parameters: &GlobalParameters,
) -> Vec<DemoSceneStep> {
    let mut steps = Vec::new();
    let mut boundary_probed = BTreeSet::new();
    push_checkpoint(&mut steps, DemoCheckpoint::new("scene.start"));

    // Exercise the read-only PATCH context before the exhaustive input sweep.
    // Both rejected actions leave its page and generation unchanged, and the
    // direct MIXER key proves the next semantic event is still accepted.
    push_key_press(&mut steps, WindowKey::Digit2);
    push_key_press(&mut steps, WindowKey::W);
    push_checkpoint(
        &mut steps,
        DemoCheckpoint::after_rejection(
            "context.patch.navigateRejected",
            EventRejection::ActionUnavailableInContext,
        ),
    );
    steps.push(DemoSceneStep::WindowInput(WindowInput::key_down(
        WindowKey::K,
    )));
    push_key_press(&mut steps, WindowKey::D);
    steps.push(DemoSceneStep::WindowInput(WindowInput::key_up(
        WindowKey::K,
    )));
    push_checkpoint(
        &mut steps,
        DemoCheckpoint::after_rejection(
            "context.patch.adjustRejected",
            EventRejection::ActionUnavailableInContext,
        ),
    );
    push_key_press(&mut steps, WindowKey::Digit1);
    push_checkpoint(&mut steps, DemoCheckpoint::new("context.mixer.recovered"));

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

    for patch in patches {
        let descriptor = capabilities
            .descriptor(patch.instrument_config().capability_id())
            .expect("validated scene Patch capability is installed");
        let targets = patch
            .editable_targets(descriptor)
            .expect("validated scene Patch has a canonical editable surface");
        for target in targets {
            let metadata = patch_target_metadata(patch, descriptor, &target)
                .expect("every editable target has bounded adjustment metadata");
            if boundary_probed.insert(target.name().to_owned()) {
                push_parameter_boundary_probe(
                    &mut steps,
                    &format!("patch.{}.{}", patch.id().value(), target.name()),
                    metadata.0,
                    metadata.1,
                    metadata.2,
                    metadata.3,
                    metadata.4,
                );
            }
            push_reversible_adjustments(&mut steps, metadata.0, metadata.1, metadata.2, metadata.4);
            push_checkpoint(
                &mut steps,
                DemoCheckpoint::new(format!(
                    "patch.{}.parameter.{}",
                    patch.id().value(),
                    target.name()
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
        push_reversible_adjustments(
            &mut steps,
            global_parameters.value(parameter),
            descriptor.minimum(),
            descriptor.maximum(),
            descriptor.coarse_step(),
        );
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
        let descriptor = capabilities
            .descriptor(patch.instrument_config().capability_id())
            .expect("validated MIDI Patch capability is installed");
        for message in midi_messages(patch.channel(), descriptor.supported_midi_kinds()) {
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

fn push_reversible_adjustments(
    steps: &mut Vec<DemoSceneStep>,
    initial: f32,
    minimum: f32,
    maximum: f32,
    coarse_step: f32,
) {
    steps.push(DemoSceneStep::WindowInput(WindowInput::key_down(
        WindowKey::K,
    )));
    let can_increase = initial + coarse_step <= maximum || initial <= minimum;
    let (fine_first, fine_restore, coarse_first, coarse_restore) = if can_increase {
        (WindowKey::D, WindowKey::A, WindowKey::W, WindowKey::S)
    } else {
        (WindowKey::A, WindowKey::D, WindowKey::S, WindowKey::W)
    };
    push_key_press(steps, fine_first);
    push_key_press(steps, fine_restore);
    push_key_press(steps, coarse_first);
    push_key_press(steps, coarse_restore);
    steps.push(DemoSceneStep::WindowInput(WindowInput::key_up(
        WindowKey::K,
    )));
}

fn patch_target_metadata(
    patch: &Patch,
    descriptor: &CapabilityDescriptor,
    target: &PatchEditableTarget,
) -> Option<(f32, f32, f32, f32, f32)> {
    match target {
        PatchEditableTarget::Mixer(parameter) => {
            let metadata = parameter.descriptor();
            Some((
                patch.parameters().value(*parameter),
                metadata.minimum(),
                metadata.maximum(),
                metadata.fine_step(),
                metadata.coarse_step(),
            ))
        }
        PatchEditableTarget::Envelope(parameter) => {
            let metadata = parameter.descriptor();
            Some((
                patch.envelope().value(*parameter),
                metadata.minimum(),
                metadata.maximum(),
                metadata.fine_step(),
                metadata.coarse_step(),
            ))
        }
        PatchEditableTarget::Instrument(parameter_id) => {
            let spec = descriptor.parameter(parameter_id)?;
            let value = patch.instrument_config().value(parameter_id)?;
            let initial = spec.scalar_value(value).ok()?;
            match spec.kind() {
                ParameterKind::Continuous | ParameterKind::Stepped => {
                    let range = spec.range()?;
                    Some((
                        initial,
                        range.minimum() as f32,
                        range.maximum() as f32,
                        spec.fine_step()? as f32,
                        spec.coarse_step()? as f32,
                    ))
                }
                ParameterKind::Choice => Some((
                    initial,
                    0.0,
                    spec.choices().len().saturating_sub(1) as f32,
                    1.0,
                    1.0,
                )),
                ParameterKind::Toggle => Some((initial, 0.0, 1.0, 1.0, 1.0)),
                ParameterKind::Asset => None,
            }
        }
    }
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

fn midi_messages(channel: MidiChannel, kinds: &[MidiMessageKind]) -> Vec<MidiMessage> {
    kinds
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

fn build_expected_coverage(capabilities: &CapabilityRegistry, patches: &[Patch]) -> Vec<String> {
    let mut expected = Vec::new();
    for descriptor in AppEvent::surface_descriptor() {
        match descriptor {
            crate::control::app_event::AppEventSurfaceDescriptor::SelectContext { context } => {
                expected.push("event.selectContext".to_owned());
                expected.push(format!("context.{}", context.label().to_ascii_lowercase()));
            }
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

    for patch in patches {
        let descriptor = capabilities
            .descriptor(patch.instrument_config().capability_id())
            .expect("validated MIDI coverage capability is installed");
        expected.extend(
            midi_messages(patch.channel(), descriptor.supported_midi_kinds())
                .into_iter()
                .map(|message| format!("midi.{}", midi_kind_identifier(message.kind()))),
        );
    }

    for patch in patches {
        let descriptor = capabilities
            .descriptor(patch.instrument_config().capability_id())
            .expect("validated coverage Patch capability is installed");
        let targets = patch
            .editable_targets(descriptor)
            .expect("validated coverage Patch has a canonical editable surface");
        for target in targets {
            let parameter = target.name();
            expected.push(format!(
                "parameter.patch.{}.{parameter}",
                patch.id().value()
            ));
            expected.push(format!(
                "effect.parameterSnapshot.patch.{}.{parameter}",
                patch.id().value()
            ));
            match target {
                PatchEditableTarget::Mixer(_) => {
                    expected.push(format!(
                        "property.stateTree.patch.{}.parameters.{parameter}",
                        patch.id().value()
                    ));
                    expected.push(format!(
                        "property.stateTree.parameters.patch.{}.{parameter}",
                        patch.id().value()
                    ));
                }
                PatchEditableTarget::Envelope(_) => {
                    expected.push(format!(
                        "property.stateTree.patch.{}.envelope.{parameter}",
                        patch.id().value()
                    ));
                    expected.push(format!(
                        "property.stateTree.parameters.patch.{}.envelope.{parameter}",
                        patch.id().value()
                    ));
                }
                PatchEditableTarget::Instrument(_) => {
                    expected.push(format!(
                        "property.stateTree.parameters.patch.{}.instrument.count",
                        patch.id().value()
                    ));
                    expected.push(format!(
                        "property.stateTree.parameters.patch.{}.instrument.values",
                        patch.id().value()
                    ));
                }
            }
        }
        for property in ["id", "name", "channel", "instrument.capabilityId"] {
            expected.push(format!(
                "property.stateTree.patch.{}.{property}",
                patch.id().value()
            ));
        }
        for assignment in patch.instrument_config().values() {
            for property in ["parameterId", "value.kind", "value.value"] {
                expected.push(format!(
                    "property.stateTree.patch.{}.instrument.value.{}.{property}",
                    patch.id().value(),
                    assignment.parameter_id()
                ));
            }
        }
        for assignment in patch.instrument_config().asset_references() {
            for property in ["parameterId", "reference.kind", "reference.locator"] {
                expected.push(format!(
                    "property.stateTree.patch.{}.instrument.asset.{}.{property}",
                    patch.id().value(),
                    assignment.parameter_id()
                ));
            }
        }
    }

    for descriptor in capabilities.descriptors() {
        for property in [
            "id",
            "label",
            "semanticAccent",
            "voicePolicy",
            "supportedMidiKinds",
        ] {
            expected.push(format!(
                "property.stateTree.capability.{}.{property}",
                descriptor.id()
            ));
        }
        for section in descriptor.sections() {
            for property in ["id", "label"] {
                expected.push(format!(
                    "property.stateTree.capability.{}.section.{}.{property}",
                    descriptor.id(),
                    section.id()
                ));
            }
            for parameter in section.parameters() {
                for property in [
                    "id",
                    "label",
                    "kind",
                    "update",
                    "defaultValue",
                    "range",
                    "choices",
                    "fineStep",
                    "coarseStep",
                    "unit",
                    "formatter",
                    "enabledWhen",
                    "visibleWhen",
                ] {
                    expected.push(format!(
                        "property.stateTree.capability.{}.parameter.{}.{property}",
                        descriptor.id(),
                        parameter.id()
                    ));
                }
            }
        }
        for requirement in descriptor.asset_requirements() {
            for property in ["parameterId", "required"] {
                expected.push(format!(
                    "property.stateTree.capability.{}.asset.{}.{property}",
                    descriptor.id(),
                    requirement.parameter_id()
                ));
            }
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
        (WindowInputKind::KeyDown, WindowKey::Digit1) => "keyDown.digit1",
        (WindowInputKind::KeyDown, WindowKey::Digit2) => "keyDown.digit2",
        (WindowInputKind::KeyDown, WindowKey::W) => "keyDown.w",
        (WindowInputKind::KeyDown, WindowKey::S) => "keyDown.s",
        (WindowInputKind::KeyDown, WindowKey::A) => "keyDown.a",
        (WindowInputKind::KeyDown, WindowKey::D) => "keyDown.d",
        (WindowInputKind::KeyDown, WindowKey::K) => "keyDown.k",
        (WindowInputKind::KeyDown, WindowKey::Other) => "keyDown.other",
        (WindowInputKind::KeyUp, WindowKey::Digit1) => "keyUp.digit1",
        (WindowInputKind::KeyUp, WindowKey::Digit2) => "keyUp.digit2",
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
    use crate::adapter::hidef_soundfont_capability::HiDefSoundFontCapability;
    use crate::control::app_state::EventRejection;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::MidiMessageKind;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::shell::window_input::{WindowInput, WindowInputKind, WindowKey};
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::voice_envelope::VoiceEnvelope;
    use crate::testing::automatic_midi_test::create_soundfont_config;

    fn patch(id: u32, channel: u8) -> Patch {
        let provider = HiDefSoundFontCapability::new().unwrap();
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            create_soundfont_config(
                &provider,
                SoundFontInstrument::new(0, id as u8, false).unwrap(),
            )
            .unwrap(),
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

    fn registry() -> crate::synth::CapabilityRegistry {
        HiDefSoundFontCapability::new().unwrap().registry().unwrap()
    }

    #[test]
    fn requires_discriminating_multi_patch_input() {
        assert_eq!(
            DemoScene::exhaustive(&registry(), &[], &globals()),
            Err(DemoSceneError::InsufficientPatches { actual: 0 })
        );
        assert_eq!(
            DemoScene::exhaustive(&registry(), &[patch(1, 0)], &globals()),
            Err(DemoSceneError::InsufficientPatches { actual: 1 })
        );
    }

    #[test]
    fn derives_patch_specific_parameters_and_midi_from_the_fixture() {
        let patches = patches();
        let scene = DemoScene::exhaustive(&registry(), &patches, &globals()).unwrap();

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
        let scene = DemoScene::exhaustive(&registry(), &patches, &globals()).unwrap();
        assert!(scene
            .expected_coverage()
            .windows(2)
            .all(|pair| pair[0] < pair[1]));
        assert_eq!(WindowInput::surface_descriptor().len(), 17);
        assert_eq!(
            crate::control::app_event::AppEvent::surface_descriptor().len(),
            12
        );
        assert_eq!(
            crate::kernel::midi_message::MidiMessageKind::surface_descriptor().len(),
            7
        );
        assert_eq!(ChannelParameters::surface_descriptor().len(), 4);
        assert_eq!(GlobalParameters::surface_descriptor().len(), 7);
        assert_eq!(EventRejection::surface_descriptor().len(), 11);
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
        let scene = DemoScene::exhaustive(&registry(), &patches(), &globals()).unwrap();
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
        let scene = DemoScene::exhaustive(&registry(), &patches(), &globals()).unwrap();
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
        let bounded_parameter_count = ChannelParameters::surface_descriptor().len()
            + VoiceEnvelope::surface_descriptor().len()
            + GlobalParameters::surface_descriptor().len();
        assert_eq!(boundary_rejections, bounded_parameter_count * 2);
        assert_eq!(
            checkpoints
                .iter()
                .filter(|checkpoint| checkpoint.name().starts_with("boundary.")
                    && checkpoint.name().ends_with(".restored"))
                .count(),
            bounded_parameter_count
        );
    }

    #[test]
    fn construction_and_coverage_order_are_deterministic() {
        let patches = patches();
        let first = DemoScene::exhaustive(&registry(), &patches, &globals()).unwrap();
        let second = DemoScene::exhaustive(&registry(), &patches, &globals()).unwrap();

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
