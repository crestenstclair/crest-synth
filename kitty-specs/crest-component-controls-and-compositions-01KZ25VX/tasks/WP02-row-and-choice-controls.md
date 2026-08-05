---
work_package_id: WP02
title: Row and choice controls
dependencies:
- WP01
requirement_refs:
- FR-002
- FR-003
- FR-011
planning_base_branch: feat/crest-component-controls-and-compositions
merge_target_branch: feat/crest-component-controls-and-compositions
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-controls-and-compositions. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-controls-and-compositions unless the human explicitly redirects the landing branch.
subtasks:
- T008
- T009
- T010
- T011
- T012
phase: Phase 2 - Controls
history:
- at: '2026-08-02T21:46:28Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent: claude
agent_profile: designer-dagmar
authoritative_surface: src/shell/visual/controls/
create_intent:
- src/shell/visual/controls/parameter_row.rs
- src/shell/visual/controls/choice_row.rs
- src/shell/visual/controls/toggle.rs
- src/shell/visual/controls/browser_row.rs
execution_mode: code_change
owned_files:
- src/shell/visual/controls/parameter_row.rs
- src/shell/visual/controls/choice_row.rs
- src/shell/visual/controls/toggle.rs
- src/shell/visual/controls/browser_row.rs
role: designer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP02 – Row and choice controls

## ⚡ Do This First: Load Agent Profile

Use the `/ad-hoc-profile-load` skill to load the agent profile specified in the frontmatter, and behave according to its guidance before parsing the rest of this prompt.

- **Profile**: `designer-dagmar`
- **Role**: `designer`
- **Agent/tool**: `claude`

If no profile is specified, run `spec-kitty agent profile list` and select the best match for this work package's `task_type` and `authoritative_surface`.

---

## Markdown Formatting

Wrap HTML/XML tags in backticks: `` `<div>` ``, `` `<script>` ``
Use language identifiers in code blocks: ```rust, ```bash

---

## Objectives & Success Criteria

Build the four controls that appear as listed rows, against their Figma specimens, in every state each declares applicable, at both authored viewports.

Complete when:

- `ParameterRow`, `ChoiceRow`, `Toggle`, and `BrowserRow` each render from the Figma specimen — not from what the shell happens to render today.
- Each renders every state in its `applicable_states()` with **text or shape in addition to color**.
- No file declares a literal color, type size, spacing constant, or geometry value; everything resolves through `SemanticVisualToken` and `ViewportDensityPolicy`.
- No control owns, caches, or derives any value; each returns `ControlIntent` and dispatches nothing.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and the full suite are green.

## Context you need

- `.kittify/crest-spec/contexts/shell.yaml` — `valueObject.Shell.ComponentControl`. Its invariants are the acceptance criteria.
- `src/shell/visual/controls/mod.rs` (from WP01) — the signature, `PresentationRole`, `ControlIntent`, and each control's declared applicability.
- `src/shell/visual/primitives/` — text roles, hairlines, keylines, focus frames, value displays, status marks, action hints. **Compose these.** A control that draws its own focus keyline instead of using the focus primitive is the exact duplication Phase 4 exists to remove.
- `src/shell/visual/token.rs` and `density.rs` — every value comes from here.
- `research.md` R-02 — **paint, do not use egui widgets for appearance.** Use `ui.allocate_response` for hit-testing and `Painter` for every mark. No `egui::Checkbox`, no `egui::SelectableLabel`.
- `DESIGN.md:454` (Patch strip rows), `:466` (Inspector), `:370` (Sample Browser) — what these rows are for.

## The Figma rule

Each control's geometry, spacing, and state treatment comes from the Figma file linked in `DESIGN.md`, the way Phase 4a authored its tokens.

**If a control's specimen is missing or ambiguous, raise it. Do not approximate.** An approximated control looks authoritative in the gallery and is worse than a missing one, because nobody will know to re-check it.

Record for each control which Figma frame you took it from.

---

## Subtasks

### T008 — Build the parameter row `[P]`

**Purpose**: the most common control in the product — a labelled row showing one parameter's value, editable in place.

**Steps**:

1. Create `src/shell/visual/controls/parameter_row.rs` implementing the WP01 signature.
2. Layout from the Figma specimen: label on the left in `Label/Control`, value on the right in `Code/Value`, row height and pitch from `ViewportDensityPolicy`, inset from the policy's content inset.
3. Render the value from `SemanticControlValue` — `Scalar` and `Parameter` are the two it will receive. Format through the view model's own presentation; do not invent a number format.
4. `SemanticNumericRange` is available for continuous values. If the Figma specimen shows a fill or position indicator, drive it from the range; if the range is absent, render without it rather than assuming bounds.
5. State treatment per `ComponentState`, each with its non-color signal:
   - `Focused` — the 3 px `color/accent/focus` keyline plus its halo, via the focus primitive.
   - `Adjusting` — the 3 px `color/accent/adjust` keyline plus the cursor mark.
   - `Disabled` — 1 px keyline plus the `Locked` mark.
   - `Loading` — the adjustment accent plus the authored progress word (`Preparing` / `Activating`); reuse the structural-edit vocabulary, invent no second language.
   - `Error` — the warning accent plus typed short text from `SemanticError`.
   - `Selected` — the selected background plus its mark.
6. Handle `SemanticLifecycleStatus` — a row mid-structural-edit shows active and requested value plus its status, as `DESIGN.md:454` describes.
7. Return `ControlIntent`. Dispatch nothing.

**Files**: `src/shell/visual/controls/parameter_row.rs` (~200 lines)

**Validation**:
- Renders in all seven of its applicable states with non-color evidence in each.
- Identical at both viewports except for policy-resolved sizing.
- No literal anywhere; the guard passes.

---

