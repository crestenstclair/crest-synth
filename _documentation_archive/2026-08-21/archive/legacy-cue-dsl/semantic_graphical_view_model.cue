package crestsynth

// Product-roadmap Phase 2: replace the Phase 1 hint-only shell contract with
// one canonical semantic graphical view model while retaining the graphical
// shell, diagnostic projection, reducer, audio boundaries, and teardown path.

project: goals: use_semantic_graphical_view_model: {
	description: "The player can traverse PATCH and MIXER through one reducer-owned semantic focus contract while any graphical layout renders the same immutable content, modes, valid actions, status, errors, and return path"
	priority: "required"
	actors: ["actor.player", "actor.maintainer"]
	dependsOn: ["goal.use_graphical_shell"]
	capabilities: [
		"capability.semantic_graphical_view_model",
		"capability.graphical_application_shell",
		"capability.one_way_parameter_control",
		"capability.schema_driven_patch_page",
		"capability.observable_demo_scene",
		"capability.live_observable_demo",
	]
	requirements: [
		"requirement.canonical_semantic_focus_contract",
		"requirement.descriptor_driven_graphical_projection",
		"requirement.exact_valid_action_contract",
		"requirement.deterministic_focus_recovery",
		"requirement.passive_semantic_action_boundary",
		"requirement.semantic_view_model_behavioral_proof",
	]
}

project: capabilities: semantic_graphical_view_model: {
	description: "Project canonical state into a layout-neutral semantic model with stable focus, modes, return paths, exact valid actions, descriptor-derived content, status, and typed errors"
	goals: ["goal.use_semantic_graphical_view_model"]
	acceptance: canonical_semantic_navigation: {
		description: "the reducer, projector, passive eframe shell, deterministic proof, and physical live scene agree on one semantic interaction model"
		actor: "actor.maintainer"
		steps: [
			{action: "inspect the current graphical projection in PATCH and MIXER", observes: "one SemanticGraphicalViewModel names the exact context, active surface, stable FocusPath, interaction mode, optional return path, valid SemanticActions, status, typed errors, and descriptor-derived content for the accepted generation"},
			{action: "navigate each main surface and enter its persistent Utility or Inspector", observes: "AppState.apply alone changes the semantic FocusPath, entering a side surface records the exact main-surface ReturnPath, and Return restores that origin without changing session, graph, parameter, or audio state"},
			{action: "hold and release Edit around a valid directional adjustment", observes: "normalized SemanticActions move the reducer-owned mode Navigate to Adjust and back, amber-versus-cyan state is projected explicitly, and only the accepted adjustment changes its canonical target"},
			{action: "change viewport and replace a descriptor schema while a removable control is focused", observes: "layout movement leaves the FocusPath identical and schema removal recovers to the nearest surviving visible enabled sibling by one deterministic resolver"},
			{action: "exercise structural Ready, Preparing, Activating, Failed, and recovery states", observes: "status and typed errors are projected from canonical lifecycle data without adapter strings, fallback, or UI-owned state; the physical healthy path represents the empty error set explicitly"},
			{action: "run make demo-live-semantic-view-model", observes: "real fixture audio continues while both contexts, four surfaces, Navigate and Adjust, valid actions, exact return paths, structural statuses, and focus recovery are visibly correlated before complete live teardown"},
		]
		evidence: ["evidence.semantic_graphical_view_model_contract"]
	}
}

