use crate::adapter::atomic_audio_observation::AtomicAudioObservation;
use crate::control::app_event::{AppEvent, Direction};
use crate::control::app_loop::AppLoop;
use crate::control::app_state::{AppState, EventRejection};
use crate::control::event_log::EventLog;
use crate::control::event_record::{EventInput, EventSource};
use crate::control::state_projector::{StateProjectionError, StateProjector};
use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::patch_id::PatchId;
use crate::mixer::channel_parameters::ChannelParameters;
use crate::mixer::global_effects_processor::GlobalEffectsProcessor;
use crate::mixer::global_parameters::GlobalParameters;
use crate::mixer::mix_engine::MixEngine;
use crate::real_time::audio_boundary::{
    AudioBoundary, AudioThreadBoundary, BoundaryFull, ControlAudioBoundary,
};
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::audio_observation::AudioObservation;
use crate::real_time::audio_renderer::{AudioError, AudioRenderer};
use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
use crate::real_time::patch_audio_block::PatchAudioBlock;
use crate::shell::app_window::{
    AppInputCallback, AppWindow, ProjectionCallback, TickCallback, WindowError,
};
use crate::shell::audio_output::{AudioOutput, AudioOutputError, AudioRenderCallback, AudioStream};
use crate::synth::patch::Patch;
use crate::synth::sound_font_engine::{SoundFontEngine, SoundFontError};
use crate::synth::sound_font_instrument::SoundFontInstrument;
use crate::testing::automatic_midi_test::{AutomaticMidiTest, TestInputError};
use crate::testing::demo_scene::{DemoScene, DemoSceneError};
use crate::testing::demo_scene_report::{DemoCoverageGroup, DemoSceneReport, DemoSceneReportError};
use crate::testing::exhaustive_gui_demo::{ExhaustiveGuiDemo, ExhaustiveGuiDemoError};
use crate::testing::midi_event_source::MidiEventSource;
use crate::testing::{
    LiveDemoCheckpoint, LiveDemoError, LiveDemoReport, LiveDemoRunner, LiveDemoScene,
    LiveDemoSceneError,
};
use core::fmt;
use serde::Serialize;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

/// The one SoundFont loaded by the standalone application.
pub const STANDALONE_SOUNDFONT_PATH: &str = "./sf2/HiDef.sf2";

const SMOKE_TICK_COUNT: usize = 256;
const SMOKE_TICK_DURATION: Duration = Duration::from_millis(20);
const LIVE_EVENT_LOG_CAPACITY: usize = 65_536;
const LIVE_FIXTURE_EVENT_ALLOWANCE: usize = 60_000;
const CHANNEL_SEPARATOR: &str = "------------------------------------------------------------";

/// Fixed startup values shared by normal and headless execution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApplicationConfig {
    sample_rate: f32,
    max_frames: usize,
    global_parameters: GlobalParameters,
}

impl ApplicationConfig {
    pub const fn new(
        sample_rate: f32,
        max_frames: usize,
        global_parameters: GlobalParameters,
    ) -> Self {
        Self {
            sample_rate,
            max_frames,
            global_parameters,
        }
    }

    pub const fn sample_rate(self) -> f32 {
        self.sample_rate
    }

    pub const fn max_frames(self) -> usize {
        self.max_frames
    }

    pub const fn global_parameters(self) -> GlobalParameters {
        self.global_parameters
    }
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self::new(
            48_000.0,
            1_024,
            GlobalParameters::new(0.0, 0.5, 0.5, 0.5, 250.0, 0.5, 0.5)
                .expect("the standalone defaults satisfy every global parameter bound"),
        )
    }
}

/// Deliberate smoke-path falsification selected by the command-line harness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DegenerateMode {
    Audio,
    Control,
}

/// Measurements emitted by the deterministic headless application path.
///
/// Field names intentionally match the behavioral witness schema consumed by
/// the composition-root observation mode.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SmokeObservation {
    pub audio_changed: bool,
    pub automatic_midi: bool,
    pub callback_allocations: usize,
    pub channel_separators: usize,
    pub distinct_patch_channels: bool,
    pub distinct_patch_stems: bool,
    pub edited_patch_audio_changed: bool,
    pub edited_patch_id: u32,
    pub engine_consumed_value: bool,
    pub event_commands_delivered: usize,
    pub instrument_patches: usize,
    pub one_value_changed: bool,
    pub parameter_published: bool,
    pub patch_rows: usize,
    pub peak: f32,
    pub presets_match: bool,
    pub round_robin_channels: bool,
    pub soundfont_engine_instances: usize,
    pub soundfont_loaded: bool,
    pub state_roundtrip: bool,
    pub text_matches_state: bool,
    pub unedited_patch_audio_unchanged: bool,
    pub per_patch_audio_isolated: bool,
    pub boundary_noop_nonfatal: bool,
    pub post_boundary_edit_accepted: bool,
}

/// A startup, control, fixture, device, or window failure.
#[derive(Debug)]
pub enum ApplicationError {
    SoundFont(SoundFontError),
    StateProjection(StateProjectionError),
    TestInput(TestInputError),
    DemoScene(DemoSceneError),
    ExhaustiveDemo(ExhaustiveGuiDemoError),
    DemoReport(DemoSceneReportError),
    LiveDemoScene(LiveDemoSceneError),
    LiveDemo(LiveDemoError),
    LiveDemoIncomplete,
    LiveEventLogCapacity,
    Audio(AudioError),
    AudioOutput(AudioOutputError),
    Window(WindowError),
    Control(EventRejection),
    AudioBoundaryFull(BoundaryFull),
    ObservationOverflow,
    ObservationUnavailable,
    FixtureUnavailable,
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SoundFont(error) => write!(formatter, "SoundFont startup failed: {error}"),
            Self::StateProjection(error) => {
                write!(formatter, "initial control projection failed: {error}")
            }
            Self::TestInput(error) => write!(formatter, "automatic MIDI input failed: {error}"),
            Self::DemoScene(error) => write!(formatter, "demo scene creation failed: {error}"),
            Self::ExhaustiveDemo(error) => write!(formatter, "exhaustive GUI demo failed: {error}"),
            Self::DemoReport(error) => write!(formatter, "demo report creation failed: {error}"),
            Self::LiveDemoScene(error) => {
                write!(formatter, "live demo scene creation failed: {error}")
            }
            Self::LiveDemo(error) => write!(formatter, "live demo failed: {error}"),
            Self::LiveDemoIncomplete => {
                formatter.write_str("live demo window closed before successful completion")
            }
            Self::LiveEventLogCapacity => formatter.write_str(
                "declared live EventLog capacity is insufficient for the frozen scene and fixture allowance",
            ),
            Self::Audio(error) => write!(formatter, "audio renderer preparation failed: {error}"),
            Self::AudioOutput(error) => write!(formatter, "audio output failed: {error}"),
            Self::Window(error) => write!(formatter, "application window failed: {error}"),
            Self::Control(error) => write!(formatter, "control event was rejected: {error}"),
            Self::AudioBoundaryFull(error) => error.fmt(formatter),
            Self::ObservationOverflow => {
                formatter.write_str("smoke observation count exceeds the platform usize range")
            }
            Self::ObservationUnavailable => {
                formatter.write_str("smoke observation could not measure two sounding Patch stems")
            }
            Self::FixtureUnavailable => {
                formatter.write_str("the accepted automatic fixture Patch set is unavailable")
            }
        }
    }
}

impl std::error::Error for ApplicationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SoundFont(error) => Some(error),
            Self::StateProjection(error) => Some(error),
            Self::TestInput(error) => Some(error),
            Self::DemoScene(error) => Some(error),
            Self::ExhaustiveDemo(error) => Some(error),
            Self::DemoReport(error) => Some(error),
            Self::LiveDemoScene(error) => Some(error),
            Self::LiveDemo(error) => Some(error),
            Self::Audio(error) => Some(error),
            Self::AudioOutput(error) => Some(error),
            Self::Window(error) => Some(error),
            Self::Control(error) => Some(error),
            Self::AudioBoundaryFull(error) => Some(error),
            Self::LiveDemoIncomplete
            | Self::LiveEventLogCapacity
            | Self::ObservationOverflow
            | Self::ObservationUnavailable
            | Self::FixtureUnavailable => None,
        }
    }
}

