package crestsynth

// Corrective gate after roadmap Phase 2: replace the Patch-shaped transitional
// MIXER model with the authored fixed sixteen-track domain before effects and
// bus topology expands.

project: goals: route_patches_through_sixteen_tracks: {
	description: "The player can route each Patch to one of sixteen persistent mixer tracks and control the combined routed signal through track-owned level, pan, mute, solo, sends, and meters"
	priority: "required"
	actors: ["actor.player", "actor.maintainer"]
	dependsOn: ["goal.use_semantic_graphical_view_model"]
	capabilities: ["capability.sixteen_track_mixer_routing"]
	requirements: [
		"requirement.canonical_sixteen_track_bank",
		"requirement.patch_output_route_and_trim",
		"requirement.fixed_realtime_track_routing",
		"requirement.track_owned_semantic_mixer",
		"requirement.sixteen_track_mixer_behavioral_proof",
	]
}

project: capabilities: sixteen_track_mixer_routing: {
	description: "Own, edit, project, render, meter, and prove one fixed sixteen-track mixer bank independently from Patch identity and instrument schema"
	goals: ["goal.route_patches_through_sixteen_tracks"]
	acceptance: routed_track_control: {
		description: "sixteen stable tracks receive Patch output routes and control their combined signal through the canonical reducer and bounded renderer"
		actor: "actor.maintainer"
		steps: [
			{action: "open MIXER with zero, one, and several installed Patches", observes: "T00 through T0F remain present in stable order with track-owned Level, Pan, Mute, Solo, current sends, and one meter each; Patch count and engine schemas do not create or remove columns"},
			{action: "route two sounding Patches to one track", observes: "their post-effect, post-trim stems sum before the track controls and the track fader changes both contributions together while every other track remains sample-exact"},
			{action: "change one Patch trim and output track through PATCH Utility", observes: "AppState.apply commits exactly that Patch output value, the next compatible fixed snapshot moves only its contribution, and no structural graph is built or substituted"},
			{action: "exercise track level, pan, mute, solo, reverb send, and delay send", observes: "the canonical state, semantic projection, fixed snapshot, pre-gate meter, dry output, and post-gate sends agree; mute wins and active solo excludes non-soloed tracks"},
			{action: "attempt an invalid track identity and navigate empty tracks at both reference viewports", observes: "the invalid value is a typed unchanged rejection while all sixteen stable track paths remain visible, focusable, and free of Patch or widget indices"},
			{action: "run make demo-live-sixteen-track-mixer-routing", observes: "the real window and physical stream expose all sixteen tracks, shared-track accumulation, rerouting, track controls, meters, finite nonzero audio, exact cleanup, stream release, graph collection, and normal exit"},
		]
		evidence: ["evidence.sixteen_track_mixer_routing_contract"]
	}
}