project: requirements: {
	canonical_semantic_focus_contract: {
		kind: "functional"
		description: "InteractionState owns exactly one active FocusPath plus remembered root paths, InteractionMode, and an optional ReturnPath; paths use TopLevelContext, SurfaceId, stable Patch/capability/control identities, and no vector index or widget identity, while PATCH/MIXER remain the only top-level contexts and Phase 2 admits only Navigate and Adjust as reachable modes"
		goals: ["goal.use_semantic_graphical_view_model"]
		capabilities: ["capability.semantic_graphical_view_model", "capability.one_way_parameter_control"]
	}
	descriptor_driven_graphical_projection: {
		kind: "functional"
		description: "StateProjector derives one immutable SemanticGraphicalViewModel from the accepted AppState generation; PATCH controls come from PatchControlId plus instrument/effect/output descriptors, MIXER always contains sixteen stable MixerTrackId columns with track-owned controls plus distinct globals, side surfaces use canonical Patch Utility and Mixer Inspector models, and status/errors come from typed runtime lifecycle data without engine, effect, adapter, or layout branches"
		goals: ["goal.use_semantic_graphical_view_model"]
		capabilities: ["capability.semantic_graphical_view_model", "capability.instrument_capability_model", "capability.static_patch_effect", "capability.schema_driven_patch_page"]
	}
	exact_valid_action_contract: {
		kind: "nonfunctional"
		description: "Each projected focus exposes one ordered duplicate-free set of SemanticActions that AppState would accept in that exact context, surface, mode, bounds, dependency, and structural status; unavailable directions and modes are absent, footer hints derive only from this set, and action availability never mutates canonical state or predicts worker success"
		goals: ["goal.use_semantic_graphical_view_model"]
		capabilities: ["capability.semantic_graphical_view_model", "capability.one_way_parameter_control", "capability.asynchronous_engine_selection"]
	}
	deterministic_focus_recovery: {
		kind: "nonfunctional"
		description: "Responsive layout never changes semantic focus; after a committed schema/dependency change, the reducer retains the exact stable target when valid or searches the prior surface order outward for the nearest surviving visible enabled sibling with next-before-previous tie breaking, applies the same rule to remembered and return paths, and never lets a projector or widget repair focus"
		goals: ["goal.use_semantic_graphical_view_model"]
		capabilities: ["capability.semantic_graphical_view_model", "capability.instrument_capability_model", "capability.schema_driven_patch_page"]
	}
	passive_semantic_action_boundary: {
		kind: "nonfunctional"
		description: "KeyboardInputTranslator and future passive components emit only the closed SemanticAction union; AppLoop converts user actions to AppEvents before AppState.apply, system and MIDI events retain their existing AppEvent entry, and no view, eframe adapter, component, demo, or input adapter mutates focus, mode, return path, session, runtime, graph, or audio state directly"
		goals: ["goal.use_semantic_graphical_view_model"]
		capabilities: ["capability.semantic_graphical_view_model", "capability.one_way_parameter_control", "capability.graphical_application_shell"]
	}
	semantic_view_model_behavioral_proof: {
		kind: "functional"
		description: "A named headless production-path target proves exact action/event mapping, focus/mode/return/action/status/error projection, descriptor polymorphism, schema recovery, responsive focus invariance, generation coherence, passive egui rendering, and audio neutrality; the retained release-mode make demo-live-semantic-view-model target proves healthy physical traversal, focus recovery, audio continuity, cleanup, and normal exit"
		goals: ["goal.use_semantic_graphical_view_model"]
		capabilities: ["capability.semantic_graphical_view_model", "capability.graphical_application_shell", "capability.observable_demo_scene", "capability.live_observable_demo", "capability.realtime_execution"]
	}
}

