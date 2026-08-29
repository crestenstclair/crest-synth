## Context

See `proposal.md` for motivation and the three delta specs for the behavioral contract. The current `AppState` stores only a `Vec<Patch>` and every PATCH focus path carries a `PatchId`; `SelectPatch(Right)` therefore returns a boundary rejection on the final Patch. `SavedSession` captures that Patch vector and Phase 01 accepts only one through sixteen created Patches, while `ParameterSnapshot`, prepared engine/effect racks, Patch audio storage, observations, and complete graph layout are fixed at 16.

The existing structural path already supplies the essential transaction shape: a semantic edit enters reducer-owned Loading/Validating/Preparing/Activating state, a worker prepares a complete candidate graph, the coordinator admits one declared layout delta, the callback swaps at a block boundary, and matching acknowledgement commits the canonical value through `AppState::apply`. Whole-session replacement added by Phase 01 shares the coordinator and preflights its reducer/projection result before publication. Patch creation must extend these seams rather than mutate the Patch vector in shell code or create a second graph queue.

The live Figma workflow treats Patch positions as effectively unlimited. Production audio instead has a falsifiable fixed maximum of 16 active Patches. This design preserves the navigation grammar through a trailing empty position at every count, including 16, while making a seventeenth creation visibly unavailable. It does not claim unlimited-creation parity.

## Goals / Non-Goals

**Goals:**

- Give interaction and focus a canonical way to name a created Patch or the one trailing empty position without fabricating a `PatchId`.
- Derive prospective controls and one immutable creation candidate from the same product-authored defaults.
- Extend the existing single structural request, worker, graph coordinator, and reducer acknowledgement path with an append-only Patch delta.
- Keep prior Patches playable and canonical while a candidate is prepared, and expose the created Patch only after the matching graph is active.
- Make focus rekeying, dirty-state changes, Save/Open behavior, capacity refusal, and every failure deterministic and testable.

**Non-Goals:**

- A dormant, paged, unloaded, virtualized, or otherwise audio-inactive kind of persisted Patch.
- Raising or making dynamic the callback's 16-Patch bound.
- Patch deletion, duplication, reordering, multi-select, or identity-reuse policy for future deletion.
- Zero-Patch startup/session files; Phase 01's valid session remains one through sixteen created Patches.
- A new persistence format, automatic save, or restoration of interaction focus from a file.
- Physical gamepad acceptance or broad Patch Overview visual parity beyond the empty/pending/failure/capacity states in this slice.

## Decisions

### 1. One `PatchPositionId` distinguishes created and trailing-empty interaction roots

Introduce one canonical public interaction identity:

```text
PatchPositionId
├── Created(PatchId)
└── TrailingEmpty
```

PATCH `FocusPath`, remembered root, return paths, subordinate sessions, and suspended MIDI Settings focus use this root. Existing `patch_id()` access remains a projection that returns a value only for `Created`; code that requires a real Patch must explicitly reject `TrailingEmpty`. This makes accidental MIDI, persistence, parameter, or graph lookup for the empty position a type-visible branch instead of a magic identifier convention.

The empty position is derived, not stored in `AppState::patches`: the navigable Patch-position order is every created identity in installation order followed by `TrailingEmpty`. `SelectPatch` continues to be the only sibling-position mutation. From a created position it performs existing control-identity recovery against the destination schema; from the final created Patch it recovers against the prospective schema; from empty it can move only left. Any Patch-owned subordinate surface is left by the same `set_active_main` transition used by current sibling switching.

Alternatives considered:

- **Reserve `PatchId(0)` or `u32::MAX` for empty.** Rejected because `PatchId` means a canonical Patch target and the sentinel could leak into persistence, MIDI, snapshots, or graph layout.
- **Store `Option<PatchId>` throughout.** Rejected because `None` cannot distinguish no PATCH root from the authored trailing empty position and encourages untyped special cases.
- **Append a placeholder `Patch` to the vector.** Rejected because it would immediately become saved and audio-domain content, violating the core outcome.

### 2. A prepared product blueprint is the sole source of prospective defaults

Factor the exact designated instrument identity and provider-authored default configuration used by Phase 01 into an immutable `PatchCreationBlueprint` prepared by the composition root and injected as non-session application configuration. The default-session factory and implicit-creation path share its designated capability/config resolution so registry order cannot make their defaults drift. Phase 01 keeps its authored first-Patch name `INIT`; implicit append derives `Patch {id}` plus its ordinal channel and track.

