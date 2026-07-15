package crestsynth

// Shell — infrastructure adapters for audio, MIDI, window/GUI, and gamepad.
// The engine library stays host-agnostic; the shell is the only place that
// touches devices and windows.

project: contexts: Shell: purpose: "ports and adapters for the outside world: audio output, MIDI input, windowing/GUI, and gamepad navigation (every UI action reachable via gamepad)"

project: contexts: Shell: valueObjects: {
	MidiPortInfo: {state: {name: "string", index: "u32"}, description: "one connectable MIDI input port"}
	GamepadButton: {description: "host-neutral logical controller button used for action mapping and glyph lookup"}
	ControllerType: {description: "recognized controller family used only to select display glyphs"}
	GamepadAction: {description: "current semantic mixer input: NavUp/NavDown/NavLeft/NavRight, EnterEditMode, ExitEditMode, ToggleFocusedParam; it maps one-to-one onto MixerViewEvent"}
}

project: contexts: Shell: ports: {
	AudioOutput: {
		direction: "outbound"
		contract: {
			open:  "(sampleRate: SampleRate, bufferSize: BufferSize, callback: RenderCallback) -> result<Stream, AudioError>"
			close: "(stream: Stream) -> ()"
		}
		meta: notes: "the callback runs on the audio thread and is bound by the real-time invariants"
	}
	MidiInput: {
		direction: "inbound"
		contract: {
			listPorts:  "() -> list<MidiPortInfo>"
			connect:    "(port: MidiPortInfo, onEvent: EventCallback) -> result<Connection, MidiError>"
			disconnect: "(connection: Connection) -> ()"
		}
	}
	AppWindow: {
		direction: "outbound"
		contract: {
			run: "(app: App) -> result<(), WindowError>"
		}
	}
	GuiRenderer: {
		direction: "outbound"
		contract: {
			render: "(view: ViewState) -> ()"
		}
		meta: notes: "renders only the current six-strip mixer view from AppState; additional screens are not part of the current product"
	}
	GamepadInput: {
		direction: "inbound"
		contract: {
			poll: "() -> list<GamepadAction>"
		}
	}
}

project: contexts: Shell: domainServices: {
	MidiNormalizer: {
		purpose: "converts raw MIDI 1.0 bytes into normalized MidiEvents: addressed, high-resolution values, NoteId assigned"
		uses: ["valueObject.Kernel.MidiEvent"]
		validations: [{kind: "test", command: ["cargo", "test", "midi_normalizer"], description: "running status, channel address, resolution, NoteId, and malformed bytes are handled deterministically"}]
		contributesTo: [{capability: "capability.external_midi_performance", contribution: "turns raw MIDI input into addressed high-resolution events with stable note identity"}]
	}
}

project: adapters: CpalAudioOutput: {
	implements: "port.Shell.AudioOutput"
	layer:      "infrastructure"
	profile: {kind: "device_output", device: "default desktop audio device"}
	meta: framework: "cpal"
	validations: [{kind: "test", command: ["cargo", "test", "cpal_audio_output"], description: "format conversion and exact callback slice handling are tested without moving the stream owner"}]
	contributesTo: [{capability: "capability.external_midi_performance", contribution: "delivers the production stereo render callback to the physical audio device"}]
}

project: adapters: MidirMidiInput: {
	implements: "port.Shell.MidiInput"
	layer:      "infrastructure"
	profile: {kind: "device_input", device: "external MIDI input"}
	meta: framework: "midir"
	validations: [{kind: "test", command: ["cargo", "test", "midir_midi_input"], description: "port selection and callback lifecycle translate failures without panics"}]
	contributesTo: [{capability: "capability.external_midi_performance", contribution: "discovers, connects, and delivers raw events from external MIDI hardware"}]
}

project: adapters: EframeAppWindow: {
	implements: "port.Shell.AppWindow"
	layer:      "infrastructure"
	profile: {kind: "ui", surfaces: ["mixer"]}
	meta: framework: "eframe"
	validations: [{kind: "compiles", command: ["cargo", "build", "--bin", "synth_ui"], description: "the current eframe host compiles on the supported desktop stack"}]
}

project: adapters: EguiRenderer: {
	implements: "port.Shell.GuiRenderer"
	layer:      "infrastructure"
	profile: {kind: "ui", surfaces: ["six-strip-mixer"], accessibility: ["keyboard", "gamepad"]}
	meta: framework: "egui"
	validations: [{kind: "integration", command: ["make", "ui-smoke"], description: "the pure mixer skin resolves all tokens and the host can construct without a window"}]
	contributesTo: [{capability: "capability.pointer_free_mixer_control", contribution: "renders the six-strip mixer projection without owning or mutating application state"}]
}

project: adapters: GilrsGamepadInput: {
	implements: "port.Shell.GamepadInput"
	layer:      "infrastructure"
	profile: {kind: "device_input", device: "game controller"}
	meta: framework: "gilrs"
	validations: [{kind: "test", command: ["cargo", "test", "gilrs_gamepad_input"], description: "raw controller edges and hold/double-tap timing produce semantic actions"}]
	contributesTo: [{capability: "capability.pointer_free_mixer_control", contribution: "converts physical controller input into the same mixer actions emitted by keyboard input"}]
}

// Host-neutral navigation and glyph services sit between physical input and
// AppEvent construction; they contain no window or device access themselves.

project: contexts: Shell: valueObjects: ControllerGlyph: {
	state: {button: "GamepadButton", controllerType: "ControllerType", glyphPath: "string"}
	description: "maps a logical button to the correct visual glyph for the connected controller"
	invariants: ["the #[cfg(test)] unit-test module is named `tests`, never the same name as its file/parent module (clippy::module_inception is denied under -D warnings)"]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with ControllerGlyph"},
		{kind: "test", command: ["cargo", "test", "controller_glyph"], description: "ControllerGlyph unit tests pass"},
	]
}

project: contexts: Shell: domainServices: GamepadNavigator: {
	purpose: "translates raw gamepad events into GamepadActions and drives the cursor/edit model"
	uses: ["port.Shell.GamepadInput", "valueObject.Shell.GamepadAction"]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with GamepadNavigator"},
		{kind: "test", command: ["cargo", "test", "gamepad_navigator"], description: "GamepadNavigator unit tests pass"},
	]
	contributesTo: [{capability: "capability.pointer_free_mixer_control", contribution: "drives the same mixer navigation and editing vocabulary from physical gamepads"}]
}
project: contexts: Shell: domainServices: GlyphResolver: {
	purpose: "resolves the correct controller glyph for each button based on connected controller type"
	uses: ["valueObject.Shell.ControllerGlyph"]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with GlyphResolver"},
		{kind: "test", command: ["cargo", "test", "glyph_resolver"], description: "GlyphResolver unit tests pass"},
	]
}

// ── Invariants ─────────────────────────────────────────
// Ported verbatim from the original spec's shellDesign invariant group (the
// clean base declares no invariant group under this name, so all three are
// added here rather than cherry-picked).

project: invariants: shellDesign: [
	{text: "the engine library is host-agnostic; no audio driver, window, or controller code exists in domain modules", meta: rationale: "the standalone shell remains replaceable and mechanically testable"},
	{text: "the UI is a pure view over engine state; no audio logic lives in the GUI layer", meta: rationale: "keeps DSP and voice logic testable in isolation"},
	{text: "all gamepad navigation uses the app's own cursor/edit model, not egui's built-in focus", meta: rationale: "generic focus traversal doesn't fit a controller-first workflow"},
]