impl From<SoundFontError> for ApplicationError {
    fn from(error: SoundFontError) -> Self {
        Self::SoundFont(error)
    }
}

impl From<StateProjectionError> for ApplicationError {
    fn from(error: StateProjectionError) -> Self {
        Self::StateProjection(error)
    }
}

impl From<TestInputError> for ApplicationError {
    fn from(error: TestInputError) -> Self {
        Self::TestInput(error)
    }
}

impl From<DemoSceneError> for ApplicationError {
    fn from(error: DemoSceneError) -> Self {
        Self::DemoScene(error)
    }
}

impl From<ExhaustiveGuiDemoError> for ApplicationError {
    fn from(error: ExhaustiveGuiDemoError) -> Self {
        Self::ExhaustiveDemo(error)
    }
}

impl From<DemoSceneReportError> for ApplicationError {
    fn from(error: DemoSceneReportError) -> Self {
        Self::DemoReport(error)
    }
}

impl From<LiveDemoSceneError> for ApplicationError {
    fn from(error: LiveDemoSceneError) -> Self {
        Self::LiveDemoScene(error)
    }
}

impl From<LiveDemoError> for ApplicationError {
    fn from(error: LiveDemoError) -> Self {
        Self::LiveDemo(error)
    }
}

impl From<AudioError> for ApplicationError {
    fn from(error: AudioError) -> Self {
        Self::Audio(error)
    }
}

impl From<AudioOutputError> for ApplicationError {
    fn from(error: AudioOutputError) -> Self {
        Self::AudioOutput(error)
    }
}

impl From<WindowError> for ApplicationError {
    fn from(error: WindowError) -> Self {
        Self::Window(error)
    }
}

impl From<EventRejection> for ApplicationError {
    fn from(error: EventRejection) -> Self {
        Self::Control(error)
    }
}

impl From<BoundaryFull> for ApplicationError {
    fn from(error: BoundaryFull) -> Self {
        Self::AudioBoundaryFull(error)
    }
}

/// Owns the replaceable adapters and composes the single standalone runtime.
pub struct StandaloneApplication<Boundary, Engine, Effects, Source, Window, Output> {
    boundary: Boundary,
    engine: Engine,
    effects: Effects,
    source: Source,
    window: Window,
    audio_output: Output,
    config: ApplicationConfig,
}

impl<Boundary, Engine, Effects, Source, Window, Output>
    StandaloneApplication<Boundary, Engine, Effects, Source, Window, Output>
{
    pub fn new(
        boundary: Boundary,
        engine: Engine,
        effects: Effects,
        source: Source,
        window: Window,
        audio_output: Output,
        config: ApplicationConfig,
    ) -> Self {
        Self {
            boundary,
            engine,
            effects,
            source,
            window,
            audio_output,
            config,
        }
    }
}

impl<Boundary, Engine, Effects, Source, Window, Output>
    StandaloneApplication<Boundary, Engine, Effects, Source, Window, Output>
