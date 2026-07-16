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
		TestPlayback: "serialized start/stop and position state for the MIDI-file verification player"
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
		state: {partIndex: "u32", identity: "InstrumentIdentity", soundFontInstrument: "SoundFontInstrument", patchId: "PatchId", mixerTrack: "u8"}
		description: "the HiDef.sf2 instrument, canonical sample Patch, and mixer track created for one MIDI instrument part"
		invariants: ["mixerTrack is 0..=15", "mixerTrack equals partIndex modulo 16"]
	}
	ScheduledPatchEvent: {
		state: {atSeconds: "f64", patchId: "PatchId", event: "MidiEvent"}
		description: "a test-playback event explicitly targeted at the Patch created for its instrument part"
	}
	TestPlaybackPlan: {
		state: {soundFontPath: "string", assignments: "list<PlaybackAssignment>", events: "list<ScheduledPatchEvent>", durationSeconds: "f64"}
		description: "complete deterministic test orchestration; it never drops parts when more than sixteen instruments share the sixteen mixer tracks"
		invariants: ["soundFontPath is exactly ./sf2/HiDef.sf2"]
		validations: [{kind: "test", command: ["cargo", "test", "test_playback_plan"], description: "HiDef path, instrument selection, part ordering, unique patch IDs, modulo-16 assignments, and targeted event order are deterministic"}]
	}
	PlaybackStatus: {description: "Stopped or Playing"}
	PlaybackCommand: {description: "ToggleFromStart from the L key, or Advance from the render clock; both are applied through AppEvent and AppState"}
}

project: contexts: MidiFile: aggregates: TestPlayback: {
	root: true
	purpose: "own serialized MIDI-file playback state while Sequencer remains a deterministic effect processor"
	state: {plan: "option<TestPlaybackPlan>", status: "PlaybackStatus", positionSeconds: "f64", nextEvent: "usize", generation: "u64"}
	commands: {
		ToggleFromStart: {}
		Advance: {elapsedSeconds: "f64"}
	}
	events: {
		StartedFromBeginning: {generation: "u64"}
		StoppedAndRewound: {generation: "u64"}
		Advanced: {positionSeconds: "f64", nextEvent: "usize"}
	}
	invariants: [
		"ToggleFromStart while Stopped sets Playing, positionSeconds 0, nextEvent 0, and increments generation",
		"ToggleFromStart while Playing sets Stopped, positionSeconds 0, nextEvent 0, increments generation, and causes all active test-playback notes to be released",
		"starting after any stop always begins at event zero; L never resumes from the prior position",
		"Advance changes position only while Playing and never skips or repeats a ScheduledPatchEvent",
		"all state changes occur through AppEvent::Playback and AppState.apply; the window, key handler, Sequencer, and audio callback never mutate TestPlayback directly",
	]
	validations: [{kind: "test", command: ["cargo", "test", "test_playback"], description: "L starts at zero, L stops and rewinds, the next L restarts at zero, note release is requested, and replay is deterministic"}]
	contributesTo: [{capability: "capability.instrument_partitioned_test_playback", contribution: "provides serialized L-key start/stop-from-beginning semantics for the test player"}]
}

project: contexts: MidiFile: ports: MidiFileReader: {
	direction: "inbound"
	contract: {load: "(path: Path) -> result<Song, MidiFileError>"}
	meta: notes: "tempo changes are applied globally before partitioning; parsing retains source-track provenance and assigns stable NoteIds"
}

project: contexts: MidiFile: domainServices: Sequencer: {
	purpose: "derive scheduled patch events and playback-state advances for each render block without mutating TestPlayback itself"
	uses: ["aggregate.MidiFile.TestPlayback", "valueObject.MidiFile.TestPlaybackPlan", "valueObject.MidiFile.ScheduledPatchEvent"]
	validations: [{kind: "test", command: ["cargo", "test", "sequencer"], description: "stopped silence, start-at-zero, block boundaries, ordering, tempo-derived time, patch targets, and restart emit each event exactly once"}]
	contributesTo: [{capability: "capability.instrument_partitioned_test_playback", contribution: "preserves instrument-to-patch identity while scheduling test events into render blocks"}]
}

project: contexts: MidiFile: applicationServices: TestPlaybackAssembler: {
	purpose: "resolve every MIDI instrument against HiDef.sf2, create one canonical sample Patch per part, and produce patch-targeted events with round-robin mixer tracks"
	uses: ["valueObject.MidiFile.Song", "valueObject.MidiFile.TestPlaybackPlan", "valueObject.Sample.SoundFontInstrument", "port.Sample.SoundFontPlugin", "port.Sample.SampleStore", "applicationService.Patch.PatchManager", "aggregate.Loop.AppState"]
	operations: {prepare: {input: {song: "Song", state: "&mut AppState"}, output: {plan: "result<TestPlaybackPlan, PlaybackPlanError>"}}}
	meta: rules: [
		"visit parts in Song order; part N receives a fresh PatchId and mixer track N % 16",
		"open ./sf2/HiDef.sf2 exactly once for the complete plan and reuse that SoundFontHandle for every part",
		"use exactly ./sf2/HiDef.sf2; explicit bank/program selects that SoundFont preset, percussion selects its percussion bank, and a metadata-free fallback selects bank 0 program 0 while retaining the fallback label",
		"load the resolved SoundFontInstrument through SoundFontPlugin, store its SampleSet, and create an EngineType::Sample Patch through PatchManager.createSamplePatch; virtual-analog fallback is forbidden",
		"install every generated Patch and the prepared TestPlayback plan by submitting canonical AppEvents through AppState.apply before scheduling; never mutate AppState fields directly",
		"MIDI-file targeting is an explicit test-only route; external live MIDI continues to use channel mapping through MidiDispatcher",
	]
	validations: [{kind: "test", command: ["cargo", "test", "test_playback_assembler"], description: "HiDef instrument selection, one sample Patch per instrument, first-seen ordering, modulo-16 assignment, fallback labels, and >16-part sharing are proven"}]
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
