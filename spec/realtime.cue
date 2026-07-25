package crestsynth

project: contexts: RealTime: {
	purpose: "fixed-capacity values and lock-free ports crossing into the audio callback"

	valueObjects: GraphRevision: {
		description: "a monotonic numeric identity for one completely prepared structural audio graph"
		from: "u64"
		invariants: [
			"zero denotes no published graph and every prepared replacement uses a greater revision",
			"the value is Copy, fixed-size, and contains no state owner or destructor",
		]
		contributesTo: [{capability: "capability.prepared_engine_rack", contribution: "correlates publication, block-boundary activation, parameter compatibility, acknowledgement, and retirement"}]
	}

	valueObjects: ParameterSnapshot: {
		description: "the newest complete control state required for rendering"
		state: {
			generation: "u64"
			graphRevision: "GraphRevision"
			patchCount: "usize"
			patches: "[RtPatchParameters; MAX_PATCHES]"
			global: "GlobalParameters"
		}
		invariants: [
			"MAX_PATCHES equals the bounded prepared rack and current sixteen-channel fixture capacity",
			"graphRevision identifies the PreparedGraph whose exact PatchId order and fixed capacities this snapshot targets",
			"unused entries are inactive",
			"the snapshot is fully owned, fixed-size, and readable without allocation",
			"a production-owned typed leaf descriptor covers generation, graphRevision, patchCount, every active PatchId/channel parameter, and every global parameter and exactly matches the StateTree parameters projection",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "carries accepted AppState values to audio"},
			{capability: "capability.realtime_execution", contribution: "provides a fixed-size latest-wins callback input"},
		]
	}

	aggregates: PreparedEngineRack: {
		root: true
		purpose: "own a fixed-capacity ordered set of capability-neutral prepared instruments for callback dispatch and rendering"
		state: {
			patchCount: "usize"
			slots: "[Option<{patchId: PatchId, instrument: Box<dyn PreparedInstrument>}>; MAX_PATCHES]"
		}
		invariants: [
			"the rack is constructed outside the callback and every active slot contains one unique PatchId plus exactly one fully prepared instrument",
			"slot order and PatchIds exactly match the PreparedGraph initial ParameterSnapshot and PatchAudioBlock stems",
			"dispatch resolves one PatchId through bounded storage and calls only that slot; unknown PatchId returns fixed-size status without fallback or broadcast",
			"render clears caller-owned stems and calls each active instrument once per block into only its matching stem",
			"all-notes-off visits at most MAX_PATCHES prepared instruments",
			"the rack never allocates, grows, reorders, locks, blocks, performs I/O, logs, formats, panics, unwinds, or destroys an instrument in callback operations",
			"heterogeneous trait objects are allowed across slots, but dynamic dispatch never occurs inside an instrument's inner sample loop",
		]
		validations: [{kind: "test", command: ["cargo", "test", "prepared_engine_rack"], description: "two distinct prepared test instrument implementations route targeted MIDI and render isolated bounded stems through one rack"}]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "is the capability-neutral bounded runtime owner later used by SoundFont and Braids together"},
			{capability: "capability.realtime_execution", contribution: "bounds polymorphic dispatch and rendering outside inner sample loops"},
		]
	}

	aggregates: PreparedGraph: {
		root: true
		purpose: "own one complete callback-ready engine, mixer, effect, routing, stem, and scratch configuration"
		state: {
			revision: "GraphRevision"
			sampleRate: "f32"
			maxFrames: "usize"
			initialParameters: "ParameterSnapshot"
			engineRack: "PreparedEngineRack"
			patchAudio: "PatchAudioBlock"
			mixer: "MixEngine<GlobalReverbDelay>"
		}
		invariants: [
			"all owned engines, parsed assets, voices, effect memory, stems, routing, and scratch capacity are fully prepared outside the callback",
			"revision is nonzero and equals initialParameters.graphRevision",
			"the rack, parameter snapshot, stems, and mixer routing contain the same PatchIds in the same bounded order",
			"sampleRate and maxFrames are validated once and every callback buffer is bounded by maxFrames",
			"the current increment permits replacement only for the same accepted PatchId set and does not expose a structural edit event or engine selector",
			"moving graph ownership through a queue performs no allocation or destruction; destruction is permitted only after the retired graph reaches control or worker ownership",
		]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "makes structural audio state one complete ownership-transfer unit"},
			{capability: "capability.global_mix", contribution: "keeps the existing reverb, delay, routing, and mixer prepared with the engine rack"},
			{capability: "capability.realtime_execution", contribution: "prevents partially prepared topology from reaching the callback"},
		]
	}

	valueObjects: GraphHandoffStatus: {
		description: "fixed-size control-readable acknowledgement for structural publication and retirement"
		state: {
			activeRevision: "GraphRevision"
			retiredRevision: "GraphRevision"
			swapsApplied: "u64"
			retirementRetries: "u64"
			incompatibleSnapshots: "u64"
		}
		invariants: [
			"the value is Copy, numeric, fixed-size, and published through atomics or an equivalent coherent latest-value transport",
			"activeRevision advances only after a complete graph swap at a block boundary",
			"retiredRevision advances only after ownership of the replaced graph has entered the audio-to-control return queue",
			"the callback reports swap, retirement-pressure, and incompatible-snapshot counters without logging, formatting, allocation, or backpressure",
		]
		contributesTo: [{capability: "capability.prepared_engine_rack", contribution: "lets control throttle structural work until the prior graph is active and safely returned"}]
	}

	valueObjects: PatchAudioBlock: {
		description: "caller-owned prepared stereo stems that preserve Patch identity between the prepared instrument rack and mixing"
		state: {
			patchCount: "usize"
			frameCount: "usize"
			stems: "[PatchStereoStem; MAX_PATCHES]"
		}
		invariants: [
			"capacity for MAX_PATCHES and maxFrames is allocated only while building a PreparedGraph outside the callback",
			"each active stem is keyed by the same PatchId and index as ParameterSnapshot.patches",
			"one stem contains only audio produced by that Patch's assigned PreparedInstrument slot",
			"clearing, filling, and reading active frames are allocation-free",
		]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "preserves one independently routable stem for each capability-neutral rack slot"},
			{capability: "capability.soundfont_audio", contribution: "preserves Patch identity after synthesis instead of collapsing all voices to one master stream"},
			{capability: "capability.global_mix", contribution: "gives MixEngine an independently controllable signal for every Patch"},
			{capability: "capability.realtime_execution", contribution: "provides prepared callback-owned synthesis scratch storage"},
		]
	}

	valueObjects: AudioCommand: {
		description: "a bounded discrete command for the audio callback"
		state: {
			kind: "PatchMidi | AllNotesOff"
			patchId: "Option<PatchId>"
			message: "Option<MidiMessage>"
		}
		invariants: ["contains no heap-owned data", "PatchMidi contains both patchId and message"]
		meta: rules: [
			"PatchMidi carrying MidiMessageKind::AllNotesOff and the separate AudioCommand::AllNotesOff variant are distinct typed cases and both are asserted; coverage for one never credits the other",
		]
		contributesTo: [
			{capability: "capability.soundfont_audio", contribution: "targets MIDI at the configured Patch"},
			{capability: "capability.realtime_execution", contribution: "is safe to copy through the event ring"},
		]
	}

	valueObjects: AudioObservationSnapshot: {
		description: "a fixed-size latest-value observation of audio work completed by the real-time callback"
		state: {
			sequence: "u64"
			renderedBlocks: "u64"
			renderedFrames: "u64"
			parameterGeneration: "u64"
			commandsConsumed: "u64"
			activeNotes: "u32"
			leftPeak: "f32"
			rightPeak: "f32"
			outputRms: "f32"
			reverbInputRms: "f32"
			delayInputRms: "f32"
			wetOutputRms: "f32"
			nonFiniteSamples: "u64"
			clippedSamples: "u64"
		}
		invariants: [
			"the snapshot is Copy, fixed-size, numeric, and contains no Vec, String, path, reference, mutex, decoder, allocation, or destructible owner",
			"sequence and renderedBlocks increase monotonically and parameterGeneration is the exact ParameterSnapshot generation used for the measured block",
			"peak, RMS, wet-input, and wet-output fields copy the MixObservation produced from the actual mixer-owned buffers for that observation window",
			"activeNotes is maintained by a prepared fixed-capacity Patch/channel/note bitset updated only when the callback dispatches the corresponding MIDI lifecycle command; Patch-targeted or global all-notes-off clears it with bounded work",
			"the callback updates nonFiniteSamples and clippedSamples instead of logging, formatting, panicking, or performing I/O",
		]
		contributesTo: [
			{capability: "capability.live_observable_demo", contribution: "correlates visible accepted generations with measured physical audio work"},
			{capability: "capability.realtime_execution", contribution: "keeps callback diagnostics bounded and allocation-free"},
		]
	}

	ports: AudioBoundary: {
		direction: "outbound"
		contract: {
			pushCommand: "(AudioCommand) -> Result<(), BoundaryFull>"
			publishParameters: "(ParameterSnapshot)"
			popCommand: "() -> Option<AudioCommand>"
			readLatestParameters: "() -> ParameterSnapshot"
		}
		consumes: ["valueObject.RealTime.AudioCommand", "valueObject.RealTime.ParameterSnapshot"]
		invariants: [
			"commands use a bounded single-producer single-consumer queue",
			"parameters are latest-wins and the consumer sees one complete snapshot",
			"all operations used by the audio thread are allocation-free, lock-free, and non-blocking",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "publishes accepted parameters through the single real-time seam"},
			{capability: "capability.realtime_execution", contribution: "owns only the discrete-command and latest-scalar transfer semantics"},
		]
	}

	ports: StructuralGraphBoundary: {
		direction: "outbound"
		contract: {
			publishPreparedOnControl: "(PreparedGraph) -> Result<(), StructuralBoundaryFull>"
			takePreparedOnAudio: "() -> Option<PreparedGraph>"
			returnRetiredOnAudio: "(PreparedGraph) -> Result<(), RetiredBoundaryFull>"
			collectRetiredOnControl: "()"
			readStatusOnControl: "() -> GraphHandoffStatus"
			publishStatusOnAudio: "(GraphHandoffStatus)"
		}
		consumes: [
			"aggregate.RealTime.PreparedGraph",
			"valueObject.RealTime.GraphHandoffStatus",
		]
		invariants: [
			"prepared control-to-audio ownership and retired audio-to-control ownership use distinct preallocated bounded SPSC queues and never share AudioCommand or ParameterSnapshot storage",
			"a full publish queue returns the untouched prepared graph to control and a full retirement queue returns the untouched retired graph to the callback",
			"the callback can move graph ownership and publish status without allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, unwinding, or destruction",
			"collectRetiredOnControl is the only runtime operation that destroys a replaced graph and never runs on the audio callback",
			"control publishes at most one replacement and does not publish another until status reports the prior revision retired",
		]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "transfers complete prepared graph ownership and returns replaced ownership without callback destruction"},
			{capability: "capability.realtime_execution", contribution: "keeps structural state separate from commands and latest scalar snapshots"},
		]
	}

	ports: AudioObservation: {
		direction: "outbound"
		contract: {
			publishFromCallback: "(AudioObservationSnapshot)"
			readLatestOnControl: "() -> AudioObservationSnapshot"
		}
		consumes: ["valueObject.RealTime.AudioObservationSnapshot"]
		invariants: [
			"publishFromCallback is bounded, lock-free, non-blocking, allocation-free, and cannot log or destroy owned state",
			"readLatestOnControl returns one coherent complete snapshot and never executes on the audio callback",
			"observation delivery is a dedicated latest-value transport and never shares the discrete AudioCommand ring or ParameterSnapshot publication storage",
			"a slow UI may skip intermediate observations; it cannot slow or backpressure the callback",
		]
		contributesTo: [
			{capability: "capability.live_observable_demo", contribution: "lets the control-side live runner read bounded audio consequences without callback logging"},
			{capability: "capability.realtime_execution", contribution: "separates callback-to-control observations from command and parameter transports"},
		]
	}

	applicationServices: PreparedGraphBuilder: {
		purpose: "prepare one complete graph from a bounded engine rack and current mixer topology before publication"
		uses: [
			"applicationService.Synth.PreparedEngineRackBuilder",
			"aggregate.RealTime.PreparedGraph",
			"valueObject.RealTime.ParameterSnapshot",
			"domainService.Mixer.MixEngine",
			"adapter.GlobalReverbDelay",
		]
		operations: {
			build: {input: {revision: "GraphRevision", patches: "&[Patch]", parameters: "ParameterSnapshot", sampleRate: "f32", maxFrames: "usize"}, output: {result: "Result<PreparedGraph, GraphPreparationError>"}}
		}
		meta: rules: [
			"run only on control or worker ownership and finish engine, asset, voice, effect, routing, stem, and scratch preparation before returning",
			"require the supplied parameter graphRevision, PatchIds, order, and count to match the prepared rack exactly",
			"fail atomically with a typed error on any capacity, format, capability, asset, engine, effect, routing, or allocation failure and never return a partial graph",
			"use the existing one global reverb and one global delay topology without introducing Patch effects, arbitrary graph edges, or feedback cycles",
		]
		validations: [{kind: "test", command: ["cargo", "test", "prepared_graph_builder"], description: "complete compatible graphs prepare deterministically while every partial, mismatched, unsupported, or over-capacity input fails before publication"}]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "turns accepted structural state into one complete callback-ready ownership unit"},
			{capability: "capability.realtime_execution", contribution: "keeps every allocating preparation step outside the callback"},
		]
	}

	applicationServices: StructuralGraphCoordinator: {
		purpose: "publish at most one prepared replacement and collect its predecessor after callback acknowledgement"
		uses: [
			"port.RealTime.StructuralGraphBoundary",
			"aggregate.RealTime.PreparedGraph",
			"valueObject.RealTime.GraphHandoffStatus",
		]
		operations: {
			submit: {input: {graph: "PreparedGraph"}, output: {result: "Result<(), StructuralGraphBusy>"}}
			poll: {input: {}, output: {status: "GraphHandoffStatus"}}
		}
		meta: rules: [
			"run only outside the audio callback and preserve ownership of a graph rejected by queue pressure",
			"allow exactly one revision in flight and reject or defer another submission until the previous active revision is also acknowledged retired and collected",
			"collect returned graphs on control or worker ownership where destructors are allowed",
			"never mutate AppState, fabricate an acknowledgement, publish a partial graph, or substitute another graph after failure",
			"this increment exercises handoff through deterministic orchestration but exposes no user structural edit or engine-selection event",
		]
		validations: [{kind: "test", command: ["cargo", "test", "structural_graph_coordinator"], description: "one-in-flight throttling, ownership preservation, acknowledgement, retry, and explicit control-side collection are exact"}]
		contributesTo: [{capability: "capability.prepared_engine_rack", contribution: "enforces the one-in-flight graph replacement protocol on the non-real-time side"}]
	}

	applicationServices: AudioRenderer: {
		purpose: "swap complete prepared graphs at block boundaries, consume ready commands and compatible parameters, render the active engine rack, then mix its prepared global effects"
		uses: [
			"port.RealTime.AudioBoundary",
			"port.RealTime.StructuralGraphBoundary",
			"port.RealTime.AudioObservation",
			"aggregate.RealTime.PreparedGraph",
			"aggregate.RealTime.PreparedEngineRack",
			"valueObject.Mixer.MixObservation",
		]
		operations: {
			fromPrepared: {input: {initialGraph: "PreparedGraph"}, output: {result: "Result<AudioRenderer, AudioError>"}}
			render: {input: {interleavedStereo: "&mut [f32]"}, output: {}}
		}
		meta: rules: [
			"construction receives one completely prepared initial graph before the callback starts and never prepares an engine, mixer, effect, route, or buffer itself",
			"at the start of a render block, if no prior retired graph occupies the bounded callback retirement slot, take at most one prepared replacement, swap the complete graph, activate its initial parameters, and publish the active revision",
			"move the replaced graph into the dedicated return queue; if that queue is full, retain it in the one preallocated callback retirement slot and retry on later blocks without taking another replacement or destroying it",
			"publish retiredRevision only after the return queue owns the old graph; control does not submit another structure until that acknowledgement is observed and collected",
			"render drains only currently available AudioCommands, reads one latest ParameterSnapshot compatible with the active graph revision and PatchIds, asks PreparedEngineRack to fill one PatchAudioBlock stem per active Patch, passes all matching stems and parameters to the active graph's MixEngine, and returns",
			"PatchId and Patch index remain aligned from AudioCommand through the synthesis stem and ChannelParameters; a combined engine master buffer must never be treated as one Patch's input",
			"after rendering each block, combine MixEngine's MixObservation with bounded command and active-note counters and publish one AudioObservationSnapshot tagged with the consumed ParameterSnapshot generation",
			"the active-note observer is prepared outside the callback, has explicit Patch, channel, and note bounds, saturates counters on overflow, and never controls or substitutes prepared instrument state",
			"audio observations never change rendering, synth state, mix state, event coverage, or acceptance results and never call a control-side serializer or logger",
			"render never allocates, deallocates, locks, blocks, performs I/O, logs, formats strings, grows a collection, panics, unwinds, or destroys owned state",
		]
		validations: [
			{kind: "test", command: ["cargo", "test", "audio_renderer_realtime_contract"], description: "an instrumented callback consumes commands and latest parameters through a heterogeneous prepared rack with zero callback allocations or destruction and preserves simultaneous Patch stems into the mixer"},
			{kind: "test", command: ["cargo", "test", "prepared_graph_handoff"], description: "complete graphs swap only at block boundaries, acknowledge active and retired revisions, retain on return pressure, and drop only during control collection"},
			{kind: "test", command: ["cargo", "test", "audio_observation_realtime_contract"], description: "the callback publishes coherent generation-tagged numeric observations with zero allocation, locking, blocking, logging, or callback-owned destruction"},
		]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "owns block-boundary graph activation and bounded retirement retry"},
			{capability: "capability.soundfont_audio", contribution: "joins the SoundFont and global mixer into the callback"},
			{capability: "capability.live_observable_demo", contribution: "publishes measured callback consequences for the live scene without changing the render path"},
			{capability: "capability.realtime_execution", contribution: "owns the hard real-time render operation"},
		]
	}
}

