# Quickstart: running this mission's proofs

**Mission**: `shell-hygiene-01KZD0KR` — Shell Hygiene Sweep

All commands run from the repository root. Nothing here changes product
behavior, so every proof below must read the same before and after the
mission except the two new error-path sections.

## The declared validations

```bash
# Unit layer — the touched modules' own tests (close latch, ack validation)
cargo test --lib

# Headless acceptance — runs everywhere, skips the env-gated live sections
cargo test --test webview_projection_shell --test component_vocabulary --test component_composition

# Full live acceptance — real WKWebView windows, must report `skipped: none`
CREST_WEBVIEW_TESTS=1 cargo test --test webview_projection_shell -- --nocapture
```

The live run is the one that proves NFR-001 (no product behavior change):
it must pass with `skipped: none` and its projection-to-paint p95 must stay
within the declared 50 ms threshold.

## Falsifying the two new proofs

A proof that cannot fail is not a proof. Each new section must die when its
mechanism is disabled:

- **RISK-3 (FR-001)** — restore the old `close_window_once_with_retry` return
  (record the typed error, return normally, no exit edge). The forced
  double-close-failure section must fail with a timeout or a
  never-surfaced-error signature, not pass.
- **RISK-4 (FR-003)** — bypass the retired-identity comparison in the
  superseded-late branch. The corrupted-ack section must fail; the
  well-formed-ack negative control must still pass, proving the change
  rejects only what it should.

## Verifying the deletions (FR-005)

```bash
# Each must return zero hits outside historical records after the mission
grep -rn "await_qualifying\|FrameAwaitError" src/ tests/
grep -rn "ControlIntent\|ControlRequest\|CompositionIntent" src/ tests/
grep -rn "CURSOR_GLYPH" src/ tests/
grep -rn "pub const fn step_index" src/
```

The `step_index` **field** inside `src/testing/live_demo_runner.rs` stays —
it drives the runner. Only the public accessor goes.

## Verifying the gallery is intact (C-003)

```bash
# All must still exist — the gallery is retained by operator decision
ls webview-page/gallery.js webview-page/gallery.css src/testing/component_gallery_scene.rs
grep -n "demo-live-component-library" Makefile
```

## Falsifying the scan extension (FR-007)

Plant a purity violation in each newly covered page source in a scratch tree
(e.g. a `Date.now()` call) and confirm `component_composition` fails naming
that source and the offending needle. Revert before committing.
