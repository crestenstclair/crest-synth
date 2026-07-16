package crestsynth

project: {
	name: "crest-synth"
	layers: ["domain", "application", "infrastructure"]
	layerRules: {
		application: {dependsOn: ["domain"]}
		infrastructure: {dependsOn: ["domain", "application"]}
	}

	meta: {
		language: "rust"
		style: "idiomatic Rust; explicit domain types; small modules; ports at external and real-time boundaries"
		rules: [
			"one resource owns each public type and consumers import that type",
			"the standalone binary is a composition root; behavior lives behind domain and application abstractions",
			"test support uses the same ports, reducer, parameter bridge, event ring, engine, mixer, and audio callback as the running application",
		]
		avoid: [
			"synthesis engines other than SoundFont",
			"effects other than the one global reverb and one global delay",
			"sequencer, transport, timeline, pattern, clip, or song-editing domain models",
			"presets, sessions, modulation matrices, per-channel inserts, effect chains, buses, or plugin hosting",
			"panels, dashboards, meters, faders, custom widgets, custom drawing, themes, or multiple screens",
		]
	}

	validations: {
		format: {
			scope: "project"
			kind: "custom"
			command: ["cargo", "fmt", "--all", "--", "--check"]
			description: "Rust formatting is canonical"
		}
		clippy: {
			scope: "project"
			kind: "compiles"
			command: ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]
			description: "all targets compile without warnings"
		}
		test: {
			scope: "project"
			kind: "test"
			command: ["cargo", "test", "--all-targets"]
			description: "domain, reducer, adapter, and integration tests pass"
		}
		smoke: {
			scope: "project"
			kind: "integration"
			command: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke"]
			timeout: "180s"
			resources: [
				"adapter.HiDefSoundFontEngine",
				"adapter.CorridorsMidiEventSource",
				"applicationService.Testing.AutomaticMidiTest",
				"applicationService.Shell.StandaloneApplication",
				"asset.CrestSynthMain",
			]
			capabilities: [
				"capability.soundfont_audio",
				"capability.automatic_test_midi",
				"capability.one_way_parameter_control",
				"capability.realtime_execution",
			]
			goals: ["goal.play_test_song", "goal.control_synth"]
			description: "the fixed MIDI fixture drives the real SoundFont, control, mixer, and audio path"
		}
	}

	invariants: core: [
		{text: "AppState.apply is the only control-state mutation path", meta: rationale: "every input follows one reducer path"},
		{text: "input and view adapters emit AppEvents and never mutate application or engine state", meta: rationale: "adapters remain replaceable"},
		{text: "after an event is accepted, the application commits AppState before deriving serialized state, text, parameter snapshots, or audio commands", meta: rationale: "effects always describe accepted state"},
		{text: "the audio callback never allocates, locks, blocks, performs file or device discovery I/O, logs, or destroys owned state", meta: rationale: "the callback has a hard deadline"},
		{text: "AudioBoundary carries discrete MIDI commands, latest control values, and deferred destruction through bounded lock-free primitives", meta: rationale: "the real-time seam is explicit"},
		{text: "the only synthesis source is a SoundFont engine configured from ./sf2/HiDef.sf2", meta: rationale: "patch behavior has one unambiguous source"},
		{text: "the only effects are one reverb and one delay shared globally by every channel", meta: rationale: "the signal path stays small"},
		{text: "the MIDI-file module is an automatic test input adapter, not a sequencer or product transport", meta: rationale: "crest-synth remains an instrument"},
	]

	contextMap: [
		{from: "Kernel", to: "Synth", kind: "shared-kernel"},
		{from: "Kernel", to: "Control", kind: "shared-kernel"},
		{from: "Synth", to: "Control", kind: "customer-supplier", direction: "upstream"},
		{from: "Mixer", to: "Control", kind: "customer-supplier", direction: "upstream"},
		{from: "Control", to: "RealTime", kind: "anti-corruption", direction: "downstream"},
		{from: "Testing", to: "Control", kind: "anti-corruption", direction: "downstream"},
		{from: "Shell", to: "Control", kind: "anti-corruption", direction: "downstream"},
		{from: "Shell", to: "RealTime", kind: "anti-corruption", direction: "downstream"},
	]

	assetKinds: {
		"cargo-manifest": {description: "Cargo.toml", filePattern: "Cargo.toml"}
		"rust-library-root": {description: "Rust library root", filePattern: "src/lib.rs"}
		"rust-bin-target": {description: "Rust executable composition root", filePattern: "src/bin/*.rs"}
	}
}
