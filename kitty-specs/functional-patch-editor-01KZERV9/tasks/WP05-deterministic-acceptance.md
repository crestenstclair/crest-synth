---
work_package_id: WP05
title: Deterministic acceptance
dependencies:
- WP04
requirement_refs:
- FR-001
- FR-002
- FR-006
- FR-007
- FR-008
- FR-009
- FR-010
- FR-011
- FR-012
- FR-013
- FR-014
planning_base_branch: feat/functional-patch-editor
merge_target_branch: feat/functional-patch-editor
branch_strategy: Planning artifacts for this mission were generated on feat/functional-patch-editor. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/functional-patch-editor unless the human explicitly redirects the landing branch.
subtasks:
- T029
- T030
- T031
- T032
- T033
- T034
history:
- timestamp: '2026-08-08T22:59:56Z'
  actor: planner
  action: created
  note: Initial work package definition
agent_profile: reviewer-renata
authoritative_surface: tests/
create_intent:
- tests/functional_patch_editor.rs
execution_mode: code_change
mission_slug: functional-patch-editor-01KZERV9
owned_files:
- tests/functional_patch_editor.rs
priority: P1
role: implementer
status: planned
tags: []
tracker_refs: []
---

# WP05 - Deterministic acceptance

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this prompt**, load your assigned agent profile:

```
/ad-hoc-profile-load reviewer-renata
```

This gives you the identity, governance scope, boundaries, and initialization
context for this work. Do not begin implementation until the profile is loaded.

## Objective

Write `tests/functional_patch_editor.rs` — the mission's declared project
validation. It must prove every claim this mission makes, through the production
reducer and render path, and it must prove them non-vacuously: each guard is
demonstrated to fail when its subject is defeated.

This is the deterministic half of the mission's proof. WP06 owns the physical
half.

## Context

**Mission**: functional-patch-editor-01KZERV9
**Priority**: P1
**Dependencies**: WP04 — everything under test must exist.

The crest-spec declares this target as `validation.functional_patch_editor` (a
project check, in `completion.projectChecks`) and the asset
`asset.FunctionalPatchEditorAcceptanceTests` carries the prompts that define what
it must prove. Read the asset's `prompts` list — it is the specification for this
file, and anything you feel is missing from it is a signal the asset is
underspecified, not licence to invent a requirement here.

The test must emit `CREST_ACCEPTANCE functional_patch_editor passed` on stdout
only after every declared check holds, and exit 0.

## Branch Strategy

- **Planning base branch**: `feat/functional-patch-editor`
- **Final merge target**: `feat/functional-patch-editor`
- Execution worktrees are allocated per computed lane (see `lanes.json`).
- Do not create ad-hoc branches outside the lane workflow.

## Subtasks

### T029: Prove on-screen patch selection, refusal at the ends, and focus recovery

**Purpose**: The mission's headline claim. Prove it with a fixture that can
actually falsify it.

**Steps**:
1. Build a fixture installing **more than two** Patches across **both** engines.
   A test that switches once between two Patches of the same capability is
   vacuous — the schemas would agree by accident.
2. Prove a switch reprojects identity, MIDI channel, engine, envelope, capability
   rows, effect slots, and every Utility value from the destination Patch.
3. Prove the generation advances **exactly once** per switch. Assert the delta,
   not merely that it changed.
4. Prove no intermediate projection pairs one Patch's identity with another's
   schema — check every `patchId` within a single projected model agrees.
5. Prove focus recovers against the destination's own schema: switch to a Patch
   declaring fewer rows and assert the recovered path is one that Patch hosts.
6. Prove a request at either end of the installed order is a typed unchanged
   rejection leaving the projection identical.
7. Prove an in-flight structural edit stays correlated to the Patch it started
   on, and that an open subordinate surface is left before the switch.

**Files**:
- `tests/functional_patch_editor.rs` (new, ~260 lines)

**Validation**:
- [ ] The fixture has >2 Patches across both engines
- [ ] Generation delta per switch is asserted as exactly 1
- [ ] The end-of-order refusal leaves state identical, asserted by comparison
      rather than by absence of error

---

### T030: Prove the strip is grouped structure and not a flat control run

**Purpose**: Grouping is the structure the design authority declares. A flat run
would pass a naive "the rows are all there" test.

**Steps**:
1. Drive the production render path with real projected view data.
2. Assert the workspace is produced by the `PatchStrip` composition and that it
   arranges groups: identity header, instrument selector, envelope group, and one
   group per ordered effect slot with nested occupant rows.
3. **Assert the negative**: a flat run of every projected control fails the
   check. Construct that shape and confirm the assertion fires.
4. Assert a group with no view data marks itself unavailable rather than
   vanishing, and that a workspace with no focused Patch marks the strip
   unavailable.

**Files**:
- `tests/functional_patch_editor.rs` (modified, ~150 lines)

**Validation**:
- [ ] The grouped shape is asserted structurally, not by counting rows
- [ ] The flat-run negative fires

---

### T031: Prove the five Utility rows, the single master-gain owner, and MIDI rechannelling

**Purpose**: Four carry-forward closures live here, and the master-gain claim is
the one most likely to be silently wrong.

**Steps**:
1. Assert PATCH Utility resolves exactly five rows in the declared order, each
   with a real typed value, none marked unavailable.
2. Assert the authored hint line is present.
3. **Single owner**: adjust master gain from PATCH, read it from the MIXER
   Inspector, assert equality. Then adjust from MIXER, read from PATCH, assert
   equality. Then assert no second master-gain value exists in `AppState`, the
   state tree, or the parameter snapshot — a structural check, not just two
   equal reads.
