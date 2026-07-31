use crate::adapter::braids_capability::BRAIDS_CAPABILITY_ID;
use crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID;
use crate::control::app_state::EventRejection;
use crate::control::event_log::EventLog;
use crate::control::event_record::{EmittedEvent, EventInput, EventOutcome, EventSource, MidiKind};
use crate::control::state_tree::StateTree;
use crate::control::{
    GraphicalShellProjection, InteractionMode, MixerControlId, SemanticControlId, SurfaceId,
    TopLevelContext,
};
use crate::kernel::patch_id::PatchId;
use crate::mixer::mixer_track_id::MixerTrackId;
use crate::mixer::mixer_track_parameters::MixerTrackParameter;
use crate::mixer::patch_output::{PatchOutput, PatchOutputParameter};
use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
use crate::real_time::callback_safety::{callback_safety_snapshot, CallbackSafetySnapshot};
use crate::real_time::GraphRevision;
use crate::testing::live_demo_checkpoint::{LiveCheckpoint, LiveDemoCheckpoint};
use crate::testing::live_demo_scene::{LiveEditableParameter, LiveEngineTransition};
use crate::testing::live_mixer_routing_measurement::LiveMixerDspEvidence;
use core::fmt;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Rendered-frame coverage accepted only after projection and audio correlation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveShellCoverage {
    context_line_visible: bool,
    identity_header_visible: bool,
    main_workspace_visible: bool,
    persistent_side_region_visible: bool,
    footer_visible: bool,
    patch_context_observed: bool,
    mixer_context_observed: bool,
    patch_main_observed: bool,
    patch_utility_observed: bool,
    mixer_main_observed: bool,
    mixer_inspector_observed: bool,
    navigate_observed: bool,
    adjust_observed: bool,
    patch_return_observed: bool,
    mixer_return_observed: bool,
    healthy_empty_errors_observed: bool,
    focus_recovery_observed: bool,
    physical_audio_nonzero: bool,
    qualifying_frames: u64,
}

impl LiveShellCoverage {
    pub(crate) fn mark_qualifying_frame(&mut self, frame: &crate::shell::ShellFrameObservation) {
        self.context_line_visible = true;
        self.identity_header_visible = true;
        self.main_workspace_visible = true;
        self.persistent_side_region_visible = true;
        self.footer_visible = true;
        self.physical_audio_nonzero = true;
        match frame.context() {
            TopLevelContext::Patch => self.patch_context_observed = true,
            TopLevelContext::Mixer => self.mixer_context_observed = true,
        }
        match frame.active_surface() {
            SurfaceId::PatchMain => {
                self.patch_main_observed = true;
                if self.patch_utility_observed && frame.return_path().is_none() {
                    self.patch_return_observed = true;
                }
            }
            SurfaceId::PatchUtility => self.patch_utility_observed = true,
            SurfaceId::MixerMain => {
                self.mixer_main_observed = true;
                if self.mixer_inspector_observed && frame.return_path().is_none() {
                    self.mixer_return_observed = true;
                }
            }
            SurfaceId::MixerInspector => self.mixer_inspector_observed = true,
        }
        match frame.interaction_mode() {
            InteractionMode::Navigate => self.navigate_observed = true,
            InteractionMode::Adjust => self.adjust_observed = true,
            InteractionMode::Modal | InteractionMode::MultiSelect => {}
        }
        self.healthy_empty_errors_observed |= frame.errors().is_empty();
        self.qualifying_frames = self.qualifying_frames.saturating_add(1);
    }

    pub const fn context_line_visible(&self) -> bool {
        self.context_line_visible
    }

    pub const fn identity_header_visible(&self) -> bool {
        self.identity_header_visible
    }

    pub const fn main_workspace_visible(&self) -> bool {
        self.main_workspace_visible
    }

    pub const fn persistent_side_region_visible(&self) -> bool {
        self.persistent_side_region_visible
    }

    pub const fn footer_visible(&self) -> bool {
        self.footer_visible
    }

    pub const fn patch_context_observed(&self) -> bool {
        self.patch_context_observed
    }

    pub const fn mixer_context_observed(&self) -> bool {
        self.mixer_context_observed
    }

    pub const fn all_semantic_surfaces_observed(&self) -> bool {
        self.patch_main_observed
            && self.patch_utility_observed
            && self.mixer_main_observed
            && self.mixer_inspector_observed
    }

    pub const fn both_reachable_modes_observed(&self) -> bool {
        self.navigate_observed && self.adjust_observed
    }

    pub const fn both_return_round_trips_observed(&self) -> bool {
        self.patch_return_observed && self.mixer_return_observed
    }

    pub const fn healthy_empty_errors_observed(&self) -> bool {
        self.healthy_empty_errors_observed
    }

    pub const fn focus_recovery_observed(&self) -> bool {
        self.focus_recovery_observed
    }

    pub(crate) fn mark_focus_recovery(&mut self) {
        self.focus_recovery_observed = true;
    }

    pub const fn physical_audio_nonzero(&self) -> bool {
        self.physical_audio_nonzero
    }

    pub const fn qualifying_frames(&self) -> u64 {
        self.qualifying_frames
    }

    pub const fn is_complete(&self) -> bool {
        self.context_line_visible
            && self.identity_header_visible
            && self.main_workspace_visible
            && self.persistent_side_region_visible
            && self.footer_visible
            && self.patch_context_observed
            && self.mixer_context_observed
            && self.patch_main_observed
            && self.patch_utility_observed
            && self.mixer_main_observed
            && self.mixer_inspector_observed
            && self.navigate_observed
            && self.adjust_observed
            && self.patch_return_observed
            && self.mixer_return_observed
            && self.healthy_empty_errors_observed
            && self.focus_recovery_observed
            && self.physical_audio_nonzero
            && self.qualifying_frames >= 8
    }
}

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
    #[serde(default)]
    expected_topology_transitions: Vec<String>,
    #[serde(default)]
    exercised_topology_transitions: Vec<String>,
    #[serde(default)]
    missing_topology_transitions: Vec<String>,
    #[serde(default)]
    unexpected_topology_transitions: Vec<String>,
}

