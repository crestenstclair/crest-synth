package crestsynth

// ── Mixer ──────────────────────────────────────────────
// The Mixer VIEW: the first real GUI view. A controller/keyboard-driven view
// over the 16-channel audio mixer (aggregate.Patch.ChannelMixer). Like the
// Editor view, it is a one-way (Flux) store — MixerView — with a single
// mutation entry point that reduces semantic MixerViewEvents. The egui/synth_ui
// shell and the gamepad adapter both emit the SAME events, so the whole control
// plane (cursor, viewport edge-scrolling, edit-mode, fine/coarse adjust,
// double-tap toggle) is hermetically testable with no window and no device.
//
// 16 channels exist; 6 are visible at once. A cursor selects one (channel,
// parameter) cell; the remaining channels are reached by edge-scrolling the
// viewport. The view edits the ChannelMixer; metering is read back from it.

project: contexts: Mixer: purpose: "the mixer GUI view: a one-way event-loop store over the 16-channel ChannelMixer (cursor, 6-of-16 viewport with edge scrolling, edit-mode, fine/coarse adjust, double-tap toggle)"
project: contexts: Mixer: ubiquitousLanguage: {
	MixerView:      "the single store for the mixer view: owns the cursor (channel + parameter row), the viewport offset, the edit-mode flag, and the ChannelMixer it edits"
	MixerViewEvent: "a semantic input event (navigate, edit-mode change, or toggle) emitted by the keyboard/gamepad adapter — the only thing that mutates the mixer view"
	MixerParam:     "which of a channel strip's six parameter rows the cursor is on: Volume, ReverbSend, EchoSend, Pan, Mute, Solo (top to bottom)"
	Viewport:       "the window of 6 contiguous channels currently visible; edge-scrolls by one when the cursor pushes past a visible edge"
}

project: contexts: Mixer: valueObjects: MixerParam: {
	from:        "enum"
	description: "Volume, ReverbSend, EchoSend, Pan, Mute, Solo — the six parameter rows of a channel strip, in top-to-bottom navigation order. Volume/ReverbSend/EchoSend/Pan are CONTINUOUS; Mute/Solo are TOGGLES."
	invariants: ["the row order is Volume, ReverbSend, EchoSend, Pan, Mute, Solo", "Volume, ReverbSend, EchoSend, Pan are continuous; Mute and Solo are toggles"]
	validations: [{kind: "compiles", command: ["cargo", "build"], description: "crate builds with MixerParam"}]
}

project: contexts: Mixer: valueObjects: MixerViewEvent: {
	from:        "enum"
	description: "NavUp, NavDown, NavLeft, NavRight, EnterEditMode, ExitEditMode, ToggleFocusedParam — the semantic input vocabulary of the mixer view. Keyboard and gamepad adapters both emit ONLY these. EnterEditMode/ExitEditMode track the Edit modifier (J / a face button) hold; ToggleFocusedParam is emitted by the adapter on a DOUBLE-TAP of Edit (the timing/double-tap detection lives in the adapter, never in the store)."
	validations: [{kind: "compiles", command: ["cargo", "build"], description: "crate builds with MixerViewEvent"}]
}

project: contexts: Mixer: aggregates: MixerView: {
	root:    true
	purpose: "the single mixer-view store: owns cursor (channel + parameter), viewport offset, edit-mode, and the ChannelMixer; the one entry point that reacts to MixerViewEvents"
	state: {mixer: "ChannelMixer", cursorChannel: "usize", cursorParam: "MixerParam", viewportOffset: "usize", editMode: "bool"}
	invariants: [
		"apply(MixerViewEvent) is the ONLY way to mutate the mixer view",
		"exactly 6 channels are visible; the cursor is ALWAYS within the visible window [viewportOffset, viewportOffset+5]",
		"cursorChannel stays in 0..=15 and viewportOffset stays in 0..=10",
		"in navigate mode NavUp/NavDown move the parameter row (saturating) and NavLeft/NavRight move between channels",
		"at a visible edge, pressing toward it scrolls the viewport by one and keeps the cursor on the SAME absolute channel (now one position inward); mirror at the opposite edge; no-op at channel 1 / channel 16",
		"in edit mode on a continuous param, Left/Right adjust by the fine step and Up/Down by the coarse step (= 10x fine), clamped by the ChannelMixer",
		"toggle params (Mute/Solo) change only via ToggleFocusedParam (double-tap Edit), never via directional input",
		"EnterEditMode alone changes no parameter value (it is a no-op until directional input arrives)",
	]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with MixerView"},
		{kind: "test", command: ["cargo", "test", "mixer_view"], description: "MixerView reducer unit tests pass (nav, edge-scroll, edit-mode, fine/coarse, toggle)"},
	]
}

// ── Headless mixer-view prover ─────────────────────────
// mixer_demo proves the mixer view's control logic AND the ChannelMixer's
// solo/metering semantics WITHOUT any device or window: it scripts MixerViewEvent
// sequences through MixerView and asserts the cursor / viewport / values, then
// mixes per-channel buffers through the ChannelMixer to prove solo silences other
// channels' AUDIO while every channel still meters its own level. The egui/gilrs
// adapters open real windows/devices, so they are NEVER invoked here.

