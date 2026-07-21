use crate::control::serialized_state::{
    SerializedGlobalParameters, SerializedPatch, SerializedSelection, SerializedState,
};
use crate::control::state_snapshot::StateSnapshot;
use crate::control::text_projection::TextProjection;
use crate::mixer::channel_parameters::ChannelParameters;
use crate::mixer::global_parameters::GlobalParameters;
use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
use crate::synth::instrument_capability::{CapabilityRegistry, InstrumentConfig};
use core::fmt;
use serde::Serialize;
use std::sync::{Arc, OnceLock};

/// A coherence violation while constructing the canonical observation tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateTreeError {
    /// The accepted state snapshot was not valid canonical state JSON.
    StateDeserialization,
    /// The text projection did not originate from the supplied state snapshot.
    ProjectionHashMismatch,
    /// The audio parameters did not originate from the accepted generation.
    GenerationMismatch,
    /// The state and real-time projections contained different Patch counts.
    PatchCountMismatch,
    /// A real-time Patch identity did not match the state at the same position.
    PatchIdentityMismatch { index: usize },
    /// A real-time Patch parameter set did not match the serialized state.
    PatchParametersMismatch { index: usize },
    /// The global real-time parameters did not match the serialized state.
    GlobalParametersMismatch,
    /// The complete tree could not be serialized.
    Serialization,
    /// A generation-only projection could not reuse the canonical tree shape.
    MidiTemplateMismatch,
}

impl fmt::Display for StateTreeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::StateDeserialization => {
                formatter.write_str("accepted state snapshot could not be decoded")
            }
            Self::ProjectionHashMismatch => {
                formatter.write_str("text projection does not match the state snapshot")
            }
            Self::GenerationMismatch => {
                formatter.write_str("parameter generation does not match accepted state")
            }
            Self::PatchCountMismatch => {
                formatter.write_str("parameter Patch count does not match accepted state")
            }
            Self::PatchIdentityMismatch { index } => {
                write!(
                    formatter,
                    "parameter Patch identity differs at index {index}"
                )
            }
            Self::PatchParametersMismatch { index } => {
                write!(formatter, "parameter Patch values differ at index {index}")
            }
            Self::GlobalParametersMismatch => {
                formatter.write_str("global parameter values do not match accepted state")
            }
            Self::Serialization => formatter.write_str("state tree could not be serialized"),
            Self::MidiTemplateMismatch => {
                formatter.write_str("state tree generation template does not match canonical JSON")
            }
        }
    }
}

impl std::error::Error for StateTreeError {}

/// A canonical, LLM-readable tree of one complete accepted control generation.
///
/// Construction consumes only immutable projections already derived from the
/// same accepted AppState. It verifies their shared identity before producing
/// deterministic JSON with stable property names.
#[derive(Clone, Debug)]
pub struct StateTree {
    json: TreeJson,
    generation: u64,
    patch_count: usize,
    selected_line: usize,
    state_hash: Arc<str>,
}

#[derive(Clone, Debug)]
enum TreeJson {
    Ready(Arc<str>),
    MidiGeneration {
        template: Arc<MidiTreeTemplate>,
        generation: u64,
        state_hash: Arc<str>,
        rendered: Arc<OnceLock<String>>,
    },
}

#[derive(Debug)]
struct MidiTreeTemplate {
    before_root_generation: Arc<str>,
    before_state_hash: Arc<str>,
    before_parameter_generation: Arc<str>,
    after_parameter_generation: Arc<str>,
}

