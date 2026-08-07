---
wp_id: WP01
reviewer_agent: reviewer-renata
cycle_number: 1
verdict: approved
mission_slug: shell-hygiene-01KZD0KR
reviewed_at: "2026-08-07T03:40:35Z"
affected_files:
  - path: src/shell/webview/window.rs
---

# WP01 review — cycle 1: APPROVED

Reviewed commit `70c0261` ("fix(WP01): terminate the loop when the window close
fails twice") on `kitty/mission-shell-hygiene-01KZD0KR-lane-a` (lane HEAD
`f798bb1`, a merge of the mission branch carrying `kitty-specs/` only).

`git log --no-merges --name-only kitty/mission-shell-hygiene-01KZD0KR..HEAD`
returns exactly one commit touching exactly one file:
`src/shell/webview/window.rs`, +284/−15 (net +269 = production +107, tests
+162). That matches `owned_files` exactly. No `projection_channel.rs` (WP02),
no `frame_stream.rs`/`mod.rs` (WP03), no `tests/webview_projection_shell.rs`
(WP04).

Authorities checked: `spec.md` FR-001, FR-002, NFR-001, NFR-002, C-001 and the
US1 acceptance scenarios; `research.md` D1 including both rejected
alternatives; the WP prompt's Definition of Done.

Nothing in the implementer's report was taken on trust. Every claim below was
re-derived mechanically in the lane worktree, including two independent live
runs of the shipped binary and one falsification run with the fix removed.

---

## 1. The exit-code-0 decision — VERIFIED SOUND (structural, not incidental)

The implementer chose `REQUESTED_EXIT_CODE = 0` and argued the recorded typed
error is the terminal outcome. I traced the whole path rather than accepting
the argument.

**The post-`run_return` path reads the error slot before it reads the exit
code, unconditionally** (`src/shell/webview/window.rs:677-686`):

```rust
if let Some(error) = runtime_error.borrow_mut().take() {
    return Err(error);
}
if exit_code == 0 {
    Ok(())
} else {
    Err(WindowError::new(format!(
        "tauri event loop exited with code {exit_code}"
    )))
}
```

And `CloseOutcome::RetriesExhausted` is *only* produced on the same branch that
performs the `get_or_insert` (`window.rs:397-402`). The slot is therefore
provably `Some` on every path that can reach `handle.exit(REQUESTED_EXIT_CODE)`
— `get_or_insert` either latches an earlier error or inserts the `WindowClose`.
There is no interleaving in which the exhausted-retry path fires with an empty
slot. **`exit_code` is unreachable as a return determinant on this path.** The
value 0 is never read.

Downstream: `AppWindow::run` returns `Err(WindowError)` →
`StandaloneApplication::run_live_demo_scene` → `main() -> anyhow::Result<()>`
(`src/bin/crest_synth.rs:76-79`) → Rust's `Termination` for `Result` prints the
error and returns `ExitCode::FAILURE`. Process status 1.

**Two independent facts strengthen this beyond the implementer's own
reasoning:**

1. The requested code never reaches `run_return`'s return value at all.
   `tauri-runtime-wry-2.11.4/src/lib.rs:4358-4368` handles
   `Message::RequestExit(code)` by firing `RunEvent::ExitRequested { code }`
   and then setting `*control_flow = ControlFlow::Exit` — it discards `code`.
   `tao-0.35.3/src/event_loop.rs:177` defines `ControlFlow::Exit` as
   `ExitWithCode(0)`. So `app.run_return(...)` returns `0` regardless of what
   was passed to `handle.exit`. Passing `1` would have changed nothing.
2. My own live probe (§6, PROBE A) shows a real `handle.exit(0)` producing
   process status **1** with the typed error on stderr.

**Independent verdict: the exit-code-0 choice is safe, and it is safe
structurally.** There is no path on which 0 escapes as the process status while
a typed error is recorded, because the error slot is read first and is
guaranteed non-empty.

One residual, recorded for completeness — see finding F2 below: the *only*
construct anywhere in this path that could emit process status 0 with an error
recorded is tauri's own fallback inside `AppHandle::exit`, and it is
unreachable from where WP01 calls it.

