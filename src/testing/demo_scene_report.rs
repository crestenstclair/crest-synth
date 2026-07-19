use crate::control::event_log::EventLog;
use crate::control::state_tree::StateTree;
use core::fmt;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// One stable section of the exhaustive demo coverage matrix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemoCoverageGroup {
    Events,
    Directions,
    MidiKinds,
    EditableParameters,
    SerializedProperties,
    Rejections,
    Projections,
    AudioEffects,
}

/// Sorted expected, exercised, and missing identifiers for one coverage group.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoCoverageSet {
    expected: Vec<String>,
    exercised: Vec<String>,
    missing: Vec<String>,
}

impl DemoCoverageSet {
    /// Declares the complete expected surface for one coverage group.
    pub fn new<I, S>(expected: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut expected: Vec<String> = expected.into_iter().map(Into::into).collect();
        expected.sort();
        expected.dedup();

        Self {
            missing: expected.clone(),
            expected,
            exercised: Vec::new(),
        }
    }

    /// Returns expected identifiers in deterministic order.
    pub fn expected(&self) -> &[String] {
        &self.expected
    }

    /// Returns exercised identifiers in deterministic order.
    pub fn exercised(&self) -> &[String] {
        &self.exercised
    }

    /// Returns expected identifiers not yet exercised.
    pub fn missing(&self) -> &[String] {
        &self.missing
    }

    /// Records an identifier, returning true only for its first observation.
    pub fn mark_exercised(&mut self, identifier: impl Into<String>) -> bool {
        let identifier = identifier.into();
        if !insert_sorted_unique(&mut self.exercised, identifier.clone()) {
            return false;
        }
        if let Ok(index) = self.missing.binary_search(&identifier) {
            self.missing.remove(index);
        }
        true
    }

    /// Reports whether every declared identifier was exercised.
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }
}

fn insert_sorted_unique(values: &mut Vec<String>, value: String) -> bool {
    match values.binary_search(&value) {
        Ok(_) => false,
        Err(index) => {
            values.insert(index, value);
            true
        }
    }
}

/// Exhaustive coverage grouped by every current observable surface.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoSceneCoverage {
    events: DemoCoverageSet,
    directions: DemoCoverageSet,
    midi_kinds: DemoCoverageSet,
    editable_parameters: DemoCoverageSet,
    serialized_properties: DemoCoverageSet,
    rejections: DemoCoverageSet,
    projections: DemoCoverageSet,
    audio_effects: DemoCoverageSet,
}

impl DemoSceneCoverage {
    /// Creates an empty matrix. Callers declare each expected group before use.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces one group's expected surface before observations are recorded.
    pub fn declare_expected<I, S>(&mut self, group: DemoCoverageGroup, expected: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        *self.group_mut(group) = DemoCoverageSet::new(expected);
    }

    /// Marks one stable identifier as exercised in its coverage group.
    pub fn mark_exercised(
        &mut self,
        group: DemoCoverageGroup,
        identifier: impl Into<String>,
    ) -> bool {
        self.group_mut(group).mark_exercised(identifier)
    }

    /// Returns one complete coverage group.
    pub fn group(&self, group: DemoCoverageGroup) -> &DemoCoverageSet {
        match group {
            DemoCoverageGroup::Events => &self.events,
            DemoCoverageGroup::Directions => &self.directions,
            DemoCoverageGroup::MidiKinds => &self.midi_kinds,
            DemoCoverageGroup::EditableParameters => &self.editable_parameters,
            DemoCoverageGroup::SerializedProperties => &self.serialized_properties,
            DemoCoverageGroup::Rejections => &self.rejections,
            DemoCoverageGroup::Projections => &self.projections,
            DemoCoverageGroup::AudioEffects => &self.audio_effects,
        }
    }

    /// Returns the number of expected identifiers in every group.
    pub fn expected_count(&self) -> usize {
        self.groups()
            .iter()
            .map(|group| group.expected().len())
            .sum()
    }

    /// Returns the number of unique exercised identifiers in every group.
    pub fn exercised_count(&self) -> usize {
        self.groups()
            .iter()
            .map(|group| group.exercised().len())
            .sum()
    }

