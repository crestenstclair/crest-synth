use crate::kernel::midi_message::MidiMessageKind;
use crate::kernel::patch_id::PatchId;
use crate::real_time::audio_boundary::AudioThreadBoundary;
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::audio_observation::{CallbackAudioObservation, DiscardAudioObservation};
use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
use crate::real_time::graph_handoff_status::GraphHandoffStatus;
use crate::real_time::graph_revision::GraphRevision;
use crate::real_time::parameter_snapshot::MAX_PATCHES;
use crate::real_time::prepared_engine_rack::RackDispatchError;
use crate::real_time::prepared_graph::PreparedGraph;
use crate::real_time::structural_graph_boundary::AudioStructuralGraphBoundary;
use crate::real_time::PatchEffectObservation;
/// Hard-real-time owner of one active complete prepared graph.
///
/// Construction receives every engine, effect, stem, route, and scratch owner
/// already prepared. `render` performs only bounded graph handoff, command
/// dispatch, compatible scalar selection, synthesis, mixing, and observation.
pub struct AudioRenderer<Boundary, Structural, Observation = DiscardAudioObservation> {
    boundary: Boundary,
    structural: Structural,
    observation: Observation,
    active_graph: PreparedGraph,
    pending_retirement: Option<PreparedGraph>,
    parameters: crate::real_time::parameter_snapshot::ParameterSnapshot,
    handoff_status: GraphHandoffStatus,
    active_notes: ActiveNoteObservation,
    rendered_blocks: u64,
    rendered_frames: u64,
    commands_consumed: u64,
    routing_failures: u64,
    last_unknown_patch_id: Option<PatchId>,
}

impl<Boundary, Structural> AudioRenderer<Boundary, Structural, DiscardAudioObservation>
where
    Boundary: AudioThreadBoundary,
    Structural: AudioStructuralGraphBoundary,
{
    #[must_use]
    pub fn new(boundary: Boundary, structural: Structural, initial_graph: PreparedGraph) -> Self {
        Self::with_observation(boundary, structural, initial_graph, DiscardAudioObservation)
    }
}

impl<Boundary, Structural, Observation> AudioRenderer<Boundary, Structural, Observation>
where
    Boundary: AudioThreadBoundary,
    Structural: AudioStructuralGraphBoundary,
    Observation: CallbackAudioObservation,
{
    #[must_use]
    pub fn with_observation(
        boundary: Boundary,
        mut structural: Structural,
        initial_graph: PreparedGraph,
        observation: Observation,
    ) -> Self {
        let parameters = *initial_graph.initial_parameters();
        let handoff_status = GraphHandoffStatus::with_active(initial_graph.revision());
        structural.publish_status_on_audio(handoff_status);
        Self {
            boundary,
            structural,
            observation,
            active_graph: initial_graph,
            pending_retirement: None,
            parameters,
            handoff_status,
            active_notes: ActiveNoteObservation::new(),
            rendered_blocks: 0,
            rendered_frames: 0,
            commands_consumed: 0,
            routing_failures: 0,
            last_unknown_patch_id: None,
        }
    }

    /// Applies structural ownership only at block start, drains ready commands,
    /// consumes at most one compatible latest scalar snapshot, and renders the
    /// active graph into interleaved stereo output.
    pub fn render(&mut self, interleaved_stereo: &mut [f32]) {
        interleaved_stereo.fill(0.0);
        let capacity_samples = self.active_graph.max_frames().saturating_mul(2);
        if capacity_samples == 0 {
            return;
        }
        let usable_samples = interleaved_stereo.len() - interleaved_stereo.len() % 2;
        for block in interleaved_stereo[..usable_samples].chunks_mut(capacity_samples) {
            self.render_bounded_block(block);
        }
    }

    fn render_bounded_block(&mut self, interleaved_stereo: &mut [f32]) {
        self.apply_structural_handoff();

        // Select the newest compatible complete projection before command
        // delivery so note-on/note-off latching observes this generation.
        let latest = self.boundary.read_latest_parameters();
        let compatible = latest.graph_revision() == self.active_graph.revision()
            && self.active_graph.engine_rack().matches_parameters(&latest)
            && self.active_graph.effect_rack().matches_parameters(&latest)
            && self
                .active_graph
                .bus_return_rack()
                .matches_parameters(&latest);
        if compatible {
            self.parameters = latest;
        } else {
            self.handoff_status.record_incompatible_snapshot();
        }
        self.structural.publish_status_on_audio(self.handoff_status);

        while let Some(command) = self.boundary.pop_command() {
            self.commands_consumed = self.commands_consumed.saturating_add(1);
            match command {
                AudioCommand::PatchMidi { patch_id, message } => {
                    let matching_parameters = self.parameters.patch(patch_id).copied();
                    let dispatch_result = matching_parameters.map(|parameters| {
                        let (rack, _, _) = self.active_graph.callback_parts_mut();
                        rack.dispatch(patch_id, message, &parameters)
                    });
                    match dispatch_result {
                        Some(Ok(())) => {
                            self.active_notes.observe_patch_message(patch_id, message);
                        }
                        None | Some(Err(RackDispatchError::UnknownPatch { .. })) => {
                            self.routing_failures = self.routing_failures.saturating_add(1);
                            self.last_unknown_patch_id = Some(patch_id);
                        }
                        Some(Err(
                            RackDispatchError::Instrument { .. }
                            | RackDispatchError::ParameterLayoutMismatch { .. },
                        )) => {}
                    }
                }
                AudioCommand::AllNotesOff => {
                    let (rack, _, _) = self.active_graph.callback_parts_mut();
                    rack.all_notes_off();
                    self.active_notes.clear_all();
                }
            }
        }

        let frame_count = interleaved_stereo.len() / 2;
        if frame_count == 0 {
            return;
        }

        let parameters = self.parameters;
        let mut effect_observations = [PatchEffectObservation::EMPTY; MAX_PATCHES];
        let (primary_patch_id, primary_patch_rms, patch_effect, mix) = {
            let (rack, effect_rack, patch_audio, mixer) =
                self.active_graph.callback_parts_with_effects_mut();
            if patch_audio.begin_render(&parameters, frame_count).is_err() {
                return;
            }
            if rack.render(patch_audio, &parameters).is_err() {
                interleaved_stereo.fill(0.0);
                return;
            }
            if effect_rack
                .process(patch_audio, &parameters, &mut effect_observations)
                .is_err()
            {
                interleaved_stereo.fill(0.0);
                self.routing_failures = self.routing_failures.saturating_add(1);
                return;
            }
            let primary = patch_audio.stems().first();
            let primary_patch_id = primary.and_then(|stem| stem.patch_id());
            let primary_patch_rms = primary.map_or(0.0, |stem| {
                let samples = stem.samples();
                if samples.is_empty() {
                    0.0
                } else {
                    let energy = samples.iter().fold(0.0_f64, |sum, sample| {
                        sum + f64::from(*sample) * f64::from(*sample)
                    });
                    (energy / samples.len() as f64).sqrt() as f32
                }
            });
            let patch_effect = effect_observations
                .iter()
                .copied()
                .find(|observation| observation.patch_id().is_some())
                .unwrap_or(PatchEffectObservation::EMPTY);
            (
                primary_patch_id,
                primary_patch_rms,
                patch_effect,
                mixer.mix(patch_audio, &parameters, interleaved_stereo),
            )
        };

        self.rendered_blocks = self.rendered_blocks.saturating_add(1);
        self.rendered_frames = self.rendered_frames.saturating_add(frame_count as u64);
        let primary_active_notes =
            primary_patch_id.map_or(0, |patch_id| self.active_notes.count_patch(patch_id));
        self.observation.publish_from_callback(
            AudioObservationSnapshot::from_mix_with_graph_routing_primary_and_effect(
                self.rendered_blocks,
                self.rendered_blocks,
                self.rendered_frames,
                parameters.generation(),
                self.active_graph.revision(),
                self.commands_consumed,
                self.active_notes.count(),
                self.routing_failures,
                self.last_unknown_patch_id,
                primary_patch_id,
                primary_patch_rms,
                primary_active_notes,
                patch_effect,
                mix,
            ),
        );
    }

    pub const fn active_revision(&self) -> GraphRevision {
        self.active_graph.revision()
    }

    pub fn pending_retirement_revision(&self) -> Option<GraphRevision> {
        self.pending_retirement
            .as_ref()
            .map(PreparedGraph::revision)
    }

    pub const fn handoff_status(&self) -> GraphHandoffStatus {
        self.handoff_status
    }

    pub const fn parameters(&self) -> &crate::real_time::parameter_snapshot::ParameterSnapshot {
        &self.parameters
    }

    /// Returns the active graph's caller-owned Patch stems for control-side
    /// deterministic verification when no physical callback owns the renderer.
    pub fn active_patch_audio(&self) -> &crate::real_time::patch_audio_block::PatchAudioBlock {
        self.active_graph.patch_audio()
    }

    fn apply_structural_handoff(&mut self) {
        if let Some(retired) = self.pending_retirement.take() {
            let retired_revision = retired.revision();
            match self.structural.return_retired_on_audio(retired) {
                Ok(()) => self.handoff_status.record_retired(retired_revision),
                Err(full) => {
                    self.pending_retirement = Some(full.into_graph());
                    self.handoff_status.record_retirement_retry();
                    self.structural.publish_status_on_audio(self.handoff_status);
                    return;
                }
            }
        }

        let Some(mut replacement) = self.structural.take_prepared_on_audio() else {
            self.structural.publish_status_on_audio(self.handoff_status);
            return;
        };

        let replacement_revision = replacement.revision();
        let replacement_parameters = *replacement.initial_parameters();
        // WP10 voice carry-over: before the graphs exchange, move every
        // still-live prepared instance the declared delta leaves unchanged
        // from the active graph into the replacement — bounded pointer swaps,
        // no allocation, no destruction. The fresh never-sounded instances
        // ride into the superseded graph and retire off-callback as before.
        let carry_over = replacement.carry_over_scope();
        replacement.carry_live_state_from(&mut self.active_graph);
        let retired = core::mem::replace(&mut self.active_graph, replacement);
        let retired_revision = retired.revision();
        self.parameters = replacement_parameters;
        match carry_over {
            // No declared delta: the full-reset swap semantics remain.
            None => self.active_notes.clear_all(),
            // Only the selected Patch's engine restarted; every other
            // sounding voice carried over with its engine.
            Some(crate::real_time::GraphReplacementScope::SelectedEngine(patch_id)) => {
                self.active_notes.clear_patch(patch_id);
            }
            // Occupancy deltas leave every engine identity unchanged: every
            // sounding voice carried over.
            Some(
                crate::real_time::GraphReplacementScope::PatchSlot { .. }
                | crate::real_time::GraphReplacementScope::BusReturn(_),
            ) => {}
        }
        self.handoff_status.record_swap(replacement_revision);

        match self.structural.return_retired_on_audio(retired) {
            Ok(()) => self.handoff_status.record_retired(retired_revision),
            Err(full) => {
                self.pending_retirement = Some(full.into_graph());
                self.handoff_status.record_retirement_retry();
            }
        }
        self.structural.publish_status_on_audio(self.handoff_status);
    }
}

