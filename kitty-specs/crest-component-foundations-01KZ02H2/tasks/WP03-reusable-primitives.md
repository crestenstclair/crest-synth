---
work_package_id: WP03
title: Reusable primitives
dependencies:
- WP01
- WP02
requirement_refs:
- FR-004
- FR-005
- FR-009
planning_base_branch: feat/crest-component-foundations
merge_target_branch: feat/crest-component-foundations
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-foundations. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-foundations unless the human explicitly redirects the landing branch.
subtasks:
- T013
- T014
- T015
- T016
- T017
- T018
- T019
phase: Phase 2 - Component layer
history:
- at: '2026-08-02T02:26:18Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent_profile: implementer-ivan
agent: claude
authoritative_surface: src/shell/visual/primitives
create_intent:
- src/shell/visual/primitives.rs
execution_mode: code_change
owned_files:
- src/shell/visual/primitives.rs
- src/shell/visual/primitives/**
role: implementer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP03 – Reusable primitives

## ⚡ Do This First: Load Agent Profile

Use the `/ad-hoc-profile-load` skill to load the agent profile specified in the frontmatter (or any user-defined profile), and behave according to its guidance before parsing the rest of this prompt.

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

Build the seven primitive families that later screens compose from, as **passive** functions over
immutable data plus explicit state.

Complete when:

- Text roles, hairline, keyline, focus frame, value display, status mark, and action hint all exist.
- Each accepts immutable view data plus an explicit `ComponentState` and paints — nothing more.
- Every applicable state renders with text or shape in addition to color.
- No primitive owns, caches, reads, or reaches for Patch values, focus, navigation, reducer state, or audio state.
- Every `match` on `ComponentState` is exhaustive with no wildcard arm.

## Context & Constraints

**Crest-spec resources this WP realizes**:

- `requirement.reusable_shell_primitives`
- `requirement.component_state_ownership_boundary`
- `requirement.explicit_state_rendering`
- `asset.ShellContextModules` — carries the generation prompts for this surface

**Supporting documents**: [plan.md](../plan.md) (IC-05) · [spec.md](../spec.md) (FR-004, FR-005, FR-009)

**You own the primitives module and nothing else.** WP01 declared `pub mod primitives;` in
`src/shell/visual/mod.rs`. You may promote `primitives.rs` to `primitives/mod.rs` — the declaration works
either way. If `mod.rs` needs a re-export line, that is an acceptable out-of-map edit; record a one-line
rationale in your Activity Log.

**The single most important constraint in this WP**: a primitive paints. It does not decide. If you find
yourself wanting to read focus state, look up a Patch value, or ask what context is active — stop. That
information is passed in. This is `requirement.component_state_ownership_boundary`, it is a declared
invariant, and WP06 will check it.

## Branch Strategy

- **Strategy**: feature-branch
- **Planning base branch**: `feat/crest-component-foundations`
- **Merge target branch**: `feat/crest-component-foundations`

> Populated by `spec-kitty agent mission finalize-tasks`. Do not edit manually.

Execution worktrees are allocated per computed lane from `lanes.json`.

## Subtasks & Detailed Guidance

### Subtask T013 – Text-role primitives

- **Purpose**: The foundation every other primitive composes. One call site per authored type style.

- **Steps**:
  1. Provide a text primitive taking the content, a type style from WP01's vocabulary, a color token, and a `ComponentState`.
  2. Resolve size, weight, line height, and family from the style — never from arguments. A caller
     choosing a size is the failure this mission exists to prevent.
  3. Apply the state's text treatment: `Disabled` uses `text/muted`; `Focused` and `Adjusting` keep their
     content color (the keyline carries the state, not the text color); `Error` may use `accent/warning`.
  4. Match `ComponentState` exhaustively. No wildcard arm.
  5. Provide convenience wrappers for the common styles if it reduces call-site noise — but they must go
     through the same path, not paint independently.

- **Files**: `src/shell/visual/primitives/text.rs` (or the equivalent in a single-file layout)

- **Parallel?**: No — T014–T018 build on it.

- **Notes**: Design this API against a real call site. `paint_context_line` in
  `src/adapter/eframe_graphical_window.rs:287` is the one WP04 will convert first — sketch that
  conversion before you finalize the signature.

### Subtask T014 – Hairline and keyline primitives

- **Purpose**: Separation is structural in this design — hairlines, never cards.

- **Steps**:
  1. Hairline: a 1 px separator at `border/default`, horizontal and vertical.
  2. Keyline: a structural separator at `border/strong` for stronger division.
  3. Both take an explicit rect or span. Neither computes its own layout.
  4. Widths come from WP01 geometry, not from arguments.

- **Files**: `src/shell/visual/primitives/rules.rs`

- **Parallel?**: `[P]` — independent of T015–T018 once T013 lands.

- **Notes**: The design file places the row hairline at the row's vertical middle, spanning from the end
  of the label to the start of the value. That is a composition decision for a later phase — this WP
  provides the primitive, not the placement policy.

### Subtask T015 – Focus-frame primitive with the authored halo

- **Purpose**: Focus is the primary interaction state in this product. It has to be exactly right.

- **Steps**:
  1. Paint a 3 px keyline in `accent/focus` for `Focused`, and 3 px in `accent/adjust` for `Adjusting`.
  2. Paint the halo: radius 8, spread 1, opacity 0.28, in the focus color. egui has no drop-shadow
     primitive matching this directly — approximate faithfully and record how in a doc comment. Do not
     silently omit it; the halo is authored geometry.
  3. Paint nothing for `Resting` — resting is the 1 px hairline, which is T014's job, not a suppressed frame.
  4. Provide the non-color focus indication the product requires: the design's `>` cursor prefix in the
     label column. `DESIGN.md:575` requires text or shape beyond color, and this is where focus satisfies it.
  5. Match `ComponentState` exhaustively.

- **Files**: `src/shell/visual/primitives/focus.rs`

- **Parallel?**: `[P]` — independent of T014, T016–T018.

- **Notes**: The `>` prefix occupies a 9 px column at x=10 with the label starting at x=19, measured from
  the design file. The primitive should reserve that column whether or not the cursor is drawn, so
  focusing a row does not shift its text.

### Subtask T016 – Value-display primitive

- **Purpose**: Values are right-aligned, monospace, and read as a column. That alignment is load-bearing in a controller UI.

- **Steps**:
  1. Take a formatted string and a `ComponentState`; render in `Code/Value` right-aligned to a given edge.
  2. Do **not** format numbers here. Formatting is a descriptor concern that belongs to the capability
     that owns the parameter — this primitive receives a finished string.
  3. Apply state: `Adjusting` uses `accent/adjust`; `Disabled` uses `text/muted`; otherwise `text/primary`.
  4. Match `ComponentState` exhaustively.

- **Files**: `src/shell/visual/primitives/value.rs`

- **Parallel?**: `[P]`

- **Notes**: The design file right-aligns values to the content edge — x=1442 within a 1452-wide content
  area at desktop. Take the edge as a parameter; do not hardcode it.

### Subtask T017 – Status-mark primitive covering Loading and Error

- **Purpose**: Status must be legible without color. This is the primitive most likely to be built color-only.

- **Steps**:
  1. Render a status mark for `Loading`, `Error`, `Muted`, `Soloed`, and `Selected`.
  2. Use the mapping WP02 declared in `state.rs` — do not re-derive it here, and do not invent a second
     visual language.
     - `Loading`: `accent/adjust` **plus** `PREPARING` / `ACTIVATING` text.
     - `Error`: `accent/warning` **plus** short typed text.
     - `Muted`: `accent/warning` plus explicit `M ON` (`DESIGN.md:468`).
     - `Soloed`: `accent/positive` plus explicit `S ON`.
     - `Selected`: `bg/selected` background plus a non-color mark — multi-select must be visibly distinct
       and never color-only (`DESIGN.md:512`).
  3. Every one of these carries text or shape. If any state renders as color alone, the WP is not done.
  4. Match `ComponentState` exhaustively.

- **Files**: `src/shell/visual/primitives/status.rs`

- **Parallel?**: `[P]`

- **Notes**: `Resting`, `Focused`, `Adjusting`, and `Disabled` have no status mark. Handle them explicitly
  as no-ops rather than falling into a wildcard.

### Subtask T018 – Action-hint primitive

- **Purpose**: The footer shows only actions valid at the focused target. Hints are how the player learns the grammar.

- **Steps**:
  1. Render a hint in `Instruction/Hint` — the design's compact form, e.g. `D-PAD NAV · A CONFIRM · B BACK`.
  2. Support the four tones the design file's CLI Hint component set declares: neutral, focus, adjust, back.
  3. Take the hint list as immutable input. The primitive never decides which actions are valid — that is
     reducer-owned and arrives through the view model.
  4. Handle separator rendering (` · `) so callers do not each reinvent it.

- **Files**: `src/shell/visual/primitives/hint.rs`

- **Parallel?**: `[P]`

- **Notes**: The design file's CLI Hint set has 4 variants by tone. The existing footer at
  `src/adapter/eframe_graphical_window.rs:572` is the call site WP04 converts.

### Subtask T019 – Enforce the component ownership boundary

- **Purpose**: Make the boundary structural instead of a convention that erodes.

- **Steps**:
  1. Review every primitive. None may import `AppState`, the reducer, focus types, Patch types, or audio types.
  2. Add a test asserting the primitives module's imports stay within the visual vocabulary, egui, and std.
     A crude but effective form: read the source files and assert no forbidden path appears. Crude and
     enforced beats elegant and absent.
  3. Add a doc comment at the module root stating the boundary and why: components paint, views compose,
     the reducer decides.
  4. Confirm every `match` on `ComponentState` across the module is exhaustive with no wildcard arm.

- **Files**: `src/shell/visual/primitives/mod.rs` (`#[cfg(test)]`)

- **Parallel?**: No — depends on T013–T018.

- **Notes**: WP06 extends this into the full literal-absence guard. Here, prove the narrower import claim
  so the boundary holds from the moment the primitives exist.

## Test Strategy

```bash
cargo test --lib shell::visual::primitives
make lint && make fmt-check
```

Tests are required for T019 — FR-009 is a measured claim.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| A primitive reaches for application state under convenience pressure | T019 import check, run from the moment primitives exist |
| Status states rendered color-only | T017 pairs every state with text; WP06 asserts non-color legibility |
| Focus halo silently omitted because egui lacks a direct equivalent | T015 requires a faithful approximation and a doc comment recording the method |
| Wildcard `match` arms defeat the closed set | T019 checks exhaustiveness across the module |
| API shape only discovered to be wrong during WP04 | T013 requires sketching a real adapter call site first |

## Review Guidance

- Can any primitive see application state? Check imports, not intentions.
- Is every state legible without color? Read each `match` arm and ask what a colorblind player sees.
- Are all `match` arms explicit — no `_ =>`?
- Does the focus primitive reserve the cursor column even when unfocused, so focusing does not shift text?
- Does the value primitive format anything? It should not.

## Activity Log

> **CRITICAL**: Activity log entries MUST be in chronological order (oldest first, newest last). Append at the END.

- 2026-08-02T02:26:18Z – system – Prompt created.
- 2026-08-02T05:10:13Z – claude (implementer-ivan) – Implemented T013–T019. Promoted
  `src/shell/visual/primitives.rs` to `primitives/mod.rs` with six submodules; no re-export line was
  needed in `src/shell/visual/mod.rs`, so that shared surface is untouched (WP01 review note N3 — WP02
  and WP03 use full `crate::shell::visual::…` paths).
- 2026-08-02T05:10:13Z – claude (implementer-ivan) – Design decisions and deliberate deviations, for the
  reviewer:
  1. **`rules.rs` keyline width.** The vocabulary declares exactly two widths, resting 1 px and emphasis
     3 px, and the emphasis width means focus/adjustment. A structural keyline at 3 px would read as a
     focused element, so hairline and keyline are both 1 px and differ by color — `border/default` versus
     `border/strong`, which is what `border/strong` is declared for.
  2. **`focus.rs` frames every emphasis state, not only focus.** WP02 declared `Loading` and `Error` at
     the 3 px emphasis width too. The frame paints wherever the declared width is emphasis and paints
     nothing where it is the resting hairline, so `Resting` is an absence (T014's job) rather than a
     suppressed frame, and WP02's Loading/Error emphasis is not silently dropped. A test ties `frames()`
     to `appearance().keyline_px` so the two cannot drift.
  3. **`focus.rs` halo.** egui has no drop-shadow matching radius/spread/opacity directly;
     `epaint::Shadow` is the faithful mapping (blur 8 = penumbra width, spread 1 = caster expansion,
     alpha 71 = 0.28 × 255, the `0x47` the design file carries). Recorded in a doc comment, not omitted.
     The test unmultiplies the stored premultiplied color back to the authored accent, tolerating the one
     level that 8-bit premultiplication costs and nothing wider.
  4. **`status.rs` paints `Disabled`.** The prompt's T017 note grouped `Disabled` with the unmarked
     states, but WP02 declared its signal as the word `Locked`, and FR-005 names disabled among the
     states that must read without color. Skipping it would have left disabled as muted color alone.
     Deliberate deviation, recorded in the module doc.
  5. **`status.rs` never renders a state as color alone.** `Loading` with no phase falls back to the
     first authored word and `Error` with no typed text falls back to `Failed`, so the mark is never
     absent. `Selected` pairs the `bg/selected` fill with a filled mark in `text/primary` — a mark in the
     fill color would be invisible on it.
  6. **`value.rs` formats nothing.** It takes a finished string and right-aligns it to a caller-supplied
     edge; the design file's x=1442 is a layout number and does not appear here.
  7. **API shaped against a real call site.** `text.rs`'s module doc carries the `paint_context_line`
     conversion (`src/adapter/eframe_graphical_window.rs:287`) that WP04 lands first.
- 2026-08-02T05:10:13Z – claude (implementer-ivan) – Verification. `cargo test --lib shell::visual`
  88/88; `make test` 555 lib + every integration target, 0 failures; `make lint` and `make fmt-check`
  clean. The T019 guards are mutation-verified — each was shown to fail when deliberately broken and
  then restored: (a) adding `use crate::control::AppState;` to `value.rs` fails the import check;
  (b) collapsing a `ComponentState` match into `_ =>` fails the wildcard check; (c) removing `rules.rs`
  fails the scan-coverage check; (d) painting `Selected`'s mark in its own fill fails both
  `selection_is_a_fill_plus_a_mark_and_never_the_fill_alone` and the module-wide non-color assertion.
  Note for the reviewer: `sf2/` is gitignored and absent from a fresh lane worktree, so every
  SoundFont-backed test fails there with `Asset(FileOpen)` until the directory is symlinked in from the
  repository root. That is an environment gap, not a WP03 regression; the 555/555 run above was made
  with the symlink in place, and the symlink is gitignored and not committed.
- 2026-08-02 – claude (implementer-ivan) – Review cycle 1, R1 fixed (commit `48261a4`, `mod.rs` only).
  Took the reviewer's preferred path 1 and tightened the guard rather than narrowing the claim, then made
  the doc match anyway. `no_primitive_names_a_path_outside_the_visual_vocabulary` scans the whole source
  text of every primitive for the seven non-visual crate roots in `src/lib.rs` — `adapter`, `control`,
  `kernel`, `mixer`, `real_time`, `synth`, `testing` — in both the `crate::` and `crest_synth::`
  spellings. Needles are assembled at runtime, exactly as the wildcard test already did, so `mod.rs` does
  not match itself; the first version failed on its own doc comment, which is why the comment now spells
  the example in two pieces. The `use`-line check stays alongside it and its doc now says why: a fixed
  forbidden-path list can never name a third-party crate the module has no business importing, so the two
  catch different things.
- 2026-08-02 – claude (implementer-ivan) – Scope judgement on R1, for the reviewer. Added a second guard
  the review did not ask for: `no_primitive_walks_out_of_the_vocabulary_by_a_relative_path` rejects a
  `super::` chain three or more steps deep. The inline-path hole has a relative twin — from a primitive,
  four `super::` steps reach `crate::control` with the word `crate` never appearing — and closing the
  absolute spelling while leaving the relative one open would have reproduced R1's actual defect, a guard
  documented as stronger than it is. Two steps is the deepest the module uses (out of a `#[cfg(test)]`
  module into a sibling) and the vocabulary is named absolutely everywhere else, so nothing legitimate is
  forbidden.
- 2026-08-02 – claude (implementer-ivan) – R1 mutation evidence, against the inline form this time rather
  than a `use` line. (a) The reviewer's exact snippet appended to `value.rs` — `peek_at_app_state`
  reaching `AppState` through `size_of::<crate::control::AppState>()` with no import — fails
  `no_primitive_names_a_path_outside_the_visual_vocabulary`; before this commit the same snippet left all
  47 tests green. (b) The relative form
  `size_of::<super::super::super::super::control::AppState>()` fails the new relative guard. Both
  restored, `git diff` on `value.rs` empty afterwards. (c) The `crest_synth::` spelling is **not**
  mutation-proven and is not claimed to be: the package name does not resolve inside its own lib
  (`error[E0433]: cannot find module or crate crest_synth`), so it is carried against a future move of
  these sources. The doc comment says so.
- 2026-08-02 – claude (implementer-ivan) – Module doc rewritten to state exactly what is proven, replacing
  the sentence the review quoted as false. It now enumerates the four checks and names the residual gap:
  these are textual scans, so they prove these paths are not *named* here — application state arriving
  through an argument type whose name is re-exported from the vocabulary is WP06's literal-absence proof,
  not this module's claim. The same overstatement was in the old commit message; nothing in this commit
  restates it.
- 2026-08-02 – claude (implementer-ivan) – N2 deliberately not taken, per the review's "not required this
  cycle". `#![deny(clippy::wildcard_enum_match_arm)]` is a restriction lint that fires on every
  non-exhaustive match on every enum in the module, including egui's, so it is a wider change than a
  rejection-fix cycle should carry. The named catch-all arm (`other => …`) remains undetected by the text
  scan; recording it here so it reaches WP06 rather than dying in a review file. N1, N3, and N4 need no
  action in this WP and carry forward as the review states.
- 2026-08-02 – claude (implementer-ivan) – Verification after the fix. `cargo test --lib
  shell::visual::primitives` 49/49 (47 before, plus the two new guards); `cargo test --lib shell::visual`
  90/90; `make test` 557 lib plus every integration target, 0 failures; `make lint` (clippy
  `--all-targets -D warnings`) and `make fmt-check` clean. The `sf2/` symlink note above still applies to
  a fresh lane worktree.
