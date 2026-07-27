## Why

Phase 3 established capability-polymorphic instruments, PATCH editing, and complete prepared-graph replacement, but every Patch still flows directly from its instrument into the mixer. Phase 4 should now prove the first real Patch-local effect through those same reducer, schema, preparation, hard-real-time, and observable-demo boundaries before any configurable effect graph is attempted.

## What Changes

- Add one fixed post-instrument processing stage: `PreparedEngineRack → PatchAudioBlock → PreparedPostEffectRack → MixEngine`.
- Introduce separate effect capability identities, descriptors, registry/provider/preparer ports, stable Patch-local slot identities, canonical ordered `PostEffectConfig`, and bounded prepared effect ownership while reusing the existing parameter types.
- Vendor the audited MIT-licensed Mutable Instruments Rings Chorus subset at the pinned eurorack/stmlib revisions, expose it only as `Chorus`, prepare independent delay/LFO state per instance, and admit exactly 48 kHz in this first slice.
- Configure one Chorus on the production fixture's first Patch only. Expose read-only effect identity followed by descriptor-derived `Amount` and `Depth` PATCH rows; add no selector, bypass, reorder, extra slot, modal, modulation, or arbitrary routing.
- Publish effect scalars in a separate fixed `ParameterSnapshot` section and process only the matching Patch stem before gain, pan, sends, and global reverb/delay.
- Preserve effect slot/config/layout across SoundFont preset and engine replacements while retaining the existing permission to reset effect tails on complete graph activation.
- Extend focused acceptance, the exhaustive deterministic demo, and the real-window/physical-audio `make demo-live` scene with causal effect-order, audible-output, stereo, isolation, independent-instance, structural-preservation, provenance, callback-safety, teardown, and normal-exit proof.
- **BREAKING (versioned observation schemas):** StateTree, PatchPageProjection, ParameterSnapshot, audio observation, demo coverage, and live report schemas gain effect registry/config/slot/scalar/measurement fields and must advance their schema versions.

## Capabilities

### New Capabilities

- `static-patch-effect`: Configure, project, edit, prepare, process, and observe one fixed Patch-local Chorus with explicit failure and hard-real-time guarantees.

### Modified Capabilities

- `instrument-capability-model`: Separate instrument-only voice/MIDI descriptor semantics from the new effect capability family while sharing canonical parameter types.
- `schema-driven-patch-page`: Append configured effect identity and descriptor-derived scalar controls to the reducer-owned PATCH surface.
- `one-way-parameter-control`: Route effect scalar edits through `AppState::apply` and the existing state/projection/publication order.
- `prepared-engine-rack`: Build and own a Patch-aligned prepared post-effect rack as part of every complete prepared graph.
- `realtime-execution`: Add bounded effect scalar transport, processing, observation, and off-callback destruction requirements.
- `global-mix`: Consume post-effect Patch stems while retaining mixer ownership of only the shared reverb and delay.
- `asynchronous-engine-selection`: Preserve Patch effect configuration/layout through every preset or engine candidate and activation.
- `observable-demo-scene`: Add exact effect schema, edit, order, isolation, independent-instance, and audio coverage to the deterministic witness.
- `live-observable-demo`: Exercise both Chorus scalars through the production physical device and require correlated effect observations, cleanup, and process exit.

## Impact

- Product architecture and documentation: `DESIGN.md`, `ROADMAP.md`, and evaluated CUE declarations under `spec/`.
- Domain/control: Patch state, effect registry/config types, reducer rejection cases, PATCH focus/projection, serialized state, and fixed scalar projection.
- Audio/application: complete graph building, Patch-aligned effect rack, renderer order, observations, structural replacement, and off-thread retirement.
- Infrastructure/build: an opaque C ABI, a minimal vendored Chorus source/provenance bundle, and the existing `cc` build path; no new runtime DSP framework.
- Verification/composition: production startup, smoke path, focused `static_patch_effect` target, schema tests, deterministic demo, live-demo harness, and mandatory physical `make demo-live` acceptance during apply.