impl StateTree {
    /// The stable schema version emitted in every serialized tree.
    pub const SCHEMA_VERSION: u32 = 2;
    pub const SERIALIZED_PROPERTY_DESCRIPTOR: &'static [&'static str] = &[
        "schemaVersion",
        "generation",
        "capabilities",
        "patches",
        "global",
        "selection.section",
        "selection.patchIndex",
        "selection.parameterIndex",
        "projection.body",
        "projection.selectedLine",
        "projection.stateHash",
        "parameters.generation",
        "parameters.patchCount",
        "parameters.patches",
        "parameters.global",
    ];
    /// The complete normalized nested schema, including tagged capability values.
    pub const SERIALIZED_LEAF_DESCRIPTOR: &'static [&'static str] = &[
        "schemaVersion",
        "generation",
        "capabilities.descriptors[].id",
        "capabilities.descriptors[].label",
        "capabilities.descriptors[].semanticAccent",
        "capabilities.descriptors[].sections[].id",
        "capabilities.descriptors[].sections[].label",
        "capabilities.descriptors[].sections[].parameters[].id",
        "capabilities.descriptors[].sections[].parameters[].label",
        "capabilities.descriptors[].sections[].parameters[].kind",
        "capabilities.descriptors[].sections[].parameters[].update",
        "capabilities.descriptors[].sections[].parameters[].defaultValue.kind",
        "capabilities.descriptors[].sections[].parameters[].defaultValue.value.kind",
        "capabilities.descriptors[].sections[].parameters[].defaultValue.value.value",
        "capabilities.descriptors[].sections[].parameters[].defaultValue.value.locator",
        "capabilities.descriptors[].sections[].parameters[].range.minimum",
        "capabilities.descriptors[].sections[].parameters[].range.maximum",
        "capabilities.descriptors[].sections[].parameters[].choices[].id",
        "capabilities.descriptors[].sections[].parameters[].choices[].label",
        "capabilities.descriptors[].sections[].parameters[].fineStep",
        "capabilities.descriptors[].sections[].parameters[].coarseStep",
        "capabilities.descriptors[].sections[].parameters[].unit",
        "capabilities.descriptors[].sections[].parameters[].formatter",
        "capabilities.descriptors[].sections[].parameters[].enabledWhen.parameterId",
        "capabilities.descriptors[].sections[].parameters[].enabledWhen.equals.kind",
        "capabilities.descriptors[].sections[].parameters[].enabledWhen.equals.value",
        "capabilities.descriptors[].sections[].parameters[].visibleWhen.parameterId",
        "capabilities.descriptors[].sections[].parameters[].visibleWhen.equals.kind",
        "capabilities.descriptors[].sections[].parameters[].visibleWhen.equals.value",
        "capabilities.descriptors[].assetRequirements[].parameterId",
        "capabilities.descriptors[].assetRequirements[].required",
        "capabilities.descriptors[].voiceLimit",
        "capabilities.descriptors[].supportedMidiKinds[]",
        "patches[].id",
        "patches[].name",
        "patches[].channel",
        "patches[].instrument.capabilityId",
        "patches[].instrument.values[].parameterId",
        "patches[].instrument.values[].value.kind",
        "patches[].instrument.values[].value.value",
        "patches[].instrument.assetReferences[].parameterId",
        "patches[].instrument.assetReferences[].reference.kind",
        "patches[].instrument.assetReferences[].reference.locator",
        "patches[].parameters.gainDb",
        "patches[].parameters.pan",
        "patches[].parameters.reverbSend",
        "patches[].parameters.delaySend",
        "global.masterGainDb",
        "global.reverbRoomSize",
        "global.reverbDamping",
        "global.reverbReturn",
        "global.delayMilliseconds",
        "global.delayFeedback",
        "global.delayReturn",
        "selection.section",
        "selection.patchIndex",
        "selection.parameterIndex",
        "projection.body",
        "projection.selectedLine",
        "projection.stateHash",
        "parameters.generation",
        "parameters.patchCount",
        "parameters.patches[].patchId",
        "parameters.patches[].parameters.gainDb",
        "parameters.patches[].parameters.pan",
        "parameters.patches[].parameters.reverbSend",
        "parameters.patches[].parameters.delaySend",
        "parameters.global.masterGainDb",
        "parameters.global.reverbRoomSize",
        "parameters.global.reverbDamping",
        "parameters.global.reverbReturn",
        "parameters.global.delayMilliseconds",
        "parameters.global.delayFeedback",
        "parameters.global.delayReturn",
    ];

    /// Returns the production-owned stable StateTree property surface.
    pub const fn serialized_property_descriptor() -> &'static [&'static str] {
        Self::SERIALIZED_PROPERTY_DESCRIPTOR
    }

    /// Returns the complete production-owned normalized nested schema.
    pub const fn serialized_leaf_descriptor() -> &'static [&'static str] {
        Self::SERIALIZED_LEAF_DESCRIPTOR
    }

    /// Builds one observation tree from a state snapshot and its GUI/audio
    /// projections.
    pub fn new(
        snapshot: &StateSnapshot,
        projection: &TextProjection,
        parameters: &ParameterSnapshot,
    ) -> Result<Self, StateTreeError> {
        let state: SerializedState<'_> = serde_json::from_str(snapshot.json())
            .map_err(|_| StateTreeError::StateDeserialization)?;

        Self::from_serialized_state(&state, snapshot, projection, parameters)
    }

    /// Builds the production tree from the canonical typed state that produced
    /// the supplied snapshot. This internal path preserves all coherence checks
    /// without parsing Crest's own JSON back into a second state copy.
    pub(crate) fn from_serialized_state(
        state: &SerializedState<'_>,
        snapshot: &StateSnapshot,
        projection: &TextProjection,
        parameters: &ParameterSnapshot,
    ) -> Result<Self, StateTreeError> {
        if projection.state_hash() != snapshot.hash() {
            return Err(StateTreeError::ProjectionHashMismatch);
        }
        validate_parameter_projection(state, parameters)?;

        let serializable =
            SerializableStateTree::new(state, projection, parameters, snapshot.hash());
        let json =
            serde_json::to_string(&serializable).map_err(|_| StateTreeError::Serialization)?;

        Ok(Self {
            json: TreeJson::Ready(Arc::from(json)),
            generation: state.generation,
            patch_count: state.patches.len(),
            selected_line: projection.selected_line(),
            state_hash: Arc::from(snapshot.hash()),
        })
    }

    /// Advances a coherent tree whose accepted state changed only by MIDI generation.
    pub(crate) fn with_midi_generation(
        &self,
        snapshot: &StateSnapshot,
        projection: &TextProjection,
        parameters: &ParameterSnapshot,
    ) -> Result<Self, StateTreeError> {
        if projection.state_hash() != snapshot.hash() {
            return Err(StateTreeError::ProjectionHashMismatch);
        }
        if parameters.generation() != self.generation.saturating_add(1) {
            return Err(StateTreeError::GenerationMismatch);
        }
        if parameters.patch_count() != self.patch_count {
            return Err(StateTreeError::PatchCountMismatch);
        }
        if projection.selected_line() != self.selected_line {
            return Err(StateTreeError::MidiTemplateMismatch);
        }

        let template = match &self.json {
            TreeJson::MidiGeneration { template, .. } => Arc::clone(template),
            TreeJson::Ready(json) => Arc::new(
                MidiTreeTemplate::from_json(json, self.generation, self.state_hash())
                    .ok_or(StateTreeError::MidiTemplateMismatch)?,
            ),
        };
        let generation = parameters.generation();
        let state_hash: Arc<str> = Arc::from(snapshot.hash());

        Ok(Self {
            json: TreeJson::MidiGeneration {
                template,
                generation,
                state_hash: Arc::clone(&state_hash),
                rendered: Arc::new(OnceLock::new()),
            },
            generation,
            patch_count: self.patch_count,
            selected_line: self.selected_line,
            state_hash,
        })
    }

    /// Returns the stable JSON schema version.
    pub const fn schema_version(&self) -> u32 {
        Self::SCHEMA_VERSION
    }

    /// Returns the accepted AppState generation represented by the tree.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the number of installed Patches represented by both projections.
    pub const fn patch_count(&self) -> usize {
        self.patch_count
    }

    /// Returns the selected line in the included text projection.
    pub const fn selected_line(&self) -> usize {
        self.selected_line
    }

    /// Returns the canonical StateSnapshot identity included in the tree.
    pub fn state_hash(&self) -> &str {
        &self.state_hash
    }

    /// Returns deterministic JSON containing every control and projection
    /// property.
    pub fn json(&self) -> &str {
        match &self.json {
            TreeJson::Ready(json) => json,
            TreeJson::MidiGeneration {
                template,
                generation,
                state_hash,
                rendered,
            } => rendered.get_or_init(|| template.render(*generation, state_hash)),
        }
    }

    /// Consumes the value and returns its deterministic JSON representation.
    pub fn into_json(self) -> String {
        self.json().to_owned()
    }
}

