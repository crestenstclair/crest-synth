use crate::control::event_log::EventLog;
use crate::control::event_record::{EventInput, EventOutcome, EventSource, MidiKind};
use crate::control::state_tree::StateTree;
use crate::kernel::patch_id::PatchId;
use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
use crate::testing::live_demo_checkpoint::LiveDemoCheckpoint;
use crate::testing::live_demo_scene::LiveEditableParameter;
use core::fmt;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Exact bidirectional coverage for the frozen editable-parameter surface.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveDemoCoverage {
    expected_editable_parameters: Vec<String>,
    exercised_editable_parameters: Vec<String>,
    missing_editable_parameters: Vec<String>,
    unexpected_editable_parameters: Vec<String>,
    duplicate_expected_parameters: Vec<String>,
}

impl LiveDemoCoverage {
    pub fn new(expected: &[LiveEditableParameter]) -> Self {
        let mut expected_ids = Vec::with_capacity(expected.len());
        let mut duplicate_expected = Vec::new();
        for parameter in expected {
            let identifier = parameter.identifier();
            if expected_ids.contains(&identifier) {
                insert_sorted_unique(&mut duplicate_expected, identifier);
            } else {
                expected_ids.push(identifier);
            }
        }

        Self {
            missing_editable_parameters: expected_ids.clone(),
            expected_editable_parameters: expected_ids,
            duplicate_expected_parameters: duplicate_expected,
            ..Self::default()
        }
    }

    pub fn mark_exercised(&mut self, parameter: LiveEditableParameter) {
        let identifier = parameter.identifier();
        if !insert_sorted_unique(&mut self.exercised_editable_parameters, identifier.clone()) {
            return;
        }
        if let Some(index) = self
            .missing_editable_parameters
            .iter()
            .position(|expected| expected == &identifier)
        {
            self.missing_editable_parameters.remove(index);
        } else {
            insert_sorted_unique(&mut self.unexpected_editable_parameters, identifier);
        }
    }

    pub fn mark_unexpected(&mut self, identifier: impl Into<String>) {
        let identifier = identifier.into();
        insert_sorted_unique(&mut self.exercised_editable_parameters, identifier.clone());
        insert_sorted_unique(&mut self.unexpected_editable_parameters, identifier);
    }

    pub fn expected(&self) -> &[String] {
        &self.expected_editable_parameters
    }

    pub fn exercised(&self) -> &[String] {
        &self.exercised_editable_parameters
    }

    pub fn missing(&self) -> &[String] {
        &self.missing_editable_parameters
    }

    pub fn unexpected(&self) -> &[String] {
        &self.unexpected_editable_parameters
    }

    pub fn duplicate_expected(&self) -> &[String] {
        &self.duplicate_expected_parameters
    }

    pub fn is_complete(&self) -> bool {
        self.missing_editable_parameters.is_empty()
            && self.unexpected_editable_parameters.is_empty()
            && self.duplicate_expected_parameters.is_empty()
            && self.exercised_editable_parameters.len() == self.expected_editable_parameters.len()
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

/// The complete structured result of one live observable demo.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveDemoReport {
    scene: String,
    complete: bool,
    checkpoints: Vec<LiveDemoCheckpoint>,
    event_log: EventLog,
    state_tree: StateTree,
    coverage: LiveDemoCoverage,
    summary: String,
}

impl LiveDemoReport {
    pub const SCHEMA_VERSION: u32 = 1;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scene: impl Into<String>,
        checkpoints: Vec<LiveDemoCheckpoint>,
        event_log: EventLog,
        state_tree: StateTree,
        coverage: LiveDemoCoverage,
        installed_patches: &[PatchId],
        cleanup_sequence_before: u64,
        final_observation: AudioObservationSnapshot,
    ) -> Result<Self, LiveDemoReportError> {
        let scene = scene.into();
        if scene.trim().is_empty() {
            return Err(LiveDemoReportError::EmptyScene);
        }
        let endpoint = event_log
            .records()
            .last()
            .ok_or(LiveDemoReportError::EmptyEventLog)?;
        if endpoint.generation_after() != state_tree.generation()
            || endpoint.parameter_generation() != state_tree.generation()
            || endpoint.state_hash_after() != state_tree.state_hash()
            || endpoint.projection_state_hash() != state_tree.state_hash()
            || endpoint.selected_line() != state_tree.selected_line()
        {
            return Err(LiveDemoReportError::FinalStateMismatch);
        }

        let accepted = event_log
            .records()
            .iter()
            .any(|record| record.outcome() == EventOutcome::Accepted);
        let rejected = event_log
            .records()
            .iter()
            .any(|record| record.outcome() == EventOutcome::Rejected);
        let cleanup_complete = installed_patches.iter().all(|expected_patch| {
            event_log.records().iter().any(|record| {
                record.source() == EventSource::DemoScene
                    && record.outcome() == EventOutcome::Accepted
                    && matches!(
                        record.input(),
                        EventInput::Midi { patch_id, message }
                            if *patch_id == expected_patch.value()
                                && message.kind() == MidiKind::AllNotesOff
                    )
            })
        });
        let final_audio_complete = final_observation.sequence() > cleanup_sequence_before
            && final_observation.parameter_generation() == state_tree.generation()
            && final_observation.active_notes() == 0
            && final_observation.output_rms().is_finite()
            && final_observation.non_finite_samples() == 0;
        let checkpoints_agree =
            !checkpoints.is_empty() && checkpoints.iter().all(LiveDemoCheckpoint::agrees);
        let lossless = event_log.dropped_records() == 0
            && event_log.total_observed() == event_log.records().len() as u64;
        let complete = coverage.is_complete()
            && accepted
            && rejected
            && checkpoints_agree
            && lossless
            && cleanup_complete
            && final_audio_complete;
        let summary = format!(
            "live demo {}: {}/{} editable parameters, {} checkpoints, {} events, {} dropped, cleanup={}, activeNotes={}",
            if complete { "complete" } else { "incomplete" },
            coverage.exercised().len(),
            coverage.expected().len(),
            checkpoints.len(),
            event_log.total_observed(),
            event_log.dropped_records(),
            cleanup_complete,
            final_observation.active_notes(),
        );

        Ok(Self {
            scene,
            complete,
            checkpoints,
            event_log,
            state_tree,
            coverage,
            summary,
        })
    }

