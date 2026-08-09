# WP05 review — cycle 5

**Verdict: reject.** The six-bucket partition is real. I attacked it and could not
break it: the loop's control flow is complete, exhaustive, and measured. **That
half of cycle 5's claim survives in full and is the first claim in this chain that
did.**

The rejection is not another list. It is the two sentences cycle 5 added about the
*extraction*, one of which my mutation run falsifies outright:

> A *second declaration* of a walked function is caught, because the extraction
> requires the head to be unique

It is not caught. `  function controlIdOf (control) {` — one space before the
paren — is valid, hoisted JavaScript, is the declaration the page calls, and is
**MISSED**, with all thirty targets green. Two more spellings and one attack on
the other anchor are MISSED too. Measured, not reasoned.

This is the sixth link in F-74's chain and the first one written by a control that
had otherwise finished converging. Cycle 6 is the last cycle: make the two
sentences true, or retract them and name the class. Both end it.

Everything below ran in an isolated copy at `/private/tmp/.../scratchpad/lane`,
verified identical to `a01d48a` across `src/`, `tests/`, `webview-page/`,
`Cargo.toml`, `Cargo.lock` and `build.rs`. `page.js` is `a4af8647…` at `a01d48a`,
at `a01d48a^`, at `bf855d7` and in the lane worktree; the lane worktree's
`git status --porcelain` shows only an untracked `.spec-kitty/` before and after.

## 1 — Confirmed. Do not redo any of this.

### The six-bucket partition holds exactly

I did not take this on report and I did not re-run `collect.py`. I instrumented
the **committed** walk myself (`renata5_instrument.py`), printing one tagged line
per body line without altering control flow, and ran the target. Two identical
runs of 243 lines each:

| bucket | count |
| --- | --- |
| blank (`statement.is_empty()`) | 22 |
| line beginning `//` | **0** |
| punctuation-only | 47 |
| own pin | 145 |
| own declaration | 13 |
| NUL separator | 1 |
| own scaffolding | 15 |
| **total** | **243** |

Every count matches cycle 5's to the unit. More importantly the *partition* is
complete by construction, not by luck: the loop is a linear `if`/`continue` chain
terminating in the scaffolding assert, so those seven dispositions are its entire
control flow. There is no eighth path and no seventh bucket. **I looked for one
and there is nothing there to find.**

### The geometry holds

Twelve bodies, byte ranges from the running test:

```
controlIdOf  9726–9890    startsWith 10199–10282   hintLabel  13823–13976
hintRun     14640–15094   controlValueText 17301–19765   rangeEndpointText 29230–29373
rangeHtml   29551–30040   stripGroupKey 37385–37850   groupHeadControlId 38286–38534
designedGroup 39215–39432 stripGroups 39957–42087   sideRegionHintLine 52378–53533
```

Pairwise disjoint — every start exceeds the previous end. Walked line counts sum
to 243. Each head is asserted unique and each slice asserted brace-balanced to
+1; both assertions pass on the committed script. All **66** pins occur exactly
once script-wide (the `:1249` loop, and my own per-pin trace: 66 lines, every one
a count of 1). Fifty-four of the 66 fall inside a walked body; because pins occur
once and bodies are disjoint, each belongs to at most one — `body.contains`
partitions them cleanly, as claimed.

### The ` //` strip is safe, and I probed it rather than reading it

Twenty lines across the twelve bodies contain ` //`. My strip trace shows the
surviving prefix of each: nineteen are whole-line comments reduced to `""`, and
the twentieth is `        continue; // null hints never render (spike defect, kept
fixed)` in `hintRun`, which survives as `        continue;` and is admitted by
`hintRun`'s own pin *a null hint never renders*. No ` //` sits inside a string or
regex literal. The transform loses nothing.

One correction to the bucket table's reading, in the direction of claiming less:
the `starts_with("//")` filter fires on **zero** lines. The ` //` strip empties
whole-line comments before it ever sees them, so all nineteen land in the blank
bucket. The filter is inert in the strongest sense — it is unreachable, not merely
harmless. Worth one clause so the sentence does not imply it carries work.

