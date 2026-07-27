use crate::adapter::threaded_graph_preparation_worker::{
    ThreadedGraphPreparationWorker, ThreadedGraphPreparationWorkerError,
};
use crate::control::app_event::{AppEvent, Direction};
use crate::control::app_loop::{AppLoop, StructuralAdvanceError};
use crate::control::app_state::{AppState, EventRejection};
use crate::control::event_log::EventLog;
use crate::control::event_record::{EventInput, EventSource};
use crate::control::state_projector::{StateProjectionError, StateProjector};
use crate::kernel::midi_channel::MidiChannel;
use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
use crate::kernel::patch_id::PatchId;
use crate::mixer::channel_parameters::ChannelParameters;
use crate::mixer::global_parameters::GlobalParameters;
use crate::real_time::audio_boundary::{
    AudioBoundary, AudioThreadBoundary, BoundaryFull, ControlAudioBoundary,
};
use crate::real_time::audio_command::AudioCommand;
use crate::real_time::audio_observation::{AudioObservation, ControlAudioObservation};
use crate::real_time::audio_renderer::AudioRenderer;
use crate::real_time::graph_revision::GraphRevision;
use crate::real_time::parameter_snapshot::{ParameterSnapshot, RtPatchParameters};
use crate::real_time::prepared_graph::PreparedGraph;
use crate::real_time::prepared_graph_builder::{GraphPreparationError, PreparedGraphBuilder};
use crate::real_time::structural_graph_boundary::{
    AudioStructuralGraphBoundary, StructuralGraphBoundary,
};
use crate::shell::app_window::{
    AppInputCallback, AppWindow, ProjectionCallback, TickCallback, WindowError,
};
use crate::shell::audio_device_status::{AudioDeviceStatusBoundary, AudioDeviceStatusReader};
use crate::shell::audio_output::{
    AudioDeviceConfig, AudioDeviceRuntimeError, AudioDeviceStatusCallback, AudioOutput,
    AudioOutputError, AudioRenderCallback, AudioSampleFormat, AudioStream, NegotiatedAudioOutput,
};
use crate::synth::instrument_capability::{CapabilityError, CapabilityRegistry};
use crate::synth::instrument_capability_provider::InstrumentCapabilityProvider;
use crate::synth::instrument_composition::{
    compose_instrument_registry, InstrumentCompositionError,
};
use crate::synth::instrument_preparer::{InstrumentPreparationError, InstrumentPreparer};
use crate::synth::patch::Patch;
use crate::synth::{
    compose_effect_registry, DescriptorDefaultConfigFactory, EffectCapabilityProvider,
    EffectCapabilityRegistry, EffectCompositionError, EffectPreparer, VoicePolicy,
};
use crate::testing::automatic_midi_test::{AutomaticMidiTest, TestInputError};
use crate::testing::demo_scene::{DemoScene, DemoSceneError};
use crate::testing::demo_scene_report::{DemoCoverageGroup, DemoSceneReport, DemoSceneReportError};
use crate::testing::exhaustive_gui_demo::{ExhaustiveGuiDemo, ExhaustiveGuiDemoError};
use crate::testing::midi_event_source::MidiEventSource;
use crate::testing::{
    DeterministicGraphPreparationHandle, DeterministicGraphPreparationWorker, LiveCheckpoint,
    LiveDemoError, LiveDemoReport, LiveDemoRunner, LiveDemoScene, LiveDemoSceneError,
    RuntimeAudioWitness,
};
use core::fmt;
use serde::Serialize;
use std::cell::RefCell;
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
    pub active_graph_revision: u64,
    pub audio_changed: bool,
    pub automatic_midi: bool,
    pub alternating_capabilities: bool,
    pub braids_patches: usize,
    pub callback_allocations: usize,
    pub callback_destructions: usize,
    pub channel_separators: usize,
    pub distinct_patch_channels: bool,
    pub distinct_patch_stems: bool,
    pub edited_patch_audio_changed: bool,
    pub edited_patch_id: u32,
    pub engine_consumed_value: bool,
    pub event_commands_delivered: usize,
    pub parsed_soundfont_banks: usize,
    pub prepared_instruments: usize,
    pub one_value_changed: bool,
    pub parameter_published: bool,
    pub patch_rows: usize,
    pub peak: f32,
    pub presets_match: bool,
    pub round_robin_channels: bool,
    pub soundfont_patches: usize,
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
    Capability(CapabilityError),
    InstrumentComposition(InstrumentCompositionError),
    EffectComposition(EffectCompositionError),
    Instrument(InstrumentPreparationError),
    Graph(GraphPreparationError),
    GraphWorker(ThreadedGraphPreparationWorkerError),
    Structural(StructuralAdvanceError),
    StateProjection(StateProjectionError),
    TestInput(TestInputError),
    DemoScene(DemoSceneError),
    ExhaustiveDemo(ExhaustiveGuiDemoError),
    DemoReport(DemoSceneReportError),
    LiveDemoScene(LiveDemoSceneError),
    LiveDemo(LiveDemoError),
    LiveDemoIncomplete,
    LiveEventLogCapacity,
    AudioOutput(AudioOutputError),
    AudioDeviceRuntime(AudioDeviceRuntimeError),
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
            Self::Capability(error) => write!(formatter, "capability setup failed: {error}"),
            Self::InstrumentComposition(error) => {
                write!(formatter, "instrument composition failed: {error}")
            }
            Self::EffectComposition(error) => {
                write!(formatter, "effect composition failed: {error}")
            }
            Self::Instrument(error) => write!(formatter, "instrument preparation failed: {error}"),
            Self::Graph(error) => write!(formatter, "prepared graph setup failed: {error}"),
            Self::GraphWorker(error) => write!(formatter, "graph worker setup failed: {error}"),
            Self::Structural(error) => write!(formatter, "structural control failed: {error}"),
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
            Self::AudioOutput(error) => write!(formatter, "audio output failed: {error}"),
            Self::AudioDeviceRuntime(error) => {
                write!(formatter, "running audio device failed: {error}")
            }
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
            Self::Capability(error) => Some(error),
            Self::InstrumentComposition(error) => Some(error),
            Self::EffectComposition(error) => Some(error),
            Self::Instrument(error) => Some(error),
            Self::Graph(error) => Some(error),
            Self::GraphWorker(error) => Some(error),
            Self::Structural(error) => Some(error),
            Self::StateProjection(error) => Some(error),
            Self::TestInput(error) => Some(error),
            Self::DemoScene(error) => Some(error),
            Self::ExhaustiveDemo(error) => Some(error),
            Self::DemoReport(error) => Some(error),
            Self::LiveDemoScene(error) => Some(error),
            Self::LiveDemo(error) => Some(error),
            Self::AudioOutput(error) => Some(error),
            Self::AudioDeviceRuntime(error) => Some(error),
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

impl From<InstrumentPreparationError> for ApplicationError {
    fn from(error: InstrumentPreparationError) -> Self {
        Self::Instrument(error)
    }
}

