---
work_package_id: WP03
title: Dead code removal
dependencies: []
requirement_refs:
- FR-004
- FR-005
- NFR-002
- NFR-003
planning_base_branch: feat/shell-hygiene
merge_target_branch: feat/shell-hygiene
branch_strategy: Planning artifacts for this mission were generated on feat/shell-hygiene. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/shell-hygiene unless the human explicitly redirects the landing branch.
base_branch: kitty/mission-shell-hygiene-01KZD0KR
base_commit: 575a721f392cf2cbc3e924cb4ef523fc4ab5486d
created_at: '2026-08-07T03:02:30.900550+00:00'
subtasks:
- T008
- T009
- T010
- T011
- T012
history:
- '2026-08-06: authored from plan IC-03, research D3/D4, crest-spec assets WebviewShellModules/ShellContextModules/TestingContextModules, mission-review RISK-5'
agent_profile: implementer-ivan
authoritative_surface: src/shell/
create_intent: []
execution_mode: code_change
owned_files:
- src/shell/webview/frame_stream.rs
- src/shell/webview/mod.rs
- src/shell/component_vocabulary.rs
- src/testing/live_demo_runner.rs
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

Delete the four items `research.md` D3 verified dead by caller search, so that
a reader of the shell finds no public API without a caller and no crest-spec
declaration without an implementation. "Declared" and "built" mean the same
thing again.

**C-002 is already satisfied.** The crest-spec retirement was authored and
committed at `7c7f1cf`, *before* this deletion — declaration first, deletion
second. You are executing the second half of an already-recorded decision. Do
not author any further crest-spec change, and do not edit `.kittify/crest-spec/`.

**Authorities** (cited, not restated — read them):

- `spec.md` FR-004, FR-005, NFR-002, NFR-003, C-001, C-002; User Story 3 and its
  four acceptance scenarios.
- `plan.md` IC-03.
- `research.md` **D3** — the caller-search table naming each dead item, its
  location, and its (zero) external callers; and the `CURSOR_GLYPH` reasoning.
  **D4** — why the control-intent retirement is a boundary re-authoring rather
  than a deletion of a guarantee: the *mechanism* clause retires, the
  *passivity boundary* survives and is still proven by
  `tests/component_composition.rs`.
- Crest-spec assets `WebviewShellModules`, `ShellContextModules`,
  `TestingContextModules`. Read via `spec-kitty crest-spec context`.

**Hard boundaries**:

- Do **not** edit `src/shell/webview/window.rs` (WP01),
  `src/shell/webview/projection_channel.rs` (WP02),
  `tests/webview_projection_shell.rs` (WP04), `tests/component_composition.rs`
  or `src/testing/component_gallery_scene.rs` (WP05).
- Do **not** delete the `step_index` **field**. See T009 — this is the single
  most likely way to break this WP.
- Do **not** delete, convert, or reduce anything the gallery needs (C-003). If a
  deletion would ripple into a gallery source, page asset, scene, CLI option, or
  make target, stop and report rather than proceeding.
- No product behavior change (NFR-001). A deletion that changes what the shipped
  window renders is not a deletion of dead code.

## Context: what exists today

D3's verified caller-search table, re-confirmed here with current locations:

| Item | Location | External callers |
|---|---|---|
| `QualifyingFrameStream::await_qualifying`, `FrameAwaitError` | `src/shell/webview/frame_stream.rs` (`FrameAwaitError` l.122-146, `await_qualifying` l.205-245) | none; one doc reference in `mod.rs:39` |
| `LiveDemoRunner::step_index()` *(the accessor)* | `src/testing/live_demo_runner.rs:1753-1755` | none |
| `ControlIntent`, `ControlRequest`, `CompositionIntent` | `src/shell/component_vocabulary.rs` ~l.348-585 | none — every reference is internal to the module or its own unit tests |
| `CURSOR_GLYPH` | `src/shell/component_vocabulary.rs:812` | none beyond its own unit test at l.1424 |

Additional detail you will need:

- `frame_stream.rs` module header (l.11, l.26, l.34) references
  `await_qualifying` three times in prose and once as a rustdoc link target.
- `src/shell/webview/mod.rs:39` reads
  `//! [`frame_stream::FrameAwaitError::Timeout`] naming the awaited identity —`.
- `frame_stream.rs` in-module tests at l.256, l.327, l.368, l.380-386 use
  `await_qualifying` / `FrameAwaitError` directly. Those tests exist **only** to
  prove the deleted API; they retire with it. Read them first and confirm none
  of them incidentally proves something else that must survive — if one does,
  keep that assertion by relocating it to a surviving test, and say so in the
  report.
- `live_demo_runner.rs` uses the `step_index` **field** at 14 sites (l.58, 126,
  174, 295, 301, 314, 323, 328, 356, 385, 469, 482, 487, 577). The field drives
  the runner. Only the `pub const fn step_index(&self) -> usize` accessor at
  l.1753 goes.
