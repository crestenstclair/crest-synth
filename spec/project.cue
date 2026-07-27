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
			"engine-specific branches in Patch, reducer, projector, page projection, selection workflow, rack, renderer, or demo coverage; layering, unprepared structural replacement, or synthesis fallback",
			"effects other than the one global reverb and one global delay",
			"sequencer, transport, timeline, pattern, clip, or song-editing domain models",
			"preset/session persistence or browsing, alternate SoundFont assets, modulation matrices, per-channel inserts, effect chains, buses, or plugin hosting; selecting a preset embedded in the fixed SoundFont is permitted",
			"panels, dashboards, meters, faders, custom widgets, custom drawing, themes, mouse interaction, or the Figma-derived graphical replacement; the two basic text contexts are permitted",
		]
	}

	validations: {
		[string]: {
			workingDirectory: "."
			limits: {
				timeoutMs: 300000
				stdoutBytes: 8388608
				stderrBytes: 8388608
			}
		}
		format: {
			id: "validation.format"
			scope: "project"
			kind: "custom"
			command: ["cargo", "fmt", "--all", "--", "--check"]
			description: "Rust formatting is canonical"
		}
		clippy: {
			id: "validation.clippy"
			scope: "project"
			kind: "compiles"
			command: ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]
			description: "all targets compile without warnings"
		}
		test: {
			id: "validation.test"
			scope: "project"
			kind: "test"
			command: ["cargo", "test", "--all-targets"]
			description: "domain, reducer, adapter, and integration tests pass"
		}
		production_runtime_contracts: {
			id: "validation.production_runtime_contracts"
			scope: "project"
			kind: "integration"
			command: ["cargo", "test", "--test", "production_runtime_contracts", "--", "--nocapture"]
			assertions: [
				{type: "exit-code", equals: 0},
				{type: "stdout-contains", value: "CREST_ACCEPTANCE production_runtime_contracts passed"},
			]
			resources: [
				"port.Synth.InstrumentCapabilityProvider",
				"port.Synth.InstrumentPreparer",
				"applicationService.Synth.DescriptorDefaultConfigFactory",
				"valueObject.Shell.AudioDeviceConfig",
				"valueObject.Shell.AudioDeviceRuntimeError",
				"port.Shell.AudioOutput",
				"port.RealTime.StructuralGraphBoundary",
				"port.RealTime.GraphPreparationWorker",
				"adapter.ThreadedGraphPreparationWorker",
				"port.RealTime.AudioObservation",
				"applicationService.RealTime.AudioRenderer",
				"applicationService.Shell.StandaloneApplication",
				"asset.ProductionRuntimeContractTests",
			]
			capabilities: ["capability.instrument_capability_model", "capability.asynchronous_engine_selection", "capability.prepared_engine_rack", "capability.realtime_execution"]
			goals: ["goal.play_test_song", "goal.control_synth", "goal.select_patch_engine"]
			description: "the production constructor, injected capacity-one preparation worker, negotiated graph preparation, complete callback adaptation, control-owned worker shutdown, visible device failure, and routing observation contracts pass together"
		}
		zero_selection_guard: {
			id: "validation.zero_selection_guard"
			scope: "project"
			kind: "custom"
			command: ["bash", "scripts/run_exact_test_validation.sh", "--self-test"]
			assertions: [
				{type: "exit-code", equals: 0},
				{type: "stdout-contains", value: "CREST_TEST_VALIDATION zero-selection-rejected passed"},
			]
			resources: ["asset.ProductionRuntimeContractTests"]
			capabilities: ["capability.realtime_execution"]
			goals: ["goal.play_test_song"]
			description: "a declared test selector cannot pass with zero executed tests even when broad-suite text claims success"
		}
		audio_renderer_realtime_contract: {
			id: "validation.audio_renderer_realtime_contract"
			scope: "project"
			kind: "integration"
			command: ["bash", "scripts/run_exact_test_validation.sh", "production_runtime_contracts", "audio_renderer_realtime_contract", "CREST_RT_VALIDATION audio_renderer_realtime_contract passed"]
			assertions: [
				{type: "exit-code", equals: 0},
				{type: "stdout-contains", value: "\"testsExecuted\":1"},
			]
			resources: ["applicationService.RealTime.AudioRenderer", "applicationService.Shell.StandaloneApplication", "asset.ProductionRuntimeContractTests"]
			capabilities: ["capability.prepared_engine_rack", "capability.realtime_execution"]
			goals: ["goal.play_test_song"]
			description: "the production renderer selector executes exactly one assertion-bearing negotiated-capacity and complete-chunking test"
		}
		prepared_graph_handoff_contract: {
			id: "validation.prepared_graph_handoff_contract"
			scope: "project"
			kind: "integration"
			command: ["bash", "scripts/run_exact_test_validation.sh", "production_runtime_contracts", "prepared_graph_handoff", "CREST_RT_VALIDATION prepared_graph_handoff passed"]
			assertions: [
				{type: "exit-code", equals: 0},
				{type: "stdout-contains", value: "\"testsExecuted\":1"},
			]
			resources: ["port.RealTime.StructuralGraphBoundary", "applicationService.RealTime.AudioRenderer", "asset.ProductionRuntimeContractTests"]
			capabilities: ["capability.prepared_engine_rack", "capability.realtime_execution"]
			goals: ["goal.play_test_song"]
			description: "the prepared-graph handoff selector executes exactly one assertion-bearing ownership and collection test"
		}
		audio_observation_realtime_contract: {
			id: "validation.audio_observation_realtime_contract"
			scope: "project"
			kind: "integration"
			command: ["bash", "scripts/run_exact_test_validation.sh", "production_runtime_contracts", "audio_observation_realtime_contract", "CREST_RT_VALIDATION audio_observation_realtime_contract passed"]
			assertions: [
				{type: "exit-code", equals: 0},
				{type: "stdout-contains", value: "\"testsExecuted\":1"},
			]
			resources: ["port.RealTime.AudioObservation", "applicationService.RealTime.AudioRenderer", "asset.ProductionRuntimeContractTests"]
			capabilities: ["capability.prepared_engine_rack", "capability.realtime_execution"]
			goals: ["goal.play_test_song"]
			description: "the audio-observation selector executes exactly one assertion-bearing unknown-Patch routing-failure test"
		}
			smoke: {
				id: "validation.smoke"
			scope: "project"
			kind: "integration"
			command: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke"]
			timeout: "180s"
				resources: [
					"adapter.HiDefSoundFontAsset",
					"valueObject.Synth.SoundFontPresetCatalog",
					"adapter.HiDefSoundFontPreparer",
				"adapter.BraidsPreparer",
				"applicationService.Synth.PreparedEngineRackBuilder",
				"applicationService.Synth.DescriptorDefaultConfigFactory",
				"applicationService.RealTime.PreparedGraphBuilder",
				"aggregate.RealTime.PreparedEngineRack",
				"aggregate.RealTime.PreparedGraph",
				"adapter.LockFreeStructuralGraphBoundary",
				"adapter.ThreadedGraphPreparationWorker",
				"adapter.CorridorsMidiEventSource",
				"applicationService.Testing.AutomaticMidiTest",
				"applicationService.Shell.StandaloneApplication",
				"asset.CrestSynthMain",
			]
			capabilities: [
				"capability.prepared_engine_rack",
					"capability.soundfont_audio",
					"capability.soundfont_preset_selection",
				"capability.braids_engine",
				"capability.per_voice_envelope",
				"capability.automatic_test_midi",
				"capability.one_way_parameter_control",
				"capability.asynchronous_engine_selection",
				"capability.realtime_execution",
			]
				goals: ["goal.play_test_song", "goal.control_synth", "goal.select_soundfont_preset"]
					description: "one fixed SoundFont parse supplies exact catalog choices and numeric prepared data while the MIDI fixture alternates real SoundFont and Braids engines through the control, mixer, and audio path"
			}
			prepared_engine_rack: {
				id: "validation.prepared_engine_rack"
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "prepared_engine_rack", "--", "--nocapture"]
				assertions: [
					{type: "exit-code", equals: 0},
					{type: "stdout-contains", value: "CREST_ACCEPTANCE prepared_engine_rack passed"},
				]
				resources: [
					"port.Synth.PreparedInstrument",
					"port.Synth.InstrumentPreparer",
					"applicationService.Synth.PreparedEngineRackBuilder",
					"aggregate.RealTime.PreparedEngineRack",
					"valueObject.RealTime.GraphRevision",
					"aggregate.RealTime.PreparedGraph",
					"valueObject.RealTime.GraphHandoffStatus",
					"port.RealTime.StructuralGraphBoundary",
					"adapter.LockFreeStructuralGraphBoundary",
					"applicationService.RealTime.PreparedGraphBuilder",
					"applicationService.RealTime.StructuralGraphCoordinator",
					"applicationService.RealTime.AudioRenderer",
					"adapter.HiDefSoundFontPreparer",
					"adapter.BraidsPreparer",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: ["capability.prepared_engine_rack", "capability.realtime_execution", "capability.soundfont_audio", "capability.braids_engine", "capability.per_voice_envelope"]
				goals: ["goal.play_test_song", "goal.control_synth"]
				description: "the named rack target proves bounded mixed SoundFont/Braids dispatch, matching scalar/envelope projection, graph replacement, pressure recovery, and off-callback destruction"
			}
			braids_engine: {
				id: "validation.braids_engine"
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "braids_engine", "--", "--nocapture"]
				assertions: [
					{type: "exit-code", equals: 0},
					{type: "stdout-contains", value: "CREST_ACCEPTANCE braids_engine passed"},
				]
				resources: [
					"adapter.BraidsCapability",
					"adapter.BraidsPreparer",
					"port.Synth.PreparedInstrument",
					"aggregate.RealTime.PreparedEngineRack",
					"applicationService.RealTime.AudioRenderer",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: ["capability.braids_engine", "capability.prepared_engine_rack", "capability.realtime_execution"]
				goals: ["goal.play_test_song", "goal.control_synth"]
				description: "the pinned native MacroOscillator adapter proves 47 models, exact 48/96 kHz adaptation, one independent sixteen-voice bank per Braids Patch, 16 × N scaling, Patch-local stealing, scalar isolation, finite output, lifecycle safety, and bounded timing"
			}
			per_voice_envelope: {
				id: "validation.per_voice_envelope"
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "per_voice_envelope", "--", "--nocapture"]
				assertions: [
					{type: "exit-code", equals: 0},
					{type: "stdout-contains", value: "CREST_ACCEPTANCE per_voice_envelope passed"},
				]
				resources: [
					"valueObject.Synth.VoiceEnvelope",
					"adapter.HiDefSoundFontPreparer",
					"adapter.BraidsPreparer",
					"domainService.Control.StateProjector",
					"applicationService.RealTime.AudioRenderer",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: ["capability.per_voice_envelope", "capability.schema_driven_patch_page", "capability.one_way_parameter_control", "capability.soundfont_audio", "capability.braids_engine", "capability.realtime_execution"]
				goals: ["goal.play_test_song", "goal.control_synth", "goal.edit_patch_envelope"]
				description: "all four ADSR values edit through canonical MIXER and PATCH reducer paths, project exactly, and independently shape overlapping SoundFont and Braids note voices with bounded callback work"
			}
				capability_schema: {
					id: "validation.capability_schema"
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "capability_schema", "--", "--nocapture"]
				assertions: [
					{type: "exit-code", equals: 0},
					{type: "stdout-contains", value: "CREST_ACCEPTANCE capability_schema passed"},
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
						"valueObject.Synth.SoundFontPresetId",
						"valueObject.Synth.SoundFontPresetCatalog",
						"port.Synth.InstrumentCapabilityProvider",
						"adapter.HiDefSoundFontAsset",
					"adapter.HiDefSoundFontCapability",
					"adapter.BraidsCapability",
					"valueObject.Synth.VoiceEnvelope",
					"aggregate.Synth.Patch",
					"aggregate.Control.AppState",
					"domainService.Control.StateProjector",
					"applicationService.Testing.AutomaticMidiTest",
					"asset.BehavioralAcceptanceTests",
				]
					capabilities: ["capability.instrument_capability_model", "capability.one_way_parameter_control", "capability.soundfont_audio", "capability.soundfont_preset_selection"]
					goals: ["goal.control_synth", "goal.select_soundfont_preset"]
						description: "both exact installed descriptors, the catalog-hydrated SoundFont preset Choice, and alternating generic fixture Patch configs survive reducer installation, serialization, and projection while malformed and unknown configs fail without fallback"
				}
				patch_page_projection: {
					id: "validation.patch_page_projection"
					scope: "project"
					kind: "integration"
					command: ["cargo", "test", "--test", "patch_page_projection", "--", "--nocapture"]
					assertions: [
						{type: "exit-code", equals: 0},
						{type: "stdout-contains", value: "CREST_ACCEPTANCE patch_page_projection passed"},
					]
					resources: [
						"valueObject.Control.TopLevelContext",
							"valueObject.Control.InteractionState",
							"valueObject.Control.PatchControlId",
							"valueObject.Control.StructuralEditIntent",
						"valueObject.Control.EngineSelectionStatus",
						"valueObject.Control.EngineSelectionFailure",
						"valueObject.Control.AppEvent",
						"valueObject.Control.PatchPageProjection",
						"valueObject.Control.TextProjection",
						"aggregate.Control.AppState",
						"domainService.Control.StateProjector",
						"applicationService.Control.AppLoop",
						"valueObject.Shell.WindowInput",
						"applicationService.Shell.KeyboardInputTranslator",
						"adapter.EframeTextWindow",
						"valueObject.RealTime.ParameterSnapshot",
						"port.RealTime.AudioBoundary",
						"applicationService.RealTime.AudioRenderer",
						"aggregate.RealTime.PreparedGraph",
						"asset.BehavioralAcceptanceTests",
					]
						capabilities: ["capability.schema_driven_patch_page", "capability.asynchronous_engine_selection", "capability.instrument_capability_model", "capability.one_way_parameter_control", "capability.per_voice_envelope", "capability.soundfont_preset_selection"]
						goals: ["goal.inspect_patch", "goal.select_patch_engine", "goal.edit_patch_envelope", "goal.select_soundfont_preset"]
						description: "direct 1/2 input, reducer-owned Patch and dynamic Engine-plus-ADSR-plus-descriptor-StructuralChoice focus, exact SoundFont/Braids rows and lifecycle status, preserved MIXER projection, audio-neutral focus/context navigation, canonical ADSR editing, and semantic engine/preset request projection pass through production seams"
				}
				control_dispatch_performance: {
					id: "validation.control_dispatch_performance"
					scope: "project"
					kind: "integration"
					command: ["cargo", "test", "--test", "control_dispatch_performance", "--", "--nocapture"]
					assertions: [
						{type: "exit-code", equals: 0},
						{type: "stdout-contains", value: "CREST_ACCEPTANCE control_dispatch_performance passed"},
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
				id: "validation.demo_scene"
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "exhaustive_demo_scene", "--", "--nocapture"]
				assertions: [
					{type: "exit-code", equals: 0},
					{type: "stdout-contains", value: "CREST_ACCEPTANCE exhaustive_demo_scene passed"},
				]
				timeout: "180s"
					resources: [
						"adapter.HiDefSoundFontAsset",
						"valueObject.Synth.SoundFontPresetId",
						"valueObject.Synth.SoundFontPresetCatalog",
						"valueObject.Control.AppEvent",
					"valueObject.Control.TopLevelContext",
						"valueObject.Control.InteractionState",
						"valueObject.Control.PatchControlId",
						"valueObject.Control.StructuralEditIntent",
					"valueObject.Control.EngineSelectionStatus",
					"valueObject.Control.EngineSelectionFailure",
					"valueObject.Control.EngineSelectionEffect",
					"valueObject.Control.PatchPageProjection",
					"valueObject.Control.EventRecord",
					"valueObject.Control.EventLog",
					"valueObject.Control.StateTree",
					"valueObject.Control.TextProjection",
					"aggregate.Control.AppState",
					"domainService.Control.StateProjector",
					"applicationService.Control.AppLoop",
					"applicationService.Synth.DescriptorDefaultConfigFactory",
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
					"port.RealTime.GraphPreparationWorker",
					"adapter.DeterministicGraphPreparationWorker",
					"applicationService.RealTime.StructuralGraphCoordinator",
					"applicationService.RealTime.AudioRenderer",
					"valueObject.Testing.DemoScene",
					"valueObject.Testing.DemoSceneReport",
					"valueObject.Testing.EngineSelectionObservation",
					"applicationService.Testing.ExhaustiveGuiDemo",
					"applicationService.Shell.StandaloneApplication",
					"asset.CrestSynthMain",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: [
					"capability.instrument_capability_model",
					"capability.observable_demo_scene",
					"capability.one_way_parameter_control",
					"capability.schema_driven_patch_page",
						"capability.asynchronous_engine_selection",
						"capability.soundfont_preset_selection",
					"capability.global_mix",
				]
					goals: ["goal.select_patch_engine", "goal.edit_patch_envelope", "goal.select_soundfont_preset", "goal.observe_synth"]
					description: "the exhaustive deterministic GUI scene covers every current event, focused PATCH ADSR and catalog-backed preset control, editable scalar and structural controls, coexistence, lifecycle state/effect/failure, serialized property, projection, and downstream audio consequence"
			}
			live_demo: {
				id: "validation.live_demo"
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "live_demo", "--", "--nocapture"]
				assertions: [
					{type: "exit-code", equals: 0},
					{type: "stdout-contains", value: "CREST_ACCEPTANCE live_demo_scene passed"},
				]
				timeout: "180s"
					resources: [
						"adapter.HiDefSoundFontAsset",
						"valueObject.Synth.SoundFontPresetCatalog",
						"valueObject.Control.AppEvent",
					"valueObject.Control.TopLevelContext",
						"valueObject.Control.InteractionState",
						"valueObject.Control.PatchControlId",
						"valueObject.Control.StructuralEditIntent",
					"valueObject.Control.PatchPageProjection",
					"valueObject.Control.EventRecord",
					"valueObject.Control.EventLog",
					"valueObject.Control.StateTree",
					"valueObject.Control.TextProjection",
					"valueObject.Control.EngineSelectionStatus",
					"valueObject.Control.EngineSelectionEffect",
					"aggregate.Control.AppState",
					"domainService.Control.StateProjector",
					"applicationService.Control.AppLoop",
					"applicationService.Synth.DescriptorDefaultConfigFactory",
					"valueObject.Mixer.ChannelParameters",
					"valueObject.Mixer.GlobalParameters",
					"valueObject.Mixer.MixObservation",
					"valueObject.RealTime.ParameterSnapshot",
					"valueObject.RealTime.AudioObservationSnapshot",
					"port.RealTime.AudioObservation",
					"adapter.AtomicAudioObservation",
					"applicationService.RealTime.AudioRenderer",
					"port.RealTime.GraphPreparationWorker",
					"adapter.ThreadedGraphPreparationWorker",
					"applicationService.RealTime.StructuralGraphCoordinator",
					"port.RealTime.StructuralGraphBoundary",
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
					"capability.schema_driven_patch_page",
					"capability.per_voice_envelope",
						"capability.asynchronous_engine_selection",
						"capability.soundfont_preset_selection",
					"capability.realtime_execution",
				]
					goals: ["goal.edit_patch_envelope", "goal.select_soundfont_preset", "goal.observe_live_synth"]
					description: "a deterministic-clock harness proves live pacing, schedule-independent semantic Patch probes around every scalar checkpoint, the focused Patch's four ADSR instances through PATCH, exact scalar coverage, one adjacent SoundFont preset replacement plus SoundFont-to-Braids-to-descriptor-default-SoundFont through the worker and structural seams, coherent lifecycle/revision checkpoints, finite targeted audio, semantic all-notes-off completion, mapped-input isolation, controlled no-progress timeout cleanup, one completed report, and immediate successful close without substituting a second reducer, worker workflow, or renderer"
			}
			schema_surface: {
				id: "validation.schema_surface"
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "schema_surface", "--", "--nocapture"]
				assertions: [
					{type: "exit-code", equals: 0},
					{type: "stdout-contains", value: "CREST_ACCEPTANCE schema_surface passed"},
				]
				resources: [
					"valueObject.Shell.WindowInput",
					"valueObject.Kernel.MidiMessage",
					"valueObject.Control.TopLevelContext",
					"valueObject.Control.InteractionState",
					"valueObject.Control.EngineSelectionStatus",
					"valueObject.Control.EngineSelectionFailure",
					"valueObject.Control.EngineSelectionEffect",
					"valueObject.Control.AppEvent",
					"valueObject.Control.EventRecord",
					"valueObject.Control.EventLog",
					"valueObject.Control.StateTree",
					"valueObject.Control.PatchPageProjection",
					"valueObject.Control.TextProjection",
					"domainService.Control.StateProjector",
					"valueObject.Mixer.ChannelParameters",
					"valueObject.Mixer.GlobalParameters",
					"valueObject.RealTime.ParameterSnapshot",
					"valueObject.Testing.DemoScene",
					"applicationService.Testing.ExhaustiveGuiDemo",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: ["capability.instrument_capability_model", "capability.schema_driven_patch_page", "capability.asynchronous_engine_selection", "capability.observable_demo_scene"]
				goals: ["goal.control_synth", "goal.inspect_patch", "goal.select_patch_engine", "goal.observe_synth"]
				description: "typed production surface descriptors and discovered serialized leaves are exactly equal with no missing or unexpected identifiers"
			}
			egui_context: {
				id: "validation.egui_context"
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "eframe_context", "--", "--nocapture"]
				assertions: [
					{type: "exit-code", equals: 0},
					{type: "stdout-contains", value: "CREST_ACCEPTANCE eframe_context passed"},
				]
				resources: [
					"valueObject.Shell.WindowInput",
					"valueObject.Control.TopLevelContext",
					"valueObject.Control.InteractionState",
					"valueObject.Control.EngineSelectionStatus",
					"valueObject.Control.PatchPageProjection",
					"applicationService.Shell.KeyboardInputTranslator",
					"adapter.EframeTextWindow",
					"aggregate.Control.AppState",
					"applicationService.Control.AppLoop",
					"valueObject.Control.EventLog",
					"domainService.Control.StateProjector",
					"asset.BehavioralAcceptanceTests",
				]
				capabilities: ["capability.observable_demo_scene", "capability.one_way_parameter_control", "capability.schema_driven_patch_page", "capability.asynchronous_engine_selection"]
				goals: ["goal.observe_synth", "goal.control_synth", "goal.inspect_patch", "goal.select_patch_engine"]
				description: "a headless egui Context dispatches real RawInput through EframeApplication.update into AppLoop and proves the next frame, event record, accepted state, exact projection values, engine-row lifecycle status, selection, and scroll target all reflect that event"
			}
			mutation_harness: {
				id: "validation.mutation_harness"
				scope: "project"
				kind: "integration"
				command: ["cargo", "test", "--test", "behavioral_mutation_harness", "--", "--nocapture"]
				assertions: [
					{type: "exit-code", equals: 0},
					{type: "stdout-contains", value: "CREST_ACCEPTANCE behavioral_mutation_harness passed"},
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
			{text: "PATCH and MIXER are the only top-level contexts; InteractionState owns context and stable PATCH focus resolved as Engine, canonical ADSR, then descriptor-declared StructuralChoice controls, while the basic window renders only the immutable active-context projection", meta: rationale: "page selection remains semantic, deterministic, capability-driven, and independent from UI state"},
		{text: "after an event is accepted, the application commits AppState before deriving serialized state, text, parameter snapshots, or audio commands", meta: rationale: "effects always describe accepted state"},
			{text: "engine and structural parameter selection share one app-wide, one-in-flight reducer-owned lifecycle; a capacity-one worker prepares off callback, PreparedGraph stays outside AppState, only the intended config assignment commits before target-revision projection/publication, and Ready requires block-boundary activation plus off-callback retirement collection", meta: rationale: "structural edits preserve one-way mutation and hard-real-time ownership"},
		{text: "the audio callback never allocates, locks, blocks, performs file or device discovery I/O, logs, or destroys owned state", meta: rationale: "the callback has a hard deadline"},
		{text: "AudioBoundary carries discrete MIDI commands and latest scalar values, while StructuralGraphBoundary uses distinct bounded queues for prepared and retired graph ownership plus fixed-size acknowledgement", meta: rationale: "each real-time datum uses its required delivery semantics"},
			{text: "Patch and instrument config are capability-polymorphic and PreparedEngineRack hosts exactly the HiDef SoundFont and pinned Braids preparers; one SoundFont parse yields a string-bearing control catalog and separate numeric callback bank, SoundFont uses one EngineManaged synthesizer per Patch, every Braids Patch owns FixedPerPatch(16), both use the common per-note envelope, and unknown capabilities or presets fail without fallback", meta: rationale: "two materially different voice policies and one metadata-safe structural choice let the Patch page prove schema projection without engine branches"},
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
