use crest_synth::adapter::braids_capability::BraidsCapability;
use crest_synth::adapter::eframe_text_window::EframeApplication;
use crest_synth::adapter::hidef_soundfont_capability::HiDefSoundFontCapability;
use crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
use crest_synth::adapter::lock_free_structural_graph_boundary::LockFreeStructuralGraphBoundary;
use crest_synth::adapter::production_instruments::{
    production_capability_registry, production_instrument_preparers,
};
use crest_synth::control::app_event::AppEvent;
use crest_synth::control::app_loop::AppLoop;
use crest_synth::control::app_state::{AppState, EventRejection};
use crest_synth::control::event_record::{EmittedEvent, EventInput, EventOutcome};
use crest_synth::control::patch_page_projection::PatchPageProjection;
use crest_synth::control::state_projector::StateProjector;
use crest_synth::control::{EngineSelectionStatusKind, TopLevelContext};
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crest_synth::kernel::patch_id::PatchId;
use crest_synth::mixer::channel_parameters::ChannelParameters;
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::real_time::audio_boundary::{AudioBoundary, BoundaryFull, ControlAudioBoundary};
use crest_synth::real_time::audio_command::AudioCommand;
use crest_synth::real_time::audio_renderer::AudioRenderer;
use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
use crest_synth::real_time::prepared_graph_builder::PreparedGraphBuilder;
use crest_synth::real_time::structural_graph_boundary::{
    AudioStructuralGraphBoundary, ControlStructuralGraphBoundary, NoStructuralGraphChanges,
    StructuralGraphBoundary,
};
use crest_synth::real_time::{GraphHandoffStatus, GraphRevision};
use crest_synth::shell::app_window::{AppInputCallback, ProjectionCallback, TickCallback};
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::{
    CapabilityId, InstrumentConfig, InstrumentPreparationError, InstrumentPreparer,
    ParameterDefault, ParameterKind, Patch, PreparedInstrument, VoiceEnvelope,
};
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use eframe::egui;
use eframe::App;
use serde_json::Value;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const SAMPLE_RATE: f32 = 48_000.0;
const BLOCK_FRAMES: usize = 256;
const BLOCK_SAMPLES: usize = BLOCK_FRAMES * 2;

thread_local! {
    static COUNT_MEMORY: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static DEALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct AcceptanceAllocator;

#[global_allocator]
static ACCEPTANCE_ALLOCATOR: AcceptanceAllocator = AcceptanceAllocator;

unsafe impl GlobalAlloc for AcceptanceAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation();
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record_allocation();
        record_deallocation();
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

fn record_allocation() {
    let _ = COUNT_MEMORY.try_with(|enabled| {
        if enabled.get() {
            let _ = ALLOCATIONS.try_with(|count| count.set(count.get().saturating_add(1)));
        }
    });
}

fn record_deallocation() {
    let _ = COUNT_MEMORY.try_with(|enabled| {
        if enabled.get() {
            let _ = DEALLOCATIONS.try_with(|count| count.set(count.get().saturating_add(1)));
        }
    });
}

fn begin_memory_count() {
    ALLOCATIONS.with(|count| count.set(0));
    DEALLOCATIONS.with(|count| count.set(0));
    COUNT_MEMORY.with(|enabled| enabled.set(true));
}

fn finish_memory_count() -> (usize, usize) {
    COUNT_MEMORY.with(|enabled| enabled.set(false));
    (ALLOCATIONS.with(Cell::get), DEALLOCATIONS.with(Cell::get))
}

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

struct CountingPreparer {
    inner: Box<dyn InstrumentPreparer>,
    preparations: Arc<AtomicUsize>,
}

impl InstrumentPreparer for CountingPreparer {
    fn capability_id(&self) -> &CapabilityId {
        self.inner.capability_id()
    }

    fn prepared_shared_asset_count(&self) -> usize {
        self.inner.prepared_shared_asset_count()
    }

    fn prepare(
        &self,
        patch: &Patch,
        sample_rate: f32,
        max_frames: usize,
    ) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
        self.preparations.fetch_add(1, Ordering::SeqCst);
        self.inner.prepare(patch, sample_rate, max_frames)
    }
}

