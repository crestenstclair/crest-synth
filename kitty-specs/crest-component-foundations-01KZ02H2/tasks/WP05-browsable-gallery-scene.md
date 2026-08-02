---
work_package_id: WP05
title: Browsable gallery scene
dependencies:
- WP03
- WP04
requirement_refs:
- FR-007
- FR-008
- NFR-005
planning_base_branch: feat/crest-component-foundations
merge_target_branch: feat/crest-component-foundations
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-foundations. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-foundations unless the human explicitly redirects the landing branch.
subtasks:
- T026
- T027
- T028
- T029
- T030
- T031
- T032
phase: Phase 3 - Production surface
history:
- at: '2026-08-02T02:26:18Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent_profile: implementer-ivan
agent: claude
authoritative_surface: src/testing/component_gallery_scene.rs
create_intent:
- src/testing/component_gallery_scene.rs
execution_mode: code_change
owned_files:
- src/testing/component_gallery_scene.rs
- src/testing/mod.rs
- src/bin/crest_synth.rs
- Makefile
role: implementer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP05 – Browsable gallery scene

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

One command opens a real window showing off the components. Number keys change page. It is browsable by
hand — the operator asked for exactly this.

Complete when:

- `make demo-live-component-library` opens a window rendering the components through the shared vocabulary and primitives.
- Digit keys 1–8 select the eight declared pages; the active page identity is visible on screen.
- A digit with no page bound retains the current page and changes nothing.
- Every declared `ComponentState` appears with representative content, at both authored viewport sizes.
- A `ComponentGalleryObservation` is emitted **after** painting, reporting what was actually rendered.
- Closing the window exits normally and releases everything it owns.

## Context & Constraints

**Crest-spec resources this WP realizes**:

- `valueObject.Shell.ComponentGalleryPage`
- `valueObject.Shell.ComponentGalleryObservation`
- `requirement.browsable_component_gallery`
- `witness.component_gallery` — 15 measured predicates this scene must be able to satisfy
- `asset.TestingContextModules`, `asset.BuildMakefile`

**Supporting documents**: [plan.md](../plan.md) (IC-07) · [research.md](../research.md) (R-08) · [spec.md](../spec.md) (FR-007, FR-008, NFR-005, C-004, C-005)

### This scene is browsable, not autonomous — read this before writing code

Every other `demo-live-*` scene is deliberately **input-isolated**: while active, mapped semantic key
input is not dispatched into `AppState`, so an asynchronous edit cannot replace the exact generation a
checkpoint awaits (`DESIGN.md:634-644`).

This scene is the opposite **on purpose**. It exists to be driven by hand. It therefore:

- accepts input,
- makes **no** exact-generation claim,
- asserts nothing about audio,
- does not time out after 120 seconds,
- is **not** an alias for `demo-live`.

It does not weaken the witness contract because it never claims one. The danger runs both ways: giving
this scene the witness contract would break paging, and copying this scene's input handling back into a
witness would break the generation correlation those scenes depend on. Do neither.

### The hard invariant

Page selection is **scene-local**. It never becomes a `SemanticAction`, never reaches `AppState`, never
changes focus, Patch values, graph revision, or audio behavior. The witness asserts
`app_state_generation_delta = 0` across a full page walk. Bind the keys inside the scene.

## Branch Strategy

- **Strategy**: feature-branch
- **Planning base branch**: `feat/crest-component-foundations`
- **Merge target branch**: `feat/crest-component-foundations`

> Populated by `spec-kitty agent mission finalize-tasks`. Do not edit manually.

Execution worktrees are allocated per computed lane from `lanes.json`.

## Subtasks & Detailed Guidance

### Subtask T026 – Gallery scene skeleton and the closed page set

- **Purpose**: Establish the scene and its closed page vocabulary.

- **Steps**:
  1. Create `src/testing/component_gallery_scene.rs`; declare it in `src/testing/mod.rs` alongside the
     existing scenes. Read `src/testing/live_effects_and_buses_scene.rs` first for the house style — but
     note it is an autonomous witness and this scene is not; do not copy its checkpoint machinery.
  2. Declare the closed eight-value page set:

     | Digit | Page | Contents |
     |---|---|---|
     | 1 | `Colors` | All declared semantic colors with canonical names and hex |
     | 2 | `Type` | All 8 type styles at authored size/weight/line-height/tracking |
     | 3 | `SpacingAndGeometry` | 6 spacing steps, radii, keyline widths, 48 px target |
     | 4 | `InteractionStates` | Resting, Focused, Adjusting — keylines and the halo |
     | 5 | `TextAndHairlines` | Text roles and hairline/keyline separators |
     | 6 | `ValuesAndStatus` | Value displays, status marks, Loading and Error |
     | 7 | `ActionHints` | The four hint tones |
     | 8 | `ShellBands` | The five structural bands at both viewport sizes |

  3. Provide exhaustive iteration and a `const` count, so T031 and WP06 can assert coverage.
  4. Add a test asserting the page count is 8 and that every page has exactly one digit binding — no page
     without a key, no key without a page.

