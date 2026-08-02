# Implementation Plan: Phase 3 Demo Journey Fidelity and Hygiene

**Branch**: `feat/expandable-effects-and-bus-topology` | **Date**: 2026-07-31 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `kitty-specs/demo-journey-fidelity-and-hygiene-01KYWVYG/spec.md`

## Summary

The Phase 3 retained scene (`make demo-live-effects-and-buses`) proves every
declared behavior audibly but performs slot and return occupancy changes by
injecting semantic actions directly (DRIFT-6, HIGH). This mission reworks the
scene so those changes travel the player's on-screen PATCH/MIXER journey using
the scene's existing support-script vocabulary, re-runs it on a physical
device with add-only evidence, and sweeps the post-merge review's seven open
items — compact-view retirement (bulk edit), loud startup composition
failure, the RETURN-clear continuity twin test, absent-vs-zero live
measurements, comment/wording/fixture cleanups, guard tool gating — plus the
two operator-included hardening items (fourth-entry end-to-end fixture,
per-position capability-identity carry-over refusal, witness schema v2).

## Technical Context

**Language/Version**: Rust (existing workspace toolchain; edition per `Cargo.toml` — no toolchain change)
**Primary Dependencies**: none added or upgraded; `Cargo.toml`/`Cargo.lock` unchanged
**Storage**: N/A (no persistence surfaces; recorded live evidence is file-based JSON as today)
**Testing**: `cargo test --all-targets`; declared exact-selector validations via `scripts/run_exact_test_validation.sh`; witness binary `crest-synth-witness`; retained live scene on physical hardware (RECORDED-MANUAL)
**Target Platform**: macOS development host + the physical live-demo rig used for the 2026-07-31 parent runs
**Project Type**: single Rust workspace, bounded-context source layout (`src/<context>/`)
**Performance Goals**: no regression — RT callback keeps zero allocation/locking/blocking/destruction; witness p99 timing checks stay green
**Constraints**: add-only checkpoint identity (C-001); production journey path only (C-002); single sanctioned injection (C-003); RT discipline unchanged (C-004); amend-don't-rewrite parent artifacts (C-008)
**Scale/Scope**: 36 `post_effects()` call sites across 15 files; ~66 stale WP-comment lines; 1 scene file rework; 2 new test surfaces; 1 script; 2 doc lines; parent-mission acceptance artifacts

## Charter Check

*GATE: passed 2026-07-31 (pre-Phase-0), re-checked after design.*

- Full mission rigor standard applies (charter, 2026-07-31): silent design
  drift is the costliest failure — this mission exists to close exactly such
  a drift (DRIFT-6) and its open-item tail. No waiver requested; proof gates
  ask the human (physical re-run is RECORDED-MANUAL by design).
- DIRECTIVE_031/DIRECTIVE_001 (context boundaries): all edits stay inside
  their owning contexts; no cross-context coupling is introduced.
- DIRECTIVE_035 (bulk edit): `change_mode: bulk_edit` set; approved
  `occurrence_map.yaml` accompanies this plan (decision
  DM-01KYWXHFPMFT3P092AW1H7E56C).
- DIRECTIVE_043 (close classes by construction): the compact-view retirement
  removes the two-truths seam entirely; the new Patch invariant (crest-spec)
  forbids reintroducing a compacting accessor; the guard script gains a
  structural tool-presence gate.
- DIRECTIVE_037 (living documentation): DESIGN.md wording, the parent
  acceptance matrix, and the review addendum are amended in the same mission
  that changes the behavior/evidence.
- No violations to justify; Complexity Tracking is empty.

## Crest-Spec Derivation

Authored in the crest-spec phase (commit `0328311`), before this plan.
`crest_spec_impact: structural` in `meta.json`.

**Canonical resources changed** (none added or retired; one assetKind + one asset added):
- `requirement.expandable_effects_behavioral_proof` — on-screen PATCH/MIXER
  journey required; single documented injection exception.
- `capability.expandable_effects_and_bus_topology` — `open_effect_registry`
  acceptance: end-to-end fourth-entry fixture; live-scene step observes the
  on-screen journeys.
- `aggregate.Synth.Patch` — the per-position slot view is the only chain
  representation; no compacting/position-erasing accessor may exist.
- `aggregate.Mixer.BusReturnBank` — default-occupancy composition failure
  surfaces at the production composition root; never silent empty returns.
- `valueObject.Testing.LiveDemoReport` — measurement summaries distinguish
  absent evidence from measured zero.
- `aggregate.RealTime.PreparedGraph` — layout records per-position capability
  identity; carry-over requires exact identity agreement, mismatch keeps the
  fresh instance.
