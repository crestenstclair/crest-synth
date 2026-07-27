use crest_synth::adapter::braids_capability::BRAIDS_CAPABILITY_ID;
use crest_synth::adapter::eframe_text_window::EframeApplication;
use crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
use crest_synth::adapter::production_instruments::production_capability_registry;
use crest_synth::control::app_event::AppEvent;
use crest_synth::control::app_loop::AppLoop;
use crest_synth::control::app_state::AppState;
use crest_synth::control::event_record::{
    EmittedEvent, EventDirection, EventInput, EventOutcome, EventSource,
};
use crest_synth::control::state_projector::StateProjector;
use crest_synth::control::{
    EngineSelectionEffectKind, EngineSelectionRequestId, EngineSelectionStatusKind, PatchControlId,
    TopLevelContext,
};
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::patch_id::PatchId;
use crest_synth::mixer::channel_parameters::ChannelParameters;
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::real_time::audio_boundary::AudioBoundary;
use crest_synth::shell::app_window::{AppInputCallback, ProjectionCallback, TickCallback};
use crest_synth::synth::patch::Patch;
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::{VoiceEnvelope, VoiceEnvelopeParameter};
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use eframe::egui;
use eframe::App;
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

fn globals() -> GlobalParameters {
    GlobalParameters::new(-3.0, 0.7, 0.4, 0.25, 375.0, 0.35, 0.25)
        .expect("fixture global parameters are valid")
}

fn patch(id: u32, channel: u8, parameters: ChannelParameters) -> Patch {
    let provider = crest_synth::adapter::production_instruments::production_soundfont_capability()
        .expect("fixture capability is valid");
    Patch::new(
        PatchId::new(id).expect("fixture PatchId is valid"),
        format!("Egui Fixture {id}"),
        create_soundfont_config(
            &provider,
            SoundFontInstrument::new(0, id as u8 * 8, false).expect("fixture instrument is valid"),
        )
        .expect("fixture config matches the descriptor"),
        MidiChannel::new(channel).expect("fixture channel is valid"),
        parameters,
    )
    .with_envelope(VoiceEnvelope::new(500.0, 600.0, 0.5, 700.0).unwrap())
}

fn installed_state() -> AppState {
    let mut state = AppState::new(
        production_capability_registry().expect("fixture registry is valid"),
        globals(),
    );
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

fn key_release(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    }
}

