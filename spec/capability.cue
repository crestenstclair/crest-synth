package crestsynth

project: contexts: Synth: {
	ubiquitousLanguage: {
		CapabilityId: "a stable namespaced identifier for one installed instrument implementation"
		ParameterId: "a stable namespaced identifier for one capability-owned parameter"
		CapabilityDescriptor: "the immutable control-side schema supplied by one installed instrument capability"
		InstrumentConfig: "one Patch's capability identity, parameter values, and asset references"
	}

	valueObjects: CapabilityId: {
		description: "a stable control-side identity for one installed instrument capability"
		from: "String"
		invariants: [
			"the value is nonempty ASCII lowercase kebab-case segments separated by dots",
			"the value is stable across serialization versions and never contains a display label, filesystem path, pointer, or registry index",
		]
		contributesTo: [{capability: "capability.instrument_capability_model", contribution: "keeps Patch configuration, descriptors, projections, and providers joined by one semantic identity"}]
	}

	valueObjects: ParameterId: {
		description: "a stable capability-scoped identity for one parameter"
		from: "String"
		invariants: [
			"the value is nonempty ASCII lowercase kebab-case segments separated by dots",
			"the value is unique within its CapabilityDescriptor and is never a display label or positional widget index",
		]
		contributesTo: [{capability: "capability.instrument_capability_model", contribution: "lets reducers, projections, tests, and later views address parameters without engine-specific field matching"}]
	}

	valueObjects: AssetReference: {
		description: "a stable control-side reference to an instrument asset rather than decoded runtime data"
		state: {
			kind: "SoundFont | Sample | Other"
			locator: "String"
		}
		invariants: [
			"locator is nonempty and deterministic for serialization",
			"the value contains no decoded PCM, parser, file handle, device handle, engine instance, callback reference, or destructor-bearing prepared state",
		]
		contributesTo: [{capability: "capability.instrument_capability_model", contribution: "keeps asset identity in canonical control state without leaking prepared audio state"}]
	}

	valueObjects: ParameterValue: {
		description: "the canonical tagged value accepted for a capability parameter"
		state: {
			kind: "Continuous | Stepped | Choice | Toggle"
			value: "f64 | i64 | String | bool"
		}
		invariants: [
			"kind determines exactly one matching value representation",
			"continuous values are finite",
			"choice values are stable option ids rather than display labels",
			"the value contains no engine object, closure, UI widget, path, buffer, or dynamically typed escape hatch",
		]
		contributesTo: [{capability: "capability.instrument_capability_model", contribution: "provides one canonical value union shared by every instrument descriptor and config"}]
	}

	valueObjects: ParameterAssignment: {
		description: "one capability parameter identity paired with its canonical value"
		state: {
			parameterId: "ParameterId"
			value: "ParameterValue"
		}
		invariants: ["parameterId occurs at most once within one InstrumentConfig"]
		contributesTo: [{capability: "capability.instrument_capability_model", contribution: "stores engine-specific values without adding engine-specific fields to Patch"}]
	}

	valueObjects: ParameterSpec: {
		description: "the immutable schema and adjustment contract for one capability parameter"
		state: {
			id: "ParameterId"
			label: "String"
			kind: "Continuous | Stepped | Choice | Toggle | Asset"
			update: "Scalar | Structural"
			defaultValue: "ParameterValue | AssetReference"
			range: "Option<{minimum, maximum}>"
			choices: "Vec<{id, label}>"
			fineStep: "Option<f64>"
			coarseStep: "Option<f64>"
			unit: "Option<String>"
			formatter: "stable formatter id"
			enabledWhen: "Option<declarative ParameterId/value predicate>"
			visibleWhen: "Option<declarative ParameterId/value predicate>"
		}
		invariants: [
			"id is unique within the owning CapabilityDescriptor",
			"defaultValue matches kind and satisfies the declared range or choice set",
			"continuous and stepped parameters have finite ordered bounds and positive fine/coarse steps",
			"choice ids are unique and stable; labels are presentation only",
			"Asset and every value that changes preparation topology use Structural update",
			"dependency predicates reference earlier parameters in the same descriptor and contain no callbacks or engine code",
		]
		contributesTo: [{capability: "capability.instrument_capability_model", contribution: "defines the schema later Patch views render and the control path validates"}]
	}

	valueObjects: CapabilityDescriptor: {
		description: "one immutable installed instrument schema with preparation and event metadata"
		state: {
			id: "CapabilityId"
			label: "String"
			semanticAccent: "stable semantic token id"
			sections: "Vec<{id, label, parameters: Vec<ParameterSpec>}>"
			assetRequirements: "Vec<{parameterId: ParameterId, required: bool}>"
			voiceLimit: "u16"
			supportedMidiKinds: "Vec<MidiMessageKind>"
		}
		invariants: [
			"id, section ids, ParameterIds, choice ids, and asset requirements are unique before any set conversion",
			"section and parameter order is stable and is the only order consumed by serialization, text projection, coverage, and later views",
			"voiceLimit is greater than zero and describes prepared capacity rather than a promise of unbounded allocation",
			"supportedMidiKinds contains no duplicates and unsupported input is rejected explicitly rather than silently remapped",
			"the descriptor contains immutable control metadata only and owns no engine, renderer, factory closure, decoded asset, buffer, lock, device, or callback state",
		]
		contributesTo: [{capability: "capability.instrument_capability_model", contribution: "is the single schema source for installed instrument configuration and projection"}]
	}

	valueObjects: InstrumentConfig: {
		description: "one Patch's validated capability identity, values, and stable asset references"
		state: {
			capabilityId: "CapabilityId"
			values: "Vec<ParameterAssignment>"
			assetReferences: "Vec<{parameterId: ParameterId, reference: AssetReference}>"
		}
		invariants: [
			"capabilityId resolves to exactly one installed CapabilityDescriptor",
			"values and assetReferences contain every required descriptor parameter exactly once, contain no undeclared parameter, and match kind, range, choices, and dependency rules",
			"values and assetReferences preserve descriptor order for deterministic serialization but behavior addresses them only by ParameterId",
			"the config contains no fallback capability, engine instance, decoded asset, prepared renderer, UI state, or callback-owned data",
		]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "removes SoundFont-specific shape from the Patch aggregate"},
			{capability: "capability.soundfont_audio", contribution: "carries the current SoundFont preset and asset through a generic Patch-owned contract"},
		]
	}

	valueObjects: CapabilityRegistry: {
		description: "the immutable ordered control-side registry of installed instrument descriptors"
		state: {descriptors: "Vec<CapabilityDescriptor>"}
		invariants: [
			"CapabilityIds are unique and descriptor order is stable",
			"lookup and InstrumentConfig validation use CapabilityId and ParameterId rather than label or position",
			"an unknown, duplicate, unavailable, or invalid capability produces a typed error; no descriptor or config is silently substituted",
			"the current production composition installs exactly the HiDef SoundFont descriptor and no Braids renderer, alternate product engine, layering engine, engine selector, or fallback",
			"the registry contains descriptors only; InstrumentPreparer registration, PreparedInstrument ownership, PreparedEngineRack, and structural graph handoff remain behind their separate application and real-time ports",
		]
		validations: [{kind: "test", command: ["cargo", "test", "capability_registry"], description: "duplicate ids, invalid assignments, unknown capabilities, fallback attempts, and descriptor-order drift are rejected"}]
		contributesTo: [{capability: "capability.instrument_capability_model", contribution: "makes the installed instrument schema explicit and deterministic without coupling AppState to an adapter"}]
	}

	ports: InstrumentCapabilityProvider: {
		direction: "outbound"
		contract: {
			descriptor: "() -> CapabilityDescriptor"
			createConfig: "(values: &[ParameterAssignment], assetReferences: &[{parameterId: ParameterId, reference: AssetReference}]) -> Result<InstrumentConfig, CapabilityError>"
		}
		consumes: [
			"valueObject.Synth.CapabilityDescriptor",
			"valueObject.Synth.InstrumentConfig",
			"valueObject.Synth.ParameterAssignment",
			"valueObject.Synth.AssetReference",
		]
		invariants: [
			"provider operations run on the control side before Patch installation",
			"descriptor and config creation are deterministic and return typed errors without fallback",
			"the port contains no render, dispatch, audio-buffer, device, UI, or file-I/O operation",
		]
		contributesTo: [{capability: "capability.instrument_capability_model", contribution: "lets a concrete engine adapter supply schema/config data without defining Patch"}]
	}
}

