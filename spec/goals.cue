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
			description: "Use Corridors of Time as an automatic MIDI input that creates one Patch per instrument and assigns parts to channels round-robin"
			goals: ["goal.play_test_song"]
			acceptance: corridors_starts: {
				description: "the fixed test file begins without transport input and exercises several patches"
				actor: "actor.player"
				steps: [
					{action: "open the application", observes: "the test module reads ./midi/Corridors of Time - Chrono Trigger.mid and begins emitting events automatically"},
					{action: "inspect discovered instrument parts", observes: "there is one Patch per instrument identity and part N uses channel N modulo 16"},
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
					{action: "render several active channels", observes: "gain and pan are applied independently and both sends feed the shared effects"},
					{action: "change one channel or global effect parameter", observes: "the next published snapshot changes the expected rendered signal"},
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
				]
				evidence: ["evidence.control_path"]
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
		soundfont_only: {kind: "functional", description: "The application owns exactly one SoundFont engine instance; there is no per-Patch engine instance, alternate synthesis engine, layering engine, or EngineType union", goals: ["goal.play_test_song"], capabilities: ["capability.soundfont_audio"]}
		fixed_soundfont: {kind: "functional", description: "The SoundFont adapter expects ./sf2/HiDef.sf2 and startup fails clearly when it is absent or invalid", goals: ["goal.play_test_song"], capabilities: ["capability.soundfont_audio"]}
		fixed_midi_fixture: {kind: "functional", description: "The automatic test module targets ./midi/Corridors of Time - Chrono Trigger.mid", goals: ["goal.play_test_song"], capabilities: ["capability.automatic_test_midi"]}
		global_effects_only: {kind: "functional", description: "The signal path contains one shared reverb and one shared delay; channels expose sends to those processors and no other effect slots or processors exist", goals: ["goal.play_test_song"], capabilities: ["capability.global_mix"]}
		hard_realtime_audio: {kind: "nonfunctional", description: "The audio callback uses preallocated bounded storage and performs no allocation, locks, blocking, I/O, logging, or destruction", goals: ["goal.play_test_song"], capabilities: ["capability.realtime_execution"]}
		test_input_is_not_a_sequencer: {kind: "nonfunctional", description: "MIDI-file timing is private test-adapter behavior; the domain exposes no sequencer, transport, timeline, song, clip, pattern, recording, editing, or playback-control model", goals: ["goal.play_test_song"], capabilities: ["capability.automatic_test_midi"]}
		one_way_loop: {kind: "nonfunctional", description: "Every input becomes an AppEvent; AppState.apply commits accepted state before serialization, view projection, parameter publication, or audio-command effects", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control"]}
		single_text_view: {kind: "functional", description: "The UI is one scrollable wall of text listing every Patch parameter and the global parameters; Patch sections are separated by ------------------------------------------------------------", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control"]}
		keyboard_controls: {kind: "functional", description: "Bare W/S navigate parameters, bare A/D navigate Patch sections, and K plus W/S/A/D adjusts the selected value; K is a modifier and no key mutates state directly", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control"]}
		evolvable_boundaries: {kind: "nonfunctional", description: "Sound generation, MIDI input, audio output, text rendering, and the real-time boundary are expressed as ports with replaceable adapters", goals: ["goal.control_synth"], capabilities: ["capability.one_way_parameter_control", "capability.realtime_execution"]}
	}

	evidence: {
		running_synth: {kind: "behavioral", description: "the real fixed MIDI and SoundFont path produces correctly routed, non-silent audio without callback allocation", validations: ["validation.smoke", "validation.test"], witnesses: ["witness.running_synth"]}
		control_path: {kind: "behavioral", description: "a keyboard-equivalent edit changes one serialized value, publishes it, and changes engine behavior", validations: ["validation.smoke", "validation.test"], witnesses: ["witness.control_path"]}
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
		requiredGoals: ["goal.play_test_song", "goal.control_synth"]
		projectChecks: ["validation.format", "validation.clippy", "validation.test", "validation.smoke"]
	}
}
