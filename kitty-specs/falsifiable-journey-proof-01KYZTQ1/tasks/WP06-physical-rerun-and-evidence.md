---
work_package_id: WP06
title: Physical re-run, evidence, and acceptance record
dependencies:
- WP01
- WP02
- WP03
- WP04
- WP05
requirement_refs:
- FR-009
- FR-010
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts were generated on feat/expandable-effects-and-bus-topology, which is also the final merge target. During /spec-kitty.implement this WP branches from the lane the runtime computes from lanes.json; completed changes merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T023
- T024
- T025
- T026
- T027
history:
- timestamp: '2026-08-02T00:10:35Z'
  actor: planner
  action: created from IC-08 (evidence gate) and the mission's own acceptance record
agent_profile: implementer-ivan
authoritative_surface: ROADMAP.md
create_intent: []
execution_mode: code_change
mission_id: 01KYZTQ118MXZGD4MBCR99A978
mission_slug: falsifiable-journey-proof-01KYZTQ1
model: ''
owned_files:
- ROADMAP.md
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP06 – Physical re-run, evidence, and acceptance record

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Demonstrate the strengthened record on the real rig, prove the frozen checkpoint-identity
contract held byte-for-byte, and author this mission's acceptance record so it passes the very
gate WP04 built.

This is the mission's evidence gate. Nothing is demonstrated until this runs.

## Context

The predecessor mission line's central lesson: **referencing evidence is not retaining it.** The
parent mission cited two live-run logs (`t052-run.log`, `wp10-t059-live-run.log`) that are
committed in no branch and do not exist on this host — its physical claims are unauditable today.
The successor committed its log
(`kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/evidence/wp11-t044-live-run.log`,
493 KB), which is why the identity comparison is possible at all. Commit yours.

**Run the demo yourself.** The rule that physical proof gates "ask the human" is a prohibition on
*fabricating or substituting* evidence — never on *executing the program*. Execute the run,
capture it, parse it. What genuinely needs a human is the visual judgment no log can carry
(does it look right on screen); ask for that specifically, after running, not instead of running.

## Constraints that bind this WP

- **Physical evidence is never substituted (spec C-006).** No headless output stands in for a
  physical run. If the rig cannot be driven, stop and report — do not improvise.
- **Add-only checkpoint identity (spec C-001).** `FROZEN_TOPOLOGY_IDENTITY_BASELINE`
  (`tests/effects_and_buses.rs:59`, 17 entries) must reproduce exactly: 0 modified, 0 removed,
  additions enumerated.
- **Run from the merged lane worktree**, never the repo root. The root checkout lacks unmerged
  lane work and would demonstrate the *old* behavior. This mistake has already happened once in
  this mission line — verify with a grep for a known new symbol before you run.
- **Never pipe a run or a test through `head`/`tail`.** The pipe reports the pager's exit status.
  Redirect instead: `make demo-live-effects-and-buses > run.log 2>&1`. A "green" recorded through
  a pipe is unreliable, and that mistake has also already happened once here.

## Subtasks

### T023 — Deterministic preflight

**Purpose**: never touch hardware with a broken tree.

**Steps**:

1. Confirm you are in the merged lane worktree and it carries every dependency's work. Grep for a
   symbol each WP introduced — the dispatched-kind field (WP01), the record-reading guard (WP02),
   the stamping call (WP03), the new script (WP04), the v3 schema (WP05). If any is missing, stop:
   the lane is not merged and the run would prove nothing.
