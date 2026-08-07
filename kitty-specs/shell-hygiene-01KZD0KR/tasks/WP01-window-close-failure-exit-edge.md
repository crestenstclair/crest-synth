---
work_package_id: WP01
title: Window close-failure exit edge
dependencies: []
requirement_refs:
- FR-001
- FR-002
- NFR-001
- NFR-002
planning_base_branch: feat/shell-hygiene
merge_target_branch: feat/shell-hygiene
branch_strategy: Planning artifacts for this mission were generated on feat/shell-hygiene. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/shell-hygiene unless the human explicitly redirects the landing branch.
base_branch: kitty/mission-shell-hygiene-01KZD0KR
base_commit: 09a688873166b9af1c746c0789505ec8db683ced
created_at: '2026-08-07T03:02:04.587764+00:00'
subtasks:
- T001
- T002
- T003
history:
- '2026-08-06: authored from plan IC-01, research D1, crest-spec asset WebviewShellModules, mission-review RISK-3'
agent_profile: implementer-ivan
authoritative_surface: src/shell/webview/
create_intent: []
execution_mode: code_change
owned_files:
- src/shell/webview/window.rs
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
work package. Do not begin reading source or planning edits until the profile
is loaded.

## Objective

Close the one correctness hole RISK-3 named: when both attempts to close the
webview window fail, the shell records a typed `WebviewShellError::WindowClose`
on the first-error slot and then **returns normally**, and nothing else drives
termination — so `run_return` never yields, the post-`run_return` surfacing
path never runs, and a correctly recorded fatal error never reaches the
operator.

Add the missing exit edge. Change nothing else.

**Authorities** (cited, not restated — read them):

- `spec.md` FR-001 (the loop terminates and the first recorded typed error is
  surfaced), FR-002 (single-close retry, typed `WindowClose`, and the
  first-error-wins latch behave exactly as before), NFR-001, NFR-002, C-001.
- `plan.md` IC-01.
- `research.md` **D1** — the decision, its rationale, and the two rejected
  alternatives (panic; surface inline from the callback). D1 is settled. Do not
  re-open it; do not implement either rejected alternative.
- `requirement.webview_projection_shell` in the crest-spec — typed failure
  paths, no silent fallback. Read it via `spec-kitty crest-spec context`.
- Asset `WebviewShellModules`.

**Hard boundaries**:

- Do **not** touch the `WindowEvent::Destroyed` arm or the ordinary teardown
  path. That is working product behavior and NFR-001 forbids moving it.
- Do **not** touch the reducer, the real-time callback, the projection schema,
  or any product surface (C-001).
- Do **not** edit `src/shell/webview/projection_channel.rs` (WP02),
  `src/shell/webview/frame_stream.rs` or `src/shell/webview/mod.rs` (WP03), or
  `tests/webview_projection_shell.rs` (WP04). The end-to-end live proof of your
  fix is WP04 T013's job; yours is that the shell half is provably correct in
  isolation.

## Context: what exists today

Read all of this before editing. Line numbers are as of the mission's planning
commit and may drift by a line or two.

`src/shell/webview/window.rs:317-340`:

```rust
/// Closes the single webview window, retrying a failed close once. A second
/// failure is recorded as a typed [`WebviewShellError::WindowClose`] on the
/// first-error slot so the shutdown path surfaces it rather than swallowing
/// it (foundation RISK-2); shutdown ordering is unchanged.
fn close_window_once_with_retry(
    handle: &tauri::AppHandle,
    first_error: &RefCell<Option<WindowError>>,
) {
    let Some(window) = handle.get_webview_window(WINDOW_LABEL) else {
        // Already gone: the normal Destroyed → ExitRequested teardown is in
        // flight; there is nothing left to close.
        return;
    };
    if window.close().is_ok() {
        return;
    }
    if let Err(retry_failure) = window.close() {
        first_error
            .borrow_mut()
            .get_or_insert(WindowError::from(WebviewShellError::WindowClose(
                retry_failure,
            )));
    }
}
```

