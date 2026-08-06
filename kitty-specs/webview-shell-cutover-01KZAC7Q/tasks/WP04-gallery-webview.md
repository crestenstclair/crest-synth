---
work_package_id: WP04
title: Component gallery through the webview
dependencies:
- WP01
requirement_refs:
- C-006
- FR-005
planning_base_branch: feat/webview-shell-cutover
merge_target_branch: feat/webview-shell-cutover
branch_strategy: Planning artifacts for this mission were generated on feat/webview-shell-cutover. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/webview-shell-cutover unless the human explicitly redirects the landing branch.
subtasks:
- T014
- T015
- T016
history:
- '2026-08-06: authored from plan IC-04 (operator decision C-006: rebuild, not retire), crest-spec assets TestingContextModules/WebviewProjectionPage'
agent_profile: frontend-freddy
authoritative_surface: src/testing/component_gallery_scene.rs
create_intent:
- webview-page/gallery.css
- webview-page/gallery.js
execution_mode: code_change
owned_files:
- src/testing/component_gallery_scene.rs
- webview-page/gallery.css
- webview-page/gallery.js
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

Rebuild the Phase 4 component gallery on the webview surface (operator
decision 2026-08-06, spec C-006: rebuild, not retire). The fifteen pages —
vocabulary, eight controls, eight compositions, nine component states — render
through the webview at both authored densities from the SAME generated token
table the product uses. Browsing input stays exactly where it is: Rust-side,
scene-local digit bindings plus `[`/`]` stepping. The gallery keeps its
declared scope — no audio output, no MIDI event source, silence reported as a
measured field.

Authorities: crest-spec asset `TestingContextModules` gallery prompts (the
full contract: scene-local selection, coverage assertions, post-paint
observation, no autonomous-witness claim), `WebviewProjectionPage` gallery
prompt, spec FR-005/C-006/NFR-004.

## Context

- `src/testing/component_gallery_scene.rs` drives the gallery today: page
  registry, digit bindings 1–9/0 for the first ten pages in declared order,
  `[`/`]` bidirectional stepping without wrap, gallery observation emitted
  post-paint (page/state/viewport identity actually rendered), coverage
  assertion from the closed page and state sets.
- The scene currently paints through the egui gallery pages. You re-host it:
  the scene keeps registry/selection/observation; painting becomes gallery
  documents rendered by the webview through new `webview-page/gallery.css`
  and `gallery.js` (create_intent — new files, so no overlap with WP01's
  page files). Reuse `tokens.css` custom properties; import shared layout
  primitives from `page.css` via CSS custom properties rather than
  duplicating values.
- Representative content is a gallery privilege (production never invents
  values; the gallery may hold specimens). The bank specimen shows the same
  column anatomy the shipped MIXER shows.

## Subtasks

### T014 — Gallery documents and layouts

**Purpose**: fifteen pages render through the webview from gallery documents.

**Steps**:
1. Define the gallery document the scene sends: page identity, viewport
   density, and the specimen content for that page (controls in their
   applicable states, compositions filled with representative content).
   This is a gallery-scene document, not a fork of
   `SemanticGraphicalViewModel` — it lives beside it and makes no claim on
   the production schema (the byte-identity proof only governs production
   documents).
2. `gallery.js`: render dispatch per page kind; active page identity visible
   on screen; every state specimen labeled with text or shape beyond color.
3. `gallery.css`: layouts for specimen grids at both densities resolving
   every value from `tokens.css`; no literal that shadows a token.

**Validation**: every page renders at both densities with no clipped or
overlapping specimen labels.

### T015 — Scene re-hosted on the webview surface

**Purpose**: the scene drives the webview window instead of egui pages.

**Steps**:
1. In `component_gallery_scene.rs`, push gallery documents through the
   webview projection surface (same window/transport plumbing the product
   uses; the gallery selects its own document channel or reuses the
   projection channel's push mechanics as exposed — read-only use of WP02's
   modules).
2. Keep digit/stepping bindings scene-local Rust-side: a digit with no page
   bound retains the current page; stepping does not wrap; the eight
   pre-existing digit assignments keep exactly their pages.
3. Emit the gallery observation only after the page's painted ack for that
   gallery document — copy page/state/viewport identity actually rendered.
   Construct no audio output and no MIDI source; report that as the
   measured silence field, as today.

**Validation**: keyboard walk over all fifteen pages at both densities;
observation stream matches the pages actually visited.

### T016 — Coverage assertions and the make target

**Purpose**: a missing specimen or unbound page fails; the retained target
browses the webview gallery.

**Steps**:
1. Coverage assertions derive from the closed `ComponentGalleryPage` and
   `ComponentState` sets: every declared page reachable by digit or
   stepping; every declared state has a specimen on its pages. Keep them in
   the scene (your file), not in tests/ (WP05 owns those).
2. `make demo-live-component-library` opens the webview-hosted gallery
   (Makefile is WP03-owned — if the target line itself must change, record
   the one-line edit as out-of-map with rationale, or coordinate via lane
   notes if WP03 is still open).
3. Closing the window finishes the scene cleanly.

**Validation**: delete one specimen locally → coverage assertion fails;
restore → passes. `make demo-live-component-library` browses end to end.

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-cutover`.
Execution worktrees are allocated per computed lane from `lanes.json`; enter
the lane workspace `spec-kitty agent action implement WP04 --agent claude`
gives you.

## Definition of Done

- Fifteen pages, both densities, webview-rendered from the generated tokens;
  digit/stepping behavior byte-compatible with today's contract.
- Observation emitted post-paint with rendered identity; silence measured;
  no audio/MIDI constructed.
- Coverage assertions falsifiable (kill-a-specimen check done); target
  browses cleanly.
- No second styling source; no schema claim on the production document.

## Reviewer Guidance

- Check `gallery.js` for input handlers — the page captures no input, gallery
  included.
- Compare digit bindings against the pre-existing declared order — the eight
  originals must be untouched.
- grep `gallery.css` for literals shadowing tokens.
- Confirm the observation cannot be emitted pre-paint (ack-gated).