fn globals() -> GlobalParameters {
    GlobalParameters::new(-6.0, 0.61, 0.39, 0.0, 321.0, 0.0, 0.0).unwrap()
}

fn soundfont_config() -> InstrumentConfig {
    let provider = HiDefSoundFontCapability::new().unwrap();
    create_soundfont_config(&provider, SoundFontInstrument::new(0, 80, false).unwrap()).unwrap()
}

fn production_patches() -> Vec<Patch> {
    vec![
        Patch::new(
            PatchId::new(11).unwrap(),
            "Schema Lead".to_owned(),
            soundfont_config(),
            MidiChannel::new(3).unwrap(),
            ChannelParameters::new(-4.0, -0.31, 0.0, 0.0).unwrap(),
        )
        .with_envelope(VoiceEnvelope::new(0.0, 37.0, 0.72, 91.0).unwrap()),
        Patch::new(
            PatchId::new(29).unwrap(),
            "Schema Braids".to_owned(),
            BraidsCapability::new().unwrap().default_config().unwrap(),
            MidiChannel::new(9).unwrap(),
            ChannelParameters::new(-7.0, 0.28, 0.0, 0.0).unwrap(),
        )
        .with_envelope(VoiceEnvelope::new(0.0, 23.0, 0.64, 73.0).unwrap()),
    ]
}

fn installed_state(patches: Vec<Patch>) -> AppState {
    let mut state = AppState::new(production_capability_registry().unwrap(), globals());
    state.apply(AppEvent::InstallPatches(patches)).unwrap();
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

fn raw_input(events: Vec<egui::Event>, time_seconds: f64) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1_400.0, 400.0),
        )),
        time: Some(time_seconds),
        events,
        ..Default::default()
    }
}

fn run_frame(
    application: &mut EframeApplication,
    context: &egui::Context,
    frame: &mut eframe::Frame,
    events: Vec<egui::Event>,
    time_seconds: f64,
) {
    context.begin_pass(raw_input(events, time_seconds));
    application.update(context, frame);
    let _ = context.end_pass();
}