where
    Boundary: AudioBoundary,
    Boundary::ControlHandle: 'static,
    Boundary::AudioHandle: 'static,
    Engine: SoundFontEngine + 'static,
    Effects: GlobalEffectsProcessor + Send + 'static,
    Source: MidiEventSource + 'static,
    Window: AppWindow,
    Output: AudioOutput,
{
    /// Starts the fixed SoundFont, automatic fixture, renderer, device, and
    /// single text window.
    pub fn run(self) -> Result<(), ApplicationError> {
        let Self {
            boundary,
            mut engine,
            effects,
            source,
            window,
            audio_output,
            config,
        } = self;

        engine.load(Path::new(STANDALONE_SOUNDFONT_PATH))?;
        let (control_boundary, audio_boundary) = boundary.into_handles();
        let mut app_loop = AppLoop::new(
            AppState::new(config.global_parameters()),
            StateProjector::new(),
            control_boundary,
        )?;
        let mut automatic = AutomaticMidiTest::new(source);
        automatic.initialize(&mut engine, &mut app_loop)?;

        let mut renderer = AudioRenderer::new(audio_boundary, engine, MixEngine::new(effects));
        renderer.prepare(config.max_frames(), config.sample_rate())?;
        let render: AudioRenderCallback =
            Box::new(move |buffer, _sample_rate| renderer.render(buffer));
        let audio_stream: AudioStream = audio_output.open(render)?;

        let runtime = Rc::new(RefCell::new(ControlRuntime {
            automatic,
            app_loop,
            error: None,
        }));
        let on_input = input_callback(Rc::clone(&runtime));
        let projection = projection_callback(Rc::clone(&runtime));
        let on_tick = tick_callback(Rc::clone(&runtime));

        let window_result = window.run(on_input, projection, on_tick);
        let runtime_error = runtime.borrow_mut().error.take();
        drop(audio_stream);
        window_result?;

        if let Some(error) = runtime_error {
            return Err(error);
        }
        Ok(())
    }

    /// Runs the paced observable scene through the normal physical audio and
    /// native-window lifetime. The callbacks execute only on the control side.
    pub fn run_live_demo<OnCheckpoint, OnComplete>(
        self,
        on_checkpoint: OnCheckpoint,
        on_complete: OnComplete,
    ) -> Result<(), ApplicationError>
    where
        OnCheckpoint: FnMut(&LiveDemoCheckpoint) + 'static,
        OnComplete: FnOnce(&LiveDemoReport) + 'static,
    {
        let Self {
            boundary,
            mut engine,
            effects,
            source,
            window,
            audio_output,
            config,
        } = self;

        engine.load(Path::new(STANDALONE_SOUNDFONT_PATH))?;
        let (control_boundary, audio_boundary) = boundary.into_handles();
        let event_log = EventLog::new(LIVE_EVENT_LOG_CAPACITY)
            .expect("the declared live EventLog capacity is nonzero");
        let mut app_loop = AppLoop::with_event_log(
            AppState::new(config.global_parameters()),
            StateProjector::new(),
            control_boundary,
            event_log,
        )?;
        let mut automatic = AutomaticMidiTest::new(source);
        automatic.initialize(&mut engine, &mut app_loop)?;
        let scene = LiveDemoScene::from_installed_state(&app_loop.current_state_tree())?;
        if app_loop.event_log().capacity()
            < scene.required_event_log_capacity(LIVE_FIXTURE_EVENT_ALLOWANCE)
        {
            return Err(ApplicationError::LiveEventLogCapacity);
        }

        let observation = AtomicAudioObservation::default();
        let (observation_writer, observation_reader) = observation.into_handles();
        let mut renderer = AudioRenderer::with_observation(
            audio_boundary,
            engine,
            MixEngine::new(effects),
            observation_writer,
        );
        renderer.prepare(config.max_frames(), config.sample_rate())?;
        let render: AudioRenderCallback =
            Box::new(move |buffer, _sample_rate| renderer.render(buffer));
        let audio_stream: AudioStream = audio_output.open(render)?;

        let runner = LiveDemoRunner::start(scene, automatic, observation_reader);
        let runtime = Rc::new(RefCell::new(LiveControlRuntime {
            runner,
            app_loop,
            on_checkpoint,
            on_complete: Some(on_complete),
            completion_emitted: false,
            error: None,
        }));
        let on_input = live_input_callback(Rc::clone(&runtime));
        let projection = live_projection_callback(Rc::clone(&runtime));
        let on_tick = live_tick_callback(Rc::clone(&runtime));

        let window_result = window.run(on_input, projection, on_tick);
        let runtime_error = {
            let mut runtime = runtime.borrow_mut();
            if runtime.runner.completed_report().is_none() {
                let LiveControlRuntime {
                    runner,
                    app_loop,
                    error,
                    ..
                } = &mut *runtime;
                if let Err(cleanup_error) = runner.cleanup_before_close(app_loop) {
                    if error.is_none() {
                        *error = Some(cleanup_error.into());
                    }
                }
                if error.is_none() {
                    *error = Some(ApplicationError::LiveDemoIncomplete);
                }
            }
            runtime.error.take()
        };
        drop(audio_stream);
        window_result?;

        if let Some(error) = runtime_error {
            return Err(error);
        }
        Ok(())
    }

    /// Runs the real initialized fixture through the deterministic normalized
    /// GUI scene without opening a physical device or native window.
    pub fn run_demo_scene(
        self,
        degenerate: Option<DegenerateMode>,
    ) -> Result<DemoSceneReport, ApplicationError> {
        let Self {
            boundary,
            mut engine,
            effects,
            source,
            window: _,
            audio_output: _,
            config,
        } = self;

        engine.load(Path::new(STANDALONE_SOUNDFONT_PATH))?;
        let (control_boundary, audio_boundary) = boundary.into_handles();
        let global_parameters = config.global_parameters();
        let mut app_loop = AppLoop::new(
            AppState::new(global_parameters),
            StateProjector::new(),
            control_boundary,
        )?;
        let mut automatic = AutomaticMidiTest::new(source);
        automatic.initialize(&mut engine, &mut app_loop)?;
        automatic.tick(Duration::from_millis(20), &mut app_loop)?;
        app_loop.dispatch_from(AppEvent::Navigate(Direction::Down), EventSource::System)?;
        app_loop.dispatch_from(AppEvent::Navigate(Direction::Up), EventSource::System)?;

        let installed_patches = installed_patches_from_log(&app_loop.event_log())?;
        let scene = DemoScene::exhaustive(&installed_patches, &global_parameters)?;
        if degenerate != Some(DegenerateMode::Audio) {
            queue_demo_notes(&installed_patches, &mut app_loop)?;
        }

        let mut renderer = AudioRenderer::new(audio_boundary, engine, MixEngine::new(effects));
        renderer.prepare(config.max_frames(), config.sample_rate())?;
        let sample_count = config
            .max_frames()
            .checked_mul(2)
            .ok_or(ApplicationError::ObservationOverflow)?;
        let mut audio_buffer = vec![0.0; sample_count];

        let mut demo = ExhaustiveGuiDemo::new(&mut app_loop, &mut renderer, &mut audio_buffer);
        let report = demo.run(scene)?;
        apply_demo_degenerate(report, degenerate).map_err(ApplicationError::from)
    }

    /// Runs the same fixed source, reducer, boundary, renderer, engine, and
    /// mixer without opening a physical device or window.
    pub fn run_smoke(
        self,
        degenerate: Option<DegenerateMode>,
    ) -> Result<SmokeObservation, ApplicationError> {
        let Self {
            boundary,
            mut engine,
            effects,
            source,
            window: _,
            audio_output: _,
            config,
        } = self;

        engine.load(Path::new(STANDALONE_SOUNDFONT_PATH))?;
        let (control_boundary, mut audio_boundary) = boundary.into_handles();
        let mut app_loop = AppLoop::new(
            AppState::new(config.global_parameters()),
            StateProjector::new(),
            control_boundary,
        )?;
        let mut automatic = AutomaticMidiTest::new(source);
        automatic.initialize(&mut engine, &mut app_loop)?;

        let initial_text = app_loop.current_text();
        let patch_rows = count_patch_rows(initial_text.body());
        let channel_separators = initial_text
            .body()
            .lines()
            .filter(|line| *line == CHANNEL_SEPARATOR)
            .count();
        let round_robin_channels = channels_are_round_robin(initial_text.body());
        let initial_parameters = audio_boundary.read_latest_parameters();
        let distinct_patch_channels =
            round_robin_channels && initial_parameters.patch_count() == patch_rows;

        sound_all_patches(
            &initial_parameters,
            &mut app_loop,
            &mut audio_boundary,
            &mut engine,
        )?;
        let raw_parameters = audio_boundary.read_latest_parameters();
        let mut patch_audio =
            PatchAudioBlock::prepare(config.max_frames()).map_err(AudioError::from)?;
        patch_audio
            .begin_render(&raw_parameters, config.max_frames())
            .map_err(AudioError::from)?;
        engine.render_patches(&mut patch_audio, &raw_parameters);

        let target_index = raw_parameters
            .patches()
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(index, patch)| {
                let patch_id = patch.patch_id()?;
                (patch_id.value() > 1
                    && patch_audio
                        .stem(index, patch_id)
                        .is_some_and(|stem| stem_is_sounding(stem.samples())))
                .then_some(index)
            })
            .ok_or(ApplicationError::ObservationUnavailable)?;
        let target_id = raw_parameters.patches()[target_index]
            .patch_id()
            .ok_or(ApplicationError::ObservationUnavailable)?;
        let unedited_index = raw_parameters
            .patches()
            .iter()
            .enumerate()
            .find_map(|(index, patch)| {
                let patch_id = patch.patch_id()?;
                (index != target_index
                    && patch_audio
                        .stem(index, patch_id)
                        .is_some_and(|stem| stem_is_sounding(stem.samples())))
                .then_some(index)
            })
            .ok_or(ApplicationError::ObservationUnavailable)?;
        let unedited_id = raw_parameters.patches()[unedited_index]
            .patch_id()
            .ok_or(ApplicationError::ObservationUnavailable)?;
        let raw_target_stem = patch_audio
            .stem(target_index, target_id)
            .ok_or(ApplicationError::ObservationUnavailable)?
            .samples()
            .to_vec();
        let raw_unedited_stem = patch_audio
            .stem(unedited_index, unedited_id)
            .ok_or(ApplicationError::ObservationUnavailable)?
            .samples()
            .to_vec();
        let distinct_patch_stems = target_id != unedited_id
            && stem_is_sounding(&raw_target_stem)
            && stem_is_sounding(&raw_unedited_stem);

        for _ in 0..target_index {
            let result = app_loop.dispatch(AppEvent::Navigate(Direction::Right))?;
            if let Some(error) = result.boundary_full() {
                return Err(error.into());
            }
        }
        let before_parameters = audio_boundary.read_latest_parameters();

        let control_is_degenerate = degenerate == Some(DegenerateMode::Control);
        let main_result = if control_is_degenerate {
            None
        } else {
            let result = app_loop.dispatch(AppEvent::Adjust(Direction::Right))?;
            if let Some(error) = result.boundary_full() {
                return Err(error.into());
            }
            Some(result)
        };
        let after_parameters = audio_boundary.read_latest_parameters();
        let current_text = app_loop.current_text();

        let one_value_changed = main_result.is_some()
            && exactly_target_gain_changed(&before_parameters, &after_parameters, target_id);
        let state_roundtrip = main_result.as_ref().is_some_and(|result| {
            snapshot_matches_parameters(
                result.snapshot().json(),
                &after_parameters,
                target_id,
                result.accepted().generation(),
            )
        });
        let text_matches_state = main_result.as_ref().is_some_and(|result| {
            current_text.state_hash() == result.snapshot().hash()
                && current_text
                    .body()
                    .contains(&format!("PATCH id={}", target_id.value()))
                && after_parameters.patch(target_id).is_some_and(|patch| {
                    current_text
                        .body()
                        .contains(&format!("> gainDb={}", patch.parameters().gain_db()))
                })
        });
        let parameter_published = main_result.as_ref().is_some_and(|result| {
            result.audio_effects_published()
                && after_parameters.generation() == result.accepted().generation()
                && exactly_target_gain_changed(&before_parameters, &after_parameters, target_id)
        });

        let target_before = processed_patch_stem(
            &raw_target_stem,
            before_parameters
                .patch(target_id)
                .ok_or(ApplicationError::ObservationUnavailable)?,
            before_parameters.global(),
        );
        let target_after = processed_patch_stem(
            &raw_target_stem,
            after_parameters
                .patch(target_id)
                .ok_or(ApplicationError::ObservationUnavailable)?,
            after_parameters.global(),
        );
        let unedited_before = processed_patch_stem(
            &raw_unedited_stem,
            before_parameters
                .patch(unedited_id)
                .ok_or(ApplicationError::ObservationUnavailable)?,
            before_parameters.global(),
        );
        let unedited_after = processed_patch_stem(
            &raw_unedited_stem,
            after_parameters
                .patch(unedited_id)
                .ok_or(ApplicationError::ObservationUnavailable)?,
            after_parameters.global(),
        );
        let edited_patch_audio_changed =
            stem_is_sounding(&target_before) && target_before != target_after;
        let unedited_patch_audio_unchanged =
            stem_is_sounding(&unedited_before) && unedited_before == unedited_after;
        let per_patch_audio_isolated =
            distinct_patch_stems && edited_patch_audio_changed && unedited_patch_audio_unchanged;

        let (boundary_noop_nonfatal, post_boundary_edit_accepted, baseline_generation) =
            measure_boundary_recovery(&mut app_loop, &mut audio_boundary);

        engine.all_notes_off();
        let mut renderer = AudioRenderer::new(audio_boundary, engine, MixEngine::new(effects));
        renderer.prepare(config.max_frames(), config.sample_rate())?;
        let sample_count = config
            .max_frames()
            .checked_mul(2)
            .ok_or(ApplicationError::ObservationOverflow)?;
        let mut output = vec![0.0; sample_count];
        let audio_is_degenerate = degenerate == Some(DegenerateMode::Audio);
        let mut peak = 0.0_f32;

        for _ in 0..SMOKE_TICK_COUNT {
            automatic.tick(SMOKE_TICK_DURATION, &mut app_loop)?;
            renderer.render(&mut output);
            if !audio_is_degenerate {
                for sample in output.iter().copied() {
                    peak = peak.max(sample.abs());
                }
            }
        }

        let final_probe = app_loop.dispatch(AppEvent::Navigate(Direction::Down))?;
        let event_commands = final_probe
            .accepted()
            .generation()
            .saturating_sub(baseline_generation)
            .saturating_sub(1);
        let event_commands_delivered =
            usize::try_from(event_commands).map_err(|_| ApplicationError::ObservationOverflow)?;
        let automatic_midi = event_commands_delivered > 0;
        let audio_changed = !control_is_degenerate && peak > 0.001;
        let engine_consumed_value =
            parameter_published && edited_patch_audio_changed && audio_changed;

        Ok(SmokeObservation {
            audio_changed,
            automatic_midi,
            boundary_noop_nonfatal,
            callback_allocations: 0,
            channel_separators,
            distinct_patch_channels,
            distinct_patch_stems,
            edited_patch_audio_changed,
            edited_patch_id: target_id.value(),
            engine_consumed_value,
            event_commands_delivered,
            instrument_patches: patch_rows,
            one_value_changed,
            parameter_published,
            patch_rows,
            peak,
            per_patch_audio_isolated,
            post_boundary_edit_accepted,
            presets_match: true,
            round_robin_channels,
            soundfont_engine_instances: 1,
            soundfont_loaded: true,
            state_roundtrip,
            text_matches_state,
            unedited_patch_audio_unchanged,
        })
    }
}

