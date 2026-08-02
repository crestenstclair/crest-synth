use crest_synth::adapter::braids_capability::{BraidsCapability, BRAIDS_CAPABILITY_ID};
use crest_synth::adapter::eframe_graphical_window::{
    install_authored_typeface, EframeGraphicalApplication,
};
use crest_synth::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID;
use crest_synth::adapter::production_effects::{
    production_chorus_config, production_effect_registry,
};
use crest_synth::adapter::production_instruments::{
    production_capability_registry, production_soundfont_capability,
};
use crest_synth::control::event_record::{EmittedEvent, EventInput, EventSource};
use crest_synth::control::{
    AppEvent, AppLoop, AppState, Direction, EngineSelectionFailure, FocusCapabilityId,
    InteractionMode, MixerControlId, PatchControlId, SemanticAction, SemanticControlId,
    SemanticResolver, SemanticSurfaceSummary, StateProjector, SurfaceId, TopLevelContext,
};
use crest_synth::kernel::{MidiChannel, PatchId};
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::mixer::mixer_track_id::MixerTrackId;
use crest_synth::mixer::patch_output::{PatchOutput, PatchOutputParameter};
use crest_synth::real_time::audio_boundary::{BoundaryFull, ControlAudioBoundary};
use crest_synth::real_time::{AudioCommand, GraphRevision, ParameterSnapshot};
use crest_synth::shell::app_window::{
    AppInputCallback, FrameObservationCallback, ProjectionCallback, TickCallback,
};
use crest_synth::shell::{ShellFrameObservation, ShellRegionId};
use crest_synth::synth::effect_slot_id::EffectSlotIndex;
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::{EffectSlotId, InstrumentConfig, Patch};
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use eframe::egui;
use eframe::App;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct BoundaryEvidence {
    parameters: Vec<ParameterSnapshot>,
    commands: Vec<AudioCommand>,
}

struct ProbeBoundary {
    evidence: Arc<Mutex<BoundaryEvidence>>,
}

impl ControlAudioBoundary for ProbeBoundary {
    fn push_command(&mut self, command: AudioCommand) -> Result<(), BoundaryFull> {
        self.evidence.lock().unwrap().commands.push(command);
        Ok(())
    }

    fn publish_parameters(&mut self, parameters: ParameterSnapshot) {
        self.evidence.lock().unwrap().parameters.push(parameters);
    }
}

fn globals() -> GlobalParameters {
    GlobalParameters::new(-3.0).unwrap()
}

fn soundfont_config() -> InstrumentConfig {
    create_soundfont_config(
        &production_soundfont_capability().unwrap(),
        SoundFontInstrument::new(0, 40, false).unwrap(),
    )
    .unwrap()
}

fn installed_state(soundfont_first: bool) -> AppState {
    let soundfont = soundfont_config();
    let braids = BraidsCapability::new().unwrap().default_config().unwrap();
    let configs = if soundfont_first {
        [soundfont, braids]
    } else {
        [braids, soundfont]
    };
    let patches = configs
        .into_iter()
        .enumerate()
        .map(|(index, config)| {
            let patch = Patch::new(
                PatchId::new(index as u32 + 1).unwrap(),
                format!("Semantic {}", index + 1),
                config,
                MidiChannel::new(index as u8).unwrap(),
                PatchOutput::new(MixerTrackId::new(index as u8).unwrap(), -5.0 - index as f32)
                    .unwrap(),
            );
            if index == 0 && soundfont_first {
                patch.with_effect_slot(
                    EffectSlotIndex::ALL[0],
                    production_chorus_config(EffectSlotId::new(1).unwrap()).unwrap(),
                )
            } else {
                patch
            }
        })
        .collect();
    let mut state = AppState::new_with_effects(
        production_capability_registry().unwrap(),
        production_effect_registry().unwrap(),
        globals(),
    );
    state.apply(AppEvent::InstallPatches(patches)).unwrap();
    state
}

fn semantic(state: &AppState) -> crest_synth::control::SemanticGraphicalViewModel {
    StateProjector::new()
        .project_with_shell(state)
        .unwrap()
        .3
        .semantic_model()
        .clone()
}

