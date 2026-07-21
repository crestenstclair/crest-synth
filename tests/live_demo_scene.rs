#[allow(dead_code)]
mod support;

use crest_synth::adapter::atomic_audio_observation::AtomicAudioObservation;
use crest_synth::adapter::hidef_soundfont_capability::HiDefSoundFontCapability;
use crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
use crest_synth::control::app_loop::AppLoop;
use crest_synth::control::app_state::AppState;
use crest_synth::control::event_log::EventLog;
use crest_synth::control::event_record::{EventOutcome, EventSource};
use crest_synth::control::state_projector::StateProjector;
use crest_synth::mixer::channel_parameters::ChannelParameters;
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::mixer::mix_engine::MixEngine;
use crest_synth::real_time::audio_boundary::AudioBoundary;
use crest_synth::real_time::audio_observation::AudioObservation;
use crest_synth::real_time::audio_renderer::AudioRenderer;
use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
use crest_synth::testing::automatic_midi_test::AutomaticMidiTest;
use crest_synth::testing::live_demo_runner::LiveDemoRunner;
use crest_synth::testing::live_demo_scene::LiveDemoScene;
use std::time::Duration;
use support::{
    globals, FixtureEffects, FixtureEngine, FixtureMidiSource, FRAME_COUNT, SAMPLE_RATE,
};

#[test]
fn live_demo_scene_uses_production_state_projection_render_and_observation_paths() {
    let global = globals();
    let initial = ParameterSnapshot::new(0, global, &[]).expect("initial parameters are valid");
    let boundary = LockFreeAudioBoundary::<()>::new(512, initial);
    let (control, audio) = boundary.into_handles();
    let event_log = EventLog::new(4096).expect("live test journal capacity is valid");
    let provider = HiDefSoundFontCapability::new().expect("fixture capability is valid");
    let mut app_loop = AppLoop::with_event_log(
        AppState::new(
            provider.registry().expect("fixture registry is valid"),
            global,
        ),
        StateProjector::new(),
        control,
        event_log,
    )
    .expect("initial application state projects");

    let mut engine = FixtureEngine;
    let mut automatic = AutomaticMidiTest::new(FixtureMidiSource::new());
    automatic
        .initialize(&provider, &mut engine, &mut app_loop)
        .expect("fixture initializes through AppLoop");
    let scene = LiveDemoScene::from_installed_state(&app_loop.current_state_tree())
        .expect("installed fixture produces a live scene");

    let expected_count = app_loop.current_state_tree().patch_count()
        * ChannelParameters::surface_descriptor().len()
        + GlobalParameters::surface_descriptor().len();
    assert_eq!(scene.expected_editable_parameters().len(), expected_count);
    assert_eq!(scene.minimum_parameter_dwell(), Duration::from_millis(500));
    for parameter in scene.expected_editable_parameters() {
        assert!(scene.steps().iter().any(|step| {
            step.requires_checkpoint() && step.editable_parameter() == Some(*parameter)
        }));
    }
    let rejection_index = scene
        .steps()
        .iter()
        .position(|step| step.expected_outcome() == EventOutcome::Rejected)
        .expect("scene includes a boundary rejection probe");
    assert_eq!(
        scene.steps()[rejection_index + 1].expected_outcome(),
        EventOutcome::Accepted
    );
    assert!(scene.steps()[rejection_index + 1].requires_checkpoint());

    let observation = AtomicAudioObservation::default();
    let (writer, reader) = observation.into_handles();
    let mut renderer =
        AudioRenderer::with_observation(audio, engine, MixEngine::new(FixtureEffects), writer);
    renderer
        .prepare(FRAME_COUNT, SAMPLE_RATE)
        .expect("production renderer prepares");
    let mut output = vec![0.0_f32; FRAME_COUNT * 2];
    let mut runner = LiveDemoRunner::start(scene.clone(), automatic, reader);

    let mut checkpoints = Vec::new();
    assert!(runner
        .advance(Duration::from_millis(100), &mut app_loop)
        .expect("first edit dispatches")
        .is_none());
    let records_after_dispatch = app_loop.event_log().total_observed();
    assert!(runner
        .advance(Duration::ZERO, &mut app_loop)
        .expect("stale audio remains pending")
        .is_none());
    assert_eq!(
        app_loop.event_log().total_observed(),
        records_after_dispatch
    );
    renderer.render(&mut output);

    let mut deterministic_elapsed = Duration::from_millis(100);
    let mut first_checkpoint_elapsed = None;
    for _ in 0..600 {
        let demo_records_before = app_loop
            .event_log()
            .records()
            .iter()
            .filter(|record| record.source() == EventSource::DemoScene)
            .count();
        let tick_elapsed = Duration::from_millis(100);
        deterministic_elapsed += tick_elapsed;
        if let Some(checkpoint) = runner
            .advance(tick_elapsed, &mut app_loop)
            .expect("live runner advances coherently")
        {
            assert!(checkpoint.agrees());
            first_checkpoint_elapsed.get_or_insert(deterministic_elapsed);
            checkpoints.push(checkpoint);
        }
        let demo_records_after = app_loop
            .event_log()
            .records()
            .iter()
            .filter(|record| record.source() == EventSource::DemoScene)
            .count();
        assert!(
            demo_records_after.saturating_sub(demo_records_before) <= 1,
            "one window tick dispatched more than one autonomous scene event"
        );

        renderer.render(&mut output);
        if runner.completed_report().is_some() {
            break;
        }
    }

    let report = runner
        .completed_report()
        .expect("bounded live scene completes");
    assert!(first_checkpoint_elapsed.unwrap() >= Duration::from_millis(600));
    assert!(report.complete(), "{}", report.summary());
    assert_eq!(report.checkpoints(), checkpoints);
    assert_eq!(report.coverage().expected().len(), expected_count);
    assert_eq!(report.coverage().exercised().len(), expected_count);
    assert!(report.coverage().missing().is_empty());
    assert!(report.coverage().unexpected().is_empty());
    assert!(report.coverage().duplicate_expected().is_empty());
    assert!(report
        .event_log()
        .records()
        .iter()
        .any(|record| record.outcome() == EventOutcome::Accepted));
    assert!(report
        .event_log()
        .records()
        .iter()
        .any(|record| record.outcome() == EventOutcome::Rejected));
    assert_eq!(report.event_log().dropped_records(), 0);
    assert_eq!(
        report.state_tree().generation(),
        report
            .event_log()
            .records()
            .last()
            .unwrap()
            .generation_after()
    );
    assert!(report.to_json().unwrap().contains("\"complete\":true"));
    let event_log_summary = serde_json::to_value(report.event_log_summary()).unwrap();
    assert_eq!(event_log_summary["lossless"], true);
    assert_eq!(
        event_log_summary["totalObserved"],
        report.event_log().total_observed()
    );
    assert_eq!(
        event_log_summary["generationAfter"],
        report.state_tree().generation()
    );

    let final_log = app_loop.event_log().to_json().unwrap();
    let final_tree = app_loop.current_state_tree().into_json();
    let final_projection = app_loop.current_text();
    for _ in 0..3 {
        assert!(runner
            .advance(Duration::from_secs(1), &mut app_loop)
            .unwrap()
            .is_none());
    }
    assert_eq!(app_loop.event_log().to_json().unwrap(), final_log);
    assert_eq!(app_loop.current_state_tree().json(), final_tree);
    assert_eq!(app_loop.current_text(), final_projection);

    println!("CREST_ACCEPTANCE live_demo_scene passed");
}

