---
work_package_id: WP01
title: Scene journey rework
dependencies: []
requirement_refs:
- FR-001
- FR-002
- FR-003
- FR-004
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
- T007
history:
- timestamp: '2026-07-31T20:21:28Z'
  actor: planner
  action: created from IC-01 (DRIFT-6 corrective journey rework)
agent_profile: implementer-ivan
authoritative_surface: src/testing/
create_intent: []
execution_mode: code_change
mission_id: 01KYWVYGQMTRFY314AP78KZJPY
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
model: ''
owned_files:
- src/testing/live_effects_and_buses_scene.rs
- src/bin/crest_synth.rs
- tests/effects_and_buses.rs
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP01 – Scene journey rework

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Make every effect-slot and bus-return occupancy change in the retained Phase 3
scene (`src/testing/live_effects_and_buses_scene.rs`) travel the player's
on-screen journey instead of backstage injection: focus visibly reaches the
PATCH effect-slot row (or MIXER return row), occupancy cycles by the
adjacent-choice gesture, and at least one installed occupant's scalar is
edited audibly from the PATCH page. The single controlled rejection keeps
direct injection with an inline documentation comment. Every pre-existing
checkpoint identity stays byte-identical — the rework is strictly add-only.

This heals DRIFT-6 (HIGH) from the parent mission's post-merge review: the
scene proved every behavior audibly but performed the occupancy journey
backstage, so the phase gate never demonstrated the PATCH view's new
functionality on screen.

## Context

- Today the scene injects `SemanticAction::SetSlotOccupancy` directly (see
  the `occupy` closure near `live_effects_and_buses_scene.rs:267`) and
  injects `SetReturnOccupancy` literals for return changes
  (`:466,483,518,610`). `PatchControlId::EffectSlot` appears nowhere in the
  scene.
- The scene already owns the journey pattern you need: the send-raise support
  script (`raise_sends`, ~line 280) composes
  `LiveTopologySupport::FocusMixerTrack`, `Event(EnterSurface/Navigate)`, and
  `VerifyMixerFocus` steps. Slot/return journeys follow the same shape.
- The reducer-side vocabulary is complete and deterministically proven:
  `PatchControlId::EffectSlot(n)` rows exist with focus paths
  (`src/control/semantic_graphical_view_model.rs:859,1008`), occupancy
  control ids in the PATCH projection
  (`src/control/patch_page_projection.rs:1129`), and the adjacent-choice
  contract matches the engine row (crest-spec
  `acceptance.expandable_effects_and_bus_topology.ordered_patch_effect_chain`).
- Key insight (research.md R-2): the adjacent-choice gesture resolves to the
  same structural intent the scene currently injects — same patch/slot/entry
  values — so the resulting structural transitions and their checkpoint
  identities are unchanged. The journey adds focus/navigation steps and their
  verification checkpoints; it must not rename, remove, or reorder any
  existing identity (spec C-001).
- Crest-spec authority: `requirement.expandable_effects_behavioral_proof`
  (amended) and the `open_effect_registry` acceptance's live-scene step now
  require exactly this behavior. Read them via
  `spec-kitty crest-spec context` before starting.
- Bulk-edit note: this WP does not migrate `post_effects()` callers (none in
  the owned files). All diffs must respect
  `kitty-specs/demo-journey-fidelity-and-hygiene-01KYWVYG/occurrence_map.yaml`.

## Subtasks

### T001 — Slot-row journey support steps (PATCH focus + verify)

**Purpose**: Give the scene a reusable way to walk focus to a specific PATCH
effect-slot row and verify it landed, mirroring the MIXER send-walk pattern.

**Steps**:
1. Study the existing support vocabulary in the scene (the
   `LiveTopologySupport` variants used by `raise_sends`) and how the base
   scene enters/leaves PATCH vs MIXER contexts. Reuse what exists; extend the
   support enum only if a PATCH-side focus/verify variant is genuinely
   missing (e.g., `VerifyPatchFocus { control: PatchControlId }`).
