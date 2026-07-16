package crestsynth

project: contexts: Testing: {
	purpose: "automatic MIDI input used to exercise the synth through production ports"
	meta: rules: [
		"this context is input test support, not a sequencing or transport domain",
		"its timing and MIDI-file parsing types are private implementation details",
	]

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
			"assignedChannel equals index modulo 16",
		]
		contributesTo: [
			{capability: "capability.automatic_test_midi", contribution: "defines the one-Patch-per-instrument and round-robin assignment"},
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
			"create one InstrumentPart per identity in first-sounding order and assign part N to MidiChannel N modulo 16",
			"target every emitted message at its InstrumentPart/Patch and rewrite its channel to assignedChannel",
			"start at elapsed zero automatically, run once, and stop at end; do not expose transport, seeking, looping, recording, editing, or public sequence types",
		]
	}
	validations: [{kind: "test", command: ["cargo", "test", "corridors_midi_event_source"], description: "the real fixture discovers multiple instruments, keeps note pairs together, assigns round-robin channels, and emits due bounded events"}]
	contributesTo: [
		{capability: "capability.automatic_test_midi", contribution: "implements the fixed automatic Corridors of Time test input"},
		{capability: "capability.soundfont_audio", contribution: "provides the bank/program/percussion identity used to configure every Patch"},
	]
}
