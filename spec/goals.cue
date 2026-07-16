package crestsynth

// Product intent is deliberately separate from the DDD model.
//
// Goals and capabilities describe observable vertical slices: what a musician
// or maintainer can do and what evidence proves it. The context files retain
// ownership of domain state, contracts, invariants, and implementation
// boundaries. A contribution edge connects those two views without turning a
// capability into a second aggregate or making it a dependency-graph node.
project: {
	mission: "A standalone, controller-first MIDI synthesizer for Steam Deck and desktop. A musician performs from external MIDI while crest-synth renders a stable stereo signal. During backend development the standalone host exposes deliberately primitive text views: large serialized patch-state listings operated from the keyboard, whose purpose is to prove that edits reach canonical state, persistence, the real-time parameter boundary, and audible playback before any substantial visual UI is designed."

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
			description: "A maintainer can navigate a plain text list of every Patch with W/S/A/D, edit the selected serialized value with K plus a direction, and prove the accepted value reaches canonical state, round-trip serialization, the parameter bridge, and playback"
			priority: "required"
			actors: ["actor.musician", "actor.maintainer"]
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
				"requirement.diagnostic_text_presentation",
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
				"requirement.hidef_soundfont_playback",
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
			description: "Render every Patch as a plain serialized text block and edit volume, two sends, pan, mute, and solo through the shared W/S/A/D plus K-modified event vocabulary"
			goals: ["goal.operate_live_mixer"]
			acceptance: navigate_and_edit_mixer: {
				description: "The disposable text view proves keyboard navigation, bounded editing, serialization, projection, and playback without creating a designed control surface"
				actor: "actor.musician"
				steps: [
					{action: "use W/S to move between values and A/D to move between Patch blocks", observes: "the selected line marker moves through a long, scrollable wall of text containing every canonical Patch and horizontal separators"},
					{action: "hold K and press a direction on every editable value", observes: "only the selected typed value changes, the new AppState round-trips through canonical serialization, the matching ParameterSnapshot is published, and the playback engine consumes it"},
					{action: "render before and after representative volume, pan, send, mute, and solo edits", observes: "measured engine parameters and audio change as expected while values remain bounded"},
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
			description: "For MIDI-file demonstrations, partition events by instrument, resolve each instrument from ./sf2/HiDef.sf2 into its own sample Patch, assign Patches to mixer tracks round-robin, and control start/stop-from-beginning with L"
			goals: ["goal.exercise_supported_sound_architecture"]
			acceptance: instrument_parts_become_patches: {
				description: "A multi-instrument Standard MIDI File becomes independently metered patch parts without pretending the test player is a product sequencer"
				actor: "actor.maintainer"
				steps: [
					{action: "load a file containing multiple program/bank identities and percussion", observes: "events are grouped by stable instrument identity, each identity selects its matching HiDef.sf2 preset, every Patch uses the sample engine, and each sounding note-off remains with its note-on part"},
					{action: "order parts by first musical event and assign them", observes: "part N uses mixer track N modulo 16; more than sixteen parts share tracks deterministically without dropping patches"},
					{action: "load a file with missing instrument metadata", observes: "the reader falls back deterministically to source-track/channel identity and records that fallback in the part label"},
					{action: "press L while stopped, press L while playing, then press L again", observes: "playback starts at event zero, stops and rewinds with active notes released, then starts again at event zero through serialized AppEvents"},
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
			description: "The current standalone window contains only a scrollable plain-text Patch listing and a one-line key reminder. It has no dashboard, panels, columns, meters, graphical controls, custom widgets, view switching, editor screens, or on-screen keyboard."
			goals: ["goal.operate_live_mixer"]
			capabilities: ["capability.pointer_free_mixer_control"]
		}
		diagnostic_text_presentation: {
			kind: "nonfunctional"
			description: "The initial view is intentionally disposable: default monospaced labels in one vertical scroll area. Each Patch is a multi-line serialization of its identifier, name, mixer assignment, volume, pan, sends, mute, and solo; Patch blocks are separated by a repeated ASCII horizontal rule and the selected value is prefixed with `>`. No substantial UI element or styling abstraction is permitted."
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
			description: "W/S/A/D and the gamepad d-pad emit navigation; holding K or the gamepad edit modifier makes the same directions emit adjustments instead. The reducer receives semantic Navigate or Adjust events, never raw keys."
			goals: ["goal.operate_live_mixer"]
			capabilities: ["capability.pointer_free_mixer_control"]
		}
		current_sound_scope: {
			kind: "functional"
			description: "The completion scope is virtual-analog polyphony, zoned sample playback, the built-in HiDef SoundFont instrument plugin for MIDI-file tests, modulation, patch routing, effects chains, mixing, presets, and sessions. Wavetable and FM identifiers remain unimplemented until separately proven."
			goals: ["goal.exercise_supported_sound_architecture"]
			capabilities: ["capability.polyphonic_sound_generation", "capability.configurable_instrument_graph"]
		}
		deterministic_instrument_assignment: {
			kind: "functional"
			description: "MIDI-file playback is test orchestration: identify parts by bank/program and percussion, map those identities to HiDef.sf2 instruments, use bank 0 program 0 only for metadata-free fallback parts, create one sample Patch per part, and assign first-seen parts to mixer tracks with index modulo 16."
			goals: ["goal.exercise_supported_sound_architecture"]
			capabilities: ["capability.instrument_partitioned_test_playback"]
		}
		hidef_soundfont_playback: {
			kind: "functional"
			description: "MIDI-file mode uses the built-in SoundFont instrument plugin and exactly ./sf2/HiDef.sf2. Each InstrumentIdentity selects its bank/program or percussion preset; virtual-analog fallback is prohibited. L toggles stopped/playing, and every start begins at event zero."
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
			description: "A headless mixer run proves complete serialized Patch rows, W/S/A/D and K+direction semantics, AppState round-trip identity, parameter publication, playback-engine consumption, bounded editing, solo isolation, and metering independence."
			validations: ["validation.mixer_integration", "validation.autopilot"]
			witnesses: ["witness.mixer_control_path"]
		}
		midi_instrument_partition: {
			kind: "behavioral_witness"
			description: "A real multi-track MIDI file is partitioned by instrument, resolved to matching presets in ./sf2/HiDef.sf2, materialized as sample Patches, assigned round-robin, started/stopped/restarted from zero with L-equivalent AppEvents, and rendered audibly."
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
		plugin_hosting: "VST, CLAP, AU, and third-party-effect hosting remain out of scope. The built-in SoundFontPlugin instrument-source port is part of MIDI-file test playback and is not an external plugin-hosting feature."
		additional_editor_screens: "Patch, preset-browser, modulation-matrix, and MIDI-configuration screens are future work; the current standalone GUI is intentionally mixer-only."
		onscreen_instrument: "The UI is not a performance surface and does not provide an on-screen keyboard or originate notes."
		sequencer_product: "Standard MIDI File playback and built-in note schedules are proof/demo inputs, not a composition or sequencing feature."
		pointer_input: "Mouse and touch interaction are outside the current controller-first mixer scope."
		styled_ui: "Designed panels, dashboards, meters, faders, inspectors, themes, custom widgets, animations, and visual polish are explicitly deferred until the serialized backend views and edit-to-playback path are proven."
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
