//! Focused acceptance for reducer-owned Engine and Post FX option states.
//!
//! Every witness in this target drives canonical state through `AppState::apply`
//! or `apply_semantic_action`; the file intentionally contains no parallel
//! option, lifecycle, focus, or renderer model.

#[allow(dead_code)]
mod support;

use crest_synth::adapter::braids_capability::BraidsCapability;
use crest_synth::adapter::production_effects::{
    production_chorus_config, production_effect_registry,
};
use crest_synth::adapter::production_instruments::{
    production_capability_registry, production_soundfont_capability,
};
use crest_synth::control::{
    AppEvent, AppState, Direction, EngineSelectionFailure, EngineSelectionStatusKind,
    EventRejection, InteractionMode, ModalControlId, PatchControlId, PatchSubordinateSession,
    SemanticAction, SemanticControlId, SemanticResolver, StateProjector, StructuralEditIntent,
    SurfaceId, TopLevelContext,
};
use crest_synth::kernel::{MidiChannel, PatchId};
use crest_synth::mixer::mixer_track_id::MixerTrackId;
use crest_synth::mixer::patch_output::PatchOutput;
use crest_synth::real_time::GraphRevision;
use crest_synth::synth::effect_slot_id::EffectSlotIndex;
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::{
    CapabilityAvailability, CapabilityRegistry, EffectCapabilityRegistry, EffectSlotId,
    InstrumentConfig, Patch,
};
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use serde_json::Value;

const PATCH_ID: PatchId = match PatchId::new(1) {
    Ok(id) => id,
    Err(_) => panic!("static Patch id is valid"),
};

fn soundfont_config() -> InstrumentConfig {
    create_soundfont_config(
        &production_soundfont_capability().unwrap(),
        SoundFontInstrument::new(0, 40, false).unwrap(),
    )
    .unwrap()
}

fn patch_with_duplicate_effects() -> Patch {
    Patch::new(
        PATCH_ID,
        "Option Witness".to_owned(),
        soundfont_config(),
        MidiChannel::new(0).unwrap(),
        PatchOutput::to_track(MixerTrackId::new(0).unwrap()),
    )
    .with_effect_slot(
        EffectSlotIndex::ALL[0],
        production_chorus_config(EffectSlotId::new(1).unwrap()).unwrap(),
    )
    .with_effect_slot(
        EffectSlotIndex::ALL[1],
        production_chorus_config(EffectSlotId::new(2).unwrap()).unwrap(),
    )
}

fn state_with(
    capabilities: CapabilityRegistry,
    effects: EffectCapabilityRegistry,
    patch: Patch,
) -> AppState {
    let mut state = AppState::new_with_effects(capabilities, effects, support::globals());
    state.apply(AppEvent::InstallPatches(vec![patch])).unwrap();
    state
        .apply(AppEvent::SelectContext(TopLevelContext::Patch))
        .unwrap();
    state
}

fn installed_state() -> AppState {
    state_with(
        production_capability_registry().unwrap(),
        production_effect_registry().unwrap(),
        patch_with_duplicate_effects(),
    )
}

fn navigate_to(state: &mut AppState, control: &PatchControlId) {
    for _ in 0..16 {
        if state.interaction().patch_control_focus().as_ref() == Some(control) {
            return;
        }
        state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
    }
    panic!("canonical Patch order reaches {control:?}");
}

fn open_choice(state: &mut AppState) {
    state
        .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
        .unwrap();
    state.apply(AppEvent::Adjust(Direction::Up)).unwrap();
    assert_eq!(state.interaction().active_surface(), SurfaceId::PatchChoice);
}

fn choice_subject(state: &AppState) -> &crest_synth::control::PatchChoiceSubject {
    match state.interaction().subordinate_session().unwrap() {
        PatchSubordinateSession::Choice { subject, .. } => subject,
        other => panic!("expected Choice session, got {other:?}"),
    }
}

fn focused_choice_id(state: &AppState) -> &str {
    match state.interaction().focus_path().control_id() {
        SemanticControlId::Modal(ModalControlId::Choice(id)) => id,
        other => panic!("expected choice focus, got {other:?}"),
    }
}

fn project(state: &AppState) -> crest_synth::control::SemanticGraphicalViewModel {
    StateProjector::new()
        .project_with_shell(state)
        .unwrap()
        .3
        .semantic_model()
        .clone()
}

fn surface<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["surfaces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|surface| surface["id"] == id)
        .unwrap_or_else(|| panic!("serialized surface {id} is present"))
}

fn serialized_control<'a>(
    document: &'a Value,
    surface_id: &str,
    control_id: &SemanticControlId,
) -> &'a Value {
    let expected = serde_json::to_value(control_id).unwrap();
    surface(document, surface_id)["controls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|control| control["path"]["controlId"] == expected)
        .unwrap_or_else(|| panic!("serialized control {control_id:?} is present"))
}

fn unavailable_instrument_registry() -> CapabilityRegistry {
    let mut descriptors = production_capability_registry()
        .unwrap()
        .descriptors()
        .to_vec();
    descriptors[0] =
        descriptors[0]
            .clone()
            .with_availability(CapabilityAvailability::Unavailable {
                reason: "provider offline".to_owned(),
            });
    CapabilityRegistry::new(descriptors).unwrap()
}

