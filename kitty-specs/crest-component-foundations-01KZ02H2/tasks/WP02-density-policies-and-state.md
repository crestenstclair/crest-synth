---
work_package_id: WP02
title: Viewport density policies and component state
dependencies:
- WP01
requirement_refs:
- FR-003
- FR-005
planning_base_branch: feat/crest-component-foundations
merge_target_branch: feat/crest-component-foundations
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-foundations. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-foundations unless the human explicitly redirects the landing branch.
subtasks:
- T007
- T008
- T009
- T010
- T011
- T012
phase: Phase 1 - Foundation
history:
- at: '2026-08-02T02:26:18Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent_profile: implementer-ivan
agent: claude
authoritative_surface: src/shell/visual/
create_intent:
- src/shell/visual/density.rs
- src/shell/visual/state.rs
execution_mode: code_change
owned_files:
- src/shell/visual/density.rs
- src/shell/visual/state.rs
role: implementer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP02 – Viewport density policies and component state

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

Turn "the app supports two screen sizes" from scattered constants into two declared policies, and close
the behavioral state set so adding a state names every site that must change.

Complete when:

- `ViewportDensityPolicy` resolves both authored viewports, carrying bands, split, inset, row height, row pitch, and control geometry.
- The Desktop policy reproduces today's geometry **exactly** — no silent visual change.
- The compact policy exists, is authored from the desktop frames, and is marked as authored rather than measured.
- `ComponentState` is a closed nine-value set, exhaustively matchable.
- Loading and Error map onto the structural-edit vocabulary rather than a new visual language.
- A test proves both policies retain every band, the side region, and the 48 px minimum.

## Context & Constraints

**Crest-spec resources this WP realizes**:

- `valueObject.Shell.ViewportDensityPolicy`
- `valueObject.Shell.ComponentState`
- `requirement.viewport_density_policy`, `requirement.explicit_state_rendering`

**Supporting documents**: [plan.md](../plan.md) (IC-03, IC-04) · [research.md](../research.md) (R-04, R-05, R-06) · [spec.md](../spec.md) (FR-003, FR-005, NFR-003)

**You are filling in stubs WP01 created.** `src/shell/visual/density.rs` and `src/shell/visual/state.rs`
already exist and are already declared in `src/shell/visual/mod.rs`. You own those two files and nothing
else. If `mod.rs` needs a re-export line, that is an acceptable out-of-map edit — record a one-line
rationale in your Activity Log.

**Critical**: the adapter currently derives its side-region width with an ad-hoc proportional rule at
`src/adapter/eframe_graphical_window.rs:707`. This WP defines the replacement; **WP04 performs the swap**.
Do not edit the adapter.

## Branch Strategy

- **Strategy**: feature-branch
- **Planning base branch**: `feat/crest-component-foundations`
- **Merge target branch**: `feat/crest-component-foundations`

> Populated by `spec-kitty agent mission finalize-tasks`. Do not edit manually.

Execution worktrees are allocated per computed lane from `lanes.json`.

## Subtasks & Detailed Guidance

### Subtask T007 – Implement the Desktop density policy from measured geometry

- **Purpose**: Capture the authored 1920×1080 layout as data, reproducing today's rendering exactly.

- **Steps**:
  1. In `src/shell/visual/density.rs`, declare the Desktop policy with these **measured** values:

     | Field | Value | Source |
     |---|---|---|
     | viewport | 1920 × 1080 | authored |
     | context line height | 48 | `DESIGN.md:440`, matches `eframe_graphical_window.rs:21` |
     | identity header height | 72 | `DESIGN.md:441`, matches line 22 |
     | workspace height | 896 | `DESIGN.md:442` |
     | footer height | 64 | `DESIGN.md:445`, matches line 23 |
     | main surface width | 1500 | `DESIGN.md:443` |
     | side region width | 420 | `DESIGN.md:444`, matches line 24 |
     | content inset | 24 | measured from the design file |
     | row height | 52 | measured from the design file |
     | row pitch | 66 | measured from the design file |
     | utility control | 380 × 48, 60 pitch, 5 px bar | measured from the design file |

  2. Read `src/adapter/eframe_graphical_window.rs:17-25` first and confirm your values reproduce it. If
     any differ, **stop and report** — a mismatch means either the measurement or the current code is
     wrong, and guessing hides a regression inside a refactor.
  3. Mark the Desktop policy as measured-from-authored-design in a doc comment.

