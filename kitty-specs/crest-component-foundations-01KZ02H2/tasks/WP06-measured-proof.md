---
work_package_id: WP06
title: Measured proof
dependencies:
- WP05
requirement_refs:
- NFR-001
- NFR-002
- NFR-003
planning_base_branch: feat/crest-component-foundations
merge_target_branch: feat/crest-component-foundations
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-foundations. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-foundations unless the human explicitly redirects the landing branch.
subtasks:
- T033
- T034
- T035
- T036
- T037
- T038
phase: Phase 4 - Proof
history:
- at: '2026-08-02T02:26:18Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent_profile: implementer-ivan
agent: claude
authoritative_surface: tests/component_vocabulary.rs
create_intent:
- tests/component_vocabulary.rs
execution_mode: code_change
owned_files:
- tests/component_vocabulary.rs
role: implementer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP06 – Measured proof

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

Prove every claim this mission makes, by measurement through the production render path — and prove each
guard is capable of failing.

Complete when `cargo test --test component_vocabulary` prints
`CREST_ACCEPTANCE component_vocabulary passed` only after all of these hold:

- Every declared color, type style, spacing step, and geometry value equals its authored counterpart, compared as **values**.
- No literal color, type size, or spacing constant exists outside the vocabulary module — **and the guard is shown failing** on a deliberately reintroduced literal.
- Both density policies retain every band, the side region, and the 48 px minimum at both authored viewports.
- `ComponentState` is exhaustively matched, every state is legible without color, and every gallery page has exactly one digit binding.
- An unavailable typeface produces the typed visible failure rather than a substituted face.

## Context & Constraints

**Crest-spec resources this WP realizes**:

- `asset.ComponentVocabularyAcceptanceTests` — carries the generation prompts for this target
- `validation.component_vocabulary` — a project check in `completion.projectChecks`
- `evidence.component_vocabulary_contract`

**Supporting documents**: [plan.md](../plan.md) (IC-08) · [research.md](../research.md) (R-07) · [spec.md](../spec.md) (NFR-001, NFR-002, NFR-003, NFR-005, C-006)

### The failure mode this WP exists to prevent

**A test that asserts the token names exist, while never comparing a rendered value, passes forever and
proves nothing.** That is the single most likely way this mission ends up looking complete while being
hollow. Every check here compares values through the production render path.

`C-006` is explicit: construction-only tests, success-token logs, and pre-render layout plans are not
evidence. The repository already has precedent for this discipline — `no_name_enumerated_identity` is a
declared project validation that proves an absence across whole contexts. Follow that pattern.

**A guard that has never failed is indistinguishable from no guard.** T035 is not optional.

## Branch Strategy

- **Strategy**: feature-branch
- **Planning base branch**: `feat/crest-component-foundations`
- **Merge target branch**: `feat/crest-component-foundations`

> Populated by `spec-kitty agent mission finalize-tasks`. Do not edit manually.

Execution worktrees are allocated per computed lane from `lanes.json`.

## Subtasks & Detailed Guidance

### Subtask T033 – Acceptance target skeleton and marker

- **Purpose**: Establish the target and its marker contract.

- **Steps**:
  1. Create `tests/component_vocabulary.rs`. Read `tests/engine_selection_workflow.rs` first — it is the
     house pattern for this kind of target.
  2. Structure as ordinary assertion-bearing `#[test]` functions. The marker is a summary, not a substitute
     for assertions.
  3. Emit `CREST_ACCEPTANCE component_vocabulary passed` **only** after every check has passed. Never
     unconditionally, and never before the assertions run.
  4. `validation.component_vocabulary` asserts exit code 0 and that this exact string appears in stdout.
     Match it byte-for-byte.

- **Files**: `tests/component_vocabulary.rs`

- **Parallel?**: No — everything else depends on it.

- **Notes**: The declared command is `cargo test --test component_vocabulary -- --nocapture`, timeout 180s.

### Subtask T034 – Prove authored-value fidelity through the render path

- **Purpose**: NFR-001. The values must be right **where they are painted**, not merely where they are declared.

- **Steps**:
  1. Assert every declared color resolves to its exact authored RGB. Write expected values as literals in
     the test, independent of the vocabulary — a test comparing the vocabulary to itself proves nothing.
  2. Assert every type style resolves to the authored family, weight, size, line height, and tracking.
  3. Assert every spacing step and geometry value.
  4. **Go through the production render path.** WP01's T006 already asserts the declarations; this
     subtask's job is to prove what the shell actually paints with. Drive a real projection through the
     production paint path and assert the resolved values — do not re-assert the constants.
  5. Assert counts as well as values, so a silently dropped token fails rather than passing by absence.
  6. Spot-check the values that changed most: `accent/focus` must be `#65e5ff` (cyan), not the old
     `rgb(110, 205, 174)` green; `bg/canvas` must be `#0c1015`, not `rgb(16, 18, 22)`.