fn unavailable_effect_registry() -> EffectCapabilityRegistry {
    let mut descriptors = production_effect_registry().unwrap().descriptors().to_vec();
    descriptors[0] =
        descriptors[0]
            .clone()
            .with_availability(CapabilityAvailability::Unavailable {
                reason: "asset unavailable".to_owned(),
            });
    EffectCapabilityRegistry::new(descriptors).unwrap()
}

#[test]
fn engine_and_every_post_fx_origin_open_with_exact_slot_identity() {
    let controls = [
        PatchControlId::Engine,
        PatchControlId::EffectSlot(EffectSlotIndex::ALL[0]),
        PatchControlId::EffectSlot(EffectSlotIndex::ALL[1]),
        PatchControlId::EffectSlot(EffectSlotIndex::ALL[2]),
    ];
    let mut subjects = Vec::new();
    for control in controls {
        let mut state = installed_state();
        navigate_to(&mut state, &control);
        let origin = state.interaction().focus_path().clone();
        open_choice(&mut state);
        let subject = choice_subject(&state);
        assert_eq!(subject.patch_id(), Some(PATCH_ID));
        assert_eq!(subject.control_id(), &control);
        assert_eq!(state.interaction().return_path().unwrap().origin(), &origin);
        subjects.push(subject.stable_id());
        state.apply(AppEvent::Return).unwrap();
        assert_eq!(state.interaction().focus_path(), &origin);
    }
    assert_eq!(subjects.len(), 4);
    assert_eq!(
        subjects
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
    assert_ne!(
        subjects[1], subjects[2],
        "duplicate effects retain slot identity"
    );
    println!("ENGINE_POST_FX_ENTRY_EXACT");
}

#[test]
fn registry_options_drive_order_availability_initial_focus_and_empty() {
    let installed = production_capability_registry().unwrap();
    let mut descriptors = installed.descriptors().to_vec();
    let unavailable_engine_id = descriptors[0].id().to_string();
    descriptors[0] =
        descriptors[0]
            .clone()
            .with_availability(CapabilityAvailability::Unavailable {
                reason: "provider offline".to_owned(),
            });
    let expected_engine_id = descriptors[1].id().to_string();
    let expected_engine_labels = descriptors
        .iter()
        .map(|descriptor| descriptor.label().to_owned())
        .collect::<Vec<_>>();
    let registry = CapabilityRegistry::new(descriptors).unwrap();
    let mut engine = state_with(
        registry,
        production_effect_registry().unwrap(),
        patch_with_duplicate_effects(),
    );
    open_choice(&mut engine);
    let source = SemanticResolver::new(&engine)
        .choice_source(choice_subject(&engine))
        .unwrap();
    assert_eq!(source.options().len(), expected_engine_labels.len());
    assert_eq!(
        source
            .options()
            .iter()
            .map(|option| option.label().to_owned())
            .collect::<Vec<_>>(),
        expected_engine_labels
    );
    assert_eq!(source.options()[0].id(), unavailable_engine_id);
    assert!(source.options()[0].is_current());
    assert!(!source.options()[0].is_enabled());
    assert_eq!(
        source.options()[0].availability().reason(),
        Some("provider offline")
    );
    assert_eq!(focused_choice_id(&engine), expected_engine_id);
    let model = project(&engine);
    let option_surface = model.surface(SurfaceId::PatchChoice).unwrap();
    assert_eq!(option_surface.controls().len(), source.options().len());
    assert!(!option_surface.controls()[0].focusable());
    assert!(option_surface.controls()[0].valid_actions().is_empty());

    let effects = production_effect_registry().unwrap();
    let mut effect_descriptors = effects.descriptors().to_vec();
    let unavailable_effect_id = effect_descriptors[0].id().to_string();
    effect_descriptors[0] =
        effect_descriptors[0]
            .clone()
            .with_availability(CapabilityAvailability::Unavailable {
                reason: "asset unavailable".to_owned(),
            });
    let effect_count = effect_descriptors.len();
    let mut post_fx = state_with(
        production_capability_registry().unwrap(),
        EffectCapabilityRegistry::new(effect_descriptors).unwrap(),
        patch_with_duplicate_effects(),
    );
    navigate_to(
        &mut post_fx,
        &PatchControlId::EffectSlot(EffectSlotIndex::ALL[0]),
    );
    open_choice(&mut post_fx);
    let source = SemanticResolver::new(&post_fx)
        .choice_source(choice_subject(&post_fx))
        .unwrap();
    assert_eq!(source.options().len(), effect_count + 1);
    assert_eq!(
        source.options()[0].id(),
        crest_synth::control::EMPTY_OCCUPANCY_CHOICE_ID
    );
    assert_eq!(source.options()[0].label(), "EMPTY");
    assert_eq!(source.options()[1].id(), unavailable_effect_id);
    assert!(source.options()[1].is_current());
    assert!(!source.options()[1].is_enabled());
    assert_eq!(
        focused_choice_id(&post_fx),
        crest_synth::control::EMPTY_OCCUPANCY_CHOICE_ID
    );

    let unavailable = CapabilityAvailability::Unavailable {
        reason: "disabled by fixture".to_owned(),
    };
    let all_unavailable = CapabilityRegistry::new(
        production_capability_registry()
            .unwrap()
            .descriptors()
            .iter()
            .cloned()
            .map(|descriptor| descriptor.with_availability(unavailable.clone()))
            .collect(),
    )
    .unwrap();
    let mut no_options = state_with(
        all_unavailable,
        production_effect_registry().unwrap(),
        patch_with_duplicate_effects(),
    );
    no_options
        .apply(AppEvent::SetInteractionMode(InteractionMode::Adjust))
        .unwrap();
    let before = no_options.clone();
    assert_eq!(
        no_options.apply(AppEvent::Adjust(Direction::Up)),
        Err(EventRejection::ActionUnavailableInContext)
    );
    assert_eq!(no_options, before);
    println!("ENGINE_POST_FX_REGISTRY_FOCUS_EXACT");
}

#[test]
fn engine_options_serialization_is_exact_stable_and_origin_anchored() {
    let registry = unavailable_instrument_registry();
    let expected = registry
        .descriptors()
        .iter()
        .map(|descriptor| {
            (
                descriptor.id().to_string(),
                descriptor.label().to_owned(),
                descriptor.availability().reason().map(str::to_owned),
            )
        })
        .collect::<Vec<_>>();
    let mut state = state_with(
        registry,
        production_effect_registry().unwrap(),
        patch_with_duplicate_effects(),
    );
    open_choice(&mut state);
    let first = serde_json::to_string(&project(&state)).unwrap();
    let second = serde_json::to_string(&project(&state)).unwrap();
    assert_eq!(
        first, second,
        "the exact Engine Options JSON is byte stable"
    );
    let document: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(document["activeSurface"], "patchChoice");
    assert_eq!(document["returnPath"]["enteredSurface"], "patchChoice");
    assert_eq!(
        document["returnPath"]["origin"]["patchId"],
        PATCH_ID.value()
    );
    assert_eq!(
        document["returnPath"]["origin"]["controlId"],
        serde_json::to_value(SemanticControlId::Patch(PatchControlId::Engine)).unwrap()
    );
    let options = surface(&document, "patchChoice");
    assert_eq!(options["summary"]["kind"], "patchChoice");
    assert_eq!(options["summary"]["patchPosition"], PATCH_ID.value());
    assert_eq!(options["summary"]["subject"]["patchId"], PATCH_ID.value());
    assert_eq!(options["summary"]["subject"]["controlId"], "patch.engine");
    let rows = options["controls"].as_array().unwrap();
    assert_eq!(rows.len(), expected.len());
    for (index, (id, label, unavailable_reason)) in expected.iter().enumerate() {
        assert_eq!(rows[index]["value"]["kind"], "identity");
        assert_eq!(rows[index]["value"]["value"].as_str(), Some(id.as_str()));
        assert_eq!(rows[index]["label"].as_str(), Some(label.as_str()));
        assert_eq!(
            rows[index]["availabilityLabel"],
            serde_json::to_value(unavailable_reason).unwrap()
        );
        assert_eq!(rows[index]["focused"], index == 1);
        assert_eq!(
            rows[index]["selectedLabel"],
            serde_json::to_value((index == 0).then_some("CURRENT")).unwrap()
        );
        assert_eq!(rows[index]["enabled"], index != 0);
        if index == 0 {
            assert!(rows[index]["validActions"].as_array().unwrap().is_empty());
        }
    }
    assert_eq!(document["focusPath"], rows[1]["path"]);
    println!("ENGINE_OPTIONS_SERIALIZATION_EXACT");
}

#[test]
fn post_fx_options_serialization_is_exact_for_every_slot_and_duplicate() {
    let effects = unavailable_effect_registry();
    let expected = std::iter::once((
        crest_synth::control::EMPTY_OCCUPANCY_CHOICE_ID.to_owned(),
        "EMPTY".to_owned(),
        None,
    ))
    .chain(effects.descriptors().iter().map(|descriptor| {
        (
            descriptor.id().to_string(),
            descriptor.label().to_owned(),
            descriptor.availability().reason().map(str::to_owned),
        )
    }))
    .collect::<Vec<_>>();
    let mut modal_ids = Vec::new();
    for slot in EffectSlotIndex::ALL {
        let mut state = state_with(
            production_capability_registry().unwrap(),
            effects.clone(),
            patch_with_duplicate_effects(),
        );
        navigate_to(&mut state, &PatchControlId::EffectSlot(slot));
        open_choice(&mut state);
        let first = serde_json::to_string(&project(&state)).unwrap();
        let second = serde_json::to_string(&project(&state)).unwrap();
        assert_eq!(first, second, "slot {} JSON is byte stable", slot.index());
        let document: Value = serde_json::from_str(&first).unwrap();
        let options = surface(&document, "patchChoice");
        assert_eq!(
            options["summary"]["subject"]["controlId"],
            format!("patch.effectSlot.{}", slot.index())
        );
        assert_eq!(
            document["returnPath"]["origin"]["controlId"],
            serde_json::to_value(SemanticControlId::Patch(PatchControlId::EffectSlot(slot)))
                .unwrap()
        );
        let rows = options["controls"].as_array().unwrap();
        assert_eq!(rows.len(), expected.len());
        for (index, (id, label, unavailable_reason)) in expected.iter().enumerate() {
            assert_eq!(rows[index]["value"]["value"].as_str(), Some(id.as_str()));
            assert_eq!(rows[index]["label"].as_str(), Some(label.as_str()));
            assert_eq!(
                rows[index]["availabilityLabel"],
                serde_json::to_value(unavailable_reason).unwrap()
            );
        }
        let current = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row["selectedLabel"] == "CURRENT")
            .collect::<Vec<_>>();
        assert_eq!(current.len(), 1);
        assert_eq!(
            current[0].0,
            if slot == EffectSlotIndex::ALL[2] {
                0
            } else {
                1
            }
        );
        assert_eq!(rows.iter().filter(|row| row["focused"] == true).count(), 1);
        modal_ids.push(document["focusPath"]["modalId"].clone());
    }
    assert_eq!(
        modal_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3
    );
    println!("POST_FX_OPTIONS_SERIALIZATION_EXACT");
}

