package crestsynth

// Phase 3 SoundFont preset selection. This file adds the catalog and semantic
// structural-choice contract; existing capability, control, worker, graph, demo,
// and callback declarations are reconciled to these resources in their owning
// context files.

project: goals: select_soundfont_preset: {
	description: "The player can see the fixed SoundFont's authored preset names in numeric SF2 bank/program order and replace the focused SoundFont Patch preset through the same prepared structural workflow as engine selection"
	priority: "required"
	actors: ["actor.player", "actor.maintainer"]
	dependsOn: ["goal.edit_patch_envelope"]
	capabilities: [
		"capability.soundfont_preset_selection",
		"capability.instrument_capability_model",
		"capability.schema_driven_patch_page",
		"capability.asynchronous_engine_selection",
		"capability.prepared_engine_rack",
		"capability.realtime_execution",
	]
	requirements: [
		"requirement.sf2_preset_catalog_fidelity",
		"requirement.catalog_backed_soundfont_schema",
		"requirement.descriptor_derived_preset_focus",
		"requirement.correlated_preset_replacement",
		"requirement.soundfont_metadata_rt_separation",
		"requirement.soundfont_preset_behavioral_proof",
	]
}

project: capabilities: soundfont_preset_selection: {
	description: "Discover playable SF2 presets once, preserve exact authored names over stable numeric identities, project them as one descriptor-owned structural Choice, and select an adjacent entry without fallback"
	goals: ["goal.select_soundfont_preset", "goal.observe_synth", "goal.observe_live_synth"]
	acceptance: named_ordered_structural_choice: {
		description: "the fixed SoundFont catalog, PATCH row, reducer lifecycle, prepared graph, and audible target remain one correlated production path"
		actor: "actor.maintainer"
		steps: [
			{action: "load ./sf2/HiDef.sf2 at startup", observes: "one parse produces a control-side catalog with exact achPresetName labels sorted by numeric wBank then wPreset and a separate numeric prepared bank"},
			{action: "inspect a focused SoundFont Patch", observes: "the descriptor contains one soundfont.preset Choice plus the locked soundfont.file asset, and PATCH focuses Preset after Release without a SoundFont branch"},
			{action: "hold Edit and press Right on Preset", observes: "one semantic structural intent enters AppState.apply, the old preset remains active during preparation, and only the focused Patch config commits after a matching candidate is ready"},
			{action: "render and acknowledge the candidate", observes: "the newer graph uses the exact selected numeric address, produces finite nonzero distinct target audio, retires its predecessor off callback, and reports Ready with the authored name"},
			{action: "attempt a boundary, missing address, malformed catalog, stale result, or busy edit", observes: "the typed rejection or visible failure preserves the source config and graph and never substitutes a name, preset, asset, or engine"},
		]
		evidence: ["evidence.soundfont_preset_selection_contract", "evidence.exhaustive_demo_scene", "evidence.live_demo_contract"]
	}
}