    /// Returns the number of named coverage gaps in every group.
    pub fn missing_count(&self) -> usize {
        self.groups()
            .iter()
            .map(|group| group.missing().len())
            .sum()
    }

    /// Reports whether every expected identifier in every group was exercised.
    pub fn is_complete(&self) -> bool {
        self.groups().iter().all(|group| group.is_complete())
    }

    fn group_mut(&mut self, group: DemoCoverageGroup) -> &mut DemoCoverageSet {
        match group {
            DemoCoverageGroup::Events => &mut self.events,
            DemoCoverageGroup::Directions => &mut self.directions,
            DemoCoverageGroup::MidiKinds => &mut self.midi_kinds,
            DemoCoverageGroup::EditableParameters => &mut self.editable_parameters,
            DemoCoverageGroup::SerializedProperties => &mut self.serialized_properties,
            DemoCoverageGroup::Rejections => &mut self.rejections,
            DemoCoverageGroup::Projections => &mut self.projections,
            DemoCoverageGroup::AudioEffects => &mut self.audio_effects,
        }
    }

    fn groups(&self) -> [&DemoCoverageSet; 8] {
        [
            &self.events,
            &self.directions,
            &self.midi_kinds,
            &self.editable_parameters,
            &self.serialized_properties,
            &self.rejections,
            &self.projections,
            &self.audio_effects,
        ]
    }
}

/// Invalid data supplied for a deterministic scene checkpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemoSceneCheckpointError {
    EmptyStep,
    EmptyStateHash,
    GenerationMismatch {
        generation: u64,
        parameter_generation: u64,
    },
    NonFiniteAudioMeasurement,
}

impl fmt::Display for DemoSceneCheckpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::EmptyStep => formatter.write_str("checkpoint step must not be empty"),
            Self::EmptyStateHash => {
                formatter.write_str("checkpoint state hash must not be empty")
            }
            Self::GenerationMismatch {
                generation,
                parameter_generation,
            } => write!(
                formatter,
                "checkpoint state generation {generation} does not match parameter generation {parameter_generation}"
            ),
            Self::NonFiniteAudioMeasurement => {
                formatter.write_str("checkpoint audio measurement must be finite")
            }
        }
    }
}

impl std::error::Error for DemoSceneCheckpointError {}

/// One deterministic observation made after a named demo step.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoSceneCheckpoint {
    step: String,
    state_hash: String,
    generation: u64,
    selected_line: usize,
    parameter_generation: u64,
    audio_measurement: f64,
}

impl DemoSceneCheckpoint {
    /// Builds a self-consistent checkpoint from one accepted or rejected state.
    pub fn new(
        step: impl Into<String>,
        state_hash: impl Into<String>,
        generation: u64,
        selected_line: usize,
        parameter_generation: u64,
        audio_measurement: f64,
    ) -> Result<Self, DemoSceneCheckpointError> {
        let step = step.into();
        if step.trim().is_empty() {
            return Err(DemoSceneCheckpointError::EmptyStep);
        }

        let state_hash = state_hash.into();
        if state_hash.is_empty() {
            return Err(DemoSceneCheckpointError::EmptyStateHash);
        }
        if generation != parameter_generation {
            return Err(DemoSceneCheckpointError::GenerationMismatch {
                generation,
                parameter_generation,
            });
        }
        if !audio_measurement.is_finite() {
            return Err(DemoSceneCheckpointError::NonFiniteAudioMeasurement);
        }

        Ok(Self {
            step,
            state_hash,
            generation,
            selected_line,
            parameter_generation,
            audio_measurement,
        })
    }

    pub fn step(&self) -> &str {
        &self.step
    }

    pub fn state_hash(&self) -> &str {
        &self.state_hash
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub const fn selected_line(&self) -> usize {
        self.selected_line
    }

    pub const fn parameter_generation(&self) -> u64 {
        self.parameter_generation
    }

    pub const fn audio_measurement(&self) -> f64 {
        self.audio_measurement
    }
}

/// A structural contradiction that prevents a coherent demo report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemoSceneReportError {
    EmptyScene,
    EmptyEventLog,
    EmptyCheckpoints,
    FinalEventStateMismatch,
    ReportSerialization,
}

