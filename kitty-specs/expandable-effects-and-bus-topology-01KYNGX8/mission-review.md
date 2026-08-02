# Mission Review Report: expandable-effects-and-bus-topology-01KYNGX8

**Reviewer**: Claude (orchestrator) with three independent evidence agents (FR-trace, drift/gap, risk/security/gates)
**Date**: 2026-07-31
**Mission**: `expandable-effects-and-bus-topology-01KYNGX8` — Expandable effects and bus topology (Roadmap Phase 3, mission number 1)
**Baseline commit**: `7dc3c23` (parent of the first mission commit; `meta.json`'s `baseline_merge_commit` is a post-merge stamp for successor missions, not this mission's diff base)
**HEAD at review**: `e1cac875e3183e73d3ac857e630455ccba542075`
**WPs reviewed**: WP01–WP10 (all `done`; 10 lanes squash-merged as `9a7cd09`)
**Mission diff**: 215 files, +29,066 / −2,722

---

## Gate Results

Adaptation note: the skill's Gate 1–3 commands target the spec-kitty repo's own
pytest suites, which do not exist in crest-synth. The project's declared
equivalents (crest-spec proof model) were executed instead.

### Gate 1 — Full test suite
- Command: `cargo test --all-targets` — Exit 0 — **PASS** (492 passed / 0 failed across 26 binaries; non-vacuous).

### Gate 2 — Declared static + witness validations
- `bash scripts/check_no_name_enumerated_identity.sh` — Exit 0, declared marker emitted — **PASS**.
- `cargo test --release --test expandable_effects_and_bus_topology` — Exit 0, `CREST_EFFECTS_AND_BUSES_OBSERVATION` emitted — **PASS**.
- `crest-synth-witness --case refused-topology --mutant refused-topology-published` — Exit 1 (declared negative) — **PASS**; `--mutant none` — Exit 0, all predicates true — **PASS**.

### Gate 3 — Deterministic acceptance record
- `kitty-specs/<slug>/deterministic-acceptance.json`: `.passed == true`, **28/28 declared project checks passed** — **PASS**.

### Gate 4 — Issue matrix
- `issue-matrix.md` absent; `spec.md` references zero GitHub issues — **N/A** (correctly not scaffolded).

---

## Review-History Signal

157 events. Exactly one rejection cycle (WP05, `changes_requested`: stale demo-observation
mirror checks in `src/bin/crest_synth.rs`; fix verified in HEAD at lines 788/1049 —
indexed `/sends` array check, `masterGainDb`-only projection comparison). Zero forced
transitions, zero arbiter overrides, zero self-approvals; done-transitions by actor
`merge`. The WP08→WP10 arc (witness honestly measured `clearedSlotPreservedHeldNotes=false`;
operator ruled 2026-07-31: clears preserve held notes, installs/changes may cut;
WP10 delivered identity-guarded live-instance exchange at the block boundary) is the
mission's most significant history and is fully recorded in the crest-spec, SC-001,
and the WP10 file.

---

## FR Coverage Matrix (summary)

Full per-ID trace was performed for FR-001..019, NFR-001..008, C-001..011, SC-001..008.
**Every FR, NFR, and constraint is ADEQUATE** on production-path assertions;
hardware-gated items (FR-019, NFR-004, NFR-006, C-010, SC-007) are **RECORDED-MANUAL**
against the two 2026-07-31 physical live runs (131/131 checkpoints, `droppedRecords=0`,
zero false observation keys, clean teardown). No punted FRs: every FR has at least one
assertion beyond its `requirement_refs` mention, and the 11-mutant behavioral harness
(healthy exit 0 / mutant exit 1 per case) mechanically excludes the false-positive
test class for the routed behaviors. Standout proofs: FR-004/FR-005 sample-exact
composition and tail-block independence; FR-013 sample-exact continuity vs an
untouched twin run; FR-018 pre-reroute tail released on the receiving track; SC-001
carry-over byte-exact vs a never-installed reference.

One entry graded below ADEQUATE:

| ID | Adequacy | Note |
|----|----------|------|
| SC-008 (new registry entry = 0 structural changes) | **PARTIAL** | Proven by structural absence (leaf-schema scan + name-enumeration guard + occupant-generic projection), not by an end-to-end test that registers a fourth entry and drives it through slot/return/preparation/projection/render. Direction agrees with the acceptance matrix's `pass`; degree does not. |

Minor matrix bookkeeping: constraints C-001..C-011 are enforced by the 28
deterministic checks and reviews but are not enumerated as acceptance-matrix rows;
the SC-005 matrix wording ("muted and solo-excluded") slightly overstates the single
cited witness field (solo exclusion is covered at unit level, `mix_engine.rs:739`).

---

## Drift Findings

### DRIFT-1: Transitional compact-view shim survived its own retirement plan
**Type**: OWNERSHIP/SEAM DRIFT — **Severity**: MEDIUM — **Spec ref**: plan.md "open-closed by construction"; FR-001 stable slot positions
**Evidence**: `src/synth/patch.rs:84-90` (doc comment defers retirement to WP05/WP06 — both shipped); 13+ non-test callers of position-erasing `post_effects()` remain, incl. `src/real_time/graph_preparation_worker.rs:262,343,465` and `src/shell/standalone_application.rs:1470`, which round-trips a Patch through `with_post_effects(patch.post_effects().to_vec())`.
**Analysis**: Two representations of one truth persist (canonical `effect_slots()` vs compacting `post_effects()`). The round-trip at `standalone_application.rs:1470` would silently re-compact a gapped chain (slot 0 empty, slot 1 occupied → slot 0), violating the documented never-compacted contract. Latent today (no production flow builds gapped chains through that path), but this is the exact two-truths seam the mission existed to remove, and its named owners are closed. Follow-up: migrate callers to `effect_slots()` and delete the compact view.

### DRIFT-2: Silent degradation of production default bus returns
**Type**: SILENT-FALLBACK — **Severity**: MEDIUM — **Spec ref**: CLAUDE.md "no silent fallback"; FR-009, FR-014
**Evidence**: `src/adapter/production_effects.rs:89-91` — `production_default_bus_returns(registry).unwrap_or_default()`, consumed by the production composition root (`src/shell/standalone_application.rs:715`).
**Analysis**: A failed default-occupancy composition (a genuine defect) would boot the instrument with silent returns 0/1 and no visible reason — the failure mode FR-014 exists to surface. The permissive path is documented for partial test registries but is wired into the production root. Follow-up: propagate the error (`?`) at the production composition root.

### DRIFT-3: WP-numbered handoff comments embedded in durable code, partly stale
**Type**: DOC DRIFT — **Severity**: LOW — ~20 comments (e.g. `src/bin/crest_synth.rs:1062-1065`, `src/synth/patch.rs:86-88`) narrate a WP timeline the squash merge erased; at least two are already false (they defer to WP05/WP06, which shipped). One-pass comment cleanup recommended.

### DRIFT-4: Missing-measurement defaults read as strongest pass in live evidence
**Type**: VACUOUS-PROOF RISK — **Severity**: LOW — `src/testing/live_demo_report.rs:872-886`: `frames_to_projection`/`activation gap`/`blocks-to-audible` computed with `.max().unwrap_or(0)`; a regression that stops populating the fields would read as "0 frames" instead of "no data". Presence gates check checkpoints exist, not that they carried these measurements.

### DRIFT-5: Forbidden-term residue in prose/fixtures
**Type**: TERMINOLOGY — **Severity**: LOW — `DESIGN.md:204` still says "aux buses" (pre-existing, untouched by a mission that edited DESIGN.md); test fixture literal `reverbSend=0.4` at `src/control/state_tree.rs:1389,1593`. No production identifier violations (guard-enforced).

**Clean areas**: non-goal invasion (roster/modal/persistence/modulation/ceilings) — clean; all locked decisions (C-003 zero diff on top_level_context.rs; C-004 zero diff on Cargo.toml/lock; C-005 stage order preserved vs baseline; C-006 no representable return→send path; C-007 sixteen tracks) — clean; dead code — none (all new modules have production callers); spec-required retirements — all landed and grep-verified.

