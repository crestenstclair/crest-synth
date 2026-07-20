use crate::control::state_snapshot::StateSnapshot;
use crate::control::text_projection::TextProjection;
use crate::mixer::channel_parameters::ChannelParameters;
use crate::mixer::global_parameters::GlobalParameters;
use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
use core::fmt;
use serde::{Deserialize, Serialize};

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
        }
    }
}

impl std::error::Error for StateTreeError {}

/// A canonical, LLM-readable tree of one complete accepted control generation.
///
/// Construction consumes only immutable projections already derived from the
/// same accepted AppState. It verifies their shared identity before producing
/// deterministic JSON with stable property names.
#[derive(Clone, Debug, PartialEq)]
pub struct StateTree {
    json: String,
    generation: u64,
    patch_count: usize,
    selected_line: usize,
    state_hash: String,
}

impl StateTree {
    /// The stable schema version emitted in every serialized tree.
    pub const SCHEMA_VERSION: u32 = 1;
    pub const SERIALIZED_PROPERTY_DESCRIPTOR: &'static [&'static str] = &[
        "schemaVersion",
        "generation",
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

    /// Returns the production-owned stable StateTree property surface.
    pub const fn serialized_property_descriptor() -> &'static [&'static str] {
        Self::SERIALIZED_PROPERTY_DESCRIPTOR
    }

    /// Builds one observation tree from a state snapshot and its GUI/audio
    /// projections.
    pub fn new(
        snapshot: &StateSnapshot,
        projection: &TextProjection,
        parameters: &ParameterSnapshot,
    ) -> Result<Self, StateTreeError> {
        let state: SnapshotState = serde_json::from_str(snapshot.json())
            .map_err(|_| StateTreeError::StateDeserialization)?;

        if projection.state_hash() != snapshot.hash() {
            return Err(StateTreeError::ProjectionHashMismatch);
        }
        validate_parameter_projection(&state, parameters)?;

        let serializable =
            SerializableStateTree::new(&state, projection, parameters, snapshot.hash());
        let json =
            serde_json::to_string(&serializable).map_err(|_| StateTreeError::Serialization)?;

        Ok(Self {
            json,
            generation: state.generation,
            patch_count: state.patches.len(),
            selected_line: projection.selected_line(),
            state_hash: snapshot.hash().to_owned(),
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
        &self.json
    }

    /// Consumes the value and returns its deterministic JSON representation.
    pub fn into_json(self) -> String {
        self.json
    }
}

fn validate_parameter_projection(
    state: &SnapshotState,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotState {
    generation: u64,
    patches: Vec<SnapshotPatch>,
    global: SnapshotGlobalParameters,
    selection: SnapshotSelection,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotPatch {
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

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotGlobalParameters {
    master_gain_db: f32,
    reverb_room_size: f32,
    reverb_damping: f32,
    reverb_return: f32,
    delay_milliseconds: f32,
    delay_feedback: f32,
    delay_return: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotSelection {
    section: String,
    patch_index: usize,
    parameter_index: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializableStateTree {
    schema_version: u32,
    generation: u64,
    patches: Vec<TreePatch>,
    global: TreeGlobalParameters,
    selection: SnapshotSelection,
    projection: TreeProjection,
    parameters: TreeParameterSnapshot,
}

impl SerializableStateTree {
    fn new(
        state: &SnapshotState,
        projection: &TextProjection,
        parameters: &ParameterSnapshot,
        state_hash: &str,
    ) -> Self {
        Self {
            schema_version: StateTree::SCHEMA_VERSION,
            generation: state.generation,
            patches: state.patches.iter().map(TreePatch::from).collect(),
            global: TreeGlobalParameters::from(&state.global),
            selection: state.selection.clone(),
            projection: TreeProjection {
                body: projection.body().to_owned(),
                selected_line: projection.selected_line(),
                state_hash: state_hash.to_owned(),
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
struct TreePatch {
    id: u32,
    name: String,
    channel: u8,
    instrument: TreeInstrument,
    parameters: TreeChannelParameters,
}

impl From<&SnapshotPatch> for TreePatch {
    fn from(patch: &SnapshotPatch) -> Self {
        Self {
            id: patch.id,
            name: patch.name.clone(),
            channel: patch.channel,
            instrument: TreeInstrument {
                bank: patch.bank,
                program: patch.program,
                percussion: patch.percussion,
            },
            parameters: TreeChannelParameters::from(patch),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeInstrument {
    bank: u16,
    program: u8,
    percussion: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeChannelParameters {
    gain_db: f32,
    pan: f32,
    reverb_send: f32,
    delay_send: f32,
}

impl From<&SnapshotPatch> for TreeChannelParameters {
    fn from(patch: &SnapshotPatch) -> Self {
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

impl From<&SnapshotGlobalParameters> for TreeGlobalParameters {
    fn from(parameters: &SnapshotGlobalParameters) -> Self {
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
struct TreeProjection {
    body: String,
    selected_line: usize,
    state_hash: String,
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
    use crate::control::state_snapshot::StateSnapshot;
    use crate::control::text_projection::TextProjection;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
    use serde_json::{json, Value};

    fn snapshot() -> StateSnapshot {
        StateSnapshot::new(
            r#"{"generation":42,"patches":[{"id":7,"name":"Lead","channel":2,"bank":0,"program":80,"percussion":false,"gainDb":-6.0,"pan":-0.25,"reverbSend":0.2,"delaySend":0.1},{"id":9,"name":"Drums","channel":9,"bank":128,"program":0,"percussion":true,"gainDb":-12.0,"pan":0.5,"reverbSend":0.4,"delaySend":0.3}],"global":{"masterGainDb":-3.0,"reverbRoomSize":0.7,"reverbDamping":0.4,"reverbReturn":0.25,"delayMilliseconds":375.0,"delayFeedback":0.35,"delayReturn":0.2},"selection":{"section":"Patch","patchIndex":1,"parameterIndex":2}}"#,
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

        assert_eq!(tree.schema_version(), 1);
        assert_eq!(tree.generation(), 42);
        assert_eq!(tree.patch_count(), 2);
        assert_eq!(tree.selected_line(), 1);
        assert_eq!(tree.state_hash(), snapshot.hash());

        let root = value.as_object().unwrap();
        assert_eq!(root.len(), 7);
        for property in [
            "schemaVersion",
            "generation",
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
                "instrument": {"bank": 0, "program": 80, "percussion": false},
                "parameters": {
                    "gainDb": -6.0,
                    "pan": -0.25,
                    "reverbSend": 0.2,
                    "delaySend": 0.1
                }
            })
        );
        assert_eq!(
            value["patches"][1]["instrument"],
            json!({"bank": 128, "program": 0, "percussion": true})
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
            .starts_with(r#"{"schemaVersion":1,"generation":42,"#));
        assert_eq!(first.clone().into_json(), first.json());
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
