# Tasks: Shell Hygiene Sweep

**Mission**: `shell-hygiene-01KZD0KR`
**Branch contract**: planning base and merge target are both `feat/shell-hygiene`
**Input**: `plan.md` (IC-01…IC-06), `spec.md` (FR-001…FR-008, NFR-001…003, C-001…004), `research.md` (D1…D7)

This is a hygiene mission. It changes **no product behavior** (NFR-001) and
**weakens no proof** (NFR-002). Every work package either closes a correctness
hole in an existing error path, retires code whose declaration was already
retired in the crest-spec (commit `7c7f1cf`, satisfying C-002), or makes the
record and the guards say what is actually true.

## Subtask Index

| ID | Description | WP | Parallel |
|----|-------------|----|----|
| T001 | Add the missing termination edge when the close retry is exhausted | WP01 | [P] |
| T002 | Preserve retry-once, the typed `WindowClose` error, and the first-error latch | WP01 | [P] |
| T003 | In-module tests: latch precedence + termination on double close failure | WP01 | |
| T004 | Bounded retired-identity store fed by both retirement paths | WP02 | [P] |
| T005 | Validate superseded-late acks with the `ACK_IDENTITY_FIELDS` comparison | WP02 | |
| T006 | Beyond-window acks stay lost frames; well-formed superseded-late acks unchanged | WP02 | |
| T007 | Unit tests for all three cases plus a bypass probe | WP02 | |
| T008 | Delete `await_qualifying` + `FrameAwaitError` + the stale `mod.rs` doc reference | WP03 | [P] |
| T009 | Delete the `LiveDemoRunner::step_index()` accessor — the field stays | WP03 | [P] |
| T010 | Delete the `ControlIntent`/`ControlRequest`/`CompositionIntent` family; correct the module doc | WP03 | [P] |
| T011 | Delete `CURSOR_GLYPH` and its unit test — its single-source claim is false | WP03 | [P] |
| T012 | Caller-search verification: zero dangling references, full suite unchanged | WP03 | |
| T013 | Forced double-close-failure proof: typed error surfaces, process ends | WP04 | [P] |
| T014 | Corrupted superseded-late ack proof + well-formed negative control | WP04 | [P] |
| T015 | Suite-wide negative control: a healthy live run records zero ack rejections | WP04 | |
| T016 | Run the gated suite; record the disable-the-mechanism probes | WP04 | |
| T017 | Extend the purity-needle loop to every page source it means to bind | WP05 | [P] |
| T018 | Correct the assertion message naming the deleted `ControlIntent` | WP05 | [P] |
| T019 | Narrate the gallery scene's policy-free protocol handler as deliberate | WP05 | [P] |
| T020 | Discharge DRIFT-3 documentation residue | WP05 | [P] |
| T021 | Run both guard suites; record a planted-violation probe per newly covered source | WP05 | |

`[P]` marks a subtask with no predecessor inside its own work package. **WP01,
WP02, and WP03 are wave-1 parallel with each other** — they share no file and
no ordering constraint (D7).

## Work Packages

### WP01 — Window close-failure exit edge

- **Prompt**: `tasks/WP01-window-close-failure-exit-edge.md` (~330 lines)
- **Goal**: When both window-close attempts fail, the shell reaches termination
  by a route that does not depend on the window closing, so the recorded first
  typed error reaches the operator through the existing post-`run_return`
  surfacing path (FR-001) — with the retry, the typed `WindowClose` error, and
  the first-error latch untouched (FR-002).
- **Priority**: P1 (User Story 1; mission-review RISK-3)
- **Independent test**: In-module tests pin (a) a prior `PageRenderFailed` still
  winning over a later `WindowClose` on the shared first-error slot, and (b) a
  double close failure producing the termination decision rather than a normal
  return. The live end-to-end assertion lands in WP04 T013.
- **Subtasks**:

  T001 Add the missing termination edge on exhausted retry (WP01)
  T002 Preserve retry-once, typed error, and first-error latch (WP01)
  T003 In-module tests for latch precedence and termination (WP01)

- **Implementation sketch**: read `close_window_once_with_retry`
  (`src/shell/webview/window.rs:317-340`) and its four call sites in the
  `run_return` callback → after the `get_or_insert` records the typed
  `WindowClose`, add the exit route that does not require the window to close →
  add a debug-only forced-failure seam mirroring the existing
  `cfg(debug_assertions)` `CREST_WEBVIEW_PAGE` precedent (`window.rs:77-80`,
  `233-253`) so WP04 can force the condition without a runtime flag in release →
  extend the in-module tests near `window.rs:796-830`.
- **Dependencies**: none. **Parallel with**: WP02, WP03.
- **Risks**: touching the `Destroyed` arm or the ordinary teardown path is out
  of scope and would move product behavior (NFR-001); the latch must keep
  `PageRenderFailed` winning; the seam must compile out of release exactly as
  the page override does.

