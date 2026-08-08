---
work_package_id: WP02
title: Widened PATCH control surface and reducer
dependencies:
- WP01
requirement_refs:
- FR-002
- FR-006
- FR-007
- FR-008
- FR-012
planning_base_branch: feat/functional-patch-editor
merge_target_branch: feat/functional-patch-editor
branch_strategy: Planning artifacts for this mission were generated on feat/functional-patch-editor. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/functional-patch-editor unless the human explicitly redirects the landing branch.
subtasks:
- T007
- T008
- T009
- T010
- T011
- T012
history:
- timestamp: '2026-08-08T22:59:56Z'
  actor: planner
  action: created
  note: Initial work package definition
agent_profile: implementer-ivan
authoritative_surface: src/control/
create_intent: []
execution_mode: code_change
mission_slug: functional-patch-editor-01KZERV9
owned_files:
- src/control/patch_control_id.rs
- src/control/semantic_focus.rs
- src/control/semantic_resolver.rs
- src/control/interaction_state.rs
- src/control/app_state.rs
- src/control/serialized_state.rs
- src/control/semantic_action.rs
priority: P1
role: implementer
status: planned
tags: []
tracker_refs: []
---

# WP02 - Widened PATCH control surface and reducer

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this prompt**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

This gives you the identity, governance scope, boundaries, and initialization
context for this work. Do not begin implementation until the profile is loaded.

## Objective

Give PATCH the canonical focus identities and reducer arms for the three Utility
controls that had no path at all — master volume, MIDI input, and voice limit —
and declare the Utility resolver as exactly five ordered rows. Then add the
subordinate detail surface identity with its subject, entry rules, exit rules,
and behaviour across a patch switch.

After this package, PATCH Utility carries five real canonical values, and the
detail surface exists as a reducer-owned place to be.

## Context

**Mission**: functional-patch-editor-01KZERV9
**Priority**: P1
**Dependencies**: WP01 — a control cannot address `VoiceLimit` before it exists.

The crest-spec declares `valueObject.Control.PatchControlId` (widened),
`valueObject.Control.PatchDetailSubject` (new),
`valueObject.Control.SurfaceId` (gains `PatchDetail`),
`valueObject.Control.ReturnPath` (widened), and
`aggregate.Control.InteractionState` (gains `detailSubject`). Read those
declarations before writing code — they carry the invariants this package must
satisfy, including the non-nesting rule and the cross-switch behaviour.

## Branch Strategy

- **Planning base branch**: `feat/functional-patch-editor`
- **Final merge target**: `feat/functional-patch-editor`
- Execution worktrees are allocated per computed lane (see `lanes.json`).
- Do not create ad-hoc branches outside the lane workflow.

## Subtasks

### T007: Add `Global`, `MidiInput`, `VoiceLimit` to `PatchControlId`

**Purpose**: Three focus identities for three values PATCH could not previously
address.

**Steps**:
1. In `src/control/patch_control_id.rs`, add three variants:
   - `Global(GlobalParameter)` — reuses the canonical `GlobalParameters` surface
     descriptor, the same one `MixerControlId::Global` uses.
   - `MidiInput` — the focused Patch's own channel.
   - `VoiceLimit` — the focused Patch's own limit.
2. Extend the serialization derivations: `Global(parameter)` derives
   `patch.global.<GlobalParameter.name>`, `MidiInput` serializes as
   `patch.midiInput`, `VoiceLimit` as `patch.voiceLimit`.
3. Do **not** declare a second domain-field enum. `GlobalParameter` already
   exists; reuse it.
4. Update `PatchControlId::resolve` only where the widened union forces an arm —
   the new variants belong to the Utility resolver (T008), not to the PatchMain
   order.

**Files**:
- `src/control/patch_control_id.rs` (modified, ~60 lines)
- `src/control/serialized_state.rs` (modified, ~20 lines)

**Validation**:
- [ ] All three variants serialize to their declared paths
- [ ] None of the three appears in the PatchMain focus order
- [ ] The exhaustive surface-descriptor test still passes with the new variants
      enumerated — an unexercised variant fails, which is the point

---

### T008: Declare the five-row PATCH Utility resolver order

**Purpose**: The Utility panel is exactly five rows in a declared order, bounded
by declaration rather than by what fits.

**Steps**:
1. In `src/control/semantic_resolver.rs`, replace `patch_utility_paths`'s
   two-row `PatchOutputParameter::ALL` mapping with the declared five-row order:
   1. `Global(MasterGainDb)` — master volume
   2. `Output(TrimGain)` — patch volume
   3. `MidiInput`
   4. `Output(OutputTrack)`
   5. `VoiceLimit`