project: contexts: Control: {
	ubiquitousLanguage: {
		SemanticAction: "one normalized user intent emitted by an input adapter or passive view"
		SurfaceId: "one stable graphical surface identity inside PATCH or MIXER, independent of layout placement"
		FocusPath: "the single reducer-owned semantic location of interaction"
		ReturnPath: "the exact stable origin restored when leaving a subordinate surface"
		SemanticGraphicalViewModel: "the immutable layout-neutral graphical projection consumed by every host"
	}

	valueObjects: SemanticAction: {
		description: "the closed bounded user-intent union between physical/passive input and AppEvent"
		state: {
			kind: "SelectContext | Navigate | Adjust | SetInteractionMode | EnterSurface | Return"
			payload: "TopLevelContext | Direction | InteractionMode | SurfaceId | none, according to kind"
			surfaceDescriptor: "typed exhaustive descriptors for every action and admitted payload"
		}
		invariants: [
			"SelectContext, Navigate, and Adjust reuse TopLevelContext and Direction; SetInteractionMode carries Navigate or Adjust in Phase 2; EnterSurface carries one context-compatible side SurfaceId; Return has no payload",
			"Modal and MultiSelect are named InteractionMode values but no Phase 2 SemanticAction can enter them until a later workflow defines its focus trap, exit, and valid-action rules",
			"the value contains no raw key/button, widget, label, parameter value, AppState, engine, graph, audio buffer, device, or callback",
			"AppLoop maps each user action to exactly one matching closed AppEvent before AppState.apply; startup, MIDI, worker, and system events do not masquerade as SemanticAction",
		]
		contributesTo: [
			{capability: "capability.semantic_graphical_view_model", contribution: "is both the passive view output and the exact valid-action vocabulary"},
			{capability: "capability.one_way_parameter_control", contribution: "makes physical normalization and reducer input separate explicit stages"},
		]
	}

	valueObjects: SurfaceId: {
		description: "the closed stable identity of the four Phase 2 graphical surfaces"
		from: "PatchMain | PatchUtility | MixerMain | MixerInspector"
		invariants: [
			"PatchMain and PatchUtility belong only to PATCH; MixerMain and MixerInspector belong only to MIXER",
			"the identity is invariant across desktop and Steam Deck composition and never encodes a pane index, rectangle, visibility shortcut, or egui id",
			"PatchUtility owns the canonical Patch trim and output-track controls introduced by the corrective routing gate; MixerInspector owns selected-track sends and routed-Patch summary, and later visual work must reuse these identities rather than invent parallel surfaces",
		]
		contributesTo: [{capability: "capability.semantic_graphical_view_model", contribution: "lets more than one layout place the same canonical surface model"}]
	}

	valueObjects: InteractionMode: {
		description: "the explicit reducer-owned interaction interpretation projected to every host"
		from: "Navigate | Adjust | Modal | MultiSelect"
		invariants: [
			"Navigate is the initial and focus-movement mode; Adjust is entered while Edit is held and leaves on release or physical focus loss",
			"only Navigate and Adjust are reachable in Phase 2; Modal and MultiSelect remain unavailable until their later product workflows provide complete focus and exit contracts",
			"the mode contains no color, key code, controller button, widget, or mutable view state",
		]
		contributesTo: [{capability: "capability.semantic_graphical_view_model", contribution: "makes cyan navigation and amber adjustment state explicit without letting the renderer infer input state"}]
	}

	valueObjects: MixerControlId: {
		description: "one stable semantic MIXER control identity"
		from: "Track(MixerTrackId, MixerTrackParameter) | Global(GlobalParameter)"
		invariants: [
			"Track identity is one of the fixed sixteen MixerTrackIds and target identity comes from the canonical MixerTrackParameter descriptor; Global reuses the canonical GlobalParameter enum without becoming a seventeenth track",
			"the value contains no PatchId, Patch parameter, parameter index, row, column, label, or widget identity",
		]
		contributesTo: [
			{capability: "capability.semantic_graphical_view_model", contribution: "preserves MIXER row meaning while horizontal navigation crosses fixed tracks"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "prevents Patch identity or collection position from becoming mixer-channel focus"},
		]
	}

	valueObjects: SemanticControlId: {
		description: "one context-neutral stable control identity used by FocusPath"
		from: "Patch(PatchControlId) | Mixer(MixerControlId) | SurfaceRoot"
		invariants: [
			"Patch reuses the existing descriptor-derived PatchControlId and Mixer reuses MixerControlId",
			"SurfaceRoot remains available only for a side surface with no editable child; Patch Utility output controls and Mixer Inspector track sends use their canonical PatchControlId or MixerControlId instead of a fabricated surface value",
			"the union contains no layout coordinate, collection index, display label, raw input, or value copy",
		]
		contributesTo: [{capability: "capability.semantic_graphical_view_model", contribution: "gives every active focus target one stable identity"}]
	}

	valueObjects: FocusPath: {
		description: "the canonical semantic location of exactly one focused target"
		state: {
			context: "TopLevelContext"
			surface: "SurfaceId"
			patchId: "Option<PatchId>"
			mixerTrackId: "Option<MixerTrackId>"
			capabilityId: "Option<CapabilityId | EffectCapabilityId>"
			controlId: "SemanticControlId"
			modalId: "Option<String>"
		}
		invariants: [
			"context and surface are compatible; Patch control paths carry the exact installed PatchId and descriptor capability identity where applicable, Mixer track paths carry one exact MixerTrackId, and Mixer Global paths omit both Patch and track identity",
			"modalId is None throughout Phase 2 and is reserved only as the stable identity seam for a later specified modal workflow",
			"the path resolves to exactly one current target through production descriptors and contains no collection index, layout coordinate, widget id, pointer, value, or presentation label",
		]
		contributesTo: [
			{capability: "capability.semantic_graphical_view_model", contribution: "keeps one focus identity coherent across reducer, projection, layout, live evidence, and schema recovery"},
			{capability: "capability.schema_driven_patch_page", contribution: "reuses PatchControlId and descriptor identities instead of a second page focus"},
		]
	}

	valueObjects: ReturnPath: {
		description: "the exact origin captured before entering a subordinate persistent surface"
		state: {
			origin: "FocusPath"
			enteredSurface: "PatchUtility | MixerInspector"
		}
		invariants: [
			"origin is a valid PatchMain or MixerMain path in the same TopLevelContext as enteredSurface",
			"there is at most one ReturnPath in Phase 2; Return atomically restores origin, clears the path, and enters Navigate mode",
			"a schema change repairs origin through the same deterministic focus resolver before it can be restored",
		]
		contributesTo: [{capability: "capability.semantic_graphical_view_model", contribution: "proves exact return without a view-owned navigation stack"}]
	}

	valueObjects: ValidAction: {
		description: "one currently accepted SemanticAction and its host-neutral presentation metadata"
		state: {
			action: "SemanticAction"
			label: "String"
			hint: "Option<String>"
		}
		invariants: [
			"action is accepted for the exact projected focus, mode, dependency state, value boundary, and structural lifecycle at projection time",
			"labels and hints are presentation only; the action value, not its string, is emitted by a passive component",
			"a projected list is ordered and duplicate-free and excludes unavailable directions, unavailable modes, invalid surfaces, and blocked structural edits",
		]
		contributesTo: [{capability: "capability.semantic_graphical_view_model", contribution: "is the single source for footer hints and passive component intents"}]
	}

	valueObjects: SemanticControlViewModel: {
		description: "one immutable focusable or read-only control projected without layout or widget types"
		state: {
			path: "FocusPath"
			label: "String"
			kind: "continuous | stepped | choice | toggle | asset | identity | surface"
			value: "typed canonical value or read-only presentation"
			unit: "Option<String>"
			enabled: "bool"
			visible: "bool"
			editable: "bool"
			focused: "bool"
			status: "Option<Ready | Preparing | Activating | Failed>"
			error: "Option<typed stable error projection>"
		}
		invariants: [
			"parameter kind, label, value, unit, dependency state, editability, status, and failure come from canonical state plus the owning instrument/effect/mixer descriptor",
			"focused is true for exactly the node whose path equals SemanticGraphicalViewModel.focusPath; read-only and disabled nodes never enter the focus resolver",
			"the value contains no egui type, geometry, callback, mutable state, engine/effect branch, prepared object, device, or audio buffer",
		]
		contributesTo: [{capability: "capability.semantic_graphical_view_model", contribution: "gives multiple layouts the same schema-derived control content"}]
	}

	valueObjects: SemanticSurfaceViewModel: {
		description: "one immutable semantic surface independent of where a host places it"
		state: {
			id: "SurfaceId"
			label: "String"
			role: "Main | PersistentSide"
			controls: "Vec<SemanticControlViewModel>"
			summary: "typed read-only canonical summary"
		}
		invariants: [
			"PATCH exposes PatchMain plus PatchUtility and MIXER exposes MixerMain plus MixerInspector; no third top-level context exists",
			"main controls use canonical descriptor order; PatchUtility projects Patch trim and output track, MixerInspector projects the selected track's sends and routed-Patch summary, and neither side surface owns a second value copy",
			"surface order and identities are stable while host layout order, density, wrapping, and rectangle placement remain adapter concerns",
		]
		contributesTo: [{capability: "capability.semantic_graphical_view_model", contribution: "lets desktop and Steam Deck compositions render the same semantic content"}]
	}

	valueObjects: SemanticGraphicalViewModel: {
		description: "the canonical immutable host-neutral graphical interaction projection"
		state: {
			generation: "u64"
			stateHash: "String"
			context: "TopLevelContext"
			activeSurface: "SurfaceId"
			focusPath: "FocusPath"
			interactionMode: "InteractionMode"
			returnPath: "Option<ReturnPath>"
			validActions: "Vec<ValidAction>"
			status: "{kind: Ready | Preparing | Activating | Failed, label: String, requestId: Option<EngineSelectionRequestId>, graphRevision: GraphRevision}"
			errors: "Vec<{code: stable typed code, label: String, sourcePath: Option<FocusPath>} >"
			surfaces: "Vec<SemanticSurfaceViewModel>"
			serializedLeafDescriptor: "typed stable paths for every semantic view-model field and variant"
		}
		invariants: [
			"all fields derive from one accepted AppState and StateSnapshot generation; context equals focusPath.context, activeSurface equals focusPath.surface, and exactly one visible enabled control path equals focusPath",
			"PATCH surfaces resolve instrument, effect, and PatchOutput content only through installed descriptors and PatchControlId; MIXER always resolves all sixteen tracks through MixerTrackId and MixerTrackParameter plus separate canonical globals and never derives columns from Patches",
			"validActions is computed by the same pure resolver used before reducer execution, status and errors project only typed canonical runtime lifecycle values, and a healthy state contains an explicit empty errors vector",
			"the model contains no eframe/egui type, viewport, rectangle, density rule, widget id, raw input, callback, AppState reference, mutable value, prepared object, device handle, or audio buffer",
			"GraphicalShellProjection embeds this value and derives its path/status/footer presentation from it; the shell and retained TextProjection never own parallel interaction state",
		]
		validations: [{id: "validation.value_object.semantic_graphical_view_model", kind: "test", command: ["cargo", "test", "semantic_graphical_view_model"], description: "both contexts project exact stable paths, surfaces, modes, valid actions, descriptor content, status, errors, and serialized leaves"}]
		contributesTo: [
			{capability: "capability.semantic_graphical_view_model", contribution: "is the single multi-layout graphical interaction contract"},
			{capability: "capability.graphical_application_shell", contribution: "supplies canonical surface, path, status, error, and footer data to the passive shell"},
			{capability: "capability.sixteen_track_mixer_routing", contribution: "projects the fixed track bank and PATCH output controls without UI-owned routing state"},
		]
	}
}