impl PartialEq for StateTree {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation
            && self.patch_count == other.patch_count
            && self.selected_line == other.selected_line
            && self.state_hash == other.state_hash
            && self.json() == other.json()
    }
}

impl MidiTreeTemplate {
    fn from_json(json: &str, generation: u64, state_hash: &str) -> Option<Self> {
        const ROOT_MARKER: &str = "{\"schemaVersion\":2,\"generation\":";
        const HASH_MARKER: &str = "\"stateHash\":\"";
        const PARAMETER_MARKER: &str = "\"parameters\":{\"generation\":";

        let root_start = ROOT_MARKER.len();
        let root_end = json.get(root_start..)?.find(',')? + root_start;
        if json.get(root_start..root_end)?.parse::<u64>().ok()? != generation {
            return None;
        }

        let hash_marker_start = json.get(root_end..)?.find(HASH_MARKER)? + root_end;
        let hash_start = hash_marker_start + HASH_MARKER.len();
        let hash_end = json.get(hash_start..)?.find('"')? + hash_start;
        if json.get(hash_start..hash_end)? != state_hash {
            return None;
        }

        let parameter_marker_start = json.get(hash_end..)?.find(PARAMETER_MARKER)? + hash_end;
        let parameter_start = parameter_marker_start + PARAMETER_MARKER.len();
        let parameter_end = json.get(parameter_start..)?.find(',')? + parameter_start;
        if json
            .get(parameter_start..parameter_end)?
            .parse::<u64>()
            .ok()?
            != generation
        {
            return None;
        }

        Some(Self {
            before_root_generation: Arc::from(json.get(..root_start)?),
            before_state_hash: Arc::from(json.get(root_end..hash_start)?),
            before_parameter_generation: Arc::from(json.get(hash_end..parameter_start)?),
            after_parameter_generation: Arc::from(json.get(parameter_end..)?),
        })
    }

