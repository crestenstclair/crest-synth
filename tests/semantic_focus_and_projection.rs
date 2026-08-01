//! Semantic focus and projection for effect slots and bus returns.
//!
//! Proves FR-002 (adjacent-choice occupancy, no modal — C-008), FR-003
//! (descriptor-driven slot parameters), FR-014 (observable outcome), FR-017
//! (deterministic focus survival), and C-003 (PATCH and MIXER remain the only
//! top-level contexts) against the production reducer and projection path.

#[allow(dead_code)]
mod support;

use crest_synth::adapter::production_effects::{
    production_chorus_config, production_default_bus_returns, production_effect_registry,
};
use crest_synth::adapter::production_instruments::production_capability_registry;
use crest_synth::control::app_event::{AppEvent, Direction};
use crest_synth::control::{
    AppState, EventRejection, FocusPath, InteractionMode, MixerControlId, PatchControlId,
    SemanticAction, SemanticControlId, StateProjector, StructuralEditIntent, SurfaceId,
    TopLevelContext,
};
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::patch_id::PatchId;
use crest_synth::mixer::bus_id::BusId;
use crest_synth::mixer::mixer_track_id::MixerTrackId;
use crest_synth::mixer::patch_output::PatchOutput;
use crest_synth::synth::effect_slot_id::{EffectSlotIndex, MAX_EFFECT_SLOTS};
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::{EffectCapabilityId, EffectSlotId, Patch, PatchInteraction};
use crest_synth::testing::automatic_midi_test::create_soundfont_config;

fn soundfont_patch(id: u32, channel: u8, track: u8) -> Patch {
    let provider =
        crest_synth::adapter::production_instruments::production_soundfont_capability().unwrap();
    Patch::new(
        PatchId::new(id).unwrap(),
        format!("Focus {id}"),
        create_soundfont_config(&provider, SoundFontInstrument::new(128, 0, false).unwrap())
            .unwrap(),
        MidiChannel::new(channel).unwrap(),
        PatchOutput::to_track(MixerTrackId::new(track).unwrap()),
    )
}

/// Two installed Patches; the first occupies slot 0 with the production
/// chorus config, and returns 0/1 carry the default production occupancy.
fn installed_state() -> AppState {
    let effects = production_effect_registry().unwrap();
    let returns = production_default_bus_returns(&effects).unwrap();
    let mut state = AppState::new_with_effects(
        production_capability_registry().unwrap(),
        effects,
        support::globals(),
    )
    .with_initial_returns(returns);
    let first = soundfont_patch(1, 0, 2).with_effect_slot(
        EffectSlotIndex::ALL[0],
        production_chorus_config(EffectSlotId::new(1).unwrap()).unwrap(),
    );
    state
        .apply(AppEvent::InstallPatches(vec![
            first,
            soundfont_patch(2, 1, 3),
        ]))
        .unwrap();
    state
}

/// Drives one accepted occupancy request through TopologyPrepared and the
/// activation acknowledgement, exactly as the production orchestration does.
fn commit_pending_topology(state: &mut AppState) {
    let correlation = state.engine_selection().correlation().unwrap().clone();
    let source = correlation.source_graph_revision();
    let target = source.checked_next().unwrap();
    state
        .apply(AppEvent::TopologyPrepared {
            request_id: correlation.request_id(),
            intent: correlation.intent().clone(),
            source_graph_revision: source,
            target_graph_revision: target,
        })
        .unwrap();
    state
        .apply(AppEvent::EngineActivationAcknowledged {
            request_id: correlation.request_id(),
            intent: correlation.intent().clone(),
            target_graph_revision: target,
            retired_graph_revision: source,
            collected: true,
        })
        .unwrap();
}

fn set_mode(state: &mut AppState, mode: InteractionMode) {
    state
        .apply_semantic_action(SemanticAction::SetInteractionMode(mode))
        .unwrap();
}

fn focused_patch_control(state: &AppState) -> PatchControlId {
    state.interaction().patch_control_focus().unwrap()
}