Its four call sites, all inside the `app.run_return(...)` callback beginning at
`window.rs:455`:

| Line | Arm | What it does |
|---|---|---|
| ~507 | projection-push failure | records the error, sets `close_requested = true`, calls the close helper |
| ~515 | `PageSignal::RenderError` | calls `record_render_failure`, sets `close_requested = true`, calls the close helper |
| ~540 | tick/frame failure | records the error, sets `close_requested = true`, calls the close helper |
| ~561 | `CloseRequested` | sets `close_requested = true`, calls the close helper, expecting `CloseRequested → Destroyed → ExitRequested` |

The first-error slot is a `RefCell<Option<WindowError>>` (`loop_runtime_error`,
declared near `window.rs:443`). `record_render_failure` (`window.rs:294-315`)
documents the latch contract in detail: `get_or_insert` means the **first**
error wins, so a `PageRenderFailed` recorded at l.512 must still win over a
`WindowClose` recorded a few microseconds later at l.336. The previous mission
proved that; your change must not break it.

The debug-only seam precedent you are asked to mirror is the
`CREST_WEBVIEW_PAGE` page override:

- `window.rs:77-80` — `#[cfg(debug_assertions)] const PAGE_OVERRIDE_ENV: &str
  = "CREST_WEBVIEW_PAGE";`
- `window.rs:233-253` — the debug-only reader and the `#[cfg(debug_assertions)]`
  branch inside `resolve_page_source`.
- `window.rs:877-885` — the in-module test that asserts release builds compile
  the seam out entirely.

Existing in-module tests around `window.rs:796-830` already drive synthetic
`PageSignal::RenderError` values through the handling and assert first-error-wins.
That is the shape to extend.

## Subtasks

### T001 — Add the missing termination edge

**Purpose**: A recorded typed fatal error must reach the operator even when the
window refuses to close twice. Today it cannot, because the only route to
`run_return` yielding is the window actually closing.

**Steps**:

1. Read `close_window_once_with_retry` and every one of its four call sites
   listed above, plus the post-`run_return` surfacing path (follow `exit_code`
   and `loop_runtime_error` from `window.rs:455` to the function's return).
   Write down, before editing, exactly which route yields control back from
   `run_return` today.
2. Change the close helper so that when the retry is exhausted — i.e. the branch
   that currently only calls `get_or_insert` — the shell **also reaches
   termination by a route that does not depend on the window closing**. D1
   names this precisely: the recorded first error is then surfaced by the
   existing post-`run_return` path, unchanged.
3. The mechanism is yours to choose within D1's constraints, but it must:
   - not panic (D1 rejects it explicitly — a panic in the shell adapter is the
     silent-fallback-class failure this program forbids and it would bypass the
     typed error);
   - not duplicate the post-`run_return` surfacing path (D1 rejects surfacing
     inline from the callback — two places a fatal error can be reported is
     worse than one);
   - leave the typed error the one that reaches the operator, verbatim.
4. Update the function's doc comment to say what the function now guarantees:
   a second close failure is both recorded **and** terminating. Name FR-001 and
   RISK-3 so a future reader can find the reason. Keep the existing sentence
   about shutdown ordering being unchanged only if it is still true after your
   change; if it is not, say what changed and why.
5. If the termination decision needs to be visible to the `run_return`
   callback's control flow (e.g. the helper returns something the caller acts
   on), update **all four** call sites consistently. A call site that ignores
   the new signal reintroduces the hole for that arm.

**Files**: `src/shell/webview/window.rs`.

**Validation**: `cargo build` and `cargo clippy --all-targets` clean;
`cargo test --lib` green.

**Edge cases**:

- The `handle.get_webview_window(...)` early return (window already gone) must
  keep returning quietly — that is the normal `Destroyed → ExitRequested`
  teardown, not a failure.
- A **first** close that succeeds must behave exactly as today. The exit edge
  belongs only to the exhausted-retry branch.
