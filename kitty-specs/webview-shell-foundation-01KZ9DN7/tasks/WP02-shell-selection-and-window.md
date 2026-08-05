---
work_package_id: WP02
title: Shell selection and window composition
dependencies:
- WP01
requirement_refs:
- FR-001
- FR-006
- FR-007
planning_base_branch: feat/webview-shell-foundation
merge_target_branch: feat/webview-shell-foundation
branch_strategy: lane worktree computed by finalize-tasks; merges into feat/webview-shell-foundation
subtasks:
- T006
- T007
- T008
- T009
- T010
history:
- '2026-08-05: authored from plan IC-02'
agent_profile: implementer-ivan
authoritative_surface: src/shell/webview/
create_intent:
- src/shell/webview/mod.rs
- src/shell/webview/window.rs
execution_mode: code_change
owned_files:
- src/shell/webview/mod.rs
- src/shell/webview/window.rs
- src/shell/standalone_application.rs
- src/shell/mod.rs
- src/bin/crest_synth.rs
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

Make the webview shell a real, explicitly selected peer of the eframe window:
`crest-synth --shell webview` opens a Tauri v2 window satisfying the
`AppWindow` contract, `--shell egui` (and no flag) opens the existing eframe
window untouched, webview startup failure is a typed error ending the
process, and window close drives the same owned shutdown as the eframe path.
No projection content yet — the window may show a placeholder page; WP03/WP05
fill it.

## Context

- Plan IC-02; crest-spec `adapter.TauriWebviewWindow` (peer of
  `EframeGraphicalWindow` behind `port.AppWindow`) and
  `requirement.webview_projection_shell`.
- `port.AppWindow` contract (crest-spec `contexts/shell.yaml`): `run(onInput,
  projection, audioObservation, onTick, onFrame) -> Result<(), WindowError>` —
  read the invariants there; they are the acceptance terms.
- WP01's `research/input-capture-probe.md` tells you how to install
  `input_capture::install(sink)` and any threading constraints. The sink
  feeds `KeyboardInputTranslator` exactly as
  `src/adapter/eframe_graphical_window.rs` does — read how it normalizes
  press/release and focus loss into `WindowInput` and reuse that path; do
  not fork a second key state machine (crest-spec `passive_graphical_window`
  precedent).
- Shutdown parity reference: how `StandaloneApplication` +
  `src/shell/app_window.rs` drive stream release, worker completion, and
  graph ownership collection when the eframe window closes. Trace it before
  writing the Tauri close path.

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-foundation`.
Execution happens in the lane worktree `finalize-tasks` computes; do not
branch manually.

## Subtasks

### T006 — Declare shell selection and the typed webview startup error

`src/shell/webview/mod.rs`:

- `pub enum ShellSelection { Egui, Webview }` parsed from `--shell
  <egui|webview>` (default `Egui`) — parsing lives with the existing arg
  handling in `src/bin/crest_synth.rs`, the enum lives here.
- `pub enum WebviewShellError` with typed variants at minimum:
  `RuntimeUnavailable`, `PageLoadFailed`, `WindowCreation` — each carrying
  the underlying cause, implementing `std::error::Error`, surfaced through
  the same top-level error path other startup failures use (find how
  `StandaloneApplication` reports fatal startup errors and join it).
- Module docs stating the invariant: selection is a launch-time decision;
  there is no fallback edge from one shell to the other in any code path.

### T007 — Compose the Tauri window satisfying the AppWindow contract

`src/shell/webview/window.rs`: `TauriWebviewWindow` implementing the
`AppWindow` port trait (`src/shell/app_window.rs`):

- `tauri::Builder` with one window loading the page asset (for this WP a
  minimal placeholder `index.html` may be inlined via a data URL or tauri's
  asset protocol — real page arrives in WP05; leave a `// WP05` seam).
- Wire `input_capture::install` per WP01's verdict; sink →
  `KeyboardInputTranslator` → `onInput(SemanticAction)`. Press/release and
  focus-loss fidelity per the eframe adapter's example.
- Drive `onTick` from a timer on the main/event thread at the cadence the
  eframe path uses (16 ms idle frame convention); a `false` tick result
  closes the window only after control ownership retained a terminal
  outcome (port invariant — read it verbatim).
- Call `projection()` each frame-equivalent and hold the result for WP03's
  channels (emit seam, `// WP03` marker); call `onFrame` with a
  `ShellFrameObservation` carrying what the webview shell can honestly
  observe this WP (window open/close lifecycle) — check what the
  observation type requires and populate truthfully, never invent.

### T008 — Add the launch-time selection seam

`src/shell/standalone_application.rs` + `src/bin/crest_synth.rs`: one match
on `ShellSelection` choosing which `AppWindow` implementation runs. The
eframe arm is byte-identical behavior to today. No shared mutable state
between arms; everything downstream (audio, worker, reducer wiring) stays
common. Keep the diff in these two files minimal and mechanical — reviewers
will diff the egui path for accidental drift.

### T009 — Drive window close through the owned shutdown path

Tauri's close/exit events → the same shutdown sequence the eframe close
performs: stream release, worker completion, graph ownership collection,
normal exit. Reuse the existing shutdown functions; if the eframe path has
them inline, extract them into a shared helper in `src/shell/` rather than
duplicating (DIRECTIVE_044). Prove by running `--shell webview`, closing the
window, and observing the same shutdown log/observation sequence the eframe
shell produces.

### T010 — Prove typed init failure ends the process with no fallback

Point the window at an unloadable page (nonexistent asset) in a test/probe
configuration: assert the process reports `WebviewShellError::PageLoadFailed`
through the fatal-error path and exits nonzero, with no eframe window opened
and no blank window lingering. Wire whatever hook the acceptance test (WP06,
T025) will need to trigger this deterministically — an internal
`#[doc(hidden)]` page-override or env hook is acceptable if documented in
the module docs.

## Definition of Done

- [ ] `--shell webview` opens the Tauri window; default and `--shell egui`
      byte-identical to today's behavior
- [ ] Keys reach `KeyboardInputTranslator` Rust-side with press/release
      fidelity (manual check against the MIXER vocabulary)
- [ ] Close performs the owned shutdown sequence; process exits normally
- [ ] Unloadable page → typed error, nonzero exit, no fallback
- [ ] All existing tests pass unchanged
- [ ] `spec-kitty agent tasks mark-status T006 T007 T008 T009 T010 --status done`

## Risks

- tauri main-thread constraints vs. the existing startup ordering — WP01's
  research note is the guide; audio starts before the event loop.
- Accidental behavior drift in the eframe arm — keep T008 mechanical.

## Reviewer Guidance

Reject if: any fallback edge exists between shells; a second key state
machine appears; the shutdown path is duplicated rather than shared; the
eframe arm's behavior changed; `ShellFrameObservation` fields are invented
rather than honestly observed; `WebviewShellError` is stringly-typed.
