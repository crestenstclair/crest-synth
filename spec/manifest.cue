package crestsynth

project: assets: CargoManifest: {
	kind: "cargo-manifest"
	description: "Cargo.toml for the standalone SoundFont synth"
	profile: {kind: "configuration", ecosystem: "cargo"}
	targets: [
		"adapter.HiDefSoundFontEngine",
		"adapter.GlobalReverbDelay",
		"adapter.LockFreeAudioBoundary",
		"adapter.AtomicAudioObservation",
		"adapter.CorridorsMidiEventSource",
		"adapter.EframeTextWindow",
		"adapter.CpalAudioOutput",
		"asset.BehavioralWitnessMain",
	]
	prompts: [
		"Create Cargo.toml for one Rust library, the crest-synth product binary, and the crest-synth-witness verification-only binary.",
		"Use only dependencies required by the declared resources: rustysynth, midly, cpal, eframe/egui, rtrb, triple_buffer, basedrop, serde with derive, serde_json, thiserror, and anyhow. Development-only test helpers may be added when required by the declared tests.",
		"Do not add synthesis, effect, GUI-widget, sequencing, persistence, plugin, database, networking, or async-runtime libraries.",
	]
	validations: [{kind: "custom", command: ["cargo", "metadata", "--no-deps", "--format-version", "1"], description: "the manifest resolves"}]
}

project: assets: BuildMakefile: {
	kind: "makefile"
	description: "Makefile: stable human entry points for building, checking, running, and observing the synth"
	profile: {kind: "configuration", ecosystem: "make"}
	targets: [
		"asset.CargoManifest",
		"asset.LibraryRoot",
		"asset.CrestSynthMain",
		"applicationService.Testing.LiveDemoRunner",
	]
	prompts: [
		"Create the project-root Makefile. The default target is help, which lists every public target and its one-line ## description.",
		"Provide these Cargo-backed targets: build (cargo build), check (cargo check --all-targets), test (cargo test --all-targets), lint (cargo clippy --all-targets -- -D warnings), fmt (cargo fmt --all), fmt-check (cargo fmt --all -- --check), run (cargo run --bin crest-synth), smoke (cargo run --bin crest-synth -- --smoke), observe (cargo run --bin crest-synth -- --smoke --observe), demo (cargo run --bin crest-synth -- --smoke --observe --demo-scene), demo-live (cargo run --bin crest-synth -- --demo-live), and clean (cargo clean).",
		"Provide play and ui as documented aliases for run because normal startup automatically plays the fixed MIDI fixture in the text window.",
		"Keep demo exactly headless and deterministic. demo-live is the only autonomous target that opens the real window and physical audio device; it stays open after scene completion until the user closes it.",
		"Use portable Make syntax and declare non-file targets phony. Do not reference removed proof binaries, alternate MIDI files, afplay, or obsolete synth_ui commands.",
	]
	validations: [
		{kind: "custom", command: ["make", "-n", "ui"], description: "the interactive human entry point exists"},
		{kind: "custom", command: ["make", "-n", "observe"], description: "the behavioral observation entry point exists"},
		{kind: "custom", command: ["make", "-n", "demo"], description: "the exhaustive GUI demo and trace entry point exists"},
		{kind: "custom", command: ["make", "-n", "demo-live"], description: "the live real-window and physical-audio demo entry point exists without executing an interactive process in validation"},
		{kind: "integration", command: ["make", "smoke"], timeout: "180s", description: "the Makefile drives the complete headless synth path", assertions: [
			{kind: "exit_code", expected: 0},
		]},
	]
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
		"valueObject.Mixer.MixObservation",
		"domainService.Mixer.MixEngine",
		"valueObject.RealTime.ParameterSnapshot",
		"valueObject.RealTime.AudioCommand",
		"valueObject.RealTime.AudioObservationSnapshot",
		"port.RealTime.AudioObservation",
		"adapter.AtomicAudioObservation",
		"applicationService.RealTime.AudioRenderer",
		"valueObject.Control.AppEvent",
		"valueObject.Control.EventRecord",
		"valueObject.Control.EventLog",
		"valueObject.Control.StateTree",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"valueObject.Shell.WindowInput",
		"applicationService.Shell.KeyboardInputTranslator",
		"port.Testing.MidiEventSource",
		"applicationService.Testing.AutomaticMidiTest",
		"valueObject.Testing.DemoScene",
		"valueObject.Testing.DemoSceneReport",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"valueObject.Testing.LiveDemoScene",
		"valueObject.Testing.LiveDemoCheckpoint",
		"valueObject.Testing.LiveDemoReport",
		"applicationService.Testing.LiveDemoRunner",
		"applicationService.Testing.BehavioralMutationHarness",
		"applicationService.Shell.StandaloneApplication",
	]
	prompts: [
		"Create src/lib.rs and small snake_case modules grouped as kernel, synth, mixer, real_time, control, testing, and shell.",
		"Each declared resource owns one public type or service. Re-export only the types required at context boundaries; do not create local duplicate models in adapters, tests, or the binary.",
		"The testing module is part of the crate only to drive automatic MIDI input. It must not expose a sequencer or transport API.",
	]
}

