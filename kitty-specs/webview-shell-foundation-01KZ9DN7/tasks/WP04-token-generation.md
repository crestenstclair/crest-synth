---
work_package_id: WP04
title: Token generation
dependencies:
- WP02
requirement_refs:
- FR-002
planning_base_branch: feat/webview-shell-foundation
merge_target_branch: feat/webview-shell-foundation
branch_strategy: lane worktree computed by finalize-tasks; merges into feat/webview-shell-foundation
subtasks:
- T014
- T015
- T016
history:
- '2026-08-05: authored from plan IC-04'
agent_profile: implementer-ivan
authoritative_surface: src/shell/webview/
create_intent:
- src/shell/webview/token_export.rs
- webview-page/tokens.css
execution_mode: code_change
owned_files:
- src/shell/webview/token_export.rs
- webview-page/tokens.css
- Makefile
role: implementer
tags: []
tracker_refs: []
---

## ⚡ Do This First: Load Agent Profile

Before reading anything else in this prompt, load your assigned profile:

```
/ad-hoc-profile-load implementer-ivan
```

Adopt its identity, boundaries, and governance scope for the duration of this
work package.

## Objective

`webview-page/tokens.css` exists, is generated from the authored Rust
vocabulary, and cannot silently drift: regeneration is one command, the
committed file is asserted byte-identical to fresh output, and the page (WP05)
consumes only these custom properties.

## Context

- Plan IC-04; research R-04; crest-spec `asset.WebviewProjectionPage` rule
  "the page declares no raw value the vocabulary already names" and the
  adapter rule "a hand-copied or drifted value is a defect".
- The vocabulary: `src/shell/visual/token.rs` (`SemanticColor` — 17 colors,
  `TypeStyle` — 8 styles with size/line-height/weight/tracking,
  `SpacingStep` — 6 steps, `Radius` — 3, `KEYLINE_*`, `FOCUS_HALO_*`,
  `MIN_INTERACTIVE_TARGET_PX`), each with `canonical_name()`. The spike's
  hand-written CSS block (`spike/webview-mixer/index.html`) shows the target
  shape — but the spike hand-copied; this WP makes generation the only
  source.
- The generator is product code in the webview module (not build.rs — the
  repo's build.rs builds vendored C++; keep it out of this).

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-foundation`.
Execution happens in the lane worktree `finalize-tasks` computes; do not
branch manually.

## Subtasks

### T014 — Generate tokens.css from the authored Rust vocabulary

`src/shell/webview/token_export.rs`:

- `pub fn tokens_css() -> String` iterating the declared vocabularies
  (`ALL_COLORS`, `ALL_TYPE_STYLES`, `ALL_SPACING_STEPS`, `ALL_RADII`, the
  keyline/halo/target constants) and emitting one `:root { … }` block, one
  custom property per canonical authored name, mechanically transformed
  (`color/bg/canvas` → `--color-bg-canvas`; type styles expand to
  `--type-<name>-size/line/weight/tracking`).
- Deterministic output: declaration order = the declared `ALL_*` order; a
  header comment naming the generator and warning GENERATED — DO NOT EDIT.
- Unit tests in-module: every `ALL_*` member appears exactly once; a
  spot-checked value matches its authored constant; output is stable across
  two calls.

### T015 — Wire generation into the Makefile and commit the generated table

- A small `#[test]`-ignored writer or a tiny bin entry is overkill — expose
  generation through the existing test harness convention: `make
  webview-tokens` runs `cargo test --test webview_projection_shell
  regenerate_tokens -- --ignored` … except WP06 owns that test file. Instead:
  add a `#[ignore]`d writer test INSIDE `token_export.rs`'s mod tests
  (`cargo test --lib token_export::tests::write_tokens_css -- --ignored`)
  that writes `webview-page/tokens.css`, and a `webview-tokens` Makefile
  target invoking exactly that. Document the target in the Makefile's help
  convention.
- Run it; commit the generated `webview-page/tokens.css`.

### T016 — Expose the freshness check the acceptance test consumes

`pub fn committed_tokens_are_fresh(committed: &str) -> Result<(), TokenDrift>`
comparing byte-for-byte against `tokens_css()`, with `TokenDrift` naming the
first differing property. In-module unit test: mutate one byte → drift
named; identity → Ok. WP06's T023 calls this against the committed file — the
function signature is the contract, note it in the module docs.

## Definition of Done

- [ ] `make webview-tokens` regenerates; second run produces byte-identical
      file (idempotent)
- [ ] `webview-page/tokens.css` committed, header marks it GENERATED
- [ ] Freshness function + unit tests green; all existing tests pass
- [ ] `spec-kitty agent tasks mark-status T014 T015 T016 --status done`

## Risks

- Name-transformation collisions (slash→dash must stay injective over the
  declared names — assert uniqueness in a test).
- Someone hand-edits tokens.css later: that is exactly what T016 exists to
  catch; do not weaken the byte-identity comparison.

## Reviewer Guidance

Reject if: any token value is typed by hand anywhere in the generator; the
transformation is ad-hoc per token rather than mechanical; freshness compares
parsed values instead of bytes; the Makefile target shells beyond the one
cargo invocation.