#[test]
fn option_lifecycle_failure_and_repair_serialize_at_the_exact_origin() {
    let (mut failed_engine, active, requested) = choose_changed_engine();
    open_choice(&mut failed_engine);
    let correlation = failed_engine
        .engine_selection()
        .correlation()
        .unwrap()
        .clone();
    failed_engine
        .apply(AppEvent::EnginePreparationFailed {
            request_id: correlation.request_id(),
            patch_id: PATCH_ID,
            intent: correlation.intent().clone(),
            source_capability_id: correlation.source_capability_id().unwrap().clone(),
            target_capability_id: correlation.target_capability_id().unwrap().clone(),
            source_graph_revision: correlation.source_graph_revision(),
            target_graph_revision: GraphRevision::new(2).unwrap(),
            failure: EngineSelectionFailure::PreparationFailed,
        })
        .unwrap();
    let document = serialized_model_value(&failed_engine);
    let engine = serialized_control(
        &document,
        "patchMain",
        &SemanticControlId::Patch(PatchControlId::Engine),
    );
    assert_eq!(engine["status"]["kind"], "failed");
    assert_eq!(engine["error"]["code"]["failure"], "preparationFailed");
    assert_eq!(engine["value"]["value"], "HiDef SoundFont");
    assert_eq!(
        engine["requestedValue"]["value"],
        "Mutable Instruments Braids"
    );
    assert_eq!(failed_engine.patches()[0].instrument_config(), &active);
    assert_eq!(
        correlation.target_capability_id(),
        Some(requested.capability_id())
    );

    let mut failed_slot = installed_state();
    navigate_to(
        &mut failed_slot,
        &PatchControlId::EffectSlot(EffectSlotIndex::ALL[2]),
    );
    open_choice(&mut failed_slot);
    failed_slot
        .apply(AppEvent::Navigate(Direction::Down))
        .unwrap();
    failed_slot.apply(AppEvent::Activate).unwrap();
    open_choice(&mut failed_slot);
    let correlation = failed_slot
        .engine_selection()
        .correlation()
        .unwrap()
        .clone();
    failed_slot
        .apply(AppEvent::TopologyPreparationFailed {
            request_id: correlation.request_id(),
            intent: correlation.intent().clone(),
            source_graph_revision: correlation.source_graph_revision(),
            target_graph_revision: GraphRevision::new(2).unwrap(),
            failure: EngineSelectionFailure::AssetUnavailable,
        })
        .unwrap();
    let document = serialized_model_value(&failed_slot);
    let slot = serialized_control(
        &document,
        "patchMain",
        &SemanticControlId::Patch(PatchControlId::EffectSlot(EffectSlotIndex::ALL[2])),
    );
    assert_eq!(slot["status"]["kind"], "unavailable");
    assert_eq!(slot["error"]["code"]["failure"], "assetUnavailable");
    assert_eq!(slot["value"]["value"], "Empty");
    assert_eq!(slot["requestedValue"]["value"], "Chorus");
    assert_eq!(
        surface(&document, "patchChoice")["summary"]["subject"]["controlId"],
        "patch.effectSlot.2"
    );

    let mut repaired = installed_state();
    open_choice(&mut repaired);
    repaired
        .apply(AppEvent::SetPatchOverviewOriginEnabled {
            patch_id: PATCH_ID,
            control: PatchControlId::Engine,
            enabled: false,
        })
        .unwrap();
    let document = serialized_model_value(&repaired);
    assert_eq!(document["focusRepair"]["removedControlId"], "patch.engine");
    assert_eq!(
        document["focusRepair"]["replacementControlId"],
        "patch.effectSlot.0"
    );
    assert_eq!(
        document["returnPath"]["origin"]["controlId"],
        serde_json::to_value(SemanticControlId::Patch(PatchControlId::EffectSlot(
            EffectSlotIndex::ALL[0],
        )))
        .unwrap()
    );
    println!("OPTION_LIFECYCLE_REPAIR_SERIALIZATION_EXACT");
}