impl From<GraphPreparationError> for ApplicationError {
    fn from(error: GraphPreparationError) -> Self {
        Self::Graph(error)
    }
}

impl From<ThreadedGraphPreparationWorkerError> for ApplicationError {
    fn from(error: ThreadedGraphPreparationWorkerError) -> Self {
        Self::GraphWorker(error)
    }
}

impl From<StructuralAdvanceError> for ApplicationError {
    fn from(error: StructuralAdvanceError) -> Self {
        Self::Structural(error)
    }
}

impl From<CapabilityError> for ApplicationError {
    fn from(error: CapabilityError) -> Self {
        Self::Capability(error)
    }
}

impl From<InstrumentCompositionError> for ApplicationError {
    fn from(error: InstrumentCompositionError) -> Self {
        Self::InstrumentComposition(error)
    }
}

impl From<EffectCompositionError> for ApplicationError {
    fn from(error: EffectCompositionError) -> Self {
        Self::EffectComposition(error)
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

/// Keeps the capability ports and their immutable registry as one exact
/// production composition value.
struct InstrumentRuntimeComposition {
    providers: Vec<Box<dyn InstrumentCapabilityProvider>>,
    capabilities: CapabilityRegistry,
    preparers: Vec<Box<dyn InstrumentPreparer>>,
    effect_providers: Vec<Box<dyn EffectCapabilityProvider>>,
    effects: EffectCapabilityRegistry,
    effect_preparers: Vec<Box<dyn EffectPreparer>>,
}

/// Owns the replaceable adapters and composes the single standalone runtime.
pub struct StandaloneApplication<Boundary, Structural, Observation, Source, Window, Output> {
    boundary: Boundary,
    instruments: InstrumentRuntimeComposition,
    structural: Structural,
    observation: Observation,
    source: Source,
    window: Window,
    audio_output: Output,
    config: ApplicationConfig,
}

impl<Boundary, Structural, Observation, Source, Window, Output>
    StandaloneApplication<Boundary, Structural, Observation, Source, Window, Output>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        boundary: Boundary,
        providers: Vec<Box<dyn InstrumentCapabilityProvider>>,
        preparers: Vec<Box<dyn InstrumentPreparer>>,
        structural: Structural,
        observation: Observation,
        source: Source,
        window: Window,
        audio_output: Output,
        config: ApplicationConfig,
    ) -> Result<Self, ApplicationError> {
        Self::new_with_effects(
            boundary,
            providers,
            preparers,
            Vec::new(),
            Vec::new(),
            structural,
            observation,
            source,
            window,
            audio_output,
            config,
        )
    }

    /// Composes the complete instrument and Patch-effect capability families
    /// once at the application boundary and freezes their exact preparers.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_effects(
        boundary: Boundary,
        providers: Vec<Box<dyn InstrumentCapabilityProvider>>,
        preparers: Vec<Box<dyn InstrumentPreparer>>,
        effect_providers: Vec<Box<dyn EffectCapabilityProvider>>,
        effect_preparers: Vec<Box<dyn EffectPreparer>>,
        structural: Structural,
        observation: Observation,
        source: Source,
        window: Window,
        audio_output: Output,
        config: ApplicationConfig,
    ) -> Result<Self, ApplicationError> {
        let capabilities = compose_instrument_registry(&providers, &preparers)?;
        let effects = compose_effect_registry(&effect_providers, &effect_preparers)?;
        Ok(Self {
            boundary,
            instruments: InstrumentRuntimeComposition {
                providers,
                capabilities,
                preparers,
                effect_providers,
                effects,
                effect_preparers,
            },
            structural,
            observation,
            source,
            window,
            audio_output,
            config,
        })
    }
}

struct PreparedStartup<Control, Audio, StructuralAudio, Source>
where
    Control: ControlAudioBoundary,
    Audio: AudioThreadBoundary,
    StructuralAudio: AudioStructuralGraphBoundary,
    Source: MidiEventSource,
{
    app_loop: AppLoop<Control>,
    automatic: AutomaticMidiTest<Source>,
    audio_boundary: Audio,
    structural_audio: StructuralAudio,
    initial_graph: PreparedGraph,
    deterministic_worker: Option<DeterministicGraphPreparationHandle>,
    parsed_soundfont_banks: usize,
    prepared_instruments: usize,
    capability_composition: CapabilityCompositionObservation,
}

type PreparedStartupFor<Boundary, Structural, Source> = PreparedStartup<
    <Boundary as AudioBoundary>::ControlHandle,
    <Boundary as AudioBoundary>::AudioHandle,
    <Structural as StructuralGraphBoundary>::AudioHandle,
    Source,
>;

