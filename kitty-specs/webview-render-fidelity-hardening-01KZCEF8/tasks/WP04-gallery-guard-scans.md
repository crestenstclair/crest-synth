---
work_package_id: WP04
title: Gallery guard-scan coverage
dependencies: []
requirement_refs:
- FR-007
planning_base_branch: feat/webview-shell-cutover
merge_target_branch: feat/webview-shell-cutover
branch_strategy: Planning artifacts for this mission were generated on feat/webview-shell-cutover. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/webview-shell-cutover unless the human explicitly redirects the landing branch.
created_at: '2026-08-06T22:44:12+00:00'
subtasks:
- T015
- T016
- T017
history:
- '2026-08-06: authored from plan IC-05, crest-spec assets ComponentVocabularyAcceptanceTests + ComponentCompositionAcceptanceTests, mission-review DRIFT-2'
agent_profile: implementer-ivan
authoritative_surface: tests/component_
create_intent: []
execution_mode: code_change
owned_files:
- tests/component_composition.rs
- tests/component_vocabulary.rs
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

Close DRIFT-2: the two executable guard scans that keep page sources honest
cover `page.js`/`page.css`/`index.html` only. `webview-page/gallery.js` and
`webview-page/gallery.css` are clean today but unguarded — extend the scans
so gallery drift fails the suite instead of shipping silently.

Authorities: crest-spec assets `ComponentVocabularyAcceptanceTests` and
`ComponentCompositionAcceptanceTests` (both already declare the "everywhere"
rule — this WP is the enumeration catching up, `crest_spec_impact` notes it
as predeclared), mission `spec.md` FR-007 / User Story 4, `plan.md` IC-05,
`research.md` D6.

**Hard boundaries**: scan-scope extension only — do NOT touch gallery sources
unless a scan legitimately catches a violation (T017), and do NOT weaken or
restructure the existing scan logic. `webview-page/page.js` belongs to WP01;
`tests/webview_projection_shell.rs` to WP03.

## Context

- **No-input-handler scan** — `tests/component_composition.rs:1801-1808`:

  ```rust
  for source in [&page_js, &index_html] {
      for needle in ["keydown", "keyup", "keypress"] {
          assert!(!source.contains(needle), "the page registers a key handler; ...");
      }
  }
  ```

  Gallery digit-key bindings live Rust-side (`src/testing`, crest-spec
  `TestingContextModules`: "binds one digit key per page locally" — that is
  the scene, not the page script), so `gallery.js` should pass unchanged.

- **Style-literal scan** — `tests/component_vocabulary.rs:1100-1117`
  (`check_page_sources_spell_no_visual_value`): iterates a
  `[(name, source)]` list of `page.css` / `page.js` / `index.html`, rejecting
  hex colors, `rgb(`/`rgba(`/`hsl(` constructors, and raw pixel extents
  outside the one declared fader-geometry exemption block. Both files load
  via the existing `page_source(...)` helper.

## Subtasks

### T015 — gallery.js joins the no-input-handler scan

At `tests/component_composition.rs:1801`, load `gallery.js` the same way
`page_js` is loaded above it and extend the loop:

```rust
for source in [&page_js, &index_html, &gallery_js] {
```

Keep the assertion message accurate for the wider set (it says "the page" —
generalize to name the offending source, matching the style-literal scan's
`{name}` pattern, e.g. iterate `(name, source)` pairs).

**Validation**: suite passes; adding `// keydown` to `gallery.js` in a
scratch tree fails with a message naming `gallery.js`.

### T016 — gallery.js + gallery.css join the style-literal scan

At `tests/component_vocabulary.rs:1107`, extend the list:

```rust
for (name, source) in [
    ("page.css", &page_css),
    ("page.js", &page_js),
    ("index.html", &index_html),
    ("gallery.css", &gallery_css),
    ("gallery.js", &gallery_js),
] {
```

loading both via the existing `page_source` helper. Check whether the
function's scan continues past this loop (pixel-extent section, fader-
geometry exemption) — every sub-check in
`check_page_sources_spell_no_visual_value` must cover the gallery pair, not
just the first loop. If the fader-geometry exemption block is keyed to
`page.css` specifically, gallery sources get NO exemption (they declare no
fader geometry).

**Validation**: suite passes; adding `#ff0000` to `gallery.css` in a scratch
tree fails naming `gallery.css`.

### T017 — Run both suites; fix any hit at the source

```bash
cargo test --test component_composition -- --nocapture
cargo test --test component_vocabulary -- --nocapture
```

Both must print their `CREST_ACCEPTANCE <target> passed` markers. The review
recorded both gallery files clean, so expect green. If a scan DOES hit:

- fix it in the gallery source only if it is a genuine violation of the
  declared rule, keeping the change minimal — but note gallery sources are
  outside this WP's `owned_files`, so record the finding and hand it back to
  the operator rather than editing across the boundary;
- a declared exemption (mirroring the fader-geometry block's explicit,
  commented shape) is acceptable ONLY for a construct the crest-spec
  vocabulary rules genuinely permit; never a silent carve-out or a scan
  weakening.

**Validation**: both markers printed; scratch-tree falsifiability checks from
T015/T016 reverted.

## Definition of Done

- [ ] `gallery.js` scanned by all three input-handler needles with a message
      naming the file.
- [ ] `gallery.js` + `gallery.css` covered by EVERY sub-check of the style-
      literal scan; no exemption leakage.
- [ ] Both falsifiability spot-checks demonstrated and reverted.
- [ ] Both suites green with their acceptance markers; no gallery-source or
      scan-logic changes.

## Risks / Reviewer Guidance

- The style-literal function has multiple sequential checks — the common
  miss is extending the first loop but not the pixel-extent section below
  it. Review the whole function body, not the diff hunk.
- Message text should name the offending source; "the page" wording hides
  which file tripped.
- If gallery digit-key handling ever moved page-side, T015 would catch it —
  that failure would be a real input-boundary violation (C-002), not a scan
  bug.