- The `CloseRequested` arm at l.561 is the ordinary user-initiated close. Its
  happy path must be untouched; only its double-failure path gains the edge.

### T002 — Preserve retry-once, the typed error, and the latch

**Purpose**: FR-002. The previous mission established three behaviors here and
this mission narrows none of them.

**Steps**:

1. **Retry-once**: exactly two `window.close()` attempts on the failure path,
   no more and no fewer. Do not turn this into a bounded loop, a backoff, or a
   single attempt.
2. **Typed error**: the second failure is still recorded as
   `WebviewShellError::WindowClose(retry_failure)` carrying the tauri error
   verbatim. Do not wrap it in a new variant, do not stringify it, do not add a
   new error type.
3. **First-error latch**: still `get_or_insert` on the shared
   `RefCell<Option<WindowError>>`. A prior `PageRenderFailed` **must still win**
   over the later `WindowClose`. This is the exact interleaving the
   `PageSignal::RenderError` arm produces: it records the render failure at
   l.512 and then calls the close helper at l.515. If both closes fail, the
   operator must be told the page render failed — the close failure is a
   consequence, not the cause.
4. Re-read `record_render_failure`'s doc comment (`window.rs:294-315`). If your
   change makes any sentence in it false, correct it. If it stays true, leave it
   byte-identical.

**Files**: `src/shell/webview/window.rs`.

**Validation**: the assertions you add in T003; plus a read-through diff review
confirming the three behaviors above are structurally unchanged.

**Edge cases**:

- If your termination mechanism records anything on the first-error slot, it
  must go through `get_or_insert` too — a `set`/`replace` anywhere on that slot
  breaks the latch.
- The latch is shared with the projection-push and tick/frame failure arms. Do
  not introduce a second error slot; one canonical first-error slot is the
  design.

### T003 — In-module tests: latch precedence and termination

**Purpose**: Pin both properties at the unit layer, where they can be proven
without a live webview, and prove the test can actually fail.

**Steps**:

1. Extend the in-module test module around `window.rs:796-830`. Follow its
   existing style: synthetic `PageSignal` values driven through the handling
   functions, no tauri runtime.
2. **Latch precedence test**: record a `PageRenderFailed` on the slot, then
   record a `WindowClose` (or drive the code path that would), and assert the
   surfaced error is the `PageRenderFailed` — matched on the typed variant, not
   on a formatted string.
3. **Termination test**: assert that the exhausted-retry path produces the
   termination decision rather than a plain return. What exactly you assert
   depends on the mechanism T001 chose; assert on the *decision*, at the
   narrowest seam that makes it observable, not on a side effect three layers
   away.
4. **Add the forced-failure test seam** WP04 needs. Mirror the
   `CREST_WEBVIEW_PAGE` precedent exactly:
   - `#[cfg(debug_assertions)]` on the constant, the reader, and the branch that
     consults it, so a release binary contains no trace of it;
   - a doc comment naming it a debug-only test seam, why it exists (the
     forced double-close-failure proof, FR-001), and that release builds compile
     it out;
   - an in-module test mirroring `window.rs:877-885` asserting the seam is
     `cfg(debug_assertions)`-gated.
   Prefer this over a runtime flag — IC-01 says so explicitly and a runtime flag
   in a release binary is a product surface change (C-001).
5. **Record a disable-the-mechanism probe** in your WP report: revert T001's
   exit edge locally (record the typed error, return normally), confirm the
   termination test fails, restore the edge, confirm it passes. Paste both
   outcomes. A proof that cannot fail is not a proof.

**Files**: `src/shell/webview/window.rs` (test module).

**Validation**: `cargo test --lib` green;
`cargo test --test webview_projection_shell --test component_vocabulary --test
component_composition` green (headless — these must be unchanged by your work);
the probe recorded.

**Edge cases**:

- Do not weaken or delete any existing assertion in the module's test block to
  make room (NFR-002). Add; do not rewrite.
- If the seam needs to be reachable from `tests/webview_projection_shell.rs`,
  make it reachable the way the page override is — the same visibility and the
  same `cfg` gating. Do not make anything `pub` that only your own tests need.

