package crestsynth

project: contexts: Mixer: {
	purpose: "route post-effect Patch stems through one fixed bank of sixteen configurable tracks, then through one shared reverb and one shared delay"
	ubiquitousLanguage: {
		MixerTrackId: "one stable zero-based identity in the fixed T00 through T0F mixer bank"
		PatchOutput: "one Patch-owned output-track choice plus pre-track trim"
		MixerTrackParameters: "the level, pan, mute, solo, and send values owned by one mixer track"
	}

	valueObjects: MixerTrackId: {
		description: "the stable identity of one persistent mixer track"
		from: "u8"
		invariants: [
			"the value is in 0..=15 and displays as T00 through T0F without changing identity",
			"the value is independent of PatchId, MIDI channel, Patch order, instrument schema, layout column, label, or widget id",
		]
		contributesTo: [
			{capability: "capability.sixteen_track_mixer_routing", contribution: "joins Patch routing, track state, semantic focus, fixed snapshots, meters, and proof through one bounded identity"},
			{capability: "capability.semantic_graphical_view_model", contribution: "keeps MIXER focus stable across Patch and viewport changes"},
		]
	}

	valueObjects: PatchOutputParameter: {
		description: "one stable editable field owned by Patch output configuration"
		from: "TrimGain | OutputTrack"
		invariants: [
			"TrimGain is continuous and OutputTrack is a nonwrapping stepped choice over all sixteen MixerTrackIds",
			"the identity contains no display row, control index, label, or widget id",
		]
		contributesTo: [
			{capability: "capability.sixteen_track_mixer_routing", contribution: "gives PATCH Utility canonical trim and route controls"},
			{capability: "capability.schema_driven_patch_page", contribution: "extends PATCH Utility without a second routing model"},
		]
	}

	valueObjects: PatchOutput: {
		description: "the complete Patch-owned scalar output configuration before fixed track accumulation"
		state: {
			trackId: "MixerTrackId"
			trimGainDb: "f32"
		}
		invariants: [
			"trackId is one of exactly sixteen prepared destinations and trimGainDb is finite in -60.0..=6.0",
			"the value contains no track fader, pan, mute, solo, send, meter, scratch buffer, PatchId copy, or UI state",
			"a production-owned descriptor supplies stable ids, labels, bounds, and fine/coarse or adjacent-choice behavior for both fields",
		]
		contributesTo: [
			{capability: "capability.sixteen_track_mixer_routing", contribution: "routes one post-effect Patch contribution without conflating the Patch with its destination track"},
			{capability: "capability.one_way_parameter_control", contribution: "is edited only through semantic action, AppEvent, and AppState.apply"},
		]
	}

	valueObjects: MixerTrackParameter: {
		description: "one stable configurable field owned by a mixer track"
		from: "Level | Pan | Mute | Solo | ReverbSend | DelaySend"
		invariants: [
			"MixerMain orders Level, Pan, Mute, and Solo; MixerInspector orders ReverbSend then DelaySend for the selected track",
			"the production-owned descriptor supplies kind, bounds, default, fine/coarse step, toggle behavior, label, and unit exactly once for every field",
			"the identity contains no PatchId, Patch parameter, row number, column number, label, or widget id",
		]
		contributesTo: [
			{capability: "capability.sixteen_track_mixer_routing", contribution: "defines the complete configurable surface for each of sixteen tracks"},
			{capability: "capability.semantic_graphical_view_model", contribution: "keeps row meaning stable while horizontal navigation changes tracks"},
		]
	}

	valueObjects: MixerTrackParameters: {
		description: "all canonical scalar and toggle state owned by one mixer track"
		state: {
			levelDb: "f32"
			pan: "f32"
			mute: "bool"
			solo: "bool"
			reverbSend: "f32"
			delaySend: "f32"
		}
		invariants: [
			"levelDb is finite in -60.0..=6.0 and pan is finite in -1.0..=1.0",
			"reverbSend and delaySend are finite in 0.0..=1.0",
			"mute and solo are explicit toggles; mute always wins and an active solo set excludes every non-soloed track from dry and send contribution",
			"the value contains no Patch config, output route, meter, processor, buffer, UI state, or duplicate global parameter",
		]
		contributesTo: [
			{capability: "capability.global_mix", contribution: "configures one post-accumulation track before the shared effects and master"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "is the canonical per-track state changed by the track controls"},
		]
	}

	valueObjects: MixerState: {
		description: "the canonical fixed mixer bank owned inside AppState"
		state: {tracks: "[MixerTrackParameters; 16]"}
		invariants: [
			"all sixteen tracks exist before Patch installation and remain addressable whether zero, one, or many Patches route to them",
			"array position is a bounded storage detail whose canonical semantic identity is the matching MixerTrackId",
			"Patch installation, removal, schema replacement, or rerouting never creates, removes, reorders, or silently resets a track",
		]
		contributesTo: [
			{capability: "capability.sixteen_track_mixer_routing", contribution: "separates persistent mixer state from the Patch collection"},
			{capability: "capability.one_way_parameter_control", contribution: "keeps every track edit in canonical reducer-owned state"},
		]
	}

	valueObjects: TrackMeter: {
		description: "one fixed-size numeric measurement of a post-level/pan track before its mute/solo gate"
		state: {
			leftPeak: "f32"
			rightPeak: "f32"
			rms: "f32"
		}
		invariants: [
			"all fields are finite and nonnegative and are zero for an empty or silent track",
			"the meter remains active for a muted or solo-excluded sounding track and never controls audibility",
			"the value is Copy, fixed-size, numeric, and contains no allocation, label, buffer, reference, UI state, or destructor",
		]
		contributesTo: [
			{capability: "capability.sixteen_track_mixer_routing", contribution: "makes every track's routed signal independently observable"},
			{capability: "capability.realtime_execution", contribution: "keeps track metering bounded on the callback"},
		]
	}

	valueObjects: GlobalParameters: {
		description: "all editable parameters shared by the complete mix"
		state: {
			masterGainDb: "f32"
			reverbRoomSize: "f32"
			reverbDamping: "f32"
			reverbReturn: "f32"
			delayMilliseconds: "f32"
			delayFeedback: "f32"
			delayReturn: "f32"
		}
		invariants: [
			"masterGainDb is in -60.0..=6.0",
			"room size, damping, returns, and feedback are in 0.0..=1.0",
			"delayMilliseconds is in 1.0..=2000.0",
			"values are finite",
			"a production-owned typed surface descriptor enumerates these seven fields, bounds, and fine/coarse steps; AppState, StateProjector, and DemoScene consume the same descriptor",
			"the descriptor contains each field exactly once before set conversion and is the independent pre-dispatch oracle for bounds and step sizes",
		]
		contributesTo: [
			{capability: "capability.global_mix", contribution: "configures the one shared reverb, one shared delay, and master level"},
			{capability: "capability.one_way_parameter_control", contribution: "keeps global controls distinct from the sixteen track identities"},
		]
	}

	valueObjects: MixObservation: {
		description: "fixed-size callback-local measurements produced by MixEngine from its track, send, wet, and final-output buffers"
		state: {
			tracks: "[TrackMeter; 16]"
			leftPeak: "f32"
			rightPeak: "f32"
			outputRms: "f32"
			reverbInputRms: "f32"
			delayInputRms: "f32"
			wetOutputRms: "f32"
			nonFiniteSamples: "u64"
			clippedSamples: "u64"
		}
		invariants: [
			"the value is Copy, fixed-size, numeric, and created from exact callback buffers already owned by MixEngine without allocation, logging, formatting, locking, blocking, or I/O",
			"tracks contains exactly one pre-gate meter in MixerTrackId order; reverbInputRms and delayInputRms measure only post-gate track-send sums",
			"wetOutputRms measures only the shared effect return before final master gain; peak and RMS fields are finite and nonnegative",
			"the observation never feeds back into mixing decisions and is not a second owner of audio or parameter state",
		]
		contributesTo: [
			{capability: "capability.sixteen_track_mixer_routing", contribution: "provides exact per-track routing and mute/solo meter evidence"},
			{capability: "capability.live_observable_demo", contribution: "provides causal send, wet, track, and output measurements from mixer-owned buffers"},
			{capability: "capability.realtime_execution", contribution: "keeps callback measurements fixed-size and local before latest-value publication"},
		]
	}

	ports: GlobalEffectsProcessor: {
		direction: "outbound"
		contract: {
			prepare: "(sampleRate: f32, maxFrames: usize, maxDelayMilliseconds: f32) -> Result<(), EffectError>"
			process: "(reverbInput: &[f32], delayInput: &[f32], output: &mut [f32], parameters: &GlobalParameters)"
		}
		invariants: [
			"the implementation contains exactly one mixer-owned reverb and one mixer-owned delay shared by every track; the upstream PreparedPostEffectRack is not part of this port",
			"prepare allocates all effect storage and process is allocation-free and lock-free",
			"production and verification implementations derive wet excitation only from reverbInput and delayInput; dry output is never treated as an implicit send, and zero inputs cannot create a wet return",
		]
		contributesTo: [{capability: "capability.global_mix", contribution: "defines the complete replaceable boundary for the two current global effects"}]
	}

	domainServices: MixEngine: {
		purpose: "trim and accumulate post-effect Patch stems into sixteen tracks, apply track controls and gates, process current shared effects, and produce the master mix"
		uses: [
			"valueObject.Mixer.MixerTrackId",
			"valueObject.Mixer.PatchOutput",
			"valueObject.Mixer.MixerTrackParameters",
			"valueObject.Mixer.MixerState",
			"valueObject.Mixer.TrackMeter",
			"valueObject.Mixer.GlobalParameters",
			"valueObject.Mixer.MixObservation",
			"valueObject.RealTime.PatchAudioBlock",
			"valueObject.RealTime.ParameterSnapshot",
			"port.Mixer.GlobalEffectsProcessor",
		]
		meta: rules: [
			"accept one independently rendered and already Patch-effect-processed stereo stem per active Patch, matched by PatchId and ParameterSnapshot index",
			"clear sixteen preallocated stereo track buffers, apply only each Patch's trim, and accumulate that stem into exactly its validated output MixerTrackId; multiple Patches targeting one track sum before any track control",
			"for each track apply its level and pan, measure one TrackMeter, then apply mute/solo audibility; mute always wins and when any solo is active only soloed non-muted tracks contribute",
			"feed reverb and delay only from post-fader, post-gate track output scaled by that track's sends, then sum audible dry tracks and both global returns and apply masterGainDb",
			"changing one track parameter changes every Patch contribution routed to that track and no contribution routed only to another track; changing one Patch route or trim changes only that Patch contribution",
			"a route change is compatible fixed snapshot data because all sixteen destinations and scratch buffers are already prepared; an invalid route is rejected before publication and never clamped, wrapped, dropped, or substituted",
			"MixEngine owns no Patch insert, effect slot, processor selection, effect chain, EQ, compression, chorus, distortion, limiter, or arbitrary auxiliary bus; the only current mixer-owned processors remain the shared reverb and delay",
			"all track, send, wet, and output scratch buffers are fixed-capacity and prepared before the audio callback",
			"the mix operation returns one MixObservation measured from the sixteen track buffers, mixer-owned reverb input, delay input, wet return, and final output; callers never inspect or borrow private buffers directly",
		]
		validations: [
			{id: "validation.service.mix_engine_global_mix", kind: "test", command: ["cargo", "test", "global_mix"], description: "shared-track accumulation, track isolation, mute/solo, post-gate sends, global effects, and master output are exact"},
			{id: "validation.service.mix_engine_faithful_effects", kind: "test", command: ["cargo", "test", "faithful_effects_nonzero_sends_and_baseline_restoration"], description: "effect observations use nonzero routed track sends, identical initial effect state, zero-input silence, and exact baseline restoration"},
			{id: "validation.service.mix_engine_observation", kind: "test", command: ["cargo", "test", "mix_observation"], description: "all sixteen pre-gate meters plus send, wet, final, finite, and clipping measurements come from their declared buffers"},
		]
		contributesTo: [
			{capability: "capability.sixteen_track_mixer_routing", contribution: "implements the fixed Patch-to-track-to-master signal path"},
			{capability: "capability.global_mix", contribution: "implements track sends through the current shared effects and master"},
			{capability: "capability.static_patch_effect", contribution: "consumes identity-preserving post-effect stems without owning or bypassing the upstream Patch processor"},
			{capability: "capability.realtime_execution", contribution: "routes and mixes through preallocated callback-owned buffers"},
			{capability: "capability.live_observable_demo", contribution: "measures the exact track and global stages needed by live checkpoints"},
		]
	}
}

project: adapters: GlobalReverbDelay: {
	implements: "port.Mixer.GlobalEffectsProcessor"
	layer: "infrastructure"
	profile: {kind: "in_process"}
	meta: rules: [
		"implement exactly one modest algorithmic stereo reverb and one stereo feedback delay",
		"allocate all delay lines and ring buffers only in prepare",
		"process has no locks, allocation, I/O, logging, or destruction",
	]
	contributesTo: [{capability: "capability.global_mix", contribution: "implements the one shared reverb and one shared delay"}]
}