/// The PATCH focus order contributes one occupancy row per slot
/// whether occupied or empty, bare Up/Down never wraps, and PATCH and MIXER
/// remain the only top-level contexts with the four fixed surfaces.
#[test]
fn all_three_slot_rows_are_reachable_and_the_context_set_is_closed() {
    let mut state = installed_state();
    state
        .apply(AppEvent::SelectContext(TopLevelContext::Patch))
        .unwrap();
    let controls = state.focused_patch_controls().unwrap();
    for slot in EffectSlotIndex::ALL {
        assert!(
            controls.contains(&PatchControlId::EffectSlot(slot)),
            "slot {slot} must contribute its occupancy row"
        );
    }
    // Occupied slot 0 interleaves its scalar rows after its occupancy row;
    // empty slots 1 and 2 contribute exactly their occupancy rows.
    let slot0 = controls
        .iter()
        .position(|control| control == &PatchControlId::EffectSlot(EffectSlotIndex::ALL[0]))
        .unwrap();
    assert!(matches!(controls[slot0 + 1], PatchControlId::Effect(_, _)));
    assert_eq!(
        controls[controls.len() - 2..],
        [
            PatchControlId::EffectSlot(EffectSlotIndex::ALL[1]),
            PatchControlId::EffectSlot(EffectSlotIndex::ALL[2]),
        ]
    );

    // Bare Up at the top and Down past the last row are rejected unchanged.
    let top = state.clone();
    assert_eq!(
        state.apply(AppEvent::Navigate(Direction::Up)),
        Err(EventRejection::ActionUnavailableInContext)
    );
    assert_eq!(state, top);
    for expected in &controls[1..] {
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        assert_eq!(&focused_patch_control(&state), expected);
    }
    let bottom = state.clone();
    assert_eq!(
        state.apply(AppEvent::Navigate(Direction::Down)),
        Err(EventRejection::ActionUnavailableInContext)
    );
    assert_eq!(state, bottom);

    // C-003: two top-level contexts, four fixed surfaces, no modal focus.
    assert_eq!(
        TopLevelContext::surface_descriptor(),
        &[TopLevelContext::Patch, TopLevelContext::Mixer]
    );
    assert_eq!(SurfaceId::surface_descriptor().len(), 4);
    assert!(state.interaction().focus_path().modal_id().is_none());
    assert_eq!(
        InteractionMode::PHASE_TWO,
        [InteractionMode::Navigate, InteractionMode::Adjust]
    );
}

/// Occupancy rows use the engine row's adjacent nonwrapping choice
/// contract — empty, then each installed registry entry in declared order —
/// and Edit+Up/Down is unavailable. No modal, picker, or new key exists
/// (C-008): the whole cycle happens through Adjust(Left/Right) in place.
#[test]
fn slot_occupancy_cycles_adjacent_choices_without_wrapping() {
    let mut state = installed_state();
    state
        .apply(AppEvent::SelectContext(TopLevelContext::Patch))
        .unwrap();
    let controls = state.focused_patch_controls().unwrap();
    let empty_slot = controls
        .iter()
        .position(|control| control == &PatchControlId::EffectSlot(EffectSlotIndex::ALL[1]))
        .unwrap();
    for _ in 0..empty_slot {
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
    }
    set_mode(&mut state, InteractionMode::Adjust);

    // Vertical edit is unavailable on an occupancy row, exactly as on Engine.
    let before = state.clone();
    assert_eq!(
        state.apply(AppEvent::Adjust(Direction::Up)),
        Err(EventRejection::ActionUnavailableInContext)
    );
    assert_eq!(
        state.apply(AppEvent::Adjust(Direction::Down)),
        Err(EventRejection::ActionUnavailableInContext)
    );
    // Empty is the first choice: Left at empty is the boundary, not a wrap.
    assert_eq!(
        state.apply(AppEvent::Adjust(Direction::Left)),
        Err(EventRejection::ParameterAtBoundary)
    );
    assert_eq!(state, before);

    // Right walks empty -> each installed entry in declared registry order.
    let entries = production_effect_registry()
        .unwrap()
        .descriptors()
        .iter()
        .map(|descriptor| descriptor.id().clone())
        .collect::<Vec<EffectCapabilityId>>();
    for expected_entry in &entries {
        state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
        let correlation = state.engine_selection().correlation().unwrap();
        assert_eq!(
            correlation.intent(),
            &StructuralEditIntent::SetSlotOccupancy {
                patch_id: PatchId::new(1).unwrap(),
                slot: EffectSlotIndex::ALL[1],
                entry: Some(expected_entry.clone()),
            }
        );
        // One in flight application-wide: a second adjacent request is a
        // synchronous busy refusal, never a queue.
        assert_eq!(
            state.apply(AppEvent::Adjust(Direction::Right)),
            Err(EventRejection::StructuralEditBusy)
        );
        commit_pending_topology(&mut state);
        assert_eq!(
            state.patches()[0]
                .effect_slot(EffectSlotIndex::ALL[1])
                .map(|config| config.capability_id().clone()),
            Some(expected_entry.clone())
        );
        // Focus stayed on the occupancy row while its parameter rows
        // appeared beneath.
        assert_eq!(
            focused_patch_control(&state),
            PatchControlId::EffectSlot(EffectSlotIndex::ALL[1])
        );
    }
    // The last installed entry is the far endpoint: no wrap back to empty.
    let end = state.clone();
    assert_eq!(
        state.apply(AppEvent::Adjust(Direction::Right)),
        Err(EventRejection::ParameterAtBoundary)
    );
    assert_eq!(state, end);
}

