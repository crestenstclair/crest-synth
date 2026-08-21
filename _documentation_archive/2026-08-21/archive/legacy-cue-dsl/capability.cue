package crestsynth

project: contexts: Synth: {
	ubiquitousLanguage: {
		CapabilityId: "a stable namespaced identifier for one installed instrument implementation"
		ParameterId: "a stable namespaced identifier for one capability-owned parameter"
		CapabilityDescriptor: "the immutable control-side schema supplied by one installed instrument capability"
		ParameterSpec: "the canonical immutable parameter schema shared by installed instrument and effect capabilities"
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
			patchInteraction: "ReadOnly | ScalarEdit | StructuralChoice"
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
			"ScalarEdit is valid only for a non-Asset Scalar parameter with finite descriptor-owned adjustment semantics; StructuralChoice is valid only for a non-Asset Structural Choice with at least two choices",
			"dependency predicates reference earlier parameters in the same descriptor and contain no callbacks or engine code",
		]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "defines the schema the control path validates"},
			{capability: "capability.schema_driven_patch_page", contribution: "defines each PATCH row's stable identity, presentation, value kind, update class, bounds, and dependencies"},
			{capability: "capability.soundfont_preset_selection", contribution: "declares whether one capability-owned structural Choice joins the PATCH focus/edit surface without a capability branch"},
			{capability: "capability.static_patch_effect", contribution: "lets effect descriptors expose PATCH-editable scalar controls without processor-specific field types"},
		]
	}

	valueObjects: CapabilityDescriptor: {
		description: "one immutable installed instrument schema with preparation and event metadata"
		state: {
			id: "CapabilityId"
			label: "String"
			semanticAccent: "stable semantic token id"
			sections: "Vec<{id, label, parameters: Vec<ParameterSpec>}>"
			assetRequirements: "Vec<{parameterId: ParameterId, required: bool}>"
			voicePolicy: "FixedPerPatch { voices: u16 } | EngineManaged"
			supportedMidiKinds: "Vec<MidiMessageKind>"
		}
		invariants: [
			"id, section ids, ParameterIds, choice ids, and asset requirements are unique before any set conversion",
			"section and parameter order is stable and is the only order consumed by serialization, text projection, coverage, and later views",
			"FixedPerPatch has a nonzero voice count newly owned by every prepared Patch; EngineManaged delegates allocation to one Patch-local engine instance whose preparer still proves a finite real-time safety ceiling",
			"voice policy never describes an engine-global pool and no Patch borrows, consumes, or reduces another Patch's capacity",
			"voice policy encodes neither a capability-specific Patch-count limit nor a global voice budget; the engine-agnostic prepared-rack capacity governs concurrent Patch count separately",
			"supportedMidiKinds contains no duplicates and unsupported input is rejected explicitly rather than silently remapped",
			"at most sixteen parameters are classified Scalar so their descriptor-ordered values fit the fixed real-time projection",
			"PATCH-editable parameters are derived only from patchInteraction and descriptor order; installed instrument descriptors in this increment use StructuralChoice or ReadOnly while ScalarEdit is reserved for effect descriptors, and the registry contains no duplicated per-capability PATCH field list",
			"the descriptor contains immutable control metadata only and owns no engine, renderer, factory closure, decoded asset, buffer, lock, device, or callback state",
		]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "is the single schema source for installed instrument configuration and projection"},
			{capability: "capability.schema_driven_patch_page", contribution: "supplies the active engine label, ordered sections, and generic parameter rows"},
			{capability: "capability.asynchronous_engine_selection", contribution: "supplies the ordered defaults and required asset references for a new target config"},
			{capability: "capability.soundfont_preset_selection", contribution: "supplies the ordered authored-name preset choices and their explicit structural PATCH interaction"},
		]
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
			{capability: "capability.schema_driven_patch_page", contribution: "supplies the active values and asset references rendered against the descriptor"},
			{capability: "capability.asynchronous_engine_selection", contribution: "is the only candidate configuration committed when a prepared target succeeds"},
			{capability: "capability.soundfont_preset_selection", contribution: "carries one stable preset Choice id plus the fixed SoundFont asset without duplicating bank/program/percussion assignments"},
		]
	}

	valueObjects: CapabilityRegistry: {
		description: "the immutable ordered control-side registry of installed instrument descriptors"
		state: {descriptors: "Vec<CapabilityDescriptor>"}
		invariants: [
			"CapabilityIds are unique and descriptor order is stable",
			"lookup and InstrumentConfig validation use CapabilityId and ParameterId rather than label or position",
			"an unknown, duplicate, unavailable, or invalid capability produces a typed error; no descriptor or config is silently substituted",
			"the current production composition installs exactly the HiDef SoundFont and Braids descriptors in stable order; PATCH projects them as adjacent nonwrapping engine choices, permits one prepared replacement at a time, and never layers or falls back",
			"the registry contains descriptors only; InstrumentPreparer registration, PreparedInstrument ownership, PreparedEngineRack, and structural graph handoff remain behind their separate application and real-time ports",
		]
		validations: [{id: "validation.value_object.capability_registry", kind: "test", command: ["cargo", "test", "capability_registry"], description: "duplicate ids, invalid assignments, unknown capabilities, fallback attempts, and descriptor-order drift are rejected"}]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "makes the installed instrument schema explicit and deterministic without coupling AppState to an adapter"},
			{capability: "capability.schema_driven_patch_page", contribution: "supplies the complete ordered engine-choice projection without placeholders"},
			{capability: "capability.asynchronous_engine_selection", contribution: "defines deterministic adjacent choice order and validates every committed candidate"},
			{capability: "capability.soundfont_preset_selection", contribution: "freezes the catalog-backed SoundFont choices used by config validation, projection, and reducer selection"},
		]
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
			"provider operations run on control or worker ownership before Patch installation or off-callback candidate preparation",
			"descriptor and config creation are deterministic and return typed errors without fallback",
			"the port contains no render, dispatch, audio-buffer, device, UI, or file-I/O operation",
		]
		contributesTo: [
			{capability: "capability.instrument_capability_model", contribution: "lets a concrete engine adapter supply schema/config data without defining Patch"},
			{capability: "capability.asynchronous_engine_selection", contribution: "validates descriptor-default candidates without owning selection or mutation"},
			{capability: "capability.soundfont_preset_selection", contribution: "validates catalog-backed preset assignments without owning SF2 parsing or rendering"},
		]
	}
}

