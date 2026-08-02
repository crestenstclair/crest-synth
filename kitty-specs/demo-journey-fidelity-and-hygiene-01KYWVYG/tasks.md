# Tasks: Phase 3 Demo Journey Fidelity and Hygiene

**Mission**: `demo-journey-fidelity-and-hygiene-01KYWVYG`
**Input**: `kitty-specs/demo-journey-fidelity-and-hygiene-01KYWVYG/{spec.md,plan.md,research.md,occurrence_map.yaml}`
**Branch contract**: planned on `feat/expandable-effects-and-bus-topology`; completed changes merge back into `feat/expandable-effects-and-bus-topology`.
**Bulk edit**: `change_mode: bulk_edit` — every diff is governed by the approved `occurrence_map.yaml` (post_effects → effect_slots retirement; serialized keys / CLI names / CREST_* markers do_not_change; guard fixture preserved).

## Subtask Index

*Reference table only — completion is event-sourced via `spec-kitty agent tasks mark-status`, never by editing rows.*

| ID | Description | WP | Parallel |
|----|-------------|----|----------|
| T001 | Slot-row journey support steps (PATCH focus + verify) | WP01 | [P] |
| T002 | Adjacent-choice occupancy cycling replaces slot injections | WP01 | |
| T003 | Audible occupant scalar edit from the PATCH page | WP01 | |
| T004 | MIXER return-row journeys replace return injections | WP01 | |
| T005 | Documented injection exception at the controlled rejection | WP01 | |
| T006 | Add-only checkpoint-identity assertions in deterministic coverage | WP01 | |
| T007 | crest_synth demo-observation mirror checks updated | WP01 | |
| T008 | Migrate app_state + semantic_resolver call sites | WP02 | [P] |
| T009 | Migrate semantic_graphical_view_model + patch_page_projection | WP02 | |
| T010 | Migrate event_record + serialized_state | WP02 | |
| T011 | Per-slot assertions + stale WP-comment cleanup (control files) | WP02 | |
| T012 | Migrate parameter_snapshot call sites | WP03 | [P] |
| T013 | Migrate demo_scene + exhaustive_gui_demo | WP03 | |
| T014 | Migrate live_demo_runner | WP03 | |
| T015 | Migrate tests/static_patch_effect.rs + tests/live_demo_scene.rs | WP03 | |
| T016 | Stale WP-comment cleanup (owned testing/snapshot files) | WP03 | |
| T017 | Gapped-chain round-trip regression test (test-first) | WP04 | [P] |
| T018 | Composition-root round-trip rebuilds from effect_slots | WP04 | |
| T019 | Propagate default-return composition errors at the production root | WP04 | |
| T020 | Stale WP-comment cleanup (shell/adapter files) | WP04 | |
| T021 | Delete post_effects()/with_post_effects() from Patch | WP05 | |
| T022 | Patch unit tests: gapped stability, no compacting surface | WP05 | |
| T023 | Repo-wide zero-caller verification + patch.rs comment cleanup | WP05 | |
| T024 | Optional measurement representation replaces unwrap_or(0) | WP06 | [P] |
| T025 | Report rendering distinguishes absent from measured zero | WP06 | |
| T026 | Unit tests incl. empty-observation-set absent case | WP06 | |
| T027 | RETURN-clear twin fixture (held notes through occupied return) | WP07 | [P] |
| T028 | Sample-exact continuity assertion vs untouched twin run | WP07 | |
| T029 | Wire declared selector return_clear_held_note_continuity | WP07 | |
| T030 | Migrate graph_preparation_worker call sites | WP08 | [P] |
| T031 | Prepared layout records per-position capability identity | WP08 | |
| T032 | Engine-rack carry-over identity guard | WP08 | |
| T033 | Post-effect + bus-return rack identity guards | WP08 | |
| T034 | carry_over_capability_identity tests + comment cleanup | WP08 | |
| T035 | Test-registry fourth-entry end-to-end fixture | WP09 | |
| T036 | Observation schemaVersion 2 + fourthEntryEndToEndExercised | WP09 | |
| T037 | carryOverWrongEngineIdentityRefused measured in the target | WP09 | |
| T038 | Two-run determinism and existing predicates stay green | WP09 | |
| T039 | Guard script tool gating + guard-test coverage | WP10 | [P] |
| T040 | Replace reverbSend fixture literals (guard fixture preserved) | WP10 | |
| T041 | DESIGN.md:204 "aux buses" → bus-return vocabulary | WP10 | |
| T042 | Stale WP-comment sweep in enumerated remainder files | WP10 | |
| T043 | Repo-wide final hygiene verification greps | WP10 | |
| T044 | Physical re-run of demo-live-effects-and-buses; capture evidence | WP11 | |
| T045 | Byte-level checkpoint-identity comparison vs parent evidence | WP11 | |
| T046 | Amend parent acceptance-matrix.json (add/append-only) | WP11 | |
| T047 | Amend parent review addendum: disposition all 7 open items | WP11 | |

