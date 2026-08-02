use crest_synth::adapter::eframe_graphical_window::EframeGraphicalApplication;
use crest_synth::adapter::production_instruments::{
    production_capability_registry, production_soundfont_capability,
};
use crest_synth::control::app_event::AppEvent;
use crest_synth::control::app_loop::AppLoop;
use crest_synth::control::app_state::AppState;
use crest_synth::control::event_record::{EventInput, EventOutcome};
use crest_synth::control::state_projector::StateProjector;
use crest_synth::control::{
    GraphicalShellProjection, PatchControlId, SemanticControlId, SemanticControlValue,
    ShellSideRegion, SurfaceId, TopLevelContext,
};
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::patch_id::PatchId;
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::mixer::mix_engine::MixEngine;
use crest_synth::mixer::mixer_track_id::MixerTrackId;
use crest_synth::mixer::patch_output::{PatchOutput, PatchOutputParameter};
use crest_synth::real_time::audio_boundary::{BoundaryFull, ControlAudioBoundary};
use crest_synth::real_time::audio_command::AudioCommand;
use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
use crest_synth::real_time::{AudioObservationSnapshot, PatchAudioBlock};
use crest_synth::shell::app_window::{
    AppInputCallback, FrameObservationCallback, ProjectionCallback, TickCallback,
};
use crest_synth::shell::{ShellFrameObservation, ShellRegionId};
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::Patch;
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use eframe::egui;
use eframe::App;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct BoundaryObservations {
    parameters: Vec<ParameterSnapshot>,
    commands: Vec<AudioCommand>,
}

struct ProbeBoundary {
    observations: Arc<Mutex<BoundaryObservations>>,
}

impl ControlAudioBoundary for ProbeBoundary {
    fn push_command(&mut self, command: AudioCommand) -> Result<(), BoundaryFull> {
        self.observations.lock().unwrap().commands.push(command);
        Ok(())
    }

    fn publish_parameters(&mut self, parameters: ParameterSnapshot) {
        self.observations
            .lock()
            .unwrap()
            .parameters
            .push(parameters);
    }
}

fn globals() -> GlobalParameters {
    GlobalParameters::new(-3.0).unwrap()
}

fn installed_state() -> AppState {
    let provider = production_soundfont_capability().unwrap();
    let config =
        create_soundfont_config(&provider, SoundFontInstrument::new(0, 40, false).unwrap())
            .unwrap();
    let patch = Patch::new(
        PatchId::new(1).unwrap(),
        "Graphical Shell".to_owned(),
        config,
        MidiChannel::new(0).unwrap(),
        PatchOutput::new(MixerTrackId::new(0).unwrap(), -6.0).unwrap(),
    );
    let mut state = AppState::new(production_capability_registry().unwrap(), globals());
    state.apply(AppEvent::InstallPatches(vec![patch])).unwrap();
    state
}

fn key_event(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    }
}

fn key_release(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    }
}

fn raw_input(size: [f32; 2], events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(size[0], size[1]),
        )),
        predicted_dt: 0.0,
        events,
        ..Default::default()
    }
}

fn run_frame(
    application: &mut EframeGraphicalApplication,
    context: &egui::Context,
    frame: &mut eframe::Frame,
    size: [f32; 2],
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    context.begin_pass(raw_input(size, events));
    application.update(context, frame);
    context.end_pass()
}

fn shape_contains_text(shape: &egui::Shape, expected: &str) -> bool {
    match shape {
        egui::Shape::Text(text) => text.galley.job.text == expected,
        egui::Shape::Vec(children) => children
            .iter()
            .any(|child| shape_contains_text(child, expected)),
        _ => false,
    }
}

fn output_contains_text(output: &egui::FullOutput, expected: &str) -> bool {
    output
        .shapes
        .iter()
        .any(|shape| shape_contains_text(&shape.shape, expected))
}

fn assert_close_command(output: &egui::FullOutput, expected: usize) {
    let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, egui::ViewportCommand::Close))
            .count(),
        expected
    );
}

fn assert_rect(observation: &ShellFrameObservation, id: ShellRegionId, expected: [f32; 4]) {
    let rect = observation.region(id).rect();
    let actual = [rect.min_x(), rect.min_y(), rect.max_x(), rect.max_y()];
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= 0.01,
            "{id:?}: expected {expected}, got {actual}"
        );
    }
}