## 2. `handle.exit` panic-safety — CLAIM VERIFIED against vendored source

The implementer's cited lines check out in
`~/.cargo/.../tauri-runtime-wry-2.11.4/src/lib.rs`:

- `:2748` `fn request_exit(&self, code: i32)` sends
  `Message::RequestExit(code)` through `self.context.proxy.send_event(...)`,
  with the in-source note *"request_exit cannot use the `send_user_message`
  function because it accesses the event loop callback."*
- `:3342` `Message::RequestExit(_code) => panic!("cannot handle RequestExit on
  the main thread")` lives inside `handle_user_message`, which is the
  **fallthrough** arm of the `Event::UserEvent(message)` match.
- `:4358` the `Event::UserEvent` match has an explicit
  `Message::RequestExit(code)` arm placed *before* that fallthrough. So
  `RequestExit` never reaches `handle_user_message`.

**The claim is correct: the `:3342` panic is unreachable for `RequestExit`.**

I also checked the re-entrancy question the implementer did not raise.
`handle.exit` is called from inside the `run_return` callback. It does not
invoke the callback re-entrantly — it posts to the tao proxy queue and returns.
The callback is re-entered later, from the loop, with
`RunEvent::ExitRequested { code: Some(0), api }`. Our closure's `_ => {}` arm
drops the `ExitRequestApi` (and its `tx`) without calling `prevent_exit()`, so
`rx.try_recv()` yields `Err(Empty)`, `should_prevent` is `false`, and
`ControlFlow::Exit` is set. Verified end to end, and confirmed empirically by
PROBE A.

**Independent verdict: no panic. The fix does not trade a hang for a panic.**

## 3. FR-002 — nothing else changed — VERIFIED BYTE-LEVEL

I did not rely on "no hunk between 455–565". I extracted the base blob and
diffed the files directly.

`diff base.rs new.rs` reports exactly four change ranges plus one append:

```
41,42c41,45      module doc sentence
81a85,105        new constants (CLOSE_FAILURE_OVERRIDE_ENV, REQUESTED_EXIT_CODE)
317,320c341,427  close-helper doc comment
330,338c437,445  close-helper body
842a950,1111     new tests appended
```

The complete set of lines deleted from the base file (`diff ... | grep '^<'`)
is 15 lines: 2 module-doc lines, the 4-line old close-helper doc comment, and
the 9-line old close body. **Nothing else in the file was removed.**

Positive identity checks:

- base `[343-839]` == new `[450-946]`, byte-identical. This region contains the
  entire `run_return` callback (base 455–672), which is to say the `Destroyed`
  arm, all four close call sites, the `MainEventsCleared` body, and the
  post-loop surfacing block.
- base `[294-316]` == new `[318-340]`, byte-identical: `record_render_failure`
  and its full doc comment are untouched.

Behavior checks:

- **Retry-once**: `close_with_retry` (`window.rs:388-404`) issues `attempt()`
  once, returns `Teardown` on success, otherwise issues it exactly once more.
  Two attempts on the failure path, never three, no loop, no backoff. Attempt
  counts are pinned by assertion in
  `a_close_that_fails_twice_terminates_and_records_the_typed_failure`.
- **Typed error verbatim**: `WebviewShellError::WindowClose(retry_failure)`,
  moving the tauri error unchanged. No new variant, no stringification, no
  wrapper.
- **Latch**: still `first_error.borrow_mut().get_or_insert(...)` on the one
  shared `RefCell<Option<WindowError>>`. No `set`/`replace`/`insert` anywhere
  on that slot; no second error slot introduced.
- **Precedence**: `a_recorded_render_failure_still_wins_over_a_later_close_failure`
  records `PageRenderFailed`, then drives a double-failing close, asserts the
  slot holds `PageRenderFailed` and `assert_ne!`s it against `WindowClose`.
  The expectation is constructed from the typed variants, not written as a
  formatted literal, which is the strongest form available given `WindowError`
  is a `String` newtype (`src/shell/app_window.rs:26-29`) — a pre-existing
  property of the slot, not something WP01 introduced.
- **NFR-002**: assertion count went 42 → 66 by pure addition. No existing
  assertion was reworded, weakened, or deleted.

