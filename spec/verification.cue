package crestsynth

// Engine-executed behavioral witnesses. Each real and degenerate command uses
// the same typed observation contract; crest-spec owns execution, provenance,
// falsification, and evidence currency.
project: witnesses: expressive_polyphony: {
	scope: "goal"
	goal: "goal.perform_live"
	capability: "capability.render_expressive_sound"
	resources: ["aggregate.Engine.Voice", "domainService.Engine.VoiceAllocator", "domainService.Engine.EngineRenderer", "asset.VoiceDemoMain"]
	evidence: ["evidence.polyphonic_render"]
	command: ["cargo", "run", "--bin", "voice_demo", "--", "--observe", "--out", "/tmp/crest-synth-voice-witness.wav"]
	negativeCommand: ["cargo", "run", "--bin", "voice_demo", "--", "--observe", "--degenerate-stub", "--out", "/tmp/crest-synth-voice-negative.wav"]
	timeout: "60s"
	artifacts: ["target/debug/voice_demo"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_OBSERVATION "
		schema: {peak: "number", clipped: "bool", steals: "number", expressive_delta: "number"}
	}
	predicates: [
		{field: "peak", op: "gt", value: 0.1},
		{field: "clipped", op: "eq", value: false},
		{field: "steals", op: "gt", value: 0},
		{field: "expressive_delta", op: "gt", value: 0},
	]
}

project: witnesses: mixer_signal_path: {
	scope: "goal"
	goal: "goal.design_playable_sounds"
	capability: "capability.mix_to_stereo"
	resources: ["aggregate.Mixer.ChannelStrip", "aggregate.Mixer.MixBus", "domainService.Mixer.MixEngine", "asset.MixerDemoMain"]
	evidence: ["evidence.mixer_and_effects_path"]
	command: ["cargo", "run", "--bin", "mixer_demo", "--", "--observe"]
	negativeCommand: ["cargo", "run", "--bin", "mixer_demo", "--", "--observe", "--degenerate-stub"]
	timeout: "60s"
	artifacts: ["target/debug/mixer_demo"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_OBSERVATION "
		schema: {peak: "number", bounded: "bool", solo_isolation: "bool", all_channels_metered: "bool"}
	}
	predicates: [
		{field: "peak", op: "gt", value: 0},
		{field: "bounded", op: "eq", value: true},
		{field: "solo_isolation", op: "eq", value: true},
		{field: "all_channels_metered", op: "eq", value: true},
	]
}

project: witnesses: preset_session_roundtrip: {
	scope: "goal"
	goal: "goal.preserve_work"
	capability: "capability.save_and_restore_sound_library"
	resources: ["valueObject.Preset.Preset", "aggregate.Preset.Session", "adapter.SerdePresetCodec", "asset.PresetRoundtripDemoMain"]
	evidence: ["evidence.preset_and_session_roundtrip"]
	command: ["cargo", "run", "--bin", "preset_demo", "--", "--observe", "--out", "/tmp/crest-synth-preset-witness.wav"]
	negativeCommand: ["cargo", "run", "--bin", "preset_demo", "--", "--observe", "--degenerate-stub", "--out", "/tmp/crest-synth-preset-negative.wav"]
	timeout: "60s"
	artifacts: ["target/debug/preset_demo"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_OBSERVATION "
		schema: {roundtrip_equal: "bool", render_identical: "bool", failed_restore_atomic: "bool"}
	}
	predicates: [
		{field: "roundtrip_equal", op: "eq", value: true},
		{field: "render_identical", op: "eq", value: true},
		{field: "failed_restore_atomic", op: "eq", value: true},
	]
}

project: witnesses: realtime_boundary: {
	scope: "goal"
	goal: "goal.perform_live"
	capability: "capability.preserve_realtime_safety"
	resources: ["adapter.RtrbEventRing", "adapter.TripleBufferParameterBridge", "adapter.BasedropDeferredDeallocator", "asset.MidiPlayLiveMain"]
	evidence: ["evidence.realtime_boundary_contract"]
	command: ["cargo", "run", "--bin", "midi_play_live", "--", "--no-device-dry-run", "--observe"]
	negativeCommand: ["cargo", "run", "--bin", "midi_play_live", "--", "--no-device-dry-run", "--observe", "--degenerate-stub"]
	timeout: "60s"
	artifacts: ["target/debug/midi_play_live"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_OBSERVATION "
		schema: {event_delivered: "bool", latest_snapshot_read: "bool", reclaimed_off_audio_thread: "bool"}
	}
	predicates: [
		{field: "event_delivered", op: "eq", value: true},
		{field: "latest_snapshot_read", op: "eq", value: true},
		{field: "reclaimed_off_audio_thread", op: "eq", value: true},
	]
}

project: witnesses: gamepad_editor_journey: {
	scope: "goal"
	goal: "goal.operate_standalone"
	capability: "capability.edit_without_pointer"
	resources: ["domainService.Shell.GamepadNavigator", "aggregate.Mixer.MixerView", "aggregate.Loop.AppState", "asset.GamepadNavDemoMain"]
	evidence: ["evidence.gamepad_editor_journey"]
	command: ["cargo", "run", "--bin", "gamepad_demo", "--", "--observe"]
	negativeCommand: ["cargo", "run", "--bin", "gamepad_demo", "--", "--observe", "--degenerate-stub"]
	timeout: "60s"
	artifacts: ["target/debug/gamepad_demo"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_OBSERVATION "
		schema: {actions_dispatched: "number", state_changed: "bool", bounded_edit: "bool", state_published: "bool"}
	}
	predicates: [
		{field: "actions_dispatched", op: "gt", value: 0},
		{field: "state_changed", op: "eq", value: true},
		{field: "bounded_edit", op: "eq", value: true},
		{field: "state_published", op: "eq", value: true},
	]
}
