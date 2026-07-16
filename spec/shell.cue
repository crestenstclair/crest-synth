package crestsynth

project: contexts: Shell: {
	purpose: "keyboard/text and audio-device boundaries around the application services"

	ports: AppWindow: {
		direction: "outbound"
		contract: {
			run: "(onInput: FnMut(AppEvent), projection: Fn() -> TextProjection, onTick: FnMut(Duration)) -> Result<(), WindowError>"
		}
		consumes: ["valueObject.Control.AppEvent", "valueObject.Control.TextProjection"]
		invariants: [
			"the window receives immutable TextProjection and emits AppEvent",
			"the window owns raw key and K-modifier state but no synth parameter or selection state",
		]
		contributesTo: [{capability: "capability.one_way_parameter_control", contribution: "keeps the disposable text view outside application state"}]
	}

	ports: AudioOutput: {
		direction: "outbound"
		contract: {
			open: "(render: FnMut(&mut [f32], sampleRate: f32)) -> Result<AudioStream, AudioOutputError>"
		}
		invariants: ["the device callback forwards its exact caller-owned buffer to AudioRenderer", "device setup occurs before the callback starts"]
		contributesTo: [{capability: "capability.realtime_execution", contribution: "connects the hard real-time renderer to a stereo device"}]
	}

	applicationServices: StandaloneApplication: {
		purpose: "compose the control loop, automatic MIDI fixture, audio renderer, text window, and device output"
		uses: [
			"applicationService.Control.AppLoop",
			"applicationService.Testing.AutomaticMidiTest",
			"applicationService.RealTime.AudioRenderer",
			"port.Shell.AppWindow",
			"port.Shell.AudioOutput",
			"port.Synth.SoundFontEngine",
		]
		operations: {
			run: {input: {}, output: {result: "Result<(), ApplicationError>"}}
			runSmoke: {input: {degenerate: "Option<DegenerateMode>"}, output: {observation: "Result<SmokeObservation, ApplicationError>"}}
		}
		meta: rules: [
			"startup loads ./sf2/HiDef.sf2, prepares AudioRenderer, initializes AutomaticMidiTest, opens audio, then opens the text window",
			"normal-mode MIDI begins automatically during startup; each window tick advances only the private test input and collects deferred data",
			"keypress events are dispatched to AppLoop before a new TextProjection is requested",
			"runSmoke uses the same services without a physical device or window and measures real control, routing, and rendered-sample results",
		]
		contributesTo: [
			{capability: "capability.soundfont_audio", contribution: "composes the running SoundFont audio path"},
			{capability: "capability.automatic_test_midi", contribution: "starts the fixed test input automatically"},
			{capability: "capability.one_way_parameter_control", contribution: "joins keyboard events to the shared reducer and immutable text projection"},
			{capability: "capability.realtime_execution", contribution: "starts control and audio sides in the correct preparation order"},
		]
	}
}

project: adapters: EframeTextWindow: {
	implements: "port.Shell.AppWindow"
	layer: "infrastructure"
	profile: {kind: "ui", surfaces: ["keyboard", "single_text_view"]}
	meta: {
		framework: "eframe/egui"
		rules: [
			"render exactly one vertical scroll area containing TextProjection.body in a stock monospace text label",
			"keep selectedLine visible and add no panels, menus, columns, grids, tables, meters, faders, controls, widgets, custom painting, theme, or second screen",
			"bare W/S emit Navigate Up/Down; bare A/D emit Navigate Left/Right",
			"while K is held W/S/A/D emit Adjust Up/Down/Left/Right; releasing K ends modifier state",
			"key handling emits AppEvents only and never mutates view selection, Patch values, AppState, snapshots, or audio",
		]
	}
	contributesTo: [{capability: "capability.one_way_parameter_control", contribution: "implements the single text screen and exact keyboard vocabulary"}]
}

project: adapters: CpalAudioOutput: {
	implements: "port.Shell.AudioOutput"
	layer: "infrastructure"
	profile: {kind: "device_output", medium: "stereo-pcm"}
	meta: {
		framework: "cpal"
		rules: [
			"select and open the default stereo output on the control thread",
			"the callback directly forwards the device buffer to AudioRenderer and performs no pacing, allocation, locking, I/O, logging, format construction, or deallocation",
			"convert sample formats in place with bounded arithmetic when the device is not f32",
		]
	}
	contributesTo: [{capability: "capability.realtime_execution", contribution: "implements the physical low-latency stereo output"}]
}