    pub const fn schema_version(&self) -> u32 {
        Self::SCHEMA_VERSION
    }

    pub fn scene(&self) -> &str {
        &self.scene
    }

    pub const fn complete(&self) -> bool {
        self.complete
    }

    pub fn checkpoints(&self) -> &[LiveDemoCheckpoint] {
        &self.checkpoints
    }

    pub const fn event_log(&self) -> &EventLog {
        &self.event_log
    }

    pub const fn state_tree(&self) -> &StateTree {
        &self.state_tree
    }

    pub const fn coverage(&self) -> &LiveDemoCoverage {
        &self.coverage
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn to_json(&self) -> Result<String, LiveDemoReportError> {
        serde_json::to_string(self).map_err(|_| LiveDemoReportError::Serialization)
    }
}

impl Serialize for LiveDemoReport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let state_tree: serde_json::Value =
            serde_json::from_str(self.state_tree.json()).map_err(serde::ser::Error::custom)?;
        let mut report = serializer.serialize_struct("LiveDemoReport", 8)?;
        report.serialize_field("schemaVersion", &Self::SCHEMA_VERSION)?;
        report.serialize_field("scene", &self.scene)?;
        report.serialize_field("complete", &self.complete)?;
        report.serialize_field("checkpoints", &self.checkpoints)?;
        report.serialize_field("eventLog", &self.event_log)?;
        report.serialize_field("stateTree", &state_tree)?;
        report.serialize_field("coverage", &self.coverage)?;
        report.serialize_field("summary", &self.summary)?;
        report.end()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiveDemoReportError {
    EmptyScene,
    EmptyEventLog,
    FinalStateMismatch,
    Serialization,
}

impl fmt::Display for LiveDemoReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyScene => formatter.write_str("live demo scene name must not be empty"),
            Self::EmptyEventLog => formatter.write_str("live demo report requires event evidence"),
            Self::FinalStateMismatch => {
                formatter.write_str("final StateTree is not the EventLog chain endpoint")
            }
            Self::Serialization => formatter.write_str("live demo report could not be serialized"),
        }
    }
}

impl std::error::Error for LiveDemoReportError {}

#[cfg(test)]
mod tests {
    use super::LiveDemoCoverage;
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameter;
    use crate::mixer::global_parameters::GlobalParameter;
    use crate::testing::live_demo_scene::LiveEditableParameter;

    #[test]
    fn exact_coverage_reports_missing_unexpected_and_duplicate_expected_values() {
        let patch = PatchId::new(1).unwrap();
        let gain = LiveEditableParameter::patch(patch, ChannelParameter::GainDb);
        let master = LiveEditableParameter::global(GlobalParameter::MasterGainDb);
        let mut coverage = LiveDemoCoverage::new(&[gain, master]);

        coverage.mark_exercised(gain);
        assert_eq!(coverage.missing(), &["global.masterGainDb"]);
        assert!(!coverage.is_complete());

        coverage.mark_unexpected("patch.1.pan");
        assert_eq!(coverage.unexpected(), &["patch.1.pan"]);
        coverage.mark_exercised(master);
        assert!(!coverage.is_complete());

        let duplicate = LiveDemoCoverage::new(&[gain, gain]);
        assert_eq!(duplicate.duplicate_expected(), &["patch.1.gainDb"]);
        assert!(!duplicate.is_complete());
    }
}
