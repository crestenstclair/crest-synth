---
wp_id: WP03
reviewer_agent: reviewer-renata
cycle_number: 1
mission_slug: shell-hygiene-01KZD0KR
verdict: approved
reviewed_at: "2026-08-07T04:35:00Z"
affected_files:
  - path: src/shell/component_vocabulary.rs
  - path: src/shell/webview/frame_stream.rs
  - path: src/shell/webview/mod.rs
  - path: src/testing/live_demo_runner.rs
---

# WP03 review — cycle 1: approved

Reviewed commit `8937281` on `kitty/mission-shell-hygiene-01KZD0KR-lane-c` (lane HEAD
`3730cb9` is a merge of the mission branch; merge-base is the mission tip, so the
three-dot lane diff isolates exactly WP03's contribution). Authorities checked:
`spec.md` FR-004, FR-005, NFR-002, NFR-003, C-002, C-004; `research.md` D3 and D4;
the crest-spec retirement at `7c7f1cf`.

Every claim below was verified mechanically against the diff and the pre-deletion tree,
not taken from the implementer's report.

## 1. The `step_index` field survived — verified

The `live_demo_runner.rs` diff is a **single hunk removing 4 lines** (the accessor's
three lines plus the preceding blank). Nothing else in the file changed, so every other
byte is identical by construction.

- `grep -c "step_index" src/testing/live_demo_runner.rs`: **16 before → 14 after**. The
  two removed occurrences are the accessor signature and its body.
- The 14 surviving sites are exactly D3's list: l.58 (field decl), 126 (initializer),
  174, 295, 301, 314, 323, 328, 356, 385, 469, 482, 487, 577.
- `grep -rn "pub const fn step_index" src/` → **0 hits**.
- The neighbouring accessors `shell_coverage` and `is_aborted` were left in place, per
  T009 step 4. No widening.

## 2. NFR-002 — no proof removed to accommodate a deletion

The pre-deletion tree had exactly 5 tests in `frame_stream.rs` and exactly 3
deleted-symbol references inside `component_vocabulary.rs`'s test module (l.1187, 1190,
1424). All are accounted for below; nothing else was touched.

### Retired (subject deleted)

**`the_timeout_path_is_a_typed_error_naming_the_awaited_identity`** — every assertion
targets `await_qualifying`'s timeout path, `FrameAwaitError::Timeout`, and that error's
`Display`. All three are deleted. Legitimate retirement.

*One residue, recorded as a follow-up rather than a defect:* this test transitively
exercised `impl fmt::Display for FrameExpectation` (via `assert!(text.contains("generation 11"))`
and `"state-11"`). That impl **survives** at `frame_stream.rs:107-115` and now has no
assertion. I checked whether this is a surviving guarantee that lost its proof: nothing
in `src/`, `tests/`, or `webview-page/` formats a `FrameExpectation` — the only consumer
was the deleted error's `Display`. So it is unproven *and* unconsumed, which puts it in
the same class as the Condvar (item 6), not in the class of a lost behavioral guarantee.
See "Follow-ups".

**`an_empty_composition_intent_records_nothing`** — all three assertions name
`CompositionIntent`, which is deleted. Zero residue. Clean retirement.

### Narrowed (surviving assertions preserved)

**`a_recorded_qualifying_observation_satisfies_poll_and_await` → `..._satisfies_poll`** —
only the trailing 3-line already-satisfied-await block was removed. The `poll` assertion
above it is byte-identical. Nothing lost.

**`focus_is_legible_without_color`** — only `assert_eq!(CURSOR_GLYPH, ">");` removed.
Both `draws_cursor(ComponentState::Focused)` and `draws_cursor(ComponentState::Adjusting)`
survive, as does the leading comment verbatim. Exactly T011 steps 3-4. The non-color
focus signal is still proven.

**`a_frame_recorded_from_another_thread_wakes_the_await` →
`a_frame_recorded_through_one_clone_is_visible_through_another`** — the narrowing flagged
for scrutiny. **The relocated proof is valid.** The old test incidentally proved that
clones share one underlying stream, which is a live doc claim on the surviving
`QualifyingFrameStream` ("Clones share one underlying stream", l.124-126). The new test
constrains that behavior:

- `record` still happens on `stream.clone()` inside a spawned thread;
- `recorder.join()` establishes happens-before, so the record is complete;
- `stream.poll(&expectation).expect("the clone's recording is visible to the original handle")`
  then queries the **original** handle.

If clone-sharing broke — if `Clone` stopped sharing the `Arc<StreamShared>` and deep-copied
the `VecDeque` instead — the original handle's deque would be empty, `poll` would return
`None`, and `.expect` would panic. The test **would** fail. It is also strictly *more*
deterministic than the original, which raced a 10-second await against the recorder
instead of joining first. What was lost is the condvar *wake* semantics, which is the
deleted subject.

This claim additionally carries independent redundant proof at
`src/shell/webview/window.rs:596` (`clones_of_one_window_share_one_qualifying_frame_stream`),
a WP01-owned file that was not touched and whose suite passes.

**Conclusion: no surviving behavioral guarantee lost its proof.** No baseline, threshold,
skip list, or assertion was loosened.

## 3. C-002 — declaration before deletion

`7c7f1cf` (authored during the crest-spec phase, before this deletion) retired the
mechanism clause from exactly the three prose sites D4 names:
`requirement.component_state_ownership_boundary`, its matching invariant in
`proof/invariants.yaml`, and `requirement.configurable_control_family`. The passivity
boundary was **kept and sharpened** — the requirement now reads "none reaches AppState
directly or converts an input into a semantic action".