fn assert_page_is_exact(state: &AppState, page: &PatchPageProjection) {
    let patch = &state.patches()[0];
    let descriptor = state
        .capabilities()
        .descriptor(patch.instrument_config().capability_id())
        .unwrap();

    assert_eq!(page.context(), TopLevelContext::Patch);
    assert_eq!(page.patch().id(), patch.id());
    assert_eq!(page.patch().name(), patch.name());
    assert_eq!(page.patch().midi_channel(), patch.channel());
    assert_eq!(page.engine().active_capability_id(), descriptor.id());
    assert_eq!(page.engine().active_label(), descriptor.label());
    assert!(page.engine().editable());
    assert_eq!(
        page.engine().choices().len(),
        state.capabilities().descriptors().len()
    );
    for (choice, available) in page
        .engine()
        .choices()
        .iter()
        .zip(state.capabilities().descriptors())
    {
        assert_eq!(choice.capability_id(), available.id());
        assert_eq!(choice.label(), available.label());
    }

    assert_eq!(
        page.envelope().len(),
        VoiceEnvelope::surface_descriptor().len()
    );
    for (row, spec) in page
        .envelope()
        .iter()
        .zip(VoiceEnvelope::surface_descriptor())
    {
        assert_eq!(row.id(), spec.name());
        assert_eq!(row.label(), spec.label());
        assert_eq!(row.value(), patch.envelope().value(spec.parameter()));
        assert_eq!(row.minimum(), spec.minimum());
        assert_eq!(row.maximum(), spec.maximum());
        assert_eq!(row.fine_step(), spec.fine_step());
        assert_eq!(row.coarse_step(), spec.coarse_step());
        assert_eq!(row.unit(), spec.unit());
        assert!(!row.editable());
    }

    assert_eq!(page.sections().len(), descriptor.sections().len());
    for (section, section_spec) in page.sections().iter().zip(descriptor.sections()) {
        assert_eq!(section.id(), section_spec.id());
        assert_eq!(section.label(), section_spec.label());
        assert_eq!(section.parameters().len(), section_spec.parameters().len());
        for (row, spec) in section.parameters().iter().zip(section_spec.parameters()) {
            assert_eq!(row.id(), spec.id());
            assert_eq!(row.label(), spec.label());
            assert_eq!(row.kind(), spec.kind());
            assert_eq!(row.update(), spec.update());
            assert_eq!(row.range(), spec.range());
            assert_eq!(row.choices(), spec.choices());
            assert_eq!(row.fine_step(), spec.fine_step());
            assert_eq!(row.coarse_step(), spec.coarse_step());
            assert_eq!(row.unit(), spec.unit());
            assert_eq!(row.formatter(), spec.formatter());
            assert!(!row.editable());

            let predicate_satisfied =
                |predicate: Option<&crest_synth::synth::ParameterPredicate>| {
                    predicate.is_none_or(|predicate| {
                        patch.instrument_config().value(predicate.parameter_id())
                            == Some(predicate.equals())
                    })
                };
            assert_eq!(row.enabled(), predicate_satisfied(spec.enabled_when()));
            assert_eq!(row.visible(), predicate_satisfied(spec.visible_when()));

            if spec.kind() == ParameterKind::Asset {
                let expected = patch
                    .instrument_config()
                    .asset_reference(spec.id())
                    .or_else(|| match spec.default_value() {
                        ParameterDefault::Asset(reference) => Some(reference),
                        ParameterDefault::Value(_) => None,
                    })
                    .unwrap();
                assert_eq!(row.value().asset(), Some(expected));
                assert_eq!(row.value().parameter(), None);
            } else {
                assert_eq!(
                    row.value().parameter(),
                    patch.instrument_config().value(spec.id())
                );
                assert_eq!(row.value().asset(), None);
            }
        }
    }
}

fn prove_both_production_pages() {
    let configs = vec![
        soundfont_config(),
        BraidsCapability::new().unwrap().default_config().unwrap(),
    ];
    for (index, config) in configs.into_iter().enumerate() {
        let patch = Patch::new(
            PatchId::new(index as u32 + 1).unwrap(),
            format!("Projected {index}"),
            config,
            MidiChannel::new(index as u8).unwrap(),
            ChannelParameters::new(-3.0, 0.17, 0.23, 0.31).unwrap(),
        )
        .with_envelope(VoiceEnvelope::new(12.0, 34.0, 0.56, 78.0).unwrap());
        let mut state = installed_state(vec![patch]);
        state
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        let (snapshot, page, text, parameters, tree) =
            StateProjector::new().project_with_tree(&state).unwrap();
        let page = page.unwrap();

        assert_page_is_exact(&state, &page);
        assert_eq!(page.state_hash(), snapshot.hash());
        assert_eq!(text.context(), TopLevelContext::Patch);
        assert_eq!(text.state_hash(), snapshot.hash());
        assert_eq!(tree.state_hash(), snapshot.hash());
        assert_eq!(parameters.generation(), state.generation());
        let value: Value = serde_json::from_str(tree.json()).unwrap();
        assert_eq!(value["interaction"]["context"], "patch");
        assert_eq!(
            value["interaction"]["patchFocus"],
            state.patches()[0].id().value()
        );
        let serialized_page: Value =
            serde_json::from_str(&serde_json::to_string(&page).unwrap()).unwrap();
        assert_eq!(value["patchPage"], serialized_page);
        assert_eq!(value["projection"]["body"], text.body());
    }
}

fn same_parameter_values(before: ParameterSnapshot, after: ParameterSnapshot) -> bool {
    before.graph_revision() == after.graph_revision()
        && before.global() == after.global()
        && before.patch_count() == after.patch_count()
        && before.storage() == after.storage()
}

