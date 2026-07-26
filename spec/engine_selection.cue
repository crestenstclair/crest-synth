package crestsynth

// Phase 3 planning slice: one bounded asynchronous engine change for the
// focused Patch. This file is the compact architecture delta; owning resources
// in the other context files are reconciled to it.

project: goals: select_patch_engine: {
	description: "The player can replace the focused Patch engine with an adjacent installed capability while the current graph stays audible until a complete prepared replacement activates"
	priority: "required"
	actors: ["actor.player", "actor.maintainer"]
	dependsOn: ["goal.inspect_patch"]
	capabilities: [
		"capability.instrument_capability_model",
		"capability.schema_driven_patch_page",
		"capability.asynchronous_engine_selection",
		"capability.prepared_engine_rack",
		"capability.one_way_parameter_control",
		"capability.realtime_execution",
	]
	requirements: [
		"requirement.descriptor_default_engine_config",
		"requirement.reducer_owned_engine_selection",
		"requirement.one_in_flight_structural_work",
		"requirement.atomic_prepared_activation",
		"requirement.correlated_structural_outcomes",
		"requirement.retryable_structural_publication",
		"requirement.engine_selection_demo_proof",
	]
}

project: capabilities: asynchronous_engine_selection: {
	description: "Request, prepare, commit, publish, activate, retire, and observe one capability-polymorphic Patch engine replacement without callback work or fallback"
	goals: ["goal.select_patch_engine", "goal.observe_synth", "goal.observe_live_synth"]
	acceptance: prepared_adjacent_choice: {
		description: "the focused PATCH engine row selects both installed engines through one correlated asynchronous lifecycle"
		actor: "actor.maintainer"
		steps: [
			{action: "focus the first SoundFont Patch and press Edit+Right", observes: "AppState accepts one request, enters Preparing, and leaves the active config and graph revision exact while the old SoundFont graph remains audible"},
			{action: "request another structural edit while preparation is pending", observes: "the reducer returns StructuralEditBusy with no generation or state change while MIDI, context, and valid scalar events remain available"},
			{action: "advance the injected worker and control tick", observes: "the real providers, preparers, and graph builder return a correlated Braids candidate; AppState commits it before parameter projection and complete-graph publication"},
			{action: "render and collect the replacement", observes: "the callback swaps only at a block boundary, returns the prior graph for off-callback destruction, and the reducer reaches Ready only after exact activation and retirement acknowledgement"},
			{action: "press Edit+Left and complete the reverse path", observes: "the same Patch returns to the descriptor-default HiDef SoundFont config and produces finite nonzero targeted output without restoring cached inactive-engine values"},
			{action: "inject a typed preparation failure and stale outcomes", observes: "the prior config, revision, graph, routing, and unrelated Patches remain exact; no graph is published and no capability or asset is substituted"},
			{action: "run the paced live scene", observes: "the production threaded worker completes both successful directions, each Preparing/Activating/Ready state and graph revision is visible, targeted physical output is finite and nonzero, and the final config is descriptor-default SoundFont"},
		]
		evidence: ["evidence.engine_selection_workflow"]
	}
}