project: assets: BehavioralWitnessMain: {
	kind: "rust-bin-target"
	description: "src/bin/crest_synth_witness.rs, a fast verification-only runner for isolated production-seam counterexamples"
	profile: {kind: "verification_harness", witness: "typed production-seam mutation cases", failurePolicy: "emit one measured observation before returning the declared case exit status"}
	targets: [
		"applicationService.Testing.BehavioralMutationHarness",
		"applicationService.Shell.KeyboardInputTranslator",
		"applicationService.Control.AppLoop",
		"domainService.Control.StateProjector",
		"valueObject.Control.StateTree",
		"adapter.LockFreeAudioBoundary",
		"valueObject.RealTime.ParameterSnapshot",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"port.Mixer.GlobalEffectsProcessor",
		"adapter.GlobalReverbDelay",
	]
	prompts: [
		"Create src/bin/crest_synth_witness.rs as a verification-only composition root over BehavioralMutationHarness; it is never called by the interactive crest-synth binary and exposes no product mode.",
		"Accept exactly --case dropped-adjustment, cross-patch-parameter-leak, patch-misroute, omitted-state-tree-leaf, dry-to-wet-bypass, or zero-renderer and exactly --mutant none or the matching case. Reject mismatched cases, duplicate options, and every other argument.",
		"Run healthy and mutant cases through the same deterministic multi-Patch fixture and production services. Mutants are injected only at the named seam before observation; never append coverage, replace a measured field, edit a completed DemoSceneReport, or select a pre-authored result.",
		"Print exactly one single-line CREST_MUTATION_OBSERVATION JSON object for both healthy and mutant executions. Healthy cases exit 0. Matching mutants print their actual measured counterexample and then exit 1 so witness.negativeExpectedExitCode is explicit and auditable.",
		"Keep every case deterministic and fast: no physical audio device, native window, wall clock, random input, SoundFont file parse, MIDI file parse, network, or process spawning is part of this focused harness.",
	]
	validations: [{kind: "compiles", command: ["cargo", "check", "--bin", "crest-synth-witness"], description: "the bounded verification-only mutation runner compiles"}]
	contributesTo: [
		{capability: "capability.observable_demo_scene", contribution: "provides six independently executable typed counterexamples for the behavioral gate"},
	]
}