- `validation.no_name_enumerated_identity` — tool-dependency gating declared.
- `validation.expandable_effects_and_bus_topology` — fourth-entry fixture and
  identity-refusal proof named.
- `witness.expandable_effects_and_bus_topology` — schema v2: adds
  `fourthEntryEndToEndExercised`, `carryOverWrongEngineIdentityRefused`
  predicates; resources gain `PreparedEngineRack`.
- New attached validations: `service.return_clear_held_note_continuity`
  (StructuralGraphCoordinator), `service.carry_over_capability_identity`
  (PreparedGraphBuilder).
- New `assetKind.validation-script` (`scripts/*`) + `asset.ValidationScripts`.

**Assets → files this mission produces/edits**:
- `TestingContextModules` → `src/testing/live_effects_and_buses_scene.rs`,
  `src/testing/live_demo_report.rs`, other `src/testing/*` touched by the
  migration.
- `SynthContextModules` → `src/synth/patch.rs` (compact-view deletion).
- `ControlContextModules` → `src/control/*` call-site migration +
  `state_tree.rs` fixture literals.
- `RealTimeContextModules` → `src/real_time/*` call-site migration +
  per-position identity in prepared graph/racks.
- `ShellContextModules` → `src/shell/standalone_application.rs` (round-trip
  fix, error propagation).
- `AdapterModules` → `src/adapter/production_effects.rs` (error propagation).
- `CrestSynthMain` / `BehavioralWitnessMain` → `src/bin/*.rs` (scene mirror
  checks, witness schema v2 emission).
- `BehavioralAcceptanceTests` (and sibling test assets) → `tests/*.rs`
  (twin test, carry-over identity test, fourth-entry fixture, migrated
  callers).
- `ValidationScripts` → `scripts/check_no_name_enumerated_identity.sh`.
- Docs (WP precedent: owned without asset coverage): `DESIGN.md:204`,
  parent-mission acceptance matrix + `mission-review.md` addendum.

**Validations/witnesses covering the change**: the 28 declared project checks
(unchanged set), `witness.expandable_effects_and_bus_topology` (schema v2),
the two new attached validations, `no_name_enumerated_identity` (now
tool-gated), and the RECORDED-MANUAL physical live run for
`evidence.expandable_effects_and_bus_topology_contract`.

**Forbidden artifacts**: no `data-model.md`, no `contracts/` (crest-spec
exists; they would fork canonical resources).

## Bulk Edit Classification

This mission retires the compacting accessor surface `post_effects()` /
`with_post_effects()` in favor of the canonical never-compacted
`effect_slots()` view — 36 call sites across 15 files — and removes leftover
`reverbSend` fixture literals plus the `DESIGN.md` "aux buses" wording.
Per-category rules live in [`occurrence_map.yaml`](occurrence_map.yaml)
(all 8 categories; approved by the operator). Key rulings: serialized keys
(`postEffects`, `preparedPostEffects` in StateTree/report vocabulary) are
**do_not_change** — they are retained evidence vocabulary under the add-only
checkpoint constraint; `tests/no_name_enumeration_guard.rs` keeps its
deliberate `reverbSend` detection fixture; parent-mission history and the
crest-spec are never term-scrubbed.

## Project Structure

### Documentation (this mission)

```
kitty-specs/demo-journey-fidelity-and-hygiene-01KYWVYG/
├── plan.md              # This file
├── research.md          # Phase 0 output (decisions + rationale)
├── quickstart.md        # Phase 1 output (gate-running guide)
├── occurrence_map.yaml  # Bulk-edit classification (approved)
└── tasks.md             # Phase 2 output (/spec-kitty.tasks — NOT created here)
```

### Source Code (repository root)

```
src/
├── synth/patch.rs                        # delete compact view; canonical effect_slots() only
├── control/{app_state,semantic_resolver,semantic_graphical_view_model,
│            patch_page_projection,event_record,serialized_state,
│            state_tree}.rs               # call-site migration; fixture literals
├── real_time/{parameter_snapshot,graph_preparation_worker,
│              prepared_engine_rack,prepared_post_effect_rack,
│              prepared_bus_return_rack,prepared_graph*}.rs
│                                         # migration + per-position identity
├── shell/standalone_application.rs       # round-trip fix; error propagation
├── adapter/production_effects.rs         # propagate default-return errors
├── testing/{live_effects_and_buses_scene,live_demo_report,
│            live_demo_runner,demo_scene,exhaustive_gui_demo}.rs
│                                         # journey rework; absent-vs-zero; migration
└── bin/{crest_synth,crest_synth_witness}.rs  # mirrors; witness schema v2

tests/
├── topology_change_lifecycle.rs          # return_clear_held_note_continuity twin
├── expandable_effects_and_bus_topology.rs # fourth-entry fixture; carry_over_capability_identity;
│                                          # observation schemaVersion 2
├── static_patch_effect.rs, live_demo_scene.rs  # migrated callers
└── no_name_enumeration_guard.rs          # tool-gating coverage (fixture preserved)

scripts/check_no_name_enumerated_identity.sh   # tool-dependency gate
DESIGN.md                                      # line 204 wording
kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/
├── acceptance-matrix.json                # amended (add/append-only)
└── mission-review.md                     # addendum amended (add/append-only)
```

