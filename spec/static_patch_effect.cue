package crestsynth

// Phase 4 first increment: one capability-described, statically installed
// per-Patch Chorus insert. The topology is fixed while the ownership and
// preparation contracts remain extensible to later ordered effect slots.

project: goals: shape_patch_with_effect: {
	description: "The player can edit one configured Patch-local Chorus through PATCH and hear it process only that Patch between instrument rendering and mix/routing"
	priority: "required"
	actors: ["actor.player", "actor.maintainer"]
	dependsOn: ["goal.select_soundfont_preset"]
	capabilities: [
		"capability.static_patch_effect",
		"capability.instrument_capability_model",
		"capability.schema_driven_patch_page",
		"capability.one_way_parameter_control",
		"capability.prepared_engine_rack",
		"capability.global_mix",
		"capability.realtime_execution",
		"capability.observable_demo_scene",
		"capability.live_observable_demo",
	]
	requirements: [
		"requirement.effect_capability_contract",
		"requirement.fixed_patch_effect_topology",
		"requirement.canonical_patch_effect_control",
		"requirement.pinned_chorus_adapter",
		"requirement.patch_effect_realtime_safety",
		"requirement.patch_effect_behavioral_proof",
	]
}

project: capabilities: static_patch_effect: {
	description: "Configure, project, edit, prepare, process, and observe one fixed Patch-local Chorus through capability-owned schema and callback-safe runtime boundaries"
	goals: ["goal.shape_patch_with_effect", "goal.play_test_song", "goal.observe_synth", "goal.observe_live_synth"]
	acceptance: configured_chorus_insert: {
		description: "one real Chorus instance is audible, isolated, schema-driven, independently prepared, and ordered before Patch mixing"
		actor: "actor.maintainer"
		steps: [
			{action: "start the production fixture", observes: "the first Patch owns one stable effect.chorus config with Amount and Depth while every other Patch has an empty ordered post-effect list"},
			{action: "inspect and navigate PATCH", observes: "the focused first Patch shows one read-only Chorus identity followed by descriptor-derived editable Amount and Depth rows after its instrument controls"},
			{action: "adjust Amount and Depth", observes: "each semantic edit commits through AppState.apply, reaches the matching fixed effect-scalar snapshot, changes only the first Patch's processed stem, and publishes no structural graph"},
			{action: "render simultaneous configured and unconfigured Patches", observes: "the Chorus processes the configured engine stem before gain, pan, and sends; untargeted stems remain exact and independently prepared Chorus instances retain independent delay/LFO state"},
			{action: "replace the configured Patch engine and SoundFont preset", observes: "the complete candidate preserves the effect slot/config/layout exactly while the graph swap may reset its tail under the existing structural contract"},
			{action: "run deterministic and physical demos", observes: "both effect parameters have coherent schema-derived coverage and measured pre/post effect consequences; make demo-live completes and exits normally after physical playback"},
		]
		evidence: ["evidence.static_patch_effect_contract"]
	}
}

