---
work_package_id: WP04
title: 'Composition root: gap preservation & loud defaults'
dependencies: []
requirement_refs:
- FR-007
- FR-008
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
subtasks:
- T017
- T018
- T019
- T020
history:
- timestamp: '2026-07-31T20:21:28Z'
  actor: planner
  action: created from IC-03 (DRIFT-2 silent fallback) and the DRIFT-1 round-trip site
agent_profile: implementer-ivan
authoritative_surface: src/shell/
create_intent: []
execution_mode: code_change
mission_id: 01KYWVYGQMTRFY314AP78KZJPY
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
model: ''
owned_files:
- src/shell/standalone_application.rs
- src/adapter/production_effects.rs
- tests/production_runtime_contracts.rs
priority: P2
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP04 – Composition root: gap preservation & loud defaults

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Two production-composition fixes, test-first: (1) the round-trip at
`src/shell/standalone_application.rs:1470`
(`with_post_effects(patch.post_effects().to_vec())`) must stop silently
re-compacting gapped chains — rebuild from the per-position view; (2) a
failed default bus-return composition must abort startup with an explicit
error instead of `unwrap_or_default`
(`src/adapter/production_effects.rs:89-91`, consumed at
`standalone_application.rs:715`).

## Context

- DRIFT-1's concrete latent defect lives here: a Patch with slot 0 empty and
  slot 1 occupied, round-tripped through the composition root, comes back
  with the occupant moved to slot 0 — violating the documented
  never-compacted contract (crest-spec `aggregate.Synth.Patch` invariant,
  and `EffectSlotIndex`: "position is stable").
- DRIFT-2: `production_default_bus_returns(registry).unwrap_or_default()`
  boots the instrument with silent returns 0/1 on a genuine composition
  defect — the exact failure mode FR-014 (parent) exists to surface. The
  crest-spec now declares (`aggregate.Mixer.BusReturnBank`): composing the
  declared default either succeeds exactly or surfaces its error at the
  production composition root.
- The permissive helper is documented for partial test registries — keep
  permissiveness available to tests; the PRODUCTION root propagates.
- Test-first is mandatory here (plan IC-02 risk note): the regression test
  must exist and fail against the current behavior before the fix lands.
- Migrate the remaining `post_effects()` call sites in the two owned source
  files as part of T018 (this WP owns them; WP05 needs zero callers left).

## Subtasks

### T017 — Gapped-chain round-trip regression test (test-first)

**Steps**:
1. In `tests/production_runtime_contracts.rs`, build a Patch whose chain has
   slot 0 empty and slot 1 occupied (use the production registry/services the
   target already composes — no bespoke seams).
2. Drive it through the same composition-root path that
   `standalone_application.rs:1470` exercises and assert the occupied entry
   remains at slot 1 with slot 0 empty afterward — byte-exact occupancy
   comparison per position.
3. Run it against the unfixed code and record in the WP notes that it fails
   (re-compaction observed) — that failure is the proof the test has teeth.

**Validation**: test exists, fails pre-fix, passes post-fix (T018).

### T018 — Composition-root round-trip rebuilds from effect_slots

**Steps**:
1. Rewrite the `:1470` round-trip to reconstruct the Patch's chain from the
   per-position `effect_slots()` view (position-preserving), eliminating the
   compact-view round-trip entirely.
2. Migrate every other `post_effects()` call site in
   `standalone_application.rs` and `production_effects.rs` to the
   per-position view under the same rules as WP02/WP03 (no local
   re-compaction; frozen serialized vocabulary).
3. If `with_post_effects` has no remaining production callers after this,
   note it for WP05 (which deletes the constructors/accessors).

**Validation**: T017 green; `grep -n "post_effects()"` over the two source
files → no output; full suite green.

### T019 — Propagate default-return composition errors at the production root

**Steps**:
1. Change the production path so a failed
   `production_default_bus_returns(registry)` propagates: the composition
   root (`standalone_application.rs:715` region) surfaces a typed startup
   error naming the composition failure — no `unwrap_or_default`, no silent
   returns.
2. Keep the permissive default available for partial TEST registries only —
   if both shared one helper, split them so the permissive variant is
   unreachable from the production root (closing the class by construction,
   not by comment).
3. Add a `production_runtime_contracts` case: a registry that cannot compose
   the declared default occupancy makes production composition fail with the
   typed error (assert the error identifies the default-return composition,
   not a generic boot failure).

**Validation**: new test green; production boot path has no
`unwrap_or_default` on the default-return composition; existing healthy-boot
tests unchanged.

### T020 — Stale WP-comment cleanup (shell/adapter files)

**Steps**:
1. Remove stale WP-numbered handoff comments in the two owned source files
   (planning-time counts: standalone_application 3, production_effects 3);
   rewrite genuine constraints durably.

**Validation**: grep for `WP0`/`WP10` in owned files returns nothing.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch
is also the final merge target. Execution worktrees are allocated per
computed lane from `lanes.json` during `/spec-kitty.implement`.

## Test Strategy

Test-first for both fixes: T017 precedes T018; T019's failure-path case lands
with the propagation change. Run `cargo test --test
production_runtime_contracts` and `cargo test --all-targets`.

## Definition of Done

- Gapped chain survives the composition-root round-trip exactly (T017 green,
  and it demonstrably failed pre-fix).
- Failed default-return composition = typed, attributable startup error;
  permissive path unreachable from production.
- Zero `post_effects()` callers left in owned source files; no stale WP
  comments; occurrence-map compliant.

## Reviewer Guidance

- Demand the pre-fix failure evidence for T017 (a test that never failed
  proves nothing here).
- Check the error path is genuinely reachable from the real production root,
  not a parallel constructor nobody calls (that was the DRIFT-2 pattern).
- Verify the permissive test-registry path cannot be selected by production
  composition (type/visibility, not convention).