### WP02 — Superseded-late ack identity validation

- **Prompt**: `tasks/WP02-superseded-late-ack-identity-validation.md` (~350 lines)
- **Goal**: `ProjectionChannel::forward_ack`'s superseded-late window validates
  the ack against the identity of the retired document it names, using the same
  `ACK_IDENTITY_FIELDS` comparison and the same typed error class the in-flight
  path uses, so "verbatim or typed-rejected" is true in every window (FR-003).
- **Priority**: P2 (User Story 2; mission-review RISK-4)
- **Independent test**: A corrupted superseded-late ack is typed-rejected; a
  well-formed one is consumed exactly as before; an ack older than the retained
  window keeps today's lost-frame behavior.
- **Subtasks**:

  T004 Bounded retired-identity store fed by both retirement paths (WP02)
  T005 Identity validation in the superseded-late branch (WP02)
  T006 Beyond-window and well-formed behavior preserved (WP02)
  T007 Unit tests for all three cases plus a bypass probe (WP02)

- **Implementation sketch**: add a `VecDeque` companion beside `in_flight`
  (`projection_channel.rs:295-302`) holding `(generation, identity)` for retired
  documents, bounded and evicted with the same discipline → push into it from
  BOTH retirement paths: the capacity eviction `pop_front` at l.343-345 and the
  ack-consumption `drain(..=index)` at l.456 → in the superseded-late branch
  (l.388-405) look the generation up in the store and, on a hit, run the same
  `ACK_IDENTITY_FIELDS` loop the in-flight path runs at l.409-414, returning
  `PaintedAckError::IdentityMismatch` on a mismatch → a miss keeps today's
  `ForwardedAck::SupersededLate`.
- **Dependencies**: none. **Parallel with**: WP01, WP03.
- **Risks**: feeding only one retirement path makes the validation silently
  partial (D2); the store is shell-side and must never appear near the
  real-time callback (C-001); an unbounded store was considered and rejected;
  a false rejection of a healthy ack would break NFR-001 — the negative control
  is WP04's full live run recording zero rejections.

### WP03 — Dead code removal

- **Prompt**: `tasks/WP03-dead-code-removal.md` (~320 lines)
- **Goal**: The four items D3 verified dead are absent with no dangling
  reference, and the declarations that described them — retired in the
  crest-spec at `7c7f1cf` before any deletion, per C-002 — no longer describe
  code that is gone (FR-004, FR-005).
- **Priority**: P2 (User Story 3; mission-review RISK-5)
- **Independent test**: The quickstart's four greps return zero hits outside
  historical records; `cargo test` passes with no proof removed.
- **Subtasks**:

  T008 Delete `await_qualifying` + `FrameAwaitError` + the stale doc reference (WP03)
  T009 Delete the `step_index()` accessor — the field stays (WP03)
  T010 Delete the control-intent family; correct the module doc (WP03)
  T011 Delete `CURSOR_GLYPH` and its unit test (WP03)
  T012 Caller-search verification and full-suite run (WP03)

- **Implementation sketch**: for each item, grep for callers first → delete the
  item, its own unit tests, and every doc reference that describes it →
  `component_vocabulary.rs`'s module doc (l.14-15, l.19-21) names the retiring
  types and is corrected with them → re-grep for dangling references → run the
  full suite.
- **Dependencies**: none. **Parallel with**: WP01, WP02. **Unblocks**: WP05
  (T018 corrects a message naming a type this WP deletes).
- **Risks**: `step_index` is a field AND an accessor — deleting the field breaks
  the runner (it is read at 14 sites); deleting the control-intent family must
  not disturb the surviving component vocabulary (tokens, states, controls,
  compositions) that both the production page and the gallery prove; `mod.rs`
  carries one doc reference to `FrameAwaitError` at l.39.

### WP04 — Error-path proofs under the production path

- **Prompt**: `tasks/WP04-error-path-proofs.md` (~360 lines)
- **Goal**: `tests/webview_projection_shell.rs` proves WP01's and WP02's fixes
  falsifiably under the production path: the forced double-close failure ends
  with the recorded typed error surfaced (FR-001) and a corrupted superseded-late
  ack is typed-rejected while healthy acks are not (FR-003).
- **Priority**: P1 (completing User Stories 1 and 2)
- **Independent test**: `CREST_WEBVIEW_TESTS=1 cargo test --test
  webview_projection_shell -- --nocapture` passes with `skipped: none`;
  restoring WP01's old return makes T013 fail; bypassing WP02's comparison makes
  T014 fail while its negative control still passes.
- **Subtasks**:

  T013 Forced double-close-failure section (WP04)
  T014 Corrupted superseded-late ack section + well-formed negative control (WP04)
  T015 Suite-wide zero-ack-rejection negative control (WP04)
  T016 Full gated run; record both disable-the-mechanism probes (WP04)