## Branch Strategy

- **Planning base branch**: `feat/shell-hygiene`
- **Merge target branch**: `feat/shell-hygiene`

Planning artifacts for this mission were generated on `feat/shell-hygiene`.
During implementation this WP works on its own lane branch and merges back into
`feat/shell-hygiene` unless the human explicitly redirects the landing branch.

### Gate context — read this, it prevents three known failures

1. **Commit ONLY your owned production/test files on the lane branch.** Your
   owned file is `src/shell/webview/window.rs`. The move-task gate **REFUSES**
   commits touching `kitty-specs/` on a lane branch. Review artifacts, evidence,
   and status files go on `feat/shell-hygiene` from the primary checkout — not
   from your lane. If you find yourself wanting to `git add kitty-specs/...`,
   stop: that is the gate you are about to trip.
2. **Do not park waiting for a background notification.** If you launch anything
   in the background, use bounded foreground waits and check the run state
   yourself. Never end a turn with "waiting for the build to finish".
3. **Run `cargo test --lib` and the headless suites before requesting review.**
   The headless set is
   `cargo test --test webview_projection_shell --test component_vocabulary
   --test component_composition`. Paste the results.
4. **NFR-001 forbids product behavior change; NFR-002 forbids weakening any
   proof.** No frozen baseline, threshold, skip list, or assertion may be
   loosened. If a proof fails, the fix is in your code, never in the proof.

## Definition of Done

- [ ] The exhausted-retry branch reaches termination by a route that does not
      depend on the window closing; all four call sites are consistent with it.
- [ ] The recorded first typed error is surfaced by the existing
      post-`run_return` path, unchanged and un-duplicated.
- [ ] Retry-once, the typed `WindowClose` payload, and the `get_or_insert` latch
      are structurally unchanged; a prior `PageRenderFailed` still wins.
- [ ] The `Destroyed` arm and the ordinary teardown path are untouched.
- [ ] A `cfg(debug_assertions)` forced-failure seam exists, is documented, is
      proven compiled-out of release by an in-module test, and is usable by
      WP04 T013.
- [ ] In-module tests pin latch precedence and termination; the
      disable-the-mechanism probe is recorded in the WP report with both
      outcomes.
- [ ] `cargo test --lib` and the three headless suites are green.
- [ ] `cargo clippy --all-targets` clean.
- [ ] No file outside `src/shell/webview/window.rs` is modified.
- [ ] Net effect on `src/` line count is reported (NFR-003 is a mission-level
      goal; this WP may add lines — say how many).

## Risks / Reviewer Guidance

- **The single most likely wrong fix is a panic or an `std::process::exit`.**
  D1 rejects the panic explicitly. An `exit` bypasses the typed surfacing path
  entirely, which is the same failure this WP exists to close, inverted. A
  reviewer should check the mechanism against D1's two rejected alternatives
  first.
- **The second most likely wrong fix is a mechanism that only covers one call
  site.** The `PageSignal::RenderError` arm is the one RISK-3 named, but the
  projection-push, tick/frame, and `CloseRequested` arms call the same helper.
  Check all four.
- **Latch inversion is silent.** If a reviewer can construct an ordering where
  the `WindowClose` error is what the operator sees despite an earlier
  `PageRenderFailed`, the WP is not done. T003's precedence test must make that
  impossible.
- **Check the seam is really compiled out.** `grep` the release build path: the
  forced-failure constant, its reader, and its branch must all carry
  `#[cfg(debug_assertions)]`, exactly like `PAGE_OVERRIDE_ENV`.
- **Watch for scope creep into teardown.** Any diff hunk inside the `Destroyed`
  arm or the ordinary `CloseRequested → Destroyed → ExitRequested` sequence is
  out of scope and should be questioned.
- **Ordinary single-close-failure behavior** is the regression a reviewer should
  actively try to break: one failing close followed by a succeeding retry must
  record nothing and terminate nothing.