The blueprint produces a private `PatchCreationCandidate`, not a second public Patch model. It contains the checked reserved identity, appended ordinal, and a canonical `Patch` value ready for validation/preparation. Identity is `max(created PatchId) + 1`; it is never derived from position, channel, track, or label. The appended ordinal selects zero-based MIDI channel and Mixer track, which yields channel 2/T01 for the second Patch and channel 16/T0F for the sixteenth. The candidate begins with 0 dB trim, neutral envelope, seeded voice limit, and empty effects; the triggering edit changes exactly one target.

`SemanticResolver` receives a prospective-schema view backed by the same candidate factory. It may resolve Overview, default-Engine Detail, Choice, and Utility control descriptors for `TrailingEmpty`, but these rows are tagged prospective and are never returned by APIs that enumerate created Patches. This prevents the projector and reducer from separately reconstructing defaults.

Alternatives considered:

- **Use the first enabled registry descriptor.** Rejected as silent fallback and registry-order behavior.
- **Duplicate literal defaults in the projector.** Rejected because prospective display could disagree with the candidate that audio prepares.
- **Keep a mutable multi-edit draft.** Rejected because the contract creates on the first meaningful edit; navigation needs only an immutable prospective blueprint plus one triggering delta.

### 3. Existing semantic actions are interpreted against position ownership

No UI-specific `CreatePatch` button or direct DOM event is added. Existing normalized actions (`Adjust`, `Activate`, option selection, and typed occupancy actions) continue through `AppState::apply`. When their focus root is `TrailingEmpty`, the reducer classifies the target:

- a Patch-owned accepted change becomes one `AppendPatch` structural intent containing the reserved candidate and exact triggering delta;
- explicit confirmation of the default Engine is meaningful because it confirms creation even though its prospective value equals the blueprint;
- selecting `EMPTY` for an already empty effect, navigation, mode/surface entry, return, preview, and performance input remain non-creating;
- Master Volume remains a normal global edit and does not consume the empty position;
- a rejected boundary edit cannot create as a side effect.

The one-in-flight structural status owns a private pending candidate from request admission through acknowledgement. Loading/Validating/Preparing/Activating transitions mutate runtime/interaction state through `AppState::apply`, but the Patch vector remains unchanged. The accepted activation event consumes the pending candidate and is the sole branch that appends it. Creation requests are included in the closed saved-field-action classification used by Phase 01 so an authorized New/Open/Close cannot discard them silently.

Alternatives considered:

- **Add a separate explicit Create action.** Rejected because it changes the authored first-edit workflow and duplicates normal Patch controls.
- **Create the default Patch before applying the triggering edit.** Rejected because preparation or the edit could fail after a phantom saved Patch was exposed.
- **Let `AppLoop` append the worker's Patch directly.** Rejected because it bypasses the only product-state mutation path.

### 4. Patch append is a new scoped complete-graph replacement

Extend the existing structural intent and graph request with `AppendPatch { patch_id }` plus the separately owned immutable candidate. A dedicated request constructor validates:

- source revision and nonzero correlation;
- current created count below the one canonical active-Patch capacity;
- exact source Patch order and uniqueness;
- candidate identity, capability configuration, voice limit, effect positions, route, and defaults;
- candidate parameter projection at the target revision.

It then builds one complete graph from `active_patches + candidate`, the unchanged Mixer/global/return state, and the negotiated audio configuration. All capability/asset resolution, allocation, warming, routing, voices, effects, returns, and scratch remain on the worker.

Add `GraphReplacementScope::AppendPatch(PatchId)`. Layout admission requires identical device layout, returns, and every existing Patch position; the candidate layout must have exactly one additional final position with the correlated identity and prepared shapes. It rejects insertion, replacement, removal, reordering, an unrelated slot/return change, or more than 16 positions. During the block-boundary swap, live engines/effects and returns for every prior Patch are carried by stable identity; the appended Patch starts silent with no voices or tails. The old graph still retires through the bounded callback-to-control return path.

This narrow append scope is preferable to `WholeSession`: it proves the only permitted topology change, keeps existing audio state where the prepared components support transfer, and avoids presenting creation as a document replacement. It also avoids using `SelectedEngine` or `PatchSlot`, whose layout contracts deliberately require an unchanged Patch count.

### 5. Preflight makes the post-activation reducer commit deterministic

When the worker returns a prepared append graph, the control side dispatches the prepared lifecycle event, then applies the future activation commit to a cloned `AppState`. It projects the clone's initial parameter snapshot and asserts exact target revision, Patch order, identities, scalar layout, and values against the prepared graph. Only a passing preflight may stage the graph.

