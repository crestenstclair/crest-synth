package crestsynth

// Product-roadmap Phase 1 shell, evolved in Phase 2 to embed the canonical
// semantic graphical view model while retaining its structural, diagnostic,
// reducer, audio, and lifecycle boundaries.

project: goals: use_graphical_shell: {
	description: "The player can run Crest Synth in a controller-first graphical shell, see PATCH or MIXER in the authored structural bands at desktop and Steam Deck sizes, and switch context without moving state or audio behavior into the UI"
	priority: "required"
	actors: ["actor.player", "actor.maintainer"]
	dependsOn: ["goal.observe_live_synth"]
	capabilities: [
		"capability.graphical_application_shell",
		"capability.one_way_parameter_control",
		"capability.schema_driven_patch_page",
		"capability.observable_demo_scene",
		"capability.live_observable_demo",
	]
	requirements: [
		"requirement.selected_egui_stack",
		"requirement.immutable_graphical_shell_projection",
		"requirement.authored_shell_composition",
		"requirement.responsive_shell_blockout",
		"requirement.passive_graphical_window",
		"requirement.retained_diagnostic_projection",
		"requirement.graphical_shell_behavioral_proof",
	]
}

project: capabilities: graphical_application_shell: {
	description: "Project and render the application through a passive eframe/egui shell with canonical semantic interaction, identity, workspace, persistent Utility/Inspector, and footer regions"
	goals: ["goal.use_graphical_shell"]
	acceptance: production_shell: {
		description: "the normal application and phase-specific live scene render one canonical graphical shell without changing control or audio ownership"
		actor: "actor.maintainer"
		steps: [
			{action: "launch the normal standalone application", observes: "the selected eframe/egui adapter renders the context line, identity header, main workspace, persistent Utility or Inspector region, and footer from one immutable graphical projection"},
			{action: "select PATCH and MIXER through normalized input", observes: "AppState.apply remains the only mutation path, every visible region changes to the same accepted context generation, and session values, graph revision, commands, and rendered audio remain exact"},
			{action: "render the shell at 1920 by 1080 and 1280 by 800", observes: "both layouts retain every structural band, the persistent side region, minimum readable geometry, and the same semantic hierarchy without adapter-owned navigation state"},
			{action: "inspect the workspace during the Phase 2 blockout", observes: "the shell renders the immutable semantic view-model path, mode, valid actions, status, errors, surfaces, and descriptor content alongside retained read-only diagnostics without inventing the Phase 4 component library or functional Phase 5/6 screens"},
			{action: "run make demo-live-graphical-shell", observes: "real fixture MIDI and physical audio continue while both contexts and every shell region remain visible, then semantic cleanup, window close, stream release, worker shutdown, graph collection, and parent exit all succeed"},
		]
		evidence: ["evidence.graphical_application_shell_contract"]
	}
}

project: requirements: {
	selected_egui_stack: {
		kind: "nonfunctional"
		description: "The production desktop UI uses eframe and egui, with the matching egui_extras companion for layout and image/SVG loading; Crest owns the shell, state, semantic behavior, and later component APIs, and no alternate GUI runtime or third-party component system is introduced"
		goals: ["goal.use_graphical_shell"]
		capabilities: ["capability.graphical_application_shell"]
	}
	immutable_graphical_shell_projection: {
		kind: "nonfunctional"
		description: "StateProjector derives one immutable GraphicalShellProjection embedding the exact accepted SemanticGraphicalViewModel plus retained TextProjection; AppWindow receives that projection, the latest fixed numeric AudioObservationSnapshot for meters, and a SemanticAction sink and never receives mutable AppState, graph, engine, mixer buffer, audio buffer, or device ownership"
		goals: ["goal.use_graphical_shell"]
		capabilities: ["capability.graphical_application_shell", "capability.one_way_parameter_control"]
	}
	authored_shell_composition: {
		kind: "functional"
		description: "The graphical shell visibly contains the product/context/status line, identity header, main PATCH-or-MIXER workspace, always-present Utility on PATCH or Inspector on MIXER, and current-path/action-hint footer; PATCH and MIXER remain the only top-level contexts and no region becomes a third context"
		goals: ["goal.use_graphical_shell"]
		capabilities: ["capability.graphical_application_shell", "capability.schema_driven_patch_page"]
	}
	responsive_shell_blockout: {
		kind: "nonfunctional"
		description: "The same shell projection renders at the authored 1920x1080 desktop viewport and the 1280x800 Steam Deck viewport with retained header/footer bands, visible Utility/Inspector, bounded proportional workspace split, non-overlapping labels, and no hidden required region or resolution-specific application state"
		goals: ["goal.use_graphical_shell"]
		capabilities: ["capability.graphical_application_shell"]
	}
	passive_graphical_window: {
		kind: "nonfunctional"
		description: "EframeGraphicalWindow normalizes physical input through KeyboardInputTranslator, emits only SemanticAction, paints only its immutable projection, schedules an idle frame after 16 ms, and owns no context, focus, mode, return path, Patch value, lifecycle, parameter, graph, engine, mixer, or audio state"
		goals: ["goal.use_graphical_shell"]
		capabilities: ["capability.graphical_application_shell", "capability.one_way_parameter_control"]
	}
	retained_diagnostic_projection: {
		kind: "functional"
		description: "TextProjection remains a lossless deterministic read-only diagnostic and verification projection from the same accepted generation; SemanticGraphicalViewModel is the interaction contract, and the diagnostic is not a second projector, focus model, valid-action source, or writable UI state copy"
		goals: ["goal.use_graphical_shell"]
		capabilities: ["capability.graphical_application_shell", "capability.observable_demo_scene"]
	}
	graphical_shell_behavioral_proof: {
		kind: "functional"
		description: "A named headless egui acceptance target proves exact region identity, non-overlap, responsive geometry, projection-generation coherence, input dispatch, and audio-neutral context switching at both reference viewports; the retained release-mode make demo-live-graphical-shell target proves the real window, fixture, physical audio, visible context sequence, semantic cleanup, resource teardown, and normal parent exit"
		goals: ["goal.use_graphical_shell"]
		capabilities: ["capability.graphical_application_shell", "capability.observable_demo_scene", "capability.live_observable_demo"]
	}
}

