package crestsynth

project: contexts: Control: {
	purpose: "the one-way application state, semantic events, and projections"

	valueObjects: AppEvent: {
		description: "the closed semantic input union accepted by AppState"
		state: {
			kind: "Navigate | Adjust | InstallPatches | Midi"
			payload: "event-specific bounded payload"
			surfaceDescriptor: "typed exhaustive descriptors for every variant and Direction payload"
		}
		invariants: [
			"Navigate and Adjust carry Direction",
			"InstallPatches is accepted only during startup on the control thread",
			"Midi carries PatchId and MidiMessage",
			"raw key codes, window objects, clocks, files, and audio devices never appear in AppEvent",
			"surfaceDescriptor is produced beside the closed enum and is the only exhaustive event source consumed by DemoScene; adding or removing a variant cannot compile or pass schema equality without updating the descriptor",
			"surfaceDescriptor entries are unique before set conversion and enumerate Navigate and Adjust with all four Direction payloads plus InstallPatches and Midi with their complete typed payload shapes",
		]
		contributesTo: [{capability: "capability.one_way_parameter_control", contribution: "gives every input one semantic route into AppState"}]
	}

	valueObjects: EventRecord: {
		description: "one deterministic control-side record of an input event, its reducer outcome, emitted effects, and coherent projections"
		state: {
			sequence: "u64"
			source: "Startup | Keyboard | AutomaticMidi | DemoScene | System"
			input: "stable tagged AppEvent representation including Direction, PatchId, and MidiMessage payloads when present"
			outcome: "Accepted | Rejected"
			rejection: "Option<EventRejection>"
			generationBefore: "u64"
			generationAfter: "u64"
			stateHashBefore: "String"
			stateHashAfter: "String"
			emittedEvents: "Vec<stable tagged StateAccepted or audio-effect descriptors>"
			parameterGeneration: "u64"
			projectionStateHash: "String"
			selectedLine: "usize"
			serializedLeafDescriptor: "typed stable paths for every EventRecord field and tagged payload leaf"
		}
		invariants: [
			"sequence is contiguous and strictly increasing within one EventLog",
			"accepted records increment generation exactly once and their stateHashAfter, parameterGeneration, projectionStateHash, and emitted StateAccepted generation agree",
			"rejected records keep generation and state hash identical and emit no parameter publication or audio command",
			"records contain domain identifiers and bounded numeric payloads, never raw window objects, device handles, audio buffers, or nondeterministic timestamps",
			"serializedLeafDescriptor is compared against the union of discriminating accepted and rejected EventRecords covering every input tag, Direction payload, MIDI payload, Patch installation field, rejection, and emitted-event tag; discovery from one convenient record is invalid",
		]
		contributesTo: [{capability: "capability.observable_demo_scene", contribution: "makes every input, reducer decision, and emitted effect machine-readable for debugging"}]
	}

	valueObjects: EventLog: {
		description: "a deterministic LLM-readable journal of control events and their state transition chain"
		state: {
			schemaVersion: "u32"
			totalObserved: "u64"
			droppedRecords: "u64"
			records: "Vec<EventRecord>"
			coverage: "named expected, exercised, missing, and unexpected event/input/parameter/property identifiers"
			serializedLeafDescriptor: "typed stable paths for every EventLog and coverage field"
		}
		invariants: [
			"normal interactive execution uses a bounded control-thread ring and reports any eviction instead of hiding it",
			"the exhaustive demo pre-sizes the journal so droppedRecords is zero and retains every scene record",
			"the hash/generation after one record equals the hash/generation before the next record",
			"coverage is complete only when expected and exercised are exactly equal in both directions, so both missing and unexpected are empty",
			"serialization is deterministic JSON with stable field and enum names suitable for LLM inspection",
		]
		contributesTo: [{capability: "capability.observable_demo_scene", contribution: "provides the complete event history and explicit coverage gaps"}]
	}

	valueObjects: StateTree: {
		description: "a canonical LLM-readable tree of the complete accepted control state and its current GUI/audio projections"
		state: {
			schemaVersion: "u32"
			generation: "u64"
			patches: "Vec<{id, name, channel, instrument: {bank, program, percussion}, parameters: {gainDb, pan, reverbSend, delaySend}}>"
			global: "{masterGainDb, reverbRoomSize, reverbDamping, reverbReturn, delayMilliseconds, delayFeedback, delayReturn}"
			selection: "{section, patchIndex, parameterIndex}"
			projection: "{body, selectedLine, stateHash}"
			parameters: "{generation, patchCount, patches, global}"
			serializedLeafDescriptor: "typed stable paths derived beside the StateTree serializer"
		}
		invariants: [
			"the tree contains every currently serialized AppState, Patch, SoundFontInstrument, ChannelParameters, GlobalParameters, Selection, TextProjection, and ParameterSnapshot property without an opaque debug-string substitute",
			"patch order and numeric values exactly match StateSnapshot and ParameterSnapshot",
			"projection.stateHash equals the canonical StateSnapshot hash and parameters.generation equals generation",
			"serialization is deterministic JSON with a version field and stable property names",
			"serializedLeafDescriptor exactly equals recursively discovered JSON leaf paths in both directions for a discriminating multi-Patch tree",
		]
		contributesTo: [{capability: "capability.observable_demo_scene", contribution: "exposes the complete state/projection tree for LLM observation and diagnosis"}]
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
			serializedLeafDescriptor: "body | selectedLine | stateHash"
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
			"Adjust toward a boundary when the selected value is already at that boundary is rejected as ParameterAtBoundary and leaves state identical",
			"Adjust Left/Right is the fine decrement/increment and Adjust Down/Up is the coarse decrement/increment",
			"InstallPatches preserves fixture discovery order, rejects duplicate MidiChannels or more than 16 Patches, and is rejected after startup",
			"Midi does not mutate synth parameters; it yields one AudioCommand effect after state acceptance",
			"every accepted event increments generation once and every rejected event leaves state identical",
		]
		meta: rules: [
			"publish one typed EventRejection descriptor beside the enum with a unique stable name and reachability of Scene or ReducerTable",
			"classify InstallationClosed, TooManyPatches, DuplicateMidiChannel, UnknownPatch, and ParameterAtBoundary as externally constructible; classify NoPatchesInstalled, InvalidSelection, InvalidParameterValue, and GenerationOverflow as controlled reducer-table cases",
			"the exact descriptor set must equal the enum set before scene/table partitioning, and each variant is asserted by exactly one declared test path",
		]
		validations: [{kind: "test", command: ["cargo", "test", "app_state"], description: "navigation, K-modified adjustment, bounds, installation, and MIDI effects are deterministic"}]
		contributesTo: [{capability: "capability.one_way_parameter_control", contribution: "is the single source of mutable control state"}]
	}

	domainServices: StateProjector: {
		purpose: "derive serialization, text, fixed real-time parameters, and the canonical observation tree from one accepted AppState"
		uses: [
			"aggregate.Control.AppState",
			"valueObject.Control.StateSnapshot",
			"valueObject.Control.TextProjection",
			"valueObject.Control.StateTree",
			"valueObject.RealTime.ParameterSnapshot",
		]
		meta: rules: [
			"serialize AppState deterministically with serde_json and verify round-trip identity in tests",
			"derive TextProjection only from StateSnapshot plus the typed selection",
			"copy every audio parameter into a fixed-capacity ParameterSnapshot and reject startup if Patch count exceeds MAX_PATCHES",
			"derive StateTree from the exact same StateSnapshot, TextProjection, and ParameterSnapshot without reading or mutating any second state copy",
			"for a discriminating projection, compare every Patch identity/instrument/parameter value, every global value, the selection marker, selectedLine, and stateHash exactly against the accepted state; nonempty text or mere property presence is not sufficient",
			"publish production-owned typed surface/leaf descriptors beside their enum or serializer and require exact equality with recursively discovered EventLog, EventRecord, StateTree, TextProjection, and ParameterSnapshot paths",
		]
		validations: [
			{kind: "test", command: ["cargo", "test", "state_projector_exact_projection_values"], description: "every rendered Patch/global value, selection marker, selected line, and hash exactly matches one accepted state"},
			{kind: "test", command: ["cargo", "test", "schema_derived_current_surface"], description: "production-owned typed descriptors exactly equal discovered serialized leaves with no missing or unexpected paths"},
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "keeps text and audio projections consistent with serialized accepted state"},
			{capability: "capability.realtime_execution", contribution: "converts control-owned collections into bounded callback values"},
			{capability: "capability.observable_demo_scene", contribution: "builds the canonical complete tree from one coherent accepted generation"},
		]
	}

	applicationServices: AppLoop: {
		purpose: "apply one AppEvent and publish only effects derived from the accepted state"
		uses: [
			"aggregate.Control.AppState",
			"domainService.Control.StateProjector",
			"port.RealTime.AudioBoundary",
			"valueObject.Control.EventRecord",
			"valueObject.Control.EventLog",
			"valueObject.Control.StateTree",
		]
		operations: {
			dispatch: {input: {event: "AppEvent"}, output: {result: "Result<DispatchResult, EventRejection>"}}
			dispatchFrom: {input: {source: "EventSource", event: "AppEvent"}, output: {result: "Result<DispatchResult, EventRejection>"}}
			currentText: {input: {}, output: {projection: "TextProjection"}}
			currentStateTree: {input: {}, output: {state: "StateTree"}}
			eventLog: {input: {}, output: {events: "EventLog"}}
		}
		meta: rules: [
			"dispatch order is reduce AppEvent through AppState.apply, commit accepted AppState, derive StateSnapshot/TextProjection/ParameterSnapshot, publish parameters, then enqueue any AudioCommand",
			"dispatch preserves the existing API and delegates to dispatchFrom with a stable default source; production adapters and demo inputs call dispatchFrom with their exact source",
			"record every accepted and rejected input exactly once on the control thread; observation occurs after the outcome is known and never runs in the audio callback",
			"on rejection perform no state serialization, parameter publication, command enqueue, or view change, but append one rejected EventRecord proving generation and state hash were unchanged",
			"EventRejection is a nonfatal domain result for an input event; callers may report or ignore it but must remain able to dispatch later events",
			"accepted EventRecords include the input, StateAccepted generation, snapshot hashes, parameter publication, projection identity, and any AudioCommand descriptor",
			"currentStateTree is derived from the same accepted state and projections without exposing mutable AppState",
			"views and input adapters can call only dispatch and immutable currentText/currentStateTree/eventLog reads; they never receive mutable AppState",
		]
		validations: [
			{kind: "integration", command: ["cargo", "test", "one_way_control_loop"], description: "an edit on a non-first Patch changes only its serialized value and audio contribution, and a boundary no-op leaves the loop running for the next event"},
			{kind: "test", command: ["cargo", "test", "control_observation_trace"], description: "accepted and rejected event records form an exact hash/generation chain and the state tree contains every current property"},
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "orchestrates the complete reducer-to-projection-to-audio flow"},
			{capability: "capability.realtime_execution", contribution: "publishes through the two explicit real-time ports"},
			{capability: "capability.observable_demo_scene", contribution: "records the production reducer's complete event/state/effect trace"},
		]
	}
}