The code deletion matches that scope precisely: only the returned-value mechanism went.
No surviving declaration lacks an implementation:

- `component_state_ownership_boundary` → passivity, implemented by the module and proven
  by `tests/component_composition.rs::no_component_owns_caches_or_dispatches_application_state`
  (untouched, passing).
- `configurable_control_family` → `ComponentControl` and `control_for`, both surviving and
  proven by `tests/component_vocabulary.rs` (11 passing).

The module doc rewrite states the surviving boundary rather than leaving a hole, and cites
`tests/component_composition.rs` as its proof. The `Realizes` block is intact.

**The implementer did not edit the crest-spec.** The lane diff touches four files, none
under `.kittify/crest-spec/` and none under `kitty-specs/`.

## 4. Scope — `AdjustDirection` / `AdjustGranularity` are not over-deletion

At `8937281^`, a repo-wide grep across `src/`, `tests/`, and `webview-page/` returned
**exactly four hits** for both types combined: the two enum declarations, and the two
field types inside `ControlIntent::AdjustRequested`. Zero external callers, zero
re-exports. They were structurally part of the ControlIntent family and had no
independent utility.

This is also not an implementer judgement call — T010 step 2 names both explicitly as
example support types to delete after grep verification. Correct.

## 5. File ownership — clean

`git diff --name-only kitty/mission-shell-hygiene-01KZD0KR...HEAD` returns exactly the
four owned files. `tests/` is entirely unmodified. Confirmed specifically:

- `tests/component_composition.rs:1742` still reads `"a component returns ControlIntent
  and converts nothing"` — the expected residual, owned by **WP05 T018**, correctly left
  alone.
- `src/shell/webview/window.rs` (WP01), `projection_channel.rs` (WP02),
  `tests/webview_projection_shell.rs` (WP04), `src/testing/component_gallery_scene.rs`
  (WP05) all untouched.

The four verification greps are clean: `await_qualifying|FrameAwaitError` → 0;
`CURSOR_GLYPH` → 0; `pub const fn step_index` → 0; the ControlIntent family → 1, the
WP05-owned assertion message above.

## 6. Vestigial machinery — leaving it was correct

`StreamShared.arrived: Condvar` (l.120) and its `notify_all()` (l.154) now have no waiter;
`grep` for `.wait`/`wait_timeout` across `src/shell/webview/` returns nothing.

Leaving it is the right call: it is not a dangling reference, it is private to the module,
it compiles, clippy does not flag it (the field is still written), and it is not in D3's
table. Chain-deleting it would violate C-004, and T012's edge case explicitly directs the
implementer to *report* newly-dead machinery rather than keep deleting. It was reported.
Recorded as a follow-up below.

## Independent verification results

All run by the reviewer in the lane worktree.

| Check | Result |
|---|---|
| `cargo test --lib` | **629 passed, 0 failed**, 1 ignored |
| `cargo test --test component_composition` | **15 passed, 0 failed** — unedited |
| `cargo test --test component_vocabulary` | **11 passed, 0 failed** — unedited |
| `cargo test --test webview_projection_shell` | **passed** (5 `CREST_WEBVIEW_SKIP` without `CREST_WEBVIEW_TESTS=1`, as designed) |
| `cargo test` (all targets) | **29 suites, 725 passed, 0 failed** |
| `cargo clippy --all-targets` | **clean — zero warnings**, no `dead_code` |
| `cargo doc --no-deps` | 3 warnings, all pre-existing and WP-external (see below); **zero broken links to any deleted symbol** |

`component_vocabulary` and `component_composition` pass **unedited** — neither file appears
in the lane diff at all.

**NFR-003, measured independently:** `git diff --stat kitty/mission-shell-hygiene-01KZD0KR...HEAD -- src/`
gives **42 insertions / 298 deletions = net −256 lines** in `src/`, with `tests/`
unchanged. Matches the reported figure.

**`cargo doc` warnings** are in `src/shell/webview/input_capture.rs` and
`src/testing/component_gallery_scene.rs`, both present unchanged at the mission base and
neither a WP03 file. Not WP03's doing.

**`cargo fmt --check` (known non-issue) — confirmed pre-existing.** Three diffs:
`src/shell/webview/window.rs:307` (WP01) and `tests/webview_projection_shell.rs:618,2684`
(WP04). I extracted both files at the mission base commit and ran `rustfmt --check` on
those copies: identical failure counts (1 and 2 respectively). WP03 modified neither file.
**All four WP03-owned files are fmt-clean.** Flagged for the mission owner to land under
WP01/WP04 — not chargeable to WP03.

## Follow-ups (not blocking)

1. **`StreamShared.arrived: Condvar` and its `notify_all()`** are now waiterless. Correctly
   left per C-004; worth a future hygiene pass.
2. **`impl fmt::Display for FrameExpectation`** (`frame_stream.rs:107-115`) is now both
   unproven and unconsumed — its only caller was the deleted `FrameAwaitError`'s `Display`.
   Same disposition as the Condvar: out of D3's table, so out of this mission's scope.

Both are consequences of an honest deletion, not defects in it. Neither is a dangling
reference and neither breaks a public promise.

## Verdict

**Approved.** The highest-risk item (the `step_index` field) is provably intact. Every
deleted test traces to a deleted subject, and the one incidental proof on a surviving type
was relocated into an assertion that genuinely constrains the behavior and is more
deterministic than what it replaced. C-002 ordering holds, the deletion matches the
retirement, no surviving declaration is left without an implementation, the scope matches
D3 plus the support types T010 authorized, and file ownership is exact.