fn serialized_model_value(state: &AppState) -> Value {
    serde_json::to_value(project(state)).unwrap()
}

#[test]
fn choice_navigation_actions_choose_and_close_are_reducer_owned() {
    let mut current = installed_state();
    let active_config = current.patches()[0].instrument_config().clone();
    let origin = current.interaction().focus_path().clone();
    open_choice(&mut current);
    let current_id = focused_choice_id(&current).to_owned();
    let at_start = current.clone();
    assert_eq!(
        current.apply(AppEvent::Navigate(Direction::Up)),
        Err(EventRejection::ActionUnavailableInContext)
    );
    assert_eq!(current, at_start);
    for direction in [Direction::Left, Direction::Right] {
        let unchanged = current.clone();
        assert_eq!(
            current.apply(AppEvent::Navigate(direction)),
            Err(EventRejection::ActionUnavailableInContext)
        );
        assert_eq!(current, unchanged);
    }
    let model = project(&current);
    let focused = model.focused_control().unwrap();
    assert_eq!(focused.path(), current.interaction().focus_path());
    for action in [
        SemanticAction::Navigate(Direction::Up),
        SemanticAction::Navigate(Direction::Down),
        SemanticAction::Activate,
        SemanticAction::Return,
    ] {
        assert_eq!(
            focused
                .valid_actions()
                .iter()
                .any(|valid| valid.action() == &action),
            current.accepts_semantic_action(&action),
            "projected actions use the reducer counterfactual for {action:?}"
        );
    }
    current.apply(AppEvent::Activate).unwrap();
    assert_eq!(current.interaction().focus_path(), &origin);
    assert_eq!(current.patches()[0].instrument_config(), &active_config);
    assert!(current.engine_selection().correlation().is_none());
    assert_eq!(current_id, active_config.capability_id().to_string());

    let mut changed = installed_state();
    let changed_origin = changed.interaction().focus_path().clone();
    open_choice(&mut changed);
    changed.apply(AppEvent::Navigate(Direction::Down)).unwrap();
    let selected_id = focused_choice_id(&changed).to_owned();
    let outcome = changed.apply(AppEvent::Activate).unwrap();
    assert!(outcome.engine_selection_effect().is_some());
    assert_eq!(changed.interaction().focus_path(), &changed_origin);
    assert_eq!(changed.patches()[0].instrument_config(), &active_config);
    assert_eq!(
        changed
            .engine_selection()
            .correlation()
            .unwrap()
            .target_capability_id()
            .unwrap()
            .to_string(),
        selected_id
    );
    open_choice(&mut changed);
    let busy_model = project(&changed);
    let requested_row = busy_model
        .surface(SurfaceId::PatchChoice)
        .unwrap()
        .controls()
        .iter()
        .find(|control| {
            control.value()
                == &crest_synth::control::SemanticControlValue::Identity(selected_id.clone())
        })
        .unwrap();
    assert!(
        !requested_row
            .valid_actions()
            .iter()
            .any(|valid| valid.action() == &SemanticAction::Activate),
        "a changed option cannot advertise Activate while a structural request is busy"
    );
    changed.apply(AppEvent::Return).unwrap();

    let mut current_slot = installed_state();
    navigate_to(
        &mut current_slot,
        &PatchControlId::EffectSlot(EffectSlotIndex::ALL[0]),
    );
    let current_slot_origin = current_slot.interaction().focus_path().clone();
    let current_slot_occupancy = current_slot.patches()[0].effect_slots().to_vec();
    open_choice(&mut current_slot);
    assert_ne!(
        focused_choice_id(&current_slot),
        crest_synth::control::EMPTY_OCCUPANCY_CHOICE_ID
    );
    let no_op = current_slot.apply(AppEvent::Activate).unwrap();
    assert!(no_op.engine_selection_effect().is_none());
    assert_eq!(
        current_slot.interaction().focus_path(),
        &current_slot_origin
    );
    assert_eq!(
        current_slot.patches()[0].effect_slots(),
        current_slot_occupancy.as_slice()
    );
    assert!(current_slot.engine_selection().correlation().is_none());

    let mut close = installed_state();
    navigate_to(
        &mut close,
        &PatchControlId::EffectSlot(EffectSlotIndex::ALL[2]),
    );
    let close_origin = close.interaction().focus_path().clone();
    let acknowledged = close.patches()[0].effect_slots().to_vec();
    open_choice(&mut close);
    close.apply(AppEvent::Navigate(Direction::Down)).unwrap();
    close.apply(AppEvent::Return).unwrap();
    assert_eq!(close.interaction().focus_path(), &close_origin);
    assert_eq!(close.patches()[0].effect_slots(), acknowledged.as_slice());
    assert!(close.engine_selection().correlation().is_none());
    println!("ENGINE_POST_FX_CHOICE_ACTIONS_EXACT");
}

