---
affected_files: []
cycle_number: 6
mission_slug: functional-patch-editor-01KZERV9
reproduction_command:
reviewed_at: '2026-08-09T15:38:40Z'
reviewer_agent: unknown
verdict: rejected
wp_id: WP05
---

# WP05 review — cycle 3

**Verdict: reject.** All four named fixes are confirmed and none of them needs
to be revisited. The rejection is on one thing: **the honest-coverage statement
says the residue is three items and it is four**, and the fourth is a page-text
change that empties the PATCH strip with `cargo test --all-targets` green.

That statement is the artifact the accept gate will quote. Shipping it one item
short is F-68 a second time — a control stated more confidently than its
evidence — which is the exact failure the statement exists to prevent.

The fix is bounded, mechanical, in one file, and **I ran it green**. Section 2.

Everything below ran in an isolated copy of the lane at
`/private/tmp/.../scratchpad/lane`. The lane worktree was never mutated:
`webview-page/page.js` is `a4af8647…c76f812` at `b6232be^`, at `b6232be`, at
`HEAD` and in the working tree, and `git status --porcelain` shows only an
untracked `.spec-kitty/`.

## Confirmed — do not redo any of this

**1 — Whole-line matching. Both probes flip.**

| probe | cycle 2 | now |
| --- | --- | --- |
| `    return openSlot;` at the top of `stripGroupKey` | MISSED | **CAUGHT** |
| `    return UNAVAILABLE_MARK;` at the top of `controlValueText` | MISSED | **CAUGHT** |

And the widened pin did not become a pin that matches nothing in particular.
*The control identity read* still discriminates on its own site, twice over:
`? control.path.controlId.id` → `? String(control.path.controlId.id)` is CAUGHT,
and `: ""` → `: "?"` is CAUGHT. Widening it to the whole `return String(…);`
statement cost no discrimination.

**2 — The two unpinned copied rules.**

| probe | cycle 2 | now |
| --- | --- | --- |
| `startsWith` → `return false;` | MISSED, and `--all-targets` green in full | **CAUGHT** — *the prefix test* |
| `designedGroup`'s match → `if (false)` | MISSED | **CAUGHT** — *the declared group lookup, by key* |

**3 — The table's order, with membership intact.**

| probe | cycle 2 | now |
| --- | --- | --- |
| `envelope` and `capability` entries swapped | MISSED | **CAUGHT** |
| `AMP ENVELOPE` → `ENVELOPE` | CAUGHT | **CAUGHT** |
| `slot.2` `designed: true` → `false` | CAUGHT | **CAUGHT** |

One pin now carries what six carried, and carries more. Membership did not
regress.

**The arithmetic holds.** `required` went `[…; 63]` → `[…; 66]` (+ *the prefix
test*, + *the declared group lookup, by key*, + *the designed group table, in
declared order*); the six-entry `for (key, legend, designed) in
DESIGNED_STRIP_GROUPS` loop is gone and the return went from
`required.len() + DESIGNED_STRIP_GROUPS.len()` to `required.len()`. 63 + 6 = 69
before; 63 + 2 + 1 = 66 now. The decrease is a tightening, and saying so rather
than letting 69 → 66 read as a loss was right.

**4 — The docstring.** It now says the check is over the *set* of statements
each function contains and not the order they run in, names the moved
`visible`-skip as the mutation it does not reach, and says why that is inherent
(closing it means requiring each body to be a *sequence*, a different and much
larger control) rather than pending. It also adds the head-row-resolution limit
unprompted. Both of those are true: I reproduced the moved `visible` skip as
MISSED, and `controlById`'s identity match → `if (true)` as MISSED.

**The enforcement rules still fire.** Unpinned arm added to `stripGroupKey`:
CAUGHT, naming the line. `var spans = [];` deleted: CAUGHT, naming the stale
scaffolding entry. `"patch.envelope."` → `"patch.envelopeXX."`: CAUGHT, naming
*the envelope group's prefix*.

**The numbers, run rather than taken.** `cargo test --test
functional_patch_editor` exits 0, emits `page_rules_pinned=66` then
`CREST_ACCEPTANCE functional_patch_editor passed`, 30 passed / 0 failed.
`cargo test --all-targets --no-fail-fast` exits 0, 30 targets ok.
`cargo fmt --check` clean. `cargo clippy --all-targets --all-features` clean.
`spec-kitty crest-spec doctor` OK (7 contexts / 135 resources, 33 project
validations, 20 witnesses). `input_capture_witness` is flaky in this environment
rather than merely partial — one run emitted `CREST_KEY_WITNESS_PARTIAL` and
passed, one hard-failed at 43 of 46 transitions on the focus-loss edge. Same
F-40 cause, not WP05's file, not a blocker here.

**The two smaller items were both judged right.**

- `hint.contains(':')`. The reasoning is sound and I checked its premise. The
  painted line is `D:utility A / Return:return` and the hint half is `A /
  Return` — a hint contains spaces, so a space-split "every span is
  hint:label" structural check fails on a correct page. Deleting an assertion
  that cannot fail, and saying in its place where the colon rule *is* defended
  (*the hint pairs its label with a colon*, whose defeat in `page.js` fails the
  pin table), is a better outcome than replacing it with a second assertion that
  passes for a structural reason. The `:return` check four lines below survives
  and does discriminate: it is on the label half, through `hintLabel`.
- F-70 and `grouped_strip_shape`'s inert `patch.effect.` arm, noted not fixed.
  Right, and for the reason the note gives. There is no independent source for
  the painted bound to read it back against — `page.js` is never executed by
  this target — so strengthening the range half means inventing a rule, which is
  F-65's `HINT_SEPARATOR` one line over. The inert arm is dead by construction
  and the comment names both why it is written that way and what actually
  carries the nesting claim.

