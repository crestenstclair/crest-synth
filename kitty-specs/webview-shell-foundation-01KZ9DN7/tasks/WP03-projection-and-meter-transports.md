---
work_package_id: WP03
title: Projection and meter transports
dependencies:
- WP02
requirement_refs:
- FR-004
- FR-005
planning_base_branch: feat/webview-shell-foundation
merge_target_branch: feat/webview-shell-foundation
branch_strategy: lane worktree computed by finalize-tasks; merges into feat/webview-shell-foundation
subtasks:
- T011
- T012
- T013
history:
- '2026-08-05: authored from plan IC-03'
agent_profile: implementer-ivan
authoritative_surface: src/shell/webview/
create_intent:
- src/shell/webview/projection_channel.rs
- src/shell/webview/meter_channel.rs
execution_mode: code_change
owned_files:
- src/shell/webview/projection_channel.rs
- src/shell/webview/meter_channel.rs
role: implementer
tags: []
tracker_refs: []
---

## ⚡ Do This First: Load Agent Profile

Before reading anything else in this prompt, load your assigned profile:

```
/ad-hoc-profile-load implementer-ivan
```

Adopt its identity, boundaries, and governance scope for the duration of this
work package.

## Objective

Carry projections and meters from the control side into the page: every
accepted `GraphicalShellProjection` reaches the webview as the serde
serialization of its embedded `SemanticGraphicalViewModel` (one schema, no
fork, no page-facing DTO), and `AudioObservationSnapshot` meter frames flow
on a separate 30 Hz latest-value channel that never blocks, never queues
unboundedly, and never touches the real-time callback.

## Context

- Plan IC-03; research R-03 (push via tauri `Emitter`, coalesced meters);
  crest-spec `requirement.serialized_projection_transport` — read it, it is
  the acceptance wording.
- WP02 left seams in `src/shell/webview/window.rs` marked `// WP03`: the
  held projection per tick, and the app handle for emitting.
- The serialization already exists: `SemanticGraphicalViewModel` derives
  `Serialize` (`src/control/semantic_graphical_view_model.rs`). The spike
  proved page-sufficiency (`spike/webview-mixer/`, 85 KB for the production
  MIXER fixture). You serialize the projector's value — you do not define
  any new struct.
- Meter semantics reference: DESIGN.md's transport table — meters are
  "atomics or latest-value MeterSnapshot, decimated; UI polls". How the
  eframe window obtains `AudioObservationSnapshot` per frame is the pattern;
  the webview equivalent pushes the same snapshot at the decimated rate.

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-foundation`.
Execution happens in the lane worktree `finalize-tasks` computes; do not
branch manually.

## Subtasks

### T011 — Push serialized view models on accepted projections

`src/shell/webview/projection_channel.rs`:

- On each tick where the fetched `GraphicalShellProjection` generation
  differs from the last emitted one, serialize the embedded semantic model
  with `serde_json` and emit it on a named event (constant, e.g.
  `crest://projection`) via the tauri `Emitter`.
- Generation-gating, not deep comparison: the projection already carries a
  generation/state-hash (see the view model's `generation`/`stateHash`
  fields) — emit when it changes.
- Emit errors (window gone during shutdown) are typed and reported through
  the window's error path, not swallowed and not fatal during teardown —
  match how the eframe path treats late-frame conditions.
- No trimming, no field selection, no page-specific wrapper. If WP05
  discovers a missing field, the fix is in the projector via crest-spec, not
  here.

### T012 — Coalesce meters at 30 Hz latest-value

`src/shell/webview/meter_channel.rs`:

- A 30 Hz timer (or tick-derived divider) reads the latest
  `AudioObservationSnapshot` through the same accessor the eframe window
  uses and emits it on a second named event (`crest://meters`), serialized
  with the snapshot's existing `Serialize` (verify it derives; if not, STOP —
  that derive belongs beside the type, one-line change, note it for review).
- Latest-value only: no buffering, no send queue, at most one pending frame;
  a missed emit is simply superseded by the next read. Document the loss
  semantics in module docs — display-only degradation.
- Rate is a named constant with the NFR-002 pointer, not a magic number.

### T013 — Assert the transports add nothing to the RT callback

Confirm by construction and record in module docs: both channels read
control-side state (projection callback, observation accessor) on the
window/event thread; neither installs anything into the audio callback,
allocates in it, or adds a lock it can contend on. Add a focused unit test in
`projection_channel.rs`/`meter_channel.rs` (mod tests) proving
generation-gating (same generation → no emit; changed → one emit) and
meter coalescing (N rapid reads → latest value wins). The full NFR-002/003
measurement lands in WP06.

## Definition of Done

- [ ] Reducer edit → page receives the exact projector serialization
      (manually verified against a recorded document)
- [ ] Meters stream at 30 Hz; unit tests prove gating and coalescing
- [ ] No new state in the page's direction beyond the two event payloads
- [ ] All existing tests pass unchanged
- [ ] `spec-kitty agent tasks mark-status T011 T012 T013 --status done`

## Risks

- tauri emit cost at 85 KB per projection — fine at edit rates (R-03), but
  do not emit per-tick without generation-gating.
- Shutdown races: emitting after window teardown must be typed-and-tolerated,
  not a panic.

## Reviewer Guidance

Reject if: any page-facing struct/DTO exists; emission is per-tick without
generation gating; meter frames queue; a magic 33 ms literal appears; the
snapshot's `Serialize` was forked instead of derived beside the type.
