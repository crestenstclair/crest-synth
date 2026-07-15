package crestsynth

// Mixer — channel strips, buses, and sends. Standard console architecture:
// strip (input gain → inserts → volume/pan → send taps) → bus → master.

project: contexts: Mixer: purpose: "mixing console: per-patch channel strips with insert FX and sends, aux buses with returns, and a master bus with a limiter"

project: contexts: Mixer: valueObjects: {
	SendTap: {
		state: {bus: "BusId", level: "Amplitude", preFader: "bool"}
		description: "one send from a strip to an aux bus"
	}
	PeakLevel: {from: "f64", description: "most recent peak absolute sample level for metering", invariants: ["must be non-negative"]}
}

project: contexts: Mixer: aggregates: ChannelStrip: {
	root:    true
	purpose: "one patch's channel: input gain, insert chain, volume, pan, mute/solo, and up to 8 send taps"
	state: {inputGain: "Amplitude", volumeDb: "Decibel", pan: "Pan", mute: "bool", solo: "bool", sends: "list<SendTap>", peak: "PeakLevel"}
	commands: {
		SetVolume: {volumeDb: "Decibel"}
		SetPan: {pan: "Pan"}
		SetMute: {mute: "bool"}
		SetSolo: {solo: "bool"}
		SetSend: {index: "u8", tap: "SendTap"}
	}
	events: {
		LevelChanged: {volumeDb: "Decibel"}
		SoloChanged: {solo: "bool"}
	}
	invariants: [
		"a strip has at most 8 send taps",
		"peak metering reflects the level after volume and pan are applied",
	]
	validations: [{kind: "test", command: ["cargo", "test", "channel_strip"], description: "ChannelStrip unit tests pass"}]
	contributesTo: [{capability: "capability.mix_to_stereo", contribution: "provides independent gain, inserts, pan, mute, solo, sends, and metering for one patch"}]
}

project: contexts: Mixer: aggregates: MixBus: {
	root:    true
	purpose: "a summing bus: aux buses receive send taps and return to the master; the master bus is the final summing point with its own inserts and limiter"
	state: {id: "BusId", returnLevel: "Amplitude"}
	commands: {
		SetReturnLevel: {level: "Amplitude"}
	}
	events: {
		ReturnLevelChanged: {level: "Amplitude"}
	}
	invariants: [
		"bus 0 is the master bus and cannot be removed",
		"aux buses feed the master bus, never each other",
	]
	validations: [{kind: "test", command: ["cargo", "test", "mix_bus"], description: "MixBus unit tests pass"}]
	contributesTo: [{capability: "capability.mix_to_stereo", contribution: "owns aux and master summing, ordered processing, and final output limiting"}]
}

project: contexts: Mixer: domainServices: {
	MixEngine: {
		purpose: "one full mix pass: render strips, collect send taps into aux buses, process aux inserts, sum into the master bus, process master inserts and the limiter"
		uses: ["aggregate.Mixer.ChannelStrip", "aggregate.Mixer.MixBus", "domainService.Effects.ChainRenderer"]
		contributesTo: [{capability: "capability.mix_to_stereo", contribution: "coordinates the complete strip-to-aux-to-master signal path and final limiting"}]
	}
}

project: contexts: Mixer: applicationServices: {
	MixerController: {
		purpose: "application-level mixer operations: strip CRUD, solo group handling, bus management"
		uses: ["aggregate.Mixer.ChannelStrip", "aggregate.Mixer.MixBus"]
		operations: {
			setSolo: {input: {strip: "u32", solo: "bool"}}
		}
		contributesTo: [{capability: "capability.mix_to_stereo", contribution: "coordinates user-visible strip solo and bus operations without bypassing mixer invariants"}]
	}
}

// Solo is exclusive within a mix group: when any strip is soloed, all
// non-soloed strips are muted. Enforced by MixerController, observable at
// MixEngine output.

// ── Mixer additions (editor increment) ──────────────────
// The Mixer VIEW: the first real GUI view. A controller/keyboard-driven view
// over the channel-strip mixer (aggregate.Mixer.ChannelStrip). Like the
// Editor view, it is a one-way (Flux) store — MixerView — with a single
// mutation entry point that reduces semantic MixerViewEvents. The egui/synth_ui
// shell and the gamepad adapter both emit the SAME events, so the whole control
// plane (cursor, viewport edge-scrolling, edit-mode, fine/coarse adjust,
// double-tap toggle) is hermetically testable with no window and no device.
//
// 16 channels exist; 6 are visible at once. A cursor selects one (channel,
// parameter) cell; the remaining channels are reached by edge-scrolling the
// viewport. The view edits the ChannelStrip channels; metering is read back
// from them.
//
// PORTING NOTE (see NOTES.md for detail): the original spec's mixer view sat
// on top of a single `aggregate.Patch.ChannelMixer` — a bespoke 16-slot array
// aggregate with its own ChannelStrip value object (volume/reverbSend/echoSend/
// pan/mute/solo as raw f64 0.0-1.0 fields) and a sibling `GlobalMixer`/
// `PatchMixer`. None of those three exist on the clean base. The clean base
// already has an analogous — but architecturally different — `ChannelStrip`
// AGGREGATE (one per patch: inputGain/insertChain/volumeDb/pan/mute/solo/sends/
// peak) plus a `MixBus` aggregate (bus 0 = master) and a `MixEngine` domain
// service that performs the mix pass. This file re-targets MixerView at that
// existing machinery instead of re-declaring ChannelMixer: MixerView now wraps
// a fixed list of 16 `aggregate.Mixer.ChannelStrip` instances (preserving the
// original's "16 channels, 6 visible" invariant at the view layer, since no
// aggregate on the clean base fixes the count at 16 by itself). The two
// continuous send rows (ReverbSend/EchoSend) are kept as named MixerParam rows
// verbatim from the original and map onto the first two entries of each
// ChannelStrip's `sends: list<SendTap>` (SendTap already has `bus` + `level` +
// `preFader`, a superset of the original's raw two-float sends).

