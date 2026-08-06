# Adversarial Review: webview-shell-cutover-01KZAC7Q mission diff (d41e7bd..45039aa)

**Date:** 2026-08-06 · **Mode:** commit range (mission squash merge)
**Files:** 40 surviving changed files (src/, tests/, webview-page/) · **Raised:** 35 findings · **Merged to:** 15 · **Survived skeptic:** 13 (12 CONFIRMED + 1 core-confirmed) · **Refuted:** 1 · **Downgraded to Minor:** 1 · **Minor pool (not itemized):** 20

Six hunters ran blind and in parallel (bloater, oo-abuser, change-preventer, dispensable, coupler, clean-code); the skeptic re-opened every cited line with a kill mandate. Verdicts below are the skeptic's.

## Critical

*(none — no finding met the block-merge bar on structural grounds alone)*

## Major

### Duplicate weight table — src/testing/component_gallery_scene.rs:844-851
Evidence: `const fn weight_number` restates `FontWeight::numeric()` (src/shell/tokens.rs:200-207) arm-for-arm; `numeric()` is pub const and already called by token_export.rs and typeface.rs. Harm: a weight change updates the generated tokens.css but not the gallery's captions — the proof surface reports metrics the token table no longer serves. Fix: delete `weight_number`, call `.numeric()` (2 call sites). Skeptic: CONFIRMED (9/10), agreement: 2 hunters.

### Verbatim test-harness duplication ×4 — tests/shell_event_dispatch.rs:131-206, graphical_application_shell.rs:130-207, semantic_graphical_view_model.rs:137-217, component_composition.rs:1347-1386
Evidence: `page_band_labels` + `page_painted_ack` byte-identical in three files (proven by diff), near-variant in a fourth; the helper encodes the paint-ack contract. Harm: an ack-shape change requires four lockstep edits; a missed copy leaves a suite silently exercising a stale contract. Fix: Extract Method into shared test-support. Skeptic: CONFIRMED (9/10).

### Duplicated escapeHtml + HINT_SEPARATOR — webview-page/page.js:71-81 / gallery.js:33-43
Evidence: byte-identical escaping function in both scripts; both feed innerHTML. `HINT_SEPARATOR` declared 3× (page.js:43, gallery.js:31, component_vocabulary.rs:601). Harm: an escaping hardening applied to one page silently misses the other — security-relevant divergence channel. Fix: shared script asset served by the crest:// handler. Skeptic: CONFIRMED (8/10).

### DIP violation: application layer constructs its concrete window — src/shell/standalone_application.rs:41, 88-90, 988, 1015
Evidence: imports `TauriWebviewWindow`, `live_demo_window()` constructs it, and `host_live_demo_scene` discards the injected window (`window: _`). Crest-spec shell.yaml:626-627: "StandaloneApplication imports no concrete infrastructure adapter and constructs none of those boundaries." The spec's runLiveDemo clause constrains which window runs, not where it is built — root injection would satisfy both. Harm: the next shell swap edits the application layer instead of only the composition root; tests already grew a private seam to route around it. Fix: construct in src/bin/crest_synth.rs and inject. Skeptic: CONFIRMED (8/10), agreement: 2 hunters. Deliberate (documented in-code) but violates the rule's letter.

### Gallery scene re-implements webview hosting, minus the CSP — src/testing/component_gallery_scene.rs:3034-3255
Evidence: waker thread, run_return arms, close-once, and protocol handler mirror window.rs; the copied handler serves the gallery document with no CSP header (product attaches PAGE_CSP at window.rs:146-150); no comment declares the omission. Harm: every hosting change must be made twice — this diff already missed one (the hardening). NFR-003's mapped scope is the product page, so the CSP gap is hardening parity, not a spec violation (skeptic downgrade of that sub-claim). Fix: Extract shared hosting helper in shell::webview. Skeptic: CONFIRMED (7/10), agreement: 3 hunters.

### State-precedence rule transcribed 3× with existing divergence — page.js:329-361, component_composition.rs:316-341, webview_projection_shell.rs:1238-1263
Evidence: the ComponentState precedence walk is hand-transcribed in the page and two independent Rust test oracles; the mirrors already differ (`"unknown"` vs `"unknown:{unknown}"`). The accent-pairing sub-claim was REFUTED — accents are guarded by component_vocabulary.rs:1349-1402. Harm: a precedence change costs three coordinated hand edits; mirrors drift wherever fixtures don't reach. Fix: hoist one shared oracle into `crest_synth::testing`. Skeptic: CONFIRMED (7/10) on the core, agreement: 2 hunters.