2. Compose the navigation path exactly as a player would: enter the PATCH
   context/surface for the subject Patch, `Navigate` to the target
   `PatchControlId::EffectSlot(n)` row, then verify focus against the
   canonical projection (focusedControlId), not against scene-local state.
3. Keep pacing consistent with the scene's existing dwell discipline so the
   journey is visible on screen (the runner enforces dwell; do not add
   sleeps).

**Files**: `src/testing/live_effects_and_buses_scene.rs`.
**Validation**: deterministic scene run reaches each slot row with a
focus-verified step before any occupancy change (asserted in T006).

### T002 — Adjacent-choice occupancy cycling replaces slot injections

**Purpose**: Every `SetSlotOccupancy` injection for slots becomes an on-screen
adjacent-choice cycle from the focused slot row.

**Steps**:
1. For each existing injected slot change, determine the current occupancy at
   that scene point and the target entry, and compute the number/direction of
   adjacent-choice steps (`Adjust Left/Right` in Edit mode — the same gesture
   contract as the engine row, no wrapping) that reaches the same target.
2. Replace the injection with: T001 journey to the row → enter Edit →
   adjacent steps → confirm → verify the resulting occupancy via the
   canonical projection. The resulting structural intent must be identical to
   what the injection produced (same patch, slot, entry), so the existing
   structural-transition checkpoint identity is untouched.
3. Where the scene previously batch-set several slots, keep the same order of
   resulting transitions so the identity sequence is unchanged.
4. Do NOT touch the controlled-rejection injection (T005 handles it).

**Files**: `src/testing/live_effects_and_buses_scene.rs`.
**Validation**: after this subtask, `SetSlotOccupancy` appears in the scene
only at the controlled rejection; all other slot changes originate from
gesture steps.

### T003 — Audible occupant scalar edit from the PATCH page

**Purpose**: Demonstrate descriptor-driven parameter rows live: one installed
occupant's scalar edited from the PATCH page with an audible checkpoint.

**Steps**:
1. Pick an installed occupant that is sounding at that scene point (the
   subject Patch's configured effect is the natural choice) and one of its
   descriptor scalars.
2. Journey to the occupant's parameter row on the PATCH page (same T001
   pattern), perform an accepted value change, and record a checkpoint that
   requires an audible observation — reuse the scene/runner's established
   scalar-edit checkpoint pattern (NoteOn probe → edit → dwell → NoteOff)
   rather than inventing a new one.
3. This is an ADDED checkpoint with a new identity; it must not displace or
   reorder existing ones.

**Files**: `src/testing/live_effects_and_buses_scene.rs`.
**Validation**: the deterministic run shows the new accepted parameter
checkpoint with its audible predicate; existing checkpoint sequence unchanged.

### T004 — MIXER return-row journeys replace return injections

**Purpose**: Return occupancy changes travel the MIXER return rows on screen.

**Steps**:
1. For each injected `SetReturnOccupancy` (except any inside the controlled
   rejection, which stays — see T005), compose the MIXER journey: focus the
   return row for the target `BusId` (extending the send-walk navigation as
   needed), verify focus, cycle occupancy via the adjacent-choice gesture to
   the same target entry, verify the result.
2. Preserve the scene's existing return-level and routing expectations — the
   crest-spec declares return level is return-owned and survives occupancy
   changes; the existing checkpoints already assert this and must stay
   byte-identical.
3. Note `live_effects_and_buses_scene.rs:533` — the support script that
   restores a route after a checkpoint: keep that restoration behavior
   working with journey-driven steps.

**Files**: `src/testing/live_effects_and_buses_scene.rs`.
**Validation**: `SetReturnOccupancy` appears only where T005 documents it (if
the rejection targets a return) — every other return change is gesture-driven.

### T005 — Documented injection exception at the controlled rejection

**Purpose**: Keep the FR-015/SC-006 (parent) controlled rejection as a direct
injection — the UI cannot request an unknown registry entry by design — and
document that exception inline where it happens.

**Steps**:
1. Locate the rejection block (`live_effects_and_buses_scene.rs:496-520`,
   the `ABSENT_ENTRY_ID` request) and keep the injection mechanics unchanged
   so the rejection checkpoint identity and its visible-reason expectation
   stay byte-identical.
