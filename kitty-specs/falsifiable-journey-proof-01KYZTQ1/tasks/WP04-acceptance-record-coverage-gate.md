---
work_package_id: WP04
title: Acceptance-record coverage gate
dependencies: []
requirement_refs:
- FR-006
- FR-008
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T016
- T017
- T018
history:
- timestamp: '2026-08-02T00:10:35Z'
  actor: planner
  action: created from IC-05 (declared coverage gate)
agent_profile: implementer-ivan
authoritative_surface: scripts/
create_intent:
- scripts/check_acceptance_matrix_coverage.sh
execution_mode: code_change
mission_id: 01KYZTQ118MXZGD4MBCR99A978
mission_slug: falsifiable-journey-proof-01KYZTQ1
model: ''
owned_files:
- scripts/check_acceptance_matrix_coverage.sh
- tests/no_name_enumeration_guard.rs
priority: P3
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP04 – Acceptance-record coverage gate

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Make acceptance fail when a mission's acceptance record omits a requirement kind its
specification declares, so a whole class of requirements cannot go silently ungraded.

## Context: what actually happened

The predecessor mission `demo-journey-fidelity-and-hygiene-01KYWVYG` declared 16 FRs, 4 NFRs,
and 8 constraints. Its acceptance matrix graded **16 rows, all FR** — zero NFR rows, zero
constraint rows. Twelve of twenty-eight declared requirements went ungraded and nothing caught it.

**Be careful with the precedent.** An earlier version of that mission's review claimed the parent
mission had the identical gap and concluded a template defect. Counting the rows disproved it:

| Mission | Graded rows |
|---|---|
| `expandable-effects-and-bus-topology-01KYNGX8` | 43 — FR 21, **NFR 10**, SC 11, C 1 |
| `demo-journey-fidelity-and-hygiene-01KYWVYG` | 16 — FR 16, NFR 0, C 0 |

The parent graded its NFRs. What recurs across both is **constraint** under-grading. That
correction is recorded in the predecessor's review (commit `25444d7`) and in research R-004. The
remediation stands on the corrected facts.

Crest-spec commit `ad9960b` declares
`validation.acceptance_matrix_covers_all_requirement_kinds` as a project check, wired into
`completion.projectChecks` (29/29) so it gates acceptance.

## Constraints that bind this WP

- **Scan the mission being accepted, only** (recorded decision). Do not scan siblings or archived
  missions. A sibling-scanning gate would fail today on
  `expandable-effects-and-bus-topology-01KYNGX8` (1 constraint row against ~11 declared) and would
  import a backfill this mission was not chartered for.
- **No allowlist / grandfather mechanism.** That option was considered and rejected: an allowlist
  is a silence mechanism, exactly the shape that lets a future author opt out instead of grading.
- **Tool gating is mandatory.** Absent tooling must never read as a pass. This is the same
  contract `scripts/check_no_name_enumerated_identity.sh` already implements — read it first and
  follow it rather than inventing a second style.

## Subtasks

### T016 — Write the coverage script

**Purpose**: the executable gate.

**Steps**:

1. Read `scripts/check_no_name_enumerated_identity.sh` first. It is the reference implementation
   for: tool gating via a `require_tools()` helper, exit-code discipline, repo-root resolution by
   pure parameter expansion (`${BASH_SOURCE[0]%/*}` — note it resolves the root **before** the
   tool gate, so a missing `dirname` cannot break it), and the declared success marker.
2. Write `scripts/check_acceptance_matrix_coverage.sh` to:
   - resolve the mission under evaluation;
   - read the requirement kinds its `spec.md` **declares** (FR / NFR / C tables);
   - read the kinds its `acceptance-matrix.json` **grades** (`criteria[].criterion_id` prefixes);
   - exit non-zero naming any kind declared but not graded;
   - emit exactly
     `CREST_STATIC_VALIDATION acceptance_matrix_covers_all_requirement_kinds passed`
     on success. The declared validation asserts on this exact string
     (`.kittify/crest-spec/proof/validations.yaml`) — a typo here silently fails acceptance.
3. Gate on every tool you use before scanning. Exit non-zero naming the missing tool. Use a
   distinct exit code for the tool-gate failure so it is not confused with a coverage failure —
   the reference script uses 0/1/2/3; follow its numbering.
4. A mission that declares no NFRs at all must **pass**, not fail. The rule is "grade every kind
   you declare", not "declare every kind".
5. Keep it POSIX-portable bash consistent with the existing script. No new runtime dependency.

