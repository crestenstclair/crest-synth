---
work_package_id: WP05
title: MIXER projection page
dependencies:
- WP03
- WP04
requirement_refs:
- FR-001
- FR-002
planning_base_branch: feat/webview-shell-foundation
merge_target_branch: feat/webview-shell-foundation
branch_strategy: lane worktree computed by finalize-tasks; merges into feat/webview-shell-foundation
subtasks:
- T017
- T018
- T019
- T020
- T021
history:
- '2026-08-05: authored from plan IC-05'
agent_profile: frontend-freddy
authoritative_surface: webview-page/
create_intent:
- webview-page/index.html
- webview-page/page.js
- webview-page/page.css
execution_mode: code_change
owned_files:
- webview-page/index.html
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

The real MIXER projection surface: a pure render of the serialized
`SemanticGraphicalViewModel` document into the authored composition — context
line, identity header, sixteen-column strip bank with the declared column
anatomy, persistent Inspector, hint rows — styled exclusively from the
generated `tokens.css` and the vendored Azeret Mono, animated by the meter
channel, correct at both authored viewports. Replaces WP02's placeholder in
the Tauri window.

## Context

- Plan IC-05; crest-spec `asset.WebviewProjectionPage` prompts (read them —
  they are the authoritative instructions this WP realizes) and
  `valueObject.MixerTrackColumnStructure` invariants (the column anatomy:
  TrackHeader, LevelFader, LevelReadout, PanReadout, StateLine — closed,
  ordered, header is the only name, pan compact, mute+solo one line, hex
  bound to LevelReadout only).
- **Seed**: `spike/webview-mixer/index.html` — the proven 244-line render of
  this exact document shape. Reuse its structure freely; replace its
  hand-copied token block with `tokens.css` imports, split JS into `page.js`,
  and upgrade per the deltas below.
- The document shape: `spike/webview-mixer/view-model.json` (recorded
  production fixture, MIXER context). Transport events from WP03:
  `crest://projection` (full document) and `crest://meters`
  (AudioObservationSnapshot) via the tauri event API.
- Authored design references: `figma-functional-interpretation/assets/mixer.png`,
  DESIGN.md MIXER sections; the two viewports are 1920×1080 and 1280×800.

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-foundation`.
Execution happens in the lane worktree `finalize-tasks` computes; do not
branch manually.

## Subtasks

### T017 — Author index.html

Structure only: the five shell bands (context line, identity header, main
workspace, side region, footer) as semantic containers; `<link>` tokens.css
and page.css; `<script>` page.js; @font-face for the vendored Azeret faces
(resolve how WP02 serves assets — tauri asset protocol — and reference
accordingly; coordinate the paths with the `// WP05` seam in
`src/shell/webview/window.rs`, which this WP owns the completion of).
No inline styles, no inline script beyond bootstrapping.

Swapping the window from WP02's placeholder to the real asset path is a
one-line edit in `src/shell/webview/window.rs` (WP02's file, merged before
this WP starts) — make it with a one-line out-of-map rationale in the WP
record, per the ownership rules.

### T018 — Author page.js — the pure render function

- `render(document) -> void` rebuilding the DOM from one deserialized view
  model; listener glue (`crest://projection` → parse → render) kept separate
  from the pure function so WP06 can drive `render` headlessly.
- Determinism contract: same document → identical DOM (no Date.now, no
  Math.random, no incidental iteration-order dependence).
- Meters: `crest://meters` handler updates only meter elements (presentation
  animation exempt from the purity rule, per crest-spec C-002's
  "presentation-only concerns" carve-out); a missing/stale snapshot renders
  the zero/stale meter state, mirroring the eframe rule.
- Expose for the harness: `window.crest = { render, renderObservation }`
  where `renderObservation(document)` returns the structural observation
  WP06 asserts (band presence, per-column structure list, values, focus
  target) — shape it with WP06's T024 wording in mind and document it in a
  comment block.

### T019 — Lay out the authored bank and column anatomy

page.css from tokens only:

- Bank: `grid-template-columns: repeat(16, minmax(var(--floor), var(--column-width)))`
  with the authored gutter — the declared geometry (82/86 desktop, floor
  from the density policy's declared minimum) expressed as custom properties
  derived in T014's table or declared page-side ONLY if the vocabulary does
  not name them (the spike's constants block shows which; anything the Rust
  vocabulary names must come from tokens.css).
- Column = the declared five-structure anatomy in order; LevelFader dominant;
  hairline separators; no per-structure track-name captions.
- Fader: track/fill/cap per the authored geometry (14 px track, 8 px fill,
  3 px shoulder, 34×6 cap) with fill color by state (rest/focus/mute/solo
  accent tokens) and hex LevelReadout (`(v-min)/(max-min)×127` as two
  uppercase hex digits — the spike's formula).

### T020 — Render the Inspector, hint rows, and focus/state emphasis

- Inspector from the `mixerInspector` surface controls (cursor identity, big
  level readout, mute/solo lines, sends in declared order); hint rows from
  `validActions` (filter null hints — spike had this bug, keep the fix).
- Focused column: keyline + halo from the token constants; focus and every
  applicable state visible with text or shape beyond color
  (MixerTrackColumnStructure invariant).
- A structure with no view data: omitted or explicitly unavailable — never a
  representative value (production page, not gallery).

### T021 — Hold both authored viewports

At 1920×1080 and 1280×800: all five bands present, Inspector ≥ 320 px,
sixteen columns seated (narrowing width+pitch together to the floor, never
scrolling or eliding a track), no overlap, minimum interactive target
respected. Verify by loading the recorded document at both window sizes and
capturing screenshots for the review record; encode the constraints in CSS
(clamp/minmax), not JS branching.

## Definition of Done

- [ ] `--shell webview` shows the authored MIXER from live projections;
      side-by-side with `figma-functional-interpretation/assets/mixer.png`
      reads as the same design
- [ ] `render` is pure; `renderObservation` exposed and documented
- [ ] Every vocabulary-named value comes from tokens.css (grep page.css for
      hex colors/px literals the vocabulary names — none)
- [ ] Meters animate from `crest://meters`; stale snapshot → stale state
- [ ] Both viewports hold per T021; screenshots attached to the WP record
- [ ] `spec-kitty agent tasks mark-status T017 T018 T019 T020 T021 --status done`

## Risks

- WKWebView font/grid rendering differing from the spike's Chrome — the
  viewport screenshots are the check; small metric drift is acceptable,
  structural drift is not.
- Purity erosion via incidental globals — keep render free of ambient state.

## Reviewer Guidance

Reject if: any token-named value is hand-written; the column grows or loses
a structure vs. the declared anatomy; track names caption non-header
structures; renders differ across two calls with one document; the page
registers a key handler (that is WP01/WP02's boundary — display only);
Inspector or a band disappears at 1280×800.