### T009 — Build the choice row `[P]`

**Purpose**: a row whose value is one of a declared set, changed by the adjacent-choice gesture.

**Steps**:

1. Create `src/shell/visual/controls/choice_row.rs`.
2. Layout from the Figma specimen. The distinguishing mark from a parameter row is the adjacency affordance — whatever Figma shows for "there are neighbours in this direction".
3. **Non-wrapping is visible.** `DESIGN.md:309` — adjacent choice does not wrap. At the first or last choice the affordance in that direction must show unavailable, not absent. A missing affordance reads as "no more choices"; a greyed one reads as "you are at the end". Figma decides which; if it does not say, raise it.
4. Render the active label from the view model. For a structural choice mid-edit, show active and requested plus `Preparing` / `Activating` / typed failure.
5. All seven applicable states, each with non-color evidence, as T008.
6. Return `ControlIntent` carrying the direction asked for. It does not decide whether the choice is legal — that is the reducer's, and this control never reaches it.

**Files**: `src/shell/visual/controls/choice_row.rs` (~200 lines)

**Validation**:
- At a boundary, the unavailable direction is visibly distinct from an available one.
- The control never inspects whether a neighbouring choice exists beyond what the view model reports.

---

### T010 — Build the toggle `[P]`

**Purpose**: a two-state control — the only control asked in all four presentation roles.

**Steps**:

1. Create `src/shell/visual/controls/toggle.rs`.
2. It must render correctly in `ListedRow`, `VerticalStrip`, `PanelEntry`, and `ModalEntry`. Match on `role` exhaustively and give each its Figma layout. A single layout stretched across four roles will look wrong in at least two.
3. **The on/off state is not `ComponentState`.** On/off comes from `SemanticControlValue`; `ComponentState` is focus, adjustment, disabled, and so on. Conflating them is the most likely bug here — a disabled-and-on toggle must show both facts.
4. Non-color evidence for on/off as well as for state: shape or text, never color alone.
5. All seven applicable states.

**Files**: `src/shell/visual/controls/toggle.rs` (~190 lines)

**Validation**:
- All four roles render; the `role` match is exhaustive with no `_` arm.
- A disabled toggle in the on position shows both, distinguishably.

---

### T011 — Build the browser row `[P]`

**Purpose**: a row in an asset browser — the Sample Browser's row type, and the shape an `Asset` control takes.

**Steps**:

1. Create `src/shell/visual/controls/browser_row.rs`.
2. Layout from the Figma specimen: `DESIGN.md:370` describes the Sample Browser as supporting metadata and waveform preview, so a browser row carries more than a label. Render what Figma shows; **if it shows a waveform, render the waveform region as explicitly unavailable when the view data carries no waveform** — do not draw a decorative one.
3. Render from `AssetReference` in `SemanticControlValue::Asset`.
4. A locked asset row (a SoundFont file row is locked on PATCH, `DESIGN.md:454`) renders as `Disabled` with the `Locked` mark. Locked is a state it is handed, not something it decides.
5. All seven applicable states.

**Files**: `src/shell/visual/controls/browser_row.rs` (~190 lines)

**Validation**:
- No decorative content — every mark comes from view data or is marked unavailable.
- Hold-to-preview is **not** implemented here. That is Phase 7's Sample Browser workflow and the Start control is reserved and unbound until then (`WindowInput` invariant). Adding it now is out of scope.

---

### T012 — Assert every applicable state renders non-color evidence

**Purpose**: make FR-003 checkable rather than reviewed by eye.

**Steps**:

1. Add `#[cfg(test)]` assertions in each of the four files (not a shared test file — that would collide with WP03).
2. For each control, for each state in its `applicable_states()`: render it and assert the emitted output carries a non-color signal — the `NonColorSignal` the state declares, or a shape difference that does not depend on the color channel.
3. Assert each control paints no value that did not arrive in its `SemanticControlViewModel`. The cheapest form: render with a known view model and assert every text run appears in it.
4. Assert no control constructs a `SemanticAction`.

**Files**: all four control files

**Validation**:
- Removing a state's non-color treatment fails the assertion.
- The assertions live beside the controls and do not touch files owned by WP03.

---

## Branch Strategy

- **Planning base branch**: `feat/crest-component-controls-and-compositions`
- **Final merge target**: `feat/crest-component-controls-and-compositions`, and from there to `main`
- Execution worktrees are allocated per computed lane from `lanes.json`.

## Definition of Done

- All five subtasks complete; `mark-status` recorded.
- Four controls built from Figma specimens, with the source frame recorded for each.
- Every applicable state carries non-color evidence, asserted.
- Zero literals; the guard passes.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, full suite green.
- No file outside `owned_files` modified.

## Risks

- **Approximating a missing Figma specimen.** Raise it instead. This is the failure the mission cares most about avoiding.
- **Redrawing a primitive.** If you write a focus keyline, a hairline, or a status mark by hand, stop — Phase 4a already built it and duplicating it is the thing Phase 4 exists to prevent.
- **Conflating a control's value with its `ComponentState`.** Especially in the toggle.
- **Reaching for an egui widget** because painting a row is tedious. `research.md` R-02 rejected that on NFR-004 grounds; a styled egui widget hides a literal where the guard cannot see it.

## Reviewer Guidance

1. `grep` each file for a hex literal, a numeric font size, and a bare pixel constant. Any hit is a reject.
2. `grep` for `egui::Checkbox`, `SelectableLabel`, `Slider`, `ProgressBar`. Any hit is a reject.
3. For each control, cover the color channel mentally in each state — is the state still identifiable?
4. Does the toggle handle all four roles with an exhaustive match?
5. Does anything paint a value the view model did not supply?
6. Is the Figma source frame recorded per control? Without it, nobody can re-check fidelity later.