/// Measured sixteen-track routing evidence retained by the live report.
///
/// Each predicate is assembled from a canonical StateTree/ParameterSnapshot
/// pair, exact semantic control identities, immutable generation-correlated
/// audio checkpoints, the lossless EventLog, and callback instrumentation.
/// Window, stream, and graph teardown remain shell-owned evidence because they
/// occur only after the report has completed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LiveMixerRoutingEvidence {
    track_count: usize,
    track_ids_exact: bool,
    all_tracks_projected: bool,
    empty_tracks_addressable: bool,
    shared_track_patch_count: usize,
    shared_track_sum_exact: bool,
    track_level_controls_shared_sum: bool,
    patch_trim_isolated: bool,
    patch_reroute_isolated: bool,
    invalid_route_rejected: bool,
    track_parameter_classes_exercised: usize,
    mute_wins: bool,
    any_solo_exact: bool,
    post_gate_sends_exact: bool,
    pre_gate_meters_exact: bool,
    fixed_snapshot_exact: bool,
    stable_focus_exact: bool,
    callback_allocations: usize,
    callback_destructions: usize,
}

impl LiveMixerRoutingEvidence {
    pub(crate) const fn with_callback_safety(
        mut self,
        callback_safety: CallbackSafetySnapshot,
    ) -> Self {
        self.callback_allocations = callback_safety.allocations();
        self.callback_destructions = callback_safety.destructions();
        self
    }

    pub const fn track_count(self) -> usize {
        self.track_count
    }

    pub const fn track_ids_exact(self) -> bool {
        self.track_ids_exact
    }

    pub const fn all_tracks_projected(self) -> bool {
        self.all_tracks_projected
    }

    pub const fn empty_tracks_addressable(self) -> bool {
        self.empty_tracks_addressable
    }

    pub const fn shared_track_patch_count(self) -> usize {
        self.shared_track_patch_count
    }

    pub const fn shared_track_sum_exact(self) -> bool {
        self.shared_track_sum_exact
    }

    pub const fn track_level_controls_shared_sum(self) -> bool {
        self.track_level_controls_shared_sum
    }

    pub const fn patch_trim_isolated(self) -> bool {
        self.patch_trim_isolated
    }

    pub const fn patch_reroute_isolated(self) -> bool {
        self.patch_reroute_isolated
    }

    pub const fn invalid_route_rejected(self) -> bool {
        self.invalid_route_rejected
    }

    pub const fn track_parameter_classes_exercised(self) -> usize {
        self.track_parameter_classes_exercised
    }

    pub const fn mute_wins(self) -> bool {
        self.mute_wins
    }

    pub const fn any_solo_exact(self) -> bool {
        self.any_solo_exact
    }

    pub const fn post_gate_sends_exact(self) -> bool {
        self.post_gate_sends_exact
    }

    pub const fn pre_gate_meters_exact(self) -> bool {
        self.pre_gate_meters_exact
    }

    pub const fn fixed_snapshot_exact(self) -> bool {
        self.fixed_snapshot_exact
    }

    pub const fn stable_focus_exact(self) -> bool {
        self.stable_focus_exact
    }

    pub const fn callback_allocations(self) -> usize {
        self.callback_allocations
    }

    pub const fn callback_destructions(self) -> usize {
        self.callback_destructions
    }

    pub const fn is_complete(self) -> bool {
        self.track_count == MixerTrackId::COUNT
            && self.track_ids_exact
            && self.all_tracks_projected
            && self.empty_tracks_addressable
            && self.shared_track_patch_count > 1
            && self.shared_track_sum_exact
            && self.track_level_controls_shared_sum
            && self.patch_trim_isolated
            && self.patch_reroute_isolated
            && self.invalid_route_rejected
            && self.track_parameter_classes_exercised == TRACK_PARAMETER_CLASSES
            && self.mute_wins
            && self.any_solo_exact
            && self.post_gate_sends_exact
            && self.pre_gate_meters_exact
            && self.fixed_snapshot_exact
            && self.stable_focus_exact
            && self.callback_allocations == 0
            && self.callback_destructions == 0
    }
}

/// Final sixteen-track routing witness emitted after physical teardown.
///
/// Routing predicates are frozen by `LiveDemoReport`; the remaining fields
/// are attached only after the native window returns, the stream is dropped,
/// the preparation worker is shut down, and graph ownership is collected.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SixteenTrackMixerRoutingObservation {
    #[serde(flatten)]
    routing: LiveMixerRoutingEvidence,
    physical_audio_nonzero: bool,
    active_notes_after_cleanup: u32,
    window_closed: bool,
    stream_released: bool,
    owned_graphs_remaining: usize,
}

impl SixteenTrackMixerRoutingObservation {
    pub const fn new(
        routing: LiveMixerRoutingEvidence,
        physical_audio_nonzero: bool,
        active_notes_after_cleanup: u32,
        window_closed: bool,
        stream_released: bool,
        owned_graphs_remaining: usize,
    ) -> Self {
        Self {
            routing,
            physical_audio_nonzero,
            active_notes_after_cleanup,
            window_closed,
            stream_released,
            owned_graphs_remaining,
        }
    }

    pub const fn is_complete(self) -> bool {
        self.routing.is_complete()
            && self.physical_audio_nonzero
            && self.active_notes_after_cleanup == 0
            && self.window_closed
            && self.stream_released
            && self.owned_graphs_remaining == 0
    }
}

impl LiveDemoCoverage {
    pub fn new(expected: &[LiveEditableParameter]) -> Self {
        Self::with_engine_transitions(expected, &[])
    }

