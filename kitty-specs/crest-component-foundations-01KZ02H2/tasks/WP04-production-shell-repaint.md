---
work_package_id: WP04
title: Production shell repaint and key vocabulary
dependencies:
- WP01
- WP02
- WP03
requirement_refs:
- FR-006
- NFR-004
planning_base_branch: feat/crest-component-foundations
merge_target_branch: feat/crest-component-foundations
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-foundations. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-foundations unless the human explicitly redirects the landing branch.
subtasks:
- T020
- T021
- T022
- T023
- T024
- T025
phase: Phase 3 - Production surface
history:
- at: '2026-08-02T02:26:18Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent_profile: implementer-ivan
agent: claude
authoritative_surface: src/adapter/eframe_graphical_window.rs
create_intent: []
execution_mode: code_change
owned_files:
- src/adapter/eframe_graphical_window.rs
- src/shell/window_input.rs
- src/shell/keyboard_input_translator.rs
role: implementer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP04 – Production shell repaint and key vocabulary

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

**This work package delivers the mission's only user-visible P1 outcome.** After it, `make run` shows the
authored design instead of seven hand-entered constants and a default font.

Complete when:

- The seven constants at `src/adapter/eframe_graphical_window.rs:28-34` no longer exist, and every painted value resolves through the vocabulary.
- The typeface is installed at startup, and its failure is visible rather than absorbed.
- Band, split, and side-width constants are replaced by density-policy resolution.
- `WindowKey` carries `Digit3`–`Digit8`, with the descriptor count moving 21 → 33 and the exhaustiveness assertion still honest.
- An unbound digit produces no semantic action — not a substitute one.
- `make run` visibly changed, and every existing shell and adapter test still passes.

## Context & Constraints

**Crest-spec resources this WP realizes**:

- `asset.AdapterModules` — thin adapters implementing declared ports
- `valueObject.Shell.WindowInput` — the corrected 33-value descriptor
- `requirement.semantic_visual_vocabulary`, `requirement.viewport_density_policy`

**Supporting documents**: [plan.md](../plan.md) (IC-06) · [spec.md](../spec.md) (FR-006, NFR-002, NFR-004)

**The adapter stays passive.** It renders an immutable `GraphicalShellProjection` and emits normalized
input. This WP changes *which values it paints with*, not what it decides. Nothing in the reducer,
projection, transport, or render path changes.

**Nothing about audio changes.** NFR-004 requires the callback contract to hold and the control-path
fixture to stay within its 50 ms ceiling. If you find yourself touching anything under `src/real_time/`,
stop — you are outside this WP.

## Branch Strategy

- **Strategy**: feature-branch
- **Planning base branch**: `feat/crest-component-foundations`
- **Merge target branch**: `feat/crest-component-foundations`

> Populated by `spec-kitty agent mission finalize-tasks`. Do not edit manually.

Execution worktrees are allocated per computed lane from `lanes.json`.

## Subtasks & Detailed Guidance

### Subtask T020 – Delete the seven adapter constants; paint through the vocabulary

- **Purpose**: The reason this mission exists. These seven values are wrong and they are in the wrong place.

- **Steps**:
  1. Delete these from `src/adapter/eframe_graphical_window.rs:28-34`:
     ```rust
     const BACKGROUND:    Color32 = Color32::from_rgb(16, 18, 22);   // → bg/canvas   #0c1015
     const ELEVATED:      Color32 = Color32::from_rgb(24, 27, 32);   // → bg/elevated #1d2733
     const PANEL:         Color32 = Color32::from_rgb(29, 33, 39);   // → bg/panel    #17202a
     const TEXT:          Color32 = Color32::from_rgb(230, 234, 239);// → text/primary   #f2f6f8
     const MUTED_TEXT:    Color32 = Color32::from_rgb(150, 158, 169);// → text/muted     #6f8095
     const ACCENT:        Color32 = Color32::from_rgb(110, 205, 174);// → accent/focus   #65e5ff
     const ADJUST_ACCENT: Color32 = Color32::from_rgb(232, 174, 76); // → accent/adjust  #ffb454
     ```
     Note `ACCENT` is a green being used where the authored focus color is cyan. That is the single most
     visible change in this mission.
  2. Replace every use with the corresponding token. Work through the paint functions in order:
     `paint_context_line` (:287), `paint_identity_header` (:333), `paint_main_workspace` (:352),
     `paint_patch_workspace` (:375), `paint_mixer_workspace` (:395), `paint_diagnostic` (:475),
     `paint_side_region` (:489), `paint_surface_summary` (:535), `paint_footer` (:572),
     `paint_semantic_control` (:605).
  3. Replace `.monospace()` calls (:412, :483, :544, :565, :636, :649, :659) with the appropriate type
     style. Choose by role, not by size: context line and footer use `Instruction/Hint`, control labels
     use `Label/Control`, values use `Code/Value`, headers use `Heading/Panel` or `Display/Screen`.
  4. Use the primitives from WP03 where one fits. Where the existing code does something no primitive
     covers, use the tokens directly rather than inventing a primitive here — note the gap in your
     Activity Log for the follow-on mission.
  5. When you are done, no literal color and no literal type size remains in this file. WP06 enforces this;
     do not leave it work to clean up.

