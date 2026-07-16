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

// The current Mixer VIEW is a disposable backend diagnostic, not a designed
// control surface. It prints every Patch and its canonical mixer values as a
// vertical wall of text. The egui shell uses only default labels and a scroll
// area; all behavior lives in MixerView/AppState and is proven headlessly.
//
// It deliberately reuses ChannelStrip; there is no ChannelMixer, PatchMixer,
// GlobalMixer, or parallel strip model. ReverbSend/EchoSend address sends 0/1.

project: contexts: Mixer: ubiquitousLanguage: {
	MixerView: "the host-neutral navigation state over canonical Patch and ChannelStrip values; it contains no rendering or styled-widget state"
	MixerViewEvent: "Navigate(direction) or Adjust(direction), emitted after the input adapter interprets plain W/S/A/D versus K+W/S/A/D"
	MixerParam: "which serialized Patch mixer value is selected: Volume, ReverbSend, EchoSend, Pan, Mute, Solo"
	MixerTextProjection: "a deterministic wall-of-text rendering of canonical AppState and its StateSnapshot hash"
}

project: contexts: Mixer: valueObjects: MixerTextProjection: {
	state: {body: "string", patchCount: "usize", selectedPatch: "option<PatchId>", selectedParam: "MixerParam", stateSnapshotHash: "string"}
	description: "the complete plain-text backend view served to any shell; it is derived from canonical AppState and never stores editable values"
	invariants: [
		"body begins with serialized TestPlayback status, positionSeconds, and soundFontPath followed by the literal `KEYS: W/S values | A/D patches | K+direction edit | L start/stop from start` reminder",
		"every Patch appears exactly once in stable AppState order",
		"each Patch block contains id, name, mixerStrip, volume, reverbSend, echoSend, pan, mute, and solo using the same serialized values encoded in StateSnapshot",
		"Patch blocks are separated by the literal ASCII rule `------------------------------------------------------------`",
		"exactly one selected value is prefixed by `>` when any Patch exists; all other value lines begin with a space",
		"stateSnapshotHash equals the hash of the canonical StateSnapshot from which body was projected",
	]
	validations: [{kind: "test", command: ["cargo", "test", "mixer_text_projection"], description: "multi-patch text, separators, selection, exact serialized values, and snapshot hash are deterministic"}]
}

project: contexts: Mixer: valueObjects: MixerParam: {
	from:        "enum"
	description: "Volume, ReverbSend, EchoSend, Pan, Mute, Solo — the six parameter rows of a channel strip, in top-to-bottom navigation order. Volume/ReverbSend/EchoSend/Pan are CONTINUOUS; Mute/Solo are TOGGLES. ReverbSend and EchoSend address ChannelStrip.sends[0] and ChannelStrip.sends[1] respectively."
	invariants: ["the row order is Volume, ReverbSend, EchoSend, Pan, Mute, Solo", "Volume, ReverbSend, EchoSend, Pan are continuous; Mute and Solo are toggles"]
	validations: [{kind: "compiles", command: ["cargo", "build"], description: "crate builds with MixerParam"}]
}

project: contexts: Mixer: valueObjects: MixerViewEvent: {
	from:        "enum"
	description: "NavigateUp, NavigateDown, NavigateLeft, NavigateRight, AdjustUp, AdjustDown, AdjustLeft, AdjustRight. Bare W/S/A/D and d-pad emit Navigate; holding K or the gamepad edit modifier emits Adjust. The reducer never receives raw key state."
	validations: [{kind: "compiles", command: ["cargo", "build"], description: "crate builds with MixerViewEvent"}]
}

project: contexts: Mixer: aggregates: MixerView: {
	root:    true
	purpose: "the host-neutral backend mixer store: owns sixteen canonical ChannelStrips and a Patch/parameter selection behind one reducer"
	state: {tracks: "[ChannelStrip; 16]", selectedPatch: "option<PatchId>", cursorParam: "MixerParam"}
	invariants: [
		"AppState.apply(AppEvent::Mixer(MixerViewEvent)) is the only mutation path and atomically resolves selectedPatch against AppState.patches before touching its assigned ChannelStrip",
		"the view contains exactly 16 canonical ChannelStrip tracks; multiple Patches may share a track and every Patch remains independently listed",
		"NavigateUp/NavigateDown move through MixerParam in declared order; NavigateLeft/NavigateRight move through stable AppState Patch order; all navigation saturates",
		"the initial selection is the first Patch in AppState order and Volume, or no Patch when the collection is empty",
		"AdjustLeft/AdjustRight decrement/increment a continuous value by its fine step; AdjustDown/AdjustUp decrement/increment by ten fine steps; every result clamps",
		"for Mute and Solo, AdjustLeft/AdjustDown set false and AdjustRight/AdjustUp set true; bare navigation never changes a value",
		"pressing K without a direction emits no event and changes nothing",
		"MixerView contains no copied Patch name, serialized string, widget, panel, meter, scroll, color, or layout state",
	]
	validations: [
		{kind: "compiles", command: ["cargo", "build"], description: "crate builds with MixerView"},
		{kind: "test", command: ["cargo", "test", "mixer_view"], description: "Patch/parameter navigation, K-modified fine/coarse adjustment, boolean setting, clamping, shared tracks, and empty state pass"},
	]
	contributesTo: [{capability: "capability.pointer_free_mixer_control", contribution: "provides backend Patch selection and typed adjustment semantics independently of the disposable text renderer"}]
}

// ── Invariants ─────────────────────────────────────────

project: invariants: mixerView: [
	{text: "the text renderer receives only MixerTextProjection and emits MixerViewEvents; it never mutates AppState, MixerView, Patch, or ChannelStrip directly", meta: rationale: "the disposable shell must prove the backend rather than become another backend"},
	{text: "K-modifier state lives in the input adapter, which converts K+direction into Adjust and bare direction into Navigate; MixerView is timing- and device-free", meta: rationale: "the reducer remains hermetically testable with semantic event sequences"},
	{text: "metering is independent of solo and mute: a channel silenced by another channel's solo still meters its own level", meta: rationale: "the volume strip doubles as the channel's peak meter and must show real signal even when inaudible"},
	{text: "keyboard and gamepad emit identical MixerViewEvents, so the two input paths are interchangeable", meta: rationale: "controller-first parity with the rest of the app"},
]