## 4. The seam cannot arm in production — VERIFIED, INCLUDING BINARY EVIDENCE

Gating (`window.rs:96-97`, `365-373`) mirrors the `CREST_WEBVIEW_PAGE`
precedent (`window.rs:82-83`, `276-282`) structurally:

```rust
#[cfg(debug_assertions)]
const CLOSE_FAILURE_OVERRIDE_ENV: &str = "CREST_WEBVIEW_FORCE_CLOSE_FAILURE";

fn forced_close_failure() -> Option<tauri::Error> {
    #[cfg(debug_assertions)]
    if std::env::var_os(CLOSE_FAILURE_OVERRIDE_ENV).is_some() { ... }
    None
}
```

In a release build the constant and the environment read are both absent and
the function is `{ None }`. The substitution arm at the call site
(`window.rs:438-441`) is not itself `cfg`-gated — but it is unreachable by
construction, exactly as `resolve_page_source`'s unconditional call site is in
the precedent. No path in a release build can produce `Some`.

Evidence:

- `cargo test --release --lib shell::webview::window` — **12 passed, 0 failed**,
  including `release_builds_compile_the_forced_close_failure_seam_out ... ok`.
  The debug-only test is absent from the release run (12 tests vs 13 in debug),
  which is itself the compile-out proof.
- `strings target/release/crest-synth | grep -c CREST_WEBVIEW_FORCE_CLOSE_FAILURE`
  → **0**. The same grep on the debug binary → **1**. The shipped release
  binary contains no trace of the seam.
- Nothing was made `pub`: the diff adds zero lines matching `pub `.
  `CloseOutcome`, `close_with_retry`, `forced_close_failure`,
  `CLOSE_FAILURE_OVERRIDE_ENV`, and `REQUESTED_EXIT_CODE` are all private.
- WP04 reachability confirmed: `tests/webview_projection_shell.rs` launches via
  `env!("CARGO_BIN_EXE_crest-synth")` (`:995`, `:3226`, `:3345`), which resolves
  to the same profile the test binary was built with — debug under `cargo test`.
  The seam is reachable from the harness the same way `CREST_WEBVIEW_PAGE`
  already is. No coordination gap for WP04.

## 5. NFR-001 — no behavior change — VERIFIED BY MY OWN LIVE RUN

Not taken from the implementer's report. PROBE B below is my own run of the
shipped binary with the seam disarmed:

```
exit 0, 62s, empty stderr
CREST_LIVE_SUMMARY live demo complete: 105/105 editable parameters,
3/3 engine transitions, 2038 qualifying shell frames, 114 checkpoints,
15714 events, 0 dropped, ... fallbacks=0, callbackAllocations=0,
callbackDestructions=0, cleanup=true, activeNotes=0
```

Identical shape to the implementer's P1. A normal run produces no new failure,
takes no new branch (the first close succeeds → `Teardown` → no exit request),
and drops nothing.

## 6. Independent probe reproduction

Debug binary built from the lane source at `70c0261`. Every run bounded by a
hard deadline with SIGKILL.

| Probe | Seam | Exit edge | Elapsed | Killed | Process status |
|---|---|---|---|---|---|
| A | armed | present | 64s | no | **1** |
| B | disarmed | present | 62s | no | **0** |
| C | armed | **reverted** | 150s | **yes (SIGKILL)** | 137 |

**PROBE A** — `CREST_WEBVIEW_FORCE_CLOSE_FAILURE=1 crest-synth
--demo-live-graphical-shell`. The scene ran to completion first (105/105
parameters, 3/3 engine transitions, 2066 qualifying frames, 0 dropped, 0
callback allocations), then the ordinary end-of-scene close was forced to fail
twice. stderr:

```
Error: live observable demo execution failed

Caused by:
    0: application window failed: webview window close failed after one retry: forced close failure (CREST_WEBVIEW_FORCE_CLOSE_FAILURE)
    1: webview window close failed after one retry: forced close failure (CREST_WEBVIEW_FORCE_CLOSE_FAILURE)
```

This is the SC-001 signature: `handle.exit(0)` was called, and the process
still ended **nonzero with the typed error surfaced**.

