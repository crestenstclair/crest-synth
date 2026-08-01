mod support;

use crest_synth::adapter::braids_capability::BraidsCapability;
use crest_synth::adapter::production_effects::{
    production_chorus_config, production_effect_registry,
};
use crest_synth::adapter::production_instruments::production_capability_registry;
use crest_synth::control::{
    AppEvent, AppState, EngineSelectionFailure, GraphicalShellProjection, PatchControlId,
    PatchPageProjection, StateProjector, StateTree, SurfaceId, TopLevelContext,
};
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::patch_id::PatchId;
use crest_synth::mixer::mixer_track_id::MixerTrackId;
use crest_synth::mixer::patch_output::PatchOutput;
use crest_synth::real_time::GraphRevision;
use crest_synth::synth::effect_slot_id::EffectSlotIndex;
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::{
    CapabilityId, EffectCapabilityDescriptor, EffectCapabilityId, EffectCapabilityRegistry,
    EffectSlotId, InstrumentConfig, Patch, VoiceEnvelope,
};
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use crest_synth::testing::{
    BehavioralMutationCase, BehavioralMutationHarness, BehavioralMutationObservation,
    DemoCoverageGroup,
};
use serde_json::Value;
use std::collections::BTreeSet;

fn configured_state(first: InstrumentConfig, second: InstrumentConfig) -> AppState {
    let capabilities = production_capability_registry().unwrap();
    let chorus = production_chorus_config(EffectSlotId::new(1).unwrap()).unwrap();
    let production_effects = production_effect_registry().unwrap();
    let soundfont_id =
        CapabilityId::new(crest_synth::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID)
            .unwrap();
    let soundfont_descriptor = capabilities.descriptor(&soundfont_id).unwrap();
    let schema_effect = EffectCapabilityDescriptor::new(
        EffectCapabilityId::new("effect.schema-fixture").unwrap(),
        "Schema Fixture",
        "effect.schema-fixture",
        soundfont_descriptor.sections().to_vec(),
        soundfont_descriptor.asset_requirements().to_vec(),
    )
    .unwrap();
    let soundfont_provider =
        crest_synth::adapter::production_instruments::production_soundfont_capability().unwrap();
    let schema_source_config = create_soundfont_config(
        &soundfont_provider,
        SoundFontInstrument::new(128, 11, false).unwrap(),
    )
    .unwrap();
    let schema_effect_config = schema_effect
        .create_config(
            EffectSlotId::new(2).unwrap(),
            schema_source_config.values(),
            schema_source_config.asset_references(),
        )
        .unwrap();
    let effects = EffectCapabilityRegistry::new(vec![
        production_effects.descriptors()[0].clone(),
        schema_effect,
    ])
    .unwrap();
    let patches = [first, second]
        .into_iter()
        .enumerate()
        .map(|(index, config)| {
            let patch = Patch::new(
                PatchId::new(index as u32 + 1).unwrap(),
                format!("Schema {index}"),
                config,
                MidiChannel::new(index as u8).unwrap(),
                PatchOutput::new(MixerTrackId::new(index as u8).unwrap(), -3.0 - index as f32)
                    .unwrap(),
            )
            .with_envelope(VoiceEnvelope::new(12.0, 34.0, 0.56, 78.0).unwrap());
            if index == 0 {
                patch.with_effect_slot(EffectSlotIndex::ALL[0], chorus.clone())
            } else {
                patch.with_effect_slot(EffectSlotIndex::ALL[0], schema_effect_config.clone())
            }
        })
        .collect();
    // Occupy two returns so the canonical serialized `returns` section and
    // the live `parameters.returns` entries expose their complete leaf shape:
    // a scalar-bearing occupant on bus 0 and an asset-bearing one on bus 1.
    let mut returns = crest_synth::mixer::bus_return::BusReturnBank::default();
    returns
        .set_return_occupancy(
            &effects,
            crest_synth::mixer::bus_id::BusId::new(0).unwrap(),
            Some(&EffectCapabilityId::new("effect.chorus").unwrap()),
        )
        .unwrap();
    returns
        .set_return_occupancy(
            &effects,
            crest_synth::mixer::bus_id::BusId::new(1).unwrap(),
            Some(&EffectCapabilityId::new("effect.schema-fixture").unwrap()),
        )
        .unwrap();
    let mut state = AppState::new_with_effects(capabilities, effects, support::globals())
        .with_initial_returns(returns);
    state.apply(AppEvent::InstallPatches(patches)).unwrap();
    state
}