fn assert_frame(
    observation: &ShellFrameObservation,
    projection: &GraphicalShellProjection,
    output: &egui::FullOutput,
    viewport: [f32; 2],
) {
    assert_eq!(observation.viewport_width(), viewport[0]);
    assert_eq!(observation.viewport_height(), viewport[1]);
    assert_eq!(observation.generation(), projection.generation());
    assert_eq!(observation.state_hash(), projection.state_hash());
    assert_eq!(observation.context(), projection.context());
    assert!(
        observation.regions_are_non_overlapping(),
        "{:?}",
        observation.regions()
    );
    assert_eq!(
        observation
            .regions()
            .iter()
            .map(|region| region.id())
            .collect::<Vec<_>>(),
        ShellRegionId::surface_descriptor()
    );

    let workspace_bottom = viewport[1] - 64.0;
    let side_width = if viewport[0] == 1_920.0 { 420.0 } else { 320.0 };
    let main_width = viewport[0] - side_width;
    assert_rect(
        observation,
        ShellRegionId::ContextLine,
        [0.0, 0.0, viewport[0], 48.0],
    );
    assert_rect(
        observation,
        ShellRegionId::IdentityHeader,
        [0.0, 48.0, viewport[0], 120.0],
    );
    assert_rect(
        observation,
        ShellRegionId::MainWorkspace,
        [0.0, 120.0, main_width, workspace_bottom],
    );
    assert_rect(
        observation,
        ShellRegionId::PersistentSideRegion,
        [main_width, 120.0, viewport[0], workspace_bottom],
    );
    assert_rect(
        observation,
        ShellRegionId::Footer,
        [0.0, workspace_bottom, viewport[0], viewport[1]],
    );
    assert!(
        observation
            .region(ShellRegionId::PersistentSideRegion)
            .rect()
            .width()
            >= 320.0
    );

    for region in observation.regions() {
        assert!(output_contains_text(output, region.visible_label()));
    }
    assert!(!projection.workspace().diagnostic().body().is_empty());
}

