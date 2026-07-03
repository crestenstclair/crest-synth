package crestsynth

// ── Plugin ─────────────────────────────────────────────
// Plugin wrapper: exposes the Engine library as CLAP/VST3 plugins via nih-plug.
// Includes the nih-plug host adapter.
//
// Ported from the original spec's plugin increment (last committed form:
// `git show 3ed889f^1:spec/plugin.cue`, itself descended from the original
// `phase-10.cue`). Declarations and validations below are near-verbatim from
// that source; every adaptation (renamed/re-targeted resource IDs, and the
// Loop/RealTime integration that the original couldn't have specified because
// the Loop context didn't exist yet) is called out inline with a "PORTING
// NOTE" comment and cross-referenced in NOTES.md — read NOTES.md for the full
// rationale on each judgment call before merging this into spec/.

project: contexts: Plugin: purpose: "plugin wrapper: exposes the engine library as CLAP/VST3 plugins via nih-plug"
project: contexts: Plugin: ubiquitousLanguage: {
	PluginHost:      "the DAW or host application that loads the plugin"
	PluginParameter: "an engine parameter exposed to the host for automation"
	PluginFormat:    "the wire format: CLAP or VST3, abstracted by nih-plug"
}

// ── Value objects (verbatim) ────────────────────────────

project: contexts: Plugin: valueObjects: PluginFormat: {from: "enum", description: "CLAP, VST3"}
project: contexts: Plugin: valueObjects: ParameterId:  {from: "u32", description: "stable numeric ID for a plugin parameter, used by the host for automation"}
project: contexts: Plugin: valueObjects: ParameterRange: {
	state:       {min: "f64", max: "f64", defaultValue: "f64", step: "Option<f64>"}
	description: "value range and default for a host-visible parameter"
	invariants: ["min < max", "defaultValue must be within [min, max]", "a checked constructor rejects min>=max and a defaultValue outside [min,max] (returns Err/None), proven by a unit test"]
	validations: [{kind: "test", command: ["cargo", "test", "parameter_range"], description: "ParameterRange constructor rejects min>=max and out-of-range default"}]
}

// ── Port (verbatim) ─────────────────────────────────────

project: contexts: Plugin: ports: PluginHost: {
	contract: {processBlock: "(AudioBuffer, MidiEvents) -> AudioBuffer", getParameter: "ParameterId -> f64", setParameter: "(ParameterId, f64) -> ()", saveState: "() -> Vec<u8>", loadState: "Vec<u8> -> Result<(), StateError>"}
	meta: notes: "nih-plug provides the Plugin trait; this port maps to its process(), params(), and state methods. In tests, never `assert!` on a constant expression (e.g. `assert!(SOME_CONST > 0)`) — clippy::assertions_on_constants is denied under -D warnings; assert on runtime values or use a `const _: () = assert!(...)` compile-time check instead."
}

// ── Aggregate (verbatim state/commands/events/invariants/validations) ──
// PluginInstance is the plugin wrapper's OWN bookkeeping aggregate — its
// state (format, parameter list, patchCount, sampleRate) and its own
// command→event reducer are not part of the shared control plane the Loop
// context owns (Loop.AppState holds patches/mixer/editor/session state).
// It is the nih-plug-facing mirror of that state, kept in sync by PluginShell
// (see the PORTING NOTE on PluginShell below — the central judgment call of
// this port).

project: contexts: Plugin: aggregates: PluginInstance: {
	root:    true
	purpose: "wraps the engine library as a plugin: parameter mapping, state persistence, MIDI routing via host"
	state:   {format: "PluginFormat", parameters: "Vec<PluginParameter>", patchCount: "u8", sampleRate: "SampleRate"}
	commands: [
		{name: "Initialize", payload: {sampleRate: "SampleRate", maxBlockSize: "u32"}},
		{name: "Reset", payload: {}},
		{name: "SetParameter", payload: {id: "ParameterId", value: "f64"}},
	]
	events: [
		{name: "PluginInitialized", payload: {sampleRate: "SampleRate"}},
		{name: "PluginReset", payload: {}},
		{name: "ParameterChanged", payload: {id: "ParameterId", value: "f64"}},
	]
	invariants: [
		"plugin parameters map 1:1 to engine parameters",
		"state save/load uses the same PresetCodec as the standalone app",
		"MIDI events from the host are normalized through the same MidiNormalizer",
		"a unit test proves the command→event reducer: each command (Initialize/Reset/SetParameter) yields its corresponding event (PluginInitialized/PluginReset/ParameterChanged) with matching payload",
		"a unit test proves state round-trips through the PresetCodec: saveState then loadState reconstructs an equivalent PluginInstance (same parameters/values)",
	]
	validations: [{kind: "test", command: ["cargo", "test", "plugin_instance"], description: "PluginInstance command→event reducer + PresetCodec state round-trip tests pass"}]
	entities: PluginParameter: {state: {id: "ParameterId", name: "string", range: "ParameterRange", currentValue: "f64", engineMapping: "string"}}

	// PORTING NOTE (remap): "the same PresetCodec as the standalone app" and
	// "the same MidiNormalizer" are unchanged in wording but now resolve to
	// the clean base's port.Preset.PresetCodec (was port.Presets.PresetCodec
	// in the original's "Presets" context, renamed "Preset") and
	// domainService.Shell.MidiNormalizer (unchanged name/context, already
	// present on the clean base). No other IDs in this aggregate needed
	// remapping — SampleRate is Kernel.SampleRate on both old and new spec.
}

