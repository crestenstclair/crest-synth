---
work_package_id: WP08
title: Retained live scene and measured proof
dependencies:
- WP07
requirement_refs:
- FR-019
- NFR-004
- NFR-005
- NFR-006
- NFR-007
- NFR-008
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T046
- T047
- T048
- T049
- T050
- T051
- T052
history:
- timestamp: '2026-07-29T02:11:28Z'
  actor: planner
  action: created
agent_profile: implementer-ivan
authoritative_surface: src/testing/
create_intent:
- src/testing/live_effects_and_buses_scene.rs
- tests/effects_and_buses.rs
execution_mode: code_change
mission_id: 01KYNGX8QA8V49BX2WQ1Q6G2BP
mission_slug: expandable-effects-and-bus-topology-01KYNGX8
model: ''
owned_files:
- src/testing/**
- src/bin/crest_synth.rs
- Makefile
- tests/effects_and_buses.rs
- tests/sixteen_track_mixer_routing.rs
- tests/static_patch_effect.rs
- tests/exhaustive_demo_scene.rs
- tests/live_demo_scene.rs
- tests/behavioral_mutation_harness.rs
- tests/production_runtime_contracts.rs
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP08 – Retained live scene and measured proof

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

This loads your identity, boundaries, and governance context. Do not skip this step.
Once loaded, continue with the Objective below.

## Objective

Deliver `make demo-live-effects-and-buses` — the retained live scene that is this
phase's completion gate — plus the deterministic, mutation, and measurement proofs
that make every declared behavior falsifiable.

`ROADMAP.md` is unusually explicit here: the phase is **not complete** until the
actual live target has been run successfully by the implementer with a real window,
physical audio output, and the real MIDI fixture, and the resulting visible, audible,
structured report covers every declared phase behavior. A headless, silent, mocked,
or dry-run substitute does not satisfy this gate. T052 is therefore not a formality.

## Context

- **Mission**: expandable-effects-and-bus-topology-01KYNGX8
- **Priority**: P1
- **Dependencies**: WP07 (the full behavior must exist to be demonstrated)
- **Related requirements**: FR-019 (retained scene), NFR-004 (audio continuity), NFR-005 (deterministic evidence), NFR-006 (clean teardown), NFR-007 (routing isolation), NFR-008 (edit responsiveness)
- **Constraints**: C-010 (live demo gate)
- **Read first**: `ROADMAP.md` § Live-demo requirement for every phase — it is the acceptance contract for this WP

## Branch Strategy

- **Planning base branch**: `feat/expandable-effects-and-bus-topology`
- **Merge target branch**: `feat/expandable-effects-and-bus-topology`
- **Execution**: worktree-per-lane. `finalize-tasks` computes lanes and writes `lanes.json`; each lane gets exactly one worktree and one branch.
- Do not create ad-hoc branches by hand; use the workspace the runtime resolves for this WP's lane.

## Subtasks

### T046 – Add the `demo-live-effects-and-buses` scene

- **Purpose**: The scene is the primary human-verification path for this phase.

- **Steps**:
  1. Create `src/testing/live_effects_and_buses_scene.rs`, modelled on the existing sixteen-track routing scene.
  2. Play the real MIDI fixture through the production MIDI event source and normal routing/render path **throughout**. Direct state mutation, fabricated audio, and demo-only reducers or renderers are forbidden.
  3. The scene must exercise, through semantic actions and `AppState::apply`:
     - filling all three slots on a Patch, one at a time, each audibly
     - exchanging two effects' slot positions so the order change is **audible**
     - two instances of one effect proving independent tails
     - raising sends toward several of the eight destinations
     - changing the effect occupying a return
     - one **controlled rejection** with visible reason and uninterrupted audio
     - rerouting a Patch and showing its chain follows it
  4. Pace it so the user can see the focused control, the action, the resulting state, and hear the musical consequence.
  5. Finish with semantic all-notes-off, zero active notes, window close, stream release, worker shutdown, graph collection, and normal parent-process exit.

- **Validation**: Every declared phase behavior appears in the scene, audibly where it affects sound.

### T047 – Add phase checkpoints without altering existing identities

- **Purpose**: Structured evidence correlating semantic input, accepted generation, visible projection, graph state, MIDI activity, and measured audio.

- **Steps**:
  1. Add checkpoints to `src/testing/live_demo_checkpoint.rs` for slot occupancy, slot ordering, return occupancy, send routing, rejection, and recovery.
  2. **Every existing checkpoint identity must stay byte-identical.** `occurrence_map.yaml` sets `logs_telemetry: do_not_change` precisely so earlier retained phase scenes remain comparable. Add; do not rename.
  3. Extend `live_demo_report.rs` so the report covers the new behaviors. It carries retired send-field references that must be updated to indexed form.
  4. Each checkpoint must correlate cause and observed effect, not merely announce a step.

- **Validation**: New checkpoints emit; every pre-existing checkpoint string is unchanged.

### T048 – Extend the behavioral mutation harness `[P]`

- **Purpose**: Proofs must be falsifiable. The harness verifies that seams actually fail when mutated.

- **Steps**:
  1. Extend `src/testing/behavioral_mutation_harness.rs` (~21 occurrences of retired send identifiers, ~10 of retired globals) to the new surfaces.
  2. Add mutations that **must** be caught: swapping slot order without changing output, an empty return passing input through, a send taken pre-gate instead of post-gate, a muted track still feeding a send, and permissive structural matching.
  3. Each mutation must cause a test failure. A mutation that survives means the corresponding proof is decorative.

- **Validation**: Every added mutation is caught by at least one test.

### T049 – Extend routing measurement for eight destinations `[P]`

- **Purpose**: Isolation and accumulation need measurement, not assertion.

- **Steps**:
  1. Extend `src/testing/live_mixer_routing_measurement.rs` from two destinations to eight.
  2. Measure isolation: one send raised leaves the other seven below −60 dBFS from that source (NFR-007, SC-004).
  3. Measure accumulation: two tracks into one bus sum correctly, each scaled by its own send.
  4. Measure gate behavior: muted and solo-excluded tracks contribute zero wet signal (SC-005).
  5. Measure order sensitivity: A→B ≠ B→A by output difference (SC-002).
  6. Measure edit responsiveness (NFR-008): from an accepted occupancy edit, record the frames until the projection reflects it (must be ≤ 1) and the render blocks from activation until the change is audible in the output (must be ≤ 1). Correlate through the T047 checkpoints rather than a new timing channel.

- **Validation**: All measurements produce numeric evidence in the structured report.

### T050 – Wire the Makefile target and binary flag

- **Purpose**: The scene must be runnable by its stable name.

- **Steps**:
  1. Add `demo-live-effects-and-buses` to `.PHONY` and as a target running `cargo run --release --bin crest-synth -- --demo-live-effects-and-buses`.
  2. Add the flag to `src/bin/crest_synth.rs`.
  3. Repoint `demo-live` to this scene as the newest cumulative one.
  4. **Preserve every existing target and flag exactly** — `demo-live-graphical-shell`, `demo-live-semantic-view-model`, `demo-live-sixteen-track-mixer-routing`. `occurrence_map.yaml` sets `cli_commands: do_not_change`.

- **Validation**: `make help` lists the new target; all three prior targets still exist verbatim.

### T051 – Prove earlier phase scenes still run

- **Purpose**: The roadmap requires every completed phase scene to stay runnable so later work cannot replace earlier evidence.

- **Steps**:
  1. Run `make demo-live-graphical-shell`, `make demo-live-semantic-view-model`, and `make demo-live-sixteen-track-mixer-routing`.
  2. Each must complete successfully with its full teardown contract.
  3. Where a prior scene's report references retired send or global fields, update the **projection** it reads rather than the checkpoint identity it emits.

- **Validation**: All three prior scenes pass. Report any that do not rather than adjusting them to pass.

### T052 – Run the live scene on a physical device

- **Purpose**: This is the phase gate. It cannot be satisfied any other way.

- **Steps**:
  1. Run `make demo-live-effects-and-buses` on real hardware with a real window and physical audio output.
  2. Confirm visually and audibly that every declared behavior is demonstrated.
  3. Confirm the structured report is complete and every checkpoint correlates.
  4. Confirm clean teardown: zero active notes, window closed, stream released, worker shut down, graphs collected, normal parent-process exit.
  5. **A frozen window, timeout, dropped event, silent fallback, incomplete report, or teardown failure fails the phase.** Report the failure; do not work around it.
  6. Record the outcome honestly, including anything that did not work.

- **Validation**: A successful physical run, reported with its actual observations.

## Test Strategy

- The live scene itself (T046, T052) — the phase gate.
- Deterministic scene proof with two-run logical determinism (NFR-005).
- Mutation coverage (T048) proving the new seams are falsifiable.
- Routing measurements (T049) producing numeric isolation and accumulation evidence.
- Teardown and ownership proofs (NFR-006).
- New `tests/effects_and_buses.rs` as the integration surface for this phase's contract.
- All prior phase scenes and their tests (T051).

## Definition of Done

- `make demo-live-effects-and-buses` exists and covers every declared phase behavior.
- The scene plays real MIDI through the production path with no demo-only reducer or renderer.
- New checkpoints added; every existing checkpoint identity unchanged.
- Mutation harness catches all added mutations.
- Routing measurement covers eight destinations with numeric isolation evidence.
- All three prior phase scenes still run and pass.
- `demo-live` points at the new scene; prior targets preserved verbatim.
- **T052 has actually been run on hardware** and its real outcome reported.
- `make lint`, `make fmt-check`, and `make test` pass.

## Risks & Mitigations

- **Reporting T052 as done without running it** → the single most damaging outcome for this WP, because it converts the phase gate into a claim. Run it; report what actually happened, including failures.
- **Renaming an existing checkpoint** → breaks comparability of retained evidence. Add only.
- **A mutation that survives** → means that proof is decorative. Treat a surviving mutation as a failing test.
- **Order change inaudible in the scene** → use two genuinely different processors (available since WP02) rather than two instances of one.
- **Silent fallback anywhere in the scene** → forbidden; it must fail loudly instead.

## Reviewer Guidance

- **Ask whether T052 was actually run on hardware**, and look for the real report rather than a summary of intent.
- Diff checkpoint identities against the previous phase's scene; any rename is a defect.
- Confirm the scene drives everything through semantic actions and `AppState::apply` — grep for direct state mutation.
- Confirm the controlled rejection is exercised and its audio continuity observed.
- Confirm all three prior `demo-live-*` targets still exist and still pass.
- Confirm the added mutations genuinely fail without the fix.