---

## Risk Findings

### RISK-1: Engine-identity attestation depth (defense-in-depth note)
**Type**: BOUNDARY — **Severity**: LOW — `PreparedGraphLayout` carries no engine-capability id for non-selected patches; the carry-over guards check patch_id/slot_id/scalar_count (`prepared_engine_rack.rs:187-209`, `prepared_post_effect_rack.rs:222-256`, `prepared_bus_return_rack.rs:173-195`) and fail safe (mismatch keeps the fresh instance). Exploiting this requires an upstream coordinator/preparer bug producing a same-scalar-count wrong engine at a non-selected position. Recorded as hardening, not defect.

### RISK-2: RETURN-clear held-note continuity has no dedicated sample-level test
**Type**: TEST-COVERAGE — **Severity**: LOW (accepted by WP10 review) — slot-clear continuity is proven byte-exactly (`tests/topology_change_lifecycle.rs:854,1018`); the return path shares `carry_live_returns_from`. Cheap twin test recommended in a follow-up.

**Not found**: callback-reachable panics (all `expect`/`unreachable!` candidates verified prepare-time/control-thread/const); TOCTOU in the one-in-flight lifecycle (monotonic-revision admission, exact-revision triple completion, stale-ack rejection — all test-pinned; one availability-only nit: `stage_replacement` doesn't `poll()` first, so a completed-but-unpolled in-flight yields a spurious busy refusal); merge-mangled exports — none.

---

## Silent Failure Candidates

| Location | Condition | Silent result | Spec impact |
|----------|-----------|---------------|-------------|
| `src/adapter/production_effects.rs:89-91` | default-return composition fails | empty bank (silent returns 0/1) | DRIFT-2 — FR-009/FR-014 |
| `src/testing/live_demo_report.rs:872-886` | measurement fields absent | max()=0 reads as instant | DRIFT-4 — NFR-008 evidence |
| Declared semantics (not findings): unoccupied return → silence; live/prepared mismatch → silence; refused change → no-op | — | — | Sanctioned by crest-spec invariants |

---

## Security Notes

| Finding | Location | Risk class | Recommendation |
|---------|----------|------------|----------------|
| Guard script masks missing tools (`rg`/`perl`) as "no candidates" via `\|\| true` | `scripts/check_no_name_enumerated_identity.sh` | VACUOUS-GATE (LOW) | Gate on `command -v rg perl`; mitigated today by `tests/no_name_enumeration_guard.rs` (incl. `--self-test`) running under `cargo test` |
| No subprocess/network/user-path surfaces introduced | mission diff | — | None needed |

---

## Final Verdict

**PASS WITH NOTES**

### Verdict rationale

Every FR, NFR, and constraint traces to production-path evidence or recorded
physical-device runs; both acceptance layers and all adapted hard gates pass; no
locked decision was violated; no non-goal was invaded; the single rejection cycle
(WP05) has its fix verified in HEAD; the held-notes contract was resolved by explicit
operator ruling and delivered with byte-exact proofs. No CRITICAL or HIGH findings
exist. The two MEDIUM findings (DRIFT-1 shim retirement, DRIFT-2 startup fallback)
are contained, latent-only, and suited to a small follow-up — they do not block
release.

### Open items (non-blocking)

1. DRIFT-1 — migrate `post_effects()` callers to `effect_slots()`; delete the compact view (owners: real_time worker/snapshot, shell root, testing).
2. DRIFT-2 — propagate default-return composition errors at the production root.
3. RISK-2 — add the RETURN-clear held-note sample-continuity twin test.
4. SC-008 — optional: an end-to-end "register a fourth entry" fixture to convert the structural inference into a demonstration.
5. DRIFT-3/4/5 — comment cleanup; `unwrap_or(0)` → explicit absent-evidence handling in live_demo_report; `DESIGN.md:204` "aux buses" wording; stale `reverbSend` test fixture literals.
6. RISK-1 — optional layout hardening: record engine-capability identity per position.
7. Guard script tool-presence check (security note).

## Addendum (2026-07-31, post-review operator finding)

### DRIFT-6: Retained scene bypasses the PATCH-view player journey
**Type**: DEMO-EVIDENCE GAP — **Severity**: HIGH (deferred-with-followup: ROADMAP "Current corrective gate — Phase 3 demo journey fidelity and mission hygiene")
**Spec reference**: User Story 1 ("moves focus to an effect slot row and cycles it"), FR-002, FR-003, T046 step 4, C-010
**Evidence**: `src/testing/live_effects_and_buses_scene.rs:267` — every slot change is a directly injected `SemanticAction::SetSlotOccupancy`; all return changes are injected `SetReturnOccupancy` literals; `PatchControlId::EffectSlot` appears nowhere in the scene; the only UI-driven topology interactions are the MIXER send walks and the PatchUtility reroute.
**Analysis**: The scene proves every declared behavior audibly but performs the slot/return
occupancy journey backstage, so the phase gate never demonstrates the PATCH view's new
functionality (slot rows, adjacent-choice cycling, descriptor-driven parameter rows) on
screen. T046 step 3's "through semantic actions and AppState::apply" was satisfied literally
while step 4's "see the focused control, the action, the resulting state" was not. The
reducer-level UI vocabulary is fully proven deterministically (`tests/semantic_focus_and_projection.rs`),
so this is an evidence gap in the retained scene, not a correctness hole — but the live
gate's FR-019/C-010 grading above (RECORDED-MANUAL, pass) is **superseded: inadequate for
the player journey** until the scene is reworked and re-run on hardware. Found by the
operator, missed by WP08's implementer, WP08's reviewer, and this review's first pass.

**Verdict impact**: remains **PASS WITH NOTES** solely because the finding is documented
and deferred with a concrete follow-up handle (the roadmap corrective gate, which also
carries this review's open items 1–7). Absent that deferral it would be FAIL under rule 8.

## Retrospective Reminder

The retrospective was captured at merge terminus: `kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/retrospective.yaml`
(note: this Spec Kitty version stores it in the mission dir, not `.kittify/missions/<id>/`), with
`RetrospectiveCaptured` in `status.events.jsonl` (2026-07-31T18:22:04Z, `has_findings`, 18 evidence refs);
no `RetrospectiveCaptureFailed` events. Surface findings with `spec-kitty retrospect summary`
(cross-mission, read-only) and `spec-kitty agent retrospect synthesize --mission expandable-effects-and-bus-topology-01KYNGX8`
(dry-run by default; add `--apply` to stage proposals).

## Addendum 2 (2026-08-01) — DRIFT-6 resolution, open-item disposition, and a new demo-scope finding

Mission `demo-journey-fidelity-and-hygiene-01KYWVYG` reworked the retained scene
and swept this review's open items. WP01–WP10 are approved and merged; WP11
carried the evidence gate. This section resolves the DRIFT-6 addendum above. It
appends to the record; nothing above it is rewritten.

### Scene rework

Every effect-slot and bus-return occupancy change now dispatches the
adjacent-choice gesture behind a focus-verified journey to the exact row, and at
least one occupant parameter is edited audibly from the PATCH page. The scene's
declared topology transitions grew 17 → 30. The single surviving direct
injection is the documented rejection (`Topology.refused`) — the UI cannot
request an unknown registry entry by design — asserted as the only one in
`tests/effects_and_buses.rs`.

### Refreshed physical evidence

`make demo-live-effects-and-buses` was run on the fully merged lane with a real
window, physical audio device, and the real MIDI fixture. **Process exit 0.**
The complete log is **committed**, not merely cited, at
`kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/evidence/wp11-t044-live-run.log`
(evidence commit `5238020`, 2026-08-01).

- `CREST_LIVE_SUMMARY` verbatim: `live demo complete: 105/105 editable
  parameters, 3/3 engine transitions, 7718 qualifying shell frames, 144
  checkpoints, 17462 events, 0 dropped, banks=1, instruments=15,
  soundfontPatches=8, braidsPatches=7, alternatingCapabilities=true,
  initialGraphRevision=1, graphRevision=30, engineSwitches=3, fallbacks=0,
  callbackAllocations=0, callbackDestructions=0, cleanup=true, activeNotes=0`
- Completeness: 144/144 checkpoints; `droppedRecords=0`, `lossless=true`
  (17,462 observed / 17,462 retained); **0** checkpoints with
  `audio_uninterrupted=false`; clean teardown (`cleanup=true`,
  `window_closed=true`, `stream_released=true`, `owned_graphs_remaining=0`,
  `active_notes_after_cleanup=0`).
- `CREST_EFFECTS_AND_BUSES_LIVE_OBSERVATION` carries 45 keys and **zero false
  values**, including `topology_checkpoints=30` (the parent recorded 17),
  `physical_audio_nonzero=true`, `controlled_rejection_observed=true`,
  `rejection_reason="invalidEffectConfig"`,
  `post_rejection_recovery_observed=true`, `send_isolation_exact=true`,
  `max_off_target_bus_dbfs=-200.0`, `unoccupied_return_silent=true`.
- **DRIFT-4 discharged on hardware**: the measurement fields are MEASURED, not
  defaulted — `frames_to_projection_max=1`, `activation_sequence_gap_max=3`,
  `render_blocks_to_audible_max=46`. A defaulted zero would have been a
  regression; none is present.
- Exactly **three** product effects appeared on screen (`effect.chorus`,
  `effect.delay`, `effect.reverb`). The test-only fourth registry entry
  `witness-tilt` occurs **0** times in the log, confirming on hardware that
  WP09's fourth entry never reaches the production registry.

### Identity comparison (add-only contract)

Baseline: `FROZEN_TOPOLOGY_IDENTITY_BASELINE` — 17 identities at
`tests/effects_and_buses.rs:59`, the exact pre-rework sequence, corroborated by
the acceptance matrix's own SC-007 row recording `topology_checkpoints=17` for
the parent's physical runs.

Method, reproducible against the committed log: extract lines beginning
`CREST_LIVE_CHECKPOINT `, strip the marker, parse each remainder as JSON, select
records where `.kind == "topology"`, project `.checkpoint.transition` in
emission order; then diff the baseline-member subsequence against the frozen
constant, and take the complement as the addition set.

**Result: 17/17 baseline identities preserved byte-identically and in order.
0 modified. 0 removed. 13 added. 30 total.** The 13 additions:
`SlotOccupant.scalarEdited`, `SlotFill.secondCycle1`, `SlotFill.thirdCycle1`,
`SlotFill.thirdCycle2`, `Return.contentChangedCycle1`,
`Return.emptyOccupiedCycle1`, `Return.emptyOccupiedCycle2`,
`Topology.recoveredAfterRefusalCycle1`, `Slot.startupOccupantRestoredCycle1`,
`Slot.thirdClearedCycle1`, `Slot.thirdClearedCycle2`,
`Return.emptyRestoredCycle1`, `Return.emptyRestoredCycle2`.

Recorded honestly: a byte-diff against the **parent's own** logs was impossible.
`t052-run.log` and `wp10-t059-live-run.log` were cited by filename in the
acceptance matrix and were never committed to any branch of this repository. The
frozen constant is the durable substitute, corroborated by the parent matrix's
own recorded `topology_checkpoints=17`. This amendment commits its log so the
same gap does not recur.

### FR-019 / C-010 grading — restored to adequate

On the evidence above — the committed run at
`kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/evidence/wp11-t044-live-run.log`,
exit 0, 144/144 checkpoints, zero dropped records, zero false observation keys,
clean teardown, and the 0-modified / 0-removed / 13-added identity comparison —
the DRIFT-6 note **"superseded: inadequate for the player journey" is
resolved**, and the FR-019 / C-010 RECORDED-MANUAL grading is **restored to
adequate**. The phase gate now demonstrates the PATCH view's slot rows,
adjacent-choice cycling, and descriptor-driven parameter edit on screen, and the
MIXER return-row walks, rather than performing them backstage.

### Open items 1–7 — disposition

All seven **CLOSED**; none deferred. SC-007's deferral allowance for the two
optional items (4 and 6) was not needed — both were delivered.

| # | Item | Disposition | Closing WP | Verified proof pointer |
| --- | --- | --- | --- | --- |
| 1 | DRIFT-1 compact view | **CLOSED** | WP02–WP05 | The compact-view symbol set is now **zero repo-wide**: `grep -rn "post_effects()\|with_post_effects(" --include="*.rs" .` (excluding `archive/`) → **0 hits**. `Patch::with_effect_slot` (`src/synth/patch.rs:180`) is the position-explicit replacement. The round-trip at the composition root is gone. Only `PatchInput::post_effects` (`src/control/event_record.rs:190`) survives, and solely as frozen **serialized** vocabulary — its own doc comment states it is "an output shape, never an addressable chain" — which T043 sanctions. |
| 2 | DRIFT-2 startup fallback | **CLOSED** | WP04 | `production_startup_bus_returns` (`src/adapter/production_effects.rs:93`) returns `Result`. The production composition root consumes it at `src/shell/standalone_application.rs:737-738` and propagates the typed error `ApplicationError::DefaultBusReturns` (`:289`, `:330`, `:392`) via `?`. The permissive `unwrap_or_default` survives only inside the documented TEST-only `startup_bus_returns` (`:110-111`), which is **unreachable from production** — its callers are unit tests, `src/testing/`, and `tests/`. |
| 3 | RISK-2 RETURN-clear held-note twin test | **CLOSED** | WP07 | `cargo test return_clear_held_note_continuity` → `return_clear_held_note_continuity_preserves_held_voices_sample_exactly` (`tests/topology_change_lifecycle.rs:1205`), passing in the full-suite run below. |
| 4 | SC-008 fourth-entry fixture (optional) | **CLOSED — delivered, not deferred** | WP09 | Release observation `fourthEntryEndToEndExercised: true` (schemaVersion 2) with `registryEntryAdditionStructuralChanges: 0`, satisfying the crest-spec `open_effect_registry` step-1 `observes` clause verbatim. The fourth entry is test-only (`tests/expandable_effects_and_bus_topology.rs`); the production composition root still builds the three-entry registry, so the **diff to production is zero** — corroborated on hardware by `witness-tilt` occurring 0 times in the live log. **SC-008 is therefore regraded from PARTIAL to a demonstration**: the structural inference recorded in the FR coverage matrix above is now backed by an end-to-end exercise. |
| 5 | DRIFT-3/4/5 cleanups | **CLOSED** | WP06, WP10 + per-WP sweeps | `grep -rn "WP0[0-9]\|WP10" src/ --include="*.rs"` → **0**. `grep -in "aux bus" DESIGN.md` → **0**. `grep -rn "reverbSend" src/ tests/` → **1**, the guard fixture literal only (`tests/no_name_enumeration_guard.rs:236`), as T043 specifies. DRIFT-4: `src/testing/live_demo_report.rs` measurements are `Option`-typed and distinguish absent from measured; **no `unwrap_or(0)` remains** in that file — and the hardware run above proves the fields carry real values (1 / 3 / 46), not defaults. |
| 6 | RISK-1 layout hardening (optional) | **CLOSED — delivered, not deferred** | WP08 | Per-position engine-capability identity: `PreparedGraphLayout::effect_capability_identity` (`src/real_time/prepared_graph.rs:435`), exercised by `measure_carry_over_identity_refusal` (`tests/expandable_effects_and_bus_topology.rs:1454`) and surfaced as `carryOverWrongEngineIdentityRefused: true`, asserted at `:1539`. |
| 7 | Guard script tool gating | **CLOSED** | WP10 | `require_tools()` (`scripts/check_no_name_enumerated_identity.sh:75-89`) tests `command -v` **per tool**, names each missing tool, and exits with a **distinct code 3** ("a missing tool is a failure, never a pass"), separating it from a clean pass (0) and a real violation (1). Covered by `tests/no_name_enumeration_guard.rs`. The reviewer reproduced the **pre-fix** script printing PASS with `rg` absent from `PATH`, confirming the vacuous-gate was real and is now closed. |

---

### LIMIT-1: The retained scene demonstrates the journey for ONE instrument only

**Type**: DEMO-SCOPE BOUND — **Severity**: MEDIUM (accepted for Phase 3, binding
on Phase 5) — **Found by**: the operator, at WP11 evidence review.

**Evidence**: the scene's subject is `patches.first()`
(`src/testing/live_effects_and_buses_scene.rs:284`, whose comment reads "The
demonstration subject: the first installed Patch"). Every effect-slot and
bus-return journey in the retained scene occurs on that single Patch. The live
run's `CREST_LIVE_COVERAGE` confirms it: every Patch-scoped editable parameter
in the expected set is `patch.1.*`, while all sixteen mixer tracks
(`track.T00`–`track.T0F`, level/pan/mute/solo/sends) and all fifteen installed
instruments are exercised for sends and routing.

**Root cause — there is no patch-switching gesture in the semantic vocabulary.**
`SemanticAction` (`src/control/semantic_action.rs:54`) is a closed union of
exactly eight kinds: `SelectContext`, `Navigate`, `Adjust`,
`SetInteractionMode`, `EnterSurface`, `Return`, `SetSlotOccupancy`,
`SetReturnOccupancy`. `SelectContext` takes a `TopLevelContext` and therefore
switches only PATCH↔MIXER. `FocusPath` (`src/control/semantic_focus.rs:205-213`)
carries a `patch_id: Option<PatchId>`, and `SetSlotOccupancy` carries a
`patch_id`, so focus and occupancy are both addressed **per-Patch** — but **no
`SelectPatch` / `NextPatch` semantic action exists to change which Patch is
focused.** (`LiveEnginePhase::SelectPatch` in
`src/testing/live_demo_runner.rs:473` is not a counterexample: it dispatches
`SelectContext(TopLevelContext::Patch)` and then locates the patch by id in the
model directly — a backstage lookup, exactly the pattern this mission set out to
remove.)

**Analysis — the bound is a consequence of this mission's own fix.** Before
WP01, the scene changed occupancy by backstage injection, and could have driven
effects on any Patch without ever needing a UI path to reach it. Requiring the
on-screen journey structurally pins the demonstration to whichever Patch the
scene starts on — the first one — because the UI offers no gesture to move to
another. **Making the demo honest is what exposed the missing patch selector.**
This is not a regression and not a defect in WP01; it is the true reach of the
current UI surface, now visible instead of hidden behind injection.

**Disposition**: **accepted for Phase 3**, which closes on its declared
behaviors — the phase's requirements concern slots, sends, buses, returns, and
topology lifecycle for a Patch, and every one of them is now demonstrated on
screen and on hardware. **Escalated to a Phase 5 entry condition** (see
`ROADMAP.md`, "Phase 5 — Functional Patch editor blockout"): Phase 5 must
deliver a patch-selection gesture in the semantic vocabulary, and its
`demo-live-patch-editor` scene must demonstrate the effect-slot journey on more
than one instrument.

---

### Deterministic re-verification on the merged lane (2026-08-01)

- `cargo test --all-targets` — exit 0 — **533 passed / 0 failed across 26
  targets**.
- `cargo clippy --all-targets -- -D warnings` — exit 0, zero warnings.
- `cargo fmt --all -- --check` — exit 0.
- Observation `schemaVersion: 2` with `fourthEntryEndToEndExercised: true`,
  `carryOverWrongEngineIdentityRefused: true`, `twoRunTraceEqual: true`,
  `callbackAllocations: 0`, `callbackDeallocations: 0`,
  `callbackDestructions: 0`, `activeNotesAtExit: 0`.
- **Moved numbers**: `retiredGraphsCollectedOffCallback` moved **8 → 15** (WP09
  drives seven further structural changes); the predicate is `gt 0`, so the
  grade is unaffected. This is the **only** pre-existing numeric that changed.

### Verdict impact

The DRIFT-6 deferral is discharged. The mission's verdict remains **PASS WITH
NOTES**, now on demonstrated rather than deferred grounds: the HIGH finding is
resolved with committed hardware evidence, all seven open items are closed, and
the one new finding (LIMIT-1) is a scope bound accepted for this phase with a
binding entry condition on Phase 5.
