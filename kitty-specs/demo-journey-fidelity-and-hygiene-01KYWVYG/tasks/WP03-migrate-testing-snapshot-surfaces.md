---
work_package_id: WP03
title: 'Migration: testing & snapshot surfaces'
dependencies: []
requirement_refs:
- FR-007
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
base_branch: kitty/mission-demo-journey-fidelity-and-hygiene-01KYWVYG
base_commit: 213052d6ee27b913f4250423c90ebb3f20a178e4
created_at: '2026-07-31T20:52:35.755230+00:00'
subtasks:
- T012
- T013
- T014
- T015
- T016
history:
- timestamp: '2026-07-31T20:21:28Z'
  actor: planner
  action: created from IC-02 (DRIFT-1 compact-view retirement, testing/snapshot slice)
agent_profile: implementer-ivan
authoritative_surface: src/testing/
create_intent: []
execution_mode: code_change
mission_id: 01KYWVYGQMTRFY314AP78KZJPY
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
model: ''
owned_files:
- src/real_time/parameter_snapshot.rs
- src/testing/demo_scene.rs
- src/testing/exhaustive_gui_demo.rs
- src/testing/live_demo_runner.rs
- tests/static_patch_effect.rs
- tests/live_demo_scene.rs
priority: P2
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP03 – Migration: testing & snapshot surfaces

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Migrate the `post_effects()` call sites in the widened parameter snapshot,
the retained deterministic scenes/runner, and the two caller test targets to
the canonical per-position `effect_slots()` view. Second slice of the
compact-view retirement (WP05 deletes the accessor after all slices land).

## Context

- Same ground rules as WP02 (read its Context section): semantic migration,
  per-position handling, frozen serialized/observation vocabulary, accessor
  survives until WP05, occurrence map governs every diff.
- `src/real_time/parameter_snapshot.rs` is RT-adjacent but the accessor use
  is prepare-time/composition-side; the snapshot's own postEffects section is
  ALREADY per-position ("MAX_EFFECT_SLOTS ordered entries per Patch …
  clearing one slot leaves the other positions stable rather than compacted"
  — crest-spec RT invariant). The compacted accessor is used to *feed* it;
  feeding from the per-position view removes an impedance mismatch, not adds
  one.
- The deterministic scenes (`demo_scene.rs`, `exhaustive_gui_demo.rs`) and
  `live_demo_runner.rs` consume Patch effect config for expected-parameter
  derivation; the crest-spec requires those expectations to be derived from
  production resolvers/descriptors — keep that derivation, only change the
  chain view it reads.

## Subtasks

### T012 — Migrate parameter_snapshot call sites

**Steps**:
1. `grep -n "post_effects()" src/real_time/parameter_snapshot.rs`; rewrite
   each against `effect_slots()`, mapping true positions straight onto the
   snapshot's per-position postEffects section (no intermediate compaction,
   no re-indexing).
2. Confirm the leaf descriptor enumeration and scalar layouts are untouched
   (frozen vocabulary); only the source-view plumbing changes.
3. Callback-safety: verify the touched code paths are prepare/publish-side;
   do not alter anything the callback executes (C-004).

**Validation**: snapshot tests green; a gapped chain feeds a snapshot whose
occupied entries sit at their true slot positions.

### T013 — Migrate demo_scene + exhaustive_gui_demo

**Steps**:
1. Rewrite each call site to derive expected editable-parameter sets and
   effect coverage from the per-position view; unconfigured positions
   contribute nothing (as today), configured positions contribute at their
   true index.
2. The scenes' expected-set freezing discipline (expectations frozen before
   dispatch) must be preserved exactly.

**Validation**: `make demo` deterministic scene and exhaustive GUI demo tests
green with unchanged coverage sets for the production fixture.

### T014 — Migrate live_demo_runner

**Steps**:
1. Rewrite its call sites (expected-parameter derivation for the live plan)
   per the same rules.
2. No change to runner pacing, checkpoint construction, or report fields —
   this WP is view-plumbing only (WP01 owns scene behavior; WP06 owns report
   semantics).

**Validation**: `tests/live_demo_scene.rs` (after T015) and the full suite
green.

### T015 — Migrate tests/static_patch_effect.rs + tests/live_demo_scene.rs

**Steps**:
1. Rewrite test-side call sites with per-slot assertions: where a test
   asserted against a compacted list, assert position-explicitly (index +
   occupancy) instead.
2. Do not weaken any assertion; the migration must keep every proof exactly
   as strong (DIRECTIVE_041 — tests fail exactly when the contract is
   violated).

**Validation**: both targets green; assertions are position-explicit.

### T016 — Stale WP-comment cleanup (owned testing/snapshot files)

**Steps**:
1. Remove stale WP-numbered handoff comments in the five owned source files
   (counts at planning time: demo_scene 4, live_demo_runner 3,
   exhaustive_gui_demo 1, parameter_snapshot 1); rewrite genuine constraints
   durably, delete timeline narration.

**Validation**: grep for `WP0`/`WP10` in owned files returns nothing.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch
is also the final merge target. Execution worktrees are allocated per
computed lane from `lanes.json` during `/spec-kitty.implement`.

## Test Strategy

`cargo test --all-targets`; targeted: `cargo test --test static_patch_effect`,
`cargo test --test live_demo_scene`, snapshot module tests. Assert at least
one gapped-chain path through the snapshot feed.

## Definition of Done

- `grep -n "post_effects()"` over all six owned files → no output.
- Snapshot feeding is position-direct; expectations/coverage sets unchanged
  for the production fixture; per-slot assertions in the two test targets.
- No stale WP comments in owned files; occurrence-map compliant.

## Reviewer Guidance

- Look for local re-compaction (dense collect + positional indexing) — the
  anti-pattern this retirement exists to kill.
- T012 is the highest-risk site: check the snapshot feed maps true positions
  and that nothing destructor-bearing or allocating moved toward the
  callback.
- Confirm test assertions got stronger (position-explicit), never looser.