fn raw_input(events: Vec<egui::Event>, time_seconds: f64) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1_400.0, 140.0),
        )),
        time: Some(time_seconds),
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
    app_loop: &AppLoop<crest_synth::adapter::lock_free_audio_boundary::LockFreeControlHandle>,
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
    let boundary = LockFreeAudioBoundary::new(16, initial);
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
        true
    });

    let mut application = EframeApplication::new(on_input, projection, on_tick);
    let context = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();

    let mut events = vec![key_event(egui::Key::D), key_event(egui::Key::D)];
    events.extend((0..6).map(|_| key_event(egui::Key::S)));
    events.push(key_event(egui::Key::K));
    events.push(key_event(egui::Key::D));
    events.push(key_release(egui::Key::D));
    events.push(key_release(egui::Key::K));

    context.begin_pass(raw_input(events, 0.0));
    application.update(&context, &mut frame);
    let _first_output = context.end_pass();

    context.begin_pass(raw_input(Vec::new(), 0.25));
    application.update(&context, &mut frame);
    let _edited_output = context.end_pass();

    let mut idle_output = None;
    for frame_index in 2..6 {
        context.begin_pass(raw_input(Vec::new(), f64::from(frame_index) * 0.25));
        application.update(&context, &mut frame);
        idle_output = Some(context.end_pass());
    }
    let idle_output = idle_output.expect("the idle egui fixture renders a steady frame");

    let (retained_mixer_body, retained_mixer_line, retained_mixer_selection) = {
        let app_loop = shared.borrow();
        (
            app_loop.current_text().body().to_owned(),
            app_loop.current_text().selected_line(),
            tree_value(&app_loop)["interaction"]["mixerSelection"].clone(),
        )
    };

    context.begin_pass(raw_input(vec![key_event(egui::Key::Num2)], 1.5));
    application.update(&context, &mut frame);
    let patch_output = context.end_pass();
    {
        let app_loop = shared.borrow();
        let patch_tree = tree_value(&app_loop);
        let patch_text = app_loop.current_text();
        let page = app_loop
            .current_patch_page()
            .expect("Digit2 produces the canonical PATCH page");
        let record = app_loop.event_log().records().last().cloned().unwrap();

        assert_eq!(patch_text.context(), TopLevelContext::Patch);
        assert!(patch_text.body().starts_with("PATCH | 1 MIXER | 2 PATCH"));
        assert_eq!(
            patch_text.state_hash(),
            app_loop.current_state_tree().state_hash()
        );
        assert_eq!(page.patch().id(), PatchId::new(1).unwrap());
        assert_eq!(page.state_hash(), patch_text.state_hash());
        assert_eq!(patch_tree["interaction"]["context"], "patch");
        assert_eq!(patch_tree["interaction"]["patchFocus"], 1);
        assert_eq!(
            patch_tree["interaction"]["mixerSelection"],
            retained_mixer_selection
        );
        assert_eq!(
            patch_tree["patchPage"]["stateHash"],
            patch_text.state_hash()
        );
        assert_eq!(patch_tree["projection"]["body"], patch_text.body());
        assert!(matches!(
            record.input(),
            EventInput::SelectContext {
                context: TopLevelContext::Patch
            }
        ));
        assert_eq!(record.state_hash_after(), patch_text.state_hash());

        let (clip_rect, text_shape) = painted_projection(&patch_output, patch_text.body())
            .expect("Digit2 frame paints the exact PATCH projection");
        let selected = text_shape.galley.rows[patch_text.selected_line()]
            .rect()
            .translate(text_shape.pos.to_vec2());
        assert!(clip_rect.contains(selected.center()));
    }

    let mut patch_frame_time = 1.6;
    for (index, parameter) in [
        VoiceEnvelopeParameter::AttackMilliseconds,
        VoiceEnvelopeParameter::DecayMilliseconds,
        VoiceEnvelopeParameter::Sustain,
        VoiceEnvelopeParameter::ReleaseMilliseconds,
    ]
    .into_iter()
    .enumerate()
    {
        context.begin_pass(raw_input(
            vec![key_event(egui::Key::S), key_release(egui::Key::S)],
            patch_frame_time,
        ));
        application.update(&context, &mut frame);
        let focused_output = context.end_pass();
        patch_frame_time += 0.1;

        let app_loop = shared.borrow();
        let text = app_loop.current_text();
        let page = app_loop.current_patch_page().unwrap();
        assert_eq!(
            page.focused_control_id(),
            PatchControlId::Envelope(parameter)
        );
        assert_eq!(text.selected_line(), index + 3);
        assert_eq!(
            text.body()
                .lines()
                .filter(|line| line.starts_with('>'))
                .count(),
            1
        );
        let (clip_rect, text_shape) = painted_projection(&focused_output, text.body())
            .expect("the next frame paints the reducer-selected PATCH row");
        let selected = text_shape.galley.rows[text.selected_line()]
            .rect()
            .translate(text_shape.pos.to_vec2());
        assert!(clip_rect.contains(selected.center()));
        drop(app_loop);

        if index == 0 {
            let baseline = shared.borrow().patches()[0]
                .envelope()
                .attack_milliseconds();
            context.begin_pass(raw_input(
                vec![
                    key_event(egui::Key::K),
                    key_event(egui::Key::D),
                    key_event(egui::Key::A),
                    key_event(egui::Key::W),
                    key_event(egui::Key::S),
                    key_release(egui::Key::K),
                ],
                patch_frame_time,
            ));
            application.update(&context, &mut frame);
            let adjusted_output = context.end_pass();
            patch_frame_time += 0.1;

            let app_loop = shared.borrow();
            let text = app_loop.current_text();
            assert_eq!(
                app_loop.patches()[0].envelope().attack_milliseconds(),
                baseline,
                "K+D/A/W/S must use the reducer's reversible fine/coarse steps"
            );
            assert_eq!(text.selected_line(), 3);
            let (clip_rect, text_shape) = painted_projection(&adjusted_output, text.body())
                .expect("the adjustment frame paints the next canonical projection");
            let selected = text_shape.galley.rows[text.selected_line()]
                .rect()
                .translate(text_shape.pos.to_vec2());
            assert!(clip_rect.contains(selected.center()));
        }
    }

    context.begin_pass(raw_input(
        (0..4)
            .flat_map(|_| [key_event(egui::Key::W), key_release(egui::Key::W)])
            .collect(),
        patch_frame_time,
    ));
    application.update(&context, &mut frame);
    let engine_focus_output = context.end_pass();
    patch_frame_time += 0.1;
    {
        let app_loop = shared.borrow();
        let text = app_loop.current_text();
        let page = app_loop.current_patch_page().unwrap();
        assert_eq!(page.focused_control_id(), PatchControlId::Engine);
        assert_eq!(text.selected_line(), 2);
        let (clip_rect, text_shape) = painted_projection(&engine_focus_output, text.body())
            .expect("bare W returns focus through the canonical PATCH projection");
        let selected = text_shape.galley.rows[text.selected_line()]
            .rect()
            .translate(text_shape.pos.to_vec2());
        assert!(clip_rect.contains(selected.center()));
    }

    context.begin_pass(raw_input(
        vec![
            key_event(egui::Key::K),
            key_event(egui::Key::D),
            key_release(egui::Key::D),
            key_release(egui::Key::K),
        ],
        patch_frame_time,
    ));
    application.update(&context, &mut frame);
    let pending_output = context.end_pass();
    {
        let app_loop = shared.borrow();
        let pending_tree = tree_value(&app_loop);
        let pending_text = app_loop.current_text();
        let page = app_loop
            .current_patch_page()
            .expect("K+D keeps the canonical PATCH page visible");
        let records = app_loop.event_log();
        let record = records
            .records()
            .last()
            .expect("the normalized chord emits exactly one semantic event");

        assert_eq!(records.len(), 23);
        assert_eq!(page.engine().status(), EngineSelectionStatusKind::Preparing);
        assert!(!page.engine().editable());
        assert_eq!(
            page.engine().active_capability_id().as_str(),
            "instrument.soundfont.hidef"
        );
        assert_eq!(
            page.engine()
                .requested_capability_id()
                .expect("Preparing identifies the requested capability")
                .as_str(),
            BRAIDS_CAPABILITY_ID
        );
        assert_eq!(
            page.engine().request_id(),
            Some(EngineSelectionRequestId::FIRST)
        );
        assert_eq!(page.engine().target_graph_revision(), None);
        assert_eq!(pending_tree["engineSelection"]["kind"], "preparing");
        assert_eq!(
            pending_tree["engineSelection"]["correlation"]["requestId"],
            EngineSelectionRequestId::FIRST.value()
        );
        assert_eq!(
            pending_tree["engineSelection"]["correlation"]["targetCapabilityId"],
            BRAIDS_CAPABILITY_ID
        );
        assert_eq!(
            pending_tree["parameters"]["graphRevision"],
            page.engine().active_graph_revision().value()
        );
        assert_eq!(
            pending_tree["patchPage"]["stateHash"],
            pending_text.state_hash()
        );
        assert_eq!(pending_tree["projection"]["body"], pending_text.body());
        assert_eq!(pending_text.selected_line(), 2);
        assert!(pending_text
            .body()
            .lines()
            .nth(pending_text.selected_line())
            .expect("the engine line exists")
            .starts_with("> ENGINE"));
        assert!(matches!(
            record.input(),
            EventInput::Adjust {
                direction: EventDirection::Right
            }
        ));
        assert_eq!(record.source(), EventSource::System);
        assert_eq!(record.outcome(), EventOutcome::Accepted);
        assert_eq!(
            record.generation_after(),
            app_loop.current_state_tree().generation()
        );
        assert_eq!(record.state_hash_after(), pending_text.state_hash());
        assert_eq!(record.emitted_events().len(), 3);
        match &record.emitted_events()[2] {
            EmittedEvent::EngineSelection { effect } => {
                assert_eq!(effect.kind(), EngineSelectionEffectKind::PrepareRequested);
                assert_eq!(effect.request_id(), EngineSelectionRequestId::FIRST);
                assert_eq!(effect.target_capability_id().as_str(), BRAIDS_CAPABILITY_ID);
                assert_eq!(effect.target_graph_revision(), None);
            }
            other => panic!("expected correlated preparation effect, got {other:?}"),
        }

        let (clip_rect, text_shape) = painted_projection(&pending_output, pending_text.body())
            .expect("the chord frame paints the pending PATCH projection");
        let selected = text_shape.galley.rows[pending_text.selected_line()]
            .rect()
            .translate(text_shape.pos.to_vec2());
        assert!(clip_rect.contains(selected.center()));
    }

    context.begin_pass(raw_input(
        vec![key_event(egui::Key::Num1)],
        patch_frame_time + 0.25,
    ));
    application.update(&context, &mut frame);
    let output = context.end_pass();

    let app_loop = shared.borrow();
    let after_tree = app_loop.current_state_tree();
    let after = tree_value(&app_loop);
    let text = app_loop.current_text();
    let records = app_loop.event_log();

    assert_eq!(after["interaction"]["context"], "mixer");
    assert_eq!(
        after["interaction"]["mixerSelection"],
        retained_mixer_selection
    );
    assert_eq!(after["interaction"]["mixerSelection"]["section"], "Global");
    assert_eq!(after["interaction"]["mixerSelection"]["parameterIndex"], 6);
    assert!(after["patchPage"].is_null());
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
    assert_eq!(text.context(), TopLevelContext::Mixer);
    assert_eq!(text.body(), retained_mixer_body);
    assert_eq!(text.selected_line(), retained_mixer_line);
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

    assert_eq!(records.len(), 24);
    let adjustment = &records.records()[8];
    assert_eq!(adjustment.source(), EventSource::System);
    assert_eq!(adjustment.outcome(), EventOutcome::Accepted);
    assert!(matches!(
        adjustment.input(),
        EventInput::Adjust {
            direction: EventDirection::Right
        }
    ));
    let last = records
        .records()
        .last()
        .expect("the MIXER return has an EventRecord");
    assert_eq!(last.source(), EventSource::System);
    assert_eq!(last.outcome(), EventOutcome::Accepted);
    assert!(matches!(
        last.input(),
        EventInput::SelectContext {
            context: TopLevelContext::Mixer
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
        "the exact selected line must be the scroll target: clip={clip_rect:?}, selected={selected_rect:?}"
    );
    assert_eq!(tick_count.get(), 15);
    assert_eq!(
        idle_output.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
        Duration::from_millis(16),
        "an idle live frame must schedule its successor instead of requesting an immediate repaint; causes={:?}",
        context.repaint_causes()
    );

    println!("CREST_ACCEPTANCE eframe_context passed");
}