project: requirements: {
	canonical_sixteen_track_bank: {
		kind: "functional"
		description: "AppState SHALL own one MixerState containing exactly sixteen persistent tracks identified by MixerTrackId 0 through 15; every track SHALL own Level, Pan, Mute, Solo, ReverbSend, and DelaySend while Patch installation, count, order, route, and capability schema SHALL NOT create, remove, reorder, or reset tracks"
		goals: ["goal.route_patches_through_sixteen_tracks"]
		capabilities: ["capability.sixteen_track_mixer_routing"]
	}
	patch_output_route_and_trim: {
		kind: "functional"
		description: "Every Patch SHALL own exactly one validated PatchOutput containing MixerTrackId and trimGainDb and SHALL own no track level, pan, mute, solo, send, or meter; multiple Patches MAY share one track, and PATCH Utility SHALL expose trim plus output track through SemanticAction, AppEvent, and AppState.apply without UI-owned state"
		goals: ["goal.route_patches_through_sixteen_tracks"]
		capabilities: ["capability.sixteen_track_mixer_routing"]
	}
	fixed_realtime_track_routing: {
		kind: "nonfunctional"
		description: "ParameterSnapshot SHALL carry each active PatchId with fixed PatchOutput plus exactly sixteen fixed track parameter entries, and AudioObservationSnapshot SHALL carry exactly sixteen numeric pre-gate meters; MixEngine SHALL use preallocated track scratch, apply Patch trim before accumulation and track controls after accumulation, and perform no callback allocation, deallocation, locking, blocking, I/O, logging, formatting, panic, unwind, or destruction"
		goals: ["goal.route_patches_through_sixteen_tracks"]
		capabilities: ["capability.sixteen_track_mixer_routing"]
	}
	track_owned_semantic_mixer: {
		kind: "functional"
		description: "MIXER SHALL always project all sixteen tracks independently from Patches, use MixerTrackId plus MixerTrackParameter for stable focus, preserve control row while horizontal navigation changes tracks, expose track sends and routed-Patch summary in MixerInspector, and keep global controls distinct from the sixteen tracks; PatchId plus any Patch-owned editable target SHALL NOT define a mixer column"
		goals: ["goal.route_patches_through_sixteen_tracks"]
		capabilities: ["capability.sixteen_track_mixer_routing"]
	}
	sixteen_track_mixer_behavioral_proof: {
		kind: "functional"
		description: "A named assertion-bearing production-path target and retained physical live target SHALL prove exact sixteen-track state and projection, shared-track summing, Patch reroute and trim isolation, every track parameter class, mute-wins and any-solo semantics, post-gate sends, pre-gate meters including muted input, empty tracks, invalid-route rejection, fixed snapshot equality, callback safety, responsive stable focus, finite physical audio, and complete teardown"
		goals: ["goal.route_patches_through_sixteen_tracks"]
		capabilities: ["capability.sixteen_track_mixer_routing"]
	}
}

project: contexts: Testing: valueObjects: SixteenTrackMixerRoutingObservation: {
	description: "the focused machine-readable result of the sixteen-track routing acceptance"
	state: {
		schemaVersion: "u32"
		trackCount: "u32"
		trackIdsExact: "bool"
		allTracksProjected: "bool"
		emptyTracksAddressable: "bool"
		sharedTrackPatchCount: "u32"
		sharedTrackSumExact: "bool"
		trackLevelControlsSharedSum: "bool"
		patchTrimIsolated: "bool"
		patchRerouteIsolated: "bool"
		invalidRouteRejected: "bool"
		trackParameterClassesExercised: "u32"
		muteWins: "bool"
		anySoloExact: "bool"
		postGateSendsExact: "bool"
		preGateMetersExact: "bool"
		fixedSnapshotExact: "bool"
		stableFocusExact: "bool"
		callbackAllocations: "u64"
		callbackDestructions: "u64"
		physicalAudioNonzero: "bool"
		activeNotesAfterCleanup: "u32"
		windowClosed: "bool"
		streamReleased: "bool"
		ownedGraphsRemaining: "u32"
	}
	invariants: [
		"every field is measured from the production reducer, projector, snapshot transport, prepared renderer, mixer buffers, audio observation, eframe window, and physical stream rather than copied from expected data",
		"the live marker is emitted only after semantic cleanup, stream release, worker shutdown, graph collection, window close, and successful parent-process return",
	]
	contributesTo: [{capability: "capability.sixteen_track_mixer_routing", contribution: "is the structured headless and physical acceptance result"}]
}

