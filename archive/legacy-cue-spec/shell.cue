package crestsynth

project: contexts: Shell: {
	purpose: "keyboard/graphical-window and audio-device boundaries around the application services"

	valueObjects: WindowInput: {
		description: "a normalized window-boundary input used by both eframe and deterministic GUI scenes"
		state: {
			kind: "KeyDown | KeyUp | FocusLost"
			key: "Digit1 | Digit2 | W | S | A | D | K | Other"
			surfaceDescriptor: "typed exhaustive descriptors for every valid normalized kind/key combination, including key-down, key-up, focus loss, and unrelated input"
		}
		invariants: [
			"platform key codes are normalized at the eframe boundary before translation",
			"WindowInput is shell data and never enters AppState or the audio boundary",
			"the deterministic demo feeds the same values to the same translator as the real window",
			"surfaceDescriptor is defined beside WindowInput and is the only GUI-input vocabulary consumed by DemoScene and acceptance tests; no test owns a second list of Digit1/Digit2/W/S/A/D/K, key-up, focus-loss, or unrelated-input strings",
			"surfaceDescriptor contains exactly 17 unique valid values before any set conversion: KeyDown and KeyUp for Digit1, Digit2, W, S, A, D, K, and Other plus FocusLost with no key payload",
		]
		contributesTo: [
			{capability: "capability.observable_demo_scene", contribution: "lets automated scenes exercise the actual current GUI input vocabulary"},
			{capability: "capability.schema_driven_patch_page", contribution: "normalizes the two direct page bindings before semantic translation"},
		]
	}

	valueObjects: AudioDeviceConfig: {
		description: "validated physical-device facts selected before any graph is prepared or stream callback starts"
		state: {
			sampleRate: "f32"
			channels: "u16"
			sampleFormat: "AudioSampleFormat"
			channelMapping: "StereoToFirstTwo"
			renderCapacityFrames: "usize"
		}
		invariants: [
			"sampleRate is finite and positive, channels is at least two, the sample format is supported PCM, and renderCapacityFrames is nonzero",
			"the value is returned by negotiation before PreparedGraphBuilder prepares engines, effects, stems, or scratch",
			"renderCapacityFrames is the maximum bounded block submitted to PreparedGraph even when a native device callback is larger",
		]
		contributesTo: [{capability: "capability.realtime_execution", contribution: "binds complete graph preparation to the accepted physical device configuration"}]
	}

	valueObjects: AudioDeviceRuntimeError: {
		description: "fixed-size post-start device failure transferred from callback to control ownership"
		state: kind: "DeviceBusy | DeviceChanged | DeviceUnavailable | HostUnavailable | InvalidInput | PermissionDenied | RealtimeDenied | ResourceExhausted | StreamInvalidated | UnsupportedConfig | UnsupportedOperation | Xrun | Backend | Other"
		invariants: [
			"the value is Copy, fixed-size, non-allocating, and contains no framework error, string, path, or owned resource",
			"the callback publishes only the first unconsumed failure through bounded atomics and control ownership takes and formats it",
		]
		contributesTo: [{capability: "capability.realtime_execution", contribution: "keeps a failed running device visible without callback I/O or allocation"}]
	}

	applicationServices: KeyboardInputTranslator: {
		purpose: "translate normalized 1/2/W/S/A/D/K window input and focus changes into the closed SemanticAction vocabulary"
		uses: [
			"valueObject.Shell.WindowInput",
			"valueObject.Control.SemanticAction",
		]
		operations: {
			translate: {input: {event: "WindowInput"}, output: {action: "Option<SemanticAction>"}}
		}
		meta: rules: [
			"Digit1 key-down emits SelectContext(MIXER), Digit2 key-down emits SelectContext(PATCH), and their key-up events emit nothing regardless of K modifier state",
			"bare W/S/A/D key-down emits Navigate Up/Down/Left/Right and key-up emits nothing",
			"K key-down records physical modifier state and emits SetInteractionMode(Adjust); while held W/S/A/D key-down emits Adjust Up/Down/Left/Right; K key-up clears physical modifier state and emits SetInteractionMode(Navigate)",
			"FocusLost clears physical modifier state and emits SetInteractionMode(Navigate) exactly when Adjust had been entered",
			"Other input emits nothing",
			"translation owns only raw held-key normalization and no semantic focus/mode authority, Patch, selection, parameter, projection, or audio state",
		]
		validations: [{id: "validation.service.keyboard_input_translator", kind: "test", command: ["cargo", "test", "keyboard_input_translator"], description: "both context mappings plus every direction mapping, K transition, focus loss, key release, and unrelated key are deterministic"}]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "keeps physical keyboard normalization outside the reducer"},
			{capability: "capability.schema_driven_patch_page", contribution: "maps direct page keys to semantic context events without reading AppState"},
			{capability: "capability.observable_demo_scene", contribution: "is the single translator shared by the eframe window and exhaustive scene runner"},
			{capability: "capability.semantic_graphical_view_model", contribution: "makes physical Edit press/release and directional intent explicit before AppEvent mapping"},
		]
	}

	ports: AppWindow: {
		direction: "outbound"
		contract: {
			run: "(onInput: FnMut(SemanticAction), projection: Fn() -> GraphicalShellProjection, audioObservation: Fn() -> AudioObservationSnapshot, onTick: FnMut(Duration) -> bool, onFrame: FnMut(ShellFrameObservation)) -> Result<(), WindowError>"
		}
		consumes: ["valueObject.Control.SemanticAction", "valueObject.Control.GraphicalShellProjection", "valueObject.RealTime.AudioObservationSnapshot"]
		invariants: [
			"the window receives immutable GraphicalShellProjection plus the latest fixed AudioObservationSnapshot and emits only SemanticAction",
			"the window owns raw key and K-modifier state but no synth parameter, context, FocusPath, InteractionMode, ReturnPath, or selection state",
			"an AppEvent rejection does not close the window or disable later input",
			"each interactive frame advances the injected control-side tick and then requests the current immutable GraphicalShellProjection; autonomous live-demo state is never stored or applied by the window",
			"after painting, the window emits one immutable ShellFrameObservation through the injected control-side callback; it does not infer checkpoint completion, edit a report, or retain canonical state",
			"layout class and pixel geometry are adapter-local responses to viewport size and never become canonical application or navigation state",
			"a false tick result closes the disposable window only after application control ownership has retained a terminal outcome: either the completed live report or a typed fatal runtime error",
		]
		contributesTo: [
			{capability: "capability.one_way_parameter_control", contribution: "keeps the disposable graphical view outside application state"},
			{capability: "capability.schema_driven_patch_page", contribution: "renders whichever immutable semantic context the reducer projects"},
			{capability: "capability.graphical_application_shell", contribution: "owns the passive production-window boundary for the structural shell"},
			{capability: "capability.live_observable_demo", contribution: "renders canonical projections while the control-side live runner advances and closes when its owning application reports completion"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "renders all sixteen fixed meters without storing or feeding them back into domain state"},
			{capability: "capability.semantic_graphical_view_model", contribution: "is the passive action/view-model host boundary"},
		]
	}

	ports: AudioOutput: {
		direction: "outbound"
		contract: {
			negotiate: "() -> Result<NegotiatedAudioOutput, AudioOutputError>"
			config: "(&NegotiatedAudioOutput) -> AudioDeviceConfig"
			start: "(NegotiatedAudioOutput, render: FnMut(&mut [f32]), onRuntimeError: FnMut(AudioDeviceRuntimeError)) -> Result<AudioStream, AudioOutputError>"
		}
		consumes: ["valueObject.Shell.AudioDeviceConfig", "valueObject.Shell.AudioDeviceRuntimeError"]
		invariants: [
			"negotiate selects and validates the device, sample rate, channel mapping, PCM sample format, and bounded render capacity without starting its callback",
			"a valid preferred-rate default configuration is accepted before optional format-range enumeration; if optional enumeration fails, an already validated default remains usable and no device or configuration is invented",
			"start consumes that exact negotiated owner only after a compatible PreparedGraph and AudioRenderer exist",
			"AudioRenderer always receives interleaved stereo f32 frames; a native stereo f32 device may forward its exact caller-owned buffer",
			"a device with more than two channels uses bounded preallocated callback storage, writes stereo to its first two output channels, and silences every surplus channel",
			"post-start framework errors are mapped to AudioDeviceRuntimeError and published without allocation, locking, blocking, I/O, logging, formatting, panic, or destruction",
		]
		contributesTo: [{capability: "capability.realtime_execution", contribution: "connects the hard real-time renderer to a stereo device"}]
	}

	applicationServices: StandaloneApplication: {
		purpose: "compose the control loop, capacity-one graph-preparation worker, automatic MIDI fixture, exhaustive GUI demo, audio renderer, graphical window, and device output"
			uses: [
				"valueObject.Synth.CapabilityRegistry",
				"valueObject.Synth.EffectCapabilityRegistry",
				"valueObject.Synth.SoundFontPresetCatalog",
				"port.Synth.InstrumentCapabilityProvider",
			"port.Synth.InstrumentPreparer",
			"port.Synth.EffectCapabilityProvider",
			"port.Synth.EffectPreparer",
			"applicationService.Synth.PreparedEngineRackBuilder",
			"applicationService.Synth.PreparedPostEffectRackBuilder",
			"applicationService.Control.AppLoop",
			"applicationService.Testing.AutomaticMidiTest",
			"applicationService.Testing.ExhaustiveGuiDemo",
			"applicationService.Testing.LiveDemoRunner",
			"valueObject.Testing.LiveDemoScene",
			"valueObject.Testing.LiveDemoCheckpoint",
			"valueObject.Testing.LiveDemoReport",
			"applicationService.RealTime.AudioRenderer",
			"applicationService.RealTime.PreparedGraphBuilder",
			"applicationService.RealTime.StructuralGraphCoordinator",
			"port.RealTime.StructuralGraphBoundary",
			"port.RealTime.GraphPreparationWorker",
			"applicationService.Synth.DescriptorDefaultConfigFactory",
			"port.RealTime.AudioObservation",
			"port.Shell.AppWindow",
			"port.Shell.AudioOutput",
		]
		operations: {
			run: {input: {}, output: {result: "Result<(), ApplicationError>"}}
			runLiveDemo: {input: {onCheckpoint: "FnMut(&LiveDemoCheckpoint)", onComplete: "FnOnce(&LiveDemoReport)"}, output: {result: "Result<(), ApplicationError>"}}
			runSmoke: {input: {degenerate: "Option<DegenerateMode>"}, output: {observation: "Result<SmokeObservation, ApplicationError>"}}
			runDemoScene: {input: {degenerate: "Option<DegenerateMode>"}, output: {result: "Result<DemoSceneReport, ApplicationError>"}}
		}
		meta: rules: [
				"the crest-synth composition root parses HiDef.sf2 once through HiDefSoundFontAsset, constructs and injects ordered instrument and effect capability providers, their matching separate preparers, shared SoundFontPresetCatalog, StructuralGraphBoundary, AudioObservation, AudioBoundary, MIDI source, window, and AudioOutput; StandaloneApplication imports no concrete infrastructure adapter and constructs none of those boundaries",
				"startup validates duplicate, missing, unknown, and mismatched instrument/effect provider/preparer registrations plus every fixture SoundFont address, freezes both hydrated ordered registries in AppState, and performs no structural publication before registration succeeds",
			"physical startup negotiates AudioDeviceConfig first, prepares complete engine and effect racks plus the graph from its exact sampleRate and renderCapacityFrames, constructs AudioRenderer, starts the same negotiated output owner, then opens the graphical window",
			"startup fails visibly before audio on duplicate, missing, unknown, or mismatched instrument/effect capability or preparer registration and never chooses or substitutes a fallback provider, preparer, prepared instrument/effect, config, asset, preset, graph, engine, or bypass",
			"normal-mode MIDI begins automatically only after the initial graph is fully prepared; each window tick advances the private test input, polls at most one worker outcome and structural status, retries a staged graph, and collects returned graphs outside the callback",
				"production startup alternates HiDef SoundFont and Braids instruments in PreparedEngineRack, configures one effect.chorus slot only on the first fixture Patch, and prepares it through PreparedPostEffectRack; PATCH exposes instrument StructuralChoice and effect ScalarEdit rows generically, and the injected worker prepares one complete replacement at a time without substituting an engine, preset, effect, asset, or config",
			"normal, smoke, headless-demo, and live-demo compositions inject the same instrument/effect registries, providers, preparers, DescriptorDefaultConfigFactory, GraphPreparationWorker port, graph builder, structural coordinator, reducer, projector, and renderer; only the deterministic headless harness manually advances its worker adapter while live uses ThreadedGraphPreparationWorker",
			"runLiveDemo uses the exact normal startup order, real EframeGraphicalWindow, physical CpalAudioOutput, HiDef.sf2, pinned Braids adapter, and Corridors of Time fixture, then starts LiveDemoRunner on the control side and advances it from the existing window-tick callback",
			"runLiveDemo injects the same AppLoop into AutomaticMidiTest, LiveDemoRunner, and immutable projection callbacks; its live-mode window input callback is a stateless semantic no-op, none receives mutable AppState, and no live-specific reducer, engine, mixer, window, or audio-output implementation exists",
			"in live mode only LiveDemoRunner advances AutomaticMidiTest; StandaloneApplication does not also tick the fixture and every due fixture event is dispatched exactly once",
			"the live EventLog is pre-sized to a declared bounded capacity sufficient for the frozen scene and fixture events; dropped records make the final report incomplete rather than being hidden",
			"in normal interactive mode normalized SemanticActions are dispatched through AppLoop.dispatchAction before a new GraphicalShellProjection is requested",
			"in autonomous live-demo mode mapped semantic window input is ignored without an EventRecord, generation change, projection change, parameter publication, or application failure; native window close remains available and follows the typed early-close path",
			"ParameterAtBoundary and every other EventRejection caused by ordinary user input are nonfatal no-ops: keep the window and audio running, preserve the current projection, and accept the next key event",
			"run returns ApplicationError only for startup, adapter, audio-device, window-runtime, automatic-input, or real-time boundary failures; it never promotes a rejected user edit to an application failure",
			"each window tick first consumes any post-start AudioDeviceRuntimeError on control ownership, retains the exact ApplicationError, and asks the window to close; it never formats, logs, or invokes UI behavior from the device callback",
			"runSmoke uses the same services without a physical device or window and measures real control, routing, and rendered-sample results",
			"runDemoScene initializes the real fixture, then drives normalized WindowInputs through KeyboardInputTranslator, SemanticAction mapping, and the production AppLoop; it never mutates state or projections directly",
				"runDemoScene injects DeterministicGraphPreparationWorker, exercises one adjacent SoundFont preset replacement followed by the complete SoundFont to Braids to descriptor-default SoundFont lifecycle plus one controlled failure, and never fabricates a graph, status, event, measurement, or report field",
				"runLiveDemo completes its frozen editable-scalar coverage, then submits one adjacent SoundFont preset choice followed by SoundFont to Braids and Braids to descriptor-default SoundFont for the focused first Patch through LiveDemoRunner semantic events; each window tick advances AppLoop structural work nonblockingly, each transition waits for visible Preparing, Activating, Ready, an acknowledged newer revision, and finite nonzero targeted physical output, and no live path injects failure or fabricates acknowledgement",
			"before physical live startup the composition root prints one actionable control-side status explaining the autonomous input-isolated bounded run; a typed progress or whole-run timeout follows the same fatal tick, close, semantic cleanup, stream-release, and worker/graph shutdown path as another retained live runtime failure",
			"demo execution retains the complete EventLog and final StateTree and reports zero missing coverage before returning success",
			"verification-only demo degeneracy is injected before observation at exactly one real seam: control drops one translated Adjust before AppLoop, audio clears the rendered buffer after AudioRenderer; neither mode edits coverage, a completed DemoSceneReport, or any measured observation field",
			"each checkpoint is passed exactly once to the injected control-side onCheckpoint callback; when LiveDemoRunner completes, pass its report exactly once to onComplete and return false from that same tick so the window closes without a post-completion tick or frame",
			"after the completed window returns, release the physical stream on control ownership and return success when no retained runtime error exists",
			"after every normal, error, demo, or live exit release the audio stream before shutting down the worker, then drain or destroy pending, staged, returned, and retired graph ownership only on control/worker ownership",
			"if the user closes the window before completion, dispatch semantic all-notes-off for installed Patches while the control loop is available, do not call onComplete with a successful report, and return a typed incomplete-live-demo result without fabricating coverage",
			"--demo-live has no degenerate, headless, no-device, no-window, persistent-final-window, or silent-fallback mode; startup or runtime device failures remain typed visible ApplicationErrors",
		]
		validations: [
			{id: "validation.service.standalone_application", kind: "integration", command: ["cargo", "test", "standalone_application"], description: "a boundary adjustment on a non-first Patch is ignored without terminating the window, and a following valid edit is accepted"},
			{id: "validation.service.standalone_exhaustive_demo", kind: "integration", command: ["cargo", "test", "standalone_exhaustive_gui_demo"], description: "the composed headless application emits a complete event log, state tree, coverage matrix, and audio observations"},
			{id: "validation.service.standalone_live_demo", kind: "integration", command: ["cargo", "test", "standalone_live_demo_composition"], description: "the live mode wires the threaded preparation and graph handoff plus real window/audio lifetime to the paced two-direction runner while a deterministic harness proves lifecycle/revision/audio checkpoints, mapped-input isolation, one completion, immediate close, and successful teardown"},
			{id: "validation.service.standalone_runtime_contracts", kind: "integration", command: ["cargo", "test", "--test", "production_runtime_contracts", "--", "--nocapture"], description: "the injected production constructor, negotiated device lifecycle, replaceable boundaries, oversized callback adaptation, and post-start typed error path are executable"},
		]
		contributesTo: [
				{capability: "capability.instrument_capability_model", contribution: "constructs the immutable descriptor registry and injects the provider into the fixture installation path"},
				{capability: "capability.soundfont_preset_selection", contribution: "validates the shared catalog, exact fixture choice identities, and production structural-preset composition before audio starts"},
				{capability: "capability.static_patch_effect", contribution: "validates and composes the Chorus provider/preparer, first-Patch config, prepared rack, canonical control path, and physical demo"},
			{capability: "capability.prepared_engine_rack", contribution: "composes registered preparation, the complete initial graph, distinct structural handoff, and control-side retirement collection"},
			{capability: "capability.soundfont_audio", contribution: "composes the running SoundFont audio path"},
			{capability: "capability.braids_engine", contribution: "composes the intentional second engine from its independent provider and preparer"},
			{capability: "capability.per_voice_envelope", contribution: "uses the same canonical Patch envelope and projection for both prepared implementations"},
			{capability: "capability.automatic_test_midi", contribution: "starts the fixed test input automatically"},
			{capability: "capability.one_way_parameter_control", contribution: "joins keyboard events to the shared reducer and immutable graphical plus diagnostic projections"},
			{capability: "capability.graphical_application_shell", contribution: "composes the production graphical window with the canonical shell projection and retained live lifecycle"},
			{capability: "capability.schema_driven_patch_page", contribution: "composes direct context input with the canonical Patch-page projection"},
			{capability: "capability.asynchronous_engine_selection", contribution: "composes descriptor defaults, capacity-one preparation, one-way lifecycle events, structural handoff, and control-owned shutdown"},
			{capability: "capability.realtime_execution", contribution: "starts control and audio sides in the correct preparation order"},
			{capability: "capability.observable_demo_scene", contribution: "composes the exhaustive scene against production input, reducer, projection, boundary, engine, and mixer services"},
			{capability: "capability.live_observable_demo", contribution: "composes the paced runner with the real standalone UI, physical audio stream, canonical control loop, and final visible state"},
			{capability: "capability.semantic_graphical_view_model", contribution: "composes the normalized action, canonical view-model, passive window, live traversal, and teardown path"},
		]
	}
}