#[test]
fn choice_return_is_exact_or_reports_deterministic_repair() {
    let mut exact = installed_state();
    navigate_to(
        &mut exact,
        &PatchControlId::EffectSlot(EffectSlotIndex::ALL[1]),
    );
    let origin = exact.interaction().focus_path().clone();
    open_choice(&mut exact);
    exact.apply(AppEvent::Return).unwrap();
    assert_eq!(exact.interaction().focus_path(), &origin);
    assert!(exact.focus_repair_status().is_none());

    let mut repaired = installed_state();
    navigate_to(
        &mut repaired,
        &PatchControlId::EffectSlot(EffectSlotIndex::ALL[1]),
    );
    let removed = repaired.interaction().focus_path().clone();
    open_choice(&mut repaired);
    repaired
        .apply(AppEvent::SetPatchOverviewOriginEnabled {
            patch_id: PATCH_ID,
            control: PatchControlId::EffectSlot(EffectSlotIndex::ALL[1]),
            enabled: false,
        })
        .unwrap();
    let replacement = repaired
        .interaction()
        .return_path()
        .unwrap()
        .origin()
        .clone();
    assert_ne!(replacement, removed);
    let repair = repaired.focus_repair_status().unwrap();
    assert_eq!(repair.removed_origin(), &removed);
    assert_eq!(repair.replacement_origin(), &replacement);
    let model = project(&repaired);
    let projected = model.focus_repair().unwrap();
    assert_eq!(
        projected.removed_control_id(),
        &PatchControlId::EffectSlot(EffectSlotIndex::ALL[1])
    );
    assert_eq!(
        projected.replacement_control_id(),
        &PatchControlId::EffectSlot(EffectSlotIndex::ALL[2])
    );
    repaired.apply(AppEvent::Return).unwrap();
    assert_eq!(repaired.interaction().focus_path(), &replacement);

    let unchanged = repaired.clone();
    assert_eq!(
        repaired.apply(AppEvent::SetPatchOverviewOriginEnabled {
            patch_id: PATCH_ID,
            control: PatchControlId::EffectSlot(EffectSlotIndex::ALL[1]),
            enabled: false,
        }),
        Err(EventRejection::InvalidSelection)
    );
    assert_eq!(repaired, unchanged);
    println!("ENGINE_POST_FX_RETURN_REPAIR_EXACT");
}