project: validations: semantic_graphical_view_model: {
	id: "validation.semantic_graphical_view_model"
	scope: "project"
	kind: "integration"
	command: ["cargo", "test", "--test", "semantic_graphical_view_model", "--", "--nocapture"]
	assertions: [
		{type: "exit-code", equals: 0},
		{type: "stdout-contains", value: "CREST_ACCEPTANCE semantic_graphical_view_model passed"},
	]
	resources: [
		"valueObject.Control.SemanticAction",
		"valueObject.Control.SurfaceId",
		"valueObject.Control.InteractionMode",
		"valueObject.Control.MixerControlId",
		"valueObject.Control.SemanticControlId",
		"valueObject.Control.FocusPath",
		"valueObject.Control.ReturnPath",
		"valueObject.Control.ValidAction",
		"valueObject.Control.SemanticControlViewModel",
		"valueObject.Control.SemanticSurfaceViewModel",
		"valueObject.Control.SemanticGraphicalViewModel",
		"valueObject.Control.InteractionState",
		"valueObject.Control.AppEvent",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"applicationService.Shell.KeyboardInputTranslator",
		"adapter.EframeGraphicalWindow",
		"asset.SemanticGraphicalViewModelAcceptanceTests",
	]
	capabilities: ["capability.semantic_graphical_view_model", "capability.graphical_application_shell", "capability.one_way_parameter_control", "capability.schema_driven_patch_page"]
	goals: ["goal.use_semantic_graphical_view_model"]
	description: "the production reducer/projector/action/window path proves semantic focus and descriptor-driven multi-layout projection without UI-owned state or audio mutation"
}

