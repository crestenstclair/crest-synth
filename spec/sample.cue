package crestsynth

// Sample — sample playback and soundfonts with key/velocity zone mapping.

project: contexts: Sample: purpose: "sample-based sound sources: WAV and SF2 loading, zone mapping by key and velocity range, pitched playback with interpolation and looping"

project: contexts: Sample: valueObjects: {
	InterpolationMode: {description: "resampling quality: none (nearest), linear, cubic, or sinc"}
	LoopMode: {description: "playback looping: no loop, forward, ping-pong, or release (loop until note-off then play to end)"}
	KeyRange: {state: {low: "NoteNumber", high: "NoteNumber"}, description: "inclusive note range a zone responds to", invariants: ["low must be <= high"], validations: [{kind: "test", command: ["cargo", "test", "key_range"], description: "KeyRange unit tests pass"}]}
	VelocityRange: {state: {low: "Velocity", high: "Velocity"}, description: "inclusive velocity range a zone responds to", invariants: ["low must be <= high"], validations: [{kind: "test", command: ["cargo", "test", "velocity_range"], description: "VelocityRange unit tests pass"}]}
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
			loadSf2: "(path: Path) -> result<list<SampleSet>, LoadError>"
		}
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