project: requirements: {
	sf2_preset_catalog_fidelity: {
		kind: "functional"
		description: "The single fixed SF2 parse produces one immutable SoundFontPresetCatalog containing every effective playable preset's stable wBank/wPreset identity and exact NUL-terminated achPresetName presentation; selectable entries sort numerically by bank then program rather than raw phdr or alphabetical order, preserve duplicate names, apply SF2 first-record precedence to duplicate coordinates with a typed diagnostic, and never synthesize General MIDI names"
		goals: ["goal.select_soundfont_preset"]
		capabilities: ["capability.soundfont_preset_selection", "capability.soundfont_audio"]
	}
	catalog_backed_soundfont_schema: {
		kind: "functional"
		description: "The HiDef descriptor replaces independent bank/program/percussion values with one soundfont.preset Structural Choice whose ids encode numeric SoundFontPresetId values and labels are exact catalog names, retains only the locked required soundfont.file asset, and uses the first sorted playable entry as its default"
		goals: ["goal.select_soundfont_preset"]
		capabilities: ["capability.soundfont_preset_selection", "capability.instrument_capability_model"]
	}
	descriptor_derived_preset_focus: {
		kind: "functional"
		description: "PatchControlId represents Capability(ParameterId), and the focused Patch resolver appends visible descriptor parameters classified StructuralChoice after Engine and the canonical ADSR rows; SoundFont therefore exposes one Preset control while Braids exposes none, and the projector, reducer, text shell, schema coverage, and demos consume the same resolver"
		goals: ["goal.select_soundfont_preset"]
		capabilities: ["capability.soundfont_preset_selection", "capability.schema_driven_patch_page", "capability.one_way_parameter_control"]
	}
	correlated_preset_replacement: {
		kind: "nonfunctional"
		description: "Edit+Left/Right on the focused preset creates one typed ReplaceParameterChoice intent and uses the existing one-in-flight reducer, capacity-one worker, complete graph build, structural handoff, block-boundary activation, and off-callback retirement protocol; the old config/audio remains active through Preparing, only the selected assignment changes on commit, and every failure preserves it without fallback"
		goals: ["goal.select_soundfont_preset"]
		capabilities: ["capability.soundfont_preset_selection", "capability.asynchronous_engine_selection", "capability.prepared_engine_rack"]
	}
	soundfont_metadata_rt_separation: {
		kind: "nonfunctional"
		description: "The raw parsed SF2 and all authored names remain on control/worker ownership; callback-reachable PreparedSoundFontBank storage contains only immutable numeric PCM, zones, loop data, and preset addresses, and preset replacement adds no callback allocation, destruction, string access, parsing, lookup by label, locking, blocking, I/O, logging, formatting, panic, unwind, or fallback"
		goals: ["goal.select_soundfont_preset"]
		capabilities: ["capability.soundfont_preset_selection", "capability.soundfont_audio", "capability.realtime_execution"]
	}
	soundfont_preset_behavioral_proof: {
		kind: "functional"
		description: "A named real-SF2 acceptance target plus deterministic and physical demos prove exact catalog names/order, fixture identity resolution, dynamic focus/projection, both adjacent directions and boundaries, pending source audio, target-only config mutation, controlled failure/stale/busy preservation, newer acknowledged revisions, finite distinct target output, zero callback allocation/destruction, exact final restoration, and no fallback"
		goals: ["goal.select_soundfont_preset", "goal.observe_synth", "goal.observe_live_synth"]
		capabilities: ["capability.soundfont_preset_selection", "capability.observable_demo_scene", "capability.live_observable_demo", "capability.realtime_execution"]
	}
}