    pub fn with_engine_transitions(
        expected: &[LiveEditableParameter],
        engine_transitions: &[LiveEngineTransition],
    ) -> Self {
        Self::with_engine_and_topology_transitions(expected, engine_transitions, &[])
    }

    pub fn with_engine_and_topology_transitions(
        expected: &[LiveEditableParameter],
        engine_transitions: &[LiveEngineTransition],
        topology_transitions: &[crate::testing::live_effects_and_buses_scene::LiveTopologyTransition],
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

        let mut expected_topology = Vec::with_capacity(topology_transitions.len());
        for transition in topology_transitions {
            let identifier = transition.identifier().to_owned();
            if !expected_topology.contains(&identifier) {
                expected_topology.push(identifier);
            }
        }

        Self {
            missing_editable_parameters: expected_ids.clone(),
            expected_editable_parameters: expected_ids,
            duplicate_expected_parameters: duplicate_expected,
            missing_engine_transitions: expected_engines.clone(),
            expected_engine_transitions: expected_engines,
            duplicate_expected_engine_transitions: duplicate_expected_engines,
            missing_topology_transitions: expected_topology.clone(),
            expected_topology_transitions: expected_topology,
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

    pub fn mark_topology_exercised(&mut self, identifier: &str) {
        let identifier = identifier.to_owned();
        if !insert_sorted_unique(&mut self.exercised_topology_transitions, identifier.clone()) {
            return;
        }
        if let Some(index) = self
            .missing_topology_transitions
            .iter()
            .position(|expected| expected == &identifier)
        {
            self.missing_topology_transitions.remove(index);
        } else {
            insert_sorted_unique(&mut self.unexpected_topology_transitions, identifier);
        }
    }

    pub fn expected_topology_transitions(&self) -> &[String] {
        &self.expected_topology_transitions
    }

    pub fn exercised_topology_transitions(&self) -> &[String] {
        &self.exercised_topology_transitions
    }

    pub fn missing_topology_transitions(&self) -> &[String] {
        &self.missing_topology_transitions
    }

    pub fn unexpected_topology_transitions(&self) -> &[String] {
        &self.unexpected_topology_transitions
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
            && self.missing_topology_transitions.is_empty()
            && self.unexpected_topology_transitions.is_empty()
            && self.exercised_topology_transitions.len() == self.expected_topology_transitions.len()
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
    #[serde(skip)]
    callback_safety_source: CallbackSafetySource,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum CallbackSafetySource {
    #[default]
    Fixed,
    Process,
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
            callback_safety_source: CallbackSafetySource::Fixed,
        }
    }

    pub(crate) const fn with_process_callback_safety(mut self) -> Self {
        self.callback_safety_source = CallbackSafetySource::Process;
        self
    }

    pub(crate) fn measured(mut self) -> Self {
        if self.callback_safety_source == CallbackSafetySource::Process {
            let callback_safety = callback_safety_snapshot();
            self.callback_allocations = callback_safety.allocations();
            self.callback_destructions = callback_safety.destructions();
            self.callback_safety_source = CallbackSafetySource::Fixed;
        }
        self
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
    graphical_shell: GraphicalShellProjection,
    coverage: LiveDemoCoverage,
    shell_coverage: LiveShellCoverage,
    final_audio_observation: AudioObservationSnapshot,
    runtime_audio: RuntimeAudioWitness,
    mixer_routing: LiveMixerRoutingEvidence,
    effects_and_buses: Option<LiveEffectsAndBusesEvidence>,
    summary: String,
}

/// Measured effects-and-buses evidence retained by the cumulative live
/// report (WP08): per-transition topology lifecycle coverage, NFR-008 edit
/// responsiveness, and the numeric eight-destination routing measurements
/// (SC-004, SC-005, NFR-007).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LiveEffectsAndBusesEvidence {
    topology_checkpoints: usize,
    slot_fills_exercised: usize,
    startup_occupant_cleared: bool,
    slot_order_exchange_exercised: bool,
    same_entry_twin_exercised: bool,
    sends_raised_toward_destinations: bool,
    return_content_changes: usize,
    empty_return_occupied_and_restored: bool,
    controlled_rejection_observed: bool,
    rejection_reason: String,
    post_rejection_recovery_observed: bool,
    reroute_chain_followed: bool,
    /// NFR-008: worst measured acceptance-to-projection window (frames).
    frames_to_projection_max: u64,
    /// NFR-008: worst observed activation-sequence gap (render blocks);
    /// exactly 1 when the observer sees every block.
    activation_sequence_gap_max: u64,
    /// NFR-008: worst observed distance from the target-note dispatch on an
    /// activated graph to its first audible block; exactly 1 when the
    /// observer sees every block.
    render_blocks_to_audible_max: u64,
    all_audible_on_activated_graph: bool,
    send_isolation_exact: bool,
    max_off_target_bus_dbfs: f32,
    accumulation_exact: bool,
    gated_wet_contribution: f32,
    unoccupied_return_silent: bool,
}

impl LiveEffectsAndBusesEvidence {
    pub const fn topology_checkpoints(&self) -> usize {
        self.topology_checkpoints
    }

    pub const fn frames_to_projection_max(&self) -> u64 {
        self.frames_to_projection_max
    }

    pub const fn activation_sequence_gap_max(&self) -> u64 {
        self.activation_sequence_gap_max
    }

    pub const fn render_blocks_to_audible_max(&self) -> u64 {
        self.render_blocks_to_audible_max
    }

    pub const fn max_off_target_bus_dbfs(&self) -> f32 {
        self.max_off_target_bus_dbfs
    }

    pub const fn gated_wet_contribution(&self) -> f32 {
        self.gated_wet_contribution
    }

    pub fn is_complete(&self) -> bool {
        self.topology_checkpoints > 0
            && self.slot_fills_exercised >= 3
            && self.startup_occupant_cleared
            && self.slot_order_exchange_exercised
            && self.same_entry_twin_exercised
            && self.sends_raised_toward_destinations
            && self.return_content_changes >= 2
            && self.empty_return_occupied_and_restored
            && self.controlled_rejection_observed
            && !self.rejection_reason.is_empty()
            && self.post_rejection_recovery_observed
            && self.reroute_chain_followed
            && self.frames_to_projection_max <= 1
            && self.activation_sequence_gap_max >= 1
            && self.render_blocks_to_audible_max >= 1
            && self.all_audible_on_activated_graph
            && self.send_isolation_exact
            && self.max_off_target_bus_dbfs
                < crate::testing::live_mixer_routing_measurement::ISOLATION_FLOOR_DBFS
            && self.accumulation_exact
            && self.gated_wet_contribution == 0.0
            && self.unoccupied_return_silent
    }
}

fn measure_effects_and_buses(
    checkpoints: &[LiveCheckpoint],
    dsp: &LiveMixerDspEvidence,
) -> Option<LiveEffectsAndBusesEvidence> {
    let topology: Vec<_> = checkpoints
        .iter()
        .filter_map(LiveCheckpoint::as_topology)
        .collect();
    if topology.is_empty() {
        return None;
    }
    let by_id = |prefix: &str| {
        topology
            .iter()
            .filter(|checkpoint| checkpoint.transition().starts_with(prefix))
            .count()
    };
    let complete_by_id = |id: &str| {
        topology
            .iter()
            .any(|checkpoint| checkpoint.transition() == id && checkpoint.agrees())
    };
    let rejection = topology
        .iter()
        .find(|checkpoint| checkpoint.outcome() == EventOutcome::Rejected);
    let rejection_index = topology
        .iter()
        .position(|checkpoint| checkpoint.outcome() == EventOutcome::Rejected);
    let post_rejection_recovery_observed = rejection_index.is_some_and(|index| {
        topology.iter().skip(index + 1).any(|checkpoint| {
            checkpoint.outcome() == EventOutcome::Accepted
                && checkpoint.target_graph_revision().is_some()
                && checkpoint.agrees()
        })
    });
    let frames_to_projection_max = topology
        .iter()
        .filter_map(|checkpoint| checkpoint.frames_to_projection())
        .max()
        .unwrap_or(0);
    let activation_sequence_gap_max = topology
        .iter()
        .filter_map(|checkpoint| checkpoint.observed_activation_sequence_gap())
        .max()
        .unwrap_or(0);
    let render_blocks_to_audible_max = topology
        .iter()
        .filter_map(|checkpoint| checkpoint.render_blocks_to_audible())
        .max()
        .unwrap_or(0);
    Some(LiveEffectsAndBusesEvidence {
        topology_checkpoints: topology.len(),
        slot_fills_exercised: by_id("SlotFill."),
        startup_occupant_cleared: complete_by_id("Slot.startupOccupantCleared"),
        slot_order_exchange_exercised: complete_by_id("SlotOrder.exchangeFirst")
            && complete_by_id("SlotOrder.exchangeSecond"),
        same_entry_twin_exercised: complete_by_id("SlotTwin.sameEntry"),
        sends_raised_toward_destinations: complete_by_id("Send.towardDestinations"),
        return_content_changes: by_id("Return.contentChanged")
            + by_id("Topology.recoveredAfterRefusal"),
        empty_return_occupied_and_restored: complete_by_id("Return.emptyOccupied")
            && complete_by_id("Return.emptyRestored"),
        controlled_rejection_observed: rejection.is_some_and(|checkpoint| checkpoint.agrees()),
        rejection_reason: rejection
            .and_then(|checkpoint| checkpoint.rejection())
            .unwrap_or_default()
            .to_owned(),
        post_rejection_recovery_observed,
        reroute_chain_followed: complete_by_id("Reroute.chainFollows"),
        frames_to_projection_max,
        activation_sequence_gap_max,
        render_blocks_to_audible_max,
        all_audible_on_activated_graph: topology
            .iter()
            .all(|checkpoint| checkpoint.audible_on_activated_graph()),
        send_isolation_exact: dsp.send_isolation_exact,
        max_off_target_bus_dbfs: dsp.max_off_target_bus_dbfs,
        accumulation_exact: dsp.accumulation_exact,
        gated_wet_contribution: dsp.gated_wet_contribution,
        unoccupied_return_silent: dsp.unoccupied_return_silent,
    })
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
    pub const SCHEMA_VERSION: u32 = 10;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scene: impl Into<String>,
        checkpoints: Vec<LiveCheckpoint>,
        event_log: EventLog,
        state_tree: StateTree,
        graphical_shell: GraphicalShellProjection,
        coverage: LiveDemoCoverage,
        shell_coverage: LiveShellCoverage,
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
        let tree_value: serde_json::Value = serde_json::from_str(state_tree.json())
            .map_err(|_| LiveDemoReportError::FinalShellMismatch)?;
        let shell_value = serde_json::to_value(&graphical_shell)
            .map_err(|_| LiveDemoReportError::FinalShellMismatch)?;
        if graphical_shell.generation() != state_tree.generation()
            || graphical_shell.state_hash() != state_tree.state_hash()
            || tree_value.get("graphicalShell") != Some(&shell_value)
        {
            return Err(LiveDemoReportError::FinalShellMismatch);
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
            && patch_control_checkpoint_sequence_is_complete(
                &checkpoints,
                installed_patches,
                &state_tree,
            );
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
        let mixer_routing = measure_mixer_routing(
            &checkpoints,
            &event_log,
            &state_tree,
            &graphical_shell,
            runtime_audio,
        );
        let effects_and_buses =
            measure_effects_and_buses(&checkpoints, &LiveMixerDspEvidence::measure());
        let lossless = event_log.dropped_records() == 0
            && event_log.total_observed() == event_log.records().len() as u64;
        let complete = coverage.is_complete()
            && shell_coverage.is_complete()
            && accepted
            && rejected
            && checkpoints_agree
            && lossless
            && cleanup_complete
            && final_audio_complete
            && runtime_complete
            && final_engine_complete
            && mixer_routing.is_complete()
            && effects_and_buses
                .as_ref()
                .is_none_or(LiveEffectsAndBusesEvidence::is_complete);
        let summary = format!(
            "live demo {}: {}/{} editable parameters, {}/{} engine transitions, {} qualifying shell frames, {} checkpoints, {} events, {} dropped, banks={}, instruments={}, soundfontPatches={}, braidsPatches={}, alternatingCapabilities={}, initialGraphRevision={}, graphRevision={}, engineSwitches={}, fallbacks={}, callbackAllocations={}, callbackDestructions={}, cleanup={}, activeNotes={}",
            if complete { "complete" } else { "incomplete" },
            coverage.exercised().len(),
            coverage.expected().len(),
            coverage.exercised_engine_transitions().len(),
            coverage.expected_engine_transitions().len(),
            shell_coverage.qualifying_frames(),
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
            graphical_shell,
            coverage,
            shell_coverage,
            final_audio_observation: final_observation,
            runtime_audio,
            mixer_routing,
            effects_and_buses,
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

    pub const fn graphical_shell(&self) -> &GraphicalShellProjection {
        &self.graphical_shell
    }

    pub const fn coverage(&self) -> &LiveDemoCoverage {
        &self.coverage
    }

    pub const fn shell_coverage(&self) -> &LiveShellCoverage {
        &self.shell_coverage
    }

    pub const fn final_audio_observation(&self) -> AudioObservationSnapshot {
        self.final_audio_observation
    }

    pub const fn runtime_audio(&self) -> RuntimeAudioWitness {
        self.runtime_audio
    }

    pub const fn effects_and_buses(&self) -> Option<&LiveEffectsAndBusesEvidence> {
        self.effects_and_buses.as_ref()
    }

    pub const fn mixer_routing(&self) -> LiveMixerRoutingEvidence {
        self.mixer_routing
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn to_json(&self) -> Result<String, LiveDemoReportError> {
        serde_json::to_string(self).map_err(|_| LiveDemoReportError::Serialization)
    }
}

fn measure_mixer_routing(
    checkpoints: &[LiveCheckpoint],
    event_log: &EventLog,
    state_tree: &StateTree,
    graphical_shell: &GraphicalShellProjection,
    runtime_audio: RuntimeAudioWitness,
) -> LiveMixerRoutingEvidence {
    let dsp = LiveMixerDspEvidence::measure();
    let tree = serde_json::from_str::<serde_json::Value>(state_tree.json()).ok();
    let track_count = tree
        .as_ref()
        .and_then(|value| value.pointer("/mixer/tracks"))
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);

    let expected_main_controls = MixerTrackId::ALL
        .into_iter()
        .flat_map(|track_id| {
            MixerTrackParameter::MAIN
                .into_iter()
                .map(move |parameter| (track_id, parameter))
        })
        .collect::<Vec<_>>();
    let projected_main_controls = graphical_shell
        .semantic_model()
        .surface(SurfaceId::MixerMain)
        .map(|surface| {
            surface
                .controls()
                .iter()
                .filter_map(|control| match control.path().control_id() {
                    SemanticControlId::Mixer(MixerControlId::Track {
                        track_id,
                        parameter,
                    }) => Some((*track_id, *parameter)),
                    SemanticControlId::Mixer(_)
                    | SemanticControlId::Patch(_)
                    | SemanticControlId::SurfaceRoot => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let track_ids_exact = projected_main_controls == expected_main_controls;

    let parameter_checkpoints = checkpoints
        .iter()
        .filter_map(LiveCheckpoint::as_parameter)
        .collect::<Vec<_>>();
    let mut exercised = [[false; TRACK_PARAMETER_CLASSES]; MixerTrackId::COUNT];
    let mut mute_observed = [false; MixerTrackId::COUNT];
    let mut solo_observed = [false; MixerTrackId::COUNT];
    let mut reverb_observed = [false; MixerTrackId::COUNT];
    let mut delay_observed = [false; MixerTrackId::COUNT];
    let mut every_meter_set_is_finite = true;

    for checkpoint in &parameter_checkpoints {
        let observation = checkpoint.audio_observation();
        every_meter_set_is_finite &= observation.tracks().iter().all(|meter| {
            meter.left_peak().is_finite()
                && meter.right_peak().is_finite()
                && meter.rms().is_finite()
                && meter.left_peak() >= 0.0
                && meter.right_peak() >= 0.0
                && meter.rms() >= 0.0
        });
        if let Some(LiveEditableParameter::Send { track_id, bus }) =
            checkpoint.expected_transition().editable_parameter()
        {
            // The scene's two audible indexed sends carry the send-coverage
            // evidence the retired per-name aliases carried before.
            let send_observed = checkpoint.agrees()
                && checkpoint
                    .expected_transition()
                    .value_after()
                    .is_some_and(|value| value > 0.0)
                && observation.track(*track_id).rms() > 0.0;
            match bus.index() {
                0 => {
                    exercised[track_id.index()][4] = true;
                    reverb_observed[track_id.index()] = send_observed
                        && observation.reverb_input_rms() > 0.0
                        && matches!(
                            checkpoint.audio_predicate(),
                            crate::testing::live_demo_scene::LiveAudioPredicate::TrackReverbInput {
                                track_id: observed
                            } if observed == *track_id
                        );
                }
                1 => {
                    exercised[track_id.index()][5] = true;
                    delay_observed[track_id.index()] = send_observed
                        && observation.delay_input_rms() > 0.0
                        && matches!(
                            checkpoint.audio_predicate(),
                            crate::testing::live_demo_scene::LiveAudioPredicate::TrackDelayInput {
                                track_id: observed
                            } if observed == *track_id
                        );
                }
                _ => {}
            }
            continue;
        }
        let Some(LiveEditableParameter::Track {
            track_id,
            parameter,
        }) = checkpoint.expected_transition().editable_parameter()
        else {
            continue;
        };
        exercised[track_id.index()][track_parameter_index(*parameter)] = true;
        let meter_nonzero = observation.track(*track_id).rms() > 0.0;
        let accepted_toggle = checkpoint.expected_transition().value_after() == Some(1.0);
        match parameter {
            MixerTrackParameter::Mute => {
                mute_observed[track_id.index()] = checkpoint.agrees()
                    && accepted_toggle
                    && meter_nonzero
                    && matches!(
                        checkpoint.audio_predicate(),
                        crate::testing::live_demo_scene::LiveAudioPredicate::TrackPreGateMeter {
                            track_id: observed
                        } if observed == *track_id
                    );
            }
            MixerTrackParameter::Solo => {
                solo_observed[track_id.index()] = checkpoint.agrees()
                    && accepted_toggle
                    && meter_nonzero
                    && observation.output_rms() > 0.0
                    && matches!(
                        checkpoint.audio_predicate(),
                        crate::testing::live_demo_scene::LiveAudioPredicate::TrackOutput {
                            track_id: observed
                        } if observed == *track_id
                    );
            }
            MixerTrackParameter::Level | MixerTrackParameter::Pan => {}
        }
    }

    let all_track_parameters_exercised = exercised.iter().all(|track| track.iter().all(|set| *set));
    let track_parameter_classes_exercised = (0..TRACK_PARAMETER_CLASSES)
        .filter(|parameter| exercised.iter().any(|track| track[*parameter]))
        .count();
    let all_tracks_projected =
        track_count == MixerTrackId::COUNT && track_ids_exact && all_track_parameters_exercised;

    let installed_outputs = installed_patch_outputs(event_log);
    let final_outputs = tree
        .as_ref()
        .and_then(tree_patch_outputs)
        .unwrap_or_default();
    let outputs_restored = !installed_outputs.is_empty() && final_outputs == installed_outputs;
    let final_routes = final_outputs
        .iter()
        .map(|(_, output)| output.track_id())
        .collect::<Vec<_>>();
    let empty_tracks_addressable = MixerTrackId::ALL.into_iter().any(|track_id| {
        !final_routes.contains(&track_id) && exercised[track_id.index()].iter().all(|set| *set)
    });

    let route_checkpoint = parameter_checkpoints.iter().copied().find(|checkpoint| {
        matches!(
            checkpoint.expected_transition().editable_parameter(),
            Some(LiveEditableParameter::PatchOutput {
                parameter: PatchOutputParameter::OutputTrack,
                ..
            })
        )
    });
    let trim_checkpoint = parameter_checkpoints.iter().copied().find(|checkpoint| {
        matches!(
            checkpoint.expected_transition().editable_parameter(),
            Some(LiveEditableParameter::PatchOutput {
                parameter: PatchOutputParameter::TrimGain,
                ..
            })
        )
    });

    let route_identity = route_checkpoint.and_then(|checkpoint| {
        let LiveEditableParameter::PatchOutput { patch_id, .. } =
            checkpoint.expected_transition().editable_parameter()?
        else {
            return None;
        };
        let value = checkpoint.expected_transition().value_after()?;
        if !value.is_finite() || value.fract() != 0.0 {
            return None;
        }
        let value = u8::try_from(value as i64).ok()?;
        MixerTrackId::new(value)
            .ok()
            .map(|track_id| (*patch_id, track_id))
    });
    let shared_track_patch_count = route_identity.map_or(0, |(patch_id, target)| {
        installed_outputs
            .iter()
            .filter(|(candidate, output)| {
                output.track_id() == target
                    || (*candidate == patch_id && output.track_id() != target)
            })
            .count()
    });

    let route_audio_exact =
        route_checkpoint
            .zip(route_identity)
            .is_some_and(|(checkpoint, (patch_id, track_id))| {
                let observation = checkpoint.audio_observation();
                scalar_checkpoint_matches_graph(checkpoint, runtime_audio.initial_graph_revision())
                    && observation.primary_patch_id() == Some(patch_id)
                    && observation.primary_active_notes() > 0
                    && observation.primary_patch_rms() > 0.0
                    && observation.track(track_id).rms() > 0.0
                    && observation.output_rms() > 0.0
            });
    let shared_level_exact = route_identity.is_some_and(|(_, shared_track)| {
        parameter_checkpoints.iter().any(|checkpoint| {
            matches!(
                checkpoint.expected_transition().editable_parameter(),
                Some(LiveEditableParameter::Track {
                    track_id,
                    parameter: MixerTrackParameter::Level,
                }) if *track_id == shared_track
            ) && scalar_checkpoint_matches_graph(checkpoint, runtime_audio.initial_graph_revision())
                && checkpoint.audio_observation().active_notes() >= 2
                && checkpoint.audio_observation().track(shared_track).rms() > 0.0
                && checkpoint.audio_observation().output_rms() > 0.0
        })
    });

    let patch_trim_isolated = trim_checkpoint.is_some_and(|checkpoint| {
        outputs_restored
            && scalar_checkpoint_matches_graph(checkpoint, runtime_audio.initial_graph_revision())
            && checkpoint.expected_transition().value_before()
                != checkpoint.expected_transition().value_after()
            && checkpoint.audio_observation().output_rms() > 0.0
    });
    let patch_reroute_isolated = route_checkpoint.is_some_and(|checkpoint| {
        outputs_restored
            && scalar_checkpoint_matches_graph(checkpoint, runtime_audio.initial_graph_revision())
            && checkpoint.expected_transition().value_before()
                != checkpoint.expected_transition().value_after()
            && route_audio_exact
    });
    let invalid_route_rejected = event_log.records().iter().any(|record| {
        record.source() == EventSource::DemoScene
            && record.outcome() == EventOutcome::Rejected
            && record.rejection() == Some(EventRejection::ParameterAtBoundary)
            && matches!(record.input(), EventInput::Adjust { .. })
            && record.generation_before() == record.generation_after()
            && record.parameter_generation() == record.generation_after()
            && record.state_hash_before() == record.state_hash_after()
            && record.emitted_events().is_empty()
    });

    let fixed_snapshot_exact = tree.as_ref().is_some_and(|value| {
        value.pointer("/mixer/tracks") == value.pointer("/parameters/mixerTracks")
            && value
                .pointer("/patches")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|patches| {
                    value
                        .pointer("/parameters/patches")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|parameters| {
                            patches.len() == parameters.len()
                                && patches.iter().zip(parameters).all(|(patch, parameter)| {
                                    patch.get("output") == parameter.get("output")
                                })
                        })
                })
            && value
                .pointer("/parameters/patchCount")
                .and_then(serde_json::Value::as_u64)
                == value
                    .pointer("/patches")
                    .and_then(serde_json::Value::as_array)
                    .map(|patches| patches.len() as u64)
            && track_count == MixerTrackId::COUNT
            && parameter_checkpoints
                .iter()
                .all(|checkpoint| checkpoint.agrees())
    });
    let stable_focus_exact = all_track_parameters_exercised
        && parameter_checkpoints
            .iter()
            .filter(|checkpoint| {
                matches!(
                    checkpoint.expected_transition().editable_parameter(),
                    Some(LiveEditableParameter::Track { .. } | LiveEditableParameter::Send { .. })
                )
            })
            .count()
            == MixerTrackId::COUNT * TRACK_PARAMETER_CLASSES;

    LiveMixerRoutingEvidence {
        track_count,
        track_ids_exact,
        all_tracks_projected,
        empty_tracks_addressable,
        shared_track_patch_count,
        shared_track_sum_exact: shared_track_patch_count > 1
            && shared_level_exact
            && dsp.shared_track_sum_exact,
        track_level_controls_shared_sum: shared_track_patch_count > 1
            && shared_level_exact
            && dsp.track_level_controls_shared_sum,
        patch_trim_isolated: patch_trim_isolated && dsp.patch_trim_isolated,
        patch_reroute_isolated: patch_reroute_isolated && dsp.patch_reroute_isolated,
        invalid_route_rejected,
        track_parameter_classes_exercised,
        mute_wins: mute_observed.into_iter().all(|observed| observed) && dsp.mute_wins,
        any_solo_exact: solo_observed.into_iter().all(|observed| observed) && dsp.any_solo_exact,
        post_gate_sends_exact: reverb_observed.into_iter().all(|observed| observed)
            && delay_observed.into_iter().all(|observed| observed)
            && dsp.post_gate_sends_exact,
        pre_gate_meters_exact: every_meter_set_is_finite
            && mute_observed.into_iter().all(|observed| observed)
            && dsp.pre_gate_meters_exact,
        fixed_snapshot_exact,
        stable_focus_exact,
        callback_allocations: runtime_audio.callback_allocations(),
        callback_destructions: runtime_audio.callback_destructions(),
    }
}

/// The retained sixteen-track evidence matrix width: the four `MAIN` fader
/// classes plus the scene's two production-audible indexed sends (the buses
/// whose returns are occupied by default). The retired per-name send aliases
/// held positions 4 and 5; the indexed sends toward buses 0 and 1 keep those
/// exact positions so the retained evidence stays comparable.
const TRACK_PARAMETER_CLASSES: usize = MixerTrackParameter::MAIN.len() + 2;

const fn track_parameter_index(parameter: MixerTrackParameter) -> usize {
    match parameter {
        MixerTrackParameter::Level => 0,
        MixerTrackParameter::Pan => 1,
        MixerTrackParameter::Mute => 2,
        MixerTrackParameter::Solo => 3,
    }
}

fn scalar_checkpoint_matches_graph(
    checkpoint: &LiveDemoCheckpoint,
    graph_revision: GraphRevision,
) -> bool {
    let generation = checkpoint.generation();
    checkpoint.agrees()
        && checkpoint.audio_observation().active_graph_revision() == graph_revision
        && matches!(
            checkpoint.emitted_effects(),
            [
                EmittedEvent::StateAccepted {
                    generation: accepted
                },
                EmittedEvent::ParameterSnapshotPublished {
                    generation: published,
                    graph_revision: published_graph,
                },
            ] if *accepted == generation
                && *published == generation
                && *published_graph == graph_revision
        )
}

fn installed_patch_outputs(event_log: &EventLog) -> Vec<(PatchId, PatchOutput)> {
    event_log
        .records()
        .iter()
        .find_map(|record| match record.input() {
            EventInput::InstallPatches { patches } => Some(
                patches
                    .iter()
                    .filter_map(|patch| {
                        PatchId::new(patch.id())
                            .ok()
                            .map(|patch_id| (patch_id, patch.output()))
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn tree_patch_outputs(value: &serde_json::Value) -> Option<Vec<(PatchId, PatchOutput)>> {
    value
        .pointer("/patches")?
        .as_array()?
        .iter()
        .map(|patch| {
            let patch_id = u32::try_from(patch.get("id")?.as_u64()?).ok()?;
            let patch_id = PatchId::new(patch_id).ok()?;
            let output = serde_json::from_value(patch.get("output")?.clone()).ok()?;
            Some((patch_id, output))
        })
        .collect()
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

fn patch_control_checkpoint_sequence_is_complete(
    checkpoints: &[LiveCheckpoint],
    installed_patches: &[PatchId],
    state_tree: &StateTree,
) -> bool {
    let Some(focused_patch) = installed_patches.first().copied() else {
        return false;
    };
    let patch_adsr = checkpoints
        .iter()
        .filter_map(LiveCheckpoint::as_parameter)
        .filter(|checkpoint| {
            matches!(
                checkpoint.expected_transition().patch_control_id(),
                Some(crate::control::PatchControlId::Envelope(_))
            )
        })
        .collect::<Vec<_>>();
    if patch_adsr.len() != crate::synth::VoiceEnvelope::surface_descriptor().len() {
        return false;
    }
    let adsr_complete = patch_adsr
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
        });
    if !adsr_complete {
        return false;
    }

    let Ok(value) = serde_json::from_str::<serde_json::Value>(state_tree.json()) else {
        return false;
    };
    let Ok(registry) = serde_json::from_value::<crate::synth::EffectCapabilityRegistry>(
        value.get("effects").cloned().unwrap_or_default(),
    ) else {
        return false;
    };
    let Some(configs) = value
        .pointer("/patches/0/postEffects")
        .cloned()
        .and_then(|configs| {
            serde_json::from_value::<Vec<crate::synth::PostEffectConfig>>(configs).ok()
        })
    else {
        return false;
    };
    let mut expected_effects = Vec::new();
    for config in &configs {
        let Some(descriptor) = registry.descriptor(config.capability_id()) else {
            return false;
        };
        for spec in descriptor.parameters() {
            let predicate_satisfied = |predicate: Option<&crate::synth::ParameterPredicate>| {
                predicate.is_none_or(|predicate| {
                    config.value(predicate.parameter_id()) == Some(predicate.equals())
                })
            };
            if spec.patch_interaction() == crate::synth::PatchInteraction::ScalarEdit
                && predicate_satisfied(spec.visible_when())
                && predicate_satisfied(spec.enabled_when())
            {
                expected_effects.push((
                    crate::control::PatchControlId::Effect(config.slot_id(), spec.id().clone()),
                    crate::testing::live_demo_scene::LiveEditableParameter::effect(
                        focused_patch,
                        config.slot_id(),
                        spec.id().clone(),
                    ),
                ));
            }
        }
    }
    let actual_effects = checkpoints
        .iter()
        .filter_map(LiveCheckpoint::as_parameter)
        .filter(|checkpoint| {
            matches!(
                checkpoint.expected_transition().patch_control_id(),
                Some(crate::control::PatchControlId::Effect(_, _))
            )
        })
        .collect::<Vec<_>>();
    actual_effects.len() == expected_effects.len()
        && actual_effects
            .iter()
            .zip(expected_effects)
            .all(|(checkpoint, (control, parameter))| {
                checkpoint.expected_transition().patch_control_id() == Some(control)
                    && checkpoint.expected_transition().editable_parameter() == Some(&parameter)
                    && matches!(
                        checkpoint.audio_predicate(),
                        crate::testing::live_demo_scene::LiveAudioPredicate::PatchEffect {
                            patch_id
                        } if patch_id == focused_patch
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
    let Ok(effect_registry) = serde_json::from_value::<crate::synth::EffectCapabilityRegistry>(
        value.get("effects").cloned().unwrap_or_default(),
    ) else {
        return false;
    };
    let Some(post_effects) = value
        .pointer("/patches/0/postEffects")
        .cloned()
        .and_then(|effects| {
            serde_json::from_value::<Vec<crate::synth::PostEffectConfig>>(effects).ok()
        })
    else {
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
        && post_effects.len() == 1
        && effect_registry
            .validate_patch_effects(&post_effects)
            .is_ok()
}

impl Serialize for LiveDemoReport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let state_tree: serde_json::Value =
            serde_json::from_str(self.state_tree.json()).map_err(serde::ser::Error::custom)?;
        let mut report = serializer.serialize_struct("LiveDemoReport", 14)?;
        report.serialize_field("schemaVersion", &Self::SCHEMA_VERSION)?;
        report.serialize_field("scene", &self.scene)?;
        report.serialize_field("complete", &self.complete)?;
        report.serialize_field("checkpoints", &self.checkpoints)?;
        report.serialize_field("eventLog", &self.event_log)?;
        report.serialize_field("stateTree", &state_tree)?;
        report.serialize_field("graphicalShell", &self.graphical_shell)?;
        report.serialize_field("coverage", &self.coverage)?;
        report.serialize_field("shellCoverage", &self.shell_coverage)?;
        report.serialize_field("finalAudioObservation", &self.final_audio_observation)?;
        report.serialize_field("runtimeAudio", &self.runtime_audio)?;
        report.serialize_field("mixerRouting", &self.mixer_routing)?;
        report.serialize_field("effectsAndBuses", &self.effects_and_buses)?;
        report.serialize_field("summary", &self.summary)?;
        report.end()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiveDemoReportError {
    EmptyScene,
    EmptyEventLog,
    FinalStateMismatch,
    FinalShellMismatch,
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
            Self::FinalShellMismatch => formatter
                .write_str("final graphical shell is not the canonical StateTree shell endpoint"),
            Self::Serialization => formatter.write_str("live demo report could not be serialized"),
        }
    }
}

impl std::error::Error for LiveDemoReportError {}

#[cfg(test)]
mod tests {
    use super::LiveDemoCoverage;
    use crate::mixer::global_parameters::GlobalParameter;
    use crate::mixer::mixer_track_id::MixerTrackId;
    use crate::mixer::mixer_track_parameters::MixerTrackParameter;
    use crate::testing::live_demo_scene::LiveEditableParameter;

    #[test]
    fn exact_coverage_reports_missing_unexpected_and_duplicate_expected_values() {
        let gain =
            LiveEditableParameter::track(MixerTrackId::default(), MixerTrackParameter::Level);
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
        assert_eq!(duplicate.duplicate_expected(), &["track.T00.levelDb"]);
        assert!(!duplicate.is_complete());
    }
}
