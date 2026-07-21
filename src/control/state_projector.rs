use crate::control::app_state::{AppState, Selection, SelectionSection};
use crate::control::serialized_state::SerializedState;
use crate::control::state_snapshot::StateSnapshot;
use crate::control::state_tree::{StateTree, StateTreeError};
use crate::control::text_projection::TextProjection;
use crate::real_time::parameter_snapshot::{
    ParameterSnapshot, ParameterSnapshotError, RtPatchParameters, MAX_PATCHES,
};
use crate::synth::instrument_capability::{
    InstrumentConfig, ParameterDefault, ParameterKind, ParameterSpec, ParameterValue,
};
use core::fmt;

const HEADER: &str = "KEYS: W/S parameters | A/D channels | K+direction edit";
const SEPARATOR: &str = "------------------------------------------------------------";

/// A projection failure detected on the control side before publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateProjectionError {
    StateSerialization,
    StateDeserialization,
    StateGenerationTemplateMismatch,
    SelectionDoesNotMatchSnapshot,
    InvalidSelection,
    InvalidInstrumentConfig,
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
            Self::StateGenerationTemplateMismatch => formatter
                .write_str("canonical state snapshot cannot advance a generation-only projection"),
            Self::SelectionDoesNotMatchSnapshot => {
                formatter.write_str("typed selection does not match the state snapshot")
            }
            Self::InvalidSelection => {
                formatter.write_str("selection is outside the projected control state")
            }
            Self::InvalidInstrumentConfig => {
                formatter.write_str("instrument config does not resolve through the registry")
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
        let serialized = SerializedState::from(state);
        let snapshot = self.snapshot_from_serialized(&serialized)?;
        let text = self.text_from_serialized(&serialized, state.selection(), snapshot.hash())?;
        let parameters = self.parameter_snapshot(state)?;

        Ok((snapshot, text, parameters))
    }

    /// Derives every coherent projection, including the canonical observation tree.
    pub fn project_with_tree(
        &self,
        state: &AppState,
    ) -> Result<(StateSnapshot, TextProjection, ParameterSnapshot, StateTree), StateProjectionError>
    {
        let serialized = SerializedState::from(state);
        let snapshot = self.snapshot_from_serialized(&serialized)?;
        let text = self.text_from_serialized(&serialized, state.selection(), snapshot.hash())?;
        let parameters = self.parameter_snapshot(state)?;
        let tree = StateTree::from_serialized_state(&serialized, &snapshot, &text, &parameters)?;

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

    /// Serializes every AppState field through the canonical borrowed state view.
    ///
    /// Decode/encode identity is verified outside this hot production path by
    /// tests that deserialize the emitted snapshot into the same canonical type.
    pub fn state_snapshot(&self, state: &AppState) -> Result<StateSnapshot, StateProjectionError> {
        let encoded_state = SerializedState::from(state);
        self.snapshot_from_serialized(&encoded_state)
    }

    /// Renders text exclusively from one snapshot and its typed selection.
    pub fn text_projection(
        &self,
        snapshot: &StateSnapshot,
        selection: Selection,
    ) -> Result<TextProjection, StateProjectionError> {
        let state: SerializedState<'_> = serde_json::from_str(snapshot.json())
            .map_err(|_| StateProjectionError::StateDeserialization)?;
        self.text_from_serialized(&state, selection, snapshot.hash())
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

    /// Advances all coherent projections after a validated MIDI event changed
    /// only AppState generation and emitted one discrete audio command.
    pub(crate) fn project_midi_generation(
        &self,
        state: &AppState,
        previous_snapshot: &StateSnapshot,
        previous_text: &TextProjection,
        previous_parameters: ParameterSnapshot,
        previous_tree: &StateTree,
    ) -> Result<(StateSnapshot, TextProjection, ParameterSnapshot, StateTree), StateProjectionError>
    {
        let generation = state.generation();
        if previous_parameters.generation().checked_add(1) != Some(generation)
            || previous_tree.generation().checked_add(1) != Some(generation)
        {
            return Err(StateProjectionError::StateGenerationTemplateMismatch);
        }

        let snapshot = previous_snapshot
            .with_generation(generation)
            .ok_or(StateProjectionError::StateGenerationTemplateMismatch)?;
        let text = previous_text.with_state_hash(snapshot.hash().to_owned());
        let parameters = previous_parameters.with_generation(generation);
        let tree = previous_tree.with_midi_generation(&snapshot, &text, &parameters)?;
        Ok((snapshot, text, parameters, tree))
    }

    fn snapshot_from_serialized(
        &self,
        state: &SerializedState<'_>,
    ) -> Result<StateSnapshot, StateProjectionError> {
        let json =
            serde_json::to_string(state).map_err(|_| StateProjectionError::StateSerialization)?;
        Ok(StateSnapshot::new(json))
    }

    fn text_from_serialized(
        &self,
        state: &SerializedState<'_>,
        selection: Selection,
        state_hash: &str,
    ) -> Result<TextProjection, StateProjectionError> {
        if !state.selection.matches(selection) {
            return Err(StateProjectionError::SelectionDoesNotMatchSnapshot);
        }
        render_text(state, selection, state_hash)
    }
}

fn render_text(
    state: &SerializedState<'_>,
    selection: Selection,
    state_hash: &str,
) -> Result<TextProjection, StateProjectionError> {
    let mut lines = vec![HEADER.to_owned()];
    let mut selected_line = None;

    for (patch_index, patch) in state.patches.iter().enumerate() {
        if patch_index > 0 {
            lines.push(SEPARATOR.to_owned());
        }

        let descriptor = state
            .capabilities
            .descriptor(patch.instrument.capability_id())
            .ok_or(StateProjectionError::InvalidInstrumentConfig)?;
        lines.push(format!(
            "PATCH id={} name={} channel={} capability={} ({})",
            patch.id,
            patch.name,
            patch.channel,
            descriptor.label(),
            descriptor.id()
        ));
        for section in descriptor.sections() {
            lines.push(format!(
                "  INSTRUMENT {} ({})",
                section.label(),
                section.id()
            ));
            for spec in section.parameters() {
                let value = format_instrument_value(spec, patch.instrument.as_ref())?;
                lines.push(format!("  {} ({})={value}", spec.label(), spec.id()));
            }
        }

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

fn format_instrument_value(
    spec: &ParameterSpec,
    config: &InstrumentConfig,
) -> Result<String, StateProjectionError> {
    if spec.kind() == ParameterKind::Asset {
        let reference = config
            .asset_reference(spec.id())
            .or_else(|| match spec.default_value() {
                ParameterDefault::Asset(reference) => Some(reference),
                ParameterDefault::Value(_) => None,
            })
            .ok_or(StateProjectionError::InvalidInstrumentConfig)?;
        return Ok(match spec.formatter() {
            "asset" => format!(
                "{}:{}",
                format!("{:?}", reference.kind()).to_lowercase(),
                reference.locator()
            ),
            _ => reference.locator().to_owned(),
        });
    }

    let value = config
        .value(spec.id())
        .ok_or(StateProjectionError::InvalidInstrumentConfig)?;
    let formatted = match (spec.formatter(), value) {
        ("integer", ParameterValue::Stepped(value)) => value.to_string(),
        ("toggle", ParameterValue::Toggle(value)) => value.to_string(),
        (_, ParameterValue::Continuous(value)) => {
            if let Some(unit) = spec.unit() {
                format!("{value}{unit}")
            } else {
                value.to_string()
            }
        }
        (_, ParameterValue::Stepped(value)) => value.to_string(),
        (_, ParameterValue::Choice(value)) => value.clone(),
        (_, ParameterValue::Toggle(value)) => value.to_string(),
    };
    Ok(formatted)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::hidef_soundfont_capability::{
        HiDefSoundFontCapability, HIDEF_CAPABILITY_ID, SOUNDFONT_BANK_PARAMETER_ID,
    };
    use crate::control::app_event::{AppEvent, Direction};
    use crate::control::app_state::EventRejection;
    use crate::control::serialized_state::{SerializedSelectionSection, SerializedState};
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::testing::automatic_midi_test::create_soundfont_config;

    fn global_parameters() -> GlobalParameters {
        GlobalParameters::new(-3.0, 0.7, 0.4, 0.25, 375.0, 0.35, 0.2).unwrap()
    }

    fn patch(id: u32, gain_db: f32) -> Patch {
        let provider = HiDefSoundFontCapability::new().unwrap();
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            create_soundfont_config(
                &provider,
                SoundFontInstrument::new(128, (id - 1) as u8, (id & 1) == 0).unwrap(),
            )
            .unwrap(),
            MidiChannel::new(((id - 1) % 16) as u8).unwrap(),
            ChannelParameters::new(gain_db, 0.1, 0.2, 0.3).unwrap(),
        )
    }

    fn installed_state() -> AppState {
        let provider = HiDefSoundFontCapability::new().unwrap();
        let mut state = AppState::new(provider.registry().unwrap(), global_parameters());
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
        assert_eq!(decoded.capabilities.descriptors().len(), 1);
        assert_eq!(
            decoded.patches[0].instrument.capability_id().as_str(),
            HIDEF_CAPABILITY_ID
        );
        assert_eq!(
            decoded.patches[0]
                .instrument
                .value(&crate::synth::ParameterId::new(SOUNDFONT_BANK_PARAMETER_ID).unwrap()),
            Some(&ParameterValue::Stepped(128))
        );
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
            .contains("PATCH id=1 name=Patch 1 channel=0 capability=HiDef SoundFont (instrument.soundfont.hidef)"));
        assert!(text.body().contains("Bank (soundfont.bank)=128"));
        assert!(text
            .body()
            .contains("SoundFont File (soundfont.file)=soundfont:./sf2/HiDef.sf2"));
        assert!(text.body().contains("> gainDb=-6"));
        assert!(!text.body().contains("gainDb=-5.99"));
        assert!(text.body().contains(SEPARATOR));
        assert!(text.body().contains("GLOBAL"));
        assert_eq!(
            text.body()
                .lines()
                .position(|line| line.starts_with("> gainDb=")),
            Some(text.selected_line())
        );
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
    fn state_projector_exact_projection_values() {
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
        let provider = HiDefSoundFontCapability::new().unwrap();
        let mut state = AppState::new(provider.registry().unwrap(), global_parameters());
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
    fn midi_generation_projection_is_exactly_equal_to_eager_projection() {
        let mut state = installed_state();
        let projector = StateProjector::new();
        let (snapshot, text, parameters, tree) = projector.project_with_tree(&state).unwrap();
        let patch_id = state.patches()[0].id();
        let message = MidiMessage::try_new(
            state.patches()[0].channel(),
            MidiMessageKind::NoteOn,
            60,
            100,
        )
        .unwrap();
        state.apply(AppEvent::Midi { patch_id, message }).unwrap();

        let fast = projector
            .project_midi_generation(&state, &snapshot, &text, parameters, &tree)
            .unwrap();
        let eager = projector.project_with_tree(&state).unwrap();

        assert_eq!(fast.0, eager.0);
        assert_eq!(fast.1, eager.1);
        assert_eq!(fast.2, eager.2);
        assert_eq!(fast.3, eager.3);
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
