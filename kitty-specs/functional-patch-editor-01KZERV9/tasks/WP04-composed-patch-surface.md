---
work_package_id: WP04
title: The composed PATCH surface in the webview page
dependencies:
- WP03
requirement_refs:
- FR-001
- FR-003
- FR-004
- FR-005
- FR-012
- FR-013
- FR-015
- NFR-003
planning_base_branch: feat/functional-patch-editor
merge_target_branch: feat/functional-patch-editor
branch_strategy: Planning artifacts for this mission were generated on feat/functional-patch-editor. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/functional-patch-editor unless the human explicitly redirects the landing branch.
subtasks:
- T020
- T021
- T022
- T023
- T024
- T025
- T026
- T027
- T028
history:
- timestamp: '2026-08-08T22:59:56Z'
  actor: planner
  action: created
  note: Initial work package definition
agent_profile: frontend-freddy
authoritative_surface: webview-page/
create_intent: []
execution_mode: code_change
mission_slug: functional-patch-editor-01KZERV9
owned_files:
- webview-page/**
- src/shell/component_vocabulary.rs
priority: P1
role: implementer
status: planned
tags: []
tracker_refs: []
---

# WP04 - The composed PATCH surface in the webview page

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this prompt**, load your assigned agent profile:

```
/ad-hoc-profile-load frontend-freddy
```

This gives you the identity, governance scope, boundaries, and initialization
context for this work. Do not begin implementation until the profile is loaded.

## Objective

Turn the PATCH main workspace from a flat run of every projected control into the
authored grouped strip: an identity-and-routing header, the instrument selector,
a titled envelope group, and one titled group per ordered effect slot with that
slot's occupant parameters nested beneath it. Paint every row's range, unit,
own valid actions, and mid-edit requested value. Seat the five-row Utility panel.
Render the polymorphic detail shell.

This is the package that makes the mission visible.

## Context

**Mission**: functional-patch-editor-01KZERV9
**Priority**: P1
**Dependencies**: WP03 — the projection must carry per-row actions, requested
values, and the detail surface before the page can paint them.

Today `webview-page/page.js` renders the PATCH workspace as `.prow` rows in
document order under a single `.strip` container. The crest-spec now declares
`PatchStrip` and `CapabilityDetailShell` as members of `ShellComposition`, with
the grouping rule, the titling rule, the no-placeholder rule at both levels, and
the Utility seating rule.

**Nine subtasks is above the usual target.** They are kept in one package because
`page.js` and `page.css` are a single coupled surface — splitting them across
packages would produce a merge conflict rather than parallelism. Work through
them in order and commit incrementally.

## Branch Strategy

- **Planning base branch**: `feat/functional-patch-editor`
- **Final merge target**: `feat/functional-patch-editor`
- Execution worktrees are allocated per computed lane (see `lanes.json`).
- Do not create ad-hoc branches outside the lane workflow.

## Subtasks

### T020: Render the PATCH main workspace as the grouped `PatchStrip` composition

**Purpose**: Establish the group anatomy. Everything after this fills it in.

**Steps**:
1. In `page.js`, replace the flat `.strip` row run with a group-arranging
   renderer: the workspace holds an ordered list of groups, each group holds an
   optional title and its rows.
2. In `page.css`, declare the group anatomy — group container, group title band,
   the rows within. Resolve every colour, type style, spacing step, and geometry
   value from the generated token table. Declare no raw value the vocabulary
   already names; where the vocabulary genuinely lacks a value, record the gap in
   a comment the way the existing file already does, rather than inventing a
   literal silently.
3. Carry the no-placeholder rule at both levels: a group whose designed structure
   has no view data marks that structure unavailable **inside its own group**, and
   a workspace with no focused Patch at all marks the strip unavailable.
4. Update `src/shell/component_vocabulary.rs` so `PatchStrip` and
   `CapabilityDetailShell` are declared members of the composition family and the
   region binding covers them.

**Files**:
- `webview-page/page.js` (modified, ~120 lines)
- `webview-page/page.css` (modified, ~80 lines)
- `src/shell/component_vocabulary.rs` (modified, ~40 lines)

**Validation**:
- [ ] The workspace renders groups, not one flat row run
- [ ] A group with no data marks itself unavailable; the strip does not go silent
- [ ] Every new CSS value resolves from a token; no new raw literal is introduced
      without a recorded gap comment

---

### T021: Render the Patch identity and routing header

**Purpose**: The player always knows which instrument they are editing and where
its sound goes.

**Steps**:
1. Render the focused Patch's name, MIDI channel, and output track as a composed
   header above the groups.
2. Take every value from the projection. Derive no label page-side — the
   serialization-key defect this mission closes was exactly a key reaching the
   screen, and re-deriving labels here would reopen it in a new place.
3. The header is informative, not focusable: it takes no focus and allocates no
   interactive target. The identity header band already exists as a shell band;
   do not duplicate its role — this is the strip's own header inside the
   workspace.

**Files**:
- `webview-page/page.js` (modified, ~60 lines)
- `webview-page/page.css` (modified, ~40 lines)

**Validation**:
- [ ] Name, MIDI channel, and output track all render from projected values
- [ ] The header takes no focus and appears in no focus order
- [ ] A patch switch updates all three together in one paint

---

### T022: Render the envelope group as one titled group

**Purpose**: Shaping a voice should not mean reading four unrelated rows.

**Steps**:
1. Group Attack, Decay, Sustain, and Release under one titled group carrying its
   authored legend.
2. Keep the four rows in their canonical descriptor order and keep each
   individually focusable exactly as before — this is a presentation change, not
   a focus change. No `SemanticAction` variant, focus target, or reducer
   behaviour changes.

**Files**:
- `webview-page/page.js` (modified, ~40 lines)
- `webview-page/page.css` (modified, ~20 lines)

**Validation**:
- [ ] The four rows appear under one titled group in canonical order
- [ ] Each remains individually focusable and adjustable
- [ ] The focus order is byte-identical to before the grouping

---

### T023: Render one titled group per ordered effect slot with nested occupant rows

**Purpose**: Grouping is what tells the operator which rows belong to which slot.
A flat run erases it.

**Steps**:
1. For each of the three ordered effect slots, render a titled group: the slot's
   own occupancy row first, then that slot's occupant parameter rows nested
   beneath it.
2. An **empty** slot still renders its group and its occupancy row — every
   position stays reachable. An empty slot is not an absent group.
3. Clearing one slot must not visually compact the others; positions are stable.
4. Title each slot group, because the identity header names the Patch and an
   operator reading an effect row needs to know which slot it belongs to.

**Files**:
- `webview-page/page.js` (modified, ~80 lines)
- `webview-page/page.css` (modified, ~30 lines)

**Validation**:
- [ ] Three slot groups render whether occupied or empty
- [ ] An occupied slot nests its occupant's rows; an empty one shows only its
      occupancy row
- [ ] Clearing the middle slot leaves the first and third in place
- [ ] Two slots holding the same registry entry render as distinct groups

---

### T024: Paint each numeric row's range and unit

**Purpose**: `numeric_range` and `unit` have been projected and never painted.
Adjustment without visible context is the gap this closes.

**Steps**:
1. Render the projected range and unit on every numeric row that carries them.
2. Take both from the projection. Do not format a unit the projection did not
   supply, and do not infer a range from the value.
3. A row with no projected range or unit renders without them — that is a
   projected fact, not a missing feature to fill in.

**Files**:
- `webview-page/page.js` (modified, ~50 lines)
- `webview-page/page.css` (modified, ~30 lines)

**Validation**:
- [ ] Every numeric row with a projected range shows it
- [ ] Every row with a projected unit shows it
- [ ] No row invents a range or unit the projection did not carry

---

### T025: Paint each row's own valid actions as its right-hand hint

**Purpose**: The strip tells the operator what they can do at each row without
their having to try.

**Steps**:
1. Render each row's projected `validActions` as its right-hand hint.
2. Use the projected labels. The page composes no action vocabulary of its own —
   the footer's existing hint run is an exact presentation of the model-level
   list, and the per-row hints must be the same kind of exact presentation.
3. Keep the footer's model-level hint run as it is. The two agree at the focused
   row by construction; do not special-case one against the other.

**Files**:
- `webview-page/page.js` (modified, ~50 lines)
- `webview-page/page.css` (modified, ~30 lines)

**Validation**:
- [ ] Each row shows its own actions
- [ ] The focused row's hint matches the footer's hint run
- [ ] A row with an empty action list renders no hint rather than a placeholder
- [ ] No action label is composed page-side

---

### T026: Paint the active and requested values on a row mid-structural-edit

**Purpose**: The player sees what the system is moving toward, not just that
something is happening.

**Steps**:
1. When a row projects a `requestedValue`, render both the active value and the
   requested value, plus the existing lifecycle treatment (`Preparing`,
   `Activating`, or the typed failure).
2. Reuse the structural-edit visual vocabulary this design already declares —
   the `Preparing`/`Activating` treatment and the typed-failure text. Do not
   invent a second visual language for it.
3. The active graph remains explicit throughout: the active value never
   disappears while a request is in flight.

**Files**:
- `webview-page/page.js` (modified, ~50 lines)
- `webview-page/page.css` (modified, ~30 lines)

**Validation**:
- [ ] An engine row mid-swap shows active, requested, and status together
- [ ] An effect-slot row mid-occupancy-change shows the same
- [ ] A settled row shows only its active value
- [ ] A failed request shows the typed failure without losing the active value

---

### T027: Render the five-row Utility panel and its authored hint line

**Purpose**: Four of the ten designed-but-undriven structures live on this panel.
This is the densest single closure in the mission.

**Steps**:
1. Render the five projected Utility rows in projected order: master volume,
   patch volume, MIDI input, output track, voice limit.
2. Render the panel's authored hint line, which is currently silently dropped.
3. Seat all five rows inside the persistent side region at both authored
   viewports **without a scroll affordance**. The row set is bounded by
   declaration at five, so no scroll is needed — and `#inspector`'s
   `overflow: hidden` must not be quietly relaxed to make room. If five rows do
   not seat, the row heights are wrong, not the bound.
4. Every label comes from the projection. None is a serialization key.

**Files**:
- `webview-page/page.js` (modified, ~60 lines)
- `webview-page/page.css` (modified, ~40 lines)

**Validation**:
- [ ] Exactly five rows render, in projected order, all with real values
- [ ] None is marked unavailable
- [ ] The authored hint line renders
- [ ] All five seat at 1920×1080 and at 1280×800 with no clipping and no scroll
- [ ] No label equals a serialization key

---

### T028: Render the `CapabilityDetailShell` for both subject kinds

**Purpose**: One composition serving instrument and effect subjects alike. A
branch on subject kind would reintroduce the per-engine page the vocabulary
exists to prevent.

**Steps**:
1. Render the projected detail surface as one composition: title, accent, ordered
   sections, and each section's controls.
2. **Branch on nothing.** The instrument case and the effect case differ only in
   the projection slice handed to the renderer. If you find yourself writing
   `if (subject.kind === ...)` to choose sections, stop — the projection already
   resolved that.
3. Mark a section the projection reports read-only with **text or shape** as well
   as colour.
4. Render a mid-preparation capability through its projected lifecycle status
   rather than as an empty or stale section set.
5. The detail surface replaces the main workspace while open; the persistent side
   region and the shell bands stay.

**Files**:
- `webview-page/page.js` (modified, ~110 lines)
- `webview-page/page.css` (modified, ~70 lines)

**Validation**:
- [ ] One render path serves both subject kinds — assert by inspection and by
      rendering both
- [ ] Section order matches the projection
- [ ] A read-only section is distinguishable without colour
- [ ] A mid-preparation subject shows status, not an empty section list
- [ ] Closing the detail surface restores the strip with focus on the origin row

## Definition of Done

- [ ] All nine subtasks complete
- [ ] The PATCH workspace is grouped structure, not a flat control run
- [ ] Ranges, units, per-row hints, and requested values are painted
- [ ] The Utility panel shows five real rows plus its hint line, with no scroll
- [ ] One detail composition serves both subject kinds
- [ ] Both authored viewports retain every band, the side region, and the
      authored minimum target, with no clipped or overlapping row
- [ ] No raw value bypasses the token table; no label is composed page-side
- [ ] `cargo test --all-targets` green (the composition validations run here too)

## Risks & Mitigations

| Risk | Mitigation |
| --- | --- |
| Re-deriving labels page-side reopens the serialization-key defect | Take every label from the projection; T027's validation asserts it |
| A subject-kind branch in the detail shell | Assert one render path serves both; reviewer checks by inspection |
| Relaxing `overflow: hidden` to fit the Utility rows | The bound is five; if they do not seat, fix the row heights |
| Grouping changing the focus order | T022's validation asserts the focus order is byte-identical |

## Reviewer Guidance

**Verify**:
- The workspace renders groups arranging groups, not a flat run — read the
  render function, do not trust the screenshot
- Every new CSS value traces to a token, or carries a recorded gap comment
- The detail renderer has no branch on subject kind
- Both viewports were actually rendered and checked, not reasoned about

**Red Flags**:
- A literal colour, size, or spacing value with no token and no gap comment
- `if (subject...)` selecting sections in the detail renderer
- `overflow-y: auto` appearing on `#inspector`
- An empty effect slot rendering no group
