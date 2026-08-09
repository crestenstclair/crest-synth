# WP05 review — cycle 4

**Verdict: reject.** Every claim cycle 4 makes about its own work is true and I
reproduced all of it. The rejection is on one thing, and it is the same thing as
last cycle: **the coverage statement says the residue is three and it is five.**

Two more mutations reach through it, both measured, both the same shape as F-74
— an admission pool matching more than the one site it names — moved off `pins`
onto the *other two* admission paths, which are still flat and global.

The statement's wording is right. I am not asking for a reword. **The code is
one notch short of the sentence already written**, and closing it leaves the
residue at exactly the three items the sentence names. Section 3 — I ran it
green.

Everything below ran in an isolated copy at `/private/tmp/.../scratchpad/lane`,
verified byte-identical to `bf855d7` (test file and `page.js`, plus a whole-tree
`diff -rq` that differs only in an unrelated `cross-wp-findings.md`). The lane
worktree was never mutated: `page.js` is `816ce494…` at `bf855d7^`, `bf855d7`,
`HEAD` and in the working tree; `git status --porcelain` shows only an untracked
`.spec-kitty/`.

## 1 — Confirmed. Do not redo any of this.

**The four probes flipped, and the flip is measured against the unfixed file,
not inherited.** I ran both halves myself: the four probes against
`bf855d7^:tests/functional_patch_editor.rs`, then against `bf855d7`.

| probe | at `bf855d7^` | at `bf855d7` |
| --- | --- | --- |
| `    return null;` atop `stripGroups` | MISSED | **CAUGHT** |
| `    return null;` atop `controlValueText` | MISSED | **CAUGHT** |
| `    return null;` atop `controlIdOf` | MISSED | **CAUGHT** |
| `designedGroup`'s lookup body pasted into `stripGroups` | MISSED | **CAUGHT** |

All four CAUGHT at `tests/functional_patch_editor.rs:1458` — the coverage
predicate, not something incidental — and each names the function and the added
line:

```
webview-page/page.js stripGroups line "return null;" is transcribed by
page_strip_groups and no pin covers it — pin the statement, or name it as
scaffolding that carries no rule
```

`controlValueText`/`page_value_text`, `controlIdOf`/`page_control_id`, and for
the pasted body `stripGroups line "for (var i = 0; i < DESIGNED_STRIP_GROUPS.length; i += 1) {"`.
Right site, right reason.