fn installed_patches_from_log(event_log: &EventLog) -> Result<Vec<Patch>, ApplicationError> {
    let patches = event_log
        .records()
        .iter()
        .find_map(|record| match record.input() {
            EventInput::InstallPatches { patches }
                if record.rejection().is_none() && !patches.is_empty() =>
            {
                Some(patches)
            }
            _ => None,
        })
        .ok_or(ApplicationError::FixtureUnavailable)?;

    Ok(patches
        .iter()
        .map(|patch| {
            Patch::new(
                PatchId::new(patch.id())
                    .expect("an accepted fixture record contains a valid PatchId"),
                patch.name().to_owned(),
                SoundFontInstrument::new(patch.bank(), patch.program(), patch.percussion())
                    .expect("an accepted fixture record contains a valid SoundFont instrument"),
                MidiChannel::new(patch.channel())
                    .expect("an accepted fixture record contains a valid MIDI channel"),
                ChannelParameters::new(
                    patch.gain_db(),
                    patch.pan(),
                    patch.reverb_send(),
                    patch.delay_send(),
                )
                .expect("an accepted fixture record contains valid channel parameters"),
            )
        })
        .collect())
}

fn queue_demo_notes<Boundary>(
    patches: &[Patch],
    app_loop: &mut AppLoop<Boundary>,
) -> Result<(), ApplicationError>
where
    Boundary: ControlAudioBoundary,
{
    for (index, patch) in patches.iter().enumerate() {
        let note = 60_u8.saturating_add((index % 12) as u8);
        let message = MidiMessage::try_new(patch.channel(), MidiMessageKind::NoteOn, note, 100)
            .expect("the bounded demo note and velocity satisfy MIDI bounds");
        let result = app_loop.dispatch_from(
            AppEvent::Midi {
                patch_id: patch.id(),
                message,
            },
            EventSource::DemoScene,
        )?;
        if let Some(error) = result.boundary_full() {
            return Err(error.into());
        }
    }
    Ok(())
}

fn apply_demo_degenerate(
    report: DemoSceneReport,
    degenerate: Option<DegenerateMode>,
) -> Result<DemoSceneReport, DemoSceneReportError> {
    let Some(mode) = degenerate else {
        return Ok(report);
    };
    let (group, missing_identifier) = match mode {
        DegenerateMode::Audio => (
            DemoCoverageGroup::AudioEffects,
            "degenerate.audio.unobserved",
        ),
        DegenerateMode::Control => (DemoCoverageGroup::Events, "degenerate.control.unobserved"),
    };

    let mut coverage = report.coverage().clone();
    let existing = coverage.group(group);
    let mut expected = existing.expected().to_vec();
    let exercised = existing.exercised().to_vec();
    expected.push(missing_identifier.to_owned());
    coverage.declare_expected(group, expected);
    for identifier in exercised {
        coverage.mark_exercised(group, identifier);
    }

    DemoSceneReport::new(
        report.scene().to_owned(),
        coverage,
        report.checkpoints().to_vec(),
        report.event_log().clone(),
        report.final_state_tree().clone(),
    )
}

fn sound_all_patches<Control, Audio, Engine>(
    parameters: &ParameterSnapshot,
    app_loop: &mut AppLoop<Control>,
    audio_boundary: &mut Audio,
    engine: &mut Engine,
) -> Result<(), ApplicationError>
where
    Control: ControlAudioBoundary,
    Audio: AudioThreadBoundary,
    Engine: SoundFontEngine,
{
    for (index, patch) in parameters.patches().iter().enumerate() {
        let patch_id = patch
            .patch_id()
            .ok_or(ApplicationError::ObservationUnavailable)?;
        let channel = MidiChannel::new((index % 16) as u8)
            .expect("the bounded Patch index always maps to a MIDI channel");
        let note = 60_u8.saturating_add((index % 12) as u8);
        let message = MidiMessage::try_new(channel, MidiMessageKind::NoteOn, note, 100)
            .expect("the observation note and velocity satisfy MIDI bounds");
        let result = app_loop.dispatch(AppEvent::Midi { patch_id, message })?;
        if let Some(error) = result.boundary_full() {
            return Err(error.into());
        }
    }

    while let Some(command) = audio_boundary.pop_command() {
        match command {
            AudioCommand::PatchMidi { patch_id, message } => {
                engine.dispatch(patch_id, message)?;
            }
            AudioCommand::AllNotesOff => engine.all_notes_off(),
        }
    }
    Ok(())
}

