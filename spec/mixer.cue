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
	state: {inputGain: "Amplitude", inserts: "EffectChain", volumeDb: "Decibel", pan: "Pan", mute: "bool", solo: "bool", sends: "list<SendTap>", peak: "PeakLevel"}
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
		"peak metering is measured after strip processing and before mute/solo audibility gating, so every input remains observable",
	]
	validations: [{kind: "test", command: ["cargo", "test", "channel_strip"], description: "ChannelStrip unit tests pass"}]
	contributesTo: [{capability: "capability.stereo_mix_pipeline", contribution: "provides independent gain, inserts, pan, mute, solo, sends, and pre-gate metering for one patch"}]
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
	contributesTo: [{capability: "capability.stereo_mix_pipeline", contribution: "owns aux and master summing, ordered processing, and final output limiting"}]
}

project: contexts: Mixer: domainServices: {
	MixEngine: {
		purpose: "one full mix pass: render strips, collect send taps into aux buses, process aux inserts, sum into the master bus, process master inserts and the limiter"
		uses: ["aggregate.Mixer.ChannelStrip", "aggregate.Mixer.MixBus", "domainService.Effects.ChainRenderer"]
		validations: [{kind: "test", command: ["cargo", "test", "mix_engine"], description: "strip, sends, aux, master, limiter, solo, and metering order are correct"}]
		contributesTo: [{capability: "capability.stereo_mix_pipeline", contribution: "coordinates the complete strip-to-aux-to-master signal path and final limiting"}]
	}
}

project: contexts: Mixer: applicationServices: {
	MixerController: {
		purpose: "application-level mixer operations: strip CRUD, solo group handling, bus management"
		uses: ["aggregate.Mixer.ChannelStrip", "aggregate.Mixer.MixBus"]
		operations: {
			setSolo: {input: {strip: "u32", solo: "bool"}}
		}
		validations: [{kind: "test", command: ["cargo", "test", "mixer_controller"], description: "solo groups and bus operations preserve mixer invariants"}]
		contributesTo: [{capability: "capability.stereo_mix_pipeline", contribution: "coordinates user-visible strip solo and bus operations without bypassing mixer invariants"}]
	}
}

// Solo is exclusive within a mix group: when any strip is soloed, all
// non-soloed strips are muted. Enforced by MixerController, observable at
// MixEngine output.

// The Mixer VIEW is the current GUI: a controller/keyboard-driven view
// over the channel-strip mixer (aggregate.Mixer.ChannelStrip). Like the
// Editor view, it is a one-way (Flux) store — MixerView — with a single
// mutation entry point that reduces semantic MixerViewEvents. The egui/synth_ui
// shell and the gamepad adapter both emit the SAME events, so the whole control
// plane (track/parameter cursor, edit-mode, fine/coarse adjust,
// double-tap toggle) is hermetically testable with no window and no device.
//
// All 16 tracks are visible at once as narrow terminal-like columns. A cursor
// selects one (track, parameter) cell and a derived inspector shows its patch,
// instrument, value, mute, and solo state. The view edits canonical
// ChannelStrips; metering is read back from them.
//
// It deliberately reuses ChannelStrip; there is no ChannelMixer, PatchMixer,
// GlobalMixer, or parallel strip model. ReverbSend/EchoSend address sends 0/1.

project: contexts: Mixer: ubiquitousLanguage: {
	MixerView:      "the single store for the mixer view: owns the track/parameter cursor, edit-mode flag, sixteen canonical ChannelStrips, and their patch/instrument labels"
	MixerViewEvent: "a semantic input event (navigate, edit-mode change, or toggle) emitted by the keyboard/gamepad adapter — the only thing that mutates the mixer view"
	MixerParam:     "which of a track's six parameter rows the cursor is on: Volume, ReverbSend, EchoSend, Pan, Mute, Solo"
	TrackCode:      "the stable two-hex-digit mixer label T00 through T0F"
	Inspector:      "a derived textual panel for the selected track, patch/instrument, parameter value, mute, and solo state"
}

project: contexts: Mixer: valueObjects: MixerTrackLabel: {
	state: {track: "u8", code: "string", patchId: "option<PatchId>", instrument: "string"}
	description: "compact mixer header data: T00-T0F plus the assigned patch/instrument label, or an explicit empty marker"
	invariants: ["track is 0..=15", "code is uppercase T00 through T0F and is derived from track", "instrument is terminal-safe single-line text"]
	validations: [{kind: "test", command: ["cargo", "test", "mixer_track_label"], description: "all sixteen stable codes and empty/assigned labels render deterministically"}]
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
	purpose: "the terminal-style all-tracks mixer store: owns the cursor, labels, edit mode, and sixteen canonical ChannelStrips behind one reducer"
	state: {tracks: "[ChannelStrip; 16]", labels: "[MixerTrackLabel; 16]", cursorTrack: "usize", cursorParam: "MixerParam", editMode: "bool"}
	invariants: [
		"apply(MixerViewEvent) is the ONLY way to mutate the mixer view",
		"the view contains exactly 16 ChannelStrip tracks and all sixteen T00-T0F columns are present in the initial layout with no horizontal paging",
		"cursorTrack stays in 0..=15; navigation saturates at T00 and T0F",
		"the initial selection is T00 Volume and the derived inspector and bottom status row describe that same selection",
		"in navigate mode NavUp/NavDown move the parameter row and NavLeft/NavRight move between tracks",
		"in edit mode on a continuous param, Left/Right adjust by the fine step and Up/Down by the coarse step (= 10x fine), clamped by the addressed ChannelStrip",
		"toggle params (Mute/Solo) change only via ToggleFocusedParam (double-tap Edit), never via directional input",
		"EnterEditMode alone changes no parameter value (it is a no-op until directional input arrives)",
		"volume and sends display compact 00-7F control values, pan displays L63..C..R63, and the domain values remain Decibel/Amplitude/Pan rather than hexadecimal storage",
		"the selected-track inspector is derived from tracks, labels, cursorTrack, and cursorParam and never owns mutable duplicate state",
	]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with MixerView"},
		{kind: "test", command: ["cargo", "test", "mixer_view"], description: "all-track layout state, saturating navigation, inspector projection, edit-mode, fine/coarse, and toggle tests pass"},
	]
	contributesTo: [{capability: "capability.pointer_free_mixer_control", contribution: "provides the all-sixteen-track terminal-style keyboard/gamepad mixer journey"}]
}

// ── Invariants ─────────────────────────────────────────

project: invariants: mixerView: [
	{text: "the mixer skin is a pure view over the MixerView inside AppState; it requests changes only by emitting AppEvent::Mixer into AppState.apply and never mutates MixerView or ChannelStrip directly", meta: rationale: "one authoritative reducer makes live input and scene replay identical"},
	{text: "double-tap detection and Edit-hold timing live in the input adapter, which emits clean semantic events (ToggleFocusedParam / EnterEditMode / ExitEditMode); the store is timing-free", meta: rationale: "keeps MixerView a pure reducer that unit tests can drive with event sequences"},
	{text: "metering is independent of solo and mute: a channel silenced by another channel's solo still meters its own level", meta: rationale: "the volume strip doubles as the channel's peak meter and must show real signal even when inaudible"},
	{text: "keyboard and gamepad emit identical MixerViewEvents, so the two input paths are interchangeable", meta: rationale: "controller-first parity with the rest of the app"},
]
