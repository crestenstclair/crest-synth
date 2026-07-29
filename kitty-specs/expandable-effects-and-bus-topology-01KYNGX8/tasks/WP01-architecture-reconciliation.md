---
work_package_id: WP01
title: Architecture and design reconciliation
dependencies: []
requirement_refs:
- C-009
- C-011
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T001
- T002
- T003
- T004
- T005
- T006
- T056
history:
- timestamp: '2026-07-29T02:11:28Z'
  actor: planner
  action: created
- timestamp: '2026-07-29T02:11:28Z'
  actor: planner
  action: added T056 to close pre-existing DESIGN.md reconciliation drift
agent_profile: architect-alphonso
authoritative_surface: .kittify/architecture/
create_intent: []
execution_mode: planning_artifact
mission_id: 01KYNGX8QA8V49BX2WQ1Q6G2BP
mission_slug: expandable-effects-and-bus-topology-01KYNGX8
model: ''
owned_files:
- .kittify/architecture/**
- DESIGN.md
priority: P1
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP01 – Architecture and design reconciliation

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load architect-alphonso
```

This loads your identity, boundaries, and governance context. Do not skip this step.
Once loaded, continue with the Objective below.

## Objective

The architecture spec currently declares this entire mission a **non-goal**. Three
declarations in `project.yaml` forbid exactly what the mission builds, and four
statements in `DESIGN.md` are directly superseded. This work package narrows those
declarations and adds the capability, goal, requirements, and proof entries the
mission needs, so that every later work package builds against a spec that agrees
with it.

This is not bookkeeping and it cannot be deferred. The architecture spec's own rule
is *"Never silently plan around a conflict with the architecture spec… Planning is
incomplete while the spec and the mission artifacts disagree."* `C-009` restates it
as a mission constraint. Nothing else in this mission may merge ahead of it.

## Context

- **Mission**: expandable-effects-and-bus-topology-01KYNGX8
- **Priority**: P1
- **Dependencies**: none — this is the root of the dependency graph
- **Related requirements**: C-009 (spec reconciliation is not deferrable), C-011 (roster stays deferred)
- **Read first**: `plan.md` § Architecture Reconciliation — it tabulates every declaration and its required change

## Branch Strategy

- **Planning base branch**: `feat/expandable-effects-and-bus-topology`
- **Merge target branch**: `feat/expandable-effects-and-bus-topology`
- **Execution**: worktree-per-lane. `finalize-tasks` computes lanes and writes `lanes.json`; each lane gets exactly one worktree and one branch.
- Do not create ad-hoc branches by hand; use the workspace the runtime resolves for this WP's lane.

## Subtasks

### T001 – Narrow `nonGoals.additional_effects`

- **Purpose**: The current text forbids "another insert type, more than one slot per Patch, bypass, selection, reordering, effect chains" — which is most of the mission.

- **Steps**:
  1. Open `.kittify/architecture/project.yaml`, lines 17-19.
  2. Rewrite so the *roster of additional effect types* stays excluded while slots, selection, and ordering become in-scope up to the declared ceiling. The distinction that matters: this mission adds **capacity and generality**, not new effect types.
  3. Keep the exclusion of bypass if nothing in the mission delivers it — check `spec.md` FR list before removing any clause. FR-002 delivers occupancy selection including empty, which is not the same as bypass.

- **Validation**: The narrowed text must still exclude phaser, flanger, echo, tape delay, distortion, bitcrush, granular, convolution reverb, compressor, and EQ8. If it does not, C-011 has been violated.

### T002 – Remove the Phase 3 clause from `nonGoals.later_roadmap_phases`

- **Purpose**: Lines 32-35 exclude "Phase 3 expansion beyond at most three effect slots per Patch and eight bus returns". This mission *is* that expansion.

- **Steps**:
  1. Remove only the Phase 3 clause.
  2. Leave Phases 4-9 excluded verbatim: the component library, complete Patch editor, component-polished Mixer, asset workflows, Modal/MultiSelect, modulation, arbitrary graph editing, persistence, and final visual completion.

- **Validation**: Re-read the remaining text and confirm each surviving exclusion still matches a real roadmap phase.

### T003 – Replace the `meta.avoid` enumerated-effect rules

- **Purpose**: Lines 47-52 forbid "effect selection, bypass, multiple slots, reordering, or arbitrary routing" and "arbitrary buses". The mission needs the sharper rule underneath these.

- **Steps**:
  1. Replace the effect clauses with the rule this mission actually enforces: **avoid name-enumerated effect and routing identities**. Effects, slots, sends, and returns are addressed by index into descriptor-driven arrays.
  2. Preserve the bus constraint accurately: buses remain **bounded and validated**, never arbitrary. Eight returns is a ceiling, not an invitation to arbitrary graph editing.
  3. Leave the sequencer, transport, timeline, pattern, clip, song-editing, persistence, modulation-matrix, and plugin-hosting exclusions untouched.

- **Validation**: The rewritten `avoid` list must forbid `MixerTrackParameter::ReverbSend`-style naming, and must not forbid the mission's own generic arrays.

### T004 – Add capability, goal, requirements, evidence, validation, and witness

- **Purpose**: The mission needs a traceable home in the intent model and a declared proof obligation.

- **Steps**:
  1. `capabilities.yaml` — add `expandable_effects_and_bus_topology` with acceptance scenarios drawn from `spec.md` User Stories 1-3. Link it bidirectionally to the new goal.
  2. `goals.yaml` — add `expand_effects_and_buses`.
  3. `project.yaml` `completion.requiredGoals` — append the new goal.
  4. `project.yaml` `completion.projectChecks` — append `expandable_effects_and_bus_topology`.
  5. `requirements.yaml` — add entries for the mission's functional and non-functional constraints, linked to the goal and capability.
  6. `proof/evidence.yaml` — add `expandable_effects_and_bus_topology_contract`.
  7. `proof/validations.yaml` — add the matching validation with its proof role.
  8. `proof/witnesses.yaml` — add a witness with a **positive command and a controlled negative command**, structured observations, and predicates. A witness without a negative command proves nothing.
  9. `project.yaml` `mission` (lines 2-10) — restate the executable slice to include ordered slots and bounded bus returns.

- **Validation**: `spec-kitty context architecture` reloads with no unresolved references. Every `contributesTo` edge you add points at a capability that exists.

### T005 – Declare the no-name-enumeration invariant

- **Purpose**: The expansion to three slots and eight returns was declared in DESIGN.md *before* the closed code was written, and the closed code shipped anyway. A prose constraint is a demonstrably insufficient control here.

- **Steps**:
  1. Add to `proof/invariants.yaml`:
     > **No name-enumerated effect or routing identity.** No type in Synth, Mixer, RealTime, or Control may enumerate a variant, field, or descriptor entry named after a specific effect or bus. Effects, slots, sends, and returns are addressed by index into descriptor-driven arrays. Adding a registry entry must require no change to any of these types.
  2. Reference the validation that WP09 will implement.
  3. Note the single exception: `MasterGainDb` is genuinely global, not per-effect.

- **Validation**: The invariant text names the four contexts it binds and states the exception. WP09 implements the executable check; this subtask only declares it.

### T006 – Restate the superseded DESIGN.md decisions `[P]`

- **Purpose**: `CLAUDE.md` puts durable decisions in DESIGN.md. Four statements there now contradict the mission.

- **Steps**:
  1. **Line 689** — "Chorus is the first concrete Patch effect": reverb and delay join the same registry as peers.
  2. **Line 691** — "statically bounded to zero or one post effect per Patch… no selector, bypass, reorder, placeholder, or dynamic graph editing yet": restate at three ordered slots with occupancy selection.
  3. **Line 418** — "current production bound is zero or one effect per Patch": restate at the new bound. Note the sentence already anticipates this ("Phase 3 may expand this only up to the product maximum").
  4. **Line 309** — "at most eight values for the current zero-or-one effect slot" and "MIXER contains only the sixteen track-owned controls and distinct globals": restate for three slots, and for sends addressed by bus with the set of distinct globals reduced to master gain.
  5. Update the signal-flow diagram at lines 396-415 so the two named aux paths become eight indexed ones. Meter position, gate order, and send position do not move.

- **Validation**: Lines 690 (three slots / eight returns product maximum) and 692 (sixteen-track ownership) must remain **true and unchanged** — they already authorize the target state.

### T056 – Close pre-existing DESIGN.md reconciliation drift

- **Purpose**: The architecture spec was authored in one commit and never revised, while `DESIGN.md` moved afterwards. An audit found three durable design decisions with no executable declaration. Keeping the spec reconciled to `DESIGN.md` is the point of this system, so these are closed here rather than logged for later.

- **Steps**:
  1. **Product maxima as a declared bound.** `DESIGN.md:204` and `:690` state three ordered post-FX slots per Patch and eight bus returns as *product-level maxima*. In the spec these numbers appear only at `project.yaml:33`, inside a `nonGoals.later_roadmap_phases` clause — expressed as "not yet," never as a ceiling. Declare them as real bounds in `contexts/realtime.yaml` alongside the existing `MAX_PATCHES` invariant, so the ceiling survives T002 removing the non-goal clause. Without this, deleting the non-goal deletes the only mention of the numbers.
  2. **Master stage.** `DESIGN.md:413` ends the signal flow with `master gain / safety limiter`. `mixer.yaml:113` declares `masterGainDb`, but no limiter appears anywhere in the spec. Model the master stage so the declared render path matches the designed one.
  3. **Start reservation.** `DESIGN.md:694` states that holding Start previews a focused sample and that **Start remains reserved elsewhere**. The spec has no coverage; `nonGoals` excludes sample browsers but never captures the reservation. This constrains the input vocabulary *now*, in Phases 1-6 — not in Phase 7. Declare it as an invariant in `contexts/control.yaml` or `contexts/shell.yaml` so no work package binds Start.
  4. Re-run the audit after editing: every durable `DESIGN.md` decision should have an executable declaration, or an explicit non-goal saying why not.

- **Validation**: All three have declarations; the maxima survive T002; a grep for "Start" and for the limiter now returns spec coverage.

## Test Strategy

This package produces declarations, not code, so its proof is structural:

- `spec-kitty context architecture` reloads cleanly with no dangling references.
- No surviving declaration contradicts any FR in `spec.md`.
- The deferred roster (C-011) is still excluded after every edit.
- DESIGN.md lines 690 and 692 are unchanged.

## Definition of Done

- All three `project.yaml` non-goal / avoid declarations narrowed, not deleted wholesale.
- New capability, goal, requirement, evidence, validation, and witness entries present and cross-linked.
- The no-name-enumeration invariant declared with its exception.
- Four DESIGN.md statements restated; the signal-flow diagram updated.
- `spec-kitty context architecture` reloads without error.
- The twelve-effect roster remains out of scope everywhere.

## Risks & Mitigations

- **Narrowing too far pulls the roster into scope** → after each edit, re-read the surviving text against C-011 and confirm all twelve effects are still excluded.
- **Deleting a non-goal instead of narrowing it** → these declarations also protect Phases 4-9. Narrow surgically; never remove a whole entry.
- **Editing mechanically** → `occurrence_map.yaml` marks `.kittify/architecture/**` and `DESIGN.md` as `manual_review` precisely so no bulk replace touches them. Every edit here is by hand.

## Reviewer Guidance

- **Check C-011 first.** The fastest way this WP goes wrong is an over-broad narrowing that silently admits the twelve-effect roster.
- Confirm DESIGN.md:690 and :692 are untouched — they are the authority this reconciliation leans on.
- Confirm the witness has a controlled **negative** command, not only a positive one.
- Confirm the new invariant names its exception; without it, WP09 will false-positive on `MasterGainDb`.
