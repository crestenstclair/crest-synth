---
work_package_id: WP01
title: Authored visual vocabulary and typeface
dependencies: []
requirement_refs:
- FR-001
- FR-002
- FR-010
- NFR-006
planning_base_branch: feat/crest-component-foundations
merge_target_branch: feat/crest-component-foundations
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-foundations. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-foundations unless the human explicitly redirects the landing branch.
subtasks:
- T001
- T002
- T003
- T004
- T005
- T006
phase: Phase 1 - Foundation
history:
- at: '2026-08-02T02:26:18Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent_profile: implementer-ivan
agent: claude
authoritative_surface: src/shell/visual/
create_intent:
- src/shell/visual/mod.rs
- src/shell/visual/token.rs
- src/shell/visual/typeface.rs
- src/shell/visual/density.rs
- src/shell/visual/state.rs
- src/shell/visual/primitives.rs
execution_mode: code_change
owned_files:
- src/shell/mod.rs
- src/shell/visual/mod.rs
- src/shell/visual/token.rs
- src/shell/visual/typeface.rs
- DESIGN.md
role: implementer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP01 – Authored visual vocabulary and typeface

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

Declare every semantic color, type style, spacing step, and geometry value **once**, with the authored
values exactly, and make the vendored typeface actually renderable.

Complete when:

- `src/shell/visual/` exists as a compiling module tree, with stub files for the modules WP02 and WP03 fill in.
- All 17 semantic colors, all 8 type styles, all 6 spacing steps, and the geometry values are declared with their authored values.
- The vendored typeface registers in four weights, and an unavailable face produces a typed error rather than a fallback.
- `DESIGN.md` records the three durable decisions.
- A test proves each declared value equals its authored counterpart — comparing **values**, not names.

## Context & Constraints

**Crest-spec resources this WP realizes** (canonical, do not restate them here):

- `valueObject.Shell.SemanticVisualToken`
- `valueObject.Shell.AuthoredTypeface`
- `asset.ShellContextModules` — carries the generation prompts for this surface
- `asset.ProductDesignAuthority` — carries what `DESIGN.md` must record
- `requirement.semantic_visual_vocabulary`, `requirement.authored_typeface_installation`

**Supporting documents**: [plan.md](../plan.md) (IC-01, IC-02) · [research.md](../research.md) (R-01, R-02, R-03) · [spec.md](../spec.md) (FR-001, FR-002, FR-010, NFR-001, NFR-006)

**The authored values are already confirmed.** `DESIGN.md:534-573` was verified against the design file on
2026-08-02 and matches exactly. Use it as the source. Do not re-derive values, and do not "improve" one
because it looks off on your monitor.

**Hard constraint**: raw color, size, and spacing values stay **private** to `token.rs`. Everything outside
reaches them only through a named token. This is what WP06 will prove and what the whole mission is for.

## Branch Strategy

- **Strategy**: feature-branch
- **Planning base branch**: `feat/crest-component-foundations`
- **Merge target branch**: `feat/crest-component-foundations`

> Populated by `spec-kitty agent mission finalize-tasks`. Do not edit manually.

Execution worktrees are allocated per computed lane from `lanes.json`.

## Subtasks & Detailed Guidance

### Subtask T001 – Create the `src/shell/visual/` module tree with compiling stubs

- **Purpose**: Establish the module layout in one place so WP02 and WP03 add code to files that already exist, instead of each editing `src/shell/mod.rs` and colliding.

- **Steps**:
  1. Add exactly one line to `src/shell/mod.rs`: `pub mod visual;`. Keep the existing `pub mod` and `pub use` ordering intact — that file is alphabetized.
  2. Create `src/shell/visual/mod.rs` declaring all five submodules up front:
     ```rust
     pub mod density;    // WP02
     pub mod primitives; // WP03
     pub mod state;      // WP02
     pub mod token;      // WP01 (this WP)
     pub mod typeface;   // WP01 (this WP)
     ```
  3. Create empty-but-compiling stubs for `density.rs`, `state.rs`, and `primitives.rs` with a one-line comment naming the WP that fills each in.
  4. Re-export the public surface from `src/shell/visual/mod.rs` as the other WPs land — for now, only what T002–T004 create.

- **Files**: `src/shell/mod.rs` (1 line added), `src/shell/visual/mod.rs`, `src/shell/visual/{density,state,primitives}.rs` (stubs)

- **Parallel?**: No — everything else in this WP depends on it.

