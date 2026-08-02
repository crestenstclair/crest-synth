# Tasks: Falsifiable Journey Proof

**Mission**: `falsifiable-journey-proof-01KYZTQ1`
**Planning base**: `feat/expandable-effects-and-bus-topology`
**Merge target**: `feat/expandable-effects-and-bus-topology`
**Generated**: 2026-08-02T00:10:35Z

Work packages derive from the crest-spec's `assets[]` (commit `ad9960b`). Every
`owned_files` entry traces to a declared asset's file pattern; see `plan.md`
§ Crest-Spec Derivation for the asset→file mapping.

## Governing rule

Spec **C-005**: no new guard is accepted until its failure has been observed under a
deliberate mutation and recorded. Every WP that creates a guard owns its own falsification
subtask — the demonstration lives with the guard, not in a separate cleanup package. A WP
whose guard has no recorded failure is not done, regardless of how green its suite is.

## Subtask Index

| ID | Description | WP | Parallel |
|----|-------------|----|----------|
| T001 | Add the dispatched-input-kind value type | WP01 | |
| T002 | Capture the dispatched kind at the dispatch site and thread it to every checkpoint construction | WP01 | |
| T003 | Add the occupant-scalar before/after fields, absent-not-zero | WP01 | |
| T004 | Read the occupant scalar from the canonical projection before dispatch and after acceptance | WP01 | |
| T005 | Declare which scene step edits an occupant scalar | WP01 | |
| T006 | In-module assertions over the new checkpoint fields | WP01 | |
| T007 | Journey guard asserts over the recorded dispatched kind | WP02 | |
| T008 | Permitted-injection guard counts recorded direct injections, at most one | WP02 | [P] |
| T009 | Replace the vacuous edit criterion with the recorded scalar change | WP02 | [P] |
| T010 | Falsification: revert the dispatch selection, observe failure, restore | WP02 | |
| T011 | Falsification: remove the occupant edit, observe failure, restore | WP02 | |
| T012 | Add the position-stamping constructor to PostEffectConfig | WP03 | |
| T013 | set_slot_occupancy stamps the position's identity | WP03 | |
| T014 | Composition-root round trip still places each effect at its position | WP03 | |
| T015 | Falsification: defeat the stamp, observe failure, restore | WP03 | |
| T016 | Write the acceptance-record coverage script | WP04 | |
| T017 | Tool-gating twin test for the new script | WP04 | |
| T018 | Falsification: strip a requirement kind, observe the script fail naming it | WP04 | |
| T019 | Emit the six new witness observations | WP05 | |
| T020 | Move the observation schemaVersion 2 → 3 | WP05 | |
| T021 | Surface the new fields on the live report, absent-not-zero | WP05 | [P] |
| T022 | Confirm every declared predicate passes on measured values | WP05 | |
| T023 | Deterministic preflight before touching hardware | WP06 | |
| T024 | Physical run, capture and commit the log | WP06 | |
| T025 | Checkpoint-identity comparison against the frozen baseline | WP06 | |
| T026 | Acceptance matrix grading every requirement kind this mission declares | WP06 | |
| T027 | Record remediation in ROADMAP.md | WP06 | |

27 subtasks across 6 work packages.

---

## WP01 — Checkpoint records what actually happened

**Prompt**: `tasks/WP01-checkpoint-records-what-happened.md` (~430 lines)
**Priority**: P1 — the mission's root concern; nothing else can assert over a record that does not exist.
**Dependencies**: none
**Requirements**: FR-001, FR-004

**Goal**: `LiveTopologyCheckpoint` carries the input kind the production reducer actually
received and, for a scalar edit, the projected value before dispatch and after acceptance.

**Independent test**: construct a checkpoint from a journey-driven step and from a
direct-injection step; the recorded kinds differ. This is provable without touching any guard.

**Included subtasks**: T001, T002, T003, T004, T005, T006