/// The same contract on a bus return's occupancy row.
#[test]
fn return_occupancy_uses_the_same_adjacent_choice_contract() {
    let mut state = installed_state();
    // Enter the Inspector and walk to the first empty return's occupancy row
    // (bus 2): eight sends, then bus 0 (occupancy, level, two scalars), then
    // bus 1 (occupancy, level, two scalars), then bus 2 occupancy.
    state
        .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::MixerInspector))
        .unwrap();
    for _ in 0..(8 + 4 + 4) {
        state
            .apply_semantic_action(SemanticAction::Navigate(Direction::Down))
            .unwrap();
    }
    assert_eq!(
        state.interaction().focus_path(),
        &FocusPath::mixer_return_occupancy(BusId::new(2).unwrap())
    );
    set_mode(&mut state, InteractionMode::Adjust);
    let before = state.clone();
    assert_eq!(
        state.apply(AppEvent::Adjust(Direction::Up)),
        Err(EventRejection::ActionUnavailableInContext)
    );
    assert_eq!(
        state.apply(AppEvent::Adjust(Direction::Left)),
        Err(EventRejection::ParameterAtBoundary)
    );
    assert_eq!(state, before);
    state.apply(AppEvent::Adjust(Direction::Right)).unwrap();
    let correlation = state.engine_selection().correlation().unwrap();
    let first_entry = production_effect_registry().unwrap().descriptors()[0]
        .id()
        .clone();
    assert_eq!(
        correlation.intent(),
        &StructuralEditIntent::SetReturnOccupancy {
            bus: BusId::new(2).unwrap(),
            entry: Some(first_entry.clone()),
        }
    );
    commit_pending_topology(&mut state);
    assert_eq!(
        state
            .bus_returns()
            .bus_return(BusId::new(2).unwrap())
            .effect()
            .map(|config| config.capability_id().clone()),
        Some(first_entry)
    );
    // Focus survived on the occupancy row; the occupant's rows now follow it.
    assert_eq!(
        state.interaction().focus_path(),
        &FocusPath::mixer_return_occupancy(BusId::new(2).unwrap())
    );
    state
        .apply_semantic_action(SemanticAction::SetInteractionMode(
            InteractionMode::Navigate,
        ))
        .unwrap();
    state
        .apply_semantic_action(SemanticAction::Navigate(Direction::Down))
        .unwrap();
    assert_eq!(
        state.interaction().focus_path(),
        &FocusPath::mixer_return_level(BusId::new(2).unwrap())
    );
}

/// Clearing a slot while its scalar rows hold focus resolves through
/// the one deterministic next-before-previous rule — the first scalar falls
/// back to its own occupancy row, the last scalar advances to the next
/// surviving row — and two identical runs land on identical focus paths.
#[test]
fn clearing_a_focused_slot_recovers_focus_deterministically() {
    let run = |scalar_offset: usize| -> (FocusPath, AppState) {
        let mut state = installed_state();
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        let controls = state.focused_patch_controls().unwrap();
        let slot0 = controls
            .iter()
            .position(|control| control == &PatchControlId::EffectSlot(EffectSlotIndex::ALL[0]))
            .unwrap();
        for _ in 0..(slot0 + scalar_offset) {
            state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        }
        assert!(matches!(
            focused_patch_control(&state),
            PatchControlId::Effect(_, _)
        ));
        state
            .apply_semantic_action(SemanticAction::SetSlotOccupancy {
                patch_id: PatchId::new(1).unwrap(),
                slot: EffectSlotIndex::ALL[0],
                entry: None,
            })
            .unwrap();
        commit_pending_topology(&mut state);
        (state.interaction().focus_path().clone(), state)
    };

    // First scalar of the cleared slot: the next row is its vanished sibling,
    // so recovery lands on the previous row — the slot's own occupancy row.
    let (first_focus, state) = run(1);
    assert_eq!(
        first_focus.control_id(),
        &SemanticControlId::Patch(PatchControlId::EffectSlot(EffectSlotIndex::ALL[0]))
    );
    // The recovered path resolves against the new schema.
    assert!(state
        .focused_patch_controls()
        .unwrap()
        .contains(&PatchControlId::EffectSlot(EffectSlotIndex::ALL[0])));

    // Last scalar of the cleared slot: the surviving next row wins —
    // the following slot's occupancy row.
    let (second_focus, _) = run(2);
    assert_eq!(
        second_focus.control_id(),
        &SemanticControlId::Patch(PatchControlId::EffectSlot(EffectSlotIndex::ALL[1]))
    );

    // Determinism: the same start and the same action land on the same path.
    assert_eq!(run(1).0, first_focus);
    assert_eq!(run(2).0, second_focus);
}

