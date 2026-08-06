---
work_package_id: WP01
title: Page geometry and error boundary under the shipped CSP
dependencies: []
requirement_refs:
- FR-001
- FR-005
planning_base_branch: feat/webview-shell-cutover
merge_target_branch: feat/webview-shell-cutover
branch_strategy: Planning artifacts for this mission were generated on feat/webview-shell-cutover. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/webview-shell-cutover unless the human explicitly redirects the landing branch.
created_at: '2026-08-06T22:44:12+00:00'
subtasks:
- T001
- T002
- T003
- T004
- T005
history:
- '2026-08-06: authored from plan IC-01 + IC-02 (page half), crest-spec asset WebviewProjectionPage, mission-review RISK-1/RISK-2'
agent_profile: frontend-freddy
authoritative_surface: webview-page/
create_intent: []
execution_mode: code_change
owned_files:
- webview-page/page.js
- webview-page/page.css
role: implementer
tags: []
tracker_refs: []
---

## ⚡ Do This First: Load Agent Profile

Before reading anything else in this prompt, load your assigned profile:

```
/ad-hoc-profile-load frontend-freddy
```

Adopt its identity, boundaries, and governance scope for the duration of this
work package.

## Objective

Fix the two page-side HIGH findings from the `webview-shell-cutover-01KZAC7Q`
mission review inside `webview-page/page.js`:

1. **RISK-1**: the page builds inline `style="--level:…"` / `style="--position:…"`
   attributes in its HTML strings. The production CSP (`style-src 'self'`, no
   `unsafe-inline`) blocks every such attribute in the shipped WKWebView, so
   all sixteen fader fills and every position indicator paint EMPTY while the
   readout text shows the true value. Replace the mechanism, not the policy.
2. **RISK-2**: `render(model)` is dispatched with no try/catch and no global
   error handler, so a page render exception is fully silent (stale DOM, no
   ack, no error). The shell side already listens (`RENDER_ERROR_EVENT` =
   `"crest://render-error"`) and converts the payload to the fatal typed
   `WebviewShellError::PageRenderFailed` (`src/shell/webview/window.rs:462`).
   Your job is the missing emitter.

Authorities: crest-spec asset `WebviewProjectionPage` (its prompts are your
contract — including the two new ones on CSSOM geometry and the error
boundary), `requirement.webview_projection_shell`, mission `spec.md`
(FR-001, FR-005; constraints C-001, C-002, C-004), `plan.md` IC-01/IC-02,
`research.md` D1/D2.