- **Files**: `src/adapter/eframe_graphical_window.rs`

- **Parallel?**: No — T021 and T022 touch the same file.

- **Notes**: `padded_label` (:690) and `trailing_label` (:699) are local helpers that take a `Color32`.
  They likely become thin wrappers over WP03's text primitive, or disappear.

### Subtask T021 – Install the typeface at startup and surface its failure

- **Purpose**: Type styles are nominal until the faces are registered. And a missing face must be loud.

- **Steps**:
  1. Call WP01's typeface registration during eframe setup, before the first painted frame — in the
     `EframeGraphicalApplication` construction path (`:111`) or the `AppWindow::run` entry (`:63`),
     whichever runs earlier.
  2. Register once. Never per frame in `update` (:251).
  3. On a typed registration error, make it **visible**: surface it as an application error the shell
     renders, consistent with how the product handles unavailable capabilities. Do not log-and-continue,
     do not `unwrap()`, and do not let egui's default font stack absorb it — the whole point is that a
     substituted face looks plausible while being wrong.
  4. Verify visually that text actually changed. If it still looks like the old default font, registration
     did not take — egui silently falls back on a family-name mismatch.

- **Files**: `src/adapter/eframe_graphical_window.rs`

- **Parallel?**: No.

- **Notes**: The most common failure is registering the font data but not adding the family to
  `FontDefinitions::families` for the intended `FontFamily`. Both steps are required.

### Subtask T022 – Replace band, split, and side-width constants with the policy

- **Purpose**: Layout constants become policy resolution, so the compact viewport size is declared rather than improvised.

- **Steps**:
  1. Delete `AUTHORED_WIDTH`, `AUTHORED_HEIGHT`, `MINIMUM_WIDTH`, `MINIMUM_HEIGHT`,
     `CONTEXT_LINE_HEIGHT`, `IDENTITY_HEADER_HEIGHT`, `FOOTER_HEIGHT`, `AUTHORED_SIDE_WIDTH`,
     `MINIMUM_SIDE_WIDTH` (`:17-25`). Resolve each from the density policy instead.
  2. Delete `desired_side_width` (`:707`) — its ad-hoc proportional rule is exactly what WP02's policy replaces.
  3. In `paint_shell` (`:178`), resolve the policy once per frame from the viewport (`:200`) and use it
     for every band and split. Do not branch on the raw viewport size anywhere.
  4. Keep `FRAME_INTERVAL` (`:26`) — the 16 ms idle cadence is a separate declared decision and is not
     yours to change.
  5. Verify the desktop rendering is **pixel-identical** to before. WP02's T012 asserts the policy
     reproduces these constants; this subtask makes the adapter actually use it. Any visible shift at
     1920×1080 means something is wrong.

- **Files**: `src/adapter/eframe_graphical_window.rs`

- **Parallel?**: No.

- **Notes**: `graphical_window_uses_the_two_reference_side_widths` (`:791`) tests the old rule. Update it
  to assert the policy resolution instead of deleting it — the behavior it guards is still real.

### Subtask T023 – Extend `WindowKey` with `Digit3`–`Digit8`; 21 → 33 descriptors

- **Purpose**: Normalize the digits the gallery binds, and keep the exhaustiveness assertion honest.

