use crate::control::app_state::{AppState, Selection, SelectionSection};
use crate::control::state_snapshot::StateSnapshot;
use crate::control::state_tree::{StateTree, StateTreeError};
use crate::control::text_projection::TextProjection;
use crate::mixer::global_parameters::GlobalParameters;
use crate::real_time::parameter_snapshot::{
    ParameterSnapshot, ParameterSnapshotError, RtPatchParameters, MAX_PATCHES,
};
use crate::synth::patch::Patch;
use core::fmt;
use serde::{Deserialize, Serialize};

const HEADER: &str = "KEYS: W/S parameters | A/D channels | K+direction edit";
const SEPARATOR: &str = "------------------------------------------------------------";

/// A projection failure detected on the control side before publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateProjectionError {
    StateSerialization,
    StateDeserialization,
    StateRoundTripMismatch,
    SelectionDoesNotMatchSnapshot,
    InvalidSelection,
    ParameterSnapshot(ParameterSnapshotError),
    StateTree(StateTreeError),
}

impl fmt::Display for StateProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StateSerialization => {
                formatter.write_str("accepted control state could not be serialized")
            }
            Self::StateDeserialization => {
                formatter.write_str("control state snapshot could not be decoded")
            }
            Self::StateRoundTripMismatch => {
                formatter.write_str("decoded control state does not equal the encoded state")
            }
            Self::SelectionDoesNotMatchSnapshot => {
                formatter.write_str("typed selection does not match the state snapshot")
            }
            Self::InvalidSelection => {
                formatter.write_str("selection is outside the projected control state")
            }
            Self::ParameterSnapshot(error) => error.fmt(formatter),
            Self::StateTree(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for StateProjectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParameterSnapshot(error) => Some(error),
            Self::StateTree(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ParameterSnapshotError> for StateProjectionError {
    fn from(error: ParameterSnapshotError) -> Self {
        Self::ParameterSnapshot(error)
    }
}

impl From<StateTreeError> for StateProjectionError {
    fn from(error: StateTreeError) -> Self {
        Self::StateTree(error)
    }
}

/// Derives all immutable effects from one already-accepted AppState.
///
/// This service never mutates AppState. Serialization and text rendering run on
/// the control side, while the returned ParameterSnapshot is fully owned and
/// fixed-size for publication to the audio boundary.
#[derive(Clone, Copy, Debug, Default)]
pub struct StateProjector;

impl StateProjector {
    pub const fn new() -> Self {
        Self
    }

    /// Derives the three coherent projections consumed after state acceptance.
    pub fn project(
        &self,
        state: &AppState,
    ) -> Result<(StateSnapshot, TextProjection, ParameterSnapshot), StateProjectionError> {
        let snapshot = self.state_snapshot(state)?;
        let text = self.text_projection(&snapshot, state.selection())?;
        let parameters = self.parameter_snapshot(state)?;

        Ok((snapshot, text, parameters))
    }

    /// Derives every coherent projection, including the canonical observation tree.
    pub fn project_with_tree(
        &self,
        state: &AppState,
    ) -> Result<(StateSnapshot, TextProjection, ParameterSnapshot, StateTree), StateProjectionError>
    {
        let (snapshot, text, parameters) = self.project(state)?;
        let tree = self.state_tree(&snapshot, &text, &parameters)?;

        Ok((snapshot, text, parameters, tree))
    }

    /// Builds the canonical tree from one already-derived projection set.
    pub fn state_tree(
        &self,
        snapshot: &StateSnapshot,
        text: &TextProjection,
        parameters: &ParameterSnapshot,
    ) -> Result<StateTree, StateProjectionError> {
        StateTree::new(snapshot, text, parameters).map_err(StateProjectionError::from)
    }

    /// Serializes every AppState field and verifies decode/encode identity.
    pub fn state_snapshot(&self, state: &AppState) -> Result<StateSnapshot, StateProjectionError> {
        let encoded_state = SerializedState::from(state);
        let json = serde_json::to_string(&encoded_state)
            .map_err(|_| StateProjectionError::StateSerialization)?;
        let decoded_state: SerializedState =
            serde_json::from_str(&json).map_err(|_| StateProjectionError::StateDeserialization)?;

        if decoded_state != encoded_state {
            return Err(StateProjectionError::StateRoundTripMismatch);
        }

        Ok(StateSnapshot::new(json))
    }

    /// Renders text exclusively from one snapshot and its typed selection.
    pub fn text_projection(
        &self,
        snapshot: &StateSnapshot,
        selection: Selection,
    ) -> Result<TextProjection, StateProjectionError> {
        let state: SerializedState = serde_json::from_str(snapshot.json())
            .map_err(|_| StateProjectionError::StateDeserialization)?;

        if !state.selection.matches(selection) {
            return Err(StateProjectionError::SelectionDoesNotMatchSnapshot);
        }

        render_text(&state, selection, snapshot.hash())
    }

    /// Copies every audio parameter into bounded, fully owned storage.
    pub fn parameter_snapshot(
        &self,
        state: &AppState,
    ) -> Result<ParameterSnapshot, StateProjectionError> {
        if state.patches().len() > MAX_PATCHES {
            return Err(ParameterSnapshotError::TooManyPatches {
                count: state.patches().len(),
                capacity: MAX_PATCHES,
            }
            .into());
        }

        let patches: Vec<RtPatchParameters> = state
            .patches()
            .iter()
            .map(|patch| RtPatchParameters::new(patch.id(), *patch.parameters()))
            .collect();

        ParameterSnapshot::new(state.generation(), *state.global(), &patches)
            .map_err(StateProjectionError::from)
    }
}

fn render_text(
    state: &SerializedState,
    selection: Selection,
    state_hash: &str,
) -> Result<TextProjection, StateProjectionError> {
    let mut lines = vec![HEADER.to_owned()];
    let mut selected_line = None;

    for (patch_index, patch) in state.patches.iter().enumerate() {
        if patch_index > 0 {
            lines.push(SEPARATOR.to_owned());
        }

        lines.push(format!(
            "PATCH id={} name={} channel={} bank={} program={} percussion={}",
            patch.id, patch.name, patch.channel, patch.bank, patch.program, patch.percussion
        ));

        let parameters = [
            ("gainDb", patch.gain_db),
            ("pan", patch.pan),
            ("reverbSend", patch.reverb_send),
            ("delaySend", patch.delay_send),
        ];
        for (parameter_index, (name, value)) in parameters.into_iter().enumerate() {
            let selected = selection.section() == SelectionSection::Patch
                && selection.patch_index() == patch_index
                && selection.parameter_index() == parameter_index;
            push_parameter_line(&mut lines, &mut selected_line, selected, name, value);
        }
    }

    if !state.patches.is_empty() {
        lines.push(SEPARATOR.to_owned());
    }
    lines.push("GLOBAL".to_owned());

    let global_parameters = [
        ("masterGainDb", state.global.master_gain_db),
        ("reverbRoomSize", state.global.reverb_room_size),
        ("reverbDamping", state.global.reverb_damping),
        ("reverbReturn", state.global.reverb_return),
        ("delayMilliseconds", state.global.delay_milliseconds),
        ("delayFeedback", state.global.delay_feedback),
        ("delayReturn", state.global.delay_return),
    ];
    for (parameter_index, (name, value)) in global_parameters.into_iter().enumerate() {
        let selected = selection.section() == SelectionSection::Global
            && selection.parameter_index() == parameter_index;
        push_parameter_line(&mut lines, &mut selected_line, selected, name, value);
    }

    let selected_line = selected_line.ok_or(StateProjectionError::InvalidSelection)?;
    Ok(TextProjection::new(
        lines.join("\n"),
        selected_line,
        state_hash.to_owned(),
    ))
}

fn push_parameter_line(
    lines: &mut Vec<String>,
    selected_line: &mut Option<usize>,
    selected: bool,
    name: &str,
    value: f32,
) {
    let marker = if selected { '>' } else { ' ' };
    if selected {
        *selected_line = Some(lines.len());
    }
    lines.push(format!("{marker} {name}={value}"));
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializedState {
    generation: u64,
    patches: Vec<SerializedPatch>,
    global: SerializedGlobalParameters,
    selection: SerializedSelection,
}

impl From<&AppState> for SerializedState {
    fn from(state: &AppState) -> Self {
        Self {
            generation: state.generation(),
            patches: state.patches().iter().map(SerializedPatch::from).collect(),
            global: SerializedGlobalParameters::from(state.global()),
            selection: SerializedSelection::from(state.selection()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializedPatch {
    id: u32,
    name: String,
    channel: u8,
    bank: u16,
    program: u8,
    percussion: bool,
    gain_db: f32,
    pan: f32,
    reverb_send: f32,
    delay_send: f32,
}

impl From<&Patch> for SerializedPatch {
    fn from(patch: &Patch) -> Self {
        Self {
            id: patch.id().value(),
            name: patch.name().to_owned(),
            channel: patch.channel().value(),
            bank: patch.instrument().bank(),
            program: patch.instrument().program(),
            percussion: patch.instrument().percussion(),
            gain_db: patch.parameters().gain_db(),
            pan: patch.parameters().pan(),
            reverb_send: patch.parameters().reverb_send(),
            delay_send: patch.parameters().delay_send(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializedGlobalParameters {
    master_gain_db: f32,
    reverb_room_size: f32,
    reverb_damping: f32,
    reverb_return: f32,
    delay_milliseconds: f32,
    delay_feedback: f32,
    delay_return: f32,
}

impl From<&GlobalParameters> for SerializedGlobalParameters {
    fn from(parameters: &GlobalParameters) -> Self {
        Self {
            master_gain_db: parameters.master_gain_db(),
            reverb_room_size: parameters.reverb_room_size(),
            reverb_damping: parameters.reverb_damping(),
            reverb_return: parameters.reverb_return(),
            delay_milliseconds: parameters.delay_milliseconds(),
            delay_feedback: parameters.delay_feedback(),
            delay_return: parameters.delay_return(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializedSelection {
    section: SerializedSelectionSection,
    patch_index: usize,
    parameter_index: usize,
}

impl SerializedSelection {
    fn matches(self, selection: Selection) -> bool {
        self == Self::from(selection)
    }
}

impl From<Selection> for SerializedSelection {
    fn from(selection: Selection) -> Self {
        Self {
            section: SerializedSelectionSection::from(selection.section()),
            patch_index: selection.patch_index(),
            parameter_index: selection.parameter_index(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum SerializedSelectionSection {
    Patch,
    Global,
}

impl From<SelectionSection> for SerializedSelectionSection {
    fn from(section: SelectionSection) -> Self {
        match section {
            SelectionSection::Patch => Self::Patch,
            SelectionSection::Global => Self::Global,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::app_event::{AppEvent, Direction};
    use crate::control::app_state::EventRejection;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::synth::sound_font_instrument::SoundFontInstrument;

    fn global_parameters() -> GlobalParameters {
        GlobalParameters::new(-3.0, 0.7, 0.4, 0.25, 375.0, 0.35, 0.2).unwrap()
    }

    fn patch(id: u32, gain_db: f32) -> Patch {
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            SoundFontInstrument::new(128, (id - 1) as u8, (id & 1) == 0).unwrap(),
            MidiChannel::new(((id - 1) % 16) as u8).unwrap(),
            ChannelParameters::new(gain_db, 0.1, 0.2, 0.3).unwrap(),
        )
    }

    fn installed_state() -> AppState {
        let mut state = AppState::new(global_parameters());
        state
            .apply(AppEvent::InstallPatches(vec![
                patch(1, -6.0),
                patch(2, -12.0),
            ]))
            .unwrap();
        state
    }

    #[test]
    fn serialization_is_deterministic_and_round_trips_every_field() {
        let state = installed_state();
        let projector = StateProjector::new();
        let first = projector.state_snapshot(&state).unwrap();
        let second = projector.state_snapshot(&state).unwrap();
        let decoded: SerializedState = serde_json::from_str(first.json()).unwrap();

        assert_eq!(first, second);
        assert_eq!(decoded, SerializedState::from(&state));
        assert_eq!(decoded.generation, 1);
        assert_eq!(decoded.patches.len(), 2);
        assert_eq!(decoded.patches[0].name, "Patch 1");
        assert_eq!(decoded.patches[0].bank, 128);
        assert_eq!(decoded.patches[0].gain_db, -6.0);
        assert_eq!(decoded.global.delay_milliseconds, 375.0);
        assert_eq!(decoded.selection.section, SerializedSelectionSection::Patch);
        assert_eq!(decoded.selection.patch_index, 0);
        assert_eq!(decoded.selection.parameter_index, 0);
    }

    #[test]
    fn text_is_derived_from_the_snapshot_and_typed_selection() {
        let mut state = installed_state();
        let projector = StateProjector::new();
        let snapshot = projector.state_snapshot(&state).unwrap();

        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        let text = projector
            .text_projection(&snapshot, Selection::patch(0))
            .unwrap();

        assert!(text.body().starts_with(HEADER));
        assert!(text
            .body()
            .contains("PATCH id=1 name=Patch 1 channel=0 bank=128 program=0 percussion=false"));
        assert!(text.body().contains("> gainDb=-6"));
        assert!(!text.body().contains("gainDb=-5.99"));
        assert!(text.body().contains(SEPARATOR));
        assert!(text.body().contains("GLOBAL"));
        assert_eq!(text.selected_line(), 2);
        assert_eq!(text.state_hash(), snapshot.hash());
    }

    #[test]
    fn rejects_a_typed_selection_from_a_different_snapshot() {
        let state = installed_state();
        let projector = StateProjector::new();
        let snapshot = projector.state_snapshot(&state).unwrap();

        assert_eq!(
            projector.text_projection(&snapshot, Selection::global()),
            Err(StateProjectionError::SelectionDoesNotMatchSnapshot)
        );
    }

    #[test]
    fn parameter_projection_copies_every_audio_value() {
        let state = installed_state();
        let snapshot = StateProjector::new().parameter_snapshot(&state).unwrap();

        assert_eq!(snapshot.generation(), state.generation());
        assert_eq!(snapshot.global(), state.global());
        assert_eq!(snapshot.patch_count(), state.patches().len());
        for (projected, patch) in snapshot.patches().iter().zip(state.patches()) {
            assert_eq!(projected.patch_id(), Some(patch.id()));
            assert_eq!(projected.parameters(), patch.parameters());
        }
    }

    #[test]
    fn rejects_oversized_patch_installation_before_projection() {
        let mut state = AppState::new(global_parameters());
        let patches = (1..=(MAX_PATCHES as u32 + 1))
            .map(|id| patch(id, -6.0))
            .collect();

        assert_eq!(
            state.apply(AppEvent::InstallPatches(patches)),
            Err(EventRejection::TooManyPatches)
        );
        assert_eq!(state.generation(), 0);
        assert!(state.patches().is_empty());

        let snapshot = StateProjector::new().parameter_snapshot(&state).unwrap();
        assert_eq!(snapshot.generation(), state.generation());
        assert_eq!(snapshot.patch_count(), 0);
    }

    #[test]
    fn complete_projection_uses_one_accepted_generation() {
        let state = installed_state();
        let (snapshot, text, parameters, tree) =
            StateProjector::new().project_with_tree(&state).unwrap();

        assert_eq!(text.state_hash(), snapshot.hash());
        assert_eq!(parameters.generation(), state.generation());
        assert_eq!(tree.generation(), parameters.generation());
        assert_eq!(tree.state_hash(), snapshot.hash());
        assert_eq!(tree.selected_line(), text.selected_line());
        assert_eq!(tree.patch_count(), parameters.patch_count());
        assert_eq!(
            serde_json::from_str::<SerializedState>(snapshot.json())
                .unwrap()
                .generation,
            parameters.generation()
        );
    }

    #[test]
    fn malformed_snapshot_cannot_be_rendered() {
        let snapshot = StateSnapshot::new("not-json");

        assert_eq!(
            StateProjector::new().text_projection(&snapshot, Selection::global()),
            Err(StateProjectionError::StateDeserialization)
        );
    }
}