fn state_tree_with_first_config(
    first: InstrumentConfig,
    second: InstrumentConfig,
    context: TopLevelContext,
    request_engine: bool,
    request_preset: bool,
) -> StateTree {
    let mut state = configured_state(first, second);
    if context == TopLevelContext::Patch {
        state.apply(AppEvent::SelectContext(context)).unwrap();
    }
    assert!(!(request_engine && request_preset));
    if request_engine {
        state
            .apply(AppEvent::Adjust(crest_synth::control::Direction::Right))
            .unwrap();
    } else if request_preset {
        for _ in 0..PatchControlId::surface_descriptor().len() {
            state
                .apply(AppEvent::Navigate(crest_synth::control::Direction::Down))
                .unwrap();
        }
        state
            .apply(AppEvent::Adjust(crest_synth::control::Direction::Right))
            .unwrap();
    }
    StateProjector::new().project_with_tree(&state).unwrap().4
}

fn state_tree_after(
    first: InstrumentConfig,
    second: InstrumentConfig,
    mutate: impl FnOnce(&mut AppState),
) -> StateTree {
    let mut state = configured_state(first, second);
    mutate(&mut state);
    StateProjector::new().project_with_tree(&state).unwrap().4
}

fn navigate_down_until(
    state: &mut AppState,
    predicate: impl Fn(&crest_synth::control::FocusPath) -> bool,
) {
    for _ in 0..64 {
        if predicate(state.interaction().focus_path()) {
            return;
        }
        state
            .apply(AppEvent::Navigate(crest_synth::control::Direction::Down))
            .unwrap();
    }
    panic!("semantic schema fixture could not reach its requested stable path");
}

fn discover_leaves(value: &Value, prefix: &str, output: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            for (name, child) in object {
                let path = if prefix.is_empty() {
                    name.to_owned()
                } else {
                    format!("{prefix}.{name}")
                };
                discover_leaves(child, &path, output);
            }
        }
        Value::Array(array) => {
            for child in array {
                discover_leaves(child, &format!("{prefix}[]"), output);
            }
        }
        _ => {
            output.insert(prefix.to_owned());
        }
    }
}

