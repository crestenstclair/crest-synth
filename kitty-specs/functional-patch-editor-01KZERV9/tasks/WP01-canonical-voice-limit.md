---
work_package_id: WP01
title: Canonical voice limit and its real-time enforcement
dependencies: []
requirement_refs:
- FR-009
- NFR-001
planning_base_branch: feat/functional-patch-editor
merge_target_branch: feat/functional-patch-editor
branch_strategy: Planning artifacts live on feat/functional-patch-editor; completed work merges into feat/functional-patch-editor. Execution worktrees are allocated per computed lane.
subtasks:
- T001
- T002
- T003
- T004
- T005
- T006
history:
- timestamp: '2026-08-08T22:59:56Z'
  actor: planner
  action: created
  note: Initial work package definition
agent_profile: implementer-ivan
authoritative_surface: src/synth/
create_intent:
- src/synth/voice_limit.rs
execution_mode: code_change
mission_slug: functional-patch-editor-01KZERV9
owned_files:
- src/synth/**
- src/real_time/**
priority: P1
role: implementer
status: planned
tags: []
tracker_refs: []
---

# WP01 - Canonical voice limit and its real-time enforcement

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this prompt**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

This gives you the identity, governance scope, boundaries, and initialization
context for this work. Do not begin implementation until the profile is loaded.

## Objective

Create the per-Patch voice limit and enforce it inside the audio callback. This
is the one carry-forward structure from Phase 4 with **no state, no descriptor,
and no path anywhere** — the PATCH Utility row for it could only ever be marked
unavailable. This package supplies the canonical value, carries it across the
real-time boundary, and enforces it at note-on by refusal.

It is also the first production producer of the `Stepped` control kind, which the
component vocabulary already declares and which nothing has ever driven.

## Context

**Mission**: functional-patch-editor-01KZERV9
**Priority**: P1
**Dependencies**: none — this is the mission's critical path and starts first.

The crest-spec declares `valueObject.Synth.VoiceLimit` with its bounds,
classification, seeding rule, and enforcement policy; read it before writing
code (`spec-kitty crest-spec context`, or the source at
`.kittify/crest-spec/contexts/synth.yaml`). `DESIGN.md` records the durable
decision. Every later package in this mission depends on this value existing.

## Branch Strategy

- **Planning base branch**: `feat/functional-patch-editor`
- **Final merge target**: `feat/functional-patch-editor`
- Execution worktrees are allocated per computed lane (see `lanes.json`).
- Do not create ad-hoc branches outside the lane workflow.

## Subtasks

### T001: Declare the `VoiceLimit` value object and its surface descriptor

**Purpose**: One canonical bounded value with a descriptor that is the shared
pre-dispatch oracle for bounds and step size — the same contract `VoiceEnvelope`
and `GlobalParameters` already hold.

**Steps**:
1. Create `src/synth/voice_limit.rs` with a newtype over `u16`.
2. Bound it to `1..=64` inclusive. Reject out-of-range construction with a typed
   error — never clamp, wrap, default, or substitute. The upper bound is the
   smallest declared engine polyphony ceiling in the installed registry
   (`HIDEF_POLYPHONY_CEILING` is 64); write the constant so its provenance is
   visible, not as a bare literal.
3. Declare the surface descriptor: one stable id, presentation label
   (`Voice Limit`), unitless presentation, inclusive bounds, fine step 1, coarse
   step 8, classification `Stepped`.
4. Export it from `src/synth/mod.rs`.

**Files**:
- `src/synth/voice_limit.rs` (new, ~140 lines with tests)
- `src/synth/mod.rs` (modified, ~2 lines)

**Validation**:
- [ ] Construction at 0 and at 65 returns a typed error, not a clamped value
- [ ] Construction at 1 and at 64 succeeds
- [ ] The descriptor enumerates the field exactly once
- [ ] The descriptor's classification is `Stepped`, and a test asserts it — this
      is the claim that the `Stepped` kind now has a production producer

**Edge Cases**:
- The bound's relationship to the engine ceiling must be explicit in code. If a
  lower-ceiling engine is ever registered, this bound is wrong; make that
  discoverable rather than latent.

---

### T002: Add `voiceLimit` to the Patch aggregate and seed it from the engine ceiling

**Purpose**: The limit is Patch-owned and follows the Patch. No Patch starts
unlimited-by-omission, and none starts with a limit its engine could not honour.

**Steps**:
1. Add `voice_limit: VoiceLimit` to the `Patch` aggregate in `src/synth/`.
2. At installation, seed it from the active engine's declared polyphony ceiling,
   clamped to the `VoiceLimit` bound. A SoundFont Patch seeds from its engine's
   ceiling; a Braids Patch seeds from its own fixed-per-Patch capacity.
3. Keep the limit through an engine replacement: a structural swap replaces the
   `InstrumentConfig`, not the Patch's limit. If the new engine's ceiling is
   lower than the current limit, clamp *at that point* and make the clamp
   observable — do not silently leave a limit the new engine cannot honour.
4. Add an accessor and a reducer-facing setter that validates against the bound.

**Files**:
- `src/synth/patch.rs` or equivalent (modified, ~40 lines)

**Validation**:
- [ ] Every fixture Patch has a real limit after installation; none is defaulted
      to the type's maximum as a stand-in for "unset"
- [ ] A SoundFont Patch and a Braids Patch seed from their own ceilings
- [ ] An engine swap to a lower-ceiling engine clamps rather than leaving an
      unhonourable limit
- [ ] Identity, config, envelope, effects, and routing are untouched by a limit
      change

---

### T003: Carry the limit on `RtPatchParameters` in the parameter snapshot

**Purpose**: The limit reaches the callback on the existing latest-scalar
transport, adding no field that allocates, borrows, or carries a destructor.

**Steps**:
1. Add the limit to `RtPatchParameters` in
   `src/real_time/parameter_snapshot.rs` as a plain bounded integer beside the
   envelope.
2. Keep the entry `Copy` and fixed-size. Verify the leaf descriptor enumerates
   the new field — the crest-spec requires the typed leaf descriptor to match the
   `StateTree` parameters projection exactly, so an unenumerated leaf fails.
3. Update the snapshot builder to populate it from canonical state.

**Files**:
- `src/real_time/parameter_snapshot.rs` (modified, ~25 lines)
- the snapshot-building site in `src/control/` is **not** yours — if it needs a
  change, note it for WP02 rather than editing outside your ownership map

**Validation**:
- [ ] `RtPatchParameters` remains `Copy` and destructor-free
- [ ] The leaf descriptor test still passes with the new field enumerated
- [ ] Publish cost is measured, not assumed — if a bounded-size assertion exists,
      update its expected size deliberately rather than loosening it

---

### T004: Refuse note-on beyond the limit in the audio renderer

**Purpose**: Enforcement. This is the load-bearing subtask and it sits inside the
hard real-time callback.

**Steps**:
1. In `src/real_time/audio_renderer.rs`, at the `MidiMessageKind::NoteOn` arm
   (the branch where `data2() > 0`), compare the Patch's already-tracked active
   note count against the limit carried on that Patch's `RtPatchParameters`.
2. If the count is at or above the limit, **do not start the note**. Return
   without dispatching to the prepared instrument and without setting the note
   bit.
3. Do not steal. Do not truncate a latched voice. Do not end a voice out of band
   — ending a voice inside the callback is exactly the destruction the callback
   contract forbids.
4. Never refuse a note-off, an all-notes-off, or any non-note message. A refused
   note-off would strand a latched voice.
5. The comparison must be a fixed integer test against state the active-note
   observer already maintains. No new scan proportional to polyphony, no
   allocation, no branch on capability.

**Files**:
- `src/real_time/audio_renderer.rs` (modified, ~30 lines plus tests)

**Validation**:
- [ ] A note-on at the limit does not sound and does not set a note bit
- [ ] Notes already sounding are unaffected by a limit lowered beneath them; the
      limit applies at the next note-on
- [ ] Note-off, all-notes-off, control change, program change, channel pressure,
      and pitch bend are never refused
- [ ] A test asserts the enforcement path performs no allocation

**Edge Cases**:
- A limit lowered below the currently sounding count: existing notes continue,
  new ones are refused. Assert both halves.
- An unknown `PatchId`: the existing routing-failure path still applies and is
  not shadowed by the limit check.

---

### T005: Count refusals in `AudioObservationSnapshot`

**Purpose**: Make the limit falsifiable. A limit defeated in the callback shows
zero refusals under a fixture that must exceed it — a failing predicate rather
than an unnoticed absence of silence.

**Steps**:
1. Add `voice_limit_refusals: u64` to `AudioObservationSnapshot`.
2. Increment exactly once per refused note-on. Saturate on overflow, like every
   other callback counter.
3. Wire it through the same bounded observation path `routing_failures` uses.

**Files**:
- `src/real_time/audio_observation*.rs` (modified, ~20 lines)

**Validation**:
- [ ] The counter increments once per refusal, not once per block
- [ ] It saturates rather than wrapping
- [ ] The snapshot stays `Copy`, fixed-size, and destructor-free

---

### T006: Prove the callback contract holds and that a defeated limit fails

**Purpose**: Non-vacuous proof, including falsification. WP05 owns the
mission-level acceptance target; this subtask owns the unit-level proof that
lives beside the code.

**Steps**:
1. Add tests in the modules you own covering every validation bullet above.
2. Write the falsification explicitly: with the refusal removed, a fixture that
   drives more simultaneous notes than the limit must produce a *failing*
   assertion — zero refusals under a fixture that must exceed the limit is the
   signal.
3. Assert the callback contract under enforcement: zero allocations, zero
   destructions, no locking, blocking, I/O, or logging.

**Files**:
- test modules within `src/synth/` and `src/real_time/` (modified/new, ~200 lines)

**Validation**:
- [ ] Every bullet in T001–T005 has a test
- [ ] Removing the refusal branch makes a named test fail — verify this by
      actually removing it, running the test, and restoring it
- [ ] `cargo test --all-targets` is green
- [ ] `cargo clippy --all-targets -- -D warnings` is clean

## Definition of Done

- [ ] All six subtasks complete
- [ ] `VoiceLimit` is bounded, `Stepped`-classified, and descriptor-backed
- [ ] Every Patch carries a real seeded limit; none is a placeholder
- [ ] The callback refuses note-ons beyond the limit, refuses nothing else, and
      allocates and destroys nothing
- [ ] Refusals are counted and the counter saturates
- [ ] The falsification is demonstrated, not asserted
- [ ] No regression in the existing real-time contract validations

## Risks & Mitigations

| Risk | Mitigation |
| --- | --- |
| Enforcement drifts into a polyphony-proportional scan | Compare against the count the active-note observer already maintains; assert no allocation |
| A refused note-off strands a latched voice | Explicitly exempt every non-note-on message and test it |
| Voice destruction inside the callback | Refuse the start; never end a latched voice. The callback contract forbids destruction |
| The limit silently never bites | The refusal counter exists for exactly this; a zero count under an exceeding fixture is a failure |

## Reviewer Guidance

**Verify**:
- The bound's provenance is visible in code, not a bare `64`
- Seeding is from the engine's own ceiling, per capability, not a shared constant
- The enforcement branch is a fixed integer comparison
- The falsification was actually run, not merely described

**Red Flags**:
- A `clamp()` where the crest-spec says reject
- Any voice being ended, stolen, or truncated
- A default `VoiceLimit::MAX` standing in for "unset"
- A test that asserts the limit exists without driving audio through it
