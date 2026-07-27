use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use crest_synth::adapter::braids_capability::{BraidsCapability, BRAIDS_CAPABILITY_ID};
use crest_synth::adapter::braids_native::BRAIDS_VOICE_COUNT;
use crest_synth::adapter::braids_preparer::BraidsPreparer;
use crest_synth::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID;
use crest_synth::adapter::hidef_soundfont_preparer::HiDefSoundFontPreparer;
use crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
use crest_synth::adapter::production_instruments::{
    production_capability_registry, production_instrument_preparers,
};
use crest_synth::adapter::soundfont_voice_engine::soundfont_engine_lifecycle_counts;
use crest_synth::control::app_event::{AppEvent, Direction};
use crest_synth::control::app_loop::AppLoop;
use crest_synth::control::app_state::AppState;
use crest_synth::control::event_record::EmittedEvent;
use crest_synth::control::state_projector::StateProjector;
use crest_synth::control::{PatchControlId, TopLevelContext};
use crest_synth::kernel::midi_channel::MidiChannel;
use crest_synth::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crest_synth::kernel::patch_id::PatchId;
use crest_synth::mixer::channel_parameters::ChannelParameters;
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::real_time::audio_boundary::{
    AudioBoundary, AudioThreadBoundary, ControlAudioBoundary,
};
use crest_synth::real_time::audio_command::AudioCommand;
use crest_synth::real_time::audio_renderer::AudioRenderer;
use crest_synth::real_time::parameter_snapshot::{
    ParameterSnapshot, RtInstrumentParameters, RtPatchParameters,
};
use crest_synth::real_time::prepared_graph_builder::PreparedGraphBuilder;
use crest_synth::real_time::structural_graph_boundary::NoStructuralGraphChanges;
use crest_synth::real_time::GraphRevision;
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::{InstrumentPreparer, Patch, VoiceEnvelope};
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use serde::Serialize;
use serde_json::json;
use std::alloc::System;

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

#[derive(Serialize)]
struct EnvelopeObservation {
    parameter_cases_exercised: usize,
    patch_control_cases_exercised: usize,
    patch_focus_order_exact: bool,
    fine_coarse_adjustment_exact: bool,
    mixer_patch_mutation_shared: bool,
    scalar_only_publication: bool,
    target_patch_isolated: bool,
    soundfont_synthesizers_per_patch: usize,
    braids_voices_per_patch: usize,
    state_text_snapshot_exact: bool,
    soundfont_overlap_independent: bool,
    braids_overlap_independent: bool,
    all_fields_audible: bool,
    post_stem_envelope_absent: bool,
    extremes_finite: bool,
    callback_allocations: usize,
    callback_destructions: usize,
}