- `component_vocabulary.rs`'s module doc names the retiring types at l.14-15
  ("the typed intent a control or composition returns
  ([`ControlIntent`], [`CompositionIntent`])") and describes the mechanism at
  l.19-21 ("A control's intent carries no `SemanticAction`; mapping intent onto
  an action needs focus and the reducer, which live on the other side of this
  boundary.").
- `CURSOR_GLYPH`'s doc claim is *"The glyph the design file places before a
  focused row's label."* D3: the single-source claim is false because the
  gallery hardcodes its own glyph and no surviving code consumes the constant.

## Subtasks

### T008 — Delete `await_qualifying`, `FrameAwaitError`, and the stale doc reference

**Purpose**: FR-005. A blocking-await API nobody calls, plus its error type, plus
the module prose that promises them.

**Steps**:

1. Confirm the caller search yourself before deleting:
   `grep -rn "await_qualifying\|FrameAwaitError" src/ tests/ webview-page/ Makefile`.
   Record the hit list in your report — that list is the deletion's scope.
2. Delete `pub fn await_qualifying` (l.205-245) and `pub enum FrameAwaitError`
   with its `Display` and `Error` impls (l.122-146).
3. Delete the in-module tests that exist only to prove them (l.327, l.368,
   l.380-386 and their enclosing test functions), and remove `FrameAwaitError`
   from the test module's `use` list at l.256.
4. Correct the `frame_stream.rs` module header: l.11, l.26, and the rustdoc link
   definition at l.34 all describe the blocking await. Rewrite the header to
   describe what `QualifyingFrameStream` actually is now — do not simply strike
   the sentences and leave a header that no longer explains the type.
5. Correct `src/shell/webview/mod.rs:39` — the sentence referencing
   `frame_stream::FrameAwaitError::Timeout`. Read the surrounding paragraph and
   rewrite it so it stays a true description of the shell, not a hole.
6. Whatever remains of `QualifyingFrameStream` (its recording side,
   `RECENT_OBSERVATION_CAPACITY`, `FrameExpectation`) **stays** — it has live
   callers. Verify by grep before assuming anything else is dead.

**Files**: `src/shell/webview/frame_stream.rs`, `src/shell/webview/mod.rs`.

**Validation**: `cargo build`; `cargo clippy --all-targets` clean (no
`dead_code` warnings introduced); `cargo doc --no-deps` produces no broken
intra-doc link warnings; `cargo test --lib` green.

**Edge cases**:

- Deleting `FrameAwaitError` may orphan a `use std::fmt;` or a
  `use std::time::Duration;`. Remove orphaned imports; clippy will tell you.
- If a *surviving* type's rustdoc links to `FrameAwaitError`, that link must go
  too — a broken intra-doc link is a build warning and a false record.

### T009 — Delete the `step_index()` accessor, NOT the field

**Purpose**: FR-005. **This is the highest-risk subtask in the mission.**

**Steps**:

1. Read `src/testing/live_demo_runner.rs:1745-1756`. The accessor is:
   ```rust
   pub const fn step_index(&self) -> usize {
       self.step_index
   }
   ```
   Delete **exactly** those three lines (plus any doc comment attached to them).
2. Do **not** touch `step_index: usize` at l.58, its initializer at l.126, or
   any of the 12 other read/write sites. The field drives the runner's step
   sequencing; deleting it breaks every live demo scene.
3. Verify after deletion:
   - `grep -n "pub const fn step_index" src/` → **zero** hits.
   - `grep -c "step_index" src/testing/live_demo_runner.rs` → still 14+ hits
     (the field).
   - `cargo test --lib` green.
4. If the neighbouring accessors (`shell_coverage`, `is_aborted`) turn out to be
   equally uncalled, **leave them**. D3 named one accessor; widening the
   deletion is scope creep (C-004) and this mission discharges exactly the
   deferred findings, nothing else.

**Files**: `src/testing/live_demo_runner.rs`.

**Validation**: `cargo test --lib`; the three headless suites; a live demo smoke
run if the harness permits (`make demo-live` targets are the runner's real
consumer).

**Edge cases**:

- `pub const fn` means the accessor could in principle be used in a const
  context somewhere unusual. The grep in step 1 covers `src/`; also grep
  `tests/` and `Makefile`.

### T010 — Delete the control-intent family; correct the module doc

**Purpose**: FR-004 + FR-005. The crest-spec retired the mechanism at `7c7f1cf`;
this deletes the code that realized it. D4 explains why this is a boundary
re-authoring, not the loss of a guarantee.

**Steps**:

1. Confirm the caller search:
   `grep -rn "ControlIntent\|ControlRequest\|CompositionIntent\|AdjustDirection\|AdjustGranularity" src/ tests/`.
   Record the hit list. Every hit must be either inside the deleted block, inside
   its own unit tests, or the assertion **message** at
   `tests/component_composition.rs:1742` — which belongs to **WP05 T018**, not
   to you. Do not edit that file.
2. Delete the `ControlIntent` / `ControlRequest` / `CompositionIntent` family in
   `src/shell/component_vocabulary.rs` (~l.348-585), including any supporting
   types that exist **only** for it (e.g. `AdjustDirection`, `AdjustGranularity`
   — verify each by grep; if one has a surviving consumer, it stays).
3. Delete the in-module unit tests that exist only to prove the deleted family.
4. **Correct the module doc** (l.1-26):
   - l.14-15 lists "the typed intent a control or composition returns
     ([`ControlIntent`], [`CompositionIntent`])" as one of the closed families —
     that bullet retires with the types.
   - l.19-21 describes the intent mechanism — rewrite it to state the boundary
     that **survives** (D4): component sources own nothing, cache nothing, may
     not name `AppState`, and may not convert an input into an action. That
     boundary is still live and still proven by `tests/component_composition.rs`.
   - The `Realizes` block at l.23-26 names `requirement.configurable_control_family`
     — that requirement survives with its mechanism clause retired. Keep the
     reference; do not delete it.
5. **Do not disturb the surviving vocabulary**: `ComponentControl`, `control_for`,
   `ShellComposition`, `ShellRegion`, the non-color signals (hint tones, status
   marks, unavailable mark), and everything the token generator, the production
   page, and the gallery resolve through. Both the production page and the
   gallery prove these; breaking them is a product behavior change.

**Files**: `src/shell/component_vocabulary.rs`.

**Validation**: `cargo test --lib`;
`cargo test --test component_vocabulary --test component_composition` green —
**unchanged**, not adjusted. If either suite fails, you deleted something the
surviving vocabulary needs; revert and narrow the deletion.

**Edge cases**:

- `ControlIntent` derives `Serialize`. Confirm nothing serializes it into a
  schema the page or a test fixture reads. If it does, that is a live consumer
  and the deletion is wrong — stop and report.
- The 48 gallery-borne proof references in `component_vocabulary` (noted in D5)
  are about the *surviving* vocabulary. Do not confuse them with the deleted
  family.

### T011 — Delete `CURSOR_GLYPH` and its unit test

**Purpose**: FR-005 acceptance scenario 3: no constant may document an authority
it does not hold. D3 chose the narrower of the two available fixes.

**Steps**:

1. Confirm: `grep -rn "CURSOR_GLYPH" src/ tests/ webview-page/`. Expect the
   declaration at `component_vocabulary.rs:812` and the assertion at l.1424
   only.
2. Delete the constant and its doc comment (l.811-812).
3. Delete `assert_eq!(CURSOR_GLYPH, ">");` from the
   `focus_is_legible_without_color` test at l.1424. **Keep the rest of that
   test** — `draws_cursor(ComponentState::Focused)` and
   `draws_cursor(ComponentState::Adjusting)` prove the non-color focus signal
   and are live proof (NFR-002). Only the constant's assertion goes.
4. If the test's leading comment ("The cursor is the non-color signal. Without
   it, focus would be a cyan keyline and nothing else.") is still true of what
   the test now asserts, keep it verbatim. It is.
5. Do **not** touch the gallery's hardcoded glyph (C-003, and it is WP05's file
   anyway). D3's reasoning is that the gallery hardcodes its own glyph — that is
   the *evidence* the single-source claim is false, not a thing to change.

**Files**: `src/shell/component_vocabulary.rs`.

**Validation**: `cargo test --lib`; `cargo test --test component_vocabulary`
green.

**Edge cases**:

- If the grep turns up a consumer you did not expect, the narrower fix is wrong
  and the right one is to make the claim true instead. Stop and report; do not
  silently switch fixes.

### T012 — Caller-search verification and full-suite run

**Purpose**: FR-005 requires *no dangling reference*, and US3 acceptance
scenario 4 requires the full suite passing with no behavior change and no proof
removed to accommodate a deletion.

**Steps**:

1. Run the quickstart's four verification greps and paste the output:
   ```
   grep -rn "await_qualifying\|FrameAwaitError" src/ tests/
   grep -rn "ControlIntent\|ControlRequest\|CompositionIntent" src/ tests/
   grep -rn "CURSOR_GLYPH" src/ tests/
   grep -rn "pub const fn step_index" src/
   ```
   Each must return zero hits **except** the `ControlIntent` mention in the
   assertion message at `tests/component_composition.rs:1742`, which WP05 T018
   corrects. Call that out explicitly as the one expected residual hit and name
   its owner.
2. `cargo doc --no-deps` — zero broken intra-doc links.
3. `cargo clippy --all-targets -- -D warnings` if the project's lint bar allows;
   otherwise `cargo clippy --all-targets` with a clean read.
4. Full suite: `cargo test` (all targets), plus the three headless acceptance
   suites named in the gate context.
5. Report the **net line delta** on `src/` (NFR-003 — this WP is the mission's
   main source of reduction):
   `git diff --stat` on your lane against its base.
6. Confirm in the report that **no proof was removed to accommodate a
   deletion** (NFR-002): every test you deleted existed only to exercise a
   deleted API, and you name each one.

**Files**: none modified — verification only.

**Validation**: all of the above green and pasted.

**Edge cases**:

- A `dead_code` warning appearing after your deletions means something else was
  only reachable through what you deleted. Report it; do not chain-delete
  without checking it against D3's table (C-004: this mission deletes exactly
  what was found dead, not everything that becomes dead).

