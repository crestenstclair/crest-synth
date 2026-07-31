---
work_package_id: WP02
title: Role-independent effect registry
dependencies:
- WP01
requirement_refs:
- FR-003
- FR-005
- FR-009
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T007
- T008
- T009
- T010
- T011
- T012
history:
- timestamp: '2026-07-29T02:11:28Z'
  actor: planner
  action: created
agent_profile: implementer-ivan
authoritative_surface: src/synth/
create_intent:
- src/adapter/reverb_capability.rs
- src/adapter/reverb_preparer.rs
- src/adapter/delay_capability.rs
- src/adapter/delay_preparer.rs
execution_mode: code_change
mission_id: 01KYNGX8QA8V49BX2WQ1Q6G2BP
mission_slug: expandable-effects-and-bus-topology-01KYNGX8
model: ''
owned_files:
- src/synth/effect_capability.rs
- src/synth/effect_capability_provider.rs
- src/synth/effect_capability_id.rs
- src/synth/effect_preparer.rs
- src/synth/prepared_post_effect.rs
- src/adapter/reverb_capability.rs
- src/adapter/reverb_preparer.rs
- src/adapter/delay_capability.rs
- src/adapter/delay_preparer.rs
- src/adapter/global_reverb_delay.rs
- src/adapter/production_effects.rs
- src/adapter/mod.rs
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP02 – Role-independent effect registry

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

This loads your identity, boundaries, and governance context. Do not skip this step.
Once loaded, continue with the Objective below.

## Objective

Make one descriptor-driven effect registry the single source of effect identity,
schema, and preparation — usable identically by a Patch effect slot and by a bus
return. Reverb and delay stop being a hardcoded pair behind a two-input port and
become ordinary registry entries, peers of Chorus.