**PROBE C** — same command, with `handle.exit(REQUESTED_EXIT_CODE)` replaced by
`let _ = (outcome, REQUESTED_EXIT_CODE);` and the binary rebuilt. The scene
again completed normally (2067 qualifying frames, `CREST_LIVE_SUMMARY`
printed) and the process then **hung indefinitely** — no error, no exit,
killed at the 150s deadline. That is RISK-3 reproduced exactly: a correctly
recorded fatal error that never reaches the operator. Restored afterward;
`git status` in the lane worktree is clean.

The fix is load-bearing and the proof is falsifiable.

---

## Independent test runs (lane worktree, `CARGO_TARGET_DIR` shared with primary)

| Command | Result |
|---|---|
| `cargo test --lib` | **634 passed, 0 failed**, 1 ignored |
| `cargo test --lib shell::webview::window` | **13 passed, 0 failed** (3 new) |
| `cargo test --release --lib shell::webview::window` | **12 passed, 0 failed** |
| `cargo clippy --all-targets` | **clean**, no warnings |
| `cargo test --test webview_projection_shell` | `CREST_ACCEPTANCE webview_projection_shell passed` (headless; 5 live sections skipped for `CREST_WEBVIEW_TESTS=1` absent, as designed) |
| `cargo test --test component_vocabulary` | **11 passed, 0 failed** |
| `cargo test --test component_composition` | **15 passed, 0 failed** |

---

## Findings — none blocking

### F1 (P1, must be discharged by WP04) — no test at any layer covers the exit edge itself

