## Context

The completed first Phase 3 implementation gives Control one reducer-owned PATCH context, stable focused `PatchId`, and a descriptor-driven `PatchPageProjection`. Its engine row shows the immutable registry choices but every PATCH action is rejected. The runtime already has capability-matched preparers, complete `PreparedGraph` construction, a dedicated structural ownership boundary, block-boundary activation, fixed acknowledgement, and off-callback retirement. The updated evaluated CUE now defines how those pieces connect to one user-triggered structural edit; the Rust baseline still implements the preceding read-only slice until this change is applied.

This increment connects those existing seams without changing the master design: physical input still becomes one semantic event, `AppState::apply` remains the only state mutation path, the callback receives only fully prepared ownership, and failure never selects another capability. The player is the stakeholder for visible and audible selection; maintainers need exhaustive deterministic evidence through `make demo` and a paced production-thread/device witness through `make demo-live`, not construction-only tests.

Canonical resources affected include `valueObject.Control.AppEvent`, `valueObject.Control.InteractionState`, `aggregate.Control.AppState`, `valueObject.Control.PatchPageProjection`, `valueObject.Control.StateTree`, `domainService.Control.StateProjector`, `applicationService.Control.AppLoop`, `valueObject.Synth.InstrumentConfig`, `port.Synth.InstrumentCapabilityProvider`, `aggregate.RealTime.PreparedGraph`, `applicationService.RealTime.PreparedGraphBuilder`, `applicationService.RealTime.StructuralGraphCoordinator`, `applicationService.Shell.StandaloneApplication`, `valueObject.Testing.DemoScene`, `valueObject.Testing.DemoSceneReport`, `applicationService.Testing.ExhaustiveGuiDemo`, `valueObject.Testing.LiveDemoScene`, `valueObject.Testing.LiveDemoCheckpoint`, `valueObject.Testing.LiveDemoReport`, and `applicationService.Testing.LiveDemoRunner`. The evaluated CUE adds one canonical engine-selection request identity/status/failure model and one replaceable graph-preparation worker port; application state and both runners own neither the worker nor a `PreparedGraph`.

## Goals / Non-Goals

**Goals:**

- Make Edit+Left/Right (`K+A/D` in the basic adapter) select the adjacent installed engine for the stable focused Patch through the existing `Adjust` event and reducer.
- Keep the accepted Patch config and active graph intact while preparation is pending or fails.
- Correlate every request, worker outcome, candidate config, graph revision, publication, activation, retirement, and visible status without an engine-specific branch.
- Commit a successfully prepared candidate through `AppState::apply` before publishing its graph-compatible parameter snapshot and complete graph.
- Keep MIDI and ordinary scalar control responsive while one structural request is pending, while rejecting a second structural request explicitly.
- Prove real SoundFont-to-Braids and Braids-to-SoundFont transitions, failure recovery, no fallback, acknowledgement, and callback safety in the deterministic exhaustive demo and a focused named acceptance target.
- Exercise both successful directions again in the paced live scene through the production threaded worker and physical render path, with lifecycle/revision checkpoints and a declared descriptor-default SoundFont final state.

**Non-Goals:**

- Editing PATCH ADSR or capability parameters, SoundFont preset discovery/selection, inactive-engine setting caches, undo history, or preserving a prior engine's non-default config when returning to it.
- Seamless migration of active notes, voices, or effect tails between complete graphs. The old graph remains audible until the block-boundary replacement; the replacement begins from its prepared state and later MIDI continues normally.
- More than one structural request in flight, cancellation, request queues, arbitrary graph changes, per-Patch effects, modulation, persistence, additional engines, plugin hosting, a third top-level context, or the Figma-derived graphical UI.
- A new async runtime or weakening any callback, provider/preparer, graph ownership, or no-fallback contract.

## Decisions

### 1. Reconcile CUE before implementation and link the workflow capability

Planning has updated the current Phase 3 CUE slice before Rust implementation. It replaces the explicit read-only/no-selector invariants with the lifecycle below, adds the worker boundary and evidence, and links `asynchronous_engine_selection` to the stakeholder goal, named witness, and every owning resource. `schema_driven_patch_page`, `instrument_capability_model`, `prepared_engine_rack`, `one_way_parameter_control`, and `observable_demo_scene` retain their existing responsibilities and are modified where the workflow crosses them.

The existing context direction remains:

```text
Shell input
    -> Control AppEvent / AppState
    -> Control AppLoop orchestration
    -> Synth provider + preparation ports
    -> RealTime prepared graph + structural boundary
    -> Testing observation of the same seams
```

