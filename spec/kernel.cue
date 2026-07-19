package crestsynth

project: contexts: Kernel: {
	purpose: "shared values at the MIDI, patch, and audio boundaries"

	valueObjects: PatchId: {
		description: "stable identity assigned to one instrument Patch"
		state: value: "u32"
		invariants: ["value is non-zero"]
		contributesTo: [{capability: "capability.soundfont_audio", contribution: "keeps instrument configuration, MIDI targeting, control state, and audio commands tied to one Patch"}]
	}

	valueObjects: MidiChannel: {
		description: "the assigned MIDI channel used by a Patch"
		state: value: "u8"
		invariants: ["value is in 0..=15"]
		contributesTo: [{capability: "capability.automatic_test_midi", contribution: "represents the fixture's unique Patch-to-channel assignment"}]
	}

	valueObjects: MidiMessage: {
		description: "a normalized channel MIDI message accepted by a synthesizer"
		state: {
			channel: "MidiChannel"
			kind: "MidiMessageKind"
			data1: "u8"
			data2: "u8"
		}
		invariants: [
			"data bytes are in 0..=127",
			"the variants required now are note-on, note-off, control-change, program-change, channel-pressure, pitch-bend, and all-notes-off",
		]
		contributesTo: [
			{capability: "capability.soundfont_audio", contribution: "is the canonical command delivered to the SoundFont engine"},
			{capability: "capability.automatic_test_midi", contribution: "lets the file fixture implement the same normalized input boundary as later MIDI adapters"},
		]
	}
}
