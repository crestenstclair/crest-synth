package crestsynth

project: assets: CargoManifest: {
	kind: "cargo-manifest"
	description: "Cargo.toml for the standalone SoundFont synth"
	profile: {kind: "configuration", ecosystem: "cargo"}
	targets: [
		"adapter.HiDefSoundFontEngine",
		"adapter.GlobalReverbDelay",
		"adapter.LockFreeAudioBoundary",
		"adapter.CorridorsMidiEventSource",
		"adapter.EframeTextWindow",
		"adapter.CpalAudioOutput",
	]
	prompts: [
		"Create Cargo.toml for one Rust library plus the crest-synth binary.",
		"Use only dependencies required by the declared resources: rustysynth, midly, cpal, eframe/egui, rtrb, triple_buffer, basedrop, serde with derive, serde_json, thiserror, and anyhow. Development-only test helpers may be added when required by the declared tests.",
		"Do not add synthesis, effect, GUI-widget, sequencing, persistence, plugin, database, networking, or async-runtime libraries.",
	]
	validations: [{kind: "custom", command: ["cargo", "metadata", "--no-deps", "--format-version", "1"], description: "the manifest resolves"}]
}

project: assets: LibraryRoot: {
	kind: "rust-library-root"
	description: "src/lib.rs exposing the declared bounded-context modules"
	targets: [
		"valueObject.Kernel.PatchId",
		"valueObject.Kernel.MidiChannel",
		"valueObject.Kernel.MidiMessage",
		"aggregate.Synth.Patch",
		"port.Synth.SoundFontEngine",
		"valueObject.Mixer.ChannelParameters",
		"valueObject.Mixer.GlobalParameters",
		"domainService.Mixer.MixEngine",
		"valueObject.RealTime.ParameterSnapshot",
		"valueObject.RealTime.AudioCommand",
		"applicationService.RealTime.AudioRenderer",
		"valueObject.Control.AppEvent",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"port.Testing.MidiEventSource",
		"applicationService.Testing.AutomaticMidiTest",
		"applicationService.Shell.StandaloneApplication",
	]
	prompts: [
		"Create src/lib.rs and small snake_case modules grouped as kernel, synth, mixer, real_time, control, testing, and shell.",
		"Each declared resource owns one public type or service. Re-export only the types required at context boundaries; do not create local duplicate models in adapters, tests, or the binary.",
		"The testing module is part of the crate only to drive automatic MIDI input. It must not expose a sequencer or transport API.",
	]
}

project: assets: CrestSynthMain: {
	kind: "rust-bin-target"
	description: "src/bin/crest_synth.rs, the thin standalone composition root and smoke harness"
	profile: {kind: "infrastructure", witness: "SoundFont synth vertical slice", failurePolicy: "fail startup on missing fixture, SoundFont, audio setup, or invalid configuration"}
	targets: [
		"applicationService.Shell.StandaloneApplication",
		"adapter.HiDefSoundFontEngine",
		"adapter.GlobalReverbDelay",
		"adapter.LockFreeAudioBoundary",
		"adapter.CorridorsMidiEventSource",
		"adapter.EframeTextWindow",
		"adapter.CpalAudioOutput",
	]
	prompts: [
		"Create src/bin/crest_synth.rs as composition only. Construct the concrete adapters, inject them into StandaloneApplication, and call run.",
		"Normal invocation expects ./sf2/HiDef.sf2 and ./midi/Corridors of Time - Chrono Trigger.mid and begins MIDI automatically. There is no play command, transport, file chooser, alternate engine, or alternate effect selection.",
		"Support --smoke for headless deterministic execution, --observe to print one CREST_OBSERVATION JSON object, and --degenerate-audio or --degenerate-control only with --smoke --observe for behavioral falsification. Reject other options.",
		"The smoke path loads the real fixed SoundFont and MIDI fixture, installs all instrument Patches, applies keyboard-equivalent navigation and adjustment events, renders bounded audio blocks, and reports measurements from the production services.",
	]
	validations: [
		{kind: "integration", command: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke"], timeout: "180s", description: "the complete headless path runs against HiDef.sf2 and Corridors of Time"},
	]
	contributesTo: [
		{capability: "capability.soundfont_audio", contribution: "starts the concrete SoundFont synth"},
		{capability: "capability.automatic_test_midi", contribution: "starts the fixed MIDI test input without transport controls"},
		{capability: "capability.one_way_parameter_control", contribution: "hosts the text control loop without owning behavior"},
		{capability: "capability.realtime_execution", contribution: "composes the prepared control and audio sides"},
	]
}