### The residue on the canonical 19 is exactly as reported

Full harness re-run against `a01d48a`: **16 CAUGHT, 3 MISSED, zero `<<<
UNEXPECTED`.** The three are P9 (multiplicity), P12 (order), P16 (`controlById`).

### The seven probes flip, and the flip is measured against the alternative

All seven **CAUGHT** on the committed file: S5, N5, N6, F1, F2, T1, D1.

And I did what cycle 5 asks and verified its central claim about my predecessor's
proposal by building it: `variant_r.py` writes cycle-4 section 3a/3b verbatim over
`bf855d7`. It is green, 30 passed / 0 failed, `page_rules_pinned=66` — and all
**seven** probes are **MISSED** under it. Cycle 4's fix was insufficient and cycle
5 was right to measure it before landing anything. That was the correct instinct
and it is the reason this cycle's remaining defect is small.

### Gates, on the submitted state

- `cargo test --test functional_patch_editor` — **30 passed / 0 failed**,
  `page_rules_pinned=66`, `CREST_ACCEPTANCE functional_patch_editor passed`.
- `cargo test --all-targets --no-fail-fast` — **30/30 targets green, 0 failures.**
- `input_capture_witness` **passed in its `CREST_KEY_WITNESS_PARTIAL` form** in my
  run, where cycle 5 saw the hard failure. Both faces of F-75 are now on the
  record from the same commit in the same week. Not a blocker; see §6.

## 2 — The defect: both extraction anchors fall to a whitespace variant

Cycle 5's own framing is that the walk has two places outside the loop where it
can fail to *look* at a line, and that both are now asserted. Both assertions
exist. Neither closes what its sentence claims, because each recognises exactly
one spelling of the thing it forbids.

### 2a — The head anchor counts one spelling of "declaration" (three mutations)

```rust
let head = format!("\n  function {name}(");
let declarations = script.matches(&head).count();
assert_eq!(declarations, 1, ...);
```

A second declaration that is not spelled with exactly one space and no space
before the paren is neither counted nor found. JavaScript hoists both and the
later one wins.