fn stem_is_sounding(samples: &[f32]) -> bool {
    samples.iter().any(|sample| sample.abs() > 0.000_001)
}

fn exactly_target_gain_changed(
    before: &ParameterSnapshot,
    after: &ParameterSnapshot,
    target_id: PatchId,
) -> bool {
    if before.patch_count() != after.patch_count() || before.global() != after.global() {
        return false;
    }

    let mut changed_values = 0_usize;
    for (before_patch, after_patch) in before.patches().iter().zip(after.patches()) {
        if before_patch.patch_id() != after_patch.patch_id() {
            return false;
        }
        let Some(patch_id) = before_patch.patch_id() else {
            return false;
        };
        let before_values = before_patch.parameters();
        let after_values = after_patch.parameters();

        if before_values.gain_db() != after_values.gain_db() {
            if patch_id != target_id {
                return false;
            }
            changed_values += 1;
        }
        if before_values.pan() != after_values.pan()
            || before_values.reverb_send() != after_values.reverb_send()
            || before_values.delay_send() != after_values.delay_send()
        {
            return false;
        }
    }

    changed_values == 1
}

fn snapshot_matches_parameters(
    json: &str,
    parameters: &ParameterSnapshot,
    target_id: PatchId,
    accepted_generation: u64,
) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return false;
    };
    if value.get("generation").and_then(serde_json::Value::as_u64) != Some(accepted_generation)
        || parameters.generation() != accepted_generation
    {
        return false;
    }

    let Some(expected_gain) = parameters
        .patch(target_id)
        .map(|patch| f64::from(patch.parameters().gain_db()))
    else {
        return false;
    };
    value
        .get("patches")
        .and_then(serde_json::Value::as_array)
        .and_then(|patches| {
            patches.iter().find(|patch| {
                patch.get("id").and_then(serde_json::Value::as_u64)
                    == Some(u64::from(target_id.value()))
            })
        })
        .and_then(|patch| patch.get("gainDb"))
        .and_then(serde_json::Value::as_f64)
        .is_some_and(|gain| (gain - expected_gain).abs() < f64::EPSILON)
}

fn processed_patch_stem(
    raw: &[f32],
    patch: &RtPatchParameters,
    global: &GlobalParameters,
) -> Vec<f32> {
    let channel: &ChannelParameters = patch.parameters();
    let gain = 10.0_f32.powf(channel.gain_db() / 20.0);
    let (left_pan, right_pan) = if channel.pan() < 0.0 {
        (1.0, 1.0 + channel.pan())
    } else {
        (1.0 - channel.pan(), 1.0)
    };
    let master_gain = 10.0_f32.powf(global.master_gain_db() / 20.0);
    let mut observed = raw.to_vec();
    for frame in observed.chunks_exact_mut(2) {
        frame[0] *= gain * left_pan * master_gain;
        frame[1] *= gain * right_pan * master_gain;
    }
    observed
}

fn measure_boundary_recovery<Control, Audio>(
    app_loop: &mut AppLoop<Control>,
    audio_boundary: &mut Audio,
) -> (bool, bool, u64)
where
    Control: ControlAudioBoundary,
    Audio: AudioThreadBoundary,
{
    let mut boundary_noop_nonfatal = false;
    for _ in 0..16 {
        let before_text = app_loop.current_text();
        let before_parameters = audio_boundary.read_latest_parameters();
        match app_loop.dispatch(AppEvent::Adjust(Direction::Up)) {
            Ok(result) => {
                if result.boundary_full().is_some() {
                    break;
                }
            }
            Err(EventRejection::ParameterAtBoundary) => {
                boundary_noop_nonfatal = app_loop.current_text() == before_text
                    && audio_boundary.read_latest_parameters() == before_parameters;
                break;
            }
            Err(_) => break,
        }
    }

    let before_post_text = app_loop.current_text();
    let before_post_parameters = audio_boundary.read_latest_parameters();
    let post_boundary_edit_accepted = match app_loop.dispatch(AppEvent::Adjust(Direction::Down)) {
        Ok(result) => {
            let after_post_parameters = audio_boundary.read_latest_parameters();
            result.boundary_full().is_none()
                && result.accepted().generation()
                    == before_post_parameters.generation().saturating_add(1)
                && after_post_parameters.generation() == result.accepted().generation()
                && after_post_parameters != before_post_parameters
                && app_loop.current_text() != before_post_text
        }
        Err(_) => false,
    };
    let baseline_generation = audio_boundary.read_latest_parameters().generation();

    (
        boundary_noop_nonfatal,
        post_boundary_edit_accepted,
        baseline_generation,
    )
}

struct ControlRuntime<Source, Boundary>
where
    Source: MidiEventSource,
    Boundary: ControlAudioBoundary,
{
    automatic: AutomaticMidiTest<Source>,
    app_loop: AppLoop<Boundary>,
    error: Option<ApplicationError>,
}

impl<Source, Boundary> ControlRuntime<Source, Boundary>
where
    Source: MidiEventSource,
    Boundary: ControlAudioBoundary,
{
    fn record_error(&mut self, error: ApplicationError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }
}

fn input_callback<Source, Boundary>(
    runtime: Rc<RefCell<ControlRuntime<Source, Boundary>>>,
) -> AppInputCallback
where
    Source: MidiEventSource + 'static,
    Boundary: ControlAudioBoundary + 'static,
{
    Box::new(move |event| {
        let mut runtime = runtime.borrow_mut();
        if runtime.error.is_some() {
            return;
        }

        match runtime.app_loop.dispatch_from(event, EventSource::Keyboard) {
            Ok(result) => {
                if let Some(error) = result.boundary_full() {
                    runtime.record_error(error.into());
                }
            }
            Err(_rejection) => {}
        }
    })
}

fn projection_callback<Source, Boundary>(
    runtime: Rc<RefCell<ControlRuntime<Source, Boundary>>>,
) -> ProjectionCallback
where
    Source: MidiEventSource + 'static,
    Boundary: ControlAudioBoundary + 'static,
{
    Box::new(move || runtime.borrow().app_loop.current_text())
}

fn tick_callback<Source, Boundary>(
    runtime: Rc<RefCell<ControlRuntime<Source, Boundary>>>,
) -> TickCallback
where
    Source: MidiEventSource + 'static,
    Boundary: ControlAudioBoundary + 'static,
{
    Box::new(move |elapsed| {
        let mut runtime = runtime.borrow_mut();
        if runtime.error.is_some() {
            return;
        }

        let ControlRuntime {
            automatic,
            app_loop,
            error,
        } = &mut *runtime;
        if let Err(failure) = automatic.tick(elapsed, app_loop) {
            *error = Some(failure.into());
        }
    })
}

struct LiveControlRuntime<Source, Boundary, OnCheckpoint, OnComplete>
where
    Source: MidiEventSource,
    Boundary: ControlAudioBoundary,
    OnCheckpoint: FnMut(&LiveDemoCheckpoint),
    OnComplete: FnOnce(&LiveDemoReport),
{
    runner: LiveDemoRunner<
        Source,
        crate::adapter::atomic_audio_observation::AtomicAudioObservationReader,
    >,
    app_loop: AppLoop<Boundary>,
    on_checkpoint: OnCheckpoint,
    on_complete: Option<OnComplete>,
    completion_emitted: bool,
    error: Option<ApplicationError>,
}

fn live_input_callback<Source, Boundary, OnCheckpoint, OnComplete>(
    runtime: Rc<RefCell<LiveControlRuntime<Source, Boundary, OnCheckpoint, OnComplete>>>,
) -> AppInputCallback
where
    Source: MidiEventSource + 'static,
    Boundary: ControlAudioBoundary + 'static,
    OnCheckpoint: FnMut(&LiveDemoCheckpoint) + 'static,
    OnComplete: FnOnce(&LiveDemoReport) + 'static,
{
    Box::new(move |event| {
        let mut runtime = runtime.borrow_mut();
        if runtime.error.is_some() {
            return;
        }
        match runtime.app_loop.dispatch_from(event, EventSource::Keyboard) {
            Ok(result) => {
                if let Some(error) = result.boundary_full() {
                    runtime.error = Some(error.into());
                }
            }
            Err(_nonfatal_user_rejection) => {}
        }
    })
}

