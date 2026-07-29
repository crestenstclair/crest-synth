---
work_package_id: WP06
title: Topology change lifecycle and rejection
dependencies:
- WP05
requirement_refs:
- FR-002
- FR-010
- FR-012
- FR-013
- FR-014
- FR-015
- FR-016
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: worktree-per-lane
subtasks:
- T032
- T033
- T034
- T035
- T036
- T037
- T038
history:
- timestamp: '2026-07-29T02:11:28Z'
  actor: planner
  action: created
agent_profile: implementer-ivan
authoritative_surface: src/control/state_tree.rs
create_intent: []
execution_mode: code_change
mission_id: 01KYNGX8QA8V49BX2WQ1Q6G2BP
mission_slug: expandable-effects-and-bus-topology-01KYNGX8
model: ''
owned_files:
- src/real_time/structural_graph_coordinator.rs
- src/real_time/graph_preparation_worker.rs
- src/real_time/graph_handoff_status.rs
- src/real_time/graph_revision.rs
- src/control/state_tree.rs
- src/control/state_projector.rs
- src/control/serialized_state.rs
- src/control/semantic_action.rs
- src/control/app_event.rs
- src/control/engine_selection.rs
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP06 – Topology change lifecycle and rejection

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

This loads your identity, boundaries, and governance context. Do not skip this step.
Once loaded, continue with the Objective below.

## Objective

Route slot and return occupancy changes through the **existing** correlated
structural-edit lifecycle — the same one that already handles engine selection and
SoundFont preset selection — with validation, visible outcome, controlled rejection,
recovery, and off-callback retirement.

The design instruction here is reuse, not invention. `src/control/engine_selection.rs`
and the structural graph coordinator already implement a correlated request →
preparation → acknowledgement → activation cycle with pending, busy, failure, and
stale handling. Occupancy changes are the same shape of operation. If you find
yourself writing a parallel lifecycle, stop — you are duplicating a proven one.

## Context

- **Mission**: expandable-effects-and-bus-topology-01KYNGX8
- **Priority**: P1
- **Dependencies**: WP05 (the widened transport must exist first)
- **Related requirements**: FR-002 (slot occupancy selection), FR-010 (return occupancy selection), FR-012 (prepared exchange), FR-013 (validated rejection), FR-014 (observable outcome), FR-015 (recovery), FR-016 (retirement)
- **Read first**: `contracts/realtime-snapshot.md` § Structural change lifecycle — obligations C-RT-8 through C-RT-14
- **Study**: `src/control/engine_selection.rs` — the pattern you are extending

## Branch Strategy

- **Planning base branch**: `feat/expandable-effects-and-bus-topology`
- **Merge target branch**: `feat/expandable-effects-and-bus-topology`
- **Execution**: worktree-per-lane. `finalize-tasks` computes lanes and writes `lanes.json`; each lane gets exactly one worktree and one branch.
- Do not create ad-hoc branches by hand; use the workspace the runtime resolves for this WP's lane.

## Subtasks

### T032 – Add slot and return occupancy semantic actions

- **Purpose**: Occupancy changes must enter through the canonical physical input → semantic action → `AppState::apply` → projection path, like every other edit.

- **Steps**:
  1. Add semantic actions for setting Patch slot occupancy (by `EffectSlotIndex`) and bus return occupancy (by `BusId`), each carrying an optional registry entry — `None` clears.
  2. Wire them through `src/control/semantic_action.rs` and `src/control/app_event.rs`.
  3. Call the domain transitions WP03 and WP04 provided (`SetSlotOccupancy`, `SetReturnOccupancy`).
  4. These are **structural** actions. They must not be routed onto the scalar snapshot path — occupancy changes what exists, not what a value is.
  5. Keep the action vocabulary generic: no action names an effect.

- **Validation**: Both actions dispatch through `AppState::apply` and reach the structural path.

### T033 – Validate occupancy changes before publication

- **Purpose**: An impossible topology must be refused outright rather than partly applied (FR-013).

- **Steps**:
  1. Validate before anything is published: slot index in range, bus id in range, registry entry resolvable, and the resulting configuration within the declared ceilings (3 slots, 8 returns).
  2. Reject invalid identities rather than clamping or substituting — this mirrors how invalid track identities are already rejected before publication.
  3. Preparation failure is also a refusal: an entry that cannot be prepared (for example a delay whose max-delay requirement cannot be satisfied) yields a refusal with a reason.
  4. Never silently fall back to another effect or to empty.

- **Validation**: Each invalid class produces a distinct, attributable refusal; none mutates state.

### T034 – Route occupancy through the structural preparation worker

- **Purpose**: The complete graph is prepared off-callback and exchanged whole (FR-012).

- **Steps**:
  1. Extend the request the coordinator sends to the worker so it carries the new topology — slots and returns — not just engine configuration.
  2. The worker builds a **complete** prepared graph: engine rack, post-effect rack with three slots per Patch, and the return rack. Partial graphs are never published.
  3. Preserve correlation: each request carries an identity the acknowledgement matches, so a stale acknowledgement is recognized and discarded.
  4. Preserve block-boundary activation (C-RT-9) — no rendered block may observe a partially applied topology.
  5. `src/testing/deterministic_graph_preparation_worker.rs` exists to make this lifecycle deterministically controllable. Coordinate with WP08, which owns that file, if its interface must change.