#[test]
fn common_adsr_is_per_voice_in_both_production_engines() {
    let soundfont_preparer = HiDefSoundFontPreparer::new(
        crest_synth::adapter::production_instruments::production_soundfont_asset().unwrap(),
    )
    .expect("SoundFont bank prepares");
    let braids_preparer = BraidsPreparer::new().expect("Braids preparer is valid");
    let soundfont_patch = soundfont_patch(1, 0);
    let braids_patch = braids_patch(2, 1);

    let soundfont_cases = exercise_all_fields(&soundfont_preparer, &soundfont_patch);
    let braids_cases = exercise_all_fields(&braids_preparer, &braids_patch);
    let parameter_cases_exercised = VoiceEnvelope::surface_descriptor().len();
    assert_eq!(parameter_cases_exercised, 4);
    let all_fields_audible = soundfont_cases.iter().all(|audible| *audible)
        && braids_cases.iter().all(|audible| *audible);
    assert!(all_fields_audible);

    let (
        patch_control_cases_exercised,
        patch_focus_order_exact,
        fine_coarse_adjustment_exact,
        mixer_patch_mutation_shared,
        scalar_only_publication,
        target_patch_isolated,
    ) = prove_patch_control_contract();
    assert_eq!(patch_control_cases_exercised, 4);
    assert!(patch_focus_order_exact);
    assert!(fine_coarse_adjustment_exact);
    assert!(mixer_patch_mutation_shared);
    assert!(scalar_only_publication);
    assert!(target_patch_isolated);

    let soundfont_overlap_independent =
        prove_overlapping_release(&soundfont_preparer, &soundfont_patch);
    let braids_overlap_independent = prove_overlapping_release(&braids_preparer, &braids_patch);
    assert!(soundfont_overlap_independent);
    assert!(braids_overlap_independent);

    let extremes_finite = prove_finite_extremes(&soundfont_preparer, &soundfont_patch)
        && prove_finite_extremes(&braids_preparer, &braids_patch);
    assert!(extremes_finite);

    let soundfont_synthesizers_per_patch =
        prove_one_soundfont_synthesizer_per_patch(&soundfont_preparer, &soundfont_patch);
    assert_eq!(soundfont_synthesizers_per_patch, 1);
    let braids_voices_per_patch = BRAIDS_VOICE_COUNT;
    assert_eq!(braids_voices_per_patch, 16);

    let state_text_snapshot_exact = prove_state_text_snapshot_projection();
    assert!(state_text_snapshot_exact);

    let (callback_allocations, callback_destructions, unreleased_notes_survive) =
        prove_mixed_callback_contract();
    assert_eq!(callback_allocations, 0);
    assert_eq!(callback_destructions, 0);
    assert!(unreleased_notes_survive);

    // A Patch-stem envelope cannot distinguish releasing one of two voices
    // from releasing both. These paired production-engine renders do, and the
    // same behavior survives the production rack/render path.
    let post_stem_envelope_absent = all_fields_audible
        && soundfont_overlap_independent
        && braids_overlap_independent
        && unreleased_notes_survive;
    assert!(post_stem_envelope_absent);

    let observation = EnvelopeObservation {
        parameter_cases_exercised,
        patch_control_cases_exercised,
        patch_focus_order_exact,
        fine_coarse_adjustment_exact,
        mixer_patch_mutation_shared,
        scalar_only_publication,
        target_patch_isolated,
        soundfont_synthesizers_per_patch,
        braids_voices_per_patch,
        state_text_snapshot_exact,
        soundfont_overlap_independent,
        braids_overlap_independent,
        all_fields_audible,
        post_stem_envelope_absent,
        extremes_finite,
        callback_allocations,
        callback_destructions,
    };
    println!(
        "CREST_ENVELOPE_OBSERVATION {}",
        serde_json::to_string(&observation).unwrap()
    );
    println!("CREST_ACCEPTANCE per_voice_envelope passed");
}

fn globals() -> GlobalParameters {
    GlobalParameters::new(-6.0, 0.5, 0.5, 0.0, 250.0, 0.0, 0.0).unwrap()
}

fn soundfont_patch(id: u32, channel: u8) -> Patch {
    let provider =
        crest_synth::adapter::production_instruments::production_soundfont_capability().unwrap();
    Patch::new(
        PatchId::new(id).unwrap(),
        format!("SoundFont {id}"),
        create_soundfont_config(&provider, SoundFontInstrument::new(0, 0, false).unwrap()).unwrap(),
        MidiChannel::new(channel).unwrap(),
        ChannelParameters::default(),
    )
}

fn braids_patch(id: u32, channel: u8) -> Patch {
    Patch::new(
        PatchId::new(id).unwrap(),
        format!("Braids {id}"),
        BraidsCapability::new().unwrap().default_config().unwrap(),
        MidiChannel::new(channel).unwrap(),
        ChannelParameters::default(),
    )
}

