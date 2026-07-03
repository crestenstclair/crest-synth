package crestsynth

// ── Editor ─────────────────────────────────────────────
// Keyboard/gamepad-driven parameter editor: a one-way (Elm/Flux) event loop
// over a single store (EditorState) that edits live engine parameters. The
// standalone editor app hosts the live engine (external MIDI in via the Shell
// MidiInput port, audio out via the Shell AudioOutput port) and is hermetically
// smoke-testable with no window/device.
//
// EditorState is the single store. The egui shell and the gamepad adapter both
// emit the SAME EditorEvents into it, so keyboard and gamepad are interchangeable
// and the whole control plane is hermetically testable: feed an event sequence,
// assert focus / edit-mode / field values — no window, no device.
//
// PORTING NOTE: on the original spec this context predates the Mixer view and
// was superseded by it (see spec/mixer.cue's MixerView, added in
// mixer-additions.cue) — the original's own StandaloneUiMain asset says the
// mixer view "REPLACES the previous parameter-list editor that used to live
// here." EditorEvent/ParamField/EditorState are ported here verbatim as the
// original declared them; no asset in this increment references them (see
// NOTES.md).

project: contexts: Editor: purpose: "keyboard/gamepad-driven parameter editor: a one-way event loop over a single store that edits live engine parameters"
project: contexts: Editor: ubiquitousLanguage: {
	EditorEvent: "a semantic input event (navigate or edit-mode change) emitted by the keyboard/gamepad adapter — the only thing that mutates editor state"
	EditorState: "the single store: focus position, edit-mode flag, and the list of editable parameter fields"
	ParamField:  "one editable parameter row: label, current value, bounds, and fine step"
	EditMode:    "active only while the edit modifier (J / a gamepad button) is held; directional input then adjusts the focused field's value instead of moving focus"
}

project: contexts: Editor: valueObjects: EditorEvent: {
	from:        "enum"
	description: "NavUp, NavDown, NavLeft, NavRight, EnterEditMode, ExitEditMode — the semantic input vocabulary; keyboard and gamepad adapters both emit these and nothing else"
	validations: [{kind: "compiles", command: ["cargo", "build"], description: "crate builds with EditorEvent"}]
}

project: contexts: Editor: valueObjects: ParamField: {
	state:       {id: "string", label: "string", value: "f64", min: "f64", max: "f64", step: "f64"}
	description: "one editable parameter row: a label, current value, inclusive bounds, and the fine adjustment step (coarse = 10x step)"
	invariants: ["min <= max", "value is always within [min, max]", "step > 0"]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with ParamField"},
		{kind: "test", command: ["cargo", "test", "param_field"], description: "ParamField clamp/bounds unit tests pass"},
	]
}

project: contexts: Editor: aggregates: EditorState: {
	root:    true
	purpose: "the single editor store: owns focus, edit-mode, and the editable parameter fields; the one entry point that reacts to EditorEvents"
	state: {fields: "Vec<ParamField>", focus: "usize", editMode: "bool"}
	invariants: [
		"apply(EditorEvent) is the ONLY way to mutate editor state; no setters, pure and allocation-free (no I/O, rendering, or audio)",
		"focus always stays within the fields range; navigate-mode directional events move focus by one, saturating at the ends (no wrap)",
		"in navigate mode directional events move focus; in edit mode they adjust the focused field's value instead",
		"in edit mode NavRight = +fine and NavLeft = -fine (one unit); NavUp = +coarse and NavDown = -coarse (ten units = 10x fine)",
		"every value adjustment clamps to the focused field's [min, max]",
	]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with EditorState"},
		{kind: "test", command: ["cargo", "test", "editor_state"], description: "EditorState event-reducer unit tests pass (nav, edit-mode, fine/coarse, clamping)"},
	]
}