- **Files**: `tests/component_vocabulary.rs`

- **Parallel?**: No.

- **Notes**: If reaching the render path proves impractical within the harness, say so explicitly in the
  Activity Log and describe what you asserted instead. Do not quietly downgrade to a declaration-only
  check and call it proven.

### Subtask T035 – Literal-absence guard, and prove the guard fails

- **Purpose**: NFR-002, and the highest-risk item in the mission.

- **Steps**:
  1. Implement a guard proving no literal color, type size, or spacing constant exists outside
     `src/shell/visual/token.rs`.
  2. Scan the source of every adapter, view, scene, and test file — `src/adapter/**`, `src/shell/**`
     (excluding `visual/token.rs`), `src/testing/**` — for:
     - `Color32::from_rgb`, `Color32::from_rgba`, `Color32::from_hex` and equivalents
     - numeric literals in font-size position
     - the hex strings of the authored palette appearing anywhere outside the vocabulary
  3. Allow the vocabulary module itself. Allow nothing else. If a file legitimately needs an exception,
     it belongs in the vocabulary — that is the whole point.
  4. **Prove the guard fails.** Add a test that feeds the guard a source sample containing a literal and
     asserts it reports a violation. Without this, the guard could match nothing and pass forever.
  5. Have the guard report the offending file and line, so a future failure is actionable rather than
     just red.

- **Files**: `tests/component_vocabulary.rs`

- **Parallel?**: No.

- **Notes**: R-07 in [research.md](../research.md) records why this is a scripted guard rather than a
  Clippy lint, and why review was rejected as the mechanism. `egui::Color32::TRANSPARENT` and named
  constants are not literals — match construction from raw numbers, not every mention of `Color32`.

### Subtask T036 – Prove viewport integrity at both authored sizes

- **Purpose**: NFR-003.

- **Steps**:
  1. Render the production shell projection at 1920×1080 and 1280×800 through the production paint path.
  2. Assert at both sizes: all five structural bands present with nonzero height; the persistent side
     region visible; bands plus workspace equal viewport height exactly; main plus side equal viewport
     width exactly; no interactive target below 48 px.
  3. Assert no text run clips its container or overlaps another. If exact overlap detection is impractical,
     assert the bounding-box arithmetic that would make overlap possible, and record the limitation
     honestly rather than claiming more than you measured.
  4. `ShellFrameObservation` (`src/shell/shell_frame_observation.rs`) already exposes painted region
     rectangles. Use it rather than building a parallel measurement path.

- **Files**: `tests/component_vocabulary.rs`

- **Parallel?**: `[P]` — independent of T037, T038.

- **Notes**: The existing `graphical_application_shell` validation already proves the bands render. This
  subtask adds the geometric arithmetic and the minimum-target bound.

### Subtask T037 – Prove state exhaustiveness, non-color legibility, page totality

- **Purpose**: NFR-005 and FR-005.

- **Steps**:
  1. Assert `ComponentState` has exactly 9 variants and that exhaustive iteration yields all of them.
  2. Assert every state renders text or shape in addition to color. Concretely: render each state and
     assert a non-color signal is present — the `>` cursor for Focused, `M ON` for Muted, `S ON` for
     Soloed, `PREPARING`/`ACTIVATING` for Loading, typed text for Error, a mark for Selected.
  3. Assert every declared gallery page has exactly one digit binding and every binding maps to a declared
     page — a page without a key or a key without a page fails.
  4. Assert every declared state appears in at least one gallery page's specimen set.
  5. These assertions are what make the closed sets load-bearing. Adding a state or page without a
     specimen must fail here.

- **Files**: `tests/component_vocabulary.rs`

- **Parallel?**: `[P]`

- **Notes**: "Non-color signal present" is best asserted on the rendered text content, not on the
  rendering code. Assert what a player could read.

### Subtask T038 – Prove the typeface-missing typed failure

- **Purpose**: FR-010. A substituted face looks plausible and is wrong — that is exactly why this needs proof.