struct StartupPlan {
    audio_config: AudioDeviceConfig,
    event_log: Option<EventLog>,
    worker: StartupWorker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartupWorker {
    Threaded,
    Deterministic,
}

fn configured_audio(config: ApplicationConfig) -> AudioDeviceConfig {
    AudioDeviceConfig::new(
        config.sample_rate(),
        2,
        AudioSampleFormat::F32,
        config.max_frames(),
    )
    .expect("the validated standalone configuration defines supported stereo f32 audio")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CapabilityCompositionObservation {
    soundfont_patches: usize,
    braids_patches: usize,
    alternating_capabilities: bool,
}

fn observe_capability_composition(
    capabilities: &CapabilityRegistry,
    patches: &[Patch],
) -> CapabilityCompositionObservation {
    let soundfont_patches = patches
        .iter()
        .filter(|patch| {
            capabilities
                .descriptor(patch.instrument_config().capability_id())
                .is_some_and(|descriptor| descriptor.voice_policy() == VoicePolicy::EngineManaged)
        })
        .count();
    let braids_patches = patches
        .iter()
        .filter(|patch| {
            capabilities
                .descriptor(patch.instrument_config().capability_id())
                .is_some_and(|descriptor| {
                    matches!(descriptor.voice_policy(), VoicePolicy::FixedPerPatch { .. })
                })
        })
        .count();
    let alternating_capabilities = patches.len() > 1
        && soundfont_patches > 0
        && braids_patches > 0
        && patches.windows(2).all(|pair| {
            pair[0].instrument_config().capability_id()
                != pair[1].instrument_config().capability_id()
        });
    CapabilityCompositionObservation {
        soundfont_patches,
        braids_patches,
        alternating_capabilities,
    }
}

fn prepare_startup<Boundary, Structural, Source>(
    boundary: Boundary,
    instruments: InstrumentRuntimeComposition,
    structural: Structural,
    source: Source,
    config: ApplicationConfig,
    plan: StartupPlan,
) -> Result<PreparedStartupFor<Boundary, Structural, Source>, ApplicationError>
where
    Boundary: AudioBoundary,
    Structural: StructuralGraphBoundary,
    Structural::ControlHandle: 'static,
    Source: MidiEventSource,
{
    let InstrumentRuntimeComposition {
        providers,
        capabilities,
        preparers,
        effect_providers,
        effects,
        effect_preparers,
    } = instruments;
    let revision = GraphRevision::INITIAL;
    let (control_boundary, audio_boundary) = boundary.into_handles();
    let state = AppState::for_graph_with_effects(
        capabilities.clone(),
        effects.clone(),
        config.global_parameters(),
        revision,
    );
    let projector = StateProjector::for_graph(revision);
    let mut app_loop = match plan.event_log {
        Some(event_log) => AppLoop::with_event_log(state, projector, control_boundary, event_log)?,
        None => AppLoop::new(state, projector, control_boundary)?,
    };
    let mut automatic = AutomaticMidiTest::new(source);
    automatic.initialize_with_effects(&providers, &effect_providers, &mut app_loop)?;

    let parsed_soundfont_banks = preparers
        .iter()
        .map(|preparer| preparer.prepared_shared_asset_count())
        .sum();
    let initial_graph = PreparedGraphBuilder::new(app_loop.capabilities(), &preparers)
        .with_effects(app_loop.effects(), &effect_preparers)
        .build(
            revision,
            app_loop.patches(),
            *app_loop.current_parameters(),
            plan.audio_config.sample_rate(),
            plan.audio_config.render_capacity_frames(),
        )?;
    let (structural_control, structural_audio) = structural.into_handles();
    let prepared_instruments = initial_graph.engine_rack().patch_count();
    let capability_composition =
        observe_capability_composition(app_loop.capabilities(), app_loop.patches());
    let factory = DescriptorDefaultConfigFactory::new(capabilities.clone(), providers);
    let deterministic_worker = match plan.worker {
        StartupWorker::Threaded => {
            let worker = ThreadedGraphPreparationWorker::new_with_effects(
                capabilities,
                preparers,
                effects,
                effect_preparers,
                plan.audio_config,
            )?;
            app_loop.configure_engine_selection(
                factory,
                worker,
                structural_control,
                &initial_graph,
                plan.audio_config,
            )?;
            None
        }
        StartupWorker::Deterministic => {
            let worker = DeterministicGraphPreparationWorker::new_with_effects(
                capabilities,
                preparers,
                effects,
                effect_preparers,
                plan.audio_config,
            );
            let handle = worker.advance_handle();
            app_loop.configure_engine_selection(
                factory,
                worker,
                structural_control,
                &initial_graph,
                plan.audio_config,
            )?;
            Some(handle)
        }
    };

    Ok(PreparedStartup {
        app_loop,
        automatic,
        audio_boundary,
        structural_audio,
        initial_graph,
        deterministic_worker,
        parsed_soundfont_banks,
        prepared_instruments,
        capability_composition,
    })
}

impl<Boundary, Structural, Observation, Source, Window, Output>
    StandaloneApplication<Boundary, Structural, Observation, Source, Window, Output>
where
    Boundary: AudioBoundary,
    Boundary::ControlHandle: 'static,
    Boundary::AudioHandle: 'static,
    Structural: StructuralGraphBoundary,
    Structural::ControlHandle: 'static,
    Structural::AudioHandle: 'static,
    Observation: AudioObservation,
    Observation::CallbackHandle: 'static,
    Observation::ControlHandle: 'static,
    Source: MidiEventSource + 'static,
    Window: AppWindow,
    Output: AudioOutput,
{
    /// Starts the fixed SoundFont, automatic fixture, renderer, device, and
    /// single text window.
    pub fn run(self) -> Result<(), ApplicationError> {
        let Self {
            boundary,
            instruments,
            structural,
            observation,
            source,
            window,
            audio_output,
            config,
        } = self;

        let negotiated_audio = audio_output.negotiate()?;
        let device_config = negotiated_audio.config();
        let PreparedStartup {
            app_loop,
            mut automatic,
            audio_boundary,
            structural_audio,
            initial_graph,
            ..
        } = prepare_startup(
            boundary,
            instruments,
            structural,
            source,
            config,
            StartupPlan {
                audio_config: device_config,
                event_log: None,
                worker: StartupWorker::Threaded,
            },
        )?;
        let (observation_writer, _observation_reader) = observation.into_handles();
        let mut renderer = AudioRenderer::with_observation(
            audio_boundary,
            structural_audio,
            initial_graph,
            observation_writer,
        );
        let (mut device_status_writer, device_status) =
            AudioDeviceStatusBoundary::new().into_handles();
        let render: AudioRenderCallback = Box::new(move |buffer| renderer.render(buffer));
        let on_runtime_error: AudioDeviceStatusCallback =
            Box::new(move |error| device_status_writer.publish_from_callback(error));
        let audio_stream: AudioStream = negotiated_audio.start(render, on_runtime_error)?;
        automatic.start()?;

        let runtime = Rc::new(RefCell::new(ControlRuntime {
            automatic,
            app_loop,
            device_status,
            error: None,
        }));
        let on_input = input_callback(Rc::clone(&runtime));
        let projection = projection_callback(Rc::clone(&runtime));
        let on_tick = tick_callback(Rc::clone(&runtime));

        let window_result = window.run(on_input, projection, on_tick);
        let runtime_error = runtime.borrow_mut().error.take();
        drop(audio_stream);
        let shutdown_result = runtime
            .borrow_mut()
            .app_loop
            .shutdown_engine_selection_on_control();
        window_result?;
        shutdown_result?;

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
        OnCheckpoint: FnMut(&LiveCheckpoint) + 'static,
        OnComplete: FnOnce(&LiveDemoReport) + 'static,
    {
        let Self {
            boundary,
            instruments,
            structural,
            observation,
            source,
            window,
            audio_output,
            config,
        } = self;
        let event_log = EventLog::new(LIVE_EVENT_LOG_CAPACITY)
            .expect("the declared live EventLog capacity is nonzero");
        let negotiated_audio = audio_output.negotiate()?;
        let device_config = negotiated_audio.config();
        let PreparedStartup {
            app_loop,
            mut automatic,
            audio_boundary,
            structural_audio,
            initial_graph,
            parsed_soundfont_banks,
            prepared_instruments,
            capability_composition,
            ..
        } = prepare_startup(
            boundary,
            instruments,
            structural,
            source,
            config,
            StartupPlan {
                audio_config: device_config,
                event_log: Some(event_log),
                worker: StartupWorker::Threaded,
            },
        )?;
        let scene = LiveDemoScene::from_installed_state(&app_loop.current_state_tree())?;
        if app_loop.event_log().capacity()
            < scene.required_event_log_capacity(LIVE_FIXTURE_EVENT_ALLOWANCE)
        {
            return Err(ApplicationError::LiveEventLogCapacity);
        }

        let (observation_writer, observation_reader) = observation.into_handles();
        let runtime_audio = RuntimeAudioWitness::new(
            parsed_soundfont_banks,
            prepared_instruments,
            capability_composition.soundfont_patches,
            capability_composition.braids_patches,
            capability_composition.alternating_capabilities,
            initial_graph.revision(),
            0,
            0,
        );
        let mut renderer = AudioRenderer::with_observation(
            audio_boundary,
            structural_audio,
            initial_graph,
            observation_writer,
        );
        let (mut device_status_writer, device_status) =
            AudioDeviceStatusBoundary::new().into_handles();
        let render: AudioRenderCallback = Box::new(move |buffer| renderer.render(buffer));
        let on_runtime_error: AudioDeviceStatusCallback =
            Box::new(move |error| device_status_writer.publish_from_callback(error));
        let audio_stream: AudioStream = negotiated_audio.start(render, on_runtime_error)?;
        automatic.start()?;

        let runner = LiveDemoRunner::start(scene, automatic, observation_reader, runtime_audio);
        let runtime = Rc::new(RefCell::new(LiveControlRuntime {
            runner,
            app_loop,
            device_status,
            on_checkpoint,
            on_complete: Some(on_complete),
            completion_emitted: false,
            error: None,
        }));
        let on_input = live_input_sink();
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
        let shutdown_result = runtime
            .borrow_mut()
            .app_loop
            .shutdown_engine_selection_on_control();
        window_result?;
        shutdown_result?;

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
            instruments,
            structural,
            observation,
            source,
            window: _,
            audio_output: _,
            config,
        } = self;
        let global_parameters = config.global_parameters();
        let event_log = EventLog::new(LIVE_EVENT_LOG_CAPACITY)
            .expect("the declared demo EventLog capacity is nonzero");
        let PreparedStartup {
            mut app_loop,
            mut automatic,
            audio_boundary,
            structural_audio,
            initial_graph,
            deterministic_worker,
            ..
        } = prepare_startup(
            boundary,
            instruments,
            structural,
            source,
            config,
            StartupPlan {
                audio_config: configured_audio(config),
                event_log: Some(event_log),
                worker: StartupWorker::Deterministic,
            },
        )?;
        let (observation_writer, observation_reader) = observation.into_handles();
        let mut renderer = AudioRenderer::with_observation(
            audio_boundary,
            structural_audio,
            initial_graph,
            observation_writer,
        );
        automatic.start()?;
        automatic.tick(Duration::from_millis(20), &mut app_loop)?;
        app_loop.advance_structural()?;
        app_loop.dispatch_from(AppEvent::Navigate(Direction::Down), EventSource::System)?;
        app_loop.dispatch_from(AppEvent::Navigate(Direction::Up), EventSource::System)?;

        let installed_patches = installed_patches_from_log(&app_loop.event_log())?;
        let scene = DemoScene::exhaustive_with_effects(
            app_loop.capabilities(),
            app_loop.effects(),
            &installed_patches,
            &global_parameters,
        )?;
        if degenerate != Some(DegenerateMode::Audio) {
            queue_demo_notes(&installed_patches, &mut app_loop)?;
        }

        let sample_count = config
            .max_frames()
            .checked_mul(2)
            .ok_or(ApplicationError::ObservationOverflow)?;
        let mut audio_buffer = vec![0.0; sample_count];

        let mut demo = ExhaustiveGuiDemo::new_with_worker_and_observation(
            &mut app_loop,
            &mut renderer,
            &mut audio_buffer,
            deterministic_worker.expect("deterministic startup returns its worker handle"),
            &observation_reader,
        );
        let report = demo.run(scene)?;
        drop(demo);
        drop(renderer);
        app_loop.shutdown_engine_selection_on_control()?;
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
            instruments,
            structural,
            observation,
            source,
            window: _,
            audio_output: _,
            config,
        } = self;

        let PreparedStartup {
            mut app_loop,
            mut automatic,
            audio_boundary,
            structural_audio,
            initial_graph,
            parsed_soundfont_banks,
            prepared_instruments,
            capability_composition,
            ..
        } = prepare_startup(
            boundary,
            instruments,
            structural,
            source,
            config,
            StartupPlan {
                audio_config: configured_audio(config),
                event_log: None,
                worker: StartupWorker::Threaded,
            },
        )?;
        let (observation_writer, _observation_reader) = observation.into_handles();
        let mut renderer = AudioRenderer::with_observation(
            audio_boundary,
            structural_audio,
            initial_graph,
            observation_writer,
        );
        automatic.start()?;

        let initial_text = app_loop.current_text();
        let patch_rows = count_patch_rows(initial_text.body());
        let channel_separators = initial_text
            .body()
            .lines()
            .filter(|line| *line == CHANNEL_SEPARATOR)
            .count();
        let round_robin_channels = channels_are_round_robin(initial_text.body());
        let initial_parameters = *app_loop.current_parameters();
        let distinct_patch_channels =
            round_robin_channels && initial_parameters.patch_count() == patch_rows;

        sound_all_patches(&initial_parameters, &mut app_loop)?;
        let sample_count = config
            .max_frames()
            .checked_mul(2)
            .ok_or(ApplicationError::ObservationOverflow)?;
        let mut output = vec![0.0; sample_count];
        renderer.render(&mut output);
        app_loop.advance_structural()?;
        let raw_parameters = *app_loop.current_parameters();
        let patch_audio = renderer.active_patch_audio();

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
        let before_parameters = *app_loop.current_parameters();

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
        let after_parameters = *app_loop.current_parameters();
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
            measure_boundary_recovery(&mut app_loop);

        app_loop.push_recovery_command(AudioCommand::all_notes_off())?;
        let audio_is_degenerate = degenerate == Some(DegenerateMode::Audio);
        let mut peak = 0.0_f32;

        for _ in 0..SMOKE_TICK_COUNT {
            automatic.tick(SMOKE_TICK_DURATION, &mut app_loop)?;
            renderer.render(&mut output);
            app_loop.advance_structural()?;
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

        let observation = SmokeObservation {
            active_graph_revision: renderer.active_revision().value(),
            audio_changed,
            automatic_midi,
            alternating_capabilities: capability_composition.alternating_capabilities,
            braids_patches: capability_composition.braids_patches,
            boundary_noop_nonfatal,
            callback_allocations: 0,
            callback_destructions: 0,
            channel_separators,
            distinct_patch_channels,
            distinct_patch_stems,
            edited_patch_audio_changed,
            edited_patch_id: target_id.value(),
            engine_consumed_value,
            event_commands_delivered,
            parsed_soundfont_banks,
            prepared_instruments,
            one_value_changed,
            parameter_published,
            patch_rows,
            peak,
            per_patch_audio_isolated,
            post_boundary_edit_accepted,
            presets_match: true,
            round_robin_channels,
            soundfont_patches: capability_composition.soundfont_patches,
            state_roundtrip,
            text_matches_state,
            unedited_patch_audio_unchanged,
        };
        drop(renderer);
        app_loop.shutdown_engine_selection_on_control()?;
        Ok(observation)
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
                patch.instrument_config().clone(),
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
            .with_envelope(*patch.envelope())
            .with_post_effects(patch.post_effects().to_vec())
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
    let audio_evidence = report.audio_evidence();
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
    .map(|report| report.with_audio_evidence(audio_evidence))
}

fn sound_all_patches<Control>(
    parameters: &ParameterSnapshot,
    app_loop: &mut AppLoop<Control>,
) -> Result<(), ApplicationError>
where
    Control: ControlAudioBoundary,
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

fn measure_boundary_recovery<Control>(app_loop: &mut AppLoop<Control>) -> (bool, bool, u64)
where
    Control: ControlAudioBoundary,
{
    let mut boundary_noop_nonfatal = false;
    for _ in 0..16 {
        let before_text = app_loop.current_text();
        let before_parameters = *app_loop.current_parameters();
        match app_loop.dispatch(AppEvent::Adjust(Direction::Up)) {
            Ok(result) => {
                if result.boundary_full().is_some() {
                    break;
                }
            }
            Err(EventRejection::ParameterAtBoundary) => {
                boundary_noop_nonfatal = app_loop.current_text() == before_text
                    && *app_loop.current_parameters() == before_parameters;
                break;
            }
            Err(_) => break,
        }
    }

    let before_post_text = app_loop.current_text();
    let before_post_parameters = *app_loop.current_parameters();
    let post_boundary_edit_accepted = match app_loop.dispatch(AppEvent::Adjust(Direction::Down)) {
        Ok(result) => {
            let after_post_parameters = *app_loop.current_parameters();
            result.boundary_full().is_none()
                && result.accepted().generation()
                    == before_post_parameters.generation().saturating_add(1)
                && after_post_parameters.generation() == result.accepted().generation()
                && after_post_parameters != before_post_parameters
                && app_loop.current_text() != before_post_text
        }
        Err(_) => false,
    };
    let baseline_generation = app_loop.current_parameters().generation();

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
    device_status: AudioDeviceStatusReader,
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
            return false;
        }
        if let Some(failure) = runtime.device_status.take_on_control() {
            runtime.record_error(ApplicationError::AudioDeviceRuntime(failure));
            return false;
        }

        let ControlRuntime {
            automatic,
            app_loop,
            device_status: _,
            error,
        } = &mut *runtime;
        if let Err(failure) = app_loop.advance_structural() {
            *error = Some(failure.into());
            return false;
        }
        if let Err(failure) = automatic.tick(elapsed, app_loop) {
            *error = Some(failure.into());
        }
        error.is_none()
    })
}

