use crate::control::app_state::AppState;
use crate::control::interaction_state::{Selection, SelectionSection};
use crate::control::patch_page_projection::{PatchPageProjection, PatchPageProjectionError};
use crate::control::serialized_state::SerializedState;
use crate::control::state_snapshot::StateSnapshot;
use crate::control::state_tree::{StateTree, StateTreeError};
use crate::control::text_projection::TextProjection;
use crate::real_time::parameter_snapshot::{ParameterSnapshot, ParameterSnapshotError};
use crate::real_time::GraphRevision;
use crate::synth::instrument_capability::{
    InstrumentConfig, ParameterDefault, ParameterKind, ParameterSpec, ParameterUpdate,
    ParameterValue,
};
use crate::synth::patch::{resolve_patch_editable_targets, PatchEditableTarget};
use core::fmt;

const MIXER_HEADER: &str =
    "MIXER | 1 MIXER | 2 PATCH | W/S parameters | A/D channels | K+direction edit";
const PATCH_HEADER: &str = "PATCH | 1 MIXER | 2 PATCH | W/S controls | K+direction edit";
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
    PatchPage(PatchPageProjectionError),
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
            Self::PatchPage(error) => error.fmt(formatter),
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
            Self::PatchPage(error) => Some(error),
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

impl From<PatchPageProjectionError> for StateProjectionError {
    fn from(error: PatchPageProjectionError) -> Self {
        Self::PatchPage(error)
    }
}

/// Derives all immutable effects from one already-accepted AppState.
///
/// This service never mutates AppState. Serialization and text rendering run on
/// the control side, while the returned ParameterSnapshot is fully owned and
/// fixed-size for publication to the audio boundary.
#[derive(Clone, Copy, Debug)]
pub struct StateProjector {
    graph_revision: GraphRevision,
}

impl Default for StateProjector {
    fn default() -> Self {
        Self::new()
    }
}

impl StateProjector {
    pub const fn new() -> Self {
        Self {
            graph_revision: GraphRevision::INITIAL,
        }
    }

    /// Creates a projector whose scalar output targets one prepared graph.
    pub const fn for_graph(graph_revision: GraphRevision) -> Self {
        Self { graph_revision }
    }

    pub const fn graph_revision(self) -> GraphRevision {
        self.graph_revision
    }

    /// Derives the snapshot, active text, and fixed real-time values.
    pub fn project(
        &self,
        state: &AppState,
    ) -> Result<(StateSnapshot, TextProjection, ParameterSnapshot), StateProjectionError> {
        let (snapshot, _page, text, parameters) = self.project_with_page(state)?;
        Ok((snapshot, text, parameters))
    }

    /// Derives the complete projection set including the optional PATCH page.
    pub fn project_with_page(
        &self,
        state: &AppState,
    ) -> Result<
        (
            StateSnapshot,
            Option<PatchPageProjection>,
            TextProjection,
            ParameterSnapshot,
        ),
        StateProjectionError,
    > {
        let serialized = SerializedState::from(state);
        let snapshot = self.snapshot_from_serialized(&serialized)?;
        let page = self.patch_page_projection(state, snapshot.hash())?;
        let text = self.text_from_serialized(
            &serialized,
            state.selection(),
            page.as_ref(),
            snapshot.hash(),
        )?;
        let parameters = self.parameter_snapshot(state)?;

        Ok((snapshot, page, text, parameters))
    }

    /// Derives every coherent projection, including the canonical observation tree.
    pub fn project_with_tree(
        &self,
        state: &AppState,
    ) -> Result<
        (
            StateSnapshot,
            Option<PatchPageProjection>,
            TextProjection,
            ParameterSnapshot,
            StateTree,
        ),
        StateProjectionError,
    > {
        let serialized = SerializedState::from(state);
        let snapshot = self.snapshot_from_serialized(&serialized)?;
        let page = self.patch_page_projection(state, snapshot.hash())?;
        let text = self.text_from_serialized(
            &serialized,
            state.selection(),
            page.as_ref(),
            snapshot.hash(),
        )?;
        let parameters = self.parameter_snapshot(state)?;
        let tree = StateTree::from_serialized_state(
            &serialized,
            &snapshot,
            page.as_ref(),
            &text,
            &parameters,
        )?;

        Ok((snapshot, page, text, parameters, tree))
    }