**Implementation sketch**: introduce the kind type → capture the dispatched event at
`live_demo_runner.rs:959-964` where the selection already happens → carry it on
`TopologyContext` (`:1718`) through the `LiveTopologyPhase` variants → pass it into all three
`LiveTopologyCheckpoint::new` sites → add the scalar pair, read via the canonical projection on
the `AwaitScalarAudible` path → declare the edited step in the scene.

**Risks**: the dispatched event is currently a consumed local. Threading it must not alter
dispatch behavior. New fields are control-side only — nothing may reach a callback structure.
Serialization is additive; existing keys keep their names and values (spec C-001, C-003).

---

## WP02 — Guards assert over the record

**Prompt**: `tasks/WP02-guards-assert-over-the-record.md` (~470 lines)
**Priority**: P1 — this is the mission's whole point; WP01 without WP02 changes nothing observable.
**Dependencies**: WP01
**Requirements**: FR-002, FR-003, FR-005, FR-006

**Goal**: the journey guard, the permitted-injection guard, and the occupant-edit criterion all
assert over recorded values. Both HIGH findings from the predecessor review close here.

**Independent test**: the two falsification subtasks ARE the test — each mutation must make the
suite fail, and the same mutation passes today.

**Included subtasks**: T007, T008, T009, T010, T011

**Implementation sketch**: rewrite the guard loop in `tests/effects_and_buses.rs:161-211` to read
the recorded kind rather than `transition.adjust()` → count recorded direct injections and assert
at most one → replace the `Accepted` + `audible_on_activated_graph()` + `active_notes() > 0`
triple with the recorded before ≠ after assertion → perform both mutations and record them.

**Risks**: the permitted-injection assertion must be **at most one**, not exactly one (research
R-002) — the rejection is the only permitted direct injection, not a required one. Do not delete
the existing declaration-based focus assertions; they complement the record and are not
superseded by it.

---

## WP03 — Stamp slot identity from position

**Prompt**: `tasks/WP03-stamp-slot-identity-from-position.md` (~350 lines)
**Priority**: P2 — closes RISK-1 by construction. Latent today; no current path produces it.
**Dependencies**: none
**Requirements**: FR-007, FR-006

**Goal**: an occupant's slot identity is derived from the position it occupies, so a
valid-but-mismatched identity is inexpressible rather than merely refused.

**Independent test**: install an occupant carrying another position's identity; the stored
occupant carries the correct one.

**Included subtasks**: T012, T013, T014, T015

**Implementation sketch**: add an identity-setting constructor to `PostEffectConfig`
(`src/synth/effect_capability.rs:171`) → stamp inside `set_slot_occupancy`
(`src/synth/patch.rs:194-213`) → confirm the composition-root round trip
(`standalone_application.rs:1516`) still resolves each position → falsify the stamp.

**Risks**: the serialized `slotId` vocabulary must not change (spec C-003) — this changes how a
value is produced, never what it is called. Uniqueness becomes a consequence of derivation;
removing the explicit check needs care so `EffectSlotOccupancyError`'s remaining variants stay
honest. Tests live inline in `src/synth/patch.rs` (`mod tests` at `:255`), so this WP owns no
shared test file.

---

## WP04 — Acceptance-record coverage gate

**Prompt**: `tasks/WP04-acceptance-record-coverage-gate.md` (~300 lines)
**Priority**: P3 — process hygiene; ranks below the three above but is a declared completion check.
**Dependencies**: none
**Requirements**: FR-008, FR-006

**Goal**: acceptance fails when a mission's acceptance record omits a requirement kind its
specification declares.

**Independent test**: strip the NFR rows from a copy of a matrix; the script fails naming the
omitted kind.

**Included subtasks**: T016, T017, T018

**Implementation sketch**: write `scripts/check_acceptance_matrix_coverage.sh` following the
existing guard script's contract → add the tool-gating twin to
`tests/no_name_enumeration_guard.rs` → falsify.

**Risks**: scope is the mission being accepted, only (recorded decision) — scanning siblings
would import a backfill of the parent's constraint rows that this mission was not chartered for.
The script must gate on its tool dependencies and exit non-zero naming any missing tool, so
absent tooling never reads as a pass, and must emit the exact declared marker string.

---

## WP05 — Observation schema v3