- **Steps**:
  1. Add `Digit3` through `Digit8` to `WindowKey` (`src/shell/window_input.rs:4`).
  2. Extend `WINDOW_INPUT_SURFACE_DESCRIPTOR` (`:42`) with key-down and key-up for each new key. The
     array length moves from 21 to 33: 16 keys × 2 kinds + `FocusLost`.
  3. Update the declared count at `:67` and the assertion at `:122`.
  4. **Keep the assertion honest.** It exists to catch exactly this kind of change. Update it to the real
     new count; do not relax it to a range, a minimum, or a `>=`. The crest-spec invariant requires the
     declared count and the constructed descriptor to be asserted equal.
  5. Map the new keys in the eframe normalization path — `normalize_key` (`:742`).
  6. Add the new keys to `graphical_window_normalizes_the_complete_key_vocabulary` (`:797`).

- **Files**: `src/shell/window_input.rs`, `src/adapter/eframe_graphical_window.rs`

- **Parallel?**: `[P]` — different concern from T020–T022, though it shares the adapter file.

- **Notes**: The crest-spec previously asserted 17 here while the code had 21 — the drift `SelectPatch`
  introduced, corrected during crest-spec authoring. Do not reintroduce a gap between the declared count
  and the real one.

### Subtask T024 – Make unbound digits produce no semantic action

- **Purpose**: A normalized key with no binding must do nothing — not something approximate.

- **Steps**:
  1. In `src/shell/keyboard_input_translator.rs`, confirm `Digit3`–`Digit8` produce **no** `SemanticAction`.
  2. `Digit1` and `Digit2` keep their existing PATCH/MIXER context bindings — unchanged.
  3. Add a test asserting each new digit translates to `None`.
  4. Do **not** add a gallery binding here. Gallery paging is scene-local (WP05) and never becomes a
     `SemanticAction`. That is a declared invariant, and putting it here would violate it.

- **Files**: `src/shell/keyboard_input_translator.rs`

- **Parallel?**: No — depends on T023.

- **Notes**: This is the seam where the invariant is easiest to break. The scene binds these keys itself;
  the translator stays ignorant of them.

### Subtask T025 – Confirm `make run` changed and existing tests still pass

- **Purpose**: Verify the P1 outcome by looking at it, and prove nothing regressed.

- **Steps**:
  1. Run `make run`. Confirm by eye: dark canvas at `#0c1015`, **cyan** focus (not green), amber
     adjustment, Azeret Mono throughout.
  2. Run the full suite and the guards:
     ```bash
     make test > /tmp/wp04-test.log 2>&1; echo "exit=$?"
     make lint && make fmt-check
     ```
     **Never pipe these through `head` or `tail`** — the pipe reports the pager's exit code, so a "green"
     recorded that way is a lie. Redirect to a file.
  3. Run `make demo-live` and confirm the audio behavior is unchanged. This WP must not alter what you hear.
  4. If any existing adapter test asserts an old constant, update it to assert the authored value — do not
     delete the test.
  5. Record in the Activity Log what visibly changed, so the reviewer knows what to look for.

- **Files**: none — verification only

- **Parallel?**: No — depends on everything above.

- **Notes**: The most likely regression is layout drift from T022. Compare against a screenshot taken
  before your changes.

## Test Strategy

```bash
make test > /tmp/wp04-test.log 2>&1; echo "exit=$?"
make lint && make fmt-check
make run          # visual confirmation — the P1 outcome
make demo-live    # audio unchanged
```

Existing tests must keep passing. New tests are required for T023 and T024.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Layout drifts while swapping constants for policy | T022 requires pixel-identical desktop rendering; WP02 T012 asserts the policy equals the old constants |
| Exhaustiveness assertion relaxed to make it pass | T023 explicitly forbids a range or `>=`; reviewer checks for an exact equality |
| Typeface registers but egui silently falls back | T021 requires visual confirmation, not just a successful call |
| Gallery paging leaks into the translator as a `SemanticAction` | T024 explicitly forbids it; it is a declared invariant |
| A literal survives somewhere in the adapter | T020 requires none remain; WP06 enforces it |

## Review Guidance