    /// Builds the canonical tree from one already-derived projection set.
    pub fn state_tree(
        &self,
        snapshot: &StateSnapshot,
        text: &TextProjection,
        parameters: &ParameterSnapshot,
    ) -> Result<StateTree, StateProjectionError> {
        self.state_tree_with_page(snapshot, None, text, parameters)
    }

    /// Builds the canonical tree with an explicit optional PATCH page.
    pub fn state_tree_with_page(
        &self,
        snapshot: &StateSnapshot,
        page: Option<&PatchPageProjection>,
        text: &TextProjection,
        parameters: &ParameterSnapshot,
    ) -> Result<StateTree, StateProjectionError> {
        StateTree::with_patch_page(snapshot, page, text, parameters)
            .map_err(StateProjectionError::from)
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
        self.text_projection_with_page(snapshot, selection, None)
    }

    /// Renders text with an explicit optional host-neutral PATCH page.
    pub fn text_projection_with_page(
        &self,
        snapshot: &StateSnapshot,
        selection: Selection,
        page: Option<&PatchPageProjection>,
    ) -> Result<TextProjection, StateProjectionError> {
        let state: SerializedState<'_> = serde_json::from_str(snapshot.json())
            .map_err(|_| StateProjectionError::StateDeserialization)?;
        self.text_from_serialized(&state, selection, page, snapshot.hash())
    }

    /// Resolves the optional active PATCH page from canonical state and registry data.
    pub fn patch_page_projection(
        &self,
        state: &AppState,
        state_hash: &str,
    ) -> Result<Option<PatchPageProjection>, StateProjectionError> {
        match state.context() {
            crate::control::TopLevelContext::Mixer => Ok(None),
            crate::control::TopLevelContext::Patch => {
                PatchPageProjection::project(state, state_hash)
                    .map(Some)
                    .map_err(StateProjectionError::from)
            }
        }
    }

    /// Copies every audio parameter into bounded, fully owned storage.
    pub fn parameter_snapshot(
        &self,
        state: &AppState,
    ) -> Result<ParameterSnapshot, StateProjectionError> {
        ParameterSnapshot::project_patches(
            state.generation(),
            state.engine_selection().projection_graph_revision(),
            *state.global(),
            state.patches(),
            state.capabilities(),
        )
        .map_err(|error| match error {
            ParameterSnapshotError::InvalidInstrumentConfig { .. } => {
                StateProjectionError::InvalidInstrumentConfig
            }
            other => StateProjectionError::ParameterSnapshot(other),
        })
    }

    /// Advances all coherent projections after a validated MIDI event changed
    /// only AppState generation and emitted one discrete audio command.
    pub(crate) fn project_midi_generation(
        &self,
        state: &AppState,
        previous_snapshot: &StateSnapshot,
        previous_page: Option<&PatchPageProjection>,
        previous_text: &TextProjection,
        previous_parameters: ParameterSnapshot,
        previous_tree: &StateTree,
    ) -> Result<
        (
            StateSnapshot,
            Option<PatchPageProjection>,
            TextProjection,
            ParameterSnapshot,
            StateTree,
        ),
        StateProjectionError,
    > {
        let generation = state.generation();
        if previous_parameters.generation().checked_add(1) != Some(generation)
            || previous_tree.generation().checked_add(1) != Some(generation)
            || previous_parameters.graph_revision()
                != state.engine_selection().projection_graph_revision()
            || previous_tree.graph_revision()
                != state.engine_selection().projection_graph_revision()
        {
            return Err(StateProjectionError::StateGenerationTemplateMismatch);
        }

        let snapshot = previous_snapshot
            .with_generation(generation)
            .ok_or(StateProjectionError::StateGenerationTemplateMismatch)?;
        let page = previous_page.map(|page| page.with_state_hash(snapshot.hash().to_owned()));
        let text = previous_text.with_state_hash(snapshot.hash().to_owned());
        let parameters = previous_parameters.with_generation(generation);
        let tree =
            previous_tree.with_midi_generation(&snapshot, page.as_ref(), &text, &parameters)?;
        Ok((snapshot, page, text, parameters, tree))
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
        page: Option<&PatchPageProjection>,
        state_hash: &str,
    ) -> Result<TextProjection, StateProjectionError> {
        if !state.interaction.mixer_selection.matches(selection) {
            return Err(StateProjectionError::SelectionDoesNotMatchSnapshot);
        }
        match state.interaction.context {
            crate::control::TopLevelContext::Mixer => {
                if page.is_some() {
                    return Err(StateProjectionError::InvalidSelection);
                }
                render_mixer_text(state, selection, state_hash)
            }
            crate::control::TopLevelContext::Patch => {
                let page = page.ok_or(StateProjectionError::InvalidSelection)?;
                if page.state_hash() != state_hash {
                    return Err(StateProjectionError::InvalidSelection);
                }
                render_patch_text(page, state_hash)
            }
        }
    }
}