- **Steps**:
  1. Assert that registration against a missing or unreadable face path returns the typed error naming the
     unavailable face.
  2. Assert it does **not** fall back, does not synthesize, and does not return success.
  3. Point the loader at a nonexistent path rather than deleting the real vendored files — the test must
     not mutate the repository.
  4. Assert the success path too: all four weights register, and all eight type styles resolve to a
     registered family.
  5. This is why WP01's T004 chose a runtime read over `include_bytes!` — with `include_bytes!` the
     missing-file case is a compile error and this test cannot exist. If you find registration is
     compile-time embedded, report it rather than deleting the test.

- **Files**: `tests/component_vocabulary.rs`

- **Parallel?**: `[P]`

- **Notes**: The most valuable assertion here is the negative one. A test that only proves the happy path
  would have passed before this mission started, when there was no typeface at all.

## Test Strategy

```bash
cargo test --test component_vocabulary -- --nocapture > /tmp/wp06.log 2>&1; echo "exit=$?"
grep "CREST_ACCEPTANCE component_vocabulary passed" /tmp/wp06.log
make test > /tmp/wp06-full.log 2>&1; echo "exit=$?"
make lint && make fmt-check
```

Never pipe test output through `head`/`tail` — the pipe reports the pager's exit code, so a "green"
recorded that way is a lie. Redirect to a file.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Tests assert names, not values — vacuous and permanent | T034 requires the render path and independently-written expected literals |
| Literal guard matches nothing and passes forever | T035 requires proving the guard fails on a planted literal |
| Marker emitted before assertions run | T033 requires it strictly after every check |
| Render path proves impractical and the check is quietly downgraded | T034 requires stating the limitation explicitly rather than claiming more than measured |
| Typeface test only covers the happy path | T038 makes the negative assertion the primary one |

## Review Guidance

- **Are the expected values written independently, or derived from the thing under test?** This is the question that decides whether this WP is worth anything.
- Does the literal guard have a test proving it fails? Run it on a planted literal and watch it go red.
- Is the acceptance marker strictly after all assertions?
- Does the typeface test assert the **negative** path?
- Were any limitations recorded honestly, or silently downgraded?
- Was the test run recorded by redirect rather than through a pipe?

## Activity Log

> **CRITICAL**: Activity log entries MUST be in chronological order (oldest first, newest last). Append at the END.

- 2026-08-02T02:26:18Z – system – Prompt created.
- 2026-08-02T11:35:00Z – claude – **T033–T038 complete.** `tests/component_vocabulary.rs` created (11 tests). Declared
  validation run by redirect, never through a pipe:
  `cargo test --test component_vocabulary -- --nocapture > /tmp/wp06i.log 2>&1` → `exit=0`, stdout carries
  `CREST_ACCEPTANCE component_vocabulary passed`. `make test` → exit 0, 27/27 targets ok. `make lint` → exit 0.
  `make fmt-check` → exit 0.
- 2026-08-02T11:35:00Z – claude – **Independence of the expected values (T034).** The authored table is transcribed from
  `DESIGN.md` § Colors and § Type and geometry, not read back from `token.rs`. Colors are written as the `#rrggbb`
  strings the design file publishes and parsed by a hex parser local to the test, so nothing shares a derivation with
  the vocabulary's `Color32::from_rgb(0x.., 0x.., 0x..)`. Both directions are asserted — every authored entry has a
  declaration and every declaration has an authored entry — plus counts (17 / 8 / 6 / 3), so a dropped token fails
  rather than passing by absence.
- 2026-08-02T11:35:00Z – claude – **The render path is the production one (T034, T036).** `paint_production_frames`
  drives the real `EframeGraphicalApplication` through a real `egui::Context` with `install_authored_typeface`, at
  1920×1080 and 1280×800 in both PATCH and MIXER, and reads the emitted `epaint` shapes. 334 glyph runs measured.
  Every painted fill, stroke, glyph, and halo resolves through the authored table with **zero exceptions**; every glyph
  run matches exactly one authored style's family, size, line height, and tracking; every stroked keyline is 1 px or
  3 px. No limitation was needed here — reaching the render path was practical.
