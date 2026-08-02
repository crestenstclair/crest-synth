---
work_package_id: WP06
title: Live report absent-vs-zero
dependencies: []
requirement_refs:
- FR-010
planning_base_branch: feat/expandable-effects-and-bus-topology
merge_target_branch: feat/expandable-effects-and-bus-topology
branch_strategy: Planning artifacts for this mission were generated on feat/expandable-effects-and-bus-topology. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/expandable-effects-and-bus-topology unless the human explicitly redirects the landing branch.
base_branch: kitty/mission-demo-journey-fidelity-and-hygiene-01KYWVYG
base_commit: 213052d6ee27b913f4250423c90ebb3f20a178e4
created_at: '2026-07-31T20:52:50.195110+00:00'
subtasks:
- T024
- T025
- T026
history:
- timestamp: '2026-07-31T20:21:28Z'
  actor: planner
  action: created from IC-05 (DRIFT-4 vacuous-proof risk)
agent_profile: implementer-ivan
authoritative_surface: src/testing/
create_intent: []
execution_mode: code_change
mission_id: 01KYWVYGQMTRFY314AP78KZJPY
mission_slug: demo-journey-fidelity-and-hygiene-01KYWVYG
model: ''
owned_files:
- src/testing/live_demo_report.rs
priority: P2
role: implementer
status: pending
tags: []
tracker_refs: []
---

# WP06 – Live report absent-vs-zero

## ⚡ Do This First: Load Agent Profile

**Before reading anything else in this file**, load your assigned agent profile:

```
/ad-hoc-profile-load implementer-ivan
```

## Objective

Make the live report's derived measurement fields distinguish absent evidence
from a measured zero. Today `frames_to_projection`, the activation gap, and
blocks-to-audible are computed with `.max().unwrap_or(0)`
(`src/testing/live_demo_report.rs:872-886`) — a regression that stops
populating the underlying observations reads as "0 frames", the strongest
possible pass (DRIFT-4).

## Context

- Crest-spec authority (commit `0328311`,
  `valueObject.Testing.LiveDemoReport` invariant): "every derived measurement
  or latency summary distinguishes absent evidence from a measured zero; a
  summary computed over an empty observation set renders as absent and can
  never satisfy a presence or performance expectation."
- Decision of record (research.md R-5): one optional value per measurement —
  not sentinel numerics, not a separate presence boolean.
- Boundary discipline: retained checkpoint identity fields are untouched —
  these are derived summaries. WP11 consumes the improved report on the
  physical re-run; the serialized key NAMES of the summary fields stay as
  they are (occurrence map: serialized_keys do_not_change) — what changes is
  that an absent measurement serializes/renders as an explicit absent state
  rather than a fabricated `0`.

## Subtasks

### T024 — Optional measurement representation replaces unwrap_or(0)

**Steps**:
1. Rework the derivation at `live_demo_report.rs:872-886`: each derived
   measurement becomes `Option<...>` — `None` when the contributing
   observation set is empty, `Some(measured)` otherwise (a genuinely measured
   `0` stays `Some(0)`).
2. Sweep the file for sibling `.unwrap_or(0)` / defaulted-aggregate patterns
   feeding summary or performance fields and give them the same treatment —
   the declared invariant covers every derived measurement, not just the
   three named ones. Enumerate what you found and changed in the WP notes.

**Validation**: type system now forces every consumer to face absence
explicitly; module compiles with all consumers updated (T025).

### T025 — Report rendering distinguishes absent from measured zero

**Steps**:
1. Update summary/serialization consumers inside the file: absent renders as
   an explicit absent marker (e.g., serialized `null` / "absent" in the
   human-readable summary), never as `0`.
2. Any completeness or performance expectation that reads these fields must
   treat absent as NOT SATISFIED (fail/incomplete), while `Some(0)` remains a
   legitimate measured value judged on its merits.

**Validation**: a report built from a run lacking a measurement shows the
absent marker and fails the relevant expectation; a run with a true
zero-frame measurement passes exactly as before.

### T026 — Unit tests incl. empty-observation-set absent case

**Steps**:
1. Module tests: (a) empty observation set → derived field is absent, its
   expectation fails, summary shows absent; (b) populated set with measured
   zero → `Some(0)`, renders as `0`, expectation logic evaluates it; (c)
   populated nonzero → unchanged behavior.
2. Remove the stale WP-numbered comment in this file (planning-time count: 1)
   while you are here; rewrite any genuine constraint durably.

**Validation**: tests fail if anyone reintroduces a defaulted aggregate on
these paths; grep for `WP0`/`WP10` in the file returns nothing.

## Branch Strategy

Planning happened on `feat/expandable-effects-and-bus-topology`; that branch
is also the final merge target. Execution worktrees are allocated per
computed lane from `lanes.json` during `/spec-kitty.implement`.

## Test Strategy

Module tests in `live_demo_report.rs` are the gate; then
`cargo test --all-targets` (runner/scene tests consume the report type — fix
their compile fallout within their owning WPs only if signatures leak; if a
signature would leak outside this file, prefer keeping the public surface
stable and containing optionality internally, and record the choice).

## Definition of Done

- No derived measurement in the file can render absent evidence as `0`.
- Absent fails presence/performance expectations; measured zero does not
  short-circuit them.
- Three-case unit tests present; no stale WP comments; serialized field
  names unchanged.

## Reviewer Guidance

- Hunt for surviving `unwrap_or(0)` / `unwrap_or_default()` on measurement
  paths in this file — any survivor is a finding.
- Check the absent case FAILS the expectation (an absent that renders
  "absent" but still passes is the same vacuous proof in new clothes).
- Confirm serialized key names did not change (add-only evidence
  vocabulary).