fn parameters(patch: &Patch, envelope: VoiceEnvelope) -> RtPatchParameters {
    let instrument = match patch.instrument_config().capability_id().as_str() {
        HIDEF_CAPABILITY_ID => RtInstrumentParameters::EMPTY,
        BRAIDS_CAPABILITY_ID => RtInstrumentParameters::new(&[0.0, 0.5, 0.5]).unwrap(),
        capability => panic!("unexpected production capability {capability}"),
    };
    RtPatchParameters::projected(patch.id(), *patch.parameters(), envelope, instrument)
}

fn note(patch: &Patch, kind: MidiMessageKind, key: u8, velocity: u8) -> MidiMessage {
    MidiMessage::try_new(patch.channel(), kind, key, velocity).unwrap()
}

fn finite(output: &[f32]) -> bool {
    output.iter().all(|sample| sample.is_finite())
}

fn sounding(output: &[f32]) -> bool {
    output.iter().any(|sample| sample.abs() > 1.0e-6)
}

fn difference(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right).abs())
        .sum()
}

fn audibly_distinct(left: &[f32], right: &[f32]) -> bool {
    finite(left)
        && finite(right)
        && (sounding(left) || sounding(right))
        && difference(left, right) > 1.0e-4
}

fn render_onset(
    preparer: &dyn InstrumentPreparer,
    patch: &Patch,
    envelope: VoiceEnvelope,
) -> [f32; BLOCK_SAMPLES] {
    let parameters = parameters(patch, envelope);
    let mut instrument = preparer.prepare(patch, SAMPLE_RATE, BLOCK_FRAMES).unwrap();
    instrument
        .dispatch(note(patch, MidiMessageKind::NoteOn, 60, 120), &parameters)
        .unwrap();
    let mut output = [0.0_f32; BLOCK_SAMPLES];
    instrument.render(&mut output, BLOCK_FRAMES, &parameters);
    output
}

fn render_release(
    preparer: &dyn InstrumentPreparer,
    patch: &Patch,
    release_milliseconds: f32,
) -> [f32; BLOCK_SAMPLES] {
    let envelope = VoiceEnvelope::new(0.0, 0.0, 1.0, release_milliseconds).unwrap();
    let parameters = parameters(patch, envelope);
    let mut instrument = preparer.prepare(patch, SAMPLE_RATE, BLOCK_FRAMES).unwrap();
    instrument
        .dispatch(note(patch, MidiMessageKind::NoteOn, 60, 120), &parameters)
        .unwrap();
    let mut warmup = [0.0_f32; BLOCK_SAMPLES];
    instrument.render(&mut warmup, BLOCK_FRAMES, &parameters);
    instrument
        .dispatch(note(patch, MidiMessageKind::NoteOff, 60, 0), &parameters)
        .unwrap();
    let mut output = [0.0_f32; BLOCK_SAMPLES];
    instrument.render(&mut output, BLOCK_FRAMES, &parameters);
    output
}

fn exercise_all_fields(preparer: &dyn InstrumentPreparer, patch: &Patch) -> [bool; 4] {
    let immediate = render_onset(preparer, patch, VoiceEnvelope::DEFAULT);
    let attacked = render_onset(
        preparer,
        patch,
        VoiceEnvelope::new(100.0, 0.0, 1.0, 0.0).unwrap(),
    );
    let immediate_decay = render_onset(
        preparer,
        patch,
        VoiceEnvelope::new(0.0, 0.0, 0.25, 0.0).unwrap(),
    );
    let decayed = render_onset(
        preparer,
        patch,
        VoiceEnvelope::new(0.0, 100.0, 0.25, 0.0).unwrap(),
    );
    let sustained = render_onset(
        preparer,
        patch,
        VoiceEnvelope::new(0.0, 0.0, 0.25, 0.0).unwrap(),
    );
    let immediate_release = render_release(preparer, patch, 0.0);
    let released = render_release(preparer, patch, 100.0);

    [
        audibly_distinct(&immediate, &attacked),
        audibly_distinct(&immediate_decay, &decayed),
        audibly_distinct(&immediate, &sustained),
        audibly_distinct(&immediate_release, &released),
    ]
}