fn assert_state_tree_leaf_surface_exact() -> BTreeSet<String> {
    let soundfont =
        crest_synth::adapter::production_instruments::production_soundfont_capability().unwrap();
    let soundfont_config = create_soundfont_config(
        &soundfont,
        SoundFontInstrument::new(128, 11, false).unwrap(),
    )
    .unwrap();
    let braids_config = BraidsCapability::new().unwrap().default_config().unwrap();
    let mut trees = vec![
        state_tree_with_first_config(
            soundfont_config.clone(),
            braids_config.clone(),
            TopLevelContext::Mixer,
            false,
            false,
        ),
        state_tree_with_first_config(
            soundfont_config.clone(),
            braids_config.clone(),
            TopLevelContext::Patch,
            false,
            false,
        ),
        state_tree_with_first_config(
            soundfont_config.clone(),
            braids_config.clone(),
            TopLevelContext::Patch,
            true,
            false,
        ),
        state_tree_with_first_config(
            soundfont_config.clone(),
            braids_config.clone(),
            TopLevelContext::Patch,
            false,
            true,
        ),
        state_tree_with_first_config(
            braids_config.clone(),
            soundfont_config.clone(),
            TopLevelContext::Patch,
            false,
            false,
        ),
    ];
    // Occupancy correlations expose the position-bearing intent leaves
    // (patchId/slot/bus/entry) of the shared structural lifecycle.
    trees.push(state_tree_after(
        soundfont_config.clone(),
        braids_config.clone(),
        |state| {
            state
                .apply(AppEvent::SetReturnOccupancy {
                    bus: crest_synth::mixer::bus_id::BusId::new(4).unwrap(),
                    entry: Some(EffectCapabilityId::new("effect.chorus").unwrap()),
                })
                .unwrap();
        },
    ));
    trees.push(state_tree_after(
        soundfont_config.clone(),
        braids_config.clone(),
        |state| {
            state
                .apply(AppEvent::SetSlotOccupancy {
                    patch_id: PatchId::new(1).unwrap(),
                    slot: crest_synth::synth::effect_slot_id::EffectSlotIndex::new(2).unwrap(),
                    entry: Some(EffectCapabilityId::new("effect.chorus").unwrap()),
                })
                .unwrap();
        },
    ));
    trees.push(state_tree_after(
        soundfont_config.clone(),
        braids_config.clone(),
        |state| {
            state
                .apply(AppEvent::Navigate(crest_synth::control::Direction::Right))
                .unwrap();
            state
                .apply(AppEvent::Navigate(crest_synth::control::Direction::Right))
                .unwrap();
        },
    ));
    trees.push(state_tree_after(
        soundfont_config.clone(),
        braids_config.clone(),
        |state| {
            state
                .apply(AppEvent::SelectContext(TopLevelContext::Patch))
                .unwrap();
            state
                .apply(AppEvent::EnterSurface(SurfaceId::PatchUtility))
                .unwrap();
        },
    ));
    trees.push(state_tree_after(
        soundfont_config.clone(),
        braids_config.clone(),
        |state| {
            state
                .apply(AppEvent::SelectContext(TopLevelContext::Patch))
                .unwrap();
            navigate_down_until(state, |path| path.capability_id().is_some());
            state
                .apply(AppEvent::EnterSurface(SurfaceId::PatchUtility))
                .unwrap();
        },
    ));
    trees.push(state_tree_after(
        soundfont_config.clone(),
        braids_config.clone(),
        |state| {
            state
                .apply(AppEvent::Navigate(crest_synth::control::Direction::Right))
                .unwrap();
            state
                .apply(AppEvent::Navigate(crest_synth::control::Direction::Right))
                .unwrap();
            state
                .apply(AppEvent::EnterSurface(SurfaceId::MixerInspector))
                .unwrap();
        },
    ));
    trees.push(state_tree_after(
        soundfont_config.clone(),
        braids_config.clone(),
        |state| {
            state
                .apply(AppEvent::EnterSurface(SurfaceId::MixerInspector))
                .unwrap();
            state
                .apply(AppEvent::Navigate(crest_synth::control::Direction::Down))
                .unwrap();
            state
                .apply(AppEvent::Navigate(crest_synth::control::Direction::Down))
                .unwrap();
        },
    ));
    trees.push(state_tree_after(soundfont_config, braids_config, |state| {
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        state
            .apply(AppEvent::Adjust(crest_synth::control::Direction::Right))
            .unwrap();
        let correlation = state.engine_selection().correlation().unwrap().clone();
        state
            .apply(AppEvent::EnginePreparationFailed {
                request_id: correlation.request_id(),
                patch_id: correlation.patch_id().unwrap(),
                intent: correlation.intent().clone(),
                source_capability_id: correlation.source_capability_id().unwrap().clone(),
                target_capability_id: correlation.target_capability_id().unwrap().clone(),
                source_graph_revision: correlation.source_graph_revision(),
                target_graph_revision: GraphRevision::new(2).unwrap(),
                failure: EngineSelectionFailure::AssetUnavailable,
            })
            .unwrap();
    }));
    trees.push(
        StateProjector::new()
            .project_with_tree(&AppState::new(
                production_capability_registry().unwrap(),
                support::globals(),
            ))
            .unwrap()
            .4,
    );
    let mut discovered = BTreeSet::new();
    for tree in trees {
        discover_leaves(
            &serde_json::from_str::<Value>(tree.json()).unwrap(),
            "",
            &mut discovered,
        );
    }
    let descriptor = StateTree::serialized_leaf_descriptor();
    let described = descriptor
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(descriptor.len(), described.len(), "duplicate typed leaf");
    assert_eq!(
        described,
        discovered,
        "described-only={:?}\ndiscovered-only={:?}",
        described.difference(&discovered).collect::<Vec<_>>(),
        discovered.difference(&described).collect::<Vec<_>>()
    );
    discovered
}