Immediately before staging, the graph's initial parameters are refreshed from the latest accepted global/Mixer/existing-Patch scalars plus the unchanged pending candidate. While Activating, compatible scalar snapshots use the target revision; the old renderer ignores them by revision, as in existing structural edits. Created-Patch MIDI fan-out continues to target only the source collection until the reducer commit, so the new graph position cannot sound early. Existing Patch targets remain valid across the append layout.

On matching coordinator completion, one activation event through `AppState::apply` appends the preflighted pending candidate, installs the target graph revision/status, performs focus rekeying, and marks saved content changed. Stale, mismatched, busy, failed, or duplicate events are unchanged typed rejections. A real post-swap commit rejection is treated as an invariant violation prevented by preflight and exact correlation, not as a recoverable alternate state after the old graph has retired.

Alternatives considered:

- **Commit state before graph activation.** Rejected because a later preparation/publication failure would expose a Patch the callback cannot render.
- **Publish without reducer preflight.** Rejected because a rare identity/focus/schema mismatch could surface only after the callback had irreversibly accepted the candidate.
- **Gate all MIDI and reset every voice.** Rejected for append because every old Patch target and layout position is proven unchanged; the new target is unreachable until commit. Bounded recovery remains available for an actual handoff failure.

### 6. Focus rekeying is one atomic interaction transformation

Add an `InteractionState` transformation that maps every still-live `TrailingEmpty` root to `Created(reserved_id)` across active focus, remembered PATCH main, return origin, Choice/Detail/Sample subordinate facts, and the interaction snapshot suspended by MIDI Settings. It preserves `SurfaceId`, `SemanticControlId`, effect-slot position, parameter identity, mode, and return relationship.

The transformation runs inside the activation commit after the candidate has joined the Patch collection, so `SemanticResolver` can validate every mapped path against the real schema. Exact controls win. If an alternate first Engine makes a prospective Detail control invalid, existing next-before-previous path recovery closes or repairs the surface and publishes the normal explicit focus-repair status. If the user moved left during preparation, their active/remembered created-Patch focus contains no empty root and is left unchanged; creation never steals it.

Alternatives considered:

- **Always focus the new Engine row.** Rejected because it discards the originating semantic control and violates stable return identity.
- **Keep empty paths after append.** Rejected because the derived trailing empty position now follows the newly created Patch and would make an old edit origin refer to the wrong position.
- **Use DOM focus restoration.** Rejected because layout/reprojection cannot own semantic identity.

### 7. Empty projection is a tagged variant, not a renderer heuristic

Extend the Patch page/semantic shell projection with explicit `Created` and `Empty` variants. The empty variant carries no `PatchId`; it carries projected prospective sections/controls, active global controls, `creationAvailable`, active-count/capacity facts, structural lifecycle/request/failure readings, focus/return facts, and reducer-derived valid actions. State-tree schema versioning is advanced once for the tagged shape.

The committed webview renders the same Overview/Detail/Choice/Utility components from the projection. It labels prospective rows `NEW`/`DEFAULT` (final wording and geometry remain governed by Figma and the existing vocabulary), never `CURRENT` or active, and shows capacity, pending phase, and failure in text/shape as well as color. JavaScript does not infer empty from Patch count, synthesize an identity, or decide which edits create.

Native evidence covers empty, pending phases, failure, capacity, and created transition at the existing responsive/text-scale matrix. Direct comparison is scoped to the relevant Patch Overview and interaction-map nodes; the 16-Patch limit is reported as a known mismatch rather than hidden by the visual fixture.

### 8. SavedSession version 2 remains unchanged

`SavedSession::capture` continues to iterate only `AppState::patches`. `PatchPositionId`, prospective defaults, pending candidate, lifecycle/failure, focus, graph, and capacity display are excluded by construction. No version bump or migration is needed because a successfully created Patch is an ordinary canonical `SavedPatch` and the valid one-to-sixteen shape is unchanged.

`StateAccepted::saved_session_changed` remains false for empty navigation and every precommit lifecycle event, and becomes true only when the activation event appends the Patch. Save during preparation therefore writes the old acknowledged set; if creation later succeeds, content equality leaves the document dirty against that exact written baseline. A failure leaves the baseline unchanged. Open reconstructs only saved Patches, resets focus per Phase 01, and derives a fresh empty position after the final Patch.

The session lifecycle and structural coordinator use explicit exclusion:

- once replacement/close is authorized, a new creation trigger is refused as a saved-field action;
- if creation already owns the structural lifecycle, New/Open reports or defers on Busy rather than canceling or racing it;
- any worker result with a superseded source revision is retired off-thread and cannot append into the replacement session.

### 9. One capacity constant governs control, persistence, projection, and RT layout

