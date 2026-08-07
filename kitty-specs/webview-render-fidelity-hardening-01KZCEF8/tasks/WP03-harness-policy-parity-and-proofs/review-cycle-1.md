---
wp_id: WP03
reviewer_agent: reviewer-renata
cycle_number: 1
verdict: approved
mission_slug: webview-render-fidelity-hardening-01KZCEF8
reviewed_at: "2026-08-06T23:59:00Z"
affected_files:
  - path: tests/webview_projection_shell.rs
  - path: kitty-specs/webview-render-fidelity-hardening-01KZCEF8/evidence/README.md
---

# WP03 Review — Cycle 1: APPROVED

Reviewed lane commit `d30250f` (branch
`kitty/mission-webview-render-fidelity-hardening-01KZCEF8-lane-c`, touching
only `tests/webview_projection_shell.rs`) plus the evidence commit `89df0ca`
on `feat/webview-shell-cutover`. Every claim below was verified mechanically,
not taken from the implementer's report.

## Diff verification

- **One serving path (T010, FR-002)**: exactly one
  `register_uri_scheme_protocol` in the test file
  (`tests/webview_projection_shell.rs:1239`), routing every live section
  through the exported `crest_synth::shell::webview::protocol_response`. The
  old bare no-CSP closure is deleted (DRIFT-1 closed).
- **No restated policy**: `grep` for `style-src`/`default-src`/`script-src`/
  `connect-src`/`unsafe-inline` in the test file returns zero hits. The
  parity assertion in `prove_protocol_policy_parity` compares the served
  `/index.html` header against the exported `PAGE_CSP` constant itself, and
  asserts non-HTML responses carry no CSP plus byte-identity of all 9 served
  assets against the committed page and a 404 for unknown paths.
- **T011 (FR-004, NFR-003)**: measured `.fader-fill` height and
  `.prow-position-fill` width proportional to document fractions
  (tolerance-bounded against usable track height / rail width) at both
  authored viewports; hex-73 fixture strictly nonzero; the driven-to-floor
  zero fixture asserts exactly one `data-level="0.000000"` WITH inline and
  computed `--level` applied; the inverse guard collects
  attribute-present-without-CSSOM-property violations and fails naming the
  element; every measurement is double-taken and must be identical.
- **T012 (FR-006, SC-002, analysis C1)**: forced FIRST-render throw runs the
  shipped binary as a subprocess via the debug-only `CREST_WEBVIEW_PAGE`
  override and asserts nonzero exit with exactly one distinct typed
  `PageRenderFailed` whose detail parses as the typed JSON payload (name
  `TypeError`, message, generation, stateHash); the update-render throw fires
  AFTER healthy documents painted and acked (closes C1 / the
  first-vs-update-render edge case) with exactly one typed
  `crest://render-error` and no ack; the unhandledrejection variant likewise;
  the negative control asserts zero render-errors across all healthy
  sections and runs deliberately last-but-before-the-faults. Confirmed in
  production source that the override seam is `cfg(debug_assertions)`-gated
  with a `cfg(not(debug_assertions))` compile-out guard test
  (`src/shell/webview/window.rs:79,253,883`) — unreachable in release.
- **Frozen surfaces (C-002)**: the deletion side of the diff is only the old
  protocol closure, doc comments, and signature plumbing — no existing
  assertion loosened, no baseline file touched, the 50 ms p95 threshold and
  the `CREST_ACCEPTANCE` marker unchanged, new live sections added to the
  honest-skip list (no widened skip).
- **No production-source edits**: `git show d30250f --stat` touches only
  `tests/webview_projection_shell.rs`.

## Evidence verification (89df0ca)

- README indexes every artifact to proof section + requirement; all six
  referenced artifacts exist; the production policy is quoted with an
  explicit note that the harness compares against the constant, not the
  quotation, citing `src/shell/webview/window.rs`; both falsifiability
  spot-checks recorded with lane commit and exact failure signatures.
- `acceptance-live-run.log` markers cross-check against README claims:
  `skipped: none`, p50 8.1 / p95 8.9 / max 11.7 ms over the 150-edit paced
  workload, soak 1767/1767 frames at 29.45 Hz, T012 subprocess `exit:
  Some(1)` with the typed JSON payload (also verbatim in
  `t012-forced-first-render-throw.log`).
- Screenshots opened and inspected: level-73 desktop and compact show all
  sixteen fills painted ~90% beside matching `73` readouts; level-00 desktop
  shows the focused T00 fill empty at readout `00` with the fifteen
  neighbors still filled — the zero-vs-never-applied fixture as claimed.

## Independent mechanical runs (lane worktree at d30250f)

- Headless `cargo test --test webview_projection_shell`: PASS, new sections
  named in the skip list.
- Live `CREST_WEBVIEW_TESTS=1 cargo test --test webview_projection_shell --
  --nocapture`: full suite PASS, `skipped: none`, p50 7.8 / p95 9.0 / max
  11.9 ms (threshold 50 ms), T012 subprocess exit Some(1), reproducing the
  committed evidence independently.
- **Disable-the-mechanism probe (FR-004 kill test)**: with
  `applyDynamicGeometry` neutered by an early return in the lane's
  `webview-page/page.js`, T010 parity and T024 determinism (including
  readout-text assertions) still PASSED while T011 failed at the RISK-1
  signature naming all sixteen elements (`<div data-structure=LevelFader>
  carries data-level=0.909091 but no CSSOM --level property is applied`);
  suite exited nonzero. Edit reverted; worktree confirmed clean at
  `d30250f`.

## Anti-pattern checklist

1. Dead code: N/A (test-only WP; every new fn called in the suite's flow).
2. Synthetic-fixture test: PASS — fixtures flow through the production
   reducer (`AppState::apply`), projector, emit path, and the page's real
   `render`; the kill test proves the proofs die when the implementation is
   disabled.
3. Silent empty return: PASS — no swallowed failures; the one best-effort
   write (evidence transcript) prints its path and gates no assertion.
4. FR coverage: PASS — FR-002 (T010), FR-003 (T013 + evidence), FR-004
   (T011 + kill test), FR-006 (T012 all variants), NFR-002 (p95 9.0 ≤ 50 ms
   measured), NFR-003 (double-render + double-measure identity).
5. Frozen surface: PASS (see C-002 above).
6. Locked decision: PASS — no CSP literal, no threshold adjustment, no
   production edits, no baseline loosening.
7. Shared-file ownership: PASS — only WP03 owns
   `tests/webview_projection_shell.rs` this mission.
8. Production fragility: N/A — no production code in this WP.

Verdict: **approved**.
