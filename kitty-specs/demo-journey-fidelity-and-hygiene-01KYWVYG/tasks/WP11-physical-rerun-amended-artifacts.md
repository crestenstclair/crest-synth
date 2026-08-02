---
work_package_id: WP11
title: Physical re-run & amended acceptance artifacts
dependencies:
- WP01
- WP05
- WP06
- WP07
- WP09
- WP10
requirement_refs:
- FR-005
- FR-006
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T044
- T045
- T046
- T047
history:
- timestamp: '2026-07-31T20:21:28Z'
  actor: planner
  action: created from IC-10 (evidence refresh and record amendment)
agent_profile: implementer-ivan
authoritative_surface: ROADMAP.md
create_intent: []
execution_mode: code_change
mission_id: 01KYWVYGQMTRFY314AP78KZJPY
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
model: ''
owned_files:
- ROADMAP.md
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP11 – Physical re-run & amended acceptance artifacts

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Run the reworked retained scene on the physical rig, capture refreshed
recorded evidence, prove the add-only checkpoint-identity contract by
byte-level comparison against the parent evidence, and amend the parent
mission's acceptance matrix and post-merge review addendum (add/append-only)
so every one of the review's seven open items is dispositioned. This is the
mission's evidence gate — the corrective gate is not healed until the player
journey is demonstrated ON HARDWARE and the record says so.

## Context

- This WP runs LAST (depends on every code WP). Its lane must contain the
  full merged mission state per `lanes.json`.
- **Ownership note**: WP `owned_files` cannot declare `kitty-specs/` paths
  (runtime rule `INVALID_WP_OWNED_FILES_KITTY_SPECS`). This WP therefore
  formally owns `ROADMAP.md` (the gate-closure update), and the two
  parent-mission artifact amendments (T046, T047) are executed as recorded
  out-of-map edits — permitted with a one-line rationale in the WP notes:
  "FR-006 amendment of parent acceptance record; kitty-specs paths are
  non-declarable by rule". The `occurrence_map.yaml` exception for the
  parent mission dir (manual_review) already sanctions touching them.
- The parent evidence baseline: two 2026-07-31 physical runs, 131/131
  checkpoints, `droppedRecords=0`, zero false observation keys, clean
  teardown (recorded in the parent acceptance matrix and review). Locate the
  parent evidence artifacts from the references inside
  `kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/acceptance-matrix.json`
  — store the refreshed evidence following the same convention.
- RECORDED-MANUAL discipline: this evidence class requires a real window,
  physical audio, and the real MIDI fixture (parent C-010). If this host
  cannot drive the rig, STOP and ask the operator to run the command —
  never substitute headless output, never fabricate a report (no silent
  fallback; proof gates always ask the human).
- Amendment discipline (spec C-008): add/append-only. Existing rows, grades,
  and history stay byte-identical except where the parent review itself
  already declared supersession (the DRIFT-6 addendum's "superseded:
  inadequate for the player journey" note is the hook your amendment
  resolves).
- The seven open items to disposition (parent review "Open items"): 1
  DRIFT-1 compact view (WP02-05), 2 DRIFT-2 startup fallback (WP04), 3
  RISK-2 twin test (WP07), 4 SC-008 fixture (WP09, operator-included), 5
  DRIFT-3/4/5 cleanups (WP06/WP10 + per-WP sweeps), 6 RISK-1 layout
  hardening (WP08, operator-included), 7 guard tool gating (WP10).

**Numbers that moved (from WP09's review — do not copy the parent's values
blindly):**
- The deterministic observation is now `schemaVersion: 2`, with the added
  fields `fourthEntryEndToEndExercised` and
  `carryOverWrongEngineIdentityRefused` (both true).
- `retiredGraphsCollectedOffCallback` moved **8 → 15** (WP09 drives seven
  more structural changes; the predicate is `gt 0`, so it still passes).
  This is the ONLY pre-existing numeric that changed. Any artifact you amend
  from the parent's `deterministic-acceptance.json` must expect 15, not 8.
- **The fourth entry is test-only.** It lives solely in
  `tests/expandable_effects_and_bus_topology.rs`; the production composition
  root still builds the three-entry registry. The physical
  `make demo-live-effects-and-buses` run must therefore still show exactly
  three product effects on screen — do NOT expect the witness entry to
  appear in the real window, and treat its appearance as a defect.
- SC-008 can now be graded on demonstration rather than structural absence.
  Citable evidence: `fourthEntryEndToEndExercised: true` plus the
  zero-production-diff fact, satisfying the crest-spec
  `open_effect_registry` step-1 `observes` clause verbatim.

## Subtasks

### T044 — Physical re-run of demo-live-effects-and-buses; capture evidence

**Steps**:
1. Preflight on the lane's merged state: `cargo test --all-targets` green;
   `cargo test --release --test expandable_effects_and_bus_topology --
   --nocapture` emits schemaVersion 2 with all predicates true; guard script
   healthy.
2. Run `make demo-live-effects-and-buses` with the physical rig (real
   window, physical audio device, real MIDI fixture). Watch for the journey:
   focus landing on each PATCH slot row before its occupancy cycles; the
   audible occupant edit from PATCH; MIXER return-row walks; the documented
   rejection's visible reason.
3. Capture the complete refreshed report/evidence per the parent
   convention; verify completeness: 100% checkpoints, `droppedRecords=0`,
   zero false observation keys, clean teardown, normal exit
   (NFR-001). Measurement fields must show measured values or explicit
   absent — a defaulted zero is a WP06 regression, stop and report.

**Validation**: complete report captured and stored; completeness figures
recorded in the WP notes.

### T045 — Byte-level checkpoint-identity comparison vs parent evidence