## Phase A — Journey fidelity (User Story 1, P1)

### WP01 — Scene journey rework
**Prompt**: `tasks/WP01-scene-journey-rework.md` (~460 lines)
**Goal**: Slot and return occupancy changes travel the player's on-screen PATCH/MIXER journey in the retained scene, add-only; the controlled rejection stays injected with its inline exception note.
**Priority**: P1 — the DRIFT-6 HIGH finding; gate for all later phases.
**Independent test**: deterministic scene coverage asserts every occupancy change is preceded by focus-verified journey steps and that the pre-existing checkpoint identity set is unchanged; the physical run (WP11) shows it on screen.
**Requirements**: FR-001, FR-002, FR-003, FR-004 (NFR-004; C-001, C-002, C-003)
**Subtasks**:

T001 Slot-row journey support steps (PATCH focus + verify) (WP01)
T002 Adjacent-choice occupancy cycling replaces slot injections (WP01)
T003 Audible occupant scalar edit from the PATCH page (WP01)
T004 MIXER return-row journeys replace return injections (WP01)
T005 Documented injection exception at the controlled rejection (WP01)
T006 Add-only checkpoint-identity assertions in deterministic coverage (WP01)
T007 crest_synth demo-observation mirror checks updated (WP01)

**Dependencies**: none.
**Risks**: identity preservation must be proven by diffing the declared identity set (T006), not eyeballed; `src/bin/crest_synth.rs` mirror checks went stale in the parent's only rejection cycle (WP05) — treat as a first-class deliverable, not an afterthought.

## Phase B — Compact-view retirement (User Story 2, bulk edit)

### WP02 — Migration: control surfaces
**Prompt**: `tasks/WP02-migrate-control-surfaces.md` (~340 lines)
**Goal**: All `post_effects()` call sites in `src/control/` consume the per-position `effect_slots()` view; gaps survive projection and serialization paths.
**Priority**: P2
**Independent test**: `grep -rn "post_effects()" src/control/` returns nothing; module tests assert per-slot (gapped) shapes.
**Requirements**: FR-007 (C-007)
**Subtasks**:

T008 Migrate app_state + semantic_resolver call sites (WP02)
T009 Migrate semantic_graphical_view_model + patch_page_projection (WP02)
T010 Migrate event_record + serialized_state (WP02)
T011 Per-slot assertions + stale WP-comment cleanup (control files) (WP02)

**Dependencies**: none (accessor still exists until WP05).
**Risks**: serialized/projected key names are do_not_change (occurrence map) — shape handling changes, vocabulary does not.

### WP03 — Migration: testing & snapshot surfaces
**Prompt**: `tasks/WP03-migrate-testing-snapshot-surfaces.md` (~360 lines)
**Goal**: `parameter_snapshot.rs`, the retained deterministic scenes/runner, and the two caller test targets consume the per-position view.
**Priority**: P2
**Independent test**: `grep -rn "post_effects()"` over the owned files returns nothing; suite green.
**Requirements**: FR-007 (C-007)
**Subtasks**:

T012 Migrate parameter_snapshot call sites (WP03)
T013 Migrate demo_scene + exhaustive_gui_demo (WP03)
T014 Migrate live_demo_runner (WP03)
T015 Migrate tests/static_patch_effect.rs + tests/live_demo_scene.rs (WP03)
T016 Stale WP-comment cleanup (owned testing/snapshot files) (WP03)

**Dependencies**: none.
**Risks**: the snapshot's widened layout and leaf descriptor vocabulary are frozen (serialized_keys do_not_change); only the accessor call shape changes.

### WP04 — Composition root: gap preservation & loud defaults
**Prompt**: `tasks/WP04-composition-root-gaps-and-loud-defaults.md` (~380 lines)
**Goal**: The production composition root round-trips a gapped chain without re-compacting (test-first), and a failed default-return composition aborts startup visibly instead of `unwrap_or_default`.
**Priority**: P2
**Independent test**: new `production_runtime_contracts` cases — gapped chain survives the root round-trip exactly; failing default composition surfaces a typed error.
**Requirements**: FR-007, FR-008 (NFR-002; C-007)
**Subtasks**:

T017 Gapped-chain round-trip regression test (test-first) (WP04)
T018 Composition-root round-trip rebuilds from effect_slots (WP04)
T019 Propagate default-return composition errors at the production root (WP04)
T020 Stale WP-comment cleanup (shell/adapter files) (WP04)

**Dependencies**: none.
**Risks**: partial test registries legitimately rely on the permissive helper — keep permissiveness test-only; the production root propagates.

### WP05 — Retire the compact view
**Prompt**: `tasks/WP05-retire-compact-view.md` (~300 lines)
**Goal**: `post_effects()`, `with_post_effects()`, and `set_post_effect_config()` deleted; the Patch exposes exactly one (gapped, never-compacted) chain view, per the crest-spec invariant.
**Priority**: P2
**Independent test**: repo-wide grep zero; Patch unit tests prove gapped stability and the absence of any compacting surface; full suite green.
**Scope correction (2026-07-31, from WP03's review)**: the *constructor* `with_post_effects()` has ~18 occurrences across 12 files, ~6 of which no migration WP owned. Those unowned files moved into WP05's ownership, plus `src/real_time/audio_renderer.rs` (reassigned from WP10, which needed it migrated before the deletion). WP05 is a constructor sweep across ~8 files, not a two-line deletion.
**Requirements**: FR-007 (C-007)
**Subtasks**:

T021 Delete post_effects()/with_post_effects() from Patch (WP05)
T022 Patch unit tests: gapped stability, no compacting surface (WP05)
T023 Repo-wide zero-caller verification + patch.rs comment cleanup (WP05)

**Dependencies**: WP02, WP03, WP04, WP08 (every caller migrated first).
**Risks**: none new — deletion is the enforcement.

## Phase C — Proof & report hygiene (User Story 2)

### WP06 — Live report absent-vs-zero
**Prompt**: `tasks/WP06-live-report-absent-vs-zero.md` (~300 lines)
**Goal**: Derived live-report measurements are optional values; absent evidence renders as absent and can never satisfy a presence or performance expectation.
**Priority**: P2
**Independent test**: unit test feeds a run missing a measurement — report renders absent, expectation fails; measured zero still reads as zero.
**Requirements**: FR-010 (NFR-002)
**Subtasks**:

T024 Optional measurement representation replaces unwrap_or(0) (WP06)
T025 Report rendering distinguishes absent from measured zero (WP06)
T026 Unit tests incl. empty-observation-set absent case (WP06)

**Dependencies**: none (WP11 consumes the improved report).
**Risks**: derived summaries only — retained checkpoint identity fields are untouched.

### WP07 — RETURN-clear held-note continuity twin
**Prompt**: `tasks/WP07-return-clear-continuity-twin.md` (~280 lines)
**Goal**: A dedicated sample-exact twin test proves held notes survive clearing an occupied return, matching the slot-clear proof.
**Priority**: P2
**Independent test**: `cargo test return_clear_held_note_continuity` — the declared attached-validation selector.
**Requirements**: FR-009
**Subtasks**:

T027 RETURN-clear twin fixture (held notes through occupied return) (WP07)
T028 Sample-exact continuity assertion vs untouched twin run (WP07)
T029 Wire declared selector return_clear_held_note_continuity (WP07)

**Dependencies**: none.
**Risks**: low — mirrors the existing slot-clear proof structure at `tests/topology_change_lifecycle.rs:854,1018`.

## Phase D — Hardening (User Story 3, operator-included)

### WP08 — Per-position capability identity
**Prompt**: `tasks/WP08-per-position-capability-identity.md` (~420 lines)
**Goal**: The prepared layout records capability identity per position; all three racks' carry-over guards refuse a same-scalar-count wrong-capability candidate; worker call sites migrate to the per-position view.
**Priority**: P3 (operator-included hardening)
**Independent test**: `cargo test carry_over_capability_identity` — mismatch keeps the fresh instance.
**Requirements**: FR-016, FR-007 (C-004)
**Subtasks**:

T030 Migrate graph_preparation_worker call sites (WP08)
T031 Prepared layout records per-position capability identity (WP08)
T032 Engine-rack carry-over identity guard (WP08)
T033 Post-effect + bus-return rack identity guards (WP08)
T034 carry_over_capability_identity tests + comment cleanup (WP08)

**Dependencies**: none.
**Risks**: prepare-time code only — the callback contract (C-004) must be untouched; guards fail safe (mismatch → fresh instance, never bypass).

### WP09 — Fourth-entry fixture & observation schema v2
**Prompt**: `tasks/WP09-fourth-entry-fixture-observation-v2.md` (~360 lines)
**Goal**: The declared integration target registers a fourth test-registry entry, drives it end to end, and emits observation schemaVersion 2 with both new witness fields true.
**Priority**: P3 (operator-included hardening)
**Independent test**: `cargo test --release --test expandable_effects_and_bus_topology -- --nocapture` emits schemaVersion 2 with `fourthEntryEndToEndExercised: true` and `carryOverWrongEngineIdentityRefused: true`; witness predicates pass.
**Requirements**: FR-015 (FR-016 evidence surface)
**Subtasks**:

T035 Test-registry fourth-entry end-to-end fixture (WP09)
T036 Observation schemaVersion 2 + fourthEntryEndToEndExercised (WP09)
T037 carryOverWrongEngineIdentityRefused measured in the target (WP09)
T038 Two-run determinism and existing predicates stay green (WP09)

**Dependencies**: WP08 (identity mechanism must exist to measure refusal).
**Risks**: partial fixture coverage (slot-only or return-only) would recreate the inference gap SC-008 was graded PARTIAL for — the fixture must cover slot, return, preparation, projection, and render.

## Phase E — Hygiene & evidence (closure)

### WP10 — Hygiene sweep & guard gating
**Prompt**: `tasks/WP10-hygiene-sweep-and-guard-gating.md` (~380 lines)
**Goal**: Guard script fails loudly without its tools; reverbSend fixture literals and the "aux buses" wording are gone; stale WP-numbered comments removed repo-wide.
**Priority**: P2
**Independent test**: guard script exits non-zero naming a missing tool when one is absent; final greps: zero stale WP comments, zero reverbSend outside the guard's deliberate fixture, zero "aux buses".
**Requirements**: FR-011, FR-012, FR-013, FR-014 (NFR-002)
**Subtasks**:

T039 Guard script tool gating + guard-test coverage (WP10)
T040 Replace reverbSend fixture literals (guard fixture preserved) (WP10)
T041 DESIGN.md:204 "aux buses" → bus-return vocabulary (WP10)
T042 Stale WP-comment sweep in enumerated remainder files (WP10; `src/real_time/audio_renderer.rs` reassigned to WP05)
T043 Repo-wide final hygiene verification greps (WP10)

**Dependencies**: WP05 (final verification greps run against the fully migrated tree).
**Risks**: comment removal must not delete genuine constraint statements — rewrite those in place; the guard's deliberate detection fixture is protected by the occurrence map.

### WP11 — Physical re-run & amended acceptance artifacts
**Prompt**: `tasks/WP11-physical-rerun-amended-artifacts.md` (~380 lines)
**Goal**: The reworked scene runs on the physical rig; refreshed evidence recorded; byte-level identity comparison passes; the parent mission's acceptance matrix and review addendum are amended add/append-only with all 7 open items dispositioned.
**Priority**: P1 (the evidence gate)
**Independent test**: refreshed report complete (100% checkpoints, droppedRecords=0, clean teardown); identity diff shows 0 modified / 0 removed; both parent artifacts reference the new evidence.
**Requirements**: FR-005, FR-006 (NFR-001; C-001, C-008)
**Subtasks**:

T044 Physical re-run of demo-live-effects-and-buses; capture evidence (WP11)
T045 Byte-level checkpoint-identity comparison vs parent evidence (WP11)
T046 Amend parent acceptance-matrix.json (add/append-only) (WP11)
T047 Amend parent review addendum: disposition all 7 open items (WP11)

**Dependencies**: WP01, WP05, WP06, WP07, WP09, WP10 (everything lands first).
**Risks**: RECORDED-MANUAL — requires the physical rig and a real window/audio; if the host cannot run it, the WP blocks and asks the operator rather than substituting headless output (no silent fallback).

## Parallel Opportunities

- **Wave 1 (all independent)**: WP01, WP02, WP03, WP04, WP06, WP07, WP08, WP10(T039-T041 only) — up to 7 lanes.
- **Wave 2**: WP05 (after WP02/03/04/08), WP09 (after WP08).
- **Wave 3**: WP10 completion (final greps after WP05), then WP11 last.
- Within WPs, `[P]`-marked subtasks touch disjoint files.

## MVP Scope

WP01 alone heals the DRIFT-6 journey gap deterministically; WP01 + WP11 delivers the phase-gate evidence (User Story 1 end-to-end). The sweep (WP02–WP07, WP10) and hardening (WP08–WP09) complete the corrective gate.