fn render_mixer_text(
    state: &SerializedState<'_>,
    selection: Selection,
    state_hash: &str,
) -> Result<TextProjection, StateProjectionError> {
    let mut lines = vec![MIXER_HEADER.to_owned()];
    let mut selected_line = None;

    for (patch_index, patch) in state.patches.iter().enumerate() {
        if patch_index > 0 {
            lines.push(SEPARATOR.to_owned());
        }

        let descriptor = state
            .capabilities
            .descriptor(patch.instrument.capability_id())
            .ok_or(StateProjectionError::InvalidInstrumentConfig)?;
        let editable_targets =
            resolve_patch_editable_targets(descriptor, patch.instrument.as_ref())
                .map_err(|_| StateProjectionError::InvalidInstrumentConfig)?;
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
                if spec.update() == ParameterUpdate::Scalar {
                    let target = PatchEditableTarget::Instrument(spec.id().clone());
                    let parameter_index = editable_targets
                        .iter()
                        .position(|candidate| candidate == &target)
                        .ok_or(StateProjectionError::InvalidInstrumentConfig)?;
                    let selected = selection.section() == SelectionSection::Patch
                        && selection.patch_index() == patch_index
                        && selection.parameter_index() == parameter_index;
                    push_parameter_text_line(
                        &mut lines,
                        &mut selected_line,
                        selected,
                        &format!("{} ({})", spec.label(), spec.id()),
                        &value,
                    );
                } else {
                    lines.push(format!("  {} ({})={value}", spec.label(), spec.id()));
                }
            }
        }

        let parameters = [
            patch.gain_db,
            patch.pan,
            patch.reverb_send,
            patch.delay_send,
        ];
        for (descriptor, value) in
            crate::mixer::channel_parameters::ChannelParameters::surface_descriptor()
                .iter()
                .zip(parameters)
        {
            let target = PatchEditableTarget::Mixer(descriptor.parameter());
            let parameter_index = editable_targets
                .iter()
                .position(|candidate| candidate == &target)
                .ok_or(StateProjectionError::InvalidInstrumentConfig)?;
            let selected = selection.section() == SelectionSection::Patch
                && selection.patch_index() == patch_index
                && selection.parameter_index() == parameter_index;
            push_parameter_line(
                &mut lines,
                &mut selected_line,
                selected,
                descriptor.name(),
                value,
            );
        }

        for descriptor in crate::synth::voice_envelope::VoiceEnvelope::surface_descriptor() {
            let target = PatchEditableTarget::Envelope(descriptor.parameter());
            let parameter_index = editable_targets
                .iter()
                .position(|candidate| candidate == &target)
                .ok_or(StateProjectionError::InvalidInstrumentConfig)?;
            let selected = selection.section() == SelectionSection::Patch
                && selection.patch_index() == patch_index
                && selection.parameter_index() == parameter_index;
            push_parameter_line(
                &mut lines,
                &mut selected_line,
                selected,
                descriptor.name(),
                patch.envelope.value(descriptor.parameter()),
            );
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
    Ok(TextProjection::for_context(
        crate::control::TopLevelContext::Mixer,
        lines.join("\n"),
        selected_line,
        state_hash.to_owned(),
    ))
}