**Steps**:
1. Extract the checkpoint-identity set/sequence from the parent evidence and
   from the refreshed run.
2. Compare byte-level: every parent identity present, unmodified, in order;
   every difference is an addition (the journey/focus/parameter-edit
   checkpoints). Expected: 0 modified, 0 removed, N added (record N).
3. Attach the comparison (method + result) to the WP notes — this is the
   SC-003 gate and the deterministic T006 assertion's physical twin.

**Validation**: 0 modified / 0 removed; additions enumerated.

### T046 — Amend parent acceptance-matrix.json (add/append-only)

**Steps**:
1. Append entries referencing the refreshed evidence for the superseded
   items: the live-gate rows (parent FR-019/C-010 lineage) now point at the
   journey-demonstrating run; note the add-only identity comparison result.
2. Do not edit or delete existing rows/values; additions only. Keep the
   file's existing structure/conventions (inspect before writing).

**Validation**: `git diff` on the file shows pure additions (plus any
required container syntax); parent recorded history intact.

### T047 — Amend parent review addendum: disposition all 7 open items

**Steps**:
1. Append a dated resolution section to the DRIFT-6 addendum in
   `kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/mission-review.md`:
   the scene rework summary, the refreshed run's figures, the identity
   comparison result, and the statement that the superseded FR-019/C-010
   grading is restored to adequate by the new evidence.
2. Disposition each of the seven open items with its closing WP and proof
   pointer (test selector, grep result, script behavior). Items 4 and 6
   were operator-included and delivered — record them as closed, not
   deferred. If anything was legitimately dropped mid-mission by a recorded
   decision, record deferred-with-rationale instead (SC-007 allows this only
   for the two optional items).
3. Append-only: no rewriting of the original review text.
4. Update `ROADMAP.md`'s "Current corrective gate" section (this WP's owned
   file) to record the gate as healed: the journey rework is demonstrated on
   hardware, the open items are dispositioned, and Phase 4 is unblocked.
   Keep the section (history), amend its status — do not delete it.

**Validation**: all 7 items dispositioned; SC-007 satisfied; diffs on the
two parent artifacts are append-only; ROADMAP records gate closure.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch
is also the final merge target. This WP depends on six WPs — enter the lane
the runtime computes from `lanes.json`; its base carries all dependency
commits.

## Test Strategy

The physical run IS the test (RECORDED-MANUAL). Deterministic preflight
before touching hardware; the identity comparison (T045) is the
falsifiable core — treat a single modified identity as a hard fail routed
back to WP01.

## Definition of Done

- Refreshed physical evidence captured, complete (100% checkpoints, 0
  dropped, clean teardown), journey visible on screen.
- Identity comparison: 0 modified, 0 removed, additions enumerated.
- Parent acceptance matrix + review addendum amended add/append-only; all 7
  open items dispositioned.

## Reviewer Guidance

- Verify the evidence is from a REAL run (timestamps, device identifiers,
  report continuity) — this mission exists because a demo claim outran its
  demonstration; do not let that recur in its own closure.
- Check `git diff` on both parent artifacts is genuinely append-only.
- Cross-check each disposition's proof pointer actually exists (run the
  named selector, run the greps).
- If the operator ran the hardware step, confirm the captured artifacts are
  committed and referenced, not just described.

## Activity Log

- 2026-08-01T01:03:40Z – claude – shell_pid=85401 – WP11 PARTIAL - Tasks A-D committed as d1f8f31 on lane-k; WP11 stays in_progress, T044-T047 NOT marked done. BLOCKED on operator at the physical rig. (A) Fixed merged-tree build break: 'Cross-lane integration break: WP09 branched from WP08 and used Patch::with_post_effects, which WP05 deleted in a sibling lane. Neither reviewer could observe the other lane. Fixed at the merge point with WP05's replacement API. Out-of-map edit authorized by the coordinator.' Replacement: Patch::with_effect_slot(position, occupant) at tests/expandable_effects_and_bus_topology.rs:877. (B) Corrected PROJECTION_BODY doc comment in src/control/state_tree.rs: it claimed the fixture used the projector's emitted vocabulary, but render_mixer_text emits singular send[n]= (state_projector.rs:732) while sends[n] is the serialized-snapshot leaf name; fixture unchanged, comment reworded. WP10-owned, same authorization. (C) Preflight all green: cargo test --all-targets exit 0 with 533 passed/0 failed across 26 targets; release declared-proof exit 0 with schemaVersion 2, fourthEntryEndToEndExercised true, carryOverWrongEngineIdentityRefused true, twoRunTraceEqual true, retiredGraphsCollectedOffCallback 15; witness exit 0 clean and exit 1 under refused-topology-published; guard script exit 0 with marker; clippy -D warnings and fmt --check both exit 0; T043 greps WP-comments 0, reverbSend guard fixture only, 'aux bus' 0, compact-view symbols 0 (Task A's break cleared). (D) T045 baseline established: the parent's cited logs t052-run.log and wp10-t059-live-run.log are NOT committed in any branch and do not exist on this host; the durable baseline is WP01's FROZEN_TOPOLOGY_IDENTITY_BASELINE (17 identities, tests/effects_and_buses.rs:59), corroborated by the parent's recorded topology_checkpoints=17. Comparison method and the 13 expected additions staged in kitty-specs/demo-journey-fidelity-and-hygiene-01KYWVYG/wp11-amendment-draft.md, along with DRAFT append-only amendments for the parent acceptance-matrix.json and mission-review.md with every evidence-dependent field left as TODO(physical). Parent artifacts remain byte-untouched by design. ROADMAP.md records the gate as OPEN/code-complete, not healed. REMAINING: operator must run 'make demo-live-effects-and-buses' on the rig; no physical evidence claimed, no headless substitution.
