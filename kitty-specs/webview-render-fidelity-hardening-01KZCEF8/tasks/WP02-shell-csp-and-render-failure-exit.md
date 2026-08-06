---
work_package_id: WP02
title: Shell CSP hardening, single-source seam, typed render-failure exit
dependencies: []
requirement_refs:
- FR-002
- FR-006
- NFR-001
planning_base_branch: feat/webview-shell-cutover
merge_target_branch: feat/webview-shell-cutover
branch_strategy: Planning artifacts for this mission were generated on feat/webview-shell-cutover. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/webview-shell-cutover unless the human explicitly redirects the landing branch.
created_at: '2026-08-06T22:44:12+00:00'
subtasks:
- T006
- T007
- T008
- T009
history:
- '2026-08-06: authored from plan IC-02 (shell half) + IC-04, crest-spec asset WebviewShellModules, mission-review RISK-1/RISK-2 + security note'
agent_profile: implementer-ivan
authoritative_surface: src/shell/webview/
create_intent: []
execution_mode: code_change
owned_files:
- src/shell/webview/window.rs
- src/shell/webview/mod.rs
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
work package.

## Objective

Three shell-side changes in `src/shell/webview/`:

1. Harden `PAGE_CSP` with `base-uri 'none'; form-action 'none'` (mission
   review security note; NFR-001). Never weaken any directive (C-004).
2. Export the production `crest://` response builder as the documented
   single-source policy seam so the acceptance harness (WP03) can serve the
   page EXACTLY as production does — the drift that hid RISK-1 (DRIFT-1) was
   the harness re-implementing this privately without the CSP header.
3. Verify and harden the `crest://render-error` → typed
   `WebviewShellError::PageRenderFailed` → nonzero-exit path that WP01's new
   page emitter will start exercising (FR-006, shell half).

Authorities: crest-spec asset `WebviewShellModules` (its prompts are your
contract — including the hardened-CSP and typed-nonzero-render-failure
prompts), `requirement.webview_projection_shell`, mission `spec.md` (NFR-001,
FR-006, C-003/C-004), `plan.md` IC-02/IC-04, `research.md` D2/D3.

**Hard boundaries**: no reducer/RT/projection-schema change; do NOT touch the
window-close/teardown path beyond the first-error concern below — the
review's RISK-3 (double close-failure) is explicitly out of scope for this
mission (C-003). Do not edit `webview-page/*` (WP01) or
`tests/webview_projection_shell.rs` (WP03).

## Context: what exists today

- `window.rs:92-93`:
  ```rust
  const PAGE_CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; \
       font-src 'self'; connect-src ipc: http://ipc.localhost";
  ```
- `window.rs:142-158` — private `fn protocol_response(path, index_html)`:
  resolves `page_asset` and attaches `PAGE_CSP` to `text/html` responses only.
- `window.rs:344-356` — the tauri listeners relaying `PAINTED_EVENT` and
  `RENDER_ERROR_EVENT` (`"crest://render-error"`, declared in this module
  tree) into the `PageSignal` channel.
- `window.rs:462` — `PageSignal::RenderError` already becomes
  `WebviewShellError::PageRenderFailed { detail }` on the fatal runtime path
  (`src/shell/webview/mod.rs:104`). The path exists; it has simply never had
  an emitter (RISK-2). Doc comments at `window.rs:39-40` and
  `projection_channel.rs:91` describe the intent.
- In-module policy tests around `window.rs:670-700` assert the served header
  equals `PAGE_CSP`, non-HTML assets carry no CSP, and the policy starts with
  `default-src 'none'`.

## Subtasks

### T006 — Harden PAGE_CSP

Extend the constant (additive only — every existing directive byte-for-byte
unchanged):

```rust
const PAGE_CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; \
     font-src 'self'; connect-src ipc: http://ipc.localhost; \
     base-uri 'none'; form-action 'none'";
```

Update the doc comment (`window.rs:82-91`): `base-uri`/`form-action` do not
inherit from `default-src`, so they are denied explicitly; and note that the
page's dynamic geometry is applied via CSSOM (WP01) precisely because
`style-src 'self'` blocks inline style attributes — the policy is never
weakened to make the page paint.

**Validation**: `cargo test --test webview_projection_shell` (pre-WP03
version) and the in-module tests still pass after T009 updates them.

### T007 — Export the single-source policy seam

Make `protocol_response` and `PAGE_CSP` reachable from integration tests
(they are private today — that privacy is WHY the harness drifted):

