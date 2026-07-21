package crestsynth

project: {
	mission: "crest-synth is a standalone controller-first instrument host. Its current executable slice plays MIDI through capability-configured HiDef SoundFont patches, mixes them to low-latency stereo audio, and exposes the current control surface in one keyboard-controlled text view."

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
				"capability.instrument_capability_model",
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
			description: "The player can navigate and edit every current parameter in one text view, while the maintainer can inspect a capability-polymorphic Patch/config schema that reaches the current SoundFont renderer through the one-way loop"
			priority: "required"
			actors: ["actor.player", "actor.maintainer"]
			dependsOn: ["goal.play_test_song"]
			capabilities: [
				"capability.instrument_capability_model",
				"capability.one_way_parameter_control",
				"capability.global_mix",
				"capability.realtime_execution",
			]
			requirements: [
				"requirement.generic_instrument_config",
				"requirement.descriptor_owned_instrument_schema",
				"requirement.explicit_capability_failure",
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
		observe_live_synth: {
			description: "The maintainer can launch the real standalone UI and audio device, watch a paced autonomous scene exercise every current editable parameter, hear the SoundFont result, and inspect coherent live checkpoints"
			priority: "required"
			actors: ["actor.player", "actor.maintainer"]
			dependsOn: ["goal.observe_synth"]
			capabilities: [
				"capability.live_observable_demo",
			]
			requirements: [
				"requirement.separate_live_demo",
				"requirement.paced_production_path",
				"requirement.live_current_surface",
				"requirement.canonical_live_projection",
				"requirement.bounded_audio_observation",
				"requirement.live_demo_completion",
				"requirement.headless_demo_preserved",
			]
		}
	}

	capabilities: {
		instrument_capability_model: {
			description: "Represent installed instrument implementations through stable capability descriptors and generic Patch-owned configs without installing an alternate renderer yet"
			goals: ["goal.play_test_song", "goal.control_synth"]
			acceptance: soundfont_is_one_capability: {
				description: "the current SoundFont path is expressed as one validated capability rather than the universal Patch shape"
				actor: "actor.maintainer"
				steps: [
					{action: "construct the application", observes: "the immutable registry contains exactly instrument.soundfont.hidef with ordered bank, program, percussion, and file parameter descriptors"},
					{action: "initialize the Corridors fixture", observes: "each discovered SoundFontInstrument is translated into the provider's generic typed assignments and validated as an InstrumentConfig before Patch installation"},
					{action: "inspect StateTree and TextProjection", observes: "the registry plus every Patch capability id, value, and asset reference are projected from the same canonical descriptors without SoundFont-specific projection branches"},
					{action: "attempt unknown, duplicate, missing, mismatched-kind, and out-of-range config mutations", observes: "each is rejected with a typed error and no descriptor, config, preset, asset, or renderer is substituted"},
				]
				evidence: ["evidence.capability_model_contract"]
			}
		}
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
		live_observable_demo: {
			description: "Run a paced autonomous scene inside the real standalone window and physical audio stream while preserving the canonical reducer, projection, publication, and event-log path"
			goals: ["goal.observe_live_synth"]
			acceptance: live_scene: {
				description: "one command opens the real UI and audio output, exercises every current editable parameter, and leaves the completed state visible"
				actor: "actor.player"
					steps: [
						{action: "run make demo-live", observes: "the normal eframe window and physical CPAL output open with HiDef.sf2 and the existing Corridors of Time fixture"},
						{action: "watch and listen to the paced scene", observes: "fixture MIDI remains responsive while every editable parameter instance changes through AppEvent and AppState.apply, remains visible for at least one rendered frame and the declared dwell, and reaches audio through the published ParameterSnapshot"},
					{action: "wait for the scene to complete", observes: "accepted and rejected events are present in EventLog, all active notes are stopped through semantic MIDI events, and the final canonical projection remains visible until the window is closed by the user"},
				]
				evidence: ["evidence.live_demo_contract"]
			}
			acceptance: coherent_live_trace: {
				description: "each declared live checkpoint correlates its planned input with one accepted generation, exact projections, emitted effects, and a bounded audio-thread observation"
				actor: "actor.maintainer"
				steps: [
					{action: "inspect CREST_LIVE_CHECKPOINT records while the window remains responsive", observes: "each record contains the input, expected transition, EventRecord outcome, accepted generation, projected value, emitted effects, and an AudioObservationSnapshot that has consumed that generation"},
						{action: "inspect the final live outputs", observes: "CREST_LIVE_EVENT_LOG_SUMMARY, CREST_LIVE_STATE_TREE, CREST_LIVE_COVERAGE, and CREST_LIVE_SUMMARY agree on the final generation, prove a lossless retained journal, and report no missing or unexpected editable parameters without flooding the terminal with every MIDI record"},
					{action: "run the existing make demo and project checks", observes: "the deterministic headless proof and its schema, mutation, real-time, and project gates remain unchanged and pass"},
				]
				evidence: ["evidence.live_demo_contract", "evidence.exhaustive_demo_scene"]
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
		soundfont_only: {kind: "functional", description: "The application installs exactly one instrument capability and owns exactly one SoundFontEngine adapter plus one shared parsed SoundFont bank; Patch/config types are capability-polymorphic, but this increment has no per-Patch engine object, alternate running renderer, layering engine, engine selector, or fallback", goals: ["goal.play_test_song"], capabilities: ["capability.soundfont_audio", "capability.instrument_capability_model"]}
		fixed_soundfont: {kind: "functional", description: "The SoundFont adapter expects ./sf2/HiDef.sf2 and startup fails clearly when it is absent or invalid", goals: ["goal.play_test_song"], capabilities: ["capability.soundfont_audio"]}
		fixed_midi_fixture: {kind: "functional", description: "The automatic test module targets ./midi/Corridors of Time - Chrono Trigger.mid", goals: ["goal.play_test_song"], capabilities: ["capability.automatic_test_midi"]}
		global_effects_only: {kind: "functional", description: "The signal path contains one shared reverb and one shared delay; channels expose sends to those processors and no other effect slots or processors exist", goals: ["goal.play_test_song"], capabilities: ["capability.global_mix"]}
		hard_realtime_audio: {kind: "nonfunctional", description: "The audio callback uses preallocated bounded storage and performs no allocation, locks, blocking, I/O, logging, or destruction", goals: ["goal.play_test_song"], capabilities: ["capability.realtime_execution"]}
		test_input_is_not_a_sequencer: {kind: "nonfunctional", description: "MIDI-file timing is private test-adapter behavior; the domain exposes no sequencer, transport, timeline, song, clip, pattern, recording, editing, or playback-control model", goals: ["goal.play_test_song"], capabilities: ["capability.automatic_test_midi"]}
		one_way_loop: {kind: "nonfunctional", description: "Every input becomes an AppEvent; AppState.apply commits accepted state before serialization, view projection, parameter publication, or audio-command effects", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control"]}
		responsive_control_projection: {kind: "nonfunctional", description: "A fifteen-Patch production AppLoop dispatches 512 MIDI events through reducer, coherent logical projections, journal, and audio publication within 50 ms in the unoptimized acceptance profile; unchanged immutable projection storage is shared and deferred JSON remains byte-identical to eager canonical output", goals: ["goal.control_synth", "goal.observe_live_synth"], capabilities: ["capability.one_way_parameter_control", "capability.live_observable_demo"]}
		single_text_view: {kind: "functional", description: "The UI is one scrollable wall of text listing every Patch parameter and the global parameters; Patch sections are separated by ------------------------------------------------------------", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control"]}
		keyboard_controls: {kind: "functional", description: "Bare W/S navigate parameters, bare A/D navigate Patch sections, and K plus W/S/A/D adjusts the selected value; K is a modifier and no key mutates state directly", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control"]}
		generic_instrument_config: {kind: "functional", description: "Patch owns one InstrumentConfig containing a CapabilityId, ordered typed parameter assignments, and stable asset references; Patch contains no SoundFont-only fields, engine instance, descriptor copy, prepared state, or fallback config", goals: ["goal.control_synth"], capabilities: ["capability.instrument_capability_model"]}
		descriptor_owned_instrument_schema: {kind: "nonfunctional", description: "Each installed InstrumentCapabilityProvider supplies one immutable CapabilityDescriptor whose ordered ParameterSpecs define ids, labels, kinds, Scalar or Structural update class, defaults, bounds or choices, dependencies, asset needs, voice capacity, and supported MIDI kinds; serialization, projection, validation, and coverage consume that schema", goals: ["goal.control_synth"], capabilities: ["capability.instrument_capability_model"]}
		explicit_capability_failure: {kind: "nonfunctional", description: "Unknown or duplicate capability ids and missing, duplicate, undeclared, wrong-kind, dependency-invalid, or out-of-range assignments fail with typed errors; no capability, descriptor, parameter, asset, preset, or engine is silently substituted", goals: ["goal.control_synth"], capabilities: ["capability.instrument_capability_model"]}
		evolvable_boundaries: {kind: "nonfunctional", description: "Instrument capability metadata, current SoundFont generation, MIDI input, audio output, text rendering, and the real-time boundary are expressed as ports with replaceable adapters", goals: ["goal.control_synth"], capabilities: ["capability.instrument_capability_model", "capability.one_way_parameter_control", "capability.realtime_execution"]}
		deterministic_demo_scene: {kind: "functional", description: "A headless demo scene drives the same normalized W/S/A/D/K input translator and production AppLoop as EframeTextWindow, with deterministic checkpoints and no native window or physical audio device", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
		llm_readable_trace: {kind: "functional", description: "Observation mode emits one deterministic JSON CREST_EVENT_LOG, one CREST_STATE_TREE, and one CREST_OBSERVATION summary with stable schema versions, explicit coverage gaps, and no opaque debug strings", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
		exhaustive_current_surface: {kind: "nonfunctional", description: "The demo and table-driven tests cover every declared AppEvent variant and direction, every supported MidiMessage kind, every current editable Patch parameter on every installed Patch, all seven global parameters, every serialized state/projection property, accepted and rejected outcomes, and measured downstream effects", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
		schema_derived_surface: {kind: "nonfunctional", description: "The exhaustive expected set is derived from the production WindowInput descriptor, installed capability and parameter descriptors, typed semantic/mixer descriptors, and serialized leaf discovery, then compared for exact set equality with both missing and unexpected empty; hand-maintained duplicate string lists cannot define their own passing coverage universe", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene", "capability.instrument_capability_model"]}
		faithful_audio_observation: {kind: "nonfunctional", description: "Audio proof uses only the reverb and delay inputs supplied through GlobalEffectsProcessor, establishes nonzero sends before wet-parameter comparisons, isolates each comparison from effect-tail evolution, and restores every edited value and send exactly to its captured baseline", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene", "capability.global_mix"]}
		egui_context_verification: {kind: "functional", description: "A headless egui Context processes real egui key/focus events through EframeApplication update with its callback wired to AppLoop, then proves the next frame, EventLog, accepted state, exact TextProjection values, and scroll target all reflect that event without opening a native window", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene", "capability.one_way_parameter_control"]}
		seam_mutation_falsifiability: {kind: "nonfunctional", description: "Six isolated verification-only mutants—dropped adjustment, cross-Patch parameter leak, PatchId misroute, omitted StateTree leaf, dry-to-wet bypass, and zeroed renderer output—must each falsify its own typed witness without manufacturing coverage gaps or altering a completed report", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
		separate_live_demo: {kind: "functional", description: "make demo-live invokes the optimized release binary with a dedicated --demo-live interactive option that opens the normal eframe window and physical CPAL stream; make demo retains its exact headless command and behavior", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo"]}
		paced_production_path: {kind: "nonfunctional", description: "The live scene advances incrementally on control-side window ticks, dispatches autonomous actions only as AppEvents through AppLoop, and never mutates UI, AppState, engine, mixer, or audio state directly", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo", "capability.one_way_parameter_control"]}
		live_current_surface: {kind: "functional", description: "The expected live coverage set is derived from the production ChannelParameters and GlobalParameters descriptors plus installed Patch identities; every editable parameter instance changes at least once and remains at its accepted value for at least 500 ms while the fixture is audible", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo", "capability.global_mix"]}
		canonical_live_projection: {kind: "nonfunctional", description: "The visible frame, EventRecord, StateTree, TextProjection, and ParameterSnapshot at each live checkpoint all derive from the same accepted AppState generation; the live runner has no UI-owned or engine-owned state copy", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo", "capability.one_way_parameter_control"]}
		bounded_audio_observation: {kind: "nonfunctional", description: "The callback publishes only fixed-size numeric AudioObservationSnapshots through a lock-free latest-value transport; it never logs, formats, allocates, locks, blocks, performs I/O, or destroys state, and the control side correlates observations by parameter generation and monotonically increasing block sequence", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo", "capability.realtime_execution"]}
		live_demo_completion: {kind: "functional", description: "The live runner retains the complete final EventLog for typed verification and emits structured checkpoints plus a compact lossless EventLog summary, StateTree, exact editable-parameter coverage, and human-readable summary; it dispatches all-notes-off through AppLoop for every installed Patch, waits for the audio observation to acknowledge zero active notes, then becomes inert without closing the window", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo"]}
		headless_demo_preserved: {kind: "nonfunctional", description: "Phase 2 increment 1 preserves the deterministic timing, command line, and behavioral acceptance predicates of make demo while deliberately extending its typed schema universe with capability descriptors and generic Patch configs; it introduces no Patch page, ADSR, preset selection, Braids renderer, static Patch effects, modulation, or redesigned interface behavior", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo", "capability.observable_demo_scene", "capability.instrument_capability_model"]}
	}

		evidence: {
		capability_model_contract: {kind: "behavioral", description: "the production registry, provider, Patch aggregate, reducer installation path, serializer, and projector agree on the exact SoundFont capability schema and reject invalid or unknown configs without fallback", validations: ["validation.capability_schema", "validation.schema_surface", "validation.test"]}
		running_synth: {kind: "behavioral", description: "the real fixed MIDI and SoundFont path produces independently routed non-silent Patch stems without callback allocation", validations: ["validation.smoke", "validation.test"], witnesses: ["witness.running_synth"]}
		control_path: {kind: "behavioral", description: "a keyboard-equivalent edit on a non-first Patch changes only its serialized value and audio contribution, a boundary no-op remains nonfatal, and sustained fifteen-Patch MIDI dispatch stays within its measured responsiveness ceiling", validations: ["validation.smoke", "validation.control_dispatch_performance", "validation.test"], witnesses: ["witness.control_path"]}
		exhaustive_demo_scene: {kind: "behavioral", description: "the schema-derived current GUI/event/state/audio surface is exhaustively exercised with exact projection values, faithful causal audio comparisons, a lossless journal, and a complete state tree", validations: ["validation.demo_scene", "validation.schema_surface", "validation.egui_context", "validation.test"], witnesses: ["witness.exhaustive_demo_scene"]}
		mutation_resistance: {kind: "behavioral", description: "independent production-seam mutants for dropped adjustment, cross-Patch parameter leakage, Patch misrouting, StateTree leaf omission, dry-to-wet bypass, and zero renderer output are each rejected by a typed engine-executed witness", validations: ["validation.mutation_harness", "validation.test"], witnesses: ["witness.dropped_adjustment_mutant", "witness.cross_patch_parameter_leak_mutant", "witness.patch_misroute_mutant", "witness.omitted_state_tree_leaf_mutant", "witness.dry_to_wet_bypass_mutant", "witness.zero_renderer_mutant"]}
		live_demo_contract: {kind: "behavioral", description: "the paced interactive orchestration is verified against the production reducer, responsive generation-only projections, event log, render publication, and bounded audio observations without requiring a native CI window or device", validations: ["validation.live_demo", "validation.control_dispatch_performance", "validation.test"]}
	}

	nonGoals: {
		sequencing: "crest-synth does not provide sequencing, transport, recording, arrangement, clips, patterns, a timeline, or song editing"
		other_engines: "this increment defines the polymorphic instrument capability model but installs no Braids, oscillator, virtual-analog, standalone sampler, wavetable, FM, plugin, layering, or engine-selection renderer"
		additional_effects: "crest-synth does not provide channel inserts, effect chains, EQ, compression, chorus, distortion, or limiting"
		elaborate_ui: "crest-synth does not provide dashboards, panels, meters, faders, custom widgets, themes, multiple screens, mouse interaction, or graphical editing"
		sound_library: "crest-synth does not provide preset, session, bank, sample-library, or patch-browser persistence"
		live_midi_adapter: "a physical MIDI device adapter is not included; the automatic file fixture implements the MIDI input port used by the application"
		later_roadmap_phases: "Phase 2 increment 1 does not introduce the prepared multi-engine rack, Braids C++/FFI wrapper, simultaneous mixed-engine proof, Patch page, working ADSR, SoundFont preset browsing, per-Patch effects, modulation, dynamic graph editing, or the Figma-derived replacement interface"
	}

	completion: {
		requiredGoals: ["goal.play_test_song", "goal.control_synth", "goal.observe_synth", "goal.observe_live_synth"]
		projectChecks: ["validation.format", "validation.clippy", "validation.test", "validation.smoke", "validation.capability_schema", "validation.control_dispatch_performance", "validation.demo_scene", "validation.schema_surface", "validation.egui_context", "validation.mutation_harness", "validation.live_demo"]
	}
}