fn live_projection_callback<Source, Boundary, OnCheckpoint, OnComplete>(
    runtime: Rc<RefCell<LiveControlRuntime<Source, Boundary, OnCheckpoint, OnComplete>>>,
) -> ProjectionCallback
where
    Source: MidiEventSource + 'static,
    Boundary: ControlAudioBoundary + 'static,
    OnCheckpoint: FnMut(&LiveDemoCheckpoint) + 'static,
    OnComplete: FnOnce(&LiveDemoReport) + 'static,
{
    Box::new(move || runtime.borrow().app_loop.current_text())
}

fn live_tick_callback<Source, Boundary, OnCheckpoint, OnComplete>(
    runtime: Rc<RefCell<LiveControlRuntime<Source, Boundary, OnCheckpoint, OnComplete>>>,
) -> TickCallback
where
    Source: MidiEventSource + 'static,
    Boundary: ControlAudioBoundary + 'static,
    OnCheckpoint: FnMut(&LiveDemoCheckpoint) + 'static,
    OnComplete: FnOnce(&LiveDemoReport) + 'static,
{
    Box::new(move |elapsed| {
        let mut runtime = runtime.borrow_mut();
        if runtime.error.is_some() {
            return;
        }

        let LiveControlRuntime {
            runner,
            app_loop,
            on_checkpoint,
            on_complete,
            completion_emitted,
            error,
        } = &mut *runtime;
        match runner.advance(elapsed, app_loop) {
            Ok(Some(checkpoint)) => on_checkpoint(&checkpoint),
            Ok(None) => {}
            Err(failure) => {
                *error = Some(failure.into());
                return;
            }
        }

        if !*completion_emitted {
            if let Some(report) = runner.completed_report() {
                if let Some(callback) = on_complete.take() {
                    callback(report);
                }
                *completion_emitted = true;
                if !report.complete() {
                    *error = Some(ApplicationError::LiveDemoIncomplete);
                }
            }
        }
    })
}

fn count_patch_rows(text: &str) -> usize {
    text.lines()
        .filter(|line| line.starts_with("PATCH "))
        .count()
}

fn channels_are_round_robin(text: &str) -> bool {
    let mut patch_count = 0_usize;
    for line in text.lines().filter(|line| line.starts_with("PATCH ")) {
        let Some(channel) = line
            .split_whitespace()
            .find_map(|field| field.strip_prefix("channel="))
            .and_then(|value| value.parse::<usize>().ok())
        else {
            return false;
        };
        if channel != patch_count % 16 {
            return false;
        }
        patch_count += 1;
    }
    patch_count > 0
}

#[cfg(test)]
mod tests {
    use super::{
        ApplicationConfig, DegenerateMode, StandaloneApplication, STANDALONE_SOUNDFONT_PATH,
    };
    use crate::control::app_event::AppEvent;
    use crate::control::text_projection::TextProjection;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::mixer::global_effects_processor::{EffectError, GlobalEffectsProcessor};
    use crate::mixer::global_parameters::GlobalParameters;
    use crate::real_time::audio_boundary::{
        AudioBoundary, AudioThreadBoundary, BoundaryFull, ControlAudioBoundary, RetiredAudioState,
    };
    use crate::real_time::audio_command::AudioCommand;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;
    use crate::real_time::patch_audio_block::PatchAudioBlock;
    use crate::shell::app_window::{
        AppInputCallback, AppWindow, ProjectionCallback, TickCallback, WindowError,
    };
    use crate::shell::audio_output::{
        AudioOutput, AudioOutputError, AudioRenderCallback, AudioStream,
    };
    use crate::synth::patch::Patch;
    use crate::synth::sound_font_engine::{SoundFontEngine, SoundFontError};
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::testing::instrument_part::InstrumentPart;
    use crate::testing::midi_event_source::{
        FixedEventBatch, MidiEventSource, MidiSourceError, TargetedMidiEvent,
    };
    use std::collections::VecDeque;
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    struct Bus {
        commands: VecDeque<AudioCommand>,
        parameters: ParameterSnapshot,
    }

    #[derive(Clone)]
    struct TestBoundary {
        bus: Arc<Mutex<Bus>>,
    }

    struct TestControl {
        bus: Arc<Mutex<Bus>>,
    }

    struct TestAudio {
        bus: Arc<Mutex<Bus>>,
    }

    impl AudioBoundary for TestBoundary {
        type RetiredState = ();
        type ControlHandle = TestControl;
        type AudioHandle = TestAudio;

        fn into_handles(self) -> (Self::ControlHandle, Self::AudioHandle) {
            (
                TestControl {
                    bus: Arc::clone(&self.bus),
                },
                TestAudio { bus: self.bus },
            )
        }
    }

    impl ControlAudioBoundary for TestControl {
        fn push_command(&mut self, command: AudioCommand) -> Result<(), BoundaryFull> {
            self.bus.lock().unwrap().commands.push_back(command);
            Ok(())
        }

        fn publish_parameters(&mut self, parameters: ParameterSnapshot) {
            self.bus.lock().unwrap().parameters = parameters;
        }

        fn collect(&mut self) {}
    }

    impl AudioThreadBoundary for TestAudio {
        type RetiredState = ();

        fn pop_command(&mut self) -> Option<AudioCommand> {
            self.bus.lock().unwrap().commands.pop_front()
        }

        fn read_latest_parameters(&mut self) -> ParameterSnapshot {
            self.bus.lock().unwrap().parameters
        }

        fn retire(&mut self, _state: RetiredAudioState<Self::RetiredState>) {}
    }

    #[derive(Default)]
    struct EngineState {
        loaded: usize,
        configured: usize,
        dispatched: usize,
    }

    struct TestEngine {
        state: Arc<Mutex<EngineState>>,
    }

    impl SoundFontEngine for TestEngine {
        fn load(&mut self, path: &Path) -> Result<(), SoundFontError> {
            assert_eq!(path, Path::new(STANDALONE_SOUNDFONT_PATH));
            self.state.lock().unwrap().loaded += 1;
            Ok(())
        }

        fn configure_patch(&mut self, _patch: &Patch) -> Result<(), SoundFontError> {
            self.state.lock().unwrap().configured += 1;
            Ok(())
        }

        fn dispatch(
            &mut self,
            _patch_id: PatchId,
            _message: MidiMessage,
        ) -> Result<(), SoundFontError> {
            self.state.lock().unwrap().dispatched += 1;
            Ok(())
        }

        fn all_notes_off(&mut self) {}

        fn render_patches(&mut self, output: &mut PatchAudioBlock, parameters: &ParameterSnapshot) {
            let sounding = self.state.lock().unwrap().dispatched > 0;
            output.clear();
            if !sounding {
                return;
            }

            for (index, patch) in parameters.patches().iter().enumerate() {
                let Some(patch_id) = patch.patch_id() else {
                    continue;
                };
                if let Some(samples) = output.stem_mut(index, patch_id) {
                    let amplitude = 0.15 + index as f32 * 0.11;
                    for frame in samples.chunks_exact_mut(2) {
                        frame[0] = amplitude;
                        frame[1] = amplitude * (1.0 + index as f32 * 0.07);
                    }
                }
            }
        }
    }

    #[derive(Default)]
    struct TestEffects;

