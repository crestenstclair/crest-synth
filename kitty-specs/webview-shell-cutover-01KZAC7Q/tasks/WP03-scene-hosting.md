---
work_package_id: WP03
title: Live scenes hosted on the webview shell
dependencies:
- WP01
- WP02
requirement_refs:
- C-004
- FR-003
planning_base_branch: feat/webview-shell-cutover
merge_target_branch: feat/webview-shell-cutover
branch_strategy: Planning artifacts for this mission were generated on feat/webview-shell-cutover. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/webview-shell-cutover unless the human explicitly redirects the landing branch.
subtasks:
- T010
- T011
- T012
- T013
history:
- '2026-08-06: authored from plan IC-03 (code half), crest-spec assets TestingContextModules/WebviewShellModules'
agent_profile: implementer-ivan
authoritative_surface: src/testing/
create_intent: []
execution_mode: code_change
owned_files:
- src/testing/live_demo_runner.rs
- src/testing/live_demo_scene.rs
- src/testing/live_demo_checkpoint.rs
- src/testing/live_demo_report.rs
- src/testing/live_effects_and_buses_scene.rs
- src/testing/live_mixer_routing_measurement.rs
- src/shell/standalone_application.rs
- Makefile
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

Run all four retained live scenes through `TauriWebviewWindow`: the scenes
themselves do not change shape — the shell under them changes (research.md
D-07). Checkpoint protocols and frozen identity baselines stay byte-identical;
webview-era additions are pure insertions (C-004). The scenes block on
qualifying forwarded frames from WP02's seam, never on wall-clock sleeps. The
egui shell remains the interactive default until WP07 — this WP makes the
`make demo-live-*` targets select the webview shell.

Authorities: crest-spec `context.Shell.StandaloneApplication` (runLiveDemo now
names `TauriWebviewWindow`), `adapter.TauriWebviewWindow` live-mode rules
(invoke injected tick without blocking; render only newest projection +
injected snapshot; close on completed report or retained fatal failure),
requirement `separate_live_demo`, spec C-004/C-007, ROADMAP gate terms.

## Context

- The four retained targets: `demo-live-graphical-shell`,
  `demo-live-semantic-view-model`, `demo-live-sixteen-track-mixer-routing`,
  `demo-live-effects-and-buses` (Makefile lines 44–56; each runs
  `cargo run --release --bin crest-synth -- --demo-live-<scene>`).
- `StandaloneApplication::runLiveDemo` composes the real window; today that
  is the eframe window. The webview window is a peer behind the same
  `AppWindow` port (foundation mission) — the composition seam exists.
- `src/bin/crest_synth.rs` is WP07-owned. If arg plumbing must change (e.g.,
  the live flags selecting the webview shell), prefer doing the selection
  inside `standalone_application.rs`'s live path, which you own. If a
  crest_synth.rs line is unavoidable, make the minimal edit and record it as
  an out-of-map edit with a one-line rationale in your lane notes.
- Frozen baselines: `FROZEN_TOPOLOGY_IDENTITY_BASELINE`
  (`tests/effects_and_buses.rs:59`) and the per-scene checkpoint identity
  sets. tests/ files are owned by WP05/WP07 — your deterministic twins run
  them but you edit only your owned files; if a twin must learn a
  webview-specific fixture seam, coordinate through lane notes.

## Subtasks

### T010 — Host the live scenes on TauriWebviewWindow

**Purpose**: `runLiveDemo` composes the webview window for every live mode.

**Steps**:
1. In `standalone_application.rs`, switch the live-demo composition to
   construct `TauriWebviewWindow` (the same construction the `--shell
   webview` path uses) with the identical injected tick, projection callback,
   observation snapshot injection, and shutdown ownership the eframe path
   had. Normal interactive mode keeps its current default (egui until WP07).
2. Verify the live-mode window input rule holds: mapped semantic input is
   ignored during the autonomous protocol; native close remains typed
   incomplete-demo cancellation (existing behavior — confirm it routes
   through the webview window's close path).
3. All four scenes launch, run their checkpoint protocols, and tear down
   (stream release, worker completion, graph collection, normal exit)
   through the webview window locally.

### T011 — Scenes block on qualifying forwarded frames

**Purpose**: visible-projection checkpoints correlate real painted webview
frames (the foundation review's D-07 risk: no sleeps).

**Steps**:
1. Where scenes/runner await "the projection for generation N is visible"
   (today: egui frame observations), consume WP02's await seam on forwarded
   `ShellFrameObservation`s. Replace any wall-clock pacing that stood in for
   paint confirmation; pacing that is deliberate scene rhythm (musical
   pauses) stays.
2. Frame-crediting counters in live reports (qualifying shell frames) now
   count forwarded webview observations. The report field names and
   semantics do not change — the source does.
3. Timeout on an awaited frame is a typed stage-specific error → retained
   failure → close + nonzero exit, per the existing ten-second/120-second
   guard model.

### T012 — Makefile targets select the webview shell

**Purpose**: `make demo-live-<scene>` runs the webview-hosted scene.

**Steps**:
1. Update the four demo-live targets (and the `demo-live` alias) so the
   invocation selects the webview shell for live modes — via the existing
   selection flag if exposed, or implicitly because T010 made live modes
   webview-hosted. Keep target names byte-identical (ROADMAP retention
   contract).
2. `make demo` (headless) and `make smoke`/`make observe` are untouched.

### T013 — Deterministic twins green with add-only identities

**Purpose**: prove no checkpoint identity regressed before rig time is spent.

**Steps**:
1. Run the deterministic twins:
   `cargo test --release --test expandable_effects_and_bus_topology`,
   `cargo test --test live_demo_scene`, plus the graphical-shell and
   semantic-view-model twins. All green.
2. Assert (in your owned scene code where the declared identity surfaces
   live) that every frozen identity set is preserved byte-identically and in
   order, and that any webview-era checkpoint identity is a pure insertion.
   If a twin's assertion file (tests/) needs a change, that is a WP05/WP07
   coordination item — flag it in lane notes rather than editing.
3. Run each `make demo-live-<scene>` once locally (real window, this
   machine's audio) to confirm end-to-end teardown before handing WP06 the
   rig checklist. Exit 0 required; this is a smoke pass, not the evidence
   run.

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-cutover`.
Execution worktrees are allocated per computed lane from `lanes.json`; enter
the lane workspace `spec-kitty agent action implement WP03 --agent claude`
gives you.

## Definition of Done

- All four live targets run webview-hosted locally with exit 0 and clean
  teardown; scene checkpoint protocols unchanged; frozen identities
  byte-identical with additions as pure insertions.
- Frame-visible checkpoints block on WP02's seam; no sleep stands in for a
  paint confirmation.
- Makefile target names unchanged; headless targets untouched.
- Deterministic twins green; clippy/fmt clean.

## Reviewer Guidance

- Diff the scenes for checkpoint identity changes — anything other than pure
  insertion is a reject (C-004).
- grep the diff for `sleep`/`thread::sleep` near frame awaits.
- Confirm the eframe interactive default still works (`make run`) — this WP
  must not flip the default (that is WP07, after evidence).
- Run one live target locally; watch for a frozen window at teardown.
