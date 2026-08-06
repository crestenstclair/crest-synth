---
work_package_id: WP05
title: Headless acceptance retargeting and input witness
dependencies:
- WP01
- WP02
requirement_refs:
- FR-008
planning_base_branch: feat/webview-shell-cutover
merge_target_branch: feat/webview-shell-cutover
branch_strategy: Planning artifacts for this mission were generated on feat/webview-shell-cutover. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/webview-shell-cutover unless the human explicitly redirects the landing branch.
subtasks:
- T017
- T018
- T019
- T020
- T021
history:
- '2026-08-06: authored from plan IC-05 (test half) + IC-06 (witness), crest-spec assets BehavioralAcceptanceTests/GraphicalShellAcceptanceTests/SemanticGraphicalViewModelAcceptanceTests/Component*AcceptanceTests'
agent_profile: implementer-ivan
authoritative_surface: tests/
create_intent:
- tests/shell_event_dispatch.rs
execution_mode: code_change
owned_files:
- tests/shell_event_dispatch.rs
- tests/graphical_application_shell.rs
- tests/semantic_graphical_view_model.rs
- tests/component_vocabulary.rs
- tests/component_composition.rs
- tests/input_capture.rs
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

Re-prove the five egui-path acceptance contracts against the webview
projection path while BOTH shells still exist, and land the automated
key-injection witness (foundation FR-003's accepted PARTIAL, recorded for
this mission). After this WP, every headless proof the deletion (WP07) could
orphan already has its webview twin green. `tests/eframe_context.rs` is NOT
deleted here — WP07 deletes it with the egui layer; until then both proof
generations coexist.

Authorities: the crest-spec's re-declared validations — `shell_event_dispatch`
(new name, marker `CREST_ACCEPTANCE shell_event_dispatch passed`),
`graphical_application_shell`, `semantic_graphical_view_model`,
`component_vocabulary`, `component_composition` — and their asset prompts
(`BehavioralAcceptanceTests` et al.), requirement
`headless_shell_event_verification`, spec FR-008.

## Context

- `spec-kitty accept` runs the declared validations, which at HEAD already
  name `shell_event_dispatch` and describe webview-path proofs. They go
  green as you land each target. No ignored tests, no pre-assertion markers,
  no assertion-free smoke tests (asset rules).