    fn render(&self, generation: u64, state_hash: &str) -> String {
        let generation = generation.to_string();
        let mut json = String::with_capacity(
            self.before_root_generation.len()
                + self.before_state_hash.len()
                + self.before_parameter_generation.len()
                + self.after_parameter_generation.len()
                + generation.len() * 2
                + state_hash.len(),
        );
        json.push_str(&self.before_root_generation);
        json.push_str(&generation);
        json.push_str(&self.before_state_hash);
        json.push_str(state_hash);
        json.push_str(&self.before_parameter_generation);
        json.push_str(&generation);
        json.push_str(&self.after_parameter_generation);
        json
    }
}

fn validate_parameter_projection(
    state: &SerializedState<'_>,
    parameters: &ParameterSnapshot,
) -> Result<(), StateTreeError> {
    if parameters.generation() != state.generation {
        return Err(StateTreeError::GenerationMismatch);
    }
    if parameters.patch_count() != state.patches.len() {
        return Err(StateTreeError::PatchCountMismatch);
    }

    for (index, (state_patch, parameter_patch)) in
        state.patches.iter().zip(parameters.patches()).enumerate()
    {
        if parameter_patch.patch_id().map(|patch_id| patch_id.value()) != Some(state_patch.id) {
            return Err(StateTreeError::PatchIdentityMismatch { index });
        }
        if TreeChannelParameters::from(state_patch)
            != TreeChannelParameters::from(parameter_patch.parameters())
        {
            return Err(StateTreeError::PatchParametersMismatch { index });
        }
    }

    if TreeGlobalParameters::from(&state.global) != TreeGlobalParameters::from(parameters.global())
    {
        return Err(StateTreeError::GlobalParametersMismatch);
    }

    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializableStateTree<'a> {
    schema_version: u32,
    generation: u64,
    capabilities: &'a CapabilityRegistry,
    patches: Vec<TreePatch<'a>>,
    global: TreeGlobalParameters,
    selection: &'a SerializedSelection,
    projection: TreeProjection<'a>,
    parameters: TreeParameterSnapshot,
}

