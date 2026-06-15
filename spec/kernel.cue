package crestsynth

// ── Kernel ─────────────────────────────────────────────
// Shared value types for MIDI addressing, audio primitives, and note identity.

project: contexts: Kernel: purpose: "shared value types for MIDI addressing, audio primitives, and note identity"
project: contexts: Kernel: ubiquitousLanguage: {
	MidiEvent:      "normalized internal event addressed by (group, channel) with high-res values and note-id"
	NoteId:         "unique identifier for a sounding note, enabling per-note expression"
	ChannelAddress: "a (group, channel) pair — 256 addressable destinations"
}

// Bounded Kernel value objects each expose an ADDITIVE checked constructor
// (try_new) proven by a unit test that the bound actually rejects out-of-range
// input — added without changing the existing constructor so crate-wide callers
// keep compiling. `_checkedCtor` is the shared invariant text.
_checkedCtor: "exposes an ADDITIVE checked constructor (e.g. `try_new(v) -> Option<Self>`) that returns None for out-of-range input, WITHOUT changing or removing the existing constructor (so existing callers across the crate keep compiling); a unit test asserts an out-of-range value is rejected (None) and an in-range value accepted (Some)"

project: contexts: Kernel: valueObjects: MidiGroup:   {from: "u8", description: "MIDI 2.0 group index (0-15)", invariants: ["must be 0-15", _checkedCtor], validations: [{kind: "test", command: ["cargo", "test", "midi_group"], description: "MidiGroup checked-constructor rejects values outside 0-15"}]}
project: contexts: Kernel: valueObjects: MidiChannel: {from: "u8", description: "MIDI channel (0-15 within a group)", invariants: ["must be 0-15", _checkedCtor], validations: [{kind: "test", command: ["cargo", "test", "midi_channel"], description: "MidiChannel checked-constructor rejects values outside 0-15"}]}
project: contexts: Kernel: valueObjects: NoteId:      {from: "u32", description: "unique identifier for a sounding note"}
project: contexts: Kernel: valueObjects: NoteNumber:  {from: "u8", description: "MIDI note number (0-127)", invariants: ["must be 0-127", _checkedCtor], validations: [{kind: "test", command: ["cargo", "test", "note_number"], description: "NoteNumber checked-constructor rejects values outside 0-127"}]}
project: contexts: Kernel: valueObjects: Velocity:    {from: "f64", description: "normalized note velocity (0.0-1.0)", invariants: ["must be 0.0-1.0", _checkedCtor], validations: [{kind: "test", command: ["cargo", "test", "velocity"], description: "Velocity checked-constructor rejects values outside 0.0-1.0"}]}
project: contexts: Kernel: valueObjects: SampleRate:  {from: "u32", description: "audio sample rate in Hz", invariants: ["must be positive", _checkedCtor], validations: [{kind: "test", command: ["cargo", "test", "sample_rate"], description: "SampleRate checked-constructor rejects zero"}]}
project: contexts: Kernel: valueObjects: AudioFrame:  {state: {left: "f32", right: "f32"}, description: "one stereo sample pair"}
project: contexts: Kernel: valueObjects: MidiEvent: {
	description: "normalized internal event: (group, channel) addressed, high-res values, note-id tagged"
	state: {
		group: "MidiGroup", channel: "MidiChannel", noteId: "NoteId",
		kind: "MidiEventKind", noteNumber: "NoteNumber", velocity: "Velocity", value: "f64",
	}
}
project: contexts: Kernel: valueObjects: Frequency: {from: "f64", description: "frequency in Hz", invariants: ["must be positive", _checkedCtor], validations: [{kind: "test", command: ["cargo", "test", "frequency"], description: "Frequency checked-constructor rejects non-positive values"}]}
project: contexts: Kernel: valueObjects: Amplitude: {from: "f64", description: "linear amplitude (0.0 = silence, 1.0 = unity)", invariants: ["must be non-negative", _checkedCtor], validations: [{kind: "test", command: ["cargo", "test", "amplitude"], description: "Amplitude checked-constructor rejects negative values"}]}
project: contexts: Kernel: valueObjects: ChannelAddress: {
	state:       {group: "MidiGroup", channel: "MidiChannel"}
	description: "a (group, channel) pair — the 256-destination address space for MIDI 2.0"
}
