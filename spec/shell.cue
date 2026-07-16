package crestsynth

// Shell — infrastructure adapters for audio, MIDI, window/GUI, and gamepad.
// The engine library stays host-agnostic; the shell is the only place that
// touches devices and windows.

project: contexts: Shell: purpose: "ports and adapters for the outside world: audio output, MIDI input, windowing/GUI, and gamepad navigation (every UI action reachable via gamepad)"

project: contexts: Shell: valueObjects: {
	MidiPortInfo: {state: {name: "string", index: "u32"}, description: "one connectable MIDI input port"}
	GamepadButton: {description: "host-neutral logical controller button used for action mapping and glyph lookup"}
	ControllerType: {description: "recognized controller family used only to select display glyphs"}
	GamepadAction: {description: "current semantic mixer input: Navigate(direction) or Adjust(direction); the adapter emits Adjust while the gamepad edit modifier is held, exactly as K modifies keyboard directions"}
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
			render: "(projection: MixerTextProjection) -> list<AppEvent>"
		}
		meta: notes: "renders one scrollable wall of text with default labels and returns semantic events; it owns no editable values and provides no substantial visual UI"
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
	profile: {kind: "ui", surfaces: ["plain-text-backend-diagnostic"]}
	meta: framework: "eframe"
	validations: [{kind: "compiles", command: ["cargo", "build", "--bin", "synth_ui"], description: "the current eframe host compiles on the supported desktop stack"}]
}

project: adapters: EguiRenderer: {
	implements: "port.Shell.GuiRenderer"
	layer:      "infrastructure"
	profile: {kind: "ui", surfaces: ["single-scrollable-text-list"], accessibility: ["keyboard", "gamepad"]}
	meta: {
		framework: "egui"
		rules: [
			"use only a default central container, one vertical ScrollArea, monospace Label text, and automatic scrolling to the selected line",
			"do not create panels, columns, tables, grids, meters, faders, inspectors, toolbars, menus, custom widgets, custom painting, theme abstractions, animation, icons, or layout systems",
			"render MixerTextProjection.body verbatim except for the minimum selection emphasis available on a stock Label",
			"bare W/S/A/D emit AppEvent::Mixer Navigate variants, K+W/S/A/D emit AppEvent::Mixer Adjust variants, and L emits AppEvent::Playback(ToggleFromStart); the adapter never performs the action itself",
		]
	}
	validations: [{kind: "integration", command: ["make", "ui-smoke"], description: "the stock-label renderer presents the complete text projection and returns semantic keyboard events without a window"}]
	contributesTo: [{capability: "capability.pointer_free_mixer_control", contribution: "provides a disposable wall-of-text shell over the serialized backend projection"}]
}

project: adapters: GilrsGamepadInput: {
	implements: "port.Shell.GamepadInput"
	layer:      "infrastructure"
	profile: {kind: "device_input", device: "game controller"}
	meta: framework: "gilrs"
	validations: [{kind: "test", command: ["cargo", "test", "gilrs_gamepad_input"], description: "bare d-pad emits Navigate and edit-modified d-pad emits Adjust without leaking device state"}]
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
	purpose: "translates bare d-pad input into Navigate and edit-modified d-pad input into Adjust, matching W/S/A/D and K+direction"
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
	{text: "keyboard and gamepad use the app's semantic Navigate/Adjust model, not egui focus or widget editing", meta: rationale: "the disposable renderer must exercise the canonical reducer rather than framework-owned UI state"},
]
