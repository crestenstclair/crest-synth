package crestsynth

project: witnesses: running_synth: {
	scope: "goal"
	goal: "goal.play_test_song"
	capability: "capability.soundfont_audio"
	resources: [
		"adapter.HiDefSoundFontEngine",
		"adapter.CorridorsMidiEventSource",
		"applicationService.Testing.AutomaticMidiTest",
		"applicationService.RealTime.AudioRenderer",
		"applicationService.Shell.StandaloneApplication",
		"asset.CrestSynthMain",
	]
	evidence: ["evidence.running_synth"]
	command: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke", "--observe"]
	negativeCommand: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke", "--observe", "--degenerate-audio"]
	timeout: "180s"
	artifacts: ["target/debug/crest-synth"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_OBSERVATION "
		schema: {
			soundfont_loaded: "bool"
			instrument_patches: "number"
			presets_match: "bool"
			round_robin_channels: "bool"
			automatic_midi: "bool"
			event_commands_delivered: "number"
			callback_allocations: "number"
			peak: "number"
		}
	}
	predicates: [
		{field: "soundfont_loaded", op: "eq", value: true},
		{field: "instrument_patches", op: "gt", value: 1},
		{field: "presets_match", op: "eq", value: true},
		{field: "round_robin_channels", op: "eq", value: true},
		{field: "automatic_midi", op: "eq", value: true},
		{field: "event_commands_delivered", op: "gt", value: 0},
		{field: "callback_allocations", op: "eq", value: 0},
		{field: "peak", op: "gt", value: 0.001},
		{field: "peak", op: "lte", value: 1.0},
	]
}

project: witnesses: control_path: {
	scope: "goal"
	goal: "goal.control_synth"
	capability: "capability.one_way_parameter_control"
	resources: [
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"adapter.LockFreeAudioBoundary",
		"adapter.EframeTextWindow",
		"applicationService.Shell.StandaloneApplication",
		"asset.CrestSynthMain",
	]
	evidence: ["evidence.control_path"]
	command: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke", "--observe"]
	negativeCommand: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke", "--observe", "--degenerate-control"]
	timeout: "180s"
	artifacts: ["target/debug/crest-synth"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_OBSERVATION "
		schema: {
			patch_rows: "number"
			channel_separators: "number"
			one_value_changed: "bool"
			state_roundtrip: "bool"
			text_matches_state: "bool"
			parameter_published: "bool"
			engine_consumed_value: "bool"
			audio_changed: "bool"
		}
	}
	predicates: [
		{field: "patch_rows", op: "gt", value: 1},
		{field: "channel_separators", op: "gt", value: 0},
		{field: "one_value_changed", op: "eq", value: true},
		{field: "state_roundtrip", op: "eq", value: true},
		{field: "text_matches_state", op: "eq", value: true},
		{field: "parameter_published", op: "eq", value: true},
		{field: "engine_consumed_value", op: "eq", value: true},
		{field: "audio_changed", op: "eq", value: true},
	]
}
