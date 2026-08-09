---
work_package_id: WP03
title: Projection enrichment and channel integrity
dependencies:
- WP02
requirement_refs:
- FR-010
- FR-011
- FR-012
- FR-014
- NFR-005
planning_base_branch: feat/functional-patch-editor
merge_target_branch: feat/functional-patch-editor
branch_strategy: Planning artifacts for this mission were generated on feat/functional-patch-editor. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/functional-patch-editor unless the human explicitly redirects the landing branch.
subtasks:
- T013
- T014
- T015
- T016
- T017
- T018
- T019
history:
- timestamp: '2026-08-08T22:59:56Z'
  actor: planner
  action: created
  note: Initial work package definition
agent_profile: implementer-ivan
authoritative_surface: src/control/
create_intent: []
execution_mode: code_change
mission_slug: functional-patch-editor-01KZERV9
owned_files:
- src/control/semantic_graphical_view_model.rs
- src/control/patch_page_projection.rs
- src/control/graphical_shell_projection.rs
- src/control/state_projector.rs
- src/control/text_projection.rs
- src/shell/webview/projection_channel.rs
- src/control/semantic_focus.rs
- src/testing/demo_scene.rs
priority: P1
role: implementer
status: planned
tags: []
tracker_refs: []
---

# WP03 - Projection enrichment and channel integrity

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this prompt**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

This gives you the identity, governance scope, boundaries, and initialization
context for this work. Do not begin implementation until the profile is loaded.

## Objective

Carry two things into the projection that Phase 4 recorded as designed but
undriven: each row's **own valid actions**, and the **requested value** of a row
mid-structural-edit. Project the new detail surface. Guarantee that authored
labels — never serialization keys — reach the screen. Then prove the projection
channel's load-bearing de-duplication guard, folded in from the previous
mission's follow-ups because this mission drives far more churn through that
channel than any before it.

## Context

**Mission**: functional-patch-editor-01KZERV9
**Priority**: P1
**Dependencies**: WP02 — the control identities and surface must exist first.

Phase 4 recorded that per-row hints do not exist (`validActions` are global to
the focus), that a row mid-structural-edit shows only a lifecycle status without
saying what it is moving toward, and that `numeric_range` and `unit` are
projected but never painted. This package supplies the first two; WP04 paints
the third. It also closes the `masterGainDb`-as-a-label defect: a serialization
key was reaching the screen.

## Branch Strategy

- **Planning base branch**: `feat/functional-patch-editor`
- **Final merge target**: `feat/functional-patch-editor`
- Execution worktrees are allocated per computed lane (see `lanes.json`).
- Do not create ad-hoc branches outside the lane workflow.

## You inherit the `PatchDetail` gate — removing it is yours, and it is three coupled changes

WP02 could not let `EnterSurface(PatchDetail)` be offered, because an accepted
detail state cannot yet be projected and `src/control/app_loop.rs:347` `.expect()`s
that every accepted state projects. It gated the action out of the vocabulary
(`SurfaceId::is_enterable` withholds `PatchDetail`) and left the reducer able to
own, remember, and leave the surface. See finding **F-11** in
`cross-wp-findings.md`.

**T015 removes that gate. It is not one change and the original sizing was wrong.**
WP02 measured this and WP02's reviewer independently confirmed it by applying the
partial fix and re-projecting: fixing only the containment check moves the failure
from `PatchPage(InvalidInstrumentConfig)` to `StateProjectionError::InvalidSelection`
at `render_patch_text`'s `selected_line.ok_or(...)`, because `control_id` is `Some`
only for `StructuralChoice` rows and Braids' three detail rows are
`ParameterUpdate::Scalar`. All three land together or none does:

1. the containment check at `patch_page_projection.rs::project`,
2. the detail page content **plus** `render_patch_text`'s selected line,
3. deleting the `PatchDetail` arm in `SurfaceId::is_enterable`.

