use crate::adapter::braids_capability::BRAIDS_CAPABILITY_ID;
use crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID;
use crate::control::event_log::EventLog;
use crate::control::event_record::{EventInput, EventOutcome, EventSource, MidiKind};
use crate::control::state_tree::StateTree;
use crate::kernel::patch_id::PatchId;
use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
use crate::real_time::GraphRevision;
use crate::testing::live_demo_checkpoint::LiveCheckpoint;
use crate::testing::live_demo_scene::{LiveEditableParameter, LiveEngineTransition};
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
    expected_engine_transitions: Vec<String>,
    exercised_engine_transitions: Vec<String>,
    missing_engine_transitions: Vec<String>,
    unexpected_engine_transitions: Vec<String>,
    duplicate_expected_engine_transitions: Vec<String>,
}

impl LiveDemoCoverage {
    pub fn new(expected: &[LiveEditableParameter]) -> Self {
        Self::with_engine_transitions(expected, &[])
    }

    pub fn with_engine_transitions(
        expected: &[LiveEditableParameter],
        engine_transitions: &[LiveEngineTransition],
    ) -> Self {
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

        let mut expected_engines = Vec::with_capacity(engine_transitions.len());
        let mut duplicate_expected_engines = Vec::new();
        for transition in engine_transitions {
            let identifier = transition.identifier().to_owned();
            if expected_engines.contains(&identifier) {
                insert_sorted_unique(&mut duplicate_expected_engines, identifier);
            } else {
                expected_engines.push(identifier);
            }
        }

        Self {
            missing_editable_parameters: expected_ids.clone(),
            expected_editable_parameters: expected_ids,
            duplicate_expected_parameters: duplicate_expected,
            missing_engine_transitions: expected_engines.clone(),
            expected_engine_transitions: expected_engines,
            duplicate_expected_engine_transitions: duplicate_expected_engines,
            ..Self::default()
        }
    }