fn render_patch_text(
    page: &PatchPageProjection,
    state_hash: &str,
) -> Result<TextProjection, StateProjectionError> {
    let mut lines = vec![PATCH_HEADER.to_owned()];
    let mut selected_line = None;
    lines.push(format!(
        " PATCH {}",
        serde_json::to_string(page.patch())
            .map_err(|_| StateProjectionError::StateSerialization)?
    ));
    let engine_selected = page.engine().control_id() == page.focused_control_id();
    if engine_selected {
        selected_line = Some(lines.len());
    }
    let engine_marker = if engine_selected { '>' } else { ' ' };
    lines.push(format!(
        "{engine_marker} ENGINE {}",
        serde_json::to_string(page.engine())
            .map_err(|_| StateProjectionError::StateSerialization)?
    ));
    for row in page.envelope() {
        let selected = row.control_id() == page.focused_control_id();
        if selected {
            selected_line = Some(lines.len());
        }
        let marker = if selected { '>' } else { ' ' };
        lines.push(format!(
            "{marker} ENVELOPE {}",
            serde_json::to_string(row).map_err(|_| StateProjectionError::StateSerialization)?
        ));
    }
    for section in page.sections() {
        lines.push(format!(
            " SECTION id={} label={}",
            section.id(),
            section.label()
        ));
        for row in section.parameters() {
            let selected = row.control_id() == Some(page.focused_control_id());
            if selected {
                selected_line = Some(lines.len());
            }
            let marker = if selected { '>' } else { ' ' };
            lines.push(format!(
                "{marker} PARAMETER {}",
                serde_json::to_string(row).map_err(|_| StateProjectionError::StateSerialization)?
            ));
        }
    }

    let selected_line = selected_line.ok_or(StateProjectionError::InvalidSelection)?;

    Ok(TextProjection::for_context(
        crate::control::TopLevelContext::Patch,
        lines.join("\n"),
        selected_line,
        state_hash.to_owned(),
    ))
}

pub(crate) fn format_instrument_value(
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
    push_parameter_text_line(lines, selected_line, selected, name, &value.to_string());
}