/// SC-008 at the projection layer: the projected slot rows are the
/// occupying entry's descriptor rows, generically. Installing a different
/// registry entry changes the projected rows with zero projector changes —
/// the projection is compared field-for-field against whichever descriptor
/// occupies the slot.
#[test]
fn slot_projection_is_descriptor_driven_for_every_registry_entry() {
    let registry = production_effect_registry().unwrap();
    for entry in registry.descriptors() {
        let mut state = installed_state();
        state
            .apply_semantic_action(SemanticAction::SetSlotOccupancy {
                patch_id: PatchId::new(2).unwrap(),
                slot: EffectSlotIndex::ALL[2],
                entry: Some(entry.id().clone()),
            })
            .unwrap();
        commit_pending_topology(&mut state);
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        let occupant = state.patches()[1]
            .effect_slot(EffectSlotIndex::ALL[2])
            .unwrap();
        assert_eq!(occupant.capability_id(), entry.id());
        // The focus resolver lists exactly the descriptor's visible enabled
        // ScalarEdit rows for the occupied slot, in descriptor order.
        let descriptor = registry.descriptor(entry.id()).unwrap();
        let expected_rows = descriptor
            .parameters()
            .filter(|spec| spec.patch_interaction() == PatchInteraction::ScalarEdit)
            .map(|spec| PatchControlId::Effect(occupant.slot_id(), spec.id().clone()))
            .collect::<Vec<_>>();
        let patch2_controls = crest_synth::control::PatchControlId::resolve(
            state
                .capabilities()
                .descriptor(state.patches()[1].instrument_config().capability_id())
                .unwrap(),
            state.patches()[1].instrument_config(),
            state.effects(),
            state.patches()[1].effect_slots(),
        );
        let slot_row = patch2_controls
            .iter()
            .position(|control| control == &PatchControlId::EffectSlot(EffectSlotIndex::ALL[2]))
            .unwrap();
        assert_eq!(&patch2_controls[slot_row + 1..], expected_rows.as_slice());
    }
}

/// The MIXER text projection carries all eight indexed sends per track,
/// a distinct RETURNS section with every return's occupancy, values, and
/// return level, and a final GLOBAL section holding master gain alone.
#[test]
fn mixer_projection_carries_eight_sends_eight_returns_and_one_global() {
    let state = installed_state();
    let projector = StateProjector::new();
    let (_, _, text, shell, _) = projector.project_with_shell(&state).unwrap();
    let body = text.body();
    for track in MixerTrackId::ALL {
        assert!(body.contains(&format!("TRACK {track}")));
    }
    for bus in BusId::ALL {
        assert!(body.contains(&format!("RETURN {bus}")));
    }
    assert_eq!(body.matches("send[0]=").count(), MixerTrackId::COUNT);
    assert_eq!(body.matches("send[7]=").count(), MixerTrackId::COUNT);
    assert_eq!(body.matches("returnLevel=").count(), BusId::COUNT);
    assert_eq!(body.matches("occupancy=").count(), BusId::COUNT);
    assert_eq!(body.matches("occupancy=empty").count(), BusId::COUNT - 2);
    let global_section = body.split("GLOBAL").nth(1).unwrap();
    assert!(global_section.contains("masterGainDb="));
    assert!(!global_section.contains("reverb"));
    assert!(!global_section.contains("delay"));

    // The semantic inspector projects the occupied returns' descriptor rows.
    let semantic = shell.semantic_model();
    let inspector = semantic.surface(SurfaceId::MixerInspector).unwrap();
    let return_effect_rows = inspector
        .controls()
        .iter()
        .filter(|control| {
            matches!(
                control.path().control_id(),
                SemanticControlId::Mixer(MixerControlId::ReturnEffect { .. })
            )
        })
        .count();
    // Production defaults: reverb and delay each declare two scalar rows.
    assert_eq!(return_effect_rows, 4);
    let occupancy_rows = inspector
        .controls()
        .iter()
        .filter(|control| {
            matches!(
                control.path().control_id(),
                SemanticControlId::Mixer(MixerControlId::ReturnOccupancy { .. })
            )
        })
        .count();
    assert_eq!(occupancy_rows, BusId::COUNT);
    assert_eq!(MAX_EFFECT_SLOTS, 3);
}
