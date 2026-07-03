// path: src/loop/app_event.rs

//! AppEvent: the closed union over every control-plane event.
//!
//! `AppEvent` is the single semantic vocabulary the Loop reducer accepts —
//! normalized MIDI, mapped gamepad actions, editor navigation/edit events,
//! mixer strip changes, patch commands, and preset load/save. It is the
//! only thing that mutates `AppState` (see `aggregate.Loop.AppState`), and
//! because scenes are files, every variant must serialize and deserialize
//! losslessly.
//!
//! The wrapped types (`MidiEvent`, `GamepadAction`, `EditorEvent`,
//! `MixerViewEvent`) do not derive `serde::Serialize`/`Deserialize`
//! themselves, so this module hand-rolls the codec through a private
//! `Wire*` mirror model built only from types that *do* derive serde
//! (primitives, `String`, `Option`). Converting a domain value to its wire
//! form is infallible (every domain value is already valid); converting a
//! wire value back to a domain value re-validates through the type's own
//! constructor and can fail, which `Deserialize` surfaces as a normal
//! deserialization error rather than a panic.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::editor::editor_event::EditorEvent;
use crate::kernel::channel_address::{ChannelAddress, MidiChannel, MidiGroup};
use crate::kernel::midi_event::MidiEvent;
use crate::kernel::midi_event_kind::MidiEventKind;
use crate::kernel::note_id::NoteId;
use crate::kernel::note_number::NoteNumber;
use crate::kernel::velocity::Velocity;
use crate::mixer::mixer_view_event::MixerViewEvent;
use crate::patch::patch::{ChannelMapping, MpeZone, PatchError, PatchId, VoiceConfig};
use crate::preset::preset_storage::PresetId;
use crate::shell::gamepad_action::{Direction, GamepadAction, ShoulderSide, StickAxis};

/// A patch-context command carried by an `AppEvent`.
///
/// Mirrors the command surface `PatchManager` / `Patch` already expose;
/// this enum only carries the identifying data an event needs, not a
/// handle to the manager or repository.
#[derive(Debug, Clone, PartialEq)]
pub enum PatchCommand {
    /// `PatchManager::create_patch`: allocate a new patch with default
    /// voice configuration.
    Create,
    /// `PatchManager::delete_patch`.
    Delete { id: PatchId },
    /// `PatchManager::set_mapping` / `Patch::set_mapping`.
    SetMapping {
        id: PatchId,
        mapping: ChannelMapping,
    },
    /// `PatchManager::set_mpe_zone` / `Patch::set_mpe_zone`.
    SetMpeZone { id: PatchId, zone: Option<MpeZone> },
    /// `PatchManager::assign_sample_set` / `Patch::assign_sample_set`.
    AssignSampleSet {
        id: PatchId,
        sample_set: Option<u32>,
    },
    /// `PatchManager::set_voice_config` / `Patch::set_voice_config`.
    SetVoiceConfig { id: PatchId, voice: VoiceConfig },
}

/// A preset-context command carried by an `AppEvent`.
///
/// Mirrors `SessionManager::save` / `SessionManager::load`: the event
/// carries only the identifying data (and, for `Save`, the display name);
/// the session content itself is the ambient current `AppState`, not
/// something duplicated into the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetCommand {
    /// `SessionManager::save`.
    Save { id: PresetId, name: String },
    /// `SessionManager::load`.
    Load { id: PresetId },
}

/// Closed union over every control-plane event.
///
/// Every variant carries its aggregate's existing command payload:
/// normalized `MidiEvent`, mapped `GamepadAction`, editor `EditorEvent`,
/// mixer `MixerViewEvent`, `PatchCommand`, and `PresetCommand`. This is the
/// only event type the Loop reducer (`aggregate.Loop.AppState::apply`)
/// accepts.
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// A normalized, addressed MIDI event dispatched to matching patches.
    Midi(MidiEvent),
    /// A mapped gamepad action from the shell's controller adapter.
    Gamepad(GamepadAction),
    /// A navigation/edit event for the editor view.
    Editor(EditorEvent),
    /// A navigation/edit event for the mixer view.
    Mixer(MixerViewEvent),
    /// A patch lifecycle/configuration command.
    Patch(PatchCommand),
    /// A preset/session load-or-save command.
    Preset(PresetCommand),
}

