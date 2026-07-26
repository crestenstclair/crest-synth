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

	valueObjects: VoiceEnvelope: {
		description: "the canonical Patch-owned envelope applied independently inside every note voice"
		state: {
			attackMilliseconds: "f32"
			decayMilliseconds: "f32"
			sustain: "f32"
			releaseMilliseconds: "f32"
			surfaceDescriptor: "four typed stable ids with presentation labels, units, bounds, and fine/coarse steps"
		}
		invariants: [
			"all values are finite; Attack, Decay, and Release are in 0..=10000 milliseconds and Sustain is in 0..=1",
			"the descriptor enumerates each field exactly once, supplies its stable presentation label and milliseconds-or-unitless unit, and is shared by reducer, projection, demos, and prepared engines",
			"Attack, Decay, and Sustain are latched at note-on and Release is latched at note-off independently for each voice",
			"zero-time stages transition safely with bounded work and a post-Patch-stem gain envelope is nonconforming",
		]
		contributesTo: [
			{capability: "capability.per_voice_envelope", contribution: "defines one common envelope contract for every admitted engine"},
			{capability: "capability.one_way_parameter_control", contribution: "adds four schema-derived Patch values to the existing semantic adjustment path"},
			{capability: "capability.schema_driven_patch_page", contribution: "supplies the four canonical ADSR rows without creating UI-owned envelope state"},
		]
	}

	aggregates: Patch: {
		root: true
		purpose: "own one instrument capability config, assigned MIDI channel, identity, common voice envelope, and editable mixer parameters"
		state: {
			id: "PatchId"
			name: "String"
			instrument: "InstrumentConfig"
			channel: "MidiChannel"
			envelope: "VoiceEnvelope"
			parameters: "ChannelParameters"
		}
		invariants: [
			"id is stable for the process lifetime",
			"instrument capability and configuration are validated against the installed CapabilityRegistry before Patch construction",
			"descriptor-classified Scalar values may change through the canonical reducer; the only structural mutation in this increment atomically replaces the complete InstrumentConfig after a correlated engine candidate is prepared",
			"channel is assigned by the input adapter and is in 0..=15",
			"ChannelParameters, VoiceEnvelope, and active descriptor-classified Scalar instrument values form one schema-derived editable surface",
			"Patch contains no SoundFont-only field, engine object, descriptor copy, prepared renderer, decoded asset, UI state, or fallback configuration",
		]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "makes one canonical Patch aggregate support capability-owned instrument schemas"},
			{capability: "capability.soundfont_audio", contribution: "binds the current playable Patch to the registered HiDef SoundFont capability"},
			{capability: "capability.braids_engine", contribution: "binds the same generic Patch aggregate to the registered Braids capability"},
			{capability: "capability.one_way_parameter_control", contribution: "is the unit listed and edited by the text view"},
			{capability: "capability.schema_driven_patch_page", contribution: "supplies stable Patch identity, MIDI channel, active config, and envelope to the focused page"},
			{capability: "capability.asynchronous_engine_selection", contribution: "retains Patch identity, MIDI channel, envelope, and mixer routing while replacing only its prepared InstrumentConfig"},
		]
	}

	ports: PreparedInstrument: {
		direction: "outbound"
		contract: {
			patchId: "() -> PatchId"
			dispatch: "(MidiMessage, &RtPatchParameters) -> Result<(), PreparedInstrumentError>"
			render: "(&mut [f32], frameCount: usize, &RtPatchParameters)"
			allNotesOff: "()"
		}
		consumes: [
			"valueObject.Kernel.MidiMessage",
			"valueObject.Kernel.PatchId",
		]
		invariants: [
			"the port is object-safe and contains only callback-safe operations over one already prepared Patch instrument",
			"patchId is fixed at preparation and the rack, not the implementation, selects the caller-owned stereo stem",
			"the supplied RtPatchParameters exactly matches this Patch and the immutable graph-revision scalar layout prepared for this implementation",
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
			"the current production composition registers exactly the HiDef SoundFont and Braids preparers and prepares alternating fixture Patches without a capability branch in the rack",
		]
		validations: [{id: "validation.service.prepared_engine_rack_builder", kind: "test", command: ["cargo", "test", "prepared_engine_rack_builder"], description: "exact preparation succeeds while missing, duplicate, mismatched, over-capacity, and partial configurations fail without fallback"}]
		contributesTo: [
			{capability: "capability.prepared_engine_rack", contribution: "constructs the bounded rack from canonical Patch configs and registered preparation ports"},
			{capability: "capability.asynchronous_engine_selection", contribution: "rebuilds the bounded rack from the exact candidate Patch set without engine-specific branching"},
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
			"prepare exactly one rustysynth synthesizer for each accepted instrument.soundfont.hidef Patch using its exact bank, program, percussion, channel, and fixed asset assignments; all Patch synthesizers share only the parsed immutable bank",
			"inside each prepared instrument use rustysynth's percussion channel for a percussion Patch and a melodic channel for every other Patch, regardless of the Patch's logical assigned channel",
			"the private prepared value implements PreparedInstrument, delegates polyphony to its one Patch-local synthesizer, and applies the Patch VoiceEnvelope independently through an engine-native per-note seam before native voices enter the caller-owned stem",
			"disable rustysynth's built-in reverb and chorus so the declared global effects are the only effects",
			"the descriptor declares EngineManaged polyphony and Crest does not impose a sixteen-note limit, create a synthesizer per note, or share mutable synthesizer state between Patches; the preparer still declares and proves a finite internal callback-work ceiling",
			"if rustysynth cannot implement the common ADSR independently on overlapping native voices, extend the adapter or replace the backend rather than apply a post-stem envelope, create sixteen synthesizers per Patch, ignore a control, or claim conformance",
			"prepared dispatch, all-notes-off, and render use only bounded warmed state and perform no callback allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, unwinding, or destruction",
		]
	}
	validations: [{id: "validation.adapter.hidef_soundfont_preparer", kind: "test", command: ["cargo", "test", "hidef_soundfont_preparer"], description: "one parsed bank prepares independent melodic and percussion instruments whose targeted MIDI and bounded non-silent stems remain isolated behind PreparedInstrument"}]
	contributesTo: [
		{capability: "capability.instrument_capability_model", contribution: "prepares the existing renderer from a generic capability config without becoming the Patch model"},
		{capability: "capability.asynchronous_engine_selection", contribution: "prepares a selected default SoundFont candidate on worker ownership"},
		{capability: "capability.prepared_engine_rack", contribution: "supplies the SoundFont production preparer and capability-neutral per-Patch prepared instruments"},
		{capability: "capability.soundfont_audio", contribution: "preserves one parsed HiDef.sf2 bank while adapting SoundFont to the generic prepared boundary"},
		{capability: "capability.per_voice_envelope", contribution: "applies the common envelope through one Patch-local synthesizer's independent native voices"},
		{capability: "capability.realtime_execution", contribution: "keeps prepared SoundFont operations inside the callback contract"},
	]
}

