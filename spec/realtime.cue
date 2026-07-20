package crestsynth

project: contexts: RealTime: {
	purpose: "fixed-capacity values and lock-free ports crossing into the audio callback"

	valueObjects: ParameterSnapshot: {
		description: "the newest complete control state required for rendering"
		state: {
			generation: "u64"
			patchCount: "usize"
			patches: "[RtPatchParameters; MAX_PATCHES]"
			global: "GlobalParameters"
		}
		invariants: [
			"MAX_PATCHES equals the 16 unique MIDI channels available to the SoundFont adapter",
			"unused entries are inactive",
			"the snapshot is fully owned, fixed-size, and readable without allocation",
			"a production-owned typed leaf descriptor covers generation, patchCount, every active PatchId/channel parameter, and every global parameter and exactly matches the StateTree parameters projection",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "carries accepted AppState values to audio"},
			{capability: "capability.realtime_execution", contribution: "provides a fixed-size latest-wins callback input"},
		]
	}

	valueObjects: PatchAudioBlock: {
		description: "caller-owned prepared stereo stems that preserve Patch identity between SoundFont rendering and mixing"
		state: {
			patchCount: "usize"
			frameCount: "usize"
			stems: "[PatchStereoStem; MAX_PATCHES]"
		}
		invariants: [
			"capacity for MAX_PATCHES and maxFrames is allocated only by AudioRenderer.prepare",
			"each active stem is keyed by the same PatchId and index as ParameterSnapshot.patches",
			"one stem contains only audio produced by that Patch's assigned SoundFont lane",
			"clearing, filling, and reading active frames are allocation-free",
		]
		contributesTo: [
			{capability: "capability.soundfont_audio", contribution: "preserves Patch identity after synthesis instead of collapsing all voices to one master stream"},
			{capability: "capability.global_mix", contribution: "gives MixEngine an independently controllable signal for every Patch"},
			{capability: "capability.realtime_execution", contribution: "provides prepared callback-owned synthesis scratch storage"},
		]
	}

	valueObjects: AudioCommand: {
		description: "a bounded discrete command for the audio callback"
		state: {
			kind: "PatchMidi | AllNotesOff"
			patchId: "Option<PatchId>"
			message: "Option<MidiMessage>"
		}
		invariants: ["contains no heap-owned data", "PatchMidi contains both patchId and message"]
		meta: rules: [
			"PatchMidi carrying MidiMessageKind::AllNotesOff and the separate AudioCommand::AllNotesOff variant are distinct typed cases and both are asserted; coverage for one never credits the other",
		]
		contributesTo: [
			{capability: "capability.soundfont_audio", contribution: "targets MIDI at the configured Patch"},
			{capability: "capability.realtime_execution", contribution: "is safe to copy through the event ring"},
		]
	}

	valueObjects: AudioObservationSnapshot: {
		description: "a fixed-size latest-value observation of audio work completed by the real-time callback"
		state: {
			sequence: "u64"
			renderedBlocks: "u64"
			renderedFrames: "u64"
			parameterGeneration: "u64"
			commandsConsumed: "u64"
			activeNotes: "u32"
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
			"the snapshot is Copy, fixed-size, numeric, and contains no Vec, String, path, reference, mutex, decoder, allocation, or destructible owner",
			"sequence and renderedBlocks increase monotonically and parameterGeneration is the exact ParameterSnapshot generation used for the measured block",
			"peak, RMS, wet-input, and wet-output fields copy the MixObservation produced from the actual mixer-owned buffers for that observation window",
			"activeNotes is maintained by a prepared fixed-capacity Patch/channel/note bitset updated only when the callback dispatches the corresponding MIDI lifecycle command; Patch-targeted or global all-notes-off clears it with bounded work",
			"the callback updates nonFiniteSamples and clippedSamples instead of logging, formatting, panicking, or performing I/O",
		]
		contributesTo: [
			{capability: "capability.live_observable_demo", contribution: "correlates visible accepted generations with measured physical audio work"},
			{capability: "capability.realtime_execution", contribution: "keeps callback diagnostics bounded and allocation-free"},
		]
	}

	ports: AudioBoundary: {
		direction: "outbound"
		contract: {
			pushCommand: "(AudioCommand) -> Result<(), BoundaryFull>"
			publishParameters: "(ParameterSnapshot)"
			popCommand: "() -> Option<AudioCommand>"
			readLatestParameters: "() -> ParameterSnapshot"
			retire: "(RetiredAudioState)"
			collect: "()"
		}
		consumes: ["valueObject.RealTime.AudioCommand", "valueObject.RealTime.ParameterSnapshot"]
		invariants: [
			"commands use a bounded single-producer single-consumer queue",
			"parameters are latest-wins and the consumer sees one complete snapshot",
			"retire never destroys on the audio thread and collect runs only on the control thread",
			"all operations used by the audio thread are allocation-free, lock-free, and non-blocking",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "publishes accepted parameters through the single real-time seam"},
			{capability: "capability.realtime_execution", contribution: "owns command, parameter, and deferred-destruction transfer semantics"},
		]
	}

	ports: AudioObservation: {
		direction: "outbound"
		contract: {
			publishFromCallback: "(AudioObservationSnapshot)"
			readLatestOnControl: "() -> AudioObservationSnapshot"
		}
		consumes: ["valueObject.RealTime.AudioObservationSnapshot"]
		invariants: [
			"publishFromCallback is bounded, lock-free, non-blocking, allocation-free, and cannot log or destroy owned state",
			"readLatestOnControl returns one coherent complete snapshot and never executes on the audio callback",
			"observation delivery is a dedicated latest-value transport and never shares the discrete AudioCommand ring or ParameterSnapshot publication storage",
			"a slow UI may skip intermediate observations; it cannot slow or backpressure the callback",
		]
		contributesTo: [
			{capability: "capability.live_observable_demo", contribution: "lets the control-side live runner read bounded audio consequences without callback logging"},
			{capability: "capability.realtime_execution", contribution: "separates callback-to-control observations from command and parameter transports"},
		]
	}

	applicationServices: AudioRenderer: {
		purpose: "consume ready commands and the newest parameters, render SoundFont patches, then mix the global effects"
		uses: [
			"port.RealTime.AudioBoundary",
			"port.RealTime.AudioObservation",
			"port.Synth.SoundFontEngine",
			"domainService.Mixer.MixEngine",
			"valueObject.Mixer.MixObservation",
		]
		operations: {
			prepare: {input: {sampleRate: "f32", maxFrames: "usize"}, output: {result: "Result<(), AudioError>"}}
			render: {input: {interleavedStereo: "&mut [f32]"}, output: {}}
		}
		meta: rules: [
			"prepare all engine, mixer, effect, and scratch storage on the control thread",
			"render drains only currently available AudioCommands, reads one latest ParameterSnapshot, asks SoundFontEngine to fill one PatchAudioBlock stem per active Patch, passes all matching stems and parameters to MixEngine, and returns",
			"PatchId and Patch index remain aligned from AudioCommand through the synthesis stem and ChannelParameters; a combined engine master buffer must never be treated as one Patch's input",
			"after rendering each block, combine MixEngine's MixObservation with bounded command and active-note counters and publish one AudioObservationSnapshot tagged with the consumed ParameterSnapshot generation",
			"the active-note observer is prepared outside the callback, has explicit Patch, channel, and note bounds, saturates counters on overflow, and never controls or substitutes SoundFont engine state",
			"audio observations never change rendering, synth state, mix state, event coverage, or acceptance results and never call a control-side serializer or logger",
			"render never allocates, locks, blocks, performs I/O, logs, formats strings, grows a collection, or destroys owned state",
		]
		validations: [
			{kind: "test", command: ["cargo", "test", "audio_renderer_realtime_contract"], description: "an instrumented callback consumes commands and latest parameters with zero callback allocations and preserves two simultaneous Patch stems into the mixer"},
			{kind: "test", command: ["cargo", "test", "audio_observation_realtime_contract"], description: "the callback publishes coherent generation-tagged numeric observations with zero allocation, locking, blocking, logging, or callback-owned destruction"},
		]
		contributesTo: [
			{capability: "capability.soundfont_audio", contribution: "joins the SoundFont and global mixer into the callback"},
			{capability: "capability.live_observable_demo", contribution: "publishes measured callback consequences for the live scene without changing the render path"},
			{capability: "capability.realtime_execution", contribution: "owns the hard real-time render operation"},
		]
	}
}