**19/19 matched, and the three MISSED are the three named.** I re-ran the full
harness against `bf855d7`. 16 CAUGHT, 3 MISSED, no `<<< UNEXPECTED`. The three
are `P9` (multiplicity — `    return null;` atop `stripGroupKey`), `P12` (order —
the moved `visible` skip), `P16` (`controlById`'s identity match). All twelve
previously-CAUGHT probes are still CAUGHT.

**The scaffolding additions are grounded, and neither is broader than the run
shows.** I did not take this on report. I patched a scratch copy to collect
instead of assert with `SCAFFOLDING` rolled back to its 13 pre-fix entries and
the scoping in place. It printed exactly two lines and nothing else:

```
RENATA_UNCOVERED rangeHtml :: "return (" [twin page_range_text]
RENATA_UNCOVERED rangeHtml :: "\"</span>\"" [twin page_range_text]
```

The two added entries are those two strings verbatim. The provenance claim also
holds: the pin at `:1237` (*the panel's hint line is its hint run*) contains both
lines at the same indentation, which is what admitted them before scoping.
`SCAFFOLDING` is 15 — confirmed by execution, not by reading: `P14` fails with
`left: 14, right: 15`.

**`page_rules_pinned` stayed 66.** Emitted on every green run I did.

Gates on the submitted state: `cargo fmt --check` clean, `cargo clippy
--all-targets --all-features` clean, target 30 passed / 0 failed.

The punctuation skip is also fine, and provably narrow: it drops only lines whose
every character is in `{}()[];,`, which cannot spell a statement.

## 2 — The defect: the residue is five, not three

Scoping closed the flat pool on `pins`. It left the other two admission paths
flat. Both admit a line at a site they do not name, which is F-74's sentence
verbatim.

### 2a — `SCAFFOLDING` is matched globally (three mutations)

```rust
assert!(SCAFFOLDING.contains(&statement), ...)
```

No `page_function` anywhere in that predicate. Each of the 15 entries occurs in
exactly one walked function — I measured the mapping — but any of them is
admitted in **all twelve**.

| # | mutation | result |
| --- | --- | --- |
| S1 | `    return groups;` inserted as the first statement of `controlValueText` | **MISSED** |
| S2 | the same line atop `controlIdOf` | **MISSED** |
| S3 | the same line atop `stripGroupKey` | **MISSED** |

`return groups;` occurs only in `stripGroups`. None of these three functions
contains it, so each is a statement genuinely *added* to that function's set —
not a duplicate, not a reordering, and the function is walked. None of the three
named residues covers it.

**S1 is the one that decides this.** `groups` is function-local to `stripGroups`
(no module-level declaration — I checked), so `controlValueText` throws a
ReferenceError on every parameter row and the PATCH page cannot render at all. I
measured the consequence: `cargo test --all-targets --no-fail-fast` across all 30
targets, **every target green except `input_capture_witness`**, which is the
known F-12/F-75 flake and unrelated to `page.js`. That is F-55's condition
verbatim — the condition cycles 1, 2 and 3 were each rejected for.

The `SCAFFOLDING` doc comment says "Every entry is asserted below to still occur,
so this list cannot rot into a permit." The assertion is `used.len() ==
SCAFFOLDING.len()`, which checks each entry occurs *somewhere*. It does not check
it is admitted only where it belongs, which is what "permit" means here. A
*substitution* is caught by it (the displaced entry stops occurring); an
*insertion* is not.

### 2b — `NUL_SEPARATOR_LINE` is matched by prefix (one mutation)

```rust
const NUL_SEPARATOR_LINE: &str = "var key = String(action.hint) +";
...
if statement.starts_with(NUL_SEPARATOR_LINE) { continue; }
```

Everything after the `+` is unchecked, in any walked function.

| # | mutation | result |
| --- | --- | --- |
| N2 | `String(action.hint) + "\0" + String(action.label)` → `+ "" +` | **MISSED** |

That is F-43's rule itself. With the separator emptied, `hint="AB", label="C"`
and `hint="A", label="BC"` produce the same dedup key and one hint is silently
dropped from the utility line — exactly the collision the NUL exists to prevent.
The two flanking pins do not see it: dropping the label (`N1`) and replacing the
whole line (`N3`) are both CAUGHT at `:1252`; changing only the separator is not.

The comment explains *why* the NUL is unpinned (the merge step's repair), and
that reasoning was sound when the constant was written. It is not a reason the
content after the prefix has to stay unchecked — see 3b.

### Why this is the same finding and not a new one

The statement says the check matches "whole-line against that function's **own**
pins", and then closes its exception list at three. Both halves point at `pins`.
The reader is told the scoping fix made the claim safe; two sibling admission
paths were not scoped, and both are reachable. Written as it stands, it is F-72
repeating F-68's error at one remove — a correction stated as complete when it
was partial — which is the specific thing F-74 says this chain has now cost four
cycles.

## 3 — What to do. Both fixes are proven, not proposed.

All in `tests/functional_patch_editor.rs`. **Do not touch `page.js`.** Do not
re-run the sweep in section 1 — I have reproduced all of it.

### 3a — Scope `SCAFFOLDING` per function

Make it a table of `(page_function, line)` and match on both. The pairing is
measured, not guessed: I emptied `SCAFFOLDING` in a scratch copy and collected
what fell through per function. It is exactly 15 pairs, one function each:

```rust
const SCAFFOLDING: [(&str, &str); 15] = [
    ("stripGroups", "var groups = [];"),
    ("stripGroups", "var byKey = {};"),
    ("hintRun", "var spans = [];"),
    ("sideRegionHintLine", "var actions = [];"),
    ("sideRegionHintLine", "var seen = {};"),
    ("stripGroups", "var control = controls[i];"),
    ("sideRegionHintLine", "var action = valid[a];"),
    ("hintRun", "var action = actions[i];"),
    ("hintRun", "spans.push("),
    ("stripGroups", "return groups;"),
    ("stripGroups", "for (var i = 0; i < controls.length; i += 1) {"),
    ("hintRun", "for (var i = 0; i < actions.length; i += 1) {"),
    ("sideRegionHintLine", "for (var a = 0; a < valid.length; a += 1) {"),
    ("rangeHtml", "return ("),
    ("rangeHtml", "\"</span>\""),
];
```

```rust
assert!(
    SCAFFOLDING
        .iter()
        .any(|(owner, line)| *owner == page_function && *line == statement),
    ...
);
used.insert((page_function, statement));
```

and the anti-rot assertion filters on `|pair| !used.contains(*pair)`, which makes
it per-site and strictly stronger than it is now. (Two entries need `cargo fmt`
to wrap.)

This also lets the comment say something truer than "carries no rule": each of
these lines carries no rule **in the function it belongs to**, which is the
actual justification.

### 3b — Match the NUL separator whole

The merge-repair concern is about a literal NUL byte in the `.rs` file. A
`\u{0}` escape avoids it entirely and is not repaired:

```rust
const NUL_SEPARATOR_LINE: &str =
    "var key = String(action.hint) + \"\u{0}\" + String(action.label);";
...
if statement == NUL_SEPARATOR_LINE {
```

Keep the comment's explanation of why this line is admitted separately; only the
match width changes, from prefix to whole line.

### 3c — Then the statement is true as written

**Do not reword it, and do not add a fourth item.** Under 3a and 3b I measured,
in an isolated copy:

- target green, 30 passed / 0 failed;
- `page_rules_pinned` **unchanged at 66**;
- S1, S2, S3, N2 all flip to **CAUGHT**, each naming the function and the line
  (`stripGroupKey line "return groups;" is transcribed by page_strip_group_key
  and no pin covers it`);
- N1 and N3 stay CAUGHT;
- the full 19 keep every verdict — 16 CAUGHT, 3 MISSED, no `<<< UNEXPECTED`;
- **the only mutations still MISSED are P9 (multiplicity), P12 (order) and P16
  (`controlById`)** — exactly the three the statement names.

The residue is three once the code matches the sentence. That is why this is a
rejection on the code and not on the prose.

## 4 — Wording, once 3a/3b are in

Two small things, both in the direction of claiming less:

- "matched whole-line against that function's **own** pins" should say **own pins
  and own scaffolding**, since after 3a that is what it does, and the current
  phrasing is what made the scaffolding path easy to miss.
- "a statement added to a walked function, **or a pinned statement changed**,
  fails here." The changed-pin case is real but it fails at `:1252`, in the pin
  table, not in this walk — P3, P3b, P4, P5 and P15 all panic there. "fails here"
  on this function's docstring attributes to the walk what the table does. Either
  say "fails this target" or name the table. Minor, but this statement is being
  quoted verbatim and it should not need a reader to know which assertion fired.

## 5 — For the accept gate

- **F-74 needs one more line**, recording that per-function scoping closed the
  pin pool and left the *other two* admission paths — `SCAFFOLDING` (global) and
  `NUL_SEPARATOR_LINE` (prefix) — open, both reachable, both closed by the same
  move. Written without it, F-74's "the fix is proven … exactly three MISSED
  remain" is the fourth instance of the overclaim it is about.
- Once 3a/3b land, F-72's "the residue is three items" is accurate for the first
  time and can be quoted as written.
- `input_capture_witness` failed in my `--all-targets` run in its hard-failure
  form. F-12/F-75, F-40 cause, not WP05's file, not a blocker.
