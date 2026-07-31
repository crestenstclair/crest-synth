---
work_package_id: WP05
title: Retire the compact view
dependencies:
- WP02
- WP03
- WP04
- WP08
requirement_refs:
- FR-007
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T021
- T022
- T023
history:
- timestamp: '2026-07-31T20:21:28Z'
  actor: planner
  action: created from IC-02 (DRIFT-1 endgame — accessor deletion)
agent_profile: implementer-ivan
authoritative_surface: src/synth/
create_intent: []
execution_mode: code_change
mission_id: 01KYWVYGQMTRFY314AP78KZJPY
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
model: ''
owned_files:
- src/synth/patch.rs
priority: P2
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP05 – Retire the compact view

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Delete the transitional compacting surface from the Patch aggregate —
`post_effects()` and `with_post_effects()` (`src/synth/patch.rs:84-90`
region) — so exactly one chain representation exists: the ordered,
per-position, never-compacted `effect_slots()` view. This is the enforcement
step of the DRIFT-1 retirement; it lands only after WP02, WP03, WP04, and
WP08 have migrated every caller.

## Context

- Crest-spec authority (commit `0328311`, `aggregate.Synth.Patch`): "the
  ordered per-position slot view is the only chain representation the
  aggregate exposes; no compacting or position-erasing accessor exists, and
  every consumer receives positions exactly as stored so a gapped chain can
  never be silently renumbered by a round-trip."
- The parent mission's own doc comment deferred this retirement to
  "WP05/WP06" — both shipped without it (the review's headline OWNERSHIP/SEAM
  drift). Deletion, not deprecation, is the decision of record
  (research.md R-4).
- Dependencies WP02/WP03/WP04/WP08 own all 15 caller files. If you find a
  surviving caller outside your owned file, STOP and report it against the
  owning WP — do not migrate it here (ownership discipline).

## Subtasks

### T021 — Delete post_effects()/with_post_effects() from Patch

**Steps**:
1. `grep -rn "post_effects()\|with_post_effects(" src/ tests/ --include="*.rs"`
   — expect matches only in `src/synth/patch.rs` (definitions and any
   internal uses). Anything else: stop, report to the owning WP.
2. Delete both the accessor and the constructor, plus any internal helpers
   that exist only to serve compaction. Replace internal uses with the
   per-position view.
3. Replace the transitional doc comment block (`patch.rs:84-90`) with durable
   documentation of the single-view contract, phrased as the invariant (no WP
   numbers, no timeline).

**Validation**: workspace compiles; repo-wide grep for both names returns
zero matches.

### T022 — Patch unit tests: gapped stability, no compacting surface

**Steps**:
1. In `patch.rs` module tests, prove the canonical view's contract directly:
   configure slot 1 (slot 0 empty), assert `effect_slots()` reports slot 0
   empty and slot 1 occupied; clear a middle slot in a fuller chain and
   assert the neighbors' positions and instance identities are untouched.
2. Assert unique stable `EffectSlotId`s survive occupancy changes at other
   positions (identity stability across the exact operations the compacted
   view used to blur).

**Validation**: tests fail if compaction or renumbering is ever
reintroduced.

### T023 — Repo-wide zero-caller verification + patch.rs comment cleanup

**Steps**:
1. Re-run the repo-wide grep from T021 as a final check and paste the (empty)
   result into the WP completion notes.
2. Remove the remaining stale WP-numbered comment in `patch.rs`
   (planning-time count: 1) and any timeline narration in the touched
   regions.
3. Run the full suite and the release behavioral target to confirm the
   deletion changed no behavior:
   `cargo test --all-targets && cargo test --release --test expandable_effects_and_bus_topology`.

**Validation**: suite green; zero grep matches; no stale WP comments in
`patch.rs`.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch
is also the final merge target. This WP depends on WP02/WP03/WP04/WP08 —
implement it from the lane the runtime computes (its lane branch will carry
the dependency commits per `lanes.json`).

## Test Strategy

The suite is the gate: deletion is behavior-neutral by construction once all
callers are migrated, so a green `cargo test --all-targets` plus the new
gapped-stability unit tests complete the proof.

## Definition of Done

- `post_effects()` and `with_post_effects()` no longer exist anywhere in the
  repository.
- Patch module tests pin gapped stability and identity stability.
- Durable single-view documentation in place; no stale WP comments.
- Full suite and release behavioral target green.

## Reviewer Guidance

- The deletion must be total — a `#[allow(dead_code)]` survivor or a renamed
  compact helper fails the crest-spec invariant.
- Check T022's tests assert positions AND EffectSlotId identity, not just
  occupancy counts.
- Confirm the new doc comment states the contract without narrating mission
  history.
