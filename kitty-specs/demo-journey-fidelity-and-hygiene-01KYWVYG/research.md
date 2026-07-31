# Research: Phase 3 Demo Journey Fidelity and Hygiene

**Date**: 2026-07-31
**Status**: complete — no `[NEEDS CLARIFICATION]` markers remain; all scope
decisions were resolved via recorded decision moments.

## R-1: How the scene drives the on-screen journey

- **Decision**: Compose slot/return occupancy journeys from the scene's
  existing support-script vocabulary (`LiveTopologySupport::FocusMixerTrack`,
  `Event(Navigate/EnterSurface/...)`, `VerifyMixerFocus` and PATCH-side
  equivalents), driving focus to `PatchControlId::EffectSlot(n)` rows and the
  MIXER return rows, then cycling occupancy with the adjacent-choice gesture.
- **Rationale**: The MIXER send walks in
  `src/testing/live_effects_and_buses_scene.rs` already use exactly this
  pattern (focus → verify → gesture → verify), and the reducer-level
  vocabulary is fully proven deterministically
  (`tests/semantic_focus_and_projection.rs`, per the parent mission review).
  No new UI capability is required — DRIFT-6 is an evidence gap, not a
  correctness hole.
- **Alternatives considered**: extending `LiveDemoRunner` with a new
  journey-step kind (rejected: the support-script layer already expresses
  this; a runner change would widen blast radius); scripted keyboard events
  at the window layer (rejected: the scene dispatches semantic actions
  through `AppLoop` by design — C-002 requires the production path, not
  device emulation).

## R-2: Why add-only checkpoint identity is achievable

- **Decision**: Keep every existing checkpoint/transition identity
  byte-identical by leaving the structural intents unchanged and only adding
  journey/focus verification steps and checkpoints.
- **Rationale**: The UI adjacent-choice gesture resolves to the same
  structural intent (`SetSlotOccupancy`/`SetReturnOccupancy` with the same
  patch/slot/entry values) that the scene currently injects — the reducer
  emits identical transition identities either way. The rework changes *how
  the action originates*, not *what the action is*. New identities appear
  only for the added navigation/focus/parameter-edit steps.
- **Alternatives considered**: renaming checkpoints to journey-flavored names
  (rejected: violates C-001 and breaks cross-run evidence comparison);
  keeping the injected steps alongside duplicate journey steps (rejected:
  would perform each occupancy change twice, altering the audible timeline
  and the transition sequence).

## R-3: The controlled rejection stays injected

- **Decision**: The unknown-entry rejection keeps direct injection, with an
  inline comment in the scene at the injection site documenting why.
- **Rationale**: The UI cannot request an unknown registry entry by design —
  the adjacent-choice gesture only reaches installed entries and empty. This
  is the sanctioned exception (spec C-003, crest-spec
  `requirement.expandable_effects_behavioral_proof`).
- **Alternatives considered**: none viable — building a UI path to request
  unknown entries would add a product surface solely for the demo, inverting
  the fidelity goal.

## R-4: Compact-view retirement shape

- **Decision**: Migrate all 36 `post_effects()` call sites across 15 files to
  `effect_slots()`, fix the composition-root round-trip to rebuild from the
  per-position view, then delete `post_effects()` and `with_post_effects()`.
  Test-first: a regression test with a gapped chain (slot 0 empty, slot 1
  occupied) through the round-trip precedes the deletion.
