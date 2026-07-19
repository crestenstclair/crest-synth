use crest_synth::adapter::eframe_text_window::EframeApplication;
use crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
use crest_synth::control::app_event::AppEvent;
use crest_synth::control::app_loop::AppLoop;
use crest_synth::control::app_state::AppState;
use crest_synth::control::event_record::{EventDirection, EventInput, EventOutcome, EventSource};
use crest_synth::control::state_projector::StateProjector;
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::patch_id::PatchId;
use crest_synth::mixer::channel_parameters::ChannelParameters;
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::real_time::audio_boundary::AudioBoundary;
use crest_synth::shell::app_window::{AppInputCallback, ProjectionCallback, TickCallback};
use crest_synth::synth::patch::Patch;
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use eframe::egui;
use eframe::App;
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn globals() -> GlobalParameters {
    GlobalParameters::new(-3.0, 0.7, 0.4, 0.25, 375.0, 0.35, 0.25)
        .expect("fixture global parameters are valid")
}

fn patch(id: u32, channel: u8, parameters: ChannelParameters) -> Patch {
    Patch::new(
        PatchId::new(id).expect("fixture PatchId is valid"),
        format!("Egui Fixture {id}"),
        SoundFontInstrument::new(0, id as u8 * 8, false).expect("fixture instrument is valid"),
        MidiChannel::new(channel).expect("fixture channel is valid"),
        parameters,
    )
}

fn installed_state() -> AppState {
    let mut state = AppState::new(globals());
    state
        .apply(AppEvent::InstallPatches(vec![
            patch(
                1,
                0,
                ChannelParameters::new(-12.0, -0.4, 0.2, 0.1)
                    .expect("fixture parameters are valid"),
            ),
            patch(
                2,
                1,
                ChannelParameters::new(-6.0, 0.35, 0.4, 0.3).expect("fixture parameters are valid"),
            ),
        ]))
        .expect("fixture installation is accepted");
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

fn raw_input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1_400.0, 140.0),
        )),
        events,
        ..Default::default()
    }
}

fn find_text_shape<'a>(
    shape: &'a egui::Shape,
    expected: &str,
) -> Option<&'a egui::epaint::TextShape> {
    match shape {
        egui::Shape::Text(text) if text.galley.job.text == expected => Some(text),
        egui::Shape::Vec(children) => children
            .iter()
            .find_map(|child| find_text_shape(child, expected)),
        _ => None,
    }
}

fn painted_projection<'a>(
    output: &'a egui::FullOutput,
    expected: &str,
) -> Option<(egui::Rect, &'a egui::epaint::TextShape)> {
    output.shapes.iter().find_map(|clipped| {
        find_text_shape(&clipped.shape, expected).map(|text| (clipped.clip_rect, text))
    })
}

fn tree_value(
    app_loop: &AppLoop<crest_synth::adapter::lock_free_audio_boundary::LockFreeControlHandle<()>>,
) -> Value {
    serde_json::from_str(app_loop.current_state_tree().json()).expect("StateTree is valid JSON")
}