project: contexts: Mixer: ubiquitousLanguage: {
	MixerView:      "the single store for the mixer view: owns the cursor (channel + parameter row), the viewport offset, the edit-mode flag, and the 16 ChannelStrip channels it edits"
	MixerViewEvent: "a semantic input event (navigate, edit-mode change, or toggle) emitted by the keyboard/gamepad adapter — the only thing that mutates the mixer view"
	MixerParam:     "which of a channel strip's six parameter rows the cursor is on: Volume, ReverbSend, EchoSend, Pan, Mute, Solo (top to bottom)"
	Viewport:       "the window of 6 contiguous channels currently visible; edge-scrolls by one when the cursor pushes past a visible edge"
}

project: contexts: Mixer: valueObjects: MixerParam: {
	from:        "enum"
	description: "Volume, ReverbSend, EchoSend, Pan, Mute, Solo — the six parameter rows of a channel strip, in top-to-bottom navigation order. Volume/ReverbSend/EchoSend/Pan are CONTINUOUS; Mute/Solo are TOGGLES. ReverbSend and EchoSend address ChannelStrip.sends[0] and ChannelStrip.sends[1] respectively."
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
	purpose: "the single mixer-view store: owns cursor (channel + parameter), viewport offset, edit-mode, and the 16 ChannelStrip channels; the one entry point that reacts to MixerViewEvents"
	state: {channels: "[ChannelStrip; 16]", cursorChannel: "usize", cursorParam: "MixerParam", viewportOffset: "usize", editMode: "bool"}
	invariants: [
		"apply(MixerViewEvent) is the ONLY way to mutate the mixer view",
		"the view wraps exactly 16 ChannelStrip channels; MixerViewEvents addressed at an out-of-range channel index are ignored",
		"exactly 6 channels are visible; the cursor is ALWAYS within the visible window [viewportOffset, viewportOffset+5]",
		"cursorChannel stays in 0..=15 and viewportOffset stays in 0..=10",
		"in navigate mode NavUp/NavDown move the parameter row (saturating) and NavLeft/NavRight move between channels",
		"at a visible edge, pressing toward it scrolls the viewport by one and keeps the cursor on the SAME absolute channel (now one position inward); mirror at the opposite edge; no-op at channel 1 / channel 16",
		"in edit mode on a continuous param, Left/Right adjust by the fine step and Up/Down by the coarse step (= 10x fine), clamped by the addressed ChannelStrip",
		"toggle params (Mute/Solo) change only via ToggleFocusedParam (double-tap Edit), never via directional input",
		"EnterEditMode alone changes no parameter value (it is a no-op until directional input arrives)",
	]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with MixerView"},
		{kind: "test", command: ["cargo", "test", "mixer_view"], description: "MixerView reducer unit tests pass (nav, edge-scroll, edit-mode, fine/coarse, toggle)"},
	]
	contributesTo: [{capability: "capability.edit_without_pointer", contribution: "provides the six-strip, keyboard/gamepad-driven mixer editing journey"}]
}

// ── Invariants ─────────────────────────────────────────

project: invariants: mixerView: [
	{text: "the mixer view is a pure view over its 16 ChannelStrip channels; it mutates state only by emitting MixerViewEvents applied to MixerView, never by touching a ChannelStrip's fields directly from draw code", meta: rationale: "one-way data flow keeps the control plane hermetically testable, identical to the Editor view"},
	{text: "double-tap detection and Edit-hold timing live in the input adapter, which emits clean semantic events (ToggleFocusedParam / EnterEditMode / ExitEditMode); the store is timing-free", meta: rationale: "keeps MixerView a pure reducer that unit tests can drive with event sequences"},
	{text: "metering is independent of solo and mute: a channel silenced by another channel's solo still meters its own level", meta: rationale: "the volume strip doubles as the channel's peak meter and must show real signal even when inaudible"},
	{text: "keyboard and gamepad emit identical MixerViewEvents, so the two input paths are interchangeable", meta: rationale: "controller-first parity with the rest of the app"},
]
