package crestsynth

// Editor is host-neutral bounded parameter-editing behavior retained inside
// AppState. It is not a separately visible screen in the current mixer-only UI.
//
// EditorState is the single store. The egui shell and the gamepad adapter both
// emit the SAME EditorEvents into it, so keyboard and gamepad are interchangeable
// and the whole control plane is hermetically testable: feed an event sequence,
// assert focus / edit-mode / field values — no window, no device.
//
project: contexts: Editor: purpose: "host-neutral one-way reducer for bounded parameter lists; reusable application state but not a current GUI surface"
project: contexts: Editor: ubiquitousLanguage: {
	EditorEvent: "a semantic input event (navigate or edit-mode change) emitted by the keyboard/gamepad adapter — the only thing that mutates editor state"
	EditorState: "the single store: focus position, edit-mode flag, and the list of editable parameter fields"
	ParamField:  "one editable parameter row: label, current value, bounds, and fine step"
	EditMode:    "active only while the edit modifier (K / a gamepad button) is held; directional input then adjusts the focused field's value instead of moving focus"
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
		"apply(EditorEvent) is the only mutation API; it is deterministic and performs no I/O, rendering, or audio",
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

// ── Invariants ─────────────────────────────────────────

project: invariants: standaloneEditor: [
	{text: "the standalone UI is keyboard/gamepad driven only — no mouse or touch input in this implementation", meta: rationale: "keeps the initial implementation clean; pointer input can be added later without changing the event-loop core"},
	{text: "the standalone UI is not a performance surface: it originates no notes; all note performance comes from external MIDI", meta: rationale: "the UI's job is mixing/editing, not playing"},
	{text: "the UI mutates state only by emitting AppEvent::Mixer into AppState.apply; egui draw code is a pure projection of canonical mixer state and peak levels", meta: rationale: "one application reducer keeps live input and scenes traceable and comparable"},
	{text: "the audio model consumes external MIDI plus a published parameter snapshot across the RealTime seam and never observes MixerViewEvents", meta: rationale: "keeps the engine host-agnostic and the realtime path decoupled from the UI event loop"},
	{text: "the ui smoke path opens no window, no audio device, and no MIDI device; it only constructs state and drives the event loop", meta: rationale: "keeps the standalone app mechanically checkable with no display or hardware"},
	{text: "the current renderer uses only stock text labels and scrolling over MixerTextProjection; design tokens, themes, custom controls, and layout primitives do not exist in this phase", meta: rationale: "backend serialization and edit propagation must be proven before visual architecture is built"},
	{text: "--autopilot is a real end-to-end run that opens the actual window/audio device, drives W/S/A/D and K+direction through the live input facade, and asserts in code that serialized AppState, published parameters, and device-bound audio reflect the edits", meta: rationale: "the validation must fail on a disconnected UI even if the text renders correctly"},
	{text: "no validation may assert only on a token the binary prints unconditionally; a behavioral claim (audio plays, N strips render) must be checked in code against the real observed value so a regression makes the process exit non-zero", meta: rationale: "the autopilot's `autopilot complete:` token passed while only one channel rendered and nothing played — observable behavior must be asserted, not narrated"},
	{text: "the window backend must use a current eframe/egui (0.28+ on objc2 0.5+/winit 0.30+), never the 0.27 line that pulls icrate 0.0.4", meta: rationale: "eframe 0.27/objc2-0.3-beta/icrate-0.0.4 aborts at window creation on current macOS (NSScreen enumeration ABI panic); this is invisible to ui-smoke (no window) so it can only be prevented by the dependency pin, not the validation loop"},
]

project: invariants: diagnosticTextView: [
	{text: "the diagnostic view introduces no interactive primitive beyond the existing MixerView reducer", meta: rationale: "visual component architecture is deliberately deferred"},
	{text: "the renderer uses stock egui labels and scrolling with framework defaults", meta: rationale: "there is no theme, skin, token, or custom-widget work to validate yet"},
	{text: "all selection and adjustment behavior is proven by feeding MixerViewEvents into AppState with no window or device", meta: rationale: "the UI framework remains irrelevant to behavior"},
	{text: "selection appears only as the `>` prefix already contained in MixerTextProjection", meta: rationale: "rendering selection must not create a second focus model"},
	{text: "navigation is expressed as semantic events shared by the keyboard and gamepad adapters, never as raw device input reaching a primitive", meta: rationale: "controller/keyboard parity, identical to the MixerViewEvent precedent"},
]
