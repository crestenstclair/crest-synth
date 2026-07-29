---
work_package_id: WP03
title: Ordered effect slots on the Patch
dependencies:
- WP02
requirement_refs:
- FR-001
- FR-004
- FR-005
- FR-018
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: worktree-per-lane
subtasks:
- T013
- T014
- T015
- T016
- T017
- T018
history:
- timestamp: '2026-07-29T02:11:28Z'
  actor: planner
  action: created
agent_profile: implementer-ivan
authoritative_surface: src/synth/patch.rs
create_intent: []
execution_mode: code_change
mission_id: 01KYNGX8QA8V49BX2WQ1Q6G2BP
mission_slug: expandable-effects-and-bus-topology-01KYNGX8
model: ''
owned_files:
- src/synth/patch.rs
- src/synth/effect_composition.rs
- src/synth/effect_slot_id.rs
- src/synth/prepared_post_effect_rack_builder.rs
- src/real_time/prepared_post_effect_rack.rs
- src/real_time/patch_effect_observation.rs
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP03 – Ordered effect slots on the Patch

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

This loads your identity, boundaries, and governance context. Do not skip this step.
Once loaded, continue with the Objective below.

## Objective

Replace "zero or one effect per Patch" with three ordered slots, each independently
occupied, each carrying its own values and its own instance state. Slot order is
render order, so exchanging two effects must change the sound.

The subtle part is not the count. It is that `PreparedPostEffectRack::matches_parameters`
currently proves an **exact** one-to-one correspondence between Patch index and slot,
and that proof must stay exact after widening. A permissive version would silently
accept mismatched layouts, which is precisely the class of bug the real-time design
exists to prevent.

## Context

- **Mission**: expandable-effects-and-bus-topology-01KYNGX8
- **Priority**: P1
- **Dependencies**: WP02 (needs the role-independent registry)
- **Related requirements**: FR-001 (three ordered slots), FR-004 (order-faithful processing), FR-005 (independent instance state), FR-018 (chain follows the Patch across rerouting)
- **Read first**: `data-model.md` § Patch and § Prepared graph
- **Parallel with**: WP04 — different contexts, disjoint files, both unblocked by WP02

## Branch Strategy

- **Planning base branch**: `feat/expandable-effects-and-bus-topology`
- **Merge target branch**: `feat/expandable-effects-and-bus-topology`
- **Execution**: worktree-per-lane. `finalize-tasks` computes lanes and writes `lanes.json`; each lane gets exactly one worktree and one branch.
- Do not create ad-hoc branches by hand; use the workspace the runtime resolves for this WP's lane.

## Subtasks

### T013 – Replace `post_effects` with a bounded ordered slot array

- **Purpose**: `Patch` currently holds `post_effects: Vec<PostEffectConfig>` (src/synth/patch.rs:77) — unbounded in the type, bounded only by convention.

- **Steps**:
  1. Introduce `MAX_EFFECT_SLOTS = 3`, sourced from the DESIGN.md:690 product maximum and C-001.
  2. Change the field to `effects: [Option<EffectConfig>; MAX_EFFECT_SLOTS]`. A `Vec` cannot express "exactly three positions, any of which may be empty", and its heap allocation is the wrong shape for a value that feeds a fixed real-time layout.
  3. Update `Patch::with_post_effects` (patch.rs:148) and the constructor at patch.rs:96 accordingly.
  4. Position is meaningful and stable: clearing slot 1 must leave slot 2 occupied at index 2, not compact it down to index 1.

- **Validation**: A Patch can hold zero, one, two, or three effects; a fourth is unrepresentable in the type, not merely rejected at runtime.

### T014 – Add `EffectSlotIndex` and occupancy transitions

- **Purpose**: Distinguish *position* from *configured instance*. `EffectSlotId` already identifies an instance; position needs its own type.

- **Steps**:
  1. Add `EffectSlotIndex` (0..`MAX_EFFECT_SLOTS`) as a validated newtype. Out-of-range construction fails; it is never clamped.
  2. Add the occupancy transition `SetSlotOccupancy(EffectSlotIndex, Option<RegistryEntryId>)` as a domain operation on `Patch`. WP06 owns wiring it to a semantic action — you provide the domain-level transition it will call.
  3. Occupancy change is **structural**: it changes what exists, so it cannot be a scalar snapshot value. Do not attempt to publish it as one.
  4. Do not name any slot. There is no `EffectSlotIndex::Chorus`.

- **Validation**: Setting, replacing, and clearing occupancy at each of the three positions behaves correctly and preserves the other two.

### T015 – Widen `PreparedPostEffectRack` to three slots per Patch

- **Purpose**: The rack currently holds `slots: [Option<PreparedPostEffectSlot>; MAX_PATCHES]` — one per Patch.

- **Steps**:
  1. Change to `[[Option<PreparedPostEffectSlot>; MAX_EFFECT_SLOTS]; MAX_PATCHES]`.
  2. Update `from_slots`, `slot_id`, and `scalar_count` accessors to take a slot index alongside the Patch index.
  3. Update `src/synth/prepared_post_effect_rack_builder.rs` to build all three positions.
  4. Each occupied slot keeps its **own** `input_scratch`, preallocated. Three occupied slots on one Patch means three scratch buffers, not one shared.
  5. Capacity is reserved for the full 16 × 3 grid regardless of how many are occupied — the render path must never grow.