struct LiveControlRuntime<Source, Boundary, Observation, OnCheckpoint, OnComplete>
where
    Source: MidiEventSource,
    Boundary: ControlAudioBoundary,
    Observation: ControlAudioObservation,
    OnCheckpoint: FnMut(&LiveCheckpoint),
    OnComplete: FnOnce(&LiveDemoReport),
{
    runner: LiveDemoRunner<Source, Observation>,
    app_loop: AppLoop<Boundary>,
    device_status: AudioDeviceStatusReader,
    on_checkpoint: OnCheckpoint,
    on_complete: Option<OnComplete>,
    completion_emitted: bool,
    error: Option<ApplicationError>,
}

type SharedLiveRuntime<Source, Boundary, Observation, OnCheckpoint, OnComplete> =
    Rc<RefCell<LiveControlRuntime<Source, Boundary, Observation, OnCheckpoint, OnComplete>>>;

fn live_input_sink() -> AppInputCallback {
    Box::new(|_event| {})
}

fn live_projection_callback<Source, Boundary, Observation, OnCheckpoint, OnComplete>(
    runtime: SharedLiveRuntime<Source, Boundary, Observation, OnCheckpoint, OnComplete>,
) -> ProjectionCallback
where
    Source: MidiEventSource + 'static,
    Boundary: ControlAudioBoundary + 'static,
    Observation: ControlAudioObservation + 'static,
    OnCheckpoint: FnMut(&LiveCheckpoint) + 'static,
    OnComplete: FnOnce(&LiveDemoReport) + 'static,
{
    Box::new(move || runtime.borrow().app_loop.current_text())
}