project: contexts: Synth: {
	ubiquitousLanguage: {
		SoundFontPresetId: "the stable SF2 wBank/wPreset playback address; authored name is presentation only"
		SoundFontPresetCatalog: "the immutable control-side ordered entries derived from the fixed SF2 parse"
	}

	valueObjects: SoundFontPresetId: {
		description: "one canonical numeric SoundFont preset identity with stable generic-choice encoding"
		state: {
			bank: "u16"
			program: "u8"
			choiceId: "sf2.bank-<bank>.program-<program>"
		}
		invariants: [
			"program is in 0..=127 and bank is the exact SF2 wBank value used for lookup",
			"choiceId is a reversible canonical decimal encoding with no name, ordinal, registry index, or filesystem path",
			"bank 128 identifies conventional SF2 percussion; percussion is derived for engine-channel configuration and is not a second config assignment",
		]
		contributesTo: [
			{capability: "capability.soundfont_preset_selection", contribution: "keeps choice identity stable when an authored label changes or duplicates another label"},
			{capability: "capability.soundfont_audio", contribution: "supplies the exact numeric bank/program address consumed during off-callback preparation"},
		]
	}

	valueObjects: SoundFontPresetCatalogEntry: {
		description: "one effective playable preset's identity, authored name, and source diagnostic position"
		state: {
			id: "SoundFontPresetId"
			name: "String"
			sourceOrdinal: "usize"
		}
		invariants: [
			"name is the exact nonempty achPresetName content before its first NUL, with case and authored spaces preserved",
			"name is never used for lookup or identity and duplicate names at distinct numeric addresses are preserved",
			"sourceOrdinal is diagnostic only and never determines normal display order",
		]
		contributesTo: [{capability: "capability.soundfont_preset_selection", contribution: "joins exact SF2 presentation to one numeric choice identity"}]
	}

	valueObjects: SoundFontPresetCatalog: {
		description: "the immutable control-side collection shared by the HiDef provider and preparer"
		state: {
			entries: "Vec<SoundFontPresetCatalogEntry>"
			coordinateCollisions: "Vec<{id: SoundFontPresetId, firstOrdinal, shadowedOrdinal}>"
		}
		invariants: [
			"construction occurs exactly once from the fixed parsed SF2 outside the callback and fails if there is no playable entry or an effective entry has an invalid program or empty authored name",
			"an entry is playable only when its preset resolves at least one valid prepared sample region",
			"entries are sorted by numeric bank then numeric program; the original phdr order and label collation never define normal ordering",
			"duplicate numeric coordinates retain the first playable SF2 record as the sole selectable entry and record each shadowed coordinate collision without renaming or substituting it",
			"lookup supports stable choice id and normalized MIDI fixture identity and returns a typed missing-preset error without nearest-neighbor or default fallback",
			"the catalog contains strings and never crosses into PreparedGraph, PreparedInstrument, ParameterSnapshot, AudioCommand, or callback observation ownership",
		]
		validations: [{id: "validation.value_object.soundfont_preset_catalog", kind: "test", command: ["cargo", "test", "soundfont_preset_catalog"], description: "synthetic reordered, duplicate-name, coordinate-collision, empty-name, invalid-program, and empty-zone fixtures prove exact deterministic catalog behavior"}]
		contributesTo: [
			{capability: "capability.soundfont_preset_selection", contribution: "is the single source for descriptor choices, fixture address validation, selected labels, and preparer address lookup"},
			{capability: "capability.instrument_capability_model", contribution: "hydrates one immutable descriptor before registry freeze without adding asset I/O to the provider port"},
	]
	}
}

project: contexts: Control: valueObjects: StructuralEditIntent: {
	description: "the capability-neutral target of one app-wide prepared structural request"
	state: {
		kind: "ReplaceCapability | ReplaceParameterChoice"
		targetCapabilityId: "Option<CapabilityId>"
		targetParameterId: "Option<ParameterId>"
		targetChoiceId: "Option<String>"
	}
	invariants: [
		"ReplaceCapability contains only one installed target CapabilityId and no parameter payload",
		"ReplaceParameterChoice contains the active CapabilityId, one descriptor-declared StructuralChoice ParameterId, and one adjacent declared choice id",
		"the intent contains no display label, prepared graph, worker, engine, asset contents, or callback owner",
		"engine and preset selection use this one intent type, request counter, status, effect, and busy guard rather than parallel lifecycles",
	]
	contributesTo: [
		{capability: "capability.asynchronous_engine_selection", contribution: "retains adjacent installed-capability selection as one structural intent"},
		{capability: "capability.soundfont_preset_selection", contribution: "addresses a preset change generically by ParameterId and stable choice id"},
	]
}