project: evidence: semantic_graphical_view_model_contract: {
	kind: "behavioral"
	description: "the canonical focus/action types, reducer, descriptor-driven projector, passive eframe shell, headless recovery proof, cumulative physical live scene, and teardown agree on one semantic graphical model"
	validations: ["validation.semantic_graphical_view_model", "validation.graphical_application_shell", "validation.patch_page_projection", "validation.schema_surface", "validation.egui_context", "validation.demo_scene", "validation.live_demo", "validation.test"]
	witnesses: ["witness.semantic_graphical_view_model"]
}

project: witnesses: semantic_graphical_view_model: {
	scope: "goal"
	goal: "goal.use_semantic_graphical_view_model"
	capability: "capability.semantic_graphical_view_model"
	resources: [
		"valueObject.Control.SemanticAction",
		"valueObject.Control.FocusPath",
		"valueObject.Control.ReturnPath",
		"valueObject.Control.ValidAction",
		"valueObject.Control.SemanticGraphicalViewModel",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"applicationService.Testing.LiveDemoRunner",
		"adapter.EframeGraphicalWindow",
		"applicationService.Shell.StandaloneApplication",
		"asset.CrestSynthMain",
		"asset.BuildMakefile",
	]
	repairResources: [
		"valueObject.Control.SemanticAction",
		"valueObject.Control.FocusPath",
		"valueObject.Control.ReturnPath",
		"valueObject.Control.ValidAction",
		"valueObject.Control.SemanticGraphicalViewModel",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"applicationService.Testing.LiveDemoRunner",
		"adapter.EframeGraphicalWindow",
		"applicationService.Shell.StandaloneApplication",
		"asset.CrestSynthMain",
	]
	evidence: ["evidence.semantic_graphical_view_model_contract"]
	command: ["make", "demo-live-semantic-view-model"]
	timeout: "180s"
	artifacts: ["target/release/crest-synth"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_SEMANTIC_VIEW_MODEL_LIVE_OBSERVATION "
		schema: {
			semantic_focus_correlated: "bool"
			contexts_observed: "number"
			surfaces_observed: "number"
			interaction_modes_observed: "number"
			return_paths_round_tripped: "number"
			valid_actions_exact: "bool"
			structural_statuses_observed: "number"
			healthy_errors_explicitly_empty: "bool"
			focus_recoveries_observed: "number"
			responsive_focus_invariant: "bool"
			physical_audio_nonzero: "bool"
			active_notes_after_cleanup: "number"
			window_closed: "bool"
			stream_released: "bool"
			owned_graphs_remaining: "number"
		}
	}
	predicates: [
		{field: "semantic_focus_correlated", op: "eq", value: true},
		{field: "contexts_observed", op: "eq", value: 2},
		{field: "surfaces_observed", op: "eq", value: 4},
		{field: "interaction_modes_observed", op: "eq", value: 2},
		{field: "return_paths_round_tripped", op: "eq", value: 2},
		{field: "valid_actions_exact", op: "eq", value: true},
		{field: "structural_statuses_observed", op: "gt", value: 2},
		{field: "healthy_errors_explicitly_empty", op: "eq", value: true},
		{field: "focus_recoveries_observed", op: "gt", value: 0},
		{field: "responsive_focus_invariant", op: "eq", value: true},
		{field: "physical_audio_nonzero", op: "eq", value: true},
		{field: "active_notes_after_cleanup", op: "eq", value: 0},
		{field: "window_closed", op: "eq", value: true},
		{field: "stream_released", op: "eq", value: true},
		{field: "owned_graphs_remaining", op: "eq", value: 0},
	]
}