project: requirements: {
	descriptor_default_engine_config: {
		kind: "functional"
		description: "An engine request chooses the adjacent nonwrapping installed CapabilityId and constructs its InstrumentConfig from the target descriptor's ordered typed defaults and required default asset references through the identity-matched provider; it never translates the prior engine config, caches inactive configs, branches on SoundFont/Braids, or substitutes a missing value"
		goals: ["goal.select_patch_engine"]
		capabilities: ["capability.instrument_capability_model", "capability.asynchronous_engine_selection"]
	}
	reducer_owned_engine_selection: {
		kind: "nonfunctional"
		description: "Edit+Left/Right remains the existing semantic Adjust event; only AppState.apply may accept the focused PATCH engine-row request or the correlated prepared, failed, and activation outcomes, and every accepted lifecycle transition commits before projection or external effects"
		goals: ["goal.select_patch_engine"]
		capabilities: ["capability.asynchronous_engine_selection", "capability.one_way_parameter_control", "capability.schema_driven_patch_page"]
	}
	one_in_flight_structural_work: {
		kind: "nonfunctional"
		description: "Exactly one engine-selection request is Preparing or Activating application-wide; another structural request is a typed unchanged rejection, while MIDI, context selection, and valid MIXER scalar edits continue through the normal reducer path"
		goals: ["goal.select_patch_engine"]
		capabilities: ["capability.asynchronous_engine_selection", "capability.one_way_parameter_control"]
	}
	atomic_prepared_activation: {
		kind: "nonfunctional"
		description: "A capacity-one off-callback worker builds one complete candidate graph; a correlated success commits only the selected Patch config, reprojects the exact committed target-revision snapshot, publishes the complete graph, swaps at a block boundary, and retires the prior graph off callback; active voices and effect tails may reset"
		goals: ["goal.select_patch_engine"]
		capabilities: ["capability.asynchronous_engine_selection", "capability.prepared_engine_rack", "capability.realtime_execution"]
	}
	correlated_structural_outcomes: {
		kind: "nonfunctional"
		description: "Monotonic request id, PatchId, source and target CapabilityIds, source and target GraphRevisions, candidate config, Patch order, layout, and acknowledgement must all agree; failure is visible and accepted, while early, duplicate, stale, or mismatched results are typed unchanged rejections and every rejected candidate is destroyed off callback"
		goals: ["goal.select_patch_engine"]
		capabilities: ["capability.asynchronous_engine_selection", "capability.prepared_engine_rack"]
	}
	retryable_structural_publication: {
		kind: "nonfunctional"
		description: "If structural publication is full after candidate commit, control retains exactly one staged complete graph, remains Activating, and retries without rollback, drop, fallback, or a second structural request; Ready requires active plus retired acknowledgement and explicit control-side collection"
		goals: ["goal.select_patch_engine"]
		capabilities: ["capability.asynchronous_engine_selection", "capability.realtime_execution"]
	}
	engine_selection_demo_proof: {
		kind: "functional"
		description: "The deterministic production-path demo proves SoundFont to Braids to descriptor-default SoundFont, pending old-engine audio, busy and stale rejection, controlled preparation failure, target-only config mutation, compatible scalar snapshots, block-boundary swaps, off-callback retirement, finite targeted output, zero callback allocation or destruction, and byte-identical logical evidence across two fresh runs"
		goals: ["goal.select_patch_engine", "goal.observe_synth"]
		capabilities: ["capability.asynchronous_engine_selection", "capability.observable_demo_scene", "capability.realtime_execution"]
	}
	live_engine_selection_proof: {
		kind: "functional"
		description: "The paced physical-device demo completes SoundFont to Braids to descriptor-default SoundFont for the focused first Patch through semantic events, AppState.apply, the production threaded worker, complete graph handoff, renderer, and control-side acknowledgement; it checkpoints Preparing, Activating, Ready, increasing graph revisions, finite nonzero targeted output, zero callback allocation or destruction, no fallback, and the exact final SoundFont config without injecting controlled failures"
		goals: ["goal.observe_live_synth"]
		capabilities: ["capability.asynchronous_engine_selection", "capability.live_observable_demo", "capability.realtime_execution"]
	}
}

project: evidence: engine_selection_workflow: {
	kind: "behavioral"
	description: "the named acceptance target and exhaustive demo prove the complete correlated two-direction engine-selection lifecycle through production providers, reducer, worker port, graph handoff, renderer, observation, and off-callback collection"
	validations: ["validation.engine_selection_workflow", "validation.demo_scene", "validation.test"]
	witnesses: ["witness.engine_selection_workflow"]
}

project: contexts: Synth: applicationServices: DescriptorDefaultConfigFactory: {
	purpose: "construct one target capability config from its canonical descriptor defaults before graph preparation"
	uses: [
		"valueObject.Synth.CapabilityRegistry",
		"valueObject.Synth.CapabilityDescriptor",
		"valueObject.Synth.InstrumentConfig",
		"port.Synth.InstrumentCapabilityProvider",
	]
	operations: {
		create: {input: {capabilityId: "CapabilityId"}, output: {result: "Result<InstrumentConfig, CapabilityError>"}}
	}
	meta: rules: [
		"resolve exactly one descriptor and identity-matched provider, split every ordered ParameterSpec.defaultValue into generic values and asset references, call the existing provider createConfig operation, then revalidate the result through CapabilityRegistry",
		"the required SoundFont default asset is the descriptor-declared ./sf2/HiDef.sf2 reference and the default Braids config contains exactly its descriptor defaults",
		"run only outside the audio callback and return a typed error for unknown ids, missing defaults or assets, provider mismatch, or invalid output without fallback",
		"own no Patch mutation, inactive-engine cache, prepared object, graph, UI state, file I/O, SoundFont/Braids branch, or renderer factory",
	]
	validations: [{id: "validation.service.descriptor_default_config_factory", kind: "test", command: ["cargo", "test", "descriptor_default_config_factory"], description: "both installed default configs are exact and every missing, mismatched, or invalid default fails without fallback"}]
	contributesTo: [
		{capability: "capability.instrument_capability_model", contribution: "turns the existing descriptor defaults into one validated generic config"},
		{capability: "capability.asynchronous_engine_selection", contribution: "defines the target config without engine-specific selection logic or cached inactive values"},
	]
}

