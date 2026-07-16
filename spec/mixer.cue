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
		]
		contributesTo: [{capability: "capability.global_mix", contribution: "defines the complete replaceable boundary for the two global effects"}]
	}

	domainServices: MixEngine: {
		purpose: "combine Patch output and process the two global effect returns"
		uses: [
			"valueObject.Mixer.ChannelParameters",
			"valueObject.Mixer.GlobalParameters",
			"port.Mixer.GlobalEffectsProcessor",
		]
		meta: rules: [
			"for each Patch apply gain and pan, add its reverbSend to one preallocated reverb input, add its delaySend to one preallocated delay input, then sum both global returns and apply masterGainDb",
			"there are no inserts, per-channel effects, effect slots, effect chains, auxiliary buses, EQ, compression, chorus, distortion, or limiter",
			"all scratch buffers are fixed-capacity and prepared before the audio callback",
		]
		validations: [{kind: "test", command: ["cargo", "test", "global_mix"], description: "gain, pan, both sends, both returns, and master gain affect only their declared signal path"}]
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
