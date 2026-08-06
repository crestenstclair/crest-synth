# Mission Review Report: webview-shell-cutover-01KZAC7Q

**Reviewer**: Claude (Fable 5) — mission-review orchestrator, with 2 analysis subagents, 6 blind hunters, 1 skeptic
**Date**: 2026-08-06
**Mission**: `webview-shell-cutover-01KZAC7Q` — Webview Shell Cutover
**Baseline commit**: `d41e7bd` (pre-implementation feat tip; meta.json `baseline_merge_commit` = `7e61dc7`, the pre-squash straggler commit)
**HEAD at review**: `45039aa`
**WPs reviewed**: WP01..WP07 (all done; 0 rejection cycles; 0 self-approvals; one WP01 claim release for a reviewer-agent switch, re-reviewed cleanly)

---

## Gate Results

### Gate 1 — Contract tests
- N/A for this repository: no `tests/contract/` tree exists (that gate targets the spec-kitty repo). Project-equivalent hard gate: crest-spec declared deterministic validations, run by `spec-kitty accept`.
- Equivalent result: **PASS** — `deterministic-acceptance.json` `passed: true`, acceptance commit `a38abf7`.

### Gate 2 — Architectural tests
- N/A as a directory; architecture is enforced by in-suite guards (transport-purity scans, no-input-handler scans, RT-boundary zero-diff). All green in `cargo test --all-targets` (exit 0, 29 targets) on the merged tree.
- Equivalent result: **PASS**.

### Gate 3 — Cross-repo E2E
- N/A: single-repo mission; no cross-repo scenarios exist or were claimed.

### Gate 4 — Issue Matrix
- `spec.md` references zero GitHub issues; no `issue-matrix.md` required. Result: **N/A — PASS**.

---

## FR Coverage Matrix

Full trace in the FR-analysis record (all citations verified at HEAD). Summary: **19/19 requirement rows ADEQUATE** — every FR/NFR/C row has live tests through production reducer/projector/channel/window paths or committed hardware evidence, with falsifiability negatives (planted literals, identity-mismatch, TokenDrift mutations). **Zero FALSE_POSITIVE grades, zero punted FRs.**

| ID | Grade | Key evidence |
|----|-------|--------------|
| FR-001 PATCH via canonical model | ADEQUATE | webview_projection_shell T022/T024 both viewports; shell_event_dispatch production AppLoop |
| FR-002 webview sole shell | ADEQUATE | crest_synth.rs:126 sole composition; `--shell` rejected by test; typed startup-failure subprocess proof |
| FR-003 live scenes + evidence pre-deletion | ADEQUATE | 4 committed hardware logs, exit 0, 0 dropped; run independently 4× (implementers + reviewers) |
| FR-004 ack→observation forwarding | ADEQUATE | forward_ack typed negatives; identity verbatim; 1868–2198 qualifying frames/run — **but see RISK-2/RISK-4 for the page-side error half** |
| FR-005 gallery both densities | ADEQUATE | 15-page closed enum; frozen digit baseline; coverage-before-open |
| FR-006 egui deleted | ADEQUATE | independent sweep: zero egui/eframe references; 244 transitive crates dropped |
| FR-007 crest-spec first + DESIGN pivot | ADEQUATE | ancestry 307873e < d266760; DESIGN/ROADMAP records |
| FR-008 key-injection witness | ADEQUATE | production NSEvent monitor, byte-exact translator parity; found+fixed a real WebKit double-dispatch |
| NFR-001 RT neutrality | ADEQUATE | A/B 0/0 callbackAllocations; p95 8.8 ms vs 50 ms, hard-asserted at wps:1774 |
| NFR-002 300 s soak | ADEQUATE | 29.43 Hz sustained, 0 lost, RSS declining |
| NFR-003 CSP + release gating | ADEQUATE as specified — **but the CSP itself breaks fader rendering: RISK-1** |
| NFR-004 token single source | ADEQUATE | 0 literals; byte-stable regeneration; TokenDrift falsifiability |
| C-001..C-007 | ADEQUATE | all locked decisions verified positive and negative side |