Alternative considered: leave CUE read-only and describe the selector only in OpenSpec. Rejected because the evaluated architecture explicitly forbids the requested behavior and is authoritative for planning and implementation.

### 2. Use one canonical, application-wide selection state machine

Control adds a monotonic `EngineSelectionRequestId`, a typed preparation-failure code, and one status value in runtime state. Only status data and stable identities live in `AppState`; worker handles, prepared engines, graphs, channels, and device data remain outside it.

```text
Ready(active A, revision R)
  | Adjust Left/Right
  v
Preparing(id, patch, from A, requested B)
  | worker failure ----------------------> Failed(id, A -> B, typed error)
  | correlated prepared result
  v
Activating(id, patch, A -> B, revision R+n)
  | active + retired + collected acknowledgement
  v
Ready(active B, revision R+n)
```

The request transition advances canonical generation and emits one typed preparation request but does not change the Patch's `InstrumentConfig` or graph revision. A failure outcome is an accepted semantic system event that records a visible typed failure while retaining the old config/revision. A prepared outcome carries the exact provider-created candidate config and target revision; `AppState::apply` revalidates its request id, Patch, requested capability, and config against the immutable registry, then commits that config and enters `Activating`. A correlated acknowledgement event enters `Ready`. Stale or mismatched outcomes and acknowledgements are unchanged typed rejections.

The state machine is application-wide rather than per Patch because the structural transport permits only one unacknowledged complete graph. MIDI, context changes, and Scalar edits remain legal while preparing or activating. Another engine adjustment is rejected as structural work busy. A new valid request replaces a displayed failed status and receives a new request id.

Alternative considered: mutate the Patch when the player first selects a choice and roll it back if preparation fails. Rejected because failure would make canonical state advertise a graph that never existed and rollback would create a second mutation path.

Alternative considered: commit only after audio acknowledges the swap. Rejected because audio would then render an engine not represented by canonical state, reversing the required state-to-audio projection direction.

### 3. Keep the PATCH interaction narrow and descriptor-driven

The engine row receives one stable semantic control id and is the only editable PATCH row in this increment. Opening PATCH focuses that row. The existing translator already maps held K plus A/D to `Adjust(Left/Right)`; the reducer interprets those directions as previous/next registry choices only while PATCH and the engine row are active. Choice order comes solely from the immutable registry, does not wrap, and reports the existing boundary behavior at either end. Navigate, Adjust Up/Down, ADSR rows, Scalar rows, and Structural rows remain unavailable in PATCH.

The target config is constructed generically from the selected descriptor's declared defaults and required default asset references, then passed through the identity-matched `InstrumentCapabilityProvider` and registry validation. Returning to a capability therefore creates its declared default config; this increment stores no inactive-engine config cache. The active capability, requested capability, status, request identity, target revision when known, and typed failure are projected from canonical state. During `Preparing`, the old active capability remains selected and the requested choice is shown separately. During `Activating`, the committed target is shown with an activation-pending status. The row is temporarily unavailable until acknowledgement; all other rows remain read-only.

Alternative considered: add SoundFont/Braids branches to choose their defaults. Rejected because defaults and asset requirements already belong to descriptors/providers and later installed capabilities must use the same path.

Alternative considered: add a modal and the complete future `FocusPath` now. Rejected because the basic text slice can prove quick-choice semantics with one focusable engine row; modal navigation and additional PATCH controls belong to later Phase 3/Figma increments.

### 4. Put preparation behind an injected, bounded worker port

Add a control-facing graph-preparation worker contract with nonblocking `submit` and `poll` operations. A request contains the correlated identity, target revision, immutable candidate Patch/config snapshot, a compatible provisional `ParameterSnapshot`, registry identity, and negotiated sample rate/frame capacity needed by the existing `PreparedGraphBuilder`. A result returns either the same identity plus one fully owned `PreparedGraph` or one typed control-side failure. The port never exposes partial engines or callback-owned state. Immediately before publication, the candidate graph is rebound to the exact committed target projection so scalar edits accepted while preparation ran cannot be reverted at activation.

Normal standalone composition uses one dedicated standard-library worker with bounded request/result capacity one; it may allocate, load, parse, warm, and destroy candidates. Control ticks submit once and poll without waiting. Shutdown joins the worker only after the audio stream is released. No async-runtime dependency is added.

The deterministic demo injects a manually advanced adapter implementing the same port. Its healthy path invokes the real providers, preparers, and `PreparedGraphBuilder`; its controlled failure path returns a typed error at the worker seam. Explicit demo steps advance completion, so evidence is independent of wall-clock scheduling while still exercising the production application service. The physical live demo uses the production threaded adapter and never waits synchronously: each window tick advances structural orchestration once, and the scene remains on its current step until canonical state and structural acknowledgement permit the next one.

