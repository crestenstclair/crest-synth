package crestsynth

// Patch — one instrument: engine config + optional sample set + mod matrix +
// MIDI routing + mixer channel.

project: contexts: Patch: purpose: "instruments and MIDI routing: a patch binds a voice configuration, optional sample set, mod matrix, channel mapping, and a mixer strip"

project: contexts: Patch: valueObjects: {
	ChannelMapping: {
		state: {addresses: "list<ChannelAddress>", omni: "bool"}
		description: "which MIDI addresses a patch listens on; omni responds to all channels; multiple patches may layer on the same address intentionally"
	}
	MpeZone: {
		state: {managerChannel: "MidiChannel", memberChannels: "list<MidiChannel>"}
		description: "an MPE zone: a manager channel for global CCs plus contiguous member channels for per-note expression"
		invariants: ["memberChannels must be contiguous", "memberChannels must number at most 15"]
	}
}

project: contexts: Patch: aggregates: Patch: {
	root:    true
	purpose: "one playable instrument and its complete configuration"
	state: {id: "PatchId", name: "string", voice: "VoiceConfig", sampleSet: "option<SampleSetId>", modMatrix: "ModMatrix", mapping: "ChannelMapping", mpeZone: "option<MpeZone>", mixerStrip: "u32"}
	commands: {
		Rename: {name: "string"}
		SetVoiceConfig: {voice: "VoiceConfig"}
		SetMapping: {mapping: "ChannelMapping"}
		SetMpeZone: {zone: "option<MpeZone>"}
		AssignSampleSet: {sampleSet: "option<SampleSetId>"}
		SetModMatrix: {matrix: "ModMatrix"}
		AssignMixerStrip: {strip: "u32"}
	}
	events: {
		ConfigChanged: {id: "PatchId"}
		MappingChanged: {id: "PatchId"}
	}
	invariants: ["the optional sample set is used only when EngineType selects Sample", "mixerStrip is 0..=15; multiple patches may intentionally share a strip"]
	validations: [{kind: "test", command: ["cargo", "test", "patch"], description: "all complete patch fields change only through commands and preserve source consistency"}]
	contributesTo: [{capability: "capability.configurable_instrument_graph", contribution: "binds sound source, modulation, MIDI mapping, MPE zone, and mixer assignment into one playable instrument"}]
}

project: contexts: Patch: domainServices: {
	MidiDispatcher: {
		purpose: "routes each normalized MidiEvent to exactly the patches whose channel mapping matches its address"
		uses: ["aggregate.Patch.Patch"]
		validations: [{kind: "test", command: ["cargo", "test", "midi_dispatcher"], description: "matching patches receive one copy, intentional layers all receive one, and unmapped patches receive none"}]
		contributesTo: [{capability: "capability.external_midi_performance", contribution: "delivers normalized events to exactly the intentionally mapped and layered patches"}]
	}
}

project: contexts: Patch: applicationServices: {
	PatchManager: {
		purpose: "application-level patch collection: CRUD, complete configuration, fresh IDs, valid mixer-strip assignments, and cross-patch MPE validation"
		uses: ["aggregate.Patch.Patch"]
		operations: {
			createPatch: {input: {template: "Patch", name: "string", mixerStrip: "u32"}, output: {patch: "result<Patch, PatchError>"}}
			createSamplePatch: {input: {name: "string", sampleSet: "SampleSetId", mixerStrip: "u32", mapping: "ChannelMapping"}, output: {patch: "result<Patch, PatchError>"}}
			deletePatch: {input: {id: "PatchId"}}
			applyCommand: {input: {id: "PatchId", command: "PatchCommand"}}
			validateMpeZones: {output: {result: "result<(), MpeOverlap>"}}
		}
		meta: rules: [
			"MPE non-overlap is a collection invariant enforced here, because one Patch cannot inspect its siblings",
			"created patches receive fresh stable PatchIds; a caller may choose an already-valid mixer strip, including deterministic sharing for MIDI-file tests",
			"createSamplePatch always selects EngineType::Sample, assigns the provided SampleSetId, and never constructs a virtual-analog fallback",
		]
		validations: [{kind: "test", command: ["cargo", "test", "patch_manager"], description: "CRUD is atomic, IDs are fresh, mixer strips are valid/shareable, SoundFont sample Patches cannot fall back to another engine, and overlapping MPE zones are rejected"}]
		contributesTo: [{capability: "capability.configurable_instrument_graph", contribution: "coordinates complete patch configuration and cross-patch routing invariants"}]
	}
}