---

## Drift Findings

### DRIFT-1: Evidence-method drift — the acceptance harness serves the page without the production CSP
**Type**: NFR-MISS (method) · **Severity**: HIGH (as the enabler of RISK-1)
**Spec reference**: NFR-003, T024/T026 evidence
**Evidence**: `tests/webview_projection_shell.rs:874-882` — harness protocol handler attaches only `Content-Type`; production `protocol_response` (window.rs:146-150) attaches `PAGE_CSP`. Every fidelity screenshot and determinism/latency proof ran under a laxer policy than the product ships.
**Analysis**: The harness is the mission's paint oracle; measuring under a different security policy than production is how RISK-1 shipped invisible. Fix direction: harness must serve through the production `protocol_response`.

### DRIFT-2: Guard-scope gap — gallery sources outside the executable C-001/NFR-004 scans
**Type**: proof-coverage gap · **Severity**: LOW
**Evidence**: component_composition.rs:1801-1808 and component_vocabulary.rs:1100-1110 scan page.js/page.css/index.html only; gallery.js/gallery.css (new this mission) are clean today but unguarded.

### DRIFT-3: Analysis-finding residues (recorded, non-blocking)
- A1: NFR-002 leak bound still unquantified in spec; discharged honestly via committed declining RSS series. LOW.
- I1: "migration" terminology persists in spec.md/plan.md only; shipped code/docs are clean. LOW.
- Bookkeeping: spec.md still "Draft"/"Open" after acceptance. INFO.

---

## Risk Findings

### RISK-1: Production CSP blocks the page's inline style attributes — fader fills render empty in the shipped app
**Type**: BOUNDARY-CONDITION · **Severity**: **HIGH — confirmed live**
**Location**: `src/shell/webview/window.rs:92` (`style-src 'self'`, no `unsafe-inline`) vs `webview-page/page.js:514` (`style="--level:"`), `:682` (`--position`), consumed at `page.css:270,286,439`
**Trigger condition**: every render of the real product window (`make run`).
**Analysis**: WKWebView enforces the CSP on the `crest://` response; every JS-generated `style` attribute is blocked, so `--level`/`--position` never set and each fader paints empty (cap at bottom) while the LevelReadout text shows the true value. Confirmed by launching `target/release/crest-synth`: all sixteen fills empty at hex 73 (~90%). Not caught because of DRIFT-1 (harness serves without CSP) and because acks carry geometry+text only. Fix direction (not applied per review mandate): set custom properties via CSSOM `el.style.setProperty` (CSP-exempt) or data-attributes + stylesheet — not `unsafe-inline`.

### RISK-2: `crest://render-error` has no emitter — page render exceptions are fully silent
**Type**: ERROR-PATH / DEAD-CODE · **Severity**: **HIGH**
**Location**: listener `window.rs:354-356`; declared `projection_channel.rs:94`; documented `mod.rs:100-107`; **zero emitters** in `webview-page/` (repo-wide grep); `page.js:1297-1311` calls `render(model)` with no try/catch, no `window.onerror`.
**Trigger condition**: any throw inside `render()` on the product page (unexpected document shape, DOM error).
**Analysis**: The WP02 T008 "typed render failure, never a frozen window" guarantee has no page half: an exception dies in the event dispatch, no ack fires, no typed error surfaces, and the interactive window keeps running on a stale/partial DOM. page.js's comment claiming the throw "surfaces to the adapter's typed render-exception path" is false. Harness no-progress guards would notice a stall; interactive mode notices nothing.

### RISK-3: Double close-failure freezes the recorded fatal error
**Type**: ERROR-PATH · **Severity**: LOW (improbable trigger)
**Location**: window.rs:418-420, 448-455, 522-524. If both `window.close()` attempts fail, the loop early-returns forever and the recorded error is never surfaced.

### RISK-4: Identity-verbatim enforcement narrower than documented
**Type**: BOUNDARY-CONDITION · **Severity**: LOW
**Location**: projection_channel.rs:388-405 — a `SupersededLate` ack (generation ≤ newest, no longer tracked) is consumed without identity validation. It can never construct an observation, so proofs hold; the documented "verbatim or typed-rejected" MUST is simply narrower in that window.