project: requirements: {
	effect_capability_contract: {
		kind: "nonfunctional"
		description: "Effects use stable EffectCapabilityId and EffectSlotId values, a separate immutable EffectCapabilityDescriptor/registry/provider/preparer family, and the canonical ParameterSpec, ParameterAssignment, ParameterValue, and AssetReference types; effect descriptors contain no instrument voice policy or MIDI semantics, and missing, duplicate, mismatched, invalid, or unavailable registrations fail explicitly without bypass or fallback"
		goals: ["goal.shape_patch_with_effect"]
		capabilities: ["capability.static_patch_effect", "capability.instrument_capability_model"]
	}
	fixed_patch_effect_topology: {
		kind: "functional"
		description: "Each Patch owns an ordered PostEffectConfig list with current capacity zero or one; the first fixture Patch contains effect.chorus and other fixture Patches contain none, and callback signal order is PreparedEngineRack to PatchAudioBlock to PreparedPostEffectRack to MixEngine with no selector, bypass, reordering, arbitrary edge, feedback route, or placeholder slot"
		goals: ["goal.shape_patch_with_effect", "goal.play_test_song"]
		capabilities: ["capability.static_patch_effect", "capability.prepared_engine_rack", "capability.global_mix"]
	}
	canonical_patch_effect_control: {
		kind: "functional"
		description: "PATCH appends descriptor-ordered ScalarEdit effect parameters after Engine, common ADSR, and instrument StructuralChoice rows; Edit+Left/Right applies fine decrement/increment and Edit+Down/Up applies coarse decrement/increment through AppState.apply, while effect identity remains visible and read-only and accepted edits publish only the fixed latest scalar snapshot"
		goals: ["goal.shape_patch_with_effect"]
		capabilities: ["capability.static_patch_effect", "capability.schema_driven_patch_page", "capability.one_way_parameter_control"]
	}
	pinned_chorus_adapter: {
		kind: "functional"
		description: "The first processor is the MIT-licensed Mutable Instruments Rings Chorus pinned at pichenettes/eurorack@08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4 and stmlib@e3bd7c9cc00e4364166f9905c0509b6ffd0535ec, vendored as the audited minimal source/table/header subset with hashes and provenance, exposed to the product only as Chorus with scalar Amount and Depth, prepared with independent external delay/LFO state, and admitted initially only at exactly 48 kHz"
		goals: ["goal.shape_patch_with_effect"]
		capabilities: ["capability.static_patch_effect"]
	}
	patch_effect_realtime_safety: {
		kind: "nonfunctional"
		description: "Every Chorus instance, delay buffer, LFO state, effect rack slot, scalar layout, stereo scratch requirement, and observation field is fully prepared off callback; processing is bounded and performs no allocation, deallocation, collection growth, locking, blocking, I/O, logging, formatting, panic, exception, unwind, or destruction, and complete graph retirement destroys effect state only on control or worker ownership"
		goals: ["goal.shape_patch_with_effect", "goal.play_test_song"]
		capabilities: ["capability.static_patch_effect", "capability.realtime_execution", "capability.prepared_engine_rack"]
	}
	patch_effect_behavioral_proof: {
		kind: "functional"
		description: "A named production-path target plus both demos prove exact source/license hashes, descriptor/config validation, PATCH focus and fine/coarse edits, scalar-only publication, engine-effect-mix order, target-only audible difference, stereo side energy, zero-input bounded behavior, independent instances and tails, structural preservation, unsupported-rate and missing-registration failures, finite output, zero fallback, callback safety, and render-time admission"
		goals: ["goal.shape_patch_with_effect", "goal.observe_synth", "goal.observe_live_synth"]
		capabilities: ["capability.static_patch_effect", "capability.observable_demo_scene", "capability.live_observable_demo", "capability.realtime_execution"]
	}
}

project: evidence: static_patch_effect_contract: {
	kind: "behavioral"
	description: "the canonical Patch config, effect registry/provider/preparer, PATCH reducer/projections, fixed scalar transport, prepared effect rack, production renderer, mixer, and both demos agree on one audible fallback-free Chorus insert"
	validations: ["validation.static_patch_effect", "validation.patch_page_projection", "validation.prepared_engine_rack", "validation.demo_scene", "validation.live_demo", "validation.test"]
	witnesses: ["witness.static_patch_effect"]
}