project: validations: sixteen_track_mixer_routing: {
	id: "validation.sixteen_track_mixer_routing"
	scope: "project"
	kind: "integration"
	command: ["cargo", "test", "--test", "sixteen_track_mixer_routing", "--", "--nocapture"]
	assertions: [
		{type: "exit-code", equals: 0},
		{type: "stdout-contains", value: "CREST_ACCEPTANCE sixteen_track_mixer_routing passed"},
	]
	resources: [
		"valueObject.Mixer.MixerTrackId",
		"valueObject.Mixer.PatchOutputParameter",
		"valueObject.Mixer.PatchOutput",
		"valueObject.Mixer.MixerTrackParameter",
		"valueObject.Mixer.MixerTrackParameters",
		"valueObject.Mixer.MixerState",
		"valueObject.Mixer.TrackMeter",
		"aggregate.Synth.Patch",
		"valueObject.Control.MixerControlId",
		"valueObject.Control.PatchControlId",
		"valueObject.Control.FocusPath",
		"valueObject.Control.SemanticGraphicalViewModel",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"valueObject.RealTime.ParameterSnapshot",
		"valueObject.RealTime.AudioObservationSnapshot",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"applicationService.Testing.ExhaustiveGuiDemo",
		"applicationService.Testing.LiveDemoRunner",
		"valueObject.Testing.SixteenTrackMixerRoutingObservation",
		"asset.SixteenTrackMixerRoutingAcceptanceTests",
	]
	capabilities: ["capability.sixteen_track_mixer_routing", "capability.global_mix", "capability.one_way_parameter_control", "capability.semantic_graphical_view_model", "capability.realtime_execution"]
	goals: ["goal.route_patches_through_sixteen_tracks"]
	description: "the named production path proves exact track ownership, Patch routing, semantic control, fixed real-time transport, mix behavior, metering, and callback safety"
}

project: evidence: sixteen_track_mixer_routing_contract: {
	kind: "behavioral"
	description: "the fixed MixerState, Patch output route, reducer, semantic projection, snapshots, preallocated MixEngine, headless target, and physical live scene agree on one fallback-free sixteen-track mixer"
	validations: ["validation.sixteen_track_mixer_routing", "validation.semantic_graphical_view_model", "validation.demo_scene", "validation.live_demo", "validation.test"]
	witnesses: ["witness.sixteen_track_mixer_routing"]
}

project: witnesses: sixteen_track_mixer_routing: {
	scope: "goal"
	goal: "goal.route_patches_through_sixteen_tracks"
	capability: "capability.sixteen_track_mixer_routing"
	resources: [
		"valueObject.Mixer.MixerTrackId",
		"valueObject.Mixer.PatchOutput",
		"valueObject.Mixer.MixerTrackParameters",
		"valueObject.Mixer.MixerState",
		"valueObject.Control.MixerControlId",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"valueObject.RealTime.ParameterSnapshot",
		"valueObject.RealTime.AudioObservationSnapshot",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"applicationService.Testing.LiveDemoRunner",
		"adapter.EframeGraphicalWindow",
		"applicationService.Shell.StandaloneApplication",
		"valueObject.Testing.SixteenTrackMixerRoutingObservation",
		"asset.CrestSynthMain",
		"asset.BuildMakefile",
	]
	repairResources: [
		"valueObject.Mixer.PatchOutput",
		"valueObject.Mixer.MixerState",
		"valueObject.Control.MixerControlId",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"valueObject.RealTime.ParameterSnapshot",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"applicationService.Testing.LiveDemoRunner",
		"adapter.EframeGraphicalWindow",
		"applicationService.Shell.StandaloneApplication",
	]
	evidence: ["evidence.sixteen_track_mixer_routing_contract"]
	command: ["make", "demo-live-sixteen-track-mixer-routing"]
	timeout: "180s"
	artifacts: ["target/release/crest-synth"]
	observation: {
		kind: "json_stdout"
		marker: "CREST_SIXTEEN_TRACK_MIXER_ROUTING_LIVE_OBSERVATION "
		schema: {
			track_count: "number"
			track_ids_exact: "bool"
			all_tracks_projected: "bool"
			empty_tracks_addressable: "bool"
			shared_track_patch_count: "number"
			shared_track_sum_exact: "bool"
			track_level_controls_shared_sum: "bool"
			patch_trim_isolated: "bool"
			patch_reroute_isolated: "bool"
			invalid_route_rejected: "bool"
			track_parameter_classes_exercised: "number"
			mute_wins: "bool"
			any_solo_exact: "bool"
			post_gate_sends_exact: "bool"
			pre_gate_meters_exact: "bool"
			fixed_snapshot_exact: "bool"
			stable_focus_exact: "bool"
			callback_allocations: "number"
			callback_destructions: "number"
			physical_audio_nonzero: "bool"
			active_notes_after_cleanup: "number"
			window_closed: "bool"
			stream_released: "bool"
			owned_graphs_remaining: "number"
		}
	}
	predicates: [
		{field: "track_count", op: "eq", value: 16},
		{field: "track_ids_exact", op: "eq", value: true},
		{field: "all_tracks_projected", op: "eq", value: true},
		{field: "empty_tracks_addressable", op: "eq", value: true},
		{field: "shared_track_patch_count", op: "gt", value: 1},
		{field: "shared_track_sum_exact", op: "eq", value: true},
		{field: "track_level_controls_shared_sum", op: "eq", value: true},
		{field: "patch_trim_isolated", op: "eq", value: true},
		{field: "patch_reroute_isolated", op: "eq", value: true},
		{field: "invalid_route_rejected", op: "eq", value: true},
		{field: "track_parameter_classes_exercised", op: "eq", value: 6},
		{field: "mute_wins", op: "eq", value: true},
		{field: "any_solo_exact", op: "eq", value: true},
		{field: "post_gate_sends_exact", op: "eq", value: true},
		{field: "pre_gate_meters_exact", op: "eq", value: true},
		{field: "fixed_snapshot_exact", op: "eq", value: true},
		{field: "stable_focus_exact", op: "eq", value: true},
		{field: "callback_allocations", op: "eq", value: 0},
		{field: "callback_destructions", op: "eq", value: 0},
		{field: "physical_audio_nonzero", op: "eq", value: true},
		{field: "active_notes_after_cleanup", op: "eq", value: 0},
		{field: "window_closed", op: "eq", value: true},
		{field: "stream_released", op: "eq", value: true},
		{field: "owned_graphs_remaining", op: "eq", value: 0},
	]
}

