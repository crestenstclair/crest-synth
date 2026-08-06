# Implementation Plan: Webview Render Fidelity and Error-Path Hardening

**Branch**: `feat/webview-shell-cutover` | **Date**: 2026-08-06 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `kitty-specs/webview-render-fidelity-hardening-01KZCEF8/spec.md`

## Summary

Fix the two HIGH findings from the `webview-shell-cutover-01KZAC7Q` mission review plus their proof-gap enablers, and nothing else. (1) The production CSP (`style-src 'self'`, `src/shell/webview/window.rs:92`) blocks the JS-built inline `style` attributes carrying `--level` (`webview-page/page.js:514`) and `--position` (`webview-page/page.js:682`), so fader fills and position indicators paint empty in the shipped window — fix by applying dynamic geometry via CSSOM `style.setProperty` in a post-insertion pass, and harden the policy with `base-uri 'none'; form-action 'none'`. (2) The `crest://render-error` tauri event has a listener (`window.rs:354-356`) and a typed `PageRenderFailed` path but zero emitters — add a page-side error boundary emitting a typed payload on render throw and unhandled rejection, plus a forced-throw test asserting nonzero typed exit. (3) The acceptance harness (`tests/webview_projection_shell.rs:874-885`) serves the page with no CSP — route it through the production response builder (exported as the single-source seam), re-run the affected proofs, and add a falsifiable painted-geometry proof. (4) Extend the two guard scans to `gallery.js`/`gallery.css`.

## Technical Context

