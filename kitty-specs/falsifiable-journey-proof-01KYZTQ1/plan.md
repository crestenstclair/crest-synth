# Implementation Plan: Falsifiable Journey Proof

**Branch**: `feat/expandable-effects-and-bus-topology` | **Date**: 2026-08-01 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `kitty-specs/falsifiable-journey-proof-01KYZTQ1/spec.md`

## Summary

Make the live effects-and-buses guards capable of failing. The demonstrated behavior is
correct and this mission changes none of it; what changes is what the checkpoints record and
what the checks assert over. Occupancy checkpoints gain the input kind the production reducer
actually received, the parameter-edit checkpoint gains the edited scalar's before/after pair,
and the guards move from asserting over the scene's declaration to asserting over those
records. Two smaller items ride along: an occupant's slot identity is stamped from the position
it occupies rather than trusted from the caller, and a declared validation fails acceptance when
a mission's acceptance record omits a requirement kind its spec declares.

The governing rule is spec C-005: no new guard is accepted until its failure has been observed
under a deliberate mutation and recorded. That rule exists because the predecessor mission
graded its fix with assertions that could not fail.

## Technical Context

**Language/Version**: Rust, edition 2021 (`Cargo.toml:4`), binaries `crest-synth` and `crest-synth-witness`
**Primary Dependencies**: no new dependencies — this mission adds no crate; all work is inside existing modules plus one POSIX shell script
**Storage**: N/A — checkpoints and reports are in-memory control-side values serialized to stdout markers and evidence logs
**Testing**: `cargo test --all-targets` (26 targets); the mission's own targets are `tests/effects_and_buses.rs` (journey and identity guards), `tests/expandable_effects_and_bus_topology.rs` (release witness observation), `tests/no_name_enumeration_guard.rs` (script-gating twin), plus the physical `make demo-live-effects-and-buses`
**Target Platform**: macOS (darwin) host driving a real window, physical audio device, and real MIDI fixture; the deterministic layer is host-agnostic
**Project Type**: single Rust workspace — `src/` library plus `tests/` integration targets and `scripts/` guard scripts
**Performance Goals**: no change to audio-path performance; the added checkpoint fields are control-side only and must not appear in any callback-reachable structure
**Constraints**: the real-time callback keeps zero allocations, zero destructions, no locking, blocking, I/O, logging, or panic (spec NFR-001, C-002); every existing checkpoint identity stays byte-identical (spec C-001); serialized `slotId` vocabulary and observation key names are unchanged (spec C-003)
**Scale/Scope**: ~5 source modules, 3 test targets, 1 new script, 1 witness schema bump (2 → 3), 1 physical re-run; no product behavior added

## Charter Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Charter present (`compact` mode, template set `software-dev-default`). Directives that bind
this mission:

| Directive | Bearing on this plan | Status |
|---|---|---|
| DIRECTIVE_043 (close defect classes by construction) | The stamping approach (IC-04) makes a mismatched identity inexpressible rather than refused. The dispatch record (IC-01) makes the journey's loss detectable rather than assumed. | PASS |
| DIRECTIVE_024 (locality of change) | Every change sits next to the surface it proves. No refactor rides along; the physical re-run covers only the affected scene. | PASS |
| DIRECTIVE_025 (Boy Scout Rule) | Domain-matched debt in scope: the two review items (RISK-1, matrix coverage) are folded in deliberately by operator direction, not absorbed silently. Out-of-domain items are left filed. | PASS |
| DIRECTIVE_010 (specification fidelity) | The crest-spec was authored first (commit `ad9960b`); this plan derives from it and adds nothing it does not declare. | PASS |
| DIRECTIVE_003 (decision documentation) | Four decisions recorded in the mission ledger: two in specify (identity shape, matrix remediation shape), two in plan (gate scope, falsification evidence form). `decision verify` clean. | PASS |

No violations. Complexity Tracking is therefore empty and omitted.

## Crest-Spec Derivation

Authored in `/spec-kitty.crest-spec` (commit `ad9960b`), `crest_spec_impact: structural`.
Doctor: OK — 122 resources, 89 requirements, 29 project validations (29 completion checks),
18 witnesses.

**Resources changed**

| Canonical ID | Change | Realized by |
|---|---|---|
| `valueObject.Testing.LiveDemoCheckpoint` | CHANGED — adds `dispatchedInputKind`, `occupantScalarBefore`, `occupantScalarAfter`; four invariants binding them to what the reducer received and to a required change | IC-01, IC-03 |
| `valueObject.Synth.EffectSlotId` | CHANGED — identity is derived from the occupied position, not supplied alongside it | IC-04 |
| `aggregate.Synth.Patch` | CHANGED — installing an occupant stamps the position's derived identity onto it; uniqueness follows from derivation | IC-04 |
| `requirement.expandable_effects_behavioral_proof` | CHANGED — the journey is graded on the recorded input kind, the edit on a recorded before/after change | IC-02, IC-03 |
| `requirement.falsifiable_journey_proof` | ADDED — checks assert over the record, never the declaration; each demonstrated falsifiable by a recorded mutation | IC-02, IC-03, IC-07 |
| `requirement.acceptance_record_covers_all_requirement_kinds` | ADDED | IC-05 |
| `capability.expandable_effects_and_bus_topology` → `open_effect_registry` | CHANGED — gains a mutation acceptance scenario | IC-07 |
| `validation.acceptance_matrix_covers_all_requirement_kinds` | ADDED — project scope, wired into `completion.projectChecks` | IC-05 |
| `witness.expandable_effects_and_bus_topology` | CHANGED — `schemaVersion` 2 → 3, six added observations and predicates | IC-06 |
| `asset.ValidationScripts` | CHANGED — prompt covers the new coverage script; `assetKind.validation-script` `filePattern: scripts/*` already admits it | IC-05 |

