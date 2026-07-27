package crestsynth

project: goals: edit_patch_envelope: {
	description: "The player can focus and edit the existing common per-voice Attack, Decay, Sustain, and Release values on PATCH while the same canonical Patch state, scalar publication, and engine-native DSP remain authoritative"
	priority: "required"
	actors: ["actor.player", "actor.maintainer"]
	dependsOn: ["goal.select_patch_engine"]
	capabilities: [
		"capability.schema_driven_patch_page",
		"capability.one_way_parameter_control",
		"capability.per_voice_envelope",
		"capability.asynchronous_engine_selection",
		"capability.realtime_execution",
		"capability.observable_demo_scene",
		"capability.live_observable_demo",
	]
	requirements: [
		"requirement.patch_adsr_focus_surface",
		"requirement.canonical_patch_adsr_adjustment",
		"requirement.scalar_only_patch_adsr_publication",
		"requirement.patch_adsr_structural_coexistence",
		"requirement.patch_adsr_behavioral_proof",
	]
}

project: requirements: {
	patch_adsr_focus_surface: {
		kind: "functional"
		description: "PATCH exposes one reducer-owned nonwrapping focused-Patch order whose base is Engine, Attack, Decay, Sustain, Release from PatchControlId plus the canonical VoiceEnvelope descriptor and whose suffix is the active descriptor's StructuralChoice controls; bare Up/Down moves one row, bare Left/Right remains unavailable, exactly one projected row is focused, and endpoint navigation is a typed unchanged rejection"
		goals: ["goal.edit_patch_envelope"]
		capabilities: ["capability.schema_driven_patch_page", "capability.one_way_parameter_control"]
	}
	canonical_patch_adsr_adjustment: {
		kind: "functional"
		description: "On a focused ADSR row, Edit+Left/Right applies the existing descriptor's fine decrement/increment and Edit+Down/Up applies its coarse decrement/increment through AppState.apply; the reducer reuses VoiceEnvelopeParameter, VoiceEnvelope.withValue, and the same adjustment/boundary rules as MIXER and changes exactly the focused Patch field"
		goals: ["goal.edit_patch_envelope"]
		capabilities: ["capability.schema_driven_patch_page", "capability.one_way_parameter_control", "capability.per_voice_envelope"]
	}
	scalar_only_patch_adsr_publication: {
		kind: "nonfunctional"
		description: "An accepted PATCH ADSR edit commits one canonical VoiceEnvelope value, advances one generation, reprojects StateSnapshot, PatchPageProjection, TextProjection, StateTree, and a same-revision fixed ParameterSnapshot, and emits no AudioCommand, preparation request, PreparedGraph, graph publication, or alternate envelope/DSP state"
		goals: ["goal.edit_patch_envelope"]
		capabilities: ["capability.one_way_parameter_control", "capability.per_voice_envelope", "capability.realtime_execution"]
	}
	patch_adsr_structural_coexistence: {
		kind: "nonfunctional"
		description: "PATCH ADSR focus and edits remain valid during Ready, Preparing, Activating, and recoverable Failed structural status for either engine or preset intent: Preparing publishes against the source revision and the candidate refreshes from the latest committed snapshot before publication; Activating publishes against the target revision for exact first-target consumption while the source remains audible under its last compatible snapshot; lifecycle events preserve focus and envelope"
		goals: ["goal.edit_patch_envelope"]
		capabilities: ["capability.per_voice_envelope", "capability.asynchronous_engine_selection", "capability.realtime_execution"]
	}
	patch_adsr_behavioral_proof: {
		kind: "functional"
		description: "The named Patch-page and per-voice-envelope targets plus both demos drive all four focused ADSR controls through production input, reducer, projections, fixed snapshots, and both real renderers; they prove exact focus/value coherence, fine/coarse and boundary behavior, target-only audible per-voice consequences, scalar/structural coexistence, zero structural effects, callback safety, and exact restoration or declared final state"
		goals: ["goal.edit_patch_envelope"]
		capabilities: ["capability.schema_driven_patch_page", "capability.one_way_parameter_control", "capability.per_voice_envelope", "capability.observable_demo_scene", "capability.live_observable_demo", "capability.asynchronous_engine_selection", "capability.realtime_execution"]
	}
}

project: evidence: patch_adsr_editing_contract: {
	kind: "behavioral"
	description: "production keyboard translation, AppState, PATCH projections, fixed scalar transport, SoundFont and Braids renderers, and both observable scenes agree on the four canonical focused-Patch ADSR edits without a second state or structural path"
	validations: ["validation.patch_page_projection", "validation.per_voice_envelope", "validation.demo_scene", "validation.live_demo", "validation.test"]
	witnesses: ["witness.per_voice_envelope", "witness.exhaustive_demo_scene"]
}
