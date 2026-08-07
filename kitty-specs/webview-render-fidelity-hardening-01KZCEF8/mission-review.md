# Mission Review Report: webview-render-fidelity-hardening-01KZCEF8

**Reviewer**: Claude (spec-kitty-mission-review, post-merge)
**Date**: 2026-08-06
**Mission**: `webview-render-fidelity-hardening-01KZCEF8` — Webview Render Fidelity and Error-Path Hardening
**Baseline commit**: `2691475ce260fd7def5f4fe393a9c986ea0de565`
**HEAD at review**: `3a47c82` (post-merge, retrospective captured)
**WPs reviewed**: WP01–WP04 (all `done`; every WP approved on review cycle 1, no rejections, no arbiter overrides, no self-approvals)

Mission charter: fix exactly the four findings of the `webview-shell-cutover-01KZAC7Q` post-merge review — RISK-1 (CSP-blocked fader/position painting), RISK-2 (render-error channel with no emitter), DRIFT-1 (harness measured a laxer policy than production), DRIFT-2 (gallery outside the guard scans) — and nothing else.

---

## Gate Results

This repository's declared acceptance layer is the crest-spec deterministic validations, not the spec-kitty-repo contract/architectural/cross-repo-e2e suites (those gates target the spec-kitty codebase itself and are N/A here). The gates below are the applicable equivalents, all re-run at HEAD by this review.

### Gate 1 — Crest-spec integrity
- Command: `spec-kitty crest-spec doctor`
- Result: **PASS** — 7 contexts / 132 resources, 107 requirements, 32 project validations, no diagnostics.

### Gate 2 — Declared validation suites (headless layer)
- Command: `cargo test --test component_vocabulary --test component_composition --test webview_projection_shell`
- Exit code: 0 — **PASS**. component_vocabulary 11/11, component_composition 15/15, webview_projection_shell headless PASS including T010 policy parity (served CSP equals the exported `PAGE_CSP` constant; 9 assets byte-identical; unknown path 404). Skips limited to the declared env-gated live sections, named in the honest-skip list.

### Gate 3 — Full live gated acceptance (production webview path)
- Command: `CREST_WEBVIEW_TESTS=1 cargo test --test webview_projection_shell -- --nocapture`
- Exit code: 0 — **PASS with `skipped: none`**, independently reproduced by this review on real WKWebView windows:
  - T024 determinism at both viewports, both contexts;
  - T011 painted-geometry fidelity (hex-73 nonzero on all sixteen tracks; zero-value fixture zero WITH its CSSOM property applied);
  - NFR-002 projection-to-paint p50 7.0 ms / p95 12.6 ms / max 18.3 ms (threshold p95 ≤ 50 ms), 60 s meter soak, 0 lost;
  - T012 negative control (zero render-errors across healthy sections), update-render throw, unhandled rejection, and forced first-render throw on the shipped binary → exit `Some(1)` with exactly one distinct typed `PageRenderFailed` JSON payload;
  - T026 shutdown parity exit 0.
- Command: `cargo test --lib` — 631 passed, 0 failed (includes the NFR-001 per-directive CSP pinning test and the FR-006 first-error-latch test).

### Gate 4 — Issue matrix
- **N/A** — `spec.md` references the predecessor mission-review findings (RISK-1/2, DRIFT-1/2), not GitHub issues; no `issue-matrix.md` was scaffolded, correctly. Finding disposition is covered by SC-005 and traced in the FR matrix below.

---

## FR Coverage Matrix

| FR ID | Description (brief) | WP Owner | Test / Evidence | Adequacy | Finding |
|-------|---------------------|----------|-----------------|----------|---------|
| FR-001 | Geometry via data-attribute + CSSOM, no JS-built inline `style` | WP01 | `page.js` diff (data-level/data-position + `applyDynamicGeometry` final in `render()`); T011 live; `grep 'style="' page.js` → 0 hits | ADEQUATE | — |
| FR-002 | Harness serves the production policy from the single source | WP02+WP03 | `prove_protocol_policy_parity` (headless, compares served header against the exported `PAGE_CSP` constant itself); exactly one `register_uri_scheme_protocol` in the harness, routed through `protocol_response` | ADEQUATE | — |
| FR-003 | Affected proofs re-run under production policy, evidence committed | WP03 | `evidence/` (6 artifacts + README index); independently reproduced by this review at HEAD | ADEQUATE | — |
| FR-004 | Falsifiable paint-fidelity proof | WP03 | T011 inverse guard (attribute-present-without-CSSOM-property fails naming the element); disable-the-mechanism kill test documented in WP03 review and evidence README with exact failure signature | ADEQUATE | — |
| FR-005 | Page-side error boundary emits typed `crest://render-error`; false comment corrected | WP01 | try/catch around `render`+`updateMeter` (no ack on failure), global `error`/`unhandledrejection`, shared first-error latch; comment replaced in diff; T012 live variants | ADEQUATE | — |
| FR-006 | Typed nonzero exit on render failure, falsifiably proven | WP02+WP03 | `record_render_failure` latch (lib test, probe-verified); T012 forced first-render throw subprocess: exit 1, one distinct typed JSON payload | ADEQUATE | — |
| FR-007 | Gallery inside both guard scans | WP04 | `(name, source)` tuples now include gallery pair in both scans; 6 injection probes documented, each failing with a naming message | ADEQUATE | — |
| NFR-001 | CSP hardened, never weakened; `base-uri`/`form-action` denied | WP02 | Per-directive pinning test rejects `unsafe-inline`/`unsafe-eval`/`*`/`https:` in every directive; additive-only change mechanically verified in WP02 review | MET | — |
| NFR-002 | 50 ms p95 under production policy | WP03 | p95 8.9 ms (committed), 9.0 ms (WP03 reviewer), 12.6 ms (this review) — all ≤ 50 ms | MET | — |
| NFR-003 | Determinism under production policy | WP03 | T024 double-render identical; T011 double-measure identical | MET | — |

