---
work_package_id: WP04
title: Frame compositions
dependencies:
- WP01
requirement_refs:
- FR-004
- FR-010
planning_base_branch: feat/crest-component-controls-and-compositions
merge_target_branch: feat/crest-component-controls-and-compositions
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-controls-and-compositions. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-controls-and-compositions unless the human explicitly redirects the landing branch.
subtasks:
- T018
- T019
- T020
- T021
- T022
phase: Phase 3 - Compositions
history:
- at: '2026-08-02T21:46:28Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent_profile: implementer-ivan
authoritative_surface: src/shell/visual/compositions/
create_intent:
- src/shell/visual/compositions/application_shell.rs
- src/shell/visual/compositions/context_switch.rs
- src/shell/visual/compositions/identity_header.rs
- src/shell/visual/compositions/footer.rs
execution_mode: code_change
owned_files:
- src/shell/visual/compositions/application_shell.rs
- src/shell/visual/compositions/context_switch.rs
- src/shell/visual/compositions/identity_header.rs
- src/shell/visual/compositions/footer.rs
role: implementer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP04 – Frame compositions

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

Build the four compositions that form the shell frame: the application shell that arranges the bands, the context switch, the identity header, and the footer.

Complete when:

- All four render from immutable projection slices and return typed intent.
- Every band height, split width, inset, and row pitch resolves from `ViewportDensityPolicy`.
- **None of the four declares a color, type style, spacing step, or geometry value of its own.**
- The frame renders correctly at both authored viewports with all bands and the persistent side region retained.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and the full suite are green.

## Context you need

- `.kittify/crest-spec/contexts/shell.yaml` — `valueObject.Shell.ShellComposition`. Its invariants are the acceptance criteria.
- `src/shell/visual/compositions/mod.rs` (WP01) — the family, the region binding, the signature.
- `src/adapter/eframe_graphical_window.rs` — the current painting. **Read `paint_context_line:357`, `paint_identity_header:418`, `paint_main_workspace:440`, and `paint_footer:733` carefully.** These are what you are replacing, and WP06 will delete them. The behavior they produce is what existing tests pin, so preserve it.
- `src/control/graphical_shell_projection.rs` — `ShellContextLine`, `ShellIdentityHeader`, `ShellWorkspace`, `ShellFooter`. These are the projection slices you receive.
- `DESIGN.md:444` — the authored band structure and the 420 px always-visible Utility/Inspector.
- `DESIGN.md:450` — the compact viewport must preserve bands, the visible Utility/Inspector, and minimum targets using proportional widths and controlled density, never by hiding required context.
- `DESIGN.md:514` — the footer echoes the current context/path and **only actions valid at the focused target**.

## The move-then-author order

Move the existing behavior into the composition first and confirm it still renders. Then apply Figma fidelity inside the composition. Doing fidelity first makes it impossible to tell whether a difference is an improvement or a regression.

---

## Subtasks

### T018 — Build the application shell composition

**Purpose**: the composition that arranges the five structural bands and hands each region to its own composition. This is the one the adapter will call.

**Steps**:

1. Create `src/shell/visual/compositions/application_shell.rs`.
2. Resolve band heights and the main/side split from `ViewportDensityPolicy` — never from a constant, and never by branching on a viewport size.
3. Arrange, in the authored order: context line, identity header, main workspace, persistent side region, footer. Hand each to its composition.
4. **Design the signature for WP06.** The adapter will hand you an `egui::Ui` (or the panels it has constructed) and the projection, and expects back the intent plus whatever geometry the frame observation needs. If the adapter has to make a layout decision to call you, the signature is wrong and FR-006 fails.
5. Return aggregated `ControlIntent` from the regions.
6. Do not construct `ShellFrameObservation` here. The rectangles come from egui panel responses the adapter owns (`research.md` R-03); threading them back out would buy nothing.

**Files**: `src/shell/visual/compositions/application_shell.rs` (~180 lines)

**Validation**:
- No band height, split width, or inset literal in the file.
- Both viewports render all five bands and the side region.
- The signature leaves the adapter no layout decision.

---

### T019 — Build the context switch composition `[P]`

**Purpose**: the PATCH/MIXER top-level context indicator and switch.

**Steps**:

1. Create `src/shell/visual/compositions/context_switch.rs`.
2. Read `paint_context_line` (`eframe_graphical_window.rs:357`) for current behavior — product label, context label, status label across a horizontal strip.
3. **PATCH and MIXER are the only two top-level contexts** and that is a product invariant, not a list. Match on `TopLevelContext` exhaustively; a third would be a compile error, which is correct.
4. The inactive context is visible and distinguishable from the active one **without color** — this is a two-item switch where color-only would be the easy mistake.
5. Layout, spacing, and treatment from the Figma specimen once behavior is preserved.