project: assets: BehavioralAcceptanceTests: {
	kind: "rust-integration-tests"
	description: "named integration-test targets whose absence cannot pass as a zero-test filter match"
	profile: {kind: "verification_harness", witness: "compiled black-box acceptance targets", failurePolicy: "a missing target or unexecuted assertion fails cargo before evidence can pass"}
	targets: [
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.LiveDemoRunner",
		"applicationService.Testing.BehavioralMutationHarness",
		"adapter.EframeTextWindow",
		"aggregate.Control.AppState",
		"applicationService.Control.AppLoop",
		"valueObject.Control.EventLog",
		"domainService.Control.StateProjector",
		"valueObject.RealTime.ParameterSnapshot",
		"valueObject.Mixer.MixObservation",
		"valueObject.RealTime.AudioObservationSnapshot",
		"port.RealTime.AudioObservation",
		"adapter.AtomicAudioObservation",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"port.Mixer.GlobalEffectsProcessor",
		"adapter.GlobalReverbDelay",
	]
	prompts: [
		"Create five explicit Cargo integration-test targets: tests/exhaustive_demo_scene.rs, tests/live_demo_scene.rs, tests/schema_surface.rs, tests/eframe_context.rs, and tests/behavioral_mutation_harness.rs. Project validations invoke them with cargo test --test, so a missing target is a hard failure.",
		"Each target must call the public production seam it verifies. It may assemble deterministic fixtures, but it must not duplicate reducer, routing, state projection, GUI update, render, coverage, or mutation-verdict logic inside the test file.",
		"exhaustive_demo_scene asserts bidirectionally exact input/event/state/projection/audio coverage, every typed parameter boundary, and exact baseline restoration; schema_surface asserts bidirectional equality between production-owned typed descriptors and observed serialized leaves.",
		"live_demo_scene uses a deterministic monotonic clock, a frame-observation harness, and the production renderer plus AtomicAudioObservation to prove pacing, one accepted change and audible generation-tagged observation for every current editable parameter instance, exact checkpoint agreement, rejection recovery, semantic all-notes-off, zero missing or unexpected coverage, and an inert completed runner. It must not duplicate AppState::apply, projection, render, coverage, or report logic, and it must not open or skip based on a native window or physical device.",
		"eframe_context drives real egui RawInput through EframeApplication.update with its callback wired to AppLoop.dispatch, then proves the next frame and EventLog reflect the exact accepted value and projection; rendering a separately supplied projection is forbidden. behavioral_mutation_harness executes every healthy/mutant pair and asserts the measured predicate that each named seam falsifies.",
		"Every target contains at least one ordinary #[test] function with concrete assertions and prints its exact CREST_ACCEPTANCE <target> passed marker only after all behavioral assertions succeed. Validations run with --nocapture and require that marker, so a target with zero executed tests cannot pass.",
		"No ignored tests, snapshot auto-acceptance, environment-dependent skip, success-on-missing-fixture branch, pre-assertion success marker, or assertion-free smoke test is permitted.",
	]
	validations: [
		{kind: "integration", command: ["cargo", "test", "--test", "exhaustive_demo_scene", "--", "--nocapture"], assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "CREST_ACCEPTANCE exhaustive_demo_scene passed"}], description: "the named exhaustive acceptance target executes behavioral assertions and passes"},
		{kind: "integration", command: ["cargo", "test", "--test", "live_demo_scene", "--", "--nocapture"], assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "CREST_ACCEPTANCE live_demo_scene passed"}], description: "the named live-demo contract target executes pacing, canonical-path, audio-observation, coverage, cleanup, and final-state assertions"},
		{kind: "integration", command: ["cargo", "test", "--test", "schema_surface", "--", "--nocapture"], assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "CREST_ACCEPTANCE schema_surface passed"}], description: "the named schema-equality target executes behavioral assertions and passes"},
		{kind: "integration", command: ["cargo", "test", "--test", "eframe_context", "--", "--nocapture"], assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "CREST_ACCEPTANCE eframe_context passed"}], description: "the named egui-context target executes behavioral assertions and passes"},
		{kind: "integration", command: ["cargo", "test", "--test", "behavioral_mutation_harness", "--", "--nocapture"], assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "CREST_ACCEPTANCE behavioral_mutation_harness passed"}], description: "the named six-mutant target executes behavioral assertions and passes"},
	]
	contributesTo: [
		{capability: "capability.observable_demo_scene", contribution: "makes every existing headless acceptance gate structurally executable and impossible to satisfy with zero matched tests"},
		{capability: "capability.live_observable_demo", contribution: "provides an executable deterministic harness for the interactive live orchestration contract"},
	]
}