4. Assert no projected label on **any** surface equals a serialization key.
   Compare against the serialized leaf-descriptor key set so a future key also
   fails. Confirm the guard fires when a key is deliberately reintroduced.
5. Assert a MIDI input change re-targets which incoming part drives the Patch,
   that the Patch responds on the new channel and not the old, and that identity,
   config, envelope, effects, routing, and graph revision are unchanged.

**Files**:
- `tests/functional_patch_editor.rs` (modified, ~200 lines)

**Validation**:
- [ ] Master-gain equality asserted in both directions
- [ ] The absence of a second owner asserted structurally
- [ ] The label guard fires on a deliberately reintroduced key

---

### T032: Prove per-row actions, requested values, and painted ranges and units

**Purpose**: Three projection claims and one render claim.

**Steps**:
1. Assert each control's `validActions` is computed by the same pure resolver as
   the model-level list and agrees with it exactly at the focused row.
2. Assert `requestedValue` is `Some` exactly while a correlated structural edit
   is in flight and `None` on every settled row — count the settled rows carrying
   one and assert zero.
3. Assert every projected numeric control's range and unit are **rendered** on
   the production render path, not merely projected.
4. Assert the page composes no label and no action label of its own.

**Files**:
- `tests/functional_patch_editor.rs` (modified, ~160 lines)

**Validation**:
- [ ] Per-row/model-level agreement asserted at the focus
- [ ] Settled rows carrying a requested value counted and asserted zero
- [ ] Range and unit asserted through the render path, not the projection

---

### T033: Prove one detail identity serves two subjects and returns to the exact origin

**Purpose**: The polymorphism claim. "Both render" is not the claim; "one surface
identity serves both" is.

**Steps**:
1. Open detail from an instrument row and from an effect-occupant row on the same
   Patch. Assert **one** `SurfaceId` served both.
2. Assert title, accent, sections, controls, ranges, units, and status all come
   from the installed descriptor, with no branch on subject kind.
3. Assert two slots holding the same registry entry produce **distinct** subjects
   (distinct `EffectSlotId`).
4. Assert entry is refused from an empty slot and from a Utility row, as typed
   unchanged rejections.
5. Assert subordinate surfaces do not nest, in both directions.
6. Assert return lands on the **exact** originating row after reprojection, not
   a recomputed surface default. Prove this with an origin that is not the first
   row.
7. Assert a capability-declared read-only section is marked in text or shape, and
   a mid-preparation capability shows its lifecycle status rather than an empty
   section set.

**Files**:
- `tests/functional_patch_editor.rs` (modified, ~200 lines)

**Validation**:
- [ ] `detail_surface_identities == 1` while `detail_subjects_served == 2`
- [ ] Return asserted against a non-first origin row
- [ ] Both nesting directions refused

---

### T034: Prove the voice limit is bounded, seeded, enforced, and falsifiable

**Purpose**: The mission's deepest claim, because it reaches into the callback.

**Steps**:
1. Assert the bound is `1..=64` and that out-of-range construction is rejected,
   not clamped.
2. Assert every Patch is seeded from its active engine's declared ceiling, and
   that a SoundFont Patch and a Braids Patch seed differently.
3. Assert the limit is edited as a `Stepped` control through the canonical
   reducer, honouring the descriptor's fine and coarse steps.
4. Assert it is carried on the parameter snapshot.
5. Assert the callback refuses a note-on beyond the limit, never truncates a
   latched voice, never refuses a note-off or all-notes-off, counts each refusal,
   and allocates and destroys nothing.
6. **Falsify**: with the limit defeated, a fixture that must exceed it reports
   zero refusals and the assertion fires. Run this, observe the failure, restore.
7. Emit `CREST_ACCEPTANCE functional_patch_editor passed` only after every check
   in T029–T034 holds.

**Files**:
- `tests/functional_patch_editor.rs` (modified, ~180 lines)

**Validation**:
- [ ] Both engines' seeding asserted distinctly
- [ ] The note-off exemption asserted explicitly
- [ ] The falsification observed and recorded, not described
- [ ] The marker is emitted last, after all checks

## Definition of Done

- [ ] All six subtasks complete
- [ ] `cargo test --test functional_patch_editor` exits 0 and emits the marker
- [ ] Every guard demonstrated to fail when its subject is defeated
- [ ] The fixture installs >2 Patches across both engines
- [ ] No check asserts only that a structure exists — every one drives production
      seams
- [ ] `spec-kitty crest-spec doctor` still reports the model closed

## Risks & Mitigations

| Risk | Mitigation |
| --- | --- |
| Vacuity — a test that passes without the code it claims to prove | Every subtask carries an explicit falsification step; do not skip them |
| A two-Patch same-engine fixture proving nothing about schema recovery | Fixture requirement is >2 Patches across both engines, asserted |
| Master-gain equality passing via two reads of the same accessor | Also assert the structural absence of a second owner |
| The marker emitted before all checks | Emit it last, from one place |

## Reviewer Guidance

**Verify**:
- The fixture composition — count the Patches and their capabilities
- Each falsification was performed, with observed failure text recorded
- Assertions drive the production reducer and render path, not a parallel one

**Red Flags**:
- `assert!(thing.is_some())` standing in for a behavioural claim
- A falsification recorded as "verified" with no observed failure
- The acceptance marker emitted early or from multiple places
- Any check that would still pass with the feature removed
