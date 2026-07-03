package crestsynth

// Loop — the one-way (Elm/Flux) data loop that owns the entire control plane.
// Events in, state out. Views render state; adapters emit events; NOTHING
// else mutates. The audio thread is downstream: it consumes ParameterSnapshot
// projections of AppState across the RealTime boundary, unchanged.

project: contexts: Loop: purpose: "the app-wide one-way event loop: a single AppEvent union, a single AppState store, and a pure reducer that is the only mutation path in the control plane"

project: contexts: Loop: ubiquitousLanguage: {
	AppEvent: "a semantic event — MIDI, gamepad, editor, mixer, patch, preset, scene control — the only thing that mutates AppState"
	AppState: "the single source of truth for the control plane: patches, mixer, editor view state, active preset/session"
	Reducer:  "apply(state, event) -> state; pure, total, allocation-free"
}

project: contexts: Loop: valueObjects: {
	AppEvent: {
		description: "closed union over every control-plane event: normalized MidiEvent, GamepadAction, editor navigation/edit events, mixer strip changes, patch commands, preset load/save; each variant carries its aggregate's existing command payload"
		invariants: ["every variant is serializable and deserializable losslessly (scenes are files)"]
	}
	EventRejection: {
		state: {reason: "string"}
		description: "why an event was not applicable in the current state (unknown target, out-of-range value); rejections are values, never panics"
	}
	StateSnapshot: {
		description: "a complete, deterministic serialization of AppState: stable field order, stable map ordering, no wall-clock timestamps — byte-identical for identical states"
		invariants: [
			"serializing the same AppState twice yields byte-identical output",
			"a snapshot round-trips: deserialize(serialize(state)) equals the original state",
		]
		validations: [{kind: "test", command: ["cargo", "test", "state_snapshot"], description: "StateSnapshot determinism and round-trip unit tests pass"}]
	}
	SceneStep: {
		state: {event: "AppEvent", renderBlocks: "u32"}
		description: "one step: apply an event, then optionally render N audio blocks headlessly (so audible consequences accrue between events)"
	}
	Scene: {
		state: {name: "string", steps: "list<SceneStep>"}
		description: "a named, ordered, serializable event sequence — the unit of control-plane demonstration and evaluation"
		invariants: ["a scene file round-trips losslessly through the SnapshotCodec's format"]
	}
}

project: contexts: Loop: aggregates: AppState: {
	root:    true
	purpose: "the whole control-plane state as one value: patch set, mixer, editor view state, active session"
	state: {frame: "u64"}
	commands: {
		Apply: {event: "AppEvent"}
	}
	events: {
		Applied: {frame: "u64"}
		Rejected: {reason: "string"}
	}
	invariants: [
		"apply(event) is the ONLY way to mutate AppState — no setters, no direct field mutation anywhere in the control plane",
		"apply is pure and deterministic: the same initial state and the same event sequence always produce an identical AppState",
		"apply never panics: an inapplicable event yields an EventRejection value and leaves state unchanged",
		"frame increments by exactly one per applied event — it is the event-sequence clock, not wall-clock",
	]
}

project: contexts: Loop: ports: {
	SnapshotCodec: {
		contract: {
			encode: "(state: AppState) -> StateSnapshot"
			decode: "(snapshot: StateSnapshot) -> result<AppState, CodecError>"
		}
		meta: notes: "human- and LLM-readable text format (JSON); field names are the ubiquitous language, values are plain (dB as numbers, booleans as booleans)"
	}
}

project: contexts: Loop: domainServices: {
	StateProjector: {
		purpose: "projects AppState into the ParameterSnapshot the audio thread reads — the one bridge from the loop to the RealTime boundary"
		uses: ["aggregate.Loop.AppState", "port.RealTime.ParameterBridge"]
	}
	SceneRunner: {
		purpose: "executes a Scene against a fresh AppState through the SAME apply path the live app uses: per step, apply the event, render the requested blocks, and (when asked) emit a StateSnapshot; produces the final snapshot plus per-step snapshots on demand"
		uses: ["aggregate.Loop.AppState", "valueObject.Loop.Scene", "port.Loop.SnapshotCodec", "domainService.Loop.StateProjector", "domainService.Engine.EngineRenderer"]
	}
}

project: adapters: SerdeSnapshotCodec: {
	implements: "port.Loop.SnapshotCodec"
	layer:      "infrastructure"
	meta: framework: "serde"
}