- **Implementation sketch**: drive WP01's debug-only forced-failure seam from a
  live section and assert the process ends carrying the typed error rather than
  hanging → add a headless section feeding `ProjectionChannel` a corrupted
  superseded-late ack and a well-formed one → extend the existing healthy-run
  counters (the render-error negative control at l.2958-2983 is the pattern) to
  assert zero ack rejections across every healthy section → any new
  window-bearing section is added to the honest-skip list at l.174-182 as a new
  entry; no existing entry is widened.
- **Dependencies**: **WP01, WP02** (the proofs measure their fixes).
- **Risks**: a proof that cannot fail is not a proof — both probes must be
  recorded in the report (quickstart "Falsifying the two new proofs"); no frozen
  baseline, threshold, or assertion may be loosened (NFR-002); the live run must
  still report `skipped: none` and stay within the declared 50 ms p95 (NFR-001).

### WP05 — Guard-scan coverage, gallery narration, and record residue

- **Prompt**: `tasks/WP05-guard-scan-narration-and-record-residue.md` (~340 lines)
- **Goal**: The purity-needle scan binds every page source it means to bind
  (FR-007), the gallery's policy-free serving is narrated as deliberate (FR-006),
  and the DRIFT-3 documentation residue is discharged (FR-008).
- **Priority**: P3 (User Stories 4, 5, 6; mission-review OBS-1, SMELL-1 residue,
  DRIFT-3)
- **Independent test**: A planted purity violation in each newly covered source
  fails `component_composition` naming that source and the offending needle; the
  gallery narration reads as a deliberate exemption with its blast radius stated.
- **Subtasks**:

  T017 Extend the purity-needle loop to every page source (WP05)
  T018 Correct the assertion message naming the deleted `ControlIntent` (WP05)
  T019 Narrate the gallery's policy-free protocol handler (WP05)
  T020 Discharge DRIFT-3 documentation residue (WP05)
  T021 Run both guard suites; record a planted-violation probe per source (WP05)

- **Implementation sketch**: the purity loop at
  `tests/component_composition.rs:1789-1806` binds `page_js` alone while the
  adjacent key-handler loop at l.1808-1819 already enumerates `page.js`,
  `index.html`, `gallery.js` — restructure the purity loop over the same
  enumeration so each failure names its source → correct the message at l.1742 →
  add the narration comment at
  `src/testing/component_gallery_scene.rs:3084-3095` → amend the completed
  missions' records → run both suites and plant one violation per newly covered
  source.
- **Dependencies**: **WP03** (T018 corrects a message naming a type WP03
  deletes; doing it first would leave the message correct and the type present).
- **Owned files vs documentation surface**: WP05 owns
  `tests/component_composition.rs` and `src/testing/component_gallery_scene.rs`.
  T020's target — the two completed missions' planning trees under
  `kitty-specs/` — is declared as `documentation_surface`, not `owned_files`:
  Spec Kitty rejects `kitty-specs/` paths in `owned_files`
  (`INVALID_WP_OWNED_FILES_KITTY_SPECS`) and the move-task gate refuses
  `kitty-specs/` commits on lane branches. Those edits are authored and reported
  by WP05, then landed on `feat/shell-hygiene` from the primary checkout.
- **Risks**: **C-003 — the gallery is retained by operator decision.** Deleting,
  converting, or reducing any gallery source, page asset, scene, CLI option, or
  make target fails this WP. T019 is narration only. A legitimate gallery
  construct that fires a newly bound needle is fixed at the source or gets a
  declared, narrated exemption mirroring the existing precedent — never a silent
  carve-out (D6). T020 amends status fields and terminology only; a closed
  gate's evidence or verdict is never rewritten to read better in hindsight.

## Sequencing

- **Wave 1 (parallel)**: WP01, WP02, WP03 — six disjoint surfaces, no shared
  file, no ordering constraint (D7). C-002's one ordering constraint is already
  satisfied by the crest-spec retirement at `7c7f1cf`.
- **Wave 2**: WP04 (after WP01 + WP02) and WP05 (after WP03). These two are
  parallel with each other.
- **MVP**: WP01 alone closes the one failure class this program treats as
  unacceptable — a recorded fatal error that never reaches the operator. WP04
  makes it provable.

## Cross-cutting obligations

Every work package carries these, and a reviewer checks them per WP:

- **NFR-001** — no product behavior change. The live acceptance suite passes
  with `skipped: none` and latency stays within the declared 50 ms p95.
- **NFR-002** — no proof weakened. No frozen baseline, threshold, skip list, or
  assertion is loosened, and no declared validation stops executing except where
  it retires with its declaration.
- **NFR-003** — net code reduction across `src/` and `tests/`.
- **C-001** — no reducer, real-time, projection-schema, or product-surface
  change. Shell, testing-scene, test, and document surfaces only.
- **C-004** — no Phase 5 product work, no new feature, no re-declared
  control-intent vocabulary.
