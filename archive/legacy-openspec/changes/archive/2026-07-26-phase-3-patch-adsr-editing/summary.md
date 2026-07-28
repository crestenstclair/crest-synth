# Change Summary

## Outcome

- **Problem:** PATCH displays the focused Patch's common ADSR but still forces players into MIXER to edit it.
- **Result:** PATCH gains one reducer-owned Engine → Attack → Decay → Sustain → Release focus order and edits the existing canonical per-voice envelope.

## Change Outline

- **Focus:** Bare Up/Down navigates without wrapping; horizontal navigation and endpoint movement are typed unchanged rejections.
- **Editing:** Engine keeps adjacent Edit+Left/Right selection; ADSR uses canonical fine Left/Right and coarse Down/Up adjustment semantics.
- **Publication:** Focus is audio-neutral and ADSR is scalar-only through the existing fixed snapshot; no new command, graph, transport, state, or DSP path is added.
- **Lifecycle:** ADSR remains editable through Ready, Preparing, Activating, and recoverable Failed with explicit source/target revision behavior.
- **Live robustness:** Physical startup accepts a valid preferred default before optional device-range enumeration; semantic owning-Patch probe notes make exact-generation scalar checkpoints independent of sparse fixture timing; typed milestone/whole-run deadlines replace genuine indefinite pending states while preserving cleanup and owned teardown.
- **Scope:** Capability rows and SoundFont presets/assets remain read-only; preset discovery is the next separate roadmap increment.

## System Impact

- **Capabilities:** Modifies `schema-driven-patch-page`, `one-way-parameter-control`, `per-voice-envelope`, `asynchronous-engine-selection`, `observable-demo-scene`, and `live-observable-demo`; adds no capability.
- **Architecture:** Centers `valueObject.Control.PatchControlId`, `valueObject.Control.InteractionState`, `aggregate.Control.AppState`, `valueObject.Control.PatchPageProjection`, `valueObject.Synth.VoiceEnvelope`, `domainService.Control.StateProjector`, and `applicationService.Control.AppLoop` under `goal.edit_patch_envelope`.
- **Interfaces/data:** Adds focused-control projection leaves and requires coordinated StateTree/observation schema-version and exact leaf-descriptor updates; no persistence or dependency migration.

## Delivery

- **Implementation:** Extend canonical focus/reducer behavior, update projection/text/schema, prove lifecycle-safe scalar publication, then update focused and demo acceptance paths.
- **Coverage:** Route the focused Patch's four existing envelope identifiers through PATCH exactly once; retain the canonical resolver and MIXER route for all remaining instances.
- **Validation:** Strengthen Patch-page, per-voice-envelope, schema, egui, engine-selection, exhaustive, live, real-time, mutation, performance, full Cargo, physical demo, CUE, and strict OpenSpec gates.

## Risks and Decisions

- **Key decisions:** Reuse `VoiceEnvelopeParameter` and one shared reducer mutation; separate focus from Engine editability; keep navigation nonwrapping and capability rows out of focus.
- **Risks/open questions:** Observation compatibility and Activating revision timing are explicitly mitigated in `design.md`; no open design question remains.
