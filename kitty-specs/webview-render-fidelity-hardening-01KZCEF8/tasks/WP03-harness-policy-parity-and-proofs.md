---
work_package_id: WP03
title: Harness production-policy parity, new proofs, evidence re-run
dependencies:
- WP01
- WP02
requirement_refs:
- FR-002
- FR-003
- FR-004
- FR-006
- NFR-002
- NFR-003
planning_base_branch: feat/webview-shell-cutover
merge_target_branch: feat/webview-shell-cutover
branch_strategy: Planning artifacts for this mission were generated on feat/webview-shell-cutover. During /spec-kitty.implement this WP branches from a base containing WP01 and WP02, and completed changes must merge back into feat/webview-shell-cutover unless the human explicitly redirects the landing branch.
created_at: '2026-08-06T22:44:12+00:00'
subtasks:
- T010
- T011
- T012
- T013
- T014
history:
- '2026-08-06: authored from plan IC-03, crest-spec asset WebviewProjectionShellAcceptanceTests, mission-review DRIFT-1 + FR-coverage re-run mandate'
agent_profile: implementer-ivan
authoritative_surface: tests/
create_intent:
- kitty-specs/webview-render-fidelity-hardening-01KZCEF8/evidence/README.md
execution_mode: code_change
owned_files:
- tests/webview_projection_shell.rs
- kitty-specs/webview-render-fidelity-hardening-01KZCEF8/evidence/**
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

Close DRIFT-1 and make both HIGH findings permanently falsifiable. The
acceptance harness in `tests/webview_projection_shell.rs` currently serves
the page through its own bare protocol closure with NO Content-Security-Policy
(`tests/webview_projection_shell.rs:873-885`), so every fidelity, determinism,
and latency proof measured a laxer policy than production ships — which is
exactly how empty fader fills passed acceptance. You will: serve through the
production seam WP02 exported, assert policy parity, add the painted-geometry
proof and the forced-failure proof, re-run the affected proofs, and commit the
refreshed evidence.

Authorities: crest-spec asset `WebviewProjectionShellAcceptanceTests` (its
prompts are your contract — three new ones cover exactly this WP),
`requirement.graphical_shell_behavioral_proof`,
`requirement.serialized_projection_transport` (50 ms p95 under the production
policy), `validation.webview_projection_shell`, mission `spec.md` (FR-002/003/
004/006, NFR-002/003, C-002), `plan.md` IC-03, `research.md` D3/D4/D5.

**Hard boundaries**: no production-source edits (WP01/WP02 own those; if the
proofs reveal a production defect, STOP and report — do not patch around it);
no frozen-baseline loosening; the test target keeps its name, its
`CREST_ACCEPTANCE webview_projection_shell passed` marker, and its
no-vacuous-pass rules (no ignored tests, no env-dependent skip that widens,
no pre-assertion marker).

## Context: the harness today

`run_live_sections` (`tests/webview_projection_shell.rs:863-894`) builds a
tauri app with:

```rust
.register_uri_scheme_protocol("crest", move |_context, request| {
    match assets.resolve(request.uri().path()) {
        Some((content_type, body)) => tauri::http::Response::builder()
            .header("Content-Type", content_type)
            .body(body)...,           // ← NO CSP header. This is DRIFT-1.
        None => ...404...
    }
})
```

`PageAssets::load(manifest)` reads `webview-page/*` from disk (so the harness
can exercise the local page-override seam). WP02 exported the production
`protocol_response(path, index_html)` + `PAGE_CSP` from
`src/shell/webview` for exactly this call site.

Existing sections you will re-run, not rewrite: page-render determinism
("T024" in the cutover mission's numbering), the 50 ms p95 latency
measurement, and the screenshot captures. Prior evidence conventions:
`kitty-specs/webview-shell-cutover-01KZAC7Q/evidence/README.md` (named logs +
index table). That old evidence is the immutable historical record — yours
supersedes it under the corrected method in THIS mission's `evidence/`.

## Subtasks

### T010 — Serve through the production seam; assert policy parity

1. Replace the bare closure with the production builder. The harness serves
   disk-loaded assets (override seam) while production embeds them — so
   either:
   - **(a) preferred**: call `protocol_response(request.uri().path(),
     &disk_index_html)` directly — it already takes the index document as a
     parameter; confirm the other assets it serves (embedded) are
     byte-identical to the disk copies and assert that in a test, or
   - **(b) fallback** if (a) can't serve disk copies of css/js: keep the
     disk resolution but attach the header from the exported `PAGE_CSP`
     constant on `text/html` responses, mirroring `protocol_response`'s
     content-type rule, AND add the parity assertion below so a future
     divergence in attach-logic fails loudly.
2. Parity assertion (both routes): fetch/build the `/index.html` response in
   the test and assert its `Content-Security-Policy` header is EXACTLY
   `crest_synth::shell::webview::PAGE_CSP` — the constant, not a copied
   string literal. Also assert non-HTML responses carry no CSP (mirror of the
   in-module test, now proven at the harness boundary).
3. Every live section (determinism, latency, screenshots, paint proof,
   forced-failure) must run against this serving path — one protocol
   registration, no per-section variants.

**Validation**: the suite passes; deleting the CSP header attachment makes
the parity assertion fail (try it, revert it).

### T011 — Painted-geometry proof under the shipped policy

The proof that dies when RISK-1 regresses. In the live section, render a
MIXER fixture with known level values through the production projection path
and measure ACTUAL painted geometry in the page (via the harness's existing
JS-evaluation/observation channel):

1. **Nonzero case**: a track with level ≈ 0.9 (the review's hex 73 repro) —
   assert the measured `.fader-fill` box height is proportional to the
   `--level` custom property's resolved value within a small tolerance, and
   strictly greater than zero. Same for a PATCH `.prow-position-fill` width
   against `--position`.
2. **Zero vs never-applied**: a track with level exactly 0 — assert
   `data-level="0.000000"` is present AND
   `getComputedStyle(el).getPropertyValue("--level")` resolves to the applied
   value (property applied, geometry legitimately zero). Then assert the
   inverse guard: for every element carrying `data-level`/`data-position`,
   the corresponding custom property IS set on its inline CSSOM style
   (`el.style.getPropertyValue(...)` non-empty). An element with the
   attribute but no applied property — the exact RISK-1 signature — fails
   with a message naming the element.
3. Run at both authored viewports (Desktop 1920x1080, SteamDeck 1280x800)
   like the neighboring sections.

**Validation (falsifiability)**: temporarily revert WP01's
`applyDynamicGeometry` call in a scratch tree — this proof MUST fail while
readout-text assertions still pass. Record that check in the evidence README
(one line, commit the passing state only).

### T012 — Forced render throw and rejection → typed nonzero exit

FR-006's end-to-end proof, exercising WP01's emitter and WP02's fatal path:

1. Use the existing deterministic page-override seam (the
   `CREST_WEBVIEW_PAGE` index override honored by `page_asset` — dev builds
   only) to serve an index/page variant whose `render` throws on the first
   projection (e.g. a `data-crest-force-render-throw` marker the override
   page reads; keep the variant minimal and inside the test's fixtures).
2. Drive one projection push; assert:
   - exactly one `crest://render-error` event arrives with the typed payload
     (name, message, document identity fields present),
   - NO painted ack is credited for that document,
   - the shell run terminates through `WebviewShellError::PageRenderFailed`
     with a NONZERO exit. Depending on the harness's process model, assert at
     the strongest available boundary: spawn the shell as a subprocess and
     assert exit status, or assert the typed error surfaces from the window
     run-loop API the test drives. Prefer the subprocess route if the
     existing typed-startup-failure section already has one — mirror it.
3. Second variant: reject a promise (unhandledrejection) instead of throwing
   in render; same assertions.
4. Negative control: the healthy page produces zero render-error events over
   the full suite run.

**Validation**: suite green; removing WP01's try/catch (scratch tree) turns
case 2 into the old silence and this section MUST fail.

### T013 — Re-run the affected proofs under the production policy

The determinism, latency, and screenshot sections now run under the T010
serving path — re-run and re-capture:

1. Page-render determinism (the cutover's "T024"): double-render identical at
   both viewports for the MIXER document and the PATCH documents — now with
   CSSOM-applied geometry included in the compared observation.
2. Latency: reducer state change visible within 50 ms p95 under the paced
   workload (NFR-002 / `requirement.serialized_projection_transport`) — the
   CSSOM pass is new per-render work; if p95 regresses past 50 ms that is a
   FINDING to report against WP01, not a threshold to adjust.
3. Screenshots at both viewports with nonzero levels — fills visibly filled,
   matching readouts (SC-001's committed visual record).

**Validation**: all three sections pass with their markers; screenshot files
land in evidence (T014).

### T014 — Commit the refreshed evidence

Create `kitty-specs/webview-render-fidelity-hardening-01KZCEF8/evidence/`:

1. `README.md` — index table mapping each artifact to its proof section and
   requirement (mirror the cutover `evidence/README.md` format); include the
   two falsifiability spot-checks (T011, T012) as one-line entries with the
   commit they were exercised in.
2. Named artifacts: determinism run log, latency measurements (p50/p95/max +
   workload description), screenshots (viewport + level in the filename),
   forced-failure run transcript showing the typed error and exit status.
3. State explicitly in the README that all artifacts were collected with the
   page served under the production `PAGE_CSP` via the exported seam, and
   name the policy string version (quote it or cite `window.rs`).

**Validation**: `git status` clean after commit; every artifact referenced
from the README exists.

## Definition of Done

- [ ] Harness serves via the production seam; CSP parity asserted against the
      exported constant; no section bypasses it.
- [ ] Painted-geometry proof passes and demonstrably fails on a reverted
      geometry fix (spot-check recorded).
- [ ] Forced throw AND forced rejection produce the typed payload, no ack,
      nonzero typed termination; healthy run emits zero render-errors.
- [ ] Determinism, latency (≤50 ms p95), screenshots re-collected under the
      production policy.
- [ ] `evidence/` committed with README index; suite prints
      `CREST_ACCEPTANCE webview_projection_shell passed`.
- [ ] No production-source edits; no baseline loosening; only
      `tests/webview_projection_shell.rs` + this mission's `evidence/`
      touched.

## Risks / Reviewer Guidance

- The parity assertion must reference the exported constant — a copied string
  recreates DRIFT-1 one abstraction up. Grep the test for the CSP text; it
  should appear zero times as a literal.
- T011's inverse guard (attribute present ⇒ CSSOM property set) is the load-
  bearing regression tripwire — review it hardest.
- Latency regression from the CSSOM pass is a report-don't-adjust boundary.
- The forced-throw page variant must be unreachable in release builds (the
  override seam is compiled out — confirm, don't assume).
