# Adversarial Review: mission webview-render-fidelity-hardening-01KZCEF8 diff

**Date:** 2026-08-06 · **Mode:** commit range `2691475c..HEAD` (mission squash-merge)
**Files:** 6 (`src/shell/webview/{mod,window}.rs`, `tests/{component_composition,component_vocabulary,webview_projection_shell}.rs`, `webview-page/page.js`)
**Raised:** 20 hunter findings → 13 after merge/dedup · **Survived skeptic at Critical/Major:** 0

Pipeline: six blind parallel hunters (bloater, OO-abuser, change-preventer, dispensable, coupler, clean-code) → orchestrator merge → skeptic with kill mandate. Strongest cross-hunter agreement: five of six hunters independently converged on the guard-scan allowance dispatch cluster in `tests/component_vocabulary.rs` (F1).

## Critical

None.

## Major

None. Nothing met the bar: no broken contract, no LSP violation, no duplicated copy that has actually diverged.

## Minor (4 confirmed; reported because each touches an FR-owning file)

### F1 — allowance dispatch by file-name strings; px-detector copied — `tests/component_vocabulary.rs:1122-1204`, `tests/component_composition.rs:1785-1819`
Facts confirmed 8/10, severity Minor. Allowances (`rgba(0,0,0,0)` sentinel erasure, `fonts.check(` px allowance) live as `name == "gallery.js"` comparisons inside loop bodies rather than beside the source table; the digit+`px` detector is verbatim-copied between the page.css and gallery loops. Skeptic kills on severity: the sentinel match is exact (near-misses still fail), the `//`-comment gap fails closed, and the verbatim copy was mission-mandated (research D6: "detection logic copied verbatim"). One real residue for the hygiene mission: gallery.js joined the key-handler scan but not the purity-needle loop (`Date.now`/`Math.random`/…, `component_composition.rs:1790-1805`), while gallery.js's own header claims those properties. Fix (recorded, not applied): per-source descriptor table carrying allowances as data.

### F5 — triple double-measure guard — `tests/webview_projection_shell.rs:2797-2861`
CONFIRMED 8/10, Minor. The `if first != second { return Err(…) }` identity check appears three times, verbatim modulo message, inside `prove_painted_geometry`. Fix: fold into `measure_geometry` parameterized by fixture label.

### F7 — ceremonial unreachable match arm — `src/shell/webview/window.rs:811-825`
CONFIRMED 7/10 (2 hunters agreed), Minor. The latch unit test wraps its two payloads in `PageSignal` and matches with an unreachable `PaintedAck` arm whose `unreachable!("a render error is never a painted ack")` message claims a discrimination the test never exercises (the real dispatch is the event-loop drain, covered by T012). Fix: call `record_render_failure` directly on the two payload strings.

### F11 — dead fallback branch — `tests/component_vocabulary.rs:1192` (new copy of pre-existing 1171)
CONFIRMED 8/10, Minor nit. `line.split("/*").next().unwrap_or(line)` — `str::split` always yields at least one item, so the fallback is unreachable. Propagated under D6's verbatim-copy mandate.

## Downgraded to Minor/nit (facts real, harm latent or design-mandated — not individually actionable)

- **F2** cross-language replication of page `fraction`/`innerValue` in the harness (`tests/webview_projection_shell.rs:572-604`): edge divergence (NaN vs clamped) is real but unreachable — `SemanticNumericRange` serializes non-optional `f64` bounds, and every fixture cross-checks against the page-computed attribute.
- **F3** `data-level`/`data-position` pair-wise duplication across page.js and harness; `filter_map` fails open only if the collect script 30 lines above changes without either copy.
- **F4** `(window, receiver)` clump through 6 harness signatures; 5-param `force_page_failures` (window derivable from handle).
- **F6** `force_page_failures` long method — 223 lines (hunter claimed 237), three sequentially-coupled narrated proofs; shared bindings are shadowed immutable `let`s, not mutable state.
- **F8** all uncaught page faults exit as `PageRenderFailed` with last-rendered identity — routing is FR-005-mandated and T012-pinned; only the name's breadth survives.
- **F9** two render-error payload checkers of different strictness — structurally forced (the subprocess section parses stderr and cannot know exact identity values).
- **F12** the "two callers by design" narrative restated in three doc comments — defensive documentation; doc-drift is the only harm.

## Refuted (appendix)

- **F10** Divergent Change in `window.rs` — killed: this very mission changed both claimed axes together for one reason; the policy seam is documented as deliberately co-located; no falsifiable harm scenario.
- **F13** Message chains in document navigation — killed: raw wire-shape navigation is the declared subject of a schema-fidelity harness; the pattern pre-existed 6× and the new sites follow the file idiom.

## Summary

Twenty raw findings reduced to zero Critical/Major and four confirmed Minors, all in test scaffolding or a unit-test ceremony, none demonstrating a behavior defect. The recurring kill patterns: harms unreachable with the production serializers and fixtures, duplication explicitly mandated verbatim by the mission's research decisions, and behavior pinned by the very proofs this mission added. The diff is structurally sound for its scope; the actionable residue (per-source scan-descriptor table, gallery purity-needle coverage, the three small extracts) is hygiene-mission material, not rework.