project: contexts: Synth: {
	ubiquitousLanguage: {
		EffectCapabilityId: "a stable namespaced identity for one installed Patch post-effect implementation"
		EffectSlotId: "a stable Patch-local semantic identity for one ordered effect instance"
		EffectCapabilityDescriptor: "the immutable control-side parameter schema supplied by one installed effect capability"
		PostEffectConfig: "one Patch-owned effect slot identity, capability identity, values, and asset references"
	}

	valueObjects: EffectCapabilityId: {
		description: "a stable control-side identity for one installed Patch effect capability"
		from: "String"
		invariants: [
			"the value is nonempty ASCII lowercase kebab-case segments separated by dots",
			"the value is stable across serialization and never contains a label, path, pointer, registry index, PatchId, or slot position",
		]
		contributesTo: [{capability: "capability.static_patch_effect", contribution: "joins effect descriptor, config, provider, preparer, prepared instance, projections, and proof without processor-specific branching"}]
	}

	valueObjects: EffectSlotId: {
		description: "a stable Patch-local identity for one ordered effect instance"
		from: "u16"
		invariants: [
			"zero is invalid and identities are unique within one Patch",
			"the value remains stable across projection and structural engine/preset replacement and is never a widget or vector index",
		]
		contributesTo: [{capability: "capability.static_patch_effect", contribution: "keeps focus, scalar projection, prepared state, and observations attached to the same semantic slot"}]
	}

	valueObjects: EffectCapabilityDescriptor: {
		description: "one immutable installed effect schema without instrument-only voice or MIDI metadata"
		state: {
			id: "EffectCapabilityId"
			label: "String"
			semanticAccent: "stable semantic token id"
			sections: "Vec<{id, label, parameters: Vec<ParameterSpec>}>"
			assetRequirements: "Vec<{parameterId: ParameterId, required: bool}>"
		}
		invariants: [
			"id, section ids, ParameterIds, choice ids, and asset requirements are unique and descriptor order is stable",
			"parameters reuse the canonical ParameterSpec and at most eight are Scalar for one fixed RtPostEffectParameters slot",
			"the descriptor contains no voice policy, MIDI semantics, engine factory, prepared processor, FFI object, delay buffer, closure, UI state, or callback owner",
			"the current effect.chorus descriptor contains exactly Amount then Depth as finite Continuous Scalar ScalarEdit parameters in 0..=1 with declared defaults and positive fine/coarse steps",
		]
		contributesTo: [
			{capability: "capability.static_patch_effect", contribution: "is the single schema source for Chorus config, PATCH rows, validation, scalar layout, and coverage"},
			{capability: "capability.schema_driven_patch_page", contribution: "supplies the visible Chorus identity and generic editable effect rows"},
		]
	}

	valueObjects: PostEffectConfig: {
		description: "one validated Patch-owned post-effect instance configuration"
		state: {
			slotId: "EffectSlotId"
			capabilityId: "EffectCapabilityId"
			values: "Vec<ParameterAssignment>"
			assetReferences: "Vec<{parameterId: ParameterId, reference: AssetReference}>"
		}
		invariants: [
			"capabilityId resolves to exactly one installed EffectCapabilityDescriptor",
			"values and assetReferences match every required descriptor field exactly once in descriptor order and contain no undeclared, wrong-kind, non-finite, out-of-range, or dependency-invalid assignment",
			"the config contains no bypass, wet/dry duplicate, processor object, descriptor copy, prepared state, buffer, UI state, or fallback capability",
		]
		contributesTo: [
			{capability: "capability.static_patch_effect", contribution: "stores one canonical effect instance independently from engine and mixer state"},
			{capability: "capability.asynchronous_engine_selection", contribution: "remains byte-exact across instrument and preset replacement candidates"},
		]
	}

	valueObjects: EffectCapabilityRegistry: {
		description: "the immutable ordered registry of installed Patch effect descriptors"
		state: {descriptors: "Vec<EffectCapabilityDescriptor>"}
		invariants: [
			"EffectCapabilityIds are unique and descriptor order is stable",
			"lookup and PostEffectConfig validation use stable ids rather than labels or positions",
			"unknown, duplicate, invalid, missing-provider, or missing-preparer cases are typed and never bypass or substitute an effect",
			"the current production composition installs exactly effect.chorus and exposes no effect choice surface",
		]
		contributesTo: [{capability: "capability.static_patch_effect", contribution: "freezes the one installed effect schema before Patch installation and graph preparation"}]
	}

	ports: EffectCapabilityProvider: {
		direction: "outbound"
		contract: {
			descriptor: "() -> EffectCapabilityDescriptor"
			createConfig: "(slotId: EffectSlotId, values: &[ParameterAssignment], assetReferences: &[{parameterId: ParameterId, reference: AssetReference}]) -> Result<PostEffectConfig, EffectCapabilityError>"
		}
		consumes: ["valueObject.Synth.EffectCapabilityDescriptor", "valueObject.Synth.PostEffectConfig", "valueObject.Synth.ParameterAssignment", "valueObject.Synth.AssetReference"]
		invariants: [
			"operations run only on control or worker ownership and are deterministic",
			"the port contains no process, audio-buffer, graph, device, UI, file-I/O, engine, or fallback operation",
		]
		contributesTo: [{capability: "capability.static_patch_effect", contribution: "supplies effect schema/config without defining Patch or prepared DSP"}]
	}

	ports: PreparedPostEffect: {
		direction: "outbound"
		contract: {
			patchId: "() -> PatchId"
			slotId: "() -> EffectSlotId"
			process: "(&mut [f32], frameCount: usize, &RtPostEffectParameters) -> Result<(), PreparedEffectError>"
		}
		consumes: ["valueObject.Kernel.PatchId", "valueObject.Synth.EffectSlotId", "valueObject.RealTime.RtPostEffectParameters"]
		invariants: [
			"the port is object-safe and contains only bounded callback operations over one fully prepared Patch-local instance",
			"process mutates only the supplied matching interleaved stereo stem for at most the prepared maximum frames",
			"dynamic dispatch occurs at most once per configured slot per block and never inside the inner sample loop",
			"process performs no allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, exception, unwind, or destruction",
		]
		contributesTo: [
			{capability: "capability.static_patch_effect", contribution: "gives the effect rack one processor-neutral callback contract"},
			{capability: "capability.realtime_execution", contribution: "keeps effect implementation details outside the renderer"},
		]
	}

	ports: EffectPreparer: {
		direction: "outbound"
		contract: {
			capabilityId: "() -> EffectCapabilityId"
			prepare: "(patchId: PatchId, config: &PostEffectConfig, sampleRate: f32, maxFrames: usize) -> Result<Box<dyn PreparedPostEffect>, EffectPreparationError>"
		}
		consumes: ["valueObject.Kernel.PatchId", "valueObject.Synth.PostEffectConfig", "port.Synth.PreparedPostEffect"]
		invariants: [
			"preparation and every allocation, source validation, buffer construction, and warmup run outside the callback",
			"capabilityId exactly matches accepted configs and unsupported rate, capacity, source, config, or allocation fails with a typed error",
			"a successful result has finished every allocation and capacity decision and no failure selects bypass, a null effect, or another processor",
		]
		contributesTo: [{capability: "capability.static_patch_effect", contribution: "separates effect configuration from prepared callback ownership"}]
	}

	applicationServices: PreparedPostEffectRackBuilder: {
		purpose: "build the fixed-capacity ordered prepared post-effect slots for every accepted Patch outside the callback"
		uses: ["aggregate.Synth.Patch", "valueObject.Synth.EffectCapabilityRegistry", "port.Synth.EffectPreparer", "aggregate.RealTime.PreparedPostEffectRack"]
		operations: {
			build: {input: {patches: "&[Patch]", registry: "&EffectCapabilityRegistry", preparers: "&[Box<dyn EffectPreparer>]", sampleRate: "f32", maxFrames: "usize"}, output: {result: "Result<PreparedPostEffectRack, EffectRackPreparationError>"}}
		}
		meta: rules: [
			"preserve canonical Patch and ordered PostEffectConfig order and require at most one stable slot per Patch in this increment",
			"resolve every config through exactly one descriptor and identity-matched preparer and validate the returned PatchId and EffectSlotId",
			"fail atomically for duplicate ids, missing or extra registrations, capacity, sample-rate, config, or preparation errors and never publish a partial rack or bypass",
		]
		validations: [{id: "validation.service.prepared_post_effect_rack_builder", kind: "test", command: ["cargo", "test", "prepared_post_effect_rack_builder"], description: "exact zero/one-slot preparation succeeds while every registration, identity, config, rate, and capacity mismatch fails atomically"}]
		contributesTo: [
			{capability: "capability.static_patch_effect", contribution: "constructs one capability-neutral prepared effect rack from canonical Patch configs"},
			{capability: "capability.prepared_engine_rack", contribution: "joins effect preparation to the complete graph builder without changing engine rack ownership"},
		]
	}
}