- The webview headless mechanism: drive normalized `WindowInput` through the
  production adapter callback wired to `AppLoop.dispatchAction`; render the
  next serialized document; assert on the document and forwarded
  `ShellFrameObservation` rectangles (WP02's seam) instead of egui
  tessellation. `tests/webview_projection_shell.rs` (WP01) shows the
  document-assertion harness style — reuse its fixtures/helpers by
  reference.
- "Production render path" for component proofs now means: authored values
  reach the page through the generated token table and the rendered
  document — compare values end-to-end, not names.

## Subtasks

### T017 — tests/shell_event_dispatch.rs

**Purpose**: the renamed headless event-dispatch contract
(`validation.shell_event_dispatch`) proves event → document coherence.

**Steps**:
1. Create `tests/shell_event_dispatch.rs`. Port the behavioral intent of
   `tests/eframe_context.rs` (read it — it is the contract inventory):
   dispatch normalized key/focus events through the production shell
   adapter's event path with the callback wired to
   `AppLoop.dispatchAction`; prove the next serialized document, EventLog
   record, accepted state, exact shell and diagnostic projection values,
   frame-observation region geometry, engine-row lifecycle status,
   selection, and scroll target all reflect that event. No native window.
2. Rendering a separately supplied projection must be structurally
   impossible in the harness (the assertion reads the document the real
   callback produced).
3. Marker: `CREST_ACCEPTANCE shell_event_dispatch passed` after all
   assertions; `cargo test --test shell_event_dispatch -- --nocapture`.

### T018 — Retarget tests/graphical_application_shell.rs

**Purpose**: region/structure proof through the webview path.

**Steps**:
1. Replace the egui RawInput/tessellation mechanics: drive normalized
   `WindowInput` at 1920x1080 and 1280x800; assert exact context line,
   header, workspace, side region, footer identity, visibility, ordering,
   bounds, and non-overlap from the serialized rendered document plus
   forwarded frame-observation rectangles.
2. Keep the PATCH/MIXER switch proof: switch through
   KeyboardInputTranslator/SemanticAction/AppLoop; assert document,
   SemanticGraphicalViewModel, GraphicalShellProjection, TextProjection,
   StateTree, EventRecord, ParameterSnapshot generation agreement; reject
   adapter-owned context/focus/mode/return state.
3. Marker unchanged: `CREST_ACCEPTANCE graphical_application_shell passed`.

### T019 — Retarget tests/semantic_graphical_view_model.rs

**Purpose**: only its render half changes — semantic assertions stay.

**Steps**:
1. The action/focus/recovery/projection assertions are renderer-neutral —
   do not touch them.
2. Replace the "render through production egui frames at both viewports"
   section: render the same immutable model through the webview projection
   path at both viewports; prove focus, valid actions, return path,
   generation, state hash, session values, graph revision,
   ParameterSnapshot, and audio behavior identical despite different
   rectangles.

### T020 — Retarget component vocabulary/composition tests

**Purpose**: authored-value fidelity and composition coverage proven on the
webview path.

**Steps**:
1. `tests/component_vocabulary.rs`: compare every declared color/type/
   spacing/geometry value against the authored table THROUGH the render
   path — authored Rust value → generated token → rendered document/page
   usage. Values, not names. Keep the no-literal guard, density-policy
   band/target assertions, state-coverage and gallery-reachability checks,
   and typed typeface-failure proof, retargeted to the webview surface.
2. `tests/component_composition.rs`: keep selection-totality,
   state-applicability, region-from-declared-composition, no-invented-value,
   ownership-boundary, and mixer-column-anatomy proofs; the render-path
   drive becomes the webview document. "The render adapter holds no paint
   decision" now asserts against `TauriWebviewWindow` (transport-only) —
   the guard must fail if a paint/layout decision reappears Rust-side.
3. Markers unchanged.

### T021 — Automated key-injection witness

**Purpose**: close the foundation's FR-003 PARTIAL — the full key vocabulary
provably reaches the translator in the running webview shell without a
human.

**Steps**:
1. Extend `tests/input_capture.rs` (or a focused harness within it): with
   the webview shell running (debug seam page is fine), synthesize the full
   `WindowKey` vocabulary through the NSEvent local-monitor path — press,
   release, and focus-loss — and assert each arrives at
   `KeyboardInputTranslator` exactly once with correct kind/key, using the
   production monitor, not a translator-level shortcut.
2. Where true NSEvent synthesis needs the harness process to own the window,
   structure it as a `#[cfg(target_os = "macos")]` integration test binary
   run; if a CI/headless environment cannot host it, gate on window
   availability with a typed skip that FAILS (not skips) when explicitly
   requested via env (`CREST_REQUIRE_KEY_WITNESS=1`) — the operator runs it
   locally as part of WP06's checklist. No silent skip in the default local
   run on this machine.
3. Keep the existing bijectivity test intact.

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-cutover`.
Execution worktrees are allocated per computed lane from `lanes.json`; enter
the lane workspace `spec-kitty agent action implement WP05 --agent claude`
gives you.

## Definition of Done

- Five markers green: `shell_event_dispatch`, `graphical_application_shell`,
  `semantic_graphical_view_model`, `component_vocabulary`,
  `component_composition` — all through the webview path.
- Key-injection witness drives the full vocabulary through the real NSEvent
  monitor; bijectivity test intact.
- `tests/eframe_context.rs` untouched and still green (both generations
  coexist until WP07).
- No ignored tests, no pre-assertion markers, full suite green.

## Reviewer Guidance

- Open each retargeted file and hunt for assertions that went vacuous in
  translation (name-existence checks where value comparisons stood).
- Verify T017 asserts on the document produced by the REAL callback — try to
  hand it a fabricated projection; the harness should have no such input.
- Run the witness locally; confirm each key arrives exactly once (no
  double-dispatch through monitor + window path).
- Confirm markers print only after assertions (grep for early prints).
