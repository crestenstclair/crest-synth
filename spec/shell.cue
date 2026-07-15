package crestsynth

// Shell — infrastructure adapters for audio, MIDI, window/GUI, and gamepad.
// The engine library stays host-agnostic; the shell is the only place that
// touches devices and windows.

project: contexts: Shell: purpose: "ports and adapters for the outside world: audio output, MIDI input, windowing/GUI, and gamepad navigation (every UI action reachable via gamepad)"

project: contexts: Shell: valueObjects: {
	MidiPortInfo: {state: {name: "string", index: "u32"}, description: "one connectable MIDI input port"}
	GamepadAction: {description: "a mapped gamepad input: navigate (d-pad), fine-adjust (left stick), scroll (right stick), select (A), back (B), switch view (triggers), switch patch (bumpers), save session (start), open browser (select)"}
}

project: contexts: Shell: ports: {
	AudioOutput: {
		contract: {
			open:  "(sampleRate: SampleRate, bufferSize: BufferSize, callback: RenderCallback) -> result<Stream, AudioError>"
			close: "(stream: Stream) -> ()"
		}
		meta: notes: "the callback runs on the audio thread and is bound by the real-time invariants"
		contributesTo: [{capability: "capability.operate_audio_and_midi_devices", contribution: "owns lifecycle and callback delivery for the selected desktop audio stream"}]
	}
	MidiInput: {
		contract: {
			listPorts:  "() -> list<MidiPortInfo>"
			connect:    "(port: MidiPortInfo, onEvent: EventCallback) -> result<Connection, MidiError>"
			disconnect: "(connection: Connection) -> ()"
		}
		contributesTo: [
			{capability: "capability.accept_external_midi", contribution: "owns connection and event delivery for external MIDI hardware"},
			{capability: "capability.operate_audio_and_midi_devices", contribution: "owns selection and lifecycle of the external MIDI connection"},
		]
	}
	AppWindow: {
		contract: {
			run: "(app: App) -> result<(), WindowError>"
		}
	}
	GuiRenderer: {
		contract: {
			render: "(view: ViewState) -> ()"
		}
		meta: notes: "renders the patch editor, mixer view, preset browser, and mod matrix editor"
		contributesTo: [{capability: "capability.edit_without_pointer", contribution: "presents every sound-design and instrument-management view"}]
	}
	GamepadInput: {
		contract: {
			poll: "() -> list<GamepadAction>"
		}
		contributesTo: [{capability: "capability.edit_without_pointer", contribution: "defines the complete pointer-free editing action vocabulary"}]
	}
}

project: contexts: Shell: domainServices: {
	MidiNormalizer: {
		purpose: "converts raw MIDI 1.0 bytes into normalized MidiEvents: addressed, high-resolution values, NoteId assigned"
		uses: ["valueObject.Kernel.MidiEvent"]
		contributesTo: [{capability: "capability.accept_external_midi", contribution: "turns raw MIDI input into addressed high-resolution events with stable note identity"}]
	}
}

project: adapters: CpalAudioOutput: {
	implements: "port.Shell.AudioOutput"
	layer:      "infrastructure"
	meta: framework: "cpal"
	contributesTo: [{capability: "capability.operate_audio_and_midi_devices", contribution: "opens the physical audio stream and invokes the accepted real-time callback"}]
}

project: adapters: MidirMidiInput: {
	implements: "port.Shell.MidiInput"
	layer:      "infrastructure"
	meta: framework: "midir"
	contributesTo: [
		{capability: "capability.accept_external_midi", contribution: "delivers raw events from the selected external MIDI device"},
		{capability: "capability.operate_audio_and_midi_devices", contribution: "implements external MIDI port discovery and connection lifecycle"},
	]
}

project: adapters: EframeAppWindow: {
	implements: "port.Shell.AppWindow"
	layer:      "infrastructure"
	meta: framework: "eframe"
}

project: adapters: EguiRenderer: {
	implements: "port.Shell.GuiRenderer"
	layer:      "infrastructure"
	meta: framework: "egui"
	contributesTo: [{capability: "capability.edit_without_pointer", contribution: "renders the gamepad- and keyboard-navigable instrument views"}]
}

project: adapters: GilrsGamepadInput: {
	implements: "port.Shell.GamepadInput"
	layer:      "infrastructure"
	meta: framework: "gilrs"
	contributesTo: [{capability: "capability.edit_without_pointer", contribution: "converts physical controller input into normalized editor actions"}]
}

// ── Shell additions (editor increment) ──────────────────
// Host-agnostic navigation/glyph domain services, ported from the original
// spec. These sit alongside the existing Shell ports/adapters (audio, MIDI,
// window, gamepad) and translate raw gamepad input into the app's own
// cursor/edit model plus per-controller-type button glyphs.
//
// PORTING NOTE (see NOTES.md): the original's GamepadInput port contract was
// richer (`poll`, `connectedControllers`, `controllerType`) than the clean
// base's (`poll: () -> list<GamepadAction>` only), and the original's
// GamepadAction enum (Navigate/Select/Back/TweakUp/TweakDown/AssignMod/
// NextPage/PreviousPage/QuickSave) used different vocabulary than the clean
// base's ALREADY-DECLARED `valueObject.Shell.GamepadAction` (navigate/
// fine-adjust/scroll/select/back/switch view/switch patch/save session/open
// browser). GamepadAction is NOT redeclared here — GamepadNavigator targets
// the EXISTING clean-base GamepadAction and GamepadInput port. ControllerGlyph
// and its GamepadNavigator/GlyphResolver domain services did not exist on the
// clean base at all, so they are new additions, ported verbatim from main.

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
	contributesTo: [{capability: "capability.edit_without_pointer", contribution: "drives the same navigation and editing vocabulary from physical gamepads"}]
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
	{text: "the engine library is host-agnostic; no audio driver, window, or controller code in the library", meta: rationale: "standalone and plugin wrapper are different shells over the same library"},
	{text: "the UI is a pure view over engine state; no audio logic lives in the GUI layer", meta: rationale: "keeps DSP and voice logic testable in isolation"},
	{text: "all gamepad navigation uses the app's own cursor/edit model, not egui's built-in focus", meta: rationale: "generic focus traversal doesn't fit a controller-first workflow"},
]