project: adapters: LockFreeAudioBoundary: {
	implements: "port.RealTime.AudioBoundary"
	layer: "infrastructure"
	meta: {
		framework: "rtrb + triple_buffer"
		rules: [
			"use one rtrb queue for AudioCommand and one triple_buffer for ParameterSnapshot; structural graph ownership is not carried by either transport",
			"keep the control and audio handles separate so callback code cannot call control-only operations",
		]
	}
	contributesTo: [
		{capability: "capability.one_way_parameter_control", contribution: "publishes the latest accepted parameters"},
		{capability: "capability.realtime_execution", contribution: "implements the complete lock-free control/audio boundary"},
	]
}

project: adapters: LockFreeStructuralGraphBoundary: {
	implements: "port.RealTime.StructuralGraphBoundary"
	layer: "infrastructure"
	meta: {
		framework: "rtrb + atomics"
		rules: [
			"preallocate a control-to-audio PreparedGraph ownership queue and a distinct audio-to-control retired PreparedGraph ownership queue before audio starts",
			"use coherent fixed-size atomics for GraphHandoffStatus and never serialize, allocate, block, or log from the callback handle",
			"return ownership intact on queue pressure; never drop a PreparedGraph in push error handling on the callback",
			"keep narrow control and audio handles so only control can publish, collect, and destroy and only audio can take, return, and acknowledge",
			"complete graph ownership return is the only retirement path for replaced engine and effect state",
		]
	}
	validations: [{kind: "test", command: ["cargo", "test", "lock_free_structural_graph_boundary"], description: "both directions are bounded FIFO ownership transfer, pressure preserves values, status is coherent, and destructors run only under explicit control collection"}]
	contributesTo: [
		{capability: "capability.prepared_engine_rack", contribution: "implements the dedicated prepared/retired graph handoff and acknowledgement path"},
		{capability: "capability.realtime_execution", contribution: "makes structural ownership transfer distinct from discrete commands and scalar snapshots"},
	]
}

project: adapters: AtomicAudioObservation: {
	implements: "port.RealTime.AudioObservation"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "latest-value-atomics"}
	meta: {
		rules: [
			"store all floating measurements as their f32 bit patterns and use a sequence protocol or equivalent bounded atomic publication so the control side never combines fields from different snapshots",
			"the audio-side handle can only publish and the control-side handle can only read; neither handle exposes serialization, formatting, reset, wait, or blocking APIs",
			"publishing overwrites the previous observation without allocation or backpressure and never touches AudioCommand or ParameterSnapshot storage",
		]
	}
	validations: [{kind: "test", command: ["cargo", "test", "atomic_audio_observation"], description: "publication and reads are coherent, latest-wins, monotonic, and allocation-free on the callback side"}]
	contributesTo: [
		{capability: "capability.live_observable_demo", contribution: "implements the bounded callback-to-control observation seam used by live checkpoints"},
		{capability: "capability.realtime_execution", contribution: "keeps meters and health data out of the event and parameter transports"},
	]
}