project: adapters: HiDefSoundFontCapability: {
	implements: "port.Synth.InstrumentCapabilityProvider"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "capability_descriptor", system: "HiDef.sf2"}
	meta: {
		framework: "rust"
		rules: [
			"provide the one installed descriptor with CapabilityId instrument.soundfont.hidef, label HiDef SoundFont, a stable semantic instrument accent, declared bounded voice capacity, and the MIDI kinds supported by HiDefSoundFontPreparer",
			"declare soundfont.bank as Structural Stepped, soundfont.program as Structural Stepped, soundfont.percussion as Structural Toggle, and soundfont.file as a required Structural Asset fixed to ./sf2/HiDef.sf2",
			"create InstrumentConfig from caller-supplied generic assignments and asset references without reading a file, loading an engine, changing a Patch, or inventing a second SoundFont preset model",
			"return a typed error for invalid or unknown values and never substitute a preset, percussion identity, asset, descriptor, or engine",
			"own no InstrumentPreparer, PreparedInstrument, PreparedEngineRack, renderer factory closure, C++/FFI type, engine selection, editable instrument parameter, ADSR, or Patch-page behavior; preparation remains a separate port even when ids match",
		]
	}
	validations: [{kind: "test", command: ["cargo", "test", "hidef_soundfont_capability"], description: "the descriptor and every fixture-derived config are exact, deterministic, schema-valid, and rejected when altered"}]
	contributesTo: [
		{capability: "capability.instrument_capability_model", contribution: "proves SoundFont is one registry entry rather than the universal Patch model"},
		{capability: "capability.prepared_engine_rack", contribution: "provides the stable capability identity independently matched to the production SoundFont preparer"},
		{capability: "capability.soundfont_audio", contribution: "preserves the fixed bank/program/percussion and HiDef.sf2 identity used by the existing renderer"},
	]
}
