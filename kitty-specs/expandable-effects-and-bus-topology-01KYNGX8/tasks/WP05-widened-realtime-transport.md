---
work_package_id: WP05
title: Widened real-time transport
dependencies:
- WP03
- WP04
requirement_refs:
- FR-012
- FR-016
- NFR-001
- NFR-002
- NFR-003
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T026
- T027
- T028
- T029
- T030
- T031
history:
- timestamp: '2026-07-29T02:11:28Z'
  actor: planner
  action: created
agent_profile: implementer-ivan
authoritative_surface: src/real_time/
create_intent: []
execution_mode: code_change
mission_id: 01KYNGX8QA8V49BX2WQ1Q6G2BP
mission_slug: expandable-effects-and-bus-topology-01KYNGX8
model: ''
owned_files:
- src/real_time/parameter_snapshot.rs
- src/real_time/prepared_graph.rs
- src/real_time/prepared_graph_builder.rs
- src/real_time/audio_renderer.rs
- src/real_time/patch_audio_block.rs
- src/real_time/callback_safety.rs
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP05 – Widened real-time transport

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

This loads your identity, boundaries, and governance context. Do not skip this step.
Once loaded, continue with the Objective below.

## Objective

Grow the single fixed latest-value `ParameterSnapshot` to carry everything WP03 and
WP04 introduced — 16 patches × 3 slots × 8 scalars, 16 tracks × 8 sends, and 8
returns — while preserving fixed layout, exact structural matching, and callback
safety.

**This is the highest real-time risk in the mission.** The block roughly triples.
The design decision (research.md R-01) was to keep one monolithic snapshot rather
than split into three transports, on the grounds that the architecture declares
exactly three transport *categories* and splitting would multiply transports without
adding a category — while introducing cross-snapshot correlation and staleness
questions the current design does not have to answer. That decision trades copy size
for proof simplicity, which means the copy size must actually be **measured**, not
assumed acceptable.

## Context

- **Mission**: expandable-effects-and-bus-topology-01KYNGX8
- **Priority**: P1
- **Dependencies**: WP03 (slots), WP04 (buses) — both must land first
- **Related requirements**: FR-012 (prepared topology exchange), FR-016 (graph retirement), NFR-001 (render-path safety), NFR-002 (bounded capacity), NFR-003 (atomic activation)
- **Read first**: `research/contracts/realtime-snapshot.md` — obligations C-RT-1 through C-RT-14
- **Reference**: `research.md` R-01

## Branch Strategy

- **Planning base branch**: `feat/expandable-effects-and-bus-topology`
- **Merge target branch**: `feat/expandable-effects-and-bus-topology`
- **Execution**: worktree-per-lane. `finalize-tasks` computes lanes and writes `lanes.json`; each lane gets exactly one worktree and one branch.
- Do not create ad-hoc branches by hand; use the workspace the runtime resolves for this WP's lane.

## Subtasks

### T026 – Widen `RtPatchParameters` to three effect slots

- **Purpose**: `RtPatchParameters` currently holds one `effect: RtPostEffectParameters` (parameter_snapshot.rs:197).

- **Steps**:
  1. Change to `effects: [RtPostEffectParameters; MAX_EFFECT_SLOTS]`.
  2. Update `RtPostEffectParameters::EMPTY` initialization at lines 209, 225, 242, and 279 — there are several construction sites, and missing one leaves a partially initialized snapshot.
  3. Update `projected_with_effect` (line 230) to take all three slots.
  4. Update the `effect()` accessor (line 269) to take a slot index; update the borrowed-view struct at lines 295-303.
  5. Keep `MAX_EFFECT_SCALAR_PARAMETERS` at 8 per slot.
  6. Storage stays `[f32; N]` arrays — fixed, destructor-free, no heap.

- **Validation**: A fully occupied 16 × 3 snapshot constructs with every slot correctly initialized; no construction site left at `EMPTY` by accident.

### T027 – Add indexed sends to real-time track parameters

- **Purpose**: The real-time projection of a track must carry eight sends.

- **Steps**:
  1. Add `sends: [f32; MAX_BUS_RETURNS]` to the real-time track parameters.
  2. Remove the two named send projections.
  3. Validation happens in the domain type (WP04); the real-time projection carries already-validated finite values. Preserve the existing `NonFinite`-class error handling on projection.

- **Validation**: All eight sends project; a non-finite value is rejected at projection time, not passed to the render path.

### T028 – Add `RtBusReturnParameters` to the snapshot

- **Purpose**: Returns need a bounded real-time projection.

- **Steps**:
  1. Add `RtBusReturnParameters { active, slot_id, scalar_count, scalars: [f32; MAX_EFFECT_SCALAR_PARAMETERS], return_level }`, modelled directly on `RtPostEffectParameters` (lines 99-153) since it plays the same role.
  2. Add `returns: [RtBusReturnParameters; MAX_BUS_RETURNS]` to `ParameterSnapshot`.
  3. Provide an `EMPTY` const following the existing pattern.
  4. Reuse the existing effect-projection error variants (capacity exceeded, non-finite scalar, unresolved config) rather than inventing parallel ones.