- **Notes**: `primitives` may become a directory in WP03. Declaring it as `pub mod primitives;` works either way (`primitives.rs` or `primitives/mod.rs`), so WP03 can promote it without touching this file.

### Subtask T002 – Declare the 17 semantic colors with authored values

- **Purpose**: One authoritative color set, addressed by canonical name, with raw values private.

- **Steps**:
  1. In `src/shell/visual/token.rs`, declare the color vocabulary. Names come from the design file's variables, not invented ones — see the Domain Language table in [spec.md](../spec.md).
  2. Declare all 13:

     | Canonical name | Value |
     |---|---|
     | `bg/canvas` | `#0c1015` |
     | `bg/surface` | `#121821` |
     | `bg/panel` | `#17202a` |
     | `bg/elevated` | `#1d2733` |
     | `bg/selected` | `#2a3745` |
     | `border/default` | `#2a3745` |
     | `border/strong` | `#415166` |
     | `text/primary` | `#f2f6f8` |
     | `text/secondary` | `#b8c4d1` |
     | `text/muted` | `#6f8095` |
     | `accent/focus` | `#65e5ff` |
     | `accent/adjust` | `#ffb454` |
     | `accent/positive` | `#58e887` |
     | `accent/warning` | `#ff6868` |
     | `accent/instrument` | `#b894ff` |
     | `accent/patch` | `#ff6fbe` |
     | `accent/chorus` | `#f6f178` |

     That table has 17 rows because the design file publishes 13 variables while `DESIGN.md` adds four
     more; R-02 in [research.md](../research.md) resolved this as a **union**. Declare all of them. The
     "13" in the crest-spec refers to the design file's published variable count — the union is what
     ships.
  3. Keep the raw `Color32` construction private to this module. Expose a resolver, not the constants.
  4. `bg/selected` and `border/default` share the hex value `#2a3745` deliberately. Keep them as two
     distinct named tokens — they mean different things and will diverge.

- **Files**: `src/shell/visual/token.rs`

- **Parallel?**: No — T003 and T006 build on the shape chosen here.

- **Notes**: `accent/instrument` is `color/accent/instrument/plates` in the design file and `instrument`
  in `DESIGN.md`. Same value. Use a name that reads well in Rust; record the mapping in a doc comment so
  the next person can find it in the design file.

### Subtask T003 – Declare the 8 type styles, 6 spacing steps, and geometry

- **Purpose**: Complete the vocabulary so nothing downstream needs a literal.

- **Steps**:
  1. Declare the eight type styles with their authored metrics:

     | Style | Size / line | Weight | Tracking |
     |---|---|---|---|
     | `Display/Screen` | 32 / 40 | SemiBold 600 | 0.4 |
     | `Heading/Section` | 18 / 24 | SemiBold 600 | 1.4 |
     | `Heading/Panel` | 14 / 20 | Bold 700 | 1.2 |
     | `Body/Default` | 15 / 22 | Regular 400 | 0 |
     | `Body/Compact` | 13 / 18 | Regular 400 | 0 |
     | `Label/Control` | 12 / 16 | Medium 500 | 0.8 |
     | `Code/Value` | 14 / 20 | SemiBold 600 | 0.2 |
     | `Instruction/Hint` | 11 / 16 | Medium 500 | 0.8 |

  2. Declare the six spacing steps: 4, 8, 12, 16, 24, 32.
  3. Declare geometry: radii 0 / 4 / 8; resting keyline 1 px; emphasis keyline 3 px; minimum interactive
     target 48 px; focus halo radius 8, spread 1, opacity 0.28.
  4. egui has no first-class letter-spacing control. If tracking cannot be applied faithfully, declare the
     value anyway and record the rendering limitation in a doc comment — do not silently drop it, and do
     not fake it by padding glyphs.

- **Files**: `src/shell/visual/token.rs`

- **Parallel?**: `[P]` — independent of T002.

- **Notes**: The halo opacity is `0x47` alpha in the design file — 71/255 = 0.278. `DESIGN.md` rounds to
  0.28. Either is faithful; pick one and note it.

### Subtask T004 – Register the vendored typeface with typed failure on absence

- **Purpose**: Make the authored type styles renderable, and make an unavailable face loud rather than invisible.

