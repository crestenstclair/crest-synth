package crestsynth

// Sample — sample playback and soundfonts with key/velocity zone mapping.

project: contexts: Sample: purpose: "sample-based sound sources: WAV and SF2 loading, zone mapping by key and velocity range, pitched playback with interpolation and looping"

project: contexts: Sample: valueObjects: {
	InterpolationMode: {description: "resampling quality: none (nearest), linear, cubic, or sinc"}
	LoopMode: {description: "playback looping: no loop, forward, ping-pong, or release (loop until note-off then play to end)"}
	KeyRange: {state: {low: "NoteNumber", high: "NoteNumber"}, description: "inclusive note range a zone responds to", invariants: ["low must be <= high"], validations: [{kind: "test", command: ["cargo", "test", "key_range"], description: "KeyRange unit tests pass"}]}
	VelocityRange: {state: {low: "Velocity", high: "Velocity"}, description: "inclusive velocity range a zone responds to", invariants: ["low must be <= high"], validations: [{kind: "test", command: ["cargo", "test", "velocity_range"], description: "VelocityRange unit tests pass"}]}
	SoundFontInstrument: {
		state: {bank: "u16", program: "u8", percussion: "bool", name: "string"}
		description: "the SF2 preset selector derived from a MIDI InstrumentIdentity; bank/program values select one instrument in HiDef.sf2"
		invariants: ["program is 0..=127", "percussion selects the SoundFont percussion bank when present", "name records the resolved SF2 preset name"]
		validations: [{kind: "test", command: ["cargo", "test", "soundfont_instrument"], description: "melodic, banked, percussion, and fallback selectors are stable"}]
	}
	SampleData: {
		state: {channels: "u16", sampleRate: "SampleRate", frames: "Arc<[AudioFrame]>"}
		description: "immutable decoded PCM shared by zones and players"
		invariants: ["channels is 1 or 2", "frames are finite"]
	}
	Zone: {
		state: {sample: "SampleData", keys: "KeyRange", velocities: "VelocityRange", rootKey: "NoteNumber", fineTuneCents: "f64", gain: "Amplitude", pan: "Pan", loopMode: "LoopMode"}
		description: "maps a key range + velocity range to one sample with per-zone playback settings"
	}
}

project: contexts: Sample: aggregates: SampleSet: {
	root:    true
	purpose: "a named collection of zones backing one instrument sound"
	state: {id: "SampleSetId", name: "string", zones: "list<Zone>", interpolation: "InterpolationMode"}
	commands: {
		AddZone: {zone: "Zone"}
		RemoveZone: {index: "u32"}
	}
	events: {
		ZoneAdded: {index: "u32"}
		ZoneRemoved: {index: "u32"}
	}
	invariants: [
		"resolving a note returns every zone whose key range and velocity range both match",
	]
	validations: [{kind: "test", command: ["cargo", "test", "sample_set"], description: "SampleSet unit tests pass"}]
	contributesTo: [{capability: "capability.configurable_instrument_graph", contribution: "owns canonical decoded sample references, zone mappings, and interpolation policy for sample-based patches"}]
}

project: contexts: Sample: ports: {
	SampleLoader: {
		direction: "outbound"
		contract: {
			loadWav: "(path: Path) -> result<SampleSet, LoadError>"
		}
	}
	SoundFontPlugin: {
		direction: "outbound"
		contract: {
			open: "(path: Path) -> result<SoundFontHandle, SoundFontError>"
			loadInstrument: "(font: &SoundFontHandle, instrument: SoundFontInstrument) -> result<SampleSet, SoundFontError>"
		}
		meta: notes: "a built-in instrument-source plugin, not VST/CLAP/AU hosting; file parsing and preset materialization occur on the control thread"
	}
	SampleStore: {
		direction: "outbound"
		contract: {
			put: "(id: SampleSetId, set: SampleSet) -> result<(), StoreError>"
			get: "(id: SampleSetId) -> result<option<SampleSet>, StoreError>"
		}
	}
}

project: contexts: Sample: domainServices: {
	ZoneResolver: {
		purpose: "given a note and velocity, finds every matching zone in a sample set"
		uses: ["aggregate.Sample.SampleSet"]
		validations: [{kind: "test", command: ["cargo", "test", "zone_resolver"], description: "key and velocity matching returns exactly the applicable zones"}]
	}
	SamplePlayer: {
		purpose: "plays a zone's sample at the correct pitch with the configured interpolation and loop mode"
		uses: ["aggregate.Sample.SampleSet", "domainService.Sample.ZoneResolver"]
			validations: [{kind: "test", command: ["cargo", "test", "sample_player"], description: "SamplePlayer unit tests pass"}]
		contributesTo: [{capability: "capability.configurable_instrument_graph", contribution: "renders canonical matched sample zones at the requested pitch, interpolation, and loop mode"}]
	}
}

project: adapters: SymphoniaSampleLoader: {
	implements: "port.Sample.SampleLoader"
	layer:      "infrastructure"
	profile: {kind: "persistence", medium: "user-selected audio file"}
	meta: framework: "symphonia"
	validations: [{kind: "test", command: ["cargo", "test", "symphonia_sample_loader"], description: "decoded mono/stereo PCM becomes canonical SampleData without duplicate sample models"}]
}

project: adapters: HiDefSoundFontPlugin: {
	implements: "port.Sample.SoundFontPlugin"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "SoundFont 2"}
	meta: {
		framework: "rustysynth"
			rules: [
			"open exactly ./sf2/HiDef.sf2 once on the control thread for a playback plan and fail clearly when it is missing or invalid",
			"resolve the requested bank/program or percussion preset before rendering; never substitute a virtual-analog oscillator",
			"load/index on the control thread and expose immutable prepared sample data to SamplePlayer; no file I/O, allocation, lock, or destruction occurs in the audio callback",
		]
	}
	validations: [{kind: "integration", command: ["cargo", "test", "hidef_soundfont_plugin"], description: "HiDef.sf2 resolves two distinct melodic instruments plus percussion and each renders non-silent bounded samples through SamplePlayer"}]
	contributesTo: [{capability: "capability.instrument_partitioned_test_playback", contribution: "materializes each MIDI instrument from the fixed HiDef.sf2 SoundFont for test playback"}]
}