### RISK-5: Dead code (new public API, zero production callers)
**Type**: DEAD-CODE · **Severity**: LOW–MEDIUM
- `QualifyingFrameStream::await_qualifying` + `FrameAwaitError` — documented consumer (WP03) used the callback+poll instead. Corroborated by the structural pipeline (CONFIRMED 7/10).
- `LiveDemoRunner::step_index()` — zero callers.
- `ControlIntent`/`ControlRequest`/`CompositionIntent` family (~140 lines, component_vocabulary.rs) — crest-spec-declared vocabulary orphaned by the cutover; spec and code now disagree about where control intent lives. (Structural pipeline CONFIRMED 7/10.)
- `CURSOR_GLYPH` — false single-source authority; gallery hardcodes its own glyph. (CONFIRMED 8/10.)
Accepted seams re-verified still true: `in_flight_documents()`, `FrameExpectation` accessors.

### RISK-6: Cross-WP integration — clean
Event namespaces disjoint; no double listener registration; the `SupersededLate` panics in two test harnesses are unreachable by construction; Cargo.toml WP05/WP07 edits coherent; condvar/teardown/dedup-ring concurrency all verified correct (input-capture residuals are outside realistic operation).

---

## Silent Failure Candidates

| Location | Condition | Silent result | Spec impact |
|----------|-----------|---------------|-------------|
| page.js:1297-1305 | `render(model)` throws | no ack, no typed error, stale DOM, app keeps running | RISK-2: T008 guarantee has no page half |
| window.rs:92 + page.js:514,682 | CSP blocks style attrs | faders/positions empty at every level | RISK-1 |
| window.rs:513 + double close failure | close fails twice | recorded fatal never surfaces | RISK-3 |
| projection_channel.rs:398-404 | late ack ≤ newest | consumed unchecked as SupersededLate | RISK-4 |
| component_gallery_scene.rs:2072-2079 | unknown state in a control's ack list | dropped without a defect entry | gallery ledger inconsistency (structural F15) |
| window.rs:499, bounded ring/tracker evictions, poisoned gallery inbox, set_focus failure | various | documented lost-frame / minor | accepted by design |

---

## Security Notes