fn prove_overlapping_release(preparer: &dyn InstrumentPreparer, patch: &Patch) -> bool {
    let envelope = VoiceEnvelope::new(0.0, 0.0, 1.0, 100.0).unwrap();
    let parameters = parameters(patch, envelope);
    let mut first_released = preparer.prepare(patch, SAMPLE_RATE, BLOCK_FRAMES).unwrap();
    let mut both_held = preparer.prepare(patch, SAMPLE_RATE, BLOCK_FRAMES).unwrap();
    let mut both_released = preparer.prepare(patch, SAMPLE_RATE, BLOCK_FRAMES).unwrap();

    for key in [60, 67] {
        let message = note(patch, MidiMessageKind::NoteOn, key, 120);
        first_released.dispatch(message, &parameters).unwrap();
        both_held.dispatch(message, &parameters).unwrap();
        both_released.dispatch(message, &parameters).unwrap();
    }

    let mut first_warmup = [0.0_f32; BLOCK_SAMPLES];
    let mut held_warmup = [0.0_f32; BLOCK_SAMPLES];
    let mut both_warmup = [0.0_f32; BLOCK_SAMPLES];
    first_released.render(&mut first_warmup, BLOCK_FRAMES, &parameters);
    both_held.render(&mut held_warmup, BLOCK_FRAMES, &parameters);
    both_released.render(&mut both_warmup, BLOCK_FRAMES, &parameters);
    let synchronized = first_warmup == held_warmup && first_warmup == both_warmup;

    first_released
        .dispatch(note(patch, MidiMessageKind::NoteOff, 60, 0), &parameters)
        .unwrap();
    for key in [60, 67] {
        both_released
            .dispatch(note(patch, MidiMessageKind::NoteOff, key, 0), &parameters)
            .unwrap();
    }

    let mut first_output = [0.0_f32; BLOCK_SAMPLES];
    let mut held_output = [0.0_f32; BLOCK_SAMPLES];
    let mut both_output = [0.0_f32; BLOCK_SAMPLES];
    first_released.render(&mut first_output, BLOCK_FRAMES, &parameters);
    both_held.render(&mut held_output, BLOCK_FRAMES, &parameters);
    both_released.render(&mut both_output, BLOCK_FRAMES, &parameters);

    synchronized
        && finite(&first_output)
        && finite(&held_output)
        && finite(&both_output)
        && sounding(&first_output)
        && audibly_distinct(&first_output, &held_output)
        && audibly_distinct(&first_output, &both_output)
}

fn prove_finite_extremes(preparer: &dyn InstrumentPreparer, patch: &Patch) -> bool {
    let extremes = [
        VoiceEnvelope::new(0.0, 0.0, 0.0, 0.0).unwrap(),
        VoiceEnvelope::new(10_000.0, 10_000.0, 1.0, 10_000.0).unwrap(),
    ];
    extremes.into_iter().all(|envelope| {
        let parameters = parameters(patch, envelope);
        let mut instrument = preparer.prepare(patch, SAMPLE_RATE, BLOCK_FRAMES).unwrap();
        instrument
            .dispatch(note(patch, MidiMessageKind::NoteOn, 60, 127), &parameters)
            .unwrap();
        let mut onset = [0.0_f32; BLOCK_SAMPLES];
        instrument.render(&mut onset, BLOCK_FRAMES, &parameters);
        instrument
            .dispatch(note(patch, MidiMessageKind::NoteOff, 60, 0), &parameters)
            .unwrap();
        let mut release = [0.0_f32; BLOCK_SAMPLES];
        instrument.render(&mut release, BLOCK_FRAMES, &parameters);
        instrument.all_notes_off();
        finite(&onset) && finite(&release)
    })
}