Alternative considered: prepare synchronously inside the keyboard callback or `AppState::apply`. Rejected because asset and engine preparation may allocate or block and the reducer must remain pure domain behavior.

Alternative considered: let the demo mutate AppState or publish a graph directly. Rejected because that would not prove the production workflow requested by the user.

### 5. Commit, project, then publish one complete layout-changing graph

`AppLoop` becomes the single control-side coordinator for selection outcomes, parameter publication, and the control half of `StructuralGraphCoordinator`; the callback half remains owned by `AudioRenderer`. The worker's owned graph is associated with, but never embedded in, the semantic completion event.

For a healthy result, `AppLoop` performs this order:

1. Validate that the result, candidate config, staged graph, target revision, and pending state all identify the same request and Patch.
2. Apply the prepared outcome through `AppState::apply`, committing the candidate config and `Activating` status.
3. Derive the new state/text/tree and a `ParameterSnapshot` tagged with the target graph revision.
4. Refresh the candidate graph's fixed initial snapshot from that exact committed state so Scalar/mixer/envelope edits accepted during preparation are not lost.
5. Publish the parameter snapshot, then transfer the complete graph through the structural boundary and record the structural effect.
6. On later control ticks, poll activation status, collect retired ownership, and dispatch the correlated acknowledgement through `AppState::apply` only after active revision, retired revision, and control-side collection all agree.

The structural coordinator no longer requires an identical capability/scalar layout forever. It admits a candidate only when stable Patch ids/order, Patch count, sample rate, maximum frames, stem/mixer capacities, and request metadata match; the selected Patch's capability and descriptor-ordered Scalar layout may differ. It adopts the new required layout only after complete acknowledgement. The renderer continues rejecting snapshots incompatible with whichever graph is actually active and uses the replacement's embedded compatible snapshot if activation wins the transport race.

If the prepared queue is unexpectedly unavailable after state commit, `AppLoop` retains the complete graph in its one bounded control-owned staged slot, remains visibly `Activating`, and retries on later ticks; it neither rolls back nor drops/substitutes the graph. No second request is accepted until publication, activation, retirement, and collection complete.

Alternative considered: send a patch-level engine object through a new structural transport. Rejected because `DESIGN.md` requires complete prepared graph ownership and three distinct existing RT data classes.

Alternative considered: publish the graph before reducer commit. Rejected because the callback could activate audio state that canonical state and projections had not accepted.

### 6. Make status and causality observable without UI-owned state

Versioned `StateSnapshot`, `StateTree`, `PatchPageProjection`, `TextProjection`, `EventRecord`, and demo schemas add the exact lifecycle fields and typed effects. The text shell renders the canonical engine row, requested target, status, and failure; it does not poll the worker, infer graph state, or retain a second selection. Event records distinguish the user adjustment, worker outcome, and activation acknowledgement and correlate their request id, generations, state hashes, graph revisions, parameter publication, and structural effect.

The live demo keeps its existing frozen editable-scalar coverage, mapped-input isolation, and bounded cleanup contract, then adds a distinct ordered engine-transition set. It requests SoundFont → Braids → descriptor-default SoundFont for the focused first fixture Patch only through semantic `Adjust` events. The runner observes, but does not manufacture, `Preparing`, `Activating`, `Ready`, request correlation, and increasing active revisions from canonical projections and structural status. After each acknowledgement it dispatches targeted MIDI and waits for a newer matching-generation finite nonzero render observation. A changed label, a constructed candidate, source-engine audio, or unrelated Patch output cannot credit the transition.

Alternative considered: expose worker strings or graph objects to the view for progress. Rejected because runtime state permits status values only and errors must remain typed, deterministic, and serializable.

### 7. Make engine switching part of both autonomous demo proofs

The headless scene uses the focused first fixture Patch and production keyboard translation to request SoundFont → Braids, observes `Preparing`, attempts a second request and proves the busy rejection, advances the real preparation worker, observes the prepared commit and graph publication, renders the block-boundary swap, collects/acknowledges retirement, sends targeted MIDI, and measures nonzero finite Braids output. It then performs Braids → SoundFont through the same path, proving the engine-managed SoundFont result and finishing with the descriptor-default SoundFont config.

A separate scripted request returns a typed preparation failure and proves the prior config, graph revision, rendered engine, and unrelated Patch state remain exact; stale outcome and acknowledgement probes are rejected and later valid selection still succeeds. Coverage derives the expanded event/effect/rejection/status/serialized-leaf universe from production descriptors. Two fresh runs must produce byte-identical logical evidence. Scalar parameters, sends, MIXER selection, and context are still restored to their captured baseline; the final structural config is the explicitly declared descriptor-default SoundFont result.