project: assets: SemanticGraphicalViewModelAcceptanceTests: {
	kind: "rust-integration-tests"
	description: "tests/semantic_graphical_view_model.rs, the non-vacuous production-path contract for Phase 2 semantic focus and projection"
	profile: {kind: "verification_harness", witness: "semantic focus, actions, recovery, and multi-layout projection", failurePolicy: "missing action, stale path, adapter repair, schema branch, audio mutation, missing marker, or vacuous test fails"}
	targets: [
		"valueObject.Control.SemanticAction",
		"valueObject.Control.FocusPath",
		"valueObject.Control.ReturnPath",
		"valueObject.Control.SemanticGraphicalViewModel",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"applicationService.Shell.KeyboardInputTranslator",
		"adapter.EframeGraphicalWindow",
	]
	prompts: [
		"Create tests/semantic_graphical_view_model.rs with ordinary assertions and emit CREST_ACCEPTANCE semantic_graphical_view_model passed only after every semantic and projection assertion succeeds.",
		"Drive real normalized WindowInput and passive-view SemanticActions through AppLoop and AppState.apply; prove exact action-to-event mapping, Navigate/Adjust mode transitions, both contexts, all four surfaces, two exact ReturnPath round trips, and no UI-owned interaction state.",
		"Project discriminating SoundFont, Braids, and configured Chorus content from registries plus all sixteen typed mixer tracks and Patch outputs; assert every visible/enabled/focused/status/error/value/action field and serialized leaf exactly without concrete capability field lists or Patch-derived mixer columns.",
		"During a real correlated SoundFont-to-Braids replacement, focus a SoundFont-only structural row while Preparing and prove commit recovers to the next surviving enabled sibling before the previous one; separately exercise typed Failed projection and later recovery without fallback.",
		"Render the same immutable model through production egui frames at 1920x1080 and 1280x800 and prove semantic focus, valid actions, return path, generation, state hash, session values, graph revision, ParameterSnapshot, and audio behavior remain identical despite different rectangles.",
	]
	validations: [{id: "validation.asset.semantic_graphical_view_model_acceptance", kind: "integration", command: ["cargo", "test", "--test", "semantic_graphical_view_model", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE semantic_graphical_view_model passed"}], description: "the named Phase 2 target executes assertion-bearing action, focus, projection, recovery, responsive, and audio-neutral cases"}]
	contributesTo: [{capability: "capability.semantic_graphical_view_model", contribution: "provides the deterministic semantic model and focus-recovery witness"}]
}