// ── Application service (adapted — see NOTES.md judgment call #1) ──────

project: contexts: Plugin: applicationServices: PluginShell: {
	purpose: "orchestrates plugin lifecycle: init, process, param sync, state persistence via the host"

	// PORTING NOTE / JUDGMENT CALL (flagged in NOTES.md #1): the original spec
	// had no Loop context — PluginShell was free-standing and its "param sync"
	// clause was left unspecified beyond this one-line purpose. The clean base
	// now funnels ALL control-plane mutation through Loop.AppState.apply (see
	// project.cue's core invariant "all control-plane state mutation flows
	// through the Loop reducer; views and adapters read state and emit
	// events, never mutate") and moves every audio-thread-readable value
	// across the boundary as a RealTime.ParameterSnapshot. Mirroring the
	// original's intent (host param changes reach the live engine; the
	// audio callback reads current values) through that architecture:
	//
	//   - PluginHost.setParameter(id, value) is received by PluginShell,
	//     which (a) applies PluginInstance's own SetParameter command (kept
	//     verbatim above, so PluginInstance's parameter list/current values
	//     and its saveState/loadState round-trip keep working exactly as
	//     originally specified) and (b) translates the SAME change into a
	//     Loop.AppEvent dispatched through aggregate.Loop.AppState.apply, so
	//     the live engine parameter actually updates through the one
	//     reducer the rest of the app uses — PluginShell is the ONLY place
	//     that touches both, so the two views of the value cannot diverge.
	//   - The plugin's real-time process() callback (PluginHost.processBlock)
	//     never reads PluginInstance state directly; like the standalone
	//     synth_ui shell, it reads the current values as a
	//     valueObject.RealTime.ParameterSnapshot via port.RealTime.
	//     ParameterBridge, published by domainService.Loop.StateProjector —
	//     the SAME lock-free seam the standalone app's audio thread uses.
	//   - saveState/loadState still go through port.Preset.PresetCodec
	//     exactly as the original specified (unchanged by this note).
	//
	// This was not written by the user for this architecture and is a
	// synthesized mapping of stated original intent onto Loop/RealTime — an
	// alternative (dropping PluginInstance's own reducer and reading its
	// parameter state straight from a Loop.AppState projection) was
	// considered and rejected because it would have rewritten the original's
	// verbatim aggregate rather than adapting only the integration seam.
	uses: [
		"aggregate.Plugin.PluginInstance",
		"aggregate.Loop.AppState",
		"valueObject.Loop.AppEvent",
		"domainService.Loop.StateProjector",
		"port.RealTime.ParameterBridge",
		"valueObject.RealTime.ParameterSnapshot",
		"port.Preset.PresetCodec",
		"domainService.Shell.MidiNormalizer",
	]
}

// ── Infrastructure adapter (implements PluginHost) ─────
// PORTING NOTE (mechanical): the clean base's adapter convention puts the
// crate name in `meta: framework: "..."` (see e.g. shell.cue's CpalAudioOutput),
// not in prose under `meta: notes:` as the original did. Adapted to that
// convention; no semantic change.

project: adapters: NihPlugHost: {implements: "port.Plugin.PluginHost", layer: "infrastructure", meta: framework: "nih-plug"}

// ── Invariants (verbatim) ───────────────────────────────
// project: invariants: is a named-group struct (each spec file contributes
// its own key), so this new "pluginCompat" group unifies safely alongside
// the clean base's existing groups (core, shellDesign, ...) without touching
// project.cue.

project: invariants: pluginCompat: [
	{text: "plugin state save/load uses the same PresetCodec as the standalone for format compatibility", meta: rationale: "presets created in standalone should load in the plugin and vice versa"},
	{text: "plugin parameters have stable numeric IDs across versions for host automation compatibility", meta: rationale: "changing parameter IDs breaks saved automation in DAW projects"},
]

// ── Context map (NOT applied here — see NOTES.md judgment call #2) ─────
// The original also declared:
//   project: contextMap: pluginToSynth:   {from: "Plugin", to: "Synth",   kind: "customer-supplier", direction: "downstream"}
//   project: contextMap: pluginToPatch:   {from: "Plugin", to: "Patch",   kind: "customer-supplier", direction: "downstream"}
//   project: contextMap: pluginToPresets: {from: "Plugin", to: "Presets", kind: "customer-supplier", direction: "downstream"}
// These were dropped from the original spec itself in the historical
// domain-grouped refactor (commit c385d72) because project.cue's contextMap
// is a plain CUE list (`project: contextMap: [...]`), and a list cannot be
// unified additively from a second file the way a keyed struct (like
// invariants) can — attempting `project: contextMap: pluginToSynth: {...}`
// against a list field is a type conflict. Re-adding the remapped
// equivalents (Plugin→Engine, Plugin→Patch, Plugin→Preset, all
// customer-supplier/downstream) requires editing spec/project.cue's
// contextMap list directly, which is out of scope for this file (deliverable
// constraint: do not touch spec/). Left as a to-do for whoever merges this
// increment; see NOTES.md judgment call #2 for the exact list literal to add.