Replace the duplicated control-side `MAX_PATCH_COUNT` and RT-only `MAX_PATCHES` authority with one canonical public active-Patch capacity constant in a dependency-neutral domain contract. Fixed RT arrays continue to use its const value 16; reducer admission, saved-session validation, worker requests, graph layout, snapshots, projection, and tests import the same authority.

At 16 created Patches, `TrailingEmpty` remains part of interaction order. Its projection sets `creationAvailable = false`, states the 16-Patch audio capacity explicitly, and omits creation-triggering valid actions. A stale/direct trigger receives a typed capacity rejection before candidate allocation or worker submission. This is the explicit reconciliation for Phase 02; supporting more created Patches would require a separately specified audio ownership/activation model and cannot be inferred from Figma fixture counts.

### 10. Proof must distinguish five states through production seams

Focused deterministic tests record separate witnesses for:

1. navigation to/from empty: accepted interaction change, unchanged saved capture and graph revision;
2. prospective inspection/no-op/global edit: no Patch creation and correct dirty semantics;
3. pending creation: one request, no created Patch, requested/active separation;
4. acknowledged creation: one stable identity, exact candidate values, append layout, renderable audio, focus rekey, and dirty transition;
5. refusal/failure: capacity, identity exhaustion, registry/default/asset/preparation/publication/stale failures with exact prior state and graph preservation.

Graph-layout tests prove only a final append is admitted and existing positions transfer by identity. Callback instrumentation proves no allocation, deallocation, lock, block, I/O, logging, formatting, panic, unwind, or destruction during preparation handoff and render. Persistence tests cover Save before, during, and after creation plus Save/Open round-trip and no sentinel fields. Native webview tests prove projection-to-paint correspondence and responsive reachability. A bounded keyboard handoff proves Shift+Right → empty → first edit → created → Shift+Left through production input normalization; physical gamepad work remains out of scope.

Implementation completion updates `DESIGN.md` with the durable position identity, append-graph scope, capacity reconciliation, saved-session behavior, and measured results. OpenSpec artifacts remain planning evidence only.

## Risks / Trade-offs

- **[Risk] Prospective controls drift from the candidate built for audio.** → Use one injected blueprint/candidate factory for reducer resolution, projection, worker input, and tests; forbid literal defaults in the renderer.
- **[Risk] The callback activates a graph whose canonical append could reject.** → Clone-preflight the exact activation event and parameter projection before staging, then require exact request/source/target/candidate correlation on acknowledgement.
- **[Risk] Scalar edits accepted during preparation make the candidate snapshot stale.** → Refresh target-revision initial parameters from latest source state plus the immutable pending candidate immediately before staging and retain revision compatibility checks.
- **[Risk] Rekeying misses a suspended or subordinate empty path.** → Centralize the transformation in `InteractionState`, enumerate every path owner, assert invariants, and test active, remembered, return, Choice/Detail, and MIDI Settings cases.
- **[Risk] Showing an empty position at 16 implies it can still be created.** → Project explicit `CAPACITY 16/16` text/shape, omit creating actions, reject stale dispatches, and record the Figma mismatch in native acceptance.
- **[Trade-off] Monotonic `max + 1` identity can exhaust despite unused gaps.** → Fail visibly and leave state unchanged. Reuse after future deletion is a separate identity-policy decision and is not smuggled into this phase.
- **[Trade-off] Creation may carry existing voices/tails but does not promise seamless migration.** → Admit only an append-exact layout and preserve live state where current transfer supports it; retain the established allowance that a complete graph change may reset state without corrupting the callback.
- **[Risk] State-tree shape changes break stale consumers.** → Bump the schema once, update Rust/JavaScript serializers and exact-schema tests together, and reject mismatched documents rather than inferring old empty behavior.

## Migration Plan

1. Introduce the canonical Patch-position identity and single capacity authority while keeping created-Patch behavior unchanged; migrate focus/resolver/event-record/projection tests first.
2. Prepare and inject the shared creation blueprint, then add prospective resolver/projection behavior with navigation-only and renderer evidence.
3. Add creation-trigger classification, pending reducer state, worker append request, append layout scope, preflight, block-boundary activation, and atomic focus-rekey commit.
4. Integrate Phase 01 dirty/Save/Open/structural exclusion behavior and prove version-2 round trips without a migration.
5. Complete deterministic, callback-safety, native responsive, keyboard, controlled-failure, and capacity evidence; update `DESIGN.md` with only verified as-built results.

Rollback is a code/projection rollback: saved-session version 2 is unchanged, so files written after successful creation remain valid for the Phase 01 reader as long as they contain at most 16 canonical Patches. A rollback removes access to the trailing empty workflow but does not require rewriting user files.
