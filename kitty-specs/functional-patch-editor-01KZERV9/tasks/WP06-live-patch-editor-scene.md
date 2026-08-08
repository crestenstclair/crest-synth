---
work_package_id: WP06
title: The live scene, its target, and the roadmap close
dependencies:
- WP04
requirement_refs:
- FR-016
- NFR-002
- NFR-004
planning_base_branch: feat/functional-patch-editor
merge_target_branch: feat/functional-patch-editor
branch_strategy: Planning artifacts live on feat/functional-patch-editor; completed work merges into feat/functional-patch-editor. Execution worktrees are allocated per computed lane.
subtasks:
- T035
- T036
- T037
- T038
- T039
- T040
- T041
history:
- timestamp: '2026-08-08T22:59:56Z'
  actor: planner
  action: created
  note: Initial work package definition
agent_profile: implementer-ivan
authoritative_surface: src/testing/
create_intent:
- src/testing/live_patch_editor_scene.rs
- src/testing/functional_patch_editor_observation.rs
execution_mode: code_change
mission_slug: functional-patch-editor-01KZERV9
owned_files:
- src/testing/**
- src/bin/crest_synth.rs
- Makefile
- ROADMAP.md
priority: P1
role: implementer
status: planned
tags: []
tracker_refs: []
---

# WP06 - The live scene, its target, and the roadmap close

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this prompt**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

This gives you the identity, governance scope, boundaries, and initialization
context for this work. Do not begin implementation until the profile is loaded.

## Objective

Close LIMIT-1. `make demo-live-patch-editor` must navigate from one Patch to
another **through the `SelectPatch` gesture on screen**, then perform a full
focus-verified effect-slot occupancy journey and an audible occupant parameter
edit on the **second** instrument — with checkpoints correlating the patch
switch, the resulting focus, and the audible consequence.

This is the mission's declared exit gate. Phase 3 proved the effect-slot journey
for `patches.first()` only, and it was pinned there because the vocabulary had no
patch-switching gesture. The gesture landed 2026-08-02. The demonstration is
this package's to deliver.

## Context

**Mission**: functional-patch-editor-01KZERV9
**Priority**: P1
**Dependencies**: WP04. Runs in parallel with WP05.

The crest-spec declares `witness.functional_patch_editor` with a 44-field
observation schema and 44 predicates, and
`valueObject.Testing.FunctionalPatchEditorObservation` with the measurement
rules. Read both before writing code — the observation type must mirror the
witness schema exactly, because the witness is what acceptance runs.

`valueObject.Testing.LiveDemoScene` now carries invariants specific to this
plan: it is **not** pinned to `patches.first()`, its effect-slot journey visits
every declared position on the second Patch, and its parameter edit measures the
audible delta on the second Patch's own output while requiring the first Patch's
output to be unchanged.

**The gated live suite requires a display seating 1920×1080.** The harness
refuses rather than degrading. The external LS28AG700N must be awake to run it.

## Branch Strategy

- **Planning base branch**: `feat/functional-patch-editor`
- **Final merge target**: `feat/functional-patch-editor`
- Execution worktrees are allocated per computed lane (see `lanes.json`).
- Do not create ad-hoc branches outside the lane workflow.

## Subtasks

### T035: Declare `FunctionalPatchEditorObservation` and its measurement rules

**Purpose**: The structured result acceptance reads. It must mirror the declared
witness schema exactly.

**Steps**:
1. Create `src/testing/functional_patch_editor_observation.rs` with every field
   the witness declares, in the declared camelCase-on-the-wire form.
2. Every field is **measured** from the production reducer, projector, snapshot
   transport, prepared renderer, audio observation, webview window, and physical
   stream — never copied from expected data.
3. Implement the declared measurement rules:
   - `patchesFocused` counts **distinct** PatchIds that actually held focus, so a
     scene that switched and switched back credits one reach, not two.
   - `detailSurfaceIdentities` counts distinct surface identities that served a
     subject; two subjects served by one identity is the claim.
   - The audible-edit deltas are measured on **each Patch's own output**.
4. Emit the marker `CREST_FUNCTIONAL_PATCH_EDITOR_LIVE_OBSERVATION ` only after
   semantic cleanup, stream release, worker shutdown, graph collection, window
   close, and successful parent-process return.

**Files**:
- `src/testing/functional_patch_editor_observation.rs` (new, ~230 lines)
- `src/testing/mod.rs` (modified, ~2 lines)

**Validation**:
- [ ] Every witness schema field is present with a matching wire name
- [ ] `patchesFocused` counts distinct ids — assert with a switch-and-return
- [ ] No field is populated from an expected value

---

### T036: Build the live scene plan whose subject is the second Patch

**Purpose**: A plan pinned to `patches.first()` is exactly what LIMIT-1 rejects.

**Steps**:
1. Create `src/testing/live_patch_editor_scene.rs`.
2. The plan selects a second Patch through `SemanticAction::SelectPatch` — the
   production gesture, dispatched through `AppLoop`, translated from physical
   input. Never a direct state poke.
3. After the switch, derive every editable target, the effect-slot journey, and
   the audible probes from **that second Patch's own descriptors**. Do not carry
   the first Patch's target list across.
4. The effect-slot journey visits **every** declared position on the second
   Patch, setting and clearing occupancy through the correlated structural
   lifecycle, checkpointing focus before and after each transition.
5. The parameter edit is on an **occupant** of an occupied slot on the second
   Patch, bracketed by a bounded semantic NoteOn/NoteOff probe for exact-generation
   audible observation, exactly as the existing scenes do.
6. Include the end-of-order refusal: request a switch past the last Patch and
   assert the typed unchanged rejection.
7. Prove the voice limit bites during the run so `voiceLimitRefusals > 0`.

**Files**:
- `src/testing/live_patch_editor_scene.rs` (new, ~420 lines)

**Validation**:
- [ ] The scene's subject after the switch is the second Patch, asserted by id
- [ ] Targets derive from the second Patch's descriptors, not a carried list
- [ ] All three slot positions are visited on the second Patch
- [ ] The switch travels the production input → action → reducer path

---

### T037: Emit checkpoints correlating the switch, the focus, and the audible edit

**Purpose**: The gate's exact words: "Checkpoints must correlate the patch
switch, the resulting focus, and the audible consequence."

**Steps**:
1. Emit a checkpoint at the switch carrying the destination PatchId, the
   resulting `FocusPath`, and the generation.
2. Emit a checkpoint at each occupancy transition carrying the focus path before
   and after and the graph revision.
3. Emit a checkpoint at the parameter edit carrying the parameter identity, the
   value change, the second Patch's measured output delta, **and** the first
   Patch's measured delta.
4. Populate `checkpointsCorrelatingSwitchFocusAudio` from checkpoints that
   actually carry all three facts — not from a count of checkpoints emitted.
5. Assert `firstPatchAudibleEditDelta == 0` while
   `secondPatchAudibleEditDelta > 0`. An edit that moved both signals proves
   nothing about reach and must be credited to neither.

**Files**:
- `src/testing/live_patch_editor_scene.rs` (modified, ~140 lines)
- `src/testing/live_demo_checkpoint.rs` (modified, ~40 lines)

**Validation**:
- [ ] Each correlating checkpoint carries switch, focus, and audio together
- [ ] The first Patch's delta is asserted at zero
- [ ] The counter counts correlating checkpoints, not all checkpoints

---

### T038: Add the `--demo-live-patch-editor` mode to the binary

**Purpose**: A stable entry point on the production composition path.

**Steps**:
1. In `src/bin/crest_synth.rs`, accept `--demo-live-patch-editor` by itself,
   following the existing `--demo-live-*` mode contracts: real window, physical
   audio device, autonomous run, mapped semantic window input ignored so it
   cannot interleave a generation, final report emitted once, clean teardown,
   successful return.
2. Print the existing concise pre-start status explaining the run is autonomous.
3. It is **additive**: it is not an alias target and does not subsume the
   cumulative effects-and-buses scene. `demo-live` keeps pointing at
   `demo-live-effects-and-buses`.

**Files**:
- `src/bin/crest_synth.rs` (modified, ~110 lines)

**Validation**:
- [ ] The mode runs standalone and refuses to combine with incompatible flags
- [ ] Teardown releases the stream, closes the window, collects owned graphs
- [ ] `demo-live` still resolves to the effects-and-buses scene

---

### T039: Add the `--defeat-patch-selection` controlled negative

**Purpose**: Without a working negative, the whole demonstration is
unfalsifiable — it could pass by a route that never left the first instrument.

**Steps**:
1. Accept `--defeat-patch-selection` **only** alongside
   `--demo-live-patch-editor`, and only as the declared controlled negative.
2. Under it, defeat the patch switch — the scene's own predicates must then fail
   and the process must exit **1**.
3. The negative must fail for the right reason. It should trip
   `patchesFocused`, `secondPatchIdDistinct`, and the audible-delta predicates —
   not merely error out early on an unrelated path. Record which predicates fired.

**Files**:
- `src/bin/crest_synth.rs` (modified, ~60 lines)
- `src/testing/live_patch_editor_scene.rs` (modified, ~40 lines)

**Validation**:
- [ ] The negative exits 1
- [ ] It fails on the reach predicates, and the failing set is recorded
- [ ] The flag is rejected outside `--demo-live-patch-editor`

---

### T040: Add the `make demo-live-patch-editor` target

**Purpose**: The stable human command the witness runs.

**Steps**:
1. Add `demo-live-patch-editor` to the Makefile:
   `cargo run --release --bin crest-synth -- --demo-live-patch-editor`.
2. Give it a one-line `##` description and declare it phony.
3. Leave `demo-live` aliased to `demo-live-effects-and-buses` — this scene is
   additive.
4. While in this file, correct the stale alias documentation if the Makefile's
   own comments still describe `demo-live` as the sixteen-track routing scene;
   the crest-spec asset was corrected during authoring and the two should agree.

**Files**:
- `Makefile` (modified, ~8 lines)

**Validation**:
- [ ] `make -n demo-live-patch-editor` succeeds
- [ ] `make help` lists it with its description
- [ ] `demo-live` is unchanged

---

### T041: Record the Phase 5 completion note and LIMIT-1 closure in ROADMAP.md

**Purpose**: The roadmap is the delivery record. LIMIT-1's second bullet has been
open since Phase 3; close it against real evidence.

**Steps**:
1. **Run this subtask last**, after the live evidence exists. A completion note
   written ahead of the evidence is a claim, not a record.
2. Strike through the second LIMIT-1 bullet in the Phase 5 entry condition and
   mark it closed with the date, exactly as the first bullet was closed on
   2026-08-02.
3. Add a Phase 5 completion note in the style of Phase 4's: what closes it, and
   what carries forward. Carry forward the items `spec.md`'s Scope Decisions table
   defers, with their owners — do not silently drop them.
4. Report the measured evidence: the observation's actual field values, not a
   summary adjective.
5. If any predicate was graded rather than met, say so plainly with the number.

**Files**:
- `ROADMAP.md` (modified, ~60 lines)

**Validation**:
- [ ] The LIMIT-1 bullet is struck and dated
- [ ] The completion note reports measured values
- [ ] Deferred carry-forwards are listed with owners
- [ ] Nothing is claimed that the evidence does not show

## Definition of Done

- [ ] All seven subtasks complete
- [ ] `make demo-live-patch-editor` exits 0 with a complete report on a real
      window with physical audio
- [ ] Every witness predicate holds, or any shortfall is reported with its number
- [ ] `--defeat-patch-selection` exits 1 on the reach predicates
- [ ] The scene's subject after the switch is the second Patch, and the audible
      edit moved only that Patch's output
- [ ] ROADMAP.md records the closure against measured evidence

## Risks & Mitigations

| Risk | Mitigation |
| --- | --- |
| The rig has no 1920×1080 display and the harness refuses | Wake the external LS28AG700N before running; the refusal is correct behaviour, not a bug to work around |
| An edit that moves both Patches' output | Assert the first Patch's delta at zero; credit neither if both moved |
| The negative failing early for an unrelated reason | Record which predicates fired; it must fail on reach |
| A completion note written ahead of the evidence | T041 runs last, by instruction |

## Reviewer Guidance

**Verify**:
- The switch travels the production input → semantic action → reducer path, not
  a direct state mutation
- Post-switch targets derive from the second Patch's descriptors
- `firstPatchAudibleEditDelta` is asserted at zero, not merely reported
- The negative's failing predicate set is recorded and is about reach

**Red Flags**:
- `patches.first()` anywhere in the scene's subject resolution
- A checkpoint counter incremented per checkpoint rather than per correlating one
- `demo-live` re-aliased to this scene
- A roadmap completion note with adjectives where numbers belong
