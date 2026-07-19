package crestsynth

project: contexts: Synth: {
	purpose: "Patch identity and the SoundFont synthesis boundary"
	ubiquitousLanguage: {
		Patch: "one playable instrument configuration"
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
		purpose: "own one instrument's identity, SoundFont preset, assigned channel, and editable mixer parameters"
		state: {
			id: "PatchId"
			name: "String"
			instrument: "SoundFontInstrument"
			channel: "MidiChannel"
			parameters: "ChannelParameters"
		}
		invariants: [
			"id is stable for the process lifetime",
			"instrument configuration is immutable after the Patch is installed",
			"channel is assigned by the input adapter and is in 0..=15",
			"only ChannelParameters may be edited after installation",
		]
		contributesTo: [
			{capability: "capability.soundfont_audio", contribution: "binds one playable Patch to one HiDef.sf2 instrument"},
			{capability: "capability.one_way_parameter_control", contribution: "is the unit listed and edited by the text view"},
		]
	}

	ports: SoundFontEngine: {
		direction: "outbound"
		contract: {
			load: "(path: &Path) -> Result<(), SoundFontError>"
			configurePatch: "(&Patch) -> Result<(), SoundFontError>"
			dispatch: "(PatchId, MidiMessage) -> Result<(), SoundFontError>"
			renderPatches: "(&mut PatchAudioBlock, &ParameterSnapshot)"
			allNotesOff: "()"
		}
		consumes: [
			"aggregate.Synth.Patch",
			"valueObject.Kernel.MidiMessage",
			"valueObject.RealTime.ParameterSnapshot",
			"valueObject.RealTime.PatchAudioBlock",
		]
		invariants: [
			"the running application owns exactly one SoundFontEngine instance",
			"load and configurePatch run on the control thread before the Patch can receive a note",
			"every configured Patch has a unique assigned MIDI channel and a unique output stem indexed by PatchId",
			"dispatch and renderPatches use bounded preallocated storage on the audio thread",
			"renderPatches fills caller-owned per-Patch stereo stems and never returns a combined master stream in place of those stems",
			"renderPatches performs no allocation, locking, I/O, logging, or destruction",
		]
		contributesTo: [
			{capability: "capability.soundfont_audio", contribution: "keeps SoundFont synthesis behind the engine abstraction later implementations must satisfy"},
			{capability: "capability.realtime_execution", contribution: "makes the callback contract explicit at the synthesis boundary"},
		]
	}

}

project: adapters: HiDefSoundFontEngine: {
	implements: "port.Synth.SoundFontEngine"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "sf2", system: "HiDef.sf2"}
	meta: {
		framework: "rustysynth"
		rules: [
			"expect exactly ./sf2/HiDef.sf2 and load it once on the control thread; return a clear startup error if it is missing or invalid",
			"own exactly one HiDefSoundFontEngine adapter and parse one SoundFont bank shared by all render lanes; do not create per-Patch SoundFontEngine objects",
			"because rustysynth 1.3 exposes only a combined stereo render, prepare one bounded rustysynth synthesizer lane per configured MIDI channel inside that adapter so each Patch is rendered into a distinct caller-owned stem",
			"SoundFont is the only synthesis implementation; do not define EngineType, oscillator, virtual-analog, sampler, or fallback paths",
			"configure each Patch's unique assigned channel lane from SoundFontInstrument bank, program, and percussion identity; keep the fixed PatchId-to-channel-to-stem lookup preallocated",
			"inside each independent lane use rustysynth's percussion channel for a percussion Patch and a melodic channel for every other Patch, regardless of the Patch's logical assigned channel",
			"dispatch routes the targeted Patch's MIDI message only to its assigned lane and its prepared internal melodic or percussion channel without allocation or locking",
			"render every active lane into its matching PatchAudioBlock stem; never render all lanes to one buffer and associate that buffer with the first Patch",
			"disable rustysynth's built-in reverb and chorus so the declared global effects are the only effects",
			"renderPatches uses caller-owned stems without callback allocation, locking, I/O, logging, or deallocation",
		]
	}
	validations: [{kind: "test", command: ["cargo", "test", "hidef_soundfont_engine"], description: "two simultaneous melodic or percussion Patches use unique lanes and produce distinct non-silent bounded stems; silencing one stem leaves the other unchanged"}]
	contributesTo: [
		{capability: "capability.soundfont_audio", contribution: "implements the only synthesis engine using ./sf2/HiDef.sf2"},
		{capability: "capability.realtime_execution", contribution: "renders prepared SoundFont voices inside the callback contract"},
	]
}