This is the foundation every other work package consumes. It is also the subtask
where the mission's rationale is most concrete: `GlobalEffectsProcessor::process(
reverb_input, delay_input, output, parameters)` encodes both the *number* and the
*identity* of returns in a type signature. No downstream generality can survive
that, which is why the port is deleted rather than widened.

## Context

- **Mission**: expandable-effects-and-bus-topology-01KYNGX8
- **Priority**: P1
- **Dependencies**: WP01 (the architecture spec must permit this before it lands)
- **Related requirements**: FR-003 (descriptor-driven parameters), FR-005 (independent instance state), FR-009 (reverb and delay as registry effects)
- **Read first**: `research/contracts/effect-registry.md` — obligations C-ER-1 through C-ER-5
- **Reference**: `research.md` R-02 (why the port is retired, not widened)

## Branch Strategy

- **Planning base branch**: `feat/expandable-effects-and-bus-topology`
- **Merge target branch**: `feat/expandable-effects-and-bus-topology`
- **Execution**: worktree-per-lane. `finalize-tasks` computes lanes and writes `lanes.json`; each lane gets exactly one worktree and one branch.
- Do not create ad-hoc branches by hand; use the workspace the runtime resolves for this WP's lane.

## Subtasks

### T007 – Make effect registry entries role-independent

- **Purpose**: A registry entry must not know or declare whether it may occupy a Patch slot or a bus return. Role admissibility is the caller's decision.

- **Steps**:
  1. Review `src/synth/effect_capability.rs` and `effect_capability_provider.rs`. The existing shape already declares identity, visible parameters, bounds, units, and preparation requirements — confirm none of it is Patch-specific.
  2. Remove or generalize anything that assumes a Patch context (naming, doc comments, parameter classification that presumes a Patch surface).
  3. Do **not** add a `role` or `is_send_suitable` field. The user's decision was that returns draw from the same registry with no role marker.
  4. Keep `MAX_EFFECT_SCALAR_PARAMETERS` at 8; it applies identically in both roles.

- **Validation**: A single registry entry can be resolved and prepared without any caller-supplied role hint. Obligation C-ER-1.

### T008 – Extract reverb DSP into a registry capability provider `[P]`

- **Purpose**: Reverb becomes an ordinary registry entry with descriptor scalars, mirroring `adapter/chorus_capability.rs`.

- **Steps**:
  1. Create `src/adapter/reverb_capability.rs` implementing `port.Synth.EffectCapabilityProvider`.
  2. Declare descriptor scalars **room size** and **damping**, taking their exact bounds, defaults, fine steps, coarse steps, and units from the current `GlobalParameter::ReverbRoomSize` and `::ReverbDamping` descriptors in `src/mixer/global_parameters.rs`. Do not invent new ranges — a changed range is an audible behavior change.
  3. Do **not** declare return level as a descriptor scalar. Return level belongs to the `BusReturn` (WP04), because it is a property of the destination rather than of the effect. See `research.md` R-04.
  4. Model the file structure on `chorus_capability.rs` so the three entries read as peers.

- **Validation**: The reverb entry's declared scalars match the retired global descriptors value-for-value.

### T009 – Extract the reverb preparer `[P]`

- **Purpose**: Reverb processing moves behind the generic `EffectPreparer` / `PreparedPostEffect` boundary.

- **Steps**:
  1. Create `src/adapter/reverb_preparer.rs` implementing `port.Synth.EffectPreparer`, modelled on `chorus_preparer.rs`.
  2. Move the reverb DSP out of `src/adapter/global_reverb_delay.rs` unchanged. **This is a migration, not a redesign** — the algorithm, coefficients, and internal state layout must not change.
  3. Allocate every buffer and delay line in `prepare`. `process` must remain allocation-free, lock-free, non-blocking, and free of I/O, logging, and destruction.
  4. Preserve the two obligations inherited from the retired port: wet excitation derives **exclusively** from the declared input, never from samples already in the output; and zero input cannot produce a wet return.

- **Validation**: A/B the rendered output of the old and new reverb paths on identical input. They must match. Obligations C-ER-3, C-BR-7, C-BR-8.

### T010 – Extract delay DSP into a registry capability provider `[P]`

- **Purpose**: Same as T008, for delay.

- **Steps**:
  1. Create `src/adapter/delay_capability.rs`.
  2. Declare descriptor scalars **milliseconds** and **feedback**, copying bounds, defaults, steps, and units exactly from `GlobalParameter::DelayMilliseconds` and `::DelayFeedback`.
  3. Return level is excluded here too — it belongs to the `BusReturn`.
  4. Note the existing `EffectError::InvalidMaxDelayMilliseconds` preparation vocabulary: the delay entry must declare its maximum-delay requirement so preparation can reject an unsatisfiable configuration.

- **Validation**: Declared scalars match the retired global descriptors value-for-value.

### T011 – Extract the delay preparer `[P]`

- **Purpose**: Same as T009, for delay.

- **Steps**:
  1. Create `src/adapter/delay_preparer.rs`.
  2. Move the delay DSP unchanged; preserve the delay-line sizing logic and its `max_delay_milliseconds` contract.
  3. Same real-time obligations as T009.
  4. Same two inherited input obligations as T009.

- **Validation**: A/B rendered output against the old delay path on identical input; they must match.

### T012 – Retire `global_reverb_delay.rs` and register the new entries

- **Purpose**: Complete the migration so exactly one prepared-effect boundary exists.

- **Steps**:
  1. Delete `src/adapter/global_reverb_delay.rs` once T008-T011 have absorbed its DSP. `occurrence_map.yaml` declares this move: `from: src/adapter/global_reverb_delay.rs` → `to: src/adapter`.
  2. Register reverb and delay in `src/adapter/production_effects.rs` alongside Chorus, in a stable declared order.
  3. Update `src/adapter/mod.rs` exports.
  4. Leave `src/mixer/global_effects_processor.rs` and `MixEngine`'s call site alone — WP04 owns retiring the port and the `GlobalParameters` reduction. Your job ends at making the registry entries exist.

- **Validation**: `cargo build` succeeds. Three registry entries are resolvable. Nothing outside this WP's `owned_files` was edited.

## Test Strategy

Tests are requested for this mission. This package needs:

- **Contract test (C-ER-1)**: prepare one registry entry into a Patch-slot context and into a return context; assert both succeed and produce independent instances.
- **Independence test (C-ER-2, FR-005)**: two prepared instances of one entry must not share delay lines, LFO phase, or tails. Extend the existing two-Chorus independence proof to cover reverb and delay.
- **Migration fidelity test**: A/B old vs new reverb and delay output on identical input, sample-exact.
- **Real-time contract**: the existing callback-safety validation must cover the new preparers.
- **Negative test (C-ER-4)**: an entry that fails preparation yields a refusal and publishes no partially prepared instance.

## Definition of Done

- Registry entries carry no role marker; role is the caller's decision.
- Reverb and delay exist as capability + preparer pairs, peers of Chorus.
- Their descriptor scalars match the retired global descriptors value-for-value.
- Return level is **not** a descriptor scalar on either entry.
- `global_reverb_delay.rs` is deleted and its DSP preserved unchanged.
- A/B tests prove reverb and delay sound identical to before.
- `cargo build`, `make lint`, `make fmt-check`, and `make test` pass.

## Risks & Mitigations

- **Reverb or delay sounds different after the move** → this is the primary risk. Move the DSP verbatim; A/B test sample-exactly. Resist "improving" the algorithm while relocating it.
- **Copying descriptor ranges by eye** → read them out of `global_parameters.rs` and transcribe exactly. A changed fine step is a silent behavior change on the control surface.
- **Adding a role flag "just in case"** → explicitly rejected. It would reintroduce the two-model split FR-009 exists to remove.
- **Scope creep into WP04** → the port and `GlobalParameters` are not yours. Stop at registration.

## Reviewer Guidance

- **A/B fidelity first.** If reverb or delay changed sound, nothing else about this WP matters.
- Confirm no `role`, `is_send_suitable`, or equivalent field appeared on registry entries.
- Confirm return level did not sneak in as a descriptor scalar.
- Confirm the new preparers allocate only in `prepare` — check for `Vec::push`, `Box::new`, or any allocation inside `process`.
- Confirm `src/mixer/` is untouched; that surface belongs to WP04.