#[test]
fn production_update_renders_both_contexts_at_both_reference_viewports() {
    let observations = Arc::new(Mutex::new(BoundaryObservations::default()));
    let app_loop = AppLoop::new(
        installed_state(),
        StateProjector::new(),
        ProbeBoundary {
            observations: Arc::clone(&observations),
        },
    )
    .unwrap();
    let initial_tree: serde_json::Value =
        serde_json::from_str(app_loop.current_state_tree().json()).unwrap();
    let shared = Rc::new(RefCell::new(app_loop));
    let rejections = Rc::new(RefCell::new(Vec::new()));
    let frames = Rc::new(RefCell::new(Vec::new()));

    let input_loop = Rc::clone(&shared);
    let input_rejections = Rc::clone(&rejections);
    let on_input: AppInputCallback = Box::new(move |event| {
        if let Err(rejection) = input_loop.borrow_mut().dispatch_action(event) {
            input_rejections.borrow_mut().push(rejection);
        }
    });
    let projection_loop = Rc::clone(&shared);
    let projection: ProjectionCallback =
        Box::new(move || projection_loop.borrow().current_graphical_shell());
    let on_tick: TickCallback = Box::new(|_| true);
    let observed_frames = Rc::clone(&frames);
    let on_frame: FrameObservationCallback = Box::new(move |observation| {
        observed_frames.borrow_mut().push(observation);
    });
    let mut application = EframeGraphicalApplication::new(on_input, projection, on_tick, on_frame);
    let context = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();

    let cases = [
        ([1_920.0, 1_080.0], None, TopLevelContext::Mixer),
        (
            [1_920.0, 1_080.0],
            Some(egui::Key::Num2),
            TopLevelContext::Patch,
        ),
        (
            [1_280.0, 800.0],
            Some(egui::Key::Num1),
            TopLevelContext::Mixer,
        ),
        (
            [1_280.0, 800.0],
            Some(egui::Key::Num2),
            TopLevelContext::Patch,
        ),
    ];

    for (viewport, key, expected_context) in cases {
        let before = frames.borrow().len();
        let events = key.map_or_else(Vec::new, |key| vec![key_event(key)]);
        let output = run_frame(&mut application, &context, &mut frame, viewport, events);
        assert_eq!(frames.borrow().len(), before + 1);
        let projection = shared.borrow().current_graphical_shell();
        assert_eq!(projection.context(), expected_context);
        assert_eq!(
            projection.workspace().side_region(),
            match expected_context {
                TopLevelContext::Patch => ShellSideRegion::Utility,
                TopLevelContext::Mixer => ShellSideRegion::Inspector,
            }
        );
        assert_frame(
            frames.borrow().last().unwrap(),
            &projection,
            &output,
            viewport,
        );
        match expected_context {
            TopLevelContext::Mixer => {
                for track_id in MixerTrackId::ALL {
                    assert!(
                        output_contains_text(&output, &track_id.to_string()),
                        "{track_id} must remain visible in the stable mixer index at {viewport:?}"
                    );
                }
                let main = projection
                    .semantic_model()
                    .surface(crest_synth::control::SurfaceId::MixerMain)
                    .unwrap();
                assert_eq!(
                    main.controls().len(),
                    MixerTrackId::COUNT * 4,
                    "the horizontal strip must retain every track's four main controls"
                );
                assert!(output_contains_text(&output, "METER 0.000"));
                assert!(output_contains_text(&output, "SELECTED T00"));
                assert!(output_contains_text(&output, "PATCH 01 · Graphical Shell"));
                let inspector = projection
                    .semantic_model()
                    .surface(SurfaceId::MixerInspector)
                    .unwrap();
                // Eight indexed sends, then each unoccupied return's
                // occupancy and level rows, then master gain alone.
                let bus_count = crest_synth::mixer::bus_id::BusId::COUNT;
                assert_eq!(
                    inspector.controls().len(),
                    bus_count + bus_count * 2 + GlobalParameters::surface_descriptor().len()
                );
                assert!(inspector.controls()[..bus_count].iter().all(|control| {
                    matches!(
                        control.path().control_id(),
                        SemanticControlId::Mixer(crest_synth::control::MixerControlId::Send { .. })
                    )
                }));
                assert!(inspector.controls()[bus_count..bus_count * 3]
                    .iter()
                    .all(|control| {
                        matches!(
                            control.path().control_id(),
                            SemanticControlId::Mixer(
                                crest_synth::control::MixerControlId::ReturnOccupancy { .. }
                                    | crest_synth::control::MixerControlId::ReturnLevel { .. }
                            )
                        )
                    }));
                assert!(inspector.controls()[bus_count * 3..]
                    .iter()
                    .all(|control| matches!(
                        control.path().control_id(),
                        SemanticControlId::Mixer(
                            crest_synth::control::MixerControlId::Global { .. }
                        )
                    )));
            }
            TopLevelContext::Patch => {
                assert!(output_contains_text(&output, "Trim Gain"));
                assert!(output_contains_text(&output, "Output Track"));
            }
        }
    }

    assert!(rejections.borrow().is_empty());
    assert_eq!(application.frame_observation_error(), None);
    let app_loop = shared.borrow();
    let final_tree: serde_json::Value =
        serde_json::from_str(app_loop.current_state_tree().json()).unwrap();
    for path in [
        "/patches",
        "/global",
        "/parameters/patches",
        "/parameters/global",
        "/parameters/graphRevision",
    ] {
        assert_eq!(
            final_tree.pointer(path),
            initial_tree.pointer(path),
            "{path}"
        );
    }
    assert_eq!(app_loop.event_log_ref().records().len(), 3);
    assert!(app_loop.event_log_ref().records().iter().all(|record| {
        record.outcome() == EventOutcome::Accepted
            && matches!(record.input(), EventInput::SelectContext { .. })
    }));
    let boundary = observations.lock().unwrap();
    assert!(boundary.commands.is_empty());
    let parameter_values = boundary
        .parameters
        .iter()
        .map(|snapshot| {
            let mut value = serde_json::to_value(snapshot).unwrap();
            value["generation"] = serde_json::Value::Null;
            value
        })
        .collect::<Vec<_>>();
    assert!(parameter_values.windows(2).all(|pair| pair[0] == pair[1]));

    println!("CREST_ACCEPTANCE graphical_application_shell passed");
}