// ── DesignSystem ───────────────────────────────────────
// The reusable UI foundation: the vocabulary and rules every crest-synth view
// is built from, so views compose from a small consistent base instead of
// re-inventing controls. This context defines ONLY the foundation — semantic
// design tokens and the Theme abstraction skins read them through — plus the
// invariants that keep every future primitive consistent. No primitive controls
// (value controls, toggles, lists, …) are declared here yet; each is added in a
// later increment when a view actually needs it.
//
// The governing pattern (behavior ÷ skin ÷ token), applied to every primitive
// when it is eventually added:
//   1. BEHAVIOR — a pure reducer (State + Event + apply); no egui, no I/O;
//      proven headlessly by feeding events and asserting state.
//   2. VIEW-MODEL — a projection of State into host-neutral parts (role, value,
//      focus/edit flags) so behavior never depends on egui.
//   3. SKIN — the only part that touches egui; draws a view-model using a Theme,
//      reading colors only as SemanticTokens. Swap the Theme → restyle the whole
//      app with zero behavior change.
// Inspiration (not a dependency — we author and generate our own): JUCE's
// Component+LookAndFeel split and Ark/Zag's state-machine + connect() model.

project: contexts: DesignSystem: purpose: "the reusable UI foundation: semantic design tokens and the Theme abstraction skins resolve them through, plus the behavior÷skin÷token invariants every view's primitives must follow"

project: contexts: DesignSystem: ubiquitousLanguage: {
	SemanticToken: "a named UI intent (focus ring, value fill, panel background, …) that a skin asks the Theme to resolve — never a literal color"
	Rgba:          "the raw 8-bit RGBA color a SemanticToken resolves to; the only place a literal color value lives"
	Theme:         "the abstraction a skin reads tokens through: resolves a SemanticToken to an Rgba; swapping the Theme restyles the whole app"
	Skin:          "the egui-drawing half of a primitive: renders a primitive's view-model using a Theme; the only part that touches egui"
}

// The semantic intents the mixer renders today. This set grows ONE entry at a
// time when a new view needs it — never preemptively.
project: contexts: DesignSystem: valueObjects: SemanticToken: {
	from:        "enum"
	description: "FocusRing, EditActive, ValueFill, MeterPeak, ToggleOn, ToggleOff, TextDefault, TextMuted, PanelBg, Separator — the named UI intents a skin resolves through the Theme. FocusRing marks the focused cell; EditActive marks focused-and-in-edit-mode; ValueFill is a value readout/bar; MeterPeak is the live peak overlay; ToggleOn/ToggleOff are toggle states; TextDefault/TextMuted are text; PanelBg/Separator are container chrome."
	invariants: ["the variant set is exactly FocusRing, EditActive, ValueFill, MeterPeak, ToggleOn, ToggleOff, TextDefault, TextMuted, PanelBg, Separator"]
	validations: [{kind: "compiles", command: ["cargo", "build"], description: "crate builds with SemanticToken"}]
}

project: contexts: DesignSystem: valueObjects: Rgba: {
	state:       {r: "u8", g: "u8", b: "u8", a: "u8"}
	description: "an 8-bit straight-alpha RGBA color — the raw value a SemanticToken resolves to, and the only place a literal color lives. Convertible to egui's Color32 in skin code."
	validations: [{kind: "compiles", command: ["cargo", "build"], description: "crate builds with Rgba"}]
}

// The abstraction skins depend on (DIP): a skin never names a literal color, it
// asks the Theme to resolve a SemanticToken. Concretions (DefaultTheme, future
// alternate themes) implement this, so the whole app re-skins by swapping one.
project: contexts: DesignSystem: ports: Theme: {
	contract: {color: "SemanticToken -> Rgba"}
	meta: notes: "Skins take a Theme (trait object or generic bound) and resolve every color through `color(token)`. No skin reads an Rgba except via the Theme. This is the seam that makes restyling/dark-mode/alternate-skins free."
	validations: [{kind: "compiles", command: ["cargo", "build"], description: "crate builds with Theme port"}]
}

// The default concretion of Theme: binds every SemanticToken to a raw Rgba (a
// dark, dense-tool palette). Pure domain data — no I/O — so it is a domain
// service, not an infrastructure adapter.
project: contexts: DesignSystem: domainServices: DefaultTheme: {
	purpose: "implements the Theme port with the default dark palette: maps every SemanticToken to a concrete Rgba, with no token left unresolved"
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with DefaultTheme"},
		{kind: "test", command: ["cargo", "test", "default_theme"], description: "DefaultTheme resolves every SemanticToken variant to an Rgba (exhaustive, no panic/fallback)"},
	]
}

