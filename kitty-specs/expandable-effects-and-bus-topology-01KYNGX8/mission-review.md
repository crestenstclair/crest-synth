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

## Retrospective Reminder

The retrospective was captured at merge terminus: `kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/retrospective.yaml`
(note: this Spec Kitty version stores it in the mission dir, not `.kittify/missions/<id>/`), with
`RetrospectiveCaptured` in `status.events.jsonl` (2026-07-31T18:22:04Z, `has_findings`, 18 evidence refs);
no `RetrospectiveCaptureFailed` events. Surface findings with `spec-kitty retrospect summary`
(cross-mission, read-only) and `spec-kitty agent retrospect synthesize --mission expandable-effects-and-bus-topology-01KYNGX8`
(dry-run by default; add `--apply` to stage proposals).
