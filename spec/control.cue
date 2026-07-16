package crestsynth

project: contexts: Control: {
	purpose: "the one-way application state, semantic events, and projections"

	valueObjects: AppEvent: {
		description: "the closed semantic input union accepted by AppState"
		state: {
			kind: "Navigate | Adjust | InstallPatches | Midi"
			payload: "event-specific bounded payload"
		}
		invariants: [
			"Navigate and Adjust carry Direction",
			"InstallPatches is accepted only during startup on the control thread",
			"Midi carries PatchId and MidiMessage",
			"raw key codes, window objects, clocks, files, and audio devices never appear in AppEvent",
		]
		contributesTo: [{capability: "capability.one_way_parameter_control", contribution: "gives every input one semantic route into AppState"}]
	}

	valueObjects: StateSnapshot: {
		description: "canonical serialized control state"
		state: {
			json: "String"
			hash: "String"
		}
		invariants: ["JSON contains every Patch, every ChannelParameters value, GlobalParameters, and Selection", "decode(encode(state)) equals state"]
		contributesTo: [{capability: "capability.one_way_parameter_control", contribution: "makes accepted state serialization explicit and testable"}]
	}

	valueObjects: TextProjection: {
		description: "the complete single-screen text body derived from StateSnapshot"
		state: {
			body: "String"
			selectedLine: "usize"
			stateHash: "String"
		}
		invariants: [
			"the body begins with KEYS: W/S parameters | A/D channels | K+direction edit",
			"each Patch appears once in stable AppState order with id, name, channel, bank, program, percussion, gainDb, pan, reverbSend, and delaySend",
			"Patch sections are separated by the literal ------------------------------------------------------------",
			"the final GLOBAL section lists masterGainDb, reverbRoomSize, reverbDamping, reverbReturn, delayMilliseconds, delayFeedback, and delayReturn",
			"the selected line begins with > and every other parameter line begins with one space",
			"stateHash equals the StateSnapshot hash used to create the body",
		]
		contributesTo: [{capability: "capability.one_way_parameter_control", contribution: "provides the entire UI as one deterministic wall of text"}]
	}

	aggregates: AppState: {
		root: true
		purpose: "own installed Patches, global parameters, selection, and the accepted generation"
		state: {
			patches: "Vec<Patch>"
			global: "GlobalParameters"
			selection: "Selection { section: Patch | Global, patchIndex: usize, parameterIndex: usize }"
			generation: "u64"
		}
		commands: Apply: {event: "AppEvent"}
		events: StateAccepted: {generation: "u64"}
		invariants: [
			"Apply is the only mutation method",
			"Navigate changes only Selection",
			"bare Up/Down moves between parameters and bare Left/Right moves between Patch sections plus the GLOBAL section",
			"Adjust changes exactly one selected value and clamps it to the owning value object's invariant",
			"Adjust Left/Right is the fine decrement/increment and Adjust Down/Up is the coarse decrement/increment",
			"InstallPatches preserves fixture discovery order and is rejected after startup",
			"Midi does not mutate synth parameters; it yields one AudioCommand effect after state acceptance",
			"every accepted event increments generation once and every rejected event leaves state identical",
		]
		validations: [{kind: "test", command: ["cargo", "test", "app_state"], description: "navigation, K-modified adjustment, bounds, installation, and MIDI effects are deterministic"}]
		contributesTo: [{capability: "capability.one_way_parameter_control", contribution: "is the single source of mutable control state"}]
	}

	domainServices: StateProjector: {
		purpose: "derive serialization, text, and fixed real-time parameters from one accepted AppState"
		uses: [
			"aggregate.Control.AppState",
			"valueObject.Control.StateSnapshot",
			"valueObject.Control.TextProjection",
			"valueObject.RealTime.ParameterSnapshot",
		]
		meta: rules: [
			"serialize AppState deterministically with serde_json and verify round-trip identity in tests",
			"derive TextProjection only from StateSnapshot plus the typed selection",
			"copy every audio parameter into a fixed-capacity ParameterSnapshot and reject startup if Patch count exceeds MAX_PATCHES",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "keeps text and audio projections consistent with serialized accepted state"},
			{capability: "capability.realtime_execution", contribution: "converts control-owned collections into bounded callback values"},
		]
	}

	applicationServices: AppLoop: {
		purpose: "apply one AppEvent and publish only effects derived from the accepted state"
		uses: [
			"aggregate.Control.AppState",
			"domainService.Control.StateProjector",
			"port.RealTime.AudioBoundary",
		]
		operations: {
			dispatch: {input: {event: "AppEvent"}, output: {result: "Result<DispatchResult, EventRejection>"}}
			currentText: {input: {}, output: {projection: "TextProjection"}}
		}
		meta: rules: [
			"dispatch order is reduce AppEvent through AppState.apply, commit accepted AppState, derive StateSnapshot/TextProjection/ParameterSnapshot, publish parameters, then enqueue any AudioCommand",
			"on rejection perform no serialization, publication, command enqueue, or view change",
			"views and input adapters can call only dispatch and currentText; they never receive mutable AppState",
		]
		validations: [{kind: "integration", command: ["cargo", "test", "one_way_control_loop"], description: "one edit changes one serialized value, publishes the matching snapshot, and changes rendered audio"}]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "orchestrates the complete reducer-to-projection-to-audio flow"},
			{capability: "capability.realtime_execution", contribution: "publishes through the two explicit real-time ports"},
		]
	}
}