## The defect: the residue is four, not three

The coverage predicate matches a line against **a flat pool of every pin in the
file**, not against the pins that belong to the function being walked:

```rust
if pins
    .iter()
    .any(|(_, fragment)| fragment.lines().any(|pinned| pinned == code))
```

So a statement added to a walked function is admitted whenever its exact text —
indentation included — appears anywhere inside *any* pin, including a pin for a
different function entirely.

| # | mutation | result |
| --- | --- | --- |
| F1 | `    return null;` inserted as the first statement of `stripGroups` — the page arranges no groups at all | **MISSED** |
| F2 | `    return null;` inserted as the first statement of `controlValueText` — every row paints `null` | **MISSED** |
| F3 | `    return null;` inserted as the first statement of `controlIdOf` | **MISSED** |
| F4 | `designedGroup`'s whole five-line lookup body pasted into `stripGroups` | **MISSED** |

**None of `stripGroups`, `controlValueText` or `controlIdOf` contains
`    return null;`.** Each of F1–F3 is therefore a statement genuinely added to
that function's set — not a reordering, not a duplicate — admitted by a pin
belonging to `designedGroup`, `stripGroupKey` or `groupHeadControlId`. That is
not "the check is over the set of statements each function contains". It is
F-68's own mechanism, narrowed from *substring of any pin* to *exact line of any
pin*, and not closed. The file asserts one line above that an anchor matching
more than the one site it names is not a pin; the coverage predicate still
admits by unanchored line.

**F1 is the one that decides this.** `stripGroups` returning immediately empties
the PATCH strip — T030's whole claim — and I measured the consequence:
`cargo test --all-targets --no-fail-fast` **exits 0 with all 30 targets green**.
That is F-55's condition verbatim, the condition cycle 1 and cycle 2 were each
rejected for, still open on cycle 3. `webview_projection_shell` skips its DOM
layer headless, so nothing in the repository executes `page.js` — this target's
pins are the only guard there is, which is exactly why the guard has to hold.

### A fifth probe, which is genuinely inherent and must be *named*, not fixed

| # | mutation | result |
| --- | --- | --- |
| F5 | `    return null;` inserted as the first statement of `stripGroupKey` — which already contains that line | **MISSED**, and stays MISSED under the fix below |

This one is real set-based inherence: duplicating a statement the function
already has does not change the set. It is the twin of the order gap the
docstring names, it collapses grouping in one line exactly the way `return
openSlot;` did, and it is not named anywhere. The docstring's "the check is over
the *set* of statements" is technically already true of it, but a reader will
not derive it — the sentence names order and stops.

## What to do

Bounded, mechanical, all in `tests/functional_patch_editor.rs`. **Do not touch
`page.js`.** Do not re-run the sweep above — I have reproduced all of it.

1. **Scope the pin pool to the function being walked.** In
   `check_every_line_of_a_transcribed_page_rule_carries_a_pin`:

   ```rust
   for (page_function, twin) in TRANSCRIBED_WHOLE {
       let body = page_function_body(script, page_function);
       // Only pins that live in *this* function may admit its lines. A pin for
       // `designedGroup` admitting a line in `controlValueText` is an anchor
       // matching more than the one site it names — the thing the table above
       // asserts against, applied to the coverage predicate (F-68).
       let own_pins = pins
           .iter()
           .filter(|(_, fragment)| body.contains(*fragment))
           .collect::<Vec<_>>();
       for line in body.split('\n') {
   ```

   and match against `own_pins`.

2. **Skip punctuation-only lines.** Scoping makes closing braces fail, because
   they were being admitted cross-function. A brace is not a statement:

   ```rust
   || statement.chars().all(|c| "{}()[];,".contains(c))
   ```

3. **Name the two lines the loose predicate was hiding.** With 1 and 2 in place
   exactly two real gaps surface, both in `rangeHtml`, both scaffolding:
   `return (` and `"</span>"`. `SCAFFOLDING` goes 13 → 15. That these were the
   only two is the same point cycle 2 made about `controlIdOf`: the loose
   predicate was hiding real gaps while admitting fabricated ones.

4. **Say set-*multiplicity* alongside set-order in the docstring**, with F5 as
   its example — `    return null;` added to a `stripGroupKey` that already ends
   with it collapses the grouping rule and no set-based check can see it. Same
   register as the order paragraph: inherent, named, not implied away.

5. **Then the residue statement is true as written, at three items**, and it is
   the right statement. Do not add a fourth item to it — close the fourth
   instead. Under 1–4 I measured, in an isolated copy: target green,
   `page_rules_pinned=66` unchanged, **F1–F4 all flip to CAUGHT**, every probe
   that was CAUGHT above stays CAUGHT, and the only mutations still MISSED are
   set order (the moved `visible` skip), set multiplicity (F5), and
   `controlById`'s identity match — order/multiplicity, the unwalked function,
   and the equivalence gap. Exactly the three the statement names.

## For the accept gate

- **F-68 needs one more line**, recording that whole-line matching narrowed the
  admission surface from *substring of any pin* to *exact line of any pin* and
  did not close it, and that the closure is per-function pin scoping. Written
  without that, F-68 repeats F-64's error at one remove: a correction stated as
  complete when it was partial.
- Everything cycle 2 carried forward — F-69's named character, F-70's three
  tiers, F-71 — is discharged in this cycle and needs no further work.
- The live witness stays built-and-unrun (F-40, F-58) and is flaky rather than
  merely partial in an unfocused console; `functional_patch_editor` is the
  deterministic layer and stands on its own.
