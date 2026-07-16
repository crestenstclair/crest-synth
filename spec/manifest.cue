package crestsynth

// Operational artifacts are first-class assets. `targets` is the dependency
// field crest-spec follows when building generation context and execution waves.
project: assets: RootCargoToml: {
	kind: "cargo-manifest"
	description: "Cargo.toml for the standalone crest-synth crate and its proof binaries"
	profile: {kind: "build_manifest", ecosystem: "cargo", constraint: "one library crate plus explicit src/bin proof and host targets"}
	prompts: [
		"File path: Cargo.toml. Package crest-synth, Rust 2021, one library plus binaries under src/bin.",
		"Dependencies: cpal, midir, eframe/egui, gilrs, rtrb, triple_buffer, basedrop, serde/serde_json, symphonia, midly, and rustysynth for the built-in SoundFont instrument plugin. Do not add parallel frameworks for responsibilities these dependencies already cover.",
		"Use an eframe/egui release whose winit/objc2 stack works on current macOS; never pin the known-broken eframe 0.27 / winit 0.29 / objc2 beta chain.",
	]
	validations: [{kind: "compiles", command: ["cargo", "metadata", "--no-deps"], description: "the manifest resolves and declares all targets"}]
}

project: assets: ToneTestMain: {
	kind: "rust-bin-target"
	description: "src/bin/tone_test.rs: fast measured virtual-analog render smoke"
	profile: {kind: "verification_harness", witness: "audible bounded tone", failurePolicy: "non-zero for silence or clipping"}
	targets: ["domainService.Engine.VoiceAllocator", "domainService.Engine.EngineRenderer", "valueObject.Kernel.AudioFrame"]
	prompts: [
		"File path: src/bin/tone_test.rs. Use canonical Engine and Kernel types; do not define a local voice, renderer, frequency, or audio-frame type.",
		"Trigger A440, render one second, calculate the absolute stereo peak, print `peak=<value>`, and exit non-zero unless 0.1 < peak <= 1.0.",
	]
	validations: [{kind: "integration", command: ["cargo", "run", "--bin", "tone_test"], description: "the canonical engine produces audible bounded audio", assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "peak="}]}]
	contributesTo: [{capability: "capability.polyphonic_sound_generation", contribution: "provides a fast non-silent render smoke over the canonical engine"}]
}