project: assets: MixerDemoMain: {
	kind:        "rust-bin-target"
	description: "src/bin/mixer_demo.rs: headless prover for MixerView + ChannelMixer — scripted events drive cursor/viewport/edit, and a mixdown proves solo-vs-metering independence"
	uses: ["aggregate.Mixer.MixerView", "valueObject.Mixer.MixerViewEvent", "valueObject.Mixer.MixerParam", "aggregate.Patch.ChannelMixer", "valueObject.Patch.ChannelStrip"]
	prompts: [
		"File path: src/bin/mixer_demo.rs",
		"CLI: `mixer_demo`. Takes no arguments and opens NO device and NO window — it is a headless harness over the Mixer view store and the ChannelMixer domain. Do NOT import gilrs, egui, eframe, cpal, or midir.",
		#"EDGE-SCROLL PROOF: build a MixerView (16 channels, 6 visible). Put the cursor on the trailing visible channel (channel index 5) with viewportOffset 0, then feed NavRight and assert IN CODE that viewportOffset becomes 1 AND cursorChannel stays 5 (panic with a clear message on mismatch). Feed a few more NavRights and assert the viewport keeps scrolling while the cursor never leaves the visible window. Then mirror at the left edge (NavLeft scrolls the viewport back, cursor stays). Print a verbatim line `edge scroll ok`."#,
		#"FINE/COARSE PROOF: navigate to a continuous row (e.g. Volume) on some channel, EnterEditMode, then assert NavRight raises that channel's volume by the fine step (0.01) and NavUp raises it by the coarse step (0.10), and that values clamp at 1.0 (repeated NavUp never exceeds 1.0) and 0.0. Read the values back from the ChannelMixer. Panic on mismatch. Print a verbatim line `fine/coarse ok`."#,
		#"TOGGLE PROOF: navigate to the Mute row of a channel, feed ToggleFocusedParam and assert the channel's mute flips true; feed it again and assert it flips back. Then EnterEditMode on the Mute row and feed NavRight/NavUp and assert the mute flag is UNCHANGED (directional input is a no-op on a toggle). Panic on mismatch. Print a verbatim line `toggle ok`."#,
		#"SOLO-VS-METERING PROOF (this is the one that catches a wrong gating implementation): on a fresh ChannelMixer, build 16 small per-channel audio buffers each with a clearly non-zero signal. Solo exactly one channel (e.g. channel 3) via the view (navigate to its Solo row + ToggleFocusedParam). Run the ChannelMixer mixdown over the 16 buffers. Assert IN CODE: (a) the stereo mix equals ONLY channel 3's contribution — every other channel added zero (compare against a reference mix of channel 3 alone); (b) EVERY channel's peak level is still > 0, including the solo-silenced ones (so a channel inaudible due to another's solo still meters its own level). Panic with a clear message on either failure. Print the verbatim lines `solo mutes others: true` and `metering independent of solo: true`."#,
		"Print a short summary. The tokens `edge scroll ok`, `fine/coarse ok`, `toggle ok`, `solo mutes others: true`, and `metering independent of solo: true` MUST appear verbatim so a validation can assert each behavior ran correctly with no device.",
		"Exit 0 on success (all in-code assertions must pass).",
	]
	validations: [
		{kind: "compiles", command: ["make", "build"], description: "mixer demo builds"},
		{kind: "integration", command: ["make", "demo-mixer"], description: "scripted mixer-view events drive cursor/viewport/edit/toggle, and the mixdown proves solo silences other channels' audio but not their metering — no device", assertions: [
			{kind: "exit_code", expected: 0},
			{kind: "stdout_contains", pattern: "edge scroll ok"},
			{kind: "stdout_contains", pattern: "fine/coarse ok"},
			{kind: "stdout_contains", pattern: "toggle ok"},
			{kind: "stdout_contains", pattern: "solo mutes others: true"},
			{kind: "stdout_contains", pattern: "metering independent of solo: true"},
		]},
	]
}

// ── Invariants ─────────────────────────────────────────

project: invariants: mixerView: [
	{text: "the mixer view is a pure view over the ChannelMixer; it mutates state only by emitting MixerViewEvents applied to MixerView, never by touching mixer values directly from draw code", meta: rationale: "one-way data flow keeps the control plane hermetically testable, identical to the Editor view"},
	{text: "double-tap detection and Edit-hold timing live in the input adapter, which emits clean semantic events (ToggleFocusedParam / EnterEditMode / ExitEditMode); the store is timing-free", meta: rationale: "keeps MixerView a pure reducer that unit tests can drive with event sequences"},
	{text: "metering is independent of solo and mute: a channel silenced by another channel's solo still meters its own level", meta: rationale: "the volume strip doubles as the channel's peak meter and must show real signal even when inaudible"},
	{text: "keyboard and gamepad emit identical MixerViewEvents, so the two input paths are interchangeable", meta: rationale: "controller-first parity with the rest of the app"},
]
