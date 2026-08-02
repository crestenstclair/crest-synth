---
affected_files:
- src/shell/visual/primitives/mod.rs
cycle_number: 1
mission_slug: crest-component-foundations-01KZ02H2
reproduction_command: cargo test --lib shell::visual::primitives
reviewed_at: '2026-08-02T05:30:00Z'
reviewer_agent: claude
verdict: rejected
wp_id: WP03
---

# WP03 Review — Cycle 1

**Verdict**: Changes requested. One blocking finding.

Reviewed `kitty/mission-crest-component-foundations-01KZ02H2..kitty/mission-crest-component-foundations-01KZ02H2-lane-c`,
own commit `0e9e473`. Verification re-run in the lane worktree: `cargo test --lib` 555/555,
`cargo test --lib shell::visual` 88/88, `make lint` clean, `make fmt-check` clean. The Activity Log's
numbers reproduce.

The work is strong. The seven families are all present, every primitive takes immutable data plus one
explicit `ComponentState` and paints, the state treatment is centralised in `text::resolved_color` instead
of branching per caller, and the four deliberate deviations recorded in the Activity Log are each the right
call — in particular painting `Locked` for `Disabled`, which the prompt's T017 note would have left as muted
colour alone in violation of FR-005. The halo is a faithful `epaint::Shadow` mapping with the derivation in
a doc comment, and its test unmultiplies back to the authored accent rather than asserting a magic byte.

## Blocking

### R1 — the T019 ownership guard is defeated by an inline path, and its doc comment claims it is not

`no_primitive_imports_anything_outside_the_vocabulary_egui_and_std`
(`src/shell/visual/primitives/mod.rs:96-114`) only inspects lines whose trimmed form starts with `use `.
A primitive that reaches application state through a fully-qualified inline path never writes such a line,
so the guard passes green.

Verified by mutation. Adding this to `value.rs` — no `use` line, nothing else changed:

```rust
pub fn peek_at_app_state() -> bool {
    std::mem::size_of::<crate::control::AppState>() > 0
}
```

`cargo test --lib shell::visual::primitives` → **47 passed, 0 failed**. It compiles, it reaches `AppState`
from inside a primitive, and every T019 guard is green.

The defect is not the narrow scope by itself — the prompt's T019 note explicitly scopes this WP to "the
narrower import claim" and leaves the full literal-absence guard to WP06. The defect is that the module
documents a stronger claim than it proves. `mod.rs:93-95` reads:

> Deliberately crude: it reads the source text. A primitive cannot reach `AppState`, the reducer, a Patch,
> focus state, or audio state without naming a path this rejects.