fn advance_request(state: &mut AppState, lifecycle: EngineSelectionStatusKind) {
    let request_id = state.engine_selection().correlation().unwrap().request_id();
    state
        .apply(AppEvent::EngineSelectionLifecycleAdvanced {
            request_id,
            lifecycle,
        })
        .unwrap();
}

fn choose_changed_engine() -> (AppState, InstrumentConfig, InstrumentConfig) {
    let mut state = installed_state();
    let active = state.patches()[0].instrument_config().clone();
    open_choice(&mut state);
    state.apply(AppEvent::Navigate(Direction::Down)).unwrap();
    state.apply(AppEvent::Activate).unwrap();
    let requested = BraidsCapability::new().unwrap().default_config().unwrap();
    (state, active, requested)
}

fn assert_active_and_requested_engine(
    state: &AppState,
    active: &InstrumentConfig,
    requested: &InstrumentConfig,
) {
    assert_eq!(state.patches()[0].instrument_config(), active);
    let correlation = state.engine_selection().correlation().unwrap();
    assert_eq!(
        correlation.target_capability_id(),
        Some(requested.capability_id())
    );
    let model = project(state);
    let active_label = state
        .capabilities()
        .descriptor(active.capability_id())
        .unwrap()
        .label();
    let requested_label = state
        .capabilities()
        .descriptor(requested.capability_id())
        .unwrap()
        .label();
    let engine = model
        .surface(SurfaceId::PatchMain)
        .unwrap()
        .controls()
        .iter()
        .find(|control| {
            control.path().control_id() == &SemanticControlId::Patch(PatchControlId::Engine)
        })
        .unwrap();
    assert_eq!(
        engine.value(),
        &crest_synth::control::SemanticControlValue::Identity(active_label.to_owned())
    );
    assert_eq!(
        engine.requested_value(),
        Some(&crest_synth::control::SemanticControlValue::Identity(
            requested_label.to_owned()
        ))
    );
}