- Do any literal colors or type sizes remain in `eframe_graphical_window.rs`? Grep, do not skim.
- Is focus **cyan** now? That is the headline visible change.
- Is the descriptor count assertion an exact equality at 33?
- Do `Digit3`–`Digit8` translate to no action?
- Did desktop rendering shift? It must not.
- Was `make test` recorded by redirect rather than through a pipe?

## Activity Log

> **CRITICAL**: Activity log entries MUST be in chronological order (oldest first, newest last). Append at the END.

- 2026-08-02T02:26:18Z – system – Prompt created.
- 2026-08-02T04:10:00Z – claude (implementer-ivan) – T020 done. The seven colour constants are
  deleted. Every painted colour now resolves through `SemanticColor`, every text run through a
  `TypeStyle`, and every gap through a `SpacingStep`. No `RichText`, `.monospace()`, `.small()`,
  `.strong()`, or `Color32::from_*` survives in the adapter. **What visibly changed:** focus is
  cyan `#65e5ff` where it was green `rgb(110,205,174)`; the canvas is `#0c1015` where it was
  `rgb(16,18,22)`; adjustment is `#ffb454`; all text is Azeret Mono at the eight authored sizes
  instead of egui's default stack. Re-roled three uses of the old green that were not focus:
  workspace and side titles are `text/primary`, `NO ERRORS` is `accent/positive`, and semantic
  errors are `accent/warning` rather than the adjust amber — `color/accent/focus` is declared to
  mean focus and nothing else.
- 2026-08-02T04:10:00Z – claude (implementer-ivan) – T020 primitive reuse. Control rows now paint
  through `focus::halo`, `focus::cursor`, `text::paint_text`, and `value::value_color`, with the
  row's one `ComponentState` resolved in `control_state`. The design's `>` cursor is drawn for the
  first time, in the reserved column `focus::LABEL_START_X_PX` declares, so focusing a row no
  longer shifts its text. `ui.separator()` was replaced by `rules::hairline`. `padded_label` and
  `trailing_label` became thin wrappers over the text primitive.
- 2026-08-02T04:10:00Z – claude (implementer-ivan) – T021 done. `AuthoredTypeface::load()` runs in
  `AppWindow::run` *before* `run_native`, so a missing face is a typed `WindowError` naming the
  file and no window opens at all; registration itself happens once in the eframe creation
  closure, before the first painted frame, never in `update`. Registration is exposed as
  `install_authored_typeface(&Context)` because egui binds fonts to a `Context`, not to an app —
  every owner of a context this adapter paints into calls it. Verified the registration actually
  took rather than being silently absorbed: the new render test asserts every painted glyph run
  carries an `Azeret Mono …` family, so a family-name mismatch fails instead of falling back.
- 2026-08-02T04:10:00Z – claude (implementer-ivan) – T022 done. All nine band/split/side constants
  and `desired_side_width` are deleted; `paint_shell` resolves `ViewportDensityPolicy` once per
  frame from the viewport and reads every band and split from it, and `run` opens at the policy's
  authored desktop size with the Steam Deck size as the minimum. Desktop rendering is unchanged:
  `production_update_renders_both_contexts_at_both_reference_viewports` asserts the region rects
  at 1920×1080 through the policy and they are the same numbers as before.
- 2026-08-02T04:10:00Z – claude (implementer-ivan) – T022 correction found by the policy. The
  identity header's 39 px sub-row was tuned to the 72 px desktop band and overflowed the Steam
  Deck's 60 px band (39 + spacing + 30 = 72), pushing the header 12 px too tall at 1280×800. The
  fixed split is gone: the band now sizes from the authored type it carries — `Heading/Section`
  over `Body/Compact`, one `space/12` inset and one `space/4` between — which is 58 px and fits
  both bands. This was invisible before this WP because the adapter used the desktop constants at
  every size.
