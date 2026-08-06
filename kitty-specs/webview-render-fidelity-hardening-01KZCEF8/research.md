# Research: Webview Render Fidelity and Error-Path Hardening

All spec clarifications were resolved during specify/crest-spec; this records the technical decisions and their grounding. Defect locations verified against the working tree at plan time (they match the mission review's citations).

## D1 — Dynamic geometry mechanism: CSSOM post-insertion pass

- **Decision**: Emit the level/position as a `data-` attribute in the rendered HTML string, then run one post-insertion pass that reads those attributes and applies `element.style.setProperty('--level', …)` / `('--position', …)` via CSSOM.
- **Rationale**: CSP `style-src` governs *parsed* inline style attributes and `<style>` elements; CSSOM property assignment is exempt by spec, so `style-src 'self'` stays intact. The data attribute keeps the string-template render path (page.js builds HTML by concatenation) and gives the paint-fidelity proof a way to distinguish "value is zero" (attribute present, property applied, geometry zero) from "variable never applied" (attribute present, property missing).
- **Alternatives considered**:
  - `unsafe-inline` / hashed inline styles — forbidden by C-004; hashes don't cover attribute styles anyway.
  - Pure data-attribute + CSS `attr()` binding — CSS typed `attr()` for non-content properties is not reliably supported in WKWebView; rejected as the primary mechanism (remains the spec's declared fallback if a WKWebView CSSOM deviation surfaces).
  - Direct element construction (`createElement` + `style.setProperty`, no innerHTML) — larger rewrite of the render path than the defect warrants; violates the smallest-fix posture of a fix mission.

## D2 — Error emission channel and boundary shape

- **Decision**: Emit `RENDER_ERROR_EVENT` through `tauri.event.emit` — the same transport the painted ack already uses — from (a) a try/catch around the projection listener's `render(model)` call, (b) `window.onerror`, and (c) `window.onunhandledrejection`. Typed payload: error name, message, and the failing document's semantic identity when available. First error wins (latch); a failed render never acks.
- **Rationale**: The channel is proven CSP-compatible in production (`connect-src ipc: http://ipc.localhost` — painted acks flow through it live today), and the shell listener + `PageSignal::RenderError` + typed `PageRenderFailed` path already exist (`window.rs:344-356`); only the emitter is missing. The forced-throw test still asserts end-to-end delivery under the served policy rather than assuming it.
- **Alternatives considered**: a `crest://` fetch endpoint — new protocol surface for no benefit; console-capture on the Rust side — not typed, string-matching, rejected.

## D3 — Harness policy parity: export the production seam

- **Decision**: Make `protocol_response` (and `PAGE_CSP` behind it) the exported single-source seam in `src/shell/webview`, and route the acceptance harness's `register_uri_scheme_protocol` through it; add an assertion that the harness-served document's `Content-Security-Policy` header equals the production constant.
- **Rationale**: Both fns are currently private (`window.rs:92,142`); the integration-test crate cannot reach them, which is exactly how the harness drifted to a policy-free server. One exported seam with two callers (production window, harness) makes drift structurally impossible and satisfies "never a restated copy".
- **Alternatives considered**: duplicating the CSP string into the test — the restated-copy failure mode the crest-spec now explicitly forbids; a test-only `#[cfg(test)]` export — invisible to integration tests (separate crate compilation), doesn't work.
- **Note**: This is a new public API with a non-production caller — document its single-source purpose at the declaration so it is distinguishable from the review's RISK-5 dead-code pattern.

## D4 — Paint-fidelity proof method

- **Decision**: In the live harness section, render a fixture with known non-zero levels under the production policy, then assert measured fill geometry (computed style / bounding box of `.fader-fill` and `.prow-position-fill`) is proportional to the fixture value, and that a zero-value fixture measures zero — proving the property applied rather than defaulted.
- **Rationale**: This is the assertion that was structurally impossible to fail before (the harness had no CSP, and acks carry geometry+text only). Measuring painted boxes under the shipped policy is the only oracle that dies when RISK-1 regresses.

## D5 — Evidence refresh

- **Decision**: Re-run the affected proofs (T024 page-render determinism, 50 ms p95 latency, screenshots) with the production policy served and commit artifacts under `kitty-specs/webview-render-fidelity-hardening-01KZCEF8/evidence/`, following the cutover mission's `evidence/README.md` conventions (named logs + index).
- **Rationale**: The prior evidence in `kitty-specs/webview-shell-cutover-01KZAC7Q/evidence/` measured a laxer policy (DRIFT-1) and stays immutable as the historical record; this mission's evidence supersedes it under the corrected method.

## D6 — Gallery scan extension shape

- **Decision**: Add `gallery.js` to the no-input-handler loop (`tests/component_composition.rs:1801`) and `gallery.js` + `gallery.css` to the style-literal source list (`tests/component_vocabulary.rs:1107`); no new scan logic.
- **Rationale**: Both scans already implement the declared "everywhere" rule; the gap was enumeration only. Gallery digit-key bindings live Rust-side (`TestingContextModules`), so the input-handler scan should pass as-is; any hit is fixed in the gallery source, not exempted silently.