project: contexts: Control: {
	valueObjects: PatchControlId: {
		description: "a stable semantic focus target inside the PATCH context"
		from: "Engine"
		invariants: [
			"Engine is the only focusable PATCH control in this increment and serializes as patch.engine",
			"the value is never a widget index, text line, capability id, or platform input code",
		]
		contributesTo: [
			{capability: "capability.schema_driven_patch_page", contribution: "gives the engine row one stable focus identity"},
			{capability: "capability.asynchronous_engine_selection", contribution: "lets the reducer resolve Edit+Left/Right without UI-owned focus"},
		]
	}

	valueObjects: EngineSelectionRequestId: {
		description: "a monotonic correlation identity for one structural engine-selection attempt"
		from: "u64"
		invariants: [
			"zero denotes no request; accepted requests use a greater nonzero id and overflow is a typed unchanged rejection",
			"the value is copyable and contains no clock, pointer, worker handle, graph, or destructor",
		]
		contributesTo: [{capability: "capability.asynchronous_engine_selection", contribution: "correlates reducer state, worker results, graph publication, activation, retirement, and trace evidence"}]
	}

	valueObjects: EngineSelectionFailure: {
		description: "the stable visible control-side reason an engine candidate could not be prepared"
		state: {
			kind: "UnknownCapability | MissingDefault | InvalidDefaultConfig | ProviderMismatch | PreparerMissing | AssetUnavailable | UnsupportedAudioConfig | PreparationFailed | GraphIncompatible | WorkerUnavailable"
		}
		invariants: [
			"the value contains no adapter error string, path, engine, graph, decoder, allocation owner, or fallback choice",
			"adapters map concrete errors to exactly one declared case outside the callback and retain diagnostic detail only outside canonical state",
		]
		contributesTo: [{capability: "capability.asynchronous_engine_selection", contribution: "makes failed preparation explicit, serializable, and recoverable"}]
	}

	valueObjects: EngineSelectionStatus: {
		description: "the reducer-owned one-in-flight structural lifecycle"
		state: {
			kind: "Ready | Preparing | Activating | Failed"
			activeGraphRevision: "GraphRevision"
			correlation: "Option<{requestId: EngineSelectionRequestId, patchId: PatchId, sourceCapabilityId: CapabilityId, targetCapabilityId: CapabilityId, sourceGraphRevision: GraphRevision, targetGraphRevision: Option<GraphRevision>}>"
			failure: "Option<EngineSelectionFailure>"
		}
		invariants: [
			"activeGraphRevision is always nonzero; Ready has no correlation or failure, Preparing and Failed preserve the source revision, Activating keeps the source active while naming one newer target revision, and acknowledgement advances activeGraphRevision to that target before returning to Ready",
			"Preparing has exact source correlation and no target revision; Activating has one nonzero target revision; Failed retains exact request correlation plus one typed failure while the active config and graph remain the source",
			"at most one status is Preparing or Activating application-wide and only AppState.apply changes the status",
			"the status contains no PreparedGraph, worker handle, channel, provider, preparer, asset data, UI object, or callback owner",
		]
		contributesTo: [
			{capability: "capability.asynchronous_engine_selection", contribution: "is the canonical pending, activation, and failure state"},
			{capability: "capability.schema_driven_patch_page", contribution: "projects exact engine-row status without UI-owned lifecycle state"},
		]
	}

	valueObjects: EngineSelectionEffect: {
		description: "the control-side structural effect emitted only after an accepted lifecycle transition"
		state: {
			kind: "PrepareRequested | CandidateCommitted | GraphStaged | GraphPublished | ActivationAcknowledged"
			requestId: "EngineSelectionRequestId"
			patchId: "PatchId"
			sourceCapabilityId: "CapabilityId"
			targetCapabilityId: "CapabilityId"
			sourceGraphRevision: "GraphRevision"
			targetGraphRevision: "Option<GraphRevision>"
		}
		invariants: [
			"the effect contains stable control metadata only and never owns a PreparedGraph or adapter error",
			"PrepareRequested names the active source revision and no target; candidate, staged, published, and acknowledged effects name the same newer target revision",
			"EventRecord serializes every effect exactly once at the control transition or orchestration step that produced it",
		]
		contributesTo: [
			{capability: "capability.asynchronous_engine_selection", contribution: "makes structural orchestration observable without leaking graph ownership into AppState"},
			{capability: "capability.observable_demo_scene", contribution: "adds exact lifecycle effects to schema-derived coverage"},
		]
	}
}

