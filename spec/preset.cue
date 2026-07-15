package crestsynth

// Persistence owns versioned product sound state. It references the canonical
// domain values directly; it never defines a parallel Setup/Patch/Mixer model.
project: contexts: Preset: purpose: "versioned, complete, atomic save and restore for patches, banks, and sessions"

project: contexts: Preset: valueObjects: {
	PresetMetadata: {
		state: {name: "string", author: "string", tags: "list<string>", category: "string", description: "string"}
		description: "searchable user metadata carried by a preset"
		validations: [{kind: "test", command: ["cargo", "test", "preset_metadata"], description: "metadata normalizes and round-trips"}]
	}
	Preset: {
		state: {id: "PresetId", version: "u32", meta: "PresetMetadata", patch: "Patch", strip: "ChannelStrip"}
		description: "a versioned snapshot of one complete patch and its canonical mixer strip, including source, sample reference, modulation, mapping, MPE, inserts, sends, and levels"
		invariants: ["version is explicit and non-zero", "patch.mixerStrip identifies the captured strip"]
		validations: [{kind: "test", command: ["cargo", "test", "preset"], description: "complete patch and strip state round-trip without omitted fields"}]
		contributesTo: [{capability: "capability.versioned_sound_state", contribution: "defines the complete versioned representation of one playable sound"}]
	}
}

project: contexts: Preset: aggregates: Bank: {
	root: true
	purpose: "an ordered collection of preset identifiers"
	state: {id: "BankId", name: "string", presets: "list<PresetId>", readOnly: "bool"}
	commands: {AddPreset: {preset: "PresetId"}, RemovePreset: {preset: "PresetId"}, Rename: {name: "string"}}
	events: {PresetAdded: {preset: "PresetId"}, PresetRemoved: {preset: "PresetId"}}
	invariants: ["a bank never contains the same preset twice", "a read-only bank rejects every mutating command"]
	validations: [{kind: "test", command: ["cargo", "test", "bank"], description: "ordering, uniqueness, and read-only behavior pass"}]
}

project: contexts: Preset: aggregates: Session: {
	root: true
	purpose: "the complete reproducible sound state consumed by AppState"
	state: {
		id: "SessionId"
		version: "u32"
		patches: "list<Patch>"
		mixer: "MixerView"
		auxBuses: "list<MixBus>"
		masterBus: "MixBus"
		tempo: "Tempo"
		timeSignature: "TimeSignature"
	}
	commands: {Replace: {candidate: "Session"}}
	events: {Replaced: {id: "SessionId", version: "u32"}}
	invariants: [
		"version is explicit and non-zero",
		"patch IDs and mixer-strip assignments are unique and refer to the captured mixer",
		"replacement occurs only after the entire candidate has decoded, migrated, and validated",
		"a failed restore leaves the prior session byte-identical",
	]
	validations: [{kind: "test", command: ["cargo", "test", "session"], description: "complete state consistency and atomic replacement are enforced"}]
	contributesTo: [{capability: "capability.versioned_sound_state", contribution: "owns atomic replacement of complete reproducible patch, mixer, bus, tempo, and meter state"}]
}

project: contexts: Preset: ports: {
	PresetCodec: {
		direction: "outbound"
		contract: {
			encodePreset: "(preset: &Preset) -> result<Vec<u8>, CodecError>"
			decodePreset: "(data: &[u8]) -> result<Preset, CodecError>"
			encodeSession: "(session: &Session) -> result<Vec<u8>, CodecError>"
			decodeSession: "(data: &[u8]) -> result<Session, CodecError>"
		}
		meta: notes: "decoders migrate supported older versions into the current canonical model and reject unsupported versions"
	}
	PresetStorage: {
		direction: "outbound"
		contract: {
			savePreset: "(preset: &Preset) -> result<(), StorageError>"
			loadPreset: "(id: PresetId) -> result<option<Preset>, StorageError>"
			listPresets: "() -> result<list<PresetMetadata>, StorageError>"
			deletePreset: "(id: PresetId) -> result<(), StorageError>"
			saveSession: "(session: &Session) -> result<(), StorageError>"
			loadSession: "(id: SessionId) -> result<option<Session>, StorageError>"
		}
	}
}

project: contexts: Preset: applicationServices: {
	PresetBrowser: {
		purpose: "headless library operations for search, load/save, import, and export; no preset-browser GUI is currently claimed"
		uses: ["aggregate.Preset.Bank", "valueObject.Preset.Preset", "port.Preset.PresetCodec", "port.Preset.PresetStorage"]
		operations: {savePatch: {input: {patch: "Patch", strip: "ChannelStrip"}, output: {preset: "Preset"}}, loadPatch: {input: {id: "PresetId"}, output: {preset: "result<Preset, LoadError>"}}}
		validations: [{kind: "test", command: ["cargo", "test", "preset_browser"], description: "library operations preserve complete canonical preset state"}]
	}
	SessionManager: {
		purpose: "validate, encode, store, load, migrate, and atomically replace complete sessions"
		uses: ["aggregate.Preset.Session", "port.Preset.PresetCodec", "port.Preset.PresetStorage"]
		operations: {save: {input: {session: "Session"}}, restore: {input: {id: "SessionId", active: "Session"}, output: {session: "result<Session, RestoreError>"}}}
		validations: [{kind: "test", command: ["cargo", "test", "session_manager"], description: "successful restore is complete and failed restore cannot mutate active state"}]
		contributesTo: [{capability: "capability.versioned_sound_state", contribution: "coordinates migration and atomic replacement of complete sessions"}]
	}
}

project: adapters: SerdePresetCodec: {
	implements: "port.Preset.PresetCodec"
	layer: "infrastructure"
	profile: {kind: "persistence", medium: "memory-or-user-selected-json"}
	meta: {framework: "serde + serde_json", rules: ["serialization DTOs, if required for migration, convert at the adapter boundary and never escape as a second domain model"]}
	validations: [{kind: "test", command: ["cargo", "test", "serde_preset_codec"], description: "preset/session bytes are deterministic, complete, versioned, and migratable"}]
	contributesTo: [{capability: "capability.versioned_sound_state", contribution: "encodes and decodes complete versioned state without semantic loss"}]
}

project: adapters: FsPresetStorage: {
	implements: "port.Preset.PresetStorage"
	layer: "infrastructure"
	profile: {kind: "persistence", medium: "user sound-library directory"}
	meta: rules: ["these are user product artifacts, not crest-spec operational manifests"]
	validations: [{kind: "test", command: ["cargo", "test", "fs_preset_storage"], description: "preset and session writes are atomic and list/load/delete are consistent"}]
}
