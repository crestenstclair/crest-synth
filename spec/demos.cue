package crestsynth

// Each proof binary imports canonical library resources and measures a focused
// behavioral slice. None is an earlier "phase" implementation of the product.
project: assets: VoiceDemoMain: {
	kind: "rust-bin-target"
	description: "src/bin/voice_demo.rs: expressive over-polyphonic virtual-analog proof"
	profile: {kind: "verification_harness", witness: "voice stealing and per-note expression", failurePolicy: "silence, clipping, no steal, or no expression delta fails"}
	targets: ["aggregate.Engine.Voice", "domainService.Engine.VoiceAllocator", "domainService.Engine.EngineRenderer"]
	prompts: [
		"Use canonical Voice, VoiceAllocator, VoiceRenderer, EngineRenderer, and AudioFrame types. Configure four voices, hold more than four notes, and render attack/decay/sustain/release through the virtual-analog path.",
		"Measure steals, stereo peak, clipping, and the rendered delta after expression is applied to one matching NoteId; assert other voices are unchanged.",
		"Write voice-demo.wav. --observe emits CREST_OBSERVATION with peak, clipped, steals, expressive_delta. --degenerate-stub is schema-compatible silence/no-expression and exits 0 for crest-spec classification.",
	]
	validations: [{kind: "integration", command: ["make", "demo-voices"], description: "bounded polyphony audibly steals and honors isolated expression", assertions: [{kind: "exit_code", expected: 0}, {kind: "file_exists", path: "voice-demo.wav"}, {kind: "stdout_contains", pattern: "steals="}]}]
	contributesTo: [{capability: "capability.polyphonic_sound_generation", contribution: "falsifies silent, unbounded, non-stealing, and non-expressive voice implementations"}]
}

project: assets: SamplePlayDemoMain: {
	kind: "rust-bin-target"
	description: "src/bin/sample_demo.rs: hermetic zoned sample loading and pitched playback proof"
	profile: {kind: "verification_harness", witness: "sample decode, zone selection, interpolation, and render", failurePolicy: "single-zone or no-op interpolation fails"}
	targets: ["adapter.SymphoniaSampleLoader", "aggregate.Sample.SampleSet", "domainService.Sample.ZoneResolver", "domainService.Sample.SamplePlayer"]
	prompts: [
		"Synthesize a temporary mono WAV in code, load it through SymphoniaSampleLoader, and clean it up. Use canonical SampleData, SampleSet, Zone, ZoneResolver, and SamplePlayer types; do not create SampleLibrary or SampleInterpolator substitutes.",
		"Configure at least two non-overlapping key/velocity zones referencing the loaded sample. Play values that hit distinct zones and pitch at least one away from its root through the configured interpolation mode.",
		"Assert >=2 zones loaded, >=2 distinct zones selected, pitch-shifted output differs from root-rate output, and rendered peak is non-zero. Write sample-demo.wav and print measured zone markers.",
	]
	validations: [{kind: "integration", command: ["make", "demo-samples"], description: "real decoded sample data routes through distinct zones and interpolation", assertions: [{kind: "exit_code", expected: 0}, {kind: "file_exists", path: "sample-demo.wav"}, {kind: "stdout_contains", pattern: "distinct zones hit="}]}]
	contributesTo: [{capability: "capability.configurable_instrument_graph", contribution: "proves the sample source can participate as a real configured instrument path"}]
}

project: assets: EffectsDemoMain: {
	kind: "rust-bin-target"
	description: "src/bin/effects_demo.rs: ordered and bypassable strip/master effect-chain proof"
	profile: {kind: "verification_harness", witness: "effect order and bypass", failurePolicy: "order-insensitive or lossy bypass fails"}
	targets: ["aggregate.Effects.EffectChain", "domainService.Effects.ChainRenderer", "port.Effects.EffectProcessor", "domainService.Mixer.MixEngine"]
	prompts: [
		"Use canonical EffectChain, EffectSlot, EffectProcessor, ChainRenderer, AudioFrame, and MixEngine types. Supply small concrete gain and delay processors as port adapters, not replacement domain models.",
		"Process a measured block through two non-commutative slots in normal and reverse order and assert outputs differ. Bypass the chain and assert bit-identical passthrough.",
		"Render a multi-patch passage through strip effects and master effects to effects-demo.wav; print `slot order matters: true` and `bypass passthrough: true` only after assertions.",
	]
	validations: [{kind: "integration", command: ["make", "demo-effects"], description: "effect slot order matters and bypass is transparent", assertions: [{kind: "exit_code", expected: 0}, {kind: "file_exists", path: "effects-demo.wav"}, {kind: "stdout_contains", pattern: "slot order matters: true"}, {kind: "stdout_contains", pattern: "bypass passthrough: true"}]}]
	contributesTo: [{capability: "capability.stereo_mix_pipeline", contribution: "proves canonical insert chains execute in order and bypass transparently"}]
}