- **Files**: `src/shell/visual/density.rs`

- **Parallel?**: No — T008 and T009 build on it.

- **Notes**: Row height 52 exceeds the 48 px minimum target. That is correct, not a rounding error.

### Subtask T008 – Author the compact density policy

- **Purpose**: Give the 1280×800 viewport a declared policy. No authored design exists for it, so this is authorship, not transcription.

- **Steps**:
  1. Declare the compact policy at 1280×800.
  2. Constraints that must hold (`DESIGN.md:450`):
     - All five structural bands remain present.
     - The Utility/Inspector side region stays **visible** — never hidden to fit.
     - No interactive target drops below 48 px.
     - Hierarchy and proportion are preserved through controlled density, **not** uniform scaling.
  3. Suggested starting point — refine by eye, do not treat as gospel:
     - Bands: context 40, identity 60, footer 56. These carry text at fixed authored sizes, so they
       shrink less than proportionally.
     - Side region: 320 (the existing `MINIMUM_SIDE_WIDTH`, `eframe_graphical_window.rs:25`), leaving 960 main.
     - Content inset 16 rather than 24.
     - Row height 48 (the floor, not below), pitch 56.
  4. Mark this policy **authored, not measured** in a doc comment, and note that it needs visual review.
  5. Declare how a viewport between or below the two authored sizes resolves. Simplest defensible rule:
     use the compact policy at or below 1280 wide, Desktop above. State the rule; do not leave it implicit.

- **Files**: `src/shell/visual/density.rs`

- **Parallel?**: No — depends on T007.

- **Notes**: R-05 in [research.md](../research.md) records why this is authored and that the operator
  approved it. This is the one part of the mission that genuinely needs a human eye — get it viewable
  early rather than perfect on the first pass.

### Subtask T009 – Expose the policy API the adapter will consume

- **Purpose**: Give WP04 something to call, so the swap is mechanical rather than a redesign.

- **Steps**:
  1. Provide resolution from a viewport size to a policy.
  2. Provide accessors for every field a surface needs: band heights, main/side split, inset, row height,
     row pitch, control geometry.
  3. Do **not** expose the raw viewport dimensions in a way that invites branching on them. The invariant
     is that no surface branches on a raw size — the API should make asking the policy the easy path and
     branching the awkward one.
  4. Add a doc comment naming `eframe_graphical_window.rs` as the intended first consumer and pointing at WP04.

- **Files**: `src/shell/visual/density.rs`

- **Parallel?**: No.

- **Notes**: If you find yourself wanting a field the adapter does not need yet, leave it out. WP04 will
  say what is missing.

### Subtask T010 – Declare the closed nine-value `ComponentState`

- **Purpose**: Make the state set closed so the compiler names every site that must change when a state is added.

- **Steps**:
  1. In `src/shell/visual/state.rs`, declare exactly nine states: `Resting`, `Focused`, `Adjusting`,
     `Disabled`, `Loading`, `Error`, `Muted`, `Soloed`, `Selected`.
  2. Provide an exhaustive iteration over all nine, plus a `const` count, so WP05 and WP06 can assert coverage.
  3. Add a test asserting the count is 9 and that iteration yields every variant, so adding a variant
     without updating the iterator fails.
  4. Do **not** add a catch-all or `Unknown` variant. The closedness is the feature — it is what made
     `SelectPatch` safe to add, per the mission kickoff.

- **Files**: `src/shell/visual/state.rs`

- **Parallel?**: `[P]` — independent of T007–T009.

- **Notes**: Any `match` on this type anywhere must be exhaustive with no wildcard arm. Say so in a doc comment.

### Subtask T011 – Map Loading and Error onto the structural-edit vocabulary

- **Purpose**: Reuse the visual language the product already declares instead of inventing a second one.

- **Steps**:
  1. Declare Loading as the adjustment accent (`accent/adjust`) plus progress text — `PREPARING` or
     `ACTIVATING`, matching the vocabulary at `DESIGN.md:454`.
  2. Declare Error as the warning accent (`accent/warning`) plus short typed text.
  3. Both carry text, never color alone. `DESIGN.md:575` requires text or shape in addition to color for
     every state, and this is the pair most likely to be implemented as color-only.
  4. No animation or spinner. Text or shape only — an animated indicator also forces a per-frame repaint
     the 16 ms idle cadence does not want.
  5. Declare the mapping as data next to `ComponentState`, not as branching inside each primitive. WP03
     consumes it.