impl fmt::Display for DemoSceneReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::EmptyScene => formatter.write_str("demo scene name must not be empty"),
            Self::EmptyEventLog => {
                formatter.write_str("demo report requires at least one event record")
            }
            Self::EmptyCheckpoints => {
                formatter.write_str("demo report requires at least one checkpoint")
            }
            Self::FinalEventStateMismatch => formatter.write_str(
                "final state tree does not match the event journal generation and hash endpoint",
            ),
            Self::ReportSerialization => {
                formatter.write_str("demo scene report could not be serialized")
            }
        }
    }
}

impl std::error::Error for DemoSceneReportError {}

/// The complete machine-readable result of one exhaustive GUI demo run.
#[derive(Clone, Debug, PartialEq)]
pub struct DemoSceneReport {
    scene: String,
    complete: bool,
    coverage: DemoSceneCoverage,
    checkpoints: Vec<DemoSceneCheckpoint>,
    event_log: EventLog,
    final_state_tree: StateTree,
}

impl DemoSceneReport {
    /// Stable schema version for the top-level report.
    pub const SCHEMA_VERSION: u32 = 1;

    /// Packages a scene only after checking the journal/tree endpoint.
    ///
    /// Coverage gaps, dropped journal records, or inconsistent checkpoints
    /// produce a valid diagnostic report with complete=false. A final tree that
    /// is not the journal endpoint is rejected because it cannot describe the
    /// scene's final state.
    pub fn new(
        scene: impl Into<String>,
        coverage: DemoSceneCoverage,
        checkpoints: Vec<DemoSceneCheckpoint>,
        event_log: EventLog,
        final_state_tree: StateTree,
    ) -> Result<Self, DemoSceneReportError> {
        let scene = scene.into();
        if scene.trim().is_empty() {
            return Err(DemoSceneReportError::EmptyScene);
        }
        let endpoint = event_log
            .records()
            .last()
            .ok_or(DemoSceneReportError::EmptyEventLog)?;
        if checkpoints.is_empty() {
            return Err(DemoSceneReportError::EmptyCheckpoints);
        }
        if endpoint.generation_after() != final_state_tree.generation()
            || endpoint.parameter_generation() != final_state_tree.generation()
            || endpoint.state_hash_after() != final_state_tree.state_hash()
            || endpoint.projection_state_hash() != final_state_tree.state_hash()
            || endpoint.selected_line() != final_state_tree.selected_line()
        {
            return Err(DemoSceneReportError::FinalEventStateMismatch);
        }

        let checkpoints_agree =
            checkpoint_chain_agrees(&checkpoints, &event_log, &final_state_tree);
        let complete = coverage.is_complete()
            && event_log.coverage().is_complete()
            && event_log.dropped_records() == 0
            && event_log.total_observed() == event_log.records().len() as u64
            && checkpoints_agree;

        Ok(Self {
            scene,
            complete,
            coverage,
            checkpoints,
            event_log,
            final_state_tree,
        })
    }

    pub const fn schema_version(&self) -> u32 {
        Self::SCHEMA_VERSION
    }

    pub fn scene(&self) -> &str {
        &self.scene
    }

    pub const fn is_complete(&self) -> bool {
        self.complete
    }

    pub const fn coverage(&self) -> &DemoSceneCoverage {
        &self.coverage
    }

    pub fn checkpoints(&self) -> &[DemoSceneCheckpoint] {
        &self.checkpoints
    }

    pub const fn event_log(&self) -> &EventLog {
        &self.event_log
    }

    pub const fn final_state_tree(&self) -> &StateTree {
        &self.final_state_tree
    }

    /// Serializes stable schema fields without timestamps, paths, or maps.
    pub fn to_json(&self) -> Result<String, DemoSceneReportError> {
        serde_json::to_string(self).map_err(|_| DemoSceneReportError::ReportSerialization)
    }
}

