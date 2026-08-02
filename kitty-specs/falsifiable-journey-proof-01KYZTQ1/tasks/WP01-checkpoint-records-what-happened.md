---
work_package_id: WP01
title: Checkpoint records what actually happened
dependencies: []
requirement_refs:
- FR-001
- FR-004
- NFR-005
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T001
- T002
- T003
- T004
- T005
- T006
history:
- timestamp: '2026-08-02T00:10:35Z'
  actor: planner
  action: created from IC-01 and IC-03 (record the dispatched input and the scalar change)
agent_profile: implementer-ivan
authoritative_surface: src/testing/
create_intent: []
execution_mode: code_change
mission_id: 01KYZTQ118MXZGD4MBCR99A978
mission_slug: falsifiable-journey-proof-01KYZTQ1
model: ''
owned_files:
- src/testing/live_demo_checkpoint.rs
- src/testing/live_demo_runner.rs
- src/testing/live_effects_and_buses_scene.rs
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP01 – Checkpoint records what actually happened

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Make `LiveTopologyCheckpoint` carry two things it does not carry today: the kind of input the
production reducer **actually received** for each step, and — for the step that edits an
occupant scalar — the scalar's projected value **before dispatch and after acceptance**.

This WP adds no guard and changes no behavior. It creates the record that WP02's guards will
assert over. Nothing else in the mission is possible until the record exists.

## Context: why this WP exists

The predecessor mission reworked the live effects-and-buses demo so occupancy changes travel
the player's on-screen journey. That behavior is correct on hardware today. But it was proven
by executing the defect that its guards cannot detect:

> Replacing the dispatch selection at `src/testing/live_demo_runner.rs:959-964` with an
> unconditional `AppEvent::from_semantic_action(action.clone())` — which back-injects every
> occupancy change and reintroduces the exact defect the predecessor was chartered to close —
> leaves `cargo test --test effects_and_buses` at **exit 0, 1 passed, 0 failed**, and leaves
> every checkpoint identity, the live report, and the recorded hardware evidence
> **byte-identical**.

The cause is structural: `LiveTopologyCheckpoint` (`src/testing/live_demo_checkpoint.rs:336`)
records `action: Option<SemanticAction>` — the *declared expected result* — and has no field for
the `AppEvent` actually dispatched. The dispatched event is a local, consumed at dispatch.

The same shape defeats the parameter-edit criterion: acceptance, `audible_on_activated_graph()`,
and `active_notes() > 0` are all satisfied by the ambient probe note on the already-sounding
chain whether or not the edit dispatches at all.

Crest-spec (commit `ad9960b`) now declares both fields and their invariants on
`valueObject.Testing.LiveDemoCheckpoint`. This WP realizes that declaration.

## Constraints that bind this WP

- **Add-only checkpoint identity (spec C-001)**: existing checkpoint identities stay
  byte-identical and in order. `FROZEN_TOPOLOGY_IDENTITY_BASELINE`
  (`tests/effects_and_buses.rs:59`, 17 entries) is the guard. New fields are additions; do not
  rename, reorder, or repurpose an existing serialized key.
- **Real-time discipline (spec NFR-001, C-002)**: both new fields are control-side only. Nothing
  added here may appear in any structure the audio callback can reach. The checkpoint is already
  declared to contain "no device, callback, engine, mixer, window, or mutable-state handle" —
  keep it that way.
- **Absent is not zero (spec NFR-005)**: the scalar fields are `Option`-shaped. A step that
  edits no occupant scalar records absent, never a defaulted `0.0`. This is a contract the
  predecessor mission established; regressing it is a defect.
- **No behavior change**: dispatch selection logic is untouched. You are observing it, not
  altering it. If threading the value seems to require changing what gets dispatched, stop and
  raise it — that is a finding, not a refactor.

## Subtasks

### T001 — Add the dispatched-input-kind value type

**Purpose**: give the checkpoint a typed discriminator for what the reducer received, distinct
from the declared expected outcome it already stores.

**Steps**:

1. Define the type next to the checkpoint in `src/testing/live_demo_checkpoint.rs`. Two variants
   suffice for what the runner can dispatch on this path:
   - the adjacent-choice gesture on a focused row (the journey path);
   - a direct semantic action (the injection path).
2. Derive what the surrounding types derive. Check `LiveTopologyCheckpoint`'s own derives and
   match them; the type must be `Copy`-able and serializable alongside its owner.