#[derive(Clone, Copy)]
struct PatchNoteBits {
    patch_id: Option<PatchId>,
    low: u64,
    high: u64,
}

impl PatchNoteBits {
    const EMPTY: Self = Self {
        patch_id: None,
        low: 0,
        high: 0,
    };

    fn set(&mut self, note: u8) {
        if note < 64 {
            self.low |= 1_u64 << note;
        } else {
            self.high |= 1_u64 << (note - 64);
        }
    }

    fn clear(&mut self, note: u8) {
        if note < 64 {
            self.low &= !(1_u64 << note);
        } else {
            self.high &= !(1_u64 << (note - 64));
        }
    }

    fn clear_all(&mut self) {
        self.low = 0;
        self.high = 0;
    }

    fn count(self) -> u32 {
        self.low.count_ones().saturating_add(self.high.count_ones())
    }
}

struct ActiveNoteObservation {
    patches: [PatchNoteBits; MAX_PATCHES],
}

impl ActiveNoteObservation {
    const fn new() -> Self {
        Self {
            patches: [PatchNoteBits::EMPTY; MAX_PATCHES],
        }
    }

    fn observe_patch_message(
        &mut self,
        patch_id: PatchId,
        message: crate::kernel::midi_message::MidiMessage,
    ) {
        if message.kind() == MidiMessageKind::AllNotesOff {
            if let Some(patch) = self.patch_mut(patch_id, false) {
                patch.clear_all();
            }
            return;
        }

        let note = message.data1();
        match message.kind() {
            MidiMessageKind::NoteOn if message.data2() > 0 => {
                if let Some(patch) = self.patch_mut(patch_id, true) {
                    patch.set(note);
                }
            }
            MidiMessageKind::NoteOn | MidiMessageKind::NoteOff => {
                if let Some(patch) = self.patch_mut(patch_id, false) {
                    patch.clear(note);
                }
            }
            MidiMessageKind::ControlChange
            | MidiMessageKind::ProgramChange
            | MidiMessageKind::ChannelPressure
            | MidiMessageKind::PitchBend
            | MidiMessageKind::AllNotesOff => {}
        }
    }

    fn patch_mut(&mut self, patch_id: PatchId, create: bool) -> Option<&mut PatchNoteBits> {
        if let Some(index) = self
            .patches
            .iter()
            .position(|patch| patch.patch_id == Some(patch_id))
        {
            return self.patches.get_mut(index);
        }
        if !create {
            return None;
        }
        let slot = self
            .patches
            .iter_mut()
            .find(|patch| patch.patch_id.is_none())?;
        slot.patch_id = Some(patch_id);
        Some(slot)
    }

    fn clear_all(&mut self) {
        for patch in &mut self.patches {
            patch.clear_all();
        }
    }

    /// Clears only one Patch's observed notes with bounded work — the
    /// carry-over swap path uses this when exactly one engine restarted.
    fn clear_patch(&mut self, patch_id: PatchId) {
        if let Some(patch) = self.patch_mut(patch_id, false) {
            patch.clear_all();
        }
    }

    fn count(&self) -> u32 {
        self.patches
            .iter()
            .fold(0_u32, |count, patch| count.saturating_add(patch.count()))
    }

    fn count_patch(&self, patch_id: PatchId) -> u32 {
        self.patches
            .iter()
            .find(|patch| patch.patch_id == Some(patch_id))
            .map_or(0, |patch| patch.count())
    }
}