- **Files**: `src/shell/visual/state.rs`

- **Parallel?**: No — depends on T010.

- **Notes**: R-06 in [research.md](../research.md) records this decision and its rationale.

### Subtask T012 – Prove both policies retain bands, split, and minimum target

- **Purpose**: Make NFR-003 a test rather than a hope.

- **Steps**:
  1. Add tests asserting, for **both** policies: all five bands present with nonzero height; side region
     width nonzero; band heights plus workspace equal the viewport height exactly; main plus side equal
     the viewport width exactly; row height and every interactive target at or above 48.
  2. Add a test asserting the Desktop policy reproduces the current adapter constants exactly — 48, 72,
     64, 420, 1920×1080. This is the regression guard for T007.
  3. Add a test for the between-sizes resolution rule from T008.

- **Files**: `src/shell/visual/density.rs` (`#[cfg(test)]`)

- **Parallel?**: No — depends on T007, T008.

- **Notes**: The arithmetic assertions are the valuable ones. "Bands sum to viewport height" catches a
  whole class of layout errors that eyeballing misses.

## Test Strategy

```bash
cargo test --lib shell::visual
make lint && make fmt-check
```

Tests are required for T010 and T012 — NFR-003 and the closed-set guarantee are measured claims.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Desktop policy silently changes today's geometry | T012 asserts it equals the current adapter constants exactly |
| compact policy is never reviewed by a human | Flag it in the Activity Log as needing visual review; WP05 makes it viewable |
| Loading/Error implemented as color-only | Declared as data pairing accent **with** text, and WP06 asserts non-color legibility |
| A wildcard `match` arm creeps in | Doc comment forbids it; WP06 asserts exhaustiveness |

## Review Guidance

- Does the Desktop policy reproduce `eframe_graphical_window.rs:17-25` exactly? This is the regression risk.
- Is the compact policy clearly marked authored rather than measured?
- Does the side region stay visible at 1280×800, or was it dropped to fit?
- Is `ComponentState` genuinely closed — no catch-all, no `Unknown`?
- Do Loading and Error carry text, or only color?

## Activity Log

> **CRITICAL**: Activity log entries MUST be in chronological order (oldest first, newest last). Append at the END.