fn prove_one_soundfont_synthesizer_per_patch(
    preparer: &HiDefSoundFontPreparer,
    patch: &Patch,
) -> usize {
    let before = soundfont_engine_lifecycle_counts();
    let instrument = preparer.prepare(patch, SAMPLE_RATE, BLOCK_FRAMES).unwrap();
    let during = soundfont_engine_lifecycle_counts();
    let active_delta = during.active.saturating_sub(before.active) as usize;
    let created_delta = during.created.saturating_sub(before.created) as usize;
    drop(instrument);
    let after = soundfont_engine_lifecycle_counts();
    assert_eq!(after.active, before.active);
    assert_eq!(after.destroyed.saturating_sub(before.destroyed), 1);
    assert_eq!(created_delta, active_delta);
    active_delta
}

fn prove_state_text_snapshot_projection() -> bool {
    let registry = production_capability_registry().unwrap();
    let initial = ParameterSnapshot::for_graph(0, GraphRevision::INITIAL, globals(), &[]).unwrap();
    let boundary = LockFreeAudioBoundary::new(16, initial);
    let (control, _audio) = boundary.into_handles();
    let mut app_loop = AppLoop::new(
        AppState::new(registry, globals()),
        StateProjector::for_graph(GraphRevision::INITIAL),
        control,
    )
    .unwrap();
    let soundfont = soundfont_patch(1, 0);
    let braids = braids_patch(2, 1);
    app_loop
        .dispatch(AppEvent::InstallPatches(vec![soundfont.clone(), braids]))
        .unwrap();
    for _ in 0..4 {
        app_loop
            .dispatch(AppEvent::Navigate(Direction::Down))
            .unwrap();
    }
    app_loop.dispatch(AppEvent::Adjust(Direction::Up)).unwrap();

    let expected = VoiceEnvelope::new(100.0, 0.0, 1.0, 0.0).unwrap();
    let text = app_loop.current_text();
    let tree = app_loop.current_state_tree();
    let tree_json: serde_json::Value = serde_json::from_str(tree.json()).unwrap();
    let expected_json = json!({
        "attackMilliseconds": 100.0,
        "decayMilliseconds": 0.0,
        "sustain": 1.0,
        "releaseMilliseconds": 0.0
    });
    let selected = text.body().lines().nth(text.selected_line());

    app_loop.patches()[0].envelope() == &expected
        && app_loop
            .current_parameters()
            .patch(soundfont.id())
            .is_some_and(|parameters| parameters.envelope() == &expected)
        && tree_json["generation"] == 6
        && tree_json["parameters"]["generation"] == 6
        && tree_json["interaction"]["mixerSelection"]["patchIndex"] == 0
        && tree_json["interaction"]["mixerSelection"]["parameterIndex"] == 4
        && tree_json["patches"][0]["envelope"] == expected_json
        && tree_json["parameters"]["patches"][0]["envelope"] == expected_json
        && selected == Some("> attackMilliseconds=100")
        && tree_json["projection"]["body"].as_str() == Some(text.body())
        && tree_json["projection"]["selectedLine"].as_u64() == Some(text.selected_line() as u64)
        && tree_json["projection"]["stateHash"].as_str() == Some(text.state_hash())
        && tree.state_hash() == text.state_hash()
}

