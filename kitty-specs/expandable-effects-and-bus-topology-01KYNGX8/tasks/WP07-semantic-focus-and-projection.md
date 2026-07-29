---
work_package_id: WP07
title: Semantic focus and projection for slots and returns
dependencies:
- WP06
requirement_refs:
- FR-002
- FR-003
- FR-014
- FR-017
- FR-018
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T039
- T040
- T041
- T042
- T043
- T044
- T045
history:
- timestamp: '2026-07-29T02:11:28Z'
  actor: planner
  action: created
agent_profile: frontend-freddy
authoritative_surface: src/control/semantic_focus.rs
create_intent: []
execution_mode: code_change
mission_id: 01KYNGX8QA8V49BX2WQ1Q6G2BP
mission_slug: expandable-effects-and-bus-topology-01KYNGX8
model: ''
owned_files:
- src/control/semantic_focus.rs
- src/control/patch_page_projection.rs
- src/control/text_projection.rs
- src/control/interaction_state.rs
- src/control/semantic_graphical_view_model.rs
- src/control/graphical_shell_projection.rs
- src/control/patch_control_id.rs
- src/control/semantic_resolver.rs
- src/shell/**
- src/adapter/eframe_graphical_window.rs
priority: P2
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP07 – Semantic focus and projection for slots and returns

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load frontend-freddy
```

This loads your identity, boundaries, and governance context. Do not skip this step.
Once loaded, continue with the Objective below.

## Objective

Make slots and returns reachable and editable through the interface, using the
adjacent-choice contract that already governs the engine row — and nothing new.
Phase 7 owns choice modals; this phase edits in place (C-008).

The interaction contract already exists and is documented at DESIGN.md:309. On a
descriptor-declared structural-choice row, Edit+Left/Right requests the adjacent
declared choice without wrapping, and Edit+Up/Down is unavailable. Slot and return
occupancy rows are structural-choice rows. Reuse that contract exactly rather than
inventing a slot-specific gesture.

## Context

- **Mission**: expandable-effects-and-bus-topology-01KYNGX8
- **Priority**: P2
- **Dependencies**: WP06 (occupancy actions and status must exist first)
- **Related requirements**: FR-002 (adjacent-choice occupancy), FR-003 (descriptor-driven parameters), FR-014 (observable outcome), FR-017 (focus survival), FR-018 (chain follows the Patch)
- **Constraints**: C-003 (PATCH and MIXER only), C-008 (no choice modal)
- **Reference**: DESIGN.md:309 (focus order and edit vocabulary), DESIGN.md:488 (return paths)

## Branch Strategy

- **Planning base branch**: `feat/expandable-effects-and-bus-topology`
- **Merge target branch**: `feat/expandable-effects-and-bus-topology`
- **Execution**: worktree-per-lane. `finalize-tasks` computes lanes and writes `lanes.json`; each lane gets exactly one worktree and one branch.
- Do not create ad-hoc branches by hand; use the workspace the runtime resolves for this WP's lane.

## Subtasks

### T039 – Extend PATCH focus order with slot rows

- **Purpose**: The reducer-owned ordered focus surface must include the three slots.

- **Steps**:
  1. The current order (DESIGN.md:309) is: Engine, Attack, Decay, Sustain, Release, visible instrument `StructuralChoice` parameters, then each configured effect's visible `ScalarEdit` parameters.
  2. Extend to: after the instrument choices, each of the three slots contributes an occupancy row followed by that slot's visible parameters when occupied.
  3. An **unoccupied slot still shows its occupancy row** — otherwise the player cannot fill it. This is the key structural difference from today, where an absent effect contributes nothing.
  4. Bare Up/Down moves through the order without wrapping, unchanged.
  5. Focus order is reducer-owned and resolved from the registries. The UI never computes it.

- **Validation**: All three slot rows are reachable whether occupied or empty; ordering is stable and non-wrapping.

### T040 – Add adjacent-choice edit on slot and return rows

- **Purpose**: Occupancy is chosen in place, using the existing structural-choice vocabulary.

- **Steps**:
  1. On a slot occupancy row: Edit+Left/Right requests the adjacent choice — empty, then each installed registry entry in declared order — **without wrapping**. Edit+Up/Down is unavailable, matching the engine row.
  2. Same contract on a return occupancy row.
  3. The request goes through the WP06 semantic action and the correlated structural-edit lifecycle. Nothing about this row is special-cased.
  4. On slot **scalar** rows, the normal contract applies: Edit+Left/Right is fine decrement/increment, Edit+Down/Up is coarse.
  5. Do not introduce a modal, a picker, or a new key. C-008.

- **Validation**: Occupancy cycles through empty and every installed entry without wrapping; Edit+Up/Down does nothing on occupancy rows.

### T041 – Project descriptor-driven slot parameters

- **Purpose**: Each configured effect presents its own parameters with no bespoke per-effect logic (FR-003).

- **Steps**:
  1. Extend `patch_page_projection.rs` so each occupied slot projects its descriptor-declared visible parameters, in descriptor order, with labels, bounds, units, and current values.
  2. The projection must be driven entirely by the descriptor. No branch anywhere may test which effect occupies a slot.
  3. Slot identity in the projection is positional — "slot 2", not "the chorus".
  4. Update `text_projection.rs` correspondingly; it carries retired send-field references that must go.

- **Validation**: Adding a registry entry changes the projected rows with zero projector changes — the concrete test of SC-008 at this layer.

### T042 – Extend MIXER projection with indexed sends and returns

- **Purpose**: MIXER must expose eight sends per track and the eight returns.

- **Steps**:
  1. Project the four track controls (level, pan, mute, solo) plus eight indexed sends per track.
  2. Project each return: its occupancy, its descriptor-driven parameters when occupied, and its return level.
  3. DESIGN.md:466 says the Inspector identifies cursor, value/range, mute/solo, and route/sends — extend that to the eight sends.
  4. Distinct globals reduce to master gain alone (WP04's `GlobalParameter` reduction).
  5. Keep control state explicit in text or shape as well as colour, per the project's accessibility stance.

- **Validation**: All sixteen tracks project eight sends; all eight returns project; master gain is the only remaining global.

### T043 – Prove deterministic focus recovery

- **Purpose**: Rows appear and disappear as occupancy changes. Focus must never dangle (FR-017).

- **Steps**:
  1. Clear a slot **while its scalar parameters hold focus**. Focus must resolve deterministically to a valid neighbouring position — specify which, and prove it, rather than leaving it emergent.
  2. Occupy an empty slot while focus is on its occupancy row; the new parameter rows appear beneath without moving focus off the occupancy row.
  3. Prove recovery is deterministic across reprojection — the same starting state and the same action always yield the same resulting focus.
  4. Reuse the existing focus-recovery machinery proven in Phase 2 rather than adding a slot-specific path.

- **Validation**: Focus is always on a valid row; recovery is deterministic across two runs.

### T044 – Render slot and return rows in the shell

- **Purpose**: The shell paints the projection and emits semantic actions. Nothing more.

- **Steps**:
  1. Render slot occupancy rows, slot parameter rows, indexed sends, and return rows in `src/adapter/eframe_graphical_window.rs` and `src/shell/`.
  2. The shell owns **no** Patch values, focus, navigation, reducer state, or audio state. It receives immutable view data and emits semantic actions.
  3. Render the topology status from WP06 — pending, accepted, refused with reason — without caching or recomputing it.
  4. No effect-specific rendering branch. A row is a row.

- **Validation**: The shell compiles with no domain state; every interaction emits a semantic action.

### T045 – Prove PATCH and MIXER remain the only top-level contexts

- **Purpose**: C-003. Adding slots and returns must not tempt a third context.

- **Steps**:
  1. Assert the top-level context enumeration is unchanged (`src/control/top_level_context.rs` is **not** in this WP's owned files — if you believe it must change, stop and escalate).
  2. Assert returns are reachable within MIXER and slots within PATCH.
  3. Assert no modal or detail surface was introduced (C-008).

- **Validation**: Two top-level contexts; no new surface.

## Test Strategy

- Focus-order tests covering occupied and empty slots.
- Adjacent-choice tests: no wrapping, Edit+Up/Down unavailable on occupancy rows.
- Descriptor-driven projection test proving a new registry entry needs no projector change (SC-008 at this layer).
- Focus-recovery determinism tests (T043) across two runs.
- MIXER projection tests for eight sends and eight returns.
- Existing `tests/patch_page_projection.rs`, `tests/semantic_graphical_view_model.rs`, and `tests/graphical_application_shell.rs` must keep passing or be updated deliberately.

## Definition of Done

- All three slot rows reachable whether occupied or empty.
- Adjacent-choice occupancy editing matches the engine-row contract exactly; no modal.
- Slot and return parameters are descriptor-driven with no per-effect branch.
- MIXER projects eight sends per track and eight returns; master gain is the only global.
- Focus recovery is deterministic and proven.
- The shell owns no domain state.
- PATCH and MIXER remain the only top-level contexts.
- `make lint`, `make fmt-check`, and `make test` pass.

## Risks & Mitigations

- **Dangling focus when a slot is cleared** → the most likely defect here. Decide the recovery target explicitly and test it; do not let it be emergent.
- **A per-effect rendering or projection branch creeping in** → this would defeat FR-003 and SC-008. The projection test in T041 is the guard.
- **Introducing a modal because in-place editing feels cramped** → C-008 forbids it; Phase 7 owns modals.
- **Hiding empty slot rows** → then occupancy can never be set. Empty slots must still show their row.
- **UI caching status or values** → reducer-owned; the shell renders only.

## Reviewer Guidance

- **Try clearing a slot while its parameters hold focus.** If focus lands somewhere undefined, reject.
- Confirm empty slot rows are visible and editable.
- Grep the projection and shell for any branch on effect identity — there must be none.
- Confirm Edit+Up/Down is unavailable on occupancy rows and that adjacent choice does not wrap.
- Confirm `src/control/top_level_context.rs` was not modified.
- Confirm no new modal, detail, or top-level surface appeared.
