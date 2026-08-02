# Mission Review Report: demo-journey-fidelity-and-hygiene-01KYWVYG

**Reviewer**: Claude (orchestrator) with three independent evidence agents (FR-trace, drift/cross-WP, risk/security), plus two decisive falsifications run by the orchestrator directly
**Date**: 2026-08-01
**Mission**: `demo-journey-fidelity-and-hygiene-01KYWVYG` — Phase 3 Demo Journey Fidelity and Hygiene
**Diff base**: `8e97b82` (parent mission's tip; `meta.json`'s `baseline_merge_commit` is a post-merge stamp)
**HEAD at review**: `a9fe91c`
**WPs reviewed**: WP01–WP11 (all `done`; 11 lanes merged)
**Mission diff**: 112 files, +13,184 / −565

---

## Gate Results

The skill's Gates 1–3 target the spec-kitty repo's own pytest suites, which do not
exist in crest-synth. The project's declared equivalents were executed instead.

### Gate 1 — Full test suite
- `cargo test --all-targets` — exit 0 — **PASS** (533 passed / 0 failed, 26 targets, post-merge).

### Gate 2 — Declared static + witness validations
- `bash scripts/check_no_name_enumerated_identity.sh` — exit 0 + declared marker — **PASS**.
  Tool-gate verified: with `rg` absent, exit **3** naming the tool (checked without a pipe,
  because piping masks the exit status).
- `cargo test --release --test expandable_effects_and_bus_topology` — exit 0,
  `schemaVersion: 2` — **PASS**.
- `crest-synth-witness --case refused-topology --mutant refused-topology-published` — exit 1
  (declared negative); `--mutant none` — exit 0 — **PASS**.

### Gate 3 — Deterministic acceptance record
- `deterministic-acceptance.json`: `passed: true`, **28/28 declared project checks** — **PASS**.

### Gate 4 — Issue matrix
- `issue-matrix.md` absent; `spec.md` references zero GitHub issues — **N/A**.

**All gates pass.** The verdict below is driven by proof-adequacy findings, not gate failures.

---

## Review-History Signal

11 WPs, 1 rejection cycle (WP08, missing truncation-refusal test, resolved cycle 2),
**0 self-approvals**, 2 forced transitions (both WP11). The WP11 forces are accounted
for: the generic lane guard blocks `kitty-specs/` edits, but WP11's entire deliverable
is `kitty-specs/` paths that `INVALID_WP_OWNED_FILES_KITTY_SPECS` makes non-declarable
as `owned_files`, and the guard's suggested remedy would have deleted the evidence log.
The recorded event shows `pre_review_gate: force_bypassed=false, no_new_failures` — no
proof gate was bypassed. Sanctioned by the `occurrence_map.yaml` exception for the parent
mission directory. **The guard is structurally incompatible with this WP shape and should
be fixed upstream.**

---

## FR Coverage Matrix (summary)

Full per-ID trace performed for FR-001..016, NFR-001..004, C-001..008, with **eight
falsifications** actually executed (implementation broken, test re-run, restored).

| Grade | Count | IDs |
|---|---|---|
| ADEQUATE | 19 | FR-004..016 (except as noted), NFR-001, NFR-003, NFR-004, C-001..C-004, C-007, C-008 |
| PARTIAL | 3 | FR-001, FR-003, NFR-002 |
| **FALSE_POSITIVE** | 1 | **FR-002** |
| Process-only (no test) | 2 | C-005, C-006 |

Falsification results — six of eight defects were caught; **two were not**:

| Broke | Caught? |
|---|---|
| Scene declaration `adjust: Some→None` on `SlotFill.first` | YES — `effects_and_buses.rs:213` |
| Composition-root rebuild by list index instead of slot identity | YES — `production_runtime_contracts.rs:946` |
| Live-report gate `unwrap_or(0)` reintroduced | YES — `live_demo_report.rs:2267` |
| Guard `require_tools` commented out | YES — 2 guard tests |
| Fourth-entry DSP neutered to pass-through | YES — `expandable_effects_and_bus_topology.rs:1537` |
| Rack carry-over identity conjunct dropped | YES — `prepared_engine_rack.rs:438` |
| **Runner dispatch → always inject (DRIFT-6 reverted)** | **NO — 4 targets stayed green** |
| **`SlotOccupant.scalarEdited` edit removed entirely** | **NO — green** |

---

## Drift Findings

### DRIFT-1: The journey-vs-injection mechanism is unfalsifiable
**Type**: PROOF GAP — recurrence of the exact class this mission exists to close
**Severity**: **HIGH**
**Spec ref**: FR-001, FR-003, C-002, C-003, SC-001; crest-spec
`requirement.expandable_effects_behavioral_proof` (amended this mission)
**Evidence**: The mechanism is correct — `src/testing/live_demo_runner.rs:959-964`
dispatches `AppEvent::Adjust(direction)` for journey transitions and reserves
`from_semantic_action` for the rejection. But **the orchestrator replaced that block with
an unconditional `AppEvent::from_semantic_action(action.clone())` — every occupancy change
back-injected, the journey reduced to decoration — and `cargo test --test effects_and_buses`
returned exit 0, 1 passed, 0 failed.** Three further targets also stayed green.

The guard at `tests/effects_and_buses.rs:177-213` asserts over the scene's *declaration*
(`transition.action()`, `transition.adjust()`, `transition.support_before()`), never over
what the runner dispatched. `LiveTopologyCheckpoint` (`src/testing/live_demo_checkpoint.rs`)
records `action: Option<SemanticAction>` — the declared expected result — with **no field
recording the `AppEvent` actually dispatched**. The live log confirms this: it stores
`"action":{"kind":"setSlotOccupancy"…}` regardless. No test on this path inspects
`app_loop.event_log().records()`, where `EventInput::Adjust` vs `EventInput::SetSlotOccupancy`
is exactly the discriminator (other targets do use that pattern —
`tests/exhaustive_demo_scene.rs:316` — so the capability existed and was not applied here).