3. Serialize it under a new key. Do not overload or reinterpret the existing `action` key —
   `action` keeps its current name, meaning, and emitted values.
4. Name the key so a reader can tell it apart from `action` at a glance in the live log. It
   records *how the change was made*, not *what was requested*.

**Files**: `src/testing/live_demo_checkpoint.rs`

**Validation**:
- The type has exactly the variants the runner can produce — no speculative third variant for an
  input kind nothing dispatches.
- Serializing a checkpoint emits the new key alongside, not instead of, `action`.
- `cargo test --all-targets` still compiles and passes (nothing reads the field yet).

### T002 — Capture the dispatched kind and thread it to every checkpoint

**Purpose**: carry the truth from the point of dispatch to the point of recording.

**Steps**:

1. At `src/testing/live_demo_runner.rs:959-964` the selection already exists:
   ```rust
   let event = match transition.adjust() {
       Some(direction) if transition.expected_rejection().is_none() => {
           AppEvent::Adjust(direction)
       }
       _ => AppEvent::from_semantic_action(action.clone()),
   };
   ```
   Derive the kind from **which arm produced the event**, at the moment it is produced. Do not
   re-derive it later from `transition.adjust()` — that would reintroduce exactly the
   declaration-reading defect this mission exists to close, and the crest-spec invariant forbids
   it explicitly.
2. Add the field to `TopologyContext` (`src/testing/live_demo_runner.rs:1718`). It already
   carries nine per-transition values through the phase machine; this is the tenth.
3. `TopologyContext` flows through the `LiveTopologyPhase` variants (`:1730` onward:
   `AwaitActivating`, `AwaitReady`, `AwaitAudible`, `AwaitScalarAudible`, and the rejection
   path). Confirm the value survives every transition between phases — a phase that rebuilds the
   context with `..context` will carry it, but one that constructs a fresh `TopologyContext` will
   silently drop it.
4. Pass it into all **three** `LiveTopologyCheckpoint::new` call sites (approximately `:1117`,
   `:1170`, `:1224`). The constructor takes positional arguments; adding a parameter is a
   compile-time break at every site, which is the desired loud failure.
5. The rejection path dispatches through `dispatch_rejected_topology_event`. Its checkpoint must
   record the direct-action kind — the rejection is the one documented direct injection, and
   WP02 identifies it *by this record*.

**Files**: `src/testing/live_demo_runner.rs`, `src/testing/live_demo_checkpoint.rs`

**Validation**:
- Grep the runner: the kind is assigned inside the `match` that selects the event, and nowhere
  else. There must be exactly one place that decides it.
- Every `LiveTopologyCheckpoint::new` site passes a value derived from an actual dispatch.
- A journey-driven step and the rejection step produce different recorded kinds.
- `cargo test --all-targets` passes.

**Edge cases**:
- A phase that constructs a fresh `TopologyContext` rather than updating one drops the field
  silently. Search for every `TopologyContext {` literal and confirm each either sets the field
  or spreads an existing context.
- A transition with no action produces no checkpoint on this path — leave it alone.

### T003 — Add the occupant-scalar before/after fields

**Purpose**: give the parameter-edit criterion something that can fail.

**Steps**:

1. Add two `Option`-shaped fields to `LiveTopologyCheckpoint` for the edited occupant scalar's
   value before dispatch and after acceptance.
2. `None` means "this step edits no occupant scalar" — it must never be `Some(0.0)` standing in
   for an unmeasured value (spec NFR-005).
3. Serialize both under new keys, additive to the existing schema.
4. Add an accessor pair so WP02 can read them without reaching into private state, matching how
   the checkpoint's other fields are exposed (see `audible_on_activated_graph`).

**Files**: `src/testing/live_demo_checkpoint.rs`

**Validation**:
- A non-editing step serializes both keys as absent, not as zero.
- The accessors return `Option`, preserving the distinction to the caller.

### T004 — Read the scalar from the canonical projection

**Purpose**: measure the value from the production projection, not from the scene's intent.

**Steps**:

1. The scalar-edit path already has its own phase: `LiveTopologyPhase::AwaitScalarAudible`
   (`src/testing/live_demo_runner.rs:1738`). That is where this measurement belongs.
2. Read the **before** value from the canonical projection immediately prior to dispatching the
   edit — the same projection the checkpoint's `projectedValue` already draws from. Do not read
   the scene's declared target value; the point is to measure, not to restate the plan.