project: assets: SynthUiMain: {
	kind: "rust-bin-target"
	description: "src/bin/synth_ui.rs: thin CLI and eframe composition root for the mixer-only standalone application"
	profile: {kind: "verification_harness", witness: "standalone live, smoke, and autopilot host", failurePolicy: "unknown or contradictory modes fail clearly"}
	targets: [
		"applicationService.Loop.StandaloneApplication", "applicationService.Loop.SceneRunner", "applicationService.MidiFile.TestPlaybackAssembler",
		"adapter.CpalAudioOutput", "adapter.MidirMidiInput", "adapter.EframeAppWindow", "adapter.EguiRenderer", "adapter.GilrsGamepadInput",
		"adapter.RtrbEventRing", "adapter.TripleBufferParameterBridge", "adapter.BasedropDeferredDeallocator",
		"adapter.MidlyMidiFileReader", "adapter.HiDefSoundFontPlugin", "adapter.SerdeSnapshotCodec",
	]
	prompts: [
		"File path: src/bin/synth_ui.rs. Keep this file a thin argument parser, adapter constructor, and eframe/cpal host. Application state, event handling, MIDI dispatch, rendering, smoke logic, and scene execution belong to the targeted application services.",
		"Do not declare another AppState, scene parser, audio graph, MIDI event, patch, channel strip, or render function. Do not mutate MixerView or ChannelStrip directly: translate input into AppEvent and call StandaloneApplication.handleEvent.",
		"The initial and only view is intentionally a big wall of text. Ask StandaloneApplication.mixerTextView for MixerTextProjection and display its body in one stock egui vertical ScrollArea using default monospace Labels. Do not construct any panels, columns, grids, tables, meters, faders, inspectors, dashboards, toolbars, styled menus, theme/token system, custom widgets, custom painting, animation, iconography, or substantial layout code.",
		"Render every Patch as a multi-line block containing its exact serialized id, name, mixerStrip, volume, reverbSend, echoSend, pan, mute, and solo values. Separate blocks with the literal ASCII horizontal rule from MixerTextProjection. Render the projection's `>` selection marker verbatim and keep that line visible by scrolling; the shell owns no second selection or value model.",
		"Bare W/S move between values and bare A/D move between Patch blocks. K+W/S/A/D emit the corresponding Adjust event. L emits Playback(ToggleFromStart). Translate all keys to AppEvents and call StandaloneApplication.handleEvent; the key handler never changes Patch, MixerView, TestPlayback, serialized text, parameters, or audio directly.",
		"The first text lines show serialized playback status and `KEYS: W/S values | A/D patches | K+direction edit | L start/stop from start`. These are ordinary labels, not a menu component.",
		"The cpal callback synchronously renders exactly its requested frame slice through StandaloneApplication.renderAudio. It never uses wall-clock pacing, guessed blocks, silence stubs, locks, allocation, I/O, or callback deallocation. The non-Send stream remains on its owning thread.",
		"CLI: synth_ui [--smoke | --autopilot] [--seconds N] [--play FILE.mid] [--scene FILE] [--loop-scene] [--observe] [--degenerate-stub]. Reject unknown and contradictory options.",
		"--play FILE delegates to MidlyMidiFileReader and TestPlaybackAssembler. The assembler must use HiDefSoundFontPlugin with exactly ./sf2/HiDef.sf2, select each part's bank/program or percussion instrument, create an EngineType::Sample Patch per part, assign part N to mixer track N % 16, and install the plan through AppEvents. Print the SoundFont path, resolved instrument, Patch, and track assignment for every part.",
		"--smoke opens no device/window and calls StandaloneApplication.runSmoke. It must assert that every Patch appears in MixerTextProjection with exact serialized values; a K+direction edit changes only the selected AppState value; StateSnapshot round-trips; the matching ParameterSnapshot is published and consumed; and rendered audio changes as expected. With --play, it additionally proves HiDef.sf2 instrument resolution and L start/stop/restart from event zero.",
		"--autopilot opens the real window/audio path but evaluates backend behavior, not layout: send W/S/A/D, K+direction, and L through the same AppEvent facade, then assert serialized state, text projection, published parameters, SoundFont playback state, and device-bound audio all agree before self-terminating. Do not capture or compare UI screenshots.",
		"--scene decodes via SnapshotCodec and delegates to SceneRunner. Windowed mode paces captions for observation; smoke mode runs headlessly. Combining --play and --scene means the MIDI song is the audio source while scene events manipulate the mixer.",
		"--degenerate-stub is accepted only with --observe and deliberately replaces one seam with a schema-compatible no-op so crest-spec theater detection can reject it.",
	]
	validations: [
		{kind: "integration", command: ["make", "ui-smoke"], description: "the text projection, one-way edit path, serialization, parameter publication, and render consumption work headlessly", assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "patch rows serialized: true"}, {kind: "stdout_contains", pattern: "state roundtrip: true"}, {kind: "stdout_contains", pattern: "parameter published: true"}, {kind: "stdout_contains", pattern: "engine consumed edit: true"}, {kind: "stdout_contains", pattern: "audio changed: true"}]},
		{kind: "integration", command: ["cargo", "run", "--bin", "synth_ui", "--", "--smoke", "--play", "midi/Corridors of Time - Chrono Trigger.mid"], description: "format-1 instruments resolve from HiDef.sf2, become sample Patches, round-robin assign, and restart from zero", assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "soundfont=./sf2/HiDef.sf2"}, {kind: "stdout_contains", pattern: "instrument parts="}, {kind: "stdout_contains", pattern: "generated patches="}, {kind: "stdout_contains", pattern: "track assignment:"}, {kind: "stdout_contains", pattern: "restart from zero: true"}, {kind: "stdout_contains", pattern: "peak="}]},
	]
	contributesTo: [
		{capability: "capability.external_midi_performance", contribution: "hosts MIDI and device adapters around the standalone application facade"},
		{capability: "capability.pointer_free_mixer_control", contribution: "renders the disposable serialized text projection while forwarding W/S/A/D and K+direction as AppEvents"},
		{capability: "capability.instrument_partitioned_test_playback", contribution: "hosts HiDef.sf2-backed per-instrument plans and L-key start/stop-from-beginning through AppEvents"},
		{capability: "capability.behavioral_proof_harness", contribution: "provides hermetic smoke, real-device autopilot, and falsification-gated observation modes"},
	]
}

project: assets: SceneRunMain: {
	kind: "rust-bin-target"
	description: "src/bin/scene_run.rs: thin headless CLI over SnapshotCodec and SceneRunner"
	profile: {kind: "verification_harness", witness: "deterministic production-path scene replay", failurePolicy: "reject malformed scenes and any unexpected event rejection"}
	targets: ["applicationService.Loop.SceneRunner", "adapter.SerdeSnapshotCodec"]
	prompts: [
		"File path: src/bin/scene_run.rs. CLI: --scene FILE [--dump-every-step] [--observe] [--degenerate-stub]. Decode the canonical Scene and call SceneRunner; do not define a local event model, reducer, snapshot extractor, or renderer.",
		"Print the canonical final StateSnapshot and `events_applied=<N> rejections=<M> blocks_rendered=<B> peak=<P>`. Exit non-zero for malformed input or unexpected rejection.",
		"--observe executes the same scene twice and emits the declared SceneResult fields as CREST_OBSERVATION JSON. --degenerate-stub runs the same harness with an explicit no-op reducer/render seam so at least one predicate fails.",
	]
	validations: [{kind: "integration", command: ["make", "demo-scenes"], description: "all committed scenes execute and assert measured facts", assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "events_applied="}]}]
	contributesTo: [
		{capability: "capability.deterministic_scene_replay", contribution: "exposes the production SceneRunner as a reproducible headless command"},
		{capability: "capability.behavioral_proof_harness", contribution: "emits measured observations and a falsifiable no-op comparison"},
	]
}

