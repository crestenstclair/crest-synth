---
work_package_id: WP03
title: Stamp slot identity from position
dependencies: []
requirement_refs:
- FR-006
- FR-007
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts were generated on feat/expandable-effects-and-bus-topology, which is also the final merge target. During /spec-kitty.implement this WP branches from the lane the runtime computes from lanes.json; completed changes merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T012
- T013
- T014
- T015
history:
- timestamp: '2026-08-02T00:10:35Z'
  actor: planner
  action: created from IC-04 (close RISK-1 by construction)
agent_profile: implementer-ivan
authoritative_surface: src/synth/
create_intent: []
execution_mode: code_change
mission_id: 01KYZTQ118MXZGD4MBCR99A978
mission_slug: falsifiable-journey-proof-01KYZTQ1
model: ''
owned_files:
- src/synth/patch.rs
- src/synth/effect_capability.rs
priority: P2
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP03 – Stamp slot identity from position

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Make an occupant's slot identity **derived from the position it occupies** rather than accepted
from the caller, so an effect carrying another position's identity is inexpressible rather than
merely refused.

## Context: RISK-1

`Patch::set_slot_occupancy` (`src/synth/patch.rs:194-213`) enforces that occupied slot ids are
**unique**, but never that an occupant's id matches the position it is being installed at:

```rust
let duplicate = self.effects.iter().enumerate()
    .filter(|(position, _)| *position != index.index())
    .filter_map(|(_, slot)| slot.as_ref())
    .any(|other| other.slot_id() == config.slot_id());
if duplicate { return Err(EffectSlotOccupancyError::DuplicateSlotId(config.slot_id())); }
self.effects[index.index()] = occupant;
```

A valid-but-mismatched identity therefore passes and silently relocates an effect. No current
path produces one — this is latent, which is why it is P2 and not P1.

**This is not a new design.** `EffectSlotIndex::instance_identity()`
(`src/synth/effect_slot_id.rs:98-101`) already documents the intended contract verbatim:

> "the stable instance identity an occupancy change derives for this position: positions map
> one-to-one onto non-zero slot ids (position + 1), exactly as bus returns derive theirs, so the
> identity is deterministic and unique per position **by construction**."

Production already relies on it (`app_state.rs:1202,1281` construct via
`default_config(slot.instance_identity())`). This WP makes the aggregate enforce the contract its
own code already documents. Crest-spec commit `ad9960b` declares it on
`valueObject.Synth.EffectSlotId` and `aggregate.Synth.Patch`.

**Why stamping and not validation** (recorded decision, research R-001): validating and returning
a `MismatchedSlotId` error closes the *path*; stamping closes the *class* — the wrong value
becomes unrepresentable at the only gate all occupancy changes pass through. `SemanticAction`
has no move or exchange variant (`src/control/semantic_action.rs:54-70`), so occupancy is set per
position and instances are never relocated; deriving identity from position loses nothing.

## Constraints that bind this WP

- **Serialized vocabulary unchanged (spec C-003)**: `slotId` appears in 10+ serialized paths
  (`patches[].effects[].slotId`, `returns[].slotId`, `patchPage.effects[].slotId`, and others),
  frozen by the predecessor mission's occurrence map. This changes **how a value is produced**,
  never what it is called or what it equals for a correctly-constructed patch.
- **Locality (DIRECTIVE_024)**: do not remove `slot_id` from `PostEffectConfig`. That was the
  rejected third option — it reaches into real-time snapshot construction and every serialized
  path.
- **Real-time discipline**: `src/real_time/` is not yours and must not need changing. If it does,
  stop and raise it.

## Subtasks

### T012 — Add the position-stamping constructor to PostEffectConfig

**Purpose**: give `Patch` a way to produce an occupant carrying a specified identity without
trusting the one it was handed.

**Steps**:

1. `PostEffectConfig` is at `src/synth/effect_capability.rs:171`. Add a method that returns the
   config with its slot id replaced by a supplied `EffectSlotId`, leaving every other field
   untouched.
2. Take `self` by value and return `Self` if the type's existing style is value-oriented; match
   the surrounding conventions rather than introducing a new one.
3. Keep it minimal — this is a field replacement, not a validation point. The aggregate decides
   what identity is correct; this method only applies it.
4. Scope its visibility no wider than `Patch` needs. A `pub` method here invites callers to stamp
   arbitrary identities, which is the door you are closing.

**Files**: `src/synth/effect_capability.rs`

**Validation**:
- Every other field survives unchanged.
- Visibility is the narrowest that compiles.

### T013 — `set_slot_occupancy` stamps the position's identity

**Purpose**: make the mismatch inexpressible at the single chokepoint.

**Steps**:

1. In `Patch::set_slot_occupancy` (`src/synth/patch.rs:194`), before storing, replace the
   occupant's identity with `index.instance_identity()`.
