package crestsynth

project: contexts: Mixer: {
	purpose: "mix post-effect Patch stems with per-Patch level/pan/sends followed by one shared reverb and one shared delay"

	valueObjects: ChannelParameters: {
		description: "all editable parameters owned by one Patch"
		state: {
			gainDb: "f32"
			pan: "f32"
			reverbSend: "f32"
			delaySend: "f32"
		}
		invariants: [
			"gainDb is in -60.0..=6.0",
			"pan is in -1.0..=1.0",
			"reverbSend and delaySend are in 0.0..=1.0",
			"values are finite",
			"a production-owned typed surface descriptor enumerates these four fields, bounds, and fine/coarse steps; DemoScene consumes it instead of duplicating parameter-name strings",
			"the descriptor contains each field exactly once before set conversion and is the independent pre-dispatch oracle for bounds and step sizes",
		]
		contributesTo: [
			{capability: "capability.global_mix", contribution: "provides the complete editable per-Patch mix surface"},
			{capability: "capability.one_way_parameter_control", contribution: "provides the bounded values edited through AppState"},
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
			{capability: "capability.one_way_parameter_control", contribution: "completes the parameter list after the Patch sections"},
		]
	}

	valueObjects: MixObservation: {
		description: "fixed-size callback-local measurements produced by MixEngine from its owned dry, send, wet, and final-output buffers"
		state: {
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
			"the value is Copy, fixed-size, numeric, and created from the exact callback buffers already owned by MixEngine without allocation, logging, formatting, locking, blocking, or I/O",
			"reverbInputRms and delayInputRms measure only their declared Patch-send sums; wetOutputRms measures only the shared effect return before final master gain",
			"peak and RMS fields are finite and nonnegative; non-finite or clipped output increments its bounded counter instead of panicking or hiding the condition",
			"the observation never feeds back into mixing decisions and is not a second owner of audio or parameter state",
		]
		contributesTo: [
			{capability: "capability.live_observable_demo", contribution: "provides causal send, wet, and output measurements from the mixer-owned buffers"},
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
			"the implementation contains exactly one mixer-owned reverb and one mixer-owned delay shared by every Patch; the upstream PreparedPostEffectRack is not part of this port",
			"prepare allocates all effect storage and process is allocation-free and lock-free",
			"production and verification implementations derive wet excitation only from reverbInput and delayInput; dry output is never treated as an implicit send, and zero inputs cannot create a wet return",
		]
		contributesTo: [{capability: "capability.global_mix", contribution: "defines the complete replaceable boundary for the two global effects"}]
	}

	domainServices: MixEngine: {
		purpose: "combine already post-effect-processed Patch output and process the two global effect returns"
		uses: [
			"valueObject.Mixer.ChannelParameters",
			"valueObject.Mixer.GlobalParameters",
			"valueObject.Mixer.MixObservation",
			"valueObject.RealTime.PatchAudioBlock",
			"valueObject.RealTime.ParameterSnapshot",
			"port.Mixer.GlobalEffectsProcessor",
		]
		meta: rules: [
			"accept one independently rendered and already Patch-effect-processed stereo stem per active Patch, matched by PatchId and ParameterSnapshot index",
			"for each Patch apply only that Patch's gain and pan to its stem, add only that stem scaled by reverbSend to one preallocated reverb input, add only that stem scaled by delaySend to one preallocated delay input, then sum dry audio and both global returns and apply masterGainDb",
			"changing Patch N gain, pan, reverbSend, or delaySend must not change any other Patch's dry contribution or send contribution",
			"reject or silence a missing or mismatched stem; never substitute a combined master stream or the first Patch's parameters",
			"MixEngine owns no Patch insert, effect slot, processor selection, effect chain, EQ, compression, chorus, distortion, limiter, or arbitrary auxiliary bus; the only mixer-owned processors remain the shared reverb and delay",
			"all scratch buffers are fixed-capacity and prepared before the audio callback",
			"the mix operation returns one MixObservation measured from the mixer-owned reverb input, delay input, wet return, and final output buffers; callers never inspect or borrow those private buffers directly",
			"behavioral tests establish measured nonzero reverb and delay inputs through Patch sends before comparing wet controls and render paired cases from identical reset effect state so unrelated tail evolution cannot satisfy a predicate",
			"paired sensitivity cases independently vary reverbRoomSize, reverbDamping, reverbReturn, delayMilliseconds, delayFeedback, and delayReturn through adapter.GlobalReverbDelay with nonzero routed input, identical reset state, and exact send restoration",
		]
		validations: [
			{id: "validation.service.mix_engine_global_mix", kind: "test", command: ["cargo", "test", "global_mix"], description: "with two distinct simultaneous stems, editing either Patch's gain, pan, or sends changes only that Patch's declared dry/send path while global controls affect the complete mix"},
			{id: "validation.service.mix_engine_faithful_effects", kind: "test", command: ["cargo", "test", "faithful_effects_nonzero_sends_and_baseline_restoration"], description: "effect observations use nonzero routed sends, identical initial effect state, zero-input silence, and exact parameter/send baseline restoration"},
			{id: "validation.service.mix_engine_effect_sensitivity", kind: "test", command: ["cargo", "test", "global_effects_parameter_sensitivity"], description: "each reverb and delay parameter causes its own measured response from nonzero routed input using paired identical effect state"},
			{id: "validation.service.mix_engine_observation", kind: "test", command: ["cargo", "test", "mix_observation"], description: "dry, reverb-input, delay-input, wet-return, final-output, finite, and clipping measurements come from their declared mixer-owned buffers without changing output"},
		]
		contributesTo: [
			{capability: "capability.global_mix", contribution: "implements the complete channel-to-global-effects-to-master signal path"},
			{capability: "capability.static_patch_effect", contribution: "consumes identity-preserving post-effect stems without owning or bypassing the upstream Patch processor"},
			{capability: "capability.realtime_execution", contribution: "mixes through preallocated callback-owned buffers"},
			{capability: "capability.live_observable_demo", contribution: "measures the exact mixer-owned signal stages needed by live checkpoints"},
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
