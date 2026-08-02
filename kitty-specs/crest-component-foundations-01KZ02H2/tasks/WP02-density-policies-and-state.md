---
work_package_id: WP02
title: Viewport density policies and component state
dependencies:
- WP01
requirement_refs:
- FR-003
- FR-005
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
authoritative_surface: src/shell/visual/
create_intent: []
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
- The Steam Deck policy exists, is authored from the desktop frames, and is marked as authored rather than measured.
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

### Subtask T008 – Author the Steam Deck density policy

- **Purpose**: Give the 1280×800 viewport a declared policy. No authored design exists for it, so this is authorship, not transcription.

- **Steps**:
  1. Declare the Steam Deck policy at 1280×800.
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
     use the Steam Deck policy at or below 1280 wide, Desktop above. State the rule; do not leave it implicit.

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
| Steam Deck policy is never reviewed by a human | Flag it in the Activity Log as needing visual review; WP05 makes it viewable |
| Loading/Error implemented as color-only | Declared as data pairing accent **with** text, and WP06 asserts non-color legibility |
| A wildcard `match` arm creeps in | Doc comment forbids it; WP06 asserts exhaustiveness |

## Review Guidance

- Does the Desktop policy reproduce `eframe_graphical_window.rs:17-25` exactly? This is the regression risk.
- Is the Steam Deck policy clearly marked authored rather than measured?
- Does the side region stay visible at 1280×800, or was it dropped to fit?
- Is `ComponentState` genuinely closed — no catch-all, no `Unknown`?
- Do Loading and Error carry text, or only color?

## Activity Log

> **CRITICAL**: Activity log entries MUST be in chronological order (oldest first, newest last). Append at the END.

- 2026-08-02T02:26:18Z – system – Prompt created.
