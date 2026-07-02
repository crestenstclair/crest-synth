package crestsynth

// Manifest — the crate manifest and the demo binary. This file is the ONLY
// place in the spec where crate dependencies are named; everywhere else the
// spec is language-profile-clean (adapters carry a framework name only).

project: assets: RootCargoToml: {
	kind:        "cargo-manifest"
	description: "Cargo.toml for the crest-synth crate"
	prompts: [
		"File path: Cargo.toml",
		"Package name crest-synth, edition 2021, a lib target plus binary targets under src/bin/.",
		"Dependencies and why each exists: cpal (audio output), midir (MIDI input), eframe + egui (GUI), gilrs (gamepad input), rtrb (lock-free SPSC ring buffer), triple_buffer (lock-free latest-wins parameter sharing), basedrop (deferred deallocation for real-time), serde + serde_json (preset/session serialization), symphonia (audio file decoding), midly (Standard MIDI File parsing).",
		"Choose current stable versions; the whole-tree gate (build/clippy/test) proves the resolution works.",
	]
}

project: assets: ToneTestMain: {
	kind:        "rust-bin-target"
	description: "src/bin/tone_test.rs: renders one second of A440 through the engine and asserts the output is audible"
	uses: ["domainService.Engine.EngineRenderer", "valueObject.Kernel.Frequency", "valueObject.Kernel.AudioFrame"]
	prompts: [
		"File path: src/bin/tone_test.rs",
		"Trigger a single A440 note through the engine, render one second of audio into a buffer, and MEASURE the peak absolute sample value of the rendered buffer.",
		#"Print exactly one line `peak=<value>` with the measured peak, then exit non-zero unless 0.1 < peak <= 1.0 — a silent or clipping render must fail the run."#,
	]
	validations: [
		{kind: "integration", command: ["cargo", "run", "--bin", "tone_test"], description: "renders an audible, non-clipping tone", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "peak="},
		]},
	]
}

project: assets: SynthUiMain: {
	kind:        "rust-bin-target"
	description: "src/bin/synth_ui.rs: the standalone synthesizer application — window, GUI views, gamepad navigation, and MIDI playback through the full engine"
	uses: [
		"port.Shell.AppWindow", "port.Shell.GuiRenderer", "port.Shell.GamepadInput",
		"port.Shell.AudioOutput", "port.Shell.MidiInput",
		"domainService.Engine.EngineRenderer", "domainService.Mixer.MixEngine",
		"domainService.Patch.MidiDispatcher", "domainService.MidiFile.Sequencer",
	]
	prompts: [
		"File path: src/bin/synth_ui.rs",
		"The standalone app: open the audio output and the window, render the GUI views, poll the gamepad for navigation, and play notes from connected MIDI inputs and/or a MIDI file through the full engine-to-mixer signal path.",
		#"--play <FILE.mid>: load the file via the MidiFileReader port and sequence it through the engine, looping until quit."#,
		#"--smoke: headless self-check with no window and no audio device — build the full stack (dispatcher, engine, mixer), sequence the first seconds of the --play file (or a synthetic note-on if none was given), render blocks through the SAME render path the live app uses, MEASURE the peak absolute sample and the count of dispatched events, print exactly one line `peak=<value>` and one line `events=<count>`, and exit non-zero unless 0.05 < peak <= 1.0 and events > 0."#,
	]
	validations: [
		{kind: "integration", command: ["cargo", "run", "--bin", "synth_ui", "--", "--smoke", "--play", "midi/Megalovania.mid"], description: "headless smoke: a real MIDI file drives audible, non-clipping output through the full engine", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "peak="},
			{kind: "stdout_contains", pattern: "events="},
		]},
	]
}