fn counted_render<Boundary, Structural>(
    renderer: &mut AudioRenderer<Boundary, Structural>,
    output: &mut [f32],
) -> (usize, usize)
where
    Boundary: crest_synth::real_time::audio_boundary::AudioThreadBoundary,
    Structural: AudioStructuralGraphBoundary,
{
    begin_memory_count();
    renderer.render(output);
    finish_memory_count()
}

#[test]
fn patch_page_context_is_exact_recoverable_and_audio_neutral() {
    prove_both_production_pages();

    let patches = production_patches();
    let observations = Arc::new(Mutex::new(BoundaryObservations::default()));
    let app_loop = AppLoop::new(
        installed_state(patches.clone()),
        StateProjector::for_graph(GraphRevision::INITIAL),
        ProbeBoundary {
            observations: Arc::clone(&observations),
        },
    )
    .unwrap();
    let before_parameters = *app_loop.current_parameters();
    let before_tree: Value = serde_json::from_str(app_loop.current_state_tree().json()).unwrap();
    let before_body = app_loop.current_text().body().to_owned();
    let before_line = app_loop.current_text().selected_line();

    let structural = LockFreeStructuralGraphBoundary::new(
        1,
        1,
        GraphHandoffStatus::with_active(GraphRevision::INITIAL),
    )
    .unwrap();
    let (mut structural_control, mut structural_audio) = structural.into_handles();
    let handoff_before = structural_control.read_status_on_control();

    let preparations = Arc::new(AtomicUsize::new(0));
    let preparers = production_instrument_preparers()
        .unwrap()
        .into_iter()
        .map(|inner| {
            Box::new(CountingPreparer {
                inner,
                preparations: Arc::clone(&preparations),
            }) as Box<dyn InstrumentPreparer>
        })
        .collect::<Vec<_>>();

    let shared = Rc::new(RefCell::new(app_loop));
    let rejections = Rc::new(RefCell::new(Vec::new()));
    let input_loop = Rc::clone(&shared);
    let input_rejections = Rc::clone(&rejections);
    let on_input: AppInputCallback = Box::new(move |event| {
        if let Err(rejection) = input_loop.borrow_mut().dispatch(event) {
            input_rejections.borrow_mut().push(rejection);
        }
    });
    let projection_loop = Rc::clone(&shared);
    let projection: ProjectionCallback = Box::new(move || projection_loop.borrow().current_text());
    let on_tick: TickCallback = Box::new(|_| true);
    let mut application = EframeApplication::new(on_input, projection, on_tick);
    let context = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();

    run_frame(
        &mut application,
        &context,
        &mut frame,
        vec![key_event(egui::Key::Num2)],
        0.0,
    );

    let (after_parameters, patch_tree, patch_page, patch_text, patch_generation) = {
        let app_loop = shared.borrow();
        (
            *app_loop.current_parameters(),
            serde_json::from_str::<Value>(app_loop.current_state_tree().json()).unwrap(),
            app_loop.current_patch_page().unwrap(),
            app_loop.current_text(),
            app_loop.current_state_tree().generation(),
        )
    };

    assert_eq!(preparations.load(Ordering::SeqCst), 0);
    assert_eq!(structural_control.read_status_on_control(), handoff_before);
    assert!(structural_audio.take_prepared_on_audio().is_none());
    assert_eq!(structural_control.collect_retired_on_control(), None);
    assert_eq!(
        after_parameters.generation(),
        before_parameters.generation() + 1
    );
    assert!(same_parameter_values(before_parameters, after_parameters));
    assert_eq!(patch_tree["capabilities"], before_tree["capabilities"]);
    assert_eq!(patch_tree["patches"], before_tree["patches"]);
    assert_eq!(patch_tree["global"], before_tree["global"]);
    assert_eq!(
        patch_tree["interaction"]["mixerSelection"],
        before_tree["interaction"]["mixerSelection"]
    );
    assert_eq!(
        patch_tree["interaction"]["patchFocus"],
        before_tree["interaction"]["patchFocus"]
    );
    assert_eq!(
        patch_tree["parameters"]["graphRevision"],
        before_tree["parameters"]["graphRevision"]
    );
    assert_eq!(
        patch_tree["parameters"]["patches"],
        before_tree["parameters"]["patches"]
    );
    assert_eq!(
        patch_tree["parameters"]["global"],
        before_tree["parameters"]["global"]
    );
    assert_eq!(patch_tree["interaction"]["context"], "patch");
    let serialized_patch_page: Value =
        serde_json::from_str(&serde_json::to_string(&patch_page).unwrap()).unwrap();
    assert_eq!(patch_tree["patchPage"], serialized_patch_page);
    assert_eq!(patch_text.context(), TopLevelContext::Patch);
    assert_eq!(patch_text.state_hash(), patch_page.state_hash());
    assert_page_is_exact(&installed_state(patches.clone()), &patch_page);

    {
        let observations = observations.lock().unwrap();
        assert_eq!(observations.parameters.len(), 2);
        assert_eq!(observations.parameters[1], after_parameters);
        assert!(observations.commands.is_empty());
    }
    {
        let app_loop = shared.borrow();
        let record = app_loop.event_log_ref().records().last().unwrap();
        assert_eq!(record.outcome(), EventOutcome::Accepted);
        assert!(matches!(
            record.input(),
            EventInput::SelectContext {
                context: TopLevelContext::Patch
            }
        ));
        assert_eq!(record.generation_after(), patch_generation);
        assert_eq!(record.parameter_generation(), after_parameters.generation());
        assert_eq!(record.state_hash_after(), patch_text.state_hash());
        assert_eq!(record.projection_state_hash(), patch_text.state_hash());
        assert_eq!(
            record.emitted_events(),
            &[
                EmittedEvent::StateAccepted {
                    generation: patch_generation
                },
                EmittedEvent::ParameterSnapshotPublished {
                    generation: patch_generation,
                    graph_revision: GraphRevision::INITIAL
                }
            ]
        );
    }

    run_frame(
        &mut application,
        &context,
        &mut frame,
        vec![key_event(egui::Key::W)],
        0.25,
    );
    run_frame(
        &mut application,
        &context,
        &mut frame,
        vec![key_event(egui::Key::K), key_event(egui::Key::D)],
        0.5,
    );
    assert_eq!(
        rejections.borrow().as_slice(),
        &[EventRejection::ActionUnavailableInContext]
    );
    {
        let app_loop = shared.borrow();
        let pending_page = app_loop.current_patch_page().unwrap();
        assert_eq!(
            app_loop.current_state_tree().generation(),
            patch_generation + 1
        );
        assert_eq!(
            pending_page.engine().status(),
            EngineSelectionStatusKind::Preparing
        );
        assert_eq!(
            pending_page.engine().active_capability_id(),
            patch_page.engine().active_capability_id()
        );
        assert!(pending_page.engine().requested_capability_id().is_some());
        assert!(pending_page.engine().request_id().is_some());
        assert_eq!(
            app_loop.current_parameters().graph_revision(),
            GraphRevision::INITIAL
        );
        assert!(same_parameter_values(
            *app_loop.current_parameters(),
            after_parameters
        ));
    }
    assert_eq!(observations.lock().unwrap().parameters.len(), 3);

    run_frame(
        &mut application,
        &context,
        &mut frame,
        vec![key_event(egui::Key::Num1)],
        0.75,
    );
    {
        let app_loop = shared.borrow();
        let recovered: Value = serde_json::from_str(app_loop.current_state_tree().json()).unwrap();
        assert_eq!(app_loop.current_text().context(), TopLevelContext::Mixer);
        assert_eq!(app_loop.current_text().body(), before_body);
        assert_eq!(app_loop.current_text().selected_line(), before_line);
        assert!(app_loop.current_patch_page().is_none());
        assert_eq!(
            recovered["interaction"]["mixerSelection"],
            before_tree["interaction"]["mixerSelection"]
        );
        assert_eq!(recovered["patches"], before_tree["patches"]);
        assert_eq!(recovered["global"], before_tree["global"]);
        assert!(same_parameter_values(
            before_parameters,
            *app_loop.current_parameters()
        ));
        assert!(matches!(
            app_loop.event_log_ref().records().last().unwrap().input(),
            EventInput::SelectContext {
                context: TopLevelContext::Mixer
            }
        ));
    }
    {
        let observations = observations.lock().unwrap();
        assert_eq!(observations.parameters.len(), 4);
        assert!(observations.commands.is_empty());
    }
    assert_eq!(preparations.load(Ordering::SeqCst), 0);
    assert_eq!(structural_control.read_status_on_control(), handoff_before);
    assert!(structural_audio.take_prepared_on_audio().is_none());
    assert_eq!(structural_control.collect_retired_on_control(), None);

    let before_graph =
        PreparedGraphBuilder::new(&production_capability_registry().unwrap(), &preparers)
            .build(
                GraphRevision::INITIAL,
                &patches,
                before_parameters,
                SAMPLE_RATE,
                BLOCK_FRAMES,
            )
            .unwrap();
    let after_graph =
        PreparedGraphBuilder::new(&production_capability_registry().unwrap(), &preparers)
            .build(
                GraphRevision::INITIAL,
                &patches,
                after_parameters,
                SAMPLE_RATE,
                BLOCK_FRAMES,
            )
            .unwrap();
    assert_eq!(preparations.load(Ordering::SeqCst), patches.len() * 2);

    let before_boundary = LockFreeAudioBoundary::new(16, before_parameters);
    let after_boundary = LockFreeAudioBoundary::new(16, after_parameters);
    let (mut before_control, before_audio) = before_boundary.into_handles();
    let (mut after_control, after_audio) = after_boundary.into_handles();
    for patch in &patches {
        let command = AudioCommand::patch_midi(
            patch.id(),
            MidiMessage::try_new(patch.channel(), MidiMessageKind::NoteOn, 60, 110).unwrap(),
        );
        before_control.push_command(command).unwrap();
        after_control.push_command(command).unwrap();
    }

    let mut before_renderer =
        AudioRenderer::new(before_audio, NoStructuralGraphChanges::new(), before_graph);
    let mut after_renderer =
        AudioRenderer::new(after_audio, NoStructuralGraphChanges::new(), after_graph);
    let mut before_output = [0.0_f32; BLOCK_SAMPLES];
    let mut after_output = [0.0_f32; BLOCK_SAMPLES];
    let before_memory = counted_render(&mut before_renderer, &mut before_output);
    let after_memory = counted_render(&mut after_renderer, &mut after_output);

    assert_eq!(before_memory, (0, 0));
    assert_eq!(after_memory, (0, 0));
    assert!(before_output.iter().all(|sample| sample.is_finite()));
    assert!(before_output.iter().any(|sample| sample.abs() > 1.0e-6));
    assert_eq!(before_output, after_output);
    assert_eq!(before_renderer.active_revision(), GraphRevision::INITIAL);
    assert_eq!(after_renderer.active_revision(), GraphRevision::INITIAL);
    assert_eq!(before_renderer.pending_retirement_revision(), None);
    assert_eq!(after_renderer.pending_retirement_revision(), None);
    assert_eq!(before_renderer.handoff_status().swaps_applied(), 0);
    assert_eq!(after_renderer.handoff_status().swaps_applied(), 0);
    assert_eq!(before_renderer.handoff_status().retired_revision(), None);
    assert_eq!(after_renderer.handoff_status().retired_revision(), None);
    assert!(same_parameter_values(
        *before_renderer.parameters(),
        *after_renderer.parameters()
    ));

    println!("CREST_ACCEPTANCE patch_page_projection passed");
}