fn live_tick_callback<Source, Boundary, Observation, OnCheckpoint, OnComplete>(
    runtime: SharedLiveRuntime<Source, Boundary, Observation, OnCheckpoint, OnComplete>,
) -> TickCallback
where
    Source: MidiEventSource + 'static,
    Boundary: ControlAudioBoundary + 'static,
    Observation: ControlAudioObservation + 'static,
    OnCheckpoint: FnMut(&LiveCheckpoint) + 'static,
    OnComplete: FnOnce(&LiveDemoReport) + 'static,
{
    Box::new(move |elapsed| {
        let mut runtime = runtime.borrow_mut();
        if runtime.error.is_some() || runtime.completion_emitted {
            return false;
        }
        if let Some(failure) = runtime.device_status.take_on_control() {
            runtime.error = Some(ApplicationError::AudioDeviceRuntime(failure));
            return false;
        }

        let LiveControlRuntime {
            runner,
            app_loop,
            device_status: _,
            on_checkpoint,
            on_complete,
            completion_emitted,
            error,
        } = &mut *runtime;
        if let Err(failure) = app_loop.advance_structural() {
            *error = Some(failure.into());
            return false;
        }
        match runner.advance(elapsed, app_loop) {
            Ok(Some(checkpoint)) => on_checkpoint(&checkpoint),
            Ok(None) => {}
            Err(failure) => {
                *error = Some(failure.into());
                return false;
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
                return false;
            }
        }
        error.is_none()
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
    use super::{ApplicationConfig, ApplicationError, DegenerateMode, StandaloneApplication};
    use crate::adapter::atomic_audio_observation::AtomicAudioObservation;
    use crate::adapter::braids_capability::{BraidsCapability, BRAIDS_CAPABILITY_ID};
    use crate::adapter::lock_free_structural_graph_boundary::LockFreeStructuralGraphBoundary;
    use crate::control::app_event::{AppEvent, Direction};
    use crate::control::event_record::EventSource;
    use crate::control::text_projection::TextProjection;
    use crate::control::PatchControlId;
    use crate::kernel::midi_channel::MidiChannel;
    use crate::kernel::midi_message::{MidiMessage, MidiMessageKind};
    use crate::kernel::patch_id::PatchId;
    use crate::real_time::audio_boundary::{
        AudioBoundary, AudioThreadBoundary, BoundaryFull, ControlAudioBoundary,
    };
    use crate::real_time::audio_command::AudioCommand;
    use crate::real_time::parameter_snapshot::ParameterSnapshot;
    use crate::real_time::{GraphHandoffStatus, GraphRevision};
    use crate::shell::app_window::{
        AppInputCallback, AppWindow, ProjectionCallback, TickCallback, WindowError,
    };
    use crate::shell::audio_output::{
        AudioDeviceConfig, AudioDeviceStatusCallback, AudioOutput, AudioOutputError,
        AudioRenderCallback, AudioSampleFormat, AudioStream, NegotiatedAudioOutput,
    };
    use crate::synth::sound_font_instrument::SoundFontInstrument;
    use crate::synth::{
        CapabilityId, InstrumentCapabilityProvider, InstrumentPreparationError, InstrumentPreparer,
        Patch, PreparedInstrument, PreparedInstrumentError, VoiceEnvelope,
    };
    use crate::testing::instrument_part::InstrumentPart;
    use crate::testing::midi_event_source::{
        FixedEventBatch, MidiEventSource, MidiSourceError, TargetedMidiEvent,
    };
    use crate::testing::{LiveDemoError, LIVE_DEMO_NO_PROGRESS_TIMEOUT, LIVE_DEMO_TOTAL_TIMEOUT};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    struct Bus {
        commands: VecDeque<AudioCommand>,
        parameters: ParameterSnapshot,
        parameter_publications: usize,
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
            let mut bus = self.bus.lock().unwrap();
            bus.parameters = parameters;
            bus.parameter_publications += 1;
        }
    }

    impl AudioThreadBoundary for TestAudio {
        fn pop_command(&mut self) -> Option<AudioCommand> {
            self.bus.lock().unwrap().commands.pop_front()
        }

        fn read_latest_parameters(&mut self) -> ParameterSnapshot {
            self.bus.lock().unwrap().parameters
        }
    }

    #[derive(Default)]
    struct EngineState {
        preparer_instances: usize,
        prepared: usize,
        dispatched: usize,
    }

    struct TestPreparer {
        state: Arc<Mutex<EngineState>>,
        capability_id: CapabilityId,
    }

    impl InstrumentPreparer for TestPreparer {
        fn capability_id(&self) -> &CapabilityId {
            &self.capability_id
        }

        fn prepared_shared_asset_count(&self) -> usize {
            usize::from(
                self.capability_id.as_str()
                    == crate::adapter::hidef_soundfont_capability::HIDEF_CAPABILITY_ID,
            )
        }

        fn prepare(
            &self,
            patch: &Patch,
            _sample_rate: f32,
            _max_frames: usize,
        ) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError> {
            self.state.lock().unwrap().prepared += 1;
            Ok(Box::new(TestInstrument {
                patch_id: patch.id(),
                state: Arc::clone(&self.state),
                sounding: false,
            }))
        }
    }

    struct TestInstrument {
        patch_id: PatchId,
        state: Arc<Mutex<EngineState>>,
        sounding: bool,
    }

    impl PreparedInstrument for TestInstrument {
        fn patch_id(&self) -> PatchId {
            self.patch_id
        }

        fn dispatch(
            &mut self,
            _message: MidiMessage,
            _parameters: &crate::real_time::RtPatchParameters,
        ) -> Result<(), PreparedInstrumentError> {
            self.state.lock().unwrap().dispatched += 1;
            self.sounding = true;
            Ok(())
        }

        fn render(
            &mut self,
            output: &mut [f32],
            _frame_count: usize,
            _parameters: &crate::real_time::RtPatchParameters,
        ) {
            if !self.sounding {
                return;
            }
            let index = self.patch_id.value().saturating_sub(1) as usize;
            let amplitude = 0.15 + index as f32 * 0.11;
            for frame in output.chunks_exact_mut(2) {
                frame[0] = amplitude;
                frame[1] = amplitude * (1.03 + index as f32 * 0.07);
            }
        }

        fn all_notes_off(&mut self) {
            self.sounding = false;
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
            assert!(on_tick(Duration::from_millis(20)));
            Ok(())
        }
    }

    struct TestOutput;

    impl AudioOutput for TestOutput {
        type Negotiated = Self;

        fn negotiate(self) -> Result<Self::Negotiated, AudioOutputError> {
            Ok(self)
        }
    }

    impl NegotiatedAudioOutput for TestOutput {
        fn config(&self) -> AudioDeviceConfig {
            AudioDeviceConfig::new(48_000.0, 2, AudioSampleFormat::F32, 256).unwrap()
        }

        fn start(
            self,
            mut render: AudioRenderCallback,
            _on_runtime_error: AudioDeviceStatusCallback,
        ) -> Result<AudioStream, AudioOutputError> {
            let mut buffer = [0.0; 512];
            render(&mut buffer);
            Ok(AudioStream::new(()))
        }
    }

    type SharedRender = Arc<Mutex<Option<AudioRenderCallback>>>;

    struct LiveTestOutput {
        render: SharedRender,
    }

    impl AudioOutput for LiveTestOutput {
        type Negotiated = Self;

        fn negotiate(self) -> Result<Self::Negotiated, AudioOutputError> {
            Ok(self)
        }
    }

    impl NegotiatedAudioOutput for LiveTestOutput {
        fn config(&self) -> AudioDeviceConfig {
            AudioDeviceConfig::new(48_000.0, 2, AudioSampleFormat::F32, 256).unwrap()
        }

        fn start(
            self,
            render: AudioRenderCallback,
            _on_runtime_error: AudioDeviceStatusCallback,
        ) -> Result<AudioStream, AudioOutputError> {
            *self.render.lock().unwrap() = Some(render);
            Ok(AudioStream::new(()))
        }
    }

    #[derive(Clone, Default)]
    struct StalledLiveWitness {
        close_requests: Arc<Mutex<usize>>,
        reports: Arc<Mutex<usize>>,
        stream_released: Arc<Mutex<bool>>,
    }

    struct StalledStreamHandle {
        released: Arc<Mutex<bool>>,
    }

    impl Drop for StalledStreamHandle {
        fn drop(&mut self) {
            *self.released.lock().unwrap() = true;
        }
    }

    struct StalledLiveOutput {
        released: Arc<Mutex<bool>>,
    }

    impl AudioOutput for StalledLiveOutput {
        type Negotiated = Self;

        fn negotiate(self) -> Result<Self::Negotiated, AudioOutputError> {
            Ok(self)
        }
    }

    impl NegotiatedAudioOutput for StalledLiveOutput {
        fn config(&self) -> AudioDeviceConfig {
            AudioDeviceConfig::new(48_000.0, 2, AudioSampleFormat::F32, 16).unwrap()
        }

        fn start(
            self,
            _render: AudioRenderCallback,
            _on_runtime_error: AudioDeviceStatusCallback,
        ) -> Result<AudioStream, AudioOutputError> {
            Ok(AudioStream::new(StalledStreamHandle {
                released: self.released,
            }))
        }
    }

    struct StalledLiveWindow {
        close_requests: Arc<Mutex<usize>>,
        tick_duration: Duration,
    }

    impl AppWindow for StalledLiveWindow {
        fn run(
            &self,
            _on_input: AppInputCallback,
            _projection: ProjectionCallback,
            mut on_tick: TickCallback,
        ) -> Result<(), WindowError> {
            for _ in 0..64 {
                if !on_tick(self.tick_duration) {
                    *self.close_requests.lock().unwrap() += 1;
                    return Ok(());
                }
            }
            Err(WindowError::new(
                "stalled live window did not receive a bounded close request",
            ))
        }
    }

    #[derive(Clone, Default)]
    struct LiveWindowWitness {
        reports: Arc<Mutex<Vec<crate::testing::LiveDemoReport>>>,
        final_projection: Arc<Mutex<Option<TextProjection>>>,
        input_injected: Arc<Mutex<bool>>,
        close_requests: Arc<Mutex<usize>>,
        post_completion_ticks: Arc<Mutex<usize>>,
    }

    struct LiveTestWindow {
        render: SharedRender,
        bus: Arc<Mutex<Bus>>,
        witness: LiveWindowWitness,
    }

    impl AppWindow for LiveTestWindow {
        fn run(
            &self,
            mut on_input: AppInputCallback,
            projection: ProjectionCallback,
            mut on_tick: TickCallback,
        ) -> Result<(), WindowError> {
            let mut last_rendered_projection = projection();
            let mut input_injected = false;
            for _ in 0..800 {
                let reports_before = self.witness.reports.lock().unwrap().len();
                let keep_open = on_tick(Duration::from_millis(100));
                let reports_after = self.witness.reports.lock().unwrap().len();
                if !keep_open {
                    if reports_after > 0 {
                        assert_eq!(reports_before, 0);
                        assert_eq!(reports_after, 1);
                        *self.witness.close_requests.lock().unwrap() += 1;
                        *self.witness.final_projection.lock().unwrap() =
                            Some(last_rendered_projection);
                    }
                    return Ok(());
                }

                if reports_after > 0 {
                    *self.witness.post_completion_ticks.lock().unwrap() += 1;
                }

                if !input_injected {
                    // The first tick has dispatched the first autonomous scene
                    // event. Inject a mapped semantic adjustment before audio
                    // renders that pending generation, matching the production
                    // race that previously replaced the awaited snapshot.
                    let projection_before = projection();
                    let (parameters_before, publications_before) = {
                        let bus = self.bus.lock().unwrap();
                        (bus.parameters, bus.parameter_publications)
                    };
                    on_input(AppEvent::Adjust(Direction::Down));
                    let (parameters_after, publications_after) = {
                        let bus = self.bus.lock().unwrap();
                        (bus.parameters, bus.parameter_publications)
                    };
                    assert_eq!(
                        parameters_after.generation(),
                        parameters_before.generation()
                    );
                    assert_eq!(parameters_after, parameters_before);
                    assert_eq!(publications_after, publications_before);
                    assert_eq!(projection(), projection_before);
                    *self.witness.input_injected.lock().unwrap() = true;
                    input_injected = true;
                }

                {
                    let mut render = self.render.lock().unwrap();
                    let render = render
                        .as_mut()
                        .ok_or_else(|| WindowError::new("live audio callback was not opened"))?;
                    let mut buffer = [0.0_f32; 512];
                    render(&mut buffer);
                }
                std::thread::yield_now();
                std::thread::sleep(Duration::from_millis(1));
                last_rendered_projection = projection();
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

    fn preparer_for(state: Arc<Mutex<EngineState>>, capability_id: &str) -> TestPreparer {
        state.lock().unwrap().preparer_instances += 1;
        TestPreparer {
            state,
            capability_id: CapabilityId::new(capability_id).unwrap(),
        }
    }

    fn providers() -> Vec<Box<dyn InstrumentCapabilityProvider>> {
        vec![
            Box::new(
                crate::adapter::production_instruments::production_soundfont_capability().unwrap(),
            ),
            Box::new(BraidsCapability::new().unwrap()),
        ]
    }

    fn preparers(state: Arc<Mutex<EngineState>>) -> Vec<Box<dyn InstrumentPreparer>> {
        vec![
            Box::new(preparer_for(
                Arc::clone(&state),
                "instrument.soundfont.hidef",
            )),
            Box::new(preparer_for(state, BRAIDS_CAPABILITY_ID)),
        ]
    }

    fn structural_boundary() -> LockFreeStructuralGraphBoundary {
        LockFreeStructuralGraphBoundary::new(
            1,
            1,
            GraphHandoffStatus::with_active(GraphRevision::INITIAL),
        )
        .unwrap()
    }

    type TestApplication<Window, Output> = StandaloneApplication<
        TestBoundary,
        LockFreeStructuralGraphBoundary,
        AtomicAudioObservation,
        TestSource,
        Window,
        Output,
    >;

    fn application(
        due: Vec<TargetedMidiEvent>,
        engine_state: Arc<Mutex<EngineState>>,
        projection: Arc<Mutex<Option<TextProjection>>>,
    ) -> TestApplication<TestWindow, TestOutput> {
        StandaloneApplication::new_with_effects(
            TestBoundary {
                bus: Arc::new(Mutex::new(Bus {
                    commands: VecDeque::new(),
                    parameters: parameters(),
                    parameter_publications: 0,
                })),
            },
            providers(),
            preparers(engine_state),
            crate::adapter::production_effects::production_effect_providers().unwrap(),
            crate::adapter::production_effects::production_effect_preparers().unwrap(),
            structural_boundary(),
            AtomicAudioObservation::default(),
            TestSource {
                parts: parts(),
                due,
                started: false,
            },
            TestWindow { projection },
            TestOutput,
            ApplicationConfig::new(
                48_000.0,
                256,
                ApplicationConfig::default().global_parameters(),
            ),
        )
        .unwrap()
    }

    fn live_application(
        due: Vec<TargetedMidiEvent>,
        engine_state: Arc<Mutex<EngineState>>,
        witness: LiveWindowWitness,
    ) -> TestApplication<LiveTestWindow, LiveTestOutput> {
        let render = Arc::new(Mutex::new(None));
        let bus = Arc::new(Mutex::new(Bus {
            commands: VecDeque::new(),
            parameters: parameters(),
            parameter_publications: 0,
        }));
        StandaloneApplication::new_with_effects(
            TestBoundary {
                bus: Arc::clone(&bus),
            },
            providers(),
            preparers(engine_state),
            crate::adapter::production_effects::production_effect_providers().unwrap(),
            crate::adapter::production_effects::production_effect_preparers().unwrap(),
            structural_boundary(),
            AtomicAudioObservation::default(),
            TestSource {
                parts: parts(),
                due,
                started: false,
            },
            LiveTestWindow {
                render: Arc::clone(&render),
                bus,
                witness,
            },
            LiveTestOutput { render },
            ApplicationConfig::new(
                48_000.0,
                256,
                ApplicationConfig::default().global_parameters(),
            ),
        )
        .unwrap()
    }

    fn early_close_application() -> TestApplication<EarlyCloseWindow, TestOutput> {
        StandaloneApplication::new_with_effects(
            TestBoundary {
                bus: Arc::new(Mutex::new(Bus {
                    commands: VecDeque::new(),
                    parameters: parameters(),
                    parameter_publications: 0,
                })),
            },
            providers(),
            preparers(Arc::new(Mutex::new(EngineState::default()))),
            crate::adapter::production_effects::production_effect_providers().unwrap(),
            crate::adapter::production_effects::production_effect_preparers().unwrap(),
            structural_boundary(),
            AtomicAudioObservation::default(),
            TestSource {
                parts: parts(),
                due: vec![TargetedMidiEvent::new(0, message())],
                started: false,
            },
            EarlyCloseWindow,
            TestOutput,
            ApplicationConfig::new(
                48_000.0,
                256,
                ApplicationConfig::default().global_parameters(),
            ),
        )
        .unwrap()
    }

    fn stalled_live_application(
        witness: StalledLiveWitness,
        tick_duration: Duration,
    ) -> TestApplication<StalledLiveWindow, StalledLiveOutput> {
        StandaloneApplication::new_with_effects(
            TestBoundary {
                bus: Arc::new(Mutex::new(Bus {
                    commands: VecDeque::new(),
                    parameters: parameters(),
                    parameter_publications: 0,
                })),
            },
            providers(),
            preparers(Arc::new(Mutex::new(EngineState::default()))),
            crate::adapter::production_effects::production_effect_providers().unwrap(),
            crate::adapter::production_effects::production_effect_preparers().unwrap(),
            structural_boundary(),
            AtomicAudioObservation::default(),
            TestSource {
                parts: parts(),
                due: vec![TargetedMidiEvent::new(0, message())],
                started: false,
            },
            StalledLiveWindow {
                close_requests: Arc::clone(&witness.close_requests),
                tick_duration,
            },
            StalledLiveOutput {
                released: Arc::clone(&witness.stream_released),
            },
            ApplicationConfig::new(
                48_000.0,
                256,
                ApplicationConfig::default().global_parameters(),
            ),
        )
        .unwrap()
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
        assert_eq!(engine_state.preparer_instances, 2);
        assert_eq!(engine_state.prepared, 2);
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

        assert!(
            report.is_complete(),
            "coverage={:?}; audio={:?}; eventMissing={:?}; eventUnexpected={:?}",
            report.coverage(),
            report.audio_evidence(),
            report.event_log().coverage().missing(),
            report.event_log().coverage().unexpected(),
        );
        assert_eq!(report.coverage().missing_count(), 0);
        assert_eq!(report.event_log().dropped_records(), 0);
        assert!(report.event_log().records().len() > 1);
        assert_eq!(report.final_state_tree().patch_count(), 2);
        assert!(report.checkpoints().len() > 10);
        assert!(engine_state.lock().unwrap().dispatched > 0);
    }

    #[test]
    fn standalone_live_demo_composition_isolates_input_and_closes_on_completion() {
        let engine_state = Arc::new(Mutex::new(EngineState::default()));
        let witness = LiveWindowWitness::default();
        let checkpoints = Arc::new(Mutex::new(Vec::new()));
        let checkpoints_for_callback = Arc::clone(&checkpoints);
        let reports_for_callback = Arc::clone(&witness.reports);

        live_application(
            vec![TargetedMidiEvent::new(0, message())],
            Arc::clone(&engine_state),
            witness.clone(),
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

        let reports = witness.reports.lock().unwrap();
        assert_eq!(reports.len(), 1);
        let report = &reports[0];
        assert!(report.complete(), "{}", report.summary());
        assert_eq!(checkpoints.lock().unwrap().as_slice(), report.checkpoints());
        let patch_adsr = report
            .checkpoints()
            .iter()
            .filter_map(|checkpoint| checkpoint.as_parameter())
            .filter(|checkpoint| {
                matches!(
                    checkpoint.expected_transition().patch_control_id(),
                    Some(PatchControlId::Envelope(_))
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(patch_adsr.len(), VoiceEnvelope::surface_descriptor().len());
        assert!(patch_adsr.iter().all(|checkpoint| {
            checkpoint
                .expected_transition()
                .patch_control_id()
                .is_some_and(|control| control != PatchControlId::Engine)
                && checkpoint.projected_value().patch_control_id()
                    == checkpoint.expected_transition().patch_control_id()
        }));
        let patch_effects = report
            .checkpoints()
            .iter()
            .filter_map(|checkpoint| checkpoint.as_parameter())
            .filter(|checkpoint| {
                matches!(
                    checkpoint.expected_transition().patch_control_id(),
                    Some(PatchControlId::Effect(_, _))
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(patch_effects.len(), 2);
        assert!(patch_effects.iter().all(|checkpoint| {
            let effect = checkpoint.audio_observation().patch_effect();
            effect.patch_id() == Some(PatchId::new(1).unwrap())
                && effect.difference_rms() > 0.0
                && effect.side_rms() > 0.0
        }));
        let structural = report
            .checkpoints()
            .iter()
            .filter_map(|checkpoint| checkpoint.as_engine())
            .collect::<Vec<_>>();
        assert_eq!(structural.len(), 9);
        assert!(structural[..3]
            .iter()
            .all(|checkpoint| checkpoint.preset().is_some()));
        assert!(structural[3..]
            .iter()
            .all(|checkpoint| checkpoint.focused_control_id() == PatchControlId::Engine));
        assert_eq!(report.runtime_audio().callback_allocations(), 0);
        assert_eq!(report.runtime_audio().callback_destructions(), 0);
        assert!(*witness.input_injected.lock().unwrap());
        assert_eq!(*witness.close_requests.lock().unwrap(), 1);
        assert_eq!(*witness.post_completion_ticks.lock().unwrap(), 0);
        assert!(report
            .event_log()
            .records()
            .iter()
            .all(|record| record.source() != EventSource::Keyboard));
        let projection = witness.final_projection.lock().unwrap();
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
    fn standalone_live_demo_stall_times_out_closes_and_releases_stream() {
        let witness = StalledLiveWitness::default();
        let reports_for_callback = Arc::clone(&witness.reports);

        let error = stalled_live_application(witness.clone(), Duration::from_secs(1))
            .run_live_demo(|_| {}, move |_| *reports_for_callback.lock().unwrap() += 1)
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::LiveDemo(LiveDemoError::ProgressTimedOut {
                stage: "parameter audio observation",
                stalled_for,
            }) if stalled_for >= LIVE_DEMO_NO_PROGRESS_TIMEOUT
        ));
        assert_eq!(*witness.reports.lock().unwrap(), 0);
        assert_eq!(*witness.close_requests.lock().unwrap(), 1);
        assert!(*witness.stream_released.lock().unwrap());
    }

    #[test]
    fn standalone_live_demo_total_bound_closes_and_releases_stream() {
        let witness = StalledLiveWitness::default();
        let reports_for_callback = Arc::clone(&witness.reports);

        let error = stalled_live_application(witness.clone(), LIVE_DEMO_TOTAL_TIMEOUT)
            .run_live_demo(|_| {}, move |_| *reports_for_callback.lock().unwrap() += 1)
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::LiveDemo(LiveDemoError::TotalTimeout { elapsed })
                if elapsed >= LIVE_DEMO_TOTAL_TIMEOUT
        ));
        assert_eq!(*witness.reports.lock().unwrap(), 0);
        assert_eq!(*witness.close_requests.lock().unwrap(), 1);
        assert!(*witness.stream_released.lock().unwrap());
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

        assert_eq!(observation.parsed_soundfont_banks, 1);
        assert_eq!(observation.prepared_instruments, 2);
        assert_eq!(observation.active_graph_revision, 1);
        assert_eq!(observation.callback_allocations, 0);
        assert_eq!(observation.callback_destructions, 0);
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