- **Validation**: An occupancy change is prepared off-callback and activates atomically at a block boundary.

### T035 – Prove controlled rejection leaves the active graph intact

- **Purpose**: The roadmap requires exercising "one controlled rejection" that does not replace the active graph.

- **Steps**:
  1. Drive a refusal while audio is playing.
  2. Assert: no graph is published; the active graph is untouched; audio continues without dropout; the previously configured effects and routing remain exactly as they were.
  3. Assert the refusal is attributable to the specific slot or return that failed, not reported generically.
  4. This is a **controlled negative** — it belongs in the witness WP01 declared, paired with the positive command.

- **Validation**: Sample-exact continuity across the refusal; configuration unchanged. Obligation C-RT-10.

### T036 – Project pending, accepted, and refused outcomes

- **Purpose**: The player must be able to tell what the instrument is doing (FR-014).

- **Steps**:
  1. Extend `src/control/state_tree.rs` and `state_projector.rs` so a topology change's status — pending, accepted, refused — is projected with its reason and the position it applies to.
  2. Reuse the existing structural lifecycle status vocabulary (pending, busy, failure, stale) rather than inventing a parallel one.
  3. Update `serialized_state.rs`. Note it carries ~12 occurrences of the retired reverb/delay global fields; those go, replaced by return-owned state.
  4. Status must be reducer-owned. The UI renders it; it never computes or caches it.

- **Validation**: Each status is projected with reason and position; no UI-owned status copy exists.

### T037 – Prove recovery and acknowledgement ordering

- **Purpose**: A mistake must not require a restart (FR-015), and concurrent requests must not corrupt correlation.

- **Steps**:
  1. **Recovery**: after a refusal, make a valid change; assert it is accepted, becomes audible, and leaves no residue of the failed attempt.
  2. **Ordering (C-RT-13)**: request two changes before the first is acknowledged; assert acknowledgements are neither reordered nor dropped, and that the final state matches the last accepted request.
  3. **Coexistence**: assert scalar and structural changes within one block both survive — an existing proven property that must not regress at the widened size.

- **Validation**: All three properties hold deterministically across two runs.

### T038 – Prove off-callback retirement at the widened size

- **Purpose**: Superseded graphs now carry roughly three times the state; retirement must still happen off-callback (FR-016, NFR-006).

- **Steps**:
  1. Assert superseded graphs are retired away from the audio path — nothing destroyed inside the callback.
  2. Assert nothing is left owned at exit: zero retained topology owners, zero leaked audio or worker resources.
  3. Exercise repeated topology changes so several graphs are retired in sequence.

- **Validation**: No destruction on the callback path; clean ownership at exit. Obligation C-RT-14.

## Test Strategy

- Validation tests for each invalid class in T033 (out-of-range index, unresolvable entry, unpreparable entry).
- Rejection continuity test (T035) — sample-exact audio continuity across a refusal.
- Recovery test and acknowledgement-ordering test (T037).
- Scalar/structural coexistence test at the widened size.
- Retirement and ownership tests (T038).
- Two-run determinism for the whole lifecycle.
- Existing `tests/engine_selection_workflow.rs` and `tests/soundfont_preset_selection.rs` must keep passing — they cover the lifecycle you are extending.

## Definition of Done

- Occupancy changes enter as semantic actions on the canonical path.
- Validation happens before publication; refusals are attributable and never silently substitute.
- The worker prepares complete graphs off-callback; activation is atomic at a block boundary.
- Refusal leaves audio uninterrupted and the active graph intact, proven sample-exactly.
- Status projects with reason and position, reducer-owned.
- Recovery, acknowledgement ordering, and scalar/structural coexistence proven.
- Retirement stays off-callback with clean ownership at exit.
- `make lint`, `make fmt-check`, and `make test` pass.

## Risks & Mitigations

- **Writing a parallel lifecycle instead of extending the existing one** → read `engine_selection.rs` first. If your diff introduces a second correlation mechanism, revert and reuse.
- **Acknowledgement reordering under concurrent requests** → T037 exists specifically for this; write that test early.
- **A refusal that mutates state before failing** → validate fully before any mutation or publication. Order matters.
- **Status cached in the UI** → status is reducer-owned; the shell renders only.
- **Coordination with WP08 over the deterministic worker** → that file is WP08's. Agree on the interface rather than editing across the boundary.

## Reviewer Guidance

- **Verify the rejection path first.** Confirm no publication occurs, no state mutates, and audio continuity is proven sample-exactly rather than asserted in prose.
- Confirm the existing lifecycle was extended, not duplicated — look for a second correlation or status vocabulary.
- Confirm occupancy is on the structural path, never the scalar snapshot.
- Confirm refusals name the specific slot or return.
- Confirm nothing is destroyed on the callback path.
