package crestsynth

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
// See docs/superpowers/specs/2026-06-14-designsystem-architecture-design.md.

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
// The consistency rules every primitive added later must honor. They are
// forward-looking by design — no primitives exist yet — and mirror the
// invariant style already used in mixer.cue.

project: invariants: designSystem: [
	{text: "every interactive primitive is a pure reducer (State + Event + apply); its behavior layer never touches egui, performs no I/O, and is allocation-free", meta: rationale: "behavior must be unit-testable headlessly and reusable across any renderer — the same discipline MixerView already follows"},
	{text: "a skin reads colors only by resolving a SemanticToken through the Theme port; no literal color value or hard-coded size appears in draw code", meta: rationale: "this single seam is what lets the whole app be restyled by swapping the Theme, with zero behavior change (the JUCE LookAndFeel property)"},
	{text: "every primitive's behavior is proven by feeding Events and asserting State with no window and no device, exactly as the mixer demo proves MixerView", meta: rationale: "keeps the control plane hermetically testable and decoupled from egui/gilrs/cpal"},
	{text: "a primitive reports focus only via its view-model flags (focused, editing); it never decides where focus is — the consuming view owns cursor traversal and edit-mode, the skin renders the flags via FocusRing/EditActive", meta: rationale: "traversal rules are layout-specific (a 2D channel×param grid scrolls unlike a 1D list), so views own traversal while focus still LOOKS identical everywhere through two tokens"},
	{text: "navigation is expressed as semantic events shared by the keyboard and gamepad adapters, never as raw device input reaching a primitive", meta: rationale: "controller/keyboard parity, identical to the MixerViewEvent precedent"},
]