#[cfg(test)]
mod tests {
    use super::{ActiveNoteObservation, AudioRenderer};
    use crate::adapter::braids_capability::BRAIDS_CAPABILITY_ID;
    use crate::adapter::hidef_soundfont_capability::{
        HiDefSoundFontCapability, HIDEF_CAPABILITY_ID,
    };
    use crate::adapter::hidef_soundfont_preparer::HiDefSoundFontPreparer;
    use crate::adapter::production_instruments::{
        production_capability_registry, production_instrument_preparers,
        production_instrument_providers,
    };
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::mixer::mixer_state::MixerState;
    use crate::mixer::mixer_track_id::MixerTrackId;
    use crate::mixer::patch_output::PatchOutput;
    use crate::real_time::audio_boundary::AudioThreadBoundary;
    use crate::real_time::audio_command::AudioCommand;
    use crate::real_time::audio_observation::CallbackAudioObservation;
    use crate::real_time::audio_observation_snapshot::AudioObservationSnapshot;
    use crate::real_time::graph_handoff_status::GraphHandoffStatus;
    use crate::real_time::graph_revision::GraphRevision;
    use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
    use crate::real_time::prepared_graph::PreparedGraph;
    use crate::real_time::prepared_graph_builder::PreparedGraphBuilder;
    use crate::real_time::structural_graph_boundary::{
        AudioStructuralGraphBoundary, RetiredBoundaryFull,
    };
    use crate::real_time::MAX_PATCHES;
    use crate::synth::capability_id::CapabilityId;
    use crate::synth::instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
    use crate::synth::patch::Patch;
    use crate::synth::prepared_instrument::{PreparedInstrument, PreparedInstrumentError};
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::DescriptorDefaultConfigFactory;
    use crate::testing::automatic_midi_test::create_soundfont_config;
    use core::alloc::{GlobalAlloc, Layout};
    use core::cell::Cell;
    use std::alloc::System;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    thread_local! {
        static COUNT_MEMORY: Cell<bool> = const { Cell::new(false) };
        static ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
        static DEALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
    }

    struct TestAllocator;

    #[global_allocator]
    static TEST_ALLOCATOR: TestAllocator = TestAllocator;