**Files**: `scripts/check_acceptance_matrix_coverage.sh` (new)

**Validation**:
- Run against this mission's own spec (10 FR, 6 NFR, 7 C) — before WP06 authors the matrix, it
  should **fail**, naming the missing kinds. That is correct behavior, not a bug.
- Run against `expandable-effects-and-bus-topology-01KYNGX8` manually to sanity-check parsing:
  it grades FR, NFR, SC, and one C. Do not wire it in — this is a parsing check only.
- `bash -n` clean; the script is executable.

**Edge cases**:
- A spec with a declared-but-empty requirement table (headers, no rows) declares nothing of that
  kind — must pass.
- A matrix with `SC-` rows: success criteria are not a requirement *kind* in the spec's tables.
  Decide whether they count and comment the choice; the requirement text names "functional,
  nonfunctional, and constraint".
- Missing `acceptance-matrix.json` entirely: fail, naming the absence. A missing record is not
  full coverage.

### T017 — Tool-gating twin test

**Purpose**: prove the script fails loudly when its tooling is absent, rather than reporting no
findings.

**Steps**:

1. `tests/no_name_enumeration_guard.rs` already contains this pattern for the existing script
   (added by the predecessor mission under its FR-014). Read it and extend it in the same shape
   for the new script.
2. Cover: healthy path exits 0 with the marker; a deliberately emptied `PATH` (or a shadowed
   required tool) exits with the tool-gate code and names the tool.
3. When invoking the script from the test, **never pipe through `head`/`tail`** — the pipe
   reports the pager's exit status. Capture the output and check the exit code directly.

**Files**: `tests/no_name_enumeration_guard.rs`

**Validation**:
- Both cases assert on the exit code, not only on stdout.
- The existing tests for the other script still pass unchanged.

### T018 — Falsification: strip a requirement kind

**Purpose**: observe the gate failing. Spec C-005.

**Steps**:

1. Copy a real acceptance matrix to a scratch location and delete every NFR row.
2. Run the script against it and observe a non-zero exit naming the NFR kind.
3. Restore/discard the scratch copy. **Do not mutate a committed matrix** — unlike WP02 and WP03,
   the mutation here is on a copy, because the real records are mission history.
4. Also confirm the positive direction: a matrix grading every declared kind exits 0 with the
   marker.
5. Write the record to
   `kitty-specs/falsifiable-journey-proof-01KYZTQ1/evidence/falsification/guard-matrix-coverage.md`:
   the mutation, the command, the observed non-zero exit and message, and the observed pass.

**Files**: evidence file is a recorded out-of-map edit — rationale: "mission falsification
record; kitty-specs paths are non-declarable by rule".

**Validation**:
- Non-zero exit naming the stripped kind; zero exit on the intact matrix.
- No committed matrix was modified — `git status` clean.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch is also the final
merge target. This WP has **no dependencies** and can run in parallel with WP01 and WP03 — enter
the lane the runtime computes from `lanes.json`.

You own `tests/no_name_enumeration_guard.rs`. No other WP touches it. WP02 owns
`tests/effects_and_buses.rs` and WP05 owns `tests/expandable_effects_and_bus_topology.rs` — stay
out of both.

## Test Strategy

Deterministic. The script is exercised by T017's twin test and by T018's falsification. Note that
until WP06 authors this mission's acceptance matrix, the gate legitimately fails on this mission
— that is the gate working, and WP06 closes it.

## Definition of Done

- `scripts/check_acceptance_matrix_coverage.sh` exists, is executable, gates on its tools with a
  distinct exit code, and emits the exact declared marker on success.
- It scans only the mission under evaluation; no sibling scan, no allowlist.
- A mission declaring no NFRs passes; a mission omitting a declared kind fails naming it.
- The twin test covers healthy and tool-missing paths by exit code.
- A falsification record exists showing a non-zero exit on a stripped copy, with no committed
  matrix modified.
- `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
  all exit 0, none piped.

## Reviewer Guidance

- **Check the marker string character-for-character** against
  `.kittify/crest-spec/proof/validations.yaml`. A typo makes the declared validation fail at
  acceptance in a way that looks like a coverage failure.
- Re-run the falsification on your own copy.
- Confirm the script does not scan sibling missions — grep it for any glob over `kitty-specs/*`.
- Confirm no allowlist or skip-list exists anywhere in the script.
- Confirm the repo-root resolution happens before the tool gate, matching the reference script;
  the reverse order is the defect the predecessor mission already fixed once.
- Confirm a no-NFR mission passes rather than failing.
