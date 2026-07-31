---
work_package_id: WP10
title: Voice carry-over across topology activation
dependencies:
- WP08
- WP09
requirement_refs:
- FR-001
- FR-002
- NFR-001
- NFR-004
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T057
- T058
- T059
- T060
history:
- timestamp: '2026-07-31T18:30:00Z'
  actor: planner
  action: created after the WP08 witness measured clearedSlotPreservedHeldNotes false and the operator chose voice carry-over over declaration revision
agent_profile: implementer-ivan
authoritative_surface: src/real_time/
create_intent:
- tests/expandable_effects_and_bus_topology.rs
- tests/topology_change_lifecycle.rs
- tests/effects_and_buses.rs
execution_mode: code_change
mission_id: 01KYNGX8QA8V49BX2WQ1Q6G2BP
mission_slug: expandable-effects-and-bus-topology-01KYNGX8
model: ''
owned_files:
- src/real_time/**
- src/synth/prepared_engine_rack_builder.rs
- src/testing/**
- tests/expandable_effects_and_bus_topology.rs
- tests/topology_change_lifecycle.rs
- tests/effects_and_buses.rs
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP10 – Voice carry-over across topology activation

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Make held notes survive an accepted topology change. Today
`AudioRenderer::apply_structural_handoff` clears all voices at graph
activation because the replacement graph carries freshly prepared engines; the
crest-spec (`contexts/realtime.yaml`, PreparedGraph invariants) now declares
the opposite: activation preserves every sounding voice of the Patches whose
engine identity the delta leaves unchanged, with no allocation or destruction
on the callback; only the position the delta changed may restart its local
processing state.

The witness field `clearedSlotPreservedHeldNotes` (measured honestly as
`false` by WP08, emitted with a `MEASURED GAP` diagnostic) must measure `true`
through the production path, satisfying the declared predicate at
`spec-kitty accept`, spec acceptance scenario 1.5, and SC-001.

## Context

- **Dependencies**: WP08 (retained scene and witness measure the behavior), WP09 (guard must stay green)
- **Related requirements**: FR-001/FR-002 (slot edits during held notes), NFR-001 (render-path safety), NFR-004 (audio continuity)
- **Study first**: `src/real_time/audio_renderer.rs::apply_structural_handoff` (the `clear_all`), `src/real_time/graph_preparation_worker.rs` (candidate construction), `src/real_time/prepared_graph.rs` (ownership + `GraphReplacementScope`), `tests/topology_change_lifecycle.rs`
- **Design authority**: `.kittify/crest-spec/contexts/realtime.yaml` — the new voice-continuity invariant plus every existing callback-safety and ownership invariant, all of which still bind

## Subtasks

### T057 – Design and implement the carry-over mechanism

- **Purpose**: Sounding voices must survive the swap without violating single ownership or callback discipline.
- **Steps**:
  1. Choose and RECORD the mechanism (DIRECTIVE_003 — a decision note in the code and in your report). Candidate shapes, none mandated: (a) reuse unchanged prepared components — the worker builds the replacement around the still-active graph's engine rack, transferring ownership at the block boundary via the existing prepared-handoff queues; (b) bounded voice-state transfer — the callback copies the fixed-capacity active-note state (note, channel, velocity, envelope phase) from superseded to replacement engines at activation, within preallocated capacity; (c) another shape that satisfies every invariant.
  2. Whatever the shape: no allocation, locking, blocking, or destruction on the callback; the superseded graph still retires off-callback intact; block-boundary atomicity holds; `matches_parameters` exactness is untouched.
  3. Only the delta position may restart local processing state; unchanged effect instances SHOULD keep their tails where the mechanism makes that reachable, and MUST NOT gain wrong state.

### T058 – Prove voice continuity deterministically

- **Steps**:
  1. Extend `tests/topology_change_lifecycle.rs`: hold notes through an accepted slot occupancy change; assert the notes remain active and audibly continuous across the activation block (sample-level continuity of the engine output for untouched Patches; no retrigger, no gap).
  2. Assert the negative space: the changed position's processing restarts cleanly; a refused change still leaves everything untouched (existing proofs must not regress).
  3. Prove callback discipline at the new path: 0 allocations, 0 deallocations, 0 drops during carry-over activation (reuse the counting-allocator harness).

### T059 – Re-measure the witness and the retained scene

- **Steps**:
  1. `tests/expandable_effects_and_bus_topology.rs`: `clearedSlotPreservedHeldNotes` must now measure `true` through the production observation; remove the `MEASURED GAP` diagnostic; keep the measurement honest (it must still be able to measure false if the mechanism regresses).
  2. `src/testing/live_effects_and_buses_scene.rs`: rework the slot-clear step to hold a note across the change (the scene previously used the engine-transition model as a workaround); keep every existing checkpoint identity byte-identical — add, never rename.
  3. Run the retained scene on the physical device (`make demo-live-effects-and-buses`) and record the observation — the live gate re-run is required because the audible contract changed.

### T060 – Regression sweep

- **Steps**:
  1. `cargo test --all-targets` green; `make lint` clean; demo observation false-set not grown.
  2. Earlier phase scenes still run (T051 surface untouched or re-verified).
  3. `scripts/check_no_name_enumerated_identity.sh` still passes; every WP09 guard test green.

## Definition of Done

- The declared voice-continuity invariant is implemented and proven deterministically and live.
- `clearedSlotPreservedHeldNotes` measures `true` honestly; the witness passes both its positive and controlled-negative commands.
- No callback-safety, ownership, atomicity, or matching proof weakened; full suite green.

## Risks & Mitigations

- **Ownership vs continuity tension**: moving live engines between graphs risks violating single ownership or doing work on the callback — whatever mechanism is chosen must be provable with the existing allocator/drop harnesses, not argued.
- **Retrigger masquerading as continuity**: a replayed note-on can sound "held" at coarse RMS. The deterministic proof must assert sample-level continuity, not merely `activeNotes > 0`.
- **Scene identity freeze**: `logs_telemetry: do_not_change` still binds — checkpoint additions only.
