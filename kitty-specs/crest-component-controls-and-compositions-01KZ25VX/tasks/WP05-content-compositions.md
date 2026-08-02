---
work_package_id: WP05
title: Content compositions and the no-placeholder rule
dependencies:
- WP01
- WP02
- WP03
requirement_refs:
- FR-004
- FR-010
planning_base_branch: feat/crest-component-controls-and-compositions
merge_target_branch: feat/crest-component-controls-and-compositions
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-controls-and-compositions. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-controls-and-compositions unless the human explicitly redirects the landing branch.
subtasks:
- T023
- T024
- T025
- T026
- T027
phase: Phase 3 - Compositions
history:
- at: '2026-08-02T21:46:28Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent_profile: implementer-ivan
authoritative_surface: src/shell/visual/compositions/
create_intent:
- src/shell/visual/compositions/section.rs
- src/shell/visual/compositions/patch_strip_row.rs
- src/shell/visual/compositions/utility_inspector_panel.rs
execution_mode: code_change
owned_files:
- src/shell/visual/compositions/section.rs
- src/shell/visual/compositions/patch_strip_row.rs
- src/shell/visual/compositions/utility_inspector_panel.rs
role: implementer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP05 – Content compositions and the no-placeholder rule

## ⚡ Do This First: Load Agent Profile

Use the `/ad-hoc-profile-load` skill to load the agent profile specified in the frontmatter, and behave according to its guidance before parsing the rest of this prompt.

- **Profile**: `implementer-ivan`
- **Role**: `implementer`
- **Agent/tool**: `claude`

If no profile is specified, run `spec-kitty agent profile list` and select the best match for this work package's `task_type` and `authoritative_surface`.

---

## Markdown Formatting

Wrap HTML/XML tags in backticks: `` `<div>` ``, `` `<script>` ``
Use language identifiers in code blocks: ```rust, ```bash

---

## Objectives & Success Criteria

Build the three compositions that arrange controls — section, Patch strip row, Utility/Inspector panel — and implement the rule that decides what happens when the design shows structure the projection does not drive.

Complete when:

- All three render from immutable projection slices, arrange controls through the WP01 selector, and return typed intent.
- **The omit-or-mark rule is implemented and asserted**: a designed structure with no view data behind it is omitted or marked explicitly unavailable, never invented.
- Every designed-but-undriven structure found while building is recorded.
- Zero literals; everything through `SemanticVisualToken` and `ViewportDensityPolicy`.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and the full suite are green.

## Context you need

- `.kittify/crest-spec/contexts/shell.yaml` — `valueObject.Shell.ShellComposition`, especially the invariant forbidding invented or representative values in the production shell.
- `src/shell/visual/compositions/mod.rs` (WP01), `src/shell/visual/controls/` (WP01–WP03).
- `src/adapter/eframe_graphical_window.rs` — read `paint_patch_workspace:468`, `paint_mixer_workspace:488`, `paint_side_region:627`, `paint_surface_summary:687`, and `paint_semantic_control:816`. That last one is the generic label-and-value row you are replacing with real controls.
- `src/control/semantic_graphical_view_model.rs` — `SemanticSurfaceViewModel`, `SemanticControlViewModel`, `SemanticSurfaceSummary` with its `Mixer`, `PatchUtility`, and `MixerInspector` variants.
- `DESIGN.md:454` — what the Patch strip contains: patch identity/routing, instrument selector, ordered post-effect selectors, and a persistent Utility panel for master/patch volume, MIDI input, output track, voice limit.
- `DESIGN.md:462-466` — mixer tracks and what the Inspector identifies.
- `DESIGN.md:444, 450` — the 420 px Utility/Inspector, always visible, retained at the compact viewport.

## The rule this work package exists to enforce

C-003. Where the Figma layout shows structure and the projection has no data for it:

- **Omit it**, or
- **Mark it explicitly unavailable.**

Never paint a plausible value. A placeholder in the shipped product misrepresents absent state as present, which is the same failure the product's "no silent fallback" principle exists to prevent everywhere else.

Representative content belongs in the gallery. That is what a gallery is for.

---

## Subtasks

### T023 — Build the section composition `[P]`

**Purpose**: a titled group of rows — the unit both PATCH and the Inspector are built from.

**Steps**:

1. Create `src/shell/visual/compositions/section.rs`.
2. It takes a title and an ordered slice of `SemanticControlViewModel`, and renders each through the WP01 selector at the `ListedRow` role.
3. Row pitch, inset, and the separator treatment come from `ViewportDensityPolicy` and the hairline primitive.
4. **The section supplies the role.** It never lets a control decide what it is (`ComponentControl` invariant).
5. A section with zero rows renders its title and an empty body, or is omitted by its caller. It does not invent a row.
6. Aggregate the intents its rows return.

**Files**: `src/shell/visual/compositions/section.rs` (~150 lines)

**Validation**:
- Every row goes through the selector; no direct control call bypasses it.
- Zero rows produces no invented row.

---

### T024 — Build the Patch strip row composition `[P]`

**Purpose**: one row of the PATCH strip — the composition the Phase 5 Patch editor will be built from.

**Steps**:

1. Create `src/shell/visual/compositions/patch_strip_row.rs`.
2. Read `paint_patch_workspace:468` for current behavior, then compose the designed row: identity, value, and status, with each control chosen through the selector at `ListedRow`.
3. **Structural-edit status is visible.** A row mid-structural-edit displays its active and requested value plus `Preparing`, `Activating`, or a typed failure, with the active graph explicit (`DESIGN.md:454`). That comes from `SemanticLifecycleStatus`; render it, do not summarize it away.
4. Read-only rows (Braids instrument scalars on PATCH, a locked SoundFont file row) are handed `Disabled` and render with the `Locked` mark. The composition does not decide read-only-ness.
5. **This composition adds no row.** The reducer owns the PATCH focus order (`DESIGN.md:309`); this renders what it is given, in the order given. Adding a row here would be adding product behavior, which C-002 forbids.
6. Both viewports from the density policy.

**Files**: `src/shell/visual/compositions/patch_strip_row.rs` (~180 lines)

**Validation**:
- Structural-edit status renders, not just the value.
- The composition adds no row and reorders nothing.
- No literal.

---

### T025 — Build the Utility/Inspector panel composition `[P]`

**Purpose**: the persistent 420 px side region — Utility on PATCH, Inspector on MIXER.

**Steps**:

1. Create `src/shell/visual/compositions/utility_inspector_panel.rs`.
2. Read `paint_side_region:627` and `paint_surface_summary:687` for current behavior.
3. Match exhaustively on `SemanticSurfaceSummary` — `Mixer`, `PatchUtility`, `MixerInspector` — with no `_` arm, so a new summary variant is a compile error.
4. **PATCH Utility** (`DESIGN.md:454`): master/patch volume, MIDI input, output track, voice limit. Render each control the projection supplies, through the selector at `PanelEntry` or `ListedRow` as the Figma layout dictates.
5. **MIXER Inspector** (`DESIGN.md:466`): cursor, value/range, mute/solo, route/sends.
6. **This is where the placeholder temptation is strongest.** The Figma panel is dense and the projection may not drive every field. For each field the projection does not supply: omit it or mark it explicitly unavailable, and record it for T027.
7. The panel stays visible at the compact viewport (`DESIGN.md:450`) — proportional width and controlled density, never hidden.

**Files**: `src/shell/visual/compositions/utility_inspector_panel.rs` (~200 lines)

**Validation**:
- Exhaustive match on `SemanticSurfaceSummary`, no `_` arm.
- Visible at 1280×800 with minimum targets intact.
- Every undriven field is omitted or marked — none invented.

---

### T026 — Implement and assert the omit-or-mark rule

**Purpose**: make C-003 a mechanism rather than a discipline.

**Steps**:

1. Add a shared way for a composition to express "this designed structure has no data": a typed unavailable marker rendered through the existing status primitive, reusing Phase 4a's vocabulary rather than inventing a second one.
2. **The marker is for the production shell.** In the gallery, representative content is supplied and the marker does not appear — that difference is legitimate and comes from the view data, not from a mode flag inside the composition. A composition must not know whether it is in the gallery.
3. Add `#[cfg(test)]` assertions in each of the three files: hand the composition a projection slice missing part of its designed structure and assert it paints no text that was not in the slice, other than the declared unavailable marker and static labels.
4. Assert none of the three constructs a `SemanticAction`, reads `AppState`, or takes a raw viewport size.

**Files**: all three composition files

**Validation**:
- The assertion fails if a composition is changed to paint a default value.
- No composition branches on gallery-versus-production.

---

### T027 — Record every designed structure the projection does not drive

**Purpose**: this list is the real input to Phase 5. Capture it while you are looking at it, not by reconstructing it later.

**Steps**:

1. As you build T023–T025, keep a running list. For each item: the Figma frame, the structure shown, what would drive it, and what you did (omitted or marked).
2. Write it into this work package's completion notes as a table.
3. **Do not add view model fields to fix any of them.** C-002 — the view model is not extended in this mission. The gap is the finding; closing it is Phase 5's job.
4. If the list is empty, say so explicitly. That is a meaningful result, not an omission.

**Files**: work package completion notes (no source file)

**Validation**:
- The list exists and is specific enough that Phase 5 can act on it without re-deriving.
- No view model change was made.

---

## Branch Strategy

- **Planning base branch**: `feat/crest-component-controls-and-compositions`
- **Final merge target**: `feat/crest-component-controls-and-compositions`, and from there to `main`
- Execution worktrees are allocated per computed lane from `lanes.json`.

## Definition of Done

- All five subtasks complete; `mark-status` recorded.
- Three compositions built, all arranging controls through the WP01 selector.
- The omit-or-mark rule implemented and asserted.
- The undriven-structure list recorded (or explicitly empty).
- Zero literals; the guard passes.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, full suite green.
- No file outside `owned_files` modified; no view model change.

## Risks

- **Painting a placeholder.** The single risk this work package exists to manage. A dense designed panel with sparse data looks broken, and filling it in feels like doing the job. It is not.
- **Extending the view model.** The natural fix for an undriven field, and forbidden by C-002. Record it instead.
- **Bypassing the selector** by calling a control directly because you know which one you want. That defeats FR-001 and the totality proof in WP08 will catch it — but late.
- **A gallery-versus-production flag inside a composition.** If a composition knows where it is being rendered, the boundary has leaked.

## Reviewer Guidance

1. `grep` all three files for hex literals, numeric font sizes, bare pixel constants — any hit is a reject.
2. `grep` for direct control calls that bypass the selector. Every control render must go through it.
3. Hand a composition a sparse projection. Does anything appear that was not in it? Any invented value is a reject.
4. Does any composition branch on whether it is in the gallery? That is a reject.
5. Is the undriven-structure list present and specific? "None found" is acceptable; silence is not.
6. Is the Utility/Inspector still visible and usable at 1280×800?
