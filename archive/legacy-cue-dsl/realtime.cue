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
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "correlates publication, block-boundary activation, parameter compatibility, acknowledgement, and retirement"},
			{capability: "capability.asynchronous_engine_selection", contribution: "joins one request's source and target graphs without carrying ownership"},
		]
	}

	valueObjects: ParameterSnapshot: {
		description: "the newest complete control state required for rendering"
		state: {
			generation: "u64"
			graphRevision: "GraphRevision"
			patchCount: "usize"
			patches: "[RtPatchParameters; MAX_PATCHES]"
			postEffects: "[RtPostEffectParameters; MAX_PATCHES]"
			tracks: "[MixerTrackParameters; 16]"
			global: "GlobalParameters"
		}
		invariants: [
			"MAX_PATCHES equals the bounded prepared rack capacity and is independent from the exactly sixteen persistent mixer tracks",
			"graphRevision identifies the PreparedGraph whose exact PatchId order and fixed capacities this snapshot targets",
			"unused entries are inactive",
			"the snapshot is fully owned, fixed-size, and readable without allocation",
			"each RtPatchParameters contains PatchId, fixed PatchOutput, VoiceEnvelope, scalarCount, and a descriptor-ordered [f32; 16] engine-scalar array fixed by the active graph revision",
			"postEffects is a separate Patch-aligned zero-or-one-slot section whose active entry carries stable EffectSlotId, scalarCount, and descriptor-ordered [f32; 8] effect values fixed by the active graph revision",
			"tracks contains exactly sixteen fixed MixerTrackParameters entries in MixerTrackId order whether or not a Patch targets them",
			"choice values are encoded as descriptor indices and snapshots contain no string, vector, asset, capability union, engine object, or destructor-bearing owner",
			"a production-owned typed leaf descriptor covers generation, graphRevision, patchCount, every active PatchId/output/envelope/scalar parameter, all sixteen track parameter sets, and every global parameter and exactly matches the StateTree parameters projection",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "carries accepted AppState values to audio"},
			{capability: "capability.asynchronous_engine_selection", contribution: "carries the exact descriptor scalar layout for the source or committed target graph revision"},
			{capability: "capability.static_patch_effect", contribution: "carries the latest complete effect scalars without sharing instrument layout storage"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "carries Patch routes/trims and the complete sixteen-track bank through fixed latest-value storage"},
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
			"dispatch resolves one PatchId through bounded storage and passes only that slot's compatible RtPatchParameters; unknown PatchId or layout mismatch returns fixed-size status without fallback or broadcast",
			"render clears caller-owned stems and calls each active instrument once per block with only its matching RtPatchParameters into only its matching stem",
			"all-notes-off visits at most MAX_PATCHES prepared instruments",
			"the rack never allocates, grows, reorders, locks, blocks, performs I/O, logs, formats, panics, unwinds, or destroys an instrument in callback operations",
			"heterogeneous trait objects are allowed across slots, but dynamic dispatch never occurs inside an instrument's inner sample loop",
		]
		validations: [{id: "validation.aggregate.prepared_engine_rack", kind: "test", command: ["cargo", "test", "prepared_engine_rack"], description: "two distinct prepared test instrument implementations route targeted MIDI and render isolated bounded stems through one rack"}]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "is the capability-neutral bounded runtime owner later used by SoundFont and Braids together"},
			{capability: "capability.asynchronous_engine_selection", contribution: "replaces one selected slot implementation without branching on capability identity"},
			{capability: "capability.realtime_execution", contribution: "bounds polymorphic dispatch and rendering outside inner sample loops"},
		]
	}

	aggregates: PreparedGraph: {
		root: true
		purpose: "own one complete callback-ready engine, ordered Patch effect, mixer/global-effect, routing, stem, and scratch configuration"
		state: {
			revision: "GraphRevision"
			sampleRate: "f32"
			maxFrames: "usize"
			initialParameters: "ParameterSnapshot"
			engineRack: "PreparedEngineRack"
			patchAudio: "PatchAudioBlock"
			postEffectRack: "PreparedPostEffectRack"
			mixer: "MixEngine<GlobalReverbDelay>"
		}
		invariants: [
			"all owned engines, parsed assets, voices, effect memory, stems, routing, and scratch capacity are fully prepared outside the callback",
			"revision is nonzero and equals initialParameters.graphRevision",
			"the engine rack, post-effect rack, parameter snapshot, instrument/effect scalar layouts, stems, and mixer routing contain the same PatchIds in the same bounded order",
			"sampleRate and maxFrames come from the accepted negotiated AudioDeviceConfig and are validated once before the device stream starts",
			"every graph render block is bounded by maxFrames; a larger native device callback is completely rendered as consecutive bounded blocks without truncation or a silent tail",
			"a replacement preserves the accepted PatchId set, order, rack capacity, every PatchOutput, the complete MixerState, ordered PostEffectConfigs, effect scalar layouts, and device bounds while permitting exactly the selected instrument config delta named by StructuralEditIntent—capability/default layout or one descriptor-owned structural choice—to change to its validated candidate config",
			"initialParameters is refreshed from the exact committed EnginePrepared generation immediately before publication so scalar edits accepted during preparation cannot be reverted by the swap",
			"moving graph ownership through a queue performs no allocation or destruction; destruction is permitted only after the retired graph reaches control or worker ownership",
		]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "makes structural audio state one complete ownership-transfer unit"},
			{capability: "capability.asynchronous_engine_selection", contribution: "carries one fully prepared target engine and its committed scalar layout as an atomic replacement"},
			{capability: "capability.global_mix", contribution: "keeps the current reverb, delay, and sixteen-track mixer prepared with the engine rack"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "owns all sixteen preallocated destinations and scratch buffers before callback activation"},
			{capability: "capability.static_patch_effect", contribution: "owns the prepared ordered Patch-local processing stage independently from the mixer-owned global effects"},
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
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "lets control throttle structural work until the prior graph is active and safely returned"},
			{capability: "capability.asynchronous_engine_selection", contribution: "supplies the fixed activation and retirement facts required before Ready"},
		]
	}

	valueObjects: PatchAudioBlock: {
		description: "caller-owned prepared stereo stems that preserve Patch identity from instrument rendering through ordered Patch effects into mixing"
		state: {
			patchCount: "usize"
			frameCount: "usize"
			stems: "[PatchStereoStem; MAX_PATCHES]"
		}
		invariants: [
			"capacity for MAX_PATCHES and maxFrames is allocated only while building a PreparedGraph outside the callback",
			"each active stem is keyed by the same PatchId and index as ParameterSnapshot.patches",
			"one stem contains only audio produced by that Patch's assigned PreparedInstrument slot",
			"a configured PreparedPostEffectRack slot may mutate only that same stem in place before MixEngine reads it",
			"clearing, filling, and reading active frames are allocation-free",
		]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "preserves one independently routable stem for each capability-neutral rack slot"},
			{capability: "capability.soundfont_audio", contribution: "preserves Patch identity after synthesis instead of collapsing all voices to one master stream"},
			{capability: "capability.global_mix", contribution: "gives MixEngine one identity-preserving source contribution to route into a track"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "preserves Patch identity until trim and track accumulation"},
			{capability: "capability.static_patch_effect", contribution: "is the exact identity-preserving carrier processed between engine and mixer"},
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
			routingFailures: "u64"
			lastUnknownPatchId: "Option<PatchId>"
			tracks: "[TrackMeter; 16]"
			leftPeak: "f32"
			rightPeak: "f32"
			outputRms: "f32"
			reverbInputRms: "f32"
			delayInputRms: "f32"
			wetOutputRms: "f32"
			patchEffect: "PatchEffectObservation"
			nonFiniteSamples: "u64"
			clippedSamples: "u64"
		}
		invariants: [
			"the snapshot is Copy, fixed-size, numeric, and contains no Vec, String, path, reference, mutex, decoder, allocation, or destructible owner",
			"sequence and renderedBlocks increase monotonically and parameterGeneration is the exact ParameterSnapshot generation used for the measured block",
			"tracks plus peak, RMS, wet-input, and wet-output fields copy the MixObservation produced from the actual mixer-owned buffers for that observation window",
			"tracks contains exactly one post-level/pan, pre-gate numeric meter per MixerTrackId, including zero-valued empty tracks and sounding muted tracks",
			"patchEffect copies fixed pre/post/difference/side measurements from the exact configured Patch stem around PreparedPostEffectRack processing",
			"activeNotes is maintained by a prepared fixed-capacity Patch/channel/note bitset updated only when the callback dispatches the corresponding MIDI lifecycle command; Patch-targeted or global all-notes-off clears it with bounded work",
			"routingFailures increments exactly once for each PatchMidi command whose PatchId is absent from either the compatible parameter projection or prepared rack, and lastUnknownPatchId retains that exact identity without fallback or broadcast",
			"the callback updates nonFiniteSamples and clippedSamples instead of logging, formatting, panicking, or performing I/O",
		]
		contributesTo: [
			{capability: "capability.live_observable_demo", contribution: "correlates visible accepted generations with measured physical audio work"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "publishes the fixed complete track-meter bank without UI-owned measurement state"},
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
			"publication pressure returns complete ownership to control for staged retry; it never rolls back committed state, drops a graph, or selects a substitute",
		]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "transfers complete prepared graph ownership and returns replaced ownership without callback destruction"},
			{capability: "capability.asynchronous_engine_selection", contribution: "carries the correlated candidate only after its control config commits"},
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
		purpose: "prepare one complete graph from bounded engine/effect racks and current mixer topology before publication"
		uses: [
			"applicationService.Synth.PreparedEngineRackBuilder",
			"applicationService.Synth.PreparedPostEffectRackBuilder",
			"aggregate.RealTime.PreparedGraph",
			"valueObject.RealTime.ParameterSnapshot",
			"domainService.Mixer.MixEngine",
			"adapter.GlobalReverbDelay",
		]
		operations: {
			build: {input: {revision: "GraphRevision", patches: "&[Patch]", parameters: "ParameterSnapshot", sampleRate: "f32", maxFrames: "usize"}, output: {result: "Result<PreparedGraph, GraphPreparationError>"}}
		}
		meta: rules: [
			"run only on control or worker ownership and finish engine, asset, voice, Patch effect, global effect, routing, stem, and scratch preparation before returning",
			"require the supplied parameter graphRevision, PatchIds, order, count, stable effect slots, and instrument/effect scalar layouts to match both prepared racks exactly",
			"fail atomically with a typed error on any capacity, format, capability, asset, engine, effect, routing, or allocation failure and never return a partial graph",
			"construct the fixed PreparedEngineRack to PatchAudioBlock to PreparedPostEffectRack to MixEngine topology; MixEngine retains exactly one global reverb and delay and no arbitrary graph edge or feedback cycle exists",
			"for structural editing preserve PatchIds, order, count, every PatchOutput, the complete MixerState, envelopes, ordered PostEffectConfigs, effect scalar layouts, device bounds, and untargeted instrument configs exactly while allowing only the selected Patch instrument candidate and resulting instrument scalar layout to differ according to StructuralEditIntent",
		]
		validations: [{id: "validation.service.prepared_graph_builder", kind: "test", command: ["cargo", "test", "prepared_graph_builder"], description: "complete compatible graphs prepare deterministically while every partial, mismatched, unsupported, or over-capacity input fails before publication"}]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "turns accepted structural state into one complete callback-ready ownership unit"},
			{capability: "capability.asynchronous_engine_selection", contribution: "builds the exact complete candidate consumed by the correlated worker result"},
			{capability: "capability.static_patch_effect", contribution: "prepares the fixed ordered Patch effect stage and preserves it across instrument rebuilds"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "prepares all track scratch and validates the complete fixed routing snapshot"},
			{capability: "capability.realtime_execution", contribution: "keeps every allocating preparation step outside the callback"},
		]
	}

	applicationServices: StructuralGraphCoordinator: {
		purpose: "stage or publish at most one prepared replacement, preserve its layout correlation, and collect its predecessor after callback acknowledgement"
		uses: [
			"port.RealTime.StructuralGraphBoundary",
			"aggregate.RealTime.PreparedGraph",
			"valueObject.RealTime.GraphHandoffStatus",
			"valueObject.Control.EngineSelectionRequestId",
		]
		operations: {
			submit: {input: {requestId: "EngineSelectionRequestId", sourceRevision: "GraphRevision", graph: "PreparedGraph"}, output: {result: "Result<Published | Staged, StructuralGraphBusy>"}}
			retryStaged: {input: {}, output: {result: "Result<Option<GraphRevision>, StructuralBoundaryError>"}}
			poll: {input: {}, output: {status: "GraphHandoffStatus"}}
		}
		meta: rules: [
			"run only outside the audio callback and preserve ownership of a graph rejected by queue pressure",
			"allow exactly one correlated revision in flight and reject another submission until the previous target is active, its source is acknowledged retired, and the returned graph is collected",
			"if publication pressure returns the graph, retain exactly one staged complete graph on control ownership and retry it before polling another worker result; never rollback committed AppState or drop, rebuild, or substitute the graph",
			"a candidate may change only the selected Patch InstrumentConfig delta and resulting instrument scalar layout named by StructuralEditIntent while keeping PatchIds, order, capacities, every PatchOutput, the complete MixerState, ordered PostEffectConfigs, effect scalar layouts, and device configuration exact; retain the source layout until acknowledgement and adopt the target layout only after collection",
			"collect returned graphs on control or worker ownership where destructors are allowed",
			"never mutate AppState, fabricate an acknowledgement, publish a partial graph, or substitute another graph after failure",
			"correlate request id, StructuralEditIntent, and source/target revisions with EngineSelectionStatus but never mutate AppState or store the status itself",
		]
		validations: [{id: "validation.service.structural_graph_coordinator", kind: "test", command: ["cargo", "test", "structural_graph_coordinator"], description: "one-in-flight throttling, ownership preservation, acknowledgement, retry, and explicit control-side collection are exact"}]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "enforces the one-in-flight graph replacement protocol on the non-real-time side"},
			{capability: "capability.asynchronous_engine_selection", contribution: "owns staged retry and layout adoption without becoming a second state reducer"},
		]
	}

	applicationServices: AudioRenderer: {
		purpose: "swap complete prepared graphs at block boundaries, consume ready commands and compatible parameters, render instruments, process ordered Patch effects, route them through sixteen tracks, then mix prepared global effects"
		uses: [
			"port.RealTime.AudioBoundary",
			"port.RealTime.StructuralGraphBoundary",
			"port.RealTime.AudioObservation",
			"aggregate.RealTime.PreparedGraph",
			"aggregate.RealTime.PreparedEngineRack",
			"aggregate.RealTime.PreparedPostEffectRack",
			"valueObject.Mixer.MixObservation",
			"valueObject.RealTime.PatchEffectObservation",
		]
		operations: {
			fromPrepared: {input: {initialGraph: "PreparedGraph"}, output: {result: "Result<AudioRenderer, AudioError>"}}
			render: {input: {interleavedStereo: "&mut [f32]"}, output: {}}
		}
		meta: rules: [
			"construction receives one completely prepared initial graph before the callback starts and never prepares an engine, Patch effect, mixer/global effect, route, or buffer itself",
			"at the start of a render block, if no prior retired graph occupies the bounded callback retirement slot, take at most one prepared replacement, swap the complete graph, activate its initial parameters, and publish the active revision",
			"every engine or preset replacement follows the identical bounded swap path; it may reset voices and global-effect tails and makes no seamless migration claim",
			"move the replaced graph into the dedicated return queue; if that queue is full, retain it in the one preallocated callback retirement slot and retry on later blocks without taking another replacement or destroying it",
			"publish retiredRevision only after the return queue owns the old graph; control does not submit another structure until that acknowledgement is observed and collected",
			"render drains only currently available AudioCommands, reads one latest ParameterSnapshot compatible with the active graph revision, PatchIds, stable effect slots, and instrument/effect scalar layouts, gives each targeted dispatch and per-Patch render only its matching RtPatchParameters, processes each configured stem once through PreparedPostEffectRack using matching RtPostEffectParameters, then passes processed stems, PatchOutput values, and all sixteen track parameter sets to MixEngine",
			"the callback signal order is engine rack, PatchAudioBlock, ordered post-effect rack, Patch trim, fixed track accumulation, track level/pan, pre-gate meter, mute/solo gate, track sends, and global mix; neither MixEngine nor a renderer capability branch implements Chorus",
			"render divides an oversized interleaved stereo callback into complete consecutive blocks of at most PreparedGraph.maxFrames and renders every complete device frame without a silently cleared tail",
			"an unknown PatchMidi identity leaves every prepared instrument unchanged and is preserved as one fixed-size routing failure in the same injected AudioObservation path",
			"one engine-managed SoundFont synthesizer per SoundFont Patch plus every Patch-local Braids FFI bank, sixteen-voice bank iteration, 24-sample internal rendering, per-note envelopes, and 2:1 conversion remain inside the same no-allocation callback contract",
			"PatchId and Patch index remain aligned from AudioCommand through synthesis, effect processing, PatchOutput, and track accumulation; a combined engine master buffer must never be treated as one Patch's input",
			"after rendering each block, combine PatchEffectObservation, MixEngine's MixObservation, and bounded command/active-note counters into one AudioObservationSnapshot tagged with the consumed ParameterSnapshot generation",
			"the active-note observer is prepared outside the callback, has explicit Patch, channel, and note bounds, saturates counters on overflow, and never controls or substitutes prepared instrument state",
			"audio observations never change rendering, synth state, mix state, event coverage, or acceptance results and never call a control-side serializer or logger",
			"render never allocates, deallocates, locks, blocks, performs I/O, logs, formats strings, grows a collection, panics, unwinds, or destroys owned state",
		]
		validations: [
			{id: "validation.service.audio_renderer_realtime_contract", kind: "integration", command: ["bash", "scripts/run_exact_test_validation.sh", "production_runtime_contracts", "audio_renderer_realtime_contract", "CREST_RT_VALIDATION audio_renderer_realtime_contract passed"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "\"testsExecuted\":1"}], description: "the production application prepares from a supported non-default negotiated rate and exact capacity, then completely chunks an oversized callback through injected boundaries"},
			{id: "validation.service.audio_renderer_graph_handoff", kind: "integration", command: ["bash", "scripts/run_exact_test_validation.sh", "production_runtime_contracts", "prepared_graph_handoff", "CREST_RT_VALIDATION prepared_graph_handoff passed"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "\"testsExecuted\":1"}], description: "complete graphs swap only at block boundaries, acknowledge active and retired revisions, and drop only during control collection"},
			{id: "validation.service.audio_renderer_observation_contract", kind: "integration", command: ["bash", "scripts/run_exact_test_validation.sh", "production_runtime_contracts", "audio_observation_realtime_contract", "CREST_RT_VALIDATION audio_observation_realtime_contract passed"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "\"testsExecuted\":1"}], description: "the production renderer preserves exact unknown-Patch failure through coherent bounded callback observation without fallback or mutation"},
		]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "owns block-boundary graph activation and bounded retirement retry"},
			{capability: "capability.asynchronous_engine_selection", contribution: "activates the complete committed candidate without inspecting engine identity"},
			{capability: "capability.soundfont_preset_selection", contribution: "activates the exact prepared preset candidate without strings, label lookup, or a second transport"},
			{capability: "capability.soundfont_audio", contribution: "joins the SoundFont and global mixer into the callback"},
			{capability: "capability.braids_engine", contribution: "joins the pinned Braids renderer through the same matching Patch projection and rack"},
			{capability: "capability.per_voice_envelope", contribution: "delivers common ADSR values to independent note voices without adding a post-stem processor"},
			{capability: "capability.static_patch_effect", contribution: "processes the matching Patch stem through the prepared effect rack before mix and publishes bounded causal measurements"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "routes identity-preserving stems through the fixed track bank and publishes bounded per-track meters"},
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
	validations: [{id: "validation.adapter.lock_free_structural_graph_boundary", kind: "test", command: ["cargo", "test", "lock_free_structural_graph_boundary"], description: "both directions are bounded FIFO ownership transfer, pressure preserves values, status is coherent, and destructors run only under explicit control collection"}]
	contributesTo: [
		{capability: "capability.prepared_engine_rack", contribution: "implements the dedicated prepared/retired graph handoff and acknowledgement path"},
		{capability: "capability.asynchronous_engine_selection", contribution: "preserves candidate and retired ownership under queue pressure"},
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
	validations: [{id: "validation.adapter.atomic_audio_observation", kind: "test", command: ["cargo", "test", "atomic_audio_observation"], description: "publication and reads are coherent, latest-wins, monotonic, and allocation-free on the callback side"}]
	contributesTo: [
		{capability: "capability.live_observable_demo", contribution: "implements the bounded callback-to-control observation seam used by live checkpoints"},
		{capability: "capability.realtime_execution", contribution: "keeps meters and health data out of the event and parameter transports"},
	]
}