1. `pub fn protocol_response(...)` and `pub const PAGE_CSP` in `window.rs`,
   re-exported through `src/shell/webview/mod.rs` (follow the module's
   existing re-export style, e.g. alongside the `WebviewShellError` export).
2. Doc-comment the seam explicitly as the SINGLE SOURCE both the production
   window and the acceptance harness serve through, citing
   `requirement.graphical_shell_behavioral_proof` ("policy parity asserted
   from the single policy source"). This documentation is what
   distinguishes a deliberate two-caller seam from the review's RISK-5
   dead-public-API pattern — without it a future reviewer deletes the export.
3. Confirm the signature is harness-usable: it takes `path: &str` and
   `index_html: &str` and returns `tauri::http::Response<Vec<u8>>` — exactly
   what `register_uri_scheme_protocol` closures need. Do not add parameters
   for the harness's benefit; the harness adapts to production, never the
   reverse.

**Validation**: `use crest_synth::shell::webview::{protocol_response, PAGE_CSP};`
compiles from an integration-test context (spot-check with `cargo check
--tests`; adjust the path to the crate's actual public module route).

### T008 — Typed nonzero render-failure exit, first-error-wins

Read the full journey of `PageSignal::RenderError` from `window.rs:354` to
process exit and make these properties hold (most likely they already do —
prove it, fix only what falls short):

1. **Typed**: the payload string lands in
   `WebviewShellError::PageRenderFailed { detail }` verbatim — no parsing, no
   string-matching on console output.
2. **Fatal and nonzero**: the error reaches the same fatal runtime path a
   startup failure takes (doc comment `window.rs:39-41` promises this) and
   the process exits nonzero. Trace it to the actual exit: `WindowError` →
   application error surface → `main`'s exit code.
3. **First error wins**: if multiple `RenderError` signals arrive (the page
   latches, but the channel does not know that), the FIRST payload is the
   one reported; later signals must not overwrite the recorded failure or
   disturb teardown. Add a latch on the receiving side if the current select
   loop would process a second one destructively. Keep this narrowly on the
   render-error handling — the close-failure freeze (RISK-3) is out of
   scope.
4. **No ack conflation**: a `RenderError` for a document must not be
   creditable as that document's painted ack anywhere in the forwarding path.

Add or extend an in-module unit test driving a synthetic
`PageSignal::RenderError` through the handling to assert (1) and (3) without
a live webview.

**Validation**: unit test green; the end-to-end nonzero-exit assertion lands
in WP03 (T012) — your job is that the shell half is provably correct in
isolation.

### T009 — Pin the hardened policy in the in-module tests

Update the policy tests around `window.rs:670-700`:

1. The served-header equality test keeps passing (it compares against the
   constant, so it should be unchanged — verify).
2. Extend the structural assertion (`starts_with("default-src 'none'")`) to
   pin the full hardened policy: contains `base-uri 'none'` and
   `form-action 'none'`; contains NO `unsafe-inline`, `unsafe-eval`, or `*`
   source in any directive. Write it as directive-level checks, not one
   brittle full-string equality, so a future ADDITIVE hardening does not
   break the test but any weakening does.
3. Keep the non-HTML-assets-carry-no-CSP assertion as-is.

**Validation**: `cargo test` for the crate's unit tests passes; deliberately
inserting `unsafe-inline` into the constant fails the new assertion.

## Definition of Done

- [ ] `PAGE_CSP` carries `base-uri 'none'; form-action 'none'`; all prior
      directives byte-identical; doc comment updated.
- [ ] `protocol_response` + `PAGE_CSP` publicly re-exported with
      single-source doc comments naming both callers and the crest-spec
      requirement.
- [ ] RenderError→PageRenderFailed proven typed, fatal, nonzero,
      first-error-wins by an in-module test; no teardown changes beyond it.
- [ ] Policy tests pin the hardened directives and reject any weakening.
- [ ] `cargo test --test webview_projection_shell -- --nocapture` (current
      version) still green; `cargo clippy` clean; no changes outside
      `src/shell/webview/window.rs` + `mod.rs`.

## Risks / Reviewer Guidance

- The export is the one architectural change — check the doc comment
  justifies it as the anti-DRIFT-1 seam, and that nothing else became public
  as a side effect.
- Directive-level policy assertions: reviewers should try to sneak
  `unsafe-inline` into any directive and watch T009's test fail.
- Confirm the first-error latch cannot starve the painted-ack channel (they
  share the `PageSignal` mpsc — the latch must drop later RenderErrors, not
  block the receiver).
