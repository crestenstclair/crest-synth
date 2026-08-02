pub mod instrument_part;
pub use instrument_part::InstrumentPart;

pub mod automatic_midi_test;
pub mod midi_event_source;
pub use automatic_midi_test::{AutomaticMidiTest, TestInputError};

pub mod demo_scene_report;
pub use demo_scene_report::{
    DemoAudioEvidence, DemoCoverageGroup, DemoCoverageSet, DemoEngineCheckpoint,
    DemoPresetCheckpoint, DemoSceneCheckpoint, DemoSceneCheckpointError, DemoSceneCoverage,
    DemoSceneReport, DemoSceneReportError,
};

pub mod demo_scene;
pub use demo_scene::{
    DemoCheckpoint, DemoEngineExpectation, DemoEngineProbe, DemoPresetExpectation, DemoScene,
    DemoSceneError, DemoSceneStep, DemoWorkerAdvance, MidiProbe,
};

pub mod deterministic_graph_preparation_worker;
pub use deterministic_graph_preparation_worker::{
    DeterministicGraphPreparationHandle, DeterministicGraphPreparationWorker,
};

pub mod exhaustive_gui_demo;
pub use exhaustive_gui_demo::{ExhaustiveGuiDemo, ExhaustiveGuiDemoError};

pub mod behavioral_mutation_harness;
pub use behavioral_mutation_harness::{
    BehavioralMutationCase, BehavioralMutationHarness, BehavioralMutationObservation,
    BehavioralMutationRun,
};

pub mod live_demo_scene;
pub use live_demo_scene::{
    LiveAudioPredicate, LiveDemoScene, LiveDemoSceneError, LiveDemoStep, LiveEditableParameter,
    LiveEngineTransition, LiveExpectedTransition,
};

pub mod live_effects_and_buses_scene;
pub use live_effects_and_buses_scene::{
    LiveTopologyAudibleWitness, LiveTopologySupport, LiveTopologyTransition,
    EFFECTS_AND_BUSES_SCENE_NAME,
};

pub mod live_demo_checkpoint;
pub use live_demo_checkpoint::{
    LiveCheckpoint, LiveDemoCheckpoint, LiveDemoCheckpointError, LiveEngineCheckpoint,
    LivePresetProjection, LiveProjectedValue, LiveTopologyCheckpoint,
};

pub mod live_demo_report;
pub use live_demo_report::{
    LiveDemoCoverage, LiveDemoReport, LiveDemoReportError, LiveEffectsAndBusesEvidence,
    LiveEventLogSummary, LiveMixerRoutingEvidence, LiveShellCoverage, RuntimeAudioWitness,
    SixteenTrackMixerRoutingObservation,
};

mod live_mixer_routing_measurement;

pub mod live_demo_runner;
pub use live_demo_runner::{
    LiveDemoError, LiveDemoRunner, LIVE_DEMO_NO_PROGRESS_TIMEOUT, LIVE_DEMO_TOTAL_TIMEOUT,
};

// Browsable rather than autonomous: it accepts input, makes no exact-generation
// claim, and is not a `demo-live` alias. See the module documentation.
pub mod component_gallery_scene;
pub use component_gallery_scene::{
    ComponentGalleryError, ComponentGalleryObservation, ComponentGalleryPage,
    ComponentGalleryScene, GalleryPageSelection, PageSelection, PaintedStateRecord,
    ALL_GALLERY_PAGES, COMPONENT_GALLERY_OBSERVATION_MARKER, COMPONENT_GALLERY_WINDOW_TITLE,
    GALLERY_PAGE_COUNT,
};