#[test]
fn engine_and_slot_lifecycles_retain_acknowledged_truth_until_matching_ack() {
    let (mut engine, active, requested) = choose_changed_engine();
    assert_eq!(
        engine.engine_selection().kind(),
        EngineSelectionStatusKind::Loading
    );
    assert_active_and_requested_engine(&engine, &active, &requested);
    for lifecycle in [
        EngineSelectionStatusKind::Validating,
        EngineSelectionStatusKind::Preparing,
    ] {
        advance_request(&mut engine, lifecycle);
        assert_eq!(engine.engine_selection().kind(), lifecycle);
        assert_active_and_requested_engine(&engine, &active, &requested);
    }
    let correlation = engine.engine_selection().correlation().unwrap().clone();
    let target_revision = GraphRevision::new(2).unwrap();
    engine
        .apply(AppEvent::EnginePrepared {
            request_id: correlation.request_id(),
            patch_id: PATCH_ID,
            intent: correlation.intent().clone(),
            source_capability_id: correlation.source_capability_id().unwrap().clone(),
            target_capability_id: correlation.target_capability_id().unwrap().clone(),
            source_graph_revision: correlation.source_graph_revision(),
            target_graph_revision: target_revision,
            candidate_config: requested.clone(),
            prepared_visualization: None,
        })
        .unwrap();
    assert_eq!(
        engine.engine_selection().kind(),
        EngineSelectionStatusKind::Activating
    );
    assert_active_and_requested_engine(&engine, &active, &requested);
    let stale_ack = engine.clone();
    assert_eq!(
        engine.apply(AppEvent::EngineActivationAcknowledged {
            request_id: correlation.request_id().checked_next().unwrap(),
            intent: correlation.intent().clone(),
            target_graph_revision: target_revision,
            retired_graph_revision: GraphRevision::INITIAL,
            collected: true,
        }),
        Err(EventRejection::StaleEngineSelection)
    );
    assert_eq!(engine, stale_ack);
    let mismatched_ack = engine.clone();
    assert_eq!(
        engine.apply(AppEvent::EngineActivationAcknowledged {
            request_id: correlation.request_id(),
            intent: correlation.intent().clone(),
            target_graph_revision: target_revision.checked_next().unwrap(),
            retired_graph_revision: GraphRevision::INITIAL,
            collected: true,
        }),
        Err(EventRejection::MismatchedEngineSelection)
    );
    assert_eq!(engine, mismatched_ack);
    let before_early_ack = engine.clone();
    assert_eq!(
        engine.apply(AppEvent::EngineActivationAcknowledged {
            request_id: correlation.request_id(),
            intent: correlation.intent().clone(),
            target_graph_revision: target_revision,
            retired_graph_revision: GraphRevision::INITIAL,
            collected: false,
        }),
        Err(EventRejection::MismatchedEngineSelection)
    );
    assert_eq!(engine, before_early_ack);
    engine
        .apply(AppEvent::EngineActivationAcknowledged {
            request_id: correlation.request_id(),
            intent: correlation.intent().clone(),
            target_graph_revision: target_revision,
            retired_graph_revision: GraphRevision::INITIAL,
            collected: true,
        })
        .unwrap();
    assert_eq!(
        engine.engine_selection().kind(),
        EngineSelectionStatusKind::Ready
    );
    assert_eq!(engine.patches()[0].instrument_config(), &requested);

    for failure in [
        EngineSelectionFailure::ProviderMismatch,
        EngineSelectionFailure::UnsupportedAssetFormat,
        EngineSelectionFailure::PreparationFailed,
        EngineSelectionFailure::GraphCapacityExceeded,
        EngineSelectionFailure::GraphIncompatible,
    ] {
        let (mut failed, active, requested) = choose_changed_engine();
        let correlation = failed.engine_selection().correlation().unwrap().clone();
        failed
            .apply(AppEvent::EnginePreparationFailed {
                request_id: correlation.request_id(),
                patch_id: PATCH_ID,
                intent: correlation.intent().clone(),
                source_capability_id: correlation.source_capability_id().unwrap().clone(),
                target_capability_id: correlation.target_capability_id().unwrap().clone(),
                source_graph_revision: correlation.source_graph_revision(),
                target_graph_revision: GraphRevision::new(2).unwrap(),
                failure,
            })
            .unwrap();
        assert_eq!(
            failed.engine_selection().kind(),
            EngineSelectionStatusKind::Failed
        );
        assert_eq!(failed.engine_selection().failure(), Some(failure));
        assert_active_and_requested_engine(&failed, &active, &requested);
    }
    let (mut unavailable, active, requested) = choose_changed_engine();
    let unavailable_correlation = unavailable
        .engine_selection()
        .correlation()
        .unwrap()
        .clone();
    unavailable
        .apply(AppEvent::EnginePreparationFailed {
            request_id: unavailable_correlation.request_id(),
            patch_id: PATCH_ID,
            intent: unavailable_correlation.intent().clone(),
            source_capability_id: unavailable_correlation
                .source_capability_id()
                .unwrap()
                .clone(),
            target_capability_id: unavailable_correlation
                .target_capability_id()
                .unwrap()
                .clone(),
            source_graph_revision: unavailable_correlation.source_graph_revision(),
            target_graph_revision: GraphRevision::new(2).unwrap(),
            failure: EngineSelectionFailure::AssetUnavailable,
        })
        .unwrap();
    assert_eq!(
        unavailable.engine_selection().kind(),
        EngineSelectionStatusKind::Unavailable
    );
    assert_active_and_requested_engine(&unavailable, &active, &requested);

    let mut slot = installed_state();
    navigate_to(
        &mut slot,
        &PatchControlId::EffectSlot(EffectSlotIndex::ALL[2]),
    );
    open_choice(&mut slot);
    slot.apply(AppEvent::Navigate(Direction::Down)).unwrap();
    slot.apply(AppEvent::Activate).unwrap();
    let intent = slot
        .engine_selection()
        .correlation()
        .unwrap()
        .intent()
        .clone();
    assert!(matches!(
        intent,
        StructuralEditIntent::SetSlotOccupancy {
            patch_id: PATCH_ID,
            slot,
            entry: Some(_),
        } if slot == EffectSlotIndex::ALL[2]
    ));
    assert!(slot.patches()[0]
        .effect_slot(EffectSlotIndex::ALL[2])
        .is_none());
    for lifecycle in [
        EngineSelectionStatusKind::Validating,
        EngineSelectionStatusKind::Preparing,
    ] {
        advance_request(&mut slot, lifecycle);
        assert!(slot.patches()[0]
            .effect_slot(EffectSlotIndex::ALL[2])
            .is_none());
    }
    let slot_correlation = slot.engine_selection().correlation().unwrap().clone();
    slot.apply(AppEvent::TopologyPrepared {
        request_id: slot_correlation.request_id(),
        intent: slot_correlation.intent().clone(),
        source_graph_revision: slot_correlation.source_graph_revision(),
        target_graph_revision: GraphRevision::new(2).unwrap(),
    })
    .unwrap();
    assert_eq!(
        slot.engine_selection().kind(),
        EngineSelectionStatusKind::Activating
    );
    assert!(slot.patches()[0]
        .effect_slot(EffectSlotIndex::ALL[2])
        .is_none());
    slot.apply(AppEvent::EngineActivationAcknowledged {
        request_id: slot_correlation.request_id(),
        intent: slot_correlation.intent().clone(),
        target_graph_revision: GraphRevision::new(2).unwrap(),
        retired_graph_revision: GraphRevision::INITIAL,
        collected: true,
    })
    .unwrap();
    assert!(slot.patches()[0]
        .effect_slot(EffectSlotIndex::ALL[2])
        .is_some());

    for failure in [
        EngineSelectionFailure::AssetUnavailable,
        EngineSelectionFailure::ProviderMismatch,
        EngineSelectionFailure::PreparationFailed,
        EngineSelectionFailure::GraphCapacityExceeded,
        EngineSelectionFailure::GraphIncompatible,
    ] {
        let mut failed_slot = installed_state();
        navigate_to(
            &mut failed_slot,
            &PatchControlId::EffectSlot(EffectSlotIndex::ALL[2]),
        );
        open_choice(&mut failed_slot);
        failed_slot
            .apply(AppEvent::Navigate(Direction::Down))
            .unwrap();
        failed_slot.apply(AppEvent::Activate).unwrap();
        let correlation = failed_slot
            .engine_selection()
            .correlation()
            .unwrap()
            .clone();
        failed_slot
            .apply(AppEvent::TopologyPreparationFailed {
                request_id: correlation.request_id(),
                intent: correlation.intent().clone(),
                source_graph_revision: correlation.source_graph_revision(),
                target_graph_revision: GraphRevision::new(2).unwrap(),
                failure,
            })
            .unwrap();
        assert_eq!(
            failed_slot.engine_selection().kind(),
            if failure == EngineSelectionFailure::AssetUnavailable {
                EngineSelectionStatusKind::Unavailable
            } else {
                EngineSelectionStatusKind::Failed
            }
        );
        assert_eq!(failed_slot.engine_selection().failure(), Some(failure));
        assert!(
            failed_slot.patches()[0]
                .effect_slot(EffectSlotIndex::ALL[2])
                .is_none(),
            "terminal slot outcome preserves acknowledged Empty for {failure:?}"
        );
        let model = project(&failed_slot);
        let row = model
            .surface(SurfaceId::PatchMain)
            .unwrap()
            .controls()
            .iter()
            .find(|control| {
                control.path().control_id()
                    == &SemanticControlId::Patch(PatchControlId::EffectSlot(
                        EffectSlotIndex::ALL[2],
                    ))
            })
            .unwrap();
        assert_eq!(
            row.value(),
            &crest_synth::control::SemanticControlValue::Identity("Empty".to_owned())
        );
        assert!(row.requested_value().is_some());
        assert_eq!(
            row.status().unwrap().kind(),
            failed_slot.engine_selection().kind()
        );
        assert_eq!(
            row.error().unwrap().code(),
            crest_synth::control::SemanticErrorCode::EngineSelection(failure)
        );
    }

    let mut clear = installed_state();
    navigate_to(
        &mut clear,
        &PatchControlId::EffectSlot(EffectSlotIndex::ALL[0]),
    );
    let duplicate_sibling = clear.patches()[0]
        .effect_slot(EffectSlotIndex::ALL[1])
        .unwrap()
        .clone();
    open_choice(&mut clear);
    clear.apply(AppEvent::Navigate(Direction::Up)).unwrap();
    assert_eq!(
        focused_choice_id(&clear),
        crest_synth::control::EMPTY_OCCUPANCY_CHOICE_ID
    );
    clear.apply(AppEvent::Activate).unwrap();
    let clear_correlation = clear.engine_selection().correlation().unwrap().clone();
    for lifecycle in [
        EngineSelectionStatusKind::Validating,
        EngineSelectionStatusKind::Preparing,
    ] {
        advance_request(&mut clear, lifecycle);
        assert!(clear.patches()[0]
            .effect_slot(EffectSlotIndex::ALL[0])
            .is_some());
    }
    clear
        .apply(AppEvent::TopologyPrepared {
            request_id: clear_correlation.request_id(),
            intent: clear_correlation.intent().clone(),
            source_graph_revision: clear_correlation.source_graph_revision(),
            target_graph_revision: GraphRevision::new(2).unwrap(),
        })
        .unwrap();
    assert!(clear.patches()[0]
        .effect_slot(EffectSlotIndex::ALL[0])
        .is_some());
    clear
        .apply(AppEvent::EngineActivationAcknowledged {
            request_id: clear_correlation.request_id(),
            intent: clear_correlation.intent().clone(),
            target_graph_revision: GraphRevision::new(2).unwrap(),
            retired_graph_revision: GraphRevision::INITIAL,
            collected: true,
        })
        .unwrap();
    assert!(clear.patches()[0]
        .effect_slot(EffectSlotIndex::ALL[0])
        .is_none());
    assert_eq!(
        clear.patches()[0]
            .effect_slot(EffectSlotIndex::ALL[1])
            .unwrap(),
        &duplicate_sibling,
        "duplicate-capability sibling stays independently correlated"
    );
    println!("ENGINE_POST_FX_LIFECYCLE_EXACT");
}