2. Add a comment at the injection site stating the constraint the code
   cannot show: the adjacent-choice gesture can only reach installed entries
   and empty, so an unknown-entry request is inexpressible through the UI;
   direct injection is the sanctioned exception (spec C-003, crest-spec
   `requirement.expandable_effects_behavioral_proof`).
3. Keep the comment factual and durable — no WP numbers, no timeline.

**Files**: `src/testing/live_effects_and_buses_scene.rs`.
**Validation**: comment present at the injection site; rejection still shows
its visible reason; a following valid change still succeeds.

### T006 — Add-only checkpoint-identity assertions in deterministic coverage

**Purpose**: Prove C-001 mechanically: the pre-rework identity set survives
byte-identically and all changes are additions.

**Steps**:
1. In `tests/effects_and_buses.rs`, capture the scene's declared checkpoint /
   transition identity sequence (the identities the report keys evidence by —
   structural transition ids, checkpoint labels/steps as applicable).
2. Assert that the pre-existing identity subsequence (as recorded in the
   parent evidence — enumerate it explicitly in the test as the frozen
   baseline) appears unchanged and in order, with new journey identities as
   pure insertions.
3. Assert the scene contains exactly one direct occupancy injection (the
   documented rejection) — e.g., by inspecting the built plan's steps — so a
   future regression that reintroduces backstage injection fails this test.

**Files**: `tests/effects_and_buses.rs`.
**Validation**: test fails if any baseline identity is renamed, removed, or
reordered, or if a second direct occupancy injection appears.

### T007 — crest_synth demo-observation mirror checks updated

**Purpose**: The standalone binary's demo-observation mirror checks must track
the reworked scene — this exact surface caused the parent mission's only
review rejection (stale mirrors in `src/bin/crest_synth.rs`, fixed at lines
788/1049).

**Steps**:
1. Read the mirror checks in `src/bin/crest_synth.rs` around lines 788 and
   1049 (indexed `/sends` array check, `masterGainDb`-only projection
   comparison) and any expectations keyed to scene step counts or transition
   sets.
2. Update them for the added journey steps/checkpoints without weakening
   them; expectations stay exact, not `>=` sloppiness.
3. Remove any stale WP-numbered handoff comments encountered in the touched
   regions of this file (rewrite genuine constraints in durable language).

**Files**: `src/bin/crest_synth.rs`.
**Validation**: `make demo-live-effects-and-buses`'s deterministic twin and
`cargo test --all-targets` pass; mirror checks assert the new exact shape.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch
is also the final merge target. Execution worktrees are allocated per
computed lane from `lanes.json` during `/spec-kitty.implement` — enter the
lane workspace the runtime names; do not hand-create branches.

## Test Strategy

Deterministic-first: T006's identity assertions and the existing
`tests/effects_and_buses.rs` coverage gate this WP; the physical run is WP11's
job. Run locally: `cargo test --test effects_and_buses`,
`cargo test --release --test expandable_effects_and_bus_topology -- --nocapture`
(must stay green — its observation work belongs to WP09, not this WP), and
`cargo test --all-targets` before review.

## Definition of Done

- Zero slot/return occupancy changes originate from injection except the one
  documented rejection (T005).
- Focus-verified journey precedes every occupancy change; one occupant scalar
  edit from PATCH carries an audible checkpoint.
- Pre-existing checkpoint identity set byte-identical (T006 proves it).
- `src/bin/crest_synth.rs` mirrors exact; full suite green.
- All diffs comply with `occurrence_map.yaml`; no new dependencies.

## Reviewer Guidance

- Diff the scene: search for `SetSlotOccupancy`/`SetReturnOccupancy` — every
  remaining use must be inside the documented rejection block.
- Verify T006 is non-vacuous: temporarily rename one baseline identity
  locally and confirm the test fails (do not commit that).
- Check the rejection comment states the UI-inexpressibility rationale, not a
  timeline.
- Confirm no reducer/projection/production code changed in this WP — scene,
  binary mirrors, and its test only.
