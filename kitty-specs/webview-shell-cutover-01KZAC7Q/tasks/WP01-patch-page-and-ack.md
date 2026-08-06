---
work_package_id: WP01
title: PATCH surface and acknowledgment in the page
dependencies: []
requirement_refs:
- FR-001
planning_base_branch: feat/webview-shell-cutover
merge_target_branch: feat/webview-shell-cutover
branch_strategy: Planning artifacts for this mission were generated on feat/webview-shell-cutover. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/webview-shell-cutover unless the human explicitly redirects the landing branch.
subtasks:
- T001
- T002
- T003
- T004
history:
- '2026-08-06: authored from plan IC-01 (page half of IC-02), crest-spec asset WebviewProjectionPage'
agent_profile: frontend-freddy
authoritative_surface: webview-page/
create_intent: []
execution_mode: code_change
owned_files:
- webview-page/index.html
- webview-page/page.css
- webview-page/page.js
- tests/webview_projection_shell.rs
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

Extend the webview page from MIXER-only to the full PATCH context, and make the
page acknowledge every painted document. The page renders the received serde
serialization of `SemanticGraphicalViewModel` — the SAME document the MIXER
path consumes today — and PATCH is already fully present in that model (the
egui shell renders it now). Your job is layout and rendering, never schema:
if you find yourself wanting a field the document doesn't carry, STOP — that
is a projector change and out of scope (crest-spec
`requirement.serialized_projection_transport`: one schema, no page-invented
field, no webview variant).

Authorities: crest-spec asset `WebviewProjectionPage` (its prompts are your
contract), `adapter.TauriWebviewWindow` rules, `DESIGN.md` + Figma for the
authored PATCH composition, and the shipped egui PATCH rendering as the
behavioral reference for what content appears where.

## Context

- `webview-page/` today: `index.html`, `page.js`, `page.css`, `tokens.css`
  (generated — NEVER hand-edit). The MIXER composition ships and is proven by
  `tests/webview_projection_shell.rs` (document byte-identity, token
  freshness, deterministic render, both viewports).
- The document arrives via the projection channel; meters arrive separately.
  The page holds no state between documents beyond presentation-only
  animation, registers NO input handlers (asserted by an existing test in
  `src/shell/webview/window.rs` — do not break it), and escapes every
  document-derived string.
- Both authored viewports must pass: 1920x1080 and 1280x800 (context line,
  identity header, main workspace, persistent Utility region, footer all
  visible, non-overlapping).

## Subtasks

### T001 — PATCH workspace layout in page CSS

**Purpose**: the authored PATCH composition seats at both viewports through
the browser layout engine, resolving every value from `tokens.css`.

**Steps**:
1. Study the shipped egui PATCH screen (run the app, press `1` for PATCH) and
   the Figma reference. Identify the regions the projection carries: Patch
   strip row, identity header content, envelope (ADSR) rows, engine row,
   effect-slot rows, generic parameter rows, persistent Utility region rows,
   footer hints.
2. Add PATCH layout rules to `page.css` using CSS grid/flex — no absolute-px
   hand arithmetic where a token or grid rule serves. Every color, type
   style, spacing step, and geometry value comes from `var(--…)` custom
   properties in `tokens.css`. If a needed value has no token, use the
   nearest declared token and record the gap in your lane notes — do NOT
   invent a literal (the no-literal proof will hunt it).
3. Both viewports: reuse the existing viewport-density class mechanism the
   MIXER layout uses; PATCH must not introduce a third layout mode.

**Validation**: with `CREST_WEBVIEW_PAGE` pointing at a fixture document for
PATCH, the composition shows every region, nothing overlaps, nothing clips at
either viewport.

### T002 — PATCH render sections + focus/state treatments in page JS

**Purpose**: `page.js` renders PATCH surfaces from the document with the same
determinism and passivity the MIXER path has.