fn push_parameter_text_line(
    lines: &mut Vec<String>,
    selected_line: &mut Option<usize>,
    selected: bool,
    name: &str,
    value: &str,
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
    use crate::adapter::braids_capability::{BraidsCapability, BRAIDS_CAPABILITY_ID};
    use crate::adapter::hidef_soundfont_capability::{
        HIDEF_CAPABILITY_ID, SOUNDFONT_PRESET_PARAMETER_ID,
    };
    use crate::adapter::production_instruments::production_capability_registry;
    use crate::control::app_event::{AppEvent, Direction};
    use crate::control::app_state::EventRejection;
    use crate::control::serialized_state::{SerializedSelectionSection, SerializedState};
    use crate::control::{EngineSelectionFailure, EngineSelectionStatusKind};
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::MAX_PATCHES;
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::PatchInteraction;
    use crate::testing::automatic_midi_test::create_soundfont_config;

    fn global_parameters() -> GlobalParameters {
        GlobalParameters::new(-3.0, 0.7, 0.4, 0.25, 375.0, 0.35, 0.2).unwrap()
    }

    fn patch(id: u32, gain_db: f32) -> Patch {
        let provider =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        Patch::new(
            PatchId::new(id).unwrap(),
            format!("Patch {id}"),
            create_soundfont_config(
                &provider,
                SoundFontInstrument::new(128, ((id - 1) % 2) as u8, (id & 1) == 0).unwrap(),
            )
            .unwrap(),
            MidiChannel::new(((id - 1) % 16) as u8).unwrap(),
            ChannelParameters::new(gain_db, 0.1, 0.2, 0.3).unwrap(),
        )
    }

    fn installed_state_for_graph(graph_revision: GraphRevision) -> AppState {
        let provider =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        let mut state = AppState::for_graph(
            provider.registry().unwrap(),
            global_parameters(),
            graph_revision,
        );
        state
            .apply(AppEvent::InstallPatches(vec![
                patch(1, -6.0),
                patch(2, -12.0),
            ]))
            .unwrap();
        state
    }

    fn installed_state() -> AppState {
        installed_state_for_graph(GraphRevision::INITIAL)
    }

    fn mixed_patch_state() -> AppState {
        mixed_patch_state_with_config(patch(1, -6.0).instrument_config().clone())
    }

    fn mixed_patch_state_with_config(config: InstrumentConfig) -> AppState {
        let mut state = AppState::new(
            production_capability_registry().unwrap(),
            global_parameters(),
        );
        let patch = Patch::new(
            PatchId::new(1).unwrap(),
            "Patch 1".to_owned(),
            config,
            MidiChannel::new(0).unwrap(),
            ChannelParameters::new(-6.0, 0.1, 0.2, 0.3).unwrap(),
        )
        .with_envelope(crate::synth::VoiceEnvelope::new(12.0, 34.0, 0.56, 78.0).unwrap());
        state.apply(AppEvent::InstallPatches(vec![patch])).unwrap();
        state
            .apply(AppEvent::SelectContext(
                crate::control::TopLevelContext::Patch,
            ))
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
                .value(&crate::synth::ParameterId::new(SOUNDFONT_PRESET_PARAMETER_ID).unwrap()),
            Some(&ParameterValue::Choice(
                SoundFontInstrument::new(128, 0, false)
                    .unwrap()
                    .preset_id()
                    .choice_id()
            ))
        );
        assert_eq!(decoded.patches[0].gain_db, -6.0);
        assert_eq!(decoded.global.delay_milliseconds, 375.0);
        assert_eq!(
            decoded.interaction.mixer_selection.section,
            SerializedSelectionSection::Patch
        );
        assert_eq!(decoded.interaction.mixer_selection.patch_index, 0);
        assert_eq!(decoded.interaction.mixer_selection.parameter_index, 0);
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

        assert!(text.body().starts_with(MIXER_HEADER));
        assert!(text
            .body()
            .contains("PATCH id=1 name=Patch 1 channel=0 capability=HiDef SoundFont (instrument.soundfont.hidef)"));
        assert!(text
            .body()
            .contains("Preset (soundfont.preset)=sf2.bank-0.program-0"));
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
        let provider =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
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
        let revision = GraphRevision::new(17).unwrap();
        let state = installed_state_for_graph(revision);
        let (snapshot, page, text, parameters, tree) = StateProjector::for_graph(revision)
            .project_with_tree(&state)
            .unwrap();

        assert!(page.is_none());
        assert_eq!(text.state_hash(), snapshot.hash());
        assert_eq!(parameters.generation(), state.generation());
        assert_eq!(parameters.graph_revision(), revision);
        assert_eq!(tree.generation(), parameters.generation());
        assert_eq!(tree.graph_revision(), revision);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(tree.json()).unwrap()["parameters"]
                ["graphRevision"],
            revision.value()
        );
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
        let revision = GraphRevision::new(23).unwrap();
        let mut state = installed_state_for_graph(revision);
        let projector = StateProjector::for_graph(revision);
        let (snapshot, page, text, parameters, tree) = projector.project_with_tree(&state).unwrap();
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
            .project_midi_generation(&state, &snapshot, page.as_ref(), &text, parameters, &tree)
            .unwrap();
        let eager = projector.project_with_tree(&state).unwrap();

        assert_eq!(fast.0, eager.0);
        assert_eq!(fast.1, eager.1);
        assert_eq!(fast.2, eager.2);
        assert_eq!(fast.3, eager.3);
        assert_eq!(fast.4, eager.4);
        assert_eq!(fast.3.graph_revision(), revision);
        assert_eq!(fast.4.graph_revision(), revision);
    }

    #[test]
    fn patch_projection_and_generation_only_midi_are_context_coherent_and_shared() {
        let revision = GraphRevision::new(29).unwrap();
        let mut state = installed_state_for_graph(revision);
        state
            .apply(AppEvent::SelectContext(
                crate::control::TopLevelContext::Patch,
            ))
            .unwrap();
        let projector = StateProjector::for_graph(revision);
        let (snapshot, page, text, parameters, tree) = projector.project_with_tree(&state).unwrap();
        let page = page.expect("PATCH context projects one focused page");

        assert_eq!(page.patch().id(), state.patches()[0].id());
        assert_eq!(page.state_hash(), snapshot.hash());
        assert_eq!(text.context(), crate::control::TopLevelContext::Patch);
        assert!(text.body().starts_with(PATCH_HEADER));
        assert_eq!(text.state_hash(), snapshot.hash());
        let tree_value: serde_json::Value = serde_json::from_str(tree.json()).unwrap();
        assert_eq!(tree_value["interaction"]["context"], "patch");
        assert_eq!(tree_value["projection"]["context"], "patch");
        assert_eq!(
            tree_value["patchPage"]["patch"]["id"],
            page.patch().id().value()
        );

        let body_address = text.body().as_ptr();
        let message = MidiMessage::try_new(
            state.patches()[0].channel(),
            MidiMessageKind::NoteOn,
            60,
            100,
        )
        .unwrap();
        state
            .apply(AppEvent::Midi {
                patch_id: state.patches()[0].id(),
                message,
            })
            .unwrap();
        let fast = projector
            .project_midi_generation(&state, &snapshot, Some(&page), &text, parameters, &tree)
            .unwrap();
        let eager = projector.project_with_tree(&state).unwrap();

        assert_eq!(fast, eager);
        let fast_page = fast.1.as_ref().unwrap();
        assert!(page.shares_content_with(fast_page));
        assert_eq!(fast.2.body().as_ptr(), body_address);
        assert_eq!(fast_page.state_hash(), fast.0.hash());
        assert_eq!(fast.2.state_hash(), fast.0.hash());
        assert_eq!(fast.3.generation(), state.generation());
        assert_eq!(fast.4.generation(), state.generation());
    }

    #[test]
    fn patch_text_marker_and_selected_line_follow_every_canonical_control() {
        let mut state = mixed_patch_state();
        for (index, expected_control) in crate::control::PatchControlId::surface_descriptor()
            .iter()
            .cloned()
            .enumerate()
        {
            let (_, page, text, _, _) = StateProjector::new().project_with_tree(&state).unwrap();
            let page = page.unwrap();
            assert_eq!(page.focused_control_id(), expected_control);
            assert_eq!(text.selected_line(), index + 2);
            assert_eq!(
                text.body()
                    .lines()
                    .filter(|line| line.starts_with('>'))
                    .count(),
                1
            );
            assert!(text
                .body()
                .lines()
                .nth(text.selected_line())
                .unwrap()
                .contains(expected_control.as_str().as_ref()));

            if index + 1 < crate::control::PatchControlId::surface_descriptor().len() {
                state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
            }
        }

        let mut preparing = mixed_patch_state();
        preparing.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        let (_, page, text, _, _) = StateProjector::new().project_with_tree(&preparing).unwrap();
        let page = page.unwrap();
        assert_eq!(
            page.focused_control_id(),
            crate::control::PatchControlId::Engine
        );
        assert!(!page.engine().editable());
        assert_eq!(text.selected_line(), 2);
        assert!(text.body().lines().nth(2).unwrap().starts_with("> ENGINE"));
    }

    #[test]
    fn patch_projection_is_generic_for_both_engines_every_focus_and_context_round_trip() {
        let soundfont = patch(1, -6.0).instrument_config().clone();
        let braids = BraidsCapability::new().unwrap().default_config().unwrap();

        for config in [soundfont, braids] {
            let mut state = mixed_patch_state_with_config(config);
            let descriptor = state
                .capabilities()
                .descriptor(state.patches()[0].instrument_config().capability_id())
                .unwrap()
                .clone();
            let controls = crate::control::PatchControlId::resolve(
                &descriptor,
                state.patches()[0].instrument_config(),
            );

            for (index, control) in controls.iter().cloned().enumerate() {
                let (_, page, text, _, tree) =
                    StateProjector::new().project_with_tree(&state).unwrap();
                let page = page.unwrap();
                assert_eq!(page.focused_control_id(), control);
                assert_eq!(
                    text.body()
                        .lines()
                        .filter(|line| line.starts_with('>'))
                        .count(),
                    1
                );
                assert_eq!(page.sections().len(), descriptor.sections().len());
                for (section, section_spec) in page.sections().iter().zip(descriptor.sections()) {
                    assert_eq!(section.id(), section_spec.id());
                    for (row, spec) in section.parameters().iter().zip(section_spec.parameters()) {
                        assert_eq!(row.id(), spec.id());
                        assert!(text.body().contains(spec.id().as_str()));
                        assert_eq!(
                            row.editable(),
                            spec.patch_interaction() == PatchInteraction::StructuralChoice
                        );
                    }
                }
                let tree_json: serde_json::Value = serde_json::from_str(tree.json()).unwrap();
                assert_eq!(
                    tree_json["patchPage"]["focusedControlId"],
                    control.as_str().as_ref()
                );

                if index + 1 < controls.len() {
                    state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
                }
            }

            let retained = state.interaction().patch_control_focus();
            let retained_line = StateProjector::new()
                .project_with_tree(&state)
                .unwrap()
                .2
                .selected_line();
            state
                .apply(AppEvent::SelectContext(
                    crate::control::TopLevelContext::Mixer,
                ))
                .unwrap();
            assert!(StateProjector::new()
                .project_with_tree(&state)
                .unwrap()
                .1
                .is_none());
            state
                .apply(AppEvent::SelectContext(
                    crate::control::TopLevelContext::Patch,
                ))
                .unwrap();
            let (_, page, text, _, _) = StateProjector::new().project_with_tree(&state).unwrap();
            assert_eq!(page.unwrap().focused_control_id(), retained.unwrap());
            assert_eq!(text.selected_line(), retained_line);
        }
    }

    #[test]
    fn every_engine_selection_generation_is_cross_projection_coherent() {
        let projector = StateProjector::new();
        let mut state = mixed_patch_state();
        let source_config = state.patches()[0].instrument_config().clone();
        let source_revision = GraphRevision::INITIAL;

        let assert_projection =
            |state: &AppState,
             expected_kind: EngineSelectionStatusKind,
             expected_projection_revision: GraphRevision,
             expected_editable: bool,
             expected_failure: Option<EngineSelectionFailure>| {
                let (snapshot, page, text, parameters, tree) =
                    projector.project_with_tree(state).unwrap();
                let page = page.expect("PATCH context always projects its focused page");
                let snapshot_value: serde_json::Value =
                    serde_json::from_str(snapshot.json()).unwrap();
                let tree_value: serde_json::Value = serde_json::from_str(tree.json()).unwrap();
                let focused_control = state.interaction().patch_control_focus().unwrap();
                let focused_index = crate::control::PatchControlId::surface_descriptor()
                    .iter()
                    .position(|control| *control == focused_control)
                    .unwrap();

                assert_eq!(page.engine().status(), expected_kind);
                assert_eq!(page.focused_control_id(), focused_control);
                assert_eq!(page.engine().editable(), expected_editable);
                assert_eq!(page.engine().failure(), expected_failure);
                assert_eq!(parameters.generation(), state.generation());
                assert_eq!(parameters.graph_revision(), expected_projection_revision);
                assert_eq!(tree.generation(), state.generation());
                assert_eq!(tree.graph_revision(), expected_projection_revision);
                assert_eq!(page.state_hash(), snapshot.hash());
                assert_eq!(text.state_hash(), snapshot.hash());
                assert_eq!(tree.state_hash(), snapshot.hash());
                assert_eq!(text.selected_line(), focused_index + 2);
                assert!(text
                    .body()
                    .lines()
                    .nth(text.selected_line())
                    .unwrap()
                    .starts_with('>'));
                assert_eq!(
                    snapshot_value["engineSelection"],
                    tree_value["engineSelection"]
                );
                assert_eq!(
                    serde_json::to_value(page.engine()).unwrap(),
                    tree_value["patchPage"]["engine"]
                );
                assert_eq!(
                    tree_value["parameters"]["graphRevision"],
                    expected_projection_revision.value()
                );
                assert!(text
                    .body()
                    .contains(&format!("\"status\":\"{}\"", expected_kind.name())));
                assert_eq!(
                    text.body()
                        .lines()
                        .find(|line| line.contains("ENGINE"))
                        .unwrap()
                        .starts_with('>'),
                    page.focused_control_id() == crate::control::PatchControlId::Engine
                );
                if let Some(failure) = expected_failure {
                    assert!(text
                        .body()
                        .contains(&format!("\"failure\":\"{}\"", failure.name())));
                }
            };

        assert_projection(
            &state,
            EngineSelectionStatusKind::Ready,
            source_revision,
            true,
            None,
        );

        let ready_generation = state.generation();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        assert_eq!(state.generation(), ready_generation + 1);
        assert_eq!(state.patches()[0].instrument_config(), &source_config);
        assert_projection(
            &state,
            EngineSelectionStatusKind::Preparing,
            source_revision,
            false,
            None,
        );

        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_projection(
            &state,
            EngineSelectionStatusKind::Preparing,
            source_revision,
            false,
            None,
        );

        let failed_correlation = state.engine_selection().correlation().unwrap().clone();
        let target_revision = source_revision.checked_next().unwrap();
        state
            .apply(AppEvent::EnginePreparationFailed {
                request_id: failed_correlation.request_id(),
                patch_id: failed_correlation.patch_id(),
                intent: failed_correlation.intent().clone(),
                source_capability_id: failed_correlation.source_capability_id().clone(),
                target_capability_id: failed_correlation.target_capability_id().clone(),
                source_graph_revision: failed_correlation.source_graph_revision(),
                target_graph_revision: target_revision,
                failure: EngineSelectionFailure::WorkerUnavailable,
            })
            .unwrap();
        assert_eq!(state.patches()[0].instrument_config(), &source_config);
        assert_projection(
            &state,
            EngineSelectionStatusKind::Failed,
            source_revision,
            true,
            Some(EngineSelectionFailure::WorkerUnavailable),
        );

        state.apply(AppEvent::Navigate(Direction::Up)).unwrap();
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_projection(
            &state,
            EngineSelectionStatusKind::Preparing,
            source_revision,
            false,
            None,
        );
        let correlation = state.engine_selection().correlation().unwrap().clone();
        let candidate_config = BraidsCapability::new().unwrap().default_config().unwrap();
        state
            .apply(AppEvent::EnginePrepared {
                request_id: correlation.request_id(),
                patch_id: correlation.patch_id(),
                intent: correlation.intent().clone(),
                source_capability_id: correlation.source_capability_id().clone(),
                target_capability_id: correlation.target_capability_id().clone(),
                source_graph_revision: correlation.source_graph_revision(),
                target_graph_revision: target_revision,
                candidate_config,
            })
            .unwrap();
        assert_eq!(
            state.patches()[0]
                .instrument_config()
                .capability_id()
                .as_str(),
            BRAIDS_CAPABILITY_ID
        );
        assert_projection(
            &state,
            EngineSelectionStatusKind::Activating,
            target_revision,
            false,
            None,
        );

        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_projection(
            &state,
            EngineSelectionStatusKind::Activating,
            target_revision,
            false,
            None,
        );

        state
            .apply(AppEvent::EngineActivationAcknowledged {
                request_id: correlation.request_id(),
                intent: correlation.intent().clone(),
                target_graph_revision: target_revision,
                retired_graph_revision: source_revision,
                collected: true,
            })
            .unwrap();
        assert_projection(
            &state,
            EngineSelectionStatusKind::Ready,
            target_revision,
            true,
            None,
        );
    }

    #[test]
    fn projector_rejects_a_tree_projection_for_another_graph_revision() {
        let first_revision = GraphRevision::new(31).unwrap();
        let second_revision = GraphRevision::new(32).unwrap();
        let first_state = installed_state_for_graph(first_revision);
        let second_state = installed_state_for_graph(second_revision);
        let first = StateProjector::for_graph(first_revision);
        let second = StateProjector::for_graph(second_revision);
        let (_, _, parameters) = first.project(&first_state).unwrap();
        let (snapshot, text, _) = second.project(&second_state).unwrap();

        assert_eq!(
            second.state_tree(&snapshot, &text, &parameters),
            Err(StateProjectionError::StateTree(
                StateTreeError::GraphRevisionMismatch
            ))
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