    pub fn mark_exercised(&mut self, parameter: &LiveEditableParameter) {
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

    pub fn mark_engine_exercised(&mut self, transition: &LiveEngineTransition) {
        let identifier = transition.identifier().to_owned();
        if !insert_sorted_unique(&mut self.exercised_engine_transitions, identifier.clone()) {
            return;
        }
        if let Some(index) = self
            .missing_engine_transitions
            .iter()
            .position(|expected| expected == &identifier)
        {
            self.missing_engine_transitions.remove(index);
        } else {
            insert_sorted_unique(&mut self.unexpected_engine_transitions, identifier);
        }
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

    pub fn expected_engine_transitions(&self) -> &[String] {
        &self.expected_engine_transitions
    }

    pub fn exercised_engine_transitions(&self) -> &[String] {
        &self.exercised_engine_transitions
    }

    pub fn missing_engine_transitions(&self) -> &[String] {
        &self.missing_engine_transitions
    }

    pub fn unexpected_engine_transitions(&self) -> &[String] {
        &self.unexpected_engine_transitions
    }

    pub fn is_complete(&self) -> bool {
        self.missing_editable_parameters.is_empty()
            && self.unexpected_editable_parameters.is_empty()
            && self.duplicate_expected_parameters.is_empty()
            && self.exercised_editable_parameters.len() == self.expected_editable_parameters.len()
            && self.missing_engine_transitions.is_empty()
            && self.unexpected_engine_transitions.is_empty()
            && self.duplicate_expected_engine_transitions.is_empty()
            && self.exercised_engine_transitions.len() == self.expected_engine_transitions.len()
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

/// Compact control-side evidence for the prepared runtime used by a live run.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAudioWitness {
    parsed_soundfont_banks: usize,
    prepared_instruments: usize,
    soundfont_patches: usize,
    braids_patches: usize,
    alternating_capabilities: bool,
    initial_graph_revision: GraphRevision,
    active_graph_revision: GraphRevision,
    engine_switches: usize,
    ready_capabilities: [Option<RuntimeReadyCapability>; 3],
    fallbacks: usize,
    callback_allocations: usize,
    callback_destructions: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum RuntimeReadyCapability {
    #[serde(rename = "instrument.braids")]
    Braids,
    #[serde(rename = "instrument.soundfont.hidef")]
    SoundFont,
}

impl RuntimeAudioWitness {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        parsed_soundfont_banks: usize,
        prepared_instruments: usize,
        soundfont_patches: usize,
        braids_patches: usize,
        alternating_capabilities: bool,
        active_graph_revision: GraphRevision,
        callback_allocations: usize,
        callback_destructions: usize,
    ) -> Self {
        Self {
            parsed_soundfont_banks,
            prepared_instruments,
            soundfont_patches,
            braids_patches,
            alternating_capabilities,
            initial_graph_revision: active_graph_revision,
            active_graph_revision,
            engine_switches: 0,
            ready_capabilities: [None, None, None],
            fallbacks: 0,
            callback_allocations,
            callback_destructions,
        }
    }

    pub const fn parsed_soundfont_banks(self) -> usize {
        self.parsed_soundfont_banks
    }

    pub const fn prepared_instruments(self) -> usize {
        self.prepared_instruments
    }

    pub const fn soundfont_patches(self) -> usize {
        self.soundfont_patches
    }

    pub const fn braids_patches(self) -> usize {
        self.braids_patches
    }

    pub const fn alternating_capabilities(self) -> bool {
        self.alternating_capabilities
    }

    pub const fn active_graph_revision(self) -> GraphRevision {
        self.active_graph_revision
    }

    pub const fn initial_graph_revision(self) -> GraphRevision {
        self.initial_graph_revision
    }

    pub const fn engine_switches(self) -> usize {
        self.engine_switches
    }

    pub const fn fallbacks(self) -> usize {
        self.fallbacks
    }

    pub const fn callback_destructions(self) -> usize {
        self.callback_destructions
    }

    pub const fn callback_allocations(self) -> usize {
        self.callback_allocations
    }

    pub const fn with_active_graph_revision(mut self, revision: GraphRevision) -> Self {
        self.active_graph_revision = revision;
        self
    }

    pub fn record_ready_capability(
        &mut self,
        capability_id: &crate::synth::CapabilityId,
        revision: GraphRevision,
    ) -> bool {
        let capability = match capability_id.as_str() {
            BRAIDS_CAPABILITY_ID => RuntimeReadyCapability::Braids,
            HIDEF_CAPABILITY_ID => RuntimeReadyCapability::SoundFont,
            _ => return false,
        };
        let Some(slot) = self.ready_capabilities.get_mut(self.engine_switches) else {
            return false;
        };
        *slot = Some(capability);
        self.engine_switches = self.engine_switches.saturating_add(1);
        self.active_graph_revision = revision;
        true
    }

    const fn has_exact_ready_sequence(self) -> bool {
        matches!(
            self.ready_capabilities,
            [
                Some(RuntimeReadyCapability::SoundFont),
                Some(RuntimeReadyCapability::Braids),
                Some(RuntimeReadyCapability::SoundFont)
            ]
        )
    }
}

/// The complete structured result of one live observable demo.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveDemoReport {
    scene: String,
    complete: bool,
    checkpoints: Vec<LiveCheckpoint>,
    event_log: EventLog,
    state_tree: StateTree,
    coverage: LiveDemoCoverage,
    runtime_audio: RuntimeAudioWitness,
    summary: String,
}

/// Compact terminal evidence for a potentially large retained live journal.
///
/// The complete EventLog remains available on LiveDemoReport and is exercised
/// by deterministic verification; interactive output reports its lossless
/// bounds and canonical endpoints without printing every performance event.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveEventLogSummary {
    schema_version: u32,
    event_log_schema_version: u32,
    total_observed: u64,
    retained_records: usize,
    dropped_records: u64,
    first_sequence: Option<u64>,
    last_sequence: Option<u64>,
    generation_before: Option<u64>,
    generation_after: Option<u64>,
    state_hash_before: Option<String>,
    state_hash_after: Option<String>,
    active_graph_revision: GraphRevision,
    lossless: bool,
}

impl LiveEventLogSummary {
    pub const SCHEMA_VERSION: u32 = 3;

