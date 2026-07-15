package crestsynth

// Loop is the application control plane. It composes the DDD contexts; it does
// not duplicate their state or contracts. AppState contains canonical domain
// resources and AppEvent carries their commands through one reducer.
project: contexts: Loop: {
	purpose: "the app-wide one-way control and render loop shared by the standalone host, smoke mode, autopilot, and deterministic scenes"
	ubiquitousLanguage: {
		AppEvent: "a closed semantic union over MIDI, gamepad, editor, mixer, patch, and preset commands"
		AppState: "the authoritative non-audio application state composed from canonical Patch, MixerView, EditorState, and Session values"
		StateSnapshot: "a deterministic serialized projection of AppState, never a second state model"
		Scene: "a versioned sequence of AppEvents and render-block advances"
		SceneResult: "measured reducer and audio observations from executing a Scene"
	}
}

project: contexts: Loop: valueObjects: {
	AppEvent: {
		state: {variant: "Midi(MidiEvent) | Gamepad(GamepadAction) | Editor(EditorEvent) | Mixer(MixerViewEvent) | Patch(PatchCommand) | Preset(PresetCommand)"}
		description: "the losslessly serializable event vocabulary used by every live and replay input; PatchCommand covers create/delete/config/mapping/sample/MPE changes and PresetCommand covers save/load/session restore"
		invariants: [
			"the union is closed and exhaustively matched",
			"scene serialization preserves every variant and payload exactly",
			"Scene is not an AppEvent variant; a scene supplies events to the loop",
		]
		validations: [{kind: "test", command: ["cargo", "test", "app_event_roundtrip"], description: "every event variant round-trips losslessly"}]
	}
	EventRejection: {
		state: {eventIndex: "u64", category: "RejectionCategory", reason: "string"}
		description: "a typed explanation of an inapplicable event; rejection is data, never a panic"
	}
	StateSnapshot: {
		state: {version: "u32", bytes: "Vec<u8>", hash: "string"}
		description: "canonical serialization of the complete AppState with stable ordering and no wall-clock data"
		invariants: ["equal AppState values produce byte-identical snapshots", "decode(encode(state)) equals state"]
		validations: [{kind: "test", command: ["cargo", "test", "state_snapshot"], description: "snapshot determinism and complete round-trip pass"}]
	}
	SceneStep: {
		state: {event: "AppEvent", renderBlocks: "u32", caption: "option<string>"}
		description: "apply one event and then advance the production renderer by an explicit number of blocks"
	}
	Scene: {
		state: {version: "u32", name: "string", initialState: "option<StateSnapshot>", steps: "list<SceneStep>"}
		description: "a complete, ordered, versioned application scenario"
		validations: [{kind: "test", command: ["cargo", "test", "scene_roundtrip"], description: "all supported scene event variants round-trip"}]
	}
	SceneResult: {
		state: {finalState: "StateSnapshot", eventsApplied: "u64", rejections: "list<EventRejection>", blocksRendered: "u64", peak: "f64", activeVoiceCount: "u32", stateChanges: "u64"}
		description: "observations measured while the real reducer and production render coordinator execute a scene"
		validations: [{kind: "test", command: ["cargo", "test", "scene_result"], description: "result counters and samples are derived from execution"}]
	}
}

project: contexts: Loop: aggregates: AppState: {
	root: true
	purpose: "the one authoritative control-plane state, composed from the owning DDD contexts"
	state: {
		frame: "u64"
		patches: "list<Patch>"
		mixer: "MixerView"
		editor: "EditorState"
		activeSession: "Session"
		selectedPatch: "option<PatchId>"
	}
	commands: {Apply: {event: "AppEvent"}}
	events: {Applied: {frame: "u64"}, Rejected: {rejection: "EventRejection"}}
	invariants: [
		"apply is the only mutation API exposed by AppState",
		"each event delegates to the owning canonical domain resource and commits the resulting state atomically",
		"accepted events increment frame exactly once; rejected events leave every field including frame unchanged",
		"the same initial state and event sequence always produces equal state and rejection values",
		"MIDI and mixer edits do not bypass the reducer to mutate VoiceAllocator, MixerView, or ChannelStrip directly",
	]
	validations: [{kind: "test", command: ["cargo", "test", "app_state"], description: "all variants delegate, rejected events are atomic, and replay is deterministic"}]
	contributesTo: [{capability: "capability.shared_control_reducer", contribution: "composes canonical DDD state behind the single reducer used by live and replay inputs"}]
}

project: contexts: Loop: ports: SnapshotCodec: {
	direction: "outbound"
	contract: {
		encodeState: "(state: AppState) -> StateSnapshot"
		decodeState: "(snapshot: StateSnapshot) -> result<AppState, CodecError>"
		encodeScene: "(scene: Scene) -> result<Vec<u8>, CodecError>"
		decodeScene: "(bytes: &[u8]) -> result<Scene, CodecError>"
	}
	meta: notes: "serde JSON is an adapter detail; decoding validates the full value before returning it"
}

