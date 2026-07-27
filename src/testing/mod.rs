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

pub mod live_demo_checkpoint;
pub use live_demo_checkpoint::{
    LiveCheckpoint, LiveDemoCheckpoint, LiveDemoCheckpointError, LiveEngineCheckpoint,
    LivePresetProjection, LiveProjectedValue,
};

pub mod live_demo_report;
pub use live_demo_report::{
    LiveDemoCoverage, LiveDemoReport, LiveDemoReportError, LiveEventLogSummary, RuntimeAudioWitness,
};

pub mod live_demo_runner;
pub use live_demo_runner::{
    LiveDemoError, LiveDemoRunner, LIVE_DEMO_NO_PROGRESS_TIMEOUT, LIVE_DEMO_TOTAL_TIMEOUT,
};