fn prove_patch_control_contract() -> (usize, bool, bool, bool, bool, bool) {
    let envelope = VoiceEnvelope::new(500.0, 600.0, 0.5, 700.0).unwrap();
    let patches = vec![
        soundfont_patch(1, 0).with_envelope(envelope),
        braids_patch(2, 1).with_envelope(envelope),
    ];
    let comparison = patches[1].clone();
    let mut state = AppState::for_graph(
        production_capability_registry().unwrap(),
        globals(),
        GraphRevision::INITIAL,
    );
    state.apply(AppEvent::InstallPatches(patches)).unwrap();
    let initial = ParameterSnapshot::for_graph(0, GraphRevision::INITIAL, globals(), &[]).unwrap();
    let boundary = LockFreeAudioBoundary::new(64, initial);
    let (control, mut audio) = boundary.into_handles();
    let mut app_loop = AppLoop::new(
        state,
        StateProjector::for_graph(GraphRevision::INITIAL),
        control,
    )
    .unwrap();
    app_loop
        .dispatch(AppEvent::SelectContext(TopLevelContext::Patch))
        .unwrap();

    let mut focus_order = vec![app_loop.current_patch_page().unwrap().focused_control_id()];
    let mut patch_control_cases_exercised = 0;
    let mut fine_coarse_adjustment_exact = true;
    let mut scalar_only_publication = true;

    for descriptor in VoiceEnvelope::surface_descriptor() {
        app_loop
            .dispatch(AppEvent::Navigate(Direction::Down))
            .unwrap();
        let control_id = PatchControlId::Envelope(descriptor.parameter());
        let page = app_loop.current_patch_page().unwrap();
        focus_order.push(page.focused_control_id());
        patch_control_cases_exercised += 1;
        fine_coarse_adjustment_exact &= page.focused_control_id() == control_id;

        let baseline = envelope.value(descriptor.parameter());
        for (direction, expected) in [
            (Direction::Right, baseline + descriptor.fine_step()),
            (
                Direction::Up,
                baseline + descriptor.fine_step() + descriptor.coarse_step(),
            ),
            (Direction::Down, baseline + descriptor.fine_step()),
            (Direction::Left, baseline),
        ] {
            app_loop.dispatch(AppEvent::Adjust(direction)).unwrap();
            let canonical = app_loop.patches()[0]
                .envelope()
                .value(descriptor.parameter());
            let projected = app_loop
                .current_parameters()
                .patch(PatchId::new(1).unwrap())
                .unwrap()
                .envelope()
                .value(descriptor.parameter());
            let page_value = app_loop
                .current_patch_page()
                .unwrap()
                .envelope()
                .iter()
                .find(|row| row.control_id() == control_id)
                .unwrap()
                .value();
            fine_coarse_adjustment_exact &=
                canonical == expected && projected == expected && page_value == expected;

            let record = app_loop.event_log_ref().records().last().unwrap();
            scalar_only_publication &= record.emitted_events().len() == 2
                && matches!(
                    record.emitted_events()[0],
                    EmittedEvent::StateAccepted { .. }
                )
                && matches!(
                    record.emitted_events()[1],
                    EmittedEvent::ParameterSnapshotPublished {
                        graph_revision: GraphRevision::INITIAL,
                        ..
                    }
                )
                && app_loop.current_parameters().generation() == record.generation_after()
                && app_loop.current_parameters().graph_revision() == GraphRevision::INITIAL
                && audio.pop_command().is_none();
        }
    }

    let patch_focus_order_exact = focus_order == PatchControlId::surface_descriptor();
    let target_patch_isolated = app_loop.patches()[1] == comparison;

    let mut mixer_patch_mutation_shared = true;
    for (parameter_index, descriptor) in VoiceEnvelope::surface_descriptor().iter().enumerate() {
        let make_state = || {
            let mut state = AppState::new(production_capability_registry().unwrap(), globals());
            state
                .apply(AppEvent::InstallPatches(vec![
                    soundfont_patch(1, 0).with_envelope(envelope),
                    braids_patch(2, 1).with_envelope(envelope),
                ]))
                .unwrap();
            state
        };

        let mut mixer = make_state();
        for _ in 0..(ChannelParameters::surface_descriptor().len() + parameter_index) {
            mixer.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        }
        mixer.apply(AppEvent::Adjust(Direction::Right)).unwrap();

        let mut patch = make_state();
        patch
            .apply(AppEvent::SelectContext(TopLevelContext::Patch))
            .unwrap();
        for _ in 0..=parameter_index {
            patch.apply(AppEvent::Navigate(Direction::Down)).unwrap();
        }
        patch.apply(AppEvent::Adjust(Direction::Right)).unwrap();

        mixer_patch_mutation_shared &= mixer.patches()[0].envelope()
            == patch.patches()[0].envelope()
            && mixer.patches()[0].envelope().value(descriptor.parameter())
                == envelope.value(descriptor.parameter()) + descriptor.fine_step()
            && mixer.patches()[1] == patch.patches()[1];
    }

    (
        patch_control_cases_exercised,
        patch_focus_order_exact,
        fine_coarse_adjustment_exact,
        mixer_patch_mutation_shared,
        scalar_only_publication,
        target_patch_isolated,
    )
}

