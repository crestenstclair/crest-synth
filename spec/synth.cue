package crestsynth

project: contexts: Synth: {
	purpose: "capability-polymorphic Patch identity, off-thread instrument preparation, and bounded prepared rendering"
	ubiquitousLanguage: {
		Patch: "one playable instrument configuration identified by a capability rather than an engine-specific aggregate shape"
		SoundFontInstrument: "the bank, program, and percussion identity selected in HiDef.sf2"
	}

	valueObjects: SoundFontInstrument: {
		description: "the preset selector derived from a MIDI instrument part"
		state: {
			bank: "u16"
			program: "u8"
			percussion: "bool"
		}
		invariants: [
			"program is in 0..=127",
			"percussion identity remains distinct from a melodic preset with the same numeric bank and program",
		]
		contributesTo: [
			{capability: "capability.soundfont_audio", contribution: "selects the correct HiDef.sf2 instrument for a Patch"},
			{capability: "capability.automatic_test_midi", contribution: "preserves the instrument identity discovered in the MIDI fixture"},
		]
	}

	aggregates: Patch: {
		root: true
		purpose: "own one instrument capability config, assigned MIDI channel, identity, and editable mixer parameters"
		state: {
			id: "PatchId"
			name: "String"
			instrument: "InstrumentConfig"
			channel: "MidiChannel"
			parameters: "ChannelParameters"
		}
		invariants: [
			"id is stable for the process lifetime",
			"instrument capability and configuration are validated against the installed CapabilityRegistry before Patch construction",
			"instrument configuration is immutable after the Patch is installed in this increment",
			"channel is assigned by the input adapter and is in 0..=15",
			"only ChannelParameters may be edited after installation",
			"Patch contains no SoundFont-only field, engine object, descriptor copy, prepared renderer, decoded asset, UI state, or fallback configuration",
		]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "makes one canonical Patch aggregate support capability-owned instrument schemas"},
			{capability: "capability.soundfont_audio", contribution: "binds the current playable Patch to the registered HiDef SoundFont capability"},
			{capability: "capability.one_way_parameter_control", contribution: "is the unit listed and edited by the text view"},
		]
	}

	ports: PreparedInstrument: {
		direction: "outbound"
		contract: {
			patchId: "() -> PatchId"
			dispatch: "(MidiMessage) -> Result<(), PreparedInstrumentError>"
			render: "(&mut [f32], frameCount: usize)"
			allNotesOff: "()"
		}
		consumes: [
			"valueObject.Kernel.MidiMessage",
			"valueObject.Kernel.PatchId",
		]
		invariants: [
			"the port is object-safe and contains only callback-safe operations over one already prepared Patch instrument",
			"patchId is fixed at preparation and the rack, not the implementation, selects the caller-owned stereo stem",
			"dispatch and allNotesOff have bounded work and return only fixed-size typed status",
			"render fills only the supplied interleaved stereo stem for at most its prepared maximum frame count",
			"dispatch, allNotesOff, and render perform no allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, unwinding, or destruction",
			"dynamic dispatch may occur once per targeted message or once per Patch render block, never in an inner sample loop",
		]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "gives the rack one capability-neutral callback contract per Patch"},
			{capability: "capability.realtime_execution", contribution: "keeps capability implementation details outside the hard-real-time caller"},
		]
	}

	ports: InstrumentPreparer: {
		direction: "outbound"
		contract: {
			capabilityId: "() -> CapabilityId"
			prepare: "(&Patch, sampleRate: f32, maxFrames: usize) -> Result<Box<dyn PreparedInstrument>, InstrumentPreparationError>"
		}
		consumes: [
			"aggregate.Synth.Patch",
			"valueObject.Synth.CapabilityId",
			"port.Synth.PreparedInstrument",
		]
		invariants: [
			"all preparation runs outside the audio callback and may perform validated asset I/O, parsing, allocation, and warmup",
			"capabilityId is stable and exactly matches the InstrumentConfig identities this preparer accepts",
			"prepare rejects an unsupported capability, invalid config, unsupported sample rate or frame capacity, asset failure, and voice-capacity failure with typed errors",
			"prepare never selects another capability, config, asset, preset, voice limit, or renderer as fallback",
			"a successful result has finished every allocation and capacity decision before ownership can cross the structural boundary",
		]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "joins a validated capability config to its separate preparation port without putting a factory in Patch"},
			{capability: "capability.prepared_engine_rack", contribution: "builds callback-ready instruments behind one generic preparation contract"},
		]
	}

	applicationServices: PreparedEngineRackBuilder: {
		purpose: "build one bounded capability-neutral prepared instrument slot for every accepted Patch outside the callback"
		uses: [
			"aggregate.Synth.Patch",
			"valueObject.Synth.CapabilityRegistry",
			"port.Synth.InstrumentPreparer",
			"aggregate.RealTime.PreparedEngineRack",
		]
		operations: {
			build: {input: {patches: "&[Patch]", registry: "&CapabilityRegistry", preparers: "&[Box<dyn InstrumentPreparer>]", sampleRate: "f32", maxFrames: "usize"}, output: {result: "Result<PreparedEngineRack, RackPreparationError>"}}
		}
		meta: rules: [
			"resolve every Patch InstrumentConfig through the immutable CapabilityRegistry and exactly one matching InstrumentPreparer by CapabilityId",
			"reject duplicate PatchIds, duplicate preparers, missing or extra capability preparation, capacity overflow, invalid audio format, or any preparer/config disagreement before constructing a rack",
			"preserve accepted Patch order and assign one unique slot and output stem to every Patch",
			"return no partial rack and never substitute a preparer, instrument, config, asset, or slot after any failure",
			"the current production composition registers only the HiDef SoundFont preparer; heterogeneous test instruments prove the rack boundary without presenting an unavailable product engine",
		]
		validations: [{kind: "test", command: ["cargo", "test", "prepared_engine_rack_builder"], description: "exact preparation succeeds while missing, duplicate, mismatched, over-capacity, and partial configurations fail without fallback"}]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "constructs the bounded rack from canonical Patch configs and registered preparation ports"},
		]
	}

}

