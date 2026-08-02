---
work_package_id: WP02
title: Guards assert over the record
dependencies:
- WP01
requirement_refs:
- FR-002
- FR-003
- FR-005
- FR-006
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T007
- T008
- T009
- T010
- T011
history:
- timestamp: '2026-08-02T00:10:35Z'
  actor: planner
  action: created from IC-02 and IC-07 (grade on the record; falsify each guard)
agent_profile: implementer-ivan
authoritative_surface: tests/effects_and_buses.rs
create_intent: []
execution_mode: code_change
mission_id: 01KYZTQ118MXZGD4MBCR99A978
mission_slug: falsifiable-journey-proof-01KYZTQ1
model: ''
owned_files:
- tests/effects_and_buses.rs
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP02 – Guards assert over the record

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Move the journey guard, the permitted-injection guard, and the occupant-edit criterion from
asserting over the scene's **declaration** to asserting over what WP01 **recorded**. Then prove
each one can fail by breaking the behavior it guards and observing the failure.

Both HIGH findings from the predecessor mission's post-merge review close here. This is the WP
the mission exists for.

## Context: the two defects this WP closes

**DRIFT-1 — the journey proof cannot detect its own loss.** The guard at
`tests/effects_and_buses.rs:161-211` iterates `scene.expected_topology_transitions()` and
asserts over `transition.adjust()` and `transition.support_before()` — the scene's own
declaration of what it *intended*. It never reads what was dispatched. Replacing the runner's
dispatch selection with unconditional direct injection therefore leaves the suite at exit 0,
1 passed, 0 failed, with every checkpoint identity and every recorded artifact byte-identical.

**DRIFT-2 — the edit criterion cannot fail.** The occupant-edit checkpoint asserts `Accepted`,
`audible_on_activated_graph()`, and `active_notes() > 0`. All three are satisfied by the ambient
probe note on the already-sounding chain whether or not the edit dispatches. Remove the edit
entirely and the criterion still passes.

Both are proof defects, not behavior defects: the demo does the right thing on hardware today.
Nothing prevents it silently stopping.

## The governing rule (spec C-005)

> No new guard is accepted until its failure has been observed under a deliberate mutation and
> recorded.

T010 and T011 are not optional cleanup. A guard whose failure has never been observed is exactly
what this mission was chartered to eliminate. If you finish T007–T009 and skip the falsification
subtasks, this WP has reproduced its own charter's defect and must be rejected.

## Constraints that bind this WP

- **At most one direct injection, not exactly one.** The documented rejection is the only
  *permitted* direct injection because the UI cannot express an unknown registry entry. It is
  not a *required* one. `assert_eq!(injected, ["Topology.refused"])` freezes today's limitation
  as a rule and would fail a future scene expressing the rejection by gesture. Use "at most one"
  (research R-002; witness predicate `directInjectionsRecorded: lte 1`).
- **Do not delete the declaration-based assertions.** The focus-verification checks
  (`VerifyPatchFocus` / `VerifyMixerFocus` in `support_before()`) prove the scene *planned* a
  focus-verified journey. The new record proves it *happened*. They are complements. Keep both.
- **Add-only checkpoint identity (spec C-001)**: `FROZEN_TOPOLOGY_IDENTITY_BASELINE` at
  `tests/effects_and_buses.rs:59` (17 entries) stays byte-identical. You are changing what is
  asserted, not which steps exist.

## Subtasks

### T007 — Journey guard asserts over the recorded dispatched kind

**Purpose**: make the guard's subject what happened, not what was declared.

**Steps**:

1. The current guard walks `scene.expected_topology_transitions()`. That is a declaration, and a
   declaration cannot witness its own execution. Restructure so the guard walks the **emitted
   checkpoints** from a completed run and reads WP01's recorded kind.
2. For every occupancy checkpoint whose step declares an adjacent-choice journey, assert the
   recorded kind is the gesture.
3. Keep the existing focus-verification assertion over `support_before()` alongside it, so the
   guard proves both that the journey was planned and that it was taken.
4. The failure message must name the step whose recorded kind was wrong. A guard that fails
   without naming the step costs the next engineer an hour.

**Files**: `tests/effects_and_buses.rs`

**Validation**:
- The assertion reads a checkpoint field, not `transition.adjust()`.
- Failure output names the offending step identifier.

**Edge cases**:
- The run must actually produce checkpoints for the guard to read. If the existing test
  structure only builds the scene without running it, you need the run — check how the emitted-side
  guard at `tests/effects_and_buses.rs:337-347` already obtains its checkpoints and follow that
  path rather than inventing a second one.

### T008 — Permitted-injection guard counts recorded injections `[P]`

**Purpose**: identify the one permitted direct injection by its record, and detect an added one.

**Steps**:

1. Count checkpoints whose recorded kind is the direct action.
2. Assert the count is **at most one** — see the constraint above; do not use `eq 1`.
3. When the count is one, assert it is the documented rejection step and no other. An injection
   that appears somewhere else must fail even though the count is still one.
4. Name the offending step(s) in the failure message.

**Files**: `tests/effects_and_buses.rs`

**Validation**:
- Two direct injections fail the assertion.
- One direct injection at a step other than the documented rejection fails.
- Zero direct injections pass.

### T009 — Replace the vacuous edit criterion `[P]`

**Purpose**: make the occupant-edit criterion falsifiable.

**Steps**:

1. Locate the current assertions for the occupant edit (`Accepted`,
   `audible_on_activated_graph()`, `active_notes() > 0` — around
   `tests/effects_and_buses.rs:351-357`).