- **Validation**: Eight returns project; capacity and finiteness errors fire correctly.

### T029 – Extend `SERIALIZED_LEAF_DESCRIPTOR`

- **Purpose**: `ParameterSnapshot::SERIALIZED_LEAF_DESCRIPTOR` (line 383) enumerates every leaf of the block. An incomplete descriptor silently under-reports state.

- **Steps**:
  1. Add leaves for all three slots per Patch — the existing `"patches[].effect.active"` style entry becomes slot-indexed.
  2. Add the eight per-track send leaves.
  3. Add every leaf of the eight returns.
  4. Remove the retired reverb/delay global leaves.
  5. `occurrence_map.yaml` sets `serialized_keys: rename` — indexed names are correct here, and the justification is recorded in the map.

- **Validation**: A completeness test asserts the descriptor enumerates exactly the leaves the struct actually has. Obligation C-RT-6.

### T030 – Wire the return rack into `PreparedGraph` and the renderer

- **Purpose**: The prepared graph must own the return rack as a peer of the post-effect rack.

- **Steps**:
  1. Add `PreparedBusReturnRack` (built in WP04) to `PreparedGraph` and to `prepared_graph_builder.rs`. Note `prepared_graph.rs` currently references `GlobalReverbDelay` in 6 places — those become the return rack.
  2. Update `audio_renderer.rs` to drive the widened seam: engine rack → patch audio block → post-effect rack (3 slots) → mix engine (indexed sends) → return rack → master.
  3. Preserve off-callback retirement of superseded graphs (FR-016). Nothing may be destroyed inside the callback.
  4. Preserve block-boundary activation: no rendered block may observe a partially applied topology (NFR-003, C-RT-9).

- **Validation**: The renderer produces correct output with slots and returns occupied; retirement stays off-callback; activation is atomic at the block boundary.

### T031 – Measure publish cost and prove zero render-time growth

- **Purpose**: The design decision in R-01 is only sound if measured. Contract C-RT-7 says measure, do not assume.

- **Steps**:
  1. Measure snapshot publish cost through the existing `triple_buffer` transport at the widened size, under a **fully occupied** configuration (16 patches × 3 slots, all 8 sends non-zero on all 16 tracks, all 8 returns occupied).
  2. Prove zero dynamic growth at render time in that configuration (NFR-002, C-RT-3). Use the existing callback-safety validation surface in `callback_safety.rs`.
  3. Confirm the render path still performs no allocation, locking, blocking, I/O, logging, panic, or destruction — **including during topology activation** (NFR-001, C-RT-1).
  4. If publish cost proves problematic, **stop and report** rather than silently splitting the snapshot. Splitting reverses a recorded decision (R-01) and must be re-decided, not improvised.

- **Validation**: Measurements recorded; zero growth events; callback contract intact.

## Test Strategy

- Layout test: the widened snapshot has exactly the expected fixed shape.
- Descriptor completeness test (T029) — cheap, and catches a whole class of silent under-reporting.
- **Exactness tests**: `matches_parameters` between snapshot and both racks must stay exact at the widened size (C-RT-5). Extend the negative tests WP03 added, and add equivalents for returns.
- Callback-safety validation at the widened size, under full occupancy.
- Publish-cost measurement (T031).
- Existing `tests/production_runtime_contracts.rs` and the audio-renderer real-time contract validation must keep passing.

## Definition of Done

- `RtPatchParameters` carries three slots; every construction site correctly initialized.
- Track parameters carry eight indexed sends; returns carry eight projections.
- `SERIALIZED_LEAF_DESCRIPTOR` is complete and proven so.
- The return rack is owned by `PreparedGraph` and driven by the renderer.
- Structural matching is exact at the widened size, with negative tests.
- Publish cost measured; zero render-time growth proven under full occupancy.
- Callback contract holds during topology activation.
- `make lint`, `make fmt-check`, and `make test` pass.

## Risks & Mitigations

- **Publish cost regression** → measure under full occupancy, not a typical case. If it is a problem, report rather than re-architecting; R-01 is a recorded decision.
- **A missed `EMPTY` construction site** → there are at least four in `parameter_snapshot.rs`. Grep for `RtPostEffectParameters::EMPTY` and check each.
- **Weakening exactness to make the widened matching pass** → same failure mode as WP03/T016. Negative tests first.
- **Destruction inside the callback** → retiring a graph with three times the state makes this easier to get wrong. Verify retirement stays off-callback explicitly.

## Reviewer Guidance

- **Check that exactness was not relaxed.** Compare the widened `matches_parameters` paths against their originals condition by condition.
- Confirm the descriptor completeness test exists and actually enumerates against the struct rather than a hand-copied list.
- Confirm the publish-cost measurement used a fully occupied configuration, not a default one.
- Confirm no allocation or destruction appears in the callback path during activation — this is the property the whole architecture rests on.
- Confirm `src/mixer/` and `src/synth/` are untouched.