project: contexts: RealTime: {
	valueObjects: GraphPreparationRequest: {
		description: "one immutable control-owned request to prepare a complete candidate graph"
		state: {
			requestId: "EngineSelectionRequestId"
			patchId: "PatchId"
			sourceCapabilityId: "CapabilityId"
			targetCapabilityId: "CapabilityId"
			sourceGraphRevision: "GraphRevision"
			targetGraphRevision: "GraphRevision"
			candidatePatches: "Vec<Patch>"
			candidateParameters: "ParameterSnapshot"
			audioConfig: "AudioDeviceConfig"
		}
		invariants: [
			"candidatePatches has the exact active PatchIds, order, capacity, routes, mixer values, and envelopes with only patchId's InstrumentConfig replaced by the one descriptor-default target config",
			"candidateParameters is compatible with candidatePatches and targetGraphRevision; the prepared graph is refreshed from the exact committed target snapshot before publication",
			"targetGraphRevision is greater than sourceGraphRevision and the selected candidate Patch config matches targetCapabilityId through the immutable registry",
			"the request is created and consumed outside the callback and contains no active graph, device owner, UI object, or fallback",
		]
		contributesTo: [{capability: "capability.asynchronous_engine_selection", contribution: "freezes the exact candidate and correlation sent to off-callback preparation"}]
	}

	valueObjects: GraphPreparationResult: {
		description: "one ownership-bearing worker outcome returned to the control coordinator"
		state: {
			kind: "Prepared | Failed"
			correlation: "{requestId: EngineSelectionRequestId, patchId: PatchId, sourceCapabilityId: CapabilityId, targetCapabilityId: CapabilityId, sourceGraphRevision: GraphRevision, targetGraphRevision: GraphRevision}"
			candidateConfig: "Option<InstrumentConfig>"
			preparedGraph: "Option<PreparedGraph>"
			failure: "Option<EngineSelectionFailure>"
		}
		invariants: [
			"Prepared owns exactly one matching candidate config and complete graph and no failure; Failed owns one typed failure and no config or graph",
			"the result never enters AppState; AppLoop extracts stable semantic payload for AppState.apply and retains or destroys graph ownership only outside the callback",
			"a stale, mismatched, or rejected Prepared result is destroyed on worker/control ownership and never published or used as fallback",
		]
		contributesTo: [
			{capability: "capability.asynchronous_engine_selection", contribution: "keeps complete graph ownership external to canonical control state"},
			{capability: "capability.prepared_engine_rack", contribution: "returns a complete capability-neutral candidate or an explicit failure"},
		]
	}

	ports: GraphPreparationWorker: {
		direction: "outbound"
		contract: {
			trySubmit: "(GraphPreparationRequest) -> Result<(), WorkerBusy>"
			tryPoll: "() -> Option<GraphPreparationResult>"
			shutdownOnControl: "() -> Result<(), WorkerShutdownError>"
		}
		consumes: ["valueObject.RealTime.GraphPreparationRequest", "valueObject.RealTime.GraphPreparationResult"]
		invariants: [
			"request and result capacities are exactly one and control operations never block a UI tick",
			"all provider lookup, asset access, parsing, allocation, engine/effect preparation, graph construction, rejected-result destruction, and worker joining occur outside the audio callback",
			"the port has one production threaded adapter and one manually advanced deterministic adapter; neither changes lifecycle semantics or constructs a fallback",
			"shutdown begins only after the audio stream is released and drains or destroys every retained request, result, staged graph, and retired graph on non-real-time ownership",
		]
		contributesTo: [
			{capability: "capability.asynchronous_engine_selection", contribution: "separates slow preparation from reducer, UI tick, and callback ownership"},
			{capability: "capability.realtime_execution", contribution: "keeps structural construction and destruction off the callback"},
		]
	}
}

