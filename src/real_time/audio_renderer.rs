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
            && self.active_graph.engine_rack().matches_parameters(&latest);
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
        let mix = {
            let (rack, patch_audio, mixer) = self.active_graph.callback_parts_mut();
            if patch_audio.begin_render(&parameters, frame_count).is_err() {
                return;
            }
            if rack.render(patch_audio, &parameters).is_err() {
                interleaved_stereo.fill(0.0);
                return;
            }
            mixer.mix(patch_audio, &parameters, interleaved_stereo)
        };

        self.rendered_blocks = self.rendered_blocks.saturating_add(1);
        self.rendered_frames = self.rendered_frames.saturating_add(frame_count as u64);
        self.observation
            .publish_from_callback(AudioObservationSnapshot::from_mix_with_routing(
                self.rendered_blocks,
                self.rendered_blocks,
                self.rendered_frames,
                parameters.generation(),
                self.commands_consumed,
                self.active_notes.count(),
                self.routing_failures,
                self.last_unknown_patch_id,
                mix,
            ));
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

        let Some(replacement) = self.structural.take_prepared_on_audio() else {
            self.structural.publish_status_on_audio(self.handoff_status);
            return;
        };

        let replacement_revision = replacement.revision();
        let replacement_parameters = *replacement.initial_parameters();
        let retired = core::mem::replace(&mut self.active_graph, replacement);
        let retired_revision = retired.revision();
        self.parameters = replacement_parameters;
        self.active_notes.clear_all();
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

    fn count(&self) -> u32 {
        self.patches
            .iter()
            .fold(0_u32, |count, patch| count.saturating_add(patch.count()))
    }
}

#[cfg(test)]
mod tests {
    use super::{ActiveNoteObservation, AudioRenderer};
    use crate::adapter::hidef_soundfont_capability::{
        HiDefSoundFontCapability, HIDEF_CAPABILITY_ID,
    };
    use crate::adapter::hidef_soundfont_preparer::HiDefSoundFontPreparer;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::channel_parameters::ChannelParameters;
    use crate::mixer::global_parameters::GlobalParameters;
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
            let provider = HiDefSoundFontCapability::new().unwrap();
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
                    ChannelParameters::new(0.0, 0.0, 0.0, 0.0).unwrap(),
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
                GlobalParameters::new(0.0, 0.5, 0.5, 0.0, 250.0, 0.5, 0.0).unwrap(),
                &self
                    .patches
                    .each_ref()
                    .map(|patch| RtPatchParameters::new(patch.id(), *patch.parameters())),
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
        let provider = HiDefSoundFontCapability::new().unwrap();
        let patch = Patch::new(
            PatchId::new(1).unwrap(),
            "Prepared piano".to_owned(),
            create_soundfont_config(&provider, SoundFontInstrument::new(0, 0, false).unwrap())
                .unwrap(),
            MidiChannel::new(0).unwrap(),
            ChannelParameters::default(),
        );
        let preparer = HiDefSoundFontPreparer::new().unwrap();
        let mut instrument = preparer.prepare(&patch, 48_000.0, 512).unwrap();
        let message =
            MidiMessage::try_new(patch.channel(), MidiMessageKind::NoteOn, 60, 100).unwrap();
        let mut output = [0.0; 1_024];
        let parameters = RtPatchParameters::new(patch.id(), *patch.parameters());

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
}
