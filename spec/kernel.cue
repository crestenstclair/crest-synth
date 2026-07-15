package crestsynth

// Kernel — shared value types used by every context. No logic, just data.

project: contexts: Kernel: purpose: "foundation value types for MIDI addressing, audio primitives, time, and identifiers, shared by all contexts"

project: contexts: Kernel: ubiquitousLanguage: {
	NoteId:         "unique id per sounding note; enables per-note expression and MPE"
	ChannelAddress: "(group, channel) pair — 256 addressable MIDI destinations"
	AudioFrame:     "one stereo sample pair"
}

project: contexts: Kernel: valueObjects: {
	// MIDI addressing
	MidiGroup: {from: "u8", description: "MIDI 2.0 group index", invariants: ["must be 0-15"], validations: [{kind: "test", command: ["cargo", "test", "midi_group"], description: "MidiGroup unit tests pass"}]}
	MidiChannel: {from: "u8", description: "channel within a group", invariants: ["must be 0-15"], validations: [{kind: "test", command: ["cargo", "test", "midi_channel"], description: "MidiChannel unit tests pass"}]}
	ChannelAddress: {state: {group: "MidiGroup", channel: "MidiChannel"}, description: "a fully-qualified MIDI destination"}
	NoteId: {from: "u32", description: "unique per sounding note"}
	NoteNumber: {from: "u8", description: "MIDI note number", invariants: ["must be 0-127"], validations: [{kind: "test", command: ["cargo", "test", "note_number"], description: "NoteNumber unit tests pass"}]}
	Velocity: {from: "f64", description: "note velocity, upconverted from MIDI 1.0 7-bit to high resolution", invariants: ["must be 0.0-1.0"], validations: [{kind: "test", command: ["cargo", "test", "velocity"], description: "Velocity unit tests pass"}]}
	MidiEventKind: {description: "event discriminator: NoteOn, NoteOff, ControlChange, PitchBend, Aftertouch, PolyPressure, ProgramChange"}
	MidiEvent: {
		state: {address: "ChannelAddress", kind: "MidiEventKind", noteId: "NoteId", note: "NoteNumber", velocity: "Velocity"}
		description: "normalized internal MIDI event: addressed, high-resolution values, NoteId tagged"
	}

	// Audio primitives
	SampleRate: {from: "u32", description: "samples per second, e.g. 44100 or 48000", invariants: ["must be positive"], validations: [{kind: "test", command: ["cargo", "test", "sample_rate"], description: "SampleRate unit tests pass"}]}
	AudioFrame: {state: {left: "f32", right: "f32"}, description: "stereo sample pair"}
	Frequency: {from: "f64", description: "frequency in Hz", invariants: ["must be positive and finite"], validations: [{kind: "test", command: ["cargo", "test", "frequency"], description: "Frequency unit tests pass"}]}
	Amplitude: {from: "f64", description: "linear amplitude; 0.0 silence, 1.0 unity", invariants: ["must be non-negative and finite"], validations: [{kind: "test", command: ["cargo", "test", "amplitude"], description: "Amplitude unit tests pass"}]}
	Decibel: {from: "f64", description: "logarithmic level; 0 dB is unity, negative infinity is silence"}
	Pan: {from: "f64", description: "stereo position", invariants: ["must be -1.0 (hard left) to 1.0 (hard right)"]}

	// Time
	SampleCount: {from: "u64", description: "absolute sample position since stream start"}
	BufferSize: {from: "u32", description: "frames per audio callback", invariants: ["must be positive"]}
	TimeSignature: {state: {numerator: "u8", denominator: "u8"}, description: "e.g. 4/4 or 6/8", invariants: ["numerator and denominator must be positive"]}
	Tempo: {from: "f64", description: "beats per minute", invariants: ["must be positive and finite"]}

	// Identifiers
	PatchId: {from: "u32", description: "identifies a Patch"}
	SampleSetId: {from: "u32", description: "identifies a SampleSet"}
	PresetId: {from: "u32", description: "identifies a Preset"}
	SessionId: {from: "u32", description: "identifies a complete saved Session"}
	BankId: {from: "u32", description: "identifies a Bank"}
	BusId: {from: "u32", description: "identifies a mixer bus; bus 0 is reserved for the master"}
}
