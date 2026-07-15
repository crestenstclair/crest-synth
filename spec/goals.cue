package crestsynth

// Product intent is deliberately separate from the DDD model.
//
// Goals and capabilities describe observable vertical slices: what a musician
// or maintainer can do and what evidence proves it. The context files retain
// ownership of domain state, contracts, invariants, and implementation
// boundaries. A contribution edge connects those two views without turning a
// capability into a second aggregate or making it a dependency-graph node.
project: {
	mission: "A standalone, controller-first MIDI synthesizer for Steam Deck and desktop. A musician performs from external MIDI while crest-synth renders a stable stereo signal and exposes all sixteen mixer tracks in a dense, terminal-like overview operable from keyboard or gamepad. The same one-way application loop must support deterministic scenes and instrument-partitioned MIDI-file test playback so humans and coding agents can inspect what happened, hear the result, and mechanically falsify broken implementations."

	actors: {
		musician: {
			description: "a musician who performs from external MIDI and adjusts the live mixer without relying on a pointer"
		}
		maintainer: {
			description: "a developer or coding agent regenerating crest-synth and evaluating the resulting system through executable proofs"
		}
	}

	goals: {
		perform_through_standalone: {
			description: "A musician can send MIDI performance events into the standalone application and hear audible, bounded stereo output without violating the audio callback deadline"
			priority: "required"
			actors: ["actor.musician"]
			capabilities: [
				"capability.external_midi_performance",
				"capability.polyphonic_sound_generation",
				"capability.stereo_mix_pipeline",
				"capability.realtime_safe_execution",
			]
			requirements: [
				"requirement.external_midi_is_performance_input",
				"requirement.hard_realtime_callback",
				"requirement.canonical_signal_flow",
			]
		}
		operate_live_mixer: {
			description: "A musician can see all sixteen mixer tracks at once, navigate their compact terminal-style controls using keyboard or gamepad, inspect the selected track, and hear accepted edits affect the live audio path"
			priority: "required"
			actors: ["actor.musician"]
			dependsOn: ["goal.perform_through_standalone"]
			capabilities: [
				"capability.pointer_free_mixer_control",
				"capability.shared_control_reducer",
				"capability.stereo_mix_pipeline",
			]
			requirements: [
				"requirement.mixer_only_ui_scope",
				"requirement.single_control_mutation_path",
				"requirement.gamepad_keyboard_parity",
				"requirement.terminal_mixer_presentation",
			]
		}
		exercise_supported_sound_architecture: {
			description: "Every currently supported synthesis subsystem is executable as a coherent path: voices, samples, modulation, patches, effects, buses, and MIDI-file input produce measured non-theatrical results"
			priority: "required"
			actors: ["actor.maintainer"]
			capabilities: [
				"capability.polyphonic_sound_generation",
				"capability.configurable_instrument_graph",
				"capability.stereo_mix_pipeline",
				"capability.behavioral_proof_harness",
				"capability.instrument_partitioned_test_playback",
			]
			requirements: [
				"requirement.current_sound_scope",
				"requirement.measured_proofs",
				"requirement.canonical_resource_types",
				"requirement.deterministic_instrument_assignment",
			]
		}
		preserve_reproducible_sound_state: {
			description: "A complete patch or session can be versioned, restored, and rendered equivalently, while malformed or unsupported data leaves the active state untouched"
			priority: "required"
			actors: ["actor.musician", "actor.maintainer"]
			dependsOn: ["goal.exercise_supported_sound_architecture"]
			capabilities: [
				"capability.configurable_instrument_graph",
				"capability.versioned_sound_state",
			]
			requirements: ["requirement.versioned_atomic_restore"]
		}
		inspect_and_replay_behavior: {
			description: "A maintainer can run a serialized scenario through the same reducer and render path used by the application, inspect deterministic state and audio observations, and distinguish real behavior from a no-op implementation"
			priority: "required"
			actors: ["actor.maintainer"]
			dependsOn: ["goal.perform_through_standalone", "goal.operate_live_mixer"]
			capabilities: [
				"capability.shared_control_reducer",
				"capability.deterministic_scene_replay",
				"capability.behavioral_proof_harness",
			]
			requirements: [
				"requirement.single_control_mutation_path",
				"requirement.deterministic_observation",
				"requirement.measured_proofs",
			]
		}
	}

	capabilities: {
		external_midi_performance: {
			description: "Normalize raw MIDI, preserve channel and note identity, route intentional layers exactly once, and deliver the resulting events to the standalone render graph"
			goals: ["goal.perform_through_standalone"]
			acceptance: routed_note_to_audio: {
				description: "A note entering through the MIDI input contract reaches only matching patches and produces device-bound audio through the same path used by the standalone application"
				actor: "actor.musician"
				steps: [
					{action: "deliver note-on, expression, and note-off bytes on a configured MIDI address", observes: "the bytes normalize to stable addressed MidiEvents and every intentionally layered matching patch receives one copy"},
					{action: "render the resulting voices through the standalone graph", observes: "the measured stereo peak is audible and bounded and an unmapped patch remains unchanged"},
				]
				evidence: ["evidence.standalone_runtime"]
			}
		}
		polyphonic_sound_generation: {
			description: "Render virtual-analog voices with oscillator, filter, ADSR envelopes, bounded polyphony, configured voice stealing, and per-note expression; exercise sample playback through its dedicated subsystem proof"
			goals: ["goal.perform_through_standalone", "goal.exercise_supported_sound_architecture"]
			acceptance: audible_over_polyphony: {
				description: "An overlapping passage beyond the configured voice limit remains audible and applies the selected stealing and expression behavior"
				actor: "actor.maintainer"
				steps: [
					{action: "hold more notes than the configured polyphony limit", observes: "active voice count remains bounded and a measurable steal occurs according to policy"},
					{action: "apply expression to one NoteId and render", observes: "only the matching voice changes and output remains non-silent and unclipped"},
				]
				evidence: ["evidence.polyphonic_render"]
			}
		}
		stereo_mix_pipeline: {
			description: "Route independent patch signals through strip inserts, volume and pan, send taps, aux returns, master processing, limiting, and per-strip metering"
			goals: ["goal.perform_through_standalone", "goal.operate_live_mixer", "goal.exercise_supported_sound_architecture"]
			acceptance: independent_strips_to_master: {
				description: "Multiple addressed signals remain independently controllable while following the canonical strip-to-master signal path"
				actor: "actor.musician"
				steps: [
					{action: "render non-zero signals on multiple strips and solo one", observes: "only the soloed strip is audible while every strip continues to meter its own pre-solo signal"},
					{action: "process inserts, sends, aux returns, and master output", observes: "processors run in declared order and the final stereo signal remains bounded"},
				]
				evidence: ["evidence.mixer_behavior"]
			}
		}
		realtime_safe_execution: {
			description: "Move discrete events, latest parameter snapshots, and retired memory across the audio boundary without locks, blocking, I/O, callback allocation, or callback deallocation"
			goals: ["goal.perform_through_standalone"]
			acceptance: lock_free_boundary: {
				description: "The real-time seam delivers events, exposes the newest snapshot, and reclaims retired state away from the simulated audio thread"
				actor: "actor.maintainer"
				steps: [
					{action: "push an event and publish two distinct snapshots", observes: "the consumer receives the event and reads only the newest complete snapshot without blocking"},
					{action: "retire tracked state from the audio side and collect it", observes: "destruction occurs on the non-audio side"},
				]
				evidence: ["evidence.realtime_boundary"]
			}
		}
		pointer_free_mixer_control: {
			description: "Render all sixteen mixer tracks in one dense terminal-style grid and edit volume, two sends, pan, mute, and solo using the shared keyboard/gamepad event vocabulary"
			goals: ["goal.operate_live_mixer"]
			acceptance: navigate_and_edit_mixer: {
				description: "Keyboard and gamepad actions drive the same reducer semantics across the full sixteen-track grid, edit mode, toggles, the selected-track inspector, and bounded values"
				actor: "actor.musician"
				steps: [
					{action: "navigate from T00 through T0F and back", observes: "all sixteen narrow track columns remain visible while the cursor highlight, inspector, and status line follow the selected track"},
					{action: "hold edit and apply fine and coarse adjustments, then double-tap edit on mute and solo", observes: "only the focused channel and parameter change, continuous values clamp, and toggles never react to directional input"},
				]
				evidence: ["evidence.mixer_behavior", "evidence.standalone_runtime"]
			}
		}
		shared_control_reducer: {
			description: "Apply MIDI, gamepad, editor, mixer, patch, and preset events through one authoritative AppState reducer and project accepted state to the audio model"
			goals: ["goal.operate_live_mixer", "goal.inspect_and_replay_behavior"]
			acceptance: one_event_path: {
				description: "Live input and scene input produce identical state transitions through the same reducer"
				actor: "actor.maintainer"
				steps: [
					{action: "apply the same event sequence through the live-input facade and through SceneRunner", observes: "both paths produce byte-identical snapshots and identical rejection records"},
					{action: "apply an invalid event", observes: "the reducer returns a typed rejection and changes neither domain state nor event-sequence frame"},
				]
				evidence: ["evidence.scene_replay"]
			}
		}
		configurable_instrument_graph: {
			description: "Compose patches from canonical engine, sample, modulation, MIDI mapping, effects, and mixer resources without parallel substitute types"
			goals: ["goal.exercise_supported_sound_architecture", "goal.preserve_reproducible_sound_state"]
			acceptance: subsystem_vertical_slices: {
				description: "Each supported subsystem participates in a measured executable slice and all slices use the same canonical resource types"
				actor: "actor.maintainer"
				steps: [
					{action: "run the voice, sample, modulation, patch, effects, mixer, MIDI, and preset proof targets", observes: "each target exits successfully after asserting measured behavior rather than merely printing a success token"},
					{action: "inspect the compiled crate interfaces", observes: "a spec resource has one canonical public Rust type and consumers import it instead of redeclaring local substitutes"},
				]
				evidence: ["evidence.sound_subsystem_suite"]
			}
		}
		instrument_partitioned_test_playback: {
			description: "For MIDI-file demonstrations, partition note events into deterministic instrument parts, create one canonical Patch per discovered instrument, and assign those patches to the sixteen mixer tracks round-robin"
			goals: ["goal.exercise_supported_sound_architecture"]
			acceptance: instrument_parts_become_patches: {
				description: "A multi-instrument Standard MIDI File becomes independently metered patch parts without pretending the test player is a product sequencer"
				actor: "actor.maintainer"
				steps: [
					{action: "load a file containing multiple program/bank identities and percussion", observes: "events are grouped by stable instrument identity, each sounding note-off remains with its note-on part, and one Patch is created per discovered part"},
					{action: "order parts by first musical event and assign them", observes: "part N uses mixer track N modulo 16; more than sixteen parts share tracks deterministically without dropping patches"},
					{action: "load a file with missing instrument metadata", observes: "the reader falls back deterministically to source-track/channel identity and records that fallback in the part label"},
				]
				evidence: ["evidence.midi_instrument_partition"]
			}
		}
		versioned_sound_state: {
			description: "Encode complete patches and sessions with explicit versions, migrate supported versions, and replace active state atomically only after full validation"
			goals: ["goal.preserve_reproducible_sound_state"]
			acceptance: equivalent_atomic_roundtrip: {
				description: "A restored setup equals and re-renders the saved setup, and a failed restore cannot partially mutate the running application"
				actor: "actor.maintainer"
				steps: [
					{action: "encode and decode a complete multi-patch session", observes: "the restored patch, routing, mixer, tempo, and time-signature state is equal and a fixed passage renders bit-identically"},
					{action: "attempt to decode malformed and unsupported-version bytes", observes: "the operation fails and the previously active AppState snapshot remains byte-identical"},
				]
				evidence: ["evidence.sound_state_roundtrip"]
			}
		}
		deterministic_scene_replay: {
			description: "Serialize complete AppEvents, replay them in order through the production reducer and renderer, and emit deterministic snapshots plus measured render observations"
			goals: ["goal.inspect_and_replay_behavior"]
			acceptance: replay_inspect_compare: {
				description: "A scene is both a headless executable acceptance scenario and a paced live observation script"
				actor: "actor.maintainer"
				steps: [
					{action: "run a scene twice against the same initial state", observes: "event results, final snapshot bytes, rendered-block count, and measured peak are identical"},
					{action: "run the showcase scene in the window with MIDI-file playback", observes: "the same ordered mixer transitions are visible and audible with captions identifying each event"},
				]
				evidence: ["evidence.scene_replay"]
			}
		}
		behavioral_proof_harness: {
			description: "Provide hermetic demos, smokes, scenes, and falsification-gated observation modes that fail on silent, bypassed, lossy, or no-op implementations"
			goals: ["goal.exercise_supported_sound_architecture", "goal.inspect_and_replay_behavior"]
			acceptance: regenerate_then_prove: {
				description: "A blank regeneration can be accepted only after the whole crate and every supported vertical slice prove their behavior"
				actor: "actor.maintainer"
				steps: [
					{action: "run the complete proof target after regeneration", observes: "every proof asserts measured state or audio and exits non-zero for its explicit degenerate path"},
					{action: "execute each declared behavioral witness through crest-spec", observes: "the committed real implementation passes and the schema-equivalent degenerate case fails"},
				]
				evidence: ["evidence.sound_subsystem_suite", "evidence.scene_replay"]
			}
		}
	}

	requirements: {
		external_midi_is_performance_input: {
			kind: "functional"
			description: "The UI never originates notes; performance notes enter through external MIDI. MIDI-file and built-in note playback are demonstration and verification inputs only."
			goals: ["goal.perform_through_standalone"]
			capabilities: ["capability.external_midi_performance"]
		}
		hard_realtime_callback: {
			kind: "nonfunctional"
			description: "The audio callback allocates no heap memory, acquires no lock, performs no blocking I/O, and never destroys retired owned state."
			goals: ["goal.perform_through_standalone"]
			capabilities: ["capability.realtime_safe_execution"]
		}
		canonical_signal_flow: {
			kind: "functional"
			description: "Audio flows from the selected engine or sample source through strip inserts, volume and pan, send taps, aux returns, master inserts, limiter, and stereo output."
			goals: ["goal.perform_through_standalone"]
			capabilities: ["capability.stereo_mix_pipeline"]
		}
		mixer_only_ui_scope: {
			kind: "functional"
			description: "The current standalone GUI is the live mixer and no other screen: all sixteen compact track columns, a selected-track inspector, and terminal-like status/help rows; no view switching, patch/preset/modulation editor, or on-screen keyboard."
			goals: ["goal.operate_live_mixer"]
			capabilities: ["capability.pointer_free_mixer_control"]
		}
		terminal_mixer_presentation: {
			kind: "nonfunctional"
			description: "The initial view uses a monospaced, hard-edged, low-decoration grid: T00-T0F headers with patch/instrument labels, narrow textual or segmented level displays, compact pan/mute/solo cells, a right-side cursor/value/state inspector, and bottom status plus command-hint rows. It avoids skeuomorphic knobs, glossy faders, ornamental animation, and oversized graphical controls."
			goals: ["goal.operate_live_mixer"]
			capabilities: ["capability.pointer_free_mixer_control"]
		}
		single_control_mutation_path: {
			kind: "nonfunctional"
			description: "All live, automated, and scene inputs become AppEvents applied by the same AppState reducer; views render state and never maintain an independent mutable model."
			goals: ["goal.operate_live_mixer", "goal.inspect_and_replay_behavior"]
			capabilities: ["capability.shared_control_reducer", "capability.deterministic_scene_replay"]
		}
		gamepad_keyboard_parity: {
			kind: "functional"
			description: "Keyboard and gamepad adapters emit the same semantic mixer events for navigation, momentary edit mode, fine/coarse adjustment, and double-tap toggles."
			goals: ["goal.operate_live_mixer"]
			capabilities: ["capability.pointer_free_mixer_control"]
		}
		current_sound_scope: {
			kind: "functional"
			description: "The completion scope is the existing executable system: virtual-analog polyphony, zoned sample playback, modulation, patch routing, effects chains, mixing, MIDI files, presets, and sessions. Wavetable and FM discriminators may remain forward-compatible but are not claimed as rendered engines until dedicated resources and proofs exist."
			goals: ["goal.exercise_supported_sound_architecture"]
			capabilities: ["capability.polyphonic_sound_generation", "capability.configurable_instrument_graph"]
		}
		deterministic_instrument_assignment: {
			kind: "functional"
			description: "MIDI-file playback is test orchestration: identify parts by bank/program and percussion where possible, fall back to source-track/channel identity when metadata is absent, create one Patch per part, and assign first-seen parts to mixer tracks with index modulo 16."
			goals: ["goal.exercise_supported_sound_architecture"]
			capabilities: ["capability.instrument_partitioned_test_playback"]
		}
		versioned_atomic_restore: {
			kind: "functional"
			description: "Preset and session payloads include an explicit version; decoding and migration finish successfully before the active AppState is replaced."
			goals: ["goal.preserve_reproducible_sound_state"]
			capabilities: ["capability.versioned_sound_state"]
		}
		deterministic_observation: {
			kind: "nonfunctional"
			description: "Identical initial AppState plus identical scene bytes produces byte-identical snapshots and equal render observations; snapshots contain no wall-clock or unordered-map state."
			goals: ["goal.inspect_and_replay_behavior"]
			capabilities: ["capability.deterministic_scene_replay"]
		}
		measured_proofs: {
			kind: "nonfunctional"
			description: "A proof asserts measured state, routing, samples, or audio in code and exits non-zero when the claim is false; unconditional success text is never sufficient evidence."
			goals: ["goal.exercise_supported_sound_architecture", "goal.inspect_and_replay_behavior"]
			capabilities: ["capability.behavioral_proof_harness"]
		}
		canonical_resource_types: {
			kind: "nonfunctional"
			description: "Each spec resource owns one canonical public Rust type in its module. Other resources import that type; they never recreate sibling value objects, ports, aggregates, or local lookalikes."
			goals: ["goal.exercise_supported_sound_architecture"]
			capabilities: ["capability.configurable_instrument_graph"]
		}
	}

	evidence: {
		standalone_runtime: {
			kind: "behavioral_witness"
			description: "The hermetic standalone composition consumes normalized events through AppState and produces bounded, metered audio through the production render function."
			validations: ["validation.ui_smoke"]
			witnesses: ["witness.standalone_runtime"]
		}
		polyphonic_render: {
			kind: "behavioral_witness"
			description: "A committed over-polyphonic engine run proves audible output, voice stealing, and isolated per-note expression; its silent/no-expression baseline fails."
			witnesses: ["witness.expressive_polyphony"]
		}
		mixer_behavior: {
			kind: "behavioral_witness"
			description: "A headless mixer run proves all-track visibility, cursor/inspector navigation, bounded editing, solo isolation, ordered stereo mixing, and metering independence."
			validations: ["validation.mixer_integration", "validation.autopilot"]
			witnesses: ["witness.mixer_control_path"]
		}
		midi_instrument_partition: {
			kind: "behavioral_witness"
			description: "A real multi-track MIDI file is partitioned into instrument parts, materialized as one patch each, assigned round-robin to mixer tracks, and rendered with independent patch/track observations."
			validations: ["validation.midi_multitrack_regression"]
			witnesses: ["witness.midi_instrument_partition"]
		}
		realtime_boundary: {
			kind: "behavioral_witness"
			description: "Instrumented boundary execution proves event delivery, latest-wins snapshots, and off-audio-thread reclamation."
			validations: ["validation.realtime_contract"]
			witnesses: ["witness.realtime_boundary"]
		}
		sound_subsystem_suite: {
			kind: "project_validation"
			description: "The complete proof suite exercises the supported voice, sample, modulation, patch, effect, mixer, MIDI, and persistence slices."
			validations: ["validation.proof_suite"]
		}
		sound_state_roundtrip: {
			kind: "behavioral_witness"
			description: "A complete versioned session restores equivalently, re-renders identically, and rejects corrupt state atomically."
			validations: ["validation.preset_roundtrip"]
			witnesses: ["witness.preset_session_roundtrip"]
		}
		scene_replay: {
			kind: "behavioral_witness"
			description: "The same serialized events drive the authoritative reducer and renderer twice with deterministic snapshots and measured non-silent results; a no-op reducer fails."
			validations: ["validation.scene_suite"]
			witnesses: ["witness.scene_replay"]
		}
	}

	nonGoals: {
		plugin_hosting: "Plugin, VST, CLAP, AU, and third-party-effect hosting were explicitly removed; the generated Effects context is the current effects system."
		additional_editor_screens: "Patch, preset-browser, modulation-matrix, and MIDI-configuration screens are future work; the current standalone GUI is intentionally mixer-only."
		onscreen_instrument: "The UI is not a performance surface and does not provide an on-screen keyboard or originate notes."
		sequencer_product: "Standard MIDI File playback and built-in note schedules are proof/demo inputs, not a composition or sequencing feature."
		pointer_input: "Mouse and touch interaction are outside the current controller-first mixer scope."
		unproven_engine_modes: "Wavetable and FM may remain modeled identifiers, but project completion does not claim working renderers until each has explicit configuration, integration, and behavioral evidence."
	}

	completion: {
		requiredGoals: [
			"goal.perform_through_standalone",
			"goal.operate_live_mixer",
			"goal.exercise_supported_sound_architecture",
			"goal.preserve_reproducible_sound_state",
			"goal.inspect_and_replay_behavior",
		]
		projectChecks: [
			"validation.format",
			"validation.clippy",
			"validation.build",
			"validation.test",
			"validation.ui_smoke",
			"validation.scene_suite",
			"validation.proof_suite",
		]
	}
}
