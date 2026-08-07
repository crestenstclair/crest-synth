---
wp_id: WP04
reviewer_agent: reviewer-renata
cycle_number: 1
verdict: approved
mission_slug: webview-render-fidelity-hardening-01KZCEF8
reviewed_at: "2026-08-06T23:59:00Z"
affected_files:
  - path: tests/component_composition.rs
  - path: tests/component_vocabulary.rs
---

# WP04 review — cycle 1: approved

Reviewed commit `446c39f` on `kitty/mission-webview-render-fidelity-hardening-01KZCEF8-lane-d`
(1 commit over the mission branch). Diff touches exactly the two `owned_files`:
`tests/component_composition.rs` (+15/-2) and `tests/component_vocabulary.rs` (+56/-5).
No gallery source, no `webview-page/page.js` (WP01), no
`tests/webview_projection_shell.rs` (WP03) modified. Authorities checked:
spec.md FR-007 / User Story 4, research.md D6, plan IC-05 via the WP prompt.

## Verified items

1. **T015 — no-input-handler scan covers gallery.js.**
   `gallery_js` is loaded identically to `page_js` (line-wise `strip(line, true)`:
   `//` comments removed, string contents kept), and the loop now iterates
   `(name, source)` over `page.js` / `index.html` / `gallery.js` with the message
   generalized to name the offending source and needle
   (`"{name} registers a key handler (`{needle}`) ..."`). The extension carries
   its own narration comment (digit keys are bound Rust-side by the testing
   scene). Coverage of the pre-existing sources is behavior-identical: same
   needle set, same stripped/raw source treatment, only the tuple form and
   message text changed.

2. **T016 — style-literal scan covers gallery.css + gallery.js in EVERY sub-check.**
   - Color-constructor + hex + authored/retired-palette sub-checks: the
     `(name, source)` list now includes both gallery files, loaded via the
     existing `page_source` helper. For `page.css`/`page.js`/`index.html` the
     scanned text is `line.to_owned()` — byte-equivalent to the previous
     `line`-based scan; the pre-existing `page.css` pixel loop and the trailing
     `index.html` inline-style asserts are untouched. No weakening of existing
     coverage.
   - Pixel-extent sub-check: the fader-geometry exemption remains keyed to
     `page.css` (`starts_with("--fader-")` over `page_css` only), so the
     gallery pair correctly gets no fader exemption. A dedicated loop applies
     the *verbatim same* detection logic (`split("/*")` comment guard,
     digit-followed-by-`px`) to `gallery.css` and `gallery.js` — copied, not
     new, satisfying D6's "no new scan logic".

3. **T017 — both suites green with markers** on the clean worktree at `446c39f`:
   `CREST_ACCEPTANCE component_composition passed` (15 passed) and
   `CREST_ACCEPTANCE component_vocabulary passed` (11 passed).

## Allowance scrutiny (the review's crux)

**(a) Transparent read-back sentinel — `"rgba(0, 0, 0, 0)"` erased from
gallery.js lines before the color scan.**
- Cited construct exists as claimed: `webview-page/gallery.js:512` —
  `if (!color || color === "rgba(0, 0, 0, 0)" || color === "transparent")`
  inside `paintedColors`' `record()`, a computed-style read-back that SKIPS
  unpainted values. It compares; it never assigns. It is the sole occurrence
  of the sentinel and the sole `rgb(`/`rgba(`/`hsl(` text in the file.
- Keying: `name == "gallery.js"` only — no other source shares it (verified
  by inspection of the branch).
- Narrowness: exact computed-style serialization including the closing paren.
  Probed near-miss `rgba(0, 0, 0, 0.25)` still fails (erasure cannot match
  it), and erasure removes a substring, so it cannot mask any other
  constructor or hex run on the line. Granularity matches the fader-geometry
  precedent.
- Narration: dedicated comment block at the const declaration mirroring the
  fader exemption's declared-and-exact style. Present.

