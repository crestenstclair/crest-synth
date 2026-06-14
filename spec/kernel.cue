package crestsynth

// ── Kernel ─────────────────────────────────────────────
// Shared value types for MIDI addressing, audio primitives, and note identity.

project: contexts: Kernel: purpose: "shared value types for MIDI addressing, audio primitives, and note identity"
project: contexts: Kernel: ubiquitousLanguage: {
	MidiEvent:      "normalized internal event addressed by (group, channel) with high-res values and note-id"
	NoteId:         "unique identifier for a sounding note, enabling per-note expression"
	ChannelAddress: "a (group, channel) pair — 256 addressable destinations"
}

project: contexts: Kernel: valueObjects: MidiGroup:   {from: "u8", description: "MIDI 2.0 group index (0-15)", invariants: ["must be 0-15"]}
project: contexts: Kernel: valueObjects: MidiChannel: {from: "u8", description: "MIDI channel (0-15 within a group)", invariants: ["must be 0-15"]}
project: contexts: Kernel: valueObjects: NoteId:      {from: "u32", description: "unique identifier for a sounding note"}
project: contexts: Kernel: valueObjects: NoteNumber:  {from: "u8", description: "MIDI note number (0-127)", invariants: ["must be 0-127"]}
project: contexts: Kernel: valueObjects: Velocity:    {from: "f64", description: "normalized note velocity (0.0-1.0)", invariants: ["must be 0.0-1.0"]}
project: contexts: Kernel: valueObjects: SampleRate:  {from: "u32", description: "audio sample rate in Hz", invariants: ["must be positive"]}
project: contexts: Kernel: valueObjects: AudioFrame:  {state: {left: "f32", right: "f32"}, description: "one stereo sample pair"}
project: contexts: Kernel: valueObjects: MidiEvent: {
	description: "normalized internal event: (group, channel) addressed, high-res values, note-id tagged"
	state: {
		group: "MidiGroup", channel: "MidiChannel", noteId: "NoteId",
		kind: "MidiEventKind", noteNumber: "NoteNumber", velocity: "Velocity", value: "f64",
	}
}
project: contexts: Kernel: valueObjects: Frequency: {from: "f64", description: "frequency in Hz", invariants: ["must be positive"]}
project: contexts: Kernel: valueObjects: Amplitude: {from: "f64", description: "linear amplitude (0.0 = silence, 1.0 = unity)", invariants: ["must be non-negative"]}
project: contexts: Kernel: valueObjects: ChannelAddress: {
	state:       {group: "MidiGroup", channel: "MidiChannel"}
	description: "a (group, channel) pair — the 256-destination address space for MIDI 2.0"
}