Constraints: **C-001** held (diff touches only `webview-page/page.js`, `src/shell/webview/{window,mod}.rs`, three test files, mission artifacts — no reducer/RT/projection-schema change). **C-002** held (no baseline file in the diff; no threshold loosened; skip-list additions are additions, not widenings). **C-003** held (RISK-3/4/5, DRIFT-3 territory untouched — teardown path, dead-code items not modified beyond the latch extraction with identical semantics). **C-004** held (additive-only policy change, executable per-directive guard).

Predecessor-finding disposition (SC-005): RISK-1 **resolved** (T011 + live run), RISK-2 **resolved** (T012 all variants), DRIFT-1 **resolved** (T010 parity from the single source), DRIFT-2 **resolved** (FR-007 scans). No new findings introduced in the touched seams (see Risk Findings).

---

## Drift Findings

None. No non-goal invasion, no locked-decision violation, no punted FR, no NFR miss. The diff's file set matches the plan's declared surfaces exactly (plus `mod.rs` for the documented seam re-export).

---

## Risk Findings

No CRITICAL or HIGH findings. Two non-blocking observations:

### OBS-1: Gallery testing scene serves its page with no CSP

**Type**: CROSS-WP-INTEGRATION (latent, pre-existing)
**Severity**: LOW (out of mission scope by C-003/FR-007; testing-only surface)
**Location**: `src/testing/component_gallery_scene.rs:3084-3095` (protocol handler attaches Content-Type only); `webview-page/gallery.js` (10 JS-built inline `style="` emissions, e.g. lines 55, 185, 192)
**Trigger condition**: none today — `page_asset` never serves gallery assets, so the shipped window cannot load them.

**Analysis**: The gallery scene has the same shape DRIFT-1 had — a surface whose harness serves a laxer policy than `PAGE_CSP` — and `gallery.js` still paints via inline `style` attributes that `style-src 'self'` would block. This is not a defect of this mission (the spec deliberately scoped gallery work to the two guard scans, and the gallery is a testing scene the production binary never serves), but if the gallery is ever served under the production policy, its fills will paint empty exactly as RISK-1 did. Recommend the deferred hygiene mission either serve the gallery through `protocol_response` and convert its geometry to the data-attribute + CSSOM pattern, or record the policy-free serving as a declared property of the testing scene.

### OBS-2: Paint-fidelity and error-path proofs live only in the env-gated live layer

**Type**: BOUNDARY-CONDITION (proof topology, by design)
**Severity**: LOW
**Location**: `tests/webview_projection_shell.rs` (T011, T012, T024 behind `CREST_WEBVIEW_TESTS=1`)
**Trigger condition**: a regression merged after running only the headless default suite.