project: assets: ModPlayMain: {
	kind: "rust-bin-target"
	description: "src/bin/mod_play.rs: audible modulation-matrix proof"
	profile: {kind: "verification_harness", witness: "LFO pitch and filter routing", failurePolicy: "configured routes must measurably alter rendered output"}
	targets: ["aggregate.Modulation.ModMatrix", "domainService.Modulation.ModProcessor", "aggregate.Patch.Patch", "applicationService.Loop.RenderCoordinator"]
	prompts: [
		"Use canonical ModMatrix/ModRoute/LfoConfig/ModProcessor and Patch types. Configure LFO-to-pitch vibrato and LFO-or-envelope-to-filter-cutoff sweep on at least one patch.",
		"Render the same sustained passage with routing enabled and bypassed through RenderCoordinator. Assert the buffers differ and remain non-silent/bounded; print measured route descriptions and write mod-play.wav.",
	]
	validations: [{kind: "integration", command: ["make", "demo-mod"], description: "configured modulation routes measurably alter the canonical render graph", assertions: [{kind: "exit_code", expected: 0}, {kind: "file_exists", path: "mod-play.wav"}, {kind: "stdout_contains", pattern: "mod routing:"}]}]
	contributesTo: [{capability: "capability.configurable_instrument_graph", contribution: "proves modulation routes alter real patch rendering rather than existing as inert configuration"}]
}

project: assets: PatchPlayMain: {
	kind: "rust-bin-target"
	description: "src/bin/patch_play.rs: multi-patch MIDI routing and independent voice-pool proof"
	profile: {kind: "verification_harness", witness: "addressed layering and independent polyphony", failurePolicy: "leakage, missed delivery, or shared voice exhaustion fails"}
	targets: ["aggregate.Patch.Patch", "applicationService.Patch.PatchManager", "domainService.Patch.MidiDispatcher", "applicationService.Loop.RenderCoordinator"]
	prompts: [
		"Create two or three canonical Patch aggregates with distinct virtual-analog configs, channel mappings, mixer strips, and independent VoiceAllocators. Use PatchManager to validate the collection and MidiDispatcher for every event.",
		"The built-in timeline must address every patch and include an intentional layered address. Assert matching delivery exactly once, no delivery to an unmapped patch, independent peak voice counts, non-zero bounded stereo output, and at least one configured voice steal.",
		"This proof uses its built-in addressed timeline; MIDI-file instrument partitioning is proved separately by midi_play and synth_ui. Write patch-play.wav and print `Peak Voices` per patch from measured allocators.",
	]
	validations: [{kind: "integration", command: ["make", "demo-patches"], description: "MIDI dispatch, per-patch pools, and the global mix compose end to end", assertions: [{kind: "exit_code", expected: 0}, {kind: "file_exists", path: "patch-play.wav"}, {kind: "stdout_contains", pattern: "Peak Voices"}]}]
	contributesTo: [
		{capability: "capability.external_midi_performance", contribution: "proves intentional channel layering without leakage"},
		{capability: "capability.configurable_instrument_graph", contribution: "proves multiple canonical patches own independent sound and mixer configuration"},
	]
}

