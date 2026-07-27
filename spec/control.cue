package crestsynth

project: contexts: Control: {
	purpose: "the one-way application state, semantic events, and projections"

	valueObjects: TopLevelContext: {
		description: "the only top-level product context selected through the reducer"
		from: "Patch | Mixer"
		invariants: [
			"PATCH and MIXER are the complete closed set; the transitional diagnostic view is MIXER and no third text, utility, engine, or global context exists",
			"the value is semantic application state and never a key code, window tab index, widget id, display label, or audio route",
		]
		contributesTo: [{capability: "capability.schema_driven_patch_page", contribution: "gives direct page selection one canonical semantic identity"}]
	}

	valueObjects: InteractionState: {
		description: "reducer-owned context and per-context focus retained independently from session and runtime state"
		state: {
			context: "TopLevelContext"
			mixerSelection: "Selection { section: Patch | Global, patchIndex: usize, parameterIndex: usize }"
			patchFocus: "Option<PatchId>"
			patchControlFocus: "Option<PatchControlId>"
		}
		invariants: [
			"initial context is MIXER so normal startup preserves the existing interface",
			"mixerSelection retains the existing complete Patch/GLOBAL navigation position while PATCH is active",
			"after at least one Patch is installed patchFocus contains exactly one installed stable PatchId, never a vector or widget index; before installation it is None",
			"InstallPatches initializes patchFocus to the first Patch in accepted stable installation order and patchControlFocus to Engine; later context switches and structural lifecycle events preserve both",
			"when a Patch is installed patchControlFocus is exactly one member of the focused Patch resolver's Engine, canonical ADSR, descriptor-declared instrument StructuralChoice, then ordered effect ScalarEdit order; before installation it is None",
			"the value contains no Patch/config copy, capability descriptor copy, engine, graph, parameter snapshot, UI widget, or device state",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "keeps context and existing MIXER selection inside the only reducer-owned state"},
			{capability: "capability.schema_driven_patch_page", contribution: "retains stable PATCH focus independently from MIXER navigation"},
		]
	}

	valueObjects: AppEvent: {
		description: "the closed semantic input union accepted by AppState"
		state: {
			kind: "SelectContext | Navigate | Adjust | InstallPatches | Midi | EnginePrepared | EnginePreparationFailed | EngineActivationAcknowledged"
			payload: "event-specific bounded payload"
			surfaceDescriptor: "typed exhaustive descriptors for every variant and Direction payload"
		}
		invariants: [
			"SelectContext carries TopLevelContext",
			"Navigate and Adjust carry Direction",
			"InstallPatches is accepted only during startup on the control thread",
			"Midi carries PatchId and MidiMessage",
			"EnginePrepared carries stable request/Patch/StructuralEditIntent/capability/revision correlation plus one candidate InstrumentConfig but never a PreparedGraph; the historical variant name covers every prepared instrument-config replacement",
			"EnginePreparationFailed carries the same correlation plus one typed EngineSelectionFailure and no adapter error string",
			"EngineActivationAcknowledged carries requestId, target GraphRevision, retired GraphRevision, and collected=true from control ownership",
			"raw key codes, window objects, clocks, files, and audio devices never appear in AppEvent",
			"surfaceDescriptor is produced beside the closed enum and is the only exhaustive event source consumed by DemoScene; adding or removing a variant cannot compile or pass schema equality without updating the descriptor",
			"surfaceDescriptor entries are unique before set conversion and enumerate SelectContext with both TopLevelContext values, Navigate and Adjust with all four Direction payloads, InstallPatches and Midi, plus every correlated structural prepared, failed, and activation payload shape",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "gives every input one semantic route into AppState"},
			{capability: "capability.schema_driven_patch_page", contribution: "routes direct context selection through AppState.apply"},
		]
	}

	valueObjects: EventRecord: {
		description: "one deterministic control-side record of an input event, its reducer outcome, emitted effects, and coherent projections"
		state: {
			sequence: "u64"
			source: "Startup | Keyboard | AutomaticMidi | DemoScene | Worker | System"
			input: "stable tagged AppEvent representation including TopLevelContext, Direction, PatchId, MidiMessage, structural-edit intent/correlation, config, failure, and acknowledgement payloads when present"
			outcome: "Accepted | Rejected"
			rejection: "Option<EventRejection>"
			generationBefore: "u64"
			generationAfter: "u64"
			stateHashBefore: "String"
			stateHashAfter: "String"
			emittedEvents: "Vec<stable tagged StateAccepted, audio-effect, or EngineSelectionEffect descriptors>"
			parameterGeneration: "u64"
			projectionStateHash: "String"
			selectedLine: "usize"
			serializedLeafDescriptor: "typed stable paths for every EventRecord field and tagged payload leaf"
		}
		invariants: [
			"sequence is contiguous and strictly increasing within one EventLog",
			"accepted records increment generation exactly once and their stateHashAfter, parameterGeneration, projectionStateHash, emitted StateAccepted generation, and emitted ParameterSnapshotPublished graphRevision agree with the canonical projections",
			"rejected records keep generation and state hash identical and emit no parameter publication or audio command",
			"records contain domain identifiers and bounded numeric payloads, never raw window objects, device handles, audio buffers, or nondeterministic timestamps",
			"serializedLeafDescriptor is compared against the union of discriminating accepted and rejected EventRecords covering every input tag, Direction payload, MIDI payload, Patch installation capability/config field, rejection, and emitted-event tag; discovery from one convenient record is invalid",
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
			capabilities: "Vec<CapabilityDescriptor>"
			effectCapabilities: "Vec<EffectCapabilityDescriptor>"
			patches: "Vec<{id, name, channel, instrument: {capabilityId, values: Vec<{parameterId, value}>, assetReferences: Vec<{parameterId, reference}>}, postEffects: Vec<{slotId, capabilityId, values: Vec<{parameterId, value}>, assetReferences: Vec<{parameterId, reference}>}>, envelope: {attackMilliseconds, decayMilliseconds, sustain, releaseMilliseconds}, parameters: {gainDb, pan, reverbSend, delaySend}}>"
			global: "{masterGainDb, reverbRoomSize, reverbDamping, reverbReturn, delayMilliseconds, delayFeedback, delayReturn}"
			interaction: "{context, mixerSelection: {section, patchIndex, parameterIndex}, patchFocus, patchControlFocus}"
			engineSelection: "EngineSelectionStatus"
			patchPage: "Option<PatchPageProjection>"
			projection: "{context, body, selectedLine, stateHash}"
			parameters: "{generation, graphRevision, patchCount, patches, global}"
			serializedLeafDescriptor: "typed stable paths derived beside the StateTree serializer"
		}
		invariants: [
			"the tree contains every currently serialized CapabilityDescriptor, EffectCapabilityDescriptor, ParameterSpec, Patch, InstrumentConfig, PostEffectConfig, ParameterAssignment, AssetReference, VoiceEnvelope, ChannelParameters, GlobalParameters, InteractionState, EngineSelectionStatus, EngineSelectionFailure, PatchPageProjection, TextProjection, and ParameterSnapshot property without an opaque debug-string substitute",
			"capabilities exactly equal the installed CapabilityRegistry in stable order and every Patch InstrumentConfig resolves to exactly one listed descriptor",
			"effectCapabilities exactly equal the installed EffectCapabilityRegistry in stable order and every PostEffectConfig resolves by stable capability and slot identity",
			"patch order and numeric values exactly match StateSnapshot and ParameterSnapshot",
			"projection.context equals interaction.context, projection.stateHash equals the canonical StateSnapshot hash, parameters.generation equals generation, and parameters.graphRevision equals the target PreparedGraph revision",
			"patchPage is present exactly in PATCH context and exactly equals the projector's focused PatchPageProjection; it is absent in MIXER context",
			"serialization is deterministic JSON with a version field and stable property names",
			"serializedLeafDescriptor exactly equals recursively discovered JSON leaf paths in both directions for a discriminating multi-Patch tree",
		]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "exposes the installed descriptors and generic Patch configs as exact canonical data"},
			{capability: "capability.static_patch_effect", contribution: "exposes the effect registry, ordered Patch configs, focused rows, and fixed scalar projection as exact canonical data"},
			{capability: "capability.observable_demo_scene", contribution: "exposes the complete state/projection tree for LLM observation and diagnosis"},
		]
	}

	valueObjects: StateSnapshot: {
		description: "canonical serialized control state"
		state: {
			json: "String"
			hash: "String"
		}
			invariants: [
				"JSON contains the complete installed instrument and effect registries, every Patch InstrumentConfig, ordered PostEffectConfig, asset reference, VoiceEnvelope, ChannelParameters value, GlobalParameters, InteractionState, and EngineSelectionStatus including any typed failure",
				"decode(encode(state)) equals state",
				"a MIDI-only accepted generation may share the immutable canonical JSON suffix and defer full String materialization, but requested JSON and its hash exactly equal eager serialization of the same AppState",
			]
		contributesTo: [{capability: "capability.one_way_parameter_control", contribution: "makes accepted state serialization explicit and testable"}]
	}

	valueObjects: PatchPageProjection: {
		description: "the immutable host-neutral PATCH view model for one stable focused Patch"
		state: {
			context: "Patch"
			patch: "{id: PatchId, name: String, midiChannel: MidiChannel}"
			focusedControlId: "PatchControlId"
			engine: "{controlId: PatchControlId, activeCapabilityId: CapabilityId, activeLabel: String, choices: Vec<{capabilityId: CapabilityId, label: String}>, status: Ready | Preparing | Activating | Failed, activeGraphRevision: GraphRevision, requestedCapabilityId: Option<CapabilityId>, requestId: Option<EngineSelectionRequestId>, targetGraphRevision: Option<GraphRevision>, failure: Option<EngineSelectionFailure>, editable: bool}"
			envelope: "Vec<{controlId: PatchControlId, id: stable VoiceEnvelopeParameter id, label: String, value: f32, minimum: f32, maximum: f32, fineStep: f32, coarseStep: f32, unit: Option<String>, editable: true}>"
			sections: "Vec<{id: stable instrument descriptor section id, label: String, parameters: Vec<{controlId: Option<PatchControlId>, id: ParameterId, label: String, kind, update, patchInteraction, value: ParameterValue | AssetReference, choices: Vec<{id, label}>, requestedChoiceId: Option<String>, status: Option<Ready | Preparing | Activating | Failed>, requestId: Option<EngineSelectionRequestId>, targetGraphRevision: Option<GraphRevision>, failure: Option<EngineSelectionFailure>, unit: Option<String>, enabled: bool, visible: bool, editable: bool}>}>"
			effects: "Vec<{slotId: EffectSlotId, capabilityId: EffectCapabilityId, label: String, editableIdentity: false, sections: Vec<{id, label, parameters: Vec<{controlId: Option<PatchControlId>, id: ParameterId, label: String, kind, update, patchInteraction, value: ParameterValue | AssetReference, choices: Vec<{id, label}>, unit: Option<String>, enabled: bool, visible: bool, editable: bool}>}>}>"
			stateHash: "String"
			serializedLeafDescriptor: "typed stable paths for every Patch-page field and row variant"
		}
		invariants: [
			"patch resolves InteractionState.patchFocus by stable PatchId and copies identity, name, and MIDI channel from that exact canonical Patch",
			"focusedControlId exactly equals InteractionState.patchControlFocus; engine active identity and label resolve from the Patch InstrumentConfig and installed CapabilityRegistry; choices contain every registry entry exactly once in registry order and engine.controlId is Engine",
			"status, activeGraphRevision, requestId, targetGraphRevision, intent, and failure are exact projections of EngineSelectionStatus; active capability or preset choice changes only at the accepted EnginePrepared commit while activeGraphRevision advances only at acknowledgement",
			"engine editable is true only when the focused Patch exists, at least two installed choices exist, and status is Ready or Failed; Preparing and Activating remain visible but disabled without changing focus",
			"envelope contains exactly Attack, Decay, Sustain, and Release from the canonical Patch VoiceEnvelope descriptor in stable order; each row derives controlId from the same VoiceEnvelopeParameter, is editable in every lifecycle status, and owns no second envelope state",
			"sections and parameters exactly preserve the active CapabilityDescriptor order, stable ids, labels, kinds, update classes, patch interactions, choices, units, dependency visibility/enabled results, and canonical InstrumentConfig values or assets; only visible enabled StructuralChoice fields receive Capability(ParameterId), adjacent-choice metadata, and structural editability",
			"effects exactly preserve Patch PostEffectConfig order and stable slot identity, resolve each EffectCapabilityDescriptor generically, show effect identity read-only, and give only visible enabled ScalarEdit parameters Effect(slotId, ParameterId) control ids and scalar editability",
			"the targeted engine or capability row projects active and requested identity, Preparing/Activating/Failed status, request/revision correlation, and failure while every other structural row remains inactive; Ready has no pending target",
			"projection walks instrument and effect descriptors generically and contains no SoundFont, Braids, Chorus, preset, bank, program, percussion, Model, Timbre, Color, Amount, or Depth field list or capability-id branch",
			"stateHash equals the StateSnapshot hash for the accepted InteractionState and session values used to create the page",
		]
		validations: [{id: "validation.value_object.patch_page_projection", kind: "integration", command: ["cargo", "test", "--test", "patch_page_projection", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE patch_page_projection passed"}], description: "both production capabilities project exact descriptor-derived rows, registry choices, Patch identity, MIDI channel, and ADSR without engine-specific page logic"}]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "projects active instrument configuration through its owning descriptor"},
			{capability: "capability.schema_driven_patch_page", contribution: "is the canonical PATCH page data with one structural engine row, four scalar ADSR rows, and descriptor-declared structural-choice rows under one focus identity"},
			{capability: "capability.per_voice_envelope", contribution: "projects editable rows directly from canonical VoiceEnvelope state and descriptors"},
			{capability: "capability.asynchronous_engine_selection", contribution: "projects active/requested identity and exact lifecycle without owning worker or graph state"},
			{capability: "capability.soundfont_preset_selection", contribution: "projects the exact authored preset label, ordered choices, focus, request status, and typed failure from the generic descriptor/config/lifecycle"},
			{capability: "capability.static_patch_effect", contribution: "projects the read-only Chorus identity and editable Amount/Depth rows from canonical effect descriptor/config state"},
		]
	}

	valueObjects: TextProjection: {
		description: "the active basic text body derived from StateSnapshot and its reducer-owned context projection"
		state: {
			context: "TopLevelContext"
			body: "String"
			selectedLine: "usize"
			stateHash: "String"
			serializedLeafDescriptor: "context | body | selectedLine | stateHash"
		}
		invariants: [
			"context exactly equals InteractionState.context and the body begins with its semantic PATCH or MIXER identity plus the direct 1/2 page bindings",
			"in MIXER, each Patch appears once in stable AppState order with id, name, channel, capability id and label, every InstrumentConfig value/asset rendered in CapabilityDescriptor order, every ordered PostEffectConfig identity/value rendered in EffectCapabilityDescriptor order, gainDb, pan, reverbSend, delaySend, the four envelope values, and every Scalar engine value; Patch sections retain the literal separator and the final GLOBAL section retains all seven values",
			"in PATCH, the body is a lossless deterministic rendering of PatchPageProjection including Patch identity, MIDI channel, active/requested structural target, Ready/Preparing/Activating/Failed status, typed failure, installed engine and parameter choices, ADSR, every instrument descriptor field, and every ordered read-only effect identity plus effect field with its stable slot/parameter id and editability status",
			"the text projector contains no SoundFont/Braids capability-id branch or duplicate engine-specific field list",
			"the line matching focusedControlId begins with > even when the focused structural action is temporarily disabled; every other parameter line begins with one space and selectedLine names the marked line",
			"stateHash equals the StateSnapshot hash used to create the body",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "provides the current basic UI as one immutable active-context projection"},
			{capability: "capability.schema_driven_patch_page", contribution: "renders the host-neutral PATCH page through the existing adapter"},
		]
	}

	aggregates: AppState: {
		root: true
		purpose: "own immutable instrument/effect registries, installed Patches, global parameters, interaction state, one structural-edit runtime status, and accepted generation"
		state: {
			capabilities: "CapabilityRegistry"
			effectCapabilities: "EffectCapabilityRegistry"
			patches: "Vec<Patch>"
			global: "GlobalParameters"
			interaction: "InteractionState"
			engineSelection: "EngineSelectionStatus"
			generation: "u64"
		}
		commands: Apply: {event: "AppEvent"}
		events: StateAccepted: {generation: "u64"}
		invariants: [
			"Apply is the only mutation method",
			"CapabilityRegistry is supplied at construction, remains immutable, and the current increment contains exactly instrument.soundfont.hidef and instrument.braids",
			"EffectCapabilityRegistry is supplied at construction, remains immutable, and the current increment contains exactly effect.chorus",
			"SelectContext changes only InteractionState.context, requires an installed valid patchFocus before selecting PATCH, preserves mixerSelection and patchFocus, emits no AudioCommand, and accepts both direct context values through the same reducer",
			"in MIXER, Navigate changes only InteractionState.mixerSelection; bare Up/Down moves between parameters and bare Left/Right moves between Patch sections plus the GLOBAL section",
			"in MIXER, Adjust resolves the selected Patch target as mixer, common envelope, then descriptor-classified Scalar value and transactionally changes exactly that value within its owning descriptor; Structural values remain nonselectable",
			"in PATCH, Navigate Up/Down moves one step through the focused Patch resolver's nonwrapping Engine, Attack, Decay, Sustain, Release, visible instrument StructuralChoice order, then ordered visible effect ScalarEdit order and changes only InteractionState; Navigate Left/Right and navigation beyond either endpoint are ActionUnavailableInContext",
			"in PATCH, Adjust Left/Right on Engine resolves the adjacent nonwrapping registry capability and accepts one PrepareRequested transition, while Adjust Up/Down on Engine is ActionUnavailableInContext",
			"in PATCH, Adjust on Envelope(parameter) reuses the canonical VoiceEnvelope descriptor and mutation path: Left/Right is fine decrement/increment, Down/Up is coarse decrement/increment, and exactly that focused Patch envelope field changes",
			"in PATCH, Adjust Left/Right on Capability(parameter) validates that the active descriptor marks it StructuralChoice, resolves the adjacent nonwrapping declared choice, and requests a candidate changing only that assignment; Adjust Up/Down is ActionUnavailableInContext and a choice endpoint is ParameterAtBoundary",
			"in PATCH, Adjust on Effect(slotId, parameter) resolves the exact Patch PostEffectConfig and EffectCapabilityDescriptor ScalarEdit row: Left/Right is fine decrement/increment and Down/Up is coarse decrement/increment, exactly that value changes, and no structural effect, AudioCommand, or fallback is emitted",
			"an accepted engine or capability-choice request allocates the next nonzero EngineSelectionRequestId, records one StructuralEditIntent, enters Preparing, emits exactly one EngineSelectionEffect::PrepareRequested, and leaves every Patch InstrumentConfig plus the active GraphRevision unchanged",
			"while Preparing or Activating another PATCH structural Adjust is StructuralEditBusy and unchanged; MIDI, SelectContext, PATCH focus Navigate, and valid scalar Adjust events from MIXER or PATCH ADSR retain their normal behavior",
			"a correlated EnginePreparationFailed transition enters Failed with its typed visible failure and preserves every Patch config, graph revision, route, parameter, and unrelated state; Failed may accept a later adjacent request",
			"a correlated EnginePrepared transition revalidates the candidate config against StructuralEditIntent, changes only the requested Patch InstrumentConfig and only the permitted capability or single structural assignment, enters Activating with the target GraphRevision, and preserves Patch identity, MIDI channel, envelope, mixer parameters, every unrelated Patch, and all accepted scalar edits made while preparing",
			"EngineActivationAcknowledged reaches Ready only when request id, target active revision, retired source revision, and control-side collection all match; early, duplicate, stale, or mismatched worker and acknowledgement events are typed unchanged rejections",
			"Adjust toward a boundary when the selected value is already at that boundary is rejected as ParameterAtBoundary and leaves state identical",
			"Adjust Left/Right is the fine decrement/increment and Adjust Down/Up is the coarse decrement/increment",
			"InstallPatches preserves supplied order, rejects duplicate MidiChannels, more than 16 Patches, invalid instrument or effect capability/config/asset/slot identities, duplicate ParameterIds, or more than one post effect per Patch, initializes patchFocus to the first accepted PatchId and patchControlFocus to Engine, and is rejected after startup; the production composition separately supplies exactly one effect.chorus slot on its first fixture Patch",
				"Midi validates its Patch target read-only, preserves the exact CapabilityRegistry and Patch storage, changes only generation, and yields one AudioCommand effect after state acceptance",
			"every accepted event increments generation once and every rejected event leaves state identical",
		]
		meta: rules: [
			"publish one typed EventRejection descriptor beside the enum with a unique stable name and reachability of Scene or ReducerTable",
			"classify InstallationClosed, TooManyPatches, DuplicateMidiChannel, InvalidInstrumentConfig, InvalidEffectConfig, UnknownPatch, ParameterAtBoundary, ActionUnavailableInContext, EngineSelectionUnavailable, StructuralEditBusy, StaleEngineSelection, and MismatchedEngineSelection as externally constructible; classify NoPatchesInstalled, InvalidSelection, InvalidParameterValue, RequestIdOverflow, and GenerationOverflow as controlled reducer-table cases",
			"the exact descriptor set must equal the enum set before scene/table partitioning, and each variant is asserted by exactly one declared test path",
		]
		validations: [{id: "validation.aggregate.app_state", kind: "test", command: ["cargo", "test", "app_state"], description: "capability-aware installation, direct context selection, stable focus, context-gated navigation/adjustment, bounds, and MIDI effects are deterministic"}]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "validates every installed Patch against one immutable descriptor registry through the canonical reducer"},
			{capability: "capability.one_way_parameter_control", contribution: "is the single source of mutable control state"},
			{capability: "capability.schema_driven_patch_page", contribution: "owns page context and stable Patch focus without moving state into the UI"},
			{capability: "capability.asynchronous_engine_selection", contribution: "owns every request, failure, candidate commit, and activation transition while prepared graph ownership stays external"},
			{capability: "capability.soundfont_preset_selection", contribution: "owns preset intent, boundary, source-preserving pending state, exact single-assignment commit, and failure through the same reducer"},
			{capability: "capability.static_patch_effect", contribution: "owns canonical effect config installation and Amount/Depth scalar mutation through the same reducer"},
		]
	}

	domainServices: StateProjector: {
		purpose: "derive serialization, text, fixed real-time parameters, and the canonical observation tree from one accepted AppState"
		uses: [
			"aggregate.Control.AppState",
			"valueObject.Synth.CapabilityRegistry",
			"valueObject.Synth.CapabilityDescriptor",
			"valueObject.Synth.InstrumentConfig",
			"valueObject.Synth.EffectCapabilityRegistry",
			"valueObject.Synth.EffectCapabilityDescriptor",
			"valueObject.Synth.PostEffectConfig",
			"valueObject.Control.StateSnapshot",
			"valueObject.Control.InteractionState",
			"valueObject.Control.EngineSelectionStatus",
			"valueObject.Control.EngineSelectionFailure",
			"valueObject.Control.PatchPageProjection",
			"valueObject.Control.TextProjection",
			"valueObject.Control.StateTree",
			"valueObject.RealTime.ParameterSnapshot",
			"valueObject.RealTime.GraphRevision",
		]
			meta: rules: [
				"construct one canonical borrowed serialized-state view per eager projection, serialize AppState deterministically with serde_json, and verify round-trip identity in tests rather than deserializing Crest's own JSON on the production dispatch path",
				"derive InteractionState and TextProjection only from the accepted StateSnapshot; in MIXER render the preserved diagnostic body from mixerSelection, and in PATCH first derive PatchPageProjection by stable PatchId and then render it without capability-specific matching",
			"derive PatchPageProjection from the focused canonical Patch, PatchControlId, VoiceEnvelope descriptor, active CapabilityDescriptor, InstrumentConfig, EngineSelectionStatus, and complete installed registry; expose the exact focusedControlId, use stable ids and descriptor order, and never match on SoundFont or Braids",
			"derive the focused PATCH control order from Engine, the VoiceEnvelope descriptor, visible enabled instrument StructuralChoice parameters, then each ordered PostEffectConfig's visible enabled EffectCapabilityDescriptor ScalarEdit parameters; the same resolver supplies reducer navigation, row control ids, schema coverage, and demos",
				"copy every mixer and envelope value plus at most sixteen descriptor-ordered Scalar engine values and a separate zero-or-one fixed effect slot with at most eight descriptor-ordered Scalar values per Patch plus the caller-owned target GraphRevision into ParameterSnapshot; reject Patch, slot, or scalar capacity overflow",
				"derive StateTree from the exact same StateSnapshot, TextProjection, and ParameterSnapshot without reading or mutating any second state copy",
				"when accepted Midi changes only generation, share the prior immutable state suffix, text body, and StateTree template; advance snapshot, text, fixed parameters, and tree coherently and materialize large JSON only on observation",
				"force generation-only snapshot and tree JSON to materialize in equivalence tests and require byte-for-byte equality with an eager projection from the same accepted AppState",
		"for a discriminating projection, compare every installed capability/parameter descriptor including patchInteraction and authored choice labels, Patch identity, MIDI channel, InstrumentConfig value/asset, envelope value and descriptor, Patch parameter value, global value, interaction context/focus, PatchControlId, structural intent, engine/preset choice, selection marker, selectedLine, and stateHash exactly against accepted state; nonempty text or mere property presence is not sufficient",
			"publish production-owned typed surface/leaf descriptors beside their enum or serializer and require exact equality with recursively discovered EventLog, EventRecord, StateTree, TextProjection, and ParameterSnapshot paths",
		]
			validations: [
				{id: "validation.service.state_projector_exact_values", kind: "test", command: ["cargo", "test", "state_projector_exact_projection_values"], description: "every rendered Patch/global value, selection marker, selected line, and hash exactly matches one accepted state"},
				{id: "validation.service.state_projector_performance", kind: "integration", command: ["cargo", "test", "--test", "control_dispatch_performance", "--", "--nocapture"], description: "the complete fifteen-Patch production control path dispatches 512 MIDI events within 50 ms and deferred projections equal eager canonical output"},
				{id: "validation.service.state_projector_schema_surface", kind: "test", command: ["cargo", "test", "schema_derived_current_surface"], description: "production-owned typed descriptors exactly equal discovered serialized leaves with no missing or unexpected paths"},
		]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "derives generic descriptor/config serialization and text from one accepted state"},
			{capability: "capability.one_way_parameter_control", contribution: "keeps text and audio projections consistent with serialized accepted state"},
			{capability: "capability.schema_driven_patch_page", contribution: "derives the PATCH view model and text from canonical state and installed descriptors"},
			{capability: "capability.asynchronous_engine_selection", contribution: "projects every lifecycle generation and candidate scalar layout from committed canonical state"},
			{capability: "capability.soundfont_preset_selection", contribution: "derives the authored-name preset row, dynamic focus, requested choice, and structural lifecycle generically from catalog-backed schema"},
			{capability: "capability.static_patch_effect", contribution: "derives effect registry/config serialization, PATCH rows, stable focus, and fixed effect scalar slots without processor branches"},
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
			"valueObject.RealTime.GraphRevision",
			"port.RealTime.GraphPreparationWorker",
			"applicationService.Synth.DescriptorDefaultConfigFactory",
			"applicationService.RealTime.StructuralGraphCoordinator",
			"valueObject.Control.EngineSelectionEffect",
			"valueObject.Control.EventRecord",
			"valueObject.Control.EventLog",
			"valueObject.Control.StateTree",
		]
		operations: {
			dispatch: {input: {event: "AppEvent"}, output: {result: "Result<DispatchResult, EventRejection>"}}
			dispatchFrom: {input: {source: "EventSource", event: "AppEvent"}, output: {result: "Result<DispatchResult, EventRejection>"}}
			currentText: {input: {}, output: {projection: "TextProjection"}}
			currentPatchPage: {input: {}, output: {projection: "Option<PatchPageProjection>"}}
			currentStateTree: {input: {}, output: {state: "StateTree"}}
			eventLog: {input: {}, output: {events: "EventLog"}}
			advanceStructural: {input: {}, output: {result: "Result<StructuralProgress, ApplicationError>"}}
		}
		meta: rules: [
			"construction receives the nonzero revision of the complete initial graph and initializes EngineSelectionStatus.activeGraphRevision; later revisions advance only through correlated prepared and acknowledgement events and never become a second mutable Patch config",
			"dispatch order is reduce AppEvent through AppState.apply, commit accepted AppState, derive StateSnapshot/PatchPageProjection when active/TextProjection/ParameterSnapshot tagged with the target GraphRevision, publish parameters, then enqueue any AudioCommand",
			"an accepted SelectContext publishes a generation-coherent ParameterSnapshot with identical parameter values and GraphRevision, publishes no AudioCommand or structural request, and changes no prepared or rendered state",
			"an accepted PATCH focus Navigate publishes generation-coherent logical projections and a same-revision ParameterSnapshot with identical values, emits no AudioCommand or EngineSelectionEffect, and changes no Patch, graph, engine, mixer, or audio behavior",
			"an accepted PATCH ADSR Adjust commits one VoiceEnvelope field before projection, publishes the complete same-graph-revision fixed ParameterSnapshot, emits no AudioCommand, preparation request, or structural ownership, and lets the renderer consume the value through the existing per-voice envelope seam",
			"an accepted PATCH effect ScalarEdit commits one PostEffectConfig assignment before projection, publishes the complete same-graph-revision fixed ParameterSnapshot, emits no AudioCommand, preparation request, or structural ownership, and lets the prepared effect rack consume the matching slot value",
			"after an accepted PATCH structural request, use StructuralEditIntent to build either the descriptor-default engine candidate or the active config with exactly one adjacent StructuralChoice assignment through the installed provider, revalidate it against the immutable registry, and trySubmit it exactly once; WorkerBusy after reducer acceptance is a retained typed application fault and never causes fallback or a hidden second queue",
			"advanceStructural polls at most one worker result and graph status per control tick, maps only stable semantic payloads into AppEvents, and routes every lifecycle mutation back through AppState.apply with EventSource::Worker",
			"for a prepared result, first correlate intent and retain graph ownership, dispatch EnginePrepared, commit only its permitted candidate config delta, derive the exact target-revision StateSnapshot/PatchPageProjection/TextProjection/ParameterSnapshot, refresh PreparedGraph.initialParameters from that committed projection, publish scalar parameters, then stage or publish the complete graph",
			"a PATCH ADSR edit during Preparing publishes against the active source revision and is included when the candidate's initial parameters are refreshed after commit; during Activating it publishes against the target revision for exact consumption on activation while the old source continues with its last compatible snapshot",
			"if structural publication is full after commit, retain exactly one staged graph, remain Activating, retry on later control ticks, and reject new structural work without rollback, drop, or substitution",
			"only after GraphHandoffStatus reports the target active, the source retired, and collectRetiredOnControl has destroyed the returned source graph may AppLoop dispatch EngineActivationAcknowledged and reach Ready",
			"a failed, stale, mismatched, or rejected worker result publishes no graph; every owned candidate is destroyed on worker/control ownership and later valid input remains dispatchable",
			"dispatch preserves the existing API and delegates to dispatchFrom with a stable default source; production adapters and demo inputs call dispatchFrom with their exact source",
			"record every accepted and rejected input exactly once on the control thread; observation occurs after the outcome is known and never runs in the audio callback",
			"on rejection perform no state serialization, parameter publication, command enqueue, or view change, but append one rejected EventRecord proving generation and state hash were unchanged",
			"EventRejection is a nonfatal domain result for an input event; callers may report or ignore it but must remain able to dispatch later events",
				"accepted EventRecords include the input, StateAccepted generation, snapshot hashes, parameter publication, projection identity, and any AudioCommand descriptor",
				"accepted Midi still passes through AppState.apply and produces every generation-coherent logical projection, but AppLoop selects the projector's generation-only sharing path because no canonical parameter, selection, capability, or Patch data changed",
				"currentPatchPage and currentStateTree are derived from the same accepted state and projections without exposing mutable AppState",
				"control-side verifiers borrow EventLog for repeated reads and clone the complete retained history only when an owned final report is required",
			"views and input adapters can call only dispatch and immutable currentText/currentStateTree/eventLog reads; they never receive mutable AppState",
		]
		validations: [
			{id: "validation.service.app_loop_one_way_control", kind: "integration", command: ["cargo", "test", "one_way_control_loop"], description: "an edit on a non-first Patch changes only its serialized value and audio contribution, and a boundary no-op leaves the loop running for the next event"},
			{id: "validation.service.app_loop_observation_trace", kind: "test", command: ["cargo", "test", "control_observation_trace"], description: "accepted and rejected event records form an exact hash/generation chain and the state tree contains every current property"},
		]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "keeps capability-aware Patch installation and projection inside the existing one-way loop"},
			{capability: "capability.prepared_engine_rack", contribution: "tags fixed parameter projections with the complete prepared graph revision they target without exposing graph ownership"},
			{capability: "capability.one_way_parameter_control", contribution: "orchestrates the complete reducer-to-projection-to-audio flow"},
			{capability: "capability.schema_driven_patch_page", contribution: "exposes immutable current-context projections after semantic page events"},
			{capability: "capability.asynchronous_engine_selection", contribution: "coordinates request, worker outcome, reducer commit, snapshot publication, graph handoff, acknowledgement, and trace ordering"},
			{capability: "capability.soundfont_preset_selection", contribution: "routes an adjacent preset intent through the identical candidate, worker, commit, projection, graph, acknowledgement, and trace order"},
			{capability: "capability.static_patch_effect", contribution: "routes effect parameter edits through the canonical reducer and same latest-snapshot publication while structural replacements preserve effect config"},
			{capability: "capability.realtime_execution", contribution: "uses distinct discrete/scalar, preparation-worker, and structural-ownership seams without moving work into the callback"},
			{capability: "capability.observable_demo_scene", contribution: "records the production reducer's complete event/state/effect trace"},
		]
	}
}