- 2026-08-02T02:26:18Z – system – Prompt created.
- 2026-08-01 – claude (implementer-ivan) – T007. Read `src/adapter/eframe_graphical_window.rs:17-25` before writing anything. Every measured value in the T007 table matches the constants already there: 1920×1080, 1280×800, context 48, identity 72, footer 64, side 420, minimum side 320. No mismatch to report. Workspace 896 and main 1500 are the remainders the adapter derives through `StripBuilder` rather than naming; the policy names them. Declared `ViewportDensityPolicy::Desktop` with `PolicyProvenance::MeasuredFromAuthoredDesign`.
- 2026-08-01 – claude (implementer-ivan) – T008. Authored `ViewportDensityPolicy::SteamDeck` at 1280×800 from the suggested starting point: bands 40/60/56 with workspace 644 (the remainder, so the four bands tile 800 exactly), side region 320 with main 960, inset 16, row 48 on a 56 pitch. The side region is narrowed to the existing `MINIMUM_SIDE_WIDTH`, never hidden. Utility control width 280 rather than the desktop 380, keeping the same 40 px total side-region padding; height stays 48 because it is an interactive target and pitch drops to 56. Bar thickness stays 5 px in both policies — it is a visual weight, not a spatial one, and thinning it would cost legibility without buying room. Marked `PolicyProvenance::AuthoredFromDesktopFrames`. **This policy needs the operator's eye.** Every number in it is authorship against declared minimums, not transcription, and no design exists at this size to check it against. WP05 makes it viewable; please look at it there before it hardens.
- 2026-08-01 – claude (implementer-ivan) – T008 resolution rule. Declared `STEAM_DECK_MAX_WIDTH_PX = 1280` and `ViewportDensityPolicy::resolve(viewport_width_px)`: at or below that width — including below the authored Steam Deck size — the Steam Deck policy applies, above it the Desktop policy does. One threshold, two outcomes, no interpolated third layout.
- 2026-08-01 – claude (implementer-ivan) – T009. Exposed `bands()`, `split()`, `rhythm()`, `utility_control()`, `provenance()`, `authored_viewport()`, and `canonical_name()`. `resolve` is the only function that reads a raw viewport width, and it is documented as such; `authored_viewport()` exists for opening a window or sizing a specimen frame and is documented as not being a layout branch. Kept the surface to what WP04 needs to delete `desired_side_width` and the band constants — nothing speculative. Doc comment names `eframe_graphical_window.rs` as the intended first consumer and points at WP04.
- 2026-08-01 – claude (implementer-ivan) – T010. Declared `ComponentState` as exactly nine variants with no catch-all and no `Unknown`, plus `ALL_COMPONENT_STATES` and `COMPONENT_STATE_COUNT = 9`. The module doc states that any `match` on the type is exhaustive with no wildcard arm, and why: a `_` arm converts the compile error this type exists to produce into a silent visual default. The iteration test matches exhaustively over each variant, so adding one without updating `ALL_COMPONENT_STATES` fails rather than silently under-covering.
- 2026-08-01 – claude (implementer-ivan) – T011. Declared state appearance as data next to `ComponentState`, not as branching inside primitives: accent, keyline width, halo, row fill, and a `NonColorSignal`. Loading is `color/accent/adjust` with `NonColorSignal::ProgressWord` drawn from `LOADING_PROGRESS_WORDS = ["Preparing", "Activating"]`; Error is `color/accent/warning` with `NonColorSignal::TypedFailure`. Both carry text, neither animates. Casing follows `DESIGN.md:454` and the existing `EngineSelectionStatusKind::Preparing`/`Activating` rather than the prompt's uppercase rendering of the same words, so one vocabulary reads the same everywhere. Disabled carries `"Locked"` and Muted/Soloed carry `"M ON"`/`"S ON"` from `DESIGN.md:468` — authored words, not invented ones. Loading deliberately shares the adjustment accent with Adjusting and is told apart by text, which is the point of the reuse.
- 2026-08-01 – claude (implementer-ivan) – Judgement call on `Resting`. The crest-spec invariant reads "every state renders with text or shape in addition to color". Resting is the baseline the other eight read against, so it carries no distinguishing mark of its own and inventing one would be dishonest. The test asserts the falsifiable form instead: no two of the nine states differ only in accent, and every state other than Resting is distinguishable from Resting without color. Flagging it here rather than burying it, since a reviewer reading the invariant literally will look for a ninth mark.
- 2026-08-01 – claude (implementer-ivan) – T012. Ten tests in `density.rs`: the Desktop regression guard against the current adapter constants, the authored-design values, the Steam Deck values, all five structural regions present, bands tiling the viewport height exactly, the split tiling its width exactly, every interactive target at or above `MIN_INTERACTIVE_TARGET_PX`, positive row and control gaps, the control fitting inside its side region, and the resolution threshold at 1280/1281 plus each authored size resolving to itself. Eleven in `state.rs`, including the count, exhaustive iteration, and the no-two-states-differ-only-in-color pairwise proof.
- 2026-08-01 – claude (implementer-ivan) – Out-of-map edit: added `pub use density::{…}` and `pub use state::{…}` re-export lines to `src/shell/visual/mod.rs`, matching the `pub use token::{…}` line WP01 established so downstream work packages import from `shell::visual` rather than reaching into submodules. No other change to that file.
- 2026-08-01 – claude (implementer-ivan) – Environment note, not a code finding: `cargo test --lib` initially reported 145 failures in this lane, all `FileOpen` on `./sf2/HiDef.sf2`. That asset is gitignored (`/sf2/`, 247 MB) and exists only in the primary checkout, so a fresh git worktree cannot see it. Symlinked it into the lane (git-excluded, not committed) and the suite went green. Any lane that runs the SoundFont tests needs the same link.
- 2026-08-01 – claude (implementer-ivan) – Verification. `cargo test --lib shell::visual`: 41 passed, 0 failed. Full `cargo test`: 508 lib plus every integration target, 0 failed across the suite. `make lint` (clippy `--all-targets -D warnings`) and `make fmt-check` both clean. Committed as `8f57722`.
