package crestsynth

project: {
	mission: "crest-synth is a standalone SoundFont synthesizer. It plays MIDI through patches backed by HiDef.sf2, mixes them to low-latency stereo audio, and exposes every current parameter in one keyboard-controlled text view."

	actors: {
		player: {
			description: "the person listening to the running synth and changing its parameters from the keyboard"
		}
		maintainer: {
			description: "the person verifying that control events, state, real-time parameters, MIDI routing, and audio agree"
		}
	}

	goals: {
		play_test_song: {
			description: "The application automatically plays Corridors of Time through correctly configured SoundFont patches and produces stereo audio"
			priority: "required"
			actors: ["actor.player", "actor.maintainer"]
			capabilities: [
				"capability.soundfont_audio",
				"capability.automatic_test_midi",
				"capability.global_mix",
				"capability.realtime_execution",
			]
			requirements: [
				"requirement.soundfont_only",
				"requirement.fixed_soundfont",
				"requirement.fixed_midi_fixture",
				"requirement.global_effects_only",
				"requirement.hard_realtime_audio",
				"requirement.test_input_is_not_a_sequencer",
			]
		}
		control_synth: {
			description: "The player can navigate and edit every current parameter in one text view, and each accepted edit reaches serialized state and the audio engine through the one-way loop"
			priority: "required"
			actors: ["actor.player", "actor.maintainer"]
			dependsOn: ["goal.play_test_song"]
			capabilities: [
				"capability.one_way_parameter_control",
				"capability.global_mix",
				"capability.realtime_execution",
			]
			requirements: [
				"requirement.one_way_loop",
				"requirement.single_text_view",
				"requirement.keyboard_controls",
				"requirement.evolvable_boundaries",
			]
		}
		observe_synth: {
			description: "The maintainer can run an exhaustive deterministic GUI demo and inspect a complete event log, state tree, coverage matrix, projections, and audio effects"
			priority: "required"
			actors: ["actor.maintainer"]
			dependsOn: ["goal.control_synth"]
			capabilities: [
				"capability.observable_demo_scene",
			]
			requirements: [
				"requirement.deterministic_demo_scene",
				"requirement.llm_readable_trace",
				"requirement.exhaustive_current_surface",
				"requirement.schema_derived_surface",
				"requirement.faithful_audio_observation",
				"requirement.egui_context_verification",
				"requirement.seam_mutation_falsifiability",
			]
		}
	}

	capabilities: {
		soundfont_audio: {
			description: "Load HiDef.sf2, configure each Patch with its MIDI instrument, and render its notes through the SoundFont engine"
			goals: ["goal.play_test_song"]
			acceptance: audible_patches: {
				description: "different MIDI instruments become correctly configured audible SoundFont patches"
				actor: "actor.maintainer"
				steps: [
					{action: "start the application with HiDef.sf2 present", observes: "the SoundFont loads once before audio rendering"},
					{action: "play notes belonging to multiple instrument parts", observes: "each note reaches the Patch configured for that part's bank, program, or percussion instrument and produces bounded stereo samples"},
				]
				evidence: ["evidence.running_synth"]
			}
		}
		automatic_test_midi: {
			description: "Use Corridors of Time as an automatic MIDI input that creates one Patch per instrument and assigns every part a distinct channel"
			goals: ["goal.play_test_song"]
			acceptance: corridors_starts: {
				description: "the fixed test file begins without transport input and exercises several patches"
				actor: "actor.player"
				steps: [
					{action: "open the application", observes: "the test module reads ./midi/Corridors of Time - Chrono Trigger.mid and begins emitting events automatically"},
					{action: "inspect discovered instrument parts", observes: "there is one Patch per instrument identity, part N uses unique channel N, and channel exhaustion fails instead of reusing a channel"},
				]
				evidence: ["evidence.running_synth"]
			}
		}
		global_mix: {
			description: "Mix every Patch with gain, pan, reverb send, and delay send through one global reverb and one global delay"
			goals: ["goal.play_test_song", "goal.control_synth"]
			acceptance: shared_effects: {
				description: "channel parameters and the two shared effects alter the final stereo signal"
				actor: "actor.maintainer"
				steps: [
					{action: "render at least two simultaneously active Patch channels", observes: "each Patch remains a separate stereo stem; gain and pan are applied independently and both sends feed the shared effects"},
					{action: "change a non-first Patch parameter", observes: "only that Patch's dry or send contribution changes while the other Patch contribution is sample-identical"},
					{action: "change a global effect parameter", observes: "the next published snapshot changes the complete expected rendered signal"},
				]
				evidence: ["evidence.running_synth", "evidence.control_path"]
			}
		}
		one_way_parameter_control: {
			description: "Translate keyboard input to AppEvents, reduce them into AppState, serialize accepted state, project text and parameters, and publish them to audio"
			goals: ["goal.control_synth"]
			acceptance: keyboard_edit: {
				description: "navigation and editing use the same reducer and projections as every other control input"
				actor: "actor.player"
				steps: [
					{action: "press W, S, A, or D", observes: "the selection moves without changing a synth parameter"},
					{action: "hold K and press a direction", observes: "exactly the selected bounded parameter changes through AppState.apply"},
					{action: "inspect the resulting screen and audio snapshot", observes: "serialized state, text, published parameters, and engine consumption contain the same accepted value"},
					{action: "adjust again toward an already reached boundary, then issue a valid edit", observes: "the boundary no-op does not close the application and the later edit succeeds"},
				]
				evidence: ["evidence.control_path"]
			}
		}
		observable_demo_scene: {
			description: "Drive the current GUI vocabulary through production event/reducer/projection/audio seams and emit an exhaustive machine-readable trace"
			goals: ["goal.observe_synth"]
			acceptance: exhaustive_trace: {
				description: "the deterministic demo proves every current input, event, editable parameter, serialized property, rejection path, and emitted effect"
				actor: "actor.maintainer"
				steps: [
					{action: "run make demo", observes: "the real fixture initializes and normalized GUI inputs exercise the same translator, AppLoop, projections, real-time boundary, SoundFont engine, and mixer as the application"},
					{action: "inspect CREST_EVENT_LOG", observes: "every input has one accepted or rejected record with a contiguous sequence, before/after generation and state hashes, emitted effects, parameter generation, and projection identity"},
					{action: "inspect CREST_STATE_TREE", observes: "every current Patch identity/instrument/parameter, global parameter, selection property, text projection property, and parameter snapshot property is present with the exact value from the same accepted generation"},
					{action: "inspect CREST_OBSERVATION coverage", observes: "the typed current-surface descriptor exactly equals the exercised normalized-input, event, direction, MIDI-kind, editable-parameter, state-tree leaf, projection, rejection, and emitted-effect set; missing and unexpected are both empty"},
					{action: "inspect audio checkpoints", observes: "both effect inputs are nonzero when global wet controls are compared, each comparison starts from identical effect state, and all parameters, sends, selection, and projections return exactly to their captured baseline"},
					{action: "repeat the demo", observes: "the same scene produces the same logical event/state trace and final tree without relying on a native window, device, wall clock, or random input"},
				]
				evidence: ["evidence.exhaustive_demo_scene", "evidence.mutation_resistance"]
			}
			acceptance: falsifiable_seams: {
				description: "the release proof rejects independently injected defects at the production control, routing, serialization, and rendering seams"
				actor: "actor.maintainer"
				steps: [
					{action: "drop one translated Adjust before AppLoop dispatch", observes: "the adjustment event, exact selected value, and projection checks fail while unrelated state remains unchanged"},
					{action: "copy one edited Patch parameter into a different published Patch entry", observes: "exact StateTree-to-ParameterSnapshot equality and untargeted Patch audio isolation fail"},
					{action: "rewrite one PatchId at the audio-command routing seam", observes: "the target/untargeted stem and command-to-event identity checks fail"},
					{action: "omit one required leaf while serializing StateTree", observes: "typed schema equality and exact projection/state checks fail"},
					{action: "excite global effects from dry audio while both supplied sends are zero", observes: "the paired zero-input wet-output isolation predicate fails from identical effect state"},
					{action: "zero the renderer output buffer after the production render path", observes: "the finite nonzero audio predicate fails even though the control trace remains complete"},
				]
				evidence: ["evidence.mutation_resistance"]
			}
		}
		realtime_execution: {
			description: "Render audio through fixed-capacity lock-free boundaries without callback allocation, locking, blocking, I/O, logging, or destruction"
			goals: ["goal.play_test_song", "goal.control_synth"]
			acceptance: callback_contract: {
				description: "the audio callback consumes ready commands and the newest complete parameters within its real-time constraints"
				actor: "actor.maintainer"
				steps: [
					{action: "publish control snapshots and MIDI commands while rendering", observes: "the callback reads them through the lock-free AudioBoundary without blocking"},
					{action: "replace engine-owned state", observes: "retired data is destroyed on the control side through the same boundary"},
				]
				evidence: ["evidence.running_synth"]
			}
		}
	}

	requirements: {
		soundfont_only: {kind: "functional", description: "The application owns exactly one SoundFontEngine adapter and one shared parsed SoundFont bank; the adapter may own bounded per-channel rustysynth render lanes required to expose independent Patch stems, but there is no per-Patch SoundFontEngine object, alternate synthesis engine, layering engine, or EngineType union", goals: ["goal.play_test_song"], capabilities: ["capability.soundfont_audio"]}
		fixed_soundfont: {kind: "functional", description: "The SoundFont adapter expects ./sf2/HiDef.sf2 and startup fails clearly when it is absent or invalid", goals: ["goal.play_test_song"], capabilities: ["capability.soundfont_audio"]}
		fixed_midi_fixture: {kind: "functional", description: "The automatic test module targets ./midi/Corridors of Time - Chrono Trigger.mid", goals: ["goal.play_test_song"], capabilities: ["capability.automatic_test_midi"]}
		global_effects_only: {kind: "functional", description: "The signal path contains one shared reverb and one shared delay; channels expose sends to those processors and no other effect slots or processors exist", goals: ["goal.play_test_song"], capabilities: ["capability.global_mix"]}
		hard_realtime_audio: {kind: "nonfunctional", description: "The audio callback uses preallocated bounded storage and performs no allocation, locks, blocking, I/O, logging, or destruction", goals: ["goal.play_test_song"], capabilities: ["capability.realtime_execution"]}
		test_input_is_not_a_sequencer: {kind: "nonfunctional", description: "MIDI-file timing is private test-adapter behavior; the domain exposes no sequencer, transport, timeline, song, clip, pattern, recording, editing, or playback-control model", goals: ["goal.play_test_song"], capabilities: ["capability.automatic_test_midi"]}
		one_way_loop: {kind: "nonfunctional", description: "Every input becomes an AppEvent; AppState.apply commits accepted state before serialization, view projection, parameter publication, or audio-command effects", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control"]}
		single_text_view: {kind: "functional", description: "The UI is one scrollable wall of text listing every Patch parameter and the global parameters; Patch sections are separated by ------------------------------------------------------------", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control"]}
		keyboard_controls: {kind: "functional", description: "Bare W/S navigate parameters, bare A/D navigate Patch sections, and K plus W/S/A/D adjusts the selected value; K is a modifier and no key mutates state directly", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control"]}
		evolvable_boundaries: {kind: "nonfunctional", description: "Sound generation, MIDI input, audio output, text rendering, and the real-time boundary are expressed as ports with replaceable adapters", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control", "capability.realtime_execution"]}
		deterministic_demo_scene: {kind: "functional", description: "A headless demo scene drives the same normalized W/S/A/D/K input translator and production AppLoop as EframeTextWindow, with deterministic checkpoints and no native window or physical audio device", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
		llm_readable_trace: {kind: "functional", description: "Observation mode emits one deterministic JSON CREST_EVENT_LOG, one CREST_STATE_TREE, and one CREST_OBSERVATION summary with stable schema versions, explicit coverage gaps, and no opaque debug strings", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
		exhaustive_current_surface: {kind: "nonfunctional", description: "The demo and table-driven tests cover every declared AppEvent variant and direction, every supported MidiMessage kind, every current editable Patch parameter on every installed Patch, all seven global parameters, every serialized state/projection property, accepted and rejected outcomes, and measured downstream effects", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
		schema_derived_surface: {kind: "nonfunctional", description: "The exhaustive expected set is derived from the production WindowInput descriptor, typed semantic/parameter descriptors, and serialized leaf discovery, then compared for exact set equality with both missing and unexpected empty; hand-maintained duplicate string lists cannot define their own passing coverage universe", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
		faithful_audio_observation: {kind: "nonfunctional", description: "Audio proof uses only the reverb and delay inputs supplied through GlobalEffectsProcessor, establishes nonzero sends before wet-parameter comparisons, isolates each comparison from effect-tail evolution, and restores every edited value and send exactly to its captured baseline", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene", "capability.global_mix"]}
		egui_context_verification: {kind: "functional", description: "A headless egui Context processes real egui key/focus events through EframeApplication update with its callback wired to AppLoop, then proves the next frame, EventLog, accepted state, exact TextProjection values, and scroll target all reflect that event without opening a native window", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene", "capability.one_way_parameter_control"]}
		seam_mutation_falsifiability: {kind: "nonfunctional", description: "Six isolated verification-only mutants—dropped adjustment, cross-Patch parameter leak, PatchId misroute, omitted StateTree leaf, dry-to-wet bypass, and zeroed renderer output—must each falsify its own typed witness without manufacturing coverage gaps or altering a completed report", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
	}

		evidence: {
		running_synth: {kind: "behavioral", description: "the real fixed MIDI and SoundFont path produces independently routed non-silent Patch stems without callback allocation", validations: ["validation.smoke", "validation.test"], witnesses: ["witness.running_synth"]}
		control_path: {kind: "behavioral", description: "a keyboard-equivalent edit on a non-first Patch changes only its serialized value and audio contribution, while a boundary no-op remains nonfatal", validations: ["validation.smoke", "validation.test"], witnesses: ["witness.control_path"]}
		exhaustive_demo_scene: {kind: "behavioral", description: "the schema-derived current GUI/event/state/audio surface is exhaustively exercised with exact projection values, faithful causal audio comparisons, a lossless journal, and a complete state tree", validations: ["validation.demo_scene", "validation.schema_surface", "validation.egui_context", "validation.test"], witnesses: ["witness.exhaustive_demo_scene"]}
		mutation_resistance: {kind: "behavioral", description: "independent production-seam mutants for dropped adjustment, cross-Patch parameter leakage, Patch misrouting, StateTree leaf omission, dry-to-wet bypass, and zero renderer output are each rejected by a typed engine-executed witness", validations: ["validation.mutation_harness", "validation.test"], witnesses: ["witness.dropped_adjustment_mutant", "witness.cross_patch_parameter_leak_mutant", "witness.patch_misroute_mutant", "witness.omitted_state_tree_leaf_mutant", "witness.dry_to_wet_bypass_mutant", "witness.zero_renderer_mutant"]}
	}

	nonGoals: {
		sequencing: "crest-synth does not provide sequencing, transport, recording, arrangement, clips, patterns, a timeline, or song editing"
		other_engines: "crest-synth does not provide oscillator, virtual-analog, standalone sampler, wavetable, FM, or plugin synthesis"
		additional_effects: "crest-synth does not provide channel inserts, effect chains, EQ, compression, chorus, distortion, or limiting"
		elaborate_ui: "crest-synth does not provide dashboards, panels, meters, faders, custom widgets, themes, multiple screens, mouse interaction, or graphical editing"
		sound_library: "crest-synth does not provide preset, session, bank, sample-library, or patch-browser persistence"
		live_midi_adapter: "a physical MIDI device adapter is not included; the automatic file fixture implements the MIDI input port used by the application"
	}

	completion: {
		requiredGoals: ["goal.play_test_song", "goal.control_synth", "goal.observe_synth"]
		projectChecks: ["validation.format", "validation.clippy", "validation.test", "validation.smoke", "validation.demo_scene", "validation.schema_surface", "validation.egui_context", "validation.mutation_harness"]
	}
}