project: adapters: ThreadedGraphPreparationWorker: {
	implements: "port.RealTime.GraphPreparationWorker"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "standard-library-thread-and-bounded-channels"}
	meta: rules: [
		"own one dedicated standard-library thread with one-slot request and result transports; no async runtime is added",
		"use only the injected immutable registry, ordered preparers, PreparedGraphBuilder, negotiated AudioDeviceConfig, and the already provider-validated candidate in GraphPreparationRequest",
		"revalidate the candidate config and exact Patch/layout correlation before invoking preparers; never call a provider twice or reconstruct a different candidate inside the worker",
		"preserve exact request correlation, return one typed failure without fallback, and never call AppState.apply, publish a graph, render audio, or mutate a report",
		"return all unconsumed ownership to the control-side shutdown path after the audio stream has stopped",
	]
	validations: [{id: "validation.adapter.threaded_graph_preparation_worker", kind: "test", command: ["cargo", "test", "threaded_graph_preparation_worker"], description: "capacity, both real engine directions, failure mapping, correlation, nonblocking polling, and control-side shutdown are exact"}]
	contributesTo: [{capability: "capability.asynchronous_engine_selection", contribution: "implements production off-callback structural preparation without adding an async runtime"}]
}

project: adapters: DeterministicGraphPreparationWorker: {
	implements: "port.RealTime.GraphPreparationWorker"
	layer: "infrastructure"
	profile: {kind: "test_double", medium: "manual-step-same-thread"}
	meta: rules: [
		"trySubmit stores the same capacity-one GraphPreparationRequest and advance performs exactly one deterministic unit of work only when the scene asks",
		"the composed healthy path receives a request created by the real DescriptorDefaultConfigFactory and production provider, then calls the production preparers and PreparedGraphBuilder; it does not return a fake PreparedGraph",
		"one declared controlled mode returns a typed EngineSelectionFailure at the worker seam without editing AppState, EventLog, DemoSceneReport, or audio measurements",
		"logical outputs contain no thread timing, wall clock, pointer, or nondeterministic ordering so two fresh scene runs are byte-identical",
	]
	validations: [{id: "validation.adapter.deterministic_graph_preparation_worker", kind: "test", command: ["cargo", "test", "deterministic_graph_preparation_worker"], description: "manual pending, real healthy preparation, controlled failure, capacity, and deterministic ownership are exact"}]
	contributesTo: [
		{capability: "capability.asynchronous_engine_selection", contribution: "makes the asynchronous lifecycle deterministically controllable without weakening production preparation"},
		{capability: "capability.observable_demo_scene", contribution: "lets the demo prove pending, completion, failure, and acknowledgement at explicit steps"},
	]
}

project: contexts: Testing: valueObjects: EngineSelectionObservation: {
	description: "the focused measured evidence emitted by the named engine-selection acceptance"
	state: {
		schemaVersion: "u32"
		patchId: "PatchId"
		sourceCapabilityId: "String"
		forwardCapabilityId: "String"
		finalCapabilityId: "String"
		revisions: "[GraphRevision; 3]"
		preparingSourceAudible: "bool"
		busyRejected: "bool"
		staleResultRejected: "bool"
		controlledFailurePreservedSource: "bool"
		fallbackCount: "u64"
		graphPublications: "u64"
		activationAcknowledgements: "u64"
		braidsPrimaryPatchRms: "f32"
		soundfontPrimaryPatchRms: "f32"
		targetAudioFinite: "bool"
		targetedPatchExact: "bool"
		untargetedPatchExact: "bool"
		finalDescriptorDefaultSoundFont: "bool"
		callbackAllocations: "u64"
		callbackDeallocations: "u64"
		callbackDestructions: "u64"
		retiredGraphsCollectedOffCallback: "u64"
	}
	invariants: [
		"all fields are measured from the production reducer, providers, preparers, graph builder, structural boundary, renderer, and audio observations rather than copied from expected data",
		"the marker is emitted only after both successful directions, the controlled failure, stale probe, exact target/untargeted Patch checks, off-callback collection, and targeted audible output pass; byte-identical two-run evidence remains owned by the exhaustive demo witness",
	]
	contributesTo: [
		{capability: "capability.asynchronous_engine_selection", contribution: "is the machine-readable focused acceptance result"},
		{capability: "capability.observable_demo_scene", contribution: "shares exact structural predicates with the exhaustive demo"},
	]
}

