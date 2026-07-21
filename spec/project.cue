package crestsynth

project: {
	name: "crest-synth"
	layers: ["domain", "application", "infrastructure"]
	layerRules: {
		application: {dependsOn: ["domain"]}
		infrastructure: {dependsOn: ["domain", "application"]}
	}

		meta: {
		language: "rust"
		style: "idiomatic Rust; explicit domain types; small modules; ports at external and real-time boundaries"
			rules: [
			"one resource owns each public type and consumers import that type",
			"the standalone binary is a composition root; behavior lives behind domain and application abstractions",
			"test support uses the same ports, reducer, parameter bridge, event ring, engine, mixer, and audio callback as the running application",
			]
		avoid: [
			"alternate running synthesis engines, the Braids C++/FFI wrapper, a prepared multi-engine rack, engine selection, or synthesis fallback in this increment",
			"effects other than the one global reverb and one global delay",
			"sequencer, transport, timeline, pattern, clip, or song-editing domain models",
			"presets, sessions, modulation matrices, per-channel inserts, effect chains, buses, or plugin hosting",
			"panels, dashboards, meters, faders, custom widgets, custom drawing, themes, or multiple screens",
		]
	}

	validations: {
		format: {
			scope: "project"
			kind: "custom"
			command: ["cargo", "fmt", "--all", "--", "--check"]
			description: "Rust formatting is canonical"
		}
		clippy: {
			scope: "project"
			kind: "compiles"
			command: ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]
			description: "all targets compile without warnings"
		}
		test: {
			scope: "project"
			kind: "test"
			command: ["cargo", "test", "--all-targets"]
			description: "domain, reducer, adapter, and integration tests pass"
		}
			smoke: {
			scope: "project"
			kind: "integration"
			command: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke"]
			timeout: "180s"
			resources: [
				"adapter.HiDefSoundFontEngine",
				"adapter.CorridorsMidiEventSource",
				"applicationService.Testing.AutomaticMidiTest",
				"applicationService.Shell.StandaloneApplication",
				"asset.CrestSynthMain",
			]
			capabilities: [
				"capability.soundfont_audio",
				"capability.automatic_test_midi",
				"capability.one_way_parameter_control",
				"capability.realtime_execution",
			]
			goals: ["goal.play_test_song", "goal.control_synth"]
				description: "the fixed MIDI fixture drives the real SoundFont, control, mixer, and audio path"
			}
				capability_schema: {
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "capability_schema", "--", "--nocapture"]
				assertions: [
					{kind: "exit_code", expected: 0},
					{kind: "stdout_contains", pattern: "CREST_ACCEPTANCE capability_schema passed"},
				]
				resources: [
					"valueObject.Synth.CapabilityId",
					"valueObject.Synth.ParameterId",
					"valueObject.Synth.AssetReference",
					"valueObject.Synth.ParameterValue",
					"valueObject.Synth.ParameterAssignment",
					"valueObject.Synth.ParameterSpec",
					"valueObject.Synth.CapabilityDescriptor",
					"valueObject.Synth.InstrumentConfig",
					"valueObject.Synth.CapabilityRegistry",
					"port.Synth.InstrumentCapabilityProvider",
					"adapter.HiDefSoundFontCapability",
					"aggregate.Synth.Patch",
					"aggregate.Control.AppState",
					"domainService.Control.StateProjector",
					"applicationService.Testing.AutomaticMidiTest",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: ["capability.instrument_capability_model", "capability.one_way_parameter_control", "capability.soundfont_audio"]
				goals: ["goal.control_synth"]
					description: "the exact installed SoundFont descriptor and generic fixture Patch configs survive reducer installation, serialization, and projection while malformed and unknown configs fail without fallback"
				}
				control_dispatch_performance: {
					scope: "project"
					kind: "integration"
					command: ["cargo", "test", "--test", "control_dispatch_performance", "--", "--nocapture"]
					assertions: [
						{kind: "exit_code", expected: 0},
						{kind: "stdout_contains", pattern: "CREST_ACCEPTANCE control_dispatch_performance passed"},
					]
					resources: [
						"aggregate.Control.AppState",
						"domainService.Control.StateProjector",
						"applicationService.Control.AppLoop",
						"valueObject.Control.EventLog",
						"valueObject.Control.StateSnapshot",
						"valueObject.Control.StateTree",
						"valueObject.Control.TextProjection",
						"valueObject.RealTime.ParameterSnapshot",
						"asset.BehavioralAcceptanceTests",
					]
					capabilities: ["capability.one_way_parameter_control", "capability.live_observable_demo"]
					goals: ["goal.control_synth", "goal.observe_live_synth"]
					description: "the production fifteen-Patch reducer/projector/journal/publication path dispatches 512 MIDI events within 50 ms while lazy materialization remains exactly equal to eager canonical projections"
				}
				demo_scene: {
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "exhaustive_demo_scene", "--", "--nocapture"]
				assertions: [
					{kind: "exit_code", expected: 0},
					{kind: "stdout_contains", pattern: "CREST_ACCEPTANCE exhaustive_demo_scene passed"},
				]
				timeout: "180s"
				resources: [
					"valueObject.Control.AppEvent",
					"valueObject.Control.EventRecord",
					"valueObject.Control.EventLog",
					"valueObject.Control.StateTree",
					"valueObject.Control.TextProjection",
					"aggregate.Control.AppState",
					"domainService.Control.StateProjector",
					"applicationService.Control.AppLoop",
					"valueObject.Shell.WindowInput",
					"applicationService.Shell.KeyboardInputTranslator",
					"adapter.EframeTextWindow",
					"valueObject.Mixer.ChannelParameters",
					"valueObject.Mixer.GlobalParameters",
					"valueObject.Mixer.MixObservation",
					"port.Mixer.GlobalEffectsProcessor",
					"adapter.GlobalReverbDelay",
					"domainService.Mixer.MixEngine",
					"valueObject.RealTime.ParameterSnapshot",
					"valueObject.RealTime.AudioCommand",
					"valueObject.RealTime.PatchAudioBlock",
					"applicationService.RealTime.AudioRenderer",
					"valueObject.Testing.DemoScene",
					"valueObject.Testing.DemoSceneReport",
					"applicationService.Testing.ExhaustiveGuiDemo",
					"applicationService.Shell.StandaloneApplication",
					"asset.CrestSynthMain",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: [
					"capability.instrument_capability_model",
					"capability.observable_demo_scene",
					"capability.one_way_parameter_control",
					"capability.global_mix",
				]
				goals: ["goal.observe_synth"]
				description: "the exhaustive deterministic GUI scene covers every current event, editable parameter, serialized property, projection, and downstream effect"
			}
			live_demo: {
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "live_demo_scene", "--", "--nocapture"]
				assertions: [
					{kind: "exit_code", expected: 0},
					{kind: "stdout_contains", pattern: "CREST_ACCEPTANCE live_demo_scene passed"},
				]
				timeout: "180s"
				resources: [
					"valueObject.Control.AppEvent",
					"valueObject.Control.EventRecord",
					"valueObject.Control.EventLog",
					"valueObject.Control.StateTree",
					"valueObject.Control.TextProjection",
					"aggregate.Control.AppState",
					"domainService.Control.StateProjector",
					"applicationService.Control.AppLoop",
					"valueObject.Mixer.ChannelParameters",
					"valueObject.Mixer.GlobalParameters",
					"valueObject.Mixer.MixObservation",
					"valueObject.RealTime.ParameterSnapshot",
					"valueObject.RealTime.AudioObservationSnapshot",
					"port.RealTime.AudioObservation",
					"adapter.AtomicAudioObservation",
					"applicationService.RealTime.AudioRenderer",
					"applicationService.Testing.AutomaticMidiTest",
					"valueObject.Testing.LiveDemoScene",
					"valueObject.Testing.LiveDemoCheckpoint",
					"valueObject.Testing.LiveDemoReport",
					"applicationService.Testing.LiveDemoRunner",
					"port.Shell.AppWindow",
					"adapter.EframeTextWindow",
					"port.Shell.AudioOutput",
					"adapter.CpalAudioOutput",
					"applicationService.Shell.StandaloneApplication",
					"asset.CrestSynthMain",
					"asset.BuildMakefile",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: [
					"capability.live_observable_demo",
					"capability.one_way_parameter_control",
					"capability.realtime_execution",
				]
				goals: ["goal.observe_live_synth"]
				description: "a deterministic-clock harness proves the live runner's pacing, exact editable-parameter coverage, coherent checkpoints, bounded audio observations, semantic all-notes-off completion, and inert final state without substituting a second reducer or renderer"
			}
			schema_surface: {
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "schema_surface", "--", "--nocapture"]
				assertions: [
					{kind: "exit_code", expected: 0},
					{kind: "stdout_contains", pattern: "CREST_ACCEPTANCE schema_surface passed"},
				]
				resources: [
					"valueObject.Shell.WindowInput",
					"valueObject.Kernel.MidiMessage",
					"valueObject.Control.AppEvent",
					"valueObject.Control.EventRecord",
					"valueObject.Control.EventLog",
					"valueObject.Control.StateTree",
					"valueObject.Control.TextProjection",
					"domainService.Control.StateProjector",
					"valueObject.Mixer.ChannelParameters",
					"valueObject.Mixer.GlobalParameters",
					"valueObject.RealTime.ParameterSnapshot",
					"valueObject.Testing.DemoScene",
					"applicationService.Testing.ExhaustiveGuiDemo",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: ["capability.instrument_capability_model", "capability.observable_demo_scene"]
				goals: ["goal.control_synth", "goal.observe_synth"]
				description: "typed production surface descriptors and discovered serialized leaves are exactly equal with no missing or unexpected identifiers"
			}
			egui_context: {
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "eframe_context", "--", "--nocapture"]
				assertions: [
					{kind: "exit_code", expected: 0},
					{kind: "stdout_contains", pattern: "CREST_ACCEPTANCE eframe_context passed"},
				]
				resources: [
					"valueObject.Shell.WindowInput",
					"applicationService.Shell.KeyboardInputTranslator",
					"adapter.EframeTextWindow",
					"aggregate.Control.AppState",
					"applicationService.Control.AppLoop",
					"valueObject.Control.EventLog",
					"domainService.Control.StateProjector",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: ["capability.observable_demo_scene", "capability.one_way_parameter_control"]
				goals: ["goal.observe_synth", "goal.control_synth"]
				description: "a headless egui Context dispatches real RawInput through EframeApplication.update into AppLoop and proves the next frame, event record, accepted state, exact projection values, selection, and scroll target all reflect that event"
			}
			mutation_harness: {
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "behavioral_mutation_harness", "--", "--nocapture"]
				assertions: [
					{kind: "exit_code", expected: 0},
					{kind: "stdout_contains", pattern: "CREST_ACCEPTANCE behavioral_mutation_harness passed"},
				]
				resources: [
					"applicationService.Testing.BehavioralMutationHarness",
					"applicationService.Testing.ExhaustiveGuiDemo",
					"applicationService.Shell.KeyboardInputTranslator",
					"aggregate.Control.AppState",
					"applicationService.Control.AppLoop",
					"valueObject.Control.EventRecord",
					"valueObject.Control.EventLog",
					"valueObject.Control.StateTree",
					"domainService.Control.StateProjector",
					"valueObject.RealTime.AudioCommand",
					"valueObject.RealTime.ParameterSnapshot",
					"valueObject.RealTime.PatchAudioBlock",
					"adapter.LockFreeAudioBoundary",
					"applicationService.RealTime.AudioRenderer",
					"valueObject.Mixer.ChannelParameters",
					"valueObject.Mixer.GlobalParameters",
					"domainService.Mixer.MixEngine",
					"port.Mixer.GlobalEffectsProcessor",
					"adapter.GlobalReverbDelay",
					"asset.BehavioralWitnessMain",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: ["capability.observable_demo_scene", "capability.one_way_parameter_control", "capability.global_mix"]
				goals: ["goal.observe_synth"]
				description: "six isolated real-seam mutants cover dropped edits, cross-Patch parameter leakage, MIDI misrouting, schema omission, dry-to-wet bypass, and zero rendering with schema-valid measured counterexamples"
			}
		}

	invariants: core: [
		{text: "AppState.apply is the only control-state mutation path", meta: rationale: "every input follows one reducer path"},
		{text: "input and view adapters emit AppEvents and never mutate application or engine state", meta: rationale: "adapters remain replaceable"},
		{text: "after an event is accepted, the application commits AppState before deriving serialized state, text, parameter snapshots, or audio commands", meta: rationale: "effects always describe accepted state"},
		{text: "the audio callback never allocates, locks, blocks, performs file or device discovery I/O, logs, or destroys owned state", meta: rationale: "the callback has a hard deadline"},
		{text: "AudioBoundary carries discrete MIDI commands, latest control values, and deferred destruction through bounded lock-free primitives", meta: rationale: "the real-time seam is explicit"},
		{text: "Patch and instrument config are capability-polymorphic, while the only installed renderer in this increment is the SoundFont engine configured from ./sf2/HiDef.sf2; unknown capabilities fail without fallback", meta: rationale: "the domain can evolve without pretending an unavailable engine can render"},
		{text: "the only effects are one reverb and one delay shared globally by every channel", meta: rationale: "the signal path stays small"},
		{text: "the MIDI-file module is an automatic test input adapter, not a sequencer or product transport", meta: rationale: "crest-synth remains an instrument"},
	]

	contextMap: [
		{from: "Kernel", to: "Synth", kind: "shared-kernel"},
		{from: "Kernel", to: "Control", kind: "shared-kernel"},
		{from: "Synth", to: "Control", kind: "customer-supplier", direction: "upstream"},
		{from: "Mixer", to: "Control", kind: "customer-supplier", direction: "upstream"},
		{from: "Control", to: "RealTime", kind: "anti-corruption", direction: "downstream"},
		{from: "Testing", to: "Control", kind: "anti-corruption", direction: "downstream"},
		{from: "Testing", to: "Shell", kind: "anti-corruption", direction: "downstream"},
		{from: "Shell", to: "Control", kind: "anti-corruption", direction: "downstream"},
		{from: "Shell", to: "RealTime", kind: "anti-corruption", direction: "downstream"},
	]

	assetKinds: {
		"cargo-manifest": {description: "Cargo.toml", filePattern: "Cargo.toml"}
		"makefile": {description: "GNU Makefile exposing stable human entry points", filePattern: "Makefile"}
		"rust-library-root": {description: "Rust library root", filePattern: "src/lib.rs"}
		"rust-bin-target": {description: "Rust executable composition root", filePattern: "src/bin/*.rs"}
		"rust-integration-tests": {description: "Named Cargo integration-test targets", filePattern: "tests/*.rs"}
	}
}
