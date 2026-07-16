package crestsynth

// RealTime — the lock-free seam between the audio thread and everything else.
// Everything crossing the boundary goes through one of these three ports.

project: contexts: RealTime: purpose: "the lock-free boundary: discrete events over a SPSC ring, latest-wins parameter snapshots, and deferred deallocation of memory retired by the audio thread"

project: contexts: RealTime: valueObjects: {
	BoundaryMessage: {description: "a discrete message crossing the boundary: NoteOn, NoteOff, ControlChange, PatchChange, PresetLoad, ParameterUpdate, or TestPlaybackGenerationReset/all-notes-off"}
	ParameterSnapshot: {
		description: "a complete immutable snapshot of every audio-thread-readable Patch, sample instrument, and mixer value, including volume, pan, sends, mute, solo, and mixer assignment"
		invariants: ["a snapshot is derived from exactly one accepted AppState frame and contains no UI text, cursor, key, widget, or playback-clock state"]
		validations: [{kind: "test", command: ["cargo", "test", "parameter_snapshot"], description: "every editable mixer value round-trips into the typed audio snapshot without loss"}]
	}
}

project: contexts: RealTime: ports: {
	EventRing: {
		direction: "inbound"
		contract: {
			push: "(message: BoundaryMessage) -> result<(), RingFull>"
			pop:  "() -> option<BoundaryMessage>"
		}
		meta: notes: "single producer (UI/MIDI thread), single consumer (audio thread)"
	}
	ParameterBridge: {
		direction: "inbound"
		contract: {
			publish: "(snapshot: ParameterSnapshot) -> ()"
			read:    "() -> ParameterSnapshot"
		}
		meta: notes: "writer publishes; reader always gets the latest snapshot without blocking"
	}
	DeferredDeallocator: {
		direction: "outbound"
		contract: {
			retire:  "(allocation: Retired) -> ()"
			collect: "() -> u32"
		}
		meta: notes: "the audio thread retires; a background thread frees"
	}
}

project: adapters: RtrbEventRing: {
	implements: "port.RealTime.EventRing"
	layer:      "infrastructure"
	profile: {kind: "in_process", topology: "single-producer-single-consumer"}
	meta: framework: "rtrb"
	validations: [{kind: "test", command: ["cargo", "test", "rtrb_event_ring"], description: "full/empty behavior is non-blocking and preserves event order"}]
	contributesTo: [{capability: "capability.realtime_safe_execution", contribution: "implements the accepted lock-free SPSC event boundary"}]
}

project: adapters: TripleBufferParameterBridge: {
	implements: "port.RealTime.ParameterBridge"
	layer:      "infrastructure"
	profile: {kind: "in_process", topology: "single-writer-latest-reader"}
	meta: framework: "triple_buffer"
	validations: [{kind: "test", command: ["cargo", "test", "triple_buffer_parameter_bridge"], description: "read returns the newest complete published snapshot without blocking"}]
	contributesTo: [{capability: "capability.realtime_safe_execution", contribution: "implements latest-wins lock-free parameter snapshots"}]
}

project: adapters: BasedropDeferredDeallocator: {
	implements: "port.RealTime.DeferredDeallocator"
	layer:      "infrastructure"
	profile: {kind: "in_process", topology: "audio-retire-control-collect"}
	meta: framework: "basedrop"
	validations: [{kind: "test", command: ["cargo", "test", "basedrop_deferred_deallocator"], description: "tracked state is destroyed only by control-side collection"}]
	contributesTo: [{capability: "capability.realtime_safe_execution", contribution: "implements off-audio-thread reclamation of retired state"}]
}
