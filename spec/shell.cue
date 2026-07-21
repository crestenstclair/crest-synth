package crestsynth

project: contexts: Shell: {
	purpose: "keyboard/text and audio-device boundaries around the application services"

	valueObjects: WindowInput: {
		description: "a normalized window-boundary input used by both eframe and deterministic GUI scenes"
		state: {
			kind: "KeyDown | KeyUp | FocusLost"
			key: "W | S | A | D | K | Other"
			surfaceDescriptor: "typed exhaustive descriptors for every valid normalized kind/key combination, including key-down, key-up, focus loss, and unrelated input"
		}
		invariants: [
			"platform key codes are normalized at the eframe boundary before translation",
			"WindowInput is shell data and never enters AppState or the audio boundary",
			"the deterministic demo feeds the same values to the same translator as the real window",
			"surfaceDescriptor is defined beside WindowInput and is the only GUI-input vocabulary consumed by DemoScene and acceptance tests; no test owns a second list of W/S/A/D/K, key-up, focus-loss, or unrelated-input strings",
			"surfaceDescriptor contains exactly 13 unique valid values before any set conversion: KeyDown and KeyUp for W, S, A, D, K, and Other plus FocusLost with no key payload",
		]
		contributesTo: [{capability: "capability.observable_demo_scene", contribution: "lets automated scenes exercise the actual current GUI input vocabulary"}]
	}

	applicationServices: KeyboardInputTranslator: {
		purpose: "translate normalized W/S/A/D/K window input and focus changes into the closed AppEvent vocabulary"
		uses: [
			"valueObject.Shell.WindowInput",
			"valueObject.Control.AppEvent",
		]
		operations: {
			translate: {input: {event: "WindowInput"}, output: {event: "Option<AppEvent>"}}
		}
		meta: rules: [
			"bare W/S/A/D key-down emits Navigate Up/Down/Left/Right and key-up emits nothing",
			"K key-down enters modifier state; while held W/S/A/D key-down emits Adjust Up/Down/Left/Right; K key-up exits modifier state",
			"FocusLost clears modifier state and emits nothing",
			"Other input emits nothing",
			"translation owns no Patch, selection, parameter, projection, or audio state",
		]
		validations: [{kind: "test", command: ["cargo", "test", "keyboard_input_translator"], description: "every WindowInput and direction mapping, K transition, focus loss, key release, and unrelated key is deterministic"}]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "keeps physical keyboard normalization outside the reducer"},
			{capability: "capability.observable_demo_scene", contribution: "is the single translator shared by the eframe window and exhaustive scene runner"},
		]
	}

	ports: AppWindow: {
		direction: "outbound"
		contract: {
			run: "(onInput: FnMut(AppEvent), projection: Fn() -> TextProjection, onTick: FnMut(Duration)) -> Result<(), WindowError>"
		}
		consumes: ["valueObject.Control.AppEvent", "valueObject.Control.TextProjection"]
		invariants: [
			"the window receives immutable TextProjection and emits AppEvent",
			"the window owns raw key and K-modifier state but no synth parameter or selection state",
			"an AppEvent rejection does not close the window or disable later input",
			"each interactive frame advances the injected control-side tick and then requests the current immutable TextProjection; autonomous live-demo state is never stored or applied by the window",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "keeps the disposable text view outside application state"},
			{capability: "capability.live_observable_demo", contribution: "keeps the final canonical projection visible while the control-side live runner advances and after it finishes"},
		]
	}

	ports: AudioOutput: {
		direction: "outbound"
		contract: {
			open: "(render: FnMut(&mut [f32], sampleRate: f32)) -> Result<AudioStream, AudioOutputError>"
		}
		invariants: [
			"AudioRenderer always receives interleaved stereo f32 frames; a native stereo f32 device may forward its exact caller-owned buffer",
			"a device with more than two channels uses bounded preallocated callback storage, writes stereo to its first two output channels, and silences every surplus channel",
			"device setup occurs before the callback starts",
		]
		contributesTo: [{capability: "capability.realtime_execution", contribution: "connects the hard real-time renderer to a stereo device"}]
	}

	applicationServices: StandaloneApplication: {
		purpose: "compose the control loop, automatic MIDI fixture, exhaustive GUI demo, audio renderer, text window, and device output"
		uses: [
			"valueObject.Synth.CapabilityRegistry",
			"port.Synth.InstrumentCapabilityProvider",
			"applicationService.Control.AppLoop",
			"applicationService.Testing.AutomaticMidiTest",
			"applicationService.Testing.ExhaustiveGuiDemo",
			"applicationService.Testing.LiveDemoRunner",
			"valueObject.Testing.LiveDemoScene",
			"valueObject.Testing.LiveDemoCheckpoint",
			"valueObject.Testing.LiveDemoReport",
			"applicationService.RealTime.AudioRenderer",
			"port.RealTime.AudioObservation",
			"port.Shell.AppWindow",
			"port.Shell.AudioOutput",
			"port.Synth.SoundFontEngine",
		]
		operations: {
			run: {input: {}, output: {result: "Result<(), ApplicationError>"}}
			runLiveDemo: {input: {onCheckpoint: "FnMut(&LiveDemoCheckpoint)", onComplete: "FnOnce(&LiveDemoReport)"}, output: {result: "Result<(), ApplicationError>"}}
			runSmoke: {input: {degenerate: "Option<DegenerateMode>"}, output: {observation: "Result<SmokeObservation, ApplicationError>"}}
			runDemoScene: {input: {degenerate: "Option<DegenerateMode>"}, output: {result: "Result<DemoSceneReport, ApplicationError>"}}
		}
		meta: rules: [
			"startup obtains the installed CapabilityDescriptor from exactly one InstrumentCapabilityProvider, validates and freezes the CapabilityRegistry in AppState, constructs exactly one SoundFontEngine, loads ./sf2/HiDef.sf2 into it, prepares AudioRenderer, initializes AutomaticMidiTest, opens audio, then opens the text window",
			"startup fails visibly on duplicate, missing, unknown, or mismatched capability registration and never chooses or substitutes a fallback provider, config, asset, preset, or engine",
			"normal-mode MIDI begins automatically during startup; each window tick advances only the private test input and collects deferred data",
			"runLiveDemo uses the exact normal startup order, real EframeTextWindow, physical CpalAudioOutput, HiDef.sf2, and Corridors of Time fixture, then starts LiveDemoRunner on the control side and advances it from the existing window-tick callback",
			"runLiveDemo injects the same AppLoop into keyboard input, AutomaticMidiTest, LiveDemoRunner, and immutable projection callbacks; none receives mutable AppState and no live-specific reducer, engine, mixer, window, or audio-output implementation exists",
			"in live mode only LiveDemoRunner advances AutomaticMidiTest; StandaloneApplication does not also tick the fixture and every due fixture event is dispatched exactly once",
			"the live EventLog is pre-sized to a declared bounded capacity sufficient for the frozen scene and fixture events; dropped records make the final report incomplete rather than being hidden",
			"keypress events are dispatched to AppLoop before a new TextProjection is requested",
			"ParameterAtBoundary and every other EventRejection caused by ordinary user input are nonfatal no-ops: keep the window and audio running, preserve the current projection, and accept the next key event",
			"run returns ApplicationError only for startup, adapter, audio-device, window-runtime, automatic-input, or real-time boundary failures; it never promotes a rejected user edit to an application failure",
			"runSmoke uses the same services without a physical device or window and measures real control, routing, and rendered-sample results",
			"runDemoScene initializes the real fixture, then drives normalized WindowInputs through KeyboardInputTranslator and the production AppLoop; it never mutates state or projections directly",
			"demo execution retains the complete EventLog and final StateTree and reports zero missing coverage before returning success",
			"verification-only demo degeneracy is injected before observation at exactly one real seam: control drops one translated Adjust before AppLoop, audio clears the rendered buffer after AudioRenderer; neither mode edits coverage, a completed DemoSceneReport, or any measured observation field",
			"each checkpoint is passed exactly once to the injected control-side onCheckpoint callback; when LiveDemoRunner completes, pass its report exactly once to onComplete, stop advancing it, keep servicing keyboard input and canonical projection frames, and keep the audio/window lifetime under the user's close action",
			"if the user closes the window before completion, dispatch semantic all-notes-off for installed Patches while the control loop is available, do not call onComplete with a successful report, and return a typed incomplete-live-demo result without fabricating coverage",
			"--demo-live has no degenerate, headless, no-device, no-window, auto-close, or silent-fallback mode; startup or runtime device failures remain typed visible ApplicationErrors",
		]
		validations: [
			{kind: "integration", command: ["cargo", "test", "standalone_application"], description: "a boundary adjustment on a non-first Patch is ignored without terminating the window, and a following valid edit is accepted"},
			{kind: "integration", command: ["cargo", "test", "standalone_exhaustive_gui_demo"], description: "the composed headless application emits a complete event log, state tree, coverage matrix, and audio observations"},
			{kind: "integration", command: ["cargo", "test", "standalone_live_demo_composition"], description: "the live mode wires the real window/audio lifetime to the paced runner while a deterministic harness proves final output and no auto-close"},
		]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "constructs the immutable descriptor registry and injects the provider into the fixture installation path"},
			{capability: "capability.soundfont_audio", contribution: "composes the running SoundFont audio path"},
			{capability: "capability.automatic_test_midi", contribution: "starts the fixed test input automatically"},
			{capability: "capability.one_way_parameter_control", contribution: "joins keyboard events to the shared reducer and immutable text projection"},
			{capability: "capability.realtime_execution", contribution: "starts control and audio sides in the correct preparation order"},
			{capability: "capability.observable_demo_scene", contribution: "composes the exhaustive scene against production input, reducer, projection, boundary, engine, and mixer services"},
			{capability: "capability.live_observable_demo", contribution: "composes the paced runner with the real standalone UI, physical audio stream, canonical control loop, and final visible state"},
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
				"normalize egui key presses, releases, and focus loss into WindowInput and delegate every W/S/A/D/K decision to KeyboardInputTranslator",
				"do not retain a second private keyboard state machine or duplicate the translator in tests",
				"key handling emits AppEvents only and never mutates view selection, Patch values, AppState, snapshots, event logs, or audio",
				"the headless adapter contract drives real egui RawInput through an egui Context and EframeApplication.update with the production on_input callback wired to AppLoop.dispatch, then runs the next frame from AppLoop.currentText; capturing a callback without applying it is not acceptance evidence",
				"the next frame must prove the event-log record, accepted generation, selected parameter value, every unrelated value, TextProjection body/stateHash/selectedLine, and selected-line scroll target all reflect that same dispatched GUI event",
				"the headless adapter contract begins with a projection containing discriminating values for every Patch and global parameter and inspects egui output for the exact values; calling normalize_egui_event directly or rendering an unrelated supplied projection is not sufficient integration evidence",
				"in --demo-live, invoke the injected tick without blocking, then render only the newest AppLoop.currentText projection; the adapter never receives LiveDemoScene, LiveDemoReport, AudioObservationSnapshot, or mutable state",
				"schedule the next idle repaint after 16 ms instead of requesting an immediate perpetual repaint; native input and window events may wake the event loop sooner",
				"after live completion continue requesting and painting the final canonical projection until the user closes the native window; completion never sends a viewport-close command",
			]
		}
		validations: [
			{kind: "test", command: ["cargo", "test", "eframe_text_window"], description: "the adapter normalizes the complete egui input vocabulary through the shared translator"},
			{kind: "integration", command: ["cargo", "test", "--test", "eframe_context", "--", "--nocapture"], assertions: [{kind: "exit_code", expected: 0}, {kind: "stdout_contains", pattern: "CREST_ACCEPTANCE eframe_context passed"}], description: "a headless egui Context executes EframeApplication.update, invokes the real callback, renders exact projection values, and targets the exact selected line without a native window"},
		]
	contributesTo: [
		{capability: "capability.one_way_parameter_control", contribution: "implements the single text screen and exact keyboard vocabulary"},
		{capability: "capability.observable_demo_scene", contribution: "shares its production input translator with the deterministic scene"},
		{capability: "capability.live_observable_demo", contribution: "renders each accepted live generation and preserves the completed canonical frame"},
	]
}

project: adapters: CpalAudioOutput: {
	implements: "port.Shell.AudioOutput"
	layer: "infrastructure"
	profile: {kind: "device_output", medium: "stereo-pcm"}
	meta: {
		framework: "cpal"
		rules: [
			"select and open the default PCM output with at least two channels on the control thread",
			"use a direct callback fast path for native stereo f32; otherwise render into fixed-capacity stereo scratch storage, map left and right to device channels 1 and 2, and write silence to surplus device channels",
			"the callback performs no pacing, allocation, locking, I/O, logging, format construction, or deallocation",
			"convert sample formats with bounded arithmetic when the device is not f32",
		]
	}
	contributesTo: [
		{capability: "capability.realtime_execution", contribution: "implements the physical low-latency stereo output"},
		{capability: "capability.live_observable_demo", contribution: "keeps the real device callback running while the visible scene advances"},
	]
}
