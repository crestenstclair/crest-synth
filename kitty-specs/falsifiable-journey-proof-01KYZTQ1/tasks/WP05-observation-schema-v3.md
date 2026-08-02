---
work_package_id: WP05
title: Observation schema v3
dependencies:
- WP01
- WP03
requirement_refs:
- FR-001
- FR-004
- FR-007
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T019
- T020
- T021
- T022
history:
- timestamp: '2026-08-02T00:10:35Z'
  actor: planner
  action: created from IC-06 (measured surface for the declared predicates)
agent_profile: implementer-ivan
authoritative_surface: tests/expandable_effects_and_bus_topology.rs
create_intent: []
execution_mode: code_change
mission_id: 01KYZTQ118MXZGD4MBCR99A978
mission_slug: falsifiable-journey-proof-01KYZTQ1
model: ''
owned_files:
- tests/expandable_effects_and_bus_topology.rs
- src/testing/live_demo_report.rs
priority: P2
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP05 – Observation schema v3

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Emit the six observations the crest-spec's witness now declares, so its predicates have measured
values to read rather than declared ones, and surface the new checkpoint fields on the live
report preserving absent-vs-zero.

## Context

Crest-spec commit `ad9960b` moved `witness.expandable_effects_and_bus_topology` to
`schemaVersion: 3` and added six observations with predicates:

| Field | Predicate | Meaning |
|---|---|---|
| `occupancyStepsDeclaringJourney` | `gt 0` | there is a journey at all |
| `occupancyStepsNotGradedOnRecordedDispatch` | `eq 0` | every journey step was graded on the record |
| `directInjectionsRecorded` | `lte 1` | at most one direct injection — **not** exactly one |
| `occupantScalarEditsExercised` | `gt 0` | an edit happened |
| `occupantScalarEditsWithoutRecordedChange` | `eq 0` | every edit measurably changed something |
| `mismatchedSlotIdentityInexpressible` | `eq true` | WP03's guarantee holds |

The "not graded" / "without change" fields are phrased as counts-that-must-be-zero deliberately:
the predicate language compares a field to a literal, not to another field, so a relational
assertion has to be expressed as a difference that must vanish.

## Constraints that bind this WP

- **`schemaVersion` moves 2 → 3.** Every artifact quoting version 2 moves with it. Grep before
  you finish.
- **`retiredGraphsCollectedOffCallback` is 15**, not the parent mission's 8. The predecessor's
  WP09 drove seven more structural changes. Do not copy a stale number forward from an older
  artifact; the predicate is `gt 0` so it still passes either way, which is exactly why a wrong
  value would go unnoticed.
- **`directInjectionsRecorded` is `lte 1`.** If you find yourself asserting `eq 1`, stop: the
  documented rejection is the only *permitted* direct injection, not a required one (research
  R-002).
- **Absent is not zero (spec NFR-005).** The report must distinguish an unmeasured field from a
  measured zero. A defaulted zero here regresses a contract the predecessor mission established.
- **Do not touch `tests/effects_and_buses.rs`** — WP02 owns it.

## Subtasks

### T019 — Emit the six observations

**Purpose**: compute each field from the recorded checkpoints WP01 and WP03 produce.

**Steps**:

1. Find where the existing `CREST_EFFECTS_AND_BUSES_OBSERVATION` payload is assembled in
   `tests/expandable_effects_and_bus_topology.rs` and extend it. Follow the existing fields'
   style rather than introducing a parallel mechanism.
2. Compute each from the emitted checkpoints, not from the scene declaration:
   - `occupancyStepsDeclaringJourney` — occupancy steps whose declaration carries an
     adjacent-choice direction. This one legitimately reads the declaration: it is the
     denominator, the count of steps that *should* have travelled the journey.
   - `occupancyStepsNotGradedOnRecordedDispatch` — of those, how many have a recorded dispatched
     kind that is **not** the gesture. Must be 0.
   - `directInjectionsRecorded` — checkpoints whose recorded kind is the direct action.
   - `occupantScalarEditsExercised` — checkpoints with both scalar fields `Some`.
   - `occupantScalarEditsWithoutRecordedChange` — of those, how many have before == after.
     Must be 0.
   - `mismatchedSlotIdentityInexpressible` — exercise WP03's guarantee: attempt to install an
     occupant carrying another position's identity and confirm the stored occupant carries the
     position's own. Report `true` only if actually exercised; never hardcode it.
3. The last one is the only field that requires driving a case rather than counting a run. Keep
   that case small and local.

**Files**: `tests/expandable_effects_and_bus_topology.rs`

**Validation**:
- Every field is computed, none hardcoded. Grep for literal `true` / `0` on the right-hand side
  of these assignments — a hardcoded observation is the same class of defect this mission exists
  to close.
