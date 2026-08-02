---
work_package_id: WP09
title: No-name-enumeration guard
dependencies:
- WP07
requirement_refs:
- FR-006
- FR-009
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T053
- T054
- T055
history:
- timestamp: '2026-07-29T02:11:28Z'
  actor: planner
  action: created
agent_profile: paula-patterns
authoritative_surface: tests/no_name_enumeration_guard.rs
create_intent:
- tests/no_name_enumeration_guard.rs
execution_mode: code_change
mission_id: 01KYNGX8QA8V49BX2WQ1Q6G2BP
mission_slug: expandable-effects-and-bus-topology-01KYNGX8
model: ''
owned_files:
- tests/no_name_enumeration_guard.rs
priority: P2
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP09 – No-name-enumeration guard

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load paula-patterns
```

This loads your identity, boundaries, and governance context. Do not skip this step.
Once loaded, continue with the Objective below.

## Objective

Make the open-closed property mechanically enforceable, so that reintroducing a
name-enumerated effect or routing identity fails the build rather than depending on
a reviewer noticing.

This work package exists because of a specific, documented failure. The expansion to
three effect slots and eight bus returns was declared in `DESIGN.md:690` **before**
the closed code was written — and `MixerTrackParameter::ReverbSend`,
`GlobalEffectsProcessor::process(reverb_input, delay_input, ...)`, and
`GlobalParameter::ReverbRoomSize` shipped anyway. A prose constraint in a design
document is therefore demonstrably insufficient as a control here. The architecture
spec already treats `validations` as executable gates and warns against replacing
measured proof with self-reported text; this applies that principle to the design
property itself.

## Context

- **Mission**: expandable-effects-and-bus-topology-01KYNGX8
- **Priority**: P2
- **Dependencies**: WP07 (the renames must be complete or the check cannot pass)
- **Related requirements**: FR-006 (explicit bus identities), FR-009 (registry effects), SC-008 (adding an entry requires no structural change)
- **Declared by**: WP01/T005, which adds the invariant to `proof/invariants.yaml`
- **Reference**: `research.md` R-05

## Branch Strategy

- **Planning base branch**: `feat/expandable-effects-and-bus-topology`
- **Merge target branch**: `feat/expandable-effects-and-bus-topology`
- **Execution**: worktree-per-lane. `finalize-tasks` computes lanes and writes `lanes.json`; each lane gets exactly one worktree and one branch.
- Do not create ad-hoc branches by hand; use the workspace the runtime resolves for this WP's lane.

## Subtasks

### T053 – Implement the static check

- **Purpose**: A build-time check that scans source for name-enumerated effect and routing identities.

- **Steps**:
  1. Create `tests/no_name_enumeration_guard.rs` as a standard test so it runs under `cargo test` and `make test`.
  2. Scan the four bound contexts: `src/synth/`, `src/mixer/`, `src/real_time/`, `src/control/`.
  3. Fail on identifiers that name a specific effect or bus in a type, variant, field, or descriptor position — for example `reverb_send`, `ReverbSend`, `delay_input`, `ReverbRoomSize`, `DelayReturn`, `reverb_return`.
  4. The property is structural and knowable at build time. Do **not** implement this as a runtime assertion — a render-path assertion would also violate the callback contract.
  5. Report failures with file, line, and the offending identifier, so the message is actionable rather than merely a red build.

- **Validation**: The check runs under `make test` and produces actionable output.

### T054 – Seed it with the retired identifiers and its exception

- **Purpose**: Precision. The check must be narrow enough to enforce without false positives.

- **Steps**:
  1. Seed the forbidden set from the concrete identifiers this mission retires — the four rename families listed in `plan.md` § Bulk Edit Classification and in `occurrence_map.yaml`.
  2. **Allow `MasterGainDb`.** It is genuinely global — a property of the master stage, not of any effect — and is the single documented exception in the invariant WP01 declared. A check that rejects it is wrong.
  3. Scope the check to effect and routing identity only. It does **not** constrain all naming, and it must not fire on genuinely singular concepts or on unrelated domain vocabulary.
  4. Exclude adapter implementation files that legitimately name their own DSP: `reverb_capability.rs`, `reverb_preparer.rs`, `delay_capability.rs`, `delay_preparer.rs`, `chorus_*.rs`. An adapter implementing reverb is *supposed* to say "reverb" — the invariant binds the four domain contexts, not the adapters that register concrete entries.
  5. Exclude `archive/`, `kitty-specs/`, and test fixtures that deliberately reference retired names for migration assertions.

- **Validation**: The check passes on the delivered tree and fires on a deliberately reintroduced violation.

### T055 – Register it as a project check

- **Purpose**: The gate only works if it runs in the acceptance path.

- **Steps**:
  1. Confirm `expandable_effects_and_bus_topology` was added to `completion.projectChecks` by WP01/T004; this check runs under it.
  2. Ensure it executes under `make test` so `spec-kitty accept` and the pre-review gate both exercise it.
  3. Write a **negative proof**: temporarily reintroduce a name-enumerated identifier in a scratch fixture and confirm the check fails. A guard nobody has seen fail is not known to work.
  4. Document in the test file what the check enforces and why, with a pointer to the invariant. The next person to hit a failure needs to understand it is deliberate.

- **Validation**: The check runs in the acceptance path; its failure mode has been observed, not assumed.

## Test Strategy

- The check itself is a test.
- **A negative proof is mandatory** (T055): the guard must be observed failing on a reintroduced violation.
- False-positive check: confirm `MasterGainDb` and the four adapter files pass cleanly.
- The check must pass on the delivered tree — if it does not, an earlier work package left a closed identity behind, which is the guard doing its job.

## Definition of Done

- `tests/no_name_enumeration_guard.rs` exists and runs under `make test`.
- It scans the four bound contexts and reports file, line, and identifier on failure.
- `MasterGainDb` and the concrete-DSP adapter files are excluded and pass.
- The guard has been **observed failing** on a deliberately reintroduced violation.
- It passes on the delivered tree.
- It runs in the acceptance path via the project check WP01 registered.
- `make lint` and `make fmt-check` pass.

## Risks & Mitigations

- **False positives make the check hated and then disabled** → scope narrowly to effect and routing identity, exclude the adapters that legitimately name their DSP, and make failure messages actionable.
- **A guard that can never fail** → T055's negative proof exists precisely to rule this out. Do not skip it.
- **Over-reach into general naming policy** → this check enforces one invariant. It is not a style linter.
- **The check fails on the delivered tree** → do not weaken the check. Report it: an earlier WP left a closed identity behind, and that is the finding.

## Reviewer Guidance

- **Ask to see the guard fail.** A guard whose failure mode has never been observed is not known to work — this is the same class of gap that let the original closed design ship.
- Confirm `MasterGainDb` passes and that the exception is documented in the test file.
- Confirm the four concrete-DSP adapter files are excluded, with the reason stated.
- Confirm the failure message names file, line, and identifier.
- If the check needed weakening to pass, find out which work package left the violation and fix that instead.
