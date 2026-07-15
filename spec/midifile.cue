package crestsynth

// Standard MIDI Files are deterministic demonstration inputs, not a product
// sequencer. The reader preserves musical instrument identity; the application
// service turns each discovered part into its own canonical Patch and assigns
// those patches to mixer tracks round-robin for broad integration coverage.
project: contexts: MidiFile: {
	purpose: "parse Standard MIDI Files into instrument-partitioned test songs and schedule their patch-targeted events through the standalone render path"
	ubiquitousLanguage: {
		InstrumentIdentity: "the best stable instrument identity available from bank/program and percussion metadata, or a documented source-track/channel fallback"
		InstrumentPart: "all musical events belonging to one discovered instrument, ordered by absolute time"
		TestPlaybackPlan: "one generated Patch per instrument plus deterministic round-robin mixer-track assignments"
	}
}

project: contexts: MidiFile: valueObjects: {
	InstrumentIdentity: {
		state: {bankMsb: "option<u8>", bankLsb: "option<u8>", program: "option<u8>", percussion: "bool", fallbackSourceTrack: "option<u16>", fallbackSourceChannel: "option<MidiChannel>", label: "string", usedFallback: "bool"}
		description: "bank/program identity when explicit metadata exists; MIDI channel 10 is a distinct percussion identity; otherwise source track plus channel is the deterministic fallback"
		invariants: [
			"bank and program values are 0..=127",
			"explicit bank/program and percussion keys do not contain source track or channel, so the same instrument may merge across physical tracks and channels",
			"usedFallback is true exactly when explicit program/percussion identity was unavailable; then and only then both fallbackSourceTrack and fallbackSourceChannel are present",
			"the label states the program/percussion name or the source-track/channel fallback used",
		]
	}
	TimedEvent: {
		state: {atSeconds: "f64", sourceTrack: "u16", event: "MidiEvent"}
		description: "one normalized MIDI event with absolute song time and source-track provenance"
		invariants: ["atSeconds is non-negative"]
	}
	InstrumentPart: {
		state: {identity: "InstrumentIdentity", firstEventSeconds: "f64", events: "list<TimedEvent>"}
		description: "one deterministic part; the matching note-off remains in the same part selected for its note-on even if a program change occurs while the note is held"
		invariants: ["events are ordered by absolute time", "every part contains at least one musical event"]
	}
	Song: {
		state: {parts: "list<InstrumentPart>", durationSeconds: "f64"}
		description: "a fully parsed test song partitioned by instrument rather than merely by SMF physical track"
		invariants: [
			"parts are ordered by first musical event, then stable identity as a tie-breaker",
			"durationSeconds is at least the last event time in every part",
			"identical file bytes produce byte-identical part ordering and labels",
		]
	}
	PlaybackAssignment: {
		state: {partIndex: "u32", identity: "InstrumentIdentity", patchId: "PatchId", mixerTrack: "u8"}
		description: "the canonical Patch and mixer track created for one instrument part"
		invariants: ["mixerTrack is 0..=15", "mixerTrack equals partIndex modulo 16"]
	}
	ScheduledPatchEvent: {
		state: {atSeconds: "f64", patchId: "PatchId", event: "MidiEvent"}
		description: "a test-playback event explicitly targeted at the Patch created for its instrument part"
	}
	TestPlaybackPlan: {
		state: {assignments: "list<PlaybackAssignment>", events: "list<ScheduledPatchEvent>", durationSeconds: "f64"}
		description: "complete deterministic test orchestration; it never drops parts when more than sixteen instruments share the sixteen mixer tracks"
		validations: [{kind: "test", command: ["cargo", "test", "test_playback_plan"], description: "part ordering, unique patch IDs, modulo-16 assignments, and targeted event order are deterministic"}]
	}
}

project: contexts: MidiFile: ports: MidiFileReader: {
	direction: "inbound"
	contract: {load: "(path: Path) -> result<Song, MidiFileError>"}
	meta: notes: "tempo changes are applied globally before partitioning; parsing retains source-track provenance and assigns stable NoteIds"
}

project: contexts: MidiFile: domainServices: Sequencer: {
	purpose: "for each render block, emit the ScheduledPatchEvents whose absolute times fall in that block, in stable order; optionally loop the complete plan"
	uses: ["valueObject.MidiFile.TestPlaybackPlan", "valueObject.MidiFile.ScheduledPatchEvent"]
	validations: [{kind: "test", command: ["cargo", "test", "sequencer"], description: "block boundaries, ordering, tempo-derived time, patch targets, and looping emit each event exactly once"}]
	contributesTo: [{capability: "capability.instrument_partitioned_test_playback", contribution: "preserves instrument-to-patch identity while scheduling test events into render blocks"}]
}

project: contexts: MidiFile: applicationServices: TestPlaybackAssembler: {
	purpose: "create one canonical Patch per Song instrument part and produce deterministic patch-targeted events with round-robin mixer-track assignments"
	uses: ["valueObject.MidiFile.Song", "valueObject.MidiFile.TestPlaybackPlan", "applicationService.Patch.PatchManager", "aggregate.Loop.AppState"]
	operations: {prepare: {input: {song: "Song", basePatch: "Patch", state: "&mut AppState"}, output: {plan: "result<TestPlaybackPlan, PlaybackPlanError>"}}}
	meta: rules: [
		"visit parts in Song order; part N receives a fresh PatchId and mixer track N % 16",
		"copy the base sound configuration but replace identity, display name, and mixer-strip assignment for each part; test targeting does not depend on live channel mapping",
		"create with PatchManager and install every generated Patch by submitting the canonical PatchCommand through AppState.apply before scheduling; never create a parallel patch type or mutate AppState fields directly",
		"MIDI-file targeting is an explicit test-only route; external live MIDI continues to use channel mapping through MidiDispatcher",
	]
	validations: [{kind: "test", command: ["cargo", "test", "test_playback_assembler"], description: "one patch per instrument, first-seen ordering, modulo-16 track assignment, fallback labels, and >16-part sharing are proven"}]
	contributesTo: [
		{capability: "capability.instrument_partitioned_test_playback", contribution: "materializes every discovered instrument as its own canonical patch and assigns mixer tracks round-robin"},
		{capability: "capability.configurable_instrument_graph", contribution: "uses the real PatchManager and AppState rather than test-only sound models"},
	]
}

project: adapters: MidlyMidiFileReader: {
	implements: "port.MidiFile.MidiFileReader"
	layer: "infrastructure"
	profile: {kind: "persistence", medium: "user-selected Standard MIDI File"}
	meta: {
		framework: "midly"
		rules: [
			"merge all SMF tracks against the shared tempo map; a conductor-only first track never hides notes in later tracks",
			"maintain bank-select and program state while walking merged events and treat channel 10 as percussion",
			"bind a note-off to the InstrumentIdentity selected for its note-on using NoteId, even across intervening program changes",
			"route channel-wide expression/control events to the currently active or most recently selected instrument part on that channel",
			"merge matching explicit bank/program identities across SMF tracks and MIDI channels; source provenance remains on each TimedEvent rather than splitting the instrument",
			"when no explicit program/percussion identity exists, partition by source track plus MIDI channel and label the fallback",
		]
	}
	validations: [{kind: "test", command: ["cargo", "test", "midly_midi_file_reader"], description: "format-0/1 tempo merge, program changes, percussion, note pairing, fallback grouping, and deterministic part order pass"}]
	contributesTo: [{capability: "capability.instrument_partitioned_test_playback", contribution: "discovers stable instrument parts from real multi-track MIDI files with deterministic fallbacks"}]
}