2. Keep `ensure_unique`. Keep the `NoPatchesInstalled` rejection.
3. The order is declared, not derived from a descriptor iteration order — write
   it explicitly so a descriptor reordering cannot silently reshuffle the panel.

**Files**:
- `src/control/semantic_resolver.rs` (modified, ~30 lines)

**Validation**:
- [ ] `patch_utility_paths` returns exactly five paths in the declared order
- [ ] Navigation from the first row upward and the last row downward refuses
      rather than wrapping, matching the existing non-wrapping contract
- [ ] The PatchMain order is unchanged by this subtask

---

### T009: Add reducer arms for master gain from PATCH, MIDI input, and voice limit

**Purpose**: Make all three editable through `AppState::apply`. This is where the
one-way loop is either preserved or quietly broken.

**Steps**:
1. In `src/control/app_state.rs`, add adjustment arms for the three new control
   identities.
2. **Master gain**: the PATCH arm must reach the *same canonical
   `GlobalParameters` value* the MIXER Inspector's arm reaches, through the
   *same descriptor*, with the same bounds and steps. Do not add a PATCH-side
   field. Do not add a second setter. If the existing mixer arm can be reused
   directly, reuse it; if it needs extracting to a shared helper, extract it
   rather than copying it.
3. **MIDI input**: adjust the focused Patch's `channel` within `0..=15`, refusing
   at the boundaries. Identity, config, envelope, effects, routing, and the
   active graph revision must be untouched.
4. **Voice limit**: adjust the focused Patch's limit within the `VoiceLimit`
   bound using the descriptor's fine step (Edit+Left/Right) and coarse step
   (Edit+Up/Down), refusing at the boundaries as `ParameterAtBoundary`.
5. Every arm returns the existing typed rejection vocabulary. Add no new
   rejection kind unless the crest-spec declares one.

**Files**:
- `src/control/app_state.rs` (modified, ~140 lines plus tests)

**Validation**:
- [ ] Adjusting master gain from PATCH and reading it from MIXER yields the same
      value — assert this directly, in both directions
- [ ] No second master-gain field exists in `AppState`, the state tree, or the
      parameter snapshot. Grep for it and assert the count
- [ ] A MIDI channel change re-targets which incoming part drives the Patch and
      changes nothing else
- [ ] Voice limit honours fine and coarse steps from the descriptor, not literals
- [ ] Every boundary refusal is the typed unchanged rejection

**Edge Cases**:
- Master gain adjusted from PATCH while MIXER holds a remembered focus on the
  same control: the remembered path stays valid; no focus repair is needed
  because the identity did not change.

---

### T010: Add `PatchDetailSubject`, `SurfaceId::PatchDetail`, and the widened `ReturnPath`

**Purpose**: One subordinate surface identity that serves both instrument and
effect subjects.

**Steps**:
1. In `src/control/semantic_focus.rs`, add `SurfaceId::PatchDetail`. Update
   `ALL`, `context()`, and every exhaustive match. It is **not** a `main_for` or
   `side_for` result — it is subordinate.
2. Add `PatchDetailSubject` as a two-variant union:
   `Instrument(CapabilityId)` and `Effect(EffectSlotId, EffectCapabilityId)`.
   It names a capability identity only — no section list, no control list, no
   label, no accent, no descriptor copy.
3. Widen `ReturnPath::enteredSurface` to admit `PatchDetail`.
4. Add a resolver that derives a `PatchDetailSubject` from a PatchMain
   `FocusPath`: the engine row and active-instrument capability rows resolve
   `Instrument`; an occupied effect slot and its occupant parameter rows resolve
   `Effect`. Every other path resolves `None` — including empty slots and every
   Utility row.

**Files**:
- `src/control/semantic_focus.rs` (modified, ~120 lines)

**Validation**:
- [ ] `SurfaceId::ALL` has five members and every exhaustive match compiles
- [ ] `PatchDetail.context()` is `Patch`; it is never returned by `main_for` or
      `side_for`
- [ ] Subject resolution returns `Effect` with the *exact* slot identity, so two
      positions holding the same registry entry are distinct subjects
- [ ] An empty slot and every Utility row resolve to `None`

---

### T011: Add `detailSubject` to `InteractionState` with entry, exit, and refusal rules

**Purpose**: The reducer owns whether a detail surface is open, on what, and
where it came from — and the three facts move together.

**Steps**:
1. Add `detail_subject: Option<PatchDetailSubject>` to `InteractionState`.
2. Enforce the invariant that `detail_subject` is `Some` **exactly** when
   `focus_path.surface` is `PatchDetail` and `return_path.entered_surface` is
   `PatchDetail`. There must be no reachable state in which a detail surface is
   open without a subject, or a subject outlives its surface. Write this as a
   debug assertion or a constructor invariant, not just as a convention.
