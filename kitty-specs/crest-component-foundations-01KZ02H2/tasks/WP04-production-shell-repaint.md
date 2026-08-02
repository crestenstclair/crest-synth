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

- **Purpose**: Layout constants become policy resolution, so the Steam Deck size is declared rather than improvised.

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