impl<'a> SerializableStateTree<'a> {
    fn new(
        state: &'a SerializedState<'_>,
        projection: &'a TextProjection,
        parameters: &ParameterSnapshot,
        state_hash: &'a str,
    ) -> Self {
        Self {
            schema_version: StateTree::SCHEMA_VERSION,
            generation: state.generation,
            capabilities: state.capabilities.as_ref(),
            patches: state.patches.iter().map(TreePatch::from).collect(),
            global: TreeGlobalParameters::from(&state.global),
            selection: &state.selection,
            projection: TreeProjection {
                body: projection.body(),
                selected_line: projection.selected_line(),
                state_hash,
            },
            parameters: TreeParameterSnapshot {
                generation: parameters.generation(),
                patch_count: parameters.patch_count(),
                patches: parameters
                    .patches()
                    .iter()
                    .map(TreeParameterPatch::from)
                    .collect(),
                global: TreeGlobalParameters::from(parameters.global()),
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TreePatch<'a> {
    id: u32,
    name: &'a str,
    channel: u8,
    instrument: &'a InstrumentConfig,
    parameters: TreeChannelParameters,
}

impl<'a> From<&'a SerializedPatch<'_>> for TreePatch<'a> {
    fn from(patch: &'a SerializedPatch<'_>) -> Self {
        Self {
            id: patch.id,
            name: patch.name.as_ref(),
            channel: patch.channel,
            instrument: patch.instrument.as_ref(),
            parameters: TreeChannelParameters::from(patch),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeChannelParameters {
    gain_db: f32,
    pan: f32,
    reverb_send: f32,
    delay_send: f32,
}

impl From<&SerializedPatch<'_>> for TreeChannelParameters {
    fn from(patch: &SerializedPatch<'_>) -> Self {
        Self {
            gain_db: patch.gain_db,
            pan: patch.pan,
            reverb_send: patch.reverb_send,
            delay_send: patch.delay_send,
        }
    }
}

impl From<&ChannelParameters> for TreeChannelParameters {
    fn from(parameters: &ChannelParameters) -> Self {
        Self {
            gain_db: parameters.gain_db(),
            pan: parameters.pan(),
            reverb_send: parameters.reverb_send(),
            delay_send: parameters.delay_send(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeGlobalParameters {
    master_gain_db: f32,
    reverb_room_size: f32,
    reverb_damping: f32,
    reverb_return: f32,
    delay_milliseconds: f32,
    delay_feedback: f32,
    delay_return: f32,
}

impl From<&SerializedGlobalParameters> for TreeGlobalParameters {
    fn from(parameters: &SerializedGlobalParameters) -> Self {
        Self {
            master_gain_db: parameters.master_gain_db,
            reverb_room_size: parameters.reverb_room_size,
            reverb_damping: parameters.reverb_damping,
            reverb_return: parameters.reverb_return,
            delay_milliseconds: parameters.delay_milliseconds,
            delay_feedback: parameters.delay_feedback,
            delay_return: parameters.delay_return,
        }
    }
}

impl From<&GlobalParameters> for TreeGlobalParameters {
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeProjection<'a> {
    body: &'a str,
    selected_line: usize,
    state_hash: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeParameterSnapshot {
    generation: u64,
    patch_count: usize,
    patches: Vec<TreeParameterPatch>,
    global: TreeGlobalParameters,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeParameterPatch {
    patch_id: u32,
    parameters: TreeChannelParameters,
}

impl From<&RtPatchParameters> for TreeParameterPatch {
    fn from(patch: &RtPatchParameters) -> Self {
        Self {
            patch_id: patch
                .patch_id()
                .expect("active ParameterSnapshot entries always carry a PatchId")
                .value(),
            parameters: TreeChannelParameters::from(patch.parameters()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{StateTree, StateTreeError};
    use crate::adapter::hidef_soundfont_capability::{
        HiDefSoundFontCapability, HIDEF_CAPABILITY_ID,
    };
    use crate::control::state_snapshot::StateSnapshot;
    use crate::control::text_projection::TextProjection;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::testing::automatic_midi_test::create_soundfont_config;
    use serde_json::{json, Value};

    fn snapshot() -> StateSnapshot {
        let provider = HiDefSoundFontCapability::new().unwrap();
        StateSnapshot::new(
            json!({
                "generation": 42,
                "capabilities": provider.registry().unwrap(),
                "patches": [
                    {
                        "id": 7,
                        "name": "Lead",
                        "channel": 2,
                        "instrument": create_soundfont_config(
                            &provider,
                            SoundFontInstrument::new(0, 80, false).unwrap()
                        ).unwrap(),
                        "gainDb": -6.0,
                        "pan": -0.25,
                        "reverbSend": 0.2,
                        "delaySend": 0.1
                    },
                    {
                        "id": 9,
                        "name": "Drums",
                        "channel": 9,
                        "instrument": create_soundfont_config(
                            &provider,
                            SoundFontInstrument::new(128, 0, true).unwrap()
                        ).unwrap(),
                        "gainDb": -12.0,
                        "pan": 0.5,
                        "reverbSend": 0.4,
                        "delaySend": 0.3
                    }
                ],
                "global": {
                    "masterGainDb": -3.0,
                    "reverbRoomSize": 0.7,
                    "reverbDamping": 0.4,
                    "reverbReturn": 0.25,
                    "delayMilliseconds": 375.0,
                    "delayFeedback": 0.35,
                    "delayReturn": 0.2
                },
                "selection": {"section": "Patch", "patchIndex": 1, "parameterIndex": 2}
            })
            .to_string(),
        )
    }

    fn global() -> GlobalParameters {
        GlobalParameters::new(-3.0, 0.7, 0.4, 0.25, 375.0, 0.35, 0.2).unwrap()
    }

    fn parameters() -> ParameterSnapshot {
        ParameterSnapshot::new(
            42,
            global(),
            &[
                RtPatchParameters::new(
                    PatchId::new(7).unwrap(),
                    ChannelParameters::new(-6.0, -0.25, 0.2, 0.1).unwrap(),
                ),
                RtPatchParameters::new(
                    PatchId::new(9).unwrap(),
                    ChannelParameters::new(-12.0, 0.5, 0.4, 0.3).unwrap(),
                ),
            ],
        )
        .unwrap()
    }

    fn projection(snapshot: &StateSnapshot) -> TextProjection {
        TextProjection::new(
            "PATCH Lead\n> reverbSend=0.4\nGLOBAL".to_owned(),
            1,
            snapshot.hash().to_owned(),
        )
    }

    #[test]
    fn serializes_every_state_text_and_audio_property_with_stable_names() {
        let snapshot = snapshot();
        let tree = StateTree::new(&snapshot, &projection(&snapshot), &parameters()).unwrap();
        let value: Value = serde_json::from_str(tree.json()).unwrap();

        assert_eq!(tree.schema_version(), 2);
        assert_eq!(tree.generation(), 42);
        assert_eq!(tree.patch_count(), 2);
        assert_eq!(tree.selected_line(), 1);
        assert_eq!(tree.state_hash(), snapshot.hash());

        let root = value.as_object().unwrap();
        assert_eq!(root.len(), 8);
        for property in [
            "schemaVersion",
            "generation",
            "capabilities",
            "patches",
            "global",
            "selection",
            "projection",
            "parameters",
        ] {
            assert!(root.contains_key(property), "missing {property}");
        }

        assert_eq!(
            value["patches"][0],
            json!({
                "id": 7,
                "name": "Lead",
                "channel": 2,
                "instrument": value["patches"][0]["instrument"].clone(),
                "parameters": {
                    "gainDb": -6.0,
                    "pan": -0.25,
                    "reverbSend": 0.2,
                    "delaySend": 0.1
                }
            })
        );
        assert_eq!(
            value["capabilities"]["descriptors"][0]["id"],
            HIDEF_CAPABILITY_ID
        );
        assert_eq!(
            value["patches"][0]["instrument"]["capabilityId"],
            HIDEF_CAPABILITY_ID
        );
        assert_eq!(
            value["patches"][1]["instrument"]["values"][0]["value"],
            json!({"kind": "stepped", "value": 128})
        );
        assert_eq!(
            value["patches"][1]["instrument"]["values"][2]["value"],
            json!({"kind": "toggle", "value": true})
        );
        assert_eq!(
            value["global"],
            json!({
                "masterGainDb": -3.0,
                "reverbRoomSize": 0.7,
                "reverbDamping": 0.4,
                "reverbReturn": 0.25,
                "delayMilliseconds": 375.0,
                "delayFeedback": 0.35,
                "delayReturn": 0.2
            })
        );
        assert_eq!(
            value["selection"],
            json!({"section": "Patch", "patchIndex": 1, "parameterIndex": 2})
        );
        assert_eq!(
            value["projection"]["body"],
            "PATCH Lead\n> reverbSend=0.4\nGLOBAL"
        );
        assert_eq!(value["projection"]["selectedLine"], 1);
        assert_eq!(value["projection"]["stateHash"], snapshot.hash());
        assert_eq!(value["parameters"]["generation"], 42);
        assert_eq!(value["parameters"]["patchCount"], 2);
        assert_eq!(
            value["parameters"]["patches"][1],
            json!({
                "patchId": 9,
                "parameters": {
                    "gainDb": -12.0,
                    "pan": 0.5,
                    "reverbSend": 0.4,
                    "delaySend": 0.3
                }
            })
        );
        assert_eq!(value["parameters"]["global"], value["global"]);
    }

    #[test]
    fn serialization_is_deterministic_and_consumable_as_an_owned_value() {
        let snapshot = snapshot();
        let first = StateTree::new(&snapshot, &projection(&snapshot), &parameters()).unwrap();
        let second = StateTree::new(&snapshot, &projection(&snapshot), &parameters()).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.json(), second.json());
        assert!(first
            .json()
            .starts_with("{\"schemaVersion\":2,\"generation\":42,\"capabilities\":"));
        assert_eq!(first.clone().into_json(), first.json());
    }

    #[test]
    fn nested_schema_descriptor_covers_registry_configs_and_tagged_values_once() {
        let descriptor = StateTree::serialized_leaf_descriptor();
        let unique = descriptor
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), descriptor.len());
        for required in [
            "capabilities.descriptors[].sections[].parameters[].defaultValue.value.kind",
            "patches[].instrument.values[].value.kind",
            "patches[].instrument.assetReferences[].reference.locator",
            "parameters.patches[].parameters.gainDb",
        ] {
            assert!(unique.contains(required), "missing {required}");
        }
    }

    #[test]
    fn rejects_a_text_projection_from_another_snapshot() {
        let snapshot = snapshot();
        let other_projection =
            TextProjection::new("unrelated".to_owned(), 0, "other-hash".to_owned());

        assert_eq!(
            StateTree::new(&snapshot, &other_projection, &parameters()),
            Err(StateTreeError::ProjectionHashMismatch)
        );
    }

    #[test]
    fn rejects_parameter_generation_patch_order_and_values_that_do_not_match() {
        let snapshot = snapshot();
        let projection = projection(&snapshot);
        let wrong_generation =
            ParameterSnapshot::new(43, global(), parameters().patches()).unwrap();
        assert_eq!(
            StateTree::new(&snapshot, &projection, &wrong_generation),
            Err(StateTreeError::GenerationMismatch)
        );

        let reversed = [parameters().patches()[1], parameters().patches()[0]];
        let wrong_order = ParameterSnapshot::new(42, global(), &reversed).unwrap();
        assert_eq!(
            StateTree::new(&snapshot, &projection, &wrong_order),
            Err(StateTreeError::PatchIdentityMismatch { index: 0 })
        );

        let wrong_values = [
            parameters().patches()[0],
            RtPatchParameters::new(
                PatchId::new(9).unwrap(),
                ChannelParameters::new(-10.0, 0.5, 0.4, 0.3).unwrap(),
            ),
        ];
        let wrong_parameters = ParameterSnapshot::new(42, global(), &wrong_values).unwrap();
        assert_eq!(
            StateTree::new(&snapshot, &projection, &wrong_parameters),
            Err(StateTreeError::PatchParametersMismatch { index: 1 })
        );
    }

    #[test]
    fn rejects_malformed_state_and_mismatched_global_parameters() {
        let malformed = StateSnapshot::new("not-json");
        let malformed_projection = projection(&malformed);
        assert_eq!(
            StateTree::new(&malformed, &malformed_projection, &parameters()),
            Err(StateTreeError::StateDeserialization)
        );

        let snapshot = snapshot();
        let projection = projection(&snapshot);
        let different_global =
            GlobalParameters::new(-2.0, 0.7, 0.4, 0.25, 375.0, 0.35, 0.2).unwrap();
        let wrong_global =
            ParameterSnapshot::new(42, different_global, parameters().patches()).unwrap();
        assert_eq!(
            StateTree::new(&snapshot, &projection, &wrong_global),
            Err(StateTreeError::GlobalParametersMismatch)
        );
    }
}