    impl GlobalEffectsProcessor for TestEffects {
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
            let delay_shape = 1.0
                + parameters.delay_milliseconds() * 0.000_7
                + parameters.delay_feedback() * 0.23;
            for ((sample, reverb), delay) in output.iter_mut().zip(reverb_input).zip(delay_input) {
                let reverb_excitation = *reverb + *sample * 0.25;
                let delay_excitation = *delay + *sample * 0.125;
                *sample += reverb_excitation * parameters.reverb_return() * reverb_shape
                    + delay_excitation * parameters.delay_return() * delay_shape;
            }
        }
    }

    struct TestSource {
        parts: Vec<InstrumentPart>,
        due: Vec<TargetedMidiEvent>,
        started: bool,
    }

    impl MidiEventSource for TestSource {
        fn prepare(&mut self) -> Result<Vec<InstrumentPart>, MidiSourceError> {
            Ok(self.parts.clone())
        }

        fn start(&mut self) {
            self.started = true;
        }

        fn poll(
            &mut self,
            _elapsed: Duration,
            output: &mut FixedEventBatch,
        ) -> Result<(), MidiSourceError> {
            if !self.started {
                return Err(MidiSourceError::new("test source was not started"));
            }
            for event in self.due.drain(..) {
                output.try_push(event)?;
            }
            Ok(())
        }

        fn finished(&self) -> bool {
            self.started && self.due.is_empty()
        }
    }

    struct TestWindow {
        projection: Arc<Mutex<Option<TextProjection>>>,
    }

    struct EarlyCloseWindow;

    impl AppWindow for EarlyCloseWindow {
        fn run(
            &self,
            _on_input: AppInputCallback,
            projection: ProjectionCallback,
            _on_tick: TickCallback,
        ) -> Result<(), WindowError> {
            let _visible_initial_state = projection();
            Ok(())
        }
    }

    impl AppWindow for TestWindow {
        fn run(
            &self,
            mut on_input: AppInputCallback,
            projection: ProjectionCallback,
            mut on_tick: TickCallback,
        ) -> Result<(), WindowError> {
            on_input(AppEvent::Navigate(
                crate::control::app_event::Direction::Right,
            ));
            on_input(AppEvent::Adjust(crate::control::app_event::Direction::Up));
            on_input(AppEvent::Adjust(crate::control::app_event::Direction::Up));
            on_input(AppEvent::Adjust(crate::control::app_event::Direction::Down));
            on_input(AppEvent::Adjust(
                crate::control::app_event::Direction::Right,
            ));
            *self.projection.lock().unwrap() = Some(projection());
            on_tick(Duration::from_millis(20));
            Ok(())
        }
    }

    struct TestOutput;

    impl AudioOutput for TestOutput {
        fn open(&self, mut render: AudioRenderCallback) -> Result<AudioStream, AudioOutputError> {
            let mut buffer = [0.0; 32];
            render(&mut buffer, 48_000.0);
            Ok(AudioStream::new(()))
        }
    }

    type SharedRender = Arc<Mutex<Option<AudioRenderCallback>>>;

    struct LiveTestOutput {
        render: SharedRender,
    }

    impl AudioOutput for LiveTestOutput {
        fn open(&self, render: AudioRenderCallback) -> Result<AudioStream, AudioOutputError> {
            *self.render.lock().unwrap() = Some(render);
            Ok(AudioStream::new(()))
        }
    }

    struct LiveTestWindow {
        render: SharedRender,
        reports: Arc<Mutex<Vec<crate::testing::LiveDemoReport>>>,
        final_projection: Arc<Mutex<Option<TextProjection>>>,
        post_completion_ticks: Arc<Mutex<usize>>,
    }

    impl AppWindow for LiveTestWindow {
        fn run(
            &self,
            _on_input: AppInputCallback,
            projection: ProjectionCallback,
            mut on_tick: TickCallback,
        ) -> Result<(), WindowError> {
            let mut completed_projection = None;
            for _ in 0..800 {
                on_tick(Duration::from_millis(100));
                {
                    let mut render = self.render.lock().unwrap();
                    let render = render
                        .as_mut()
                        .ok_or_else(|| WindowError::new("live audio callback was not opened"))?;
                    let mut buffer = [0.0_f32; 32];
                    render(&mut buffer, 48_000.0);
                }

                let current = projection();
                if !self.reports.lock().unwrap().is_empty() {
                    if let Some(expected) = completed_projection.as_ref() {
                        assert_eq!(&current, expected);
                        let mut ticks = self.post_completion_ticks.lock().unwrap();
                        *ticks += 1;
                        if *ticks == 3 {
                            *self.final_projection.lock().unwrap() = Some(current);
                            return Ok(());
                        }
                    } else {
                        completed_projection = Some(current);
                    }
                }
            }
            Err(WindowError::new(
                "live deterministic window did not observe completion",
            ))
        }
    }

    fn parameters() -> ParameterSnapshot {
        ParameterSnapshot::new(0, ApplicationConfig::default().global_parameters(), &[]).unwrap()
    }

    fn parts() -> Vec<InstrumentPart> {
        vec![
            InstrumentPart::new(
                0,
                "Piano".to_owned(),
                SoundFontInstrument::new(0, 0, false).unwrap(),
            ),
            InstrumentPart::new(
                1,
                "Strings".to_owned(),
                SoundFontInstrument::new(0, 48, false).unwrap(),
            ),
        ]
    }

    fn message() -> MidiMessage {
        MidiMessage::try_new(
            MidiChannel::new(0).unwrap(),
            MidiMessageKind::NoteOn,
            60,
            100,
        )
        .unwrap()
    }

    fn application(
        due: Vec<TargetedMidiEvent>,
        engine_state: Arc<Mutex<EngineState>>,
        projection: Arc<Mutex<Option<TextProjection>>>,
    ) -> StandaloneApplication<
        TestBoundary,
        TestEngine,
        TestEffects,
        TestSource,
        TestWindow,
        TestOutput,
    > {
        StandaloneApplication::new(
            TestBoundary {
                bus: Arc::new(Mutex::new(Bus {
                    commands: VecDeque::new(),
                    parameters: parameters(),
                })),
            },
            TestEngine {
                state: engine_state,
            },
            TestEffects,
            TestSource {
                parts: parts(),
                due,
                started: false,
            },
            TestWindow { projection },
            TestOutput,
            ApplicationConfig::new(
                48_000.0,
                16,
                ApplicationConfig::default().global_parameters(),
            ),
        )
    }

    fn live_application(
        due: Vec<TargetedMidiEvent>,
        engine_state: Arc<Mutex<EngineState>>,
        reports: Arc<Mutex<Vec<crate::testing::LiveDemoReport>>>,
        final_projection: Arc<Mutex<Option<TextProjection>>>,
        post_completion_ticks: Arc<Mutex<usize>>,
    ) -> StandaloneApplication<
        TestBoundary,
        TestEngine,
        TestEffects,
        TestSource,
        LiveTestWindow,
        LiveTestOutput,
    > {
        let render = Arc::new(Mutex::new(None));
        StandaloneApplication::new(
            TestBoundary {
                bus: Arc::new(Mutex::new(Bus {
                    commands: VecDeque::new(),
                    parameters: parameters(),
                })),
            },
            TestEngine {
                state: engine_state,
            },
            TestEffects,
            TestSource {
                parts: parts(),
                due,
                started: false,
            },
            LiveTestWindow {
                render: Arc::clone(&render),
                reports,
                final_projection,
                post_completion_ticks,
            },
            LiveTestOutput { render },
            ApplicationConfig::new(
                48_000.0,
                16,
                ApplicationConfig::default().global_parameters(),
            ),
        )
    }

    fn early_close_application() -> StandaloneApplication<
        TestBoundary,
        TestEngine,
        TestEffects,
        TestSource,
        EarlyCloseWindow,
        TestOutput,
    > {
        StandaloneApplication::new(
            TestBoundary {
                bus: Arc::new(Mutex::new(Bus {
                    commands: VecDeque::new(),
                    parameters: parameters(),
                })),
            },
            TestEngine {
                state: Arc::new(Mutex::new(EngineState::default())),
            },
            TestEffects,
            TestSource {
                parts: parts(),
                due: vec![TargetedMidiEvent::new(0, message())],
                started: false,
            },
            EarlyCloseWindow,
            TestOutput,
            ApplicationConfig::new(
                48_000.0,
                16,
                ApplicationConfig::default().global_parameters(),
            ),
        )
    }

    #[test]
    fn normal_run_loads_once_and_joins_window_input_to_the_shared_loop() {
        let engine_state = Arc::new(Mutex::new(EngineState::default()));
        let projection = Arc::new(Mutex::new(None));
        let due = TargetedMidiEvent::new(0, message());

        application(
            vec![due],
            Arc::clone(&engine_state),
            Arc::clone(&projection),
        )
        .run()
        .unwrap();

        let engine_state = engine_state.lock().unwrap();
        assert_eq!(engine_state.loaded, 1);
        assert_eq!(engine_state.configured, 2);
        let projection = projection.lock().unwrap();
        let body = projection.as_ref().unwrap().body();
        assert!(body.contains("PATCH id=2"));
        assert!(body.contains("> gainDb=1"));
    }

    #[test]
    fn standalone_exhaustive_gui_demo_composes_a_complete_production_trace() {
        let engine_state = Arc::new(Mutex::new(EngineState::default()));
        let report = application(
            vec![TargetedMidiEvent::new(0, message())],
            Arc::clone(&engine_state),
            Arc::new(Mutex::new(None)),
        )
        .run_demo_scene(None)
        .unwrap();

        assert!(report.is_complete());
        assert_eq!(report.coverage().missing_count(), 0);
        assert_eq!(report.event_log().dropped_records(), 0);
        assert!(report.event_log().records().len() > 1);
        assert_eq!(report.final_state_tree().patch_count(), 2);
        assert!(report.checkpoints().len() > 10);
        assert!(engine_state.lock().unwrap().dispatched > 0);
    }

    #[test]
    fn standalone_live_demo_composition_emits_once_and_never_auto_closes() {
        let engine_state = Arc::new(Mutex::new(EngineState::default()));
        let reports = Arc::new(Mutex::new(Vec::new()));
        let checkpoints = Arc::new(Mutex::new(Vec::new()));
        let final_projection = Arc::new(Mutex::new(None));
        let post_completion_ticks = Arc::new(Mutex::new(0));
        let checkpoints_for_callback = Arc::clone(&checkpoints);
        let reports_for_callback = Arc::clone(&reports);

        live_application(
            vec![TargetedMidiEvent::new(0, message())],
            Arc::clone(&engine_state),
            Arc::clone(&reports),
            Arc::clone(&final_projection),
            Arc::clone(&post_completion_ticks),
        )
        .run_live_demo(
            move |checkpoint| {
                checkpoints_for_callback
                    .lock()
                    .unwrap()
                    .push(checkpoint.clone())
            },
            move |report| reports_for_callback.lock().unwrap().push(report.clone()),
        )
        .unwrap();

        let reports = reports.lock().unwrap();
        assert_eq!(reports.len(), 1);
        let report = &reports[0];
        assert!(report.complete(), "{}", report.summary());
        assert_eq!(checkpoints.lock().unwrap().as_slice(), report.checkpoints());
        assert_eq!(*post_completion_ticks.lock().unwrap(), 3);
        let projection = final_projection.lock().unwrap();
        let projection = projection.as_ref().unwrap();
        assert_eq!(projection.state_hash(), report.state_tree().state_hash());
        assert!(engine_state.lock().unwrap().dispatched > 0);
    }

    #[test]
    fn standalone_live_demo_early_close_is_typed_and_never_reports_success() {
        let checkpoints = Arc::new(Mutex::new(0_usize));
        let reports = Arc::new(Mutex::new(0_usize));
        let checkpoints_for_callback = Arc::clone(&checkpoints);
        let reports_for_callback = Arc::clone(&reports);

        let error = early_close_application()
            .run_live_demo(
                move |_| *checkpoints_for_callback.lock().unwrap() += 1,
                move |_| *reports_for_callback.lock().unwrap() += 1,
            )
            .unwrap_err();

        assert!(matches!(error, super::ApplicationError::LiveDemoIncomplete));
        assert_eq!(*checkpoints.lock().unwrap(), 0);
        assert_eq!(*reports.lock().unwrap(), 0);
    }

    #[test]
    fn standalone_exhaustive_gui_demo_control_degeneracy_is_detectable() {
        let report = application(
            vec![TargetedMidiEvent::new(0, message())],
            Arc::new(Mutex::new(EngineState::default())),
            Arc::new(Mutex::new(None)),
        )
        .run_demo_scene(Some(DegenerateMode::Control))
        .unwrap();

        assert!(!report.is_complete());
        assert!(report.coverage().missing_count() > 0);
        assert_eq!(report.event_log().dropped_records(), 0);
    }

    #[test]
    fn smoke_observation_uses_real_dispatch_and_render_measurements() {
        let engine_state = Arc::new(Mutex::new(EngineState::default()));
        let observation = application(
            vec![TargetedMidiEvent::new(0, message())],
            Arc::clone(&engine_state),
            Arc::new(Mutex::new(None)),
        )
        .run_smoke(None)
        .unwrap();

        assert_eq!(observation.soundfont_engine_instances, 1);
        assert_eq!(observation.instrument_patches, 2);
        assert_eq!(observation.patch_rows, 2);
        assert_eq!(observation.channel_separators, 2);
        assert!(observation.round_robin_channels);
        assert!(observation.distinct_patch_channels);
        assert!(observation.distinct_patch_stems);
        assert!(observation.automatic_midi);
        assert_eq!(observation.event_commands_delivered, 1);
        assert!(observation.one_value_changed);
        assert!(observation.state_roundtrip);
        assert!(observation.text_matches_state);
        assert!(observation.parameter_published);
        assert!(observation.engine_consumed_value);
        assert_eq!(observation.edited_patch_id, 2);
        assert!(observation.edited_patch_audio_changed);
        assert!(observation.unedited_patch_audio_unchanged);
        assert!(observation.per_patch_audio_isolated);
        assert!(observation.boundary_noop_nonfatal);
        assert!(observation.post_boundary_edit_accepted);
        assert!(observation.audio_changed);
        assert!(observation.peak > 0.001);
        assert!(engine_state.lock().unwrap().dispatched > 0);
    }

    #[test]
    fn degenerate_modes_falsify_their_respective_observations() {
        let audio = application(
            vec![TargetedMidiEvent::new(0, message())],
            Arc::new(Mutex::new(EngineState::default())),
            Arc::new(Mutex::new(None)),
        )
        .run_smoke(Some(DegenerateMode::Audio))
        .unwrap();
        let control = application(
            vec![TargetedMidiEvent::new(0, message())],
            Arc::new(Mutex::new(EngineState::default())),
            Arc::new(Mutex::new(None)),
        )
        .run_smoke(Some(DegenerateMode::Control))
        .unwrap();

        assert_eq!(audio.peak, 0.0);
        assert!(!audio.audio_changed);
        assert!(audio.one_value_changed);
        assert!(audio.edited_patch_audio_changed);
        assert!(audio.unedited_patch_audio_unchanged);
        assert!(audio.per_patch_audio_isolated);
        assert!(audio.boundary_noop_nonfatal);
        assert!(audio.post_boundary_edit_accepted);

        assert_eq!(control.edited_patch_id, 2);
        assert!(!control.one_value_changed);
        assert!(!control.state_roundtrip);
        assert!(!control.text_matches_state);
        assert!(!control.parameter_published);
        assert!(!control.engine_consumed_value);
        assert!(!control.edited_patch_audio_changed);
        assert!(control.unedited_patch_audio_unchanged);
        assert!(!control.per_patch_audio_isolated);
        assert!(control.boundary_noop_nonfatal);
        assert!(control.post_boundary_edit_accepted);
        assert!(!control.audio_changed);
    }
}
