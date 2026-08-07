# Semantic Acceptance: Shell Hygiene Sweep

**Mission**: `shell-hygiene-01KZD0KR`
**Layer**: semantic (the obligation ledger deterministic acceptance cannot compute)
**Authored**: 2026-08-07
**Verdict**: **PASS**, with one requirement graded FAIL and recorded rather than reinterpreted.

Deterministic acceptance runs the declared validations. This layer discharges the obligations that require judgement: whether the crest-spec changed honestly, whether declarations and code still agree, and whether any proof was weakened to make a deletion pass.

---

## Obligation ledger — changed crest-spec declarations

The mission's only declaration change is the retirement authored at commit `7c7f1cf`, before any lane was claimed.

| Declaration | Change | Obligation | Discharged by |
|---|---|---|---|
| `requirement.component_state_ownership_boundary` | Retired the "return typed semantic UI intent" clause; kept and sharpened the passivity boundary ("none reaches AppState directly **or converts an input into a semantic action**") | The surviving boundary must still be enforced, not merely asserted | `tests/component_composition.rs` passivity scan, now binding three page sources instead of one (WP05 FR-007); reviewer-planted violations fail by name |
| `proof/invariants.yaml` component-passivity invariant | Same retirement, rationale records why | Invariant text must match what the code can actually violate | Reviewer traced the invariant to the scan that enforces it |
| `requirement.configurable_control_family` | Lost the "returning typed semantic intent" clause; controls present rather than act | `ComponentControl`/`control_for` must still resolve every kind/role pair | `validation.component_vocabulary`, 11 passed, unedited |
| `validation.webview_projection_shell` | Description deepened to name two new error-path proofs | The named proofs must exist and be falsifiable | T013 and T014, each independently made to fail by the reviewer |

No resource was added. No canonical ID was retired — the control-intent family was declared in prose, not as named resources (research D4).

---

## Drift guards

**1. Forbidden artifacts.** No `data-model.md`, no `contracts/` directory. The mission produced `spec.md`, `plan.md`, `research.md`, `quickstart.md`, `tasks.md`, `analysis-report.md`, `acceptance-matrix.json`, WP prompts and review artifacts. Verified by directory listing.

**2. Spec authored first.** Crest-spec `7c7f1cf` precedes WP03's lane commit `8937281` and every other lane claim. No WP edited `.kittify/crest-spec/` during implementation — each reviewer verified the lane diff contains no `.kittify/` path. C-002 is satisfied by construction rather than by discipline.

**3. Asset ownership.** Every touched file traces to a declared asset: `WebviewShellModules` (window.rs, projection_channel.rs, frame_stream.rs, webview/mod.rs), `ShellContextModules` (component_vocabulary.rs), `TestingContextModules` (live_demo_runner.rs, component_gallery_scene.rs), `WebviewProjectionShellAcceptanceTests` and `ComponentCompositionAcceptanceTests` (the two test files). No WP owned a file outside its asset's pattern; `finalize-tasks --validate-only` reported `ownership_warnings: []`.

**4. No silent fallback.** The mission's substance is the opposite of silent fallback: RISK-3 was a recorded fatal error that never surfaced, and it now terminates the process typed and nonzero. RISK-4 was a validation window that silently accepted unverified identities, and it now typed-rejects them. Neither fix introduces a swallowed error — WP01's reviewer confirmed the one residual `std::process::exit(0)` fallback inside tauri is unreachable from this call site (it fires only on `EventLoopClosed`, and the call is made from inside the live loop).

**5. Unchanged validation command surfaces.** All four declared validation commands are byte-identical to before the mission. Only their assertions deepened. `spec-kitty crest-spec doctor` reports OK: 7 contexts / 132 resources, 107 requirements, 32 project validations, 19 witnesses.

**6. Baselines not loosened.** WP04 added exactly one skip entry with zero deletion lines against the five pre-existing entries. WP02 had zero test-module deletions with all 13 pre-existing test bodies hash-identical. WP05 broadened the purity scan from one source to three with all nine needles byte-identical. The 50 ms p95 threshold was never touched; measured 8.3–14.0 ms across every run.