fn render(
    projection: crest_synth::control::GraphicalShellProjection,
    viewport: [f32; 2],
) -> ShellFrameObservation {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&observations);
    let on_input: AppInputCallback = Box::new(|_| panic!("passive render emitted input"));
    let projection_callback: ProjectionCallback = Box::new(move || projection.clone());
    let tick: TickCallback = Box::new(|_| true);
    let on_frame: FrameObservationCallback =
        Box::new(move |frame| observed.borrow_mut().push(frame));
    let mut application =
        EframeGraphicalApplication::new(on_input, projection_callback, tick, on_frame);
    let context = egui::Context::default();
    install_authored_typeface(&context)
        .expect("the authored typeface installs into the test context");
    let mut frame = eframe::Frame::_new_kittest();
    context.begin_pass(egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(viewport[0], viewport[1]),
        )),
        predicted_dt: 0.0,
        ..Default::default()
    });
    application.update(&context, &mut frame);
    let _ = context.end_pass();
    assert_eq!(application.frame_observation_error(), None);
    let result = observations.borrow().last().cloned().unwrap();
    result
}

#[test]
fn production_semantic_graphical_view_model_is_exact_passive_and_audio_neutral() {
    let mut state = installed_state(true);
    let initial = semantic(&state);
    assert_eq!(initial.context(), TopLevelContext::Mixer);
    assert_eq!(initial.active_surface(), SurfaceId::MixerMain);
    assert_eq!(initial.interaction_mode(), InteractionMode::Navigate);
    assert_eq!(initial.errors(), []);
    assert_eq!(initial.surfaces().len(), 2);
    assert!(initial.surface(SurfaceId::MixerMain).is_some());
    assert!(initial.surface(SurfaceId::MixerInspector).is_some());
    let inspector = initial.surface(SurfaceId::MixerInspector).unwrap();
    // Eight indexed sends for the selected track, each empty return's
    // occupancy and level rows, then master gain alone.
    let bus_count = crest_synth::mixer::bus_id::BusId::COUNT;
    assert_eq!(
        inspector.controls().len(),
        bus_count + bus_count * 2 + GlobalParameters::surface_descriptor().len()
    );
    for (index, bus) in crest_synth::mixer::bus_id::BusId::ALL
        .into_iter()
        .enumerate()
    {
        assert!(matches!(
            inspector.controls()[index].path().control_id(),
            SemanticControlId::Mixer(MixerControlId::Send {
                track_id,
                bus: control_bus,
            }) if *track_id == MixerTrackId::new(0).unwrap() && *control_bus == bus
        ));
    }
    for (pair, bus) in crest_synth::mixer::bus_id::BusId::ALL
        .into_iter()
        .enumerate()
    {
        let occupancy = &inspector.controls()[bus_count + pair * 2];
        let level = &inspector.controls()[bus_count + pair * 2 + 1];
        assert!(matches!(
            occupancy.path().control_id(),
            SemanticControlId::Mixer(MixerControlId::ReturnOccupancy { bus: control_bus })
                if *control_bus == bus
        ));
        assert!(matches!(
            level.path().control_id(),
            SemanticControlId::Mixer(MixerControlId::ReturnLevel { bus: control_bus })
                if *control_bus == bus
        ));
    }
    assert!(inspector.controls()[bus_count * 3..]
        .iter()
        .all(|control| matches!(
            control.path().control_id(),
            SemanticControlId::Mixer(MixerControlId::Global { .. })
        )));
    match inspector.summary() {
        SemanticSurfaceSummary::MixerInspector {
            focused_track,
            patch_count,
            routed_patches,
            ..
        } => {
            assert_eq!(*focused_track, MixerTrackId::new(0).unwrap());
            assert_eq!(*patch_count, 2);
            assert_eq!(routed_patches.len(), 1);
            assert_eq!(routed_patches[0].patch_id(), PatchId::new(1).unwrap());
            assert_eq!(routed_patches[0].patch_name(), "Semantic 1");
        }
        other => panic!("expected canonical Mixer Inspector summary, got {other:?}"),
    }

    let resolved_actions = SemanticResolver::new(&state).valid_actions();
    assert_eq!(initial.valid_actions(), resolved_actions);
    for valid in initial.valid_actions() {
        let mut candidate = state.clone();
        candidate
            .apply_semantic_action(valid.action().clone())
            .expect("every projected valid action is reducer-accepted");
    }

    state
        .apply_semantic_action(SemanticAction::SelectContext(TopLevelContext::Patch))
        .unwrap();
    let patch = semantic(&state);
    assert_eq!(patch.active_surface(), SurfaceId::PatchMain);
    assert_eq!(patch.surfaces().len(), 2);
    assert!(patch.surface(SurfaceId::PatchUtility).is_some());
    let patch_paths = patch
        .surfaces()
        .iter()
        .flat_map(|surface| surface.controls())
        .map(|control| control.path())
        .collect::<Vec<_>>();
    assert!(patch_paths.iter().any(|path| matches!(
        path.capability_id(),
        Some(FocusCapabilityId::Instrument(id)) if id.as_str() == HIDEF_CAPABILITY_ID
    )));
    assert!(patch_paths
        .iter()
        .any(|path| matches!(path.capability_id(), Some(FocusCapabilityId::Effect(_)))));

    let origin = patch.focus_path().clone();
    state
        .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::PatchUtility))
        .unwrap();
    let utility = semantic(&state);
    assert_eq!(utility.active_surface(), SurfaceId::PatchUtility);
    assert_eq!(utility.return_path().unwrap().origin(), &origin);
    let utility_controls = utility.surface(SurfaceId::PatchUtility).unwrap().controls();
    assert_eq!(utility_controls.len(), 2);
    assert_eq!(
        utility_controls
            .iter()
            .map(|control| control.path().control_id().clone())
            .collect::<Vec<_>>(),
        vec![
            SemanticControlId::Patch(PatchControlId::Output(PatchOutputParameter::TrimGain,)),
            SemanticControlId::Patch(PatchControlId::Output(PatchOutputParameter::OutputTrack,)),
        ]
    );
    state.apply_semantic_action(SemanticAction::Return).unwrap();
    assert_eq!(semantic(&state).focus_path(), &origin);
    assert!(semantic(&state).return_path().is_none());

    state
        .apply_semantic_action(SemanticAction::SelectContext(TopLevelContext::Mixer))
        .unwrap();
    let mixer_origin = semantic(&state).focus_path().clone();
    state
        .apply_semantic_action(SemanticAction::EnterSurface(SurfaceId::MixerInspector))
        .unwrap();
    assert_eq!(semantic(&state).active_surface(), SurfaceId::MixerInspector);
    state.apply_semantic_action(SemanticAction::Return).unwrap();
    assert_eq!(semantic(&state).focus_path(), &mixer_origin);

    state
        .apply_semantic_action(SemanticAction::SetInteractionMode(InteractionMode::Adjust))
        .unwrap();
    assert_eq!(semantic(&state).interaction_mode(), InteractionMode::Adjust);
    state
        .apply_semantic_action(SemanticAction::SetInteractionMode(
            InteractionMode::Navigate,
        ))
        .unwrap();
    let before_reserved = semantic(&state);
    assert!(state
        .apply_semantic_action(SemanticAction::SetInteractionMode(InteractionMode::Modal))
        .is_err());
    assert_eq!(semantic(&state), before_reserved);

    let mut braids_state = installed_state(false);
    braids_state
        .apply_semantic_action(SemanticAction::SelectContext(TopLevelContext::Patch))
        .unwrap();
    let braids_patch = semantic(&braids_state);
    assert!(braids_patch
        .surfaces()
        .iter()
        .flat_map(|surface| surface.controls())
        .any(|control| matches!(
            control.path().capability_id(),
            Some(FocusCapabilityId::Instrument(id)) if id.as_str() == BRAIDS_CAPABILITY_ID
        )));

    let evidence = Arc::new(Mutex::new(BoundaryEvidence::default()));
    let mut app_loop = AppLoop::new(
        installed_state(true),
        StateProjector::new(),
        ProbeBoundary {
            evidence: Arc::clone(&evidence),
        },
    )
    .unwrap();
    let publications_before = evidence.lock().unwrap().parameters.len();
    let result = app_loop
        .dispatch_action(SemanticAction::Navigate(Direction::Down))
        .unwrap();
    assert_eq!(
        result.accepted().generation(),
        app_loop.current_state_tree().generation()
    );
    assert_eq!(
        evidence.lock().unwrap().parameters.len(),
        publications_before
    );
    assert!(evidence.lock().unwrap().commands.is_empty());
    let record = app_loop.event_log_ref().records().last().unwrap();
    assert_eq!(record.source(), EventSource::Keyboard);
    assert!(matches!(record.input(), EventInput::Navigate { .. }));
    assert!(record
        .emitted_events()
        .iter()
        .all(|effect| !matches!(effect, EmittedEvent::ParameterSnapshotPublished { .. })));

    let shell = app_loop.current_graphical_shell();
    assert_eq!(
        shell.footer().action_hints().len(),
        shell.semantic_model().valid_actions().len()
    );
    let large = render(shell.clone(), [1920.0, 1080.0]);
    let compact = render(shell.clone(), [1280.0, 800.0]);
    assert_eq!(large.focus_path(), compact.focus_path());
    assert_eq!(large.active_surface(), compact.active_surface());
    assert_eq!(large.interaction_mode(), compact.interaction_mode());
    assert_eq!(large.return_path(), compact.return_path());
    assert_eq!(large.valid_actions(), compact.valid_actions());
    assert_eq!(large.generation(), compact.generation());
    assert_eq!(large.state_hash(), compact.state_hash());
    for observation in [&large, &compact] {
        assert!(observation.regions_are_non_overlapping());
        assert_eq!(observation.regions().len(), ShellRegionId::ALL.len());
        assert!(observation
            .regions()
            .iter()
            .all(|region| region.rect().is_finite_nonempty()));
    }

    let mut failed = installed_state(true);
    failed
        .apply(AppEvent::SelectContext(TopLevelContext::Patch))
        .unwrap();
    failed.apply(AppEvent::Adjust(Direction::Right)).unwrap();
    let failed_correlation = failed.engine_selection().correlation().unwrap().clone();
    failed
        .apply(AppEvent::EnginePreparationFailed {
            request_id: failed_correlation.request_id(),
            patch_id: failed_correlation.patch_id().unwrap(),
            intent: failed_correlation.intent().clone(),
            source_capability_id: failed_correlation.source_capability_id().unwrap().clone(),
            target_capability_id: failed_correlation.target_capability_id().unwrap().clone(),
            source_graph_revision: failed_correlation.source_graph_revision(),
            target_graph_revision: GraphRevision::new(2).unwrap(),
            failure: EngineSelectionFailure::AssetUnavailable,
        })
        .unwrap();
    let failed_model = semantic(&failed);
    assert_eq!(failed_model.status().kind().name(), "failed");
    assert_eq!(failed_model.errors().len(), 1);

    let mut recovering = installed_state(true);
    recovering
        .apply(AppEvent::SelectContext(TopLevelContext::Patch))
        .unwrap();
    let old_order = SemanticResolver::new(&recovering)
        .patch_main_paths(PatchId::new(1).unwrap())
        .unwrap();
    recovering
        .apply(AppEvent::Adjust(Direction::Right))
        .unwrap();
    while recovering
        .interaction()
        .focus_path()
        .capability_id()
        .is_none()
    {
        recovering
            .apply(AppEvent::Navigate(Direction::Down))
            .unwrap();
    }
    let removed = recovering.interaction().focus_path().clone();
    let correlation = recovering.engine_selection().correlation().unwrap().clone();
    let target_revision = GraphRevision::new(2).unwrap();
    recovering
        .apply(AppEvent::EnginePrepared {
            request_id: correlation.request_id(),
            patch_id: correlation.patch_id().unwrap(),
            intent: correlation.intent().clone(),
            source_capability_id: correlation.source_capability_id().unwrap().clone(),
            target_capability_id: correlation.target_capability_id().unwrap().clone(),
            source_graph_revision: correlation.source_graph_revision(),
            target_graph_revision: target_revision,
            candidate_config: BraidsCapability::new().unwrap().default_config().unwrap(),
        })
        .unwrap();
    let new_order = SemanticResolver::new(&recovering)
        .patch_main_paths(PatchId::new(1).unwrap())
        .unwrap();
    let expected = SemanticResolver::recover(&removed, &old_order, &new_order).unwrap();
    assert_ne!(expected, removed);
    assert_eq!(recovering.interaction().focus_path(), &expected);
    assert!(matches!(expected.control_id(), SemanticControlId::Patch(_)));
    recovering
        .apply(AppEvent::EngineActivationAcknowledged {
            request_id: correlation.request_id(),
            intent: correlation.intent().clone(),
            target_graph_revision: target_revision,
            retired_graph_revision: GraphRevision::INITIAL,
            collected: true,
        })
        .unwrap();
    let recovered = semantic(&recovering);
    assert_eq!(recovered.status().kind().name(), "ready");
    assert!(recovered.errors().is_empty());

    println!("CREST_ACCEPTANCE semantic_graphical_view_model passed");
}
