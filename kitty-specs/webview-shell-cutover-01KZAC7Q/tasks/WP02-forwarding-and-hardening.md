---
work_package_id: WP02
title: Frame observation forwarding and shell hardening
dependencies: []
requirement_refs:
- FR-004
- NFR-003
planning_base_branch: feat/webview-shell-cutover
merge_target_branch: feat/webview-shell-cutover
branch_strategy: Planning artifacts for this mission were generated on feat/webview-shell-cutover. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/webview-shell-cutover unless the human explicitly redirects the landing branch.
subtasks:
- T005
- T006
- T007
- T008
- T009
history:
- '2026-08-06: authored from plan IC-02 + IC-06 (CSP/gating half), crest-spec asset WebviewShellModules'
agent_profile: implementer-ivan
authoritative_surface: src/shell/webview/
create_intent: []
execution_mode: code_change
owned_files:
- src/shell/webview/**
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

Give the painted-ack → `ShellFrameObservation` forwarding an explicit owner
(the foundation mission's DRIFT-4/RISK-1 successor item), and land the shell
hardening: restrictive CSP, `CREST_WEBVIEW_PAGE` compiled out of release
builds, typed page render-exception surfacing, and window-close
retry-or-surface. Everything is Rust-side in `src/shell/webview/`.

Authorities: crest-spec `adapter.TauriWebviewWindow` rules (painted-ack
forwarding, typed errors, close handling), `valueObject.Shell.
ShellFrameObservation` invariants (emitted only after painting, copies
semantic identity exactly, "expected region names or a pre-render layout plan
alone cannot construct a passing observation"), asset `WebviewShellModules`
prompts.

## Context

- `src/shell/webview/projection_channel.rs` pushes serialized documents; the
  page's ack (WP01 T003 builds the emit side; the receive plumbing partially
  exists from the foundation) arrives back on the IPC channel. Today nothing
  forwards it — the stale comment at `src/shell/webview/window.rs:183` still
  points at "WP06" of the foundation mission.
- `src/shell/shell_frame_observation.rs` defines the observation the egui
  adapter emits post-paint. The webview forwarding must satisfy the same
  invariants: constructed only from a real painted document's identity plus
  measured region rectangles.
- WP01 runs in parallel and owns `webview-page/**` and
  `tests/webview_projection_shell.rs`. You own `src/shell/webview/**` only.
  If the ack payload shape needs negotiation, the shape is: generation,
  stateHash, context, activeSurface, focusPath, interactionMode, plus the
  page-measured region rectangles — matching `ShellFrameObservation` fields.
  Coordinate through the lane notes, not by editing WP01's files.

## Subtasks

### T005 — Painted-ack → ShellFrameObservation forwarding

**Purpose**: each page ack becomes exactly one `ShellFrameObservation`
available to the control side.

**Steps**:
1. In `projection_channel.rs`, receive the ack and construct a
   `ShellFrameObservation`: copy generation/stateHash/context/surface/focus/
   mode verbatim from the ack; take viewport and region rectangles from the
   ack's measured geometry. Reject (typed error, not panic) an ack whose
   identity matches no in-flight pushed document — the observation must be
   impossible to construct from expectations alone.
2. Track in-flight documents minimally (generation → pushed-at) so acks
   correlate; bounded structure, no unbounded queue. A superseded document
   whose ack never arrives is dropped from tracking when its successor acks —
   lost frames degrade observation only.
3. Emit the observation on the same control-side path the egui adapter uses
   today (find the consumer of the eframe adapter's observations — the live
   report crediting — and present an identical seam). No RT-callback work
   anywhere in this path.
4. Fix the stale `window.rs:183` comment to describe the now-owned
   forwarding.

### T006 — Qualifying-frame stream for live-report crediting

**Purpose**: WP03's scenes must block on "a qualifying frame for generation N
was painted" instead of sleeping.

**Steps**:
1. Expose a control-side await/poll seam: given a generation (or predicate on
   observation fields), report when a qualifying observation has been
   forwarded. Non-blocking poll + blocking-with-timeout variants; the
   timeout path returns a typed error naming the awaited identity.
2. Qualifying = the observation's generation/stateHash match the awaited
   accepted generation and the context/surface match the expectation —
   mirror how the egui live path credits frames today (read
   `src/testing/live_demo_runner.rs` usage; do not edit it — WP03 owns it).
3. Document the seam in `mod.rs` — one paragraph, what qualifies and what
   the timeout means.

### T007 — CSP + release gating

**Purpose**: hardening from the foundation review (open item 5).

**Steps**:
1. Set a restrictive CSP for the page: default-src 'none' baseline, then
   allow exactly what the embedded `crest://` assets need (style/script/font/
   img from the app scheme; no remote hosts, no inline-eval). Verify the
   shipped page still loads — the page inlines nothing remote by contract.
2. Gate `CREST_WEBVIEW_PAGE` behind `cfg(debug_assertions)` in `window.rs`
   (both the env read and the serving branch). Release builds must not
   contain the override path at all. Keep the debug seam working — WP01's
   validation and the T025-style tests use it.

### T008 — Typed render-exception and close handling

**Purpose**: a page that loads then fails to render must surface, and a
close failure must never be ignored (foundation RISK-2).

**Steps**:
1. Catch page render exceptions (WP01's render throws on failure) via the
   IPC error channel; convert to the existing typed webview error enum and
   route through the same fatal-runtime-failure path a startup failure uses —
   window closes, process exits nonzero, no frozen window.
2. `window.close()` failure: retry once, then surface the typed error
   through the shutdown path rather than swallowing it. Shutdown ordering
   (stream release, worker completion, graph collection) is unchanged.

### T009 — Test coverage for forwarding and typed errors

**Purpose**: the forwarding contract is falsifiable without hardware.

**Steps**:
1. In `src/shell/webview/` unit/integration tests: one pushed document + its
   ack → exactly one observation with verbatim identity; ack with unknown
   identity → typed rejection, zero observations; two documents where the
   first never acks → one observation, tracker bounded.
2. Await-seam tests: qualifying observation satisfies the await; timeout
   returns the typed error naming the identity.
3. Release-gating test: `#[cfg(debug_assertions)]`-guarded test proves the
   debug seam works; a compile-time assertion (or cfg-gated absence test)
   documents that release builds exclude the override.

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-cutover`.
Execution worktrees are allocated per computed lane from `lanes.json`; enter
the lane workspace `spec-kitty agent action implement WP02 --agent claude`
gives you.

## Definition of Done

- One ack → one observation, identity verbatim, post-paint-only
  constructibility enforced and tested.
- Await seam documented and tested; no sleeps anywhere in it.
- CSP restrictive with the shipped page loading; `CREST_WEBVIEW_PAGE` absent
  from release builds.
- Render exceptions and close failures are typed and surfaced; teardown
  ordering unchanged.
- `cargo test` green; clippy/fmt clean; no RT-callback work added.

## Reviewer Guidance

- Try to construct an observation without a pushed document — the type
  system or a typed rejection must stop you.
- grep the forwarding path for `sleep`/`Duration::from` waits — reject any.
- Check the CSP against `index.html`'s actual needs; a too-loose CSP
  (unsafe-inline without need, remote hosts) is a finding.
- Confirm `window.rs:183`'s comment now tells the truth.