**Language/Version**: Rust (edition per workspace `Cargo.toml`, toolchain pinned by `rust-toolchain`/CI) for shell + tests; vanilla ES5-style JavaScript (`webview-page/page.js`, no build step, no framework) for the page
**Primary Dependencies**: Tauri v2 (WKWebView on macOS) — window, `crest://` custom protocol, event transport (`tauri.event.emit/listen` over `connect-src ipc:`); serde/serde_json for the serialized projection
**Storage**: N/A — evidence artifacts committed under `kitty-specs/webview-render-fidelity-hardening-01KZCEF8/evidence/` (same conventions as the cutover mission's `evidence/` directory)
**Testing**: Cargo integration-test targets invoked per crest-spec validations — `tests/webview_projection_shell.rs` (primary), `tests/component_vocabulary.rs`, `tests/component_composition.rs`; live check via `make run` / `make demo-live-graphical-shell`
**Target Platform**: macOS desktop (WKWebView); authored viewports 1920x1080 and 1280x800
**Project Type**: Single Rust crate with embedded webview page assets (`webview-page/`)
**Performance Goals**: Reducer state change visible in the webview within 50 ms p95 under the paced live demo workload, now measured with the production CSP served (`requirement.serialized_projection_transport`)
**Constraints**: CSP never weakened — no `unsafe-inline`/`unsafe-eval`/wildcards anywhere (C-004); page/harness/transport-side only — no reducer, RT, or projection schema changes (C-001); frozen baselines, token single-source, and input-boundary rules unchanged (C-002); review's structural/dead-code items out of scope (C-003)
**Scale/Scope**: 4 findings, ~5 files touched (`webview-page/page.js`, `src/shell/webview/window.rs`, `tests/webview_projection_shell.rs`, `tests/component_vocabulary.rs`, `tests/component_composition.rs`) plus refreshed evidence

## Charter Check

*GATE: passed.* Compact charter (software-dev-default template set; git + spec-kitty tooling). Relevant directives honored: DIRECTIVE_001 (architectural integrity — fixes stay inside the declared adapter/page/harness boundary; the one new public seam is the single-source policy response builder, justified below), DIRECTIVE_003 (decisions recorded in research.md), RECONCILE_CHANGE_SCOPE_TENSIONS (scope deliberately narrowed to the four findings; hygiene items deferred to a separate mission). No conflicts found; no Complexity Tracking entries needed.

## Crest-Spec Derivation

Authored in the `/spec-kitty.crest-spec` phase (commit `07cf450`), `crest_spec_impact: structural` (tightenings only — no resources added or retired):

- **Changed declarations**:
  - `requirement.webview_projection_shell` — dynamic geometry paints under the production CSP without JS-built inline style attributes; render throw/unhandled rejection ends the process typed and nonzero.
  - `requirement.serialized_projection_transport` — 50 ms p95 latency measured with the production policy served.
  - `requirement.graphical_shell_behavioral_proof` — harness serves the identical policy from the single source; painted-fader-geometry and forced-throw proofs named.
  - `validation.webview_projection_shell` — description extended to cover policy parity, painted geometry, typed render-failure exit.
- **Assets → files**:
  - `WebviewShellModules` → `src/shell/webview/window.rs` (CSP hardening incl. `base-uri`/`form-action` denial; typed nonzero render-failure exit; policy as the single served source).
  - `WebviewProjectionPage` → `webview-page/page.js` (CSSOM dynamic geometry; error boundary emitting the render-error channel).
  - `WebviewProjectionShellAcceptanceTests` → `tests/webview_projection_shell.rs` (production-policy serving, paint-fidelity proof, forced-throw proof, re-run evidence).
  - `ComponentVocabularyAcceptanceTests` / `ComponentCompositionAcceptanceTests` → `tests/component_vocabulary.rs`, `tests/component_composition.rs` (gallery sources join the already-declared "everywhere" scans — predeclared; no crest-spec edit needed).
- **Validations/witnesses covering the change**: `validation.webview_projection_shell`, `validation.component_vocabulary`, `validation.component_composition`, `validation.graphical_application_shell` (unchanged command surfaces; deepened assertions), rolled up by `evidence.graphical_application_shell_contract` and `evidence.component_vocabulary_contract`.
- `data-model.md` / `contracts/`: not produced (forbidden — crest-spec exists).

## Project Structure

### Documentation (this mission)

```
kitty-specs/webview-render-fidelity-hardening-01KZCEF8/
├── plan.md              # This file
├── research.md          # Phase 0 output — decisions with rationale
├── quickstart.md        # Phase 1 output — how to run the affected proofs
└── evidence/            # Created during implementation: re-run proof artifacts
```

### Source Code (repository root)

```
src/shell/webview/
└── window.rs            # PAGE_CSP (l.92) + protocol_response (l.142): harden policy,
                         #   export the single-source response seam; typed PageRenderFailed
                         #   exit already wired from RENDER_ERROR_EVENT (l.354-356)
webview-page/
├── page.js              # l.509-518 --level, l.677-684 --position: drop inline style=,
│                        #   emit data attributes + CSSOM setProperty pass; error boundary
│                        #   around render dispatch (l.1292-1311) + onerror/unhandledrejection
├── page.css             # stylesheet-side hooks if the geometry pass needs them
├── gallery.js           # untouched — joins the guard scans
└── gallery.css          # untouched — joins the guard scans
tests/
├── webview_projection_shell.rs   # l.873-885 harness: serve via production seam; new
│                                 #   paint-fidelity + forced-throw sections; re-run T024
│                                 #   determinism, latency, screenshots under the policy
├── component_vocabulary.rs       # l.1100-1117 style-literal scan: + gallery.js, gallery.css
└── component_composition.rs      # l.1801-1808 no-input-handler scan: + gallery.js
```

**Structure Decision**: Single-crate layout unchanged; every touched path is inside the surfaces the three webview assets already own. No new modules, no new test targets — deepened assertions inside the existing named targets keep the crest-spec validation commands identical.

## Implementation Concern Map

> Implementation concerns are NOT work packages. `/spec-kitty.tasks` translates these into executable WPs.

### IC-01 — CSP-conformant dynamic geometry

- **Purpose**: Make fader fills and position indicators paint under `style-src 'self'` by removing JS-built inline `style` attributes.
- **Relevant requirements**: FR-001; C-002, C-004; `requirement.webview_projection_shell`
- **Affected surfaces**: `webview-page/page.js` (l.509-518, l.677-684, plus the post-insertion pass), `webview-page/page.css` if needed
- **Sequencing/depends-on**: none
- **Risks**: The page renders via HTML string concatenation then insertion — the geometry pass must run after every insertion path (initial render, re-render), not just one. Keep determinism: same document → same DOM → same computed geometry. Distinguish value-zero from variable-never-applied (data attribute present but property unset must be impossible).

### IC-02 — Page error boundary and typed render-failure exit

- **Purpose**: Give the existing `RENDER_ERROR_EVENT` listener (`window.rs:354-356`) its missing emitter so a page render exception ends the shell typed and nonzero.
- **Relevant requirements**: FR-005, FR-006; `requirement.webview_projection_shell`
- **Affected surfaces**: `webview-page/page.js` (projection listener l.1297-1311, global `onerror`/`unhandledrejection`, the false comment at l.1299-1301), `src/shell/webview/window.rs` (confirm `PageSignal::RenderError` → `PageRenderFailed` nonzero exit path; harden first-error-wins)
- **Sequencing/depends-on**: none
- **Risks**: Emission uses the same `tauri.event.emit` channel as the painted ack (`connect-src ipc:` — already CSP-permitted, verified live by acks working in production); the forced-throw test must still assert emission survives the served policy. Boundary must not ack a failed render. Repeated errors keep the first typed error (adjacent RISK-3 is out of scope — do not touch the close path beyond this).

### IC-03 — Harness production-policy parity and re-run proofs

- **Purpose**: Serve the acceptance harness through the production response seam so every proof measures the shipped policy, then re-collect the affected evidence.
- **Relevant requirements**: FR-002, FR-003, FR-004; NFR-002, NFR-003; `requirement.graphical_shell_behavioral_proof`
- **Affected surfaces**: `src/shell/webview/window.rs` (export `protocol_response`/`PAGE_CSP` as one public single-source seam — currently private; the harness's disk-loaded `PageAssets` must flow through it or assert byte-equality against it), `tests/webview_projection_shell.rs` (l.873-885 protocol registration; new paint-fidelity section; re-run T024 determinism, 50 ms p95 latency, screenshots), `kitty-specs/webview-render-fidelity-hardening-01KZCEF8/evidence/`
- **Sequencing/depends-on**: IC-01 (paint proof must pass against fixed geometry), IC-04 (evidence must reflect the final hardened policy)
- **Risks**: The exported seam is a new public API with a test consumer — document it as the deliberate single-source policy boundary so it does not read as RISK-5-style dead code. Harness serves disk assets for the override seam; parity assertion must close that gap.

### IC-04 — CSP hardening directives

- **Purpose**: Add `base-uri 'none'; form-action 'none'` to `PAGE_CSP` while touching the policy; never weaken any directive.
- **Relevant requirements**: NFR-001; C-004
- **Affected surfaces**: `src/shell/webview/window.rs` (l.92-93 and its in-module policy tests, l.676-696)
- **Sequencing/depends-on**: none (IC-03 evidence depends on it)
- **Risks**: Minimal — additive directives; the executable policy check must pin the hardened string and reject `unsafe-inline` anywhere.

### IC-05 — Gallery guard-scan coverage

- **Purpose**: Extend the no-input-handler scan and the style-literal scan to `gallery.js`/`gallery.css` so the clean-today gallery cannot drift.
- **Relevant requirements**: FR-007
- **Affected surfaces**: `tests/component_composition.rs` (l.1801-1808), `tests/component_vocabulary.rs` (l.1100-1117)
- **Sequencing/depends-on**: none
- **Risks**: If a scan hits a legitimate gallery construct, the fix is in the gallery source or a declared exemption block mirroring the existing fader-geometry exemption — never a silent scan carve-out.