project: adapters: BraidsPreparer: {
	implements: "port.Synth.InstrumentPreparer"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "opaque-cpp-ffi", system: "Mutable Instruments Braids MacroOscillator"}
	meta: {
		framework: "pinned C++ DSP + Rust RAII adapter"
		rules: [
			"compile only the audited MIT-licensed DSP subset pinned at pichenettes/eurorack@08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4 and stmlib@e3bd7c9cc00e4364166f9905c0509b6ffd0535ec behind one opaque extern-C wrapper with exceptions and RTTI disabled",
			"prepare a distinct bank of exactly sixteen fully initialized MacroOscillator values and matching envelope/voice metadata for every accepted Braids Patch outside the callback",
			"for N admitted Braids Patches own N independent banks and 16 × N voices, including forty-eight voices for three Patches; never share a bank, voice slot, envelope, stealing decision, or capacity globally, and impose no Braids-specific Patch-count limit below the engine-agnostic rack capacity",
			"accept exactly 48000 Hz host rendering, call upstream at 96000 Hz in chunks of at most 24 samples, and perform bounded 2:1 decimation into the caller-owned stereo stem",
			"interpret only the three descriptor-ordered Scalar slots for Model, Timbre, and Color and reject a scalar-layout or model mismatch without fallback",
			"assign idle voices first, steal the oldest deterministically, release matching-key slots, apply velocity and bounded pitch bend, and clear all sixteen slots on all-notes-off",
			"construct and destroy the opaque bank only outside callback ownership; dispatch and render cross no exception, allocation, destruction, lock, blocking, I/O, logging, formatting, panic, or unwind path",
		]
	}
	validations: [
		{id: "validation.adapter.braids_preparer", kind: "test", command: ["cargo", "test", "braids_preparer"], description: "source pins, lifecycle, exact-rate preparation, MIDI routing, scalar response, finite audio, sixteen voices, and deterministic stealing are proven"},
		{id: "validation.adapter.braids_preparer_integration", kind: "integration", command: ["cargo", "test", "--test", "braids_engine", "--", "--nocapture"], description: "the named production-path Braids acceptance runs"},
	]
	contributesTo: [
		{capability: "capability.braids_engine", contribution: "defers synthesis to the pinned upstream MacroOscillator while adapting it to Crest's bounded contracts"},
		{capability: "capability.asynchronous_engine_selection", contribution: "prepares a selected default Braids candidate on worker ownership"},
		{capability: "capability.per_voice_envelope", contribution: "owns one independent oscillator and envelope per Braids note voice"},
		{capability: "capability.prepared_engine_rack", contribution: "supplies the intentional second production preparer"},
		{capability: "capability.realtime_execution", contribution: "keeps native DSP work bounded and prepared across the callback boundary"},
	]
}
