---
work_package_id: WP09
title: Fourth-entry fixture & observation schema v2
dependencies:
- WP08
requirement_refs:
- FR-015
- FR-016
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T035
- T036
- T037
- T038
history:
- timestamp: '2026-07-31T20:21:28Z'
  actor: planner
  action: created from IC-08 (SC-008 PARTIAL -> demonstration, operator-included)
agent_profile: implementer-ivan
authoritative_surface: tests/
create_intent: []
execution_mode: code_change
mission_id: 01KYWVYGQMTRFY314AP78KZJPY
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
model: ''
owned_files:
- tests/expandable_effects_and_bus_topology.rs
priority: P3
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP09 – Fourth-entry fixture & observation schema v2

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Convert the parent mission's SC-008 structural inference into a
demonstration: the declared release behavioral target registers a FOURTH
test-registry effect entry and drives it through slot occupancy, return
occupancy, preparation, projection, and render as a full citizen with zero
structural changes. Bump the emitted observation to `schemaVersion: 2` with
two new measured fields: `fourthEntryEndToEndExercised` and
`carryOverWrongEngineIdentityRefused` (mechanism from WP08). Operator-included
(decision DM-01KYWWM8M963DE79XAZE7EZXC9).

## Context

- The parent review graded SC-008 PARTIAL: openness was proven by structural
  absence (leaf-schema scan + name-enumeration guard + occupant-generic
  projection), not by an executable end-to-end registration. The crest-spec
  `open_effect_registry` acceptance now names the executable fixture, and
  `witness.expandable_effects_and_bus_topology` declares schema v2 with both
  new predicates `eq true` and `schemaVersion eq 2` (commit `0328311`).
- This target already emits `CREST_EFFECTS_AND_BUSES_OBSERVATION` and runs as
  both the witness positive command
  (`cargo test --release --test expandable_effects_and_bus_topology`) and a
  declared exact-selector validation — every existing field and predicate
  must stay green (add fields, never repurpose).
- The fourth entry lives in a TEST registry inside this target (the immutable
  production registry is not touched — `nonGoals` still exclude new product
  effects). Zero structural changes means: no edits outside this test file
  are needed to make the fourth entry a full citizen; if you find yourself
  needing one, that is a FINDING against openness — stop and report it, do
  not patch production code from here.

## Subtasks

### T035 — Test-registry fourth-entry end-to-end fixture

**Steps**:
1. Extend the target's test registry with a fourth descriptor-driven entry
   (distinct id, its own scalar descriptors; simple DSP is fine — the proof
   is structural citizenship, not sonic character).
2. Drive it end to end through production services: resolve it into a Patch
   slot AND into a bus return; prepare via the production
   builder/coordinator path; verify projection exposes it generically (rows
   derive from descriptors, no role marker); render and measure a finite
   nonzero wet/processing consequence attributable to it in both roles.
3. Assert zero structural change: the fixture uses only the existing public
   surfaces (registry construction + the same calls the other three entries
   use).

**Validation**: fourth entry behaves identically to the built-ins in every
stage; test green.

### T036 — Observation schemaVersion 2 + fourthEntryEndToEndExercised

**Steps**:
1. Bump the emitted observation's `schemaVersion` to `2`.
2. Add `fourthEntryEndToEndExercised: bool`, measured from T035's actual
   drive-through (set true only when slot, return, preparation, projection,
   and render stages each contributed real evidence — no hardcoded true).

**Validation**: emitted JSON carries schemaVersion 2 and the field; the
declared witness predicates (`schemaVersion eq 2`,
`fourthEntryEndToEndExercised eq true`) pass.

### T037 — carryOverWrongEngineIdentityRefused measured in the target

**Read first — seam guidance from WP08's reviewer (2026-07-31).** Refusal is
observable at two distinct layers, and you must be explicit about which one
your witness field measures:

- **(a) Admission-time**: `StructuralGraphCoordinator::stage_replacement` →
  `GraphPublicationFailure::IncompatibleLayout`. A discrete, observable
  signal.
- **(b) Activation-time**: the rack guards inside `carry_live_state_from`.
  The guard `continue`s silently and emits no status, so refusal is
  observable ONLY as "the fresh instance is still there" — an audio/sample
  level inference. **There is no counter or status field for a refused
  carry-over.** If you need a discrete signal at this layer, that plumbing
  does not exist and you must not fabricate it in the test — either measure
  (a), or measure (b) by its sample-level consequence, and say which.

Do not conflate either with a third, unrelated path:
`GraphPreparationError::UnrecordableCapabilityIdentity` →
`EngineSelectionFailure::InvalidDefaultConfig` (surfaced as
`"invalidDefaultConfig"`), which is about an unrecordable identity, not a
refused carry-over.

WP08's handoff note names the single intended read seam (layout accessors vs
rack accessors — it was asked to pick one). Use that seam; if both still
exist when you start, report it rather than choosing arbitrarily.

**Steps**:
1. Using WP08's mechanism through production types, stage a carry-over
   scenario where a candidate agrees on patch/slot/scalar layout but
   mismatches the recorded per-position capability identity; measure that
   the carry-over is refused and the fresh instance is kept.
2. Emit the measured result as `carryOverWrongEngineIdentityRefused: bool` —
   measured, not asserted-by-construction. State in a comment which layer
   (a or b) the measurement witnesses.

**Validation**: field emitted true from a real refusal; predicate passes;
the negative witness case (`--case refused-topology --mutant
refused-topology-published`) still exits 1.

### T038 — Two-run determinism and existing predicates stay green

**Steps**:
1. Re-run the target twice and confirm the two-run logical determinism field
   still holds with the new fixture in the plan.
2. Verify every pre-existing observation field is still measured and every
   declared predicate passes (`spec-kitty crest-spec context` lists them;
   the witness runs healthy exit 0 / mutant exit 1).
3. Confirm `orderedSlotCasesExercised`, isolation dBFS, rejection fields
   etc. did not silently change meaning from plan reshuffling.

**Validation**:
`cargo test --release --test expandable_effects_and_bus_topology -- --nocapture`
exit 0 with all predicates true, twice.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch
is also the final merge target. This WP depends on WP08 — the runtime's
computed lane carries the dependency commits per `lanes.json`.

## Test Strategy

The target is the proof surface. Gate:
`cargo test --release --test expandable_effects_and_bus_topology -- --nocapture`
(twice, for determinism), plus the witness negative case unchanged.

## Definition of Done

- Fourth entry drives slot + return + preparation + projection + render with
  zero non-test structural change.
- Observation schemaVersion 2 with both new fields measured true; all
  existing predicates green; two-run determinism holds.
- No production file edited from this WP.

## Reviewer Guidance

- Partial coverage is the failure mode that made SC-008 PARTIAL — verify all
  five stages contribute real evidence to `fourthEntryEndToEndExercised`.
- Both new fields must be measured from actual behavior; grep for hardcoded
  `true` literals feeding them.
- Confirm zero production-code diffs in this WP and that the negative
  witness case still fails correctly (exit 1).
