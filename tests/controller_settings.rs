use crest_synth::adapter::production_instruments::production_soundfont_capability;
use crest_synth::control::event_record::EventSource;
use crest_synth::control::{
    AppEvent, AppLoop, AppState, ControllerBindings, ControllerButton, ControllerDevice,
    ControllerEvent, ControllerFailure, ControllerPreferenceStatus, ControllerRole,
    ControllerSettingId, Direction, FocusPath, SemanticAction, StateProjector, SurfaceId,
};
use crest_synth::kernel::{MidiChannel, PatchId};
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::mixer::mixer_track_id::MixerTrackId;
use crest_synth::mixer::patch_output::PatchOutput;
use crest_synth::real_time::{AudioCommand, BoundaryFull, ControlAudioBoundary, ParameterSnapshot};
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::Patch;
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use std::sync::{Arc, Mutex};

fn state() -> AppState {
    let provider = production_soundfont_capability().unwrap();
    let config =
        create_soundfont_config(&provider, SoundFontInstrument::new(0, 0, false).unwrap()).unwrap();
    let mut state = AppState::new(
        provider.registry().unwrap(),
        GlobalParameters::new(0.0).unwrap(),
    );
    state
        .apply(AppEvent::InstallPatches(vec![Patch::new(
            PatchId::new(1).unwrap(),
            "Controller test".into(),
            config,
            MidiChannel::new(0).unwrap(),
            PatchOutput::to_track(MixerTrackId::default()),
        )]))
        .unwrap();
    state
}
fn controller(state: &mut AppState, event: ControllerEvent) {
    state.apply(AppEvent::Controller(event)).unwrap();
}
fn ready(state: &mut AppState) {
    controller(
        state,
        ControllerEvent::PreferencesLoaded { result: Ok(None) },
    );
    controller(
        state,
        ControllerEvent::DevicesChanged {
            devices: vec![ControllerDevice {
                id: 7,
                name: "Test gamepad".into(),
            }],
        },
    );
}
fn settings(state: &mut AppState) {
    state
        .apply_semantic_action(SemanticAction::OpenMidiSettings)
        .unwrap();
    state
        .apply_semantic_action(SemanticAction::Navigate(Direction::Right))
        .unwrap();
    assert_eq!(
        state.interaction().active_surface(),
        SurfaceId::ControllerSettings
    );
}

#[test]
fn mapping_swaps_keep_every_role_reachable_and_capture_return_is_two_distinct_edges() {
    let mut state = state();
    ready(&mut state);
    let origin = state.interaction().clone();
    let patches = state.patches().to_vec();
    let revision = state.engine_selection().active_graph_revision();
    settings(&mut state);
    state
        .apply_semantic_action(SemanticAction::Activate)
        .unwrap();
    assert_eq!(state.controller().capture(), Some(ControllerRole::Up));
    let before = state.clone();
    assert!(state
        .apply(AppEvent::Controller(ControllerEvent::ButtonCaptured {
            device_id: 99,
            button: ControllerButton::South
        }))
        .is_err());
    assert_eq!(state, before, "unconnected device must not bind");
    controller(
        &mut state,
        ControllerEvent::ButtonCaptured {
            device_id: 7,
            button: ControllerButton::South,
        },
    );
    assert_eq!(
        state.controller().bindings().button(ControllerRole::Up),
        ControllerButton::South
    );
    assert_eq!(
        state.controller().bindings().button(ControllerRole::Edit),
        ControllerButton::DPadUp
    );
    for role in ControllerRole::ALL {
        assert_eq!(
            state
                .controller()
                .bindings()
                .role(state.controller().bindings().button(role)),
            Some(role)
        );
    }
    assert_eq!(
        state.controller().preference_status(),
        ControllerPreferenceStatus::Saving
    );
    state
        .apply_semantic_action(SemanticAction::Activate)
        .unwrap();
    state
        .apply_semantic_action(SemanticAction::NavigatePage(Direction::Right))
        .unwrap();
    assert_eq!(state.controller().capture(), None);
    assert_eq!(
        state.interaction().active_surface(),
        SurfaceId::ControllerSettings
    );
    state
        .apply_semantic_action(SemanticAction::NavigatePage(Direction::Right))
        .unwrap();
    assert_eq!(state.interaction(), &origin);
    assert_eq!(state.patches(), patches);
    assert_eq!(state.engine_selection().active_graph_revision(), revision);
}

