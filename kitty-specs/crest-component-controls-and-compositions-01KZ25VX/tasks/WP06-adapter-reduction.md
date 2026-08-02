---
work_package_id: WP06
title: Adapter reduction
dependencies:
- WP04
- WP05
requirement_refs:
- FR-005
- FR-006
planning_base_branch: feat/crest-component-controls-and-compositions
merge_target_branch: feat/crest-component-controls-and-compositions
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-controls-and-compositions. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-controls-and-compositions unless the human explicitly redirects the landing branch.
subtasks:
- T028
- T029
- T030
- T031
- T032
- T033
phase: Phase 4 - Production recomposition
history:
- at: '2026-08-02T21:46:28Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent_profile: paula-patterns
authoritative_surface: src/adapter/
create_intent: []
execution_mode: code_change
owned_files:
- src/adapter/eframe_graphical_window.rs
role: architecture-scout
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP06 – Adapter reduction

## ⚡ Do This First: Load Agent Profile

Use the `/ad-hoc-profile-load` skill to load the agent profile specified in the frontmatter, and behave according to its guidance before parsing the rest of this prompt.

- **Profile**: `paula-patterns`
- **Role**: `architecture-scout`
- **Agent/tool**: `claude`

If no profile is specified, run `spec-kitty agent profile list` and select the best match for this work package's `task_type` and `authoritative_surface`.

---

## Markdown Formatting

Wrap HTML/XML tags in backticks: `` `<div>` ``, `` `<script>` ``
Use language identifiers in code blocks: ```rust, ```bash

---

## Objectives & Success Criteria

Move every region and control paint out of `src/adapter/eframe_graphical_window.rs` and into the compositions and controls. Leave window plumbing, event translation, and the frame-observation emit.

Complete when:

- Every region the window shows is painted by a `ShellComposition`; every control within one by a `ComponentControl`.
- The adapter decides **no** paint, layout, band height, or state visualization.
- `wc -l src/adapter/eframe_graphical_window.rs` reports **≤ 512** (from 1,282) — NFR-003.
- The full existing suite passes with **no existing test file modified** — NFR-005.
- `make run` renders the product correctly at both viewports.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings` clean.

## This is the highest-risk work package in the mission

It touches the one file every existing shell test observes. Two rules govern it:

1. **NFR-005 is not negotiable.** If a shell, projection, or focus test fails, the recomposition changed behavior. **Fix the recomposition. Do not edit the test.** Editing the test to match new behavior converts a caught regression into a shipped one.
2. **`ShellFrameObservation` must survive intact.** It is exactly what those tests assert on. Its construction stays in the adapter, built from the rectangles the egui panels actually produced (`research.md` R-03).

## Context you need

- `src/adapter/eframe_graphical_window.rs` — read the whole file before changing anything. The functions in scope: `paint_shell:201`, `paint_context_line:357`, `paint_identity_header:418`, `paint_main_workspace:440`, `paint_patch_workspace:468`, `paint_mixer_workspace:488`, `paint_diagnostic:602`, `paint_side_region:627`, `paint_surface_summary:687`, `paint_footer:733`, `control_state:797`, `paint_semantic_control:816`, `semantic_value_label:898`, `chrome_text:920`, `padded_text:930`, `trailing_text:939`, `hairline_separator:950`, `margin:968`, `install_authored_chrome:322`, `shell_frame:350`.
- `src/shell/visual/compositions/` and `controls/` (WP01, WP04, WP05) — the destinations.
- `.kittify/crest-spec/contexts/shell.yaml` — `port.Shell.AppWindow`, including the new invariant that every region is painted by a composition and the existing ones about passivity and observation.
- `research.md` R-03 — the three things that stay and why.

## What stays in the adapter

1. **Window plumbing** — `eframe::App::update`, panel construction (`TopBottomPanel`, `CentralPanel`), close requests, the tick callback.
2. **Event translation** — the egui key → `WindowKey` normalization at `:1287`. Boundary work, not paint.
3. **The observation emit** — `ShellFrameObservation` constructed after painting from real panel rectangles, with its current shape and invariants.

Everything else moves.

---

## Subtasks

### T028 — Capture the pre-reduction behavioral baseline

**Purpose**: make "unchanged" checkable rather than asserted.

**Steps**:

1. Run the full suite on the current tree and record the result to a file. **Never pipe test output through `head` or `tail`** — the pipe reports the pager's exit code and a "green" recorded that way is a lie. Redirect:
   ```bash
   cargo test --release > /tmp/wp06-baseline.log 2>&1; echo "exit=$?"
   ```
2. Record `wc -l src/adapter/eframe_graphical_window.rs` — the 1,282 starting point.
3. Run `make run`, look at both PATCH and MIXER, and capture what the product currently looks like. You need this to tell a regression from an improvement.
4. List every existing test that touches the shell, the projection, or focus. These are the ones NFR-005 protects; know their names before you start.

**Files**: none (baseline capture)

**Validation**:
- The baseline log exists, exit code recorded, and the protected test list is written down.

---

### T029 — Move the band and workspace painting into compositions

**Purpose**: relocate the frame.

**Steps**:

1. Replace `paint_context_line`, `paint_identity_header`, `paint_main_workspace`, and `paint_footer` with calls into their compositions.
2. Restructure `paint_shell:201` so it constructs the egui panels and hands each to `ApplicationShell`, keeping the panel responses it needs for the observation.
3. **Move, do not rewrite.** Where a composition already implements the behavior (WP04 preserved it), call it. Where the adapter has behavior WP04 did not carry over, that is a gap — move it into the composition, do not leave it in the adapter.
4. Move `paint_patch_workspace` and `paint_mixer_workspace` into the compositions that own those surfaces.
5. Run the suite after each region. A regression is much cheaper to find one region at a time than five.

**Files**: `src/adapter/eframe_graphical_window.rs`

**Validation**:
- The suite is green after each region moves.
- No band height or paint-order decision remains in the adapter for these regions.

---

### T030 — Move the side region and control painting into compositions

**Purpose**: relocate the content, including the generic control row that FR-002 replaces.

**Steps**:

1. Replace `paint_side_region` and `paint_surface_summary` with calls into `UtilityInspectorPanel`.
2. **Delete `paint_semantic_control:816` and `control_state:797`.** Control rendering now goes through the WP01 selector. This is the single biggest visible change in the mission — every control stops being a generic label-and-value row and becomes its designed shape.
3. Move `semantic_value_label:898` into the control that needs it, or delete it if the controls already format through the view model. Do not leave a formatting helper in the adapter.
4. Move or delete the local text helpers — `chrome_text:920`, `padded_text:930`, `trailing_text:939`, `hairline_separator:950`, `margin:968`. Each is a primitive Phase 4a already provides; they should not survive in either place as duplicates.
5. `paint_diagnostic:602` — decide deliberately. If it is product surface, it becomes a composition or part of one. If it is developer scaffolding, say so and keep it minimal and clearly marked. Do not leave it undecided.
6. `install_authored_chrome:322` and `shell_frame:350` — these configure egui style from tokens. Keeping them means the adapter still holds visual decisions. Move them behind the vocabulary or into `ApplicationShell`.

**Files**: `src/adapter/eframe_graphical_window.rs`

**Validation**:
- `paint_semantic_control` and `control_state` are gone.
- No text, spacing, or separator helper remains in the adapter.
- Controls visibly render as their designed shapes when you run the product.

---

### T031 — Keep event translation and the frame observation intact

**Purpose**: preserve the two things the adapter must keep, exactly.

**Steps**:

1. Leave the egui key → `WindowKey` mapping at `:1287` in place. It is boundary normalization.
2. Keep `ShellFrameObservation` construction in the adapter, built from panel responses. Confirm every field still carries what it carried: viewport, generation, stateHash, context, activeSurface, focusPath, interactionMode, and the five region rectangles with their visible-label evidence.
3. **The observation is emitted only after painting** and copies the projection's semantic identity exactly. Do not let the recomposition move the emit earlier or compute a rectangle from a layout plan instead of a painted result — that invariant is what makes the observation non-vacuous.
4. Verify the region rectangles still correspond to what the compositions actually painted, not to what the panels were asked for.

**Files**: `src/adapter/eframe_graphical_window.rs`

**Validation**:
- Every existing observation assertion passes unmodified.
- The emit is still after painting and still derived from real rectangles.

---

### T032 — Delete the vacated code and verify the threshold

**Purpose**: NFR-003. The line count is how we know the move actually happened.

**Steps**:

1. Delete every function whose body moved. Do not leave a one-line shim that forwards to a composition — seven forwarding shims still constitute the adapter deciding paint order, which FR-006 forbids.
2. Delete now-unused imports and helpers.
3. Verify:
   ```bash
   wc -l src/adapter/eframe_graphical_window.rs
   ```
   Must be **≤ 512**.
4. **If it lands well above 512** — say 600 — that is a signal something visual stayed behind, not that the threshold was too strict. Find it. The most likely candidates are style configuration, a text helper, or a layout branch.
5. `cargo clippy --all-targets -- -D warnings` must be clean, including no dead code.

**Files**: `src/adapter/eframe_graphical_window.rs`

**Validation**:
- ≤ 512 lines.
- No forwarding shims.
- Clippy clean, no dead-code allowances added.

---

### T033 — Verify the full suite passes with no existing test modified

**Purpose**: NFR-005 — the gate that proves this was a re-composition and not a behavior change.

**Steps**:

1. Run the full suite, redirected to a file:
   ```bash
   cargo test --release > /tmp/wp06-after.log 2>&1; echo "exit=$?"
   ```
2. Compare against the T028 baseline. Same tests, same passes.
3. Run:
   ```bash
   git diff --name-only <base> -- tests/ src/**/tests.rs
   ```
   **Any existing test file appearing in that diff is a violation.** Adding a new test file is fine; modifying an existing one to accommodate this work package is not.
4. Run `make run` and compare both contexts against the T028 capture. Controls should now look like their designed shapes; nothing should be missing, misplaced, or clipped.
5. Confirm no `SemanticAction` variant, focus target, or reducer behavior was added (C-002).

**Files**: none (verification)

**Validation**:
- Suite green, exit 0, recorded to a file rather than piped.
- Zero existing test files modified.
- The product renders correctly at both viewports.

---

## Branch Strategy

- **Planning base branch**: `feat/crest-component-controls-and-compositions`
- **Final merge target**: `feat/crest-component-controls-and-compositions`, and from there to `main`
- Execution worktrees are allocated per computed lane from `lanes.json`.

## Definition of Done

- All six subtasks complete; `mark-status` recorded.
- Adapter ≤ 512 lines with no paint, layout, band-height, or state-visualization decision left.
- `paint_semantic_control` deleted; controls render as designed shapes.
- Frame observation intact and still non-vacuous.
- Full suite green with zero existing test files modified.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings` clean.
- No file outside `owned_files` modified.

## Risks

- **Editing a test to make it pass.** The failure mode that would make this whole mission worthless. If a shell test fails, the recomposition is wrong.
- **Forwarding shims instead of deletion.** Hits the line count without satisfying FR-006. Seven one-line shims still encode paint order.
- **The observation quietly becoming vacuous** — computed from a layout plan instead of painted rectangles. Its own invariant forbids this and it is easy to do accidentally while restructuring `paint_shell`.
- **Style configuration left behind.** `install_authored_chrome` looks like plumbing and is actually visual authority.

## Reviewer Guidance

1. `wc -l src/adapter/eframe_graphical_window.rs`. Over 512 is a reject; just under it, look for what is still hiding.
2. `git diff --name-only` over `tests/`. Any existing test modified is a reject, regardless of the justification.
3. `grep` the adapter for a color, size, spacing, or band-height constant. Any hit is a reject.
4. Are there forwarding shims — functions whose whole body is one call into a composition? Reject.
5. Read the observation construction. Is it after painting? Does it use real panel rectangles?
6. Run `make run`. Do the controls look like designed shapes, or still like generic rows? If still generic, `paint_semantic_control` did not really go away.