project: assets: SceneLibrary: {
	kind: "scene-library"
	description: "scenes/: five versioned scenarios that combine into the product showcase and assert the shared reducer/render path"
	profile: {kind: "verification_harness", witness: "serialized application behavior", failurePolicy: "every assertion reads canonical snapshot or SceneResult data"}
	targets: ["asset.SceneRunMain", "valueObject.Loop.Scene", "valueObject.Loop.SceneResult"]
	prompts: [
		"Directory scenes/. Create five JSON Scene files in the SerdeSnapshotCodec format plus check.sh. A scene contains canonical AppEvents; phases are not product behavior and must not appear in scene names or formats.",
		"mixer-solo.json: render non-zero signals, solo one strip, and assert solo isolation while all input strips continue to meter.",
		"volume-edit.json: navigate through AppEvent::Mixer, apply K-equivalent Adjust events to lower volume exactly 6 dB, and assert the canonical MixerView/ChannelStrip snapshot, serialized projection, and published parameter value.",
		"voice-steal.json: use AppEvent::Patch to configure polyphony 2 and oldest stealing, then AppEvent::Midi for three overlapping notes with render blocks; assert two active voices, at least one steal observation, and non-zero peak.",
		"preset-roundtrip.json: edit complete patch/session state through AppEvent::Patch, save through AppEvent::Preset, mutate, restore, and assert the restored state equals the saved snapshot. This requires Patch and Preset variants; an event-vocabulary sweep is not a round-trip proof.",
		"showcase.json: combine the supported behaviors into a text-only windowed journey with optional MIDI-file music: Playback ToggleFromStart, solo/unsolo, pan left/right, volume dip/restore, mute/unmute, another Playback ToggleFromStart to stop/rewind, captions, and renderBlocks between transitions.",
		"check.sh runs scene_run and uses jq over StateSnapshot/SceneResult data. It asserts exact event counts, zero unexpected rejections, deterministic replay, changed state, rendered blocks, and non-zero audio where the scene claims sound. Never accept an unconditional success token.",
	]
	validations: [{kind: "integration", command: ["make", "demo-scenes"], description: "the combined scene fixture proves reducer, persistence, polyphony, mixer, and audio behavior", assertions: [{kind: "exit_code", expected: 0}]}]
	contributesTo: [
		{capability: "capability.deterministic_scene_replay", contribution: "provides inspectable scenarios over the complete AppEvent vocabulary"},
		{capability: "capability.shared_control_reducer", contribution: "proves serialized and live-equivalent events share AppState semantics"},
	]
}

project: assets: BuildMakefile: {
	kind: "makefile"
	description: "Makefile: stable build, run, proof, scene, and observation entry points"
	profile: {kind: "configuration", ecosystem: "make", constraint: "portable recipes with quoted paths"}
	targets: [
		"asset.SynthUiMain", "asset.ToneTestMain", "asset.VoiceDemoMain", "asset.SamplePlayDemoMain", "asset.EffectsDemoMain",
		"asset.ModPlayMain", "asset.PatchPlayMain", "asset.PresetRoundtripDemoMain", "asset.MidiPlayMain", "asset.MidiPlayLiveMain",
		"asset.MixerDemoMain", "asset.GamepadNavDemoMain", "asset.SceneRunMain", "asset.SceneLibrary",
	]
	prompts: [
		"File path: Makefile. Provide help, build, test, lint, fmt, ui, play, smoke, ui-smoke, autopilot, watch, scene, demo-scenes, tone, demo-voices, demo-samples, demo-effects, demo-mod, demo-patches, demo-presets, demo-midi, check-live, demo-mixer, check-gamepad, and proofs.",
		"proofs runs every hermetic proof: test, tone, all demo-* targets, check-live, check-gamepad, ui-smoke, and demo-scenes. Autopilot is excluded from hermetic proofs because it requires a display and device, but retains its own target and goal validation.",
		"watch runs synth_ui with scenes/showcase.json and the default MIDI file for human observation. Quote every path variable because MIDI filenames contain spaces.",
	]
	validations: [
		{kind: "custom", command: ["make", "-n", "proofs"], description: "the aggregate proof target exists"},
		{kind: "integration", command: ["make", "smoke"], description: "the stable smoke entry point produces measured audio", assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "peak="}]},
	]
	contributesTo: [{capability: "capability.behavioral_proof_harness", contribution: "provides the stable aggregate command that executes every hermetic subsystem and vertical-slice proof"}]
}