3. Read the **after** value from the projection once the edit is accepted, at the same point the
   existing checkpoint fields are copied from the production `EventRecord`.
4. Store both on `TopologyContext` and copy them onto the checkpoint alongside T002's field.

**Files**: `src/testing/live_demo_runner.rs`

**Validation**:
- Both readings come from the canonical projection. Grep for the scene's declared value near
  this code — if it appears, you are recording intent, not measurement.
- On the editing step, both are `Some`.
- On every other topology step, both are `None`.

**Edge cases**:
- If the edit is dispatched but **rejected**, before and after are equal. Record them anyway —
  WP02's criterion is "they differ", so an equal pair correctly fails. Do not special-case
  rejection into absent.
- A rounding-equal float pair is a real no-op edit and must record as equal. Do not fuzz the
  comparison here; WP02 decides the comparison semantics.

### T005 — Declare which scene step edits an occupant scalar

**Purpose**: let the runner know which step to measure, from the scene's declaration.

**Steps**:

1. The scene already declares the edit at `src/testing/live_effects_and_buses_scene.rs:510`
   (`"SlotOccupant.scalarEdited"`). Confirm how that step is currently expressed and what it
   carries.
2. Ensure the step declares enough for the runner to identify **which** occupant scalar to read
   — the position and the parameter — so T004 measures the right value rather than guessing.
3. Do not add a new checkpoint identity here. `SlotOccupant.scalarEdited` already exists in the
   frozen baseline; this WP enriches what that step records, it does not rename or duplicate it.

**Files**: `src/testing/live_effects_and_buses_scene.rs`

**Validation**:
- `FROZEN_TOPOLOGY_IDENTITY_BASELINE` (`tests/effects_and_buses.rs:59`) still matches exactly —
  17 entries, unchanged, in order. If this test fails, you have broken spec C-001.
- The declared step carries the position and parameter the runner needs.

### T006 — In-module assertions over the new fields

**Purpose**: prove the record is correct before any guard depends on it.

**Steps**:

1. Add assertions in the existing `#[cfg(test)]` module in `src/testing/live_demo_checkpoint.rs`
   (or the runner's, whichever owns the construction path you changed):
   - a checkpoint built from a gesture dispatch records the gesture kind;
   - a checkpoint built from a direct action records the direct kind;
   - a non-editing step records both scalar fields absent;
   - an editing step records both `Some` with differing values.
2. Assert on **serialized output** for at least one case, so an accidental key rename or a
   dropped field is caught here rather than at the physical run.

**Files**: `src/testing/live_demo_checkpoint.rs` (or `src/testing/live_demo_runner.rs`)

**Validation**:
- `cargo test --all-targets` — 0 failures.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` both exit 0.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch is also the final
merge target. This WP has no dependencies — enter the lane the runtime computes from
`lanes.json`.

WP02 and WP05 both depend on this WP. Their reviewers cannot see your lane and you cannot see
theirs; keep the public surface you add (field names, accessor names, the kind type's variants)
stable once landed, because a later rename breaks a sibling lane invisibly. That exact failure —
a sibling lane deleting an API another lane called — broke the predecessor mission's merged
build.

## Test Strategy

Deterministic only. This WP touches no hardware. `cargo test --all-targets` must be green, and
the frozen identity baseline test is the binding check that spec C-001 held.

## Definition of Done

- The dispatched input kind is recorded at the point of dispatch and reaches all three
  checkpoint construction sites, including the rejection path.
- The occupant scalar before/after pair is measured from the canonical projection on the editing
  step and absent everywhere else.
- `FROZEN_TOPOLOGY_IDENTITY_BASELINE` matches byte-identically — 17 entries, unchanged, in order.
- In-module assertions cover both kinds and both scalar states, with at least one serialized-output
  assertion.
- `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
  all exit 0. Never pipe these through `head`/`tail` — the pipe masks the exit status.

## Reviewer Guidance

- **The central check**: grep the runner for every place the dispatched kind is assigned. There
  must be exactly one, inside the `match` that selects the event. If it is computed anywhere
  from `transition.adjust()` or any other scene declaration, the WP has reproduced the defect it
  was written to fix — reject it.
- Confirm no `TopologyContext` construction site drops the new field.
- Confirm the scalar readings come from the canonical projection, not from the scene's declared
  target value.
- Confirm absent-vs-zero survives serialization for a non-editing step.
- Confirm nothing added is reachable from the audio callback.
- Verify the identity baseline test actually ran and passed — this is the spec C-001 gate.
