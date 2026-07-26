package crestsynth

project: witnesses: running_synth: {
	scope: "goal"
	goal: "goal.play_test_song"
	capability: "capability.soundfont_audio"
	resources: [
		"adapter.HiDefSoundFontPreparer",
		"adapter.BraidsPreparer",
		"adapter.CorridorsMidiEventSource",
		"applicationService.Testing.AutomaticMidiTest",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"applicationService.Shell.StandaloneApplication",
		"asset.CrestSynthMain",
	]
	repairResources: [
		"adapter.HiDefSoundFontPreparer",
		"adapter.BraidsPreparer",
		"applicationService.Testing.AutomaticMidiTest",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"applicationService.Shell.StandaloneApplication",
		"asset.CrestSynthMain",
	]
	counterexampleRepairResources: ["applicationService.Shell.StandaloneApplication", "asset.CrestSynthMain"]
	evidence: ["evidence.running_synth"]
	command: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke", "--observe"]
	negativeCommand: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke", "--observe", "--degenerate-audio"]
	negativeExpectedExitCode: 0
	timeout: "180s"
	artifacts: ["target/debug/crest-synth"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_OBSERVATION "
			schema: {
			parsed_soundfont_banks: "number"
			prepared_instruments: "number"
			soundfont_patches: "number"
			braids_patches: "number"
			alternating_capabilities: "bool"
			active_graph_revision: "number"
			presets_match: "bool"
			distinct_patch_channels: "bool"
			distinct_patch_stems: "bool"
			automatic_midi: "bool"
			event_commands_delivered: "number"
			callback_allocations: "number"
			callback_destructions: "number"
			peak: "number"
		}
	}
	predicates: [
		{field: "parsed_soundfont_banks", op: "eq", value: 1},
		{field: "prepared_instruments", op: "gt", value: 1},
		{field: "soundfont_patches", op: "gt", value: 0},
		{field: "braids_patches", op: "gt", value: 0},
		{field: "alternating_capabilities", op: "eq", value: true},
		{field: "active_graph_revision", op: "gt", value: 0},
		{field: "presets_match", op: "eq", value: true},
		{field: "distinct_patch_channels", op: "eq", value: true},
		{field: "distinct_patch_stems", op: "eq", value: true},
		{field: "automatic_midi", op: "eq", value: true},
		{field: "event_commands_delivered", op: "gt", value: 0},
		{field: "callback_allocations", op: "eq", value: 0},
		{field: "callback_destructions", op: "eq", value: 0},
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
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"adapter.HiDefSoundFontPreparer",
		"asset.CrestSynthMain",
	]
	repairResources: [
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"applicationService.Shell.StandaloneApplication",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"asset.CrestSynthMain",
	]
	counterexampleRepairResources: ["applicationService.Shell.StandaloneApplication", "asset.CrestSynthMain"]
	evidence: ["evidence.control_path"]
	command: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke", "--observe"]
	negativeCommand: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke", "--observe", "--degenerate-control"]
	negativeExpectedExitCode: 0
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
			edited_patch_id: "number"
			edited_patch_audio_changed: "bool"
			unedited_patch_audio_unchanged: "bool"
			per_patch_audio_isolated: "bool"
			boundary_noop_nonfatal: "bool"
			post_boundary_edit_accepted: "bool"
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
		{field: "edited_patch_id", op: "gt", value: 1},
		{field: "edited_patch_audio_changed", op: "eq", value: true},
		{field: "unedited_patch_audio_unchanged", op: "eq", value: true},
		{field: "per_patch_audio_isolated", op: "eq", value: true},
		{field: "boundary_noop_nonfatal", op: "eq", value: true},
		{field: "post_boundary_edit_accepted", op: "eq", value: true},
	]
}