project: contexts: RealTime: {
	valueObjects: RtPostEffectParameters: {
		description: "one fixed destructor-free effect slot scalar projection inside a Patch entry"
		state: {
			active: "bool"
			slotId: "EffectSlotId"
			scalarCount: "usize"
			scalars: "[f32; 8]"
		}
		invariants: [
			"inactive entries contain zero scalar count and active entries match one immutable prepared effect descriptor layout",
			"values are finite, choices use descriptor indices, and the value contains no String, Vec, asset, config, processor, pointer, reference, or destructor",
		]
		contributesTo: [{capability: "capability.static_patch_effect", contribution: "carries latest accepted Amount and Depth to one matching prepared Chorus slot"}]
	}

	valueObjects: PatchEffectObservation: {
		description: "fixed-size callback-local measurements around the configured Patch effect stage"
		state: {
			patchId: "Option<PatchId>"
			inputRms: "f32"
			outputRms: "f32"
			differenceRms: "f32"
			sideRms: "f32"
		}
		invariants: [
			"the value is Copy, fixed-size, numeric, finite, nonnegative, and measured from the exact stem immediately before and after PreparedPostEffectRack processing",
			"the observation never controls processing and uses no allocation, logging, formatting, locking, blocking, I/O, or borrowed audio buffer",
		]
		contributesTo: [
			{capability: "capability.static_patch_effect", contribution: "makes effect order and audible stereo consequence falsifiable in deterministic and physical demos"},
			{capability: "capability.live_observable_demo", contribution: "lets the control-side live checkpoint observe the real Patch effect stage"},
		]
	}

	aggregates: PreparedPostEffectRack: {
		root: true
		purpose: "own a fixed-capacity Patch-aligned set of ordered capability-neutral prepared post effects"
		state: {
			patchCount: "usize"
			slots: "[Option<{patchId: PatchId, slotId: EffectSlotId, effect: Box<dyn PreparedPostEffect>}>; MAX_PATCHES]"
		}
		invariants: [
			"construction occurs outside the callback and slot positions align exactly with PreparedEngineRack, PatchAudioBlock, and ParameterSnapshot Patch order",
			"the current bound is zero or one effect per Patch while processing semantics remain ordered",
			"process visits at most MAX_PATCHES slots, requires exact PatchId/EffectSlotId/scalar-layout agreement, mutates each configured Patch stem in place after synthesis and before mixing, and reports mismatch without bypass or cross-Patch processing",
			"each prepared instance owns independent delay/LFO state and no state, buffer, or tail is shared between Patches",
			"callback operations never allocate, deallocate, destroy, lock, block, log, format, perform I/O, panic, throw, or unwind",
		]
		validations: [{id: "validation.aggregate.prepared_post_effect_rack", kind: "test", command: ["cargo", "test", "prepared_post_effect_rack"], description: "ordered target-only in-place processing, layout rejection, zero-slot pass-through, and independent instance state are exact"}]
		contributesTo: [
			{capability: "capability.static_patch_effect", contribution: "owns the bounded Patch-local processing stage between synthesis and mix"},
			{capability: "capability.realtime_execution", contribution: "bounds effect dynamic dispatch and callback work"},
		]
	}
}

