package crestsynth

project: assets: CargoManifest: {
	kind: "cargo-manifest"
	description: "Cargo.toml and build.rs for the capability-configured standalone synth with Rust SoundFont plus pinned C++ Braids and Chorus renderers"
	profile: {kind: "configuration", ecosystem: "cargo"}
	targets: [
		"adapter.HiDefSoundFontAsset",
		"adapter.HiDefSoundFontCapability",
		"adapter.HiDefSoundFontPreparer",
		"adapter.BraidsCapability",
		"adapter.BraidsPreparer",
		"adapter.ChorusCapability",
		"adapter.ChorusPreparer",
		"asset.ChorusSourceBundle",
		"adapter.GlobalReverbDelay",
		"adapter.LockFreeAudioBoundary",
		"adapter.LockFreeStructuralGraphBoundary",
		"adapter.ThreadedGraphPreparationWorker",
		"adapter.DeterministicGraphPreparationWorker",
		"adapter.AtomicAudioObservation",
		"adapter.CorridorsMidiEventSource",
		"adapter.EframeGraphicalWindow",
		"adapter.CpalAudioOutput",
		"valueObject.Shell.AudioDeviceConfig",
		"valueObject.Shell.AudioDeviceRuntimeError",
		"asset.BehavioralWitnessMain",
	]
	prompts: [
		"Create Cargo.toml for one Rust library, the crest-synth product binary, and the crest-synth-witness verification-only binary.",
		"Use only dependencies required by the declared resources: rustysynth, midly, cpal, matching eframe/egui and egui_extras, rtrb, triple_buffer, serde with derive, serde_json, thiserror, anyhow, and the build-only cc crate for the fixed audited Braids and Chorus C++ source lists. Enable only the egui_extras layout and image/SVG loader features used by the shell. Complete graph destruction uses the dedicated ownership-return structural queue and no deferred-drop dependency. Development-only test helpers may be added when required by the declared tests.",
		"Do not add another synthesis/effect framework, GUI runtime, component library, sequencing, persistence, plugin, database, networking, or async-runtime library; the pinned vendored native subsets are built only through the declared opaque adapters.",
	]
	validations: [{id: "validation.asset.cargo_manifest", kind: "custom", command: ["cargo", "metadata", "--no-deps", "--format-version", "1"], description: "the manifest resolves"}]
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
		"Provide these Cargo-backed targets: build (cargo build), check (cargo check --all-targets), test (cargo test --all-targets), lint (cargo clippy --all-targets -- -D warnings), fmt (cargo fmt --all), fmt-check (cargo fmt --all -- --check), run (cargo run --bin crest-synth), smoke (cargo run --bin crest-synth -- --smoke), observe (cargo run --bin crest-synth -- --smoke --observe), demo (cargo run --bin crest-synth -- --smoke --observe --demo-scene), demo-live-graphical-shell (cargo run --release --bin crest-synth -- --demo-live-graphical-shell), demo-live-semantic-view-model (cargo run --release --bin crest-synth -- --demo-live-semantic-view-model), demo-live-sixteen-track-mixer-routing (cargo run --release --bin crest-synth -- --demo-live-sixteen-track-mixer-routing), demo-live as an alias of the newest sixteen-track routing scene, and clean (cargo clean).",
		"Provide play and ui as documented aliases for run because normal startup automatically plays the fixed MIDI fixture in the graphical window.",
		"Keep demo exactly headless and deterministic. Retain demo-live-graphical-shell as the Phase 1 witness and demo-live-semantic-view-model as the Phase 2 witness. demo-live-sixteen-track-mixer-routing cumulatively opens the real graphical window and physical audio device, displays all sixteen tracks, proves shared-track routing, Patch trim/reroute, track controls and meters, retains the semantic/scalar/preset/engine proof, then emits final evidence once, closes, releases the stream, and returns success. demo-live points to this newest cumulative scene.",
		"Use portable Make syntax and declare non-file targets phony. Do not reference removed proof binaries, alternate MIDI files, afplay, or obsolete synth_ui commands.",
	]
	validations: [
		{id: "validation.asset.make_ui", kind: "custom", command: ["make", "-n", "ui"], description: "the interactive human entry point exists"},
		{id: "validation.asset.make_observe", kind: "custom", command: ["make", "-n", "observe"], description: "the behavioral observation entry point exists"},
		{id: "validation.asset.make_demo", kind: "custom", command: ["make", "-n", "demo"], description: "the exhaustive GUI demo and trace entry point exists"},
		{id: "validation.asset.make_demo_live", kind: "custom", command: ["make", "-n", "demo-live"], description: "the live real-window and physical-audio demo entry point exists without executing an interactive process in validation"},
		{id: "validation.asset.make_demo_live_graphical_shell", kind: "custom", command: ["make", "-n", "demo-live-graphical-shell"], description: "the retained Phase 1 graphical-shell live scene has its own stable human command"},
		{id: "validation.asset.make_demo_live_semantic_view_model", kind: "custom", command: ["make", "-n", "demo-live-semantic-view-model"], description: "the Phase 2 semantic-view-model live scene has its own stable human command"},
		{id: "validation.asset.make_demo_live_sixteen_track_mixer_routing", kind: "custom", command: ["make", "-n", "demo-live-sixteen-track-mixer-routing"], description: "the corrective sixteen-track routing live scene has its own stable human command"},
		{id: "validation.asset.make_smoke", kind: "integration", command: ["make", "smoke"], timeout: "180s", description: "the Makefile drives the complete headless synth path", assertions: [
			{type: "exit-code", equals: 0},
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
		"valueObject.Synth.CapabilityId",
		"valueObject.Synth.ParameterId",
		"valueObject.Synth.AssetReference",
		"valueObject.Synth.ParameterValue",
		"valueObject.Synth.ParameterAssignment",
		"valueObject.Synth.ParameterSpec",
		"valueObject.Synth.CapabilityDescriptor",
		"valueObject.Synth.InstrumentConfig",
		"valueObject.Synth.CapabilityRegistry",
		"valueObject.Synth.EffectCapabilityId",
		"valueObject.Synth.EffectSlotId",
		"valueObject.Synth.EffectCapabilityDescriptor",
		"valueObject.Synth.PostEffectConfig",
		"valueObject.Synth.EffectCapabilityRegistry",
		"valueObject.Synth.SoundFontPresetId",
		"valueObject.Synth.SoundFontPresetCatalogEntry",
		"valueObject.Synth.SoundFontPresetCatalog",
		"valueObject.Synth.VoiceEnvelope",
		"aggregate.Synth.Patch",
		"valueObject.Mixer.MixerTrackId",
		"valueObject.Mixer.PatchOutput",
		"valueObject.Mixer.MixerTrackParameters",
		"valueObject.Mixer.MixerState",
		"port.Synth.InstrumentCapabilityProvider",
		"port.Synth.PreparedInstrument",
		"port.Synth.InstrumentPreparer",
		"port.Synth.EffectCapabilityProvider",
		"port.Synth.PreparedPostEffect",
		"port.Synth.EffectPreparer",
		"applicationService.Synth.PreparedEngineRackBuilder",
		"applicationService.Synth.PreparedPostEffectRackBuilder",
		"applicationService.Synth.DescriptorDefaultConfigFactory",
		"valueObject.Mixer.MixerTrackId",
		"valueObject.Mixer.PatchOutputParameter",
		"valueObject.Mixer.PatchOutput",
		"valueObject.Mixer.MixerTrackParameter",
		"valueObject.Mixer.MixerTrackParameters",
		"valueObject.Mixer.MixerState",
		"valueObject.Mixer.TrackMeter",
		"valueObject.Mixer.GlobalParameters",
		"valueObject.Mixer.MixObservation",
		"domainService.Mixer.MixEngine",
		"valueObject.RealTime.ParameterSnapshot",
		"valueObject.RealTime.RtPostEffectParameters",
		"valueObject.RealTime.PatchEffectObservation",
		"valueObject.RealTime.GraphRevision",
		"aggregate.RealTime.PreparedEngineRack",
		"aggregate.RealTime.PreparedPostEffectRack",
		"aggregate.RealTime.PreparedGraph",
		"valueObject.RealTime.GraphHandoffStatus",
		"valueObject.RealTime.GraphPreparationRequest",
		"valueObject.RealTime.GraphPreparationResult",
		"valueObject.RealTime.AudioCommand",
		"valueObject.RealTime.AudioObservationSnapshot",
		"port.RealTime.AudioObservation",
		"port.RealTime.StructuralGraphBoundary",
		"port.RealTime.GraphPreparationWorker",
		"applicationService.RealTime.PreparedGraphBuilder",
		"applicationService.RealTime.StructuralGraphCoordinator",
		"adapter.AtomicAudioObservation",
		"applicationService.RealTime.AudioRenderer",
		"valueObject.Control.TopLevelContext",
		"valueObject.Control.SemanticAction",
		"valueObject.Control.SurfaceId",
		"valueObject.Control.InteractionMode",
		"valueObject.Control.MixerControlId",
		"valueObject.Control.SemanticControlId",
		"valueObject.Control.FocusPath",
		"valueObject.Control.ReturnPath",
		"valueObject.Control.ValidAction",
		"valueObject.Control.SemanticControlViewModel",
		"valueObject.Control.SemanticSurfaceViewModel",
		"valueObject.Control.SemanticGraphicalViewModel",
		"valueObject.Control.InteractionState",
		"valueObject.Control.PatchControlId",
		"valueObject.Control.StructuralEditIntent",
		"valueObject.Control.EngineSelectionRequestId",
		"valueObject.Control.EngineSelectionFailure",
		"valueObject.Control.EngineSelectionStatus",
		"valueObject.Control.EngineSelectionEffect",
		"valueObject.Control.AppEvent",
		"valueObject.Control.EventRecord",
		"valueObject.Control.EventLog",
		"valueObject.Control.StateTree",
		"valueObject.Control.PatchPageProjection",
		"valueObject.Control.GraphicalShellProjection",
		"valueObject.Shell.ShellFrameObservation",
		"valueObject.Control.TextProjection",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"valueObject.Shell.WindowInput",
		"applicationService.Shell.KeyboardInputTranslator",
		"port.Testing.MidiEventSource",
		"applicationService.Testing.AutomaticMidiTest",
		"valueObject.Testing.DemoScene",
		"valueObject.Testing.DemoSceneReport",
		"valueObject.Testing.EngineSelectionObservation",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"valueObject.Testing.LiveDemoScene",
			"valueObject.Testing.LiveDemoCheckpoint",
			"valueObject.Testing.LiveDemoReport",
			"valueObject.Testing.LiveEventLogSummary",
		"applicationService.Testing.LiveDemoRunner",
		"applicationService.Testing.BehavioralMutationHarness",
		"applicationService.Shell.StandaloneApplication",
		"valueObject.Shell.AudioDeviceConfig",
		"valueObject.Shell.AudioDeviceRuntimeError",
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
		"aggregate.RealTime.PreparedEngineRack",
		"aggregate.RealTime.PreparedGraph",
		"adapter.LockFreeStructuralGraphBoundary",
		"valueObject.RealTime.ParameterSnapshot",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"port.Mixer.GlobalEffectsProcessor",
		"adapter.GlobalReverbDelay",
	]
	prompts: [
		"Create src/bin/crest_synth_witness.rs as a verification-only composition root over BehavioralMutationHarness; it is never called by the interactive crest-synth binary and exposes no product mode.",
		"Accept exactly --case dropped-adjustment, cross-track-parameter-leak, patch-misroute, omitted-state-tree-leaf, dry-to-wet-bypass, or zero-renderer and exactly --mutant none or the matching case. Reject mismatched cases, duplicate options, and every other argument.",
		"Run healthy and mutant cases through the same deterministic multi-Patch fixture and production services. Mutants are injected only at the named seam before observation; never append coverage, replace a measured field, edit a completed DemoSceneReport, or select a pre-authored result.",
		"Print exactly one single-line CREST_MUTATION_OBSERVATION JSON object for both healthy and mutant executions. Healthy cases exit 0. Matching mutants print their actual measured counterexample and then exit 1 so witness.negativeExpectedExitCode is explicit and auditable.",
		"Keep every case deterministic and fast: no physical audio device, native window, wall clock, random input, SoundFont file parse, MIDI file parse, network, or process spawning is part of this focused harness.",
	]
	validations: [{id: "validation.asset.behavioral_witness_main", kind: "compiles", command: ["cargo", "check", "--bin", "crest-synth-witness"], description: "the bounded verification-only mutation runner compiles"}]
	contributesTo: [
		{capability: "capability.observable_demo_scene", contribution: "provides six independently executable typed counterexamples for the behavioral gate"},
	]
}

project: assets: BehavioralAcceptanceTests: {
	kind: "rust-integration-tests"
	description: "named integration-test targets whose absence cannot pass as a zero-test filter match"
	profile: {kind: "verification_harness", witness: "compiled black-box acceptance targets", failurePolicy: "a missing target or unexecuted assertion fails cargo before evidence can pass"}
	targets: [
		"valueObject.Synth.CapabilityRegistry",
		"valueObject.Synth.EffectCapabilityRegistry",
		"valueObject.Synth.PostEffectConfig",
		"valueObject.Synth.SoundFontPresetId",
		"valueObject.Synth.SoundFontPresetCatalog",
		"port.Synth.InstrumentCapabilityProvider",
		"adapter.HiDefSoundFontAsset",
		"adapter.HiDefSoundFontCapability",
		"adapter.BraidsCapability",
		"adapter.HiDefSoundFontPreparer",
		"adapter.BraidsPreparer",
		"adapter.ChorusCapability",
		"adapter.ChorusPreparer",
		"valueObject.Synth.VoiceEnvelope",
		"port.Synth.PreparedInstrument",
		"port.Synth.InstrumentPreparer",
		"applicationService.Synth.PreparedEngineRackBuilder",
		"aggregate.RealTime.PreparedEngineRack",
		"aggregate.RealTime.PreparedPostEffectRack",
		"aggregate.RealTime.PreparedGraph",
		"port.RealTime.StructuralGraphBoundary",
		"adapter.LockFreeStructuralGraphBoundary",
		"applicationService.RealTime.PreparedGraphBuilder",
		"applicationService.RealTime.StructuralGraphCoordinator",
		"aggregate.Synth.Patch",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.LiveDemoRunner",
		"applicationService.Testing.BehavioralMutationHarness",
		"adapter.EframeGraphicalWindow",
		"valueObject.Control.TopLevelContext",
		"valueObject.Control.SemanticAction",
		"valueObject.Control.FocusPath",
		"valueObject.Control.ReturnPath",
		"valueObject.Control.SemanticGraphicalViewModel",
		"valueObject.Control.InteractionState",
		"valueObject.Control.AppEvent",
		"valueObject.Control.PatchPageProjection",
		"valueObject.Control.GraphicalShellProjection",
		"valueObject.Control.TextProjection",
		"valueObject.Shell.WindowInput",
		"applicationService.Shell.KeyboardInputTranslator",
		"aggregate.Control.AppState",
		"applicationService.Control.AppLoop",
		"valueObject.Control.EventLog",
		"domainService.Control.StateProjector",
		"valueObject.RealTime.ParameterSnapshot",
		"port.RealTime.AudioBoundary",
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
		"Create sixteen explicit Cargo integration-test targets: tests/capability_schema.rs, tests/patch_page_projection.rs, tests/engine_selection_workflow.rs, tests/prepared_engine_rack.rs, tests/braids_engine.rs, tests/per_voice_envelope.rs, tests/soundfont_preset_selection.rs, tests/control_dispatch_performance.rs, tests/exhaustive_demo_scene.rs, tests/live_demo_scene.rs, tests/schema_surface.rs, tests/eframe_context.rs, tests/graphical_application_shell.rs, tests/semantic_graphical_view_model.rs, tests/sixteen_track_mixer_routing.rs, and tests/behavioral_mutation_harness.rs. Project validations invoke them with cargo test --test, using --release for declared callback timing witnesses, so a missing target is a hard failure.",
		"Each target must call the public production seam it verifies. It may assemble deterministic fixtures, but it must not duplicate reducer, routing, state projection, GUI update, render, coverage, or mutation-verdict logic inside the test file.",
		"capability_schema proves exact instrument/effect registries and generic Patch configs plus typed no-fallback rejection. patch_page_projection drives the production translator/AppLoop through PATCH Main and Utility, exact SoundFont/Braids/Chorus rows, canonical ADSR/effect edits, and exact PatchOutput trim/route controls while preserving MIXER focus. soundfont_preset_selection proves real-SF2 authored identity and correlated replacement. braids_engine proves pinned DSP, one sixteen-voice bank per Patch, scalar isolation, and callback safety. per_voice_envelope drives all four ADSR fields only through PATCH and proves exact fixed projections plus independent SoundFont/Braids note lifecycles. sixteen_track_mixer_routing proves all sixteen persistent tracks, shared-track accumulation, Patch trim/reroute isolation, all six track parameter classes, meters, stable focus, and callback safety. exhaustive_demo_scene and schema_surface include every PatchOutput, MixerTrackId, MixerTrackParameter, fixed track snapshot, meter, semantic path, and structural lifecycle with exact bidirectional coverage.",
		"live_demo_scene uses a deterministic clock, frame-observation harness, production worker/structural ports, renderer, MixEngine, and AtomicAudioObservation to prove pacing and exact-generation causal audio. It routes Patch-owned controls through PATCH, all sixteen track controls through MIXER, two Patches through one track, and one Patch through a reroute; proves fixed meters, one adjacent SoundFont preset replacement, both engine directions, exact lifecycle/revision/effect preservation, rejection recovery, semantic all-notes-off, and inert completion without duplicating AppState.apply, routing, graph handoff, projection, render, coverage, or verdict logic.",
		"control_dispatch_performance installs fifteen descriptor-configured Patches, sends 512 MIDI events through the production AppState, StateProjector, StateTree, EventLog, and ControlAudioBoundary path, requires completion within 50 ms in the unoptimized test profile, and relies on the projector unit equivalence proof that deferred snapshot/tree JSON equals eager canonical output.",
		"prepared_engine_rack uses the production builders, HiDef preparer, generic rack, complete graph, structural boundary, coordinator, and renderer plus two distinct deterministic PreparedInstrument implementations to prove exact Patch targeting, isolated stems, block-boundary swap, graph-revision acknowledgement, one-in-flight throttling, queue-pressure retention, and destructor execution only during control collection; it must instrument the production callback path for zero allocation and destruction.",
		"eframe_context drives real egui RawInput through EframeGraphicalApplication.update with its callback wired to AppLoop.dispatchAction, then proves the next frame and EventLog reflect the exact accepted value and semantic, shell, plus diagnostic projections; rendering a separately supplied projection is forbidden. graphical_application_shell renders both reference viewports through that same production update path and proves all five regions are visible, ordered, bounded, non-overlapping, and generation-coherent. semantic_graphical_view_model proves exact actions, stable paths, four surfaces, Navigate/Adjust, return, valid actions, status/errors, descriptor content, schema recovery, responsive invariance, and audio neutrality. behavioral_mutation_harness executes every healthy/mutant pair and asserts the measured predicate that each named seam falsifies.",
		"Every target contains at least one ordinary #[test] function with concrete assertions and prints its exact CREST_ACCEPTANCE <target> passed marker only after all behavioral assertions succeed. Validations run with --nocapture and require that marker, so a target with zero executed tests cannot pass.",
		"No ignored tests, snapshot auto-acceptance, environment-dependent skip, success-on-missing-fixture branch, pre-assertion success marker, or assertion-free smoke test is permitted.",
	]
	validations: [
		{id: "validation.asset.acceptance_capability_schema", kind: "integration", command: ["cargo", "test", "--test", "capability_schema", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE capability_schema passed"}], description: "the named capability-schema target proves generic registry/config behavior and typed no-fallback rejection"},
		{id: "validation.asset.acceptance_patch_page_projection", kind: "integration", command: ["cargo", "test", "--test", "patch_page_projection", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE patch_page_projection passed"}], description: "the named Patch-page target proves semantic page selection, schema-derived exact instrument/effect projection, dynamic focus, canonical fine/coarse ADSR/effect editing, and structural neutrality of scalar focus movement"},
		{id: "validation.asset.acceptance_prepared_engine_rack", kind: "integration", command: ["cargo", "test", "--test", "prepared_engine_rack", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE prepared_engine_rack passed"}], description: "the named rack target proves capability-neutral prepared rendering and destruction-safe structural ownership handoff"},
		{id: "validation.asset.acceptance_braids_engine", kind: "integration", command: ["cargo", "test", "--release", "--test", "braids_engine", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE braids_engine passed"}], description: "the named Braids target proves pinned sixteen-voice native DSP and hard-real-time mixed-engine behavior"},
		{id: "validation.asset.acceptance_per_voice_envelope", kind: "integration", command: ["cargo", "test", "--test", "per_voice_envelope", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE per_voice_envelope passed"}], description: "the named envelope target proves canonical PATCH/MIXER editing and independent overlapping note lifecycles in both engines"},
		{id: "validation.asset.acceptance_soundfont_preset_selection", kind: "integration", command: ["cargo", "test", "--release", "--test", "soundfont_preset_selection", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE soundfont_preset_selection passed"}], description: "the named real-SF2 target proves exact authored catalog order, generic structural preset replacement, numeric callback ownership, measured target audio, restoration, and no fallback"},
		{id: "validation.asset.acceptance_control_dispatch_performance", kind: "integration", command: ["cargo", "test", "--test", "control_dispatch_performance", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE control_dispatch_performance passed"}], description: "the named control-performance target proves responsive sustained MIDI dispatch through every production control seam"},
		{id: "validation.asset.acceptance_exhaustive_demo_scene", kind: "integration", command: ["cargo", "test", "--test", "exhaustive_demo_scene", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE exhaustive_demo_scene passed"}], description: "the named exhaustive acceptance target executes behavioral assertions and passes"},
		{id: "validation.asset.acceptance_live_demo_scene", kind: "integration", command: ["cargo", "test", "--test", "live_demo_scene", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE live_demo_scene passed"}], description: "the named live-demo contract target executes pacing, canonical scalar and two-direction engine paths, lifecycle/revision/audio observations, coverage, cleanup, and final-state assertions"},
		{id: "validation.asset.acceptance_schema_surface", kind: "integration", command: ["cargo", "test", "--test", "schema_surface", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE schema_surface passed"}], description: "the named schema-equality target executes behavioral assertions and passes"},
		{id: "validation.asset.acceptance_eframe_context", kind: "integration", command: ["cargo", "test", "--test", "eframe_context", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE eframe_context passed"}], description: "the named egui-context target executes behavioral assertions and passes"},
		{id: "validation.asset.acceptance_graphical_application_shell", kind: "integration", command: ["cargo", "test", "--test", "graphical_application_shell", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE graphical_application_shell passed"}], description: "the named graphical-shell target proves desktop and Steam Deck structure through the production egui update path"},
		{id: "validation.asset.acceptance_mutation_harness", kind: "integration", command: ["cargo", "test", "--test", "behavioral_mutation_harness", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE behavioral_mutation_harness passed"}], description: "the named six-mutant target executes behavioral assertions and passes"},
	]
	contributesTo: [
		{capability: "capability.instrument_capability_model", contribution: "provides the named executable acceptance target for generic descriptors, configs, projection, and explicit failure"},
		{capability: "capability.soundfont_preset_selection", contribution: "provides the named real-SF2 acceptance target for exact catalog fidelity and correlated structural preset replacement"},
		{capability: "capability.schema_driven_patch_page", contribution: "provides the named executable target for direct context selection and exact generic Patch projection"},
		{capability: "capability.prepared_engine_rack", contribution: "provides the named executable target for generic preparation, heterogeneous rack dispatch, graph handoff, and off-callback retirement"},
		{capability: "capability.static_patch_effect", contribution: "extends the named schema/page/demo gates with exact effect registry, control, preservation, order, isolation, and audible-output assertions"},
			{capability: "capability.observable_demo_scene", contribution: "makes every existing headless acceptance gate structurally executable and impossible to satisfy with zero matched tests"},
			{capability: "capability.graphical_application_shell", contribution: "provides named non-vacuous egui acceptance for every Phase 1 region and both reference viewports"},
			{capability: "capability.live_observable_demo", contribution: "provides an executable deterministic harness for the interactive live orchestration contract"},
			{capability: "capability.asynchronous_engine_selection", contribution: "makes both live engine directions executable through the production worker and structural seams"},
	]
}

project: assets: ProductionRuntimeContractTests: {
	kind: "rust-integration-tests"
	description: "tests/production_runtime_contracts.rs plus an exact-selector guard for injected composition, negotiated preparation, runtime device status, structural handoff, and routing observation"
	profile: {kind: "verification_harness", witness: "production application and renderer boundary contracts", failurePolicy: "a missing target, zero selected tests, missing post-assertion marker, or behavioral contradiction fails independently"}
	targets: [
		"applicationService.Shell.StandaloneApplication",
		"port.Shell.AudioOutput",
		"valueObject.Shell.AudioDeviceConfig",
		"valueObject.Shell.AudioDeviceRuntimeError",
		"port.Synth.InstrumentCapabilityProvider",
		"port.Synth.InstrumentPreparer",
		"port.Synth.EffectCapabilityProvider",
		"port.Synth.EffectPreparer",
		"applicationService.Synth.DescriptorDefaultConfigFactory",
		"port.RealTime.StructuralGraphBoundary",
		"port.RealTime.GraphPreparationWorker",
		"adapter.ThreadedGraphPreparationWorker",
		"port.RealTime.AudioObservation",
		"applicationService.RealTime.AudioRenderer",
	]
	prompts: [
		"Use the public production constructor with injected instrument/effect providers and preparers, capacity-one graph-preparation worker, structural, observation, device, and window fixtures; prove matching registration and typed duplicate, missing, unknown, and mismatched rejection before structural publication.",
		"Prove normal and early-error teardown release the audio stream before worker shutdown and drain every pending, staged, returned, or retired graph only on control/worker ownership.",
		"Negotiate a supported non-default sample rate and exact render capacity before preparation, render both exact-capacity and oversized callbacks completely, reject unsupported negotiation before preparation, and surface a controlled post-start device failure as the exact ApplicationError on control ownership.",
		"Provide ordinary exact tests named audio_renderer_realtime_contract, prepared_graph_handoff, and audio_observation_realtime_contract. Each prints its exact CREST_RT_VALIDATION marker only after assertions.",
		"Run each selector through scripts/run_exact_test_validation.sh, which requires exactly one executed test and its marker. Its self-test must reject zero-selection evidence even when supplied text claims a broad suite passed.",
	]
	validations: [
		{id: "validation.asset.production_runtime_contracts", kind: "integration", command: ["cargo", "test", "--test", "production_runtime_contracts", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE production_runtime_contracts passed"}], description: "the complete production runtime repair target passes"},
		{id: "validation.asset.production_audio_renderer", kind: "integration", command: ["bash", "scripts/run_exact_test_validation.sh", "production_runtime_contracts", "audio_renderer_realtime_contract", "CREST_RT_VALIDATION audio_renderer_realtime_contract passed"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "\"testsExecuted\":1"}], description: "the renderer selector executes exactly one test"},
		{id: "validation.asset.production_graph_handoff", kind: "integration", command: ["bash", "scripts/run_exact_test_validation.sh", "production_runtime_contracts", "prepared_graph_handoff", "CREST_RT_VALIDATION prepared_graph_handoff passed"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "\"testsExecuted\":1"}], description: "the graph-handoff selector executes exactly one test"},
		{id: "validation.asset.production_audio_observation", kind: "integration", command: ["bash", "scripts/run_exact_test_validation.sh", "production_runtime_contracts", "audio_observation_realtime_contract", "CREST_RT_VALIDATION audio_observation_realtime_contract passed"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "\"testsExecuted\":1"}], description: "the observation selector executes exactly one test"},
		{id: "validation.asset.production_zero_selection_guard", kind: "custom", command: ["bash", "scripts/run_exact_test_validation.sh", "--self-test"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_TEST_VALIDATION zero-selection-rejected passed"}], description: "zero-selection output is rejected independently of broad-suite success text"},
	]
	contributesTo: [
		{capability: "capability.instrument_capability_model", contribution: "proves production constructor ownership and exact provider/preparer registration"},
		{capability: "capability.static_patch_effect", contribution: "proves production effect provider/preparer injection, exact registration, negotiated preparation, and off-callback teardown"},
		{capability: "capability.asynchronous_engine_selection", contribution: "proves production worker injection and control-owned lifecycle teardown"},
		{capability: "capability.prepared_engine_rack", contribution: "proves negotiated preparation, complete callback chunking, structural replacement, and observable unknown-Patch failure"},
		{capability: "capability.realtime_execution", contribution: "makes each declared callback witness non-vacuous and runtime device errors visible"},
	]
}

project: assets: CrestSynthMain: {
	kind: "rust-bin-target"
	description: "src/bin/crest_synth.rs, the thin standalone composition root and smoke harness"
	profile: {kind: "infrastructure", witness: "mixed SoundFont/Braids instruments plus first-Patch Chorus vertical slice", failurePolicy: "fail startup on missing fixture, SoundFont, Braids/Chorus preparation, audio setup, or invalid configuration"}
	targets: [
		"applicationService.Shell.StandaloneApplication",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.LiveDemoRunner",
		"adapter.AtomicAudioObservation",
		"adapter.HiDefSoundFontAsset",
		"valueObject.Synth.SoundFontPresetCatalog",
		"adapter.HiDefSoundFontCapability",
		"adapter.HiDefSoundFontPreparer",
		"adapter.BraidsCapability",
		"adapter.BraidsPreparer",
		"adapter.ChorusCapability",
		"adapter.ChorusPreparer",
		"asset.ChorusSourceBundle",
		"applicationService.Synth.PreparedEngineRackBuilder",
		"applicationService.Synth.PreparedPostEffectRackBuilder",
		"applicationService.Synth.DescriptorDefaultConfigFactory",
		"applicationService.RealTime.PreparedGraphBuilder",
		"applicationService.RealTime.StructuralGraphCoordinator",
		"adapter.LockFreeStructuralGraphBoundary",
		"adapter.ThreadedGraphPreparationWorker",
		"adapter.DeterministicGraphPreparationWorker",
		"adapter.GlobalReverbDelay",
		"adapter.LockFreeAudioBoundary",
		"adapter.CorridorsMidiEventSource",
		"adapter.EframeGraphicalWindow",
		"adapter.CpalAudioOutput",
	]
	prompts: [
		"Create src/bin/crest_synth.rs as composition only. Parse HiDef.sf2 exactly once through HiDefSoundFontAsset, construct HiDefSoundFontCapability from its immutable catalog and HiDefSoundFontPreparer from that catalog plus its numeric prepared bank, then construct BraidsCapability/BraidsPreparer, ChorusCapability/ChorusPreparer, both prepared rack builders, ThreadedGraphPreparationWorker, and distinct audio, structural, and observation boundaries; inject all ports into StandaloneApplication, which validates and freezes the hydrated instrument/effect descriptors, and call run.",
		"Fail startup for duplicate, missing, unknown, or mismatched instrument/effect capability or preparer registration. Install exactly instrument.soundfont.hidef, instrument.braids, and effect.chorus in production and never choose or substitute a fallback descriptor, config, asset, preset, provider, preparer, prepared instrument/effect, bypass, graph, or renderer.",
		"Normal invocation expects ./sf2/HiDef.sf2, the vendored pinned Braids and Chorus DSP, and ./midi/Corridors of Time - Chrono Trigger.mid; resolves every fixture SoundFont identity to an exact catalog choice without fallback, configures one Chorus slot on the first Patch only, negotiates the physical device without starting it, builds the complete alternating instrument plus effect graph for that exact accepted configuration before starting the stream, and begins MIDI only after that succeeds. PATCH structural rows use the injected capacity-one preparation worker; there is no play command, transport, file chooser, preset browser/modal, engine-choice modal, effect selector, bypass, or alternate effect selection.",
		"Support --smoke for headless deterministic execution, --observe to print one CREST_OBSERVATION JSON object, --demo-scene only with --smoke --observe to run the exhaustive current-GUI scene, --demo-live-graphical-shell by itself to retain the Phase 1 physical witness, --demo-live-semantic-view-model by itself to retain the Phase 2 semantic scene, --demo-live-sixteen-track-mixer-routing by itself to run the cumulative corrective routing scene, --demo-live as a compatibility alias of that newest routing scene, and verification-only --degenerate-audio or --degenerate-control only with --smoke --observe for behavioral falsification. Reject duplicate, mixed, or other options; every live spelling is mutually exclusive with every headless, observation, demo-scene, and degenerate flag.",
		"When --demo-scene is present, print exactly one single-line CREST_EVENT_LOG JSON object, one single-line CREST_STATE_TREE JSON object, and one single-line CREST_OBSERVATION JSON object. Use stable Serialize data, never Debug output or Markdown; the ordinary interactive run emits no trace unless explicitly requested.",
			"When --demo-live is present, pass StandaloneApplication.runLiveDemo control-side callbacks that print each returned checkpoint as CREST_LIVE_CHECKPOINT JSON, then print exactly one compact CREST_LIVE_EVENT_LOG_SUMMARY JSON with lossless counts and chain endpoints, CREST_LIVE_STATE_TREE JSON, CREST_LIVE_COVERAGE JSON, and CREST_LIVE_SUMMARY human-readable line from the completed report after note cleanup; retain the complete EventLog in LiveDemoReport verification and never dump every performance MIDI record or print/format from the runner or audio callback.",
			"The live option uses the same HiDef.sf2, Braids/Chorus adapters, alternating Corridors of Time composition with first-Patch Chorus, AppLoop, EframeGraphicalWindow, CpalAudioOutput, prepared engine/effect racks, PreparedGraph, AudioRenderer, and mixer as normal interactive execution. It has no fake window, null device, offline-only renderer, silent fallback/bypass, direct state edit, or alternate demo reducer.",
		"After the graphical-shell live scene completes and teardown is observed on control ownership, print exactly one CREST_GRAPHICAL_SHELL_LIVE_OBSERVATION JSON object containing the declared region, PATCH/MIXER, physical-audio, note-cleanup, window-close, stream-release, and graph-collection fields; never infer those fields from labels or print them from the callback. The enclosing witness command's actual zero exit proves normal parent-process completion separately.",
		"After the semantic-view-model live scene completes and teardown is observed on control ownership, print exactly one CREST_SEMANTIC_VIEW_MODEL_LIVE_OBSERVATION JSON object containing measured context/surface/mode/return/action/status/healthy-error/focus-recovery/responsive, physical-audio, cleanup, close, stream, and graph fields; never infer those fields from labels or print them from the callback.",
		"After the sixteen-track routing live scene completes and teardown is observed on control ownership, print exactly one CREST_SIXTEEN_TRACK_MIXER_ROUTING_LIVE_OBSERVATION JSON object containing measured fixed-track, shared-route, trim/reroute, track-control, meter, snapshot, physical-audio, cleanup, close, stream, and graph fields; never infer those fields from labels or print them from the callback.",
		"After frozen scalar coverage, --demo-live selects one adjacent preset on the focused first SoundFont Patch, then SoundFont to Braids and back to descriptor-default SoundFont through semantic AppEvents and ThreadedGraphPreparationWorker; wait nonblockingly for every Preparing, Activating, Ready sequence and newer acknowledged graph revision, dispatch targeted MIDI, and require finite nonzero target output before advancing or reporting completion.",
		"Before --demo-live device startup, print one concise status explaining that the run is autonomous, input-isolated, and bounded. If the runner makes no semantic/checkpoint/lifecycle/cleanup progress for ten seconds or exceeds 120 seconds total, retain a typed stage-specific error, close the window, clean up notes, release the stream, shut down structural ownership off callback, and exit nonzero without a report.",
		"During --demo-live, ignore mapped semantic window input so it cannot interleave an AppState generation with the autonomous checkpoint protocol; native window close remains available as typed incomplete-demo cancellation and normal interactive mode keeps its existing keyboard dispatch.",
		"After live completion emit the four final records synchronously, return false from the same window tick to send a viewport-close command, release the physical stream after the window returns, and exit successfully when no runtime error was retained.",
		"The smoke path loads the real fixed SoundFont, pinned Braids and Chorus adapters, and MIDI fixture, installs alternating instrument Patches with one first-Patch Chorus, applies keyboard-equivalent navigation and adjustment events, renders bounded synthesis-to-effect-to-mix audio blocks, and reports measurements from the production services.",
		"The observation must exercise at least two simultaneously sounding Patches, edit a Patch whose id is greater than 1, compare the edited and unedited Patch stems separately, and prove the unedited stem is sample-identical while the edited stem changes.",
		"The observation must also drive the selected parameter to a boundary, send one more adjustment toward that boundary, then prove a later valid edit is accepted without restarting the application.",
		"The demo-scene observation delegates to ExhaustiveGuiDemo, includes its coverage summary in CREST_OBSERVATION, and returns a failure when the event journal drops a record, the final state tree disagrees, or any expected current event/property/parameter/effect identifier is missing.",
		"The demo scene uses DeterministicGraphPreparationWorker through the production port, selects one adjacent authored SoundFont preset and then the first Patch SoundFont to Braids to descriptor-default SoundFont, proves pending/busy/activation/retirement and audible target output for each structural edit, then injects one controlled preparation failure plus stale/mismatched outcomes without editing the report.",
		"Degenerate modes mutate the real seam before observation: --degenerate-control drops exactly one selected Adjust before AppLoop dispatch and --degenerate-audio clears the caller-owned buffer immediately after AudioRenderer.render. They never append a fake missing identifier, suppress peak accumulation, or overwrite an observation field. Demo-scene mutants emit the complete schema-valid trace and observation first, then exit 1 when detected; smoke mutants emit their schema-valid falsifying observation and exit 0 as declared by their legacy witnesses.",
	]
	validations: [
		{id: "validation.asset.crest_synth_smoke", kind: "integration", command: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke"], timeout: "180s", description: "the complete headless path runs against HiDef.sf2 and Corridors of Time"},
		{id: "validation.asset.crest_synth_demo_live", kind: "custom", command: ["make", "-n", "demo-live"], description: "the dedicated live option is reachable through one stable human command"},
	]
	contributesTo: [
		{capability: "capability.instrument_capability_model", contribution: "composes the installed providers and preparers for application-side exact registration without owning their behavior"},
		{capability: "capability.soundfont_preset_selection", contribution: "parses the fixed asset once, wires its catalog and numeric bank to the provider/preparer pair, and hosts live preset-selection proof without owning behavior"},
		{capability: "capability.soundfont_audio", contribution: "starts the concrete SoundFont synth"},
		{capability: "capability.braids_engine", contribution: "starts the pinned concrete Braids synth"},
		{capability: "capability.per_voice_envelope", contribution: "composes the canonical ADSR into both prepared engines"},
		{capability: "capability.static_patch_effect", contribution: "composes the one installed Chorus provider/preparer and first-Patch config into the complete graph and both demos"},
		{capability: "capability.prepared_engine_rack", contribution: "composes generic preparation and the dedicated structural ownership handoff without owning their behavior"},
		{capability: "capability.automatic_test_midi", contribution: "starts the fixed MIDI test input without transport controls"},
		{capability: "capability.one_way_parameter_control", contribution: "hosts the passive graphical control loop without owning behavior"},
		{capability: "capability.graphical_application_shell", contribution: "composes the selected eframe/egui shell and its phase-specific live proof without owning state or layout semantics"},
		{capability: "capability.schema_driven_patch_page", contribution: "composes direct page input and context projection without owning page state"},
		{capability: "capability.asynchronous_engine_selection", contribution: "composes the production worker and deterministic demo worker through one port with the canonical reducer and structural handoff"},
		{capability: "capability.realtime_execution", contribution: "composes the prepared control and audio sides"},
		{capability: "capability.observable_demo_scene", contribution: "emits stable event-log, state-tree, and coverage JSON for the exhaustive scene"},
		{capability: "capability.live_observable_demo", contribution: "selects the paced real-window mode and emits its control-side checkpoints and final summary without owning behavior"},
		{capability: "capability.sixteen_track_mixer_routing", contribution: "composes the canonical fixed mixer, newest physical routing witness, and fallback-free teardown without owning behavior"},
	]
}