project: validations: engine_selection_workflow: {
	id: "validation.engine_selection_workflow"
	scope: "project"
	kind: "integration"
	command: ["cargo", "test", "--test", "engine_selection_workflow", "--", "--nocapture"]
	assertions: [
		{type: "exit-code", equals: 0},
		{type: "stdout-contains", value: "CREST_ACCEPTANCE engine_selection_workflow passed"},
	]
	timeout: "180s"
	resources: [
		"applicationService.Synth.DescriptorDefaultConfigFactory",
		"valueObject.Control.PatchControlId",
		"valueObject.Control.EngineSelectionRequestId",
		"valueObject.Control.EngineSelectionFailure",
		"valueObject.Control.EngineSelectionStatus",
		"valueObject.Control.EngineSelectionEffect",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"valueObject.RealTime.GraphPreparationRequest",
		"valueObject.RealTime.GraphPreparationResult",
		"port.RealTime.GraphPreparationWorker",
		"adapter.ThreadedGraphPreparationWorker",
		"adapter.DeterministicGraphPreparationWorker",
		"applicationService.RealTime.PreparedGraphBuilder",
		"applicationService.RealTime.StructuralGraphCoordinator",
		"port.RealTime.StructuralGraphBoundary",
		"applicationService.RealTime.AudioRenderer",
		"valueObject.Testing.EngineSelectionObservation",
		"asset.EngineSelectionAcceptanceTests",
	]
	capabilities: ["capability.asynchronous_engine_selection", "capability.instrument_capability_model", "capability.prepared_engine_rack", "capability.realtime_execution", "capability.observable_demo_scene"]
	goals: ["goal.select_patch_engine", "goal.observe_synth"]
	description: "the production-path target proves both engine directions, pending/busy/failure/stale semantics, exact config and revision correlation, complete graph activation/retirement, audible target output, deterministic evidence, and zero callback allocation or destruction"
}

