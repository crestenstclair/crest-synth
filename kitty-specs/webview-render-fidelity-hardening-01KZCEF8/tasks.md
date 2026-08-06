# Tasks: Webview Render Fidelity and Error-Path Hardening

**Mission**: `webview-render-fidelity-hardening-01KZCEF8`
**Branch contract**: planning base and merge target are both `feat/webview-shell-cutover`
**Input**: `plan.md` (IC-01…IC-05), `spec.md` (FR-001…FR-007, NFR-001…003, C-001…004), `research.md` (D1…D6)

## Subtask Index

| ID | Description | WP | Parallel |
|----|-------------|----|----|
| T001 | Fader level via data attribute, not inline style | WP01 | [P] |
| T002 | Position indicator via data attribute, not inline style | WP01 | [P] |
| T003 | CSSOM post-insertion geometry pass on every render path | WP01 | |
| T004 | Render error boundary in projection listener; fix false comment | WP01 | |
| T005 | Global onerror + unhandledrejection emitters | WP01 | |
| T006 | Harden PAGE_CSP: base-uri 'none'; form-action 'none' | WP02 | [P] |
| T007 | Export protocol_response/PAGE_CSP as documented single-source seam | WP02 | |
| T008 | Verify RenderError→PageRenderFailed typed nonzero exit, first-error-wins | WP02 | |
| T009 | Pin hardened policy in in-module tests; reject unsafe-inline | WP02 | |
| T010 | Harness serves through production seam; CSP parity assertion | WP03 | |
| T011 | Painted fader/position geometry proof under shipped policy | WP03 | |
| T012 | Forced render throw + rejection → typed nonzero exit test | WP03 | |
| T013 | Re-run determinism, latency, screenshot proofs under production policy | WP03 | |
| T014 | Commit refreshed evidence + README index | WP03 | |
| T015 | Add gallery.js to no-input-handler scan | WP04 | [P] |
| T016 | Add gallery.js/gallery.css to style-literal scan | WP04 | [P] |
| T017 | Run both guard suites; fix any gallery hit at the source | WP04 | |

## Work Packages

### WP01 — Page geometry and error boundary under the shipped CSP

- **Prompt**: `tasks/WP01-page-geometry-and-error-boundary.md` (~420 lines)
- **Goal**: `webview-page/page.js` paints fader fills/position indicators without inline style attributes (FR-001) and emits the typed render-error on any render throw or unhandled rejection (FR-005).
- **Priority**: P1 (User Stories 1 and 3)
- **Independent test**: Serve the page under the production CSP and observe non-empty fills matching readouts; throw inside render and observe the emitted typed event.
- **Subtasks**:

  T001 Fader level via data attribute (WP01)
  T002 Position indicator via data attribute (WP01)
  T003 CSSOM post-insertion geometry pass (WP01)
  T004 Render error boundary + comment fix (WP01)
  T005 Global onerror/unhandledrejection emitters (WP01)

- **Implementation sketch**: swap the two inline-style emissions for data attributes → add one `applyDynamicGeometry(root)` pass invoked after every DOM insertion → wrap the projection listener's render dispatch in the boundary → add global handlers → correct the false comment.
- **Dependencies**: none. **Parallel with**: WP02, WP04.
- **Risks**: geometry pass must cover every insertion path; failed render must not ack; determinism (same document → same DOM) must hold.

### WP02 — Shell CSP hardening, single-source seam, typed render-failure exit

- **Prompt**: `tasks/WP02-shell-csp-and-render-failure-exit.md` (~380 lines)
- **Goal**: `src/shell/webview/window.rs` ships the hardened policy (NFR-001), exports the single-source response seam WP03 serves through (FR-002 enabler), and turns the page's render-error event into the typed nonzero `PageRenderFailed` exit (FR-006 shell half).
- **Priority**: P1 (User Stories 2 and 3)
- **Independent test**: In-module tests pin the hardened policy string and the RenderError→typed-exit path.
- **Subtasks**:

  T006 Harden PAGE_CSP (WP02)
  T007 Export single-source seam (WP02)
  T008 RenderError→PageRenderFailed nonzero exit (WP02)
  T009 Pin hardened policy in tests (WP02)

- **Implementation sketch**: extend the `PAGE_CSP` string → make `protocol_response`/`PAGE_CSP` public with single-source doc comments → confirm/complete the `PageSignal::RenderError` handling into the typed fatal path with a first-error latch → update the in-module policy tests.
- **Dependencies**: none. **Parallel with**: WP01, WP04.
- **Risks**: the new public seam must be documented as deliberate (distinguish from RISK-5 dead-code pattern); do not touch the close path beyond first-error-wins (RISK-3 out of scope, C-003).

### WP03 — Harness production-policy parity, new proofs, evidence re-run

- **Prompt**: `tasks/WP03-harness-policy-parity-and-proofs.md` (~520 lines)
- **Goal**: `tests/webview_projection_shell.rs` serves the page through the production seam (FR-002), proves painted geometry under the shipped policy (FR-004), proves the typed nonzero exit on forced page failure (FR-006), and re-collects determinism/latency/screenshot evidence under the production policy (FR-003, NFR-002, NFR-003).
- **Priority**: P1 (User Story 2, completing 1 and 3)
- **Independent test**: `cargo test --test webview_projection_shell -- --nocapture` passes and prints its acceptance marker; deliberately reverting WP01's geometry fix makes the paint proof fail.
- **Subtasks**:

  T010 Serve through production seam + CSP parity assertion (WP03)
  T011 Painted geometry proof under shipped policy (WP03)
  T012 Forced throw/rejection → typed nonzero exit test (WP03)
  T013 Re-run determinism/latency/screenshot proofs (WP03)
  T014 Commit refreshed evidence + README index (WP03)

- **Implementation sketch**: replace the harness's bare protocol closure with the exported production response builder (or byte-equal parity assertion for the disk-override seam) → add the paint-fidelity live section (nonzero and zero fixture levels) → add the forced-failure section → re-run the affected proofs → write `evidence/` artifacts.
- **Dependencies**: **WP01, WP02** (proofs measure the fixed page under the hardened exported policy).
- **Risks**: paint proof must distinguish value-zero from variable-never-applied; evidence follows the cutover mission's `evidence/README.md` conventions; frozen baselines never loosened (C-002).

### WP04 — Gallery guard-scan coverage

- **Prompt**: `tasks/WP04-gallery-guard-scans.md` (~250 lines)
- **Goal**: The no-input-handler scan and style-literal scan cover `gallery.js`/`gallery.css` (FR-007).
- **Priority**: P3 (User Story 4)
- **Independent test**: Adding a `keydown` handler or hex color to a gallery source in a scratch tree fails the suites.
- **Subtasks**:

  T015 gallery.js in no-input-handler scan (WP04)
  T016 gallery.js/gallery.css in style-literal scan (WP04)
  T017 Run both suites; fix hits at the source (WP04)

- **Implementation sketch**: extend the source arrays at `tests/component_composition.rs:1801` and `tests/component_vocabulary.rs:1107` → run both suites → any hit is fixed in the gallery source or via a declared exemption mirroring the existing fader-geometry block, never a silent carve-out.
- **Dependencies**: none. **Parallel with**: WP01, WP02.
- **Risks**: minimal; gallery digit keys are bound Rust-side, so the input scan should pass unchanged.

## Sequencing

- **Wave 1 (parallel)**: WP01, WP02, WP04
- **Wave 2**: WP03 (after WP01 + WP02)
- **MVP**: WP01 alone makes the shipped window render truthfully; WP03 makes it provable.