project: adapters: ChorusCapability: {
	implements: "port.Synth.EffectCapabilityProvider"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "effect_descriptor", system: "Chorus"}
	meta: rules: [
		"provide EffectCapabilityId effect.chorus, label Chorus, the chorus semantic accent, and exactly one section containing chorus.amount then chorus.depth",
		"declare Amount and Depth as finite Continuous Scalar ScalarEdit parameters in 0..=1 with default 0.5, fine step 0.01, coarse step 0.1, stable formatter metadata, and no asset requirement",
		"create a descriptor-ordered PostEffectConfig for the supplied stable EffectSlotId without reading files, constructing native state, or selecting another effect",
		"return typed errors for every invalid identity or assignment and own no prepared processor, C++ value, delay memory, Patch mutation, focus, or renderer behavior",
	]
	validations: [{id: "validation.adapter.chorus_capability", kind: "test", command: ["cargo", "test", "chorus_capability"], description: "the exact two-parameter schema, defaults, config validation, and no-fallback failures are stable"}]
	contributesTo: [
		{capability: "capability.static_patch_effect", contribution: "owns the product-facing Chorus schema independently from pinned native preparation"},
		{capability: "capability.schema_driven_patch_page", contribution: "supplies generic Amount and Depth PATCH rows without hard-coded projector logic"},
	]
}