3. **Entry**: accepted only from a PatchMain path whose control resolves a
   subject. Record the exact origin, derive the subject, focus the subject
   descriptor's first visible enabled control. Every other origin is a typed
   unchanged rejection.
4. **Exit**: `Return` restores the origin, clears `return_path` and
   `detail_subject` together, enters `Navigate`.
5. **Non-nesting**: entering `PatchDetail` from `PatchUtility`, or `PatchUtility`
   from `PatchDetail`, is a typed unchanged rejection. One remembered origin is
   the whole contract.

**Files**:
- `src/control/interaction_state.rs` (modified, ~130 lines)
- `src/control/app_state.rs` (modified, ~80 lines — the entry/exit arms)

**Validation**:
- [ ] No reachable state pairs an open detail surface with a `None` subject, or
      a `Some` subject with a non-detail surface
- [ ] Entry from an empty slot is a typed unchanged rejection, not an empty shell
- [ ] Entry from a Utility row is a typed unchanged rejection
- [ ] Entry from Utility, and Utility from detail, are both refused
- [ ] Return lands on the *exact* originating row, not a recomputed default

---

### T012: Clear subordinate surfaces and recover focus across a patch switch

**Purpose**: A detail subject belongs to the Patch it was opened on. Carrying it
across a switch would show one Patch's capability under another Patch's identity.

**Steps**:
1. In the `SelectPatch` reducer path, leave any open subordinate surface first —
   clear `return_path` and `detail_subject` before the switch resolves.
2. Rebuild `remembered_patch_path` and the active path against the destination
   Patch's own descriptor schema: keep the same control when the destination
   offers it, otherwise recover deterministically through the existing
   next-before-previous sibling rule.
3. Reproject every Utility value from the destination Patch — `MidiInput` and
   `VoiceLimit` are Patch-local and must follow the switch. `Global(MasterGainDb)`
   is not Patch-local and must *not* change.
4. Leave an in-flight structural edit correlated to the Patch it was started on.
   The switch moves focus; it does not move or cancel the edit.
5. The whole switch is **one** advanced generation. No intermediate state may
   pair one Patch's identity with another's schema.

**Files**:
- `src/control/app_state.rs` (modified, ~90 lines plus tests)
- `src/control/interaction_state.rs` (modified, ~30 lines)

**Validation**:
- [ ] A switch with a detail surface open closes it first and lands on a valid
      destination path
- [ ] `MidiInput` and `VoiceLimit` reproject from the destination; master gain
      does not change
- [ ] A destination declaring fewer rows recovers focus deterministically rather
      than carrying a stale control identity
- [ ] An in-flight structural edit stays attached to its origin Patch
- [ ] The switch advances the generation exactly once — assert the delta is 1
- [ ] A switch at either end of the installed order is a typed unchanged
      rejection and leaves state identical

**Edge Cases**:
- Switching while an edit is in flight on the *destination* Patch (possible if
  the player switched away and back): the edit is still that Patch's, and focus
  recovery must not disturb it.

## Definition of Done

- [ ] All six subtasks complete
- [ ] PATCH Utility resolves exactly five rows, all editable through the reducer
- [ ] Master gain has exactly one canonical owner, reachable from two surfaces
- [ ] `PatchDetail` exists with its subject, entry rules, and exact return
- [ ] Subordinate surfaces do not nest
- [ ] A patch switch is one generation, recovers focus against the destination's
      schema, and refuses at the ends
- [ ] `cargo test --all-targets` green; `cargo clippy --all-targets -- -D warnings` clean

## Risks & Mitigations

| Risk | Mitigation |
| --- | --- |
| A second master-gain owner that looks right until it drifts | Reuse the existing canonical setter; assert cross-surface equality in both directions and grep for a duplicate field |
| The three detail fields drifting apart | Enforce the together-or-not-at-all invariant structurally, not by convention |
| Focus carrying a control the destination cannot host | Recover through the existing deterministic resolver; assert with a destination that declares fewer rows |
| A patch switch spanning two generations | Assert the generation delta is exactly 1 |

## Reviewer Guidance

**Verify**:
- The master-gain path reaches the same value the mixer arm reaches — read the
  code, do not trust the test name
- Subject resolution carries the exact `EffectSlotId`, so duplicate registry
  entries in two slots are distinct subjects
- Every new rejection is from the existing typed vocabulary

**Red Flags**:
- A `master_gain_db` field appearing anywhere on the PATCH side
- `detail_subject` set or cleared independently of the surface and return path
- A `SurfaceId::PatchDetail` returned from `main_for` or `side_for`
- Focus recovery falling back to "first row" instead of the sibling rule