project: adapters: HiDefSoundFontPreparer: {
	implements: "port.Synth.InstrumentPreparer"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "sf2", system: "HiDef.sf2"}
	meta: {
		framework: "rustysynth"
		rules: [
			"expect exactly ./sf2/HiDef.sf2 and parse it once outside the callback; return a clear preparation error if it is missing or invalid",
			"own one HiDefSoundFontPreparer and share its immutable parsed SoundFont bank across the per-Patch PreparedInstrument values it creates; never parse or clone the full bank per Patch",
			"prepare one bounded rustysynth synthesizer for each accepted instrument.soundfont.hidef Patch using its exact bank, program, percussion, channel, and fixed asset assignments",
			"inside each prepared instrument use rustysynth's percussion channel for a percussion Patch and a melodic channel for every other Patch, regardless of the Patch's logical assigned channel",
			"the private prepared value implements PreparedInstrument, routes only its own Patch MIDI, and renders only into the caller-owned stem selected by the rack",
			"disable rustysynth's built-in reverb and chorus so the declared global effects are the only effects",
			"SoundFont remains the only production InstrumentPreparer in this increment; do not add Braids source, C++/FFI, engine selection, layering, PATCH-page behavior, or fallback",
			"prepared dispatch, all-notes-off, and render use only bounded warmed state and perform no callback allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, unwinding, or destruction",
		]
	}
	validations: [{kind: "test", command: ["cargo", "test", "hidef_soundfont_preparer"], description: "one parsed bank prepares independent melodic and percussion instruments whose targeted MIDI and bounded non-silent stems remain isolated behind PreparedInstrument"}]
	contributesTo: [
		{capability: "capability.instrument_capability_model", contribution: "prepares the existing renderer from a generic capability config without becoming the Patch model"},
		{capability: "capability.prepared_engine_rack", contribution: "supplies the only production preparer and capability-neutral per-Patch prepared instruments"},
		{capability: "capability.soundfont_audio", contribution: "preserves one parsed HiDef.sf2 bank while adapting SoundFont to the generic prepared boundary"},
		{capability: "capability.realtime_execution", contribution: "keeps prepared SoundFont operations inside the callback contract"},
	]
}