- **Steps**:
  1. In `src/shell/visual/typeface.rs`, load the four faces from `vendor/azeret-mono/`:
     `AzeretMono-Regular.ttf`, `-Medium.ttf`, `-SemiBold.ttf`, `-Bold.ttf`.
  2. Register them into egui's `FontDefinitions` — one family per weight, since egui selects by family
     name, not by weight. Map each of the eight type styles to the family matching its declared weight.
  3. Registration happens **once**, before the first painted frame. Never per frame, and never from the audio callback.
  4. Return a typed error naming the unavailable face if a file is missing or unparseable. Do not fall
     back, do not synthesize, and do not let egui's default stack absorb it. WP04 surfaces this error;
     this WP only needs to produce it.
  5. Decide `include_bytes!` vs runtime read and record why. `include_bytes!` makes the binary
     self-contained and the missing-file case a compile error — which would make T038's runtime test
     impossible. Prefer a runtime read so the failure path is reachable and testable.

- **Files**: `src/shell/visual/typeface.rs`

- **Parallel?**: No — depends on T003 for the style/weight mapping.

- **Notes**: See R-03 in [research.md](../research.md) for why four static faces exist rather than the
  upstream variable font: `ab_glyph` supports variation axes but `epaint` does not expose them, so a
  variable font would paint every style at one weight.

### Subtask T005 – Record the three durable decisions in `DESIGN.md`

- **Purpose**: `DESIGN.md` is the product authority. Decisions made here belong in it, not only in mission artifacts.

- **Steps**:
  1. Add to the Durable Decisions section:
     - The color set is the **union** of the design file's published variables and `DESIGN.md`'s table.
       The design file publishes a selected-row background this document omitted; this document declares
       elevated, strong border, patch, and chorus accents the design file does not publish as variables.
       Neither source is trimmed to match the other.
     - The Steam Deck density policy is **authored** from the desktop frames, not measured from an
       authored small-viewport design, because none exists.
     - Loading and error appearances **reuse** the structural-edit vocabulary this document already
       declares (`DESIGN.md:454`) rather than inventing a second visual language.
  2. Add `bg/selected` to the color table at `DESIGN.md:534-551`.
  3. Match the existing prose register. Terse, declarative, no hedging. Read the surrounding entries first.

- **Files**: `DESIGN.md`

- **Parallel?**: `[P]` — independent of the code subtasks.

- **Notes**: Do not restructure the document or rewrite unrelated sections. `asset.ProductDesignAuthority`
  is explicit that this file is never generated and that edits are deliberate authorial acts.

### Subtask T006 – Prove declared values equal the authored table

- **Purpose**: Make value drift a test failure. This is the first half of NFR-001; WP06 completes it through the render path.

- **Steps**:
  1. Add unit tests in `src/shell/visual/token.rs` asserting each declared color resolves to its exact
     authored RGB, each type style to its exact size / line height / weight / tracking, and each spacing
     and geometry value to its exact number.
  2. Assert the **counts** too — 17 colors, 8 type styles, 6 spacing steps — so a silently dropped token
     fails rather than passing by absence.
  3. Assert the typeface registers all four weights and that each of the eight styles resolves to a
     registered family.
  4. Write the expected values as literals in the test, independent of the implementation. A test that
     compares the vocabulary to itself proves nothing.

- **Files**: `src/shell/visual/token.rs`, `src/shell/visual/typeface.rs` (`#[cfg(test)]` modules)

- **Parallel?**: No — it depends on T002, T003, T004.

- **Notes**: Resist a loop over the vocabulary comparing it to a table derived from the vocabulary. Spell
  the expected values out.

## Test Strategy

```bash
cargo test --lib shell::visual
make lint && make fmt-check
```

Tests are required for T006 — NFR-001 is a measured claim.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Token API is awkward to consume, so WP03/WP04 work around it with literals | Write one primitive call site by hand before finalizing the API shape |
| Test compares the vocabulary against itself | Expected values are literals in the test, written independently |
| `include_bytes!` makes the missing-typeface path unreachable | Runtime read, so T038 can actually test it |
| Editing `DESIGN.md` drifts into rewriting unrelated sections | Additive edits only; match the existing register |

## Review Guidance

- Are the values **exactly** the authored ones? Spot-check `accent/focus` = `#65e5ff` and `Display/Screen` = 32/40 SemiBold 0.4.
- Are raw values genuinely private to `token.rs`, or does the module leak `Color32` constants?
- Does the missing-typeface path return a typed error, or does something absorb it?
- Are the T006 expected values independent of the implementation?
- Does `DESIGN.md` record all three decisions without collateral edits?

## Activity Log

> **CRITICAL**: Activity log entries MUST be in chronological order (oldest first, newest last). Append at the END.

- 2026-08-02T02:26:18Z – system – Prompt created.
