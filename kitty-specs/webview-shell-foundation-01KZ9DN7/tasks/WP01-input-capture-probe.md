---
work_package_id: WP01
title: Input capture and coexistence probe
dependencies: []
requirement_refs:
- FR-003
planning_base_branch: feat/webview-shell-foundation
merge_target_branch: feat/webview-shell-foundation
branch_strategy: lane worktree computed by finalize-tasks; merges into feat/webview-shell-foundation
subtasks:
- T001
- T002
- T003
- T004
- T005
history:
- '2026-08-05: authored from plan IC-01 / research R-02'
agent_profile: implementer-ivan
authoritative_surface: src/shell/webview/
create_intent:
- src/shell/webview/input_capture.rs
- src/bin/webview_input_probe.rs
- kitty-specs/webview-shell-foundation-01KZ9DN7/research/input-capture-probe.md
execution_mode: code_change
owned_files:
- Cargo.toml
- Cargo.lock
- src/shell/webview/input_capture.rs
- src/bin/webview_input_probe.rs
- kitty-specs/webview-shell-foundation-01KZ9DN7/research/input-capture-probe.md
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

Settle the mission's only unresolved mechanism BEFORE any dependent work
exists: (a) can the Rust side observe every key in the MIXER vocabulary, with
press/release fidelity, while a focused WKWebView owns the window content;
(b) does the existing cpal stream + graph-preparation worker model run
undisturbed inside a Tauri v2 process. Deliver a recorded verdict, a working
`input_capture.rs` on the winning path, and the tauri dependency the rest of
the mission builds on.

This WP is allowed to be ugly. The probe binary is disposable evidence, not
product code. `input_capture.rs` is the one durable output.

## Context

- Plan: `kitty-specs/webview-shell-foundation-01KZ9DN7/plan.md` IC-01.
- Research: `research.md` R-01 (event-loop exclusivity — the probe runs the
  Tauri loop only, never eframe), R-02 (capture paths and the declared STOP).
- Crest-spec: `adapter.TauriWebviewWindow` rules — input normalized Rust-side
  through the same `KeyboardInputTranslator` path the eframe window uses
  (`src/shell/keyboard_input_translator.rs`); the page registers no key
  handler.
- The MIXER key vocabulary is the existing one the translator already
  normalizes (see `src/shell/keyboard_input_translator.rs` and DESIGN.md's
  interaction map): 1, 2, W, S, A, D, K (Edit press/release semantics), plus
  Shift chords. Press AND release must both be observable — Edit-hold is a
  modifier.

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-foundation`.
Execution happens in the lane worktree `finalize-tasks` computes (see
`lanes.json` after finalization); do not branch manually.

## Subtasks

### T001 — Add tauri 2.x dependency to Cargo.toml

Add `tauri = { version = "2", features = [...] }` (macOS default bundle;
no tray/updater/CLI features) and whatever `tauri-build`/config scaffolding a
window-only, no-frontend-bundle setup needs. Keep the dependency unconditional
(no cargo feature gate) — shell selection is a runtime decision (R-01).
`cargo build` and `cargo test` must stay green with eframe untouched.
Watch for: tauri wanting a `build.rs` addition — the repo already has one for
the C++ vendors; extend, don't replace.

### T002 — Build the disposable input-capture probe window

`src/bin/webview_input_probe.rs`: a minimal Tauri app whose single window
loads a trivial inline HTML page (a focused `<input>` plus instructions), and
whose Rust side logs every observable key event through
`on_window_event` / tao's `WindowEvent::KeyboardInput` equivalent, tagged
`PROBE_KEY {key} {state}`. The page registers no key handler (that is the
contract under test). Run it, focus the webview, press the full MIXER
vocabulary, and capture the log.

Exit criterion for the tao path: every vocabulary key logged with distinct
press and release while the webview has focus. Partial capture (e.g. keys
swallowed once the `<input>` is focused) = path fails.

### T003 — Wire the NSEvent local-monitor fallback

Only if T002's tao path fails: add the macOS
`NSEvent.addLocalMonitorForEventsMatchingMask(.keyDown/.keyUp)` local monitor
(via `objc2` bindings or the lightest equivalent already reachable), installed
from the Rust side at window setup, feeding the same `PROBE_KEY` log. The
monitor sees events in-process before dispatch, independent of first
responder. Same exit criterion as T002.

Whichever path wins, extract it into `src/shell/webview/input_capture.rs` as
a small module: `install(sink: impl FnMut(RawKeyEvent))` where `RawKeyEvent`
carries key identity + pressed/released — shaped so WP02 can feed
`KeyboardInputTranslator` without re-touching platform code. Keep the losing
path out of the module entirely.

### T004 — Run the cpal production-fixture stream inside the probe

Inside the probe binary, start the production audio path the way
`StandaloneApplication` does (production registries, fixture patches, real
cpal stream, graph-preparation worker) and let it sound while you type. Then
close the window through the normal path.

Watch for: cpal callback underruns or worker stalls while the tao event loop
runs (log the existing RT health counters at exit, tagged `PROBE_RT`); panics
on shutdown ordering (stream must release before worker collection, exactly
as the eframe path does).

Exit criterion: audible fixture, `PROBE_RT` counters showing zero underrun
delta attributable to the webview loop, clean process exit.

### T005 — Record the probe verdict

`kitty-specs/webview-shell-foundation-01KZ9DN7/research/input-capture-probe.md`:
which capture path won and the evidence (paste the `PROBE_KEY` log for the
vocabulary sweep), the `PROBE_RT` counters, any shutdown-ordering findings,
and explicit guidance WP02 consumes (how `input_capture::install` is called,
any threading constraint tauri imposes on the sink). If BOTH paths failed:
write the failure evidence, do NOT improvise a page-side handler, and mark
the WP blocked — the mission returns to `/spec-kitty.crest-spec` per plan
IC-01.

## Definition of Done

- [ ] `cargo build` green with tauri added; all existing tests pass unchanged
- [ ] Probe demonstrates full-vocabulary press/release capture Rust-side with
      webview focused, OR documented double-failure and blocked status
- [ ] `input_capture.rs` exposes the winning path as `install(sink)`
- [ ] Audible production fixture + zero-delta RT counters + clean exit
- [ ] `research/input-capture-probe.md` records verdict and WP02 guidance
- [ ] `spec-kitty agent tasks mark-status T001 T002 T003 T004 T005 --status done`

## Risks

- tao key events under WKWebView are the known unknown (research R-02);
  the NSEvent monitor is the designed fallback, not an improvisation.
- tauri's macOS main-thread requirement vs. the probe's audio startup
  ordering — start audio before `run()` and confirm the stream owner thread.

## Reviewer Guidance

Reject if: the page registers any key handler; the probe conclusion is
asserted without the `PROBE_KEY`/`PROBE_RT` logs in the research note;
`input_capture.rs` carries both paths or platform code leaks outside it;
Cargo features gate the shell at compile time.