- 2026-08-02T11:35:00Z – claude – **Six production defects found by measuring rather than declaring, and fixed.**
  Operator was consulted on the first two and directed "fix the adapter, assert the real claim" and "fix it too, allow
  nothing"; the remaining four are the same categories found while implementing that direction. All fixes are in
  `src/adapter/eframe_graphical_window.rs`, which is WP04's authoritative surface — recorded here explicitly because
  WP06 owns only `tests/component_vocabulary.rs`:
  (1) **Every interactive target was below the authored 48 px minimum** — valid-action buttons 18 px, semantic control
  rows 38 px, the diagnostic collapsing header 22 px. Both density policies already declared ≥ 48 px; the adapter never
  applied it. Now `MIN_INTERACTIVE_TARGET_PX` binds each. Measured after: buttons and header 48 px, rows 50 px resting
  / 54 px focused.
  (2) **Four unauthored grays reached the screen** — the meter's unfilled track (`#0a0a0a`), the rule between panels and
  beside an indented body (`#3c3c3c`), and the disclosure triangle (`#b4b4b4`), all from the rendering stack's default
  visuals. `install_authored_chrome` now names them as authored roles once per context.
  (3) **A mixer track's controls ran left to right instead of stacking**, because `egui::Frame` inherits the enclosing
  horizontal layout; each row's right-aligned value landed on top of its label at both viewports. Fixed with an explicit
  `ui.vertical` and `set_width` for the column.
  (4) **The meter's `Code/Value` text was taller than the bar holding it** (20 px text in an 18 px bar). The bar now
  takes its height from that type style.
- 2026-08-02T11:35:00Z – claude – **The literal guard is shown failing, twice (T035).**
  `the_literal_guard_reports_a_planted_literal` feeds planted samples covering all four families (raw-channel colors,
  authored palette hex, font-size literals, spacing literals) and asserts each is reported with file, line, and kind;
  `the_literal_guard_allows_what_the_vocabulary_permits` asserts named constants, `Color32::TRANSPARENT`, resolved
  tokens, zero, and comment narration stay clean. Falsified against the **real tree** as well: a
  `Color32::from_rgb(0x65, 0xe5, 0xff)` planted in `src/adapter/production_effects.rs` produced
  `src/adapter/production_effects.rs:890: color literal outside the vocabulary` and
  `…:890: palette literal outside the vocabulary — color/accent/focus (#65e5ff) is spelled here`; the file was restored
  and `git diff --stat` is clean. The scan reads 59 sources / 40 990 lines, and asserts that footprint before asserting
  emptiness so it cannot pass vacuously.
- 2026-08-02T11:35:00Z – claude – **Recorded limitations, stated rather than downgraded** (all also in the module docs):
  (a) *Spacing steps do not reach the shape stream* — `ui.add_space` emits no shape, so what is measured for spacing is
  the band/split arithmetic plus the declared step values.
  (b) *Corner radii are not asserted through the render path* — the rendering stack composes its own radii for the
  widgets it owns; radii are compared at the declaration.
  (c) *Interactive-target measurement is split in two* — the shell's own framed rects are measured as painted geometry;
  the pointer targets the stack lays out are read from its interactive-widget registry via the debug overlay, filtered
  to the click-only sense because the stack registers every label as click-and-drag for text selection.
  (d) *Clipping is asserted only where nothing scrolls* — the shell composes three scroll regions whose content
  legitimately exceeds their viewport at 1280×800 and a shape stream cannot tell "scrolled" from "cut". Containment is
  asserted for the two bands with no scroll region inside them; the other 14 runs that left their container are counted
  and reported as `runs_scrolled_out_of_view` in the observation line. Overlap is asserted only between runs that are
  both fully visible.
  (e) *Per-page painted specimen coverage lives elsewhere* — `paint_gallery` is private to
  `src/testing/component_gallery_scene.rs`, so this target proves page/digit totality in both directions and leaves
  "every state painted a specimen at both authored sizes" to that module's own tests over its real paint pass.
- 2026-08-02T11:35:00Z – claude – **Open design question for a follow-on mission, not answered here.** At 1280×800 the
  footer's valid-action hints and the mixer track strip extend past the right edge and are reachable only by scrolling.
  That is inside the shell's declared scroll regions, so it is not asserted as a defect, but whether a controller-first
  Steam Deck layout should scroll its action hints at all is a design decision `DESIGN.md` does not settle.
- 2026-08-02T11:35:00Z – claude – **Marker discipline (T033).** `CREST_ACCEPTANCE component_vocabulary passed` is
  printed by `component_vocabulary_acceptance` and nowhere else, strictly after every check function returns; a failing
  check panics before the print. The same checks are also exposed as ten individual `#[test]`s so a failure names which
  claim broke.
- 2026-08-02T11:35:00Z – claude – **Verified by running the program.** `cargo run --bin crest-synth` opened the real
  window; a screenshot confirms the mixer now renders as stacked track columns with label left and value right and no
  collision, the focus keyline is the authored cyan, the footer targets are full height, and the band rules are the
  authored hairline.
- 2026-08-02T11:35:00Z – claude – Note for whoever runs this next: `sf2/` is gitignored and absent from a fresh
  worktree, so the render-path tests (and the pre-existing `graphical_application_shell` ones) need it symlinked in.
  Nothing was committed for it.