project: adapters: HiDefSoundFontCapability: {
	implements: "port.Synth.InstrumentCapabilityProvider"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "capability_descriptor", system: "HiDef.sf2"}
	meta: {
		framework: "rust"
		rules: [
			"provide the SoundFont descriptor with CapabilityId instrument.soundfont.hidef, label HiDef SoundFont, a stable semantic instrument accent, EngineManaged voice policy, and the MIDI kinds supported by HiDefSoundFontPreparer",
			"be constructed from the immutable SoundFontPresetCatalog produced by the single shared HiDef asset load; provider methods themselves perform no file I/O or parsing",
			"declare soundfont.preset as one Structural Choice whose ordered ids and exact labels come from the catalog and whose PATCH interaction is StructuralChoice, plus soundfont.file as a required Structural Asset fixed to ./sf2/HiDef.sf2 and ReadOnly on PATCH",
			"use the first bank/program-sorted playable catalog entry as the descriptor default and fail construction if no playable entry exists",
			"create InstrumentConfig from caller-supplied generic assignments and asset references without loading an engine, changing a Patch, decoding a choice label, or inventing a second SoundFont preset model",
			"return a typed error for invalid or unknown values and never substitute a preset, percussion identity, asset, descriptor, or engine",
			"own no InstrumentPreparer, PreparedInstrument, PreparedEngineRack, numeric prepared bank, renderer factory closure, C++/FFI type, structural lifecycle, ADSR, or Patch-page behavior; preparation remains a separate port even when ids and shared catalog provenance match",
		]
	}
	validations: [{id: "validation.adapter.hidef_soundfont_capability", kind: "test", command: ["cargo", "test", "hidef_soundfont_capability"], description: "the descriptor and every fixture-derived config are exact, deterministic, schema-valid, and rejected when altered"}]
	contributesTo: [
		{capability: "capability.instrument_capability_model", contribution: "proves SoundFont is one registry entry rather than the universal Patch model"},
		{capability: "capability.asynchronous_engine_selection", contribution: "builds the exact descriptor-default SoundFont candidate including HiDef.sf2 without fallback"},
		{capability: "capability.soundfont_preset_selection", contribution: "projects exact authored names over stable numeric-address choice ids and validates adjacent selections"},
		{capability: "capability.prepared_engine_rack", contribution: "provides the stable capability identity independently matched to the production SoundFont preparer"},
		{capability: "capability.soundfont_audio", contribution: "preserves one catalog-backed numeric preset identity and HiDef.sf2 reference for the existing renderer"},
	]
}

project: adapters: BraidsCapability: {
	implements: "port.Synth.InstrumentCapabilityProvider"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "capability_descriptor", system: "Mutable Instruments Braids"}
	meta: {
		framework: "rust"
		rules: [
			"provide CapabilityId instrument.braids with FixedPerPatch { voices: 16 } and only the MIDI kinds implemented by BraidsPreparer",
			"declare braids.model as a Scalar Choice containing the 47 named playable upstream models in stable source order, excluding the question-mark sentinel, and keep it ReadOnly on PATCH",
			"declare braids.timbre and braids.color as finite Scalar Continuous values in 0..=1 with positive fine and coarse steps and keep both ReadOnly on PATCH",
			"create a canonical descriptor-ordered InstrumentConfig without reading files, constructing C++ objects, or selecting another capability",
			"return typed errors for missing, duplicate, undeclared, wrong-kind, non-finite, out-of-range, or unknown-choice values and never substitute a model, descriptor, config, or engine",
			"own no InstrumentPreparer, PreparedInstrument, PreparedEngineRack, C++/FFI value, envelope, engine selector, or Patch-page behavior",
		]
	}
	validations: [{id: "validation.adapter.braids_capability", kind: "test", command: ["cargo", "test", "braids_capability"], description: "the full 47-model descriptor and default/config validation are stable, exact, and fallback-free"}]
	contributesTo: [
		{capability: "capability.instrument_capability_model", contribution: "proves a second materially different schema is handled by the same registry/config model"},
		{capability: "capability.asynchronous_engine_selection", contribution: "builds the exact descriptor-default Braids candidate without cached SoundFont state"},
		{capability: "capability.braids_engine", contribution: "owns the canonical Model, Timbre, Color, FixedPerPatch(16), and supported-MIDI declaration independently of native preparation"},
		{capability: "capability.prepared_engine_rack", contribution: "provides the identity independently matched to the Braids preparer"},
	]
}
