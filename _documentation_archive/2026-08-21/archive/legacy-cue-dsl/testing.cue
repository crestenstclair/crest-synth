package crestsynth

project: contexts: Testing: {
	purpose: "automatic MIDI input and bounded demo support used to exercise the synth through production ports"
	meta: rules: [
		"this context is input test support, not a sequencing or transport domain",
		"its timing and MIDI-file parsing types are private implementation details",
	]

	valueObjects: DemoScene: {
		description: "a deterministic sequence of normalized GUI inputs, MIDI probes, fixture ticks, explicit worker/graph advancement, and immutable checkpoints"
		state: {
			name: "String"
			schemaVersion: "u32"
			steps: "Vec<WindowInput | SemanticAction | MidiProbe | Tick | AdvanceGraphWorker | RenderBlock | PollStructural | Checkpoint>"
			surfaceDescriptor: "typed WindowInput kind/key, SemanticAction, AppEvent, TopLevelContext, SurfaceId, InteractionMode, FocusPath/control, Direction, MidiMessageKind, installed instrument/effect capability, parameter and slot, StructuralEditIntent, status/error, rejection, emitted-effect, and serialized-leaf descriptors from production owners"
			rejectionDescriptor: "typed unique EventRejection cases partitioned into Scene and ReducerTable reachability"
			expectedCoverage: "the exact normalized identifier set derived from surfaceDescriptor plus installed Patch identities"
		}
		invariants: [
			"the exhaustive scene is derived from typed descriptors owned beside WindowInput, SemanticAction, AppEvent, TopLevelContext, SurfaceId, InteractionMode, FocusPath/control ids, installed instrument/effect registries, parameter schemas, effect slots, emitted effects, and serializers plus the installed fixture Patch list; it never defines a second hand-maintained list of GUI inputs, actions, paths, page identities, or field-name strings",
			"a contract test discovers the serialized EventLog, EventRecord, StateTree, PatchPageProjection, SemanticGraphicalViewModel, GraphicalShellProjection, TextProjection, and ParameterSnapshot leaf paths and requires exact bidirectional set equality with surfaceDescriptor, so an added, removed, renamed, duplicated, or unexercised item fails",
			"descriptor uniqueness is asserted before converting to sets, and expectedCoverage is frozen before the first event so actual post-state, discovered output, or coverage observations can never define their own expected values",
			"every expected state value is computed before dispatch from the captured baseline plus the typed owner descriptor's bound and step; it is never copied from the actual post-dispatch StateTree, GraphicalShellProjection, TextProjection, ParameterSnapshot, or rendered audio",
			"every GUI adjustment step enters through KeyboardInputTranslator and every user semantic input enters through AppLoop.dispatchAction before AppEvent; MIDI, worker, startup, and system inputs enter through typed AppEvent dispatch",
			"ticks use deterministic elapsed durations and no wall clock, native window, physical audio device, or random input",
			"AdvanceGraphWorker, RenderBlock, and PollStructural drive the injected production worker port, graph boundary, and renderer at explicit deterministic points; they never mutate AppState, synth state, EventLog, or expected observations directly",
			"the scene is test/demo support and exposes no transport, playback, arrangement, recording, or editing feature to the product domain",
		]
			contributesTo: [
				{capability: "capability.observable_demo_scene", contribution: "declares the exhaustive deterministic control-surface exercise"},
				{capability: "capability.soundfont_preset_selection", contribution: "derives preset focus and structural-transition coverage from production descriptors rather than a test-owned list"},
			]
	}

	valueObjects: DemoSceneReport: {
		description: "the complete machine-readable result of one exhaustive GUI demo run"
		state: {
			scene: "String"
			complete: "bool"
			eventLog: "EventLog"
			initialStateTree: "StateTree"
			finalStateTree: "StateTree"
				coverage: "{expected, exercised, missing, unexpected} grouped by normalized GUI inputs, semantic actions, events, contexts, surfaces, modes, focus/return paths, valid actions, directions, MIDI kinds, instrument/effect capabilities and slots, editable parameters, structural choices/intents/statuses/failures/effects, serialized properties, rejections, projections, and audio effects"
				checkpoints: "Vec<{step, expectedStateValues, actualStateValues, expectedProjectionValues, actualProjectionValues, focusPath, interactionMode, returnPath, validActions, viewStatus, viewErrors, stateHash, generation, selectedLine, parameterGeneration, graphRevision, structuralEditIntent, engineSelectionStatus, audioMeasurement, reverbInputEnergy, delayInputEnergy}>"
		}
		invariants: [
			"complete is true only when expected and exercised identifiers are exactly equal in both directions, missing and unexpected are empty in both report and EventLog coverage, the event journal dropped no records, and all checkpoints agree",
			"the final tree is exactly the last accepted event state and the last EventRecord hash/generation chain endpoint",
			"each checkpoint compares exact typed state and projection values rather than checking only property presence, nonempty text, generation identity, or a changed aggregate buffer",
			"after every reversible scalar probe the selected Patch, PatchOutput, track, global parameter, send, and semantic selection equal the captured baseline; the declared final structural state is the first Patch on descriptor-default HiDef SoundFont in Ready with every unrelated Patch and all sixteen tracks exact",
			"JSON serialization is deterministic and contains no debug-only pointer, timestamp, platform path, or nondeterministic map ordering",
			"two independent complete runs from freshly constructed identical fixtures produce byte-identical EventLog, StateTree, coverage, checkpoints, and report JSON with no excluded fields",
		]
			contributesTo: [
				{capability: "capability.observable_demo_scene", contribution: "packages the event log, state tree, checkpoints, and explicit coverage gaps for an LLM"},
				{capability: "capability.soundfont_preset_selection", contribution: "reports exact catalog-backed preset transitions alongside their control and audio consequences"},
			]
	}

	valueObjects: LiveDemoScene: {
			description: "a bounded paced plan of semantic navigation, parameter edits, targeted parameter-audio probes, one preset replacement, two engine replacements, fixture advancement, rejection probes, checkpoints, and note cleanup for the real standalone application"
		state: {
			name: "String"
			schemaVersion: "u32"
			minimumParameterDwell: "Duration"
				steps: "Vec<{input: SemanticAction | AppEvent | FixtureTick | Checkpoint | Finish, expectedTransition, editableParameterId?, structuralTransitionId?, semanticTransitionId?, requireAudibleObservation}>"
				expectedEditableParameters: "ordered unique identifiers derived from the focused first Patch's canonical editable targets, both PatchOutput descriptors, configured effect scalars, every MixerTrackParameter for all sixteen MixerTrackIds, and every GlobalParameters descriptor"
				expectedStructuralTransitions: "ordered [SoundFontPresetToNext, SoundFontToBraids, BraidsToDescriptorDefaultSoundFont] identities for the focused first fixture Patch"
				expectedSemanticTransitions: "ordered context, four-surface, Navigate/Adjust, two return-round-trip, valid-action, status, healthy-error, focus-recovery, and responsive-invariance identities"
		}
		invariants: [
				"construction begins only after AutomaticMidiTest installs the real Corridors of Time fixture Patches from exact catalog identities and freezes the expected editable-parameter and structural-transition sets before any live action is dispatched",
			"the plan derives the focused first Patch's envelope and editable instrument targets, both of its PatchOutput fields, its configured effect scalars, every track parameter across all sixteen MixerTrackIds, and every global parameter from production resolvers and typed descriptors; it contains no duplicate hand-maintained field list or engine/effect branch",
			"Patch-owned identifiers are exercised through PATCH Main or Utility while track-owned identifiers are exercised through MIXER Main or Inspector; no PatchId becomes a mixer track and no identifier receives duplicate credit",
			"the first Patch's configured Chorus Amount and Depth identifiers are exercised exactly once through PATCH after instrument controls; effect identity remains read-only and unconfigured Patches contribute no effect identifier",
			"every expected editable parameter has at least one planned accepted value change, one checkpoint, a minimum dwell of 500 ms, and an audible observation requirement; the checkpoint is bracketed by one semantic NoteOn before the edit and matching NoteOff afterward for the owning Patch, while global edits use the focused first Patch",
			"parameter-audio probes are ordinary bounded Patch-targeted Midi AppEvents through AppLoop, never direct AudioCommands; they establish schedule-independent signal while fixture advancement is frozen for exact-generation correlation, and they never earn editable-parameter coverage",
			"navigation, mode, surface, return, and adjustment steps contain SemanticActions and expected transitions only; MIDI/system steps contain AppEvents, and the scene contains no mutable AppState, SemanticGraphicalViewModel, GraphicalShellProjection, TextProjection, ParameterSnapshot, SoundFont engine, Chorus processor, mixer, audio buffer, UI widget, or device handle",
			"the semantic plan visits PatchMain, PatchUtility, MixerMain, and MixerInspector, round-trips both side-surface ReturnPaths, holds Navigate and Adjust visibly, and compares each projected valid-action set before frozen scalar coverage continues",
			"after frozen scalar coverage, the plan focuses the descriptor-derived Preset row and submits exactly one semantic Adjust Right to the next numerically ordered catalog entry, waits for Preparing, Activating, and Ready on a newer SoundFont graph revision, then returns focus to Engine, requests Braids, focuses the removable Preset row while Preparing, proves deterministic focus recovery on candidate commit, and returns to descriptor-default SoundFont through the same lifecycle",
			"each Ready structural transition is followed by Patch-targeted semantic MIDI and a finite nonzero target-output checkpoint before the next transition or cleanup; the first Patch's exact PostEffectConfig/effect layout remains present through every replacement while its tail may reset, and transition coverage cannot be credited by a label, constructed candidate, source audio, or unrelated Patch",
			"at least one planned boundary adjustment is rejected as ParameterAtBoundary and is followed by a valid accepted adjustment proving the live scene remains active",
			"Finish contains one Patch-targeted semantic all-notes-off Midi AppEvent for every installed Patch and no direct AudioCommand",
			"the live scene is test/demo support around the existing MIDI fixture and exposes no transport, sequencer, song, clip, timeline, recording, or playback-control product model",
			"the live scene injects no preparation failure, stale result, fabricated acknowledgement, manual graph, or direct worker action; exhaustive negative-path evidence remains in DemoScene while live composition uses ThreadedGraphPreparationWorker",
			"the healthy physical plan requires errors to be explicitly empty; deterministic headless coverage alone drives a typed Failed projection and recovery without treating an injected failure as physical acceptance",
		]
			contributesTo: [
				{capability: "capability.live_observable_demo", contribution: "declares the bounded human-paced scalar and engine-switching production-path scene without weakening the exhaustive headless scene"},
				{capability: "capability.asynchronous_engine_selection", contribution: "declares both successful directions and their lifecycle/audio checkpoints for the physical demo"},
				{capability: "capability.soundfont_preset_selection", contribution: "declares one visible and audible adjacent catalog choice through the production structural path"},
		]
	}

	valueObjects: LiveDemoCheckpoint: {
		description: "one immutable correlation between a planned live input, its canonical control transition, visible projection, emitted effects, and measured audio observation"
		state: {
			step: "usize"
			input: "SemanticAction | AppEvent"
			expectedTransition: "typed expected outcome and exact values captured before dispatch"
			outcome: "Accepted | Rejected"
			generation: "u64"
			stateHash: "String"
			projectedValue: "typed selected SemanticGraphicalViewModel, GraphicalShellProjection, nested TextProjection, and ParameterSnapshot value"
			focusPath: "FocusPath"
			interactionMode: "InteractionMode"
			returnPath: "Option<ReturnPath>"
			validActions: "Vec<ValidAction>"
			viewStatus: "typed semantic status"
			viewErrors: "typed semantic error list"
			parameterGeneration: "u64"
				graphRevision: "GraphRevision"
				structuralEditIntent: "Option<StructuralEditIntent>"
				engineSelectionStatus: "EngineSelectionStatus"
				activeCapabilityId: "CapabilityId"
				activeStructuralChoiceId: "Option<String>"
			emittedEffects: "bounded EventRecord effect descriptors"
			audioObservation: "AudioObservationSnapshot"
			audioPredicate: "typed parameter- or engine-specific predicate and measured result"
		}
		invariants: [
			"expectedTransition is computed and frozen before input dispatch; no actual state, projection, effect, or audio value can define its own expectation",
			"all control fields come from one production EventRecord and the canonical projections plus structural status for that record's accepted generation or unchanged rejected generation",
				"audioObservation is copied from the bounded callback-to-control port only after its sequence advances and its parameterGeneration matches the checkpoint generation; a structural-output checkpoint additionally requires Ready, the acknowledged active graphRevision, and the exact target capability or preset choice identity",
			"the checkpoint is constructed and serialized only on the control side and contains no device, callback, engine, mixer, window, or mutable-state handle",
		]
			contributesTo: [
				{capability: "capability.live_observable_demo", contribution: "is the canonical structured live checkpoint returned to the standalone output adapter"},
				{capability: "capability.soundfont_preset_selection", contribution: "correlates the requested preset identity with visible Ready state, graph revision, and target audio"},
			]
	}

	valueObjects: LiveDemoReport: {
		description: "the control-side structured checkpoints and final result of one live observable demo"
		state: {
			scene: "String"
			schemaVersion: "u32"
			complete: "bool"
			checkpoints: "Vec<LiveDemoCheckpoint>"
			eventLog: "EventLog"
			stateTree: "StateTree"
				coverage: "{expectedEditableParameters, exercisedEditableParameters, missingEditableParameters, unexpectedEditableParameters, expectedStructuralTransitions, exercisedStructuralTransitions, missingStructuralTransitions, unexpectedStructuralTransitions, expectedSemanticTransitions, exercisedSemanticTransitions, missingSemanticTransitions, unexpectedSemanticTransitions, expectedShellRegions, exercisedShellRegions, missingShellRegions, unexpectedShellRegions, shellContextsObserved, semanticSurfacesObserved, interactionModesObserved, returnPathsRoundTripped, focusRecoveriesObserved}"
				runtimeAudio: "{parsedSoundfontBanks, preparedInstruments, preparedPostEffects, soundfontPatches, braidsPatches, chorusPatches, alternatingCapabilities, initialGraphRevision, activeGraphRevision, structuralSwitches, readyCapabilities, readyPresetChoices, fallbacks, callbackAllocations, callbackDestructions}"
			summary: "String"
		}
		invariants: [
			"each checkpoint captures its expected transition before dispatch and then copies the actual outcome, generation, state hash, projected value, parameter generation, and emitted effects from the production EventRecord and canonical projections",
				"each accepted parameter checkpoint requires an AudioObservationSnapshot whose sequence advanced after dispatch, whose parameterGeneration equals the accepted generation, whose output is finite, and whose parameter-specific audible predicate passed while fixture audio was nonzero; each structural checkpoint additionally proves the correlated intent/status, increasing active graph revision, exact active capability or preset choice, and finite nonzero targeted output after acknowledgement",
				"complete is true only when every expected editable parameter including Chorus Amount and Depth changed, all three ordered structural transitions and every semantic transition completed, every expected Phase 1 shell region, both contexts, all four Phase 2 surfaces, Navigate and Adjust, two exact ReturnPath round trips, exact valid actions, structural status, explicitly empty healthy errors, focus recovery, and responsive focus invariance were observed through canonical semantic/shell frames, all missing and unexpected sets are empty, at least one accepted and one rejected EventRecord exist, every checkpoint agrees, no event records were dropped, the focused Patch is Ready on descriptor-default SoundFont and its descriptor-default preset with the exact Chorus config, all semantic all-notes-off events were accepted, a later audio observation reports zero active notes, and runtimeAudio reports one parsed bank, one prepared instrument per Patch and one Chorus instance after each rebuild, exact final engine/effect Patch counts and alternation, three switches, increasing revisions ending at StateTree graph revision, zero fallbacks, and zero callback allocation or destruction",
			"eventLog and stateTree are the existing canonical Control values, not live-demo copies; stateTree.generation equals the final checkpoint and EventLog chain endpoint",
			"the complete EventLog remains retained for deterministic report verification while interactive terminal output uses one compact LiveEventLogSummary containing lossless counts and canonical first/last chain endpoints",
			"summary is human-readable control-side text derived from the structured report after completion and is never constructed or printed in the audio callback",
		]
			contributesTo: [
				{capability: "capability.live_observable_demo", contribution: "packages coherent live checkpoints, exact coverage, final canonical state, and a readable summary"},
				{capability: "capability.soundfont_preset_selection", contribution: "requires exact ordered preset transition coverage and descriptor-default restoration"},
			]
	}

	valueObjects: LiveEventLogSummary: {
		description: "compact interactive evidence for a potentially large complete live EventLog"
		state: {
			schemaVersion: "u32"
			eventLogSchemaVersion: "u32"
			totalObserved: "u64"
			retainedRecords: "usize"
			droppedRecords: "u64"
			firstSequence: "Option<u64>"
			lastSequence: "Option<u64>"
			generationBefore: "Option<u64>"
			generationAfter: "Option<u64>"
			stateHashBefore: "Option<String>"
			stateHashAfter: "Option<String>"
			activeGraphRevision: "GraphRevision"
			lossless: "bool"
		}
		invariants: [
			"lossless is true exactly when droppedRecords is zero and totalObserved equals retainedRecords",
			"the first and last fields are copied from the retained canonical EventLog chain rather than recomputed from UI or audio state",
			"serialization and printing occur only on the control side after successful live completion",
		]
		contributesTo: [{capability: "capability.live_observable_demo", contribution: "keeps final interactive proof bounded without discarding the complete typed report journal"}]
	}

	valueObjects: InstrumentPart: {
		description: "one stable MIDI instrument identity discovered by the fixture"
		state: {
			index: "usize"
			name: "String"
			instrument: "SoundFontInstrument"
			assignedChannel: "MidiChannel"
		}
		invariants: [
			"one part exists for each distinct bank/program/percussion identity used by sounding events",
			"index is in 0..15 and assignedChannel equals index",
			"assignedChannel is unique among all InstrumentParts so simultaneously sounding Patches never share a render lane",
		]
		contributesTo: [
			{capability: "capability.automatic_test_midi", contribution: "defines the one-Patch-per-instrument and one-channel-per-Patch assignment"},
			{capability: "capability.instrument_capability_model", contribution: "retains SoundFontInstrument only as fixture source identity before provider conversion"},
			{capability: "capability.soundfont_audio", contribution: "carries the SoundFont preset required by the Patch"},
		]
	}

	ports: MidiEventSource: {
		direction: "inbound"
		contract: {
			prepare: "() -> Result<Vec<InstrumentPart>, MidiSourceError>"
			start: "()"
			poll: "(elapsed: Duration, output: &mut FixedEventBatch) -> Result<(), MidiSourceError>"
			finished: "() -> bool"
		}
		consumes: ["valueObject.Testing.InstrumentPart", "valueObject.Kernel.MidiMessage"]
		invariants: [
			"prepare and start run outside the audio callback",
			"poll appends due Patch-targeted MIDI messages in source order until caller-owned bounded storage is full and retains any remaining overdue messages for later polls without treating elapsed-time catch-up as an error",
			"the port exposes no seek, pause, record, loop, timeline, edit, song, clip, pattern, or transport operation",
		]
		contributesTo: [{capability: "capability.automatic_test_midi", contribution: "keeps automatic file input replaceable by later input adapters without adding a sequencer"}]
	}

	applicationServices: AutomaticMidiTest: {
		purpose: "install fixture Patches and dispatch due fixture MIDI through AppLoop"
			uses: [
				"port.Testing.MidiEventSource",
				"aggregate.Synth.Patch",
				"valueObject.Synth.CapabilityRegistry",
				"valueObject.Synth.EffectCapabilityRegistry",
				"valueObject.Synth.SoundFontPresetCatalog",
			"port.Synth.InstrumentCapabilityProvider",
			"port.Synth.EffectCapabilityProvider",
			"applicationService.Control.AppLoop",
			"valueObject.Testing.InstrumentPart",
		]
		operations: {
			initialize: {input: {}, output: {result: "Result<(), TestInputError>"}}
			start: {input: {}, output: {result: "Result<(), TestInputError>"}}
			tick: {input: {elapsed: "Duration"}, output: {result: "Result<(), TestInputError>"}}
		}
		meta: rules: [
			"initialize prepares the source and asks injected capability-neutral factories to create a schema-valid InstrumentConfig for each discovered part plus one effect.chorus PostEffectConfig on the first Patch only, assigns stable PatchIds plus default VoiceEnvelope and PatchOutput whose MixerTrackId is the deterministic part index modulo sixteen, and dispatches one InstallPatches AppEvent without configuring or starting an engine/effect",
				"production composition maps zero-based even parts to a HiDef SoundFont config by resolving the fixture's normalized numeric bank/program identity to the exact SoundFontPresetCatalog choice and odd parts to the default Braids config; this alternation exists only in the fixture adapter and the resulting Patch/rack path remains capability-polymorphic",
				"a missing or ambiguous fixture preset address is a typed initialization failure before Patch installation; authored labels, General MIDI names, nearest entries, first entries, or descriptor defaults are never used as identity fallback",
			"initialization rejects a missing instrument/effect provider, registry/provider mismatch, invalid config/slot, or factory failure before installation; it never substitutes a descriptor, config, preset, effect, asset, preparer, prepared processor, engine, or bypass",
			"start is accepted exactly once after StandaloneApplication has successfully built the complete initial PreparedGraph; failed graph preparation leaves the source stopped",
			"tick polls into reusable bounded storage and dispatches each item as AppEvent::Midi through AppLoop",
			"no transport state or playback controls are added to AppState",
		]
		contributesTo: [
			{capability: "capability.automatic_test_midi", contribution: "starts Corridors of Time automatically and sends all test input through the production reducer"},
				{capability: "capability.instrument_capability_model", contribution: "creates all fixture Patch configs through the installed capability provider"},
				{capability: "capability.soundfont_preset_selection", contribution: "resolves each fixture SoundFont identity to one exact stable catalog choice without fallback"},
				{capability: "capability.static_patch_effect", contribution: "installs exactly one canonical Chorus config on the first fixture Patch without preparing DSP"},
			{capability: "capability.prepared_engine_rack", contribution: "separates canonical Patch installation from later off-thread graph preparation and starts MIDI only after that graph is ready"},
			{capability: "capability.one_way_parameter_control", contribution: "uses the same AppEvent/AppState path as keyboard input"},
		]
	}

	applicationServices: ExhaustiveGuiDemo: {
		purpose: "exercise every current GUI input, semantic event, editable parameter, serialized property, and observable audio effect through production services"
		uses: [
			"valueObject.Testing.DemoScene",
			"valueObject.Testing.DemoSceneReport",
			"valueObject.Shell.WindowInput",
			"applicationService.Shell.KeyboardInputTranslator",
			"valueObject.Control.SemanticAction",
			"applicationService.Control.AppLoop",
			"applicationService.Testing.AutomaticMidiTest",
			"valueObject.Control.EventLog",
			"valueObject.Control.StateTree",
			"valueObject.Control.TopLevelContext",
				"valueObject.Control.InteractionState",
				"valueObject.Control.FocusPath",
				"valueObject.Control.ReturnPath",
				"valueObject.Control.SemanticGraphicalViewModel",
				"valueObject.Control.StructuralEditIntent",
				"valueObject.Control.PatchPageProjection",
				"valueObject.Control.GraphicalShellProjection",
			"valueObject.Kernel.MidiMessage",
			"valueObject.Synth.EffectCapabilityRegistry",
			"valueObject.Synth.PostEffectConfig",
			"aggregate.RealTime.PreparedPostEffectRack",
			"valueObject.RealTime.PatchEffectObservation",
			"port.RealTime.AudioBoundary",
			"port.RealTime.GraphPreparationWorker",
			"adapter.DeterministicGraphPreparationWorker",
			"applicationService.RealTime.StructuralGraphCoordinator",
			"applicationService.RealTime.AudioRenderer",
			"valueObject.Testing.EngineSelectionObservation",
		]
		operations: {
			run: {input: {scene: "DemoScene"}, output: {report: "Result<DemoSceneReport, DemoSceneError>"}}
		}
		meta: rules: [
			"begin after AutomaticMidiTest installs the real fixture Patches so the state tree contains immutable instrument/effect registries, every Patch identity/config/effect/output, and all sixteen persistent mixer-track parameter sets including empty tracks",
			"prove the registry contains instrument.soundfont.hidef and instrument.braids and the fixture alternates them in stable part order; every installed config matches its descriptor and unknown, duplicate, missing, undeclared, wrong-kind, non-finite, and out-of-range mutations fail without fallback or partial installation",
			"exercise all six SemanticAction variants and all eleven AppEvent variants: SelectContext, Navigate, Adjust, SetInteractionMode, EnterSurface, Return, InstallPatches, Midi, EnginePrepared, EnginePreparationFailed, and EngineActivationAcknowledged; both contexts, four surfaces, Navigate/Adjust, each direction, each Patch's declared MIDI kinds, and the complete canonical MIDI union are exact",
			"exercise every valid normalized WindowInput from its production-owned descriptor through KeyboardInputTranslator and prove each emits the exact expected SemanticAction or no action, then prove AppLoop maps each action to the exact AppEvent",
			"navigate the configured first Patch through read-only Chorus identity to Amount and Depth, edit both through K+A/D and K+S/W, prove exact state/page/text/tree/fixed-effect-snapshot values with scalar-only effects, and measure target-only pre/post difference plus stereo side energy before exact restoration",
			"render a focused test graph with two configured Chorus instances to prove independent delay/LFO/tail ownership while production fixture coverage retains only the first Patch slot",
				"drive Digit2 through KeyboardInputTranslator and AppLoop, prove the accepted InteractionState FocusPath, Navigate mode, and stable PatchId focus, then compare SemanticGraphicalViewModel, PatchPageProjection, and rendered PATCH text exactly against the focused Patch, PatchControlId, VoiceEnvelope descriptor, active CapabilityDescriptor, InstrumentConfig, and full registry choices for both SoundFont and Braids without capability-specific expected field lists; SoundFont ends with the exact authored Preset label while Braids ends at Release",
			"enter PatchUtility and MixerInspector from discriminating main paths, assert exact SurfaceRoot and ReturnPath projection, then Return to each exact origin with session/audio state unchanged",
			"enter and leave Adjust through SemanticAction around edits, and assert every projected valid-action set equals the reducer's accepted action set at bounds, disabled dependencies, side surfaces, and structural lifecycle states",
			"while a SoundFont-only Preset path is stored during a pending SoundFont-to-Braids change, commit the new schema and require reducer-owned next-before-previous visible-enabled sibling recovery for active, remembered, and return paths; resize-only composition preserves paths byte-for-byte",
				"from Engine drive bare S through every canonical ADSR row and descriptor-classified StructuralChoice row, verify exact nonwrapping focus, selected marker, selectedLine, unchanged session/audio values and graph revision, then use K+A/D for fine and K+S/W for coarse reversible edits on Attack, Decay, Sustain, and Release; compare exact canonical state, page/text/tree/snapshot values and require no AudioCommand or structural effect",
				"on the focused SoundFont Preset row, drive K+D to the next numerically ordered catalog choice; checkpoint Preparing with the source config/revision and audible source graph exact, reject another request as StructuralEditBusy, manually advance real preparation, require target-only assignment change, checkpoint Activating, publish/render/collect/acknowledge, then send targeted MIDI and measure finite nonzero output distinct from identical fresh source state before restoring the source choice",
			"on the focused PATCH Engine row, drive K+D to request SoundFont to Braids, checkpoint Preparing with the source config/revision and audible source graph exact, reject another request as StructuralEditBusy, manually advance real preparation, accept the candidate, checkpoint Activating, publish/render/collect/acknowledge, then send targeted MIDI and measure finite nonzero Braids output",
			"while the first request is Preparing navigate to one ADSR row and edit it against the source revision, then after candidate commit edit another during Activating against the target revision; prove the prepared graph refreshes the first value, the activated target consumes both latest values, the old source remains finite, and neither edit publishes structural work",
			"drive K+A through the same path to descriptor-default HiDef SoundFont and measure finite nonzero engine-managed output; then run one controlled worker failure plus early, stale, and mismatched outcome/acknowledgement probes and prove exact source preservation, no publication or fallback, and later valid recovery",
				"PATCH Main Navigate Left, Navigate Up at Engine, Navigate Down at the final descriptor-derived row, and Adjust Up/Down on structural rows are ActionUnavailableInContext; Navigate Right enters Utility and Left or Return restores its origin; adjacent engine or preset selection beyond its declared choice boundary is StructuralSelectionUnavailable, and every rejection leaves generation, hash, config, lifecycle, graph revision, and audio-command count exact and accepts a later valid action",
			"prove context-only acceptance advances coherent serialization, GraphicalShellProjection, TextProjection, StateTree, and ParameterSnapshot generation while retaining byte-identical session values, parameter values, active GraphRevision, prepared ownership, routing, and rendered audio",
			"exercise every target from the canonical Patch Main and Utility resolvers for every installed Patch plus all six MixerTrackParameter classes for every one of sixteen tracks and every global target through their normal GUI paths; assert exact state/text/snapshot values, shared-track or target-only measured behavior as applicable, and exact equality of every unrelated Patch, track, and global value",
			"before global wet-parameter probes, make at least two Patches sound, route them through known tracks, and establish nonzero track ReverbSend and DelaySend through the same GUI/reducer path; assert nonzero reverb and delay input energy at GlobalEffectsProcessor, then compare each typed GlobalParameters field from identical reset effect state",
			"the faithful effects observer may inspect and forward the supplied reverbInput and delayInput but may never synthesize wet excitation from dry output, bypass post-gate track sends, add report-only coverage, or mark an effect exercised merely because time-varying tails changed",
			"select each typed GlobalParameters field and perform reversible fine and coarse edits; prove the exact selected value and complete expected mix response while Patch identity and unrelated values remain stable, then restore all global values and both sends to the captured baseline",
			"cover Track-to-Track across T00 and T0F, Track-to-GLOBAL, GLOBAL-to-Track, PatchMain-to-PatchUtility, parameter boundaries, surface return, and selected-line projection movement in both directions",
			"explicitly prove horizontal MIXER navigation preserves Level/Pan/Mute/Solo row across all sixteen fixed tracks independently of routed Patch count or schema, and that global controls remain distinct from the track bank",
				"for every numeric target returned by the scalar resolver and every global value, drive the selected value to both declared boundaries and record ParameterAtBoundary; for every structural Choice use its first/last entries to prove nonwrapping StructuralSelectionUnavailable, then prove a valid subsequent edit succeeds",
			"derive the expected surface from the production-owned installed capability/parameter descriptors, other typed descriptors, and discovered serialization leaves; require exact expected-versus-observed set equality and report both missing and unexpected identifiers",
			"observe and compare every current StateTree value, SemanticGraphicalViewModel focus/mode/return/action/status/error/surface/control value, GraphicalShellProjection region/label/diagnostic value, TextProjection line/value/selection marker, and ParameterSnapshot value against the same accepted AppState generation; property existence or a nonempty body alone is insufficient",
			"verify all publicly reachable EventRejection outcomes in the scene and cover internal-only rejection variants with a table-driven reducer test; no rejection terminates later scene steps",
			"for every scene step compare the complete EventRecord source, tagged input payload, outcome/rejection, generations, state hashes, emitted-event payloads, parameter generation, projection hash, and selected line against an oracle fixed before dispatch",
			"exercise Startup, Keyboard, AutomaticMidi, DemoScene, Worker, and System EventSource tags through their real dispatch entry points and require each source's exact payload/outcome in EventRecord coverage",
			"schema discovery unions discriminating EventRecords for every input, outcome, rejection, and emitted-event tag; table cases removing one expected leaf or inserting one unexpected leaf in EventRecord and EventLog JSON must both make exact schema equality fail",
			"discover ParameterSnapshot paths and exact values from the actual StateProjector output/getters and StateTree parameters projection, never by serializing or echoing the expected descriptor itself",
			"exercise the separate AudioCommand::AllNotesOff renderer command in addition to PatchMidi(MidiMessageKind::AllNotesOff), and require both unique coverage identifiers",
			"each Tick calls AutomaticMidiTest.tick with the declared deterministic elapsed duration, records every resulting fixture MIDI event through AppLoop, and asserts the exact EventRecord and audio consequence; an ignored elapsed value or render-only tick fails",
			"audio comparison uses discriminating stems, nonzero effect inputs, paired renders from identical engine/effect state, and measured finite output; construction, success strings, dry-derived fake excitation, unrelated tail evolution, or a changed master buffer alone are not evidence",
			"restore every reversible Patch, PatchOutput, track, send, global, selection, and context value to its exact captured baseline; finish with the declared first-Patch descriptor-default SoundFont config, Ready lifecycle, matching active GraphRevision, deterministic projections, and every unrelated Patch plus all sixteen tracks exact",
			"render real SoundFont and Braids Patches simultaneously, require distinct nonzero finite isolated stems, and measure that Model, Timbre, Color, and all four ADSR controls affect only their declared target through the production reducer and renderer",
			"run the complete scene twice from fresh identical services and require byte-identical EventLog, StateTree, coverage, checkpoints, and report JSON; no timestamp, map-order, pointer, or first-run effect tail may be excluded",
		]
		validations: [
			{id: "validation.service.exhaustive_gui_demo", kind: "integration", command: ["cargo", "test", "--test", "exhaustive_demo_scene", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE exhaustive_demo_scene passed"}], description: "the generated scene covers every typed current input/event/property/parameter, compares exact state/projection values, records accepted and rejected transitions, and restores its baseline"},
			{id: "validation.service.exhaustive_gui_schema_surface", kind: "integration", command: ["cargo", "test", "--test", "schema_surface", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE schema_surface passed"}], description: "typed production descriptors and discovered serialized leaves are exactly equal in both directions"},
			{id: "validation.service.exhaustive_gui_effects", kind: "test", command: ["cargo", "test", "faithful_effects_nonzero_sends_and_baseline_restoration"], description: "wet controls are measured with nonzero routed sends, identical effect state, no dry bypass, and exact baseline restoration"},
		]
		contributesTo: [
			{capability: "capability.observable_demo_scene", contribution: "runs exhaustive stateful GUI and event coverage through production seams"},
			{capability: "capability.instrument_capability_model", contribution: "proves registry/config serialization, generic projection, and explicit no-fallback rejection through production seams"},
			{capability: "capability.one_way_parameter_control", contribution: "proves all current editable values use the one reducer and projection path"},
			{capability: "capability.schema_driven_patch_page", contribution: "proves both direct page events, exact generic Patch projection, stable Engine-plus-ADSR focus, canonical ADSR edits, typed unsupported-action rejection, and audio-neutral focus/context switching"},
				{capability: "capability.per_voice_envelope", contribution: "proves all four PATCH ADSR controls reach only the canonical focused Patch and both real per-voice renderers"},
			{capability: "capability.soundfont_preset_selection", contribution: "proves exact catalog-backed focus, ordered choice, correlated structural replacement, target audio, failures, boundaries, and restoration"},
			{capability: "capability.static_patch_effect", contribution: "proves schema-derived effect focus/editing, scalar publication, ordering, isolation, independent state, structural preservation, and audible output"},
			{capability: "capability.asynchronous_engine_selection", contribution: "proves both engine directions, pending/busy/failure/stale states, complete activation, off-callback retirement, and targeted audible output through production seams"},
			{capability: "capability.global_mix", contribution: "measures every current Patch and global mix parameter case"},
			{capability: "capability.realtime_execution", contribution: "observes parameter and command effects through the real-time boundary"},
			{capability: "capability.semantic_graphical_view_model", contribution: "proves exhaustive action, focus, mode, surface, return, valid-action, status/error, recovery, and projection behavior"},
		]
	}

	applicationServices: LiveDemoRunner: {
		purpose: "advance the human-observable scene on window ticks and correlate canonical control projections with bounded audio observations"
		uses: [
			"valueObject.Testing.LiveDemoScene",
			"valueObject.Testing.LiveDemoCheckpoint",
			"valueObject.Testing.LiveDemoReport",
			"applicationService.Testing.AutomaticMidiTest",
			"applicationService.Control.AppLoop",
			"valueObject.Control.SemanticAction",
			"valueObject.Control.AppEvent",
			"valueObject.Control.EventLog",
			"valueObject.Control.StateTree",
			"valueObject.Control.GraphicalShellProjection",
			"valueObject.Control.SemanticGraphicalViewModel",
			"valueObject.Control.FocusPath",
			"valueObject.Control.ReturnPath",
			"valueObject.Control.ValidAction",
			"valueObject.Shell.ShellFrameObservation",
				"valueObject.Control.EngineSelectionStatus",
				"valueObject.Control.StructuralEditIntent",
			"valueObject.Mixer.MixerTrackId",
			"valueObject.Mixer.PatchOutput",
			"valueObject.Mixer.MixerTrackParameter",
			"valueObject.Mixer.MixerTrackParameters",
			"valueObject.Mixer.MixerState",
			"valueObject.Synth.VoiceEnvelope",
			"valueObject.Synth.CapabilityDescriptor",
			"valueObject.Mixer.GlobalParameters",
			"port.RealTime.AudioObservation",
			"valueObject.RealTime.AudioObservationSnapshot",
		]
		operations: {
			start: {input: {scene: "LiveDemoScene"}, output: {result: "Result<(), LiveDemoError>"}}
			advance: {input: {elapsed: "Duration"}, output: {checkpoint: "Result<Option<LiveDemoCheckpoint>, LiveDemoError>"}}
			observeShellFrame: {input: {observation: "ShellFrameObservation"}, output: {result: "Result<(), LiveDemoError>"}}
			completedReport: {input: {}, output: {report: "Option<&LiveDemoReport>"}}
		}
		meta: rules: [
			"start runs on the control thread after the fixture Patches, audio observation handles, bounded EventLog capacity, physical audio stream, and window application are prepared; it never opens a device or mutates canonical state",
			"advance is called by the real window tick with monotonic elapsed time and never sleeps or blocks the UI thread; it advances AutomaticMidiTest through its existing tick operation and dispatches at most one due autonomous SemanticAction through AppLoop.dispatchAction or one MIDI/system AppEvent through dispatchFrom with EventSource::DemoScene",
			"before each runner advance the owning standalone tick calls AppLoop.advanceStructural exactly once, allowing the injected production worker and structural coordinator to progress without blocking; LiveDemoRunner observes canonical results and never submits, polls, advances, joins, or owns the worker or a graph directly",
			"before each dispatch compute the exact expected generation, selected parameter value, FocusPath, InteractionMode, ReturnPath, valid actions, status/errors, StateTree value, SemanticGraphicalViewModel value, GraphicalShellProjection value, nested TextProjection value, ParameterSnapshot value, outcome, and emitted effects from the captured prior canonical state and owning typed descriptors",
			"for every Patch-owned envelope coverage instance, dispatch PATCH Navigate and Adjust events in canonical descriptor order, require the marked row and selectedLine to follow focusedControlId, and never expose that identifier as a mixer-track control",
			"exercise Patch trim and output track only through PatchUtility, then exercise all sixteen fixed MixerTrackIds through MixerMain and MixerInspector; route two sounding Patches to one track before its Level checkpoint and prove the fader controls their combined meter/output while every unrelated track remains exact",
			"before every accepted scalar checkpoint dispatch the plan's Patch-targeted semantic NoteOn probe through AppLoop, then dispatch the parameter edit on a later tick; after exact-generation capture and visible dwell dispatch the matching semantic NoteOff before advancing to another checkpoint, so sparse fixture scheduling cannot strand a predicate",
			"after an accepted semantic transition or edit wait until the canonical SemanticGraphicalViewModel and GraphicalShellProjection have been available across a rendered frame; scalar edits additionally dwell at least 500 ms and wait for AudioObservation at the exact accepted ParameterSnapshot generation before returning one LiveDemoCheckpoint",
			"observeShellFrame runs only on control ownership after the production adapter paints; it correlates observation generation, state hash, context, named rectangles, and labels with the canonical GraphicalShellProjection and credits a PATCH or MIXER shell frame only while the bounded AudioObservation reports finite nonzero production-path fixture output; physical composition supplies that observation from the device callback and the deterministic harness supplies it from the same renderer and transport",
			"shell coverage is the exact five declared regions, two top-level contexts, four semantic surfaces, two admitted modes, two return round trips, and frozen semantic transition set compared in both directions; planned labels, projected geometry, helper-only result, stale frame, invalid valid-action set, hidden/overlapping region, or silent audio earns no credit",
			"record progress only for autonomous scene dispatch, exact-generation checkpoint capture, engine lifecycle advancement, or completed cleanup; ten seconds without one of those milestones returns a typed stage-specific LiveDemoError and 120 seconds total returns a typed whole-run timeout instead of waiting indefinitely",
			"audible predicates use actual finite observation fields from the physical render path: mixer/global edits observe their owned signal stages, ADSR edits observe envelope timing/level, and Braids Model/Timbre/Color observe waveform or energy; fixture timing is recorded so unrelated musical evolution cannot be presented as the parameter consequence",
			"a rejected event is read from the existing EventLog, leaves generation and all projections unchanged, emits no effects, does not close the window, and does not skip the following valid scene step",
				"after scalar coverage the runner focuses the descriptor-derived SoundFont Preset row and dispatches one planned adjacent choice, then returns PATCH focus to Engine and dispatches the two planned engine adjustments through AppLoop; while SoundFont-to-Braids prepares it moves focus to the removable Preset row, then requires committed reducer recovery to the next surviving visible enabled sibling; it holds later scene actions while each request is pending and emits checkpoints only after the same request progresses through canonical Preparing, Activating, and Ready with exact target identities and increasing graph revisions",
				"after each Ready acknowledgement the runner dispatches targeted semantic MIDI for the selected Patch and waits for a newer matching-generation finite nonzero AudioObservation before crediting that structural transition; labels, candidate construction, source audio, or unrelated stems cannot satisfy the checkpoint",
				"expected and exercised editable-parameter and ordered structural-transition identifiers are each compared in both directions; an added, removed, duplicated, unmodified, unprojected, inaudible, unacknowledged, out-of-order, or unexpected item makes the report incomplete",
			"completion dispatches Patch-targeted MidiMessageKind::AllNotesOff AppEvents through AppLoop for every installed Patch, waits for a newer AudioObservationSnapshot with zero active notes, captures the final EventLog and StateTree, exposes one completed LiveDemoReport, and then performs no more actions",
			"the runner owns no window lifecycle and never requests window close itself; after completedReport exposes its inert final report, the owning StandaloneApplication consumes that report and requests close on the same window tick",
			"the runner never calls AppState.apply directly, edits the immutable capability registry or Patch instrument config, edits a projection or report to manufacture agreement, publishes ParameterSnapshot, AudioCommand, or PreparedGraph directly, invokes PreparedInstrument, PreparedEngineRack, or MixEngine directly, writes an audio buffer, prints output, or logs from the callback",
		]
		validations: [
				{id: "validation.service.live_demo_runner", kind: "integration", command: ["cargo", "test", "live_demo", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE live_demo_scene passed"}], description: "the deterministic-clock integration harness and standalone composition tests drive the live runner through production events, schedule-independent semantic parameter probes, one preset and both engine transitions, lifecycle/revision checkpoints, targeted render observations, rejection recovery, exact scalar and structural coverage, all-notes-off, inert report completion, and controlled no-audio and whole-run stalls that close with typed timeouts without giving the runner window or worker ownership"},
		]
		contributesTo: [
			{capability: "capability.live_observable_demo", contribution: "orchestrates the paced real-window scene and returns its coherent control-side evidence"},
			{capability: "capability.one_way_parameter_control", contribution: "routes autonomous actions through the same AppLoop as keyboard and fixture input"},
			{capability: "capability.realtime_execution", contribution: "reads only bounded generation-tagged callback observations"},
			{capability: "capability.prepared_engine_rack", contribution: "observes the same rack-backed renderer without receiving graph ownership or preparation operations"},
				{capability: "capability.asynchronous_engine_selection", contribution: "observes both production-path replacements through canonical lifecycle, revision, and target-audio evidence"},
			{capability: "capability.soundfont_preset_selection", contribution: "drives one adjacent authored preset through that same lifecycle and measures the exact target"},
			{capability: "capability.semantic_graphical_view_model", contribution: "drives and observes the cumulative physical semantic focus, mode, surface, return, valid-action, status/error, and recovery scene"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "drives Patch output, shared-track, six-class control, fixed-meter, and teardown evidence through the newest physical scene"},
		]
	}

	applicationServices: BehavioralMutationHarness: {
		purpose: "run fast verification-only healthy and single-mutant cases through the production control, routing, serialization, and render seams"
		uses: [
			"applicationService.Shell.KeyboardInputTranslator",
			"applicationService.Control.AppLoop",
			"domainService.Control.StateProjector",
			"valueObject.Control.StateTree",
			"port.RealTime.AudioBoundary",
			"applicationService.RealTime.AudioRenderer",
			"domainService.Mixer.MixEngine",
			"port.Mixer.GlobalEffectsProcessor",
			"applicationService.Testing.ExhaustiveGuiDemo",
		]
		operations: {
			run: {input: {case: "DroppedAdjustment | CrossTrackParameterLeak | PatchMisroute | OmittedStateTreeLeaf | DryToWetBypass | ZeroRenderer", mutantEnabled: "bool"}, output: {observation: "BehavioralMutationObservation", exitCode: "0 | 1"}}
		}
		meta: rules: [
			"healthy and mutant executions use the same deterministic fixture, inputs, assertions, marker, JSON schema, and production services; the mutant execution changes exactly one named seam",
			"DroppedAdjustment suppresses exactly one translated SemanticAction::Adjust before AppLoop.dispatchAction; it does not edit the mapped AppEvent, EventLog, or coverage report",
			"CrossTrackParameterLeak applies one edited track's MixerTrackParameters to exactly one different MixerTrackId at the ParameterSnapshot-to-MixEngine ownership seam while accepted AppState, PatchOutput values, published track ids, and stems remain otherwise correct; it does not edit StateTree, EventLog, measured energies, or observation fields",
			"PatchMisroute rewrites exactly one accepted PatchMidi command to a different installed PatchId at the command-routing seam before engine dispatch; it does not edit measured stems or observation fields",
			"OmittedStateTreeLeaf removes exactly one required typed leaf while constructing the serialized StateTree before coverage/property discovery; it does not append a fake missing identifier after report construction",
			"DryToWetBypass uses a nonzero dry signal as wet excitation while both supplied effect inputs are exactly zero at the GlobalEffectsProcessor seam; paired healthy and mutant renders begin from identical reset effect state, and the mutant does not edit input-energy or output-delta measurements",
			"ZeroRenderer clears the caller-owned audio buffer immediately after the production AudioRenderer render path and before measurement; it does not override the reported peak or completion flag",
			"every mutant emits exactly one schema-valid CREST_MUTATION_OBSERVATION describing actual downstream measurements, then exits with status 1; every matching healthy case emits the same schema and exits 0",
			"the harness is verification-only and exposes no mutation switch, alternate engine, alternate routing mode, or debug behavior to the interactive crest-synth application",
		]
		validations: [{id: "validation.service.behavioral_mutation_harness", kind: "integration", command: ["cargo", "test", "--test", "behavioral_mutation_harness", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE behavioral_mutation_harness passed"}], description: "all six isolated seam mutants alter only their named seam and produce measured falsifying observations without report tampering"}]
		contributesTo: [
			{capability: "capability.observable_demo_scene", contribution: "makes the exhaustive proof independently falsifiable at six production seams"},
			{capability: "capability.one_way_parameter_control", contribution: "proves a dropped adjustment, cross-track parameter leak, and Patch MIDI misroute cannot masquerade as accepted behavior"},
			{capability: "capability.global_mix", contribution: "proves cross-track leakage, dry-to-wet bypass, zeroed render output, and incorrect Patch routing are detected by causal measurements"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "falsifies track-parameter isolation independently from Patch stem identity"},
		]
	}
}

project: adapters: CorridorsMidiEventSource: {
	implements: "port.Testing.MidiEventSource"
	layer: "infrastructure"
	profile: {kind: "device_input", medium: "standard-midi-file"}
	meta: {
		framework: "midly"
		rules: [
			"expect exactly ./midi/Corridors of Time - Chrono Trigger.mid and fail clearly when it is missing or malformed",
			"parse the complete SMF and build private elapsed-time test events in prepare; no file access occurs after start",
			"track bank-select MSB/LSB and program changes, treat MIDI channel 10 as percussion, and attach each sounding note pair to its stable instrument identity",
			"create one InstrumentPart per identity in first-sounding order and assign part N to unique MidiChannel N; return a clear prepare error rather than reuse a channel if the fixture contains more than 16 sounding identities",
			"target every emitted message at its InstrumentPart/Patch and rewrite its channel to assignedChannel",
			"start at elapsed zero automatically, run once, and stop at end; do not expose transport, seeking, looping, recording, editing, or public sequence types",
		]
	}
	validations: [{id: "validation.adapter.corridors_midi_event_source", kind: "test", command: ["cargo", "test", "corridors_midi_event_source"], description: "the real fixture discovers multiple instruments, keeps note pairs together, assigns a unique channel to every Patch, rejects channel exhaustion, and emits due bounded events"}]
	contributesTo: [
		{capability: "capability.automatic_test_midi", contribution: "implements the fixed automatic Corridors of Time test input"},
		{capability: "capability.soundfont_audio", contribution: "provides the bank/program/percussion identity used to configure every Patch"},
		{capability: "capability.soundfont_preset_selection", contribution: "provides normalized numeric source identities for exact catalog resolution without supplying display names"},
	]
}