// ── Invariants ─────────────────────────────────────────

project: invariants: standaloneEditor: [
	{text: "the standalone UI is keyboard/gamepad driven only — no mouse or touch input in this implementation", meta: rationale: "keeps the initial implementation clean; pointer input can be added later without changing the event-loop core"},
	{text: "the standalone UI is not a performance surface: it originates no notes; all note performance comes from external MIDI", meta: rationale: "the UI's job is mixing/editing, not playing"},
	{text: "the UI mutates state only by emitting MixerViewEvents applied to MixerView; egui draw code is a pure view that reads channel values and per-channel peak levels from the ChannelStrip channels it wraps", meta: rationale: "one-way data flow keeps state changes traceable and the control plane hermetically testable"},
	{text: "the audio model consumes external MIDI plus a published parameter snapshot across the RealTime seam and never observes MixerViewEvents", meta: rationale: "keeps the engine host-agnostic and the realtime path decoupled from the UI event loop"},
	{text: "the ui smoke path opens no window, no audio device, and no MIDI device; it only constructs state and drives the event loop", meta: rationale: "keeps the standalone app mechanically checkable with no display or hardware"},
	{text: "the mixer draw code is a skin: it holds no literal color and resolves every color through a DesignSystem Theme by SemanticToken; the only raw-color touch is converting the Theme's returned Rgba to egui Color32", meta: rationale: "the behavior÷skin÷token seam — swapping the Theme restyles the whole mixer with zero draw-code change (the DesignSystem invariant applied to its first consumer)"},
	{text: "--autopilot is a real end-to-end run that PROVES behavior, not just clean exit: it opens the actual window and audio device, injects its own built-in notes through the live engine, and asserts IN CODE that the real device-bound audio peak is > 0 and that all 6 channel strips fit on screen (exit non-zero on silence or off-screen strips) before self-terminating via a viewport Close", meta: rationale: "the validation must fail when the app is silent or renders one channel; asserting on a self-printed completion token alone is theater that lets real bugs pass"},
	{text: "no validation may assert only on a token the binary prints unconditionally; a behavioral claim (audio plays, N strips render) must be checked in code against the real observed value so a regression makes the process exit non-zero", meta: rationale: "the autopilot's `autopilot complete:` token passed while only one channel rendered and nothing played — observable behavior must be asserted, not narrated"},
	{text: "the window backend must use a current eframe/egui (0.28+ on objc2 0.5+/winit 0.30+), never the 0.27 line that pulls icrate 0.0.4", meta: rationale: "eframe 0.27/objc2-0.3-beta/icrate-0.0.4 aborts at window creation on current macOS (NSScreen enumeration ABI panic); this is invisible to ui-smoke (no window) so it can only be prevented by the dependency pin, not the validation loop"},
]

project: invariants: designSystem: [
	{text: "every interactive primitive is a pure reducer (State + Event + apply); its behavior layer never touches egui, performs no I/O, and is allocation-free", meta: rationale: "behavior must be unit-testable headlessly and reusable across any renderer — the same discipline MixerView already follows"},
	{text: "a skin reads colors only by resolving a SemanticToken through the Theme port; no literal color value or hard-coded size appears in draw code", meta: rationale: "this single seam is what lets the whole app be restyled by swapping the Theme, with zero behavior change (the JUCE LookAndFeel property)"},
	{text: "every primitive's behavior is proven by feeding Events and asserting State with no window and no device, exactly as the mixer demo proves MixerView", meta: rationale: "keeps the control plane hermetically testable and decoupled from egui/gilrs/cpal"},
	{text: "a primitive reports focus only via its view-model flags (focused, editing); it never decides where focus is — the consuming view owns cursor traversal and edit-mode, the skin renders the flags via FocusRing/EditActive", meta: rationale: "traversal rules are layout-specific (a 2D channel×param grid scrolls unlike a 1D list), so views own traversal while focus still LOOKS identical everywhere through two tokens"},
	{text: "navigation is expressed as semantic events shared by the keyboard and gamepad adapters, never as raw device input reaching a primitive", meta: rationale: "controller/keyboard parity, identical to the MixerViewEvent precedent"},
]
