package crestsynth

// The goals file owns product intent. This file owns the architectural rules
// and whole-project gates that keep independently generated resources coherent.
project: name: "crest-synth"

project: layers: ["domain", "application", "infrastructure"]
project: layerRules: {
	application: {dependsOn: ["domain"]}
	infrastructure: {dependsOn: ["domain", "application"]}
}

project: meta: {
	language: "rust"
	style: "idiomatic Rust; explicit domain newtypes; small focused modules; keyboard/gamepad-first mixer UI; deterministic headless proofs"
	rules: [
		"one spec resource owns one canonical public Rust type in its module; every consumer imports it instead of declaring a local lookalike",
		"an asset is a composition root or proof harness, not a second implementation of the resources it targets",
		"the standalone binary is thin: application orchestration belongs to StandaloneApplication and all state changes go through AppState.apply",
		"live input, smoke runs, autopilot, and scenes call the same reducer and audio-render functions",
		"tests and demos exercise production resource types; they do not replace missing behavior with local substitutes",
		"proof output is calculated from state, routing, or rendered samples and must fail for an explicit no-op or lossy implementation",
	]
	avoid: [
		"heap allocation, locks, blocking I/O, or deallocation on the audio callback",
		"dynamic dispatch in the inner sample loop",
		"parallel AppState, AudioFrame, MIDI, patch, session, or sample model types",
		"view-owned mutable state or direct mutation from input adapters",
		"mouse, touch, on-screen-note input, or non-mixer screens in the current UI",
		"unconditional success tokens presented as behavioral evidence",
	]
}

