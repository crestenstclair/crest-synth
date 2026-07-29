---
description: "Work packages for expandable effects and bus topology"
---

# Tasks: Expandable Effects and Bus Topology

**Input**: Design documents from `kitty-specs/expandable-effects-and-bus-topology-01KYNGX8/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md, occurrence_map.yaml

**Tests**: Tests ARE requested. This project's charter is measured, falsifiable proof
using the production reducer and render path; `ROADMAP.md` makes a retained live demo
scene a phase-completion gate. Testing work is therefore first-class here, not optional.

**Organization**: Work packages roll up fine-grained subtasks. Each is independently
implementable and testable.

## Format: `WPxx` (work package) + `Txxx` (subtask)

- **Subtask completion is event-sourced.** Rows below are reference rows, not checkboxes.
  Record completion with `spec-kitty agent tasks mark-status T001 T002 --status done`.

## Path Conventions

Single Rust project: `src/` and `tests/` at repository root, seven bounded contexts
under `src/` (kernel, synth, mixer, real_time, control, shell, adapter, testing).

## The governing rule

Every work package below serves one rule from `plan.md`: **no name-enumerated effect
or routing identity**. If a WP leaves behind a type, variant, field, or descriptor
named after a specific effect or bus, that WP is not done — regardless of whether its
functional requirement passes. WP09 makes this mechanically enforceable.

## Subtask Index

| Subtask | Description | Work Package | Parallel |
|---------|-------------|--------------|----------|
| T001 | Narrow `nonGoals.additional_effects` in project.yaml | WP01 | |
| T002 | Remove Phase 3 clause from `nonGoals.later_roadmap_phases` | WP01 | |
| T003 | Replace `meta.avoid` enumerated-effect rules | WP01 | |
| T004 | Add capability, goal, requirements, evidence, validation, witness | WP01 | |
| T005 | Declare the no-name-enumeration invariant in proof/invariants.yaml | WP01 | |
| T006 | Restate DESIGN.md lines 309, 418, 689, 691 | WP01 | [P] |
| T007 | Make effect registry entries role-independent | WP02 | |
| T008 | Extract reverb DSP into a registry capability provider | WP02 | [P] |
| T009 | Extract reverb preparer behind `EffectPreparer` | WP02 | [P] |
| T010 | Extract delay DSP into a registry capability provider | WP02 | [P] |
| T011 | Extract delay preparer behind `EffectPreparer` | WP02 | [P] |
| T012 | Retire `global_reverb_delay.rs` and register new entries | WP02 | |
| T013 | Replace `Patch::post_effects` with a bounded ordered slot array | WP03 | |
| T014 | Add `EffectSlotIndex` and slot occupancy transitions | WP03 | |
| T015 | Widen `PreparedPostEffectRack` to 3 slots per Patch | WP03 | |
| T016 | Keep `matches_parameters` exact at the widened size | WP03 | |
| T017 | Process slots in index order, in place | WP03 | |
| T018 | Prove instance independence and order sensitivity | WP03 | |
| T019 | Add bounded validated `BusId` | WP04 | |
| T020 | Replace named send fields with an indexed send array | WP04 | |
| T021 | Reduce `GlobalParameters` to master gain | WP04 | |
| T022 | Add the `BusReturn` aggregate with return level | WP04 | |
| T023 | Add `PreparedBusReturnRack` and retire `GlobalEffectsProcessor` | WP04 | |
| T024 | Generalize send accumulation in `MixEngine` | WP04 | |
| T025 | Prove send position, isolation, and no-passthrough | WP04 | |
| T026 | Widen `RtPatchParameters` to 3 effect slots | WP05 | |
| T027 | Add indexed sends to real-time track parameters | WP05 | |
| T028 | Add `RtBusReturnParameters` to the snapshot | WP05 | |
| T029 | Extend `SERIALIZED_LEAF_DESCRIPTOR` for every new leaf | WP05 | |
| T030 | Wire the return rack into `PreparedGraph` and the renderer | WP05 | |
| T031 | Measure publish cost and prove zero render-time growth | WP05 | |
| T032 | Add slot and return occupancy semantic actions | WP06 | |
| T033 | Validate occupancy changes before publication | WP06 | |
| T034 | Route occupancy through the structural preparation worker | WP06 | |
| T035 | Prove controlled rejection leaves the active graph intact | WP06 | |
| T036 | Project pending, accepted, and refused outcomes with reason | WP06 | |
| T037 | Prove recovery after rejection and acknowledgement ordering | WP06 | |
| T038 | Prove off-callback retirement at the widened size | WP06 | |
| T039 | Extend PATCH focus order with slot rows | WP07 | |
| T040 | Add adjacent-choice edit on slot and return rows | WP07 | |
| T041 | Project descriptor-driven slot parameters | WP07 | |
| T042 | Extend MIXER projection with indexed sends and returns | WP07 | |
| T043 | Prove deterministic focus recovery when rows disappear | WP07 | |
| T044 | Render slot and return rows in the shell | WP07 | |
| T045 | Prove PATCH and MIXER remain the only top-level contexts | WP07 | |
| T046 | Add the `demo-live-effects-and-buses` scene | WP08 | |
| T047 | Add phase checkpoints without altering existing identities | WP08 | |
| T048 | Extend the behavioral mutation harness | WP08 | [P] |
| T049 | Extend routing measurement for 8 destinations | WP08 | [P] |
| T050 | Wire the Makefile target and binary flag | WP08 | |
| T051 | Prove earlier phase scenes still run | WP08 | |
| T052 | Run the live scene on a physical device | WP08 | |
| T053 | Implement the no-name-enumeration static check | WP09 | |
| T054 | Seed it with the retired identifiers and contexts | WP09 | |
| T055 | Register it as a project check | WP09 | |

*Reference table only. `[P]` marks parallel-safe work; it is not a status column.*

## Phase 1: Reconciliation (must land first)

### WP01 – Architecture and design reconciliation (Priority: P1)

- **Goal**: Narrow the declarations that make this mission a non-goal, and add the capability, goal, requirements, and proof entries it needs.
- **Independent test**: `spec-kitty context architecture` reloads cleanly and no longer contradicts the mission; DESIGN.md no longer states the zero-or-one bound.
- **Prompt**: `tasks/WP01-architecture-reconciliation.md`
- **Dependencies**: none
- **Subtasks**:
  T001 Narrow `nonGoals.additional_effects` in project.yaml
  T002 Remove Phase 3 clause from `nonGoals.later_roadmap_phases`
  T003 Replace `meta.avoid` enumerated-effect rules
  T004 Add capability, goal, requirements, evidence, validation, witness
  T005 Declare the no-name-enumeration invariant in proof/invariants.yaml
  T006 Restate DESIGN.md lines 309, 418, 689, 691
- **Risks / Notes**: C-009 makes this non-deferrable. Narrowing too far would silently pull the deferred twelve-effect roster into scope — C-011 must survive the edit. Estimated prompt: ~330 lines.

## Phase 2: Foundational generalization

### WP02 – Role-independent effect registry (Priority: P1)

- **Goal**: Make one descriptor-driven registry the single source of effect identity, usable identically by a Patch slot and a bus return, with reverb and delay as ordinary entries.
- **Independent test**: One registry entry prepares into both roles and produces independent instances; reverb and delay sound unchanged.
- **Prompt**: `tasks/WP02-role-independent-effect-registry.md`
- **Dependencies**: WP01
- **Subtasks**:
  T007 Make effect registry entries role-independent
  T008 [P] Extract reverb DSP into a registry capability provider
  T009 [P] Extract reverb preparer behind `EffectPreparer`
  T010 [P] Extract delay DSP into a registry capability provider
  T011 [P] Extract delay preparer behind `EffectPreparer`
  T012 Retire `global_reverb_delay.rs` and register new entries
- **Risks / Notes**: Reverb and delay must sound identical after the move — this is a migration, not a redesign. Estimated prompt: ~400 lines.

### WP03 – Ordered effect slots on the Patch (Priority: P1)

- **Goal**: Replace one optional effect per Patch with three ordered, independently occupied slots.
- **Independent test**: A Patch configured with three effects processes them in slot order; exchanging two produces measurably different output.
- **Prompt**: `tasks/WP03-ordered-effect-slots.md`
- **Dependencies**: WP02
- **Subtasks**:
  T013 Replace `Patch::post_effects` with a bounded ordered slot array
  T014 Add `EffectSlotIndex` and slot occupancy transitions
  T015 Widen `PreparedPostEffectRack` to 3 slots per Patch
  T016 Keep `matches_parameters` exact at the widened size
  T017 Process slots in index order, in place
  T018 Prove instance independence and order sensitivity
- **Risks / Notes**: `matches_parameters` currently proves exact one-to-one Patch/slot correspondence. A permissive widening silently accepts mismatched layouts. Estimated prompt: ~390 lines.

### WP04 – Bus identity, indexed sends, and returns (Priority: P1)

- **Goal**: Introduce validated bus identities, replace the two named send fields with an indexed array, and add the bounded return rack that retires the global effects port.
- **Independent test**: Eight destinations are addressable; a send raised toward one leaves the other seven silent; a muted track feeds nothing.
- **Prompt**: `tasks/WP04-bus-identity-sends-returns.md`
- **Dependencies**: WP02
- **Subtasks**:
  T019 Add bounded validated `BusId`
  T020 Replace named send fields with an indexed send array
  T021 Reduce `GlobalParameters` to master gain
  T022 Add the `BusReturn` aggregate with return level
  T023 Add `PreparedBusReturnRack` and retire `GlobalEffectsProcessor`
  T024 Generalize send accumulation in `MixEngine`
  T025 Prove send position, isolation, and no-passthrough
- **Risks / Notes**: Post-fader/post-gate send position and "mute always wins" must survive byte-for-byte in behavior. A muted track feeding a wet return is a real regression no current unit test catches by accident. Estimated prompt: ~450 lines.

### WP05 – Widened real-time transport (Priority: P1)

- **Goal**: Grow the single fixed latest-value snapshot to carry 16×3 slots, 16×8 sends, and 8 returns while preserving fixed layout, exact matching, and callback safety.
- **Independent test**: Layout and descriptor completeness tests pass; zero render-time growth measured under a fully occupied configuration.
- **Prompt**: `tasks/WP05-widened-realtime-transport.md`
- **Dependencies**: WP03, WP04
- **Subtasks**:
  T026 Widen `RtPatchParameters` to 3 effect slots
  T027 Add indexed sends to real-time track parameters
  T028 Add `RtBusReturnParameters` to the snapshot
  T029 Extend `SERIALIZED_LEAF_DESCRIPTOR` for every new leaf
  T030 Wire the return rack into `PreparedGraph` and the renderer
  T031 Measure publish cost and prove zero render-time growth
- **Risks / Notes**: **Highest real-time risk in the mission.** The block roughly triples; publish cost must be measured, not assumed (contract C-RT-7). Estimated prompt: ~410 lines.

## Phase 3: Behavior

### WP06 – Topology change lifecycle and rejection (Priority: P1)

- **Goal**: Route slot and return occupancy changes through the correlated structural-edit lifecycle with validation, visible outcome, controlled rejection, recovery, and off-callback retirement.
- **Independent test**: A refused change leaves audio uninterrupted and the prior configuration intact; a valid change immediately after succeeds.
- **Prompt**: `tasks/WP06-topology-lifecycle-and-rejection.md`
- **Dependencies**: WP05
- **Subtasks**:
  T032 Add slot and return occupancy semantic actions
  T033 Validate occupancy changes before publication
  T034 Route occupancy through the structural preparation worker
  T035 Prove controlled rejection leaves the active graph intact
  T036 Project pending, accepted, and refused outcomes with reason
  T037 Prove recovery after rejection and acknowledgement ordering
  T038 Prove off-callback retirement at the widened size
- **Risks / Notes**: Two changes requested before the first is acknowledged must neither reorder nor drop acknowledgements. Estimated prompt: ~460 lines.

### WP07 – Semantic focus and projection (Priority: P2)

- **Goal**: Extend reducer-owned focus and projections to slot and return rows using the existing adjacent-choice contract, with deterministic focus recovery.
- **Independent test**: Focus traverses slot and return rows, edits occupancy by adjacent choice, and resolves deterministically when a row disappears.
- **Prompt**: `tasks/WP07-semantic-focus-and-projection.md`
- **Dependencies**: WP06
- **Subtasks**:
  T039 Extend PATCH focus order with slot rows
  T040 Add adjacent-choice edit on slot and return rows
  T041 Project descriptor-driven slot parameters
  T042 Extend MIXER projection with indexed sends and returns
  T043 Prove deterministic focus recovery when rows disappear
  T044 Render slot and return rows in the shell
  T045 Prove PATCH and MIXER remain the only top-level contexts
- **Risks / Notes**: Clearing a slot while its parameters hold focus must resolve deterministically rather than leaving focus dangling. C-008 forbids introducing a choice modal. Estimated prompt: ~450 lines.

## Phase 4: Proof and guard

### WP08 – Retained live scene and measured proof (Priority: P1)

- **Goal**: Deliver `make demo-live-effects-and-buses` and the deterministic, mutation, and real-time proofs that make every declared behavior falsifiable.
- **Independent test**: The scene runs on a physical device with a real window and real MIDI, covers every declared behavior, and completes the full teardown contract.
- **Prompt**: `tasks/WP08-retained-live-scene.md`
- **Dependencies**: WP07
- **Subtasks**:
  T046 Add the `demo-live-effects-and-buses` scene
  T047 Add phase checkpoints without altering existing identities
  T048 [P] Extend the behavioral mutation harness
  T049 [P] Extend routing measurement for 8 destinations
  T050 Wire the Makefile target and binary flag
  T051 Prove earlier phase scenes still run
  T052 Run the live scene on a physical device
- **Risks / Notes**: `logs_telemetry: do_not_change` — existing checkpoint identities must stay byte-identical or retained evidence stops being comparable. T052 cannot be satisfied headlessly. Estimated prompt: ~470 lines.

### WP09 – No-name-enumeration guard (Priority: P2)

- **Goal**: Make the open-closed property mechanically enforceable so a future closed shortcut fails the build rather than the review.
- **Independent test**: The check fails when a name-enumerated effect or routing identifier is reintroduced, and passes on the delivered tree.
- **Prompt**: `tasks/WP09-no-name-enumeration-guard.md`
- **Dependencies**: WP07
- **Subtasks**:
  T053 Implement the no-name-enumeration static check
  T054 Seed it with the retired identifiers and contexts
  T055 Register it as a project check
- **Risks / Notes**: Must not false-positive on `MasterGainDb` or on genuinely singular concepts. Deliberately narrow: it constrains effect and routing identity, not all naming. Estimated prompt: ~250 lines.

## Dependencies & Execution Order

```
WP01 (reconciliation)
  └── WP02 (registry)
        ├── WP03 (slots) ─┐
        └── WP04 (buses) ─┴── WP05 (transport)
                                 └── WP06 (lifecycle)
                                       └── WP07 (focus/projection)
                                             ├── WP08 (live scene)
                                             └── WP09 (guard)
```

- WP01 must land before or alongside the first implementation WP, never after (C-009).
- WP03 and WP04 are the main parallelization opportunity — different contexts, disjoint files, both unblocked by WP02.
- WP08 and WP09 can run concurrently once WP07 lands.

## Parallel Execution Examples

- **WP03 ∥ WP04** — slots live in `src/synth/`, buses in `src/mixer/`. No file overlap.
- Within WP02, T008–T011 are `[P]`: reverb and delay capability/preparer extraction are four independent files.
- Within WP08, T048 and T049 are `[P]`: harness and measurement are separate files.

## Implementation Strategy

**MVP scope**: WP01 → WP02 → WP03. That delivers ordered effect slots on a Patch with
a role-independent registry — spec User Story 1, the highest-priority journey,
independently testable and audible on its own.

Full delivery adds WP04–WP05 (User Story 2), WP06 (User Story 3), then WP07–WP09.

Validate each package independently before advancing. Use the prompts in
`tasks/` for detailed execution guidance, and read `quickstart.md` first — its Traps
section lists the failure modes that unit tests will not catch for you.
