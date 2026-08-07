# WP03 evidence wall — render fidelity and error-path proofs under the production CSP

Mission `webview-render-fidelity-hardening-01KZCEF8`, WP03 (spec FR-002,
FR-003, FR-004, FR-006, NFR-002, NFR-003; closes the cutover mission-review's
DRIFT-1 and re-proves the RISK-1/RISK-2 fixes under the shipped policy).

Rig: Mac15,6, macOS 26.5.2, arm64 — real WKWebView windows, physical
display, no other heavy load. All runs executed 2026-08-06 from the WP03
lane worktree at lane commit
**`d30250fcd563fc38d50aa326a2b9d277f869042b`**
(`tests/webview_projection_shell.rs`). Producing command:
`CREST_WEBVIEW_TESTS=1 cargo test --test webview_projection_shell --
--nocapture`; the committed run printed
`CREST_ACCEPTANCE webview_projection_shell passed (skipped: none)`
(full log: `acceptance-live-run.log`). Every log is committed complete and
untrimmed.

## The served policy — DRIFT-1 closed

Every artifact in this directory was collected with the page served under
the production `PAGE_CSP` via the exported single-source seam
`crest_synth::shell::webview::protocol_response`
(`src/shell/webview/window.rs`). The harness registers exactly that
function as its one `crest://` protocol handler for ALL live sections, and
the T010 parity section asserts the served document's
`Content-Security-Policy` header equals the exported `PAGE_CSP` constant —
compared against the constant itself, never a restated copy — plus
byte-identity of every served asset against the committed page. Quoted from
`src/shell/webview/window.rs` for the reader (the harness compares against
the constant, not this quotation):

```
default-src 'none'; script-src 'self'; style-src 'self'; font-src 'self';
connect-src ipc: http://ipc.localhost; base-uri 'none'; form-action 'none'
```

The prior evidence in `kitty-specs/webview-shell-cutover-01KZAC7Q/evidence/`
was measured without this policy (DRIFT-1) and stays immutable as the
historical record; this directory supersedes it under the corrected method
(research D5).

## Measured results (all from `acceptance-live-run.log`)

- **T010 harness policy parity** (FR-002): PASS — 9 assets served through
  the exported production seam, byte-identical to the committed page,
  document CSP equal to the exported `PAGE_CSP` constant, no CSP on
  subresources, unknown path 404.
- **T024 page-render determinism** (FR-003, NFR-003): PASS — double-render
  identical at 1920x1080 and 1280x800 for the MIXER document and all three
  PATCH documents, now under the production policy; T011 additionally
  double-measures the CSSOM-applied geometry identically (the geometry is
  part of the compared observation).
- **NFR-002 latency** (`requirement.serialized_projection_transport`):
  projection-to-paint over 150 paced reducer edits — one real reducer edit
  per 30 Hz meter interval through the production projector and emit path,
  the paced live demo workload — **p50 8.1 ms / p95 8.9 ms / max 11.7 ms**
  against the declared p95 ≤ 50 ms threshold, production policy served.
  (The WP01 CSSOM pass is included in this per-render work; no regression.)
- **Meter cadence soak** (60 s): 29.45 Hz sustained (declared 30 Hz pace,
  quantized floor 29.0), max gap 36.7 ms, 0 lost, page received 1767/1767.
- **T011 painted-geometry fidelity** (FR-004): PASS — measured
  `.fader-fill`/`.prow-position-fill` boxes proportional to document values
  at both viewports; the hex-73 fixture (the review's RISK-1 repro value,
  fraction 0.909091) strictly nonzero on all sixteen tracks; the
  driven-to-floor zero-level fixture measures zero WITH its CSSOM `--level`
  property applied (`data-level="0.000000"` + inline property present) —
  distinguishing value-zero from variable-never-applied; an element carrying
  the attribute without the applied property fails naming the element.
- **T012 forced render failure** (FR-006, SC-002): negative control — zero
  `crest://render-error` events across every healthy live section; the
  update-render throw (healthy documents painted and acked, then a
  subsequent projection's render throws) produced exactly one typed payload
  carrying the failing document's identity and NO painted ack; the
  unhandled promise rejection on a reloaded healthy page likewise; the
  forced FIRST-render throw on the shipped binary
  (`t012-forced-first-render-throw.log`) ended the process **exit 1** with
  exactly one distinct typed `PageRenderFailed` whose detail is the page's
  typed JSON payload (name `TypeError`, message, generation, stateHash),
  surfaced through `application window failed: webview page render failed:`.

## Falsifiability spot-checks (exercised at lane commit `d30250f`, scratch `webview-page/page.js` edits reverted immediately; only the passing state is committed)

- **T011** — with WP01's `applyDynamicGeometry(doc)` call disabled in a scratch tree, T024's readout-text assertions still passed and T011 failed with the RISK-1 signature naming all sixteen elements: `<div data-structure=LevelFader> carries data-level=0.909091 but no CSSOM --level property is applied`.
- **T012** — with WP01's try/catch boundary and window error/unhandledrejection reporters removed in a scratch tree, the suite failed with `T012 update-render throw produced no crest://render-error within 10s — the old silent-stale failure mode`.

## Artifact index

| Artifact | Proof section | Requirement |
|---|---|---|
| `acceptance-live-run.log` | Full suite under the production policy: T010 parity, T022/T023, T024 determinism, T011 geometry, NFR-001/NFR-002 measurements, T012 (all variants), T025, T026 | FR-002, FR-003, FR-004, FR-006, NFR-002, NFR-003 |
| `t011-mixer-level73-desktop-1920x1080.png` | T011 — sixteen fader fills painted at hex 73 beside matching readouts, 1920x1080 | FR-003 / SC-001 visual record |
| `t011-mixer-level73-compact-1280x800.png` | T011 — the same nonzero fills at 1280x800 | FR-003 / SC-001 visual record |
| `t011-mixer-level00-desktop-1920x1080.png` | T011 — zero-vs-never-applied fixture: focused T00 at `00` painting an empty fill, fifteen tracks at `73` painting full fills | FR-004 |
| `t012-forced-first-render-throw.log` | T012 — shipped-binary transcript: one distinct typed `PageRenderFailed` payload, exit 1 | FR-006 / SC-002 |
| `wp03-t012-forced-throw-index.html` | T012 — the forced-throw page variant (committed index with the workspace band removed; derived at run time by the test) | FR-006 |
| `README.md` | This index | — |

Screenshot filenames carry viewport and level per the T014 convention.
