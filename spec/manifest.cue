package crestsynth

// Operational artifacts are first-class assets. `targets` is the dependency
// field crest-spec follows when building generation context and execution waves.
project: assets: RootCargoToml: {
	kind: "cargo-manifest"
	description: "Cargo.toml for the standalone crest-synth crate and its proof binaries"
	profile: {kind: "build_manifest", ecosystem: "cargo", constraint: "one library crate plus explicit src/bin proof and host targets"}
	prompts: [
		"File path: Cargo.toml. Package crest-synth, Rust 2021, one library plus binaries under src/bin.",
		"Dependencies: cpal, midir, eframe/egui, gilrs, rtrb, triple_buffer, basedrop, serde/serde_json, symphonia, and midly. Do not add parallel frameworks for responsibilities these dependencies already cover.",
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
		"adapter.MidlyMidiFileReader", "adapter.SerdeSnapshotCodec", "domainService.DesignSystem.DefaultTheme",
	]
	prompts: [
		"File path: src/bin/synth_ui.rs. Keep this file a thin argument parser, adapter constructor, and eframe/cpal host. Application state, event handling, MIDI dispatch, rendering, smoke logic, and scene execution belong to the targeted application services.",
		"Do not declare another AppState, scene parser, audio graph, MIDI event, patch, channel strip, or render function. Do not mutate MixerView or ChannelStrip directly: translate input into AppEvent and call StandaloneApplication.handleEvent.",
		"Current GUI scope is exactly one mixer overview. The initial frame shows ALL sixteen narrow tracks T00-T0F at once, a right-hand inspector for the cursor, and two bottom terminal-style status/help rows. There is no horizontal paging, view switching, patch/preset/modulation screen, on-screen keyboard, mouse interaction, or touch interaction.",
		"Keyboard W/S/A/D and gamepad d-pad emit identical navigation events. Holding J / gamepad select is momentary edit mode; double-tap emits ToggleFocusedParam. Timing is adapter state, while mixer behavior remains in MixerView.",
		"VISUAL DIRECTION: look like a compact terminal mixer rather than a conventional DAW. Use a monospaced font, hard rectangular grid lines, a near-black background, restrained semantic accent colors, uppercase short labels, hex-like track codes, and text/segmented readouts. Avoid knobs, bevels, gradients, shadows, glossy faders, ornamental animation, and large decorative graphics.",
		"MAIN GRID: reserve a narrow row-label gutter, then render sixteen equal narrow track columns T00-T0F. Each header shows its Txx code and a clipped one-line patch/instrument label. Volume occupies the dominant vertical cell but is a simple segmented/textual level bar, not a skeuomorphic fader. Beneath it show compact Pan, Mute, Solo, Reverb, and Echo cells. Every track meters before solo gating.",
		"INSPECTOR + FOOTER: the right pane shows CURSOR (Txx + parameter), PATCH/INSTRUMENT, VALUE in compact control notation, and STATE (mute/solo). The first bottom line repeats cursor and track status plus `16 TRACKS`; the final line lists current keyboard/gamepad commands and exit/back. These are pure projections of MixerView, never separate mutable state.",
		"CONTROL DISPLAY: volume/sends display 00-7F, pan displays L63..C..R63, toggles display on/off, while the domain keeps Decibel/Amplitude/Pan types. Initial selection is T00 VOL. At the 1280x800 Steam Deck target, all sixteen track headers/columns and the inspector must remain present without clipping or scroll; shorten labels before removing structure.",
		"Resolve every draw color through Theme/SemanticToken. UI code is a pure projection of AppState and reports semantic events only.",
		"The cpal callback synchronously renders exactly its requested frame slice through StandaloneApplication.renderAudio. It never uses wall-clock pacing, guessed blocks, silence stubs, locks, allocation, I/O, or callback deallocation. The non-Send stream remains on its owning thread.",
		"CLI: synth_ui [--smoke | --autopilot] [--seconds N] [--play FILE.mid] [--scene FILE] [--loop-scene] [--observe] [--degenerate-stub]. Reject unknown and contradictory options.",
		"--play FILE delegates to MidlyMidiFileReader and TestPlaybackAssembler: discover instrument parts, create one canonical Patch per part, assign part N to mixer track N % 16 in first-event order, install patches through AppState, and schedule patch-targeted events. Print `instrument parts=<N>`, `generated patches=<N>`, and one `track assignment: <instrument> -> Txx` line per part. This is test orchestration, not a sequencer feature.",
		"--smoke opens no device/window and calls StandaloneApplication.runSmoke. Use the complete instrument-partitioned plan when --play is supplied or a synthetic note otherwise; assert events > 0, 0.05 < peak <= 1, at least one track meters, reducer frame advances, all sixteen labels exist, the initial inspector selects T00 VOL, and all theme tokens resolve. Print `tracks visible: 16` and `inspector: T00 VOL` plus stable audio lines and the declared CREST_OBSERVATION JSON in --observe mode.",
		"--autopilot opens the real window/audio path, injects deterministic notes and AppEvents through the same facade, navigates T00-T0F, proves all sixteen complete track columns and the right inspector fit simultaneously, captures autopilot.png, reports device-bound peak, prints `autopilot tracks visible: 16` and `autopilot inspector visible: true`, and self-terminates after assertions.",
		"--scene decodes via SnapshotCodec and delegates to SceneRunner. Windowed mode paces captions for observation; smoke mode runs headlessly. Combining --play and --scene means the MIDI song is the audio source while scene events manipulate the mixer.",
		"--degenerate-stub is accepted only with --observe and deliberately replaces one seam with a schema-compatible no-op so crest-spec theater detection can reject it.",
	]
	validations: [
		{kind: "integration", command: ["make", "ui-smoke"], description: "headless application construction, all-track layout projection, reducer dispatch, theme, metering, and production render path work", assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "tracks visible: 16"}, {kind: "stdout_contains", pattern: "inspector: T00 VOL"}, {kind: "stdout_contains", pattern: "render non-silent: true"}, {kind: "stdout_contains", pattern: "track metered: true"}, {kind: "stdout_contains", pattern: "theme tokens resolved: 10"}]},
		{kind: "integration", command: ["cargo", "run", "--bin", "synth_ui", "--", "--smoke", "--play", "midi/Corridors of Time - Chrono Trigger.mid"], description: "format-1 input is partitioned by instrument, materialized as patches, round-robin assigned, and rendered", assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "instrument parts="}, {kind: "stdout_contains", pattern: "generated patches="}, {kind: "stdout_contains", pattern: "track assignment:"}, {kind: "stdout_contains", pattern: "events="}, {kind: "stdout_contains", pattern: "peak="}]},
	]
	contributesTo: [
		{capability: "capability.external_midi_performance", contribution: "hosts MIDI and device adapters around the standalone application facade"},
		{capability: "capability.pointer_free_mixer_control", contribution: "renders the all-track terminal mixer, inspector, and footer while translating keyboard/gamepad input into AppEvents"},
		{capability: "capability.instrument_partitioned_test_playback", contribution: "hosts one-patch-per-instrument MIDI-file demonstration plans and exposes their deterministic track assignments"},
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
		"volume-edit.json: navigate through AppEvent::Mixer, enter edit mode, lower volume exactly 6 dB, and assert the canonical MixerView/ChannelStrip snapshot.",
		"voice-steal.json: use AppEvent::Patch to configure polyphony 2 and oldest stealing, then AppEvent::Midi for three overlapping notes with render blocks; assert two active voices, at least one steal observation, and non-zero peak.",
		"preset-roundtrip.json: edit complete patch/session state through AppEvent::Patch, save through AppEvent::Preset, mutate, restore, and assert the restored state equals the saved snapshot. This requires Patch and Preset variants; an event-vocabulary sweep is not a round-trip proof.",
		"showcase.json: combine the supported behaviors into a paced mixer-focused journey for windowed observation with optional MIDI-file music: solo/unsolo, pan left/right, volume dip/restore, mute/unmute, captions, and renderBlocks between visible transitions.",
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
