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
			"MAX_PATCHES is a compile-time bound",
			"unused entries are inactive",
			"the snapshot is fully owned, fixed-size, and readable without allocation",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "carries accepted AppState values to audio"},
			{capability: "capability.realtime_execution", contribution: "provides a fixed-size latest-wins callback input"},
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
		contributesTo: [
			{capability: "capability.soundfont_audio", contribution: "targets MIDI at the configured Patch"},
			{capability: "capability.realtime_execution", contribution: "is safe to copy through the event ring"},
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

	applicationServices: AudioRenderer: {
		purpose: "consume ready commands and the newest parameters, render SoundFont patches, then mix the global effects"
		uses: [
			"port.RealTime.AudioBoundary",
			"port.Synth.SoundFontEngine",
			"domainService.Mixer.MixEngine",
		]
		operations: {
			prepare: {input: {sampleRate: "f32", maxFrames: "usize"}, output: {result: "Result<(), AudioError>"}}
			render: {input: {interleavedStereo: "&mut [f32]"}, output: {}}
		}
		meta: rules: [
			"prepare all engine, mixer, effect, and scratch storage on the control thread",
			"render drains only currently available AudioCommands, reads one latest ParameterSnapshot, renders Patch audio, applies MixEngine, and returns",
			"render never allocates, locks, blocks, performs I/O, logs, formats strings, grows a collection, or destroys owned state",
		]
		validations: [{kind: "test", command: ["cargo", "test", "audio_renderer_realtime_contract"], description: "an instrumented callback consumes commands and latest parameters with zero callback allocations"}]
		contributesTo: [
			{capability: "capability.soundfont_audio", contribution: "joins the SoundFont and global mixer into the callback"},
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