#[test]
fn mixer_frame_reads_one_compatible_immutable_audio_observation() {
    let state = installed_state();
    let projector = StateProjector::new();
    let parameters = projector.parameter_snapshot(&state).unwrap();
    let shell = projector.project_with_shell(&state).unwrap().3;
    let state_before = projector.project(&state).unwrap().0.json().to_owned();

    let mut patch_audio = PatchAudioBlock::prepare(2).unwrap();
    patch_audio.begin_render(&parameters, 2).unwrap();
    let patch_id = parameters.patches()[0].patch_id().unwrap();
    patch_audio
        .stem_mut(0, patch_id)
        .unwrap()
        .copy_from_slice(&[0.5, 0.5, 0.5, 0.5]);
    let mut mixer = MixEngine::new();
    mixer.prepare(48_000.0, 2).unwrap();
    let mut output = [0.0_f32; 4];
    let mix = mixer.mix(&patch_audio, &parameters, &mut output);
    let track = MixerTrackId::new(0).unwrap();
    assert!(mix.track(track).rms() > 0.0);
    let observation = AudioObservationSnapshot::from_mix_with_graph_and_routing(
        1,
        1,
        2,
        parameters.generation(),
        parameters.graph_revision(),
        0,
        0,
        0,
        None,
        mix,
    );

    let observation_reads = Rc::new(Cell::new(0_u32));
    let reads = Rc::clone(&observation_reads);
    let mut application = EframeGraphicalApplication::new_with_audio_observation(
        Box::new(|_| panic!("passive meter render emitted input")),
        Box::new(move || shell.clone()),
        Box::new(move || {
            reads.set(reads.get() + 1);
            observation
        }),
        Box::new(|_| true),
        Box::new(|_| {}),
    );
    let context = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    let rendered = run_frame(
        &mut application,
        &context,
        &mut frame,
        [1_280.0, 800.0],
        Vec::new(),
    );

    assert_eq!(observation_reads.get(), 1);
    assert!(output_contains_text(
        &rendered,
        &format!("METER {:.3}", mix.track(track).rms())
    ));
    assert_eq!(projector.project(&state).unwrap().0.json(), state_before);
}