project: assets: CrestSynthMain: {
	kind: "rust-bin-target"
	description: "src/bin/crest_synth.rs, the thin standalone composition root and smoke harness"
	profile: {kind: "infrastructure", witness: "SoundFont synth vertical slice", failurePolicy: "fail startup on missing fixture, SoundFont, audio setup, or invalid configuration"}
	targets: [
		"applicationService.Shell.StandaloneApplication",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.LiveDemoRunner",
		"adapter.AtomicAudioObservation",
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
		"Support --smoke for headless deterministic execution, --observe to print one CREST_OBSERVATION JSON object, --demo-scene only with --smoke --observe to run the exhaustive current-GUI scene, --demo-live by itself to run the paced scene in the normal window and physical audio stream, and verification-only --degenerate-audio or --degenerate-control only with --smoke --observe for behavioral falsification. Reject duplicate, mixed, or other options; --demo-live is mutually exclusive with every headless, observation, demo-scene, and degenerate flag.",
		"When --demo-scene is present, print exactly one single-line CREST_EVENT_LOG JSON object, one single-line CREST_STATE_TREE JSON object, and one single-line CREST_OBSERVATION JSON object. Use stable Serialize data, never Debug output or Markdown; the ordinary interactive run emits no trace unless explicitly requested.",
		"When --demo-live is present, pass StandaloneApplication.runLiveDemo control-side callbacks that print each returned checkpoint as CREST_LIVE_CHECKPOINT JSON, then print exactly one CREST_LIVE_EVENT_LOG JSON, CREST_LIVE_STATE_TREE JSON, CREST_LIVE_COVERAGE JSON, and CREST_LIVE_SUMMARY human-readable line from the completed report after note cleanup; never print or format from the runner or audio callback.",
		"The live option uses the same HiDef.sf2, Corridors of Time source, AppLoop, EframeTextWindow, CpalAudioOutput, AudioRenderer, engine, and mixer as normal interactive execution. It has no fake window, null device, offline-only renderer, silent fallback, direct state edit, or alternate demo reducer.",
		"After live completion keep the physical stream and normal window alive with the final AppLoop.currentText projection until the user closes it; do not exit automatically or send a viewport-close command.",
		"The smoke path loads the real fixed SoundFont and MIDI fixture, installs all instrument Patches, applies keyboard-equivalent navigation and adjustment events, renders bounded audio blocks, and reports measurements from the production services.",
		"The observation must exercise at least two simultaneously sounding Patches, edit a Patch whose id is greater than 1, compare the edited and unedited Patch stems separately, and prove the unedited stem is sample-identical while the edited stem changes.",
		"The observation must also drive the selected parameter to a boundary, send one more adjustment toward that boundary, then prove a later valid edit is accepted without restarting the application.",
		"The demo-scene observation delegates to ExhaustiveGuiDemo, includes its coverage summary in CREST_OBSERVATION, and returns a failure when the event journal drops a record, the final state tree disagrees, or any expected current event/property/parameter/effect identifier is missing.",
		"Degenerate modes mutate the real seam before observation: --degenerate-control drops exactly one selected Adjust before AppLoop dispatch and --degenerate-audio clears the caller-owned buffer immediately after AudioRenderer.render. They never append a fake missing identifier, suppress peak accumulation, or overwrite an observation field. Demo-scene mutants emit the complete schema-valid trace and observation first, then exit 1 when detected; smoke mutants emit their schema-valid falsifying observation and exit 0 as declared by their legacy witnesses.",
	]
	validations: [
		{kind: "integration", command: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke"], timeout: "180s", description: "the complete headless path runs against HiDef.sf2 and Corridors of Time"},
		{kind: "custom", command: ["make", "-n", "demo-live"], description: "the dedicated live option is reachable through one stable human command"},
	]
	contributesTo: [
		{capability: "capability.soundfont_audio", contribution: "starts the concrete SoundFont synth"},
		{capability: "capability.automatic_test_midi", contribution: "starts the fixed MIDI test input without transport controls"},
		{capability: "capability.one_way_parameter_control", contribution: "hosts the text control loop without owning behavior"},
		{capability: "capability.realtime_execution", contribution: "composes the prepared control and audio sides"},
		{capability: "capability.observable_demo_scene", contribution: "emits stable event-log, state-tree, and coverage JSON for the exhaustive scene"},
		{capability: "capability.live_observable_demo", contribution: "selects the paced real-window mode and emits its control-side checkpoints and final summary without owning behavior"},
	]
}