**Structure Decision**: single Rust workspace with bounded-context modules —
unchanged; every edit lands in its owning context per the crest-spec.

## Design Notes (Phase 1, derived — no forked artifacts)

- **Journey mechanism**: the scene composes journeys from its existing
  support-script vocabulary (`FocusMixerTrack`, `VerifyMixerFocus`,
  `Event(Navigate/EnterSurface/...)`) exactly as the send walks already do.
  Slot journeys focus `PatchControlId::EffectSlot(n)` rows on PATCH and cycle
  occupancy with the adjacent-choice gesture; return journeys focus the MIXER
  return rows. The UI gesture resolves to the same structural intent the
  reducer already emits for `SetSlotOccupancy`/`SetReturnOccupancy`, so every
  existing transition/checkpoint identity is preserved byte-identically;
  journey/focus verification checkpoints are pure additions (C-001). The
  unknown-entry rejection keeps direct injection with an inline comment
  documenting that the UI cannot express an unknown entry (C-003).
- **Occupant parameter edit**: one installed occupant scalar (subject Patch's
  configured effect) is edited from the PATCH page with an audible
  checkpoint, reusing the established scalar-edit checkpoint pattern.
- **Compact-view retirement**: migrate call sites file-by-file to
  `effect_slots()`; the composition-root round-trip
  (`standalone_application.rs:1470`) rebuilds from the per-position view so a
  gapped chain survives exactly; then delete `post_effects()`/
  `with_post_effects()` so the class is closed by construction.
- **Absent-vs-zero**: live-report derived measurements
  (`frames_to_projection`, activation gap, blocks-to-audible at
  `live_demo_report.rs:872-886`) become explicit optional values rendered as
  absent when no observation exists; presence expectations fail on absent.
- **Per-position identity**: the prepared layout records the capability id
  per prepared position; the three racks' carry-over guards require exact
  identity agreement in addition to patch/slot/scalar-layout checks
  (prepare-time code only; callback untouched).
- **Witness schema v2**: the effects-and-buses observation adds
  `fourthEntryEndToEndExercised` and `carryOverWrongEngineIdentityRefused`
  and bumps `schemaVersion` to 2 (deterministic observation, not retained
  live evidence — the add-only constraint applies to live checkpoints).

## Complexity Tracking

*No charter violations; table intentionally empty.*

## Implementation Concern Map

> Implementation concerns are NOT work packages. `/spec-kitty.tasks`
> translates these into executable WPs.

### IC-01 — Scene journey rework

- **Purpose**: Make slot/return occupancy changes travel the on-screen
  PATCH/MIXER journey in the retained scene, add-only.
- **Relevant requirements**: FR-001..FR-004, NFR-004, C-001..C-003, SC-001, SC-002
- **Affected surfaces**: `src/testing/live_effects_and_buses_scene.rs`,
  `src/testing/live_demo_runner.rs` (support vocabulary if extension needed),
  `src/bin/crest_synth.rs` (demo-observation mirror checks — parent WP05
  regression area), `tests/effects_and_buses.rs` deterministic coverage.
- **Sequencing/depends-on**: none (but the physical re-run in IC-10 depends on this).
- **Risks**: checkpoint-identity preservation must be verified by diffing the
  declared identity set against the parent evidence; the WP05 rejection
  history shows the crest_synth mirror checks go stale easily.

### IC-02 — Compact-view retirement (bulk edit)

- **Purpose**: One chain representation; gaps survive every path.
- **Relevant requirements**: FR-007, C-007, SC-005
- **Affected surfaces**: `src/synth/patch.rs` + the 15 caller files
  (`src/real_time/*`, `src/control/*`, `src/shell/standalone_application.rs`,
  `src/testing/*`, `tests/static_patch_effect.rs`, `tests/live_demo_scene.rs`).
- **Sequencing/depends-on**: none; IC-01 touches overlapping testing files —
  coordinate ownership at task slicing.
- **Risks**: the round-trip re-compaction at `standalone_application.rs:1470`
  is the latent defect — needs a regression test with a gapped chain before
  the accessor is deleted (test-first).

### IC-03 — Loud default-return composition