**Analysis**: DRIFT-6 was "the scene claimed a journey it did not perform." This mission
closed that gap *in the runner*, by construction. It did not close it *in the proof*. A
future edit reverting the runner reintroduces DRIFT-6 with a fully green suite, an
identical live report, and byte-identical hardware evidence — and SC-001 ("the only direct
injection remaining is the single documented controlled rejection") would still be graded
`pass` from evidence structurally incapable of showing it. `plan.md:46` names DIRECTIVE_043
("close defect classes by construction") as this mission's governing directive; the cheap
closure was available and not taken.

**Remediation**: record the dispatched `EventInput` discriminant on the topology checkpoint,
or assert over the topology-phase event-log records, so a runner reversion fails the gate.

### DRIFT-2: FR-002's acceptance criterion cannot fail
**Type**: FALSE-POSITIVE TEST
**Severity**: **HIGH**
**Spec ref**: FR-002, SC-002; acceptance-matrix row FR-002 (graded `pass`)
**Evidence**: FR-002 requires "at least one occupant parameter is edited audibly from the
PATCH page." Its entire verification is `tests/effects_and_buses.rs:351-357`:
`outcome() == Accepted`, `audible_on_activated_graph()`, `audio_after().active_notes() > 0`.
**No assertion anywhere constrains that the scalar value changed.** Setting `adjust: None`
on the `SlotOccupant.scalarEdited` transition (`live_effects_and_buses_scene.rs:509-520`),
so that no edit is dispatched at all, leaves the suite green — all three assertions are
satisfied by the ambient probe note on the already-audible startup chorus chain.

**Analysis**: this is the "passing test, failing system" mode in its purest form. The
behavioral content of the requirement — that a parameter's value moved, audibly, as a
consequence of the on-screen edit — is unconstrained. The requirement is *actually
implemented* (the scene does dispatch a fine-step edit, and the physical run recorded it),
so this is a proof defect, not a behavior defect. But the acceptance-matrix `pass` for
FR-002 and the SC-002 claim both rest on an assertion that cannot fail.
**The orchestrator authored that matrix row and cited exactly this vacuous evidence — the
error is the reviewer's own and is recorded here rather than quietly corrected.**

**Remediation**: capture the occupant scalar's value before and after the edit checkpoint
and assert it changed by the descriptor's fine step, plus an audio delta attributable to
that parameter rather than to ambient fixture output.

### DRIFT-3: The acceptance matrix omits 12 of 28 declared requirements
**Type**: COVERAGE GAP — **Severity**: MEDIUM — **Spec ref**: spec.md NFR-001..004, C-001..008
**Evidence**: `kitty-specs/demo-journey-fidelity-and-hygiene-01KYWVYG/acceptance-matrix.json`
contains exactly FR-001..FR-016 and **zero** rows for the four NFRs and eight constraints,
though `spec.md` declares them as mandatory requirement rows with status fields.
**Analysis**: the scaffold generated only FR rows and the orchestrator filled only what was
scaffolded, so NFR/C compliance is asserted in prose (this report, the WP DoDs) but never
recorded as graded acceptance rows. The parent mission carried the same gap — its review
noted "constraints C-001..C-011 are not enumerated as acceptance-matrix rows" — so this is
a **recurring** bookkeeping defect, now twice observed.

> **Correction (2026-08-01, successor mission's planning phase).** The sentence above
> overstates the recurrence, and the "Open items" entry below repeats the overstatement.
> Counting graded rows: the parent grades 43 — FR 21, **NFR 10**, SC 11, C 1 — so it did
> *not* omit NFR grading. This mission grades 16 rows, all FR, against 4 declared NFRs and 8
> declared constraints. **The NFR omission is this mission's alone.** What recurs is
> constraint under-grading (parent: 1 row against ~11 declared), which is what the parent's
> quoted review sentence actually says. The finding's severity and remediation are unchanged
> — 12 of 28 declared rows went ungraded and nothing caught it — but it is not evidence of a
> template defect observed twice. Recorded here rather than silently amended: this report
> exists to name claims that outran their evidence, and this was one of mine.

### DRIFT-4: NFR-002's silent-fallback closure is incomplete on the evidence path
**Type**: SILENT-FALLBACK RESIDUE — **Severity**: MEDIUM — **Spec ref**: FR-008, NFR-002, SC-006
**Evidence**: `src/adapter/production_effects.rs:110-112` still exposes
`pub fn startup_bus_returns(...) -> BusReturnBank { production_startup_bus_returns(r).unwrap_or_default() }`.
The *instrument* boot path is correctly fixed (`standalone_application.rs:737-738` propagates
`ApplicationError::DefaultBusReturns`). But this permissive variant is `pub` in a production
adapter module and has a live caller at `src/testing/exhaustive_gui_demo.rs:2237` —
and `src/testing` is `pub mod testing;` (`src/lib.rs:8`), **not** `#[cfg(test)]`, reached by
the shipped binary through `make demo`.
**Analysis**: SC-006 claims "0 startup paths remain that degrade a failed default-return
composition into silent returns." True for the instrument; false for the demo/evidence path,
which can still silently show an empty return bank. Closure is guarded by doc prose, not by
construction (`pub(crate)`, `#[cfg(test)]`, or a test-registry newtype would close it).

### DRIFT-5: FR-011's sweep is narrower than claimed and net-regresses in `tests/`
**Type**: DOC DRIFT — **Severity**: LOW
**Evidence**: `src/` went 70 → 0 WP-numbered refs (claim verified). But `tests/` went
**14 → 16**: the mission newly minted WP markers at
`tests/expandable_effects_and_bus_topology.rs:15,22,806,832,844,1410,1441` and
`tests/topology_change_lifecycle.rs:1183`. Sixteen `T0xx` task refs also survive in `src/`
files the mission owned and edited (`parameter_snapshot.rs:1180,1367,1396,1456`;
`prepared_post_effect_rack.rs:725,788,906,973`). The closure claim
(`grep "WP0[0-9]\|WP10" src/` → 0) is literally true but silently narrows FR-011 from
"durable code" to "`src/`, WP-prefix only."

### DRIFT-6: Three retained checkpoint identities changed semantic meaning
**Type**: EVIDENCE-COMPARISON BLIND SPOT — **Severity**: LOW — **Spec ref**: C-001, SC-003
**Evidence**: identities are byte-identical (verified: 17/17 in order), but because
adjacent-choice cycling is non-wrapping, `Return.emptyOccupied` now names "replace
second_entry with third_entry" rather than "occupy a previously empty destination" (that
step moved to the new `Return.emptyOccupiedCycle1`). Same for `SlotFill.second`,
`SlotFill.third`, `Return.contentChanged`. Final occupancy is unchanged in every traced case
and the scene documents the mitigation.
**Analysis**: SC-003's declared method — a byte-level identity diff — is structurally blind
to this class. Future comparisons must not read identity equality as transition equality.

**Clean areas** (verified, not assumed): crest-spec conformance — every declared invariant
has realizing code (Patch single-representation, BusReturnBank loud composition,
LiveDemoReport absent-vs-zero, PreparedGraph per-position identity, guard tool-gating,
amended behavioral proof, witness schemaVersion 2); C-006 no scope growth (**zero added
files** in `src/`/`tests/`/`scripts/`; capacity constants unchanged; `witness-tilt` occurs
**0 times in `src/`** — the fourth entry is genuinely test-only); C-001 add-only identity
(guarded on both declared and emitted surfaces); C-002 no new backdoor (both read helpers
pre-date the mission); C-007 bulk-edit discipline (serialized keys, `make` targets, witness
selectors, and the guard's deliberate `reverbSend` fixture all intact; all three
operator-added occurrence-map exceptions used as scoped, none as blanket permission);
C-008 append-only (+413/−0 across the parent artifacts, original bytes an exact prefix).

**Cross-WP integration — clean.** This was the highest-priority hunt, because one break of
this class was already caught at merge time (WP09 called `Patch::with_post_effects`, which
WP05 deleted in a sibling lane; invisible to both reviewers, found only by building the
merged tree, fixed in WP11). No second instance: zero `owned_files` intersections across all
11 WPs, every changed file maps to exactly one owner, every declared owned file shows a real
change, `cargo check --all-targets` exit 0, and the one real semantic coupling (WP04's
identity-keyed round trip vs WP05's position-explicit constructor vs WP08's identity
recording) is consistent because occupant `slot_id` is always derived from
`slot.instance_identity()`.

---

## Risk Findings

### RISK-1: `slot_id` is not bound to its position; the round trip can silently relocate
**Type**: BOUNDARY / SILENT DRIFT — **Severity**: MEDIUM
**Location**: `src/synth/patch.rs:194-213`; `src/control/app_state.rs:48-57`;
`src/shell/standalone_application.rs:1510-1524`
**Trigger**: any Patch reaching `InstallPatches` with an occupant whose `slot_id` names a
different *valid* position (e.g. `slot_id = 3` at position 0).
**Analysis**: WP04's composition root reconstructs an occupant's position by matching
`config.slot_id()` against `EffectSlotIndex::ALL`'s `instance_identity()`. But
`set_slot_occupancy` enforces only *uniqueness* of `slot_id` within the Patch (verified at
`patch.rs:200-209`), and `validate_effect_slots` likewise checks duplicates only. Neither
enforces `config.slot_id() == index.instance_identity()`. A mismatched-but-valid identity is
therefore **silently relocated** — render order changes with no error — because the typed
`ApplicationError::RecordedEffectPosition` fires only when the identity names *no* position.
Unreachable today (construction sites derive identity from position, WP05 made the fixture
structural and pinned it, and the reducer enforces the binding for `SetSlotOccupancy`), but
the invariant rests on convention plus one fixture test rather than a type or a central gate.
This is the same silent-position-drift class the mission existed to close, and the mission's
own identity-keyed round trip is what made `slot_id` load-bearing for position.

### RISK-2: `ApplicationError::RecordedEffectPosition` has zero test coverage
**Type**: ERROR-PATH — **Severity**: MEDIUM — **Location**: `standalone_application.rs:1514-1522`
**Analysis**: production-reachable from both construction sites, but referenced nowhere in
`src/` or `tests/` beyond its definition, `Display`, `source`, and those sites. The only one
of the mission's four new typed errors that fails the "at least one test pins it" bar.

### RISK-3: Carry-over identity guard is fail-open when both sides are absent
**Type**: BOUNDARY — **Severity**: LOW — **Location**: `prepared_engine_rack.rs:241`,
`prepared_post_effect_rack.rs:306`, `prepared_bus_return_rack.rs:230`
**Analysis**: `None == None` compares equal, so an unstamped position would carry over on
patch/slot/scalar agreement alone — silently reverting to pre-mission behavior. Closed today
by construction (`PreparedGraphBuilder::build` stamps every occupied position or refuses the
graph), so the risk is future: any `PreparedGraph` path bypassing the builder disarms the
guard. A stricter design refuses carry-over when *either* side is `None`.

### RISK-4: The witness predicate's declared resources name a layer it does not exercise
**Type**: PROOF PRECISION — **Severity**: LOW
**Analysis**: `carryOverWrongEngineIdentityRefused` declares
`repairResources: [PreparedEngineRack, PreparedPostEffectRack, PreparedBusReturnRack]` but
measures admission-time `permits_replacement` → `IncompatibleLayout`, not the rack guards.
The test documents this candidly and refuses to invent absent plumbing — the right call —
but the predicate's declared resources should be aligned to the measured layer. Separately,
the field name says "Engine" while the measurement mismatches an *effect* capability
identity; the engine case is covered only by unit tests.

**Not found**: callback-reachable panics (all new `expect`/`panic!` verified control-side or
structurally unreachable; WP06's documented-panic accessors have zero ungated production
callers); RT-contract violations (`PositionCapabilityIdentity` is `Copy`, which the compiler
proves excludes `String`/`Vec`/`Box`/`Drop`; the 4.7 KB layout is never materialized on the
callback; worst case is 72 fixed-size byte compares; measured `callbackAllocations=0`,
`callbackDestructions=0`); new `unsafe` (zero); dependency changes (`Cargo.toml`/`lock`
untouched); TOCTOU in the one-in-flight lifecycle (every added comparison narrows admission;
every failure direction refuses).

---

## Silent Failure Candidates

| Location | Condition | Silent result | Spec impact |
|---|---|---|---|
| `adapter/production_effects.rs:110` | default bank fails to compose | empty bank | DRIFT-4 — reachable from `make demo` via `pub mod testing` |
| `scripts/check_no_name_enumerated_identity.sh:138` | inner `rg` errors (exit ≥2) | violation dismissed as a comment | LOW — outer `rg` distinguishes 1 from ≥2; the inner one does not |
| `real_time/prepared_graph.rs:78` | recorded bytes not valid UTF-8 | `""` | LOW — unreachable by construction; not used for the carry-over decision |
| racks ×3 | both identities `None` | carry-over proceeds | RISK-3 |

---

## Security Notes

| Finding | Location | Risk class | Recommendation |
|---|---|---|---|
| Guard tool-gate — **verified closed** | `scripts/check_no_name_enumerated_identity.sh` | VACUOUS-GATE | None. Empirically exercised: exit 3 naming each missing tool, no pass marker; exit codes 0/1/2/3 structurally disjoint; no `eval`/unquoted expansion; repo-root resolution cannot escape. The rewrite also closed a hole beyond its brief — previously an out-of-repo scan made `rg` exit 2 and the old `\|\| true` printed the pass marker anyway. |
| Inner-scanner failure reads as clean | same, line 138 | VACUOUS-GATE (LOW) | Apply the outer `rg`'s 1-vs-≥2 discrimination to the inner call. |
| No new subprocess / network / path-traversal surface | mission diff | — | None needed (verified: zero `Command::new`, `std::net`, `File::open` additions in `src/`). |

---

## Final Verdict

**FAIL**

### Verdict rationale

All gates pass, all 28 declared project checks pass, the physical evidence is genuine and
its every cited figure verified in the committed log, the cross-lane integration is clean,
the real-time contract is intact, and no locked decision or non-goal was violated. The code
this mission shipped is correct.

The verdict is FAIL on **proof adequacy**, on two HIGH findings that are the same defect
class this mission was chartered to eliminate. DRIFT-1: reverting the runner to backstage
injection — literally reintroducing DRIFT-6 — leaves the suite green, because the guard
asserts over the scene's declaration rather than the dispatched event. DRIFT-2: FR-002's
audible-edit criterion is satisfied by ambient fixture output whether or not the edit
happens at all. Both were confirmed by executing the defect, not by reading the code.

This mission exists because a demo claim outran its demonstration. It fixed that claim, and
then graded the fix with assertions that cannot fail — reproducing, one level up, the exact
failure it was created to close. Under the project charter, silent design drift is the
costliest failure, and `plan.md` invoked DIRECTIVE_043 (close classes by construction) as
governing. Accepting these as notes would repeat the parent mission's mistake of deferring a
proof gap and calling the gate closed.

The remediation is small and well understood: pin the dispatched input kind on the topology
checkpoint, and assert the occupant scalar's before/after value on the edit checkpoint.
Both were available during the mission.

### Open items (non-blocking, for the follow-up)

1. **DRIFT-3** — add NFR-001..004 and C-001..008 rows to the acceptance matrix. (See the
   correction under DRIFT-3: the parent graded its NFRs; only constraint under-grading
   recurs. This mission's NFR omission is its own.)
2. **DRIFT-4** — make the permissive `startup_bus_returns` unreachable by construction.
3. **DRIFT-5** — extend the comment sweep to `tests/` and to `T0xx` refs; the mission
   net-added WP markers there.
4. **DRIFT-6** — record that byte-identical checkpoint identities may name different
   transitions; SC-003's method is blind to it.
5. **RISK-1** — bind `slot_id` to its position at the write path (a type or a central gate),
   closing the silent-relocation half of the failure space.
6. **RISK-2** — pin `RecordedEffectPosition` with a test.
7. **RISK-3 / RISK-4** — refuse carry-over when either identity is absent; align the witness
   predicate's declared resources and field name with the layer measured.
8. **LIMIT-1 (recorded in the parent addendum, binding on Phase 5)** — the retained scene
   demonstrates the journey for one instrument only, because no patch-switching gesture
   exists in the semantic vocabulary. Phase 5 carries the entry condition.
9. **Tooling** — the lane guard blocking `kitty-specs/` edits is structurally incompatible
   with a WP whose deliverables are `kitty-specs/` paths (non-declarable as `owned_files`);
   it forced two overrides. Fix upstream so no future WP needs a force.

## Retrospective Reminder

The retrospective was captured at merge terminus:
`kitty-specs/demo-journey-fidelity-and-hygiene-01KYWVYG/retrospective.yaml` (this Spec Kitty
version stores it in the mission directory). Surface findings with
`spec-kitty retrospect summary` (cross-mission, read-only) and
`spec-kitty agent retrospect synthesize --mission demo-journey-fidelity-and-hygiene-01KYWVYG`
(dry-run by default; `--apply` to stage proposals).
