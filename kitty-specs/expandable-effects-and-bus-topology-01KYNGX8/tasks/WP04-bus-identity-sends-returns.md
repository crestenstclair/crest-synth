---
work_package_id: WP04
title: Bus identity, indexed sends, and returns
dependencies:
- WP02
requirement_refs:
- FR-006
- FR-007
- FR-008
- FR-010
- FR-011
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T019
- T020
- T021
- T022
- T023
- T024
- T025
history:
- timestamp: '2026-07-29T02:11:28Z'
  actor: planner
  action: created
agent_profile: implementer-ivan
authoritative_surface: src/mixer/
create_intent:
- src/mixer/bus_id.rs
- src/mixer/bus_return.rs
- src/real_time/prepared_bus_return_rack.rs
execution_mode: code_change
mission_id: 01KYNGX8QA8V49BX2WQ1Q6G2BP
mission_slug: expandable-effects-and-bus-topology-01KYNGX8
model: ''
owned_files:
- src/mixer/**
- src/real_time/prepared_bus_return_rack.rs
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP04 – Bus identity, indexed sends, and returns

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

This loads your identity, boundaries, and governance context. Do not skip this step.
Once loaded, continue with the Objective below.

## Objective

Replace two hardcoded aux paths with eight indexed ones, and delete the port that
made them hardcoded. `MixerTrackParameter::ReverbSend`/`DelaySend` become one indexed
send array; `GlobalEffectsProcessor::process(reverb_input, delay_input, ...)` is
deleted outright; `GlobalParameter`'s six reverb and delay fields dissolve into
registry descriptor scalars and a per-return level.

`ReverbSend` and `DelaySend` are not two kinds of parameter. They are one kind of
parameter — a send level — pointing at two different destinations. Encoding the
destination in the parameter's *name* conflates identity with addressing. Once
separated, eight sends cost no more type surface than two, and the twelfth roster
effect costs none.

The behavior that must **not** change: send position, gate order, and meter
position. Generalizing the count must not move the stage.

## Context

- **Mission**: expandable-effects-and-bus-topology-01KYNGX8
- **Priority**: P1
- **Dependencies**: WP02 (returns draw from the registry it establishes)
- **Related requirements**: FR-006 (explicit bus identities), FR-007 (eight returns), FR-008 (sends address bus identities), FR-010 (configurable return contents), FR-011 (preserved send semantics)
- **Read first**: `contracts/bus-routing.md` — obligations C-BR-1 through C-BR-10
- **Reference**: `research.md` R-02, R-03, R-04
- **Parallel with**: WP03 — different contexts, disjoint files

## Branch Strategy

- **Planning base branch**: `feat/expandable-effects-and-bus-topology`
- **Merge target branch**: `feat/expandable-effects-and-bus-topology`
- **Execution**: worktree-per-lane. `finalize-tasks` computes lanes and writes `lanes.json`; each lane gets exactly one worktree and one branch.
- Do not create ad-hoc branches by hand; use the workspace the runtime resolves for this WP's lane.

## Subtasks

### T019 – Add bounded validated `BusId`

- **Purpose**: A routing destination needs a stable positional identity, independent of what currently occupies it.

- **Steps**:
  1. Create `src/mixer/bus_id.rs` with `MAX_BUS_RETURNS = 8`, modelled on the existing `src/mixer/mixer_track_id.rs` (which has `COUNT` and `ALL`).
  2. Construction validates the index is in range. Out-of-range is **rejected**, never clamped and never defaulted — this mirrors how invalid `MixerTrackId` values are handled today.
  3. Provide `COUNT` and `ALL` for exhaustive iteration.
  4. **No named constructors.** There is no `BusId::reverb()`. If you are tempted to add one for convenience, that is the exact fault this mission exists to remove.

- **Validation**: In-range construction succeeds for 0..7; 8 and above are rejected. Obligations B-1, B-2, B-3.

### T020 – Replace named send fields with an indexed send array

- **Purpose**: `MixerTrackParameter` (src/mixer/mixer_track_parameters.rs) is a 6-variant enum with `ReverbSend` and `DelaySend`, split into `MAIN: [4]` and `INSPECTOR: [2]`, driving descriptors, serde, focus order, and projections.

- **Steps**:
  1. Reduce the enum to the four genuine fader controls: `Level`, `Pan`, `Mute`, `Solo`. Keep `MAIN` as these four.
  2. Move sends to `sends: [BusSendLevel; MAX_BUS_RETURNS]` on `MixerTrackParameters`, replacing the `reverb_send` and `delay_send` fields.
  3. Address a send by `(MixerTrackId, BusId)`. The send's descriptor is one shared descriptor — range 0.0..=1.0, default 0.0, fine 0.01, coarse 0.1 — copied exactly from the current `ReverbSend`/`DelaySend` descriptors. All eight sends share it; do not author eight copies.
  4. `INSPECTOR` becomes the eight send addresses rather than two named variants.
  5. Update `new`, `scalar_value`, `toggle_value`, `with_scalar_value`, `toggled`, `Default`, and the custom `Deserialize` impl. Note the deserialize impl deliberately routes through `new` so validation cannot be bypassed — preserve that property.
  6. Serde keys: `occurrence_map.yaml` sets `serialized_keys: rename`. Emit indexed names (`send0`..`send7` or equivalent), not `reverbSend`.

- **Validation**: Every one of eight sends validates its range; serde cannot bypass validation; no variant is named after an effect.

### T021 – Reduce `GlobalParameters` to master gain

- **Purpose**: `GlobalParameter` holds `MasterGainDb`, `ReverbRoomSize`, `ReverbDamping`, `ReverbReturn`, `DelayMilliseconds`, `DelayFeedback`, `DelayReturn`. Six of those seven belong elsewhere now.

- **Steps**:
  1. `ReverbRoomSize`, `ReverbDamping`, `DelayMilliseconds`, `DelayFeedback` → already re-declared as registry descriptor scalars by WP02. Delete them here.
  2. `ReverbReturn`, `DelayReturn` → become the per-return level owned by `BusReturn` (T022). Delete them here.
  3. `MasterGainDb` stays. It is genuinely global — a property of the master stage, not of any effect. This is the one documented exception to the no-name-enumeration invariant.
  4. Update `src/mixer/global_parameters.rs` (444 lines, ~111 occurrences) and its descriptor table.

- **Validation**: `GlobalParameter` has exactly one variant. Master gain behavior is unchanged.

### T022 – Add the `BusReturn` aggregate

- **Purpose**: A return is a destination that may hold zero or one registry effect and owns its own output level.

- **Steps**:
  1. Create `src/mixer/bus_return.rs`: `BusReturn { id: BusId, effect: Option<EffectConfig>, return_level: f32 }`.
  2. Return level is **return-owned**, not a descriptor scalar of the effect (R-04). Changing which effect occupies a return must not reset or lose the return level.
  3. Add the occupancy transition `SetReturnOccupancy(BusId, Option<RegistryEntryId>)` as a domain operation. WP06 wires it to a semantic action.
  4. Effects come from the **same registry** as Patch slots (FR-010) — no role filter, no send-suitability flag.
  5. An unoccupied return contributes **silence**. It must not pass its input through. This is a real trap: a naive implementation that skips processing and leaves the accumulated input in the buffer will pass dry signal into the mix.

- **Validation**: Occupancy can be set, replaced, and cleared on each of eight returns; return level survives an effect change; an empty return contributes silence. Obligations C-BR-6, C-BR-10.

### T023 – Add `PreparedBusReturnRack` and retire `GlobalEffectsProcessor`

- **Purpose**: Replace the two-input port with a bounded rack that is a peer of `PreparedPostEffectRack`.

- **Steps**:
  1. Create `src/real_time/prepared_bus_return_rack.rs` with `[Option<PreparedReturn>; MAX_BUS_RETURNS]`, each with its own preallocated input scratch. Model its structure and its exactness checks on `prepared_post_effect_rack.rs`.
  2. Delete `src/mixer/global_effects_processor.rs` entirely, including the `GlobalEffectsProcessor` trait and its `process(reverb_input, delay_input, output, parameters)` signature. `occurrence_map.yaml` declares this move.
  3. **Preserve `EffectError`** — its variants (`InvalidSampleRate`, `InvalidMaxFrames`, `InvalidMaxDelayMilliseconds`, `StorageAllocationFailed`) are the preparation vocabulary the new preparers still need. Relocate it rather than deleting it.
  4. Carry forward the two documented input obligations as return-rack obligations: wet excitation derives exclusively from the declared input, never from samples already in the output (C-BR-7); zero input cannot produce a wet return (C-BR-8). These are currently documented at `global_effects_processor.rs:47-53` — do not lose them in the deletion.
  5. A return sums into the dry mix and **cannot feed another return**. The topology must have no return-to-send edge (C-BR-9).

- **Validation**: The rack prepares eight returns with preallocated scratch; no routing cycle is expressible in the type; `EffectError` survives.

### T024 – Generalize send accumulation in `MixEngine`

- **Purpose**: `mix_engine.rs:164-167` accumulates into `self.reverb_input` and `self.delay_input` with two hardcoded sends.

- **Steps**:
  1. Replace the two scratch buffers with `[Vec<f32>; MAX_BUS_RETURNS]`, all preallocated in `prepare`.
  2. Replace the two accumulation lines with a loop over `BusId::ALL`, scaling by `track_parameters.send(bus)`.
  3. **Do not move the send stage.** It sits after track level/pan, after the pre-gate meter, and after the mute/solo gate. Read lines 127-170 carefully before editing: `audible` is computed at line 138 from mute and any-solo, and sends only accumulate for audible tracks. That property is FR-011 and C-BR-2 and must survive verbatim.
  4. Replace the `GlobalEffectsProcessor` call at line 181 with the return rack, then sum return outputs into the mix before master gain (line 184).
  5. Meters stay pre-gate at line 170 so muted tracks remain diagnosable.

- **Validation**: A muted track contributes nothing to any send; a solo-excluded track contributes nothing; meters still read pre-gate.

### T025 – Prove send position, isolation, and no-passthrough

- **Purpose**: The generalization is only correct if the gate semantics are bit-for-bit preserved.

- **Steps**:
  1. **Position (C-BR-1)**: sample-exact test that a send is taken after fader and after gate.
  2. **Mute wins (C-BR-2)**: muted track contributes no dry signal and no send.
  3. **Solo excludes (C-BR-2)**: when any track is soloed, non-soloed tracks contribute neither dry nor send.
  4. **Isolation (C-BR-3, C-BR-5, NFR-007)**: raise one send toward one bus; assert the other seven measure below −60 dBFS from that source.
  5. **Accumulation (C-BR-4)**: two tracks sending to one bus sum there; each send scales only its own contribution.
  6. **No passthrough (C-BR-6)**: an unoccupied return contributes silence, not its input.
  7. **No cycle (C-BR-9)**: structural assertion that no return-to-send edge exists.

- **Validation**: All seven proofs pass. Extend `src/mixer/mix_engine.rs` tests and coordinate with WP08 for the integration-level routing measurement.

## Test Strategy

- Boundary tests for `BusId` construction and for all eight send ranges.
- Serde tests proving validation cannot be bypassed (the existing `serde_cannot_bypass_numeric_validation` pattern, extended).
- The seven routing proofs in T025 — the core of this package's evidence.
- Empty-return silence test (the trap most likely to slip through).
- Existing `tests/sixteen_track_mixer_routing.rs` must keep passing; it encodes the sixteen-track guarantees this WP must not disturb.

## Definition of Done

- `BusId` is bounded, validated, and has no named constructors.
- `MixerTrackParameter` has exactly four variants; sends are an indexed array.
- `GlobalParameter` has exactly one variant (`MasterGainDb`).
- `BusReturn` owns its return level; contents come from the shared registry.
- `global_effects_processor.rs` is deleted; `EffectError` survives; both input obligations are carried forward.
- Send stage position, mute-wins, solo-exclusion, and pre-gate metering are unchanged.
- All seven routing proofs pass.
- No type names an effect or a bus.
- `make lint`, `make fmt-check`, and `make test` pass.

## Risks & Mitigations

- **The send stage moves during generalization** → the highest-consequence regression here, and no current unit test catches it by accident. Write the T025 position and gate tests *before* touching `mix_engine.rs`.
- **An empty return passes its input through** → assert silence explicitly; do not assume skipping processing is equivalent.
- **Losing the two input obligations when deleting the port** → they are documented at `global_effects_processor.rs:47-53`. Copy them into the new rack's docs and tests before deleting the file.
- **Deleting `EffectError` along with the port** → the new preparers still need that vocabulary. Relocate it.
- **Authoring eight send descriptors** → one shared descriptor, eight addresses.

## Reviewer Guidance

- **Diff `mix_engine.rs` lines 127-190 with extreme care.** Confirm `audible` still gates sends, that the send stage did not move relative to the fader/gate/meter, and that meters remain pre-gate.
- Confirm an unoccupied return is proven silent by an explicit test.
- Confirm `EffectError` still exists and the two input obligations appear in the new rack.
- Confirm `GlobalParameter` has exactly one variant and that `MasterGainDb` is documented as the invariant's exception.
- Confirm no `BusId::reverb()`-style named constructor exists.
- Confirm `src/synth/patch.rs` and `src/real_time/parameter_snapshot.rs` are untouched — WP03 and WP05 own those.