**Files**: `src/shell/visual/compositions/context_switch.rs` (~150 lines)

**Validation**:
- Active and inactive distinguishable without color.
- Exhaustive match on `TopLevelContext`, no `_` arm.

---

### T020 — Build the identity header composition `[P]`

**Purpose**: the band showing what is being edited.

**Steps**:

1. Create `src/shell/visual/compositions/identity_header.rs`.
2. Read `paint_identity_header` (`:418`) — primary and secondary labels from `ShellIdentityHeader`.
3. Type styles from the token vocabulary: the primary label and secondary label take their authored styles, not sizes chosen here.
4. **Long labels.** A Patch name can be longer than the band. Figma decides — truncation with an ellipsis, or shrink. Whichever it is, the result must not clip or overlap, because the gallery witness asserts `clipped_or_overlapping_text == 0`. If Figma does not say, raise it.
5. Band height from the density policy.

**Files**: `src/shell/visual/compositions/identity_header.rs` (~140 lines)

**Validation**:
- A label longer than the band neither clips nor overlaps at either viewport.
- No type size literal.

---

### T021 — Build the footer composition `[P]`

**Purpose**: the band echoing the current path and the actions valid right now.

**Steps**:

1. Create `src/shell/visual/compositions/footer.rs`.
2. Read `paint_footer` (`:733`) — path label on one side, action hints on the other, from `ShellFooter`.
3. Compose the **action hint primitive** from Phase 4a for each hint. Do not draw hints by hand.
4. **Only valid actions appear** (`DESIGN.md:514`). The footer renders the hints the projection supplies and adds none. If the projection supplies none, the region is empty — not filled with plausible defaults. That is C-003 in miniature.
5. Hints must fit at the compact viewport. If more hints arrive than fit, Figma decides the rule; do not silently drop one.

**Files**: `src/shell/visual/compositions/footer.rs` (~150 lines)

**Validation**:
- The footer invents no hint.
- With zero hints supplied, the region renders empty rather than defaulted.
- Hints do not clip at 1280×800.

---

### T022 — Assert the frame compositions declare no visual value

**Purpose**: make the "compositions own no values" invariant checkable.

**Steps**:

1. Add `#[cfg(test)]` assertions in each of the four files (not shared — WP05 owns the other three composition files).
2. Assert each composition, given a projection slice, paints only text present in that slice.
3. Assert each renders at both authored viewports with all bands and the side region retained, no clipping, no overlap.
4. Assert none constructs a `SemanticAction`, reads `AppState`, or takes a raw viewport size.
5. Confirm the Phase 4a literal guard passes with these four files present.

**Files**: all four composition files

**Validation**:
- The guard passes.
- The both-viewport assertions fail if a band is dropped.

---

## Branch Strategy

- **Planning base branch**: `feat/crest-component-controls-and-compositions`
- **Final merge target**: `feat/crest-component-controls-and-compositions`, and from there to `main`
- Execution worktrees are allocated per computed lane from `lanes.json`.

## Definition of Done

- All five subtasks complete; `mark-status` recorded.
- Four compositions built; behavior preserved first, Figma fidelity applied second.
- `ApplicationShell`'s signature leaves the adapter no layout decision — WP06 depends on this.
- Zero literals; the guard passes.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, full suite green.
- No file outside `owned_files` modified.

## Risks

- **`ApplicationShell`'s signature is the constraint on WP06.** If it forces the adapter to keep deciding paint order, NFR-003 becomes unreachable and FR-006 fails. Design it against the adapter's `paint_shell` (`:201`) explicitly, and check that every decision that function makes has somewhere to go.
- **Doing Figma fidelity before the move.** Then a rendering difference is unattributable.
- **The footer filling itself in.** An empty region looks broken and inviting a default is natural. C-003 forbids it.

## Reviewer Guidance

1. `grep` all four files for hex literals, numeric font sizes, band-height constants — any hit is a reject.
2. Open `ApplicationShell`'s signature. Could the adapter call it without deciding anything visual? If not, reject.
3. Does the context switch distinguish active from inactive without color?
4. Render each at 1280×800. Any clipping or overlap is a reject — the gallery witness asserts zero.
5. Does the footer render exactly the hints supplied, and nothing when none are?