**Retired**: none.

**Assets producing this mission's files**

| Asset | Files |
|---|---|
| `asset.TestingContextModules` | `src/testing/live_demo_runner.rs`, `src/testing/live_demo_checkpoint.rs`, `src/testing/live_effects_and_buses_scene.rs`, `src/testing/live_demo_report.rs` |
| `asset.SynthContextModules` | `src/synth/patch.rs`, `src/synth/effect_capability.rs` |
| `asset.BehavioralAcceptanceTests` | `tests/effects_and_buses.rs`, `tests/expandable_effects_and_bus_topology.rs`, `tests/no_name_enumeration_guard.rs` |
| `asset.ValidationScripts` | `scripts/check_acceptance_matrix_coverage.sh` |

**Validations and witnesses covering the change**

- `validation.acceptance_matrix_covers_all_requirement_kinds` (project, completion check) — IC-05.
- `witness.expandable_effects_and_bus_topology` schemaVersion 3 predicates — IC-01 through IC-04:
  `occupancyStepsDeclaringJourney > 0`, `occupancyStepsNotGradedOnRecordedDispatch = 0`,
  `directInjectionsRecorded <= 1`, `occupantScalarEditsExercised > 0`,
  `occupantScalarEditsWithoutRecordedChange = 0`, `mismatchedSlotIdentityInexpressible = true`.
- `evidence.expandable_effects_and_bus_topology_contract` — the physical run, IC-08.

**Not produced** (forbidden while a crest-spec exists): `data-model.md`, `contracts/`. The
value objects and aggregates this mission touches are declared above by canonical ID.

## Project Structure

### Documentation (this mission)

```
kitty-specs/falsifiable-journey-proof-01KYZTQ1/
├── plan.md                      # This file
├── spec.md                      # Committed 7531b26
├── research.md                  # Phase 0 output
├── checklists/requirements.md   # Spec quality checklist
├── decisions/                   # Four recorded decision moments
├── evidence/falsification/      # Per-guard falsification records (IC-07)
└── tasks.md                     # /spec-kitty.tasks output — NOT created here
```

### Source Code (repository root)

```
src/
├── testing/
│   ├── live_demo_runner.rs              # dispatch site; records the input kind (IC-01)
│   ├── live_demo_checkpoint.rs          # LiveTopologyCheckpoint gains the new fields (IC-01, IC-03)
│   ├── live_effects_and_buses_scene.rs  # scene declarations, audible witnesses (IC-03)
│   └── live_demo_report.rs              # report surfacing of the new fields (IC-06)
├── synth/
│   ├── patch.rs                         # set_slot_occupancy stamps identity (IC-04)
│   └── effect_capability.rs             # PostEffectConfig identity constructor (IC-04)
└── control/                             # UNCHANGED — no reducer or semantic-action change

tests/
├── effects_and_buses.rs                 # journey + identity guards read the record (IC-02, IC-04)
├── expandable_effects_and_bus_topology.rs  # witness observation v3 (IC-06)
└── no_name_enumeration_guard.rs         # script tool-gating twin for the new script (IC-05)

scripts/
└── check_acceptance_matrix_coverage.sh  # new declared validation (IC-05)
```

**Structure Decision**: single Rust workspace, unchanged. Work concentrates in
`src/testing/` (the proof surfaces), with one narrow change in `src/synth/` and one new
script. `src/control/` and `src/real_time/` are deliberately untouched: this mission adds no
product behavior and nothing it adds may reach the audio callback.

## Implementation Concern Map

> Implementation concerns are NOT work packages. `/spec-kitty.tasks` translates these into
> executable WPs.

### IC-01 — Record what the reducer actually received

- **Purpose**: Carry the dispatched input kind from the point of dispatch onto the emitted checkpoint, so a check has something truthful to assert over.
- **Relevant requirements**: FR-001
- **Affected surfaces**: `src/testing/live_demo_runner.rs` (the dispatch selection and the three checkpoint construction sites), `src/testing/live_demo_checkpoint.rs` (`LiveTopologyCheckpoint`)
- **Sequencing/depends-on**: none — this is the mission's root concern
- **Risks**: The dispatched event is currently a local consumed at dispatch; it must be captured without changing dispatch behavior. The new field is control-side only and must not enter any callback-reachable structure. Serialization is additive: existing keys keep their names and values.

