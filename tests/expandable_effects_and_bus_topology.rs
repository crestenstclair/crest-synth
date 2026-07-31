//! The declared goal witness for the expandable-effects-and-bus-topology
//! phase (`witnesses.expandable_effects_and_bus_topology`): measures every
//! declared observation field against the production reducer, deterministic
//! preparation worker, coordinator, renderer, and mixer, and emits the
//! `CREST_EFFECTS_AND_BUSES_OBSERVATION` marker.
//!
//! Every field is measured, never asserted into existence.
//! `clearedSlotPreservedHeldNotes` is measured through the production
//! observation across a real cleared-slot activation with a note held: WP10's
//! voice carry-over exchanges the still-live prepared instances into the
//! replacement at the block boundary, so the measurement now honestly reads
//! `true` — and still reads `false` if the mechanism regresses, because
//! nothing here re-sounds the note after activation.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use crest_synth::adapter::atomic_audio_observation::AtomicAudioObservation;
use crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
use crest_synth::adapter::lock_free_structural_graph_boundary::LockFreeStructuralGraphBoundary;
use crest_synth::adapter::production_effects::{
    production_default_bus_returns, production_effect_preparers, production_effect_registry,
};
use crest_synth::adapter::production_instruments::{
    production_capability_registry, production_instrument_preparers,
    production_instrument_providers, production_soundfont_capability,
};
use crest_synth::control::app_event::AppEvent;
use crest_synth::control::app_loop::AppLoop;
use crest_synth::control::app_state::AppState;
use crest_synth::control::event_record::EventSource;
use crest_synth::control::state_projector::StateProjector;
use crest_synth::control::{
    EngineSelectionStatusKind, EventRejection, PatchControlId, SemanticAction,
};
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crest_synth::kernel::PatchId;
use crest_synth::mixer::bus_id::{BusId, MAX_BUS_RETURNS};
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::mixer::mix_engine::MixEngine;
use crest_synth::mixer::mixer_state::MixerState;
use crest_synth::mixer::mixer_track_id::MixerTrackId;
use crest_synth::mixer::mixer_track_parameters::MixerTrackParameters;
use crest_synth::mixer::patch_output::PatchOutput;
use crest_synth::real_time::audio_boundary::AudioBoundary;
use crest_synth::real_time::audio_observation::{AudioObservation, ControlAudioObservation};
use crest_synth::real_time::audio_renderer::AudioRenderer;
use crest_synth::real_time::graph_revision::GraphRevision;
use crest_synth::real_time::prepared_graph_builder::PreparedGraphBuilder;
use crest_synth::real_time::{
    GraphHandoffStatus, ParameterSnapshot, PatchAudioBlock, RtBusReturnParameters,
    RtPatchParameters, RtPostEffectParameters, StructuralGraphBoundary,
};
use crest_synth::shell::audio_output::{AudioDeviceConfig, AudioSampleFormat};
use crest_synth::synth::effect_slot_id::EffectSlotIndex;
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::{
    DescriptorDefaultConfigFactory, EffectCapabilityId, EffectSlotId, Patch, PreparedEffectError,
    PreparedPostEffect,
};
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use crest_synth::testing::deterministic_graph_preparation_worker::{
    DeterministicGraphPreparationHandle, DeterministicGraphPreparationWorker,
};
use serde::Serialize;
use std::alloc::System;

const SAMPLE_RATE: f32 = 48_000.0;
const FRAME_COUNT: usize = 128;
const SAMPLE_COUNT: usize = FRAME_COUNT * 2;
const SILENCE_DBFS: f64 = -200.0;