Proven, not suspected. With `handle.exit(REQUESTED_EXIT_CODE)` deleted (PROBE
C's build), `cargo test --lib shell::webview::window` still reports **13
passed, 0 failed**. The in-module tests exercise `close_with_retry`, the pure
helper, and assert on `CloseOutcome::RetriesExhausted` — the *decision*. The
`if outcome == CloseOutcome::RetriesExhausted { handle.exit(...) }` line in
`close_window_once_with_retry` is covered by nothing.

This is not a WP01 defect: T003 step 3 explicitly asked for the decision to be
asserted "at the narrowest seam that makes it observable, not on a side effect
three layers away," and the exit edge cannot be driven without a tauri runtime.
WP01's unit layer is correct as specified. But it does mean **the only thing
standing between a future edit and a silent regression to RISK-3 is WP04 T013's
live proof.** WP04 must drive the armed seam end-to-end and assert nonzero exit
with the typed stderr line — my PROBE A is that signature, and it works. If
WP04 does not land that, the mission closes RISK-3 with no regression guard.

### F2 (P2, informational) — the one construct that could emit exit 0 with an error recorded

`AppHandle::exit` (`tauri-2.11.5/src/app.rs:574-580`) is not a pure request:

```rust
pub fn exit(&self, exit_code: i32) {
    if let Err(e) = self.runtime_handle.request_exit(exit_code) {
      log::error!("failed to exit: {}", e);
      self.cleanup_before_exit();
      std::process::exit(exit_code);   // <-- with exit_code == 0
    }
}
```

If `request_exit` fails, tauri calls `std::process::exit(0)`, which bypasses
`run_return`, bypasses the first-error slot, and bypasses `main`. That is
precisely the silent-failure class this mission exists to close.

**It is unreachable from WP01's call site**, and I confirmed why:
`request_exit` fails only when `proxy.send_event` returns `EventLoopClosed`
(`tauri-runtime-wry-2.11.4:2748-2755`), and tao's macOS
`EventLoopProxy::send_event` (`tao-0.35.3/src/platform_impl/macos/event_loop.rs:343-354`)
fails only when the receiver has been dropped — i.e. when the event loop is
gone. WP01 calls `handle.exit` from *inside* the running `run_return` callback,
where the loop is by definition alive. PROBE A confirms the request path is
taken.

No change required. Recorded because it is the single assumption the fix rests
on that is not visible in `window.rs`, and because a future caller of
`close_window_once_with_retry` from outside the event-loop callback would
silently acquire an exit-0 path. Worth one sentence in the helper's doc
comment if the mission owner wants it belt-and-braces.

### F3 (P3, doc accuracy) — `REQUESTED_EXIT_CODE`'s rationale is true but understated

`window.rs:99-104` says the code is zero so it "must not become a second,
competing failure signal." Accurate in effect, but the stronger fact is that
the requested code never reaches `run_return`'s return value at all — wry
discards it and sets `ControlFlow::Exit`, which tao defines as
`ExitWithCode(0)`. A future reader may believe `REQUESTED_EXIT_CODE` is a live
knob; it is inert. Optional one-line correction.

### F4 (P3, quality) — `std::env::set_var` inside a parallel test harness is newly introduced here

`window.rs:1087`, `:1093`, `:1107`, `:1109` mutate the process environment from
test threads. The base file contains **no** `set_var`/`remove_var` anywhere —
the `CREST_WEBVIEW_PAGE` precedent deliberately avoids it by calling
`read_override_page` directly (base `window.rs:867-874`). Meanwhile
`resolve_page_source` (`window.rs:277`) calls `std::env::var_os` and is
exercised by `page_resolution_serves_the_projection_page_without_an_override`
in the same 634-test binary, so a concurrent `setenv`/`getenv` pair is live.
`setenv` is not thread-safe on macOS or glibc; Rust 2024 marks `set_var`
`unsafe` for exactly this reason.

Low probability, test-only, and it did not flake across the four full runs I
did. But it is a new hazard, not an inherited one. The `set_var` in
`release_builds_compile_the_forced_close_failure_seam_out` (`:1107`) is also
pure ceremony — the compile-time gating is the actual proof and the assertion
holds without touching the environment.

### F5 (mission-level, not WP01's) — three `cargo fmt --check` diffs, third WP to report them

Verified pre-existing by checking out the base tree into the lane worktree and
re-running:

| | at base | at WP01 HEAD |
|---|---|---|
| `src/shell/webview/window.rs` | `:307` | `:331` (same `record_render_failure` block, shifted +24) |
| `tests/webview_projection_shell.rs` | `:618` | `:618` |
| `tests/webview_projection_shell.rs` | `:2684` | `:2684` |

**WP01 adds zero new fmt diffs.** Not held against this WP. This is now the
third WP to report the identical three; the mission owner should land them
before accept.

---

## Definition of Done

- [x] Exhausted-retry branch reaches termination by a route independent of the
      window closing; the edge lives in the shared helper so all four call
      sites get it with no call-site edits.
- [x] Recorded first typed error surfaced by the existing post-`run_return`
      path, unchanged and un-duplicated (PROBE A stderr).
- [x] Retry-once, typed `WindowClose` payload, `get_or_insert` latch
      structurally unchanged; prior `PageRenderFailed` still wins.
- [x] `Destroyed` arm and ordinary teardown byte-identical.
- [x] `cfg(debug_assertions)` seam exists, documented, proven compiled out
      (release test + zero strings in the release binary), usable by WP04.
- [x] In-module tests pin latch precedence and the termination decision.
- [x] Disable-the-mechanism probe: reproduced independently as PROBE C, both
      outcomes recorded above. (Note: no WP report file was left in the WP
      directory; this review is the durable record. See F1 for what the
      unit-layer probe does and does not cover.)
- [x] `cargo test --lib` and the three headless suites green.
- [x] `cargo clippy --all-targets` clean.
- [x] No file outside `src/shell/webview/window.rs` modified.
- [x] Net `src/` effect reported: +269 lines (+107 production, +162 tests).
      NFR-003 is mission-level; this WP legitimately adds.

## Verdict

**APPROVED.** The mechanism is D1's decision, not either rejected alternative:
no panic, no inline surfacing. The exit-code-0 choice is structurally safe —
the error slot is read first and is provably non-empty on every path that can
request the exit. `handle.exit` cannot panic from the event-loop callback. The
one construct that could emit exit 0 with an error recorded lives in tauri and
is unreachable from here. FR-002's three behaviors are byte-level unchanged,
NFR-002 is satisfied by pure addition, and NFR-001 is confirmed by my own clean
live run. The fix is falsifiable and I falsified it.

F1 is the one thing that must not be dropped: **WP04 T013 is now the sole
regression guard on the exit edge.**