| # | mutation | result |
| --- | --- | --- |
| D1 | `  function controlIdOf(control) {` appended (cycle 5's probe) | CAUGHT |
| D2 | `  function controlIdOf (control) {` — one space before `(` | **MISSED** |
| D3 | `  function  controlIdOf(control) {` — two spaces after `function` | **MISSED** |
| D4 | `  function stripGroupKey (id, openSlot) {` — same trick, different function | **MISSED** |

**D2 is the one that decides this, and I measured its consequence rather than
arguing it.** I confirmed the override in node (`function f(x)` then
`function f (x)` → the second wins), then ran the page's *own* `stripGroups`
against a five-row fixture, pristine and mutated:

```
PRISTINE : instrument×1  envelope×1  slot.0×2  capability×1  slot.1×0  slot.2×0
D2       : ?group×5 (unknown)  instrument×0  envelope×0  slot.0×0  slot.1×0  slot.2×0
```

Every row falls into the explicit unknown marker and every designed group empties.
That is T030's whole claim destroyed by a one-space edit. And **`cargo test
--all-targets --no-fail-fast` is green across all 30 targets** under it — F-55's
condition verbatim, the condition cycles 1 through 5 were each rejected for.

D4 shows this is not about `controlIdOf`. It is the anchor.

### 2b — The brace count is naive over text the walk itself erases (one mutation)

```rust
assert_eq!(body.matches('{').count(), body.matches('}').count() + 1, ...);
```

The docstring justifies the naive count with "no walked body holds a brace inside
a string or a regex literal today." A **comment** is neither — and comments are
the one kind of text this walk deletes before any table sees it. So a `}` typed
into a comment is invisible to every list and fully visible to the brace counter.

| # | mutation | result |
| --- | --- | --- |
| T1 | dedent an inner closer of `controlValueText` to column two, insert a rule past the cut (cycle 5's probe) | CAUGHT |
| T2 | the same cut, with `}` appended to the comment four lines above it | **MISSED** |

Under T2 the walk of `controlValueText` sees 14 lines instead of 56. Every pin it
would have checked is still present in the script, so the `:1249` table stays
satisfied; the inserted rule sits in the 42 unwalked lines. The mutated `page.js`
passes `node --check`, and I ran the function:

```
PRISTINE parameter row -> 0.500
T2       parameter row -> —
```

Every continuous, stepped, choice and toggle parameter on the PATCH surface paints
the unavailable mark. MISSED.

### Why this is one finding and not two

Both are the same shape, and it is F-74's shape reflected: **an assertion that
recognises fewer spellings than the property it names.** Every prior cycle closed
an admission pool matching *more* sites than it named. Cycle 5 closed the last of
those — I confirmed it — and in the same commit introduced the mirror image twice.

The sentence "Both were defeatable, so both are checked rather than assumed" is
true. The sentence "A *second declaration* of a walked function is caught" is
false, and it is the one being quoted forward.

## 3 — Ruling on R1: the classification is honest. Do not give it a fourth class.

Cycle 5 calls `controlIdOf = function (control) { return ""; };` at module scope
the same *class* as P16 rather than a fourth residue, and records it beside
`controlById` instead of letting "three" imply "three mutations exist". I
reproduced R1 (**MISSED**) and I agree with the call, for a reason cycle 5 could
not have had: my own D2/D3/D4 and T2 land in the same class. The class is
**anything that is not a line of one of the twelve slices**, and it now has four
measured instances, not two. A residue that absorbs new instances without
changing shape is a class. R1 does not need its own line — the class does.

This is also the first time in this chain someone declined to round a number to
make it tidy, and it is the reason I could tell the class from an item at all.
Keep the instinct; it is worth more than the fix.

What the classification does *not* license is the carve-out beside it. Saying
second declarations are caught while rebindings are not draws the class boundary
in the wrong place: both are outside the twelve slices, and neither is caught.

## 4 — The coverage statement, as it should be quoted

Every clause about the **loop** holds against my measurements and should be quoted
as written. Two clauses about the **extraction** do not. Replace the paragraph
beginning "The slice each walk is handed is asserted as well" and fold the
extraction residue into the unwalked class:

> **What this covers.** The check is over the *set* of statements each of twelve
> `page.js` bodies contains, as the extraction hands them over. Four tables admit
> a line, every one keyed on the function being walked and matched whole-line,
> indentation included: that function's own pins (145 lines), its own declarations
> (13), its own scaffolding (15), and — in `sideRegionHintLine` alone — F-43's NUL
> separator (1). Three text filters run ahead of them and admit nothing able to
> carry a rule: a blank line (22), a line beginning `//` (0 — the inline-comment
> strip empties whole-line comments before this sees them), and a line spelled
> entirely from `{}()[];,` (47). Those seven dispositions are the loop's complete
> control flow, measured by instrumenting it rather than by reading it: 243 lines
> walked, 243 accounted, no eighth path. So a statement added to a line the walk is
> handed fails here.
>
> A pinned statement *changed* fails too, but in the pin table this walk is handed
> and not in this walk — that table is what `page_rules_pinned` counts, and it is
> where P3, P3b, P4, P5 and P15 panic.
>
> **What the walk is handed is bounded by two anchors, and they are heuristics.**
> `page_function_body` asserts the head occurs once and the slice holds exactly one
> unclosed brace. [Then either: each assertion now normalises whitespace / strips
> comments before it counts, so it recognises the property and not one spelling of
> it — **or**, if that is not what the code does: each assertion recognises one
> spelling of what it forbids. The head count matches `"\n  function NAME("`, so a
> second declaration written `function controlIdOf (control)` is uncounted and
> unfound; the brace count is naive over the raw slice, and the walk erases comments
> before any table sees them, so a `}` in a comment rebalances a truncated slice.
> Both measured MISSED.]
>
> **Not covered — the set's order, and its multiplicity.** [unchanged; both
> reproduced MISSED]
>
> **Not covered — anything that is not a line of one of the twelve slices.** One
> class, four measured instances: `controlById`'s identity match, which this file
> does not transcribe; `controlIdOf = function (control) { return ""; };` at module
> scope, a rebinding rather than a declaration; a second declaration the head anchor
> does not recognise; and, past a defeated brace anchor, every line after the cut.
> Closing it means walking the file with a parser rather than twelve `str::find`
> slices — a different and much larger control.
>
> **Not covered, and never was — that the Rust computes what the page computes.**
> [unchanged]