project: adapters: HiDefSoundFontAsset: {
	layer: "infrastructure"
	profile: {kind: "in_process", medium: "sf2", system: "HiDef.sf2"}
	meta: {
		framework: "rustysynth"
		rules: [
			"open and parse exactly ./sf2/HiDef.sf2 once on control ownership before registry freeze, fixture installation, graph preparation, or audio startup",
			"from that one parse build SoundFontPresetCatalog plus a separate immutable numeric PreparedSoundFontBank containing copied PCM, prepared regions, loop metadata, and SoundFontPresetId addresses",
			"drop the raw rustysynth SoundFont and every parser/name-bearing structure before returning the numeric bank to any preparer that can create callback-owned instruments",
			"give the catalog to HiDefSoundFontCapability and the catalog plus numeric bank to HiDefSoundFontPreparer through composition-root construction; neither reparses the asset",
			"return typed asset, metadata, allocation, sample, region, empty-catalog, or coordinate failures without a partial provider, preparer, catalog, bank, synthesized label, alternate asset, or fallback engine",
		]
	}
	validations: [{id: "validation.adapter.hidef_soundfont_asset", kind: "test", command: ["cargo", "test", "hidef_soundfont_asset"], description: "one parse yields exact separated catalog/numeric projections and raw string-bearing parser state is absent from callback-reachable ownership"}]
	contributesTo: [
		{capability: "capability.soundfont_preset_selection", contribution: "discovers exact selectable names and numeric order once before descriptor freeze"},
		{capability: "capability.soundfont_audio", contribution: "prepares the shared numeric sample/zone bank without retaining raw SF2 metadata"},
		{capability: "capability.realtime_execution", contribution: "keeps file parsing, strings, allocation, and raw asset destruction outside callback ownership"},
	]
}

project: validations: soundfont_preset_selection: {
	id: "validation.soundfont_preset_selection"
	scope: "project"
	kind: "integration"
	command: ["cargo", "test", "--release", "--test", "soundfont_preset_selection", "--", "--nocapture"]
	timeout: "300s"
	assertions: [
		{type: "exit-code", equals: 0},
		{type: "stdout-contains", value: "CREST_ACCEPTANCE soundfont_preset_selection passed"},
	]
	resources: [
		"valueObject.Synth.SoundFontPresetId",
		"valueObject.Synth.SoundFontPresetCatalogEntry",
		"valueObject.Synth.SoundFontPresetCatalog",
		"valueObject.Control.StructuralEditIntent",
		"adapter.HiDefSoundFontAsset",
		"adapter.HiDefSoundFontCapability",
		"adapter.HiDefSoundFontPreparer",
		"valueObject.Control.PatchControlId",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"applicationService.RealTime.AudioRenderer",
		"asset.SoundFontPresetSelectionAcceptanceTests",
	]
	capabilities: ["capability.soundfont_preset_selection", "capability.instrument_capability_model", "capability.schema_driven_patch_page", "capability.asynchronous_engine_selection", "capability.prepared_engine_rack", "capability.realtime_execution"]
	goals: ["goal.select_soundfont_preset"]
	description: "the real fixed SF2, canonical reducer, generic structural workflow, complete graph handoff, and production renderer prove authored-name preset selection without callback metadata or fallback"
}

project: assets: SoundFontPresetSelectionAcceptanceTests: {
	kind: "source"
	path: "tests/soundfont_preset_selection.rs"
	layer: "infrastructure"
	implements: ["validation.soundfont_preset_selection"]
	meta: rules: [
		"parse the real fixed SoundFont through HiDefSoundFontAsset and assert the full catalog is nonempty, numerically sorted, exact-name preserving, descriptor-identical, and observably different from raw phdr order and alphabetical order where the fixture discriminates",
		"assert every bank-0 address present is ordered by program, bank extensions follow numerically, bank 128 follows lower banks, choice ids round-trip to exact addresses, and labels equal rustysynth achPresetName values without a General MIDI lookup table",
		"drive PATCH to the descriptor-derived preset control through KeyboardInputTranslator and AppLoop, select right and left, exercise both catalog endpoints, and compare exact state, page/text/tree projection, status/effects, config isolation, generation, and graph revision",
		"render the old graph during preparation, activate the exact selected preset through the production worker/coordinator/renderer, require finite nonzero target-only output and a controlled audible difference from identical fresh state, and restore the descriptor default",
		"exercise missing fixture address, empty or invalid catalog metadata, busy, controlled preparation failure, early/stale/mismatched result and acknowledgement, queue pressure, and absent target output without fallback or callback destruction",
		"emit the acceptance marker only after the structured observation proves catalog fidelity, exact target identity, one parse, no callback-reachable strings, zero callback allocation/destruction, and off-callback retirement",
	]
}