- **Files**: `src/testing/component_gallery_scene.rs`, `src/testing/mod.rs`

- **Parallel?**: No — everything else depends on it.

- **Notes**: Keep the set closed with no catch-all, for the same reason `ComponentState` is closed.

### Subtask T027 – Digit binding, page switch, and unbound-digit retention

- **Purpose**: Make it browsable, without leaking into application state.

- **Steps**:
  1. Handle `WindowInput` inside the scene. On `KeyDown` for `Digit1`–`Digit8`, switch the active page.
  2. On any other key, including an unbound digit, **retain** the current page and change nothing.
  3. Do **not** route these through `KeyboardInputTranslator`, and do **not** create a `SemanticAction`.
     The scene owns this binding entirely. WP04's T024 deliberately left `Digit3`–`Digit8` unbound in the
     translator so this stays scene-local.
  4. Render the active page identity on screen — name and digit, so the operator always knows where they are.
  5. Add a test: switching pages leaves the `AppState` generation unchanged. This is the
     `app_state_generation_delta = 0` predicate.

- **Files**: `src/testing/component_gallery_scene.rs`

- **Parallel?**: No.

- **Notes**: `Digit1` and `Digit2` also select PATCH/MIXER in the application. Inside this scene they
  select pages. That is intentional and safe precisely because the binding is scene-local.

### Subtask T028 – Render pages 1–4

- **Purpose**: The vocabulary pages — what a reviewer checks the authored values against.

- **Steps**:
  1. **Colors**: every declared color as a swatch with its canonical name and hex. Group by role
     (backgrounds, borders, text, accents), matching how the design file's Foundations page presents them.
  2. **Type**: each of the 8 styles rendered in its own style, labeled with name and metrics —
     e.g. `Display/Screen · 32/40 · SemiBold · 0.4`. Use representative content, not lorem ipsum; the
     design file uses strings like `PATCH 00 · LEAD PAD`.
  3. **SpacingAndGeometry**: each spacing step as a measured bar with its label; radii; keyline widths at
     1 px and 3 px; a 48 px target with its bound drawn.
  4. **InteractionStates**: the same specimen row in Resting, Focused, and Adjusting, so keylines and the
     halo can be compared directly.
  5. Render everything through WP01's vocabulary and WP03's primitives. **No literals in this file** —
     WP06's guard covers this file too.

- **Files**: `src/testing/component_gallery_scene.rs`

- **Parallel?**: `[P]` — independent of T029.

- **Notes**: `figma-functional-interpretation/assets/` has exported reference images. The Foundations
  layout there is a good model for pages 1–3.

### Subtask T029 – Render pages 5–8

- **Purpose**: The primitive pages — where state legibility is judged.

- **Steps**:
  1. **TextAndHairlines**: each text role at each applicable state; horizontal and vertical hairlines and keylines.
  2. **ValuesAndStatus**: right-aligned value displays across states; every status mark — Loading with
     `PREPARING`/`ACTIVATING`, Error with typed text, Muted `M ON`, Soloed `S ON`, Selected with its
     non-color mark.
  3. **ActionHints**: all four hint tones with representative content.
  4. **ShellBands**: the five structural bands, rendered from both density policies so the desktop and
     Steam Deck compositions can be compared side by side. This is the page the operator reviews for
     T008's authored Steam Deck policy — make it clear and label which is which.
  5. Every specimen must be **labeled with its state name**. A wall of unlabeled variants is not judgable,
     and NFR-005 requires each state legible without relying on color.

- **Files**: `src/testing/component_gallery_scene.rs`

- **Parallel?**: `[P]` — independent of T028.

- **Notes**: Page 8 is the highest-value page in the mission — it is where the authored-not-measured
  Steam Deck policy gets its only human check.

### Subtask T030 – Show both authored viewports

- **Purpose**: NFR-003 and the witness both require both sizes to be actually rendered, not described.

- **Steps**:
  1. Render each page's specimens at both the Desktop and Steam Deck density policies.
  2. Simplest approach that satisfies the witness: render both compositions within one window, each
     labeled, so a single screenshot shows both. Alternatively let a key toggle which policy is active —
     if you do, the page-walk observation must still record both as painted.
  3. Whichever you choose, `desktop_viewport_painted` and `steam_deck_viewport_painted` must both be true
     from what was actually rendered, not from the scene's intent.
  4. Ensure no text clips or overlaps at either size. `clipped_or_overlapping_text` must be 0.