## Branch Strategy

- **Planning base branch**: `feat/shell-hygiene`
- **Merge target branch**: `feat/shell-hygiene`

Planning artifacts for this mission were generated on `feat/shell-hygiene`.
During implementation this WP works on its own lane branch and merges back into
`feat/shell-hygiene` unless the human explicitly redirects the landing branch.

### Gate context — read this, it prevents three known failures

1. **Commit ONLY your owned production/test files on the lane branch.** Your
   owned files are the four listed in the frontmatter. The move-task gate
   **REFUSES** commits touching `kitty-specs/` on a lane branch. Review
   artifacts, evidence, and status files go on `feat/shell-hygiene` from the
   primary checkout — not from your lane. Likewise, do **not** commit
   `.kittify/crest-spec/` changes: the retirement is already committed at
   `7c7f1cf` and needs nothing further from you.
2. **Do not park waiting for a background notification.** If you launch anything
   in the background, use bounded foreground waits and check the run state
   yourself. Never end a turn with "waiting for the build to finish".
3. **Run `cargo test --lib` and the headless suites before requesting review.**
   The headless set is
   `cargo test --test webview_projection_shell --test component_vocabulary
   --test component_composition`. Paste the results.
4. **NFR-001 forbids product behavior change; NFR-002 forbids weakening any
   proof.** No frozen baseline, threshold, skip list, or assertion may be
   loosened. A test deleted because its subject was deleted is a retirement; a
   test deleted because it failed is a violation.