**Prompt**: `tasks/WP05-observation-schema-v3.md` (~340 lines)
**Priority**: P2 — gives the declared witness predicates measured values to read.
**Dependencies**: WP01, WP03
**Requirements**: FR-001, FR-004, FR-007

**Goal**: the release witness emits the six new observations and every declared predicate passes
on measured values.

**Independent test**: run the release target; the observation carries `schemaVersion: 3` and all
six new fields with measured values.

**Included subtasks**: T019, T020, T021, T022

**Implementation sketch**: compute the six counts from the recorded checkpoints → bump
`schemaVersion` → surface the fields on `LiveDemoReport` preserving absent-vs-zero → run and
confirm.

**Risks**: `schemaVersion` moves 2 → 3 and every artifact quoting version 2 must move together.
`retiredGraphsCollectedOffCallback` is **15** in the current baseline, not the parent's 8 — do
not copy stale numbers forward. WP02 owns `tests/effects_and_buses.rs`; this WP owns
`tests/expandable_effects_and_bus_topology.rs`. They must not cross.

---

## WP06 — Physical re-run, evidence, and acceptance record

**Prompt**: `tasks/WP06-physical-rerun-and-evidence.md` (~400 lines)
**Priority**: P1 — the mission's evidence gate; nothing is demonstrated until this runs.
**Dependencies**: WP01, WP02, WP03, WP04, WP05
**Requirements**: FR-009, FR-010

**Goal**: the strengthened record is demonstrated on the real rig, the frozen identity baseline
survives byte-identically, and this mission's own acceptance matrix passes the gate WP04 built.

**Independent test**: the committed log shows the new fields carrying measured values; the
identity comparison yields 0 modified and 0 removed.

**Included subtasks**: T023, T024, T025, T026, T027

**Implementation sketch**: deterministic preflight → physical run from the merged lane
worktree with the log redirected → identity comparison against
`FROZEN_TOPOLOGY_IDENTITY_BASELINE` (`tests/effects_and_buses.rs:59`, 17 entries) → author the
acceptance matrix covering FR, NFR, and C rows → record remediation in `ROADMAP.md`.

**Risks**: run from the merged lane worktree, not a stale checkout, or the run demonstrates the
old behavior — verify with a grep for a known new symbol first. Never pipe the run or a test
through `head`/`tail`; the pipe masks the exit status. Physical evidence is never substituted
with headless output (spec C-006). Commit the log: the parent mission's cited logs were never
committed and no longer exist on this host.

**Ownership note**: WP `owned_files` cannot declare `kitty-specs/` paths (runtime rule
`INVALID_WP_OWNED_FILES_KITTY_SPECS`). This WP formally owns `ROADMAP.md`; the evidence log and
acceptance matrix under `kitty-specs/falsifiable-journey-proof-01KYZTQ1/` are recorded
out-of-map edits, permitted with the one-line rationale "mission evidence and acceptance record;
kitty-specs paths are non-declarable by rule". The same convention applies to each WP's
falsification records under `evidence/falsification/`.

---

## Dependency graph

```
WP01 ──┬──> WP02 ──┐
       │            │
       └──> WP05 ───┼──> WP06
                    │
WP03 ──────────────┤
                    │
WP04 ──────────────┘
```

## Parallelization

- **Wave 1 (three lanes in parallel)**: WP01, WP03, WP04 — no dependencies, disjoint files.
- **Wave 2 (two lanes)**: WP02 (needs WP01), WP05 (needs WP01 + WP03).
- **Wave 3**: WP06 — needs everything.

WP03 and WP04 are fully independent of the mission's core and can land early. WP01 is the
critical path: it gates both WP02 and WP05.

## MVP scope

**WP01 + WP02** deliver the mission's entire stated value: both HIGH findings from the
predecessor review close, and the journey becomes falsifiable. WP03 through WP05 are the
operator-included items and the measured surface; WP06 is the evidence gate.

If the mission had to stop early, stopping after WP02 leaves a coherent, defensible result —
provided WP02's falsification records exist, without which nothing has been proven.