project: assets: PresetRoundtripDemoMain: {
	kind: "rust-bin-target"
	description: "src/bin/preset_demo.rs: complete versioned patch/session atomic round-trip proof"
	profile: {kind: "verification_harness", witness: "complete state restore and equivalent render", failurePolicy: "lossy, unsupported, or partially mutating restore fails"}
	targets: ["valueObject.Preset.Preset", "aggregate.Preset.Session", "adapter.SerdePresetCodec", "applicationService.Preset.SessionManager", "applicationService.Loop.RenderCoordinator"]
	prompts: [
		"Build a complete Session containing patches, sample references, modulation, routing, effect chains, mixer state, tempo, and time signature using canonical resource types. Encode/decode only through SerdePresetCodec; never implement an inline parallel codec or Setup type.",
		"Assert restored Session equality and bit-identical rendering of a fixed passage through RenderCoordinator. Then decode malformed and unsupported-version bytes and assert active state remains byte-identical.",
		"Write preset-demo.wav. --observe emits roundtrip_equal, render_identical, failed_restore_atomic; --degenerate-stub uses a deliberately lossy/non-atomic codec behind the same harness.",
	]
	validations: [{kind: "integration", command: ["make", "demo-presets"], description: "complete versioned state round-trips, renders equivalently, and restores atomically", assertions: [{kind: "exit_code", expected: 0}, {kind: "file_exists", path: "preset-demo.wav"}, {kind: "stdout_contains", pattern: "session roundtrip: equal"}, {kind: "stdout_contains", pattern: "render identical: true"}]}]
	contributesTo: [{capability: "capability.versioned_sound_state", contribution: "provides the falsification-gated complete session persistence witness"}]
}

project: assets: MidiPlayMain: {
	kind: "rust-bin-target"
	description: "src/bin/midi_play.rs: offline Standard MIDI File to WAV proof"
	profile: {kind: "verification_harness", witness: "instrument-partitioned MIDI render", failurePolicy: "empty parts, missing patches, invalid assignments, or silent render fails"}
	targets: ["adapter.MidlyMidiFileReader", "applicationService.MidiFile.TestPlaybackAssembler", "domainService.MidiFile.Sequencer", "applicationService.Loop.RenderCoordinator"]
	prompts: [
		"CLI: midi_play [FILE.mid] [--observe] [--degenerate-stub]. The optional path defaults to the built-in multi-instrument Song; reject unknown or duplicate inputs clearly.",
		"Read an optional MIDI file with MidlyMidiFileReader or construct a built-in Song with at least three bank/program identities plus percussion. Prepare it with TestPlaybackAssembler, then schedule each targeted part through Sequencer and RenderCoordinator; do not define local song, patch, mixer, or renderer substitutes.",
		"Assert one generated Patch per InstrumentPart, unique patch IDs, part N assigned to mixer track N % 16, every event targeted to its part patch, events > 0, duration > 0, and 0 < peak <= 1. Include a >16-part unit case proving deterministic track sharing without dropped patches.",
		"Write midi-play.wav and print `instrument parts=<N>`, `generated patches=<N>`, one `track assignment: <label> -> Txx` per part, and `rendered seconds=<value>`.",
		"--observe emits instrument_parts, one_patch_per_instrument, round_robin_assignment, all_events_targeted, and peak. --degenerate-stub deliberately collapses parts, corrupts one assignment, or removes a target while preserving the observation schema.",
	]
	validations: [{kind: "integration", command: ["make", "demo-midi"], description: "instrument parts become round-robin-assigned patches and render audibly offline", assertions: [{kind: "exit_code", expected: 0}, {kind: "file_exists", path: "midi-play.wav"}, {kind: "stdout_contains", pattern: "instrument parts="}, {kind: "stdout_contains", pattern: "generated patches="}, {kind: "stdout_contains", pattern: "track assignment:"}, {kind: "stdout_contains", pattern: "rendered seconds="}]}]
	contributesTo: [{capability: "capability.instrument_partitioned_test_playback", contribution: "proves real instrument partitioning, patch materialization, modulo-16 assignment, and targeted offline rendering"}]
}