fn checkpoint_chain_agrees(
    checkpoints: &[DemoSceneCheckpoint],
    event_log: &EventLog,
    final_state_tree: &StateTree,
) -> bool {
    let ordered = checkpoints.windows(2).all(|pair| {
        pair[0].generation() <= pair[1].generation()
            && (pair[0].generation() != pair[1].generation()
                || pair[0].state_hash() == pair[1].state_hash())
    });

    let final_checkpoint_matches = checkpoints.last().is_some_and(|checkpoint| {
        checkpoint.generation() == final_state_tree.generation()
            && checkpoint.parameter_generation() == final_state_tree.generation()
            && checkpoint.state_hash() == final_state_tree.state_hash()
            && checkpoint.selected_line() == final_state_tree.selected_line()
    });

    let retained_records_match = event_log.dropped_records() != 0
        || checkpoints.iter().all(|checkpoint| {
            event_log.records().iter().any(|record| {
                checkpoint.generation() == record.generation_after()
                    && checkpoint.parameter_generation() == record.parameter_generation()
                    && checkpoint.state_hash() == record.state_hash_after()
                    && checkpoint.selected_line() == record.selected_line()
            })
        });

    ordered && final_checkpoint_matches && retained_records_match
}

impl Serialize for DemoSceneReport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let final_state_tree: serde_json::Value =
            serde_json::from_str(self.final_state_tree.json())
                .map_err(serde::ser::Error::custom)?;

        let mut report = serializer.serialize_struct("DemoSceneReport", 7)?;
        report.serialize_field("schemaVersion", &Self::SCHEMA_VERSION)?;
        report.serialize_field("scene", &self.scene)?;
        report.serialize_field("complete", &self.complete)?;
        report.serialize_field("coverage", &self.coverage)?;
        report.serialize_field("checkpoints", &self.checkpoints)?;
        report.serialize_field("eventLog", &self.event_log)?;
        report.serialize_field("finalStateTree", &final_state_tree)?;
        report.end()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DemoCoverageGroup, DemoSceneCheckpoint, DemoSceneCoverage, DemoSceneReport,
        DemoSceneReportError,
    };
    use crate::control::app_event::{AppEvent, Direction};
    use crate::control::app_state::EventRejection;
    use crate::control::event_log::{EventCoverage, EventLog};
    use crate::control::event_record::{EventRecord, EventSource};
    use crate::control::state_snapshot::StateSnapshot;
    use crate::control::state_tree::StateTree;
    use crate::control::text_projection::TextProjection;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};

    fn snapshot() -> StateSnapshot {
        StateSnapshot::new(
            r#"{"generation":4,"patches":[{"id":7,"name":"Lead","channel":2,"bank":0,"program":80,"percussion":false,"gainDb":-6.0,"pan":-0.25,"reverbSend":0.2,"delaySend":0.1},{"id":9,"name":"Drums","channel":9,"bank":128,"program":0,"percussion":true,"gainDb":-12.0,"pan":0.5,"reverbSend":0.4,"delaySend":0.3}],"global":{"masterGainDb":-3.0,"reverbRoomSize":0.7,"reverbDamping":0.4,"reverbReturn":0.25,"delayMilliseconds":375.0,"delayFeedback":0.35,"delayReturn":0.2},"selection":{"section":"Patch","patchIndex":1,"parameterIndex":2}}"#,
        )
    }

    fn tree_and_projection() -> (StateTree, TextProjection) {
        let snapshot = snapshot();
        let projection = TextProjection::new(
            "PATCH Lead\n> reverbSend=0.4\nGLOBAL".to_owned(),
            1,
            snapshot.hash().to_owned(),
        );
        let global = GlobalParameters::new(-3.0, 0.7, 0.4, 0.25, 375.0, 0.35, 0.2).unwrap();
        let patches = [
            RtPatchParameters::new(
                PatchId::new(7).unwrap(),
                ChannelParameters::new(-6.0, -0.25, 0.2, 0.1).unwrap(),
            ),
            RtPatchParameters::new(
                PatchId::new(9).unwrap(),
                ChannelParameters::new(-12.0, 0.5, 0.4, 0.3).unwrap(),
            ),
        ];
        let parameters = ParameterSnapshot::new(4, global, &patches).unwrap();
        let tree = StateTree::new(&snapshot, &projection, &parameters).unwrap();
        (tree, projection)
    }

    fn event_log(projection: &TextProjection, capacity: usize, count: u64) -> EventLog {
        let mut log =
            EventLog::with_coverage(capacity, EventCoverage::new(["event.adjust.right"])).unwrap();
        log.mark_exercised("event.adjust.right");

        for sequence in 0..count {
            let record = EventRecord::rejected(
                sequence,
                EventSource::DemoScene,
                &AppEvent::Adjust(Direction::Right),
                4,
                projection.state_hash(),
                4,
                projection,
                EventRejection::ParameterAtBoundary,
            )
            .unwrap();
            log.append(record).unwrap();
        }
        log
    }

    fn coverage(complete: bool) -> DemoSceneCoverage {
        let mut coverage = DemoSceneCoverage::new();
        coverage.declare_expected(DemoCoverageGroup::Events, ["adjust.right"]);
        if complete {
            coverage.mark_exercised(DemoCoverageGroup::Events, "adjust.right");
        }
        coverage
    }

    fn checkpoint(tree: &StateTree) -> DemoSceneCheckpoint {
        DemoSceneCheckpoint::new(
            "boundary-rejection",
            tree.state_hash(),
            tree.generation(),
            tree.selected_line(),
            tree.generation(),
            0.125,
        )
        .unwrap()
    }

    #[test]
    fn complete_report_embeds_stable_grouped_trace_objects() {
        let (tree, projection) = tree_and_projection();
        let report = DemoSceneReport::new(
            "exhaustive-gui",
            coverage(true),
            vec![checkpoint(&tree)],
            event_log(&projection, 2, 1),
            tree,
        )
        .unwrap();

        assert!(report.is_complete());
        assert_eq!(report.coverage().missing_count(), 0);
        assert_eq!(report.event_log().dropped_records(), 0);

        let first = report.to_json().unwrap();
        let second = report.to_json().unwrap();
        let json: serde_json::Value = serde_json::from_str(&first).unwrap();

        assert_eq!(first, second);
        assert_eq!(json["schemaVersion"], 1);
        assert_eq!(json["scene"], "exhaustive-gui");
        assert_eq!(json["complete"], true);
        assert_eq!(json["coverage"]["events"]["missing"], serde_json::json!([]));
        assert_eq!(json["checkpoints"][0]["audioMeasurement"], 0.125);
        assert_eq!(json["eventLog"]["records"].as_array().unwrap().len(), 1);
        assert_eq!(json["finalStateTree"]["generation"], 4);
        assert_eq!(
            json["finalStateTree"]["patches"].as_array().unwrap().len(),
            2
        );
    }

    #[test]
    fn missing_coverage_or_dropped_history_can_never_claim_completion() {
        let (tree, projection) = tree_and_projection();
        let report = DemoSceneReport::new(
            "diagnostic",
            coverage(false),
            vec![checkpoint(&tree)],
            event_log(&projection, 1, 2),
            tree,
        )
        .unwrap();

        assert!(!report.is_complete());
        assert_eq!(report.coverage().missing_count(), 1);
        assert_eq!(report.event_log().dropped_records(), 1);
    }

    #[test]
    fn rejects_a_final_tree_that_is_not_the_event_endpoint() {
        let (tree, projection) = tree_and_projection();
        let mismatched = EventRecord::rejected(
            0,
            EventSource::DemoScene,
            &AppEvent::Adjust(Direction::Right),
            3,
            projection.state_hash(),
            3,
            &projection,
            EventRejection::ParameterAtBoundary,
        )
        .unwrap();
        let mut log = EventLog::new(1).unwrap();
        log.append(mismatched).unwrap();

        assert_eq!(
            DemoSceneReport::new(
                "mismatch",
                coverage(true),
                vec![checkpoint(&tree)],
                log,
                tree,
            ),
            Err(DemoSceneReportError::FinalEventStateMismatch)
        );
    }
}