    unsafe impl GlobalAlloc for TestAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record_allocation();
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record_allocation();
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            record_deallocation();
            unsafe { System.dealloc(pointer, layout) }
        }

        unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            record_allocation();
            unsafe { System.realloc(pointer, layout, new_size) }
        }
    }

    fn record_allocation() {
        let _ = COUNT_MEMORY.try_with(|enabled| {
            if enabled.get() {
                let _ = ALLOCATION_COUNT.try_with(|count| count.set(count.get() + 1));
            }
        });
    }

    fn record_deallocation() {
        let _ = COUNT_MEMORY.try_with(|enabled| {
            if enabled.get() {
                let _ = DEALLOCATION_COUNT.try_with(|count| count.set(count.get() + 1));
            }
        });
    }

    fn begin_memory_count() {
        ALLOCATION_COUNT.with(|count| count.set(0));
        DEALLOCATION_COUNT.with(|count| count.set(0));
        COUNT_MEMORY.with(|enabled| enabled.set(true));
    }

    fn finish_memory_count() -> (usize, usize) {
        COUNT_MEMORY.with(|enabled| enabled.set(false));
        (
            ALLOCATION_COUNT.with(Cell::get),
            DEALLOCATION_COUNT.with(Cell::get),
        )
    }

    struct TestBoundary {
        commands: [Option<AudioCommand>; 4],
        next_command: usize,
        latest: ParameterSnapshot,
        parameter_reads: usize,
    }

    impl AudioThreadBoundary for TestBoundary {
        fn pop_command(&mut self) -> Option<AudioCommand> {
            let command = self.commands.get_mut(self.next_command)?.take();
            if command.is_some() {
                self.next_command += 1;
            }
            command
        }

        fn read_latest_parameters(&mut self) -> ParameterSnapshot {
            self.parameter_reads += 1;
            self.latest
        }
    }

    struct TestStructural {
        prepared: [Option<PreparedGraph>; 2],
        prepared_index: usize,
        retired: [Option<PreparedGraph>; 2],
        retired_count: usize,
        retirement_blocked: bool,
        latest_status: GraphHandoffStatus,
    }

    impl TestStructural {
        fn new() -> Self {
            Self {
                prepared: std::array::from_fn(|_| None),
                prepared_index: 0,
                retired: std::array::from_fn(|_| None),
                retired_count: 0,
                retirement_blocked: false,
                latest_status: GraphHandoffStatus::default(),
            }
        }

        fn with_prepared(graphs: [Option<PreparedGraph>; 2]) -> Self {
            Self {
                prepared: graphs,
                prepared_index: 0,
                retired: std::array::from_fn(|_| None),
                retired_count: 0,
                retirement_blocked: false,
                latest_status: GraphHandoffStatus::default(),
            }
        }
    }

    impl AudioStructuralGraphBoundary for TestStructural {
        fn take_prepared_on_audio(&mut self) -> Option<PreparedGraph> {
            let graph = self.prepared.get_mut(self.prepared_index)?.take();
            if graph.is_some() {
                self.prepared_index += 1;
            }
            graph
        }

        fn return_retired_on_audio(
            &mut self,
            graph: PreparedGraph,
        ) -> Result<(), RetiredBoundaryFull> {
            if self.retirement_blocked || self.retired_count == self.retired.len() {
                return Err(RetiredBoundaryFull::new(graph));
            }
            self.retired[self.retired_count] = Some(graph);
            self.retired_count += 1;
            Ok(())
        }

        fn publish_status_on_audio(&mut self, status: GraphHandoffStatus) {
            self.latest_status = status;
        }
    }

    #[derive(Default)]
    struct TestObservation {
        latest: AudioObservationSnapshot,
        publications: usize,
    }

    impl CallbackAudioObservation for TestObservation {
        fn publish_from_callback(&mut self, snapshot: AudioObservationSnapshot) {
            self.latest = snapshot;
            self.publications += 1;
        }
    }

    struct FixturePreparer {
        capability_id: CapabilityId,
        first_dispatches: Arc<AtomicUsize>,
        second_dispatches: Arc<AtomicUsize>,
        drops: Arc<AtomicUsize>,
    }

    impl InstrumentPreparer for FixturePreparer {
        fn capability_id(&self) -> &CapabilityId {
            &self.capability_id
        }

        fn prepare(
            &self,
            patch: &Patch,
            _sample_rate: f32,
            _max_frames: usize,
        ) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
            if patch.id().value() % 2 == 1 {
                Ok(Box::new(FirstInstrument {
                    patch_id: patch.id(),
                    dispatches: Arc::clone(&self.first_dispatches),
                    drops: Arc::clone(&self.drops),
                }))
            } else {
                Ok(Box::new(SecondInstrument {
                    patch_id: patch.id(),
                    dispatches: Arc::clone(&self.second_dispatches),
                    drops: Arc::clone(&self.drops),
                }))
            }
        }
    }

    struct FirstInstrument {
        patch_id: PatchId,
        dispatches: Arc<AtomicUsize>,
        drops: Arc<AtomicUsize>,
    }

    impl PreparedInstrument for FirstInstrument {
        fn patch_id(&self) -> PatchId {
            self.patch_id
        }

        fn dispatch(
            &mut self,
            _message: MidiMessage,
            _parameters: &crate::real_time::RtPatchParameters,
        ) -> Result<(), PreparedInstrumentError> {
            self.dispatches.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn render(
            &mut self,
            output: &mut [f32],
            _frame_count: usize,
            _parameters: &crate::real_time::RtPatchParameters,
        ) {
            output.fill(0.25);
        }

        fn all_notes_off(&mut self) {
            self.dispatches.store(0, Ordering::Relaxed);
        }
    }

    impl Drop for FirstInstrument {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    struct SecondInstrument {
        patch_id: PatchId,
        dispatches: Arc<AtomicUsize>,
        drops: Arc<AtomicUsize>,
    }

    impl PreparedInstrument for SecondInstrument {
        fn patch_id(&self) -> PatchId {
            self.patch_id
        }

        fn dispatch(
            &mut self,
            _message: MidiMessage,
            _parameters: &crate::real_time::RtPatchParameters,
        ) -> Result<(), PreparedInstrumentError> {
            self.dispatches.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn render(
            &mut self,
            output: &mut [f32],
            _frame_count: usize,
            _parameters: &crate::real_time::RtPatchParameters,
        ) {
            output.fill(0.5);
        }

        fn all_notes_off(&mut self) {
            self.dispatches.store(0, Ordering::Relaxed);
        }
    }

    impl Drop for SecondInstrument {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    struct Fixture {
        provider: HiDefSoundFontCapability,
        patches: [Patch; 2],
        first_dispatches: Arc<AtomicUsize>,
        second_dispatches: Arc<AtomicUsize>,
        drops: Arc<AtomicUsize>,
    }

    impl Fixture {
        fn new() -> Self {
            let provider =
                crate::adapter::production_instruments::production_soundfont_capability().unwrap();
            let patch = |id: u32| {
                Patch::new(
                    PatchId::new(id).unwrap(),
                    format!("Patch {id}"),
                    create_soundfont_config(
                        &provider,
                        SoundFontInstrument::new(0, (id - 1) as u8, false).unwrap(),
                    )
                    .unwrap(),
                    MidiChannel::new((id - 1) as u8).unwrap(),
                    PatchOutput::to_track(MixerTrackId::new((id - 1) as u8).unwrap()),
                )
            };
            Self {
                patches: [patch(1), patch(2)],
                provider,
                first_dispatches: Arc::new(AtomicUsize::new(0)),
                second_dispatches: Arc::new(AtomicUsize::new(0)),
                drops: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn parameters(&self, revision: u64, generation: u64) -> ParameterSnapshot {
            ParameterSnapshot::for_graph(
                generation,
                GraphRevision::new(revision).unwrap(),
                GlobalParameters::new(0.0).unwrap(),
                MixerState::default(),
                &self
                    .patches
                    .each_ref()
                    .map(|patch| RtPatchParameters::new(patch.id(), patch.output())),
            )
            .unwrap()
        }

        fn graph(&self, revision: u64, generation: u64) -> PreparedGraph {
            let registry = self.provider.registry().unwrap();
            let preparers: Vec<Box<dyn InstrumentPreparer>> = vec![Box::new(FixturePreparer {
                capability_id: CapabilityId::new(HIDEF_CAPABILITY_ID).unwrap(),
                first_dispatches: Arc::clone(&self.first_dispatches),
                second_dispatches: Arc::clone(&self.second_dispatches),
                drops: Arc::clone(&self.drops),
            })];
            PreparedGraphBuilder::new(&registry, &preparers)
                .build(
                    GraphRevision::new(revision).unwrap(),
                    &self.patches,
                    self.parameters(revision, generation),
                    48_000.0,
                    4,
                )
                .unwrap()
        }

        fn boundary(&self, latest: ParameterSnapshot, commands: &[AudioCommand]) -> TestBoundary {
            let mut storage = [None; 4];
            for (slot, command) in storage.iter_mut().zip(commands) {
                *slot = Some(*command);
            }
            TestBoundary {
                commands: storage,
                next_command: 0,
                latest,
                parameter_reads: 0,
            }
        }
    }

    fn midi(kind: MidiMessageKind) -> MidiMessage {
        MidiMessage::try_new(MidiChannel::new(1).unwrap(), kind, 60, 100).unwrap()
    }

    #[test]
    fn renderer_routes_heterogeneous_patches_and_publishes_finite_mix_observation() {
        let fixture = Fixture::new();
        let boundary = fixture.boundary(
            fixture.parameters(1, 9),
            &[
                AudioCommand::patch_midi(fixture.patches[1].id(), midi(MidiMessageKind::NoteOn)),
                AudioCommand::patch_midi(PatchId::new(99).unwrap(), midi(MidiMessageKind::NoteOn)),
            ],
        );
        let structural = TestStructural::new();
        let mut renderer = AudioRenderer::with_observation(
            boundary,
            structural,
            fixture.graph(1, 1),
            TestObservation::default(),
        );
        let mut output = [0.0; 8];

        renderer.render(&mut output);

        assert_eq!(fixture.first_dispatches.load(Ordering::Relaxed), 0);
        assert_eq!(fixture.second_dispatches.load(Ordering::Relaxed), 1);
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output
            .iter()
            .all(|sample| (*sample - 0.75).abs() < 0.000_001));
        assert_eq!(renderer.observation.publications, 1);
        assert_eq!(renderer.observation.latest.parameter_generation(), 9);
        assert_eq!(renderer.observation.latest.commands_consumed(), 2);
        assert_eq!(renderer.observation.latest.active_notes(), 1);
    }

    #[test]
    fn graph_swaps_apply_once_at_block_start_and_return_the_old_owner() {
        let fixture = Fixture::new();
        let boundary = fixture.boundary(fixture.parameters(2, 20), &[]);
        let structural =
            TestStructural::with_prepared([Some(fixture.graph(2, 20)), Some(fixture.graph(3, 30))]);
        let mut renderer = AudioRenderer::new(boundary, structural, fixture.graph(1, 10));
        let mut output = [0.0; 8];

        renderer.render(&mut output);

        assert_eq!(renderer.active_revision(), GraphRevision::new(2).unwrap());
        assert_eq!(renderer.structural.prepared_index, 1);
        assert_eq!(renderer.structural.retired_count, 1);
        assert_eq!(
            renderer.structural.retired[0].as_ref().unwrap().revision(),
            GraphRevision::new(1).unwrap()
        );
        assert_eq!(renderer.handoff_status().swaps_applied(), 1);
        assert_eq!(
            renderer.handoff_status().retired_revision(),
            Some(GraphRevision::new(1).unwrap())
        );
    }

    #[test]
    fn retirement_pressure_retains_and_retries_without_taking_another_graph() {
        let fixture = Fixture::new();
        let boundary = fixture.boundary(fixture.parameters(2, 20), &[]);
        let mut structural =
            TestStructural::with_prepared([Some(fixture.graph(2, 20)), Some(fixture.graph(3, 30))]);
        structural.retirement_blocked = true;
        let mut renderer = AudioRenderer::new(boundary, structural, fixture.graph(1, 10));
        let mut output = [0.0; 8];

        begin_memory_count();
        renderer.render(&mut output);
        renderer.render(&mut output);
        let (allocations, deallocations) = finish_memory_count();

        assert_eq!(allocations, 0);
        assert_eq!(deallocations, 0);
        assert_eq!(fixture.drops.load(Ordering::Relaxed), 0);
        assert_eq!(renderer.active_revision(), GraphRevision::new(2).unwrap());
        assert_eq!(
            renderer.pending_retirement_revision(),
            Some(GraphRevision::new(1).unwrap())
        );
        assert_eq!(renderer.structural.prepared_index, 1);
        assert_eq!(renderer.handoff_status().swaps_applied(), 1);
        assert_eq!(renderer.handoff_status().retirement_retries(), 2);

        renderer.structural.retirement_blocked = false;
        renderer.render(&mut output);
        assert_eq!(renderer.active_revision(), GraphRevision::new(3).unwrap());
        assert_eq!(renderer.structural.prepared_index, 2);
        assert_eq!(renderer.structural.retired_count, 2);
    }

    #[test]
    fn incompatible_scalar_races_keep_each_graphs_last_compatible_snapshot() {
        let fixture = Fixture::new();
        let boundary = fixture.boundary(fixture.parameters(2, 99), &[]);
        let structural = TestStructural::new();
        let mut renderer = AudioRenderer::with_observation(
            boundary,
            structural,
            fixture.graph(1, 10),
            TestObservation::default(),
        );
        let mut output = [0.0; 8];

        renderer.render(&mut output);
        assert_eq!(renderer.parameters().generation(), 10);
        assert_eq!(renderer.observation.latest.parameter_generation(), 10);
        assert_eq!(renderer.handoff_status().incompatible_snapshots(), 1);

        renderer.structural.prepared[0] = Some(fixture.graph(2, 20));
        renderer.render(&mut output);
        assert_eq!(renderer.active_revision(), GraphRevision::new(2).unwrap());
        assert_eq!(renderer.parameters().generation(), 99);
        assert_eq!(renderer.observation.latest.parameter_generation(), 99);

        renderer.boundary.latest = fixture.parameters(1, 100);
        renderer.render(&mut output);
        assert_eq!(renderer.parameters().generation(), 99);
        assert_eq!(renderer.handoff_status().incompatible_snapshots(), 2);
    }

    #[test]
    fn production_layout_changing_swap_is_bounded_finite_and_allocation_free() {
        let registry = production_capability_registry().unwrap();
        let factory = DescriptorDefaultConfigFactory::new(
            registry.clone(),
            production_instrument_providers().unwrap(),
        );
        let patch_id = PatchId::new(1).unwrap();
        let soundfont = factory
            .create(&CapabilityId::new(HIDEF_CAPABILITY_ID).unwrap())
            .unwrap();
        let mut patch = Patch::new(
            patch_id,
            "Layout-changing Patch".to_owned(),
            soundfont,
            MidiChannel::new(0).unwrap(),
            PatchOutput::default(),
        );
        let global = GlobalParameters::new(0.0).unwrap();
        let preparers = production_instrument_preparers().unwrap();
        let builder = PreparedGraphBuilder::new(&registry, &preparers);
        let source_parameters = ParameterSnapshot::project_patches(
            1,
            GraphRevision::INITIAL,
            global,
            MixerState::default(),
            std::slice::from_ref(&patch),
            &registry,
        )
        .unwrap();
        let source = builder
            .build(
                GraphRevision::INITIAL,
                std::slice::from_ref(&patch),
                source_parameters,
                48_000.0,
                512,
            )
            .unwrap();
        assert_eq!(source.engine_rack().scalar_count(0), Some(0));

        patch.set_instrument_config(
            factory
                .create(&CapabilityId::new(BRAIDS_CAPABILITY_ID).unwrap())
                .unwrap(),
        );
        let target_revision = GraphRevision::new(2).unwrap();
        let target_parameters = ParameterSnapshot::project_patches(
            2,
            target_revision,
            global,
            MixerState::default(),
            std::slice::from_ref(&patch),
            &registry,
        )
        .unwrap();
        let target = builder
            .build(
                target_revision,
                std::slice::from_ref(&patch),
                target_parameters,
                48_000.0,
                512,
            )
            .unwrap();
        assert_eq!(target.engine_rack().scalar_count(0), Some(3));

        let message =
            MidiMessage::try_new(patch.channel(), MidiMessageKind::NoteOn, 60, 100).unwrap();
        let boundary = TestBoundary {
            commands: [
                Some(AudioCommand::patch_midi(patch_id, message)),
                None,
                None,
                None,
            ],
            next_command: 0,
            latest: target_parameters,
            parameter_reads: 0,
        };
        let structural = TestStructural::with_prepared([Some(target), None]);
        let mut renderer = AudioRenderer::new(boundary, structural, source);
        let mut output = [0.0; 1_024];

        begin_memory_count();
        renderer.render(&mut output);
        let (allocations, deallocations) = finish_memory_count();

        assert_eq!(allocations, 0);
        assert_eq!(deallocations, 0);
        assert_eq!(renderer.active_revision(), target_revision);
        assert_eq!(renderer.active_graph.engine_rack().scalar_count(0), Some(3));
        assert_eq!(renderer.structural.retired_count, 1);
        assert_eq!(renderer.handoff_status().swaps_applied(), 1);
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().any(|sample| sample.abs() > f32::EPSILON));
    }

    #[test]
    fn normal_renderer_callback_allocates_deallocates_and_destroys_nothing() {
        let fixture = Fixture::new();
        let boundary = fixture.boundary(
            fixture.parameters(1, 2),
            &[AudioCommand::patch_midi(
                fixture.patches[0].id(),
                midi(MidiMessageKind::NoteOn),
            )],
        );
        let structural = TestStructural::new();
        let mut renderer = AudioRenderer::new(boundary, structural, fixture.graph(1, 1));
        let mut output = [0.0; 8];

        begin_memory_count();
        renderer.render(&mut output);
        let (allocations, deallocations) = finish_memory_count();

        assert_eq!(allocations, 0);
        assert_eq!(deallocations, 0);
        assert_eq!(fixture.drops.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn hidef_soundfont_preparer_callback_operations_allocate_and_deallocate_nothing() {
        let provider =
            crate::adapter::production_instruments::production_soundfont_capability().unwrap();
        let patch = Patch::new(
            PatchId::new(1).unwrap(),
            "Prepared piano".to_owned(),
            create_soundfont_config(&provider, SoundFontInstrument::new(0, 0, false).unwrap())
                .unwrap(),
            MidiChannel::new(0).unwrap(),
            PatchOutput::default(),
        );
        let preparer = HiDefSoundFontPreparer::new(
            crate::adapter::production_instruments::production_soundfont_asset().unwrap(),
        )
        .unwrap();
        let mut instrument = preparer.prepare(&patch, 48_000.0, 512).unwrap();
        let message =
            MidiMessage::try_new(patch.channel(), MidiMessageKind::NoteOn, 60, 100).unwrap();
        let mut output = [0.0; 1_024];
        let parameters = RtPatchParameters::new(patch.id(), patch.output());

        begin_memory_count();
        let dispatch = instrument.dispatch(message, &parameters);
        instrument.render(&mut output, 512, &parameters);
        instrument.all_notes_off();
        let (allocations, deallocations) = finish_memory_count();

        assert_eq!(dispatch, Ok(()));
        assert_eq!(allocations, 0);
        assert_eq!(deallocations, 0);
        assert!(output.iter().all(|sample| sample.is_finite()));
    }

    // ---- WP10: voice carry-over across topology activation ----------------

    /// Preparer whose instruments render a distinct serial-numbered level, so
    /// a test can observe which prepared instance (live or fresh) is sounding
    /// after an activation without any engine-identity branch.
    struct SerialPreparer {
        capability_id: CapabilityId,
        serials: Arc<AtomicUsize>,
        drops: Arc<AtomicUsize>,
    }

    impl InstrumentPreparer for SerialPreparer {
        fn capability_id(&self) -> &CapabilityId {
            &self.capability_id
        }

        fn prepare(
            &self,
            patch: &Patch,
            _sample_rate: f32,
            _max_frames: usize,
        ) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
            let serial = self.serials.fetch_add(1, Ordering::Relaxed) + 1;
            Ok(Box::new(SerialInstrument {
                patch_id: patch.id(),
                level: serial as f32 * 0.125,
                drops: Arc::clone(&self.drops),
            }))
        }
    }

    struct SerialInstrument {
        patch_id: PatchId,
        level: f32,
        drops: Arc<AtomicUsize>,
    }

    impl PreparedInstrument for SerialInstrument {
        fn patch_id(&self) -> PatchId {
            self.patch_id
        }

        fn dispatch(
            &mut self,
            _message: MidiMessage,
            _parameters: &crate::real_time::RtPatchParameters,
        ) -> Result<(), PreparedInstrumentError> {
            Ok(())
        }

        fn render(
            &mut self,
            output: &mut [f32],
            _frame_count: usize,
            _parameters: &crate::real_time::RtPatchParameters,
        ) {
            output.fill(self.level);
        }

        fn all_notes_off(&mut self) {}
    }

    impl Drop for SerialInstrument {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    struct CarryOverFixture {
        patch: Patch,
        provider: HiDefSoundFontCapability,
        serials: Arc<AtomicUsize>,
        drops: Arc<AtomicUsize>,
    }

    impl CarryOverFixture {
        fn new() -> Self {
            let provider =
                crate::adapter::production_instruments::production_soundfont_capability().unwrap();
            let patch = Patch::new(
                PatchId::new(1).unwrap(),
                "Carry-over Patch".to_owned(),
                create_soundfont_config(&provider, SoundFontInstrument::new(0, 0, false).unwrap())
                    .unwrap(),
                MidiChannel::new(0).unwrap(),
                PatchOutput::to_track(MixerTrackId::new(0).unwrap()),
            );
            Self {
                patch,
                provider,
                serials: Arc::new(AtomicUsize::new(0)),
                drops: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn parameters(&self, revision: u64, generation: u64) -> ParameterSnapshot {
            ParameterSnapshot::for_graph(
                generation,
                GraphRevision::new(revision).unwrap(),
                GlobalParameters::new(0.0).unwrap(),
                MixerState::default(),
                &[RtPatchParameters::new(self.patch.id(), self.patch.output())],
            )
            .unwrap()
        }

        /// Builds one graph whose single instrument sounds the next serial
        /// level, so instance movement is audible.
        fn graph(&self, revision: u64, generation: u64) -> PreparedGraph {
            let registry = self.provider.registry().unwrap();
            let preparers: Vec<Box<dyn InstrumentPreparer>> = vec![Box::new(SerialPreparer {
                capability_id: CapabilityId::new(HIDEF_CAPABILITY_ID).unwrap(),
                serials: Arc::clone(&self.serials),
                drops: Arc::clone(&self.drops),
            })];
            PreparedGraphBuilder::new(&registry, &preparers)
                .build(
                    GraphRevision::new(revision).unwrap(),
                    std::slice::from_ref(&self.patch),
                    self.parameters(revision, generation),
                    48_000.0,
                    4,
                )
                .unwrap()
        }
    }

    /// WP10/T057: an occupancy-scoped activation exchanges the live prepared
    /// instrument into the replacement — the sounding instance survives, the
    /// observed notes survive, and the callback allocates, deallocates, and
    /// destroys nothing.
    #[test]
    fn occupancy_scoped_activation_carries_the_live_instrument_and_held_notes() {
        let fixture = CarryOverFixture::new();
        let source = fixture.graph(1, 10);
        let mut target = fixture.graph(2, 20);
        target.set_carry_over_scope(crate::real_time::GraphReplacementScope::PatchSlot {
            patch_id: fixture.patch.id(),
            slot: crate::synth::effect_slot_id::EffectSlotIndex::ALL[0],
        });

        let boundary = TestBoundary {
            commands: [
                Some(AudioCommand::patch_midi(
                    fixture.patch.id(),
                    midi(MidiMessageKind::NoteOn),
                )),
                None,
                None,
                None,
            ],
            next_command: 0,
            latest: fixture.parameters(2, 20),
            parameter_reads: 0,
        };
        let structural = TestStructural::new();
        let mut renderer = AudioRenderer::with_observation(
            boundary,
            structural,
            source,
            TestObservation::default(),
        );
        let mut output = [0.0_f32; 8];

        // The live instrument (serial 1) sounds before activation.
        renderer.render(&mut output);
        let live_block = output;
        assert_eq!(renderer.observation.latest.active_notes(), 1);

        renderer.structural.prepared[0] = Some(target);
        begin_memory_count();
        renderer.render(&mut output);
        let (allocations, deallocations) = finish_memory_count();

        assert_eq!(allocations, 0, "carry-over activation must not allocate");
        assert_eq!(
            deallocations, 0,
            "carry-over activation must not deallocate"
        );
        assert_eq!(
            fixture.drops.load(Ordering::Relaxed),
            0,
            "no prepared instrument may be destroyed by the callback"
        );
        assert_eq!(renderer.active_revision(), GraphRevision::new(2).unwrap());
        assert_eq!(
            output[..],
            live_block[..],
            "the live instance carried across the swap — no restart, no fresh instance"
        );
        assert_eq!(
            renderer.observation.latest.active_notes(),
            1,
            "held notes survive an occupancy-scoped activation"
        );
        // The superseded graph carries the fresh never-sounded instance and
        // retires off-callback intact.
        assert_eq!(renderer.structural.retired_count, 1);
    }

    /// WP10 negative space: a `SelectedEngine` scope restarts exactly the
    /// selected Patch — the fresh instance sounds and its observed notes
    /// clear — and an unscoped replacement keeps the full-reset semantics.
    #[test]
    fn selected_engine_scope_restarts_only_the_selected_patch() {
        let fixture = CarryOverFixture::new();
        let source = fixture.graph(1, 10);
        let mut target = fixture.graph(2, 20);
        target.set_carry_over_scope(crate::real_time::GraphReplacementScope::SelectedEngine(
            fixture.patch.id(),
        ));

        let boundary = TestBoundary {
            commands: [
                Some(AudioCommand::patch_midi(
                    fixture.patch.id(),
                    midi(MidiMessageKind::NoteOn),
                )),
                None,
                None,
                None,
            ],
            next_command: 0,
            latest: fixture.parameters(2, 20),
            parameter_reads: 0,
        };
        let structural = TestStructural::new();
        let mut renderer = AudioRenderer::with_observation(
            boundary,
            structural,
            source,
            TestObservation::default(),
        );
        let mut output = [0.0_f32; 8];

        renderer.render(&mut output);
        let live_block = output;
        assert_eq!(renderer.observation.latest.active_notes(), 1);

        renderer.structural.prepared[0] = Some(target);
        renderer.render(&mut output);

        assert_eq!(renderer.active_revision(), GraphRevision::new(2).unwrap());
        assert_ne!(
            output[..],
            live_block[..],
            "the selected Patch's engine restarted with its fresh instance"
        );
        assert_eq!(
            renderer.observation.latest.active_notes(),
            0,
            "the restarted Patch's observed notes clear"
        );
    }

    #[test]
    fn active_note_observation_tracks_patch_lifecycle_with_bounded_bits() {
        let mut notes = ActiveNoteObservation::new();
        let patch = PatchId::new(3).unwrap();

        notes.observe_patch_message(patch, midi(MidiMessageKind::NoteOn));
        assert_eq!(notes.count(), 1);
        notes.observe_patch_message(patch, midi(MidiMessageKind::NoteOff));
        assert_eq!(notes.count(), 0);
        notes.observe_patch_message(patch, midi(MidiMessageKind::NoteOn));
        notes.observe_patch_message(patch, midi(MidiMessageKind::AllNotesOff));
        assert_eq!(notes.count(), 0);
    }

    #[test]
    fn prepared_graph_exposes_fixed_patch_stem_storage() {
        let fixture = Fixture::new();
        let graph = fixture.graph(1, 1);
        let stems: &[crate::real_time::patch_audio_block::PatchStereoStem] =
            graph.patch_audio().stems();

        assert!(stems.is_empty());
        assert_eq!(graph.patch_audio().storage().len(), MAX_PATCHES);
    }

    // ---- T031: full-occupancy measurements (C-RT-1/3/7, NFR-001/002/003) --

    mod full_occupancy {
        use super::{
            begin_memory_count, finish_memory_count, midi, AudioCommand, AudioRenderer,
            MidiMessageKind, TestBoundary, TestStructural,
        };
        use crate::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
        use crate::adapter::production_effects::{
            production_effect_preparers, production_effect_registry,
        };
        use crate::kernel::midi_channel::MidiChannel;
        use crate::kernel::midi_message::MidiMessage;
        use crate::kernel::patch_id::PatchId;
        use crate::mixer::bus_id::{BusId, MAX_BUS_RETURNS};
        use crate::mixer::global_parameters::GlobalParameters;
        use crate::mixer::mixer_state::MixerState;
        use crate::mixer::mixer_track_id::MixerTrackId;
        use crate::mixer::mixer_track_parameters::MixerTrackParameters;
        use crate::mixer::patch_output::PatchOutput;
        use crate::real_time::audio_boundary::{
            AudioBoundary, AudioThreadBoundary, ControlAudioBoundary,
        };
        use crate::real_time::graph_revision::GraphRevision;
        use crate::real_time::parameter_snapshot::{ParameterSnapshot, MAX_PATCHES};
        use crate::real_time::prepared_graph::PreparedGraph;
        use crate::real_time::prepared_graph_builder::PreparedGraphBuilder;
        use crate::synth::capability_id::CapabilityId;
        use crate::synth::effect_slot_id::MAX_EFFECT_SLOTS;
        use crate::synth::instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
        use crate::synth::patch::Patch;
        use crate::synth::prepared_instrument::{PreparedInstrument, PreparedInstrumentError};
        use crate::synth::sound_font_instrument::SoundFontInstrument;
        use crate::synth::{
            EffectCapabilityRegistry, EffectPreparer, EffectSlotId, PostEffectConfig,
        };
        use crate::testing::automatic_midi_test::create_soundfont_config;
        use std::hint::black_box;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::time::Instant;

        const SAMPLE_RATE: f32 = 48_000.0;
        const MAX_FRAMES: usize = 64;

        struct CountingPreparer {
            capability_id: CapabilityId,
            drops: Arc<AtomicUsize>,
        }

        impl InstrumentPreparer for CountingPreparer {
            fn capability_id(&self) -> &CapabilityId {
                &self.capability_id
            }

            fn prepare(
                &self,
                patch: &Patch,
                _sample_rate: f32,
                _max_frames: usize,
            ) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
                Ok(Box::new(CountingInstrument {
                    patch_id: patch.id(),
                    drops: Arc::clone(&self.drops),
                }))
            }
        }

        struct CountingInstrument {
            patch_id: PatchId,
            drops: Arc<AtomicUsize>,
        }

        impl PreparedInstrument for CountingInstrument {
            fn patch_id(&self) -> PatchId {
                self.patch_id
            }

            fn dispatch(
                &mut self,
                _message: MidiMessage,
                _parameters: &crate::real_time::RtPatchParameters,
            ) -> Result<(), PreparedInstrumentError> {
                Ok(())
            }

            fn render(
                &mut self,
                output: &mut [f32],
                _frame_count: usize,
                _parameters: &crate::real_time::RtPatchParameters,
            ) {
                output.fill(0.05);
            }

            fn all_notes_off(&mut self) {}
        }

        impl Drop for CountingInstrument {
            fn drop(&mut self) {
                self.drops.fetch_add(1, Ordering::Relaxed);
            }
        }

        /// The complete fully occupied configuration this mission must carry:
        /// 16 Patches x 3 occupied effect positions, 16 tracks x 8 non-zero
        /// sends, and all 8 bus returns occupied.
        struct FullOccupancyFixture {
            provider: crate::adapter::hidef_soundfont_capability::HiDefSoundFontCapability,
            patches: Vec<Patch>,
            effect_registry: EffectCapabilityRegistry,
            effect_preparers: Vec<Box<dyn EffectPreparer>>,
            drops: Arc<AtomicUsize>,
        }

        impl FullOccupancyFixture {
            fn new() -> Self {
                let provider =
                    crate::adapter::production_instruments::production_soundfont_capability()
                        .unwrap();
                let effect_registry = production_effect_registry().unwrap();
                let capabilities = ["effect.chorus", "effect.reverb", "effect.delay"];
                let patches = (1..=MAX_PATCHES as u32)
                    .map(|id| {
                        let effects: Vec<PostEffectConfig> = (0..MAX_EFFECT_SLOTS)
                            .map(|position| {
                                let slot = EffectSlotId::new(
                                    ((id - 1) as u16) * MAX_EFFECT_SLOTS as u16
                                        + position as u16
                                        + 1,
                                )
                                .unwrap();
                                effect_registry
                                    .descriptors()
                                    .iter()
                                    .find(|descriptor| {
                                        descriptor.id().as_str() == capabilities[position]
                                    })
                                    .unwrap()
                                    .default_config(slot)
                                    .unwrap()
                            })
                            .collect();
                        Patch::new(
                            PatchId::new(id).unwrap(),
                            format!("Full {id}"),
                            create_soundfont_config(
                                &provider,
                                SoundFontInstrument::new(0, (id - 1) as u8, false).unwrap(),
                            )
                            .unwrap(),
                            MidiChannel::new((id - 1) as u8).unwrap(),
                            PatchOutput::to_track(MixerTrackId::new((id - 1) as u8).unwrap()),
                        )
                        .with_post_effects(effects)
                    })
                    .collect();
                Self {
                    provider,
                    patches,
                    effect_registry,
                    effect_preparers: production_effect_preparers().unwrap(),
                    drops: Arc::new(AtomicUsize::new(0)),
                }
            }

            fn mixer_state(&self) -> MixerState {
                let mut mixer = MixerState::default();
                for track_id in MixerTrackId::ALL {
                    let mut sends = [0.0; MAX_BUS_RETURNS];
                    for (index, send) in sends.iter_mut().enumerate() {
                        *send = 0.1 + index as f32 * 0.05;
                    }
                    mixer.set_track(
                        track_id,
                        MixerTrackParameters::from_values(-6.0, 0.0, false, false, sends).unwrap(),
                    );
                }
                mixer
            }

            fn global() -> GlobalParameters {
                GlobalParameters::new(0.0).unwrap()
            }

            /// The canonical fully occupied bank: the production defaults on
            /// buses 0 and 1 plus a chorus occupant on every remaining bus.
            fn full_bank(&self) -> crate::mixer::bus_return::BusReturnBank {
                let chorus = crate::synth::EffectCapabilityId::new("effect.chorus").unwrap();
                let mut bank = crate::adapter::production_effects::production_default_bus_returns(
                    &self.effect_registry,
                )
                .unwrap();
                for bus_value in 2..MAX_BUS_RETURNS as u8 {
                    let bus = BusId::new(bus_value).unwrap();
                    bank.set_return_occupancy(&self.effect_registry, bus, Some(&chorus))
                        .unwrap();
                    bank.set_return_level(bus, 0.4).unwrap();
                }
                bank
            }

            /// Projects the complete fully occupied snapshot: every effect
            /// position live, every send non-zero, all eight returns active.
            fn full_snapshot(&self, revision: u64, generation: u64) -> ParameterSnapshot {
                let registry = self.provider.registry().unwrap();
                ParameterSnapshot::project_patches_with_effects_and_returns(
                    generation,
                    GraphRevision::new(revision).unwrap(),
                    Self::global(),
                    self.mixer_state(),
                    &self.patches,
                    &registry,
                    &self.effect_registry,
                    &self.full_bank(),
                )
                .unwrap()
            }

            /// Builds one callback-ready graph whose racks occupy the complete
            /// widened capacity, with its initial parameters refreshed to the
            /// matching fully occupied snapshot.
            fn full_graph(
                &self,
                revision: u64,
                generation: u64,
            ) -> (PreparedGraph, ParameterSnapshot) {
                let registry = self.provider.registry().unwrap();
                let preparers: Vec<Box<dyn InstrumentPreparer>> =
                    vec![Box::new(CountingPreparer {
                        capability_id: CapabilityId::new(
                            crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID,
                        )
                        .unwrap(),
                        drops: Arc::clone(&self.drops),
                    })];
                let snapshot = self.full_snapshot(revision, generation);
                let bank = self.full_bank();
                let graph = PreparedGraphBuilder::new(&registry, &preparers)
                    .with_effects(&self.effect_registry, &self.effect_preparers)
                    .with_returns(&bank)
                    .build(
                        GraphRevision::new(revision).unwrap(),
                        &self.patches,
                        snapshot,
                        SAMPLE_RATE,
                        MAX_FRAMES,
                    )
                    .unwrap();
                (graph, snapshot)
            }
        }

        fn assert_fully_occupied(snapshot: &ParameterSnapshot) {
            assert_eq!(snapshot.patch_count(), MAX_PATCHES);
            assert!(snapshot
                .patches()
                .iter()
                .all(|patch| patch.effects().iter().all(|effect| effect.is_active())));
            assert!(snapshot
                .mixer_tracks()
                .iter()
                .all(|track| track.sends().iter().all(|send| *send > 0.0)));
            assert!(snapshot.returns().iter().all(|entry| entry.is_active()));
        }

        /// T031/C-RT-3/NFR-002: a steady-state block and a topology-activation
        /// block at the complete widened occupancy perform zero allocation,
        /// zero deallocation, and zero destruction; the superseded graph is
        /// returned off-callback intact; activation is atomic at block start.
        #[test]
        fn fully_occupied_render_and_activation_stay_allocation_and_destruction_free() {
            let fixture = FullOccupancyFixture::new();
            let (source, source_snapshot) = fixture.full_graph(1, 10);
            let (target, target_snapshot) = fixture.full_graph(2, 20);
            assert_fully_occupied(&source_snapshot);
            assert_fully_occupied(&target_snapshot);
            assert_eq!(source.effect_rack().patch_count(), MAX_PATCHES);
            for index in 0..MAX_PATCHES {
                assert_eq!(
                    source.effect_rack().occupied_slot_count(index),
                    MAX_EFFECT_SLOTS
                );
            }
            assert_eq!(source.bus_return_rack().occupied_count(), MAX_BUS_RETURNS);

            let patch_id = fixture.patches[0].id();
            let boundary = TestBoundary {
                commands: [
                    Some(AudioCommand::patch_midi(
                        patch_id,
                        midi(MidiMessageKind::NoteOn),
                    )),
                    None,
                    None,
                    None,
                ],
                next_command: 0,
                latest: target_snapshot,
                parameter_reads: 0,
            };
            let structural = TestStructural::new();
            let mut renderer = AudioRenderer::new(boundary, structural, source);
            let mut output = [0.0_f32; MAX_FRAMES * 2];

            // Steady state at full occupancy: enough blocks for the wet-only
            // delay stage (250 ms) to reach the output, all measured.
            begin_memory_count();
            let mut any_audible = false;
            for _ in 0..256 {
                renderer.render(&mut output);
                assert!(output.iter().all(|sample| sample.is_finite()));
                any_audible |= output.iter().any(|sample| sample.abs() > 0.0);
            }
            let (allocations, deallocations) = finish_memory_count();
            assert_eq!(allocations, 0, "steady-state blocks must not allocate");
            assert_eq!(deallocations, 0, "steady-state blocks must not deallocate");
            assert!(any_audible, "the fully occupied graph must produce audio");
            assert_eq!(renderer.parameters().generation(), 10);

            // Topology activation at full occupancy, exactly at block start.
            renderer.structural.prepared[0] = Some(target);
            begin_memory_count();
            renderer.render(&mut output);
            let (allocations, deallocations) = finish_memory_count();
            assert_eq!(allocations, 0, "activation block must not allocate");
            assert_eq!(deallocations, 0, "activation block must not deallocate");
            assert_eq!(
                fixture.drops.load(Ordering::Relaxed),
                0,
                "no prepared instrument may be destroyed by the callback"
            );
            assert_eq!(renderer.active_revision(), GraphRevision::new(2).unwrap());
            // The rendered block observes the replacement's complete snapshot:
            // no partially applied topology is observable (NFR-003, C-RT-9).
            assert_eq!(renderer.parameters().generation(), 20);
            assert!(renderer
                .parameters()
                .audio_values_equal(&fixture.full_snapshot(2, 20)));
            assert_eq!(renderer.structural.retired_count, 1);
            assert_eq!(
                renderer.structural.retired[0].as_ref().unwrap().revision(),
                GraphRevision::new(1).unwrap()
            );
            // The replacement graph starts with empty wet stages, so the
            // activation block is finite but may legitimately be silent.
            assert!(output.iter().all(|sample| sample.is_finite()));
        }

        /// T031/C-RT-7: the publish cost of the widened snapshot through the
        /// production triple-buffer transport is measured — not assumed — at
        /// the complete fully occupied configuration, and the publish path is
        /// allocation- and destruction-free.
        #[test]
        fn snapshot_publish_cost_is_measured_and_bounded_at_full_occupancy() {
            let fixture = FullOccupancyFixture::new();
            let snapshot = fixture.full_snapshot(1, 1);
            assert_fully_occupied(&snapshot);

            let size = core::mem::size_of::<ParameterSnapshot>();
            assert!(!core::mem::needs_drop::<ParameterSnapshot>());

            let boundary = LockFreeAudioBoundary::new(4, snapshot);
            let (mut control, mut audio) = boundary.into_handles();
            const ITERATIONS: u32 = 10_000;

            begin_memory_count();
            let publish_start = Instant::now();
            for _ in 0..ITERATIONS {
                control.publish_parameters(snapshot);
            }
            let publish_nanos = publish_start.elapsed().as_nanos() / u128::from(ITERATIONS);
            let read_start = Instant::now();
            let mut checksum = 0_u64;
            for _ in 0..ITERATIONS {
                checksum =
                    checksum.wrapping_add(black_box(audio.read_latest_parameters()).generation());
            }
            let read_nanos = read_start.elapsed().as_nanos() / u128::from(ITERATIONS);
            let (allocations, deallocations) = finish_memory_count();

            assert_eq!(checksum, u64::from(ITERATIONS));
            assert_eq!(allocations, 0, "publishing must not allocate");
            assert_eq!(
                deallocations, 0,
                "publishing must not deallocate or destroy"
            );

            // The measured record (C-RT-7): visible with `--nocapture`, and
            // bounded by generous ceilings that fail loudly on regression.
            eprintln!(
                "C-RT-7 publish cost at full occupancy: snapshot = {size} bytes, \
                 publish mean = {publish_nanos} ns, read mean = {read_nanos} ns \
                 over {ITERATIONS} iterations"
            );
            assert!(
                size <= 16 * 1024,
                "snapshot is {size} bytes; budget is 16 KiB"
            );
            assert!(
                publish_nanos < 100_000,
                "mean publish cost {publish_nanos} ns exceeded the 100 microsecond ceiling"
            );
            assert!(
                read_nanos < 100_000,
                "mean read cost {read_nanos} ns exceeded the 100 microsecond ceiling"
            );
        }
    }
}