project: adapters: ChorusPreparer: {
	implements: "port.Synth.EffectPreparer"
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "opaque-cpp-ffi", system: "Mutable Instruments Rings Chorus"}
	meta: {
		framework: "pinned C++ DSP + Rust RAII adapter"
		rules: [
			"compile only the audited MIT-licensed Chorus/FxEngine/resource subset pinned at pichenettes/eurorack@08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4 and stmlib@e3bd7c9cc00e4364166f9905c0509b6ffd0535ec behind one opaque extern-C wrapper with exceptions and RTTI disabled",
			"prepare a distinct fully initialized Chorus instance, 2048-sample 16-bit external delay buffer, LFO state, and bounded stereo scratch needs for every configured Patch slot outside the callback",
			"accept exactly 48000 Hz for this first admission and reject every unsupported rate or malformed scalar layout without bypass, alternate effect, or silent pass-through",
			"interpret exactly descriptor-ordered Amount then Depth and process the matching Patch's interleaved stereo stem in place with bounded arithmetic",
			"construct and destroy all native/owned state only outside callback ownership and cross no allocation, destruction, lock, block, I/O, log, format, panic, exception, or unwind path during processing",
		]
	}
	validations: [
		{id: "validation.adapter.chorus_preparer", kind: "test", command: ["cargo", "test", "chorus_preparer"], description: "pins, source hashes, license, lifecycle, exact-rate policy, scalar response, finite stereo output, and independent state are proven"},
		{id: "validation.adapter.chorus_preparer_integration", kind: "integration", command: ["cargo", "test", "--release", "--test", "static_patch_effect", "--", "--nocapture"], description: "the named production-path Chorus acceptance runs"},
	]
	contributesTo: [
		{capability: "capability.static_patch_effect", contribution: "adapts the pinned upstream Chorus to the generic prepared effect boundary"},
		{capability: "capability.realtime_execution", contribution: "keeps native effect state bounded, prepared, and callback safe"},
	]
}

project: assets: ChorusSourceBundle: {
	kind: "source"
	path: "vendor/chorus"
	layer: "infrastructure"
	targets: ["adapter.ChorusPreparer"]
	meta: rules: [
		"vendor only the Chorus, FxEngine, exact required resource table/header, stmlib subset, license, and provenance files required by the opaque wrapper",
		"record upstream URLs, exact eurorack and stmlib revisions, license notices, and a SHA-256 manifest for every vendored source",
		"retain the upstream notices but expose the product label only as Chorus; do not use Mutable Instruments or Rings as Crest product branding",
	]
	validations: [{id: "validation.asset.chorus_source_bundle", kind: "custom", command: ["cargo", "test", "chorus_source_provenance"], description: "all audited files, hashes, pins, and MIT notices are exact and no unrelated firmware/module code is present"}]
	contributesTo: [{capability: "capability.static_patch_effect", contribution: "makes native source and license provenance reproducible"}]
}

project: contexts: Testing: valueObjects: StaticPatchEffectObservation: {
	description: "the focused machine-readable evidence emitted by the named first Patch-effect acceptance"
	state: {
		schemaVersion: "u32"
		upstreamRevision: "String"
		stmlibRevision: "String"
		sourceHashesMatch: "bool"
		licensePresent: "bool"
		configuredPatchId: "PatchId"
		configuredEffectSlots: "u32"
		amountDepthCasesExercised: "u32"
		patchFocusOrderExact: "bool"
		scalarOnlyPublication: "bool"
		orderedBeforeMix: "bool"
		targetAudioDistinct: "bool"
		stereoSideEnergyNonzero: "bool"
		targetPatchExact: "bool"
		untargetedPatchesExact: "bool"
		independentInstances: "bool"
		independentTails: "bool"
		structuralConfigPreserved: "bool"
		unsupportedRateRejected: "bool"
		missingRegistrationRejected: "bool"
		fallbackCount: "u64"
		callbackReachableStrings: "u64"
		callbackAllocations: "u64"
		callbackDeallocations: "u64"
		callbackDestructions: "u64"
		p99RenderMicroseconds: "u64"
	}
	invariants: [
		"every field is measured through production descriptors, reducer, projector, preparers, complete graph, renderer, effect rack, mixer, and callback observation rather than copied from expected data",
		"the marker is emitted only after exact config/focus/scalar/order/isolation/independence/structural/source/RT predicates pass",
	]
	contributesTo: [{capability: "capability.static_patch_effect", contribution: "is the focused falsifiable acceptance result"}]
}