project: contexts: Loop: domainServices: StateProjector: {
	purpose: "derive the complete immutable ParameterSnapshot from accepted AppState without maintaining another mutable model"
	uses: ["aggregate.Loop.AppState", "valueObject.RealTime.ParameterSnapshot", "port.RealTime.ParameterBridge"]
	validations: [{kind: "test", command: ["cargo", "test", "state_projector"], description: "all audio-readable patch and mixer parameters are projected after accepted events only"}]
	contributesTo: [
		{capability: "capability.shared_control_reducer", contribution: "publishes only state accepted by the authoritative reducer"},
		{capability: "capability.realtime_safe_execution", contribution: "turns control state into immutable latest-wins audio snapshots"},
	]
}

project: contexts: Loop: applicationServices: {
	RenderCoordinator: {
		purpose: "own and advance the canonical patch voice pools, sample players, modulation processors, effect chains, mixer strips, buses, and meters through one reusable stereo render_block operation"
		uses: [
			"aggregate.Loop.AppState", "domainService.Patch.MidiDispatcher",
			"domainService.Engine.VoiceAllocator", "domainService.Engine.EngineRenderer",
			"domainService.Sample.SamplePlayer", "domainService.Modulation.ModProcessor",
			"domainService.Effects.ChainRenderer", "domainService.Mixer.MixEngine",
			"valueObject.Kernel.AudioFrame",
		]
		operations: {
			dispatchMidi: {input: {state: "&AppState", event: "MidiEvent"}, output: {deliveries: "u32"}}
			renderBlock: {input: {state: "&AppState", frames: "&mut [AudioFrame]"}, output: {observation: "RenderObservation"}}
		}
		meta: rules: ["standalone, smoke, autopilot, demos, and SceneRunner call these operations; none owns a substitute audio graph"]
		validations: [{kind: "test", command: ["cargo", "test", "render_coordinator"], description: "multi-patch events produce bounded metered stereo audio through the declared signal path"}]
		contributesTo: [
			{capability: "capability.external_midi_performance", contribution: "connects normalized MIDI to independently owned patch renderers"},
			{capability: "capability.stereo_mix_pipeline", contribution: "provides the one production render function used by all hosts and proofs"},
			{capability: "capability.configurable_instrument_graph", contribution: "composes supported sound resources without local substitute types"},
		]
	}
	SceneRunner: {
		purpose: "execute Scene steps against AppState through AppState.apply and RenderCoordinator, returning observations measured from that execution"
		uses: ["aggregate.Loop.AppState", "valueObject.Loop.Scene", "valueObject.Loop.SceneResult", "port.Loop.SnapshotCodec", "applicationService.Loop.RenderCoordinator"]
		operations: {run: {input: {scene: "Scene", state: "AppState"}, output: {result: "SceneResult"}}}
		meta: rules: ["no no-op renderer, fabricated snapshot extractor, or independent scene event type is permitted"]
		validations: [{kind: "test", command: ["cargo", "test", "scene_runner"], description: "live-equivalent replay is deterministic and its no-op baseline fails observations"}]
		contributesTo: [
			{capability: "capability.shared_control_reducer", contribution: "reuses the live reducer for serialized scenarios"},
			{capability: "capability.deterministic_scene_replay", contribution: "returns deterministic state and measured audio evidence from each replay"},
		]
	}
	StandaloneApplication: {
		purpose: "coordinate Shell ports, AppState, RenderCoordinator, StateProjector, and optional MIDI-file or scene input while keeping the binary composition root thin"
		uses: [
			"aggregate.Loop.AppState", "applicationService.Loop.RenderCoordinator", "applicationService.Loop.SceneRunner", "domainService.Loop.StateProjector",
			"port.Shell.AudioOutput", "port.Shell.MidiInput", "port.Shell.AppWindow", "port.Shell.GuiRenderer", "port.Shell.GamepadInput",
			"domainService.Shell.MidiNormalizer", "domainService.Shell.GamepadNavigator", "domainService.MidiFile.Sequencer",
		]
		operations: {
			handleEvent: {input: {event: "AppEvent"}, output: {result: "result<u64, EventRejection>"}}
			renderAudio: {input: {frames: "&mut [AudioFrame]"}, output: {peak: "f64"}}
			runSmoke: {output: {result: "StandaloneObservation"}}
		}
		meta: rules: ["device adapters translate at the edge; this service owns orchestration and exposes the same functions to every run mode"]
		validations: [{kind: "integration", command: ["make", "ui-smoke"], description: "the complete headless standalone stack dispatches events and renders non-silent metered audio"}]
		contributesTo: [
			{capability: "capability.external_midi_performance", contribution: "coordinates external MIDI through the authoritative state and production render path"},
			{capability: "capability.pointer_free_mixer_control", contribution: "coordinates keyboard and gamepad mixer actions without view-owned mutation"},
			{capability: "capability.shared_control_reducer", contribution: "gives every run mode one application facade over AppState"},
		]
	}
}

project: adapters: SerdeSnapshotCodec: {
	implements: "port.Loop.SnapshotCodec"
	layer: "infrastructure"
	profile: {kind: "persistence", medium: "memory-or-user-selected-file"}
	meta: {framework: "serde + serde_json", rules: ["never writes crest-spec operational state; scene and user sound-state files are product artifacts"]}
	validations: [{kind: "test", command: ["cargo", "test", "serde_snapshot_codec"], description: "complete states and every scene event round-trip with stable bytes"}]
	contributesTo: [
		{capability: "capability.versioned_sound_state", contribution: "encodes complete deterministic application snapshots"},
		{capability: "capability.deterministic_scene_replay", contribution: "loads the versioned scene vocabulary without lossy translation"},
	]
}