project: witnesses: exhaustive_demo_scene: {
	scope: "goal"
	goal: "goal.observe_synth"
	capability: "capability.observable_demo_scene"
	resources: [
		"adapter.BraidsCapability",
		"adapter.BraidsPreparer",
		"valueObject.Synth.VoiceEnvelope",
		"valueObject.Control.TopLevelContext",
		"valueObject.Control.InteractionState",
		"valueObject.Control.AppEvent",
		"valueObject.Control.EventRecord",
		"valueObject.Control.EventLog",
		"valueObject.Control.StateTree",
		"valueObject.Control.PatchPageProjection",
		"valueObject.Control.TextProjection",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"valueObject.Shell.WindowInput",
		"applicationService.Shell.KeyboardInputTranslator",
		"adapter.EframeTextWindow",
		"valueObject.Kernel.MidiMessage",
		"valueObject.Testing.DemoScene",
		"valueObject.Testing.DemoSceneReport",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.AutomaticMidiTest",
		"valueObject.Mixer.ChannelParameters",
		"valueObject.Mixer.GlobalParameters",
		"port.Mixer.GlobalEffectsProcessor",
		"adapter.GlobalReverbDelay",
		"domainService.Mixer.MixEngine",
		"valueObject.RealTime.ParameterSnapshot",
		"valueObject.RealTime.AudioCommand",
		"valueObject.RealTime.PatchAudioBlock",
		"applicationService.RealTime.AudioRenderer",
		"applicationService.Shell.StandaloneApplication",
		"asset.CrestSynthMain",
	]
	repairResources: [
		"adapter.BraidsCapability",
		"adapter.BraidsPreparer",
		"valueObject.Synth.VoiceEnvelope",
		"valueObject.Control.TopLevelContext",
		"valueObject.Control.InteractionState",
		"valueObject.Control.AppEvent",
		"valueObject.Control.EventRecord",
		"valueObject.Control.EventLog",
		"valueObject.Control.StateTree",
		"valueObject.Control.PatchPageProjection",
		"valueObject.Control.TextProjection",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"valueObject.Shell.WindowInput",
		"applicationService.Shell.KeyboardInputTranslator",
		"adapter.EframeTextWindow",
		"valueObject.Kernel.MidiMessage",
		"valueObject.Testing.DemoScene",
		"valueObject.Testing.DemoSceneReport",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.AutomaticMidiTest",
		"valueObject.Mixer.ChannelParameters",
		"valueObject.Mixer.GlobalParameters",
		"adapter.GlobalReverbDelay",
		"domainService.Mixer.MixEngine",
		"valueObject.RealTime.ParameterSnapshot",
		"valueObject.RealTime.AudioCommand",
		"valueObject.RealTime.PatchAudioBlock",
		"applicationService.RealTime.AudioRenderer",
		"applicationService.Shell.StandaloneApplication",
		"asset.CrestSynthMain",
	]
	counterexampleRepairResources: ["applicationService.Testing.ExhaustiveGuiDemo", "applicationService.Shell.StandaloneApplication", "asset.CrestSynthMain"]
	evidence: ["evidence.exhaustive_demo_scene"]
	command: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke", "--observe", "--demo-scene"]
	negativeCommand: ["cargo", "run", "--bin", "crest-synth", "--", "--smoke", "--observe", "--demo-scene", "--degenerate-control"]
	negativeExpectedExitCode: 1
	timeout: "180s"
	artifacts: ["target/debug/crest-synth"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_OBSERVATION "
			schema: {
			demo_scene_complete: "bool"
			active_graph_revision: "number"
			parsed_soundfont_banks: "number"
			prepared_instruments: "number"
			braids_patches: "number"
			alternating_capabilities: "bool"
			callback_destructions: "number"
			event_log_records: "number"
			event_log_dropped: "number"
			state_tree_schema_version: "number"
			state_tree_patch_count: "number"
			window_input_cases_exercised: "number"
				app_event_variants_exercised: "number"
				top_level_contexts_exercised: "number"
			event_sources_exercised: "number"
			navigate_directions_exercised: "number"
			adjust_directions_exercised: "number"
			midi_message_kinds_exercised: "number"
			audio_command_variants_exercised: "number"
			rejection_variants_exercised: "number"
			global_parameter_cases_exercised: "number"
			envelope_parameter_cases_exercised: "number"
			braids_scalar_cases_exercised: "number"
			all_patch_parameter_cases_exercised: "bool"
			all_serialized_properties_observed: "bool"
			accepted_events: "number"
			rejected_events: "number"
			scene_checkpoints: "number"
			coverage_missing: "number"
			state_hash_chain_valid: "bool"
			generation_chain_valid: "bool"
			final_state_tree_matches: "bool"
			gui_projection_matches_state: "bool"
			parameter_projection_matches_state: "bool"
			all_audio_parameter_effects_observed: "bool"
			mixed_engine_stems_nonzero: "bool"
			mixed_engine_parameter_isolation: "bool"
				post_rejection_event_accepted: "bool"
				schema_surface_equal: "bool"
				unexpected_coverage: "number"
				exact_state_values: "bool"
				exact_projection_values: "bool"
				baseline_restored: "bool"
				reverb_input_nonzero: "bool"
				delay_input_nonzero: "bool"
				causal_audio_comparisons: "bool"
				faithful_effect_path: "bool"
				descriptors_unique: "bool"
				event_record_payloads_exact: "bool"
				all_parameter_boundaries_exercised: "bool"
				selection_clamps_exact: "bool"
				tick_events_exact: "bool"
				two_run_trace_equal: "bool"
			}
	}
	predicates: [
		{field: "demo_scene_complete", op: "eq", value: true, repairResources: ["valueObject.Testing.DemoSceneReport", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "active_graph_revision", op: "gt", value: 0, repairResources: ["valueObject.RealTime.GraphRevision", "valueObject.Control.StateTree", "applicationService.RealTime.AudioRenderer"]},
		{field: "parsed_soundfont_banks", op: "eq", value: 1, repairResources: ["adapter.HiDefSoundFontPreparer", "applicationService.Shell.StandaloneApplication"]},
		{field: "prepared_instruments", op: "gt", value: 1, repairResources: ["aggregate.RealTime.PreparedEngineRack", "applicationService.RealTime.PreparedGraphBuilder"]},
		{field: "braids_patches", op: "gt", value: 0, repairResources: ["adapter.BraidsCapability", "adapter.BraidsPreparer", "applicationService.Testing.AutomaticMidiTest"]},
		{field: "alternating_capabilities", op: "eq", value: true, repairResources: ["applicationService.Testing.AutomaticMidiTest", "applicationService.Shell.StandaloneApplication"]},
		{field: "callback_destructions", op: "eq", value: 0, repairResources: ["applicationService.RealTime.AudioRenderer", "port.RealTime.StructuralGraphBoundary"]},
		{field: "event_log_records", op: "gt", value: 0, repairResources: ["valueObject.Control.EventLog", "applicationService.Control.AppLoop", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "event_log_dropped", op: "eq", value: 0, repairResources: ["valueObject.Control.EventLog", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "state_tree_schema_version", op: "gt", value: 0, repairResources: ["valueObject.Control.StateTree", "domainService.Control.StateProjector"]},
		{field: "state_tree_patch_count", op: "gt", value: 1, repairResources: ["aggregate.Control.AppState", "valueObject.Control.StateTree", "domainService.Control.StateProjector", "applicationService.Testing.AutomaticMidiTest"]},
		{field: "window_input_cases_exercised", op: "eq", value: 17, repairResources: ["valueObject.Shell.WindowInput", "applicationService.Shell.KeyboardInputTranslator", "valueObject.Testing.DemoScene", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "app_event_variants_exercised", op: "eq", value: 5, repairResources: ["valueObject.Control.AppEvent", "valueObject.Testing.DemoScene", "applicationService.Control.AppLoop"]},
		{field: "top_level_contexts_exercised", op: "eq", value: 2, repairResources: ["valueObject.Control.TopLevelContext", "valueObject.Control.InteractionState", "valueObject.Testing.DemoScene", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "event_sources_exercised", op: "eq", value: 5, repairResources: ["valueObject.Control.EventRecord", "applicationService.Control.AppLoop", "applicationService.Testing.AutomaticMidiTest", "applicationService.Shell.StandaloneApplication"]},
		{field: "navigate_directions_exercised", op: "eq", value: 4, repairResources: ["valueObject.Control.AppEvent", "valueObject.Testing.DemoScene", "applicationService.Shell.KeyboardInputTranslator"]},
		{field: "adjust_directions_exercised", op: "eq", value: 4, repairResources: ["valueObject.Control.AppEvent", "valueObject.Testing.DemoScene", "applicationService.Shell.KeyboardInputTranslator"]},
		{field: "midi_message_kinds_exercised", op: "eq", value: 7, repairResources: ["valueObject.Kernel.MidiMessage", "valueObject.Testing.DemoScene", "applicationService.Testing.AutomaticMidiTest"]},
		{field: "audio_command_variants_exercised", op: "eq", value: 2, repairResources: ["valueObject.RealTime.AudioCommand", "applicationService.RealTime.AudioRenderer", "valueObject.Testing.DemoScene"]},
		{field: "rejection_variants_exercised", op: "eq", value: 11, repairResources: ["aggregate.Control.AppState", "valueObject.Testing.DemoScene", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "global_parameter_cases_exercised", op: "eq", value: 7, repairResources: ["valueObject.Mixer.GlobalParameters", "valueObject.Testing.DemoScene", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "envelope_parameter_cases_exercised", op: "gt", value: 0, repairResources: ["valueObject.Synth.VoiceEnvelope", "valueObject.Testing.DemoScene", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "braids_scalar_cases_exercised", op: "gt", value: 0, repairResources: ["adapter.BraidsCapability", "valueObject.Testing.DemoScene", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "all_patch_parameter_cases_exercised", op: "eq", value: true, repairResources: ["valueObject.Mixer.ChannelParameters", "valueObject.Testing.DemoScene", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "all_serialized_properties_observed", op: "eq", value: true, repairResources: ["valueObject.Control.EventRecord", "valueObject.Control.EventLog", "valueObject.Control.StateTree", "valueObject.Control.TextProjection", "valueObject.RealTime.ParameterSnapshot", "domainService.Control.StateProjector"]},
		{field: "accepted_events", op: "gt", value: 0, repairResources: ["valueObject.Control.EventRecord", "valueObject.Control.EventLog", "applicationService.Control.AppLoop"]},
		{field: "rejected_events", op: "gt", value: 0, repairResources: ["aggregate.Control.AppState", "valueObject.Control.EventRecord", "valueObject.Control.EventLog", "applicationService.Control.AppLoop"]},
		{field: "scene_checkpoints", op: "gt", value: 10, repairResources: ["valueObject.Testing.DemoScene", "valueObject.Testing.DemoSceneReport", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "coverage_missing", op: "eq", value: 0, repairResources: ["valueObject.Control.EventLog", "valueObject.Testing.DemoScene", "valueObject.Testing.DemoSceneReport", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "state_hash_chain_valid", op: "eq", value: true, repairResources: ["valueObject.Control.EventRecord", "valueObject.Control.EventLog", "applicationService.Control.AppLoop", "domainService.Control.StateProjector"]},
		{field: "generation_chain_valid", op: "eq", value: true, repairResources: ["aggregate.Control.AppState", "valueObject.Control.EventRecord", "valueObject.Control.EventLog", "applicationService.Control.AppLoop"]},
		{field: "final_state_tree_matches", op: "eq", value: true, repairResources: ["valueObject.Control.StateTree", "domainService.Control.StateProjector", "valueObject.Testing.DemoSceneReport"]},
		{field: "gui_projection_matches_state", op: "eq", value: true, repairResources: ["valueObject.Control.TextProjection", "domainService.Control.StateProjector", "adapter.EframeTextWindow"]},
		{field: "parameter_projection_matches_state", op: "eq", value: true, repairResources: ["valueObject.RealTime.ParameterSnapshot", "domainService.Control.StateProjector"]},
		{field: "all_audio_parameter_effects_observed", op: "eq", value: true, repairResources: ["valueObject.Mixer.ChannelParameters", "valueObject.Mixer.GlobalParameters", "adapter.GlobalReverbDelay", "domainService.Mixer.MixEngine", "applicationService.RealTime.AudioRenderer", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "mixed_engine_stems_nonzero", op: "eq", value: true, repairResources: ["adapter.HiDefSoundFontPreparer", "adapter.BraidsPreparer", "aggregate.RealTime.PreparedEngineRack", "applicationService.RealTime.AudioRenderer"]},
		{field: "mixed_engine_parameter_isolation", op: "eq", value: true, repairResources: ["domainService.Control.StateProjector", "aggregate.RealTime.PreparedEngineRack", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "post_rejection_event_accepted", op: "eq", value: true, repairResources: ["aggregate.Control.AppState", "applicationService.Control.AppLoop", "applicationService.Testing.ExhaustiveGuiDemo", "applicationService.Shell.StandaloneApplication"]},
		{field: "schema_surface_equal", op: "eq", value: true, repairResources: ["valueObject.Control.TopLevelContext", "valueObject.Control.InteractionState", "valueObject.Control.AppEvent", "valueObject.Control.EventRecord", "valueObject.Control.EventLog", "valueObject.Control.StateTree", "valueObject.Control.PatchPageProjection", "valueObject.Control.TextProjection", "domainService.Control.StateProjector", "valueObject.Testing.DemoScene"]},
		{field: "unexpected_coverage", op: "eq", value: 0, repairResources: ["valueObject.Control.EventLog", "valueObject.Testing.DemoScene", "valueObject.Testing.DemoSceneReport", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "exact_state_values", op: "eq", value: true, repairResources: ["aggregate.Control.AppState", "valueObject.Control.StateTree", "domainService.Control.StateProjector", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "exact_projection_values", op: "eq", value: true, repairResources: ["valueObject.Control.TextProjection", "valueObject.RealTime.ParameterSnapshot", "domainService.Control.StateProjector", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "baseline_restored", op: "eq", value: true, repairResources: ["aggregate.Control.AppState", "valueObject.Testing.DemoSceneReport", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "reverb_input_nonzero", op: "eq", value: true, repairResources: ["valueObject.Mixer.ChannelParameters", "domainService.Mixer.MixEngine", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "delay_input_nonzero", op: "eq", value: true, repairResources: ["valueObject.Mixer.ChannelParameters", "domainService.Mixer.MixEngine", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "causal_audio_comparisons", op: "eq", value: true, repairResources: ["valueObject.Mixer.GlobalParameters", "adapter.GlobalReverbDelay", "domainService.Mixer.MixEngine", "applicationService.RealTime.AudioRenderer", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "faithful_effect_path", op: "eq", value: true, repairResources: ["adapter.GlobalReverbDelay", "domainService.Mixer.MixEngine", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "descriptors_unique", op: "eq", value: true, repairResources: ["valueObject.Shell.WindowInput", "valueObject.Control.AppEvent", "valueObject.Kernel.MidiMessage", "valueObject.Mixer.ChannelParameters", "valueObject.Mixer.GlobalParameters", "valueObject.Testing.DemoScene"]},
		{field: "event_record_payloads_exact", op: "eq", value: true, repairResources: ["valueObject.Control.EventRecord", "applicationService.Control.AppLoop", "valueObject.Testing.DemoScene", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "all_parameter_boundaries_exercised", op: "eq", value: true, repairResources: ["aggregate.Control.AppState", "valueObject.Mixer.ChannelParameters", "valueObject.Mixer.GlobalParameters", "valueObject.Testing.DemoScene", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "selection_clamps_exact", op: "eq", value: true, repairResources: ["aggregate.Control.AppState", "domainService.Control.StateProjector", "valueObject.Testing.DemoScene"]},
		{field: "tick_events_exact", op: "eq", value: true, repairResources: ["applicationService.Testing.AutomaticMidiTest", "applicationService.Control.AppLoop", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "two_run_trace_equal", op: "eq", value: true, repairResources: ["valueObject.Control.EventLog", "valueObject.Control.StateTree", "valueObject.Testing.DemoSceneReport", "applicationService.Testing.ExhaustiveGuiDemo"]},
	]
}

project: witnesses: dropped_adjustment_mutant: {
	scope: "goal"
	goal: "goal.observe_synth"
	capability: "capability.observable_demo_scene"
	resources: [
		"aggregate.Control.AppState",
		"valueObject.Control.EventRecord",
		"applicationService.Shell.KeyboardInputTranslator",
		"applicationService.Control.AppLoop",
		"domainService.Control.StateProjector",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	repairResources: [
		"aggregate.Control.AppState",
		"valueObject.Control.EventRecord",
		"applicationService.Shell.KeyboardInputTranslator",
		"applicationService.Control.AppLoop",
		"domainService.Control.StateProjector",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	counterexampleRepairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]
	evidence: ["evidence.mutation_resistance"]
	command: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "dropped-adjustment", "--mutant", "none"]
	negativeCommand: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "dropped-adjustment", "--mutant", "dropped-adjustment"]
	negativeExpectedExitCode: 1
	timeout: "60s"
	artifacts: ["target/debug/crest-synth-witness"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_MUTATION_OBSERVATION "
		schema: {
			case: "string"
			adjustment_dispatched: "bool"
			adjust_event_recorded: "bool"
			selected_value_exact: "bool"
			unrelated_values_unchanged: "bool"
			projection_values_exact: "bool"
			baseline_restored: "bool"
		}
	}
	predicates: [
		{field: "case", op: "eq", value: "dropped-adjustment", repairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]},
		{field: "adjustment_dispatched", op: "eq", value: true, repairResources: ["applicationService.Shell.KeyboardInputTranslator", "applicationService.Control.AppLoop"]},
		{field: "adjust_event_recorded", op: "eq", value: true, repairResources: ["valueObject.Control.EventRecord", "applicationService.Control.AppLoop"]},
		{field: "selected_value_exact", op: "eq", value: true, repairResources: ["aggregate.Control.AppState"]},
		{field: "unrelated_values_unchanged", op: "eq", value: true, repairResources: ["aggregate.Control.AppState", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "projection_values_exact", op: "eq", value: true, repairResources: ["domainService.Control.StateProjector"]},
		{field: "baseline_restored", op: "eq", value: true, repairResources: ["applicationService.Testing.ExhaustiveGuiDemo", "applicationService.Testing.BehavioralMutationHarness"]},
	]
}

project: witnesses: cross_patch_parameter_leak_mutant: {
	scope: "goal"
	goal: "goal.observe_synth"
	capability: "capability.observable_demo_scene"
	resources: [
		"aggregate.Control.AppState",
		"applicationService.Control.AppLoop",
		"domainService.Control.StateProjector",
		"valueObject.Mixer.ChannelParameters",
		"valueObject.RealTime.ParameterSnapshot",
		"valueObject.RealTime.PatchAudioBlock",
		"adapter.LockFreeAudioBoundary",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	repairResources: [
		"aggregate.Control.AppState",
		"applicationService.Control.AppLoop",
		"domainService.Control.StateProjector",
		"valueObject.Mixer.ChannelParameters",
		"valueObject.RealTime.ParameterSnapshot",
		"valueObject.RealTime.PatchAudioBlock",
		"adapter.LockFreeAudioBoundary",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	counterexampleRepairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]
	evidence: ["evidence.mutation_resistance"]
	command: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "cross-patch-parameter-leak", "--mutant", "none"]
	negativeCommand: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "cross-patch-parameter-leak", "--mutant", "cross-patch-parameter-leak"]
	negativeExpectedExitCode: 1
	timeout: "60s"
	artifacts: ["target/debug/crest-synth-witness"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_MUTATION_OBSERVATION "
			schema: {
			case: "string"
			edited_patch_id: "number"
			comparison_patch_id: "number"
			patch_ids_distinct: "bool"
			parameter: "string"
			parameter_cases_exercised: "number"
			edited_value_before: "number"
			edited_value_after: "number"
			comparison_value_before: "number"
			comparison_value_after: "number"
			published_edited_value: "number"
			published_comparison_value: "number"
			edited_stem_energy_before: "number"
			edited_stem_energy_after: "number"
			comparison_stem_energy_before: "number"
			comparison_stem_energy_after: "number"
			edited_value_changed: "bool"
			comparison_value_unchanged: "bool"
			state_values_exact: "bool"
			published_values_exact: "bool"
			edited_patch_audio_changed: "bool"
			unedited_patch_audio_unchanged: "bool"
			all_channel_parameters_isolated: "bool"
			dry_path_isolated: "bool"
			reverb_path_isolated: "bool"
			delay_path_isolated: "bool"
			baseline_restored: "bool"
		}
	}
	predicates: [
		{field: "case", op: "eq", value: "cross-patch-parameter-leak", repairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]},
		{field: "edited_patch_id", op: "gt", value: 0, repairResources: ["aggregate.Control.AppState", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "comparison_patch_id", op: "gt", value: 0, repairResources: ["aggregate.Control.AppState", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "patch_ids_distinct", op: "eq", value: true, repairResources: ["aggregate.Control.AppState", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "parameter_cases_exercised", op: "eq", value: 4, repairResources: ["valueObject.Mixer.ChannelParameters", "applicationService.Testing.ExhaustiveGuiDemo", "applicationService.Testing.BehavioralMutationHarness"]},
		{field: "edited_stem_energy_before", op: "gt", value: 0, repairResources: ["valueObject.RealTime.PatchAudioBlock", "applicationService.RealTime.AudioRenderer", "domainService.Mixer.MixEngine"]},
		{field: "comparison_stem_energy_before", op: "gt", value: 0, repairResources: ["valueObject.RealTime.PatchAudioBlock", "applicationService.RealTime.AudioRenderer", "domainService.Mixer.MixEngine"]},
		{field: "edited_value_changed", op: "eq", value: true, repairResources: ["aggregate.Control.AppState", "applicationService.Control.AppLoop"]},
		{field: "comparison_value_unchanged", op: "eq", value: true, repairResources: ["aggregate.Control.AppState"]},
		{field: "state_values_exact", op: "eq", value: true, repairResources: ["aggregate.Control.AppState", "domainService.Control.StateProjector"]},
		{field: "published_values_exact", op: "eq", value: true, repairResources: ["domainService.Control.StateProjector", "valueObject.RealTime.ParameterSnapshot", "adapter.LockFreeAudioBoundary"]},
		{field: "edited_patch_audio_changed", op: "eq", value: true, repairResources: ["applicationService.RealTime.AudioRenderer", "domainService.Mixer.MixEngine"]},
		{field: "unedited_patch_audio_unchanged", op: "eq", value: true, repairResources: ["valueObject.RealTime.ParameterSnapshot", "applicationService.RealTime.AudioRenderer", "domainService.Mixer.MixEngine"]},
		{field: "all_channel_parameters_isolated", op: "eq", value: true, repairResources: ["valueObject.Mixer.ChannelParameters", "valueObject.RealTime.ParameterSnapshot", "domainService.Mixer.MixEngine", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "dry_path_isolated", op: "eq", value: true, repairResources: ["domainService.Mixer.MixEngine"]},
		{field: "reverb_path_isolated", op: "eq", value: true, repairResources: ["domainService.Mixer.MixEngine"]},
		{field: "delay_path_isolated", op: "eq", value: true, repairResources: ["domainService.Mixer.MixEngine"]},
		{field: "baseline_restored", op: "eq", value: true, repairResources: ["applicationService.Testing.ExhaustiveGuiDemo", "applicationService.Testing.BehavioralMutationHarness"]},
	]
}

project: witnesses: patch_misroute_mutant: {
	scope: "goal"
	goal: "goal.observe_synth"
	capability: "capability.observable_demo_scene"
	resources: [
		"applicationService.Control.AppLoop",
		"valueObject.RealTime.AudioCommand",
		"adapter.LockFreeAudioBoundary",
		"applicationService.RealTime.AudioRenderer",
		"valueObject.RealTime.PatchAudioBlock",
		"port.Synth.PreparedInstrument",
		"aggregate.RealTime.PreparedEngineRack",
		"adapter.HiDefSoundFontPreparer",
		"domainService.Mixer.MixEngine",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	repairResources: [
		"applicationService.Control.AppLoop",
		"valueObject.RealTime.AudioCommand",
		"adapter.LockFreeAudioBoundary",
		"applicationService.RealTime.AudioRenderer",
		"valueObject.RealTime.PatchAudioBlock",
		"port.Synth.PreparedInstrument",
		"aggregate.RealTime.PreparedEngineRack",
		"adapter.HiDefSoundFontPreparer",
		"domainService.Mixer.MixEngine",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	counterexampleRepairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]
	evidence: ["evidence.mutation_resistance"]
	command: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "patch-misroute", "--mutant", "none"]
	negativeCommand: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "patch-misroute", "--mutant", "patch-misroute"]
	negativeExpectedExitCode: 1
	timeout: "60s"
	artifacts: ["target/debug/crest-synth-witness"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_MUTATION_OBSERVATION "
		schema: {
			case: "string"
			command_patch_matches_event: "bool"
			target_patch_received_command: "bool"
			target_stem_changed: "bool"
			untargeted_stems_unchanged: "bool"
			patch_routing_exact: "bool"
		}
	}
	predicates: [
		{field: "case", op: "eq", value: "patch-misroute", repairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]},
		{field: "command_patch_matches_event", op: "eq", value: true, repairResources: ["applicationService.Control.AppLoop", "valueObject.RealTime.AudioCommand", "adapter.LockFreeAudioBoundary"]},
		{field: "target_patch_received_command", op: "eq", value: true, repairResources: ["adapter.LockFreeAudioBoundary", "applicationService.RealTime.AudioRenderer", "aggregate.RealTime.PreparedEngineRack", "port.Synth.PreparedInstrument", "adapter.HiDefSoundFontPreparer"]},
		{field: "target_stem_changed", op: "eq", value: true, repairResources: ["applicationService.RealTime.AudioRenderer", "valueObject.RealTime.PatchAudioBlock", "adapter.HiDefSoundFontPreparer", "domainService.Mixer.MixEngine"]},
		{field: "untargeted_stems_unchanged", op: "eq", value: true, repairResources: ["applicationService.RealTime.AudioRenderer", "valueObject.RealTime.PatchAudioBlock", "adapter.HiDefSoundFontPreparer", "domainService.Mixer.MixEngine"]},
		{field: "patch_routing_exact", op: "eq", value: true, repairResources: ["applicationService.Control.AppLoop", "valueObject.RealTime.AudioCommand", "adapter.LockFreeAudioBoundary", "applicationService.RealTime.AudioRenderer", "domainService.Mixer.MixEngine", "applicationService.Testing.ExhaustiveGuiDemo"]},
	]
}

project: witnesses: omitted_state_tree_leaf_mutant: {
	scope: "goal"
	goal: "goal.observe_synth"
	capability: "capability.observable_demo_scene"
	resources: [
		"valueObject.Control.StateTree",
		"domainService.Control.StateProjector",
		"valueObject.Testing.DemoScene",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	repairResources: [
		"valueObject.Control.StateTree",
		"domainService.Control.StateProjector",
		"valueObject.Testing.DemoScene",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	counterexampleRepairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]
	evidence: ["evidence.mutation_resistance"]
	command: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "omitted-state-tree-leaf", "--mutant", "none"]
	negativeCommand: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "omitted-state-tree-leaf", "--mutant", "omitted-state-tree-leaf"]
	negativeExpectedExitCode: 1
	timeout: "60s"
	artifacts: ["target/debug/crest-synth-witness"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_MUTATION_OBSERVATION "
		schema: {
			case: "string"
			schema_surface_equal: "bool"
			required_leaf_count: "number"
			missing_leaf_count: "number"
			unexpected_leaf_count: "number"
			state_values_exact: "bool"
			projection_values_exact: "bool"
		}
	}
	predicates: [
		{field: "case", op: "eq", value: "omitted-state-tree-leaf", repairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]},
		{field: "schema_surface_equal", op: "eq", value: true, repairResources: ["valueObject.Control.StateTree", "domainService.Control.StateProjector", "valueObject.Testing.DemoScene", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "required_leaf_count", op: "gt", value: 0, repairResources: ["valueObject.Control.StateTree", "valueObject.Testing.DemoScene"]},
		{field: "missing_leaf_count", op: "eq", value: 0, repairResources: ["valueObject.Control.StateTree", "domainService.Control.StateProjector", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "unexpected_leaf_count", op: "eq", value: 0, repairResources: ["valueObject.Control.StateTree", "domainService.Control.StateProjector", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "state_values_exact", op: "eq", value: true, repairResources: ["domainService.Control.StateProjector", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "projection_values_exact", op: "eq", value: true, repairResources: ["domainService.Control.StateProjector", "applicationService.Testing.ExhaustiveGuiDemo"]},
	]
}

project: witnesses: dry_to_wet_bypass_mutant: {
	scope: "goal"
	goal: "goal.observe_synth"
	capability: "capability.observable_demo_scene"
	resources: [
		"valueObject.Mixer.ChannelParameters",
		"valueObject.Mixer.GlobalParameters",
		"port.Mixer.GlobalEffectsProcessor",
		"adapter.GlobalReverbDelay",
		"domainService.Mixer.MixEngine",
		"applicationService.RealTime.AudioRenderer",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	repairResources: [
		"valueObject.Mixer.ChannelParameters",
		"valueObject.Mixer.GlobalParameters",
		"adapter.GlobalReverbDelay",
		"domainService.Mixer.MixEngine",
		"applicationService.RealTime.AudioRenderer",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	counterexampleRepairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]
	evidence: ["evidence.mutation_resistance"]
	command: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "dry-to-wet-bypass", "--mutant", "none"]
	negativeCommand: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "dry-to-wet-bypass", "--mutant", "dry-to-wet-bypass"]
	negativeExpectedExitCode: 1
	timeout: "60s"
	artifacts: ["target/debug/crest-synth-witness"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_MUTATION_OBSERVATION "
		schema: {
			case: "string"
			dry_input_energy: "number"
			zero_send_reverb_input_energy: "number"
			zero_send_delay_input_energy: "number"
			zero_send_wet_output_energy: "number"
			nonzero_send_reverb_input_energy: "number"
			nonzero_send_delay_input_energy: "number"
			nonzero_send_wet_output_energy: "number"
			identical_effect_state: "bool"
			dry_bypass_absent: "bool"
			finite_audio: "bool"
			baseline_restored: "bool"
		}
	}
	predicates: [
		{field: "case", op: "eq", value: "dry-to-wet-bypass", repairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]},
		{field: "dry_input_energy", op: "gt", value: 0, repairResources: ["applicationService.RealTime.AudioRenderer", "domainService.Mixer.MixEngine", "applicationService.Testing.BehavioralMutationHarness"]},
		{field: "zero_send_reverb_input_energy", op: "eq", value: 0, repairResources: ["valueObject.Mixer.ChannelParameters", "domainService.Mixer.MixEngine", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "zero_send_delay_input_energy", op: "eq", value: 0, repairResources: ["valueObject.Mixer.ChannelParameters", "domainService.Mixer.MixEngine", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "zero_send_wet_output_energy", op: "eq", value: 0, repairResources: ["adapter.GlobalReverbDelay", "domainService.Mixer.MixEngine", "applicationService.Testing.BehavioralMutationHarness"]},
		{field: "nonzero_send_reverb_input_energy", op: "gt", value: 0, repairResources: ["valueObject.Mixer.ChannelParameters", "domainService.Mixer.MixEngine", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "nonzero_send_delay_input_energy", op: "gt", value: 0, repairResources: ["valueObject.Mixer.ChannelParameters", "domainService.Mixer.MixEngine", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "nonzero_send_wet_output_energy", op: "gt", value: 0, repairResources: ["valueObject.Mixer.GlobalParameters", "adapter.GlobalReverbDelay", "domainService.Mixer.MixEngine"]},
		{field: "identical_effect_state", op: "eq", value: true, repairResources: ["adapter.GlobalReverbDelay", "applicationService.Testing.BehavioralMutationHarness"]},
		{field: "dry_bypass_absent", op: "eq", value: true, repairResources: ["adapter.GlobalReverbDelay", "domainService.Mixer.MixEngine", "applicationService.Testing.BehavioralMutationHarness"]},
		{field: "finite_audio", op: "eq", value: true, repairResources: ["applicationService.RealTime.AudioRenderer", "domainService.Mixer.MixEngine"]},
		{field: "baseline_restored", op: "eq", value: true, repairResources: ["applicationService.Testing.ExhaustiveGuiDemo", "applicationService.Testing.BehavioralMutationHarness"]},
	]
}

project: witnesses: zero_renderer_mutant: {
	scope: "goal"
	goal: "goal.observe_synth"
	capability: "capability.observable_demo_scene"
	resources: [
		"applicationService.Control.AppLoop",
		"adapter.LockFreeAudioBoundary",
		"applicationService.RealTime.AudioRenderer",
		"valueObject.RealTime.PatchAudioBlock",
		"domainService.Mixer.MixEngine",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	repairResources: [
		"applicationService.Control.AppLoop",
		"applicationService.RealTime.AudioRenderer",
		"valueObject.RealTime.PatchAudioBlock",
		"domainService.Mixer.MixEngine",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.BehavioralMutationHarness",
		"asset.BehavioralWitnessMain",
	]
	counterexampleRepairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]
	evidence: ["evidence.mutation_resistance"]
	command: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "zero-renderer", "--mutant", "none"]
	negativeCommand: ["cargo", "run", "--quiet", "--bin", "crest-synth-witness", "--", "--case", "zero-renderer", "--mutant", "zero-renderer"]
	negativeExpectedExitCode: 1
	timeout: "60s"
	artifacts: ["target/debug/crest-synth-witness"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_MUTATION_OBSERVATION "
		schema: {
			case: "string"
			control_trace_complete: "bool"
			renderer_called: "bool"
			renderer_nonzero: "bool"
			render_peak: "number"
			finite_audio: "bool"
		}
	}
	predicates: [
		{field: "case", op: "eq", value: "zero-renderer", repairResources: ["applicationService.Testing.BehavioralMutationHarness", "asset.BehavioralWitnessMain"]},
		{field: "control_trace_complete", op: "eq", value: true, repairResources: ["applicationService.Control.AppLoop", "applicationService.Testing.ExhaustiveGuiDemo"]},
		{field: "renderer_called", op: "eq", value: true, repairResources: ["applicationService.RealTime.AudioRenderer", "applicationService.Testing.BehavioralMutationHarness"]},
		{field: "renderer_nonzero", op: "eq", value: true, repairResources: ["applicationService.RealTime.AudioRenderer", "valueObject.RealTime.PatchAudioBlock", "domainService.Mixer.MixEngine"]},
		{field: "render_peak", op: "gt", value: 0.001, repairResources: ["applicationService.RealTime.AudioRenderer", "valueObject.RealTime.PatchAudioBlock", "domainService.Mixer.MixEngine"]},
		{field: "finite_audio", op: "eq", value: true, repairResources: ["applicationService.RealTime.AudioRenderer", "domainService.Mixer.MixEngine"]},
	]
}

project: witnesses: braids_engine: {
	scope: "goal"
	goal: "goal.play_test_song"
	capability: "capability.braids_engine"
	resources: [
		"adapter.BraidsCapability",
		"adapter.BraidsPreparer",
		"port.Synth.PreparedInstrument",
		"aggregate.RealTime.PreparedEngineRack",
		"applicationService.RealTime.AudioRenderer",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"asset.BehavioralAcceptanceTests",
	]
	repairResources: [
		"adapter.BraidsCapability",
		"adapter.BraidsPreparer",
		"aggregate.RealTime.PreparedEngineRack",
		"applicationService.RealTime.AudioRenderer",
		"applicationService.Testing.ExhaustiveGuiDemo",
	]
	evidence: ["evidence.braids_engine_contract"]
	command: ["cargo", "test", "--release", "--test", "braids_engine", "--", "--nocapture"]
	timeout: "180s"
	observation: {
		kind: "json_stdout"
		marker: "CREST_BRAIDS_OBSERVATION "
		schema: {
			upstream_revision: "string"
			stmlib_revision: "string"
			source_hashes_match: "bool"
			model_count: "number"
			voices_per_patch: "number"
			braids_patch_count: "number"
			total_braids_voice_capacity: "number"
			capacity_matches_patch_count: "bool"
			no_braids_specific_patch_limit: "bool"
			independent_patch_banks: "bool"
			sixteen_voices_audible: "bool"
			seventeenth_stole_oldest: "bool"
			scalar_cases_exercised: "number"
			unsupported_rate_rejected: "bool"
			mixed_routing_exact: "bool"
			parameter_isolation_exact: "bool"
			finite_audio: "bool"
			callback_allocations: "number"
			callback_destructions: "number"
			native_callback_destructions: "number"
			p99_render_microseconds: "number"
		}
	}
	predicates: [
		{field: "upstream_revision", op: "eq", value: "08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4"},
		{field: "stmlib_revision", op: "eq", value: "e3bd7c9cc00e4364166f9905c0509b6ffd0535ec"},
		{field: "source_hashes_match", op: "eq", value: true},
		{field: "model_count", op: "eq", value: 47},
		{field: "voices_per_patch", op: "eq", value: 16},
		{field: "braids_patch_count", op: "eq", value: 3},
		{field: "total_braids_voice_capacity", op: "eq", value: 48},
		{field: "capacity_matches_patch_count", op: "eq", value: true},
		{field: "no_braids_specific_patch_limit", op: "eq", value: true},
		{field: "independent_patch_banks", op: "eq", value: true},
		{field: "sixteen_voices_audible", op: "eq", value: true},
		{field: "seventeenth_stole_oldest", op: "eq", value: true},
		{field: "scalar_cases_exercised", op: "eq", value: 3},
		{field: "unsupported_rate_rejected", op: "eq", value: true},
		{field: "mixed_routing_exact", op: "eq", value: true},
		{field: "parameter_isolation_exact", op: "eq", value: true},
		{field: "finite_audio", op: "eq", value: true},
		{field: "callback_allocations", op: "eq", value: 0},
		{field: "callback_destructions", op: "eq", value: 0},
		{field: "native_callback_destructions", op: "eq", value: 0},
		{field: "p99_render_microseconds", op: "lt", value: 2666},
	]
}

project: witnesses: per_voice_envelope: {
	scope: "goal"
	goal: "goal.control_synth"
	capability: "capability.per_voice_envelope"
	resources: [
		"valueObject.Synth.VoiceEnvelope",
		"aggregate.Synth.Patch",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"adapter.HiDefSoundFontPreparer",
		"adapter.BraidsPreparer",
		"applicationService.RealTime.AudioRenderer",
		"asset.BehavioralAcceptanceTests",
	]
	repairResources: [
		"valueObject.Synth.VoiceEnvelope",
		"aggregate.Synth.Patch",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"adapter.HiDefSoundFontPreparer",
		"adapter.BraidsPreparer",
		"applicationService.RealTime.AudioRenderer",
	]
	evidence: ["evidence.per_voice_envelope_contract"]
	command: ["cargo", "test", "--test", "per_voice_envelope", "--", "--nocapture"]
	timeout: "180s"
	observation: {
		kind: "json_stdout"
		marker: "CREST_ENVELOPE_OBSERVATION "
		schema: {
			parameter_cases_exercised: "number"
			soundfont_synthesizers_per_patch: "number"
			braids_voices_per_patch: "number"
			state_text_snapshot_exact: "bool"
			soundfont_overlap_independent: "bool"
			braids_overlap_independent: "bool"
			all_fields_audible: "bool"
			post_stem_envelope_absent: "bool"
			extremes_finite: "bool"
			callback_allocations: "number"
			callback_destructions: "number"
		}
	}
	predicates: [
		{field: "parameter_cases_exercised", op: "eq", value: 4},
		{field: "soundfont_synthesizers_per_patch", op: "eq", value: 1},
		{field: "braids_voices_per_patch", op: "eq", value: 16},
		{field: "state_text_snapshot_exact", op: "eq", value: true},
		{field: "soundfont_overlap_independent", op: "eq", value: true},
		{field: "braids_overlap_independent", op: "eq", value: true},
		{field: "all_fields_audible", op: "eq", value: true},
		{field: "post_stem_envelope_absent", op: "eq", value: true},
		{field: "extremes_finite", op: "eq", value: true},
		{field: "callback_allocations", op: "eq", value: 0},
		{field: "callback_destructions", op: "eq", value: 0},
	]
}