## 5 — What to do, and the decision rule

**All in `tests/functional_patch_editor.rs`. Do not touch `page.js`.** Do not
re-run §1 — I have reproduced all of it.

The rejection ground is the false sentence, not a missing feature. Two ways to
land, and **either is approvable**:

1. **Make it true.** The head fix is proportionate and closes a class rather than
   a spelling — count occurrences of `function` + whitespace + `name` +
   whitespace + `(`, roughly:

   ```rust
   let declarations = script
       .match_indices("function")
       .filter(|(i, _)| {
           script[i + "function".len()..]
               .trim_start()
               .strip_prefix(name)
               .is_some_and(|rest| rest.trim_start().starts_with('('))
       })
       .count();
   ```

   (`strip_prefix` then requiring `(` is what keeps `startsWith` from matching a
   longer name.) For the brace anchor, count on the comment-stripped body — three
   lines, and it closes T2 while leaving the string/regex case the docstring
   already names.

2. **Retract it.** If either fix grows past ~15 lines or starts wanting a parser,
   **stop and delete the claim instead.** Write the bracketed second branch in §4
   and record the anchors as heuristics. That is not a lesser outcome.

**Do not add a seventh bucket, a fifth table, or a fourth residue class.** The
loop is finished. I tried to break it and could not.

## 6 — For the accept gate

- **F-72 has not been amended for cycle 5 at all.** It still reads "the residue is
  three items" and does not mention R1, which cycle 5 measured and named in code.
  It needs the class/instance distinction from §3 and this cycle's four instances.
- **F-74 needs a sixth link**, and it is the reviewer's turn: cycle 5's enumeration
  of the *loop* was the first in this chain that survived independent attack — the
  partition is complete and I verified all seven counts — and in the same commit
  its two sentences about the *extraction* asserted a property each check does not
  have. The chain's shape inverted at the last link: five overclaims about pools
  matching too much, then one about assertions recognising too little.
- **One arithmetic correction to cycle 5's narration.** "29 of 161 checked lines"
  mixes two denominators. On the committed file the `checked` counter returns
  **174** (13 declaration + 145 pin + 1 NUL + 15 scaffolding). 161 is the
  *pre-fix* figure — at `bf855d7`, declarations were absorbed by the
  `starts_with("function ")` skip ahead of `checked += 1`. The honest phrasing is
  "29 of 174", or "145 of 174 admitted by pin, 29 by the three keyed tables". The
  numerator is post-fix and the denominator is pre-fix: a small instance of the
  same habit, carried from the enumeration in front of its author.
- **F-75 now has both faces from one commit.** Cycle 5 saw
  `input_capture_witness` hard-fail; my `--all-targets` run saw it emit
  `CREST_KEY_WITNESS_PARTIAL` and pass. Same F-40 cause, not WP05's file, not a
  blocker — and direct evidence for F-75's point that a green run of that test is
  not evidence the environment was healthy.

## 7 — Does another cycle converge?

**The lists have converged. The extraction will not, and its residue should be
accepted and named.**

The five prior rejections each closed an admission pool that matched more sites
than it named, and each closure strictly increased what the check catches. That
work is done: the loop's partition is complete, exhaustive, and I could not find a
line it fails to classify.

The extraction is a different kind of thing. It locates JavaScript with
`str::find` over literal strings, and a literal string will always admit a variant
spelling. §5's head fix closes a real class and is worth doing once. After that,
the honest position is that the anchors are heuristics with a named blind spot —
not that the next spelling is one cycle away.

So: one cycle, bounded to §5 and §4, and no more notches after it. If the fixes
resist, the statement alone is enough.
