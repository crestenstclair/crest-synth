package crestsynth

project: contexts: Testing: {
	purpose: "automatic MIDI input used to exercise the synth through production ports"
	meta: rules: [
		"this context is input test support, not a sequencing or transport domain",
		"its timing and MIDI-file parsing types are private implementation details",
	]

	valueObjects: DemoScene: {
		description: "a deterministic sequence of normalized GUI inputs, MIDI probes, ticks, and immutable checkpoints"
		state: {
			name: "String"
			schemaVersion: "u32"
			steps: "Vec<WindowInput | MidiProbe | Tick | Checkpoint>"
			surfaceDescriptor: "typed WindowInput kind/key, AppEvent, Direction, MidiMessageKind, editable-parameter, rejection, emitted-effect, and serialized-leaf descriptors from production owners"
			rejectionDescriptor: "typed unique EventRejection cases partitioned into Scene and ReducerTable reachability"
			expectedCoverage: "the exact normalized identifier set derived from surfaceDescriptor plus installed Patch identities"
		}
		invariants: [
			"the exhaustive scene is derived from typed descriptors owned beside WindowInput, the production enums, parameter schemas, emitted effects, and serializers plus the installed fixture Patch list; it never defines a second hand-maintained list of GUI inputs or field-name strings",
			"a contract test discovers the serialized EventLog, EventRecord, StateTree, TextProjection, and ParameterSnapshot leaf paths and requires exact bidirectional set equality with surfaceDescriptor, so an added, removed, renamed, duplicated, or unexercised item fails",
			"descriptor uniqueness is asserted before converting to sets, and expectedCoverage is frozen before the first event so actual post-state, discovered output, or coverage observations can never define their own expected values",
			"every expected state value is computed before dispatch from the captured baseline plus the typed owner descriptor's bound and step; it is never copied from the actual post-dispatch StateTree, TextProjection, ParameterSnapshot, or rendered audio",
			"every GUI adjustment step enters through KeyboardInputTranslator and every semantic input enters through AppLoop.dispatch",
			"ticks use deterministic elapsed durations and no wall clock, native window, physical audio device, or random input",
			"the scene is test/demo support and exposes no transport, playback, arrangement, recording, or editing feature to the product domain",
		]
		contributesTo: [{capability: "capability.observable_demo_scene", contribution: "declares the exhaustive deterministic control-surface exercise"}]
	}

	valueObjects: DemoSceneReport: {
		description: "the complete machine-readable result of one exhaustive GUI demo run"
		state: {
			scene: "String"
			complete: "bool"
			eventLog: "EventLog"
			initialStateTree: "StateTree"
			finalStateTree: "StateTree"
			coverage: "{expected, exercised, missing, unexpected} grouped by normalized GUI inputs, events, directions, MIDI kinds, editable parameters, serialized properties, rejections, projections, and audio effects"
			checkpoints: "Vec<{step, expectedStateValues, actualStateValues, expectedProjectionValues, actualProjectionValues, stateHash, generation, selectedLine, parameterGeneration, audioMeasurement, reverbInputEnergy, delayInputEnergy}>"
		}
		invariants: [
			"complete is true only when expected and exercised identifiers are exactly equal in both directions, missing and unexpected are empty in both report and EventLog coverage, the event journal dropped no records, and all checkpoints agree",
			"the final tree is exactly the last accepted event state and the last EventRecord hash/generation chain endpoint",
			"each checkpoint compares exact typed state and projection values rather than checking only property presence, nonempty text, generation identity, or a changed aggregate buffer",
			"after every reversible probe the selected parameter, all unrelated parameters, effect sends, selection, and projection equal the captured baseline exactly; generation and journal history are the only permitted differences",
			"JSON serialization is deterministic and contains no debug-only pointer, timestamp, platform path, or nondeterministic map ordering",
			"two independent complete runs from freshly constructed identical fixtures produce byte-identical EventLog, StateTree, coverage, checkpoints, and report JSON with no excluded fields",
		]
		contributesTo: [{capability: "capability.observable_demo_scene", contribution: "packages the event log, state tree, checkpoints, and explicit coverage gaps for an LLM"}]
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
			"poll appends due Patch-targeted MIDI messages to caller-owned bounded storage",
			"the port exposes no seek, pause, record, loop, timeline, edit, song, clip, pattern, or transport operation",
		]
		contributesTo: [{capability: "capability.automatic_test_midi", contribution: "keeps automatic file input replaceable by later input adapters without adding a sequencer"}]
	}

	applicationServices: AutomaticMidiTest: {
		purpose: "install fixture Patches and dispatch due fixture MIDI through AppLoop"
		uses: [
			"port.Testing.MidiEventSource",
			"aggregate.Synth.Patch",
			"port.Synth.SoundFontEngine",
			"applicationService.Control.AppLoop",
			"valueObject.Testing.InstrumentPart",
		]
		operations: {
			initialize: {input: {}, output: {result: "Result<(), TestInputError>"}}
			tick: {input: {elapsed: "Duration"}, output: {result: "Result<(), TestInputError>"}}
		}
		meta: rules: [
			"initialize prepares the source, assigns stable PatchIds and default ChannelParameters, configures exactly one Patch per InstrumentPart through SoundFontEngine, dispatches one InstallPatches AppEvent, then starts the source immediately",
			"tick polls into reusable bounded storage and dispatches each item as AppEvent::Midi through AppLoop",
			"no transport state or playback controls are added to AppState",
		]
		contributesTo: [
			{capability: "capability.automatic_test_midi", contribution: "starts Corridors of Time automatically and sends all test input through the production reducer"},
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
			"applicationService.Control.AppLoop",
			"applicationService.Testing.AutomaticMidiTest",
			"valueObject.Control.EventLog",
			"valueObject.Control.StateTree",
			"valueObject.Kernel.MidiMessage",
			"port.RealTime.AudioBoundary",
			"applicationService.RealTime.AudioRenderer",
		]
		operations: {
			run: {input: {scene: "DemoScene"}, output: {report: "Result<DemoSceneReport, DemoSceneError>"}}
		}
		meta: rules: [
			"begin after AutomaticMidiTest installs the real fixture Patches so the state tree contains every current Patch identity and parameter set",
			"exercise InstallPatches, Navigate, Adjust, and Midi; Navigate and Adjust each exercise Up, Down, Left, and Right; for every installed Patch, MIDI probes cover note-on, note-off, control-change, program-change, channel-pressure, pitch-bend, and PatchMidi all-notes-off semantics with exact channel/data bytes",
			"exercise every valid normalized WindowInput from its production-owned descriptor through KeyboardInputTranslator and prove each emits the exact expected AppEvent or no event",
			"for every installed Patch select each typed ChannelParameters field and perform reversible fine and coarse edits through GUI inputs; at every step assert the exact expected bounded value, exact selected line/text value, exact ParameterSnapshot value, and exact equality of every unrelated Patch/global value",
			"before global wet-parameter probes, make at least two Patches sound and establish nonzero reverbSend and delaySend through the same GUI/reducer path; assert nonzero reverb and delay input energy at GlobalEffectsProcessor, then compare each typed GlobalParameters field from identical reset effect state",
			"the faithful effects observer may inspect and forward the supplied reverbInput and delayInput but may never synthesize wet excitation from dry output, bypass Patch sends, add report-only coverage, or mark an effect exercised merely because time-varying tails changed",
			"select each typed GlobalParameters field and perform reversible fine and coarse edits; prove the exact selected value and complete expected mix response while Patch identity and unrelated values remain stable, then restore all global values and both sends to the captured baseline",
			"cover Patch-to-Patch, Patch-to-GLOBAL, GLOBAL-to-Patch, parameter wrap, section wrap, and selected-line projection movement in both directions",
			"explicitly prove the differing parameter-count clamp: GLOBAL parameter indexes 4, 5, and 6 each move to Patch parameter index 3, while Patch index 3 moves to GLOBAL index 3; do not infer this from generic section-wrap coverage",
			"for each of gainDb, pan, reverbSend, delaySend, masterGainDb, reverbRoomSize, reverbDamping, reverbReturn, delayMilliseconds, delayFeedback, and delayReturn, drive the selected value to its typed lower and upper boundary, record ParameterAtBoundary as a nonfatal unchanged transition at each boundary, then prove a valid subsequent edit succeeds",
			"derive the expected surface from the production-owned typed descriptors and discovered serialization leaves; require exact expected-versus-observed set equality and report both missing and unexpected identifiers",
			"observe and compare every current StateTree value, TextProjection line/value/selection marker, and ParameterSnapshot value against the same accepted AppState generation; property existence or a nonempty body alone is insufficient",
			"verify all publicly reachable EventRejection outcomes in the scene and cover internal-only rejection variants with a table-driven reducer test; no rejection terminates later scene steps",
			"for every scene step compare the complete EventRecord source, tagged input payload, outcome/rejection, generations, state hashes, emitted-event payloads, parameter generation, projection hash, and selected line against an oracle fixed before dispatch",
			"exercise Startup, Keyboard, AutomaticMidi, DemoScene, and System EventSource tags through their real dispatch entry points and require each source's exact payload/outcome in EventRecord coverage",
			"schema discovery unions discriminating EventRecords for every input, outcome, rejection, and emitted-event tag; table cases removing one expected leaf or inserting one unexpected leaf in EventRecord and EventLog JSON must both make exact schema equality fail",
			"discover ParameterSnapshot paths and exact values from the actual StateProjector output/getters and StateTree parameters projection, never by serializing or echoing the expected descriptor itself",
			"exercise the separate AudioCommand::AllNotesOff renderer command in addition to PatchMidi(MidiMessageKind::AllNotesOff), and require both unique coverage identifiers",
			"each Tick calls AutomaticMidiTest.tick with the declared deterministic elapsed duration, records every resulting fixture MIDI event through AppLoop, and asserts the exact EventRecord and audio consequence; an ignored elapsed value or render-only tick fails",
			"audio comparison uses discriminating stems, nonzero effect inputs, paired renders from identical engine/effect state, and measured finite output; construction, success strings, dry-derived fake excitation, unrelated tail evolution, or a changed master buffer alone are not evidence",
			"restore every reversible parameter, send, selection, text projection, and parameter projection to its exact captured baseline so the final StateTree is deterministic while generation and EventLog still prove every transition",
			"run the complete scene twice from fresh identical services and require byte-identical EventLog, StateTree, coverage, checkpoints, and report JSON; no timestamp, map-order, pointer, or first-run effect tail may be excluded",
		]
		validations: [
			{kind: "integration", command: ["cargo", "test", "--test", "exhaustive_demo_scene", "--", "--nocapture"], assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "CREST_ACCEPTANCE exhaustive_demo_scene passed"}], description: "the generated scene covers every typed current input/event/property/parameter, compares exact state/projection values, records accepted and rejected transitions, and restores its baseline"},
			{kind: "integration", command: ["cargo", "test", "--test", "schema_surface", "--", "--nocapture"], assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "CREST_ACCEPTANCE schema_surface passed"}], description: "typed production descriptors and discovered serialized leaves are exactly equal in both directions"},
			{kind: "test", command: ["cargo", "test", "faithful_effects_nonzero_sends_and_baseline_restoration"], description: "wet controls are measured with nonzero routed sends, identical effect state, no dry bypass, and exact baseline restoration"},
		]
		contributesTo: [
			{capability: "capability.observable_demo_scene", contribution: "runs exhaustive stateful GUI and event coverage through production seams"},
			{capability: "capability.one_way_parameter_control", contribution: "proves all current editable values use the one reducer and projection path"},
			{capability: "capability.global_mix", contribution: "measures every current Patch and global mix parameter case"},
			{capability: "capability.realtime_execution", contribution: "observes parameter and command effects through the real-time boundary"},
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
			run: {input: {case: "DroppedAdjustment | CrossPatchParameterLeak | PatchMisroute | OmittedStateTreeLeaf | DryToWetBypass | ZeroRenderer", mutantEnabled: "bool"}, output: {observation: "BehavioralMutationObservation", exitCode: "0 | 1"}}
		}
		meta: rules: [
			"healthy and mutant executions use the same deterministic fixture, inputs, assertions, marker, JSON schema, and production services; the mutant execution changes exactly one named seam",
			"DroppedAdjustment suppresses exactly one translated AppEvent::Adjust before AppLoop dispatch; it does not edit the EventLog or coverage report",
			"CrossPatchParameterLeak applies the edited Patch's ChannelParameters to exactly one different Patch at the ParameterSnapshot-to-MixEngine ownership seam while accepted AppState, published PatchIds, and both stems remain otherwise correct; it does not edit StateTree, EventLog, measured energies, or observation fields",
			"PatchMisroute rewrites exactly one accepted PatchMidi command to a different installed PatchId at the command-routing seam before engine dispatch; it does not edit measured stems or observation fields",
			"OmittedStateTreeLeaf removes exactly one required typed leaf while constructing the serialized StateTree before coverage/property discovery; it does not append a fake missing identifier after report construction",
			"DryToWetBypass uses a nonzero dry signal as wet excitation while both supplied effect inputs are exactly zero at the GlobalEffectsProcessor seam; paired healthy and mutant renders begin from identical reset effect state, and the mutant does not edit input-energy or output-delta measurements",
			"ZeroRenderer clears the caller-owned audio buffer immediately after the production AudioRenderer render path and before measurement; it does not override the reported peak or completion flag",
			"every mutant emits exactly one schema-valid CREST_MUTATION_OBSERVATION describing actual downstream measurements, then exits with status 1; every matching healthy case emits the same schema and exits 0",
			"the harness is verification-only and exposes no mutation switch, alternate engine, alternate routing mode, or debug behavior to the interactive crest-synth application",
		]
		validations: [{kind: "integration", command: ["cargo", "test", "--test", "behavioral_mutation_harness", "--", "--nocapture"], assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "CREST_ACCEPTANCE behavioral_mutation_harness passed"}], description: "all six isolated seam mutants alter only their named seam and produce measured falsifying observations without report tampering"}]
		contributesTo: [
			{capability: "capability.observable_demo_scene", contribution: "makes the exhaustive proof independently falsifiable at six production seams"},
			{capability: "capability.one_way_parameter_control", contribution: "proves a dropped adjustment, cross-Patch parameter leak, and Patch misroute cannot masquerade as accepted behavior"},
			{capability: "capability.global_mix", contribution: "proves cross-Patch leakage, dry-to-wet bypass, zeroed render output, and incorrect Patch routing are detected by causal measurements"},
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
	validations: [{kind: "test", command: ["cargo", "test", "corridors_midi_event_source"], description: "the real fixture discovers multiple instruments, keeps note pairs together, assigns a unique channel to every Patch, rejects channel exhaustion, and emits due bounded events"}]
	contributesTo: [
		{capability: "capability.automatic_test_midi", contribution: "implements the fixed automatic Corridors of Time test input"},
		{capability: "capability.soundfont_audio", contribution: "provides the bank/program/percussion identity used to configure every Patch"},
	]
}
