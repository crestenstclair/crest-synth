---
wp_id: WP01
reviewer_agent: reviewer-renata
cycle: 1
verdict: approved
date: 2026-08-06
---

# WP01 Review — Page geometry and error boundary under the shipped CSP

Reviewed commit `22e3c50` on `kitty/mission-webview-render-fidelity-hardening-01KZCEF8-lane-a`
(1 commit over mission base `10d9567`). Diff: `webview-page/page.js` only,
+95/-7. Verified against FR-001/FR-005, constraints C-001/C-002/C-004,
research D1/D2, and the WP prompt's Definition of Done.

## Static verification (all mechanical, in the lane worktree)

- **No JS-built inline styles**: `grep 'style="' webview-page/page.js` → zero
  hits. The two former emissions (`page.js:515` LevelFader, `page.js:683`
  position fill) now carry `data-level` / `data-position` with the identical
  `fraction(...).toFixed(6)` / `position.toFixed(6)` formatting (byte
  determinism preserved; confirmed `"0.450000"` strings in the probe run).
- **Absence vs zero**: the `unavailable` fader branch (`page.js:520-526`)
  emits no `data-level`; the `position === null` branch (`page.js:681`) keeps
  the bare `<span class="spring">`. Zero emits `data-level="0.000000"` and
  gets the property applied — probe confirmed `--level: "0.000000"` set.
- **Single insertion point**: `grep innerHTML|insertAdjacentHTML|outerHTML` →
  only the five region assignments at `page.js:1082-1092`, all inside
  `render(model)`; `applyDynamicGeometry(doc)` (`page.js:1096`) is the final
  statement after all five. No other DOM-insertion path exists.
- **Geometry pass purity**: `applyDynamicGeometry` (`page.js:1055-1070`) is a
  pure function of the DOM — no state, no memory of prior renders, ES5 style.
- **Error boundary**: projection listener (`page.js:1360-1375`) wraps
  `render` + `updateMeter` in try/catch; catch calls `emitRenderError` and
  returns before the RAF ack is scheduled. The false "already surfaces"
  comment is replaced with an accurate one.
- **Payload identity**: `emitRenderError` (`page.js:1338-1355`) emits
  `crest://render-error` with `{name, message, generation, stateHash}` —
  exactly the accessors `paintedEvidence` leads with (`page.js:1310-1311`),
  per the prompt's instruction to reuse the ack's identity fields. Matches
  the shell listener (`src/shell/webview/window.rs:354`, payload opaque,
  converted to typed `WebviewShellError::PageRenderFailed` at line 462).
- **Latch discipline**: the only return before `renderErrorEmitted = true` is
  the latch check itself; the headless no-tauri return is after the latch and
  documented. Global `error` + `unhandledrejection` handlers
  (`page.js:1391-1404`) share the same latch.
- **No input capture / no color literals**: `addEventListener` appears only
  for `error`/`unhandledrejection` (fault reporters, not input); no key
  handlers; no CSS or color values added; `page.css` untouched (its
  `var(--level, 0)` / `var(--position, 0)` consumers unchanged).
- **Blast radius**: no changes outside `webview-page/`; no CSP, reducer, RT,
  schema, or test-file edits. `tests/webview_projection_shell.rs` untouched.

## Dynamic verification

Required suites at `22e3c50` (worktree, headless): `component_vocabulary`
11/11 PASS, `component_composition` 15/15 PASS, `webview_projection_shell`
PASS (T022/T023/T025; env-gated layers skipped).

Full gated live run (`CREST_WEBVIEW_TESTS=1 cargo test --test
webview_projection_shell`) — **passed with nothing skipped** on the WP01
page in a real WKWebView: T024 DOM determinism at both viewports/contexts,
WP01 paint-acknowledgment identity (one ack per document, identity verbatim),
NFR-001 projection-to-paint p50=5.7ms p95=6.5ms (threshold 50ms), NFR-002
60s meter soak 29.42 Hz with 0 lost, T026 real-window shutdown parity exit 0.

Node stub harness (window/document/__TAURI__ stubs, production `page.js`
evaluated verbatim; committed production MIXER document
`spike/webview-mixer/view-model.json` as the success fixture):

- **Failed render** (null document): listener catches (nothing propagates),
  exactly one `crest://render-error` emitted with payload
  `{"name":"TypeError","message":"Cannot read properties of null (reading
  'surfaces')","generation":null,"stateHash":null}`, zero painted acks, zero
  RAF callbacks scheduled.
- **First-error-wins**: two subsequent faults through the global handlers
  emitted nothing further (counts stayed at one).
- **Global handler alone** (fresh process, simulating a RAF-ack throw): one
  typed payload emitted.
- **Headless contract**: `window.crest.render(null)` still propagates the
  throw unchanged.
- **Successful render**: completes, zero error emissions, exactly one painted
  ack after RAF flush with identity `generation=2,
  stateHash=2b6c844cc65cb9ff`; geometry pass applied `--level` (including
  the zero case) and `--position` via `setProperty`; rendered workspace HTML
  contains `data-level=` and no `style="`.

## Disable-the-mechanism probes (all reverted; worktree left clean)

- **D1 — base page.js** (mission-base version through the same harness):
  failed render THREW out of the listener uncaught with ZERO emissions (the
  exact RISK-2 silence), and the success path applied no properties while
  emitting inline `style="` in the workspace HTML (the exact RISK-1 defect).
  Base ack identity matched the fixed version byte-for-byte — the change is
  paint-delivery only.
- **D2 — `applyDynamicGeometry(doc)` call removed** (temporary edit):
  success render still acks, but no `--level`/`--position` property is ever
  applied while `data-level` sits in the DOM — fills degrade to empty exactly
  as the spec predicts. Reverted; `git status` clean.
- **D3 — planted `style="--level:0.5"` literal**: current
  `component_vocabulary` scan stays green (it scans visual literals —
  colors/spacing/type — not style attributes). This is the documented
  pre-WP03/WP04 state: the WP01 DoD requires only that the existing scan
  stays green and names WP03/WP04 as the deepening owners (FR-004 falsifiable
  paint proof is WP03's; scan extension is WP04's). Not a WP01 defect;
  recorded so WP03's reviewer verifies the kill test actually lands.

## Anti-pattern checklist

1. Dead code — PASS (`applyDynamicGeometry` called from `render`;
   `emitRenderError` called from the boundary catch and both global handlers).
2. Synthetic-fixture test — N/A (WP01 adds no tests; probes above drove the
   production `page.js` verbatim with the committed production document).
3. Silent empty return — PASS (two early returns in `emitRenderError`, both
   documented: first-error latch; headless-harness no-transport).
4. FR coverage — N/A by mission decomposition (FR-001/FR-005 named proofs are
   WP03 deliverables; existing suites plus the full gated live run are green
   at this commit).
5. Frozen surface — PASS (diff touches only `webview-page/page.js`, an
   `owned_files` entry; no CSP, no test files, no `src/`).
6. Locked decision — PASS (no `unsafe-inline`, no policy edit, no key
   handler, no page-invented payload field — identity fields reuse the ack's
   accessors).
7. Shared-file ownership — PASS (lane-a is WP01-exclusive; WP03 depends on
   WP01 and only runs, never edits, the page).
8. Production fragility — PASS (no new throw sites; the boundary only
   catches and emits).

## Verdict

**APPROVED.** Both HIGH findings are mechanically fixed in the page, the fix
survives the production CSP in a real live-window run, and every
disable-the-mechanism probe degraded exactly as the spec predicts.
