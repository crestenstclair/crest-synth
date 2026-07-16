package crestsynth

// Loop is the application control plane. It composes the DDD contexts; it does
// not duplicate their state or contracts. AppState contains canonical domain
// resources and AppEvent carries their commands through one reducer.
project: contexts: Loop: {
	purpose: "the app-wide one-way control and render loop shared by the standalone host, smoke mode, autopilot, and deterministic scenes"
	ubiquitousLanguage: {
		AppEvent: "a closed semantic union over MIDI, mixer, playback, gamepad, editor, patch, and preset commands; the L key becomes Playback(ToggleFromStart)"
		AppState: "the authoritative non-audio application state composed from canonical Patch, MixerView, TestPlayback, EditorState, and Session values"
		StateSnapshot: "a deterministic serialized projection of AppState, never a second state model"
		Scene: "a versioned sequence of AppEvents and render-block advances"
		SceneResult: "measured reducer and audio observations from executing a Scene"
	}
}

project: contexts: Loop: valueObjects: {
	AppEvent: {
		state: {variant: "Midi { event: MidiEvent, targetPatch: option<PatchId> } | Mixer(MixerViewEvent) | Playback(PlaybackCommand) | Gamepad(GamepadAction) | Editor(EditorEvent) | Patch(PatchCommand) | Preset(PresetCommand)"}
		description: "the losslessly serializable event vocabulary used by every live and replay input; targetPatch is Some only for a prepared MIDI-file test event, while external MIDI supplies None and follows normal channel mapping"
		invariants: [
			"the union is closed and exhaustively matched",
			"scene serialization preserves every variant and payload exactly",
			"Scene is not an AppEvent variant; a scene supplies events to the loop",
			"a targeted MIDI event is accepted only when its PatchId exists in AppState; it is delivered exactly once to that patch and never re-dispatched by channel",
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
		playback: "TestPlayback"
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
		"after every accepted mixer edit, canonical StateSnapshot serialization, MixerTextProjection, and ParameterSnapshot describe the same typed value before audio rendering continues",
		"playback commands update TestPlayback in the same reducer; start/stop effects are derived only after the new state commits",
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
	purpose: "derive both the immutable audio ParameterSnapshot and disposable MixerTextProjection from the same accepted AppState and canonical serialization"
	uses: ["aggregate.Loop.AppState", "valueObject.RealTime.ParameterSnapshot", "valueObject.Mixer.MixerTextProjection", "port.RealTime.ParameterBridge", "port.Loop.SnapshotCodec"]
	meta: rules: [
		"expose projectParameters(state: &AppState) -> ParameterSnapshot and projectMixerText(state: &AppState) -> result<MixerTextProjection, CodecError>",
		"projectMixerText first obtains the canonical StateSnapshot from SnapshotCodec, then formats exact decoded Patch and ChannelStrip values; it never maintains a second editable view model",
		"projectParameters and projectMixerText run only after AppState accepts an event, and both are derived from that same accepted frame",
	]
	validations: [{kind: "test", command: ["cargo", "test", "state_projector"], description: "every mixer edit produces matching canonical serialization, text projection, and audio ParameterSnapshot from one AppState frame"}]
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
			dispatchMidi: {input: {state: "&AppState", event: "MidiEvent", targetPatch: "option<PatchId>"}, output: {deliveries: "result<u32, EventRejection>"}}
			releaseTestPlaybackVoices: {input: {state: "&AppState", generation: "u64"}, output: {released: "u32"}}
			renderBlock: {input: {state: "&AppState", frames: "&mut [AudioFrame]"}, output: {observation: "RenderObservation"}}
		}
		meta: rules: [
			"standalone, smoke, autopilot, demos, and SceneRunner call these operations; none owns a substitute audio graph",
			"targetPatch None delegates to MidiDispatcher channel/layer mapping; Some delivers exactly once to the generated test patch after validating it exists",
			"when an accepted Playback event stops or restarts a generation, release every voice owned by the prior test-playback generation before dispatching new scheduled events",
		]
		validations: [{kind: "test", command: ["cargo", "test", "render_coordinator"], description: "multi-patch events produce bounded metered stereo audio through the declared signal path"}]
		contributesTo: [
			{capability: "capability.external_midi_performance", contribution: "connects normalized MIDI to independently owned patch renderers"},
			{capability: "capability.instrument_partitioned_test_playback", contribution: "delivers each prepared MIDI-file event to its exact generated instrument patch without changing live MIDI routing"},
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
			"domainService.Shell.MidiNormalizer", "domainService.Shell.GamepadNavigator", "applicationService.MidiFile.TestPlaybackAssembler", "domainService.MidiFile.Sequencer",
		]
		operations: {
			handleEvent: {input: {event: "AppEvent"}, output: {result: "result<u64, EventRejection>"}}
			mixerTextView: {output: {projection: "result<MixerTextProjection, CodecError>"}}
			prepareMidiFilePlayback: {input: {song: "Song"}, output: {plan: "result<TestPlaybackPlan, PlaybackPlanError>"}}
			renderAudio: {input: {frames: "&mut [AudioFrame]"}, output: {peak: "f64"}}
			runSmoke: {output: {result: "StandaloneObservation"}}
		}
		meta: rules: [
			"device adapters translate at the edge; this service owns orchestration and exposes the same functions to every run mode",
			"after handleEvent accepts a mixer adjustment, serialize the new AppState, rebuild MixerTextProjection, publish the matching ParameterSnapshot, and let renderAudio consume that snapshot; rejection performs none of these steps",
			"mixerTextView returns the large immutable text projection consumed by the minimal renderer and exposes no mutable UI-specific model",
			"MIDI-file test playback resolves every instrument from ./sf2/HiDef.sf2, prepares its sample Patch, and installs the plan before accepting L playback commands",
			"the L key is translated at the shell edge to AppEvent::Playback(ToggleFromStart); handleEvent applies it, then orchestrates voice release or Sequencer output from the accepted TestPlayback state",
			"ScheduledPatchEvents return as AppEvent::Midi with exact targetPatch values, so playback uses the same reducer/effect loop rather than calling the engine from the key handler or Sequencer",
		]
		validations: [{kind: "integration", command: ["make", "ui-smoke"], description: "the complete headless standalone stack dispatches events and renders non-silent metered audio"}]
		contributesTo: [
			{capability: "capability.external_midi_performance", contribution: "coordinates external MIDI through the authoritative state and production render path"},
			{capability: "capability.pointer_free_mixer_control", contribution: "coordinates keyboard and gamepad mixer actions without view-owned mutation"},
			{capability: "capability.shared_control_reducer", contribution: "gives every run mode one application facade over AppState"},
			{capability: "capability.instrument_partitioned_test_playback", contribution: "orchestrates instrument discovery, canonical patch installation, targeted scheduling, and rendering through the normal application facade"},
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