**7. Real-time boundaries.** C-001 held. The retired-identity store is shell-side and unreachable from the real-time callback, verified by ownership rather than by comment: no `Arc`/`Mutex`/`spawn` in `projection_channel.rs`, and the sole production construction is a local moved into the `run_return` event-loop closure. No reducer, RT, or projection-schema change anywhere in the mission diff.

**8. Evidence satisfied.** `evidence.graphical_application_shell_contract` and `evidence.component_vocabulary_contract` are covered by their declared validations, all green on the merged tree. The full gated live run reports `skipped: none`.

**9. Security constraints.** No CSP change. The gallery's policy-free serving was recorded as a deliberate, narrated exemption (FR-006) rather than silently fixed or silently left — the narration names `page_asset` as the structural reason the shipped window cannot reach gallery sources, and states the trigger that would invalidate the exemption. WP01's debug-only close-failure seam is `cfg(debug_assertions)`-gated with a compile-out guard test; `strings` on the release binary confirms zero occurrences of the env var.

---

## Deletions: did any proof die to make a deletion pass?

This is the question a hygiene mission must answer directly, because deleting code and deleting the proof of code look identical in a green suite.

Five test changes accompanied WP03's deletions. Two tests were **retired** because every assertion targeted a deleted symbol. Three were **narrowed**. The reviewer examined each and confirmed no surviving guarantee lost its proof. The one that mattered: `a_frame_recorded_from_another_thread_wakes_the_await` incidentally proved that clones share one underlying `Arc<StreamShared>` — a live doc claim on the **surviving** `QualifyingFrameStream`. That proof was relocated into a poll assertion, and the reviewer verified it would still fail if `Clone` deep-copied instead of sharing. It also carries independent redundant proof at `window.rs:596`.

Two residues are recorded rather than chain-deleted, per C-004: `StreamShared.arrived` (a `Condvar` whose only waiter was deleted) and `Display for FrameExpectation` (whose only consumer was the deleted error). Both are unconsumed rather than unproven-live; both are follow-ups.

---

## NFR-003 — WITHDRAWN by operator decision, not regraded

NFR-003 required net code reduction across `src/` and `tests/`. Measured outcome: WP03 removed **256 lines** of dead code; the mission is **~+1,300 net** overall, because closing RISK-3 and RISK-4 required roughly 1,100 lines of falsifiable proof for two error paths that previously had **zero** coverage, plus 39 lines of narration.

Presented to the operator at the accept gate. Their judgement (2026-08-07): *"it can add or remove code I just care about functionality."* The requirement is therefore **withdrawn** — recorded in `spec.md` with a strikethrough row and in the acceptance matrix under `withdrawn_requirements`, carrying the measurement, the date, and who decided.

The distinction matters and is deliberate. The row was **not** flipped from `fail` to `pass`, and the requirement was **not** rescoped to production code only (where it would have passed at −256). Either move would have made the record read better than the work. Instead the record states plainly: the requirement existed, it was measured, the measurement was unfavourable, and the operator judged it not to be a requirement of this mission. A reader can reconstruct exactly what happened.

The underlying lesson is recorded for future missions: a hygiene mission that closes an unproven error path will always add more proof than it removes dead code. NFR-003 should not be restated.

Every other requirement passes.

---

## New findings, recorded not absorbed

Six follow-ups surfaced during implementation and review. None blocks release; all are recorded so none is silently inherited:

1. **WP02 OBS-A** — `retire()`'s de-duplication guard is load-bearing (without it a stale identity shadows a current one and causes a *false* rejection) but unproven: removing it fails no test. The reviewer wrote the proving test and left it ready to paste.
2. **WP01 F4** — `std::env::set_var` in window.rs tests races `var_os` in the same 634-test binary. The base file deliberately had no env mutation anywhere. Did not flake across four runs; latent.
3. **WP04** — `PAINTED_ACK_IDENTITY_FIELDS`'s doc claims two consumers; it has one. All three identity lists agree exactly, so no proof is weakened — the comment is false, not the code.
4. **WP03** — the two unconsumed residues above.
5. **Pre-existing soak flake** — `receive_phase("meters")` fails roughly 1 run in 5 under load. Established as pre-existing and not WP04's: mechanistically in a section WP04 never touches, and WP04's measured max gap is *lower* than base.
6. **Inherited red gate** — `validation.format` had been failing since the previous mission. Fixed on the consolidated tree at `cf75033`; no lane could have fixed it without conflicting.
