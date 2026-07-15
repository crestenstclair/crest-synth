package crestsynth

// These witnesses are provenance-bearing executable claims. crest-spec runs
// both commands against the committed tree and requires the real observation
// to pass while the schema-equivalent degenerate implementation fails.
project: witnesses: standalone_runtime: {
	scope: "goal"
	goal: "goal.perform_through_standalone"
	capability: "capability.external_midi_performance"
	resources: ["applicationService.Loop.StandaloneApplication", "applicationService.Loop.RenderCoordinator", "asset.SynthUiMain"]
	evidence: ["evidence.standalone_runtime"]
	command: ["cargo", "run", "--bin", "synth_ui", "--", "--smoke", "--observe"]
	negativeCommand: ["cargo", "run", "--bin", "synth_ui", "--", "--smoke", "--observe", "--degenerate-stub"]
	timeout: "60s"
	artifacts: ["target/debug/synth_ui"]
	observation: {
		kind: "json_stdout", marker: "CREST_OBSERVATION "
		schema: {events_dispatched: "number", peak: "number", clipped: "bool", channels_metered: "number", reducer_frames: "number"}
	}
	predicates: [
		{field: "events_dispatched", op: "gt", value: 0},
		{field: "peak", op: "gt", value: 0.05},
		{field: "peak", op: "lte", value: 1.0},
		{field: "clipped", op: "eq", value: false},
		{field: "channels_metered", op: "gt", value: 0},
		{field: "reducer_frames", op: "gt", value: 0},
	]
}

project: witnesses: expressive_polyphony: {
	scope: "goal"
	goal: "goal.exercise_supported_sound_architecture"
	capability: "capability.polyphonic_sound_generation"
	resources: ["aggregate.Engine.Voice", "domainService.Engine.VoiceAllocator", "domainService.Engine.EngineRenderer", "asset.VoiceDemoMain"]
	evidence: ["evidence.polyphonic_render"]
	command: ["cargo", "run", "--bin", "voice_demo", "--", "--observe", "--out", "/tmp/crest-synth-voice-witness.wav"]
	negativeCommand: ["cargo", "run", "--bin", "voice_demo", "--", "--observe", "--degenerate-stub", "--out", "/tmp/crest-synth-voice-negative.wav"]
	timeout: "60s"
	artifacts: ["target/debug/voice_demo"]
	observation: {kind: "json_stdout", marker: "CREST_OBSERVATION ", schema: {peak: "number", clipped: "bool", steals: "number", expressive_delta: "number"}}
	predicates: [
		{field: "peak", op: "gt", value: 0.1},
		{field: "clipped", op: "eq", value: false},
		{field: "steals", op: "gt", value: 0},
		{field: "expressive_delta", op: "gt", value: 0},
	]
}

project: witnesses: mixer_control_path: {
	scope: "goal"
	goal: "goal.operate_live_mixer"
	capability: "capability.pointer_free_mixer_control"
	resources: ["aggregate.Mixer.MixerView", "aggregate.Mixer.ChannelStrip", "domainService.Mixer.MixEngine", "asset.MixerDemoMain"]
	evidence: ["evidence.mixer_behavior"]
	command: ["cargo", "run", "--bin", "mixer_demo", "--", "--observe"]
	negativeCommand: ["cargo", "run", "--bin", "mixer_demo", "--", "--observe", "--degenerate-stub"]
	timeout: "60s"
	artifacts: ["target/debug/mixer_demo"]
	observation: {kind: "json_stdout", marker: "CREST_OBSERVATION ", schema: {peak: "number", bounded_edit: "bool", edge_scroll: "bool", solo_isolation: "bool", all_channels_metered: "bool"}}
	predicates: [
		{field: "peak", op: "gt", value: 0},
		{field: "bounded_edit", op: "eq", value: true},
		{field: "edge_scroll", op: "eq", value: true},
		{field: "solo_isolation", op: "eq", value: true},
		{field: "all_channels_metered", op: "eq", value: true},
	]
}

project: witnesses: realtime_boundary: {
	scope: "goal"
	goal: "goal.perform_through_standalone"
	capability: "capability.realtime_safe_execution"
	resources: ["adapter.RtrbEventRing", "adapter.TripleBufferParameterBridge", "adapter.BasedropDeferredDeallocator", "asset.MidiPlayLiveMain"]
	evidence: ["evidence.realtime_boundary"]
	command: ["cargo", "run", "--bin", "midi_play_live", "--", "--no-device-dry-run", "--observe"]
	negativeCommand: ["cargo", "run", "--bin", "midi_play_live", "--", "--no-device-dry-run", "--observe", "--degenerate-stub"]
	timeout: "60s"
	artifacts: ["target/debug/midi_play_live"]
	observation: {kind: "json_stdout", marker: "CREST_OBSERVATION ", schema: {event_delivered: "bool", latest_snapshot_read: "bool", reclaimed_off_audio_thread: "bool"}}
	predicates: [
		{field: "event_delivered", op: "eq", value: true},
		{field: "latest_snapshot_read", op: "eq", value: true},
		{field: "reclaimed_off_audio_thread", op: "eq", value: true},
	]
}

project: witnesses: preset_session_roundtrip: {
	scope: "goal"
	goal: "goal.preserve_reproducible_sound_state"
	capability: "capability.versioned_sound_state"
	resources: ["valueObject.Preset.Preset", "aggregate.Preset.Session", "adapter.SerdePresetCodec", "asset.PresetRoundtripDemoMain"]
	evidence: ["evidence.sound_state_roundtrip"]
	command: ["cargo", "run", "--bin", "preset_demo", "--", "--observe", "--out", "/tmp/crest-synth-preset-witness.wav"]
	negativeCommand: ["cargo", "run", "--bin", "preset_demo", "--", "--observe", "--degenerate-stub", "--out", "/tmp/crest-synth-preset-negative.wav"]
	timeout: "60s"
	artifacts: ["target/debug/preset_demo"]
	observation: {kind: "json_stdout", marker: "CREST_OBSERVATION ", schema: {roundtrip_equal: "bool", render_identical: "bool", failed_restore_atomic: "bool"}}
	predicates: [
		{field: "roundtrip_equal", op: "eq", value: true},
		{field: "render_identical", op: "eq", value: true},
		{field: "failed_restore_atomic", op: "eq", value: true},
	]
}

project: witnesses: scene_replay: {
	scope: "goal"
	goal: "goal.inspect_and_replay_behavior"
	capability: "capability.deterministic_scene_replay"
	resources: ["applicationService.Loop.SceneRunner", "applicationService.Loop.RenderCoordinator", "asset.SceneRunMain", "asset.SceneLibrary"]
	evidence: ["evidence.scene_replay"]
	command: ["cargo", "run", "--bin", "scene_run", "--", "--scene", "scenes/showcase.json", "--observe"]
	negativeCommand: ["cargo", "run", "--bin", "scene_run", "--", "--scene", "scenes/showcase.json", "--observe", "--degenerate-stub"]
	timeout: "60s"
	artifacts: ["target/debug/scene_run"]
	observation: {kind: "json_stdout", marker: "CREST_OBSERVATION ", schema: {events_applied: "number", rejections: "number", state_changes: "number", blocks_rendered: "number", peak: "number", deterministic: "bool"}}
	predicates: [
		{field: "events_applied", op: "gt", value: 0},
		{field: "rejections", op: "eq", value: 0},
		{field: "state_changes", op: "gt", value: 0},
		{field: "blocks_rendered", op: "gt", value: 0},
		{field: "peak", op: "gt", value: 0},
		{field: "deterministic", op: "eq", value: true},
	]
}