#[test]
fn stale_save_failure_and_disconnect_cannot_corrupt_current_mapping_or_capture() {
    let mut state = state();
    ready(&mut state);
    settings(&mut state);
    state
        .apply_semantic_action(SemanticAction::Activate)
        .unwrap();
    controller(
        &mut state,
        ControllerEvent::ButtonCaptured {
            device_id: 7,
            button: ControllerButton::West,
        },
    );
    let before = state.clone();
    assert!(state
        .apply(AppEvent::Controller(ControllerEvent::PreferencesSaved {
            bindings: ControllerBindings::default(),
            result: Err(ControllerFailure::PreferenceWrite)
        }))
        .is_err());
    assert_eq!(state, before);
    let bindings = state.controller().bindings().clone();
    controller(
        &mut state,
        ControllerEvent::PreferencesSaved {
            bindings: bindings.clone(),
            result: Err(ControllerFailure::PreferenceWrite),
        },
    );
    assert!(state.controller().ready());
    assert_eq!(state.controller().bindings(), &bindings);
    state
        .apply_semantic_action(SemanticAction::Activate)
        .unwrap();
    controller(
        &mut state,
        ControllerEvent::DevicesChanged { devices: vec![] },
    );
    assert_eq!(state.controller().capture(), None);
    let before = state.clone();
    assert!(state
        .apply(AppEvent::Controller(ControllerEvent::ButtonCaptured {
            device_id: 7,
            button: ControllerButton::North
        }))
        .is_err());
    assert_eq!(state, before);
}

#[test]
fn invalid_preferences_require_explicit_recovery_and_all_settings_focus_is_stable() {
    let mut state = state();
    controller(
        &mut state,
        ControllerEvent::PreferencesLoaded {
            result: Err(ControllerFailure::PreferenceDecode),
        },
    );
    assert!(!state.controller().ready());
    controller(
        &mut state,
        ControllerEvent::DevicesChanged {
            devices: vec![ControllerDevice {
                id: 7,
                name: "Test gamepad".into(),
            }],
        },
    );
    settings(&mut state);
    assert!(
        state
            .apply_semantic_action(SemanticAction::Activate)
            .is_err(),
        "invalid preferences require reset before binding"
    );
    for _ in ControllerRole::ALL {
        state
            .apply_semantic_action(SemanticAction::Navigate(Direction::Down))
            .unwrap();
    }
    assert_eq!(
        state.interaction().focus_path(),
        &FocusPath::controller_settings(state.context(), ControllerSettingId::ResetDefaults)
    );
    state
        .apply_semantic_action(SemanticAction::Activate)
        .unwrap();
    assert!(state.controller().ready());
    assert_eq!(
        state.controller().preference_status(),
        ControllerPreferenceStatus::Saving
    );
    assert_eq!(
        state.controller().bindings(),
        &ControllerBindings::default()
    );
}

#[derive(Default)]
struct Traffic {
    snapshots: usize,
    commands: usize,
}
struct Probe(Arc<Mutex<Traffic>>);
impl ControlAudioBoundary for Probe {
    fn push_command(&mut self, _: AudioCommand) -> Result<(), BoundaryFull> {
        self.0.lock().unwrap().commands += 1;
        Ok(())
    }
    fn publish_parameters(&mut self, _: ParameterSnapshot) {
        self.0.lock().unwrap().snapshots += 1;
    }
}

#[test]
fn production_loop_projects_configuration_without_session_changes_or_audio_publication() {
    let mut state = state();
    ready(&mut state);
    let traffic = Arc::new(Mutex::new(Traffic::default()));
    let mut app = AppLoop::new(state, StateProjector::new(), Probe(traffic.clone())).unwrap();
    let saved = app.capture_saved_session();
    let origin = app.current_semantic_model().focus_path().clone();
    let before = {
        let observed = traffic.lock().unwrap();
        (observed.snapshots, observed.commands)
    };
    app.dispatch_action(SemanticAction::OpenMidiSettings)
        .unwrap();
    app.dispatch_action(SemanticAction::Navigate(Direction::Right))
        .unwrap();
    for label in ["ready", "capture", "saved"] {
        if label == "capture" {
            app.dispatch_action(SemanticAction::Activate).unwrap();
        }
        if label == "saved" {
            app.dispatch_from(
                AppEvent::Controller(ControllerEvent::ButtonCaptured {
                    device_id: 7,
                    button: ControllerButton::East,
                }),
                EventSource::Controller,
            )
            .unwrap();
        }
        let model = app.current_semantic_model();
        assert_eq!(model.active_surface(), SurfaceId::ControllerSettings);
        assert_eq!(
            model
                .surfaces()
                .iter()
                .flat_map(|surface| surface.controls())
                .filter(|control| control.focused())
                .count(),
            1
        );
        assert!(app.current_patch_page().is_none());
        assert_eq!(
            app.current_graphical_shell().context_line().context_label(),
            "SETTINGS"
        );
        if let Some(directory) = std::env::var_os("CREST_CONTROLLER_FIXTURE_DIR") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                std::path::Path::new(&directory).join(format!("controller-{label}.json")),
                serde_json::to_string(&model).unwrap(),
            )
            .unwrap();
        }
    }
    app.dispatch_action(SemanticAction::NavigatePage(Direction::Right))
        .unwrap();
    assert_eq!(app.current_semantic_model().focus_path(), &origin);
    assert_eq!(app.capture_saved_session(), saved);
    assert_eq!(
        {
            let observed = traffic.lock().unwrap();
            (observed.snapshots, observed.commands)
        },
        before
    );
}