project: contexts: Control: valueObjects: GraphicalShellProjection: {
	description: "the immutable host-neutral structural projection consumed by the production graphical window"
	state: {
		semantic: "SemanticGraphicalViewModel"
		contextLine: "{productLabel: String, contextLabel: String, statusLabel: String}"
		identityHeader: "{primaryLabel: String, secondaryLabel: String}"
		workspace: "{mainRegion: Patch | Mixer, mainLabel: String, sideRegion: Utility | Inspector, sideLabel: String, diagnostic: TextProjection}"
		footer: "{pathLabel: String, modeLabel: String, actionHints: Vec<String>, errorLabels: Vec<String>}"
		serializedLeafDescriptor: "typed stable paths for every shell field and nested retained diagnostic field"
	}
	invariants: [
		"semantic and the nested diagnostic projection come from one accepted AppState generation and state hash and cannot disagree",
		"PATCH includes PatchMain and PatchUtility and MIXER includes MixerMain and MixerInspector without inventing a third context; identity derives from the semantic FocusPath and canonical state",
		"status, error, identity, path, and mode labels are presentation values derived by StateProjector from semantic and contain no adapter, worker, graph, engine, mixer, device, or audio owner",
		"actionHints are an exact presentation of semantic.validActions and never form a second hand-maintained input vocabulary",
		"the value contains no pixel geometry, widget id, egui type, callback, mutable state, prepared object, device handle, or audio buffer",
	]
	validations: [{id: "validation.value_object.graphical_shell_projection", kind: "test", command: ["cargo", "test", "graphical_shell_projection"], description: "both contexts project exact canonical labels, diagnostic content, generation, hash, and stable serialized leaves"}]
	contributesTo: [
		{capability: "capability.graphical_application_shell", contribution: "is the single immutable production-window projection for every structural shell region"},
		{capability: "capability.one_way_parameter_control", contribution: "keeps the graphical adapter downstream of accepted state"},
		{capability: "capability.semantic_graphical_view_model", contribution: "embeds one canonical interaction model and derives all path, mode, action, status, and error presentation from it"},
	]
}

project: contexts: Shell: valueObjects: ShellFrameObservation: {
	description: "one immutable adapter-boundary observation of the graphical shell frame actually painted for live and headless verification"
	state: {
		viewport: "{width: f32, height: f32}"
		generation: "u64"
		stateHash: "String"
		context: "TopLevelContext"
		activeSurface: "SurfaceId"
		focusPath: "FocusPath"
		interactionMode: "InteractionMode"
		regions: "ordered {contextLine, identityHeader, mainWorkspace, persistentSideRegion, footer} rectangles with stable identity and visible-label evidence"
	}
	invariants: [
		"the adapter emits the observation only after painting the supplied GraphicalShellProjection and copies its semantic generation, stateHash, context, surface, focus, and mode identity exactly",
		"rectangles are finite, viewport-bounded adapter output and never become AppState, navigation, focus, Patch, graph, or audio state",
		"expected region names or a pre-render layout plan alone cannot construct a passing observation",
	]
	contributesTo: [
		{capability: "capability.graphical_application_shell", contribution: "makes actual production-frame visibility and geometry observable without moving it into canonical state"},
		{capability: "capability.live_observable_demo", contribution: "lets the control-side live report credit only qualifying rendered PATCH and MIXER frames"},
	]
}

project: validations: graphical_application_shell: {
	id: "validation.graphical_application_shell"
	scope: "project"
	kind: "integration"
	command: ["cargo", "test", "--test", "graphical_application_shell", "--", "--nocapture"]
	assertions: [
		{type: "exit-code", equals: 0},
		{type: "stdout-contains", value: "CREST_ACCEPTANCE graphical_application_shell passed"},
	]
	resources: [
		"valueObject.Control.GraphicalShellProjection",
		"valueObject.Control.SemanticGraphicalViewModel",
		"valueObject.Shell.ShellFrameObservation",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"port.Shell.AppWindow",
		"adapter.EframeGraphicalWindow",
		"applicationService.Shell.StandaloneApplication",
		"asset.GraphicalShellAcceptanceTests",
	]
	capabilities: ["capability.graphical_application_shell", "capability.one_way_parameter_control", "capability.schema_driven_patch_page"]
	goals: ["goal.use_graphical_shell"]
	description: "the production egui update path proves exact graphical shell structure, responsive geometry, canonical context projection, passive input, and retained diagnostic content"
}