// Stable IDs let goals and evidence refer to executable checks directly.
project: validations: {
	format: {scope: "project", kind: "custom", command: ["cargo", "fmt"], description: "normalize Rust formatting"}
	clippy: {scope: "project", kind: "compiles", command: ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"], description: "all targets are warning-free"}
	build: {scope: "project", kind: "compiles", command: ["cargo", "build", "--all-targets"], description: "the complete standalone crate and proof binaries build"}
	test: {scope: "project", kind: "test", command: ["cargo", "test", "--all-targets"], description: "all deterministic unit and integration tests pass"}
	midi_routing_contract: {
		scope: "dependency_contract", kind: "test", command: ["cargo", "test", "midi_dispatcher"]
		resources: ["domainService.Patch.MidiDispatcher", "aggregate.Patch.Patch", "domainService.Shell.MidiNormalizer"]
		capabilities: ["capability.external_midi_performance"]
		goals: ["goal.perform_through_standalone"]
	}
	realtime_contract: {
		scope: "integration_wave", kind: "integration", command: ["make", "check-live"]
		resources: ["adapter.RtrbEventRing", "adapter.TripleBufferParameterBridge", "adapter.BasedropDeferredDeallocator", "asset.MidiPlayLiveMain"]
		capabilities: ["capability.realtime_safe_execution"]
		goals: ["goal.perform_through_standalone"]
	}
	mixer_integration: {
		scope: "integration_wave", kind: "integration", command: ["make", "demo-mixer"]
		resources: ["aggregate.Mixer.MixerView", "aggregate.Mixer.ChannelStrip", "domainService.Mixer.MixEngine", "asset.MixerDemoMain"]
		capabilities: ["capability.pointer_free_mixer_control", "capability.stereo_mix_pipeline"]
		goals: ["goal.operate_live_mixer"]
	}
	ui_smoke: {
		scope: "goal", kind: "integration", command: ["make", "ui-smoke"]
		resources: ["applicationService.Loop.StandaloneApplication", "asset.SynthUiMain", "aggregate.Loop.AppState", "aggregate.Mixer.MixerView"]
		capabilities: ["capability.external_midi_performance", "capability.pointer_free_mixer_control", "capability.shared_control_reducer"]
		goals: ["goal.perform_through_standalone", "goal.operate_live_mixer"]
	}
	autopilot: {
		scope: "goal", kind: "integration", command: ["make", "autopilot"], timeout: "30s"
		resources: ["applicationService.Loop.StandaloneApplication", "asset.SynthUiMain"]
		capabilities: ["capability.pointer_free_mixer_control", "capability.stereo_mix_pipeline"]
		goals: ["goal.operate_live_mixer"]
	}
	midi_multitrack_regression: {
		scope: "regression", kind: "integration", command: ["cargo", "run", "--bin", "synth_ui", "--", "--smoke", "--play", "midi/Corridors of Time - Chrono Trigger.mid"]
		resources: ["adapter.MidlyMidiFileReader", "applicationService.MidiFile.TestPlaybackAssembler", "domainService.MidiFile.Sequencer", "applicationService.Loop.StandaloneApplication", "asset.SynthUiMain"]
		capabilities: ["capability.instrument_partitioned_test_playback", "capability.configurable_instrument_graph", "capability.stereo_mix_pipeline"]
		goals: ["goal.exercise_supported_sound_architecture"]
	}
	preset_roundtrip: {
		scope: "goal", kind: "integration", command: ["make", "demo-presets"]
		resources: ["valueObject.Preset.Preset", "aggregate.Preset.Session", "adapter.SerdePresetCodec", "asset.PresetRoundtripDemoMain"]
		capabilities: ["capability.versioned_sound_state"]
		goals: ["goal.preserve_reproducible_sound_state"]
	}
	scene_suite: {
		scope: "goal", kind: "integration", command: ["make", "demo-scenes"]
		resources: ["applicationService.Loop.SceneRunner", "asset.SceneRunMain", "asset.SceneLibrary"]
		capabilities: ["capability.shared_control_reducer", "capability.deterministic_scene_replay"]
		goals: ["goal.inspect_and_replay_behavior"]
	}
	proof_suite: {
		scope: "project", kind: "integration", command: ["make", "proofs"]
		capabilities: ["capability.behavioral_proof_harness", "capability.configurable_instrument_graph"]
		goals: ["goal.exercise_supported_sound_architecture", "goal.inspect_and_replay_behavior"]
		description: "every supported subsystem and vertical slice produces measured evidence"
	}
}

project: invariants: core: [
	{text: "the audio callback never allocates, locks, blocks, performs I/O, or destroys retired owned state", meta: rationale: "all five operations have unbounded latency"},
	{text: "all control changes cross the real-time boundary through EventRing or ParameterBridge and retired memory returns through DeferredDeallocator", meta: rationale: "one auditable thread seam"},
	{text: "signal flows source -> strip inserts -> volume/pan -> send taps -> aux returns -> master inserts -> limiter -> output", meta: rationale: "one canonical stereo path"},
	{text: "MIDI dispatch reaches every intentionally matching patch exactly once and MPE zones do not overlap across the active patch collection", meta: rationale: "layering remains intentional and expression unambiguous"},
	{text: "preset and session payloads are explicitly versioned and replace active state only after complete decode, migration, and validation", meta: rationale: "failed restore cannot corrupt live state"},
	{text: "AppState.apply is the only control mutation path; views, adapters, demos, and scenes emit AppEvents", meta: rationale: "live and replay behavior must be comparable"},
	{text: "a canonical resource type is declared once and imported everywhere else", meta: rationale: "duplicate structural types made the generated system impossible to compose"},
]

project: contextMap: [
	{from: "Kernel", to: "Engine", kind: "shared-kernel"},
	{from: "Kernel", to: "Sample", kind: "shared-kernel"},
	{from: "Kernel", to: "Effects", kind: "shared-kernel"},
	{from: "Kernel", to: "Mixer", kind: "shared-kernel"},
	{from: "Kernel", to: "Patch", kind: "shared-kernel"},
	{from: "Kernel", to: "Preset", kind: "shared-kernel"},
	{from: "Engine", to: "Patch", kind: "customer-supplier", direction: "upstream"},
	{from: "Sample", to: "Patch", kind: "customer-supplier", direction: "upstream"},
	{from: "Modulation", to: "Patch", kind: "customer-supplier", direction: "upstream"},
	{from: "Effects", to: "Mixer", kind: "customer-supplier", direction: "upstream"},
	{from: "Patch", to: "Preset", kind: "customer-supplier", direction: "upstream"},
	{from: "Mixer", to: "Loop", kind: "customer-supplier", direction: "upstream"},
	{from: "Patch", to: "Loop", kind: "customer-supplier", direction: "upstream"},
	{from: "Preset", to: "Loop", kind: "customer-supplier", direction: "upstream"},
	{from: "Editor", to: "Loop", kind: "customer-supplier", direction: "upstream"},
	{from: "Loop", to: "RealTime", kind: "anti-corruption", direction: "downstream"},
	{from: "Shell", to: "Loop", kind: "anti-corruption", direction: "downstream"},
	{from: "MidiFile", to: "Loop", kind: "anti-corruption", direction: "downstream"},
	{from: "DesignSystem", to: "Shell", kind: "customer-supplier", direction: "upstream"},
]

project: assetKinds: {
	"cargo-manifest": {description: "the Rust workspace/package manifest", filePattern: "Cargo.toml"}
	"rust-bin-target": {description: "a thin executable composition root or behavioral proof", filePattern: "src/bin/*.rs"}
	"makefile": {description: "stable human and automation entry points", filePattern: "Makefile"}
	"scene-library": {description: "versioned deterministic AppEvent scenarios", filePattern: "scenes/*"}
}