## Definition of Done

- [ ] `await_qualifying`, `FrameAwaitError`, their tests, and every doc
      reference (including `mod.rs:39` and the `frame_stream.rs` header) are
      gone; the rewritten headers still describe what the types are.
- [ ] The `step_index()` accessor is gone; the `step_index` **field** and all 14
      of its uses are intact; `grep -n "pub const fn step_index" src/` is empty.
- [ ] The `ControlIntent`/`ControlRequest`/`CompositionIntent` family and its
      family-only support types and tests are gone; the module doc states the
      surviving passivity boundary per D4; the `Realizes` block is intact.
- [ ] The surviving component vocabulary is untouched and
      `component_vocabulary` / `component_composition` pass **unchanged**.
- [ ] `CURSOR_GLYPH` and its single assertion are gone; the rest of
      `focus_is_legible_without_color` survives.
- [ ] The four verification greps return zero hits except the one residual at
      `tests/component_composition.rs:1742`, called out and attributed to WP05.
- [ ] `cargo test` (all targets) green; `cargo doc --no-deps` link-clean;
      clippy clean.
- [ ] Net `src/` line delta reported; every deleted test named and justified as
      a retirement.
- [ ] No file outside the four owned files is modified.

## Risks / Reviewer Guidance

- **Check the field survived.** The very first thing a reviewer should do is
  `grep -c "step_index" src/testing/live_demo_runner.rs` and confirm the field
  and its 14 uses are intact. Conflating the field with the accessor breaks
  every live demo scene and is the mission's named highest risk.
- **Check the surviving vocabulary.** Run `component_vocabulary` and
  `component_composition` and confirm they pass **without any assertion having
  been touched**. If the diff shows edits inside either suite, the deletion went
  too far (and those files belong to WP05 anyway).
- **Check the module doc says something true.** After the control-intent family
  goes, `component_vocabulary.rs`'s header must describe the boundary that
  survives (D4), not merely omit the retired mechanism. A header with a hole in
  it is the record drifting again.
- **Check no proof was removed to make a deletion pass.** Every deleted test
  should be traceable to a deleted subject. Ask for the list.
- **Check `CURSOR_GLYPH` was deleted rather than softened.** D3 chose deletion
  because no surviving code consumes it. If the implementer instead reworded the
  doc comment, that is the other fix and it was not the one chosen.
- **Watch for chain deletion.** New `dead_code` warnings are a report item, not
  a licence to keep deleting (C-004).