2. Add the binding assertion: the recorded scalar **before** and **after** differ.
3. Keep the audibility assertions. They are not wrong — they are insufficient alone. The edit
   must be both audible *and* measurably a change.
4. Decide the comparison semantics deliberately and comment the choice. Exact float inequality is
   correct here: a rounding-equal pair is a genuine no-op edit and must fail. If you introduce an
   epsilon, you have reintroduced a way for a no-op to pass.
5. Assert both fields are `Some` on this step — an absent pair must fail loudly rather than
   silently skipping the comparison. This is the most likely way for this guard to quietly stop
   working.

**Files**: `tests/effects_and_buses.rs`

**Validation**:
- An equal before/after pair fails.
- An absent pair fails (not skips).
- The real run passes.

### T010 — Falsification: revert the dispatch selection

**Purpose**: observe the journey guard failing when the journey is lost. Without this, T007 is an
assertion, not a proof.

**Steps**:

1. Back up the file you are about to mutate:
   `cp src/testing/live_demo_runner.rs /tmp/wp02-runner-backup.rs`
2. Apply the mutation — replace the selection at `src/testing/live_demo_runner.rs:959-964` with:
   ```rust
   let event = AppEvent::from_semantic_action(action.clone());
   ```
   This back-injects every occupancy change and reintroduces the predecessor's defect exactly.
3. Run: `cargo test --test effects_and_buses`
   **Do not pipe to `head` or `tail`** — the pipe reports the pager's exit code, not the test's.
   A "green" recorded that way is unreliable, and this exact mistake has already happened once
   in this mission line.
4. **Observe the failure.** Capture the command, the exit code, and the failing assertion's
   message. If it passes, T007 does not work — go back and fix it, do not record a pass.
5. Restore: `cp /tmp/wp02-runner-backup.rs src/testing/live_demo_runner.rs`
6. Confirm the tree is clean (`git status`) and re-run the suite green.
7. Write the record to
   `kitty-specs/falsifiable-journey-proof-01KYZTQ1/evidence/falsification/guard-journey-dispatch.md`
   with: the exact mutation applied, the command, the observed exit code and failure message, the
   restoration, and the observed pass.

**Files**: `tests/effects_and_buses.rs` (the guard); evidence file is a recorded out-of-map edit
— rationale: "mission falsification record; kitty-specs paths are non-declarable by rule".

**Validation**:
- The record shows a **non-zero** exit under mutation and **zero** after restoration.
- `git status` is clean after restoration — the mutation must not land.

### T011 — Falsification: remove the occupant edit

**Purpose**: observe the edit criterion failing when the edit is gone but the chain still sounds.

**Steps**:

1. Back up `src/testing/live_effects_and_buses_scene.rs`.
2. Mutate so the occupant scalar edit does not dispatch, **while leaving the surrounding chain
   sounding**. That is the whole point: the ambient probe note must still be there, because that
   is what made the old criterion pass vacuously. A mutation that also silences the chain proves
   nothing.
3. Run `cargo test --test effects_and_buses` (no pipe) and observe the failure.
4. Restore, confirm clean, re-run green.
5. Write the record to `evidence/falsification/guard-occupant-scalar.md` in the same form as
   T010.

**Files**: `tests/effects_and_buses.rs`; evidence file as recorded out-of-map edit.

**Validation**:
- The failure is attributable to the missing change, not to missing audio. Confirm the run still
  had a sounding chain under mutation — otherwise you have proven the wrong thing.
- Non-zero under mutation, zero after restoration, tree clean.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch is also the final
merge target. This WP depends on **WP01** — enter the lane the runtime computes from
`lanes.json`; its base carries WP01's commits.

You own only `tests/effects_and_buses.rs`. T010 and T011 mutate files owned by other WPs
**temporarily and restore them** — that is deliberate and permitted, because a falsification that
does not touch the implementation cannot demonstrate anything. The mutation must never land; the
tree must be clean when you finish.

## Test Strategy

This WP is entirely test work. The falsification subtasks are the real proof: they are the only
evidence that T007–T009 do anything. Deterministic only — no hardware here (WP06 owns that).

## Definition of Done

- The journey guard reads the recorded dispatched kind; the declaration-based focus assertions
  remain alongside it.
- The injection guard asserts at most one recorded direct injection **and** that it is the
  documented rejection.
- The edit criterion requires a recorded before ≠ after, fails on an absent pair, and keeps the
  audibility assertions.
- Two falsification records exist under `evidence/falsification/`, each showing an observed
  non-zero exit under mutation and a zero exit after restoration.
- `FROZEN_TOPOLOGY_IDENTITY_BASELINE` unchanged — 17 entries, byte-identical, in order.
- `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
  all exit 0, none piped.
- `git status` clean — no mutation left behind.

## Reviewer Guidance

- **Re-run both falsifications yourself.** Do not accept the records on their face. Apply the
  mutation from `guard-journey-dispatch.md`, run the named command, confirm it fails, restore.
  This mission exists because a claim outran its demonstration; do not let its own closure repeat
  that.
- Confirm the injection assertion is "at most one", not "exactly one". `eq 1` is a defect here,
  not a nitpick — it would fail a future scene that expresses the rejection by gesture.
- Confirm the edit criterion fails on an **absent** scalar pair, not just an equal one. Absent
  silently skipping the comparison is the quiet way this guard dies.
- Confirm T011's mutation left the chain sounding. If it silenced the audio, the failure proves
  the audibility assertions still work — not that the change assertion does.
- Confirm the declaration-based focus assertions were not deleted in the rewrite.
- Confirm `git status` is clean and no mutation landed.