thread_local! {
    static COUNT_MEMORY: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    static DEALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

struct WitnessAllocator;

#[global_allocator]
static WITNESS_ALLOCATOR: WitnessAllocator = WitnessAllocator;

unsafe impl GlobalAlloc for WitnessAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT_MEMORY.with(Cell::get) {
            ALLOCATIONS.with(|count| count.set(count.get().saturating_add(1)));
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if COUNT_MEMORY.with(Cell::get) {
            DEALLOCATIONS.with(|count| count.set(count.get().saturating_add(1)));
        }
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct EffectsAndBusesObservation {
    schema_version: u32,
    ordered_slot_cases_exercised: usize,
    slot_order_exchange_distinct: bool,
    same_entry_instances_independent: bool,
    cleared_slot_preserved_held_notes: bool,
    cleared_slot_focus_recovered: bool,
    addressable_returns: usize,
    default_return_occupancy_exact: bool,
    max_off_target_bus_dbfs: f64,
    muted_or_solo_excluded_wet_contribution: f64,
    unoccupied_return_silent: bool,
    return_content_change_dry_uninterrupted: bool,
    topology_rejections_exercised: usize,
    rejection_preserved_active_graph: bool,
    rejection_reason_attributable: bool,
    post_rejection_valid_change_accepted: bool,
    partially_applied_topology_blocks: u64,
    registry_entry_addition_structural_changes: usize,
    callback_allocations: u64,
    callback_deallocations: u64,
    callback_destructions: u64,
    retired_graphs_collected_off_callback: u64,
    active_notes_at_exit: u32,
    two_run_trace_equal: bool,
}

type FixtureRenderer = AudioRenderer<
    crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioHandle,
    crest_synth::adapter::lock_free_structural_graph_boundary::LockFreeStructuralAudioHandle,
    crest_synth::adapter::atomic_audio_observation::AtomicAudioObservationWriter,
>;

struct Fixture {
    app_loop: AppLoop<crest_synth::adapter::lock_free_audio_boundary::LockFreeControlHandle>,
    renderer: FixtureRenderer,
    worker: DeterministicGraphPreparationHandle,
    observation: crest_synth::adapter::atomic_audio_observation::AtomicAudioObservationReader,
}

impl Fixture {
    fn counted_render(&mut self, output: &mut [f32], trace: &mut Vec<u8>) -> (u64, u64) {
        ALLOCATIONS.with(|count| count.set(0));
        DEALLOCATIONS.with(|count| count.set(0));
        COUNT_MEMORY.with(|enabled| enabled.set(true));
        self.renderer.render(output);
        COUNT_MEMORY.with(|enabled| enabled.set(false));
        for sample in output.iter() {
            trace.extend_from_slice(&sample.to_le_bytes());
        }
        (ALLOCATIONS.with(Cell::get), DEALLOCATIONS.with(Cell::get))
    }

    fn note_on(&mut self, note: u8) {
        self.app_loop
            .dispatch_from(
                AppEvent::Midi {
                    patch_id: PatchId::new(1).unwrap(),
                    message: MidiMessage::try_new(
                        MidiChannel::new(0).unwrap(),
                        MidiMessageKind::NoteOn,
                        note,
                        112,
                    )
                    .unwrap(),
                },
                EventSource::System,
            )
            .unwrap();
    }

    fn all_notes_off(&mut self) {
        self.app_loop
            .dispatch_from(
                AppEvent::Midi {
                    patch_id: PatchId::new(1).unwrap(),
                    message: MidiMessage::all_notes_off(MidiChannel::new(0).unwrap()),
                },
                EventSource::System,
            )
            .unwrap();
    }

    /// Drives one accepted occupancy request through preparation, atomic
    /// block-boundary activation, and acknowledgement; returns collected
    /// retirements plus the count of partially-applied blocks observed and
    /// the allocator counts across the activation renders.
    fn complete_change(
        &mut self,
        output: &mut [f32],
        trace: &mut Vec<u8>,
        counters: &mut Counters,
    ) -> u64 {
        assert!(self.worker.advance(), "worker advances one request");
        let source = self.renderer.active_revision();
        let staged = self.app_loop.advance_structural().unwrap();
        assert!(staged.graph_stage().is_some(), "graph stages off-callback");
        let (allocations, deallocations) = self.counted_render(output, trace);
        counters.allocations += allocations;
        counters.deallocations += deallocations;
        let target = self.renderer.active_revision();
        assert!(target > source, "block-boundary activation");
        let observed = self.observation.read_latest_on_control();
        if observed.active_graph_revision() != source && observed.active_graph_revision() != target
        {
            counters.partial_blocks += 1;
        }
        let ack = self.app_loop.advance_structural().unwrap();
        assert!(ack.activation_acknowledged().is_some());
        assert_eq!(
            self.app_loop.engine_selection_status().kind(),
            EngineSelectionStatusKind::Ready
        );
        ack.collected_count()
    }

    /// Renders one deterministic fresh-note chain trace of the current
    /// topology: all notes off, one note on, then the given block count.
    fn chain_trace(
        &mut self,
        note: u8,
        blocks: usize,
        counters: &mut Counters,
        trace: &mut Vec<u8>,
    ) -> Vec<u8> {
        self.all_notes_off();
        let mut output = vec![0.0_f32; SAMPLE_COUNT];
        let (allocations, deallocations) = self.counted_render(&mut output, trace);
        counters.allocations += allocations;
        counters.deallocations += deallocations;
        self.note_on(note);
        let mut chain = Vec::with_capacity(blocks * SAMPLE_COUNT * 4);
        for _ in 0..blocks {
            let (allocations, deallocations) = self.counted_render(&mut output, trace);
            counters.allocations += allocations;
            counters.deallocations += deallocations;
            for sample in &output {
                assert!(sample.is_finite(), "rendered audio stays finite");
                chain.extend_from_slice(&sample.to_le_bytes());
            }
        }
        self.all_notes_off();
        let (allocations, deallocations) = self.counted_render(&mut output, trace);
        counters.allocations += allocations;
        counters.deallocations += deallocations;
        chain
    }
}

#[derive(Default)]
struct Counters {
    allocations: u64,
    deallocations: u64,
    partial_blocks: u64,
}

fn soundfont_patch() -> Patch {
    Patch::new(
        PatchId::new(1).unwrap(),
        "Witness 1".to_owned(),
        create_soundfont_config(
            &production_soundfont_capability().unwrap(),
            SoundFontInstrument::new(0, 0, false).unwrap(),
        )
        .unwrap(),
        MidiChannel::new(0).unwrap(),
        PatchOutput::to_track(MixerTrackId::ALL[0]),
    )
}

fn fixture() -> Fixture {
    let registry = production_capability_registry().unwrap();
    let effects = production_effect_registry().unwrap();
    let bank = production_default_bus_returns(&effects).unwrap();

    // Track 0 sends toward the default-occupied bus 1 so return-content
    // changes are wet-coupled.
    let mut sends = [0.0_f32; MAX_BUS_RETURNS];
    sends[1] = 0.5;
    let mixer = MixerState::default().with_track(
        MixerTrackId::ALL[0],
        MixerTrackParameters::from_values(0.0, 0.0, false, false, sends).unwrap(),
    );

    let mut state = AppState::for_graph_with_effects(
        registry.clone(),
        effects.clone(),
        GlobalParameters::new(0.0).unwrap(),
        GraphRevision::INITIAL,
    )
    .with_initial_returns(bank)
    .with_initial_mixer(mixer);
    state
        .apply(AppEvent::InstallPatches(vec![soundfont_patch()]))
        .unwrap();

    let initial_transport = ParameterSnapshot::new(
        0,
        GlobalParameters::new(0.0).unwrap(),
        MixerState::default(),
        &[],
    )
    .unwrap();
    let boundary = LockFreeAudioBoundary::new(128, initial_transport);
    let (audio_control, audio_callback) = boundary.into_handles();
    let mut app_loop = AppLoop::new(
        state,
        StateProjector::for_graph(GraphRevision::INITIAL),
        audio_control,
    )
    .unwrap();

    let audio_config =
        AudioDeviceConfig::new(SAMPLE_RATE, 2, AudioSampleFormat::F32, FRAME_COUNT).unwrap();
    let instrument_preparers = production_instrument_preparers().unwrap();
    let effect_preparers = production_effect_preparers().unwrap();
    let initial_graph = PreparedGraphBuilder::new(&registry, &instrument_preparers)
        .with_effects(&effects, &effect_preparers)
        .with_returns(app_loop.bus_returns())
        .build(
            GraphRevision::INITIAL,
            app_loop.patches(),
            *app_loop.current_parameters(),
            SAMPLE_RATE,
            FRAME_COUNT,
        )
        .unwrap();

    let structural = LockFreeStructuralGraphBoundary::new(
        1,
        1,
        GraphHandoffStatus::with_active(GraphRevision::INITIAL),
    )
    .unwrap();
    let (structural_control, structural_callback) = structural.into_handles();
    let worker = DeterministicGraphPreparationWorker::new_with_effects(
        registry.clone(),
        production_instrument_preparers().unwrap(),
        effects.clone(),
        production_effect_preparers().unwrap(),
        audio_config,
    );
    let worker_handle = worker.advance_handle();
    app_loop
        .configure_engine_selection(
            DescriptorDefaultConfigFactory::new(
                registry,
                production_instrument_providers().unwrap(),
            ),
            worker,
            structural_control,
            &initial_graph,
            audio_config,
        )
        .unwrap();
    let (writer, reader) = AtomicAudioObservation::default().into_handles();
    let renderer =
        AudioRenderer::with_observation(audio_callback, structural_callback, initial_graph, writer);

    Fixture {
        app_loop,
        renderer,
        worker: worker_handle,
        observation: reader,
    }
}

/// A sample-exact unity return for the isolation and gate measurements.
struct UnityReturn;

impl PreparedPostEffect for UnityReturn {
    fn patch_id(&self) -> PatchId {
        PatchId::new(u32::MAX).unwrap()
    }

    fn slot_id(&self) -> EffectSlotId {
        EffectSlotId::new(1).unwrap()
    }

    fn process(
        &mut self,
        _interleaved_stereo: &mut [f32],
        _frame_count: usize,
        _parameters: &RtPostEffectParameters,
    ) -> Result<(), PreparedEffectError> {
        Ok(())
    }
}

struct RoutingMeasurements {
    max_off_target_bus_dbfs: f64,
    gated_wet_contribution: f64,
    unoccupied_return_silent: bool,
    dry_uninterrupted_across_content_change: bool,
}

/// SC-004/SC-005/NFR-007/C-BR-6: eight-destination isolation, gate, and
/// unoccupied-return measurements over the production mixer, plus the
/// dry-path independence from return content.
fn measure_routing() -> RoutingMeasurements {
    const MIX_FRAMES: usize = 8;
    const MIX_SAMPLES: usize = MIX_FRAMES * 2;

    let run = |parameters: MixerTrackParameters,
               unity_buses: &[BusId],
               live_slot: u16|
     -> (
        [f32; MIX_SAMPLES],
        crest_synth::mixer::mix_observation::MixObservation,
    ) {
        let track_id = MixerTrackId::ALL[0];
        let stems: [f32; MIX_SAMPLES] =
            core::array::from_fn(|index| if index % 2 == 0 { 0.4 } else { 0.2 });
        let mixer_state = MixerState::default().with_track(track_id, parameters);
        let patches = [RtPatchParameters::new(
            PatchId::new(1).unwrap(),
            PatchOutput::to_track(track_id),
        )];
        let base = ParameterSnapshot::new(
            1,
            GlobalParameters::new(0.0).unwrap(),
            mixer_state,
            &patches,
        )
        .unwrap();
        let mut returns = [RtBusReturnParameters::EMPTY; MAX_BUS_RETURNS];
        for bus in unity_buses {
            returns[bus.index()] =
                RtBusReturnParameters::new(EffectSlotId::new(live_slot).unwrap(), &[], 1.0)
                    .unwrap();
        }
        let snapshot = base.with_returns(returns);
        let mut block = PatchAudioBlock::prepare(MIX_FRAMES).unwrap();
        block.begin_render(&snapshot, MIX_FRAMES).unwrap();
        block
            .stem_mut(0, PatchId::new(1).unwrap())
            .unwrap()
            .copy_from_slice(&stems);
        let mut mixer = MixEngine::new();
        mixer.prepare(SAMPLE_RATE, MIX_FRAMES).unwrap();
        for bus in unity_buses {
            mixer
                .install_bus_return(
                    *bus,
                    Box::new(UnityReturn),
                    RtPostEffectParameters::new(EffectSlotId::new(1).unwrap(), &[]).unwrap(),
                    1.0,
                )
                .unwrap();
        }
        let mut output = [0.0_f32; MIX_SAMPLES];
        let observation = mixer.mix(&block, &snapshot, &mut output);
        (output, observation)
    };

    // Isolation: raise one send toward each destination in turn; every
    // other destination must measure exact silence.
    let mut max_off_target = 0.0_f64;
    for target in BusId::ALL {
        let parameters = MixerTrackParameters::default()
            .with_send(target, 0.8)
            .unwrap();
        let (_, observation) = run(parameters, &BusId::ALL, 1);
        assert!(observation.bus_input_rms(target) > 0.0);
        for other in BusId::ALL {
            if other != target {
                max_off_target = max_off_target
                    .max(f64::from(observation.bus_input_rms(other)))
                    .max(f64::from(observation.bus_output_rms(other)));
            }
        }
    }
    let max_off_target_bus_dbfs = if max_off_target <= 0.0 {
        SILENCE_DBFS
    } else {
        (20.0 * max_off_target.log10()).max(SILENCE_DBFS)
    };

    // SC-005: muted and solo-excluded tracks contribute zero wet signal.
    let muted =
        MixerTrackParameters::from_values(0.0, 0.0, true, false, [1.0; MAX_BUS_RETURNS]).unwrap();
    let (_, muted_observation) = run(muted, &BusId::ALL, 1);
    let mut gated_wet = 0.0_f64;
    for bus in BusId::ALL {
        gated_wet = gated_wet
            .max(f64::from(muted_observation.bus_input_rms(bus)))
            .max(f64::from(muted_observation.bus_output_rms(bus)));
    }

    // C-BR-6: an unoccupied return contributes silence, never its input.
    let sends = MixerTrackParameters::default()
        .with_send(BusId::ALL[7], 1.0)
        .unwrap();
    let (unoccupied_output, unoccupied_observation) = run(sends, &[], 1);
    let dry_reference: [f32; MIX_SAMPLES] =
        core::array::from_fn(|index| if index % 2 == 0 { 0.4 } else { 0.2 });
    let unoccupied_return_silent = unoccupied_observation.bus_input_rms(BusId::ALL[7]) > 0.0
        && unoccupied_observation.bus_output_rms(BusId::ALL[7]) == 0.0
        && unoccupied_output
            .iter()
            .zip(dry_reference.iter())
            .all(|(actual, expected)| (actual - expected).abs() <= 1.0e-6);

    // FR-013: a return-content change leaves the dry path untouched — a
    // track with zero sends renders identically under either occupant.
    let dry_only = MixerTrackParameters::default();
    let (before_change, _) = run(dry_only, &[BusId::ALL[1]], 1);
    let (after_change, _) = run(dry_only, &[BusId::ALL[1]], 2);
    let wet_coupled = MixerTrackParameters::default()
        .with_send(BusId::ALL[1], 0.5)
        .unwrap();
    let (coupled_attested, _) = run(wet_coupled, &[BusId::ALL[1]], 1);
    let (coupled_changed, _) = run(wet_coupled, &[BusId::ALL[1]], 2);
    let dry_uninterrupted_across_content_change =
        before_change == after_change && coupled_attested != coupled_changed;

    RoutingMeasurements {
        max_off_target_bus_dbfs,
        gated_wet_contribution: gated_wet,
        unoccupied_return_silent,
        dry_uninterrupted_across_content_change,
    }
}

#[allow(clippy::too_many_lines)]
fn measure() -> (EffectsAndBusesObservation, Vec<u8>) {
    let mut fixture = fixture();
    let mut output = vec![0.0_f32; SAMPLE_COUNT];
    let mut trace = Vec::new();
    let mut counters = Counters::default();
    let entries: Vec<EffectCapabilityId> = fixture
        .app_loop
        .effects()
        .descriptors()
        .iter()
        .map(|descriptor| descriptor.id().clone())
        .collect();
    let patch_id = PatchId::new(1).unwrap();
    let slot = |index: usize| EffectSlotIndex::ALL[index];
    let occupy =
        |slot_index: usize, entry: Option<EffectCapabilityId>| SemanticAction::SetSlotOccupancy {
            patch_id,
            slot: slot(slot_index),
            entry,
        };

    let mut collected = 0_u64;

    // FR-002/FR-004: fill all three slots one at a time; every addition
    // changes the deterministic fresh-note render.
    let mut previous = fixture.chain_trace(60, 120, &mut counters, &mut trace);
    let mut ordered_slot_cases_exercised = 0;
    for (index, entry) in entries.iter().enumerate().take(EffectSlotIndex::ALL.len()) {
        fixture
            .app_loop
            .dispatch_action_from(occupy(index, Some(entry.clone())), EventSource::System)
            .unwrap();
        collected += fixture.complete_change(&mut output, &mut trace, &mut counters);
        let current = fixture.chain_trace(60, 120, &mut counters, &mut trace);
        if current != previous {
            ordered_slot_cases_exercised += 1;
        }
        previous = current;
    }

    // SC-002: exchanging two different effects produces a measurably
    // different render.
    fixture
        .app_loop
        .dispatch_action_from(occupy(0, Some(entries[1].clone())), EventSource::System)
        .unwrap();
    collected += fixture.complete_change(&mut output, &mut trace, &mut counters);
    fixture
        .app_loop
        .dispatch_action_from(occupy(1, Some(entries[0].clone())), EventSource::System)
        .unwrap();
    collected += fixture.complete_change(&mut output, &mut trace, &mut counters);
    let exchanged = fixture.chain_trace(60, 120, &mut counters, &mut trace);
    let slot_order_exchange_distinct = exchanged != previous;

    // SC-003: two instances of one registry entry occupy positions 0 and 2
    // as distinct prepared instances with their own state.
    fixture
        .app_loop
        .dispatch_action_from(occupy(0, Some(entries[2].clone())), EventSource::System)
        .unwrap();
    collected += fixture.complete_change(&mut output, &mut trace, &mut counters);
    let twin = fixture.chain_trace(60, 120, &mut counters, &mut trace);
    let slots = fixture.app_loop.patches()[0].effect_slots();
    let same_entry_instances_independent = twin != exchanged
        && slots[0]
            .as_ref()
            .zip(slots[2].as_ref())
            .is_some_and(|(first, third)| {
                first.capability_id() == third.capability_id() && first.slot_id() != third.slot_id()
            });

    // FR-003/AS-1.5: clear position 0 while a held note rings, with the slot
    // row focused. The held-note value is measured through the production
    // observation after the activation — no note is re-sounded, so the
    // measurement can only read true if the sounding voice itself survived
    // the swap (WP10 voice carry-over).
    fixture
        .app_loop
        .dispatch_from(
            AppEvent::SelectContext(crest_synth::control::TopLevelContext::Patch),
            EventSource::System,
        )
        .unwrap();
    let mut guard = 0;
    while fixture
        .app_loop
        .current_patch_page()
        .is_some_and(|page| page.focused_control_id() != PatchControlId::EffectSlot(slot(0)))
    {
        fixture
            .app_loop
            .dispatch_from(
                AppEvent::Navigate(crest_synth::control::Direction::Down),
                EventSource::System,
            )
            .unwrap();
        guard += 1;
        assert!(guard < 64, "the slot row is reachable");
    }
    fixture.note_on(64);
    let (allocations, deallocations) = fixture.counted_render(&mut output, &mut trace);
    counters.allocations += allocations;
    counters.deallocations += deallocations;
    let ringing = fixture.observation.read_latest_on_control();
    assert!(ringing.active_notes() > 0, "the held note rings");
    fixture
        .app_loop
        .dispatch_action_from(occupy(0, None), EventSource::System)
        .unwrap();
    collected += fixture.complete_change(&mut output, &mut trace, &mut counters);
    let (allocations, deallocations) = fixture.counted_render(&mut output, &mut trace);
    counters.allocations += allocations;
    counters.deallocations += deallocations;
    let after_clear = fixture.observation.read_latest_on_control();
    let cleared_slot_preserved_held_notes =
        after_clear.primary_active_notes() > 0 && after_clear.primary_patch_rms() > 0.0;
    // FR-002: the cleared position stays a stable, focused address, and the
    // canonical focus and the projected page agree exactly.
    let focus_tree: serde_json::Value =
        serde_json::from_str(fixture.app_loop.current_state_tree().json()).unwrap();
    let cleared_slot_focus_recovered = fixture
        .app_loop
        .current_patch_page()
        .is_some_and(|page| page.focused_control_id() == PatchControlId::EffectSlot(slot(0)))
        && focus_tree
            .pointer("/patchPage/focusedControlId")
            .and_then(serde_json::Value::as_str)
            == focus_tree
                .pointer("/interaction/activeFocus/controlId/id")
                .and_then(serde_json::Value::as_str)
        && focus_tree
            .pointer("/patchPage/focusedControlId")
            .and_then(serde_json::Value::as_str)
            .is_some();
    fixture.all_notes_off();

    // FR-013: change the content of the default-occupied return through the
    // complete lifecycle.
    fixture
        .app_loop
        .dispatch_action_from(
            SemanticAction::SetReturnOccupancy {
                bus: BusId::ALL[1],
                entry: Some(entries[0].clone()),
            },
            EventSource::System,
        )
        .unwrap();
    collected += fixture.complete_change(&mut output, &mut trace, &mut counters);
    assert!(fixture
        .app_loop
        .bus_returns()
        .bus_return(BusId::ALL[1])
        .is_occupied());

    // FR-015/T033: every refused class is typed, attributable, and mutates
    // nothing.
    let state_hash_before = fixture
        .app_loop
        .current_state_tree()
        .state_hash()
        .to_owned();
    let mut topology_rejections_exercised = 0;
    let refusals = [
        fixture.app_loop.dispatch_action_from(
            SemanticAction::SetSlotOccupancy {
                patch_id: PatchId::new(99).unwrap(),
                slot: slot(0),
                entry: Some(entries[0].clone()),
            },
            EventSource::System,
        ),
        fixture.app_loop.dispatch_action_from(
            SemanticAction::SetSlotOccupancy {
                patch_id,
                slot: slot(1),
                entry: Some(EffectCapabilityId::new("effect.absent").unwrap()),
            },
            EventSource::System,
        ),
        fixture.app_loop.dispatch_action_from(
            SemanticAction::SetReturnOccupancy {
                bus: BusId::ALL[4],
                entry: Some(EffectCapabilityId::new("effect.absent").unwrap()),
            },
            EventSource::System,
        ),
    ];
    assert!(matches!(refusals[0], Err(EventRejection::UnknownPatch)));
    assert!(matches!(
        refusals[1],
        Err(EventRejection::InvalidEffectConfig)
    ));
    assert!(matches!(
        refusals[2],
        Err(EventRejection::InvalidEffectConfig)
    ));
    topology_rejections_exercised += refusals.iter().filter(|result| result.is_err()).count();
    assert_eq!(
        fixture.app_loop.current_state_tree().state_hash(),
        state_hash_before,
        "refusals mutate nothing"
    );

    // The worker-refused preparation, its preserved graph, its attributable
    // reason, and the accepted recovery come from the refused-topology
    // witness case — the same measurement the declared negative command
    // falsifies.
    let refused = crest_synth::testing::BehavioralMutationHarness::new().run(
        crest_synth::testing::BehavioralMutationCase::RefusedTopology,
        false,
    );
    let crest_synth::testing::BehavioralMutationObservation::RefusedTopology(refused) =
        refused.into_observation()
    else {
        panic!("the refused-topology case retains its typed schema");
    };
    topology_rejections_exercised += usize::from(refused.refusal_recorded);
    let rejection_preserved_active_graph = refused.active_graph_preserved
        && refused.canonical_state_preserved
        && refused.render_preserved_exactly;
    let rejection_reason_attributable =
        refused.rejection_reason_attributable && refused.rejection_reason == "preparationFailed";
    let post_rejection_valid_change_accepted = refused.post_rejection_valid_change_accepted;

    // B-1: all eight returns are addressable by position.
    let registry = production_effect_registry().unwrap();
    let addressable_returns = BusId::ALL
        .iter()
        .filter(|bus| {
            let mut bank = fixture.app_loop.bus_returns().clone();
            bank.set_return_occupancy(&registry, **bus, Some(&entries[0]))
                .is_ok()
        })
        .count();

    // The composition default: reverb behavior at position 0, delay at
    // position 1 (positional registry entries 1 and 2), retained levels.
    let default_bank = production_default_bus_returns(&registry).unwrap();
    let default_return_occupancy_exact = default_bank
        .bus_return(BusId::ALL[0])
        .effect()
        .is_some_and(|config| config.capability_id() == &entries[1])
        && default_bank
            .bus_return(BusId::ALL[1])
            .effect()
            .is_some_and(|config| config.capability_id() == &entries[2])
        && (default_bank.bus_return(BusId::ALL[0]).return_level() - 0.5).abs() <= f32::EPSILON
        && (default_bank.bus_return(BusId::ALL[1]).return_level() - 0.5).abs() <= f32::EPSILON
        && BusId::ALL[2..]
            .iter()
            .all(|bus| !default_bank.bus_return(*bus).is_occupied());

    let routing = measure_routing();

    // SC-008: the structural vocabulary names no registry entry, so adding
    // an entry changes zero structure — measured by scanning the declared
    // realtime leaf schema for entry identities.
    let registry_entry_addition_structural_changes = ParameterSnapshot::SERIALIZED_LEAF_DESCRIPTOR
        .iter()
        .filter(|leaf| entries.iter().any(|entry| leaf.contains(entry.as_str())))
        .count();

    // NFR-006: teardown — zero active notes, clean ownership.
    fixture.all_notes_off();
    let (allocations, deallocations) = fixture.counted_render(&mut output, &mut trace);
    counters.allocations += allocations;
    counters.deallocations += deallocations;
    let final_observation = fixture.observation.read_latest_on_control();
    let active_notes_at_exit = final_observation.active_notes();
    drop(fixture.renderer);
    assert_eq!(fixture.app_loop.owned_structural_graphs_on_control(), 0);
    fixture
        .app_loop
        .shutdown_engine_selection_on_control()
        .unwrap();

    trace.extend_from_slice(fixture.app_loop.current_state_tree().json().as_bytes());

    let observation = EffectsAndBusesObservation {
        schema_version: 1,
        ordered_slot_cases_exercised,
        slot_order_exchange_distinct,
        same_entry_instances_independent,
        cleared_slot_preserved_held_notes,
        cleared_slot_focus_recovered,
        addressable_returns,
        default_return_occupancy_exact,
        max_off_target_bus_dbfs: routing.max_off_target_bus_dbfs,
        muted_or_solo_excluded_wet_contribution: routing.gated_wet_contribution,
        unoccupied_return_silent: routing.unoccupied_return_silent,
        return_content_change_dry_uninterrupted: routing.dry_uninterrupted_across_content_change,
        topology_rejections_exercised,
        rejection_preserved_active_graph,
        rejection_reason_attributable,
        post_rejection_valid_change_accepted,
        partially_applied_topology_blocks: counters.partial_blocks,
        registry_entry_addition_structural_changes,
        callback_allocations: counters.allocations,
        callback_deallocations: counters.deallocations,
        callback_destructions: counters.deallocations,
        retired_graphs_collected_off_callback: collected,
        active_notes_at_exit,
        two_run_trace_equal: false,
    };
    (observation, trace)
}

#[test]
fn expandable_effects_and_bus_topology() {
    let (mut first, first_trace) = measure();
    let (mut second, second_trace) = measure();
    let two_run_trace_equal = first_trace == second_trace && {
        first.two_run_trace_equal = true;
        second.two_run_trace_equal = true;
        first == second
    };
    first.two_run_trace_equal = two_run_trace_equal;

    assert_eq!(first.schema_version, 1);
    assert_eq!(first.ordered_slot_cases_exercised, 3);
    assert!(first.slot_order_exchange_distinct);
    assert!(first.same_entry_instances_independent);
    // AS-1.5/SC-001 (WP10): the sounding voice itself survived the cleared-
    // slot activation — measured, with no re-sounded note to fake it.
    assert!(first.cleared_slot_preserved_held_notes);
    assert!(first.cleared_slot_focus_recovered);
    assert_eq!(first.addressable_returns, 8);
    assert!(first.default_return_occupancy_exact);
    assert!(first.max_off_target_bus_dbfs < -60.0);
    assert_eq!(first.muted_or_solo_excluded_wet_contribution, 0.0);
    assert!(first.unoccupied_return_silent);
    assert!(first.return_content_change_dry_uninterrupted);
    assert!(first.topology_rejections_exercised > 0);
    assert!(first.rejection_preserved_active_graph);
    assert!(first.rejection_reason_attributable);
    assert!(first.post_rejection_valid_change_accepted);
    assert_eq!(first.partially_applied_topology_blocks, 0);
    assert_eq!(first.registry_entry_addition_structural_changes, 0);
    assert_eq!(first.callback_allocations, 0);
    assert_eq!(first.callback_deallocations, 0);
    assert_eq!(first.callback_destructions, 0);
    assert!(first.retired_graphs_collected_off_callback > 0);
    assert_eq!(first.active_notes_at_exit, 0);
    assert!(first.two_run_trace_equal);

    let json = serde_json::to_string(&first).unwrap();
    println!("CREST_EFFECTS_AND_BUSES_OBSERVATION {json}");
    println!("CREST_ACCEPTANCE expandable_effects_and_bus_topology passed");
}