| Finding | Location | Risk class | Recommendation |
|---------|----------|------------|----------------|
| CSP lacks `base-uri`/`form-action` (don't inherit default-src) | window.rs:92-93 | hardening, low | add `base-uri 'none'; form-action 'none'` while fixing RISK-1 |
| Gallery page served with no CSP | component_gallery_scene.rs:3090-3100 | parity, info (test scene, embedded assets only) | serve via shared `protocol_response` |
| `rt_ab_measurement.sh` sources jq-derived `KEY='VALUE'` values unsanitized | scripts/rt_ab_measurement.sh | local shell injection, low (esp. `--reuse-logs` on edited logs) | sanitize all sourced values or parse instead of sourcing |
| Unescaped numeric attribute interpolation | page.js:585-586, :993 | hardening only (trusted projector, CSP confines) | route through escapeHtml for uniformity |
| crest:// protocol handler | window.rs:118-160 | clean | closed exact-match table; traversal probed by test |
| `CREST_WEBVIEW_PAGE` seam | window.rs:79-80,205-227 | debug-only residual | cfg gate confirmed structurally; acceptable |
| Network surface | page.js/gallery.js/new Rust | clean | no fetch/XHR/WebSocket; no outbound HTTP |

---

## Structural Quality (adversarial-review)

**Pipeline report**: `kitty-specs/webview-shell-cutover-01KZAC7Q/adversarial-review.md`
**Files reviewed**: 40 · **Raised**: 35 · **Merged**: 15 · **Survived skeptic**: 13 · **Minor (not itemized)**: 20 · **Refuted**: 1

Highest-value confirmed findings (severities mapped to release scale; none force FAIL alone):

- SMELL-1 (MEDIUM): duplicate `FontWeight` weight table in the gallery vs `numeric()` — CONFIRMED 9/10, 2 hunters.
- SMELL-2 (MEDIUM): verbatim `page_band_labels`/`page_painted_ack` test helpers ×4 files — CONFIRMED 9/10; the paint-ack contract has four copies.
- SMELL-3 (MEDIUM): `escapeHtml` duplicated across both pages (security-adjacent divergence channel) — CONFIRMED 8/10.
- SMELL-4 (MEDIUM): DIP violation — `StandaloneApplication` imports and constructs `TauriWebviewWindow`, discarding the injected one, against crest-spec shell.yaml:626-627 — CONFIRMED 8/10, 2 hunters; deliberate but letter-violating.
- SMELL-5 (MEDIUM): gallery scene re-implements the webview hosting loop minus the CSP — CONFIRMED 7/10, 3 hunters; corroborates the security parity note.
- SMELL-6 (MEDIUM): state-precedence rule transcribed 3× with the two Rust mirrors already divergent — CONFIRMED 7/10 (accent sub-claim refuted; accents are guarded).
- SMELL-7 (LOW): misleading provenance comments citing deleted `src/shell/visual/*` modules (9 of 10 anchors) — CONFIRMED 8/10.
- Remaining confirmed: window.rs long method with thrice-spelled fatal path (7/10), midi-hex ungated duplication (7/10), record_ack inconsistent defect recording (8/10, minor). Dead-code smells folded into RISK-5 rather than duplicated.

Refuted at skeptic: gallery `statePresentationHtml` silent-blank claim (legitimate closed-set branch).

---

## Final Verdict

## **FAIL**

### Verdict rationale

The spec→code fidelity story is exceptionally strong — 19/19 requirements adequately covered by production-path tests and committed hardware evidence, zero punted FRs, locked decisions verified on both sides, clean deletion, and a green deterministic acceptance record. But two HIGH findings block release, both in the exact seam the mission existed to build: **RISK-1** — the shipped window's own CSP blocks the page's inline style attributes, so every fader/position visual renders empty in the real app (confirmed by running the release binary), hidden because the acceptance harness serves the page without the production CSP (DRIFT-1); and **RISK-2** — the typed page-render-failure path the mission documents as a hard guarantee has no emitter, so page render exceptions are fully silent in interactive use. Neither is documented as an accepted known issue, so per the binary-verdict rule the mission review verdict is FAIL until both are fixed (small, well-understood fixes: CSSOM property-setting + harness serving through `protocol_response`; a page-side error boundary emitting `crest://render-error`) and the affected proofs re-run under the production policy.

### Open items (non-blocking)

1. DRIFT-2: extend the C-001/NFR-004 executable scans to gallery.js/gallery.css.
2. RISK-5 dead code: delete or consume `await_qualifying`, the intent family, `CURSOR_GLYPH`, `step_index()`; reconcile the crest-spec control-intent declaration.
3. Structural debt: shared test-support module for the four-fold harness duplication; shared page script for escapeHtml; DIP restoration for the live-demo window; consolidate the two webview hosts; one shared state-precedence oracle; midi-hex cross-check.
4. Hardening: `base-uri`/`form-action` in the CSP; sanitize sourced values in rt_ab_measurement.sh; RISK-3 close-failure surfacing.
5. Process: spec.md status flip; quantify the NFR-002 leak bound; reword residual "migration" terminology in spec/plan.

## Retrospective Reminder

The retrospective was auto-captured at merge: `kitty-specs/webview-shell-cutover-01KZAC7Q/retrospective.yaml` (note: this project stores it in the mission dir, not `.kittify/missions/<id>/`). Post-review sequence while context is fresh:

- `spec-kitty retrospect summary` — cross-mission aggregation (read-only)
- `spec-kitty agent retrospect synthesize --mission webview-shell-cutover-01KZAC7Q` — inspect proposals (dry-run)
- `--apply` to stage accepted proposals

The two HIGH findings above belong in a follow-up fix mission; this review's record should be its input.