- 2026-08-02T04:10:00Z – claude (implementer-ivan) – T023 done. `WindowKey` gained `Digit3`–
  `Digit8`; the descriptor is 33 entries and `WINDOW_INPUT_SURFACE_DESCRIPTOR_LEN` declares 33.
  The assertion stayed an exact equality — `assert_eq!(descriptor.len(), 33)` plus
  `ALL_WINDOW_KEYS.len() * 2 + 1 == 33` — and was not relaxed to a range or a minimum. Added
  `ALL_WINDOW_KEYS` so the per-key coverage assertions read the declared vocabulary instead of a
  hand-maintained subset that had quietly lost `Q` and `E`. Two further declared counts of 21
  existed outside the owned files and were updated to the real 33:
  `src/testing/demo_scene.rs` and `tests/exhaustive_demo_scene.rs`. The exhaustiveness
  requirement also forced `window_input_identifier` in `src/testing/demo_scene.rs` and
  `src/testing/exhaustive_gui_demo.rs` to name the six new keys — the compile error is the
  assertion working.
- 2026-08-02T04:10:00Z – claude (implementer-ivan) – T024 done. `Digit3`–`Digit8` are listed in the
  translator's explicit no-action arm, so they produce no `SemanticAction` on key-down or key-up,
  and stay unbound while K is held. No gallery binding was added: paging is scene-local and never
  becomes a semantic action. `Digit1`/`Digit2` keep their PATCH/MIXER bindings.
- 2026-08-02T04:10:00Z – claude (implementer-ivan) – T025. `make test` exit=0 (563 lib + all
  integration tests, recorded by redirect to `/tmp/wp04-test.log`, never through a pipe);
  `make lint` and `make fmt-check` exit=0. `make demo-live` exit=0 with 105/105 audio predicates
  passed and zero non-finite samples; nothing under `src/real_time/` was touched. The P1 outcome
  is also proven by measurement: the new
  `production_render_paints_the_authored_palette_and_typeface` drives the production adapter
  through a real `egui::Context` and asserts on the emitted shapes that the authored cyan and
  canvas are painted, that the old green and all six other retired constants appear nowhere in
  the frame, and that every glyph run carries an authored family at an authored size.
- 2026-08-02T04:40:00Z – claude (implementer-ivan) – T025 visual confirmation, done by eye against
  a before/after pair as T022 asks. `make run` captured at `/tmp/wp04-after.png`; the pre-WP04
  commit `a3d8b00^` was built in a scratch worktree and captured at `/tmp/wp04-before.png`. What
  changed, confirmed by looking: focus keyline green → cyan; every label green → `text/primary`;
  the whole interface proportional-default → Azeret Mono; the identity header gained its authored
  `Heading/Section` weight; the `>` focus cursor appears for the first time. The four band
  boundaries sit at the same pixels in both images, so the constants-to-policy swap moved nothing
  at 1920×1080.
- 2026-08-02T04:40:00Z – claude (implementer-ivan) – Pre-existing defect found while comparing,
  **not** introduced here and deliberately not fixed in this WP. In the mixer track strip each
  control's label and its right-aligned value paint on top of each other — `T00 Level` over
  `0.000`, `T00 Mute` over `OFF`. It is present identically in `/tmp/wp04-before.png`. The cause
  is structural rather than visual: `paint_mixer_workspace` lays the track cards out with
  `horizontal_top`, so each control inside a card inherits a horizontal layout and the
  `right_to_left` value sub-layout is handed no remaining width to sit in. Fixing it means
  changing how the mixer workspace composes, which is past "which values it paints with" and
  belongs with the mixer surface's own work.
- 2026-08-02T04:10:00Z – claude (implementer-ivan) – Vocabulary gaps recorded for the follow-on
  mission, per T020's instruction to note rather than invent. (1) No policy accessor declares
  sub-band extents, so `WORKSPACE_TITLE_ROW_PX` (42), `MIXER_TRACK_MIN_WIDTH_PX` (176), and the
  three horizontal fractions remain adapter-local named constants; they are layout extents, not
  colours or type sizes. (2) `hint::ActionHint`/`hint_line` model the footer's chord-plus-action
  hints, but `valid_actions()` renders as clickable buttons carrying one combined label, so the
  footer still composes its text directly; switching it belongs with the footer's own redesign.
  (3) No primitive covers a `ProgressBar`-style meter, a collapsing header, or a button, so those
  are styled from tokens at the call site.
- 2026-08-02T04:10:00Z – claude (implementer-ivan) – Environment note: the lane worktree was
  missing the gitignored `sf2/` SoundFont directory, which failed 145 unrelated tests before any
  of this WP's work was measured. Symlinked it to the primary checkout, matching lanes a/b/c.