project: assets: MidiPlayLiveMain: {
	kind: "rust-bin-target"
	description: "src/bin/midi_play_live.rs: live MIDI-file host and hermetic real-time-boundary proof"
	profile: {kind: "verification_harness", witness: "event ring, latest snapshot, and deferred destruction", failurePolicy: "broken boundary facts fail witness predicates"}
	targets: ["adapter.MidlyMidiFileReader", "applicationService.MidiFile.TestPlaybackAssembler", "domainService.MidiFile.Sequencer", "adapter.CpalAudioOutput", "applicationService.Loop.RenderCoordinator", "adapter.RtrbEventRing", "adapter.TripleBufferParameterBridge", "adapter.BasedropDeferredDeallocator"]
	prompts: [
		"Live mode builds the same one-patch-per-instrument TestPlaybackPlan as offline/synth_ui playback, schedules its targeted events through RenderCoordinator, and writes exactly each callback-requested stereo frame slice to CpalAudioOutput. Report unavailable devices clearly; never panic or move a non-Send stream across threads.",
		"--no-device-dry-run opens no device and performs concrete boundary facts: push/pop one event, publish two snapshots/read newest, retire tracked state on the simulated audio side and collect/drop it on the control side.",
		"--observe emits event_delivered, latest_snapshot_read, reclaimed_off_audio_thread. --degenerate-stub breaks one boundary seam while preserving the schema.",
	]
	validations: [{kind: "integration", command: ["make", "check-live"], description: "the complete real-time seam is exercised without hardware", assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "dry-run ok: pipeline constructed"}]}]
	contributesTo: [{capability: "capability.realtime_safe_execution", contribution: "provides a hermetic, falsification-gated proof of all three real-time boundary mechanisms"}]
}

project: assets: MixerDemoMain: {
	kind: "rust-bin-target"
	description: "src/bin/mixer_demo.rs: headless mixer reducer and strip-to-master proof"
	profile: {kind: "verification_harness", witness: "all-track navigation, inspector, editing, solo, and metering", failurePolicy: "missing track/inspector state, bounds, or mix semantics failure exits non-zero"}
	targets: ["aggregate.Mixer.MixerView", "aggregate.Loop.AppState", "domainService.Mixer.MixEngine"]
	prompts: [
		"Open no GUI/device. Drive canonical AppEvent::Mixer values through AppState.apply, not MixerView directly. Prove T00-T0F labels all exist simultaneously, navigation saturates at each end, the derived inspector follows cursor track/parameter/patch, compact display values are correct, fine/coarse values clamp, and directional input cannot toggle booleans.",
		"Render non-zero buffers on all strips through MixEngine, solo one, assert only it reaches master while every strip retains pre-solo metering.",
		"--observe emits peak, bounded_edit, all_tracks_visible, inspector_consistent, solo_isolation, all_channels_metered. --degenerate-stub omits a track/inspector update or bypasses reducer/solo/meter logic.",
	]
	validations: [{kind: "integration", command: ["make", "demo-mixer"], description: "the authoritative reducer drives all-track mixer control, inspector projection, and audio semantics", assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "tracks visible: 16"}, {kind: "stdout_contains", pattern: "inspector follows cursor: true"}, {kind: "stdout_contains", pattern: "metering independent of solo: true"}]}]
	contributesTo: [
		{capability: "capability.pointer_free_mixer_control", contribution: "proves the complete headless mixer interaction journey through AppState"},
		{capability: "capability.stereo_mix_pipeline", contribution: "falsifies incorrect solo gating and post-solo metering"},
	]
}

project: assets: GamepadNavDemoMain: {
	kind: "rust-bin-target"
	description: "src/bin/gamepad_demo.rs: headless raw-controller to AppEvent parity proof"
	profile: {kind: "verification_harness", witness: "gamepad translation and controller glyphs", failurePolicy: "wrong semantic action, state, or glyph mapping fails"}
	targets: ["domainService.Shell.GamepadNavigator", "domainService.Shell.GlyphResolver", "aggregate.Loop.AppState", "domainService.Loop.StateProjector"]
	prompts: [
		"Open no device/window. Feed deterministic raw GamepadEvents through GamepadNavigator, translate the resulting actions into the same AppEvent::Mixer variants as keyboard input, and apply them to AppState.",
		"Assert expected actions, final mixer cursor/edit state, bounded values, and successful StateProjector publication. Resolve distinct glyphs for at least two controller families.",
	]
	validations: [{kind: "integration", command: ["make", "check-gamepad"], description: "controller-independent semantic actions reach authoritative state", assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "nav actions ok:"}, {kind: "stdout_contains", pattern: "glyphs resolved: per-controller"}]}]
	contributesTo: [{capability: "capability.pointer_free_mixer_control", contribution: "proves gamepad input maps to the same semantic mixer reducer path as keyboard input"}]
}