2. Run, each with its exit code checked directly and **none piped**:
   - `cargo test --all-targets`
   - `cargo test --release --test expandable_effects_and_bus_topology -- --nocapture`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo fmt --check`
   - `bash scripts/check_no_name_enumerated_identity.sh`
   - `bash scripts/check_acceptance_matrix_coverage.sh`
3. Confirm the release observation reports `schemaVersion: 3` with all six new predicates
   satisfied.
4. Confirm each of the four falsification records from WP02, WP03, and WP04 exists under
   `evidence/falsification/` and that each shows an observed non-zero exit. **Spot-check at least
   one by re-running it.** A missing or unverified falsification record means the mission's
   governing rule (spec C-005) was not met, and the physical run cannot rescue that.

**Files**: none (verification only)

**Validation**: every command exits 0; four falsification records present; at least one re-run.

### T024 — Physical run

**Purpose**: demonstrate the journey and the strengthened record on real hardware.

**Steps**:

1. From the merged lane worktree, with the real window, physical audio device, and real MIDI
   fixture connected:
   ```
   make demo-live-effects-and-buses > wp06-live-run.log 2>&1
   echo "exit=$?"
   ```
   Redirect, do not `tee` or pipe — the redirect preserves the real exit code.
2. Watch the screen while it runs. The journey must be visible: focus landing on each PATCH slot
   row before its occupancy cycles, the audible occupant edit from the PATCH page, the MIXER
   return-row walks, and the documented rejection's visible reason.
3. Expect exactly **three** product effects on screen. The fourth registry entry is test-only and
   lives solely in `tests/expandable_effects_and_bus_topology.rs`; its appearance in the real
   window would be a defect.
4. Parse the log yourself: `CREST_LIVE_CHECKPOINT`, `CREST_LIVE_SUMMARY`, `droppedRecords`, and
   the declared observation blocks. Confirm 100% of declared checkpoints, `droppedRecords = 0`,
   zero false observation keys, clean teardown, normal parent-process exit (spec NFR-004).
5. Confirm the new fields carry **measured** values, not defaulted zeros (spec NFR-005). A
   defaulted zero here is a WP01/WP05 regression — stop and report rather than accepting it.
6. Commit the log under
   `kitty-specs/falsifiable-journey-proof-01KYZTQ1/evidence/`.

**Files**: evidence log is a recorded out-of-map edit — rationale: "mission evidence; kitty-specs
paths are non-declarable by rule".

**Validation**: normal exit; 100% checkpoints; 0 dropped; new fields measured; log committed.

**If the rig cannot be driven**: stop and report. Do not substitute headless output, do not
fabricate a report, and do not mark this subtask done.

### T025 — Checkpoint-identity comparison

**Purpose**: prove the add-only contract held (spec C-001, FR-010).

**Steps**:

1. Extract the checkpoint identities from the refreshed run in order.
2. Filter them down to the members of `FROZEN_TOPOLOGY_IDENTITY_BASELINE`
   (`tests/effects_and_buses.rs:59`, 17 entries) and confirm the filtered sequence reproduces the
   baseline **exactly** — same members, same order. A renamed, removed, reordered, or duplicated
   identity fails here.
3. Enumerate every identity that is **not** in the baseline. Each must be an addition introduced
   by this mission or its predecessor, and you must be able to name why each exists.
4. Record the method and the result: 0 modified, 0 removed, N added (state N and list them).

**Files**: none (recorded in WP notes and the acceptance record)

**Validation**: 0 modified, 0 removed; additions enumerated and explained.

### T026 — Acceptance matrix covering every declared requirement kind

**Purpose**: this mission's own record must pass the gate it built. Dogfooding is the point — a
gate its own author routes around is worthless.

**Steps**:

1. `spec.md` declares **10 FR, 6 NFR, and 7 constraints**. Grade all 23, plus the 7 success
   criteria. This is the first mission in this line whose matrix covers every kind.
2. Follow the existing matrix structure (`criteria[]` with `criterion_id`, `description`,
   `proof_type`, `evidence`, `pass_fail`, `verified_by`, `verified_at`, `notes`) — read a prior
   mission's file before writing.
3. Cite **real, checkable** evidence per row: a test selector, a command with its exit code, a
   log line, a grep result. Do not cite a criterion that is satisfied vacuously — that is the
   precise error the predecessor's review recorded against itself for FR-002, and repeating it
   here would be the third occurrence in this line.
4. For the two HIGH remediations, cite the falsification records: a guard's evidence is the
   observed failure, not the passing run.
5. Run `bash scripts/check_acceptance_matrix_coverage.sh` and confirm it now exits 0 with the
   declared marker. Before this subtask it legitimately fails — that is the gate working.

**Files**: acceptance matrix is a recorded out-of-map edit — rationale: "mission acceptance
record; kitty-specs paths are non-declarable by rule".

**Validation**: all 23 requirement rows plus success criteria graded; coverage script exits 0;
every cited evidence pointer resolves.

### T027 — Record remediation in ROADMAP.md

**Purpose**: close the loop on the corrective gate's proof-adequacy finding.

**Steps**:

1. `ROADMAP.md:75` carries "Corrective gate (CLOSED 2026-08-01) — Phase 3 demo journey fidelity
   and mission hygiene". Its behaviors closed; its post-merge review returned FAIL on proof
   adequacy.
2. Amend that section to record that the two HIGH proof-adequacy findings are now remediated and
   the guards are falsifiable, citing this mission. **Keep the section** — it is history and a
   permanent regression gate. Amend its status; do not delete or rewrite it.
3. Do not touch the Phase 5 entry condition (LIMIT-1, patch switching). It is explicitly out of
   scope for this mission (spec C-007) and remains open.

**Files**: `ROADMAP.md`

**Validation**: the gate section is amended, not replaced; Phase 5's entry condition is
unchanged; `git diff ROADMAP.md` shows an amendment with the prior text intact.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch is also the final
merge target. This WP depends on **all five** other WPs — enter the lane the runtime computes
from `lanes.json`; its base must carry every dependency commit.

**Build the merged tree before trusting it.** The predecessor mission's merged tree did not
compile: one lane used an API another lane had deleted, and neither reviewer could see the other.
`cargo test --all-targets` on the merged base is T023's first job for exactly that reason.

**Ownership note**: WP `owned_files` cannot declare `kitty-specs/` paths (runtime rule
`INVALID_WP_OWNED_FILES_KITTY_SPECS`). This WP formally owns `ROADMAP.md`; the evidence log
(T024) and acceptance matrix (T026) are recorded out-of-map edits with the one-line rationale
given in each subtask.

## Test Strategy

The physical run **is** the test (RECORDED-MANUAL). Deterministic preflight comes first and is
non-negotiable. The identity comparison (T025) is the falsifiable core: treat a single modified
identity as a hard fail routed back to WP01.

## Definition of Done

- Preflight fully green, including both guard scripts, with all four falsification records
  present and at least one re-run.
- Physical run completed on the real rig: 100% of declared checkpoints, 0 dropped records, clean
  teardown, normal exit, new fields carrying measured values. Log committed.
- Identity comparison: 0 modified, 0 removed, additions enumerated.
- Acceptance matrix grades all 10 FR, 6 NFR, and 7 constraint rows plus success criteria, and the
  coverage script exits 0 on it.
- `ROADMAP.md` records the remediation with the gate section preserved as history.

## Reviewer Guidance

- **Verify the evidence is from a real run** — timestamps, device identifiers, report continuity,
  and a plausible wall-clock duration. This mission line exists because a demo claim outran its
  demonstration; do not let its own closure be the next instance.
- Confirm the log is **committed**, not merely referenced. Check it exists at the cited path in
  git, not just on disk.
- Re-run at least one falsification record from WP02/WP03/WP04 yourself.
- Confirm the acceptance matrix cites no vacuously-satisfied criterion. For each HIGH remediation
  row, confirm the cited evidence is the observed *failure*, not just a passing run.
- Confirm the coverage script's exit 0 was obtained without a pipe.
- Confirm `ROADMAP.md`'s Phase 5 entry condition is untouched.
- Confirm the run showed exactly three product effects on screen — a fourth would mean the
  test-only registry entry leaked into production.