#[test]
fn patch_utility_controls_dispatch_through_the_same_semantic_callback() {
    let observations = Arc::new(Mutex::new(BoundaryObservations::default()));
    let app_loop = AppLoop::new(
        installed_state(),
        StateProjector::new(),
        ProbeBoundary {
            observations: Arc::clone(&observations),
        },
    )
    .unwrap();
    let before_parameters = *app_loop.current_parameters();
    let initial_output = app_loop.patches()[0].output();
    let shared = Rc::new(RefCell::new(app_loop));

    let input_loop = Rc::clone(&shared);
    let on_input: AppInputCallback = Box::new(move |event| {
        input_loop
            .borrow_mut()
            .dispatch_action(event)
            .expect("PATCH Utility keyboard action is reducer-accepted");
    });
    let projection_loop = Rc::clone(&shared);
    let projection: ProjectionCallback =
        Box::new(move || projection_loop.borrow().current_graphical_shell());
    let mut application =
        EframeGraphicalApplication::new(on_input, projection, Box::new(|_| true), Box::new(|_| {}));
    let context = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();

    run_frame(
        &mut application,
        &context,
        &mut frame,
        [1_280.0, 800.0],
        vec![key_event(egui::Key::Num2)],
    );
    let utility = run_frame(
        &mut application,
        &context,
        &mut frame,
        [1_280.0, 800.0],
        vec![key_event(egui::Key::D), key_release(egui::Key::D)],
    );
    assert!(output_contains_text(&utility, "Trim Gain"));
    assert!(output_contains_text(&utility, "Output Track"));
    assert_eq!(
        shared.borrow().current_semantic_model().active_surface(),
        SurfaceId::PatchUtility
    );

    let adjusted = run_frame(
        &mut application,
        &context,
        &mut frame,
        [1_280.0, 800.0],
        vec![
            key_event(egui::Key::K),
            key_event(egui::Key::D),
            key_release(egui::Key::D),
            key_release(egui::Key::K),
        ],
    );
    let app_loop = shared.borrow();
    let expected_trim = initial_output.trim_gain_db()
        + PatchOutputParameter::TrimGain
            .descriptor()
            .fine_step()
            .unwrap();
    assert!((app_loop.patches()[0].output().trim_gain_db() - expected_trim).abs() < 1.0e-6);
    assert_eq!(
        app_loop.patches()[0].output().track_id(),
        initial_output.track_id()
    );
    assert_eq!(
        app_loop.current_parameters().graph_revision(),
        before_parameters.graph_revision()
    );
    assert_eq!(
        app_loop.current_parameters().mixer_tracks(),
        before_parameters.mixer_tracks()
    );
    assert_eq!(
        app_loop.current_parameters().global(),
        before_parameters.global()
    );
    assert_eq!(
        app_loop.current_parameters().patches()[0].output(),
        app_loop.patches()[0].output()
    );
    let semantic = app_loop.current_semantic_model();
    let trim_control = &semantic
        .surface(SurfaceId::PatchUtility)
        .unwrap()
        .controls()[0];
    assert_eq!(
        trim_control.path().control_id(),
        &SemanticControlId::Patch(PatchControlId::Output(PatchOutputParameter::TrimGain))
    );
    assert_eq!(
        trim_control.value(),
        &SemanticControlValue::Scalar(expected_trim as f64)
    );
    assert!(output_contains_text(&adjusted, "Trim Gain"));
    assert!(app_loop.event_log_ref().records().iter().any(|record| {
        record.outcome() == EventOutcome::Accepted
            && matches!(
                record.input(),
                EventInput::Adjust {
                    direction: crest_synth::control::event_record::EventDirection::Right
                }
            )
    }));
    let observed_parameters = observations
        .lock()
        .unwrap()
        .parameters
        .last()
        .copied()
        .expect("the accepted trim edit publishes one parameter snapshot");
    assert_eq!(
        observed_parameters.patches()[0].output(),
        app_loop.current_parameters().patches()[0].output()
    );
    assert_eq!(
        observed_parameters.mixer_tracks(),
        app_loop.current_parameters().mixer_tracks()
    );
    assert_eq!(
        observed_parameters.graph_revision(),
        app_loop.current_parameters().graph_revision()
    );
    assert_eq!(
        observed_parameters.generation() + 1,
        app_loop.current_parameters().generation(),
        "the following release-K navigation event is audio-neutral"
    );
}

#[test]
fn false_tick_closes_once_without_projection_frame_or_repaint() {
    let shell = StateProjector::new()
        .project_with_shell(&installed_state())
        .unwrap()
        .3;
    let input_count = Rc::new(Cell::new(0));
    let projection_count = Rc::new(Cell::new(0));
    let tick_count = Rc::new(Cell::new(0));
    let frame_count = Rc::new(Cell::new(0));

    let inputs = Rc::clone(&input_count);
    let on_input: AppInputCallback = Box::new(move |_| inputs.set(inputs.get() + 1));
    let projections = Rc::clone(&projection_count);
    let projection: ProjectionCallback = Box::new(move || {
        projections.set(projections.get() + 1);
        shell.clone()
    });
    let ticks = Rc::clone(&tick_count);
    let on_tick: TickCallback = Box::new(move |_| {
        ticks.set(ticks.get() + 1);
        false
    });
    let frame_observations = Rc::clone(&frame_count);
    let on_frame: FrameObservationCallback = Box::new(move |_| {
        frame_observations.set(frame_observations.get() + 1);
    });
    let mut application = EframeGraphicalApplication::new(on_input, projection, on_tick, on_frame);
    let context = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();

    let first = run_frame(
        &mut application,
        &context,
        &mut frame,
        [1_280.0, 800.0],
        Vec::new(),
    );
    assert_close_command(&first, 1);
    let second = run_frame(
        &mut application,
        &context,
        &mut frame,
        [1_280.0, 800.0],
        vec![key_event(egui::Key::Num2)],
    );
    assert_close_command(&second, 0);
    assert_eq!(input_count.get(), 0);
    assert_eq!(projection_count.get(), 0);
    assert_eq!(tick_count.get(), 1);
    assert_eq!(frame_count.get(), 0);
    assert_eq!(
        first.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
        Duration::ZERO
    );
}
