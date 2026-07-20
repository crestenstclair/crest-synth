use crest_synth::adapter::lock_free_audio_boundary::LockFreeAudioBoundary;
use crest_synth::control::app_event::{AppEvent, Direction};
use crest_synth::control::app_loop::AppLoop;
use crest_synth::control::app_state::AppState;
use crest_synth::control::event_log::EventLog;
use crest_synth::control::state_projector::StateProjector;
use crest_synth::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crest_synth::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::mixer::mix_engine::MixEngine;
use crest_synth::real_time::audio_boundary::AudioBoundary;
use crest_synth::real_time::audio_renderer::AudioRenderer;
use crest_synth::real_time::parameter_snapshot::ParameterSnapshot;
use crest_synth::real_time::patch_audio_block::PatchAudioBlock;
use crest_synth::synth::patch::Patch;
use crest_synth::synth::sound_font_engine::{SoundFontEngine, SoundFontError};
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::testing::automatic_midi_test::AutomaticMidiTest;
use crest_synth::testing::demo_scene::DemoScene;
use crest_synth::testing::demo_scene_report::DemoSceneReport;
use crest_synth::testing::instrument_part::InstrumentPart;
use crest_synth::testing::midi_event_source::{
    FixedEventBatch, MidiEventSource, MidiSourceError, TargetedMidiEvent,
};
use crest_synth::testing::ExhaustiveGuiDemo;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

pub const FRAME_COUNT: usize = 32;
pub const SAMPLE_RATE: f32 = 48_000.0;

pub struct DemoRun {
    pub report: DemoSceneReport,
    #[allow(dead_code)]
    pub baseline: Value,
    #[allow(dead_code)]
    pub expected_coverage: Vec<String>,
}

pub fn globals() -> GlobalParameters {
    GlobalParameters::new(0.0, 0.5, 0.4, 0.35, 250.0, 0.3, 0.25)
        .expect("fixture global parameters are valid")
}

fn parts() -> Vec<InstrumentPart> {
    vec![
        InstrumentPart::new(
            0,
            "Fixture Lead".to_owned(),
            SoundFontInstrument::new(0, 8, false).expect("fixture instrument is valid"),
        ),
        InstrumentPart::new(
            1,
            "Fixture Pad".to_owned(),
            SoundFontInstrument::new(0, 48, false).expect("fixture instrument is valid"),
        ),
    ]
}

fn scene_patches() -> Vec<Patch> {
    parts()
        .into_iter()
        .enumerate()
        .map(|(index, part)| {
            let patch_id = crest_synth::kernel::patch_id::PatchId::new(index as u32 + 1)
                .expect("fixture PatchId is valid");
            Patch::new(
                patch_id,
                part.name().to_owned(),
                part.instrument(),
                part.assigned_channel(),
                crest_synth::mixer::channel_parameters::ChannelParameters::default(),
            )
        })
        .collect()
}

pub struct FixtureMidiSource {
    parts: Vec<InstrumentPart>,
    emitted: bool,
    started: bool,
}

impl FixtureMidiSource {
    pub fn new() -> Self {
        Self {
            parts: parts(),
            emitted: false,
            started: false,
        }
    }
}

impl MidiEventSource for FixtureMidiSource {
    fn prepare(&mut self) -> Result<Vec<InstrumentPart>, MidiSourceError> {
        Ok(self.parts.clone())
    }

    fn start(&mut self) {
        self.started = true;
    }

    fn poll(
        &mut self,
        elapsed: Duration,
        output: &mut FixedEventBatch,
    ) -> Result<(), MidiSourceError> {
        if self.started && !self.emitted && !elapsed.is_zero() {
            let channel = self.parts[0].assigned_channel();
            let message = MidiMessage::try_new(channel, MidiMessageKind::NoteOn, 60, 96)
                .expect("fixture MIDI bytes are valid");
            output.try_push(TargetedMidiEvent::new(0, message))?;
            self.emitted = true;
        }
        Ok(())
    }

    fn finished(&self) -> bool {
        self.emitted
    }
}

pub struct FixtureEngine;

impl SoundFontEngine for FixtureEngine {
    fn load(&mut self, _path: &Path) -> Result<(), SoundFontError> {
        Ok(())
    }