**(b) `fonts.check(` allowance for raw px in gallery.js.**
- Cited construct exists as claimed: `webview-page/gallery.js:567-570` — four
  `fonts.check('400 15px "Azeret Mono"')`-style CSS Font Loading API probes
  inside `typefaceResolved()`; the font shorthand makes the size token
  syntactically mandatory, and the probe paints nothing (it gathers the
  typeface-resolution evidence the vocabulary proof demands). These four
  lines are the only px occurrences in gallery.js.
- Keying: the assert requires `name == "gallery.js" && code.contains("fonts.check(")`,
  so gallery.css gets NO allowance of any kind (probed below) and page
  sources are not touched by this loop at all.
- Narrowness: line-granularity containment — the same granularity class as
  the fader precedent (`starts_with("--fader-")` is equally line-level).
  Given the WP's prohibition on restructuring scan logic, matching the
  established granularity is correct.
- Narration: dedicated comment block above the loop, explicitly denying the
  gallery pair any fader-style exemption and declaring the font-probe
  allowance and its rationale. Present.

## Probe results (each injected, run, and reverted; worktree left clean at 446c39f)

| Probe | Expectation | Result |
|---|---|---|
| `rgba(1,2,3,0.5)` appended to gallery.js | vocabulary suite fails naming gallery.js | FAILED as required: "gallery.js builds a color of its own" |
| `rgba(0, 0, 0, 0.25)` (near-sentinel) in gallery.js | still fails — erasure is exact | FAILED as required: "gallery.js builds a color of its own" |
| `#ff0000` appended to gallery.css | fails naming gallery.css | FAILED as required: "gallery.css spells a hex color" |
| `window.addEventListener("keydown", function () {});` in gallery.js | composition suite fails naming gallery.js | FAILED as required: "gallery.js registers a key handler (`keydown`)" |
| `var probeWidth = "500px";` in gallery.js (outside fonts.check) | fails — px allowance is fonts.check-only | FAILED as required: "gallery.js sets a raw pixel extent outside the declared font-probe allowance" |
| `.probe { width: 500px; }` in gallery.css | fails — gallery.css has no allowance | FAILED as required: "gallery.css sets a raw pixel extent ..." |

All probes reverted (`git status` clean of tracked changes; HEAD at `446c39f`).
Clean re-run after reverts: both suites pass with acceptance markers.

## Anti-pattern checklist

1. Dead code — PASS (no new public items; the new const is used in-function).
2. Synthetic-fixture test — PASS (scans read the committed production sources
   via `page_source`; deleting a gallery source fails the read).
3. Silent empty return — PASS (none introduced).
4. FR coverage — PASS (FR-007: both scans now enumerate gallery.js +
   gallery.css with behavior-naming assertions; falsifiability demonstrated).
5. Frozen surface — PASS (only the two owned test files changed).
6. Locked decision — PASS (D6 honored: enumeration extension, detection
   logic copied verbatim; the two allowances are declared, narrated, and
   permitted by T017's explicit exemption clause).
7. Shared-file ownership — PASS (lane-a and lane-b diffs touch neither test
   file; WP04 owns both exclusively).
8. Production fragility — N/A (test-only change).

## Non-blocking observation

T015's validation text ("adding `// keydown` ... fails") is internally
inconsistent with the same subtask's mandate to load gallery.js "the same way
`page_js` is loaded" — that loading applies `strip(line, true)`, which drops
`//` comments by pre-existing, narrated design ("comments narrate the rule
and are not ownership"). The implementer correctly chose loading parity; a
real handler registration — bare, single- or double-quoted — is caught
(probed above; `strip` keeps string contents). No action required.

## Process note

This record was first committed on the lane branch; the move-task guard
correctly rejected kitty-specs/ content there, so the lane was reset to
`446c39f` and the record recommitted on `feat/webview-shell-cutover`, the
planning branch that owns mission artifacts.