/// Error produced when a deserialized `AppEvent` wire value fails to
/// re-validate against the invariants of the domain type it names.
///
/// Rejections are values, matching the rest of the control plane: a
/// malformed scene file never panics the codec, it produces this error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEventCodecError {
    InvalidMidiChannel(u8),
    InvalidMidiGroup(u8),
    InvalidNoteNumber(u8),
    InvalidVelocity(String),
    InvalidMpeZone(String),
    InvalidVoiceConfig(String),
}

impl fmt::Display for AppEventCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppEventCodecError::InvalidMidiChannel(value) => {
                write!(f, "invalid MIDI channel in AppEvent: {value}")
            }
            AppEventCodecError::InvalidMidiGroup(value) => {
                write!(f, "invalid MIDI group in AppEvent: {value}")
            }
            AppEventCodecError::InvalidNoteNumber(value) => {
                write!(f, "invalid note number in AppEvent: {value}")
            }
            AppEventCodecError::InvalidVelocity(reason) => {
                write!(f, "invalid velocity in AppEvent: {reason}")
            }
            AppEventCodecError::InvalidMpeZone(reason) => {
                write!(f, "invalid MPE zone in AppEvent: {reason}")
            }
            AppEventCodecError::InvalidVoiceConfig(reason) => {
                write!(f, "invalid voice config in AppEvent: {reason}")
            }
        }
    }
}

impl Error for AppEventCodecError {}

// ---------------------------------------------------------------------
// Wire model: only primitives, String, Option, and nested Wire types, so
// every one of these can mechanically derive Serialize/Deserialize. The
// From/TryFrom impls below are the only place that ever touches the
// domain types' own (re-)validating constructors.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum WireDirection {
    Up,
    Down,
    Left,
    Right,
}

impl From<Direction> for WireDirection {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::Up => WireDirection::Up,
            Direction::Down => WireDirection::Down,
            Direction::Left => WireDirection::Left,
            Direction::Right => WireDirection::Right,
        }
    }
}