**Analysis**: A reintroduced inline-style emission or a deleted error boundary passes the headless layer (WP01's D3 probe documented this honestly); only the gated live run kills it. The skip list is honest and the crest-spec validation declares the live run, so this is the declared design, not a hole — but the protection is only as strong as the discipline of running the gated suite at acceptance. This review re-ran it at HEAD: full PASS, `skipped: none`.

---

## Silent Failure Candidates

| Location | Condition | Silent result | Spec impact |
|----------|-----------|---------------|-------------|
| `webview-page/page.js` `emitRenderError` | `window.__TAURI__` absent (headless harness only) | returns after latching | None — documented headless contract; `window.crest.render` propagates the throw to the harness caller; in the shipped window `__TAURI__` is always injected |
| `tests/webview_projection_shell.rs:3252` | evidence transcript write fails | `let _ =` discards the error | None — best-effort log after the verdict-bearing assertions; path printed; gates no assertion |

No production code path introduced by this mission returns a default value on malfunction.

---

## Security Notes

| Finding | Location | Risk class | Assessment |
|---------|----------|------------|------------|
| CSP hardened additively: `base-uri 'none'; form-action 'none'` appended; every prior directive byte-identical | `src/shell/webview/window.rs:108-110` | — (hardening) | Per-directive executable guard rejects `unsafe-inline`, `unsafe-eval`, wildcard, and remote-scheme sources anywhere; C-004 verified additive-only |
| New public surface `protocol_response` / `PAGE_CSP` | `src/shell/webview/mod.rs:62-68` | API-SURFACE | Deliberate, documented two-caller seam (production window + harness); `page_asset` and `record_render_failure` remain private; path handling is a fixed match with traversal asserted rejected (`/../Cargo.toml` → None) |
| `CREST_WEBVIEW_PAGE` override env var | `src/shell/webview/window.rs:79,253` | INPUT-OVERRIDE | `cfg(debug_assertions)`-gated with a compile-out guard test; unreachable in a release binary |
| Forced-throw subprocess in tests | `tests/webview_projection_shell.rs:3226` | SHELL-INJECTION | None — fixed `CARGO_BIN_EXE` path, list-form args, no shell |

No new network calls, credential handling, or lock-scope changes. No blocking security finding.

---

## Structural Quality (adversarial-review)

**Pipeline report**: `kitty-specs/webview-render-fidelity-hardening-01KZCEF8/adversarial-review.md`
**Files reviewed**: 6 · **Raised**: 20 (merged to 13) · **Survived skeptic at Critical/Major**: 0 · **Minor**: 4 confirmed + 7 downgraded (itemized in the pipeline report)

No structural findings survived skeptic verification above Minor. The pipeline ran per contract: six blind hunters spawned in parallel in a single message; every merged finding sent to the skeptic under its kill mandate; two findings refuted outright, seven downgraded on latent/unreachable harm or mission-mandated design. Highest cross-hunter agreement (5 of 6 hunters) was the guard-scan allowance-dispatch cluster in `tests/component_vocabulary.rs` — confirmed factually, Minor in severity, and its duplication was explicitly mandated by research decision D6.

The four confirmed Minors touch FR-owning files, so they are listed (details and suggested techniques in the pipeline report; recorded, not applied):

- **SMELL-1** (LOW): scan allowances dispatched by file-name string comparison; px-detector copied between loops — `tests/component_vocabulary.rs:1122-1204`. One substantive residue: gallery.js is absent from the purity-needle scan (`Date.now`/`Math.random`/…) though its header claims those properties.
- **SMELL-2** (LOW): double-measure identity guard triplicated — `tests/webview_projection_shell.rs:2797-2861`.
- **SMELL-3** (LOW): unreachable `PaintedAck` match arm in the latch unit test claims a discrimination it never exercises — `src/shell/webview/window.rs:811-825`.
- **SMELL-4** (LOW, nit): dead `unwrap_or` fallback copied under the D6 verbatim mandate — `tests/component_vocabulary.rs:1192`.

**FR exposure**: none demonstrates a behavior defect; all four are maintainability signals in proof scaffolding. No smell corroborates or upgrades any Risk or Security finding.

---

## Final Verdict

**PASS**

### Verdict rationale

All seven FRs trace to adequate, falsifiable coverage; all three NFRs are met with measured margin (latency p95 12.6 ms against a 50 ms threshold in this review's independent re-run); all four constraints held, including the additive-only CSP guarantee verified per directive by an executable test. All four predecessor findings (RISK-1, RISK-2, DRIFT-1, DRIFT-2) are resolved with kill-tested proofs. This review independently re-ran every gate at HEAD — crest-spec doctor, the headless suites, the full live gated acceptance on real WKWebView windows (`skipped: none`), and the 631-test lib suite — all green. No drift findings, no CRITICAL or HIGH risk findings, no blocking security findings, and zero structural findings above Minor. Nothing blocks release.

### Open items (non-blocking, for the deferred hygiene mission)

1. **OBS-1**: gallery scene serves policy-free while `gallery.js` still paints via inline `style=` — convert to the data-attribute + CSSOM pattern or declare the policy-free serving as a property of the testing scene.
2. **SMELL-1 residue**: add `gallery.js` to the purity-needle scan in `tests/component_composition.rs:1790-1805`.
3. Predecessor review's deferred items (RISK-3, RISK-4, RISK-5, DRIFT-3) remain open by design (C-003) and still belong to that hygiene mission.
4. Optional cleanups: SMELL-2/3/4 and the per-source scan-descriptor refactor named in the pipeline report.

---

## Retrospective Reminder

The canonical post-merge sequence is: **mission review (this report) → author or verify retrospective → surface findings**.

The retrospective record exists — authored automatically at merge (`runtime_post_completion`, 2026-08-07T01:23:44Z) at both `kitty-specs/webview-render-fidelity-hardening-01KZCEF8/retrospective.yaml` and `.kittify/missions/01KZCEF8PV9K67ZMFTABBHXHK1/`; the event log records `RetrospectiveCaptured`. No escalation needed. To surface findings:

```bash
spec-kitty retrospect summary                                                        # cross-mission aggregation (read-only)
spec-kitty agent retrospect synthesize --mission webview-render-fidelity-hardening-01KZCEF8          # inspect proposals (dry-run)
spec-kitty agent retrospect synthesize --mission webview-render-fidelity-hardening-01KZCEF8 --apply  # apply proposals (mutates)
```