- **Files**: `src/testing/component_gallery_scene.rs`

- **Parallel?**: No — depends on T028, T029.

- **Notes**: Side-by-side in one window is easier to review and easier to prove than a toggle. Prefer it
  unless the density makes it unreadable.

### Subtask T031 – Emit `ComponentGalleryObservation` after painting

- **Purpose**: Make coverage measurable rather than asserted. A plan is not evidence.

- **Steps**:
  1. Emit the observation **after** painting, reporting what was actually rendered. Model it on
     `ShellFrameObservation` (`src/shell/shell_frame_observation.rs`), which already does this correctly.
  2. Populate the fields `witness.component_gallery` predicates on:
     `pages_declared`, `pages_painted`, `pages_reachable_by_digit`, `unbound_digit_retained_page`,
     `states_declared`, `states_painted`, `states_distinguishable_without_color`,
     `desktop_viewport_painted`, `steam_deck_viewport_painted`, `bands_retained_both_viewports`,
     `clipped_or_overlapping_text`, `token_source_exact`, `typeface_resolved`,
     `app_state_generation_delta`, `window_closed`.
  3. Print it to stdout with the marker `CREST_COMPONENT_GALLERY_OBSERVATION ` followed by JSON.
  4. **Critical**: a constructed specimen list or a pre-render plan must not be able to satisfy this. The
     counts must come from what the paint pass actually emitted. If you can make the observation pass
     without painting, it is wrong — that is the crest-spec invariant on this value object.
  5. The observation is scene data. It never becomes `AppState`.

- **Files**: `src/testing/component_gallery_scene.rs`

- **Parallel?**: No — depends on T028–T030.

- **Notes**: Increment counters inside the paint functions, not before them. That is the difference
  between measuring and asserting.

### Subtask T032 – Add the CLI flag and the Makefile target

- **Purpose**: One command the operator runs.

- **Steps**:
  1. Add `--demo-live-component-library` to `src/bin/crest_synth.rs`. The flag parsing is at `:281-310`.
  2. **Do not** add it to the `--demo-live` alias group at `:306-310`. This scene is not a witness and
     `demo-live` must keep pointing at the newest cumulative autonomous scene.
  3. This scene has no 10-second-milestone or 120-second total timeout. Those exist to stop an autonomous
     witness hanging; a browsable scene waits for the operator by design.
  4. Add the Makefile target with a `##` description, matching the existing style:
     ```make
     demo-live-component-library: ## Browse the component gallery — digit keys change page
     	cargo run --release --bin crest-synth -- --demo-live-component-library
     ```
  5. Confirm `make help` lists it, and that its description conveys "browsable" rather than reading like
     another autonomous witness.

- **Files**: `src/bin/crest_synth.rs`, `Makefile`

- **Parallel?**: No.

- **Notes**: Release profile matches the other live targets — a visual demo should measure product
  behavior, not debug-build overhead.

## Test Strategy

```bash
cargo test --lib testing::component_gallery_scene
make demo-live-component-library    # press 1-8, then an unbound digit, then close the window
make test > /tmp/wp05-test.log 2>&1; echo "exit=$?"
make demo-live                      # confirm the autonomous witnesses still behave
```

Never pipe test or demo output through `head`/`tail` — the pipe reports the pager's exit code.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Scene copies witness checkpoint machinery and stops being browsable | Explicit constraint section; do not copy from `live_effects_and_buses_scene.rs` wholesale |
| Page selection leaks into `AppState` | Scene-local binding; T027 test asserts zero generation delta |
| Observation satisfiable without painting | T031 requires counters incremented inside paint |
| Gallery becomes a `demo-live` alias | T032 explicitly excludes it from the alias group |
| Specimens unlabeled, so states are not judgable | T029 requires every specimen labeled with its state name |

## Review Guidance

- Press each digit. Does the page change and the identity show on screen?
- Press `9` or `0`. Does the current page stay put?
- Is the generation delta genuinely 0 across a full page walk?
- Could the observation pass without painting? Read where the counters increment.
- Does `make demo-live` still point at the autonomous scene?
- Page 8: do both viewport compositions render, and is the Steam Deck one actually reviewable?

## Activity Log

> **CRITICAL**: Activity log entries MUST be in chronological order (oldest first, newest last). Append at the END.

- 2026-08-02T02:26:18Z – system – Prompt created.