**Ownership was widened so you can land a green lane.** Removing the gate turns
six tests red, and two of the files involved were not originally yours. You now
own `src/control/semantic_focus.rs` (WP02's, now approved and closed) and
`src/testing/demo_scene.rs` (carved out of WP06's set). The six tests are three
named tripwires, a fourth reducer test, and two demo-scene coverage tests that go
red with `missing: ["surface.detail"]` — those last two need the two
`surface.detail` scene steps restored in `demo_scene.rs`. That is why you have it.

Do not remove the gate until the projection actually works. A gate removed ahead
of its projection reintroduces exactly the panic it was built to prevent.

## Two things to fold in while you are in these files

**`PatchDetailSubject` serializes snake_case fields into a camelCase schema.**
`#[serde(tag, rename_all)]` renames variants, not fields, so the subject's fields
landed snake_case and are now frozen at `StateTree::SCHEMA_VERSION = 14`. No
declared invariant mandates camelCase, and it was visible at WP02's cycle 1 where
the reviewer did not flag it — so it is not WP02's to redo. But you are already
moving the semantic leaf descriptor and will bump the version anyway. Fold the
casing fix in rather than paying a second version bump later.

**`VoiceLimitCarryOver`'s discriminant is yours to consume.** F-07 ruled the
engine-swap carry-over asymmetry deliberate and lossy by design — narrowing clamps,
widening preserves — and required that the loss be *reported* rather than left for
a player to discover. The engine-swap status projection is the declared home. A
swap that narrowed the limit must say so; today the discriminant is bound only
inside a `debug_assert_eq!`.

## Subtasks

### T013: Project per-row `validActions` from the model-level resolver

**Purpose**: A per-row hint needs a per-row action list. It must come from the
same resolver that computes the model-level list — a second hand-maintained
vocabulary is exactly what the declaration forbids, and it would look correct
until the two drifted.

**Steps**:
1. Add `valid_actions: Vec<ValidAction>` to `SemanticControlViewModel` in
   `src/control/semantic_graphical_view_model.rs`.
2. Populate it by calling the **same pure resolver** the model-level
   `validActions` uses, evaluated for the counterfactual focus in which this
   control is focused. Do not write a parallel computation.
3. At the actually-focused row, the per-row list and the model-level list must
   agree exactly. Assert this, not as a coincidence but as the contract.
4. Keep the list ordered and duplicate-free, excluding unavailable directions,
   unavailable modes, invalid surfaces, and blocked structural edits — the same
   exclusions the model-level list already applies.

**Files**:
- `src/control/semantic_graphical_view_model.rs` (modified, ~90 lines)

**Validation**:
- [ ] The focused row's list equals the model-level list, element for element
- [ ] A disabled row's list excludes the actions its disabled state blocks
- [ ] A row at a value boundary excludes the adjustment direction that would
      exceed it
- [ ] Only one resolver exists — grep for a second action-computation site

**Edge Cases**:
- A read-only row: its list is legitimately short, possibly empty. An empty list
  is a projected fact, not a bug — but assert it is empty rather than absent.

---

### T014: Project `requestedValue` for exactly the in-flight correlated row

**Purpose**: A row mid-structural-edit shows what it is moving toward. The
lifecycle status alone reports that something is happening without saying what.

**Steps**:
1. Add `requested_value: Option<SemanticControlValue>` to
   `SemanticControlViewModel`.
2. Populate it **only** from the correlated structural lifecycle already in
   canonical state — the in-flight request's target. Never write a locally
   optimistic value ahead of the reducer; that would break the one-way loop while
   appearing to work.
3. It is `Some` exactly while a correlated structural edit targets this control,
   and `None` on every settled row.
4. Wire it through `patch_page_projection.rs` so the PATCH page projection
   carries it for engine rows, structural-choice rows, and effect-slot occupancy
   rows alike.

**Files**:
- `src/control/semantic_graphical_view_model.rs` (modified, ~50 lines)
- `src/control/patch_page_projection.rs` (modified, ~70 lines)

**Validation**:
- [ ] An engine row mid-swap projects both the active and the requested capability
- [ ] An effect-slot row mid-occupancy-change projects both
- [ ] Every settled row projects `None` — count them and assert zero
- [ ] The value is sourced from canonical state, never from the input that
      triggered the edit

---

### T015: Project the `Detail` surface role and its capability-resolved content

**Purpose**: One surface whose entire content comes from the descriptor its
subject names.

**Steps**:
1. Add `Detail` to `SemanticSurfaceRole`.
2. Project a `PatchDetail` surface exactly while `InteractionState` holds a
   detail entry, and not otherwise. Hosts must never see a stale detail surface
   and must never have to synthesize one.
3. Resolve its label, accent, ordered sections, controls, ranges, units, status,
   and errors from the installed descriptor named by the `PatchDetailSubject`.
   The projection declares no section list and branches on no subject kind — the
   instrument case and the effect case differ only in which descriptor is read.
4. A capability-declared read-only section projects controls with `editable`
   false, so the page can mark it in text or shape rather than by colour.
5. A capability mid-preparation projects its typed lifecycle status rather than
   an empty or stale section set.
6. Extend `SemanticSurfaceSummary` with a detail variant carrying the subject's
   capability identity.

**Files**:
- `src/control/semantic_graphical_view_model.rs` (modified, ~180 lines)

**Validation**:
- [ ] The detail surface is present exactly while the entry is open
- [ ] An instrument subject and an effect subject both produce a surface with the
      same `SurfaceId` — one identity, two subjects
- [ ] Section order matches descriptor order
- [ ] A read-only section projects `editable: false` on its controls
- [ ] A mid-preparation subject projects `Preparing`/`Activating`, not an empty
      section list
- [ ] Grep confirms no `match` on subject kind selects a section list

---

### T016: Guarantee authored labels, never serialization keys

**Purpose**: Close the recorded defect where global rows projected
`descriptor.name()` — `masterGainDb`, a serialization key — as their on-screen
label.

**Steps**:
1. Audit every label-producing site in the files you own. Each must project the
   owning descriptor's **authored label**, not its serialization name.
2. Fix the global-row site specifically: master gain projects `Master Volume`
   (or the descriptor's authored label), never `masterGainDb`.
3. Add a guard test that scans every projected label in a full projection of the
   fixture and fails if any equals a known serialization key. Make it a set
   comparison against the serialized leaf descriptor's names, so a *future* key
   leaking into a label also fails — not just this one.

**Files**:
- `src/control/semantic_graphical_view_model.rs` (modified, ~40 lines)
- `src/control/patch_page_projection.rs` (modified, ~20 lines)

**Validation**:
- [ ] No projected label on any surface equals any serialization key
- [ ] The guard fails when a key is deliberately reintroduced as a label —
      verify by doing it, then reverting
- [ ] The guard covers PATCH, PATCH Utility, detail, MIXER, and MIXER Inspector,
      not just the row that was reported

---

### T017: Make the one-generation patch switch observable in the projection

**Purpose**: NFR-005 — no intermediate projection may pair one Patch's identity
with another's schema. That claim needs a surface to assert against.

**Steps**:
1. Ensure the projection derives every PATCH field from one accepted `AppState`
   generation and state hash, as the existing invariant already requires.
2. Add a projection-level consistency check: within one projected model, the
   `focusPath.patchId`, every PATCH surface's summary `patchId`, and every
   control path's `patchId` agree. A disagreement is a defect, not a transient.
3. Expose enough for WP05 and WP06 to assert a generation delta of exactly one
   across a switch — the generation is already projected; confirm it is not
   advanced twice by the reprojection.

**Files**:
- `src/control/state_projector.rs` (modified, ~50 lines)
- `src/control/graphical_shell_projection.rs` (modified, ~25 lines)

**Validation**:
- [ ] Every PATCH identity in one projection agrees
- [ ] A switch produces exactly one advanced generation
- [ ] The diagnostic `TextProjection` and the graphical projection still come
      from the same accepted generation and cannot disagree

---

### T018: Prove `retire()`'s de-duplication guard in the projection channel

**Purpose**: A folded-in follow-up. The guard is load-bearing — without it a
stale identity shadows a current one and causes a *false* rejection — but no test
fails when it is removed. This mission drives a whole-surface reprojection per
patch switch plus a detail surface appearing and disappearing through that same
channel, which puts the guard squarely in this mission's blast radius.

**Steps**:
1. Read `src/shell/webview/projection_channel.rs:564` and the surrounding
   in-flight document handling.
2. Read the previous mission's note at
   `kitty-specs/shell-hygiene-01KZD0KR/semantic-acceptance.md` under "New
   findings" — item 1 records that the reviewer wrote the proving test and left
   it ready to paste. Use it if it is still applicable; write your own if the
   surrounding code has moved.
3. The test must construct the exact situation the guard prevents: a stale
   identity present alongside a current one, where the absence of de-duplication
   produces a false rejection of a valid acknowledgement.

**Files**:
- `src/shell/webview/projection_channel.rs` (modified, ~90 lines of test)

**Validation**:
- [ ] The test constructs a stale-plus-current identity pair, not a synthetic
      stand-in
- [ ] It asserts the *false rejection* the guard prevents, not merely that
      `retire()` returns

---

### T019: Confirm the guard's proving test fails when the guard is removed

**Purpose**: A test that passes with and without the code it claims to prove is
not a proof. This is the falsification step and it is not optional.

**Steps**:
1. Remove the de-duplication guard.
2. Run the test from T018. It must fail.
3. Restore the guard. Run again. It must pass.
4. Record the observed failure — the assertion that fired and its message — in
   the WP's completion notes so the reviewer can confirm the falsification was
   performed rather than described.

**Files**:
- none — this is a verification step whose output is a recorded observation

**Validation**:
- [ ] The failure was observed and recorded verbatim
- [ ] The guard is restored and the test passes
- [ ] If the test does *not* fail without the guard, stop: the test is
      non-discriminating and must be rewritten before this WP can close

## Definition of Done

- [ ] All seven subtasks complete
- [ ] Per-row actions come from the model-level resolver and agree at the focus
- [ ] `requestedValue` is present on exactly the in-flight row
- [ ] The detail surface projects from descriptor content with no subject branch
- [ ] No serialization key reaches any projected label, guarded by a set check
- [ ] The `retire()` guard is proven, and the proof is shown to discriminate
- [ ] `cargo test --all-targets` green; `cargo clippy --all-targets -- -D warnings` clean

## Risks & Mitigations

| Risk | Mitigation |
| --- | --- |
| A second action-computation site drifting from the resolver | Call the existing resolver; grep for a second site as a validation step |
| An optimistic `requestedValue` written ahead of the reducer | Source it only from the correlated lifecycle in canonical state |
| The label guard covering only the reported row | Assert against the full serialized leaf-descriptor key set across every surface |
| A non-discriminating guard test | T019 exists specifically to catch this; do not skip it |

## Reviewer Guidance

**Verify**:
- The per-row action list is produced by the same function as the model-level
  list, not a lookalike
- `requestedValue` traces to canonical state, not to the triggering input
- The label guard compares against the key set, not a hardcoded `masterGainDb`
- T019's recorded failure is present and plausible

**Red Flags**:
- A `match` on `PatchDetailSubject` selecting sections
- A detail surface projected while no entry is open
- A `requestedValue` set anywhere outside the lifecycle projection
- T019 recorded as "verified" with no observed failure text

## Activity Log

- 2026-08-09T05:00:39Z – claude – shell_pid=12527 – T019 falsification performed, not described. Guard removed from ProjectionChannel::retire (the retired_identities.remove(stale) block); the T018 proving test failed with, verbatim: thread 'shell::webview::projection_channel::tests::a_re_pushed_generation_retires_once_so_no_stale_identity_shadows_the_current_one' panicked at src/shell/webview/projection_channel.rs:1273:14: 'a verbatim ack for the re-pushed document must not be rejected: IdentityMismatch { generation: 1, field: "stateHash" }'. Guard restored; test passes; production diff for that file is +85 lines of test only. T016's guard was falsified the same way: reintroducing descriptor.name() as the MIXER Inspector global row's label failed with 'Mixer(Global { parameter: MasterGainDb }) on MixerInspector is labelled with the serialization key masterGainDb'; reverted.
- 2026-08-09T06:12:58Z – unknown – shell_pid=0 – Cycle 2, B1. The cycle-1 label guard walked SemanticGraphicalViewModel only, so the reviewer's counter-falsification on patch_page_projection.rs:212 was green and the guard was not a proof of the site it claimed. It now walks every label-producing projection through the production StateProjector::project_with_shell -- semantic surfaces and controls, the PATCH page's engine/envelope/output/sections/effects/detail labels plus their choice, selected and requested labels, and every segment of graphicalShell.footer.pathLabel -- over ten fixtures covering both engines, both detail subjects, all five surfaces, and both surfaces that host master gain with the cursor on it. Falsification performed, not described. Reverting patch_page_projection.rs:212 from descriptor.label() to descriptor.name() now fails with, verbatim: thread 'control::semantic_graphical_view_model::projection_enrichment_tests::no_projected_label_on_any_surface_is_a_serialization_key' panicked at src/control/semantic_graphical_view_model.rs:3187:17: 'soundfont PATCH Main: page output masterGainDb is labelled with the serialization key masterGainDb'. Restored; passes.
- 2026-08-09T06:13:11Z – unknown – shell_pid=0 – Cycle 2, F-25. state_projector's per-control-identity path composition is deleted, not repaired arm by arm: footer_path_label is one composition reading the focused row's authored label through the new SemanticGraphicalViewModel::focused_control, so no arm exists in which a key could be spelled. Falsified both ways by restoring the key composition: the set check failed with 'soundfont PATCH Main: shell footer pathLabel segment is labelled with the serialization key patch.engine', and the derivation check the_footer_breadcrumb_is_the_focused_rows_authored_label failed with left "PATCH / patch.engine" right "PATCH / Engine". Restored; both pass. The breadcrumb now reads "PATCH / Master Volume", "MIXER / T00 Level", "PATCH / Model".
- 2026-08-09T06:13:23Z – unknown – shell_pid=0 – Cycle 2, B2. Resolved by correcting the record, NOT by changing the code. The reason is the crest-spec: aggregate.Control.SemanticControlViewModel.state is declared field by field at contexts/control.yaml:806 and patchInteraction is not among them, while the same declaration carries it explicitly on PatchPageProjection's sections[].parameters[] and detail.sections[].parameters[] (:691, :701). Carrying it onto the semantic model would mean editing the crest-spec mid-implementation to permit code already written, and would create a second producer for one declared fact. Corrected record, in three places review can check: the detail_editable binding's comment now states that editable is a uniform surface-level fact about what the reducer accepts and discriminates nothing about the capability's declaration, and names patchInteraction plus its path as the home of the declared fact; the editable-false test's docstring says explicitly that it is not a read-only proof and points at the one that is; and a new test, patch_page_projection::tests::the_declared_patch_interaction_reaches_the_detail_page_and_discriminates, asserts every projected detail row against the installed descriptor's declaration and then spells out the discriminating pair on one descriptor -- SoundFont's file projects ReadOnly, its preset projects StructuralChoice, editable is false on both. WP05's T033 escape hatch stays shut; the producer is patchPage.detail.sections[].parameters[].patchInteraction.
- 2026-08-09T06:13:37Z – unknown – shell_pid=0 – Cycle 2, remaining items. T017's tautological comparand removed: semantic.patch_identity() resolves to the same expression as snapshot_patch so it could never fire; the page half stays and the comment now says why the model's identity is guarded by validate_data plus the generation/state-hash equality above instead. VoiceLimitCarryOver::Clamped{previous} kept with a documented reason rather than consumed: the projection reports the loss before the commit, so the row still carries the previous limit as its own projected value and requested_value carries the clamped one -- reading previous there would project what the row already shows. It earns a consumer only in F-22's durable report, after the commit has overwritten canonical state, which needs a crest-spec field. N6 corrected: with_counterfactual_focus's doc no longer claims every branch goes through reducer transitions, and interaction_state.rs's 'two sites' is now four with all four named. Gates: cargo test --all-targets green (701 lib + every integration target, 0 failed), cargo clippy --all-targets -D warnings clean, cargo fmt --all --check clean with no reformatting needed this cycle. F-12 did not fire. Out-of-map edits this cycle are doc-comment-only into closed packages (app_state.rs, interaction_state.rs, synth/patch.rs); no WP06 file touched.
- 2026-08-09T06:14:13Z – unknown – shell_pid=0 – Cycle 2 complete. F-19 resolved by correcting the record, not the code: PatchInteraction::ReadOnly is real, crest-spec declared, and already projected at patchPage.detail.sections[].parameters[].patchInteraction; carrying it onto SemanticControlViewModel was rejected because the crest-spec declares that value object field-by-field and does not list it there, so adding it would mean editing the bedrock to permit already-written code (the F-07 inversion). Proved by a discriminating test: SoundFont declares ReadOnly on 'file' and StructuralChoice on 'preset' in one section, so a hardcoded projection fails on one of the two. F-25 resolved: the footer breadcrumb's per-variant match is deleted; one footer_path_label composed from the focused row's authored label. Two falsifications performed this cycle, both observed then reverted: (1) reverting patch_page_projection's master-gain label to descriptor.name() fails the widened guard with 'soundfont PATCH Main: page output masterGainDb is labelled with the serialization key masterGainDb' — cycle 1's guard could NOT fail on that site; (2) re-coupling the breadcrumb to the control identity fails with left: "PATCH / Patch(Engine)" right: "PATCH / Engine". Also removed a cycle-1 check that compared a value to itself (semantic.patch_identity() resolves to active_focus.patch_id()). Suite 29/29 binaries, 701 lib tests, clippy clean, fmt clean. NOTE: the lane commit includes kitty-specs/cross-wp-findings.md (+84 lines, the F-19 cycle-2 resolution note) and tripped the protected-path guard warning; the coordination branch does not yet have that text, so it arrives at merge. Flagging for relocation if the guard is meant to be strict.
