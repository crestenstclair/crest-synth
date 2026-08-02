---
affected_files: []
cycle_number: 3
mission_slug: crest-component-foundations-01KZ02H2
reproduction_command: cargo test --lib shell::visual::primitives
reviewed_at: '2026-08-02T06:05:00Z'
reviewer_agent: claude
verdict: approved
wp_id: WP03
---

# WP03 Review — Cycle 3

**Verdict**: Approved.

Reviewed `kitty/mission-crest-component-foundations-01KZ02H2..kitty/mission-crest-component-foundations-01KZ02H2-lane-c`
in the lane worktree. The only change since cycle 1 is `48261a4`, which touches
`src/shell/visual/primitives/mod.rs` and nothing else.

## R1 — closed, and verified by mutation here rather than taken on report

The implementer took the preferred path 1: tighten the guard, then make the doc match.

`no_primitive_names_a_path_outside_the_visual_vocabulary` (`mod.rs:198-216`) scans the whole source
text of every primitive for the seven non-visual crate roots in `src/lib.rs` — `adapter`, `control`,
`kernel`, `mixer`, `real_time`, `synth`, `testing` — in both the `crate::` and `crest_synth::`
spellings, with needles assembled at runtime so `mod.rs` does not match itself. The `use`-line check
stays alongside it and its doc now says why the two are not redundant: a fixed forbidden-path list
can never name a third-party crate the module has no business importing.

Independently re-verified in the lane worktree, not accepted from the Activity Log:

1. The exact snippet from cycle 1's R1 appended to `value.rs` — `peek_at_app_state` reaching
   `AppState` through `std::mem::size_of::<crate::control::AppState>()`, no `use` line —
   → `no_primitive_names_a_path_outside_the_visual_vocabulary` **FAILED**:
   `value.rs names crate::control, which is application state reaching into a component`.
   Restored; `git status --porcelain src/` empty.
2. The relative twin — `size_of::<super::super::super::super::control::AppState>()` —
   → `no_primitive_walks_out_of_the_vocabulary_by_a_relative_path` **FAILED**:
   `value.rs climbs out of the visual vocabulary by a relative path`. Restored; tree clean.

The relative guard was not asked for by the review. It is the right call and not scope creep: closing
the absolute spelling while leaving the relative one open would have reproduced R1's actual defect —
a guard documented as stronger than it is. Two `super::` steps is the deepest the module legitimately
uses (`value.rs:67`, out of a `#[cfg(test)]` module into a sibling), so nothing real is forbidden.

The doc no longer overstates. `mod.rs:16-30` enumerates the four checks and names the residual gap in
the same breath: the scans are textual, so they prove these paths are not *named* here; application
state arriving through an argument type re-exported from the vocabulary is WP06's literal-absence
proof, not this module's claim. The `crest_synth::` spelling is explicitly carried as future-proofing
rather than claimed as proven — the package name does not resolve inside its own lib. That is the
honest posture cycle 1 asked for.

## Verification re-run

| Command | Result |
|---|---|
| `cargo test --lib` | 557 passed, 0 failed |
| `cargo test --lib shell::visual` | 90 passed, 0 failed |
| `make lint` (clippy `--all-targets -D warnings`) | clean |
| `make fmt-check` | clean |

The Activity Log's numbers reproduce exactly. The one pre-existing baseline failure
(`<declared-command>`) is unrelated and not touched by this WP.

## Anti-pattern checklist

| # | Item | Verdict | Note |
|---|------|---------|------|
| 1 | Dead code | **PASS** (conditional) | `grep -rn "visual::primitives\|primitives::" src --include="*.rs"` returns nothing outside the module. Deliberate per `tasks.md`'s WP03 → WP04 sequencing and `plan.md` IC-05. See N4. |
| 2 | Synthetic-fixture test | **PASS** | Every assertion runs production functions — `status_mark`, `frames`, `draws_cursor`, `halo`, `hint_line`, `text_format`, `resolved_color`, `value_color`, `RuleSpan::rect`. Deleting an implementation breaks them. |
| 3 | Silent empty return | **PASS** | All early returns are the declared "this state paints nothing" contract (`focus.rs:122,143`, `status.rs:165`) or an unreachable branch held by a test (`status.rs:195` / `every_state_routed_to_the_declared_word_declares_one`, `status.rs:121` / `rest_focus_and_adjustment_carry_no_status_mark`). |
| 4 | FR coverage | **PASS** | FR-004: seven families present, each tested. FR-005: every non-resting state carries text or shape, asserted per state and module-wide. FR-009: now asserted at the strength the doc claims — this was the cycle-1 blocker. |
| 5 | Frozen surface | **PASS** | `git log --oneline <base>..HEAD -- src/shell/visual/mod.rs` shows only WP01's and WP02's commits; WP03 needed no re-export, as the Activity Log claims. |
| 6 | Locked decision | **PASS** | No primitive imports or names `AppState`, the reducer, Patch, focus, or audio types. Every `match` on `ComponentState` is exhaustive and explicit; a grep for named catch-all arms across the module returns nothing. `value.rs` formats nothing. |
| 7 | Shared-file ownership | **PASS** | WP03's own commits touch `src/shell/visual/primitives/**` and the `primitives.rs` stub deletion only; `85eba12` correctly reverted the task-file edit off the lane branch. `48261a4` is `mod.rs` alone. |
| 8 | Production fragility | **PASS** | No `panic!`/`unwrap` in any production path; all eight `expect` calls are inside `#[cfg(test)] mod tests`. |

## Carried forward — non-blocking, no action in this WP

- **N1** — `paint_row_fill` and `paint_status_mark` are independent entry points (`status.rs:146,158`).
  A composer that paints the fill without the mark leaves `Selected` as color alone. **WP04 and WP05
  reviewers must require that every `paint_row_fill` call site also paints the mark**, and WP06's
  non-color-legibility proof should assert it at the composed level.
- **N2** — a named catch-all arm (`other => …`) still defeats `no_match_arm_falls_through_a_wildcard`.
  None exists today. The implementer declined `#![deny(clippy::wildcard_enum_match_arm)]` this cycle
  because it is a restriction lint that fires on every non-exhaustive match on every enum including
  egui's — correct call for a rejection-fix cycle. **WP06 closes this.**
- **N3** — two `DESIGN.md` citations remain off by one: `rules.rs:4` cites `:447` for "Separators are
  hairlines" (line 448); `focus.rs:16` cites `:575` for the text-or-shape requirement (line 576).
  Cosmetic. Worth quoting the anchor phrase alongside the number when either file is next touched.
- **N4** — zero production callers, as above. **This stops being acceptable if WP04 lands without
  converting `paint_context_line` (`src/adapter/eframe_graphical_window.rs:287`) and the footer
  (`:572`) onto these primitives.** WP04's reviewer should re-run the grep and require hits.

## Note on the review artifacts

`review-cycle-2.md` is a tooling artifact: it carries a cycle-2 frontmatter block prepended to a
verbatim copy of `review-cycle-1.md`, including that file's own frontmatter. It records no findings
of its own. R1 was raised once, in cycle 1, and is closed here.