    fn from_event_log(event_log: &EventLog, active_graph_revision: GraphRevision) -> Self {
        let first = event_log.records().first();
        let last = event_log.records().last();
        Self {
            schema_version: Self::SCHEMA_VERSION,
            event_log_schema_version: event_log.schema_version(),
            total_observed: event_log.total_observed(),
            retained_records: event_log.records().len(),
            dropped_records: event_log.dropped_records(),
            first_sequence: first.map(|record| record.sequence()),
            last_sequence: last.map(|record| record.sequence()),
            generation_before: first.map(|record| record.generation_before()),
            generation_after: last.map(|record| record.generation_after()),
            state_hash_before: first.map(|record| record.state_hash_before().to_owned()),
            state_hash_after: last.map(|record| record.state_hash_after().to_owned()),
            active_graph_revision,
            lossless: event_log.dropped_records() == 0
                && event_log.total_observed() == event_log.records().len() as u64,
        }
    }
}

impl LiveDemoReport {
    pub const SCHEMA_VERSION: u32 = 5;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scene: impl Into<String>,
        checkpoints: Vec<LiveCheckpoint>,
        event_log: EventLog,
        state_tree: StateTree,
        coverage: LiveDemoCoverage,
        installed_patches: &[PatchId],
        cleanup_sequence_before: u64,
        final_observation: AudioObservationSnapshot,
        runtime_audio: RuntimeAudioWitness,
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
        let checkpoints_agree = !checkpoints.is_empty()
            && checkpoints.iter().all(LiveCheckpoint::agrees)
            && engine_checkpoint_sequence_is_complete(&checkpoints, &coverage)
            && patch_adsr_checkpoint_sequence_is_complete(&checkpoints, installed_patches);
        let runtime_complete = runtime_audio.parsed_soundfont_banks() > 0
            && runtime_audio.prepared_instruments() == installed_patches.len()
            && runtime_composition_matches(&state_tree, runtime_audio)
            && runtime_audio.initial_graph_revision() < runtime_audio.active_graph_revision()
            && runtime_audio.active_graph_revision() == state_tree.graph_revision()
            && runtime_audio.engine_switches() == 3
            && runtime_audio.has_exact_ready_sequence()
            && runtime_audio.fallbacks() == 0
            && runtime_audio.callback_allocations() == 0
            && runtime_audio.callback_destructions() == 0;
        let final_engine_complete = final_soundfont_config_is_default(&state_tree);
        let lossless = event_log.dropped_records() == 0
            && event_log.total_observed() == event_log.records().len() as u64;
        let complete = coverage.is_complete()
            && accepted
            && rejected
            && checkpoints_agree
            && lossless
            && cleanup_complete
            && final_audio_complete
            && runtime_complete
            && final_engine_complete;
        let summary = format!(
            "live demo {}: {}/{} editable parameters, {}/{} engine transitions, {} checkpoints, {} events, {} dropped, banks={}, instruments={}, soundfontPatches={}, braidsPatches={}, alternatingCapabilities={}, initialGraphRevision={}, graphRevision={}, engineSwitches={}, fallbacks={}, callbackAllocations={}, callbackDestructions={}, cleanup={}, activeNotes={}",
            if complete { "complete" } else { "incomplete" },
            coverage.exercised().len(),
            coverage.expected().len(),
            coverage.exercised_engine_transitions().len(),
            coverage.expected_engine_transitions().len(),
            checkpoints.len(),
            event_log.total_observed(),
            event_log.dropped_records(),
            runtime_audio.parsed_soundfont_banks(),
            runtime_audio.prepared_instruments(),
            runtime_audio.soundfont_patches(),
            runtime_audio.braids_patches(),
            runtime_audio.alternating_capabilities(),
            runtime_audio.initial_graph_revision(),
            runtime_audio.active_graph_revision(),
            runtime_audio.engine_switches(),
            runtime_audio.fallbacks(),
            runtime_audio.callback_allocations(),
            runtime_audio.callback_destructions(),
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
            runtime_audio,
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

    pub fn checkpoints(&self) -> &[LiveCheckpoint] {
        &self.checkpoints
    }

    pub const fn event_log(&self) -> &EventLog {
        &self.event_log
    }

    pub fn event_log_summary(&self) -> LiveEventLogSummary {
        LiveEventLogSummary::from_event_log(
            &self.event_log,
            self.runtime_audio.active_graph_revision(),
        )
    }

    pub const fn state_tree(&self) -> &StateTree {
        &self.state_tree
    }

    pub const fn coverage(&self) -> &LiveDemoCoverage {
        &self.coverage
    }

    pub const fn runtime_audio(&self) -> RuntimeAudioWitness {
        self.runtime_audio
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn to_json(&self) -> Result<String, LiveDemoReportError> {
        serde_json::to_string(self).map_err(|_| LiveDemoReportError::Serialization)
    }
}

fn runtime_composition_matches(tree: &StateTree, runtime: RuntimeAudioWitness) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(tree.json()) else {
        return false;
    };
    let Some(patches) = value.get("patches").and_then(serde_json::Value::as_array) else {
        return false;
    };
    let capabilities: Vec<&str> = patches
        .iter()
        .filter_map(|patch| {
            patch
                .pointer("/instrument/capabilityId")
                .and_then(serde_json::Value::as_str)
        })
        .collect();
    if capabilities.len() != patches.len() {
        return false;
    }
    let soundfont = capabilities
        .iter()
        .filter(|capability| **capability == HIDEF_CAPABILITY_ID)
        .count();
    let braids = capabilities
        .iter()
        .filter(|capability| **capability == BRAIDS_CAPABILITY_ID)
        .count();
    let alternating = capabilities.len() > 1
        && soundfont > 0
        && braids > 0
        && capabilities.windows(2).all(|pair| pair[0] != pair[1]);
    soundfont.saturating_add(braids) == patches.len()
        && runtime.soundfont_patches() == soundfont
        && runtime.braids_patches() == braids
        && runtime.alternating_capabilities() == alternating
}

fn engine_checkpoint_sequence_is_complete(
    checkpoints: &[LiveCheckpoint],
    coverage: &LiveDemoCoverage,
) -> bool {
    let engines: Vec<_> = checkpoints
        .iter()
        .filter_map(LiveCheckpoint::as_engine)
        .collect();
    if engines.len()
        != coverage
            .expected_engine_transitions()
            .len()
            .saturating_mul(3)
    {
        return false;
    }
    for (index, expected) in coverage.expected_engine_transitions().iter().enumerate() {
        let offset = index * 3;
        let lifecycle = &engines[offset..offset + 3];
        if lifecycle.iter().any(|checkpoint| {
            checkpoint.transition_index() != index || checkpoint.transition() != expected
        }) || lifecycle[0].status() != crate::control::EngineSelectionStatusKind::Preparing
            || lifecycle[1].status() != crate::control::EngineSelectionStatusKind::Activating
            || lifecycle[2].status() != crate::control::EngineSelectionStatusKind::Ready
            || !lifecycle[2].target_audio_nonzero()
        {
            return false;
        }
        if expected == "SoundFontPresetToNext"
            && (!lifecycle[0].source_audio_nonzero()
                || lifecycle.iter().any(|checkpoint| {
                    checkpoint.preset().is_none()
                        || !matches!(
                            checkpoint.intent(),
                            crate::control::StructuralEditIntent::ReplaceParameterChoice { .. }
                        )
                }))
        {
            return false;
        }
        if expected != "SoundFontPresetToNext"
            && lifecycle.iter().any(|checkpoint| {
                checkpoint.preset().is_some()
                    || !matches!(
                        checkpoint.intent(),
                        crate::control::StructuralEditIntent::ReplaceCapability { .. }
                    )
            })
        {
            return false;
        }
        if index > 0 && lifecycle[2].graph_revision() <= engines[offset - 1].graph_revision() {
            return false;
        }
    }
    true
}

fn patch_adsr_checkpoint_sequence_is_complete(
    checkpoints: &[LiveCheckpoint],
    installed_patches: &[PatchId],
) -> bool {
    let Some(focused_patch) = installed_patches.first().copied() else {
        return false;
    };
    let patch_adsr = checkpoints
        .iter()
        .filter_map(LiveCheckpoint::as_parameter)
        .filter(|checkpoint| {
            checkpoint
                .expected_transition()
                .patch_control_id()
                .is_some()
        })
        .collect::<Vec<_>>();
    if patch_adsr.len() != crate::synth::VoiceEnvelope::surface_descriptor().len() {
        return false;
    }
    patch_adsr
        .iter()
        .zip(crate::synth::VoiceEnvelope::surface_descriptor())
        .all(|(checkpoint, descriptor)| {
            let parameter = descriptor.parameter();
            checkpoint.expected_transition().patch_control_id()
                == Some(crate::control::PatchControlId::Envelope(parameter))
                && matches!(
                    checkpoint.expected_transition().editable_parameter(),
                    Some(LiveEditableParameter::Patch {
                        patch_id,
                        target: crate::synth::PatchEditableTarget::Envelope(actual),
                    }) if *patch_id == focused_patch && *actual == parameter
                )
        })
}

fn final_soundfont_config_is_default(tree: &StateTree) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(tree.json()) else {
        return false;
    };
    let Some(instrument) = value.pointer("/patches/0/instrument").cloned() else {
        return false;
    };
    let Ok(actual) = serde_json::from_value::<crate::synth::InstrumentConfig>(instrument) else {
        return false;
    };
    let Ok(registry) = serde_json::from_value::<crate::synth::CapabilityRegistry>(
        value.get("capabilities").cloned().unwrap_or_default(),
    ) else {
        return false;
    };
    let Some(descriptor) = registry.descriptor(actual.capability_id()) else {
        return false;
    };
    let mut values = Vec::new();
    let mut assets = Vec::new();
    for parameter in descriptor.parameters() {
        match parameter.default_value() {
            crate::synth::ParameterDefault::Value(value) => values.push(
                crate::synth::ParameterAssignment::new(parameter.id().clone(), value.clone()),
            ),
            crate::synth::ParameterDefault::Asset(reference) => assets.push(
                crate::synth::AssetAssignment::new(parameter.id().clone(), reference.clone()),
            ),
        }
    }
    let Ok(expected) = descriptor.create_config(&values, &assets) else {
        return false;
    };
    value
        .pointer("/engineSelection/kind")
        .and_then(serde_json::Value::as_str)
        == Some("ready")
        && actual.capability_id().as_str() == HIDEF_CAPABILITY_ID
        && actual == expected
}

impl Serialize for LiveDemoReport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let state_tree: serde_json::Value =
            serde_json::from_str(self.state_tree.json()).map_err(serde::ser::Error::custom)?;
        let mut report = serializer.serialize_struct("LiveDemoReport", 9)?;
        report.serialize_field("schemaVersion", &Self::SCHEMA_VERSION)?;
        report.serialize_field("scene", &self.scene)?;
        report.serialize_field("complete", &self.complete)?;
        report.serialize_field("checkpoints", &self.checkpoints)?;
        report.serialize_field("eventLog", &self.event_log)?;
        report.serialize_field("stateTree", &state_tree)?;
        report.serialize_field("coverage", &self.coverage)?;
        report.serialize_field("runtimeAudio", &self.runtime_audio)?;
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
        let mut coverage = LiveDemoCoverage::new(&[gain.clone(), master.clone()]);

        coverage.mark_exercised(&gain);
        assert_eq!(coverage.missing(), &["global.masterGainDb"]);
        assert!(!coverage.is_complete());

        coverage.mark_unexpected("patch.1.pan");
        assert_eq!(coverage.unexpected(), &["patch.1.pan"]);
        coverage.mark_exercised(&master);
        assert!(!coverage.is_complete());

        let duplicate = LiveDemoCoverage::new(&[gain.clone(), gain]);
        assert_eq!(duplicate.duplicate_expected(), &["patch.1.gainDb"]);
        assert!(!duplicate.is_complete());
    }
}
