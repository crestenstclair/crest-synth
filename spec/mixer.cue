package crestsynth

project: contexts: Mixer: {
	purpose: "per-Patch level and pan followed by one shared reverb and one shared delay"

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

	ports: GlobalEffectsProcessor: {
		direction: "outbound"
		contract: {
			prepare: "(sampleRate: f32, maxFrames: usize, maxDelayMilliseconds: f32) -> Result<(), EffectError>"
			process: "(reverbInput: &[f32], delayInput: &[f32], output: &mut [f32], parameters: &GlobalParameters)"
		}
		invariants: [
			"the implementation contains exactly one reverb and one delay shared by every Patch",
			"prepare allocates all effect storage and process is allocation-free and lock-free",
			"production and verification implementations derive wet excitation only from reverbInput and delayInput; dry output is never treated as an implicit send, and zero inputs cannot create a wet return",
		]
		contributesTo: [{capability: "capability.global_mix", contribution: "defines the complete replaceable boundary for the two global effects"}]
	}

	domainServices: MixEngine: {
		purpose: "combine Patch output and process the two global effect returns"
		uses: [
			"valueObject.Mixer.ChannelParameters",
			"valueObject.Mixer.GlobalParameters",
			"valueObject.RealTime.PatchAudioBlock",
			"port.Mixer.GlobalEffectsProcessor",
		]
		meta: rules: [
			"accept one independently rendered stereo stem per active Patch, matched by PatchId and ParameterSnapshot index",
			"for each Patch apply only that Patch's gain and pan to its stem, add only that stem scaled by reverbSend to one preallocated reverb input, add only that stem scaled by delaySend to one preallocated delay input, then sum dry audio and both global returns and apply masterGainDb",
			"changing Patch N gain, pan, reverbSend, or delaySend must not change any other Patch's dry contribution or send contribution",
			"reject or silence a missing or mismatched stem; never substitute a combined master stream or the first Patch's parameters",
			"there are no inserts, per-channel effects, effect slots, effect chains, auxiliary buses, EQ, compression, chorus, distortion, or limiter",
			"all scratch buffers are fixed-capacity and prepared before the audio callback",
			"behavioral tests establish measured nonzero reverb and delay inputs through Patch sends before comparing wet controls and render paired cases from identical reset effect state so unrelated tail evolution cannot satisfy a predicate",
			"paired sensitivity cases independently vary reverbRoomSize, reverbDamping, reverbReturn, delayMilliseconds, delayFeedback, and delayReturn through adapter.GlobalReverbDelay with nonzero routed input, identical reset state, and exact send restoration",
		]
		validations: [
			{kind: "test", command: ["cargo", "test", "global_mix"], description: "with two distinct simultaneous stems, editing either Patch's gain, pan, or sends changes only that Patch's declared dry/send path while global controls affect the complete mix"},
			{kind: "test", command: ["cargo", "test", "faithful_effects_nonzero_sends_and_baseline_restoration"], description: "effect observations use nonzero routed sends, identical initial effect state, zero-input silence, and exact parameter/send baseline restoration"},
			{kind: "test", command: ["cargo", "test", "global_effects_parameter_sensitivity"], description: "each reverb and delay parameter causes its own measured response from nonzero routed input using paired identical effect state"},
		]
		contributesTo: [
			{capability: "capability.global_mix", contribution: "implements the complete channel-to-global-effects-to-master signal path"},
			{capability: "capability.realtime_execution", contribution: "mixes through preallocated callback-owned buffers"},
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