### IC-02 — Grade the journey on the record, not the declaration

- **Purpose**: Move the journey guard's subject from `scene.expected_topology_transitions()` to the recorded dispatched input kind, and identify the single permitted direct injection by its record.
- **Relevant requirements**: FR-002, FR-003
- **Affected surfaces**: `tests/effects_and_buses.rs`
- **Sequencing/depends-on**: IC-01
- **Risks**: The permitted-injection assertion must be "at most one", not "exactly one" — the rejection is the only *permitted* direct injection, not a required one, and `eq 1` would forbid a future scene expressing it by gesture. The declaration-based focus-verification assertions stay as a complement; they are not a substitute and are not removed.

### IC-03 — Grade the occupant edit on a measured change

- **Purpose**: Record the edited occupant scalar before dispatch and after acceptance, and satisfy the criterion only when they differ.
- **Relevant requirements**: FR-004, FR-005
- **Affected surfaces**: `src/testing/live_demo_checkpoint.rs`, `src/testing/live_demo_runner.rs`, `src/testing/live_effects_and_buses_scene.rs`
- **Sequencing/depends-on**: none (parallel with IC-01)
- **Risks**: Both readings come from the canonical projection, not from the scene's intended value. Absent must remain distinguishable from a measured zero (spec NFR-005) — a defaulted zero here would regress the predecessor's contract. A no-op edit landing on the existing value must fail, not pass.

### IC-04 — Stamp slot identity from position

- **Purpose**: Make a valid-but-mismatched slot identity inexpressible by deriving it from the occupied position at the single gate all occupancy changes pass through.
- **Relevant requirements**: FR-007
- **Affected surfaces**: `src/synth/patch.rs` (`set_slot_occupancy`), `src/synth/effect_capability.rs`
- **Sequencing/depends-on**: none
- **Risks**: The serialized `slotId` vocabulary must not change (spec C-003) — this changes how a value is produced, never what it is called. The existing uniqueness check becomes a consequence of derivation; removing it outright needs care so the error type's meaning stays honest. Round-tripping through the composition root must still place each effect at the position its identity denotes.

### IC-05 — Acceptance-record coverage gate

- **Purpose**: Fail acceptance when a mission's acceptance record omits a requirement kind its specification declares.
- **Relevant requirements**: FR-008
- **Affected surfaces**: `scripts/check_acceptance_matrix_coverage.sh` (new), `tests/no_name_enumeration_guard.rs` (tool-gating twin)
- **Sequencing/depends-on**: none
- **Risks**: Scope is the mission being accepted only (recorded decision) — it must not scan sibling or archived missions, or this mission inherits a backfill it was not chartered for. The script must gate on its tool dependencies and exit non-zero naming any missing tool, matching the existing guard script's contract, so absent tooling never reads as a pass. It must emit the exact declared marker string.

### IC-06 — Observation schema v3

- **Purpose**: Emit the six new observations so the declared witness predicates have measured values.
- **Relevant requirements**: FR-001, FR-004, FR-007 (their measured surface)
- **Affected surfaces**: `tests/expandable_effects_and_bus_topology.rs`, `src/testing/live_demo_report.rs`
- **Sequencing/depends-on**: IC-01, IC-03, IC-04
- **Risks**: `schemaVersion` moves 2 → 3; every artifact quoting version 2 must be updated together. `retiredGraphsCollectedOffCallback` is 15 in the current baseline, not the parent's 8 — do not copy stale numbers forward.

### IC-07 — Falsification records

- **Purpose**: Demonstrate each new guard can fail, and leave that demonstration on the record.
- **Relevant requirements**: FR-006 (governed by C-005)
- **Affected surfaces**: `kitty-specs/falsifiable-journey-proof-01KYZTQ1/evidence/falsification/`
- **Sequencing/depends-on**: IC-02, IC-03, IC-04, IC-05 (each guard must exist first)
- **Risks**: This is the mission's governing rule and the concern most likely to be quietly skipped under time pressure — a guard whose failure was never observed is exactly what this mission exists to eliminate. Each record carries the mutation applied, the failing command with its exit code and message, the restoration, and the passing command. Never pipe the command through `head`/`tail`; the pipe masks the exit status and a "green" recorded that way is unreliable.

### IC-08 — Physical re-run and evidence refresh

- **Purpose**: Demonstrate the strengthened record on the real rig, with the new fields carrying measured values.
- **Relevant requirements**: FR-009, FR-010
- **Affected surfaces**: `make demo-live-effects-and-buses`; refreshed evidence stored beside the existing committed log
- **Sequencing/depends-on**: every other concern
- **Risks**: Run from the merged lane worktree, not a stale checkout, or the run demonstrates the old behavior. Compare against `FROZEN_TOPOLOGY_IDENTITY_BASELINE` (`tests/effects_and_buses.rs:59`): 0 modified, 0 removed, additions enumerated. Physical evidence is never substituted with headless output (spec C-006); if the rig cannot be driven, stop and report. Commit the log — the parent mission's cited logs were never committed and no longer exist on this host.
