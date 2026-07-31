---
work_package_id: WP07
title: RETURN-clear held-note continuity twin
dependencies: []
requirement_refs:
- FR-009
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
base_branch: kitty/mission-demo-journey-fidelity-and-hygiene-01KYWVYG
base_commit: 213052d6ee27b913f4250423c90ebb3f20a178e4
created_at: '2026-07-31T20:52:57.499738+00:00'
subtasks:
- T027
- T028
- T029
history:
- timestamp: '2026-07-31T20:21:28Z'
  actor: planner
  action: created from IC-04 (RISK-2 accepted follow-up)
agent_profile: implementer-ivan
authoritative_surface: tests/
create_intent: []
execution_mode: code_change
mission_id: 01KYWVYGQMTRFY314AP78KZJPY
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
model: ''
owned_files:
- tests/topology_change_lifecycle.rs
priority: P2
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP07 – RETURN-clear held-note continuity twin

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Add the dedicated sample-level twin test proving held notes survive clearing
an occupied bus return — the return-side sibling of the existing slot-clear
byte-exact continuity proof (`tests/topology_change_lifecycle.rs:854,1018`).
The parent review accepted this gap as RISK-2 with a cheap twin recommended;
the crest-spec now declares it as attached validation
`service.return_clear_held_note_continuity` with the exact selector
`return_clear_held_note_continuity`.

## Context

- The RT contract (crest-spec `aggregate.RealTime.PreparedGraph`):
  "activation of a replacement whose delta only clears a Patch effect slot or
  bus return occupancy preserves every sounding voice — notes started before
  activation continue audibly across the swap with their envelope and channel
  state." The slot half is proven byte-exactly; the return half shares
  `carry_live_returns_from` but is only inferred today.
- Twin-run technique (already used by the slot proof): render the same
  planned MIDI through two runs — one performs the return clear at a block
  boundary, an untouched twin does not — and compare the dry/voice output
  sample-exactly across the activation boundary (the cleared return's wet
  tail differs by design; the sounding voices must not).

## Subtasks

### T027 — RETURN-clear twin fixture (held notes through occupied return)

**Steps**:
1. Study the slot-clear proof at `topology_change_lifecycle.rs:854` and
   `:1018` — reuse its fixture pattern: production services, an occupied
   return receiving a raised send from a sounding Patch, held notes sounding
   across the boundary.
2. Build the clearing run: a validated structural change whose delta ONLY
   clears that return's occupancy, activated at a block boundary while notes
   are held; and the twin run: identical plan minus the clear.

**Validation**: both runs render deterministically; the clear is the sole
delta.

### T028 — Sample-exact continuity assertion vs untouched twin run

**Steps**:
1. Compare the two runs' rendered output on the surfaces the contract
   protects: the held voices' contribution (dry/track path) must be
   byte-exact across the activation boundary; assert the cleared return
   contributes silence afterward (declared semantics) rather than a torn or
   clicking tail.
2. Mirror the assertion granularity of the slot proof (per-sample equality on
   the protected path, not RMS-style approximations).

**Validation**: assertion is byte-exact; a voice interruption at the boundary
fails it.

### T029 — Wire declared selector return_clear_held_note_continuity

**Steps**:
1. Name the test function so
   `cargo test return_clear_held_note_continuity`
   runs exactly this proof (the crest-spec attached validation's command).
2. Sanity-check non-vacuity: temporarily invert the comparison locally to
   confirm the test can fail, then restore (note the check in the WP notes;
   do not commit the inversion).

**Validation**: the declared selector matches exactly one test; suite green.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch
is also the final merge target. Execution worktrees are allocated per
computed lane from `lanes.json` during `/spec-kitty.implement`.

## Test Strategy

This WP IS a test. Gate: `cargo test return_clear_held_note_continuity` and
`cargo test --test topology_change_lifecycle` green; existing lifecycle
proofs untouched.

## Definition of Done

- Twin test exists, byte-exact, selector-named per the declared attached
  validation, non-vacuity spot-checked.
- No existing lifecycle test weakened or reordered.

## Reviewer Guidance

- Verify the delta really is clear-only (an install/change delta may cut
  notes by accepted product behavior — that would test the wrong contract).
- Check byte-exactness on the protected path and that the wet-tail
  difference is excluded deliberately, not accidentally.
- Confirm the selector uniquely matches (no accidental multi-test filter).