#[test]
fn real_egui_frames_dispatch_into_app_loop_and_render_the_accepted_projection() {
    let state = installed_state();
    let projector = StateProjector::new();
    let initial = projector
        .parameter_snapshot(&state)
        .expect("fixture parameters project");
    let boundary = LockFreeAudioBoundary::<()>::new(16, initial);
    let (control, _audio) = boundary.into_handles();
    let app_loop = AppLoop::new(state, projector, control).expect("fixture AppLoop initializes");
    let before = tree_value(&app_loop);
    let shared = Rc::new(RefCell::new(app_loop));
    let tick_count = Rc::new(Cell::new(0_u32));

    let input_loop = Rc::clone(&shared);
    let on_input: AppInputCallback = Box::new(move |event| {
        input_loop
            .borrow_mut()
            .dispatch(event)
            .expect("egui fixture event is accepted");
    });
    let projection_loop = Rc::clone(&shared);
    let projection: ProjectionCallback = Box::new(move || projection_loop.borrow().current_text());
    let ticks = Rc::clone(&tick_count);
    let on_tick: TickCallback = Box::new(move |_elapsed| {
        ticks.set(ticks.get() + 1);
    });

    let mut application = EframeApplication::new(on_input, projection, on_tick);
    let context = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();

    let mut events = vec![key_event(egui::Key::D), key_event(egui::Key::D)];
    events.extend((0..6).map(|_| key_event(egui::Key::S)));
    events.push(key_event(egui::Key::K));
    events.push(key_event(egui::Key::D));

    context.begin_pass(raw_input(events));
    application.update(&context, &mut frame);
    let _first_output = context.end_pass();

    context.begin_pass(raw_input(Vec::new()));
    application.update(&context, &mut frame);
    let output = context.end_pass();

    let app_loop = shared.borrow();
    let after_tree = app_loop.current_state_tree();
    let after = tree_value(&app_loop);
    let text = app_loop.current_text();
    let records = app_loop.event_log();

    assert_eq!(after["selection"]["section"], "Global");
    assert_eq!(after["selection"]["parameterIndex"], 6);
    assert_eq!(after["patches"], before["patches"]);
    for parameter in [
        "masterGainDb",
        "reverbRoomSize",
        "reverbDamping",
        "reverbReturn",
        "delayMilliseconds",
        "delayFeedback",
    ] {
        assert_eq!(
            after["global"][parameter], before["global"][parameter],
            "{parameter}"
        );
    }
    let expected_delay_return = before["global"]["delayReturn"]
        .as_f64()
        .expect("baseline delayReturn is numeric")
        + 0.01;
    let observed_delay_return = after["global"]["delayReturn"]
        .as_f64()
        .expect("accepted delayReturn is numeric");
    assert!((observed_delay_return - expected_delay_return).abs() < 1.0e-6);
    assert_eq!(
        after["parameters"]["global"]["delayReturn"],
        after["global"]["delayReturn"]
    );

    assert_eq!(text.state_hash(), after_tree.state_hash());
    assert_eq!(after["projection"]["body"].as_str(), Some(text.body()));
    assert_eq!(
        after["projection"]["selectedLine"].as_u64(),
        Some(text.selected_line() as u64)
    );
    let expected_line = format!("> delayReturn={}", after["global"]["delayReturn"]);
    assert_eq!(
        text.body().lines().nth(text.selected_line()),
        Some(expected_line.as_str())
    );

    assert_eq!(records.len(), 9);
    let last = records
        .records()
        .last()
        .expect("the adjustment has an EventRecord");
    assert_eq!(last.source(), EventSource::System);
    assert_eq!(last.outcome(), EventOutcome::Accepted);
    assert!(matches!(
        last.input(),
        EventInput::Adjust {
            direction: EventDirection::Right
        }
    ));
    assert_eq!(last.generation_after(), after_tree.generation());
    assert_eq!(last.parameter_generation(), after_tree.generation());
    assert_eq!(last.state_hash_after(), after_tree.state_hash());
    assert_eq!(last.projection_state_hash(), after_tree.state_hash());
    assert_eq!(last.selected_line(), text.selected_line());

    let (clip_rect, text_shape) = painted_projection(&output, text.body())
        .expect("next frame paints the exact accepted projection body");
    let selected_row = text_shape
        .galley
        .rows
        .get(text.selected_line())
        .expect("the selected projection line has a painted row");
    let selected_rect = selected_row.rect().translate(text_shape.pos.to_vec2());
    assert!(
        clip_rect.contains(selected_rect.center()),
        "the exact selected line must be the scroll target"
    );
    assert_eq!(tick_count.get(), 2);

    println!("CREST_ACCEPTANCE eframe_context passed");
}
