package crestsynth

// MidiFile — Standard MIDI File playback: how a human hears the engine
// without hardware. Parse a .mid into time-ordered kernel MidiEvents and
// sequence them into the synth at tempo.

project: contexts: MidiFile: purpose: "Standard MIDI File playback: parse a .mid into time-ordered normalized MidiEvents and sequence them into the engine in real time"

project: contexts: MidiFile: valueObjects: {
	TimedEvent: {
		state: {atSeconds: "f64", event: "MidiEvent"}
		description: "one normalized MIDI event with its absolute time offset from song start"
		invariants: ["atSeconds must be non-negative"]
	}
	Song: {
		state: {events: "list<TimedEvent>", durationSeconds: "f64"}
		description: "a fully parsed MIDI file"
		invariants: [
			"events are ordered by atSeconds ascending",
			"durationSeconds is at least the last event's atSeconds",
		]
	}
}

project: contexts: MidiFile: ports: {
	MidiFileReader: {
		direction: "inbound"
		contract: {
			load: "(path: Path) -> result<Song, MidiFileError>"
		}
		meta: notes: "tempo changes inside the file must be honored when computing each event's atSeconds; note-on events are tagged with fresh NoteIds"
	}
}

project: contexts: MidiFile: domainServices: {
	Sequencer: {
		purpose: "feeds a Song into the engine in real time: for each audio block, emits exactly the events whose time falls within that block, in order, to the MIDI dispatcher; supports looping the song"
		uses: ["valueObject.MidiFile.Song", "domainService.Patch.MidiDispatcher"]
		validations: [{kind: "test", command: ["cargo", "test", "sequencer"], description: "block boundaries, ordering, tempo changes, and looping emit each event exactly once"}]
	}
}

project: adapters: MidlyMidiFileReader: {
	implements: "port.MidiFile.MidiFileReader"
	layer:      "infrastructure"
	profile: {kind: "persistence", medium: "user-selected Standard MIDI File"}
	meta: {
		framework: "midly"
		notes:     "Standard MIDI File format 1 stores parallel tracks: note events live in tracks OTHER than the first (the first is often a conductor track holding only the tempo map). load() must merge events from ALL tracks into one ascending-time stream, applying the conductor track's tempo changes to every track. A multi-track file with notes only in later tracks must produce those notes."
	}
	validations: [{kind: "test", command: ["cargo", "test", "midly_midi_file_reader"], description: "format-0 and format-1 tracks merge against the shared tempo map with stable NoteIds"}]
}
