## Why

Phase 3 now exposes the installed SoundFont and Braids choices on a read-only PATCH page, but the player cannot select one. The next roadmap increment makes that selector real through the existing capability, reducer, prepared-graph, and acknowledgement boundaries. The headless demo must prove the exhaustive deterministic and negative-path contract, and the paced live demo must visibly and audibly exercise both successful directions through the production threaded worker.

## What Changes

- Reconcile the evaluated CUE architecture from its current read-only/no-engine-selector slice to one bounded, correlated engine-selection lifecycle; remove only the superseded increment guards and retain the two-engine, two-context, no-fallback architecture.
- Make the focused PATCH engine row the only newly editable PATCH control. Existing normalized Edit+Left/Right input requests the previous or next installed registry choice through a semantic event and `AppState::apply`; engine fields and choices remain descriptor-driven rather than matched on SoundFont or Braids.
- Add canonical control-side selection status and typed request identity so PATCH can project the active engine, requested engine, and `Preparing`, `Activating`, `Ready`, or failed state without storing a worker, prepared engine, graph, or device handle in application state.
- Build a complete candidate graph off the callback from a descriptor-default, provider-validated target config. The currently accepted config and active graph remain intact and audible during preparation; preparation failure records a visible typed result and changes neither.
- On successful preparation, commit the exact candidate config through a correlated semantic completion event, publish its graph-compatible parameter projection and complete prepared graph through the dedicated structural boundary, activate only at a block boundary, acknowledge activation and retirement, and destroy the old graph only on control/worker ownership. One structural request may be in flight; busy, stale, mismatched, unavailable, and failed requests are explicit and never fall back.
- Extend both autonomous demo scenes through the production input/reducer/orchestration/render path. The deterministic headless scene proves both audible directions plus busy, failure, stale/mismatch, callback-safety, final descriptor-default configuration, and byte-identical two-run evidence. The paced physical-device scene uses the production threaded worker, visibly checkpoints each lifecycle/revision, proves finite targeted audio in both directions, returns to descriptor-default SoundFont, and retains its existing scalar coverage, input-isolation, cleanup, and teardown gates.
- **BREAKING**: extend the closed semantic event/effect/rejection vocabulary and versioned state, PATCH projection, event-log, and demo-observation schemas with engine-selection lifecycle data.

## Capabilities

### New Capabilities

- `asynchronous-engine-selection`: Own the correlated request, off-callback preparation, reducer commit, complete-graph activation/retirement, and headless/live proof that join the existing PATCH-page, capability, control, rack, and demo capabilities.

### Modified Capabilities

- `schema-driven-patch-page`: Replace the read-only engine-choice guard with one focused descriptor-driven engine selector and canonical pending, activating, ready, and failed projection states; keep ADSR and capability parameter rows read-only.
- `instrument-capability-model`: Permit a selected installed capability to produce one descriptor-default, provider-validated candidate config for the same Patch without engine-specific state or fallback, while retaining Structural parameter editing as a later increment.
- `prepared-engine-rack`: Admit a one-at-a-time user-triggered replacement whose Patch identities and callback capacities remain bounded while one Patch's capability/scalar layout and graph revision change through complete preparation, publication, activation, acknowledgement, and off-callback retirement.
- `one-way-parameter-control`: Extend the reducer/effect path with correlated engine-selection request, preparation outcome, and activation acknowledgement events while preserving commit-before-projection/publication ordering.
- `observable-demo-scene`: Make the deterministic production-path scene exercise and falsifiably observe successful engine replacement in both directions, pending/busy/stale handling, typed preparation failure, graph acknowledgement, audible target-engine output, isolation, and one exact declared final configuration.
- `live-observable-demo`: Extend the paced real-window/physical-audio scene with SoundFont → Braids → descriptor-default SoundFont through semantic events and the production threaded worker, with canonical lifecycle/revision checkpoints and target-audio evidence kept distinct from scalar coverage.

## Impact

The change affects the canonical CUE declarations for Control, Synth, RealTime, Shell, Testing, goals/capabilities, assets, and verification. Expected implementation areas include `AppEvent`, `AppState`, interaction/runtime selection status, `PatchPageProjection`, `StateProjector`, `AppLoop`, keyboard translation, an injected control/worker preparation boundary, `PreparedGraphBuilder`, `StructuralGraphCoordinator`, `StandaloneApplication`, the text adapter, both demo scene/report/coverage schemas, and named integration tests.

The active Patch count, stable `PatchId`, MIDI channel, mixer route, common ADSR, and every untargeted Patch remain unchanged across a selection. No new engine, dependency, async runtime, preset browser, SoundFont preset editing, ADSR editing, per-Patch effect, modulation, persistence, third context, or graphical redesign is introduced.
