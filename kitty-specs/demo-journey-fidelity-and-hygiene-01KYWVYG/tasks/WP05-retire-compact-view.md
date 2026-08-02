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
- src/synth/prepared_post_effect_rack_builder.rs
- src/testing/automatic_midi_test.rs
- src/real_time/audio_renderer.rs
- tests/schema_surface.rs
- tests/support/mod.rs
- tests/semantic_graphical_view_model.rs
- tests/semantic_focus_and_projection.rs
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
- **Scope correction (found by WP03's reviewer, folded in before dispatch)**:
  the accessor `post_effects()` is fully covered by WP02/WP03/WP04/WP08, but
  the *constructor* `with_post_effects()` is used far more widely — 18
  occurrences across 12 files at planning time — and about six of those files
  belong to no migration WP. Those unowned files are now yours (see
  `owned_files`): `src/synth/prepared_post_effect_rack_builder.rs`,
  `src/testing/automatic_midi_test.rs`, `src/real_time/audio_renderer.rs`,
  `tests/schema_surface.rs`, `tests/support/mod.rs`,
  `tests/semantic_graphical_view_model.rs`,
  `tests/semantic_focus_and_projection.rs`. `audio_renderer.rs` moved here
  from WP10 (which no longer owns it) — so you also own its four stale
  WP-numbered comments. Expect a constructor sweep across ~8 files, not a
  two-line deletion.
- If a surviving caller appears in a file owned by ANOTHER WP
  (`src/control/*`, `src/testing/{demo_scene,exhaustive_gui_demo,live_demo_runner,live_demo_report}.rs`,
  `src/shell/standalone_application.rs`, `src/adapter/production_effects.rs`,
  `src/real_time/{graph_preparation_worker,prepared_*}.rs`, or the test
  targets those WPs own), STOP and report it against the owning WP — do not
  migrate it here (ownership discipline).
- Also retire `set_post_effect_config()`: WP02's migration orphaned it and
  parked it behind `#[allow(dead_code)]` with a note deferring removal to
  this WP. It is another compact-view accessor (it addresses "the nth
  occupied slot"), so it goes with the rest. Its only non-`patch.rs` caller
  was in `src/control/app_state.rs`, already migrated by WP02.
- And the backing state: WP02's implementer reports an `effects_compact`
  field on the aggregate that exists only to serve the compact view. Confirm
  it against the code and delete it with the accessors — leaving the field
  would keep the second representation alive in storage even after its
  accessors are gone, which is exactly what the crest-spec invariant
  forbids. Take the full inventory yourself before deleting (grep for
  `compact` in `src/synth/patch.rs`); the three names listed here are the
  known set, not a guaranteed-complete one.

## ⚠️ Reviewer deletion lists are LANE-LOCAL — verify against the merged tree

WP02's reviewer produced an 8-item deletion list. It is a good lead, but two
items are wrong when all lanes are merged, because each reviewer only sees its
own worktree. Verified by the coordinator across lanes on 2026-07-31:

- **`PatchInput::post_effects()` (`src/control/event_record.rs:191`) — DO NOT
  DELETE.** WP02's reviewer called it "newly orphaned" because lane-b's own
  callers moved to field access. But lane-d (WP04) calls it live at
  `src/shell/standalone_application.rs:1513`, written UFCS as
  `PatchInput::post_effects(patch)`. Post-merge it has a caller. It is also
  frozen serialized vocabulary. Deleting it breaks the merged build.
- **`EffectCapabilityRegistry::validate_patch_effects()`
  (`src/synth/effect_capability.rs`) — NOT YOURS, and probably not dead.** It
  takes a dense `&[PostEffectConfig]`, which is the correct shape for
  *serialized* input, and serialized-side callers remain
  (`src/testing/live_demo_scene.rs`, `src/testing/live_demo_report.rs`
  operate on decoded payloads). Only its `Patch`-side callers went away.
  The file is owned by WP10 in any case — report, do not delete.

**Method**: for every deletion candidate, re-check callers in the merged tree
your lane is based on (`git log --oneline -1` should show the dependency
lanes merged in), not in a single lane's view. If a candidate still has a
caller, leave it and say so in your notes.

Genuinely-yours candidates from that same review, worth verifying and likely
correct: `Patch::rebuild_compact_view()` (`patch.rs:266-268`) plus its call
inside `set_slot_occupancy` (`patch.rs:224` — the *call* goes, the function
`set_slot_occupancy` stays), and the compact-view module tests at
`patch.rs:331, 348, 386, 416, 441` that pin the retired mapping.

## Subtasks

### T021 — Delete post_effects()/with_post_effects() from Patch

**⚠️ NAME COLLISION — read before grepping.** Two unrelated methods share the
name `post_effects()`:

- `Patch::post_effects()` (`src/synth/patch.rs:148`) — the compact view.
  **DELETE THIS.**
- `PatchInput::post_effects()` (`src/control/event_record.rs:190`) — frozen
  serialized-input vocabulary, `do_not_change` per `occurrence_map.yaml`, and
  read by the retained live scene
  (`src/testing/live_effects_and_buses_scene.rs:377`, WP01). **THIS MUST
  SURVIVE.** Deleting it breaks the frozen serialization contract and the
  Phase 3 evidence gate.

So the target is "zero callers of the Patch aggregate's compact view", NOT
"zero occurrences of the string `post_effects`". Resolve every hit by
receiver type before touching it.

Note also that at least one surviving serialized-side read is written in UFCS
form — `PatchInput::post_effects(patch)` in `src/shell/standalone_application.rs`
(WP04) — which the literal `post_effects()` grep does **not** match. Do not
read a zero-match grep as proof that the record accessor is gone; it is
supposed to be there.

**Fixture invariant you must preserve (from WP04's review).** WP04's
composition-root rebuild places recorded occupants by matching
`config.slot_id()` against `EffectSlotIndex::ALL`'s `instance_identity()`.
The `src/testing/automatic_midi_test.rs` fixture — now yours — currently
satisfies `slot_id == position + 1`. If your migration off
`with_post_effects()` assigns slot identities that break that relationship,
the round trip will silently RELOCATE the occupant instead of failing.
Preserve it, and assert it in a test rather than trusting it.

**Steps**:
1. `grep -rn "post_effects()\|with_post_effects(\|set_post_effect_config" src/ tests/ --include="*.rs"`
   and triage every hit BY RECEIVER: `Patch`-typed receivers in
   `src/synth/patch.rs` (definitions/internal uses) and your seven other
   owned files are yours to migrate; `PatchInput`/serialized-side receivers
   stay untouched; a `Patch`-typed hit in another WP's owned file means stop
   and report (see Context).
2. Migrate your owned files' constructor call sites to per-position
   construction (`set_slot_occupancy` or the equivalent per-position builder
   entry point) — position-preserving, no local compaction, assertions never
   weakened. `tests/support/mod.rs` is shared fixture infrastructure: change
   it carefully and re-run every target that uses it.
3. Delete the accessor `post_effects()`, the constructor
   `with_post_effects()`, and `set_post_effect_config()`, plus any internal
   helpers that exist only to serve compaction. Replace internal uses with
   the per-position view.
4. Replace the transitional doc comment block (`patch.rs:84-90`) with durable
   documentation of the single-view contract, phrased as the invariant (no WP
   numbers, no timeline). Remove the four stale WP-numbered comments in
   `src/real_time/audio_renderer.rs` (inherited from WP10) and any others in
   your newly owned files.

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
1. Re-run the repo-wide grep from T021 as a final check and paste the result
   into the WP completion notes. Expected residue: the surviving
   `PatchInput::post_effects()` definition and its serialized-side callers
   (see the name-collision warning in T021) — nothing `Patch`-typed. State
   explicitly in your notes which surviving hits you verified as
   serialized-side.
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

- The Patch aggregate's compact view is gone: `Patch::post_effects()`,
  `with_post_effects()`, `set_post_effect_config()`, and the `effects_compact`
  backing field no longer exist. `PatchInput::post_effects()` (frozen
  serialized vocabulary) is intentionally still present and still read by the
  retained scene.
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

## Activity Log

- 2026-07-31T23:49:39Z – claude – shell_pid=90628 – Owned-file sweep complete and committed (3efb137). DELETED from the Patch aggregate: post_effects() accessor, set_post_effect_config() (+ its #[allow(dead_code)]), the effects_compact backing field, rebuild_compact_view() and its call inside set_slot_occupancy (set_slot_occupancy itself retained), and the compact-view module tests at the old patch.rs:331/348/386/416/441 sites. ADDED: pub Patch::with_effect_slot(EffectSlotIndex, PostEffectConfig), a position-explicit construction entry point routed through set_slot_occupancy (integration tests cannot reach the pub(crate) mutator); all 8 owned files migrated to it. T022 tests added: an_occupied_later_position_leaves_the_earlier_one_empty, instance_identities_stay_stable_across_occupancy_changes_elsewhere (positions AND EffectSlotId identity + uniqueness), a_fourth_position_is_unrepresentable; clearing_a_slot_never_compacts_the_others now asserts surviving identity. Fixture invariant preserved and asserted: automatic_midi_test derives its occupant identity from the position via EffectSlotIndex::instance_identity(), and the new test the_effect_fixture_derives_each_occupant_identity_from_its_position pins slot_id == position + 1 through AppLoop::patches(); tests/support/mod.rs derives its fixture identity the same way. Stale mission narration removed from all owned files (4 WP10 comments in audio_renderer.rs, the WP05/WP06 block in patch.rs, WP06 in schema_surface.rs, WP07 header in semantic_focus_and_projection.rs, plus T0xx prefixes; durable C-/NFR-/FR-/SC- ids kept). validate_patch_effects() in effect_capability.rs left untouched: still called on dense serialized slices by live_demo_scene.rs:632 and live_demo_report.rs:1852 -- not dead, and WP10-owned. BLOCKED on the last deletion: Patch::with_post_effects() still has 4 callers in WP03-owned files (src/real_time/parameter_snapshot.rs:1232,1309; tests/static_patch_effect.rs:318,430), so per ownership discipline it is reported, not migrated -- needs a coordinator decision (extend WP05 ownership to those two files, or reopen WP03). Triage grep residue verified serialized-side only: PatchInput::post_effects() def at src/control/event_record.rs:190 with its UFCS caller src/shell/standalone_application.rs:1513, the SerializedPatch.post_effects field and readers in src/control/serialized_state.rs and src/control/state_tree.rs, and the decoded-record post_effects fields in src/testing/live_demo_scene.rs and src/testing/live_demo_report.rs. Zero Patch-typed compact-view reads remain. Validation on the committed state: cargo test --all-targets exit 0 (26/26 targets ok); cargo test --release --test expandable_effects_and_bus_topology exit 0 (CREST_ACCEPTANCE passed); cargo clippy --all-targets -- -D warnings exit 0; cargo fmt --all -- --check exit 0.