#[test]
fn early_close_uses_semantic_cleanup_without_success_report() {
    let global = globals();
    let initial = ParameterSnapshot::new(0, global, &[]).unwrap();
    let boundary = LockFreeAudioBoundary::<()>::new(64, initial);
    let (control, _audio) = boundary.into_handles();
    let provider = HiDefSoundFontCapability::new().unwrap();
    let mut app_loop = AppLoop::with_event_log(
        AppState::new(provider.registry().unwrap(), global),
        StateProjector::new(),
        control,
        EventLog::new(256).unwrap(),
    )
    .unwrap();
    let mut engine = FixtureEngine;
    let mut automatic = AutomaticMidiTest::new(FixtureMidiSource::new());
    automatic
        .initialize(&provider, &mut engine, &mut app_loop)
        .unwrap();
    let scene = LiveDemoScene::from_installed_state(&app_loop.current_state_tree()).unwrap();
    let cleanup_count = scene.patch_ids().len();
    let observation = AtomicAudioObservation::default();
    let (_writer, reader) = observation.into_handles();
    let mut runner = LiveDemoRunner::start(scene, automatic, reader);

    runner.cleanup_before_close(&mut app_loop).unwrap();

    assert!(runner.is_aborted());
    assert!(runner.completed_report().is_none());
    assert_eq!(
        app_loop
            .event_log()
            .records()
            .iter()
            .filter(|record| record.source() == EventSource::DemoScene)
            .count(),
        cleanup_count
    );
    assert!(runner
        .advance(Duration::from_secs(1), &mut app_loop)
        .unwrap()
        .is_none());
}