project: adapters: LockFreeAudioBoundary: {
	implements: "port.RealTime.AudioBoundary"
	layer: "infrastructure"
	meta: {
		framework: "rtrb + triple_buffer + basedrop"
		rules: [
			"use rtrb for AudioCommand, triple_buffer for ParameterSnapshot, and basedrop for retired engine data",
			"keep the control and audio handles separate so callback code cannot call control-only operations",
		]
	}
	contributesTo: [
		{capability: "capability.one_way_parameter_control", contribution: "publishes the latest accepted parameters"},
		{capability: "capability.realtime_execution", contribution: "implements the complete lock-free control/audio boundary"},
	]
}

project: adapters: AtomicAudioObservation: {
	implements: "port.RealTime.AudioObservation"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "latest-value-atomics"}
	meta: {
		rules: [
			"store all floating measurements as their f32 bit patterns and use a sequence protocol or equivalent bounded atomic publication so the control side never combines fields from different snapshots",
			"the audio-side handle can only publish and the control-side handle can only read; neither handle exposes serialization, formatting, reset, wait, or blocking APIs",
			"publishing overwrites the previous observation without allocation or backpressure and never touches AudioCommand or ParameterSnapshot storage",
		]
	}
	validations: [{kind: "test", command: ["cargo", "test", "atomic_audio_observation"], description: "publication and reads are coherent, latest-wins, monotonic, and allocation-free on the callback side"}]
	contributesTo: [
		{capability: "capability.live_observable_demo", contribution: "implements the bounded callback-to-control observation seam used by live checkpoints"},
		{capability: "capability.realtime_execution", contribution: "keeps meters and health data out of the event and parameter transports"},
	]
}