- **Rationale**: DRIFT-1 — two representations of one truth; the round-trip
  at `src/shell/standalone_application.rs:1470` silently re-compacts gapped
  chains. Deleting the accessor closes the class by construction
  (DIRECTIVE_043), now backed by the crest-spec Patch invariant ("no
  compacting or position-erasing accessor exists").
- **Alternatives considered**: deprecation markers with a later removal
  (rejected: the parent mission already tried "transitional, retire in
  WP05/WP06" and it survived its own retirement plan); keeping a renamed
  compact helper for tests (rejected: same two-truths seam under a new name).

## R-5: Absent-vs-zero measurement representation

- **Decision**: Derived live-report measurements
  (`frames_to_projection`, activation gap, blocks-to-audible —
  `src/testing/live_demo_report.rs:872-886`) become explicit optional values;
  rendering distinguishes "absent" from `0`; expectations that need the
  measurement fail on absent.
- **Rationale**: DRIFT-4 — `.max().unwrap_or(0)` makes a regression that
  stops populating fields read as the strongest possible pass. The crest-spec
  now declares the invariant on `valueObject.Testing.LiveDemoReport`.
- **Alternatives considered**: sentinel numeric values (rejected: sentinel
  arithmetic is the same vacuous-proof class); making presence a separate
  boolean next to the number (rejected: two fields can drift; one optional
  value cannot).

## R-6: Guard script tool gating

- **Decision**: `scripts/check_no_name_enumerated_identity.sh` verifies each
  required tool with `command -v` up front and exits non-zero naming the
  missing tool; `tests/no_name_enumeration_guard.rs` covers the gate.
- **Rationale**: security note in the parent review — `|| true` masks missing
  `rg`/`perl` as "no candidates" (vacuous gate). Declared in the crest-spec
  validation description and the new `asset.ValidationScripts`.
- **Alternatives considered**: vendoring pure-shell fallbacks (rejected:
  duplicate scanning logic drifts; the cargo-test guard already provides an
  independent in-process check).

## R-7: Per-position capability identity (FR-016, operator-included)

- **Decision**: The prepared layout records the engine/effect capability id
  for every prepared position; the three racks' carry-over guards
  (`prepared_engine_rack.rs:187-209`, `prepared_post_effect_rack.rs:222-256`,
  `prepared_bus_return_rack.rs:173-195`) additionally require exact identity
  agreement; mismatch keeps the freshly prepared instance. Prepare-time code
  only.
- **Rationale**: RISK-1 — today a same-scalar-count wrong-engine candidate at
  a non-selected position is indistinguishable to the guards. Recording
  identity closes the gap by construction. Operator chose Include
  (DM-01KYWWM9BXV6CRCYXHDEHY9VTJ).
- **Alternatives considered**: deferring (my recommendation, rejected by
  operator ruling — recorded); hashing full descriptor layouts (rejected:
  capability id is the canonical identity; a hash adds a second identity).

## R-8: Fourth-entry fixture placement (FR-015, operator-included)

- **Decision**: Extend `tests/expandable_effects_and_bus_topology.rs` with a
  test-registry fourth entry driven through slot occupancy, return occupancy,
  preparation, projection, and render; surface the result as witness field
  `fourthEntryEndToEndExercised` (observation `schemaVersion: 2`).
- **Rationale**: SC-008 of the parent was graded PARTIAL (proof by structural
  absence). The declared integration target already owns the observation
  emission, so the fixture adds no new asset or validation surface. Operator
  chose Include (DM-01KYWWM8M963DE79XAZE7EZXC9).
- **Alternatives considered**: a separate test file (rejected: new
  asset/validation surface for no isolation benefit).

## R-9: Witness observation schema evolution

- **Decision**: Bump `CREST_EFFECTS_AND_BUSES_OBSERVATION` to
  `schemaVersion: 2`, adding `fourthEntryEndToEndExercised` and
  `carryOverWrongEngineIdentityRefused`.
- **Rationale**: The deterministic observation is not retained live evidence;
  the add-only constraint (C-001) governs live-scene checkpoints only. An
  explicit version bump keeps consumers honest.
- **Alternatives considered**: keeping schemaVersion 1 with added fields
  (rejected: silent schema drift is the pattern the proof model exists to
  prevent).

## R-10: Amendment discipline for parent artifacts

- **Decision**: `acceptance-matrix.json` and `mission-review.md` of the
  parent mission are amended add/append-style: new rows/addendum sections
  reference the refreshed evidence and disposition each of the 7 open items;
  no recorded grading or history is edited in place except where the parent
  review itself declared the supersession (DRIFT-6 note already present).
- **Rationale**: C-008; recorded history is evidence.
- **Alternatives considered**: a fresh acceptance matrix in this mission's
  dir only (rejected: FR-006 requires the parent's record to be made whole —
  the gap lives there).
