# Quickstart: Phase 3 Demo Journey Fidelity and Hygiene

How to run every gate this mission touches. All commands from the repository
root (`/Users/crestenstclair/workspace/crest-synth`).

## Deterministic gates

```bash
# Full suite (NFR-003 regression gate)
cargo test --all-targets

# The phase's behavioral proof target (witness observation schemaVersion 2:
# fourth-entry fixture, carry-over identity refusal, journey-neutral identities)
cargo test --release --test expandable_effects_and_bus_topology -- --nocapture
# Expect: CREST_EFFECTS_AND_BUSES_OBSERVATION {... "schemaVersion": 2,
#   "fourthEntryEndToEndExercised": true,
#   "carryOverWrongEngineIdentityRefused": true ...}

# New attached validations (declared selectors)
cargo test return_clear_held_note_continuity
cargo test carry_over_capability_identity

# Witness negative case (unchanged from parent)
cargo run --quiet --bin crest-synth-witness -- --case refused-topology --mutant none
cargo run --quiet --bin crest-synth-witness -- --case refused-topology --mutant refused-topology-published  # expect exit 1

# Name-enumeration guard (now tool-gated: with rg/perl missing it must exit
# non-zero naming the tool, never "no candidates")
bash scripts/check_no_name_enumerated_identity.sh
cargo test --test no_name_enumeration_guard
```

## Compact-view retirement checks (IC-02)

```bash
# Zero remaining callers after migration (SC-005)
grep -rn "post_effects()\|with_post_effects(" src/ tests/ --include="*.rs"   # expect: no output

# Gapped-chain round-trip regression test runs inside the suite; find it via
cargo test gapped   # selector chosen at task time; asserted before accessor deletion
```

## Live gate (RECORDED-MANUAL — physical rig required)

```bash
make demo-live-effects-and-buses
```

Watch for (FR-001..FR-004, SC-001, SC-002):
- focus visibly lands on each PATCH effect-slot row before its occupancy
  cycles by the adjacent-choice gesture;
- one installed occupant scalar edited from the PATCH page with an audible
  change;
- return occupancy changes walked through the MIXER return rows;
- the single controlled rejection (direct injection, documented inline in
  `src/testing/live_effects_and_buses_scene.rs`) shows its visible reason.

After the run (SC-003, SC-004, NFR-001):
- compare checkpoint identities byte-level against the parent evidence —
  0 modified, 0 removed, additions only;
- report completeness: 100% checkpoints, droppedRecords=0, zero false
  observation keys, clean teardown;
- measurement fields render as measured values or explicit absent — never a
  defaulted 0.

## Acceptance bookkeeping (IC-10)

- Amend `kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/acceptance-matrix.json`
  and `.../mission-review.md` (addendum) add/append-only: reference the
  refreshed evidence and disposition each of the review's 7 open items
  (SC-007 — 7 of 7 dispositioned; both optional items were operator-included,
  so deferral rationale applies only if one is genuinely dropped mid-mission
  by a recorded decision).

## Bulk-edit guardrail

`occurrence_map.yaml` (approved) governs every diff: serialized keys
(postEffects/preparedPostEffects vocabulary), CLI/make names, and CREST_*
markers are do_not_change; `tests/no_name_enumeration_guard.rs` keeps its
deliberate reverbSend fixture; parent-mission history and the crest-spec are
never term-scrubbed. The review-time diff compliance check enforces this map.