- The counts come from emitted checkpoints, except the declaring-journey denominator.

### T020 — Move schemaVersion 2 → 3

**Purpose**: keep the emitted schema and the declared predicate in agreement.

**Steps**:

1. Bump the emitted `schemaVersion` to 3.
2. Grep the whole repo for `schemaVersion` and for the literal `2` in observation contexts —
   including test assertions, fixtures, and any planning artifact that quotes the version — and
   move each together.
3. The declared predicate (`.kittify/crest-spec/proof/witnesses.yaml`) already expects 3; do not
   edit the crest-spec from this phase. If the predicate and your emission disagree, the emission
   is wrong.

**Files**: `tests/expandable_effects_and_bus_topology.rs`

**Validation**:
- `rg 'schemaVersion' --type rust` shows no remaining `2` on this observation path.
- The release run's payload reports 3.

### T021 — Surface the new fields on the live report `[P]`

**Purpose**: make the new checkpoint fields visible in the recorded evidence WP06 will capture.

**Steps**:

1. `src/testing/live_demo_report.rs` assembles the control-side report. Add the new checkpoint
   fields to what it surfaces, following how existing checkpoint fields are carried through.
2. Preserve absent-vs-zero end to end: a step with no occupant scalar must serialize absent in
   the report, exactly as it does on the checkpoint. The predecessor mission's WP06 established
   this contract for measurement fields — reread how it distinguishes the two and match it.
3. Do not add a new report-level aggregate unless a declared predicate needs it. The witness
   observation (T019) is the aggregate surface; the report carries per-checkpoint detail.

**Files**: `src/testing/live_demo_report.rs`

**Validation**:
- A non-editing step's report entry shows absent, not `0.0`.
- The report's existing fields and their names are unchanged (spec C-001, C-003).

### T022 — Confirm every declared predicate passes on measured values

**Purpose**: close the loop between what is declared and what is emitted.

**Steps**:

1. Run the release target the witness declares:
   ```
   cargo test --release --test expandable_effects_and_bus_topology -- --nocapture
   ```
   **Do not pipe to `head`/`tail`.** Redirect to a file if you need to read it:
   `... > /tmp/wp05-witness.log 2>&1` — redirection preserves the exit code, a pipe does not.
2. Read the emitted `CREST_EFFECTS_AND_BUSES_OBSERVATION` payload and check each of the six new
   fields against its declared predicate, plus the pre-existing ones.
3. Confirm `retiredGraphsCollectedOffCallback` reports its real current value (expected 15) and
   that you did not carry a stale 8 into any artifact.
4. Run the declared negative case and confirm it still fails as designed:
   ```
   cargo run --quiet --bin crest-synth-witness -- --case refused-topology --mutant refused-topology-published
   ```
   Expected exit code: 1.

**Files**: none (verification only)

**Validation**:
- All 6 new predicates satisfied on measured values; positive case exit 0, negative case exit 1.
- Record the observed values in the WP activity log so WP06 does not have to re-derive them.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch is also the final
merge target. This WP depends on **WP01 and WP03** — enter the lane the runtime computes from
`lanes.json`; its base carries both.

You own `tests/expandable_effects_and_bus_topology.rs` and `src/testing/live_demo_report.rs`.
WP01 owns the other three `src/testing/` modules — if you need a checkpoint accessor that does
not exist, that is a WP01 gap to raise, not a file to edit. A cross-lane edit is invisible to
both reviewers; that is how the predecessor mission's merged build broke.

## Test Strategy

Deterministic release run plus the declared negative case. No hardware — WP06 owns that.

## Definition of Done

- All six observations are computed from emitted checkpoints (except the declaring-journey
  denominator) and none is hardcoded.
- `schemaVersion` is 3 everywhere on this path; no stale 2 remains.
- The report surfaces the new fields preserving absent-vs-zero.
- The release positive case exits 0 with every declared predicate satisfied; the negative case
  exits 1.
- `retiredGraphsCollectedOffCallback` reports its real value and no stale 8 was propagated.
- `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
  all exit 0, none piped.

## Reviewer Guidance

- **Grep for hardcoded observations.** A field assigned a literal instead of a computed value is
  the exact defect class this mission closes — check `mismatchedSlotIdentityInexpressible`
  especially, since it is the one that is easiest to fake.
- Confirm `directInjectionsRecorded` is not asserted as `eq 1` anywhere.
- Confirm `occupancyStepsNotGradedOnRecordedDispatch` counts from the recorded kind, not from the
  declaration — if it reads `transition.adjust()` on both sides it always reports 0 and proves
  nothing.
- Confirm absent-vs-zero survives into the report for a non-editing step.
- Re-run the release target yourself and read the payload; do not accept quoted values.
- Confirm `tests/effects_and_buses.rs` and the WP01-owned modules are untouched.