That sentence is false, and the same overstatement is in the commit message ("the T019 tests enforce that
structurally") and in the module-root doc at `mod.rs:15-18`. The Activity Log's mutation evidence for this
guard is `use crate::control::AppState;` — the `use` form only. In a mission whose charter demands measured,
falsifiable proofs and where FR-009 is the headline invariant, a guard whose documentation asserts coverage
it does not have is worse than a guard that states its own limits: WP06's author will read that comment and
conclude the case is already closed.

**Fix — take either path, the first preferred:**

1. **Tighten the guard**, which is what T019 step 2 literally suggested ("read the source files and assert
   no forbidden path appears"). Scan the whole source text, not just `use` lines, for the crate's
   non-visual roots. From `src/lib.rs` those are `adapter`, `control`, `kernel`, `mixer`, `real_time`,
   `synth`, `testing`:

   ```rust
   const FORBIDDEN_PATHS: [&str; 7] = [
       "crate::adapter", "crate::control", "crate::kernel", "crate::mixer",
       "crate::real_time", "crate::synth", "crate::testing",
   ];
   ```

   Assert none appears anywhere in any primitive source. Keep the existing `use`-line allowlist alongside
   it; the two catch different things. Mutation-verify the new form with the inline snippet above, not with
   a `use` line.

2. Or **correct the claims** at `mod.rs:15-18` and `mod.rs:93-95` to say exactly what is checked — import
   statements — and name the inline-path gap as the thing WP06 closes.

Either way the doc and the guard must agree.

## Non-blocking notes

### N1 — `Selected` is colour-only unless the composer calls both entry points

`paint_row_fill` and `paint_status_mark` are independent (`status.rs:146`, `status.rs:158`). A caller that
paints the fill and not the mark produces a row whose only signal is `bg/selected` — a colour that
`token.rs`'s own test records as sharing its value with `border/default`. The module's claim that "every
state that carries a status carries text or shape" holds of `status_mark`, not of the paint path.

This is correctly out of WP03's scope: where the mark column sits is a composition decision. Recording it
so it is not lost — **WP04 and WP05 reviewers should require that every `paint_row_fill` call site also
paints the mark**, and WP06's non-colour-legibility proof should assert it at the composed level rather
than at `status_mark`.

### N2 — the wildcard guard misses a named catch-all arm

`no_match_arm_falls_through_a_wildcard` (`mod.rs:143-159`) searches for the literals `_ =>` and `.. =>`.
An arm written `other => …` is a catch-all that binds by name; it defeats the closed vocabulary identically
and passes this scan. If you take fix path 1 above, consider also putting
`#![deny(clippy::wildcard_enum_match_arm)]` at the module root — that moves the check from text matching to
the compiler and covers both forms. Not required this cycle.

### N3 — two `DESIGN.md` line citations are off by one

`rules.rs:4` cites `DESIGN.md:447` for "separators are hairlines, never cards"; the sentence is at 448, and
was at 448 before this mission too. `focus.rs:16` cites `DESIGN.md:575` for the text-or-shape requirement;
it is at 576, having shifted when WP01 inserted the `selected` colour row at line 540 in this same lane.
`DESIGN.md:454`, `:468`, and `:512` all resolve correctly. Line-number citations rot; consider quoting the
anchor phrase alongside the number so a future shift is recoverable.

### N4 — no production caller yet (same posture as WP01 N1)

`grep -rn "visual::primitives\|primitives::" src --include="*.rs"` returns nothing outside the module. By
the letter of anti-pattern item 1 this is dead code. Not treated as a FAIL: `tasks.md`'s dependency graph
makes WP04 the consumer and sequences WP03 → WP04 deliberately, and `plan.md` IC-05 names this concern the
piece later screens compose from. Same standing condition WP01 carries — **this stops being acceptable if
WP04 lands without converting `paint_context_line` (`src/adapter/eframe_graphical_window.rs:287`) and the
footer (`:572`) onto these primitives.** WP04's reviewer should re-run this grep and require hits.

## Anti-pattern checklist

| # | Item | Verdict | Note |
|---|------|---------|------|
| 1 | Dead code | **PASS** (conditional) | Zero production callers; deliberate per the dependency graph. See N4. |
| 2 | Synthetic-fixture test | **PASS** | Every assertion runs production functions — `status_mark`, `frames`, `halo`, `hint_line`, `text_format`, `resolved_color`, `value_color`. Deleting the implementations breaks them. |
| 3 | Silent empty return | **PASS** | Five hits, all documented and defended. `focus.rs:122,143` and `status.rs:165` are "this state paints nothing", which is the declared contract. `status.rs:195`'s `""` is unreachable and `every_state_routed_to_the_declared_word_declares_one` holds that. `status.rs:121`'s `None` is the three states that carry no status, asserted by `rest_focus_and_adjustment_carry_no_status_mark`. |
| 4 | FR coverage | **PASS** | FR-004: seven families present, each with tests. FR-005: all eight non-resting states carry text or shape, asserted per state and module-wide. FR-009: asserted, but the assertion is narrower than documented — see R1. |
| 5 | Frozen surface | **PASS** | `git log --oneline <base>..HEAD -- src/shell/visual/mod.rs` is empty for this WP's own commit; the promotion to `primitives/mod.rs` needed no re-export, as the Activity Log claims. |
| 6 | Locked decision | **PASS** | No primitive imports `AppState`, the reducer, Patch, focus, or audio types. Every `match` on `ComponentState` in the diff is exhaustive and explicit. `value.rs` formats nothing. |
| 7 | Shared-file ownership | **PASS** | Own commit touches only `src/shell/visual/primitives/**` plus the deletion of the `primitives.rs` stub. `85eba12` correctly reverted the task-file edit off the lane branch. |
| 8 | Production fragility | **PASS** | No `raise`/`panic!`/`unwrap` in any production path. The `expect` calls are all inside `#[cfg(test)]`. |

## What to do

Fix R1, re-run `cargo test --lib shell::visual::primitives` and `make lint`, and mutation-verify the
tightened guard against the inline snippet in R1 rather than a `use` line. N1–N4 need no action in this WP;
N1 and N4 carry forward as conditions on WP04.