**Hard boundaries**: no CSP change from this WP (that is WP02's file); no
reducer/RT/projection-schema change; no new page-invented field; the page
still captures no input; determinism holds (same document → same DOM → same
paint). Do not edit `tests/webview_projection_shell.rs` (WP03 owns it) — you
may RUN it.

## Context: how the page renders today

`render(model)` (around `webview-page/page.js:1040-1064`) builds HTML strings
and assigns them to five region roots via `innerHTML`:

```
doc.getElementById("context-line").innerHTML = contextLineHtml(model);
doc.getElementById("identity-header").innerHTML = identityHeaderHtml(...);
doc.getElementById("workspace").innerHTML = ...;
doc.getElementById("inspector").innerHTML = sideRegionHtml(model);
doc.getElementById("footer").innerHTML = footerHtml(model);
```

The two inline-style emissions live in those strings:
- `page.js:509-518` — MIXER `LevelFader`: `'" style="--level:' + level + '"'`
- `page.js:677-684` — PATCH parameter row: `'style="--position:' + position.toFixed(6) + '"'`

`page.css` consumes `--level` / `--position` in its fill/cap rules. The
custom-property *names* and consuming CSS stay exactly as they are — only the
*delivery* of the values changes.

The projection transport (`page.js:1292-1311`, `attachTransports`) listens on
`PROJECTION_EVENT`, calls `render(model)`, then acks via
`tauri.event.emit(PAINTED_EVENT, paintedEvidence(model))` on the next
animation frame. `tauri.event.emit` is the proven CSP-compatible channel
(`connect-src ipc: http://ipc.localhost`) — the painted ack uses it in
production today.

## Subtasks

### T001 — Fader level via data attribute

At `page.js:509-518`, replace the inline style emission with a data
attribute. Keep the numeric formatting identical (`fraction(...).toFixed(6)`)
so document determinism and any byte-level fixture comparisons are unaffected:

```js
fader =
  '<div class="structure level-fader" data-structure="LevelFader" data-state="' +
  state +
  '" data-level="' +
  level +
  '">' + ...
```

The `unavailable` branch (no level in the view data) stays exactly as-is —
no `data-level` attribute at all. That absence is meaningful: it is how the
paint proof distinguishes "no data" from "value zero" (attribute `"0.000000"`
present, property applied, geometry zero).

**Validation**: grep `page.js` for `style="--level` → zero hits; the fader
markup carries `data-level` on exactly the branch that previously carried the
inline style.

### T002 — Position indicator via data attribute

Same transformation at `page.js:677-684`:

```js
'<div class="prow-position"><div class="prow-position-fill"' +
' data-position="' +
position.toFixed(6) +
'"></div></div>'
```

The `position === null` branch keeps emitting the bare `<span
class="spring">` — again, absence of the attribute means "no value", never
"zero".

**Validation**: grep `page.js` for `style="--position` → zero hits.

### T003 — CSSOM post-insertion geometry pass

Add one function and wire it into the end of `render(model)`, after ALL five
`innerHTML` assignments:

```js
// CSP: style-src 'self' blocks parsed inline style attributes, but CSSOM
// property assignment is exempt — dynamic geometry must go through here.
function applyDynamicGeometry(doc) {
  var levels = doc.querySelectorAll("[data-level]");
  for (var i = 0; i < levels.length; i++) {
    levels[i].style.setProperty("--level", levels[i].getAttribute("data-level"));
  }
  var positions = doc.querySelectorAll("[data-position]");
  for (var j = 0; j < positions.length; j++) {
    positions[j].style.setProperty("--position", positions[j].getAttribute("data-position"));
  }
}
```

Rules:
- Call it exactly once per `render(model)`, as the final step — every element
  carrying a data attribute gets its property applied in the same paint, on
  both the initial render and every re-render. If any other code path assigns
  region `innerHTML` outside `render` (search for `innerHTML` to confirm;
  gallery code is out of scope for this WP), it is a finding to report, not
  to silently patch.
- Pure function of the DOM: no state, no conditionals on prior renders —
  determinism (same document → same DOM → same computed geometry) is a
  declared proof target.
- ES5 style (`var`, plain loops) matching the existing file; no new
  dependencies, no build step.

**Validation**: serve the page under the production CSP (WP03 automates this;
manually: `make run` once WP02/WP03 land, or a local file with the CSP meta
tag) and confirm `.fader-fill` height tracks `data-level` and
`.prow-position-fill` tracks `data-position`.

### T004 — Error boundary in the projection listener

At `page.js:1292-1311`, wrap the render dispatch so a throwing document emits
the typed error and never acks:

```js
tauri.event.listen(PROJECTION_EVENT, function (event) {
  var model = event.payload;
  latestModel = model;
  try {
    render(model);
    updateMeter();
  } catch (error) {
    emitRenderError(error, model);
    return; // a failed render must NOT ack
  }
  window.requestAnimationFrame(function () {
    tauri.event.emit(PAINTED_EVENT, paintedEvidence(model));
  });
});
```

Add the constant and emitter near `PAINTED_EVENT` (`page.js:30`):

```js
var RENDER_ERROR_EVENT = "crest://render-error";
var renderErrorEmitted = false;

function emitRenderError(error, model) {
  if (renderErrorEmitted) {
    return; // first error wins; the shell treats the first payload as fatal
  }
  renderErrorEmitted = true;
  var tauri = window.__TAURI__;
  if (!tauri || !tauri.event) {
    return; // headless harness: the throw already propagated to the caller
  }
  tauri.event.emit(RENDER_ERROR_EVENT, {
    name: error && error.name ? String(error.name) : "Error",
    message: error && error.message ? String(error.message) : String(error),
    contextGeneration:
      model && model.contextGeneration !== undefined ? model.contextGeneration : null,
  });
}
```

(Match the payload's identity field to what `paintedEvidence` actually reads
from the model — use the same field name the ack uses for the document's
semantic identity; inspect `paintedEvidence` and reuse its accessor. The
shell treats the payload as opaque `detail`, so shape is page-owned — keep it
small and JSON-serializable.)

Also REPLACE the false comment at `page.js:1299-1301` ("render throwing here
… surfaces the exception to the adapter's typed render-exception path") —
that claim was the RISK-2 lie; the new comment should state that the catch
emits `crest://render-error` and suppresses the ack.

The boundary itself must be minimal: no rendering, no DOM access, no
formatting beyond the strings above — it must not be able to throw before
emitting.

**Validation**: temporarily make `render` throw on a flag; observe exactly one
emitted `crest://render-error` payload, no painted ack for that document, and
`window.crest.render` (headless path) still propagating the throw unchanged.

### T005 — Global onerror and unhandledrejection emitters

Register once at script setup (top level, near `attachTransports`' caller):

```js
window.addEventListener("error", function (event) {
  emitRenderError(event.error || { name: "Error", message: event.message }, latestModel);
});
window.addEventListener("unhandledrejection", function (event) {
  emitRenderError(
    event.reason instanceof Error
      ? event.reason
      : { name: "UnhandledRejection", message: String(event.reason) },
    latestModel
  );
});
```

These share the `renderErrorEmitted` latch — any uncaught page fault after
load is the same typed fatal condition. The `requestAnimationFrame` ack
callback (`paintedEvidence`) is now also covered: if IT throws, `onerror`
fires and the shell gets a typed failure instead of silence.

**Validation**: force `paintedEvidence` to throw once — a typed payload is
emitted; force a rejected promise — same.

## Definition of Done

- [ ] Zero `style="` substrings built anywhere in `page.js` (the guard scan in
      `tests/component_vocabulary.rs` must stay green; WP03/WP04 deepen it).
- [ ] `data-level`/`data-position` emitted exactly where the inline styles
      were; absence still means "no data".
- [ ] `applyDynamicGeometry` runs at the end of every `render`, ES5-clean,
      stateless.
- [ ] Projection listener catches, emits `crest://render-error` once (typed
      payload), never acks a failed render; false comment replaced.
- [ ] Global `error` + `unhandledrejection` handlers share the first-error
      latch.
- [ ] `cargo test --test webview_projection_shell -- --nocapture` and
      `cargo test --test component_vocabulary -- --nocapture` pass at your
      branch point (pre-WP03 versions — they must not regress).
- [ ] No changes outside `webview-page/page.js` / `webview-page/page.css`.

## Risks / Reviewer Guidance

- The geometry pass covering only SOME insertion paths is the subtle failure:
  verify `render` is the single place region `innerHTML` is assigned for
  projection content, and that the pass is its final statement.
- `toFixed(6)` formatting must not change — document byte-determinism proofs
  compare rendered documents.
- The emitter must be unreachable-before-latch-set only through the latch
  check itself (no early returns before setting `renderErrorEmitted = true`
  except the latch).
- Page still registers no key handler (input-boundary rule C-002).