**Steps**:
1. Extend the render dispatch in `page.js`: when the document's context is
   PATCH, render the PATCH workspace from its surfaces/controls; keep the
   context line, identity header, side region, footer, and hint rows driven
   by the same shared code paths MIXER uses (they are structural bands, not
   per-context forks).
2. Focused-control emphasis: apply the focus treatment (keyline/halo classes
   already used by the MIXER focused column) to the focused control row from
   the document's focus path. Never compute or guess focus — it is in the
   document.
3. State treatments: render every control state present in the document
   (focused, adjusting, disabled, error, status marks…) with text or shape
   in addition to color, matching the declared ComponentState treatments.
   Match exhaustively on what the document can carry; an unknown state string
   renders as an explicit visible `?state` marker, never silently as normal.
4. `escapeHtml` every document-derived string, as the MIXER path does.

**Validation**: same document in twice → byte-identical DOM (the determinism
proof in T004 covers this); focus follows the document when a new document
arrives.

### T003 — Paint-acknowledgment emission

**Purpose**: after painting a document, the page acknowledges it so the
adapter (WP02) can emit a `ShellFrameObservation`.

**Steps**:
1. After the DOM update for a document completes (end of the render call,
   after layout-affecting mutations), emit one acknowledgment through the
   existing page→Rust channel mechanism (the same IPC surface the foundation
   used for its painted-ack; see `src/shell/webview/projection_channel.rs`
   comments for the receiving side — read-only for you, WP02 owns it).
2. The ack carries the document's semantic identity — generation, stateHash,
   context, active surface, focus path, interaction mode — copied verbatim
   from the received document, and nothing else. The page never invents,
   caches, or re-derives identity.
3. Exactly one ack per painted document, in paint order. A document that
   fails to render must NOT ack — throw so the adapter's typed
   render-exception surfacing (WP02 T008) sees it.

**Validation**: T004 asserts one ack per supplied document with the exact
identity fields.

### T004 — Extend the fidelity proof to PATCH and acknowledgments

**Purpose**: `tests/webview_projection_shell.rs` proves what T001–T003 built.

**Steps**:
1. Extend the fixture set to PATCH documents: a navigate-mode document, an
   adjust-mode document with a focused editable control, a document with a
   disabled/error state present.
2. Assert byte-identity of the page-facing document with the projector's
   serialization for the PATCH fixtures (the existing MIXER assertion,
   widened — the crest-spec asset prompt now says "across both contexts").
3. Assert deterministic render (same document → same declared observation
   structure) for PATCH at both viewports, reusing the existing harness.
4. Assert one acknowledgment per painted document carrying the exact
   generation/stateHash/context/surface/focus/mode of its document.
5. Keep the marker discipline: `CREST_ACCEPTANCE webview_projection_shell
   passed` only after every assertion.

**Validation**: `cargo test --test webview_projection_shell -- --nocapture`
green with the marker; `spec-kitty accept` validation
`webview_projection_shell` passes.

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-cutover`.
Execution worktrees are allocated per computed lane from `lanes.json`; enter
the lane workspace `spec-kitty agent action implement WP01 --agent claude`
gives you. Do not branch by hand.

## Definition of Done

- PATCH renders at authored parity from the canonical document at both
  viewports; MIXER rendering unchanged.
- Every painted document acks exactly once with exact identity; failed
  renders throw and do not ack.
- No new input handler, no page-held state, no literal styling value, no
  schema fork.
- `webview_projection_shell` marker green; `window.rs` no-input-handler test
  still green.

## Reviewer Guidance

- Diff `page.js` for any `addEventListener` on key/mouse input — instant
  reject.
- grep `page.css` diff for numeric literals that shadow a token (`px`, `rem`,
  hex colors) — each must be a `var(--…)` or have a recorded gap note.
- Verify the ack fields against a captured document — identity must be
  copied, not recomputed.
- Run the twin twice; DOM snapshots must match.