Add a focused `engine_selection_workflow` integration target with a structured observation and acceptance marker. It measures real provider/preparer usage, request correlation, old-engine preservation before success, both engine directions, compatible snapshots, one-in-flight throttling, activation/retirement acknowledgement, target-only config change, finite audible output, and zero callback allocations/destructions. `make demo` must also include the same successful and failed lifecycle; the named target cannot replace it.

The live scene completes its original scalar surface first so that its frozen expected set is stable, then runs the two successful directions through `ThreadedGraphPreparationWorker`. Each direction has separate lifecycle, revision, and target-audio checkpoints, and the final report requires two ordered transitions, no fallback, zero callback allocation/destruction, and the exact descriptor-default SoundFont config. The live scene does not inject failure or stale outcomes into a physical performance; those controlled negatives remain exhaustive and deterministic in the headless scene. Its deterministic-clock contract test drives the same runner and production worker/structural ports without opening a native window or physical device, while composition tests prove the real live command selects the threaded adapter.

Alternative considered: cover only a reducer table and coordinator unit test. Rejected because a no-op worker, missing graph publication, fallback engine, or silent renderer could pass without production-path demo evidence.

## Risks / Trade-offs

- [A complete graph swap resets active voices and effect tails] → Keep the old graph audible through preparation, swap only at a block boundary, state the reset explicitly, and prove subsequent targeted MIDI is audible and correctly routed; seamless voice migration is not claimed.
- [Scalar edits accepted while preparation runs could leave a stale embedded snapshot] → Re-project and bind the candidate's fixed initial snapshot from the exact committed completion generation immediately before publication, then publish the matching latest snapshot.
- [State commits but structural publication is momentarily full] → Retain exactly one complete staged graph on control ownership, remain `Activating`, retry without rollback or fallback, and reject new structural work until acknowledgement.
- [Worker completion races a newer request or altered Patch identity] → Use monotonic request ids plus exact Patch/capability/revision/config/layout validation; stale or mismatched outcomes are unchanged typed rejections and candidate destruction occurs off callback.
- [Target descriptor defaults do not preserve prior engine-specific settings] → Show the exact candidate before commit and document descriptor-default behavior; inactive-engine caches and preset workflows remain explicit later work.
- [The demo becomes nondeterministic because production uses a thread] → Inject the same worker port with explicit step advancement in the headless scene while using the real builder/preparers for the healthy path.
- [Live worker and device timing varies across machines] → Never use a fixed completion delay; hold each live scene step until canonical lifecycle, graph acknowledgement, and a newer qualifying audio observation arrive, keep every tick nonblocking, and retain typed timeout/device failure rather than fabricating success.
- [Schema expansion weakens existing exhaustive checks] → Derive events, effects, rejections, lifecycle states, and leaves from production-owned descriptors and require exact missing/unexpected equality plus two-run evidence.
- [The new orchestration accidentally performs work on the callback] → Keep only complete graph ownership transfer and existing bounded activation/retirement in `AudioRenderer`; allocator/destructor instrumentation remains a required predicate during both engine directions.

## Migration Plan

1. Use the already evaluated Phase 3 CUE architecture, validations, witness, and project check set as the implementation contract; keep it reconciled to `DESIGN.md` as Rust changes land.
2. Add canonical request/status/failure and event/effect schema, reducer transitions, versioned serialization, PATCH projection, and descriptor-default config construction while the engine row remains disabled at composition level.
3. Add the injected graph-preparation worker and extend `AppLoop`/`StructuralGraphCoordinator` for correlated layout-changing publication and acknowledgement; wire normal, smoke, demo, and live compositions without adding a hidden boundary.
4. Enable the engine-row quick-choice input, explicit status rendering, and recovery behavior through the existing basic adapter.
5. Extend both scene/report schemas, add the two live success directions and the deterministic headless negative paths, then add the named engine-selection acceptance; run focused, all-target, smoke, headless two-run, physical live-demo, live contract, real-time, mutation, performance, CUE, and OpenSpec gates.
6. Update `ROADMAP.md` to mark the read-only Phase 3 increment complete and engine selection as the completed/current checkpoint when acceptance succeeds.

Rollback is atomic across the schema and behavior: remove the worker/lifecycle types, restore the read-only CUE and OpenSpec requirements, and restore the prior schema versions together. No persisted state migration exists in this increment, and an unpublished or retired candidate is destroyed only on control/worker ownership before rollback completes.
