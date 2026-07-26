package crestsynth

project: {
	mission: "crest-synth is a standalone controller-first instrument host. Its current executable slice prepares alternating capability-configured HiDef SoundFont and Braids instruments with capability-polymorphic voice policies and common per-note envelopes into a bounded rack, mixes them to low-latency stereo audio, exposes reducer-owned MIXER and schema-driven PATCH contexts, and replaces the focused Patch engine through a one-in-flight off-callback prepared-graph workflow."

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
			description: "The application automatically plays Corridors of Time through alternating correctly configured SoundFont and Braids Patches and produces stereo audio"
			priority: "required"
			actors: ["actor.player", "actor.maintainer"]
			capabilities: [
				"capability.instrument_capability_model",
				"capability.prepared_engine_rack",
				"capability.soundfont_audio",
				"capability.braids_engine",
				"capability.per_voice_envelope",
				"capability.automatic_test_midi",
				"capability.global_mix",
				"capability.realtime_execution",
			]
			requirements: [
				"requirement.production_two_engines",
				"requirement.polymorphic_voice_envelopes",
				"requirement.prepared_instrument_boundary",
				"requirement.structural_graph_handoff",
				"requirement.off_callback_graph_retirement",
				"requirement.fixed_soundfont",
				"requirement.fixed_midi_fixture",
				"requirement.global_effects_only",
				"requirement.hard_realtime_audio",
				"requirement.test_input_is_not_a_sequencer",
			]
		}
		control_synth: {
			description: "The player can navigate and edit every current mixer, envelope, engine-scalar, and global parameter in the transitional MIXER text context, while the maintainer can inspect one capability-polymorphic Patch/config schema reaching both renderers through the one-way loop"
			priority: "required"
			actors: ["actor.player", "actor.maintainer"]
			dependsOn: ["goal.play_test_song"]
			capabilities: [
				"capability.instrument_capability_model",
				"capability.prepared_engine_rack",
				"capability.one_way_parameter_control",
				"capability.global_mix",
				"capability.realtime_execution",
				"capability.per_voice_envelope",
			]
			requirements: [
				"requirement.generic_instrument_config",
				"requirement.descriptor_owned_instrument_schema",
				"requirement.explicit_capability_failure",
				"requirement.one_way_loop",
				"requirement.basic_text_contexts",
				"requirement.keyboard_controls",
				"requirement.evolvable_boundaries",
			]
		}
		inspect_patch: {
			description: "The player can directly select PATCH or MIXER and inspect one stable Patch identity, MIDI channel, active engine, common ADSR, and capability-provided fields from canonical state without changing audio"
			priority: "required"
			actors: ["actor.player", "actor.maintainer"]
			dependsOn: ["goal.control_synth"]
			capabilities: [
				"capability.instrument_capability_model",
				"capability.one_way_parameter_control",
				"capability.schema_driven_patch_page",
			]
			requirements: [
				"requirement.two_top_level_contexts",
				"requirement.semantic_context_selection",
				"requirement.descriptor_driven_patch_projection",
				"requirement.stable_patch_focus",
				"requirement.projection_only_context_switch",
			]
		}
		observe_synth: {
			description: "The maintainer can run an exhaustive deterministic GUI demo and inspect a complete event log, state tree, coverage matrix, projections, and audio effects"
			priority: "required"
			actors: ["actor.maintainer"]
			dependsOn: ["goal.select_patch_engine"]
			capabilities: [
				"capability.observable_demo_scene",
				"capability.schema_driven_patch_page",
				"capability.asynchronous_engine_selection",
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
			description: "The maintainer can launch the real standalone UI and audio device, watch a paced autonomous scene exercise every current editable parameter and switch the focused Patch SoundFont to Braids and back, hear each acknowledged engine, and inspect coherent live checkpoints"
			priority: "required"
			actors: ["actor.player", "actor.maintainer"]
			dependsOn: ["goal.observe_synth"]
			capabilities: [
				"capability.live_observable_demo",
				"capability.asynchronous_engine_selection",
			]
			requirements: [
				"requirement.separate_live_demo",
				"requirement.paced_production_path",
				"requirement.live_current_surface",
				"requirement.canonical_live_projection",
				"requirement.bounded_audio_observation",
				"requirement.live_demo_completion",
				"requirement.live_engine_selection_proof",
				"requirement.headless_demo_preserved",
			]
		}
	}

	capabilities: {
		instrument_capability_model: {
			description: "Represent installed instrument implementations through stable capability descriptors and generic Patch-owned configs while keeping preparation and prepared runtime ownership behind separate ports"
			goals: ["goal.play_test_song", "goal.control_synth", "goal.inspect_patch", "goal.select_patch_engine"]
			acceptance: soundfont_is_one_capability: {
				description: "the current SoundFont path is expressed as one validated capability rather than the universal Patch shape"
				actor: "actor.maintainer"
				steps: [
					{action: "construct the application", observes: "the immutable registry contains instrument.soundfont.hidef with EngineManaged polyphony and instrument.braids with FixedPerPatch(16), plus their exact ordered structural/scalar parameter descriptors"},
					{action: "initialize the Corridors fixture", observes: "the fixture edge converts zero-based even parts to exact SoundFont configs and odd parts to default Braids configs, then validates every InstrumentConfig before Patch installation"},
					{action: "inspect StateTree and TextProjection", observes: "the registry plus every Patch capability id, value, and asset reference are projected from the same canonical descriptors without SoundFont-specific projection branches"},
					{action: "attempt unknown, duplicate, missing, mismatched-kind, and out-of-range config mutations", observes: "each is rejected with a typed error and no descriptor, config, preset, asset, or renderer is substituted"},
				]
				evidence: ["evidence.capability_model_contract"]
			}
		}
		soundfont_audio: {
			description: "Parse HiDef.sf2 once, prepare one engine-managed synthesizer per SoundFont Patch, and render it through the capability-neutral rack"
			goals: ["goal.play_test_song"]
			acceptance: audible_patches: {
				description: "different MIDI instruments become correctly configured audible SoundFont patches"
				actor: "actor.maintainer"
				steps: [
					{action: "start the application with HiDef.sf2 present", observes: "the SoundFont parses once outside the callback and prepares exactly one engine-managed synthesizer for each accepted SoundFont Patch"},
					{action: "play notes belonging to multiple instrument parts", observes: "the generic rack targets only the prepared instrument for that Patch and produces one isolated bounded stereo stem"},
				]
				evidence: ["evidence.running_synth"]
			}
		}
		braids_engine: {
			description: "Render an intentional second production engine through the pinned Mutable Instruments Braids MacroOscillator implementation"
			goals: ["goal.play_test_song", "goal.control_synth"]
			acceptance: pinned_sixteen_voice_braids: {
				description: "the official C++ DSP produces bounded, parameterized polyphonic audio without leaking engine identity into shared contracts"
				actor: "actor.maintainer"
				steps: [
					{action: "prepare a Braids Patch at 48 kHz", observes: "the pinned opaque adapter initializes exactly sixteen oscillators outside the callback and rejects every unsupported rate or config without fallback"},
					{action: "play sixteen notes and then a seventeenth", observes: "all sixteen voices render finite audio and the oldest voice is stolen deterministically"},
					{action: "admit N Braids Patches, including a three-Patch witness", observes: "the graph owns N distinct native banks and 16 × N voices, reports forty-eight for three Patches, and has no shared global pool, Braids-specific Patch-count limit, or cross-Patch stealing"},
					{action: "edit Model, Timbre, and Color through AppState.apply", observes: "the matching descriptor-ordered scalar slots reach only that Braids Patch and each produces a measured audible change"},
				]
				evidence: ["evidence.braids_engine_contract"]
			}
		}
		per_voice_envelope: {
			description: "Apply one canonical configurable Patch ADSR independently to each admitted SoundFont and Braids note voice"
			goals: ["goal.play_test_song", "goal.control_synth"]
			acceptance: independent_overlapping_notes: {
				description: "overlapping notes retain separate envelope lifecycles in both engines"
				actor: "actor.maintainer"
				steps: [
					{action: "edit every ADSR field through the normal reducer", observes: "state, text, StateTree, and the matching fixed real-time Patch projection contain the same accepted value"},
					{action: "release one of two overlapping notes", observes: "only the released voice enters Release while the held voice remains independently audible"},
					{action: "repeat for SoundFont and Braids", observes: "both production adapters apply the envelope before note audio enters the Patch stem"},
				]
				evidence: ["evidence.per_voice_envelope_contract"]
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
			goals: ["goal.control_synth", "goal.inspect_patch", "goal.select_patch_engine"]
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
		schema_driven_patch_page: {
			description: "Project one stable Patch through the installed capability registry into PATCH with one reducer-owned engine focus while preserving the basic MIXER context"
			goals: ["goal.inspect_patch", "goal.select_patch_engine", "goal.observe_synth"]
			acceptance: two_context_projection: {
				description: "direct page selection produces exact context-specific projections through the canonical reducer without an audio or graph change"
				actor: "actor.maintainer"
				steps: [
					{action: "press 2 after the fixture Patches are installed", observes: "WindowInput becomes SelectContext(PATCH), AppState.apply updates only reducer-owned InteractionState, and the next immutable projection names PATCH and the focused PatchId"},
					{action: "inspect SoundFont and Braids Patch projections", observes: "each shows Patch identity and MIDI channel, the active CapabilityId and label, all four canonical ADSR values, every active descriptor section and parameter in descriptor order, stable semantic ids, typed values, update class, and the exact two installed engine choices without an engine-specific page branch"},
					{action: "compare state and audio projections before and after page selection", observes: "session values, active graph revision, engine ownership, parameter values, and queued audio commands are unchanged; only context, accepted generation, serialization, and view projection advance"},
					{action: "press 1", observes: "SelectContext(MIXER) restores the prior MIXER selection and the existing complete diagnostic projection without a second state copy"},
					{action: "inspect the PATCH focus", observes: "the engine row has stable patch.engine identity and is editable only in Ready or Failed; ADSR and descriptor fields remain read-only"},
					{action: "send unsupported PATCH navigation or adjustment", observes: "the reducer returns typed ActionUnavailableInContext, leaves state and generation unchanged, and accepts a later context-selection event"},
				]
				evidence: ["evidence.patch_page_contract"]
			}
		}
		observable_demo_scene: {
			description: "Drive the current GUI vocabulary through production event/reducer/projection/audio seams and emit an exhaustive machine-readable trace"
			goals: ["goal.observe_synth"]
			acceptance: exhaustive_trace: {
				description: "the deterministic demo proves every current input, event, editable parameter, serialized property, rejection path, and emitted effect"
				actor: "actor.maintainer"
				steps: [
					{action: "run make demo", observes: "the real alternating fixture initializes and normalized GUI inputs exercise the same translator, AppLoop, projections, real-time boundaries, mixed prepared rack, complete graph, and mixer as the application"},
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
			description: "Run a paced autonomous scalar-and-engine scene inside the real standalone window and physical audio stream while preserving the canonical reducer, projection, preparation, graph-publication, and event-log path"
			goals: ["goal.observe_live_synth"]
			acceptance: live_scene: {
				description: "one bounded command opens the real UI and audio output, exercises every current editable parameter plus both successful engine directions, emits its completed evidence, and exits successfully"
				actor: "actor.player"
				steps: [
					{action: "run make demo-live", observes: "the normal eframe window and physical CPAL output open with HiDef.sf2 and the existing Corridors of Time fixture"},
					{action: "watch and listen to the paced scene", observes: "fixture MIDI remains responsive while every editable parameter instance changes through AppEvent and AppState.apply, remains visible for at least one rendered frame and the declared dwell, and reaches audio through the published ParameterSnapshot"},
					{action: "watch the focused first Patch switch SoundFont to Braids and back", observes: "semantic requests progress through Preparing, Activating, and Ready via the production threaded worker; each acknowledged graph revision receives targeted MIDI and produces finite nonzero physical output before the next direction begins"},
					{action: "press a mapped key while the autonomous scene is active", observes: "the window translates the semantic input but live orchestration does not dispatch it, so no keyboard EventRecord, generation, projection, or parameter publication can interleave with the pending checkpoint"},
					{action: "wait for the scene to complete", observes: "both engine transitions and accepted/rejected scalar events are present in EventLog, the first Patch is Ready on descriptor-default SoundFont, all active notes are stopped through semantic MIDI events, the four final records are emitted once, the window closes, the physical stream is released, and the command returns success"},
				]
				evidence: ["evidence.live_demo_contract"]
			}
			acceptance: coherent_live_trace: {
				description: "each declared live checkpoint correlates its planned input with one accepted generation, exact projections, emitted effects, and either a matching scalar audio observation or an acknowledged structural graph revision"
				actor: "actor.maintainer"
				steps: [
					{action: "inspect CREST_LIVE_CHECKPOINT records while the window remains responsive", observes: "scalar records contain the input, expected transition, EventRecord outcome, accepted generation, exact projection, emitted effects, and an AudioObservationSnapshot that consumed that generation; engine-selection records additionally show Preparing, Activating, and Ready with the requested capability and a newer acknowledged GraphRevision before targeted finite audio"},
					{action: "inspect the final live outputs", observes: "CREST_LIVE_EVENT_LOG_SUMMARY, CREST_LIVE_STATE_TREE, CREST_LIVE_COVERAGE, and CREST_LIVE_SUMMARY agree on the final generation, prove a lossless retained journal, report exact and distinct scalar and engine-transition coverage, and finish Ready on descriptor-default SoundFont without flooding the terminal with every MIDI record"},
					{action: "run make demo and the project checks", observes: "the deterministic headless proof exercises both successful engine-selection directions plus exhaustive busy, failure, stale, mismatch, and two-run-equality cases while the schema, mutation, real-time, and project gates pass"},
				]
				evidence: ["evidence.live_demo_contract", "evidence.exhaustive_demo_scene"]
			}
		}
		prepared_engine_rack: {
			description: "Prepare one bounded capability-neutral instrument per Patch, render heterogeneous slots through one rack, and replace complete graphs through acknowledged ownership transfer"
			goals: ["goal.play_test_song", "goal.control_synth", "goal.select_patch_engine"]
			acceptance: rack_and_handoff: {
				description: "the runtime owns no SoundFont-shaped dispatch path and graph replacement never prepares or destroys state on the callback"
				actor: "actor.maintainer"
				steps: [
					{action: "build the production graph", observes: "each accepted Patch resolves through exactly one matching InstrumentPreparer into a unique bounded PreparedEngineRack slot without fallback"},
					{action: "dispatch and render through a rack containing two distinct prepared test implementations", observes: "targeted MIDI reaches only the addressed Patch and each implementation fills only its own isolated stem with dynamic dispatch outside inner sample loops"},
					{action: "publish a complete replacement graph", observes: "the callback swaps it only at a block boundary and reports its active GraphRevision through fixed-size acknowledgement"},
					{action: "fill the retired return queue", observes: "the callback retains the old graph in bounded storage, retries without taking another replacement, and no destructor runs until control collection"},
					{action: "attempt missing, duplicate, mismatched, or over-capacity preparation", observes: "the build fails atomically and no preparer, capability, graph, instrument, asset, or route is substituted"},
				]
				evidence: ["evidence.prepared_engine_rack_contract"]
			}
		}
		realtime_execution: {
			description: "Render audio through fixed-capacity lock-free boundaries without callback allocation, locking, blocking, I/O, logging, or destruction"
			goals: ["goal.play_test_song", "goal.control_synth", "goal.select_patch_engine"]
			acceptance: callback_contract: {
				description: "the audio callback consumes ready commands and compatible latest parameters, swaps prepared graphs, and returns retired ownership within its real-time constraints"
				actor: "actor.maintainer"
				steps: [
					{action: "publish control snapshots and MIDI commands while rendering", observes: "the callback reads them through the dedicated command ring and latest-value snapshot transport without blocking"},
					{action: "replace engine-owned state", observes: "a distinct structural boundary transfers the complete prepared graph in, returns its predecessor out, and acknowledges both without callback destruction"},
				]
				evidence: ["evidence.running_synth"]
			}
		}
	}

	requirements: {
		production_two_engines: {kind: "functional", description: "The production application installs exactly the HiDef SoundFont and pinned Braids capabilities/preparers, alternates fixture Patches between them, and never layers or substitutes one for the other", goals: ["goal.play_test_song"], capabilities: ["capability.soundfont_audio", "capability.braids_engine", "capability.instrument_capability_model", "capability.prepared_engine_rack"]}
		polymorphic_voice_envelopes: {kind: "functional", description: "Every admitted Braids Patch independently adds exactly sixteen voices with no Braids-specific Patch-count or global voice budget, every SoundFont Patch owns one engine-managed synthesizer without an artificial sixteen-note cap, and both apply canonical configurable Patch ADSR independently before native note voices enter the Patch stem", goals: ["goal.play_test_song", "goal.control_synth"], capabilities: ["capability.soundfont_audio", "capability.braids_engine", "capability.per_voice_envelope"]}
		prepared_instrument_boundary: {kind: "nonfunctional", description: "Each Patch is prepared outside the callback through exactly one CapabilityId-matched InstrumentPreparer into an object-safe PreparedInstrument; callback dispatch, all-notes-off, and rendering are bounded and capability-neutral, with dynamic dispatch outside inner sample loops", goals: ["goal.play_test_song", "goal.control_synth"], capabilities: ["capability.prepared_engine_rack", "capability.realtime_execution"]}
		structural_graph_handoff: {kind: "nonfunctional", description: "One complete PreparedGraph containing the bounded engine rack, mixer/effect state, routing, stems, and scratch crosses a dedicated bounded ownership queue, swaps only at a render-block boundary, and is correlated by GraphRevision and fixed-size acknowledgement; commands and scalar snapshots use separate transports", goals: ["goal.play_test_song", "goal.control_synth", "goal.select_patch_engine"], capabilities: ["capability.prepared_engine_rack", "capability.realtime_execution", "capability.asynchronous_engine_selection"]}
		off_callback_graph_retirement: {kind: "nonfunctional", description: "A replaced PreparedGraph enters a distinct bounded audio-to-control ownership queue; return pressure retains it in a preallocated callback slot and retries, while destructors run only after explicit control or worker collection and the prior handoff is acknowledged before another publication", goals: ["goal.play_test_song", "goal.control_synth", "goal.select_patch_engine"], capabilities: ["capability.prepared_engine_rack", "capability.realtime_execution", "capability.asynchronous_engine_selection"]}
		fixed_soundfont: {kind: "functional", description: "The SoundFont adapter expects ./sf2/HiDef.sf2 and startup fails clearly when it is absent or invalid", goals: ["goal.play_test_song"], capabilities: ["capability.soundfont_audio"]}
		fixed_midi_fixture: {kind: "functional", description: "The automatic test module targets ./midi/Corridors of Time - Chrono Trigger.mid", goals: ["goal.play_test_song"], capabilities: ["capability.automatic_test_midi"]}
		global_effects_only: {kind: "functional", description: "The signal path contains one shared reverb and one shared delay; channels expose sends to those processors and no other effect slots or processors exist", goals: ["goal.play_test_song"], capabilities: ["capability.global_mix"]}
		hard_realtime_audio: {kind: "nonfunctional", description: "The audio callback uses preallocated bounded storage and performs no allocation, deallocation, locks, blocking, I/O, logging, formatting, panic, unwinding, or destruction, including during prepared graph swap and retirement pressure", goals: ["goal.play_test_song"], capabilities: ["capability.prepared_engine_rack", "capability.realtime_execution"]}
		test_input_is_not_a_sequencer: {kind: "nonfunctional", description: "MIDI-file timing is private test-adapter behavior; the domain exposes no sequencer, transport, timeline, song, clip, pattern, recording, editing, or playback-control model", goals: ["goal.play_test_song"], capabilities: ["capability.automatic_test_midi"]}
		one_way_loop: {kind: "nonfunctional", description: "Every input becomes an AppEvent; AppState.apply commits accepted state before serialization, view projection, parameter publication, or audio-command effects", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control"]}
		responsive_control_projection: {kind: "nonfunctional", description: "A fifteen-Patch production AppLoop dispatches 512 MIDI events through reducer, coherent logical projections, journal, and audio publication within 50 ms in the unoptimized acceptance profile; unchanged immutable projection storage is shared and deferred JSON remains byte-identical to eager canonical output", goals: ["goal.control_synth", "goal.observe_live_synth"], capabilities: ["capability.one_way_parameter_control", "capability.live_observable_demo"]}
		basic_text_contexts: {kind: "functional", description: "The basic adapter renders exactly one immutable text projection at a time: MIXER preserves the complete existing scrollable Patch/global wall and PATCH renders the focused schema-derived Patch page; this increment adds no graphical replacement, panels, or third top-level context", goals: ["goal.control_synth", "goal.inspect_patch"], capabilities: ["capability.one_way_parameter_control", "capability.schema_driven_patch_page"]}
		keyboard_controls: {kind: "functional", description: "1 directly selects MIXER, 2 directly selects PATCH, bare W/S navigate parameters, bare A/D navigate Patch sections, and K plus W/S/A/D adjusts the selected value in MIXER; every key is normalized and translated before AppState.apply and no key mutates state directly", goals: ["goal.control_synth", "goal.inspect_patch"], capabilities: ["capability.one_way_parameter_control", "capability.schema_driven_patch_page"]}
		two_top_level_contexts: {kind: "functional", description: "PATCH and MIXER are the only top-level contexts; the existing diagnostic view is the transitional MIXER projection and PATCH owns one semantic engine-row focus, so the basic interface introduces no third context", goals: ["goal.inspect_patch", "goal.select_patch_engine"], capabilities: ["capability.schema_driven_patch_page"]}
		semantic_context_selection: {kind: "nonfunctional", description: "Digit 1 and Digit 2 normalize to WindowInput, translate to SelectContext(MIXER|PATCH), reduce only through AppState.apply, and project from the committed InteractionState; window, test, and view adapters own no context state", goals: ["goal.inspect_patch"], capabilities: ["capability.one_way_parameter_control", "capability.schema_driven_patch_page"]}
		descriptor_driven_patch_projection: {kind: "functional", description: "PATCH projects the focused PatchId, name, MIDI channel, active and requested CapabilityIds, engine-selection status/failure/revision, installed choices, canonical ADSR, and every active ParameterSpec/value/asset in descriptor order; only the stable engine row is editable and no SoundFont/Braids field list or capability-id branch exists", goals: ["goal.inspect_patch", "goal.select_patch_engine"], capabilities: ["capability.instrument_capability_model", "capability.schema_driven_patch_page", "capability.asynchronous_engine_selection"]}
		stable_patch_focus: {kind: "nonfunctional", description: "InteractionState owns PATCH focus by stable PatchId separately from the preserved MIXER selection; installation initializes it deterministically to the first Patch, context switches retain it, and a missing or stale identity is rejected before projection rather than repaired in a view or interpreted as a vector index", goals: ["goal.inspect_patch"], capabilities: ["capability.schema_driven_patch_page"]}
		projection_only_context_switch: {kind: "nonfunctional", description: "Selecting PATCH or MIXER changes only reducer-owned interaction context and the generation-coherent serialization/view projection; Patch/config/envelope/mixer/global values, ParameterSnapshot values, PreparedGraph revision and ownership, MIDI routing, audio commands, and rendered behavior remain unchanged", goals: ["goal.inspect_patch"], capabilities: ["capability.one_way_parameter_control", "capability.schema_driven_patch_page", "capability.realtime_execution"]}
		generic_instrument_config: {kind: "functional", description: "Patch owns one InstrumentConfig containing a CapabilityId, ordered typed parameter assignments, and stable asset references; Patch contains no SoundFont-only fields, engine instance, descriptor copy, prepared state, or fallback config", goals: ["goal.control_synth"], capabilities: ["capability.instrument_capability_model"]}
		descriptor_owned_instrument_schema: {kind: "nonfunctional", description: "Each installed InstrumentCapabilityProvider supplies one immutable CapabilityDescriptor whose ordered ParameterSpecs define ids, labels, kinds, Scalar or Structural update class, defaults, bounds or choices, dependencies, asset needs, FixedPerPatch or EngineManaged voice policy, and supported MIDI kinds; serialization, projection, validation, and coverage consume that schema", goals: ["goal.control_synth"], capabilities: ["capability.instrument_capability_model"]}
		explicit_capability_failure: {kind: "nonfunctional", description: "Unknown or duplicate capability ids and missing, duplicate, undeclared, wrong-kind, dependency-invalid, or out-of-range assignments fail with typed errors; no capability, descriptor, parameter, asset, preset, or engine is silently substituted", goals: ["goal.control_synth"], capabilities: ["capability.instrument_capability_model"]}
		evolvable_boundaries: {kind: "nonfunctional", description: "Instrument capability metadata, off-thread instrument preparation, capability-neutral prepared rendering, MIDI input, audio output, text rendering, discrete commands, scalar snapshots, and structural graph ownership are expressed as separate ports with replaceable adapters", goals: ["goal.control_synth"], capabilities: ["capability.instrument_capability_model", "capability.prepared_engine_rack", "capability.one_way_parameter_control", "capability.realtime_execution"]}
		deterministic_demo_scene: {kind: "functional", description: "A headless demo scene drives the same normalized 1/2/W/S/A/D/K input translator and production AppLoop as EframeTextWindow, with deterministic checkpoints and no native window or physical audio device", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene", "capability.schema_driven_patch_page"]}
		llm_readable_trace: {kind: "functional", description: "Observation mode emits one deterministic JSON CREST_EVENT_LOG, one CREST_STATE_TREE, and one CREST_OBSERVATION summary with stable schema versions, explicit coverage gaps, and no opaque debug strings", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
		exhaustive_current_surface: {kind: "nonfunctional", description: "The demo and table-driven tests cover every declared AppEvent variant and direction, every supported MidiMessage kind, every current editable Patch parameter on every installed Patch, all seven global parameters, every serialized state/projection property, accepted and rejected outcomes, and measured downstream effects", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
		schema_derived_surface: {kind: "nonfunctional", description: "The exhaustive expected set is derived from the production WindowInput descriptor, installed capability and parameter descriptors, typed semantic/mixer descriptors, and serialized leaf discovery, then compared for exact set equality with both missing and unexpected empty; hand-maintained duplicate string lists cannot define their own passing coverage universe", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene", "capability.instrument_capability_model"]}
		faithful_audio_observation: {kind: "nonfunctional", description: "Audio proof uses only the reverb and delay inputs supplied through GlobalEffectsProcessor, establishes nonzero sends before wet-parameter comparisons, isolates each comparison from effect-tail evolution, and restores every edited value and send exactly to its captured baseline", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene", "capability.global_mix"]}
		egui_context_verification: {kind: "functional", description: "A headless egui Context processes real egui key/focus events through EframeApplication update with its callback wired to AppLoop, then proves the next frame, EventLog, accepted state, exact TextProjection values, and scroll target all reflect that event without opening a native window", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene", "capability.one_way_parameter_control"]}
		seam_mutation_falsifiability: {kind: "nonfunctional", description: "Six isolated verification-only mutants—dropped adjustment, cross-Patch parameter leak, PatchId misroute, omitted StateTree leaf, dry-to-wet bypass, and zeroed renderer output—must each falsify its own typed witness without manufacturing coverage gaps or altering a completed report", goals: ["goal.observe_synth"], capabilities: ["capability.observable_demo_scene"]}
		separate_live_demo: {kind: "functional", description: "make demo-live invokes the optimized release binary with a dedicated bounded --demo-live autonomous option that opens the normal eframe window and physical CPAL stream, emits completed evidence, closes, and returns; make demo retains its exact headless command and behavior", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo"]}
		paced_production_path: {kind: "nonfunctional", description: "The live scene advances incrementally on control-side window ticks, dispatches autonomous actions only as AppEvents through AppLoop, lets the owning tick nonblockingly advance the production worker and structural coordinator, ignores mapped semantic window input for the duration of the autonomous scene, and never mutates UI, AppState, engine, mixer, graph, or audio state directly", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo", "capability.one_way_parameter_control", "capability.asynchronous_engine_selection"]}
		live_current_surface: {kind: "functional", description: "The expected live scalar coverage set is frozen from the production Patch editable resolver and GlobalParameters descriptor plus installed Patch identities; every mixer, ADSR, Braids-scalar, and global instance changes at least once and remains at its accepted value for at least 500 ms while the mixed fixture is audible, while the separate ordered engine-transition set must contain exactly SoundFont-to-Braids and Braids-to-descriptor-default-SoundFont for the focused first Patch", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo", "capability.global_mix", "capability.per_voice_envelope", "capability.braids_engine", "capability.asynchronous_engine_selection"]}
		canonical_live_projection: {kind: "nonfunctional", description: "The visible frame, EventRecord, StateTree, TextProjection, ParameterSnapshot, engine-selection status, and active graph revision at each live checkpoint all derive from the same accepted AppState generation and correlated structural status; the live runner has no UI-owned, worker-owned, graph-owned, or engine-owned state copy", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo", "capability.one_way_parameter_control", "capability.asynchronous_engine_selection"]}
		bounded_audio_observation: {kind: "nonfunctional", description: "The callback publishes only fixed-size numeric AudioObservationSnapshots through a lock-free latest-value transport; it never logs, formats, allocates, locks, blocks, performs I/O, or destroys state, and the control side correlates observations by parameter generation and monotonically increasing block sequence", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo", "capability.realtime_execution"]}
		live_demo_completion: {kind: "functional", description: "The live runner retains the complete final EventLog for typed verification and emits structured checkpoints plus a compact lossless EventLog summary, StateTree, exact scalar and engine-transition coverage, and human-readable summary; completion requires both acknowledged audible engine directions and descriptor-default SoundFont Ready before semantic all-notes-off, zero active notes, one close request, physical stream release, and success", goals: ["goal.observe_live_synth"], capabilities: ["capability.live_observable_demo", "capability.asynchronous_engine_selection"]}
		headless_demo_preserved: {kind: "nonfunctional", description: "The engine-selection increment expands both demos through the same production lifecycle: make demo retains exhaustive busy/failure/stale, structural audio, schema, controlled-negative, and two-run deterministic proof, while make demo-live adds paced visible and physical-audio success in both directions through the threaded worker; every prior reducer, projection, DSP, real-time, mutation, scalar-live, and teardown gate remains required", goals: ["goal.observe_live_synth", "goal.select_patch_engine"], capabilities: ["capability.live_observable_demo", "capability.observable_demo_scene", "capability.schema_driven_patch_page", "capability.asynchronous_engine_selection", "capability.instrument_capability_model", "capability.prepared_engine_rack", "capability.braids_engine", "capability.per_voice_envelope"]}
	}

		evidence: {
		capability_model_contract: {kind: "behavioral", description: "the production registry, two providers, Patch aggregate, reducer installation path, serializer, and projector agree on the exact SoundFont and Braids schemas and reject invalid or unknown configs without fallback", validations: ["validation.capability_schema", "validation.schema_surface", "validation.test"]}
		prepared_engine_rack_contract: {kind: "behavioral", description: "the production preparation path and heterogeneous test rack prove exact CapabilityId matching, isolated per-Patch dispatch/rendering, complete block-boundary graph swap, one-in-flight acknowledgement, queue-pressure retention, and control-side destruction with no callback allocation or drop", validations: ["validation.prepared_engine_rack", "validation.test"]}
		running_synth: {kind: "behavioral", description: "the real fixed MIDI path prepares alternating SoundFont and Braids Patches through the generic rack and produces independently routed non-silent stems without callback allocation or destruction", validations: ["validation.smoke", "validation.prepared_engine_rack", "validation.braids_engine", "validation.test"], witnesses: ["witness.running_synth"]}
		control_path: {kind: "behavioral", description: "a keyboard-equivalent edit on a non-first Patch changes only its serialized value and audio contribution, a boundary no-op remains nonfatal, and sustained fifteen-Patch MIDI dispatch stays within its measured responsiveness ceiling", validations: ["validation.smoke", "validation.control_dispatch_performance", "validation.test"], witnesses: ["witness.control_path"]}
		exhaustive_demo_scene: {kind: "behavioral", description: "the schema-derived current GUI/event/state/audio surface is exhaustively exercised with exact projection values, faithful causal audio comparisons, a lossless journal, and a complete state tree", validations: ["validation.demo_scene", "validation.schema_surface", "validation.egui_context", "validation.test"], witnesses: ["witness.exhaustive_demo_scene"]}
		mutation_resistance: {kind: "behavioral", description: "independent production-seam mutants for dropped adjustment, cross-Patch parameter leakage, Patch misrouting, StateTree leaf omission, dry-to-wet bypass, and zero renderer output are each rejected by a typed engine-executed witness", validations: ["validation.mutation_harness", "validation.test"], witnesses: ["witness.dropped_adjustment_mutant", "witness.cross_patch_parameter_leak_mutant", "witness.patch_misroute_mutant", "witness.omitted_state_tree_leaf_mutant", "witness.dry_to_wet_bypass_mutant", "witness.zero_renderer_mutant"]}
		live_demo_contract: {kind: "behavioral", description: "the paced autonomous orchestration is verified against the production reducer, worker port, structural coordinator, responsive generation-only projections, event log, render publication, two acknowledged audible engine directions, bounded audio observations, mapped-input isolation, and successful bounded shutdown without requiring a native CI window or device", validations: ["validation.live_demo", "validation.control_dispatch_performance", "validation.test"]}
		braids_engine_contract: {kind: "behavioral", description: "the pinned native adapter, FixedPerPatch(16) descriptor, 16 × N scaling across N admitted Braids Patches, exact sample-rate policy, mixed routing, scalar effects, lifecycle, and timing are verified through production preparation/render seams", validations: ["validation.braids_engine", "validation.prepared_engine_rack", "validation.test"]}
			per_voice_envelope_contract: {kind: "behavioral", description: "the canonical four-field envelope projects exactly and independently controls overlapping SoundFont and Braids note voices", validations: ["validation.per_voice_envelope", "validation.prepared_engine_rack", "validation.test"]}
			patch_page_contract: {kind: "behavioral", description: "the production input translator, reducer, serializer, context projector, basic eframe adapter, and audio boundary prove exact two-context selection, descriptor-derived Patch rows, stable Patch/engine focus, and reducer-owned engine-row status", validations: ["validation.patch_page_projection", "validation.schema_surface", "validation.egui_context", "validation.engine_selection_workflow", "validation.test"]}
		}

	nonGoals: {
		sequencing: "crest-synth does not provide sequencing, transport, recording, arrangement, clips, patterns, a timeline, or song editing"
		other_engines: "production installs and permits prepared selection only between HiDef SoundFont and Braids; no additional oscillator, physical-model, standalone sampler, wavetable, FM, plugin, or layering is exposed"
		additional_effects: "crest-synth does not provide channel inserts, effect chains, EQ, compression, chorus, distortion, or limiting"
		elaborate_ui: "crest-synth does not yet provide dashboards, panels, meters, faders, custom widgets, themes, mouse interaction, or the Figma-derived graphical interface; this increment uses two projections in the existing basic text adapter"
		sound_library: "crest-synth does not provide preset, session, bank, sample-library, or patch-browser persistence"
		live_midi_adapter: "a physical MIDI device adapter is not included; the automatic file fixture implements the MIDI input port used by the application"
		later_phase_three_increments: "This increment does not introduce PATCH ADSR or capability-parameter editing, SoundFont preset discovery/selection, an engine-choice modal, inactive-engine config caching, or seamless voice/effect-tail migration"
		later_roadmap_phases: "This increment does not introduce per-Patch effects, modulation, arbitrary graph editing, persistence, or the Figma-derived replacement interface"
	}

	completion: {
		requiredGoals: ["goal.play_test_song", "goal.control_synth", "goal.inspect_patch", "goal.select_patch_engine", "goal.observe_synth", "goal.observe_live_synth"]
		projectChecks: ["validation.format", "validation.clippy", "validation.test", "validation.smoke", "validation.capability_schema", "validation.patch_page_projection", "validation.engine_selection_workflow", "validation.prepared_engine_rack", "validation.braids_engine", "validation.per_voice_envelope", "validation.control_dispatch_performance", "validation.demo_scene", "validation.schema_surface", "validation.egui_context", "validation.mutation_harness", "validation.live_demo", "validation.production_runtime_contracts", "validation.audio_renderer_realtime_contract", "validation.prepared_graph_handoff_contract", "validation.audio_observation_realtime_contract", "validation.zero_selection_guard"]
	}
}