#[test]
fn typed_descriptors_and_discovered_serialized_leaves_are_bidirectionally_exact() {
    let discovered = assert_state_tree_leaf_surface_exact();
    assert_eq!(StateTree::SCHEMA_VERSION, 12);
    for leaf in GraphicalShellProjection::serialized_leaf_descriptor() {
        let tree_leaf = format!("graphicalShell.{leaf}");
        assert!(
            StateTree::serialized_leaf_descriptor().contains(&tree_leaf.as_str()),
            "StateTree is missing {tree_leaf}"
        );
    }
    assert!(StateTree::serialized_leaf_descriptor().contains(&"patchPage.focusedControlId"));
    assert!(StateTree::serialized_leaf_descriptor().contains(&"patchPage.envelope[].controlId"));
    assert!(StateTree::serialized_leaf_descriptor().contains(&"graphicalShell.generation"));
    assert!(StateTree::serialized_leaf_descriptor()
        .contains(&"graphicalShell.workspace.diagnostic.stateHash"));
    assert!(PatchPageProjection::serialized_leaf_descriptor().contains(&"focusedControlId"));
    assert!(PatchPageProjection::serialized_leaf_descriptor().contains(&"envelope[].controlId"));

    for control in PatchControlId::surface_descriptor()
        .iter()
        .chain(PatchControlId::utility_surface_descriptor())
    {
        let serialized = serde_json::to_string(control).unwrap();
        assert_eq!(serialized, format!("\"{}\"", control.as_str()));
        assert_eq!(
            serde_json::from_str::<PatchControlId>(&serialized).unwrap(),
            *control
        );
    }

    let described = StateTree::serialized_leaf_descriptor()
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    let mut missing_leaf = discovered.clone();
    assert!(missing_leaf.remove("patchPage.focusedControlId"));
    assert_eq!(described.difference(&missing_leaf).count(), 1);
    assert_eq!(missing_leaf.difference(&described).count(), 0);
    let mut unexpected_leaf = discovered;
    assert!(unexpected_leaf.insert("patchPage.unexpectedControlId".to_owned()));
    assert_eq!(described.difference(&unexpected_leaf).count(), 0);
    assert_eq!(unexpected_leaf.difference(&described).count(), 1);

    let soundfont =
        crest_synth::adapter::production_instruments::production_soundfont_capability().unwrap();
    let mut state = AppState::new(
        production_capability_registry().unwrap(),
        support::globals(),
    );
    state
        .apply(AppEvent::InstallPatches(vec![Patch::new(
            PatchId::new(1).unwrap(),
            "Focus schema".to_owned(),
            create_soundfont_config(&soundfont, SoundFontInstrument::new(0, 11, false).unwrap())
                .unwrap(),
            MidiChannel::new(0).unwrap(),
            PatchOutput::to_track(MixerTrackId::new(0).unwrap()),
        )]))
        .unwrap();
    state
        .apply(AppEvent::SelectContext(TopLevelContext::Patch))
        .unwrap();
    for (index, control) in PatchControlId::surface_descriptor().iter().enumerate() {
        let tree = StateProjector::new().project_with_tree(&state).unwrap().4;
        let value: Value = serde_json::from_str(tree.json()).unwrap();
        assert_eq!(
            value["interaction"]["activeFocus"]["controlId"]["id"],
            control.as_str().as_ref()
        );
        assert_eq!(
            value["patchPage"]["focusedControlId"],
            control.as_str().as_ref()
        );
        if index + 1 < PatchControlId::surface_descriptor().len() {
            state
                .apply(AppEvent::Navigate(crest_synth::control::Direction::Down))
                .unwrap();
        }
    }

    let run = support::run_demo();
    assert_eq!(run.report.schema_version(), 7);
    let serialized = run
        .report
        .coverage()
        .group(DemoCoverageGroup::SerializedProperties);
    let projections = run.report.coverage().group(DemoCoverageGroup::Projections);
    let expected = serialized
        .expected()
        .iter()
        .chain(projections.expected())
        .cloned()
        .collect::<BTreeSet<_>>();
    let exercised = serialized
        .exercised()
        .iter()
        .chain(projections.exercised())
        .cloned()
        .collect::<BTreeSet<_>>();

    assert!(!expected.is_empty());
    assert_eq!(expected, exercised);
    assert!(serialized.missing().is_empty());
    assert!(serialized.unexpected().is_empty());
    assert!(projections.missing().is_empty());
    assert!(projections.unexpected().is_empty());
    assert!(expected
        .iter()
        .any(|identifier| identifier.starts_with("property.stateTree.")));
    assert!(expected
        .iter()
        .any(|identifier| identifier.starts_with("property.eventRecord.")));
    assert!(expected
        .iter()
        .any(|identifier| identifier.starts_with("property.eventLog.")));
    assert!(expected
        .iter()
        .any(|identifier| identifier.starts_with("property.textProjection.")));

    let tree: Value = serde_json::from_str(run.report.final_state_tree().json()).unwrap();
    assert_eq!(
        tree["capabilities"]["descriptors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|descriptor| descriptor["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["instrument.soundfont.hidef", "instrument.braids"]
    );
    assert_eq!(
        tree["patches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|patch| patch["instrument"]["capabilityId"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["instrument.soundfont.hidef", "instrument.braids"]
    );
    assert!(run.report.audio_evidence().mixed_engine_stems_nonzero());
    assert!(run
        .report
        .audio_evidence()
        .mixed_engine_parameter_isolation());

    let harness = BehavioralMutationHarness::new();
    let healthy = harness.run(BehavioralMutationCase::OmittedStateTreeLeaf, false);
    let mutant = harness.run(BehavioralMutationCase::OmittedStateTreeLeaf, true);
    let (
        BehavioralMutationObservation::OmittedStateTreeLeaf(healthy),
        BehavioralMutationObservation::OmittedStateTreeLeaf(mutant),
    ) = (healthy.into_observation(), mutant.into_observation())
    else {
        panic!("the omitted-leaf case must retain its typed observation schema");
    };

    assert!(healthy.schema_surface_equal);
    assert!(healthy.required_leaf_count > 0);
    assert_eq!(healthy.missing_leaf_count, 0);
    assert_eq!(healthy.unexpected_leaf_count, 0);
    assert!(!mutant.schema_surface_equal);
    assert_eq!(mutant.required_leaf_count, healthy.required_leaf_count);
    assert_eq!(mutant.missing_leaf_count, 1);
    assert_eq!(mutant.unexpected_leaf_count, 0);

    println!("CREST_ACCEPTANCE schema_surface passed");
}