project: evidence: graphical_application_shell_contract: {
	kind: "behavioral"
	description: "the canonical shell projection, eframe/egui adapter, headless region witness, production live target, physical audio lifecycle, and complete teardown agree on one passive graphical application shell"
	validations: ["validation.graphical_application_shell", "validation.egui_context", "validation.live_demo", "validation.test"]
	witnesses: ["witness.graphical_application_shell"]
}

project: witnesses: graphical_application_shell: {
	scope: "goal"
	goal: "goal.use_graphical_shell"
	capability: "capability.graphical_application_shell"
	resources: [
		"valueObject.Control.GraphicalShellProjection",
		"valueObject.Shell.ShellFrameObservation",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"adapter.EframeGraphicalWindow",
		"applicationService.Shell.StandaloneApplication",
		"asset.CrestSynthMain",
		"asset.BuildMakefile",
	]
	repairResources: [
		"valueObject.Control.GraphicalShellProjection",
		"valueObject.Shell.ShellFrameObservation",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"adapter.EframeGraphicalWindow",
		"applicationService.Shell.StandaloneApplication",
		"asset.CrestSynthMain",
	]
	evidence: ["evidence.graphical_application_shell_contract"]
	command: ["make", "demo-live-graphical-shell"]
	timeout: "180s"
	artifacts: ["target/release/crest-synth"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_GRAPHICAL_SHELL_LIVE_OBSERVATION "
		schema: {
			context_line_visible: "bool"
			identity_header_visible: "bool"
			main_workspace_visible: "bool"
			persistent_side_region_visible: "bool"
			footer_visible: "bool"
			patch_context_observed: "bool"
			mixer_context_observed: "bool"
			physical_audio_nonzero: "bool"
			active_notes_after_cleanup: "number"
			window_closed: "bool"
			stream_released: "bool"
			owned_graphs_remaining: "number"
		}
	}
	predicates: [
		{field: "context_line_visible", op: "eq", value: true},
		{field: "identity_header_visible", op: "eq", value: true},
		{field: "main_workspace_visible", op: "eq", value: true},
		{field: "persistent_side_region_visible", op: "eq", value: true},
		{field: "footer_visible", op: "eq", value: true},
		{field: "patch_context_observed", op: "eq", value: true},
		{field: "mixer_context_observed", op: "eq", value: true},
		{field: "physical_audio_nonzero", op: "eq", value: true},
		{field: "active_notes_after_cleanup", op: "eq", value: 0},
		{field: "window_closed", op: "eq", value: true},
		{field: "stream_released", op: "eq", value: true},
		{field: "owned_graphs_remaining", op: "eq", value: 0},
	]
}

project: assets: GraphicalShellAcceptanceTests: {
	kind: "rust-integration-tests"
	description: "tests/graphical_application_shell.rs, the non-vacuous headless egui contract for Phase 1 shell structure and responsive composition"
	profile: {kind: "verification_harness", witness: "desktop and Steam Deck graphical shell", failurePolicy: "missing region, overlap, stale projection, UI-owned state, missing marker, or vacuous test fails"}
	targets: [
		"valueObject.Control.GraphicalShellProjection",
		"valueObject.Shell.ShellFrameObservation",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"adapter.EframeGraphicalWindow",
	]
	prompts: [
		"Create tests/graphical_application_shell.rs with ordinary assertions and emit CREST_ACCEPTANCE graphical_application_shell passed only after every structural and projection assertion succeeds.",
		"Drive real egui RawInput through the production update callback at 1920x1080 and 1280x800; inspect tessellated output and recorded region rectangles for exact context line, header, workspace, side region, and footer identity, visibility, ordering, bounds, and non-overlap.",
		"Switch PATCH and MIXER through KeyboardInputTranslator, SemanticAction, and AppLoop, prove the frame, SemanticGraphicalViewModel, GraphicalShellProjection, retained TextProjection, StateTree, EventRecord, ParameterSnapshot generation, and unchanged graph/audio values agree, and reject any adapter-owned context, focus, mode, or return state.",
		"Keep the test headless and deterministic; physical audio and full lifecycle proof remain the separate make demo-live-graphical-shell witness.",
	]
	validations: [{id: "validation.asset.graphical_shell_acceptance", kind: "integration", command: ["cargo", "test", "--test", "graphical_application_shell", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE graphical_application_shell passed"}], description: "the named shell target executes assertion-bearing desktop and Steam Deck frames"}]
	contributesTo: [{capability: "capability.graphical_application_shell", contribution: "provides the focused deterministic shell and responsive-layout witness"}]
}
