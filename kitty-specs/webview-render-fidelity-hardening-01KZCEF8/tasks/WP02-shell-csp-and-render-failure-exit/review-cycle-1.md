---
wp_id: WP02
reviewer_agent: reviewer-renata
cycle_number: 1
verdict: approved
mission_slug: webview-render-fidelity-hardening-01KZCEF8
reviewed_at: 2026-08-06T23:59:00Z
affected_files:
  - path: src/shell/webview/window.rs
  - path: src/shell/webview/mod.rs
---

# WP02 Review — Cycle 1: APPROVED

Reviewed commit `9a48772` on `kitty/mission-webview-render-fidelity-hardening-01KZCEF8-lane-b`
(diff vs `kitty/mission-webview-render-fidelity-hardening-01KZCEF8`). Scope: exactly
`src/shell/webview/window.rs` + `src/shell/webview/mod.rs`, matching `owned_files`.

## What was verified

1. **T006 / C-004 — additive-only CSP hardening.** Extracted the runtime value of
   `PAGE_CSP` from both the base commit and HEAD (resolving Rust `\`-newline
   continuations) and compared mechanically: the new value is the old value with
   exactly `; base-uri 'none'; form-action 'none'` appended; every pre-existing
   directive byte-identical. Doc comment explains the non-inheritance of
   `base-uri`/`form-action` and the CSSOM-not-weakening rationale (C-004).

2. **T007 / D3 — single-source seam.** `pub const PAGE_CSP` and
   `pub fn protocol_response` (window.rs), re-exported from `shell::webview`
   (mod.rs:68) alongside the existing `TauriWebviewWindow` export. Doc comments at
   the declaration AND the re-export name both callers (production window's
   `crest://` handler at window.rs:393; acceptance harness in
   `tests/webview_projection_shell.rs`), cite
   `requirement.graphical_shell_behavioral_proof` and DRIFT-1/RISK-1, and mark it
   a two-caller seam, not dead public API (RISK-5 distinguishable). Nothing else
   became public (`page_asset`, `record_render_failure` remain private).
   Signature unchanged: `(path: &str, index_html: &str) -> tauri::http::Response<Vec<u8>>`.

3. **T008 / FR-006 — typed nonzero render-failure exit.** Full journey traced:
   `RENDER_ERROR_EVENT` listener (window.rs:408) → `PageSignal::RenderError` →
   `record_render_failure` (window.rs:310, `get_or_insert` first-error latch) →
   `WindowError` slot surfaced after `run_return` (window.rs:570) →
   `From<WindowError> for ApplicationError` (`ApplicationError::Window`,
   standalone_application.rs:547) → anyhow `Result` from `main` → nonzero exit.
   Payload lands in `PageRenderFailed { detail }` typed (only the JSON transport
   string is unwrapped; non-JSON kept verbatim — asserted in the new test). The
   latch drops later signals (loop `return`s after first error, `close_requested`
   set); mpsc send never blocks, so no receiver starvation. No ack conflation:
   `RenderError` is a distinct `PageSignal` arm that never reaches
   `forward_ack`; distinct source events (`crest://painted` vs
   `crest://render-error`).

4. **C-003 — teardown untouched.** `close_window_once_with_retry`, the
   `Destroyed` arm, and the false-tick close path are not in the diff; the
   `RenderError` arm's latch was extracted to a named helper with identical
   semantics. RISK-3 untouched.

5. **T009 — directive-level pinning.** The test iterates
   `PAGE_CSP.split(';')` and rejects `unsafe-inline`, `unsafe-eval`, `*`, and
   `https:` per directive; pins `base-uri 'none'` and `form-action 'none'`;
   served-header equality and non-HTML-no-CSP assertions retained.

6. **Boundary compliance.** `webview-page/*` and
   `tests/webview_projection_shell.rs` untouched (diff stat: 2 files). No
   reducer/RT/projection-schema changes.

## Test and probe results

- `cargo test --lib`: 631 passed, 0 failed, 1 ignored.
- `cargo test --test webview_projection_shell` (non-live): PASS (T022/T023/T025;
  live sections skipped without `CREST_WEBVIEW_TESTS=1`, as designed).
- `cargo clippy --all-targets`: 0 warnings, 0 errors.
- **Probe A (seam reachability):** temporary integration test importing
  `crest_synth::shell::webview::{protocol_response, PAGE_CSP}` compiled and
  asserted the served HTML header equals the constant — PASS, probe removed.
- **Probe B (weaken CSP):** added `'unsafe-inline'` to `style-src` in the
  constant — `the_csp_allows_exactly_what_the_shipped_page_needs` FAILED as
  required. Reverted.
- **Probe C (break latch):** replaced `get_or_insert` with an overwrite —
  `a_render_error_signal_records_the_first_typed_failure_and_latches` FAILED
  with the later payload winning, exactly the regression it must catch. Reverted.
- Worktree restored to clean at `9a48772` before this review file was added.

## Anti-pattern checklist

1. Dead code — PASS (both exports have the production caller at window.rs:393;
   seam purpose documented at declaration and re-export).
2. Synthetic-fixture test — PASS (tests invoke production
   `record_render_failure`/`render_error_detail`/`PAGE_CSP`; probes B and C
   prove the tests die when the mechanism breaks).
3. Silent empty return — PASS (no new swallowed-error paths in the diff).
4. FR coverage — PASS (FR-006: latch test; NFR-001: pinning test; FR-002 shell
   half: seam export; end-to-end nonzero exit deliberately lands in WP03 T012
   per the WP contract).
5. Frozen surface — PASS (only owned files touched).
6. Locked decision — PASS (C-004 additive-only proven; C-003 respected).
7. Shared-file ownership — PASS (no overlap with WP01/WP03/WP04 owned files).
8. Production fragility — PASS (no new panics in production paths;
   `unreachable!` is test-only).

Baseline: the one pre-existing base-branch failure (`<declared-command>`) is
unrelated; no regressions introduced.