- **Validation**: A fully occupied configuration (16 patches × 3 slots) prepares successfully with all scratch preallocated.

### T016 – Keep `matches_parameters` exact at the widened size

- **Purpose**: **The highest-risk subtask in this package.** This is the check that proves snapshot layout and prepared rack agree.

- **Steps**:
  1. Read the current implementation at `prepared_post_effect_rack.rs:74-92`. It asserts, per Patch: patch identity matches; and either the slot is absent and the parameters are inactive, or the slot is present and both `slot_id` and `scalar_count` match.
  2. Extend it to iterate all three slot positions per Patch with the **same** strictness at each position.
  3. Do not weaken any condition to make the widened version pass. Specifically: an occupied prepared slot facing inactive parameters must still be a mismatch, and vice versa.
  4. If the widened check becomes awkward, that is a signal the layout is wrong — not a reason to relax the check.

- **Validation**: Deliberately construct mismatched layouts — wrong slot_id at position 2, wrong scalar_count at position 0, occupied-vs-inactive at position 1 — and assert each is rejected.

### T017 – Process slots in index order, in place

- **Purpose**: Slot order is render order (FR-004).

- **Steps**:
  1. In `PreparedPostEffectRack::process`, iterate each Patch's slots by ascending index.
  2. Each occupied slot processes the stem **in place**, so slot 1 receives slot 0's output. This is what makes ordering audible.
  3. Preserve the existing stem identity checks — `StemIdentityMismatch` must still fire if a stem's Patch identity disagrees.
  4. Preserve the frame-capacity check per slot; each slot's scratch must be large enough for the block.
  5. Extend `PatchEffectObservation` so measurement covers each slot rather than one per Patch.

- **Validation**: With effects A and B configured as [A, B], output equals B(A(x)). Configured as [B, A], output equals A(B(x)). These must differ measurably.

### T018 – Prove instance independence and order sensitivity

- **Purpose**: FR-005 and FR-004 need falsifiable proof, not assertion.

- **Steps**:
  1. **Independence**: configure the same registry entry in two slots on one Patch. Assert neither instance's output is altered by the other's presence, and that their delay lines, LFO phase, and tails are disjoint. DESIGN.md:418 already sanctions two Chorus instances for exactly this proof.
  2. **Order sensitivity**: with reverb, delay, and chorus now all available from WP02, configure two genuinely different processors and assert A→B ≠ B→A by measured output difference.
  3. **Rerouting independence** (FR-018): change a Patch's output track and assert its effect chain, values, and instance state are unchanged.
  4. **Exhaustive positions**: prove each of the three positions can be independently occupied and cleared.

- **Validation**: All four proofs pass and are sample-exact where they compare output.

## Test Strategy

- Layout and boundary tests for the slot array and `EffectSlotIndex`.
- Negative tests for `matches_parameters` at each slot position (T016) — the most important tests in this package.
- Order-sensitivity measurement (T018) proving A→B ≠ B→A.
- Instance-independence measurement extending the existing two-Chorus proof.
- Rerouting-preservation test for FR-018.
- Existing `tests/static_patch_effect.rs` must continue to pass or be updated deliberately — it encodes the zero-or-one assumption.

## Definition of Done

- `Patch` holds exactly three ordered, independently occupiable slots; a fourth is unrepresentable.
- Slot positions are stable; clearing one does not compact the others.
- `PreparedPostEffectRack` carries a 16 × 3 grid with per-slot preallocated scratch.
- `matches_parameters` is exact at every slot position, with negative tests proving it.
- Slots process in index order, in place.
- Independence, order sensitivity, and rerouting preservation are proven by measurement.
- No slot or effect is named in any type.
- `make lint`, `make fmt-check`, and `make test` pass.

## Risks & Mitigations

- **Relaxing `matches_parameters` to make widening compile** → the single most damaging thing this WP could do. Write the negative tests in T016 *before* the widening, so a permissive version fails immediately.
- **Sharing one scratch buffer across a Patch's slots** → breaks in-place chaining and instance independence. Each slot owns its own.
- **Compacting slots on clear** → makes position unstable and breaks focus (WP07). Positions are fixed addresses.
- **Scope creep into the snapshot** → `parameter_snapshot.rs` belongs to WP05. You widen the rack; WP05 widens the snapshot to match.

## Reviewer Guidance

- **Read `matches_parameters` first and look for weakened conditions.** Compare against the original at `prepared_post_effect_rack.rs:74-92` line by line.
- Confirm negative tests exist for mismatches at each slot position — not just position 0.
- Confirm each occupied slot has its own `input_scratch`.
- Confirm in-place processing: slot 1's input must be slot 0's output.
- Confirm clearing slot 1 leaves slot 2 at index 2.
- Confirm `src/real_time/parameter_snapshot.rs` and `src/mixer/` are untouched.