impl From<WireDirection> for Direction {
    fn from(direction: WireDirection) -> Self {
        match direction {
            WireDirection::Up => Direction::Up,
            WireDirection::Down => Direction::Down,
            WireDirection::Left => Direction::Left,
            WireDirection::Right => Direction::Right,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct WireStickAxis {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum WireShoulderSide {
    Left,
    Right,
}

impl From<ShoulderSide> for WireShoulderSide {
    fn from(side: ShoulderSide) -> Self {
        match side {
            ShoulderSide::Left => WireShoulderSide::Left,
            ShoulderSide::Right => WireShoulderSide::Right,
        }
    }
}

impl From<WireShoulderSide> for ShoulderSide {
    fn from(side: WireShoulderSide) -> Self {
        match side {
            WireShoulderSide::Left => ShoulderSide::Left,
            WireShoulderSide::Right => ShoulderSide::Right,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
enum WireGamepadAction {
    Navigate(WireDirection),
    FineAdjust(WireStickAxis),
    Scroll(WireStickAxis),
    Select,
    Back,
    SwitchView(WireShoulderSide),
    SwitchPatch(WireShoulderSide),
    SaveSession,
    OpenBrowser,
}

impl From<&GamepadAction> for WireGamepadAction {
    fn from(action: &GamepadAction) -> Self {
        match *action {
            GamepadAction::Navigate(direction) => WireGamepadAction::Navigate(direction.into()),
            GamepadAction::FineAdjust(axis) => WireGamepadAction::FineAdjust(WireStickAxis {
                x: axis.x(),
                y: axis.y(),
            }),
            GamepadAction::Scroll(axis) => WireGamepadAction::Scroll(WireStickAxis {
                x: axis.x(),
                y: axis.y(),
            }),
            GamepadAction::Select => WireGamepadAction::Select,
            GamepadAction::Back => WireGamepadAction::Back,
            GamepadAction::SwitchView(side) => WireGamepadAction::SwitchView(side.into()),
            GamepadAction::SwitchPatch(side) => WireGamepadAction::SwitchPatch(side.into()),
            GamepadAction::SaveSession => WireGamepadAction::SaveSession,
            GamepadAction::OpenBrowser => WireGamepadAction::OpenBrowser,
        }
    }
}

impl From<WireGamepadAction> for GamepadAction {
    fn from(wire: WireGamepadAction) -> Self {
        match wire {
            WireGamepadAction::Navigate(direction) => GamepadAction::Navigate(direction.into()),
            WireGamepadAction::FineAdjust(axis) => {
                GamepadAction::FineAdjust(StickAxis::new(axis.x, axis.y))
            }
            WireGamepadAction::Scroll(axis) => {
                GamepadAction::Scroll(StickAxis::new(axis.x, axis.y))
            }
            WireGamepadAction::Select => GamepadAction::Select,
            WireGamepadAction::Back => GamepadAction::Back,
            WireGamepadAction::SwitchView(side) => GamepadAction::SwitchView(side.into()),
            WireGamepadAction::SwitchPatch(side) => GamepadAction::SwitchPatch(side.into()),
            WireGamepadAction::SaveSession => GamepadAction::SaveSession,
            WireGamepadAction::OpenBrowser => GamepadAction::OpenBrowser,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum WireEditorEvent {
    NavUp,
    NavDown,
    NavLeft,
    NavRight,
    EnterEditMode,
    ExitEditMode,
}

impl From<EditorEvent> for WireEditorEvent {
    fn from(event: EditorEvent) -> Self {
        match event {
            EditorEvent::NavUp => WireEditorEvent::NavUp,
            EditorEvent::NavDown => WireEditorEvent::NavDown,
            EditorEvent::NavLeft => WireEditorEvent::NavLeft,
            EditorEvent::NavRight => WireEditorEvent::NavRight,
            EditorEvent::EnterEditMode => WireEditorEvent::EnterEditMode,
            EditorEvent::ExitEditMode => WireEditorEvent::ExitEditMode,
        }
    }
}

impl From<WireEditorEvent> for EditorEvent {
    fn from(event: WireEditorEvent) -> Self {
        match event {
            WireEditorEvent::NavUp => EditorEvent::NavUp,
            WireEditorEvent::NavDown => EditorEvent::NavDown,
            WireEditorEvent::NavLeft => EditorEvent::NavLeft,
            WireEditorEvent::NavRight => EditorEvent::NavRight,
            WireEditorEvent::EnterEditMode => EditorEvent::EnterEditMode,
            WireEditorEvent::ExitEditMode => EditorEvent::ExitEditMode,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum WireMixerViewEvent {
    NavUp,
    NavDown,
    NavLeft,
    NavRight,
    EnterEditMode,
    ExitEditMode,
    ToggleFocusedParam,
}

impl From<MixerViewEvent> for WireMixerViewEvent {
    fn from(event: MixerViewEvent) -> Self {
        match event {
            MixerViewEvent::NavUp => WireMixerViewEvent::NavUp,
            MixerViewEvent::NavDown => WireMixerViewEvent::NavDown,
            MixerViewEvent::NavLeft => WireMixerViewEvent::NavLeft,
            MixerViewEvent::NavRight => WireMixerViewEvent::NavRight,
            MixerViewEvent::EnterEditMode => WireMixerViewEvent::EnterEditMode,
            MixerViewEvent::ExitEditMode => WireMixerViewEvent::ExitEditMode,
            MixerViewEvent::ToggleFocusedParam => WireMixerViewEvent::ToggleFocusedParam,
        }
    }
}

impl From<WireMixerViewEvent> for MixerViewEvent {
    fn from(event: WireMixerViewEvent) -> Self {
        match event {
            WireMixerViewEvent::NavUp => MixerViewEvent::NavUp,
            WireMixerViewEvent::NavDown => MixerViewEvent::NavDown,
            WireMixerViewEvent::NavLeft => MixerViewEvent::NavLeft,
            WireMixerViewEvent::NavRight => MixerViewEvent::NavRight,
            WireMixerViewEvent::EnterEditMode => MixerViewEvent::EnterEditMode,
            WireMixerViewEvent::ExitEditMode => MixerViewEvent::ExitEditMode,
            WireMixerViewEvent::ToggleFocusedParam => MixerViewEvent::ToggleFocusedParam,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum WireMidiEventKind {
    NoteOn,
    NoteOff,
    ControlChange,
    PitchBend,
    Aftertouch,
    PolyPressure,
    ProgramChange,
}

impl From<MidiEventKind> for WireMidiEventKind {
    fn from(kind: MidiEventKind) -> Self {
        match kind {
            MidiEventKind::NoteOn => WireMidiEventKind::NoteOn,
            MidiEventKind::NoteOff => WireMidiEventKind::NoteOff,
            MidiEventKind::ControlChange => WireMidiEventKind::ControlChange,
            MidiEventKind::PitchBend => WireMidiEventKind::PitchBend,
            MidiEventKind::Aftertouch => WireMidiEventKind::Aftertouch,
            MidiEventKind::PolyPressure => WireMidiEventKind::PolyPressure,
            MidiEventKind::ProgramChange => WireMidiEventKind::ProgramChange,
        }
    }
}

impl From<WireMidiEventKind> for MidiEventKind {
    fn from(kind: WireMidiEventKind) -> Self {
        match kind {
            WireMidiEventKind::NoteOn => MidiEventKind::NoteOn,
            WireMidiEventKind::NoteOff => MidiEventKind::NoteOff,
            WireMidiEventKind::ControlChange => MidiEventKind::ControlChange,
            WireMidiEventKind::PitchBend => MidiEventKind::PitchBend,
            WireMidiEventKind::Aftertouch => MidiEventKind::Aftertouch,
            WireMidiEventKind::PolyPressure => MidiEventKind::PolyPressure,
            WireMidiEventKind::ProgramChange => MidiEventKind::ProgramChange,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WireMidiEvent {
    channel: u8,
    group: u8,
    kind: WireMidiEventKind,
    note: u8,
    note_id: u32,
    velocity: f64,
}

impl From<&MidiEvent> for WireMidiEvent {
    fn from(event: &MidiEvent) -> Self {
        Self {
            channel: event.address().channel().value(),
            group: event.address().group().value(),
            kind: WireMidiEventKind::from(*event.kind()),
            note: event.note().value(),
            note_id: event.note_id().value(),
            velocity: event.velocity().value(),
        }
    }
}

impl TryFrom<WireMidiEvent> for MidiEvent {
    type Error = AppEventCodecError;

    fn try_from(wire: WireMidiEvent) -> Result<Self, Self::Error> {
        let channel = MidiChannel::try_new(wire.channel)
            .map_err(|_| AppEventCodecError::InvalidMidiChannel(wire.channel))?;
        let group = MidiGroup::try_new(wire.group)
            .map_err(|_| AppEventCodecError::InvalidMidiGroup(wire.group))?;
        let note = NoteNumber::try_new(wire.note)
            .map_err(|_| AppEventCodecError::InvalidNoteNumber(wire.note))?;
        let velocity = Velocity::try_new(wire.velocity)
            .map_err(|err| AppEventCodecError::InvalidVelocity(err.to_string()))?;
        Ok(MidiEvent::new(
            ChannelAddress::new(channel, group),
            wire.kind.into(),
            note,
            NoteId::new(wire.note_id),
            velocity,
        ))
    }
}

/// Reconstructs a `ChannelMapping` from its raw 16-bit channel mask.
///
/// Every bit position 0..16 is itself a valid MIDI channel number, so
/// decomposing the mask into a channel list and rebuilding through
/// `ChannelMapping::from_channels` can never fail.
fn channel_mapping_from_mask(mask: u16) -> ChannelMapping {
    let channels: Vec<u8> = (0u8..16)
        .filter(|&channel| mask & (1u16 << channel) != 0)
        .collect();
    ChannelMapping::from_channels(&channels)
        .expect("mask bit positions 0..16 are always valid MIDI channels")
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct WireMpeZone {
    master_channel: u8,
    member_start: u8,
    member_count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct WireVoiceConfig {
    polyphony: u8,
    attack_ms: f32,
    decay_ms: f32,
    sustain_level: f32,
    release_ms: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum WirePatchCommand {
    Create,
    Delete { id: u32 },
    SetMapping { id: u32, mask: u16 },
    SetMpeZone { id: u32, zone: Option<WireMpeZone> },
    AssignSampleSet { id: u32, sample_set: Option<u32> },
    SetVoiceConfig { id: u32, voice: WireVoiceConfig },
}

impl From<&PatchCommand> for WirePatchCommand {
    fn from(command: &PatchCommand) -> Self {
        match command {
            PatchCommand::Create => WirePatchCommand::Create,
            PatchCommand::Delete { id } => WirePatchCommand::Delete { id: id.value() },
            PatchCommand::SetMapping { id, mapping } => WirePatchCommand::SetMapping {
                id: id.value(),
                mask: mapping.mask(),
            },
            PatchCommand::SetMpeZone { id, zone } => WirePatchCommand::SetMpeZone {
                id: id.value(),
                zone: zone.map(|zone| WireMpeZone {
                    master_channel: zone.master_channel(),
                    member_start: *zone.member_range().start(),
                    member_count: zone.member_range().end() - zone.member_range().start() + 1,
                }),
            },
            PatchCommand::AssignSampleSet { id, sample_set } => WirePatchCommand::AssignSampleSet {
                id: id.value(),
                sample_set: *sample_set,
            },
            PatchCommand::SetVoiceConfig { id, voice } => WirePatchCommand::SetVoiceConfig {
                id: id.value(),
                voice: WireVoiceConfig {
                    polyphony: voice.polyphony(),
                    attack_ms: voice.attack_ms(),
                    decay_ms: voice.decay_ms(),
                    sustain_level: voice.sustain_level(),
                    release_ms: voice.release_ms(),
                },
            },
        }
    }
}

impl TryFrom<WirePatchCommand> for PatchCommand {
    type Error = AppEventCodecError;

    fn try_from(wire: WirePatchCommand) -> Result<Self, Self::Error> {
        Ok(match wire {
            WirePatchCommand::Create => PatchCommand::Create,
            WirePatchCommand::Delete { id } => PatchCommand::Delete {
                id: PatchId::new(id),
            },
            WirePatchCommand::SetMapping { id, mask } => PatchCommand::SetMapping {
                id: PatchId::new(id),
                mapping: channel_mapping_from_mask(mask),
            },
            WirePatchCommand::SetMpeZone { id, zone } => PatchCommand::SetMpeZone {
                id: PatchId::new(id),
                zone: zone
                    .map(|zone| {
                        MpeZone::try_new(zone.master_channel, zone.member_start, zone.member_count)
                    })
                    .transpose()
                    .map_err(|err: PatchError| {
                        AppEventCodecError::InvalidMpeZone(err.to_string())
                    })?,
            },
            WirePatchCommand::AssignSampleSet { id, sample_set } => PatchCommand::AssignSampleSet {
                id: PatchId::new(id),
                sample_set,
            },
            WirePatchCommand::SetVoiceConfig { id, voice } => PatchCommand::SetVoiceConfig {
                id: PatchId::new(id),
                voice: VoiceConfig::try_new(
                    voice.polyphony,
                    voice.attack_ms,
                    voice.decay_ms,
                    voice.sustain_level,
                    voice.release_ms,
                )
                .map_err(|err| AppEventCodecError::InvalidVoiceConfig(err.to_string()))?,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum WirePresetCommand {
    Save { id: u64, name: String },
    Load { id: u64 },
}

impl From<&PresetCommand> for WirePresetCommand {
    fn from(command: &PresetCommand) -> Self {
        match command {
            PresetCommand::Save { id, name } => WirePresetCommand::Save {
                id: id.value(),
                name: name.clone(),
            },
            PresetCommand::Load { id } => WirePresetCommand::Load { id: id.value() },
        }
    }
}

impl From<WirePresetCommand> for PresetCommand {
    fn from(wire: WirePresetCommand) -> Self {
        match wire {
            WirePresetCommand::Save { id, name } => PresetCommand::Save {
                id: PresetId::new(id),
                name,
            },
            WirePresetCommand::Load { id } => PresetCommand::Load {
                id: PresetId::new(id),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum WireAppEvent {
    Midi(WireMidiEvent),
    Gamepad(WireGamepadAction),
    Editor(WireEditorEvent),
    Mixer(WireMixerViewEvent),
    Patch(WirePatchCommand),
    Preset(WirePresetCommand),
}

impl From<&AppEvent> for WireAppEvent {
    fn from(app_event: &AppEvent) -> Self {
        match app_event {
            AppEvent::Midi(midi_event) => WireAppEvent::Midi(WireMidiEvent::from(midi_event)),
            AppEvent::Gamepad(action) => WireAppEvent::Gamepad(WireGamepadAction::from(action)),
            AppEvent::Editor(editor_event) => {
                WireAppEvent::Editor(WireEditorEvent::from(*editor_event))
            }
            AppEvent::Mixer(mixer_event) => {
                WireAppEvent::Mixer(WireMixerViewEvent::from(*mixer_event))
            }
            AppEvent::Patch(command) => WireAppEvent::Patch(WirePatchCommand::from(command)),
            AppEvent::Preset(command) => WireAppEvent::Preset(WirePresetCommand::from(command)),
        }
    }
}

impl TryFrom<WireAppEvent> for AppEvent {
    type Error = AppEventCodecError;

    fn try_from(wire: WireAppEvent) -> Result<Self, Self::Error> {
        Ok(match wire {
            WireAppEvent::Midi(midi_event) => AppEvent::Midi(MidiEvent::try_from(midi_event)?),
            WireAppEvent::Gamepad(action) => AppEvent::Gamepad(action.into()),
            WireAppEvent::Editor(editor_event) => AppEvent::Editor(editor_event.into()),
            WireAppEvent::Mixer(mixer_event) => AppEvent::Mixer(mixer_event.into()),
            WireAppEvent::Patch(command) => AppEvent::Patch(PatchCommand::try_from(command)?),
            WireAppEvent::Preset(command) => AppEvent::Preset(command.into()),
        })
    }
}

impl Serialize for AppEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        WireAppEvent::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AppEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = WireAppEvent::deserialize(deserializer)?;
        AppEvent::try_from(wire).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn midi_note_on() -> MidiEvent {
        MidiEvent::new(
            ChannelAddress::new(
                MidiChannel::try_new(2).unwrap(),
                MidiGroup::try_new(0).unwrap(),
            ),
            MidiEventKind::NoteOn,
            NoteNumber::try_new(60).unwrap(),
            NoteId::new(7),
            Velocity::try_new(0.8).unwrap(),
        )
    }

    fn round_trip(event: &AppEvent) -> AppEvent {
        let json = serde_json::to_string(event).expect("AppEvent serializes");
        serde_json::from_str(&json).expect("AppEvent deserializes")
    }

    #[test]
    fn midi_variant_round_trips_losslessly() {
        let event = AppEvent::Midi(midi_note_on());
        assert_eq!(round_trip(&event), event);
    }

    #[test]
    fn gamepad_variant_round_trips_losslessly() {
        for action in [
            GamepadAction::Navigate(Direction::Up),
            GamepadAction::FineAdjust(StickAxis::new(0.5, -0.25)),
            GamepadAction::Scroll(StickAxis::new(-1.0, 1.0)),
            GamepadAction::Select,
            GamepadAction::Back,
            GamepadAction::SwitchView(ShoulderSide::Left),
            GamepadAction::SwitchPatch(ShoulderSide::Right),
            GamepadAction::SaveSession,
            GamepadAction::OpenBrowser,
        ] {
            let event = AppEvent::Gamepad(action);
            assert_eq!(round_trip(&event), event);
        }
    }

    #[test]
    fn editor_variant_round_trips_losslessly() {
        for editor_event in [
            EditorEvent::NavUp,
            EditorEvent::NavDown,
            EditorEvent::NavLeft,
            EditorEvent::NavRight,
            EditorEvent::EnterEditMode,
            EditorEvent::ExitEditMode,
        ] {
            let event = AppEvent::Editor(editor_event);
            assert_eq!(round_trip(&event), event);
        }
    }

    #[test]
    fn mixer_variant_round_trips_losslessly() {
        for mixer_event in [
            MixerViewEvent::NavUp,
            MixerViewEvent::NavDown,
            MixerViewEvent::NavLeft,
            MixerViewEvent::NavRight,
            MixerViewEvent::EnterEditMode,
            MixerViewEvent::ExitEditMode,
            MixerViewEvent::ToggleFocusedParam,
        ] {
            let event = AppEvent::Mixer(mixer_event);
            assert_eq!(round_trip(&event), event);
        }
    }

    #[test]
    fn patch_create_and_delete_round_trip_losslessly() {
        let create = AppEvent::Patch(PatchCommand::Create);
        assert_eq!(round_trip(&create), create);

        let delete = AppEvent::Patch(PatchCommand::Delete {
            id: PatchId::new(3),
        });
        assert_eq!(round_trip(&delete), delete);
    }

    #[test]
    fn patch_set_mapping_round_trips_losslessly() {
        let event = AppEvent::Patch(PatchCommand::SetMapping {
            id: PatchId::new(1),
            mapping: ChannelMapping::omni(),
        });
        assert_eq!(round_trip(&event), event);

        let event = AppEvent::Patch(PatchCommand::SetMapping {
            id: PatchId::new(1),
            mapping: ChannelMapping::single(9).unwrap(),
        });
        assert_eq!(round_trip(&event), event);
    }

    #[test]
    fn patch_set_mpe_zone_round_trips_losslessly_including_none() {
        let with_zone = AppEvent::Patch(PatchCommand::SetMpeZone {
            id: PatchId::new(2),
            zone: Some(MpeZone::try_new(0, 1, 3).unwrap()),
        });
        assert_eq!(round_trip(&with_zone), with_zone);

        let without_zone = AppEvent::Patch(PatchCommand::SetMpeZone {
            id: PatchId::new(2),
            zone: None,
        });
        assert_eq!(round_trip(&without_zone), without_zone);
    }

    #[test]
    fn patch_assign_sample_set_round_trips_losslessly() {
        let event = AppEvent::Patch(PatchCommand::AssignSampleSet {
            id: PatchId::new(4),
            sample_set: Some(11),
        });
        assert_eq!(round_trip(&event), event);
    }

    #[test]
    fn patch_set_voice_config_round_trips_losslessly() {
        let event = AppEvent::Patch(PatchCommand::SetVoiceConfig {
            id: PatchId::new(5),
            voice: VoiceConfig::try_new(8, 5.0, 50.0, 0.8, 200.0).unwrap(),
        });
        assert_eq!(round_trip(&event), event);
    }

    #[test]
    fn preset_save_and_load_round_trip_losslessly() {
        let save = AppEvent::Preset(PresetCommand::Save {
            id: PresetId::new(42),
            name: "Init Patch".to_string(),
        });
        assert_eq!(round_trip(&save), save);

        let load = AppEvent::Preset(PresetCommand::Load {
            id: PresetId::new(42),
        });
        assert_eq!(round_trip(&load), load);
    }

    #[test]
    fn deserializing_out_of_range_midi_channel_fails_instead_of_panicking() {
        let malformed = serde_json::json!({
            "Midi": {
                "channel": 200,
                "group": 0,
                "kind": "NoteOn",
                "note": 60,
                "note_id": 1,
                "velocity": 0.5
            }
        });
        let result: Result<AppEvent, _> = serde_json::from_value(malformed);
        assert!(result.is_err());
    }

    #[test]
    fn deserializing_invalid_voice_config_fails_instead_of_panicking() {
        let malformed = serde_json::json!({
            "Patch": {
                "SetVoiceConfig": {
                    "id": 1,
                    "voice": {
                        "polyphony": 0,
                        "attack_ms": 5.0,
                        "decay_ms": 0.0,
                        "sustain_level": 1.0,
                        "release_ms": 50.0
                    }
                }
            }
        });
        let result: Result<AppEvent, _> = serde_json::from_value(malformed);
        assert!(result.is_err());
    }
}