2. Reconsider the duplicate check. Once every occupant's identity is derived from its distinct
   position, two occupied positions cannot collide — uniqueness follows from derivation. Decide
   deliberately:
   - If you remove the check, `EffectSlotOccupancyError::DuplicateSlotId` may become
     unconstructible. Removing a now-impossible error variant is correct and is the strongest
     form of this fix — but check every match site first; an exhaustive match elsewhere will fail
     to compile, which is the loud failure you want.
   - If you keep it as a defensive assertion, comment **why** it can no longer fire, so a future
     reader does not mistake dead code for a live guard.
   Either is acceptable; an unexplained leftover is not.
3. `Patch::with_effect_slot` already routes through `set_slot_occupancy` (established by the
   predecessor mission) — confirm that is still true, so the stamp covers every entry point.
4. Do not change the method's signature or error type shape beyond what step 2 decides. Callers
   across the tree depend on it.

**Files**: `src/synth/patch.rs`

**Validation**:
- Installing an occupant carrying another position's identity stores it with the correct one.
- Every path that installs an occupant goes through this method — grep for direct writes to the
  effects array.
- `cargo test --all-targets` passes.

### T014 — Composition-root round trip still resolves positions

**Purpose**: confirm the change is invisible to the recorded-patch path.

**Steps**:

1. `src/shell/standalone_application.rs:1516` recovers a position by matching
   `instance_identity() == config.slot_id()`. With stamping, a correctly-recorded patch resolves
   exactly as before.
2. Add an inline test in `src/synth/patch.rs`'s `mod tests` (`:255`) covering a round trip: build a
   patch with occupants at several positions including a gap, confirm each occupied position
   holds the identity derived from that position, and confirm gaps survive (clearing one position
   never compacts the others — an existing aggregate invariant).
3. Do **not** modify `src/shell/standalone_application.rs`. It is not in your `owned_files`. If
   it needs a change, that is a finding to raise, not an edit to make — the stamping design was
   chosen precisely because it should not require one.

**Files**: `src/synth/patch.rs`

**Validation**:
- A gapped chain round-trips with gaps intact and identities position-derived.
- `src/shell/` and `src/real_time/` are untouched — `git diff --stat` proves it.

### T015 — Falsification: defeat the stamp

**Purpose**: observe the guard failing. Spec C-005 — a guard whose failure has not been observed
is not proof.

**Steps**:

1. Back up `src/synth/patch.rs`.
2. Mutate: remove the stamp so `set_slot_occupancy` stores the caller's identity as before.
3. Run `cargo test --test effects_and_buses --all-targets` — or more precisely
   `cargo test --all-targets` — with **no pipe to `head`/`tail`**, and observe your T014
   assertions fail.
4. If everything still passes, T014's test does not actually constrain the identity. Fix the test
   rather than recording a pass.
5. Restore, confirm `git status` clean, re-run green.
6. Write the record to
   `kitty-specs/falsifiable-journey-proof-01KYZTQ1/evidence/falsification/guard-slot-identity.md`:
   the mutation, the command, the observed non-zero exit and failure message, the restoration, and
   the observed pass.

**Files**: `src/synth/patch.rs`; evidence file is a recorded out-of-map edit — rationale:
"mission falsification record; kitty-specs paths are non-declarable by rule".

**Validation**:
- Non-zero exit under mutation, zero after restoration, tree clean.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch is also the final
merge target. This WP has **no dependencies** and can run in parallel with WP01 and WP04 — enter
the lane the runtime computes from `lanes.json`.

Your tests live inline in `src/synth/patch.rs`, which you own. You must **not** touch
`tests/effects_and_buses.rs` — WP02 owns it, and a cross-lane edit there would be invisible to
both reviewers. That exact failure mode broke the predecessor mission's merged build.

## Test Strategy

Deterministic, inline in `src/synth/patch.rs`'s existing `mod tests`. The falsification (T015) is
the binding proof.

## Definition of Done

- An occupant's identity is stamped from its position inside `set_slot_occupancy`; a
  valid-but-mismatched identity cannot survive the call.
- The duplicate check is either removed (with every match site updated) or retained with a comment
  explaining why it can no longer fire.
- An inline round-trip test covers a gapped chain with position-derived identities.
- A falsification record exists showing an observed non-zero exit under mutation.
- `src/shell/`, `src/real_time/`, and `tests/` are untouched by this WP.
- `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
  all exit 0, none piped.

## Reviewer Guidance

- **Re-run the falsification.** Apply the mutation, confirm the test fails, restore.
- Confirm the serialized `slotId` values are unchanged for a correctly-constructed patch — this
  changes production, not vocabulary (spec C-003).
- Confirm no leftover dead duplicate check without an explanation.
- Confirm `git diff --stat` shows only `src/synth/patch.rs` and
  `src/synth/effect_capability.rs`. Anything in `src/shell/`, `src/real_time/`, or `tests/` is
  out of bounds for this WP.
- Confirm the stamping method's visibility is not wider than `Patch` requires.
