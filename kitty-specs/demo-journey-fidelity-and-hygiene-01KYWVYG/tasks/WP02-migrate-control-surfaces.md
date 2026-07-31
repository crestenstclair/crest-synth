---
work_package_id: WP02
title: 'Migration: control surfaces'
dependencies: []
requirement_refs:
- FR-007
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T008
- T009
- T010
- T011
history:
- timestamp: '2026-07-31T20:21:28Z'
  actor: planner
  action: created from IC-02 (DRIFT-1 compact-view retirement, control slice)
agent_profile: implementer-ivan
authoritative_surface: src/control/
create_intent: []
execution_mode: code_change
mission_id: 01KYWVYGQMTRFY314AP78KZJPY
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
model: ''
owned_files:
- src/control/app_state.rs
- src/control/semantic_resolver.rs
- src/control/semantic_graphical_view_model.rs
- src/control/patch_page_projection.rs
- src/control/event_record.rs
- src/control/serialized_state.rs
priority: P2
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP02 – Migration: control surfaces

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Migrate every `post_effects()` call site in the six owned `src/control/`
files to the canonical never-compacted `effect_slots()` view, preserving gaps
per-position everywhere. This is one slice of retiring the transitional
compact view (parent review DRIFT-1); WP05 deletes the accessor once all
slices land.

## Context

- The Patch aggregate exposes two representations today: canonical
  `effect_slots()` (per-position, gapped) and transitional `post_effects()`
  (compacted, position-erasing — `src/synth/patch.rs:84-90`). The crest-spec
  now declares the per-position view as the ONLY permitted representation
  (`aggregate.Synth.Patch` invariant, commit `0328311`).
- This migration is semantic, not mechanical (occurrence map: code_symbols =
  manual_review): the compacted `Vec` shape becomes the per-slot
  `[Option<…>]`-style shape, and each caller must handle empty positions
  explicitly instead of receiving a squeezed list.
- **Frozen vocabulary**: serialized/projected key names (`postEffects` leaves,
  StateTree paths, projection labels) are `do_not_change` — the add-only
  checkpoint constraint depends on them. You are changing how values are
  *obtained and shaped in code*, never what they are *called* in serialized
  or projected output. If a byte of serialized output would change, stop and
  re-read `occurrence_map.yaml`.
- The accessor itself stays alive until WP05; your slice must compile and
  pass the full suite standalone.

## Subtasks

### T008 — Migrate app_state + semantic_resolver call sites

**Steps**:
1. `grep -n "post_effects()" src/control/app_state.rs src/control/semantic_resolver.rs`
   and study each site's intent: iteration over configured effects, occupancy
   lookups, focus resolution over slot rows.
2. Rewrite each against `effect_slots()`: iterate positions (index +
   `Option`), skipping empty slots explicitly where the old code relied on
   compaction; where the old code used positional indices into the compacted
   list, re-derive against true `EffectSlotIndex` positions.
3. Watch focus semantics in the resolver: `PatchControlId::EffectSlot(_)`
   rows exist per-position whether occupied or not
   (`semantic_resolver.rs:54` region) — occupancy checks must consult the
   slot's `Option`, not list length.

**Validation**: `cargo test --all-targets` green; behavior identical for
non-gapped chains (the production fixture); gapped chains resolve
per-position.

### T009 — Migrate semantic_graphical_view_model + patch_page_projection

**Steps**:
1. Same per-site treatment. In the PATCH projection, slot rows are already
   per-position (`patch_page_projection.rs:1129` builds
   `occupancy_control_id: PatchControlId::EffectSlot(slot_index)`) — the
   compacted accessor is typically used for derived summaries/parameter rows;
   rebuild those from the per-position view without changing any projected
   label or ordering.
2. Projected output must stay byte-identical for every state reachable today
   (assert via existing projection tests); gapped chains must project the
   occupied slots at their true positions.

**Validation**: projection tests green with zero expectation edits (any
needed expectation edit is a red flag — justify or revert).

### T010 — Migrate event_record + serialized_state

**Steps**:
1. Rewrite each call site against the per-position view. In serialization
   paths the emitted keys, structure, and ordering are frozen — derive the
   same output from `effect_slots()`.
2. If a site cannot produce identical serialized output from the
   per-position view for currently-reachable states, STOP and record the
   discrepancy in the review notes instead of adapting the output format.

**Validation**: serialization round-trip tests green; StateTree leaf
enumeration unchanged.

### T011 — Per-slot assertions + stale WP-comment cleanup (control files)

**Steps**:
1. Add module-test coverage in the owned files for the gapped case: slot 0
   empty + slot 1 occupied flows through resolution/projection/serialization
   with the gap intact (this is the shape the deleted accessor used to
   silently squeeze).
2. Remove stale WP-numbered handoff comments in the owned files (one is at
   `src/control/semantic_graphical_view_model.rs`); rewrite any genuine
   constraint in durable language, delete pure timeline narration.

**Validation**: new gapped-case tests fail if someone re-compacts; grep for
`WP0`/`WP10` in owned files returns nothing.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch
is also the final merge target. Execution worktrees are allocated per
computed lane from `lanes.json` during `/spec-kitty.implement`.

## Test Strategy

`cargo test --all-targets` plus the targeted projection/serialization suites.
The gapped-case tests added in T011 are the regression teeth for this slice.

## Definition of Done

- `grep -rn "post_effects()" src/control/` → no output.
- Serialized and projected output byte-identical for currently-reachable
  states; gapped chains handled per-position.
- Gapped-case module tests present and non-vacuous.
- No stale WP comments in owned files; diffs comply with
  `occurrence_map.yaml`.

## Reviewer Guidance

- The one thing that must NOT happen here: a caller re-implementing
  compaction locally (`.filter_map(...).collect()` then indexing by dense
  position). Look for it explicitly.
- Any change to a projection/serialization expectation file is suspect —
  demand a written justification.
- Verify the gapped tests actually exercise a gap (slot 0 empty, later slot
  occupied), not just an all-empty or all-full chain.