- **Purpose**: Propagate composition errors at the production root instead of
  `unwrap_or_default`.
- **Relevant requirements**: FR-008, NFR-002, SC-006
- **Affected surfaces**: `src/adapter/production_effects.rs:89-91`,
  `src/shell/standalone_application.rs:715`.
- **Sequencing/depends-on**: none.
- **Risks**: partial test registries rely on the permissive helper — keep the
  permissive path test-only, production path propagating.

### IC-04 — RETURN-clear continuity twin test

- **Purpose**: Sample-exact held-note continuity proof for return clears.
- **Relevant requirements**: FR-009, SC-008
- **Affected surfaces**: `tests/topology_change_lifecycle.rs` (twin of the
  slot-clear proof at lines 854/1018), selector
  `return_clear_held_note_continuity` (declared attached validation).
- **Sequencing/depends-on**: none.
- **Risks**: low — the return path shares `carry_live_returns_from`.

### IC-05 — Absent-vs-zero live measurements

- **Purpose**: Missing measurement evidence must read as absent, never zero.
- **Relevant requirements**: FR-010, NFR-002
- **Affected surfaces**: `src/testing/live_demo_report.rs:872-886` and its
  render/summary consumers.
- **Sequencing/depends-on**: before IC-10 (the refreshed run's report uses it).
- **Risks**: report schema change must not alter retained checkpoint identity
  (summaries are derived fields, not checkpoint identity).

### IC-06 — Hygiene cleanups

- **Purpose**: Remove stale WP-numbered handoff comments (~66 lines), fix
  `DESIGN.md:204` "aux buses" wording, replace the two `reverbSend` fixture
  literals in `src/control/state_tree.rs:1389,1593`.
- **Relevant requirements**: FR-011..FR-013
- **Affected surfaces**: `src/**` comments, `DESIGN.md`,
  `src/control/state_tree.rs`.
- **Sequencing/depends-on**: after IC-02 lands in overlapping files (avoid
  churn); guard fixture in `tests/no_name_enumeration_guard.rs` is preserved
  per the occurrence map.
- **Risks**: comment removal must not delete genuine constraint statements —
  rewrite those in place.

### IC-07 — Guard script tool gating

- **Purpose**: The name-enumeration guard fails loudly when `rg`/`perl` are
  missing.
- **Relevant requirements**: FR-014, NFR-002
- **Affected surfaces**: `scripts/check_no_name_enumerated_identity.sh`,
  `tests/no_name_enumeration_guard.rs` (self-test coverage of the gate).
- **Sequencing/depends-on**: none.
- **Risks**: none notable.

### IC-08 — Fourth-entry end-to-end fixture

- **Purpose**: Demonstrate registry openness end to end (SC-008 of the parent
  graded PARTIAL → demonstration).
- **Relevant requirements**: FR-015
- **Affected surfaces**: `tests/expandable_effects_and_bus_topology.rs`
  (+ `src/testing` harness support as needed); witness field
  `fourthEntryEndToEndExercised`.
- **Sequencing/depends-on**: shares the observation schema bump with IC-09.
- **Risks**: fixture must drive slot AND return AND preparation AND
  projection AND render — partial coverage would recreate the inference gap.

### IC-09 — Per-position capability identity

- **Purpose**: Carry-over refuses a same-scalar-count wrong-engine candidate
  by recorded identity.
- **Relevant requirements**: FR-016, C-004
- **Affected surfaces**: `src/real_time/prepared_graph*.rs`,
  `prepared_engine_rack.rs:187-209`, `prepared_post_effect_rack.rs:222-256`,
  `prepared_bus_return_rack.rs:173-195`, witness harness (schemaVersion 2,
  `carryOverWrongEngineIdentityRefused`), selector
  `carry_over_capability_identity`.
- **Sequencing/depends-on**: coordinate with IC-02 (same files in
  `src/real_time/`).
- **Risks**: prepare-time only; must not touch callback paths (C-004).

### IC-10 — Physical re-run and amended acceptance artifacts

- **Purpose**: Refresh recorded evidence on hardware; amend the parent
  acceptance matrix and review addendum add-only; disposition all 7 open
  items (SC-007).
- **Relevant requirements**: FR-005, FR-006, NFR-001, C-008, SC-003, SC-004, SC-007
- **Affected surfaces**: recorded evidence artifacts,
  `kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/acceptance-matrix.json`,
  `kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/mission-review.md`.
- **Sequencing/depends-on**: IC-01, IC-05 (and green suite from all ICs);
  final concern before acceptance.
- **Risks**: RECORDED-MANUAL — needs the physical rig; byte-level checkpoint
  comparison against parent evidence is the SC-003 gate.