project: evidence: soundfont_preset_selection_contract: {
	kind: "behavioral"
	description: "the real SF2 catalog, descriptor, dynamic PATCH focus, reducer, worker, graph handoff, and renderer agree on exact authored-name preset identity and audible fallback-free replacement"
	validations: ["validation.soundfont_preset_selection", "validation.capability_schema", "validation.patch_page_projection", "validation.engine_selection_workflow", "validation.demo_scene", "validation.live_demo", "validation.test"]
	witnesses: ["witness.soundfont_preset_selection"]
}

project: witnesses: soundfont_preset_selection: {
	scope: "goal"
	goal: "goal.select_soundfont_preset"
	capability: "capability.soundfont_preset_selection"
	resources: [
		"valueObject.Synth.SoundFontPresetId",
		"valueObject.Synth.SoundFontPresetCatalog",
		"valueObject.Control.StructuralEditIntent",
		"adapter.HiDefSoundFontAsset",
		"adapter.HiDefSoundFontCapability",
		"adapter.HiDefSoundFontPreparer",
		"valueObject.Control.PatchControlId",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"applicationService.RealTime.AudioRenderer",
		"asset.SoundFontPresetSelectionAcceptanceTests",
	]
	repairResources: [
		"valueObject.Synth.SoundFontPresetId",
		"valueObject.Synth.SoundFontPresetCatalog",
		"adapter.HiDefSoundFontAsset",
		"adapter.HiDefSoundFontCapability",
		"adapter.HiDefSoundFontPreparer",
		"valueObject.Control.PatchControlId",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"applicationService.RealTime.AudioRenderer",
	]
	evidence: ["evidence.soundfont_preset_selection_contract"]
	command: ["cargo", "test", "--release", "--test", "soundfont_preset_selection", "--", "--nocapture"]
	timeout: "300s"
	artifacts: ["target/release/deps/soundfont_preset_selection-*"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_SOUNDFONT_PRESET_OBSERVATION "
		schema: {
			parsed_soundfonts: "number"
			catalog_entries: "number"
			exact_authored_names: "bool"
			numeric_order_exact: "bool"
			raw_order_rejected: "bool"
			alphabetical_order_rejected: "bool"
			gm_names_synthesized: "number"
			fixture_addresses_resolved: "bool"
			preset_control_descriptor_derived: "bool"
			adjacent_directions_exercised: "number"
			boundaries_exercised: "number"
			active_graph_revision: "number"
			target_config_exact: "bool"
			untargeted_configs_exact: "bool"
			target_audio_nonzero: "bool"
			target_audio_distinct: "bool"
			fallbacks: "number"
			callback_reachable_strings: "number"
			callback_allocations: "number"
			callback_destructions: "number"
		}
	}
	predicates: [
		{field: "parsed_soundfonts", op: "eq", value: 1},
		{field: "catalog_entries", op: "gt", value: 1},
		{field: "exact_authored_names", op: "eq", value: true},
		{field: "numeric_order_exact", op: "eq", value: true},
		{field: "raw_order_rejected", op: "eq", value: true},
		{field: "alphabetical_order_rejected", op: "eq", value: true},
		{field: "gm_names_synthesized", op: "eq", value: 0},
		{field: "fixture_addresses_resolved", op: "eq", value: true},
		{field: "preset_control_descriptor_derived", op: "eq", value: true},
		{field: "adjacent_directions_exercised", op: "eq", value: 2},
		{field: "boundaries_exercised", op: "eq", value: 2},
		{field: "active_graph_revision", op: "gt", value: 0},
		{field: "target_config_exact", op: "eq", value: true},
		{field: "untargeted_configs_exact", op: "eq", value: true},
		{field: "target_audio_nonzero", op: "eq", value: true},
		{field: "target_audio_distinct", op: "eq", value: true},
		{field: "fallbacks", op: "eq", value: 0},
		{field: "callback_reachable_strings", op: "eq", value: 0},
		{field: "callback_allocations", op: "eq", value: 0},
		{field: "callback_destructions", op: "eq", value: 0},
	]
}