project: validations: static_patch_effect: {
	id: "validation.static_patch_effect"
	scope: "project"
	kind: "integration"
	command: ["cargo", "test", "--release", "--test", "static_patch_effect", "--", "--nocapture"]
	timeout: "240s"
	assertions: [
		{type: "exit-code", equals: 0},
		{type: "stdout-contains", value: "CREST_ACCEPTANCE static_patch_effect passed"},
	]
	resources: [
		"valueObject.Synth.EffectCapabilityId",
		"valueObject.Synth.EffectSlotId",
		"valueObject.Synth.EffectCapabilityDescriptor",
		"valueObject.Synth.PostEffectConfig",
		"valueObject.Synth.EffectCapabilityRegistry",
		"port.Synth.EffectCapabilityProvider",
		"port.Synth.PreparedPostEffect",
		"port.Synth.EffectPreparer",
		"applicationService.Synth.PreparedPostEffectRackBuilder",
		"aggregate.Synth.Patch",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"valueObject.RealTime.RtPostEffectParameters",
		"valueObject.RealTime.PatchEffectObservation",
		"aggregate.RealTime.PreparedPostEffectRack",
		"aggregate.RealTime.PreparedGraph",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"adapter.ChorusCapability",
		"adapter.ChorusPreparer",
		"asset.ChorusSourceBundle",
		"valueObject.Testing.StaticPatchEffectObservation",
		"asset.StaticPatchEffectAcceptanceTests",
	]
	capabilities: ["capability.static_patch_effect", "capability.schema_driven_patch_page", "capability.one_way_parameter_control", "capability.prepared_engine_rack", "capability.global_mix", "capability.realtime_execution"]
	goals: ["goal.shape_patch_with_effect", "goal.play_test_song"]
	description: "the production path proves the pinned Chorus schema, canonical PATCH editing, fixed scalar transport, engine-effect-mix order, Patch/instance isolation, structural preservation, audible stereo output, typed failures, source provenance, and callback/timing contracts"
}