project: adapters: EframeGraphicalWindow: {
	implements: "port.Shell.AppWindow"
	layer: "infrastructure"
	profile: {kind: "ui", surfaces: ["keyboard", "context_line", "identity_header", "main_workspace", "utility_or_inspector", "footer"]}
		meta: {
		framework: "eframe/egui + egui_extras"
			rules: [
				"render GraphicalShellProjection and its embedded SemanticGraphicalViewModel as one structural composition containing the context/status line, identity header, central context workspace, persistent Utility or Inspector side region, and footer; PATCH and MIXER remain reducer-owned projections and no region is an adapter-owned tab or context",
				"when MIXER is visible, render T00 through T0F from the canonical semantic model and pair each track only with the same MixerTrackId entry from the latest fixed AudioObservationSnapshot; a missing or stale observation renders a zero/stale meter state and never changes canonical values or focus",
				"at 1920x1080 use the authored 48 px context line, 72 px identity header, 64 px footer, and approximately 1500:420 workspace split; at 1280x800 retain every band and side region with controlled vertical density, at least 320 px for Utility/Inspector, bounded proportional widths, and no overlap or hidden required label",
				"use egui panels and matching-version egui_extras layout utilities; image/SVG loaders are available for later graphical assets, but this phase adds no waveform, Sample capability, browser, component library, or alternate GUI runtime",
				"render the semantic surfaces, focused path, Navigate/Adjust state, status, typed errors, and valid-action footer from the embedded model; retain TextProjection only as read-only diagnostic content and never use it to infer focus or actions",
				"use only a small private Phase 1 shell palette and type setup sufficient to distinguish structural bands; do not define the reusable Crest token/component API reserved for Phase 4",
				"normalize egui key presses, releases, and focus loss into WindowInput and delegate every 1/2/W/S/A/D/K decision to KeyboardInputTranslator",
				"do not retain a second private keyboard state machine or duplicate the translator in tests",
				"key handling emits SemanticActions only and never mutates view focus, mode, return path, Patch values, AppState, snapshots, event logs, or audio",
				"the focused SemanticControlViewModel receives the only focus/adjust treatment, side SurfaceRoot receives surface focus, and labels/hints come from GraphicalShellProjection; let the reducer alone interpret every semantic action",
				"the headless adapter contract drives real egui RawInput through an egui Context and EframeGraphicalApplication.update with the production on_input callback wired to AppLoop.dispatchAction, then runs the next frame from AppLoop.currentGraphicalShell; capturing a callback without applying it is not acceptance evidence",
				"the next frame must prove the event-log record, accepted generation, FocusPath, mode, return path, valid actions, shell context/status/identity/footer, selected diagnostic value, every unrelated value, SemanticGraphicalViewModel, GraphicalShellProjection and TextProjection hashes, and selected-line scroll target all reflect that same dispatched GUI action",
				"the graphical shell acceptance renders 1920x1080 and 1280x800 frames and inspects production-recorded region rectangles plus egui output for exact visible labels, containment, ordering, non-overlap, and persistent side-region identity; calling only a layout helper or rendering an unrelated supplied projection is insufficient",
				"in every live mode including --demo-live-sixteen-track-mixer-routing, invoke the injected tick without blocking, then render only the newest AppLoop.currentGraphicalShell projection plus injected fixed AudioObservationSnapshot; the adapter never receives LiveDemoScene, LiveDemoReport, mutable AppState, mixer buffers, or a device owner",
				"schedule the next idle repaint after 16 ms instead of requesting an immediate perpetual repaint; native input and window events may wake the event loop sooner",
				"when the injected tick returns false after a completed live report, send a viewport-close command immediately and do not request another projection or repaint",
				"also send a viewport-close command when the injected tick returns false for an already-retained fatal runtime failure",
			]
		}
		validations: [
			{id: "validation.adapter.eframe_graphical_window", kind: "test", command: ["cargo", "test", "eframe_graphical_window"], description: "the adapter normalizes the complete egui input vocabulary and paints every structural region without owning semantic state"},
			{id: "validation.adapter.eframe_context", kind: "integration", command: ["cargo", "test", "--test", "eframe_context", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE eframe_context passed"}], description: "a headless egui Context executes EframeGraphicalApplication.update, invokes the real callback, renders exact shell/diagnostic projection values, and targets the exact selected line without a native window"},
		]
	contributesTo: [
		{capability: "capability.graphical_application_shell", contribution: "implements the authored structural shell and responsive desktop/Steam Deck composition"},
		{capability: "capability.one_way_parameter_control", contribution: "implements the passive graphical screen and exact keyboard vocabulary"},
		{capability: "capability.sixteen_track_mixer_routing", contribution: "paints fixed track controls and meter observations without UI-owned values"},
			{capability: "capability.schema_driven_patch_page", contribution: "renders PATCH and MIXER projections without owning their page state or schema"},
			{capability: "capability.soundfont_preset_selection", contribution: "renders exact authored preset labels and generic structural lifecycle state without owning catalog or selection behavior"},
			{capability: "capability.static_patch_effect", contribution: "renders the read-only Chorus identity and generic editable Amount/Depth rows without owning effect state or schema"},
		{capability: "capability.observable_demo_scene", contribution: "shares its production input translator with the deterministic scene"},
		{capability: "capability.live_observable_demo", contribution: "renders each accepted live generation and closes on the owner's completed-report tick"},
		{capability: "capability.semantic_graphical_view_model", contribution: "renders the canonical multi-layout model and emits only its semantic actions"},
	]
}

project: adapters: CpalAudioOutput: {
	implements: "port.Shell.AudioOutput"
	layer: "infrastructure"
	profile: {kind: "device_output", medium: "stereo-pcm"}
	meta: {
		framework: "cpal"
		rules: [
			"select the default PCM output, prefer 48 kHz when supported, validate at least two channels and a bounded render capacity, and return the negotiated owner without starting it",
			"accept an already valid 48 kHz default before querying optional supported ranges; if that query fails for a nonpreferred device, retain a valid reported default and fail only when neither source yields a valid PCM configuration",
			"start only that negotiated device after compatible graph preparation and never pass sample-rate data into the render callback",
			"use a direct callback fast path for native stereo f32; otherwise render into fixed-capacity stereo scratch storage, map left and right to device channels 1 and 2, and write silence to surplus device channels",
			"the callback performs no pacing, allocation, locking, I/O, logging, format construction, or deallocation",
			"convert sample formats with bounded arithmetic when the device is not f32",
			"map every post-start CPAL ErrorKind to fixed-size AudioDeviceRuntimeError and invoke only the bounded injected status callback",
		]
	}
	contributesTo: [
		{capability: "capability.realtime_execution", contribution: "implements the physical low-latency stereo output"},
		{capability: "capability.live_observable_demo", contribution: "keeps the real device callback running while the visible scene advances"},
	]
}