### 244-line event-loop method with a thrice-spelled fatal path — src/shell/webview/window.rs:289-532
Evidence: run() at 244 lines; record-error + close-once + return spelled at 448-455, 459-468, 488-494. Harm: fatal-path ordering changes must find every copy inside one closure; signal-drain logic untestable without a live tauri runtime. Fix: Extract `drain_page_signals` + `record_fatal_and_close`. Skeptic: CONFIRMED (7/10), agreement: 2 hunters.

### Orphaned intent vocabulary — src/shell/component_vocabulary.rs:313-367, 516-589
Evidence: `ControlIntent`/`ControlRequest`/`CompositionIntent` (~140 lines) have zero external callers; nothing constructs a non-None intent; `record`/`absorb` uncalled. Crest-spec declares the concept, so this is spec-declared vocabulary orphaned by the cutover (controls now render page-side), not invention. Harm: readers pay for a boundary that carries nothing; the spec and code disagree about where intent lives. Fix: delete, or reconcile the crest-spec's control-intent declaration with the page-side reality. Skeptic: CONFIRMED (7/10).

### Dead blocking API — src/shell/webview/frame_stream.rs:122-146, 211-250
Evidence: `await_qualifying` + `FrameAwaitError` have zero non-test callers; the documented consumer (WP03) used the observation callback and `poll` instead. Harm: a condvar protocol with a real deadlock rule ("never on the event thread") that no composition exercises. Fix: delete the blocking face; keep record/poll. Skeptic: CONFIRMED (7/10).

### MidiHexadecimal formula ×2 with format-only validators — page.js:128-132, component_gallery_scene.rs:866-869
Evidence: formula implemented in JS and Rust; validators check shape only; the gallery test compares the builder to itself (line 3709). Currently agree — ungated drift risk, not present divergence. Fix: cross-check page.js output against the Rust formula on boundary inputs in the existing scrape test. Skeptic: CONFIRMED (7/10), agreement: 2 hunters.

## Minor (itemized only where FR-adjacent)

- Misleading provenance comments: 9 of 10 cited comment anchors in page.js/gallery.js name Rust modules deleted by this mission (`parameter_row`, `patch_strip_row`, `utility_inspector_panel`, `primitives::hint`, …); the page is now sole holder of several transcribed values while claiming otherwise. Skeptic: CONFIRMED (8/10). Fix: repoint at component_vocabulary/component_state or state page ownership honestly.
- Dead constant with false authority: `CURSOR_GLYPH` (component_vocabulary.rs:812) — zero consumers; gallery.js hardcodes its own `&gt;`. Skeptic: CONFIRMED (8/10).
- `record_ack` inconsistent defect recording: five sibling resolutions record "undeclared X" defects; the per-control state loop (component_gallery_scene.rs:2072-2079) silently skips. Skeptic: CONFIRMED (8/10), severity minor.
- Mixer-column markup skeleton shared page.js:498-595 / gallery.js:272-305 / gallery.js:189-195 — condensed variant, not verbatim (skeptic DOWNGRADED); class-string drift unguarded.
- Non-itemized minor pool (20): long gallery methods (`gallery_coverage_failures`, gallery `run`, `record_ack` shape), `GalleryAckLedger`/`LiveDemoRunner` field counts, `drive_live_window` 570-line test driver, `mixerInspectorHtml`/`columnHtml` shape, `state_specimen` parameter list, `control.path.controlId.id` message chains ×8, gallery CSS-var naming re-derivation (one real lowercase drift), type-style CSS redeclarations contradicting gallery.css's own header, typeface module production-orphaned, four unconsumed density accessors, GallerySpecimen↔gallery.js parallel dispatch unguarded, gallery module divergent-change axis.

## Refuted (appendix)

- gallery.js `statePresentationHtml` `return ""` as silent-blank violation — killed: the rule at gallery.js:355-356 governs `specimenHtml`'s closed kind dispatch (which honors it with `?kind`); the empty presentation body is a load-bearing branch for level-less row controls; label/value/evidence still paint.

## Summary

The transport and proof machinery is structurally disciplined; the debt concentrates where the mission moved a Rust-rendered vocabulary across a language boundary under deadline: hand-transcribed mappings (weights, state precedence, midi-hex, provenance comments) that the token-generation pattern already solves elsewhere in the same diff, duplicated hosting/test scaffolding, and vocabulary orphaned by the egui deletion (intent family, blocking frame API, cursor glyph). None of it blocks merge on structural grounds; the highest-value follow-up is extending the generated-token pattern to the remaining hand-copied mappings and consolidating the two webview hosts.