project: witnesses: static_patch_effect: {
	scope: "goal"
	goal: "goal.shape_patch_with_effect"
	capability: "capability.static_patch_effect"
	resources: [
		"valueObject.Synth.PostEffectConfig",
		"aggregate.Synth.Patch",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"aggregate.RealTime.PreparedPostEffectRack",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"adapter.ChorusCapability",
		"adapter.ChorusPreparer",
		"asset.ChorusSourceBundle",
		"valueObject.Testing.StaticPatchEffectObservation",
		"asset.StaticPatchEffectAcceptanceTests",
	]
	repairResources: [
		"valueObject.Synth.PostEffectConfig",
		"aggregate.Synth.Patch",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"aggregate.RealTime.PreparedPostEffectRack",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"adapter.ChorusCapability",
		"adapter.ChorusPreparer",
	]
	evidence: ["evidence.static_patch_effect_contract"]
	command: ["cargo", "test", "--release", "--test", "static_patch_effect", "--", "--nocapture"]
	timeout: "240s"
	artifacts: ["target/release/deps/static_patch_effect-*"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_PATCH_EFFECT_OBSERVATION "
		schema: {
			schemaVersion: "number"
			upstreamRevision: "string"
			stmlibRevision: "string"
			sourceHashesMatch: "bool"
			licensePresent: "bool"
			configuredPatchId: "number"
			configuredEffectSlots: "number"
			amountDepthCasesExercised: "number"
			patchFocusOrderExact: "bool"
			scalarOnlyPublication: "bool"
			orderedBeforeMix: "bool"
			targetAudioDistinct: "bool"
			stereoSideEnergyNonzero: "bool"
			targetPatchExact: "bool"
			untargetedPatchesExact: "bool"
			independentInstances: "bool"
			independentTails: "bool"
			structuralConfigPreserved: "bool"
			unsupportedRateRejected: "bool"
			missingRegistrationRejected: "bool"
			fallbackCount: "number"
			callbackReachableStrings: "number"
			callbackAllocations: "number"
			callbackDeallocations: "number"
			callbackDestructions: "number"
			p99RenderMicroseconds: "number"
		}
	}
	predicates: [
		{field: "schemaVersion", op: "eq", value: 1},
		{field: "upstreamRevision", op: "eq", value: "08460a69a7e1f7a81c5a2abcc7189c9a6b7208d4"},
		{field: "stmlibRevision", op: "eq", value: "e3bd7c9cc00e4364166f9905c0509b6ffd0535ec"},
		{field: "sourceHashesMatch", op: "eq", value: true},
		{field: "licensePresent", op: "eq", value: true},
		{field: "configuredEffectSlots", op: "eq", value: 1},
		{field: "amountDepthCasesExercised", op: "eq", value: 2},
		{field: "patchFocusOrderExact", op: "eq", value: true},
		{field: "scalarOnlyPublication", op: "eq", value: true},
		{field: "orderedBeforeMix", op: "eq", value: true},
		{field: "targetAudioDistinct", op: "eq", value: true},
		{field: "stereoSideEnergyNonzero", op: "eq", value: true},
		{field: "targetPatchExact", op: "eq", value: true},
		{field: "untargetedPatchesExact", op: "eq", value: true},
		{field: "independentInstances", op: "eq", value: true},
		{field: "independentTails", op: "eq", value: true},
		{field: "structuralConfigPreserved", op: "eq", value: true},
		{field: "unsupportedRateRejected", op: "eq", value: true},
		{field: "missingRegistrationRejected", op: "eq", value: true},
		{field: "fallbackCount", op: "eq", value: 0},
		{field: "callbackReachableStrings", op: "eq", value: 0},
		{field: "callbackAllocations", op: "eq", value: 0},
		{field: "callbackDeallocations", op: "eq", value: 0},
		{field: "callbackDestructions", op: "eq", value: 0},
		{field: "p99RenderMicroseconds", op: "lt", value: 2666},
	]
}

project: assets: StaticPatchEffectAcceptanceTests: {
	kind: "rust-integration-tests"
	description: "tests/static_patch_effect.rs, the non-vacuous production-path acceptance for the first static Patch effect"
	profile: {kind: "verification_harness", witness: "configured Chorus insert", failurePolicy: "missing target, marker, source proof, effect difference, ordering, isolation, or callback safety fails"}
	targets: [
		"adapter.ChorusCapability",
		"adapter.ChorusPreparer",
		"applicationService.Synth.PreparedPostEffectRackBuilder",
		"aggregate.RealTime.PreparedPostEffectRack",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"valueObject.Testing.StaticPatchEffectObservation",
	]
	prompts: [
		"Create tests/static_patch_effect.rs with ordinary assertions and emit CREST_ACCEPTANCE static_patch_effect passed only after the structured observation satisfies every witness predicate.",
		"Use production provider/preparer registration, Patch/AppState, projector/AppLoop, complete graph builder, effect rack, renderer, mixer, and observation seams; deterministic fixtures may assemble inputs but must not duplicate reducer, effect processing, routing, or verdict logic.",
		"Prove exact pins/hashes/license, Amount/Depth schema and fine/coarse edits, scalar-only publication, engine-effect-mix order, first-Patch-only production config, target/untargeted isolation, at least two independently stateful Chorus instances in the focused test, structural engine/preset preservation, unsupported rate and missing/mismatch failure, finite stereo difference, zero fallback, callback allocation/destruction, and p99 timing.",
		"The exhaustive and live demos must execute the same two effect parameter controls; this focused target does not replace either demo gate.",
	]
	validations: [{id: "validation.asset.static_patch_effect_acceptance", kind: "integration", command: ["cargo", "test", "--release", "--test", "static_patch_effect", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE static_patch_effect passed"}], description: "the named Chorus target exists, executes assertions, and emits its marker only after structured predicates pass"}]
	contributesTo: [
		{capability: "capability.static_patch_effect", contribution: "provides the focused executable architecture and DSP witness"},
		{capability: "capability.observable_demo_scene", contribution: "keeps focused and exhaustive effect proof on production seams"},
	]
}