project: witnesses: engine_selection_workflow: {
	scope: "goal"
	goal: "goal.select_patch_engine"
	capability: "capability.asynchronous_engine_selection"
	resources: [
		"applicationService.Synth.DescriptorDefaultConfigFactory",
		"aggregate.Control.AppState",
		"applicationService.Control.AppLoop",
		"port.RealTime.GraphPreparationWorker",
		"adapter.DeterministicGraphPreparationWorker",
		"applicationService.RealTime.StructuralGraphCoordinator",
		"applicationService.RealTime.AudioRenderer",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"valueObject.Testing.EngineSelectionObservation",
		"asset.EngineSelectionAcceptanceTests",
	]
	repairResources: [
		"applicationService.Synth.DescriptorDefaultConfigFactory",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"port.RealTime.GraphPreparationWorker",
		"applicationService.RealTime.StructuralGraphCoordinator",
		"applicationService.RealTime.AudioRenderer",
		"applicationService.Testing.ExhaustiveGuiDemo",
	]
	evidence: ["evidence.engine_selection_workflow"]
	command: ["cargo", "test", "--test", "engine_selection_workflow", "--", "--nocapture"]
	timeout: "180s"
	observation: {
		kind: "json_stdout"
		marker: "CREST_ENGINE_SELECTION_OBSERVATION "
		schema: {
			schemaVersion: "number"
			patchId: "number"
			sourceCapabilityId: "string"
			forwardCapabilityId: "string"
			finalCapabilityId: "string"
			revisions: "array"
			preparingSourceAudible: "bool"
			busyRejected: "bool"
			staleResultRejected: "bool"
			controlledFailurePreservedSource: "bool"
			fallbackCount: "number"
			graphPublications: "number"
			activationAcknowledgements: "number"
			braidsPrimaryPatchRms: "number"
			soundfontPrimaryPatchRms: "number"
			targetAudioFinite: "bool"
			targetedPatchExact: "bool"
			untargetedPatchExact: "bool"
			finalDescriptorDefaultSoundFont: "bool"
			callbackAllocations: "number"
			callbackDeallocations: "number"
			callbackDestructions: "number"
			retiredGraphsCollectedOffCallback: "number"
		}
	}
	predicates: [
		{field: "schemaVersion", op: "eq", value: 1},
		{field: "sourceCapabilityId", op: "eq", value: "instrument.soundfont.hidef"},
		{field: "forwardCapabilityId", op: "eq", value: "instrument.braids"},
		{field: "finalCapabilityId", op: "eq", value: "instrument.soundfont.hidef"},
		{field: "preparingSourceAudible", op: "eq", value: true},
		{field: "busyRejected", op: "eq", value: true},
		{field: "staleResultRejected", op: "eq", value: true},
		{field: "controlledFailurePreservedSource", op: "eq", value: true},
		{field: "fallbackCount", op: "eq", value: 0},
		{field: "graphPublications", op: "eq", value: 2},
		{field: "activationAcknowledgements", op: "eq", value: 2},
		{field: "braidsPrimaryPatchRms", op: "gt", value: 0},
		{field: "soundfontPrimaryPatchRms", op: "gt", value: 0},
		{field: "targetAudioFinite", op: "eq", value: true},
		{field: "targetedPatchExact", op: "eq", value: true},
		{field: "untargetedPatchExact", op: "eq", value: true},
		{field: "finalDescriptorDefaultSoundFont", op: "eq", value: true},
		{field: "callbackAllocations", op: "eq", value: 0},
		{field: "callbackDeallocations", op: "eq", value: 0},
		{field: "callbackDestructions", op: "eq", value: 0},
		{field: "retiredGraphsCollectedOffCallback", op: "eq", value: 2},
	]
}

project: assets: EngineSelectionAcceptanceTests: {
	kind: "rust-integration-tests"
	description: "tests/engine_selection_workflow.rs, the non-vacuous production-path acceptance for asynchronous Patch engine replacement"
	profile: {kind: "verification_harness", witness: "two-direction structural engine selection", failurePolicy: "missing target, missing marker, fallback, silence, lifecycle mismatch, or callback ownership violation fails"}
	targets: [
		"applicationService.Synth.DescriptorDefaultConfigFactory",
		"aggregate.Control.AppState",
		"applicationService.Control.AppLoop",
		"adapter.DeterministicGraphPreparationWorker",
		"applicationService.RealTime.StructuralGraphCoordinator",
		"applicationService.RealTime.AudioRenderer",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"valueObject.Testing.EngineSelectionObservation",
	]
	prompts: [
		"Create tests/engine_selection_workflow.rs with at least one ordinary assertion-bearing test and emit CREST_ACCEPTANCE engine_selection_workflow passed only after the structured observation satisfies every witness predicate.",
		"Use the production HiDef and Braids providers/preparers, reducer/projector/AppLoop, deterministic implementation of the production worker port, complete graph builder, structural boundary/coordinator, renderer, and audio observation. Do not duplicate lifecycle, preparation, routing, render, or verdict logic in the test.",
		"Prove SoundFont to Braids to descriptor-default SoundFont, old-engine audio while pending, a busy rejection, a controlled preparation failure, stale and mismatched outcomes, target-only mutation, compatible committed snapshots, block-boundary swaps, control-side retirement, finite nonzero targeted output, zero callback allocation/destruction, no fallback, and two-run logical determinism.",
		"The exhaustive make demo must execute the same lifecycle and failure path; this focused target does not replace demo coverage.",
	]
	validations: [{id: "validation.asset.engine_selection_acceptance", kind: "integration", command: ["cargo", "test", "--test", "engine_selection_workflow", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE engine_selection_workflow passed"}], description: "the named engine-selection target exists, executes assertions, and emits its marker only after the structured predicates pass"}]
	contributesTo: [
		{capability: "capability.asynchronous_engine_selection", contribution: "provides the focused executable lifecycle witness"},
		{capability: "capability.observable_demo_scene", contribution: "keeps the focused target and exhaustive demo on the same production seams"},
	]
}