fn prove_mixed_callback_contract() -> (usize, usize, bool) {
    let registry = production_capability_registry().unwrap();
    let envelope = VoiceEnvelope::new(0.0, 0.0, 1.0, 100.0).unwrap();
    let patches = vec![
        soundfont_patch(1, 0).with_envelope(envelope),
        braids_patch(2, 1).with_envelope(envelope),
    ];
    let mut state = AppState::new(registry.clone(), globals());
    state.apply(AppEvent::InstallPatches(patches)).unwrap();
    let parameters = StateProjector::for_graph(GraphRevision::INITIAL)
        .project(&state)
        .unwrap()
        .2;
    let preparers = production_instrument_preparers().unwrap();
    let graph = PreparedGraphBuilder::new(&registry, &preparers)
        .build(
            GraphRevision::INITIAL,
            state.patches(),
            parameters,
            SAMPLE_RATE,
            BLOCK_FRAMES,
        )
        .unwrap();
    let boundary = LockFreeAudioBoundary::new(16, parameters);
    let (mut control, audio) = boundary.into_handles();
    for patch in state.patches() {
        for key in [60, 67] {
            control
                .push_command(AudioCommand::patch_midi(
                    patch.id(),
                    note(patch, MidiMessageKind::NoteOn, key, 120),
                ))
                .unwrap();
        }
    }
    let mut renderer = AudioRenderer::new(audio, NoStructuralGraphChanges::new(), graph);
    let mut output = [0.0_f32; BLOCK_SAMPLES];
    let (mut allocations, mut deallocations) = counted_render(&mut renderer, &mut output);
    let onset_stems_sound = state.patches().iter().enumerate().all(|(index, patch)| {
        renderer
            .active_patch_audio()
            .stem(index, patch.id())
            .is_some_and(|stem| finite(stem.samples()) && sounding(stem.samples()))
    });

    for patch in state.patches() {
        control
            .push_command(AudioCommand::patch_midi(
                patch.id(),
                note(patch, MidiMessageKind::NoteOff, 60, 0),
            ))
            .unwrap();
    }
    let memory = counted_render(&mut renderer, &mut output);
    allocations += memory.0;
    deallocations += memory.1;
    let unreleased_notes_survive = onset_stems_sound
        && state.patches().iter().enumerate().all(|(index, patch)| {
            renderer
                .active_patch_audio()
                .stem(index, patch.id())
                .is_some_and(|stem| finite(stem.samples()) && sounding(stem.samples()))
        });

    control.push_command(AudioCommand::all_notes_off()).unwrap();
    let memory = counted_render(&mut renderer, &mut output);
    allocations += memory.0;
    deallocations += memory.1;
    let all_notes_off_silent = state.patches().iter().enumerate().all(|(index, patch)| {
        renderer
            .active_patch_audio()
            .stem(index, patch.id())
            .is_some_and(|stem| finite(stem.samples()) && !sounding(stem.samples()))
    });
    assert!(all_notes_off_silent);

    (allocations, deallocations, unreleased_notes_survive)
}

fn counted_render<Boundary, Structural>(
    renderer: &mut AudioRenderer<Boundary, Structural>,
    output: &mut [f32],
) -> (usize, usize)
where
    Boundary: crest_synth::real_time::audio_boundary::AudioThreadBoundary,
    Structural: crest_synth::real_time::structural_graph_boundary::AudioStructuralGraphBoundary,
{
    begin_memory_count();
    renderer.render(output);
    finish_memory_count()
}
