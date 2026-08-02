---
work_package_id: WP08
title: Per-position capability identity
dependencies: []
requirement_refs:
- FR-007
- FR-016
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
base_branch: kitty/mission-demo-journey-fidelity-and-hygiene-01KYWVYG
base_commit: 213052d6ee27b913f4250423c90ebb3f20a178e4
created_at: '2026-07-31T20:53:04.874067+00:00'
subtasks:
- T030
- T031
- T032
- T033
- T034
history:
- timestamp: '2026-07-31T20:21:28Z'
  actor: planner
  action: created from IC-09 (RISK-1 hardening, operator-included) plus the worker migration slice of IC-02
agent_profile: implementer-ivan
authoritative_surface: src/real_time/
create_intent: []
execution_mode: code_change
mission_id: 01KYWVYGQMTRFY314AP78KZJPY
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
model: ''
owned_files:
- src/real_time/graph_preparation_worker.rs
- src/real_time/prepared_graph.rs
- src/real_time/prepared_graph_builder.rs
- src/real_time/prepared_engine_rack.rs
- src/real_time/prepared_post_effect_rack.rs
- src/real_time/prepared_bus_return_rack.rs
priority: P3
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP08 – Per-position capability identity

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Record the engine/effect capability identity of every prepared position in
the prepared-graph layout, and make all three racks' live-instance carry-over
guards require exact per-position identity agreement in addition to today's
patch/slot/scalar-layout checks — a mismatch keeps the freshly prepared
instance. Also migrate this WP's owned `post_effects()` call sites (the
worker's three) as part of the same slice. Operator-included hardening
(decision DM-01KYWWM9BXV6CRCYXHDEHY9VTJ) closing the parent review's RISK-1.

## Context

- Today the carry-over guards check patch_id/slot_id/scalar_count
  (`prepared_engine_rack.rs:187-209`, `prepared_post_effect_rack.rs:222-256`,
  `prepared_bus_return_rack.rs:173-195`) and fail safe. A same-scalar-count
  WRONG-capability candidate at a non-selected position is indistinguishable
  — exploitable only via an upstream preparer/coordinator bug, which is
  exactly what defense-in-depth is for.
- Crest-spec authority (commit `0328311`, `aggregate.RealTime.PreparedGraph`
  invariant): "the prepared layout records the engine or effect capability
  identity of every prepared position; carrying a live instance into a
  replacement requires exact per-position capability identity agreement in
  addition to patch, slot, and scalar-layout agreement, and any mismatch
  keeps the freshly prepared instance instead of the carried one."
- Declared proof: attached validation `service.carry_over_capability_identity`
  (selector `carry_over_capability_identity`); the release target's witness
  field `carryOverWrongEngineIdentityRefused` (WP09 measures it through the
  integration target — your mechanism must make that measurable).
- Hard boundary (C-004): all of this is prepare-time/control-side. The
  callback's contract — no allocation, locking, blocking, destruction — is
  untouched; identity comparison happens where carry-over decisions are made
  today (graph assembly), never per-block.

## Subtasks

### T030 — Migrate graph_preparation_worker call sites

**Steps**:
1. Rewrite the three `post_effects()` sites
   (`graph_preparation_worker.rs:262,343,465`) against the per-position
   `effect_slots()` view — position-direct, no local compaction (same rules
   as WP02/WP03; occurrence map governs).
2. Remove the stale WP comment in this file (planning-time count: 1).

**Validation**: `grep -n "post_effects()" src/real_time/graph_preparation_worker.rs`
→ no output; worker tests green.

### T031 — Prepared layout records per-position capability identity

**Steps**:
1. Extend the layout the racks attest against (`prepared_graph.rs` /
   `prepared_graph_builder.rs` — the `PreparedGraphLayout` surface) so every
   prepared position (engine per Patch, effect per Patch-slot, occupant per
   return) carries its `CapabilityId`/`EffectCapabilityId`.
2. Populate it at build time from the validated candidate configuration (the
   builder already knows each position's capability — record it, don't
   re-derive it later).
3. Keep the layout fixed-size/preallocated per existing conventions — no
   heap growth on any path the callback touches.

**Validation**: builder tests green; layout carries identity for every
occupied position and an explicit empty for unoccupied ones.

### T032 — Engine-rack carry-over identity guard

**Steps**:
1. In `prepared_engine_rack.rs:187-209`, add exact capability-identity
   agreement to the carry-over predicate; on mismatch, keep the freshly
   prepared instance (existing fail-safe direction — extend it, don't
   restructure it).

**Validation**: unit case — same patch/slot/scalar-count, different
capability id → carry-over refused, fresh instance kept, no panic, no
callback-path change.

### T033 — Post-effect + bus-return rack identity guards

**Steps**:
1. Apply the same predicate extension to
   `prepared_post_effect_rack.rs:222-256` and
   `prepared_bus_return_rack.rs:173-195`.
2. Remove stale WP comments in the three rack files and `prepared_graph.rs`
   (planning-time counts: post_effect 4, bus_return 3, engine 1, graph 1);
   rewrite genuine constraints durably.

**Validation**: identical refusal semantics across all three racks; grep for
`WP0`/`WP10` in owned files → nothing.

### T034 — carry_over_capability_identity tests + comment cleanup

**Steps**:
1. Add unit/module tests named so
   `cargo test carry_over_capability_identity` runs them (the declared
   attached-validation selector): for each rack, a candidate agreeing on
   patch/slot/scalar layout but mismatching recorded identity is refused and
   the fresh instance is kept; an agreeing candidate still carries over
   (don't break the WP10-of-parent held-notes behavior).
2. Cover the layout side: builder records the right identity per position
   for a mixed configuration (two engines, duplicate effect entries in two
   slots, occupied + empty returns).

**Validation**: selector matches; refusal and agreement cases both pinned;
`cargo test --all-targets` green.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch
is also the final merge target. Execution worktrees are allocated per
computed lane from `lanes.json` during `/spec-kitty.implement`.

## Test Strategy

Unit-first per rack (T032-T034), then the full suite. WP09 later surfaces
the refusal through the release behavioral target — keep your mechanism
reachable from production types (no test-only backdoor seams).

## Definition of Done

- Layout records identity per position; all three racks refuse on identity
  mismatch and keep the fresh instance; agreement still carries over.
- Worker migrated off `post_effects()`; no stale WP comments in owned files.
- `cargo test carry_over_capability_identity` green; full suite green;
  callback contract untouched (C-004).

## Reviewer Guidance

- Check the comparison happens at carry-over decision time (prepare-side),
  not per render block.
- Verify fail-safe direction: mismatch → fresh instance (never bypass, never
  panic, never partial adoption).
- Confirm the held-note carry-over contract still holds for agreeing
  candidates (run `cargo test --test topology_change_lifecycle`).
- Look for allocation or destructor movement toward the callback — zero
  tolerance (C-004).