    fn configure_patch(&mut self, _patch: &Patch) -> Result<(), SoundFontError> {
        Ok(())
    }

    fn dispatch(
        &mut self,
        _patch_id: crest_synth::kernel::patch_id::PatchId,
        _message: MidiMessage,
    ) -> Result<(), SoundFontError> {
        Ok(())
    }

    fn all_notes_off(&mut self) {}

    fn render_patches(&mut self, output: &mut PatchAudioBlock, parameters: &ParameterSnapshot) {
        for (index, patch) in parameters.patches().iter().enumerate() {
            let patch_id = patch.patch_id().expect("active parameters carry a PatchId");
            let stem = output
                .stem_mut(index, patch_id)
                .expect("renderer prepared the matching Patch stem");
            let amplitude = 0.15 + index as f32 * 0.11;
            for frame in stem.chunks_exact_mut(2) {
                frame[0] = amplitude;
                frame[1] = amplitude * (1.0 + index as f32 * 0.07);
            }
        }
    }
}

pub struct FixtureEffects;

impl GlobalEffectsProcessor for FixtureEffects {
    fn prepare(
        &mut self,
        _sample_rate: f32,
        _max_frames: usize,
        _max_delay_milliseconds: f32,
    ) -> Result<(), EffectError> {
        Ok(())
    }

    fn process(
        &mut self,
        reverb_input: &[f32],
        delay_input: &[f32],
        output: &mut [f32],
        parameters: &GlobalParameters,
    ) {
        let reverb_shape =
            1.0 + parameters.reverb_room_size() * 0.31 + parameters.reverb_damping() * 0.17;
        let delay_shape =
            1.0 + parameters.delay_milliseconds() * 0.000_7 + parameters.delay_feedback() * 0.23;
        for ((sample, reverb), delay) in output
            .iter_mut()
            .zip(reverb_input.iter())
            .zip(delay_input.iter())
        {
            *sample += reverb * parameters.reverb_return() * reverb_shape
                + delay * parameters.delay_return() * delay_shape;
        }
    }
}

pub fn run_demo() -> DemoRun {
    let patches = scene_patches();
    let global_parameters = globals();
    let scene = DemoScene::exhaustive(&patches, &global_parameters)
        .expect("the fixture contains two discriminating Patches");
    let expected_coverage = scene.expected_coverage().to_vec();
    let initial_parameters =
        ParameterSnapshot::new(0, global_parameters, &[]).expect("initial parameters are valid");
    let boundary = LockFreeAudioBoundary::<()>::new(16, initial_parameters);
    let (control, audio) = boundary.into_handles();
    let event_log = EventLog::new(scene.event_log_capacity().saturating_add(16))
        .expect("fixture EventLog capacity is valid");
    let mut app_loop = AppLoop::with_event_log(
        AppState::new(global_parameters),
        StateProjector::new(),
        control,
        event_log,
    )
    .expect("initial state projects");

    let mut engine = FixtureEngine;
    let mut automatic = AutomaticMidiTest::new(FixtureMidiSource::new());
    automatic
        .initialize(&mut engine, &mut app_loop)
        .expect("automatic fixture initializes through AppLoop");
    automatic
        .tick(Duration::from_millis(10), &mut app_loop)
        .expect("automatic fixture dispatches its due MIDI event");

    app_loop
        .dispatch(AppEvent::Navigate(Direction::Down))
        .expect("System-source navigation is accepted");
    app_loop
        .dispatch(AppEvent::Navigate(Direction::Up))
        .expect("inverse System-source navigation is accepted");

    let baseline: Value = serde_json::from_str(app_loop.current_state_tree().json())
        .expect("baseline StateTree is valid JSON");
    let mut renderer = AudioRenderer::new(audio, engine, MixEngine::new(FixtureEffects));
    renderer
        .prepare(FRAME_COUNT, SAMPLE_RATE)
        .expect("fixture renderer prepares");
    let mut audio_buffer = vec![0.0_f32; FRAME_COUNT * 2];
    let mut demo = ExhaustiveGuiDemo::new(&mut app_loop, &mut renderer, &mut audio_buffer);
    let report = demo
        .run(scene)
        .expect("the deterministic demo returns a diagnostic report");

    DemoRun {
        report,
        baseline,
        expected_coverage,
    }
}