project: assets: SixteenTrackMixerRoutingAcceptanceTests: {
	kind: "rust-integration-tests"
	description: "tests/sixteen_track_mixer_routing.rs, the non-vacuous production-path contract for canonical Patch-to-track routing"
	profile: {kind: "verification_harness", witness: "sixteen persistent tracks, shared-track routing, controls, meters, and real-time safety", failurePolicy: "missing target, marker, track, route, isolation, meter, or callback assertion fails"}
	targets: [
		"valueObject.Mixer.MixerTrackId",
		"valueObject.Mixer.PatchOutput",
		"valueObject.Mixer.MixerTrackParameters",
		"valueObject.Mixer.MixerState",
		"aggregate.Synth.Patch",
		"aggregate.Control.AppState",
		"domainService.Control.StateProjector",
		"applicationService.Control.AppLoop",
		"valueObject.RealTime.ParameterSnapshot",
		"applicationService.RealTime.AudioRenderer",
		"domainService.Mixer.MixEngine",
		"valueObject.Testing.SixteenTrackMixerRoutingObservation",
	]
	prompts: [
		"Create tests/sixteen_track_mixer_routing.rs with ordinary assertions and emit CREST_ACCEPTANCE sixteen_track_mixer_routing passed only after every structured predicate succeeds.",
		"Use the production Patch/AppState, semantic resolver/projector/AppLoop, fixed snapshots, prepared renderer, MixEngine, and audio observation seams; deterministic fixtures may assemble inputs but must not duplicate reducer, routing, mix, meter, or verdict logic.",
		"Prove exact T00 through T0F persistence with empty and populated tracks, two Patch stems summed into one track before its fader, Patch trim and reroute isolation, all six track parameter classes, mute-wins and any-solo behavior, post-gate sends, pre-gate muted meters, invalid-route rejection, stable responsive focus, fixed snapshot equality, finite output, and zero callback allocation or destruction.",
	]
	validations: [{id: "validation.asset.sixteen_track_mixer_routing_acceptance", kind: "integration", command: ["cargo", "test", "--test", "sixteen_track_mixer_routing", "--", "--nocapture"], assertions: [{type: "exit-code", equals: 0}, {type: "stdout-contains", value: "CREST_ACCEPTANCE sixteen_track_mixer_routing passed"}], description: "the named target executes production-path assertions before its marker"}]
	contributesTo: [{capability: "capability.sixteen_track_mixer_routing", contribution: "provides the focused deterministic routing and mix witness"}]
}
