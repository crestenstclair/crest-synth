---
affected_files: []
cycle_number: 4
mission_slug: functional-patch-editor-01KZERV9
reproduction_command:
reviewed_at: '2026-08-09T14:30:24Z'
reviewer_agent: unknown
verdict: rejected
wp_id: WP05
---

# WP05 review — cycle 2

**Verdict: reject.** One defect with three mechanical parts, all inside
`tests/functional_patch_editor.rs`. Everything this package claims was
reproduced and holds; the rejection is about what the new enforcement rules
**do not** reach, which is what the ruling now rests on.

I ran 53 distinct mutations of `webview-page/page.js`, restoring it and
verifying its SHA-256 after every one. 46 CAUGHT, 7 MISSED. All of this ran in
an isolated copy of the lane; the lane worktree was never mutated.

## Confirmed — do not redo any of this

**The nine mutations from cycle 1 are all CAUGHT, each naming its own rule and
fragment.** Not "the test failed" — the transcription's own sentence:

| # | mutation | now |
|---|---|---|
| 1 | `stripGroupKey` `"patch.envelope."` → `"patch.envelopeXX."` | CAUGHT — *the envelope group's prefix* |
| 2 | `patch.effect.` arm returns `null` | CAUGHT — *the occupant joins its open slot* |
| 3 | `if (!control.visible) continue` neutralized | CAUGHT — *an invisible row is not arranged* |
| 4 | `groupHeadControlId` `slot.N` → `patch.engine` | CAUGHT — *the slot group head* |
| 5 | `openSlot = stripGroupKey(id, null)` → `null` | CAUGHT — *the open slot is carried from the occupancy row* |
| 6 | unknown identities dropped | CAUGHT — *an unclaimed identity joins the explicit unknown group* |
| 7 | `rangeEndpointText` `toFixed(3)` → `(1)` | CAUGHT — *the endpoint's continuous three places* |
| 8 | toggle `"ON"/"OFF"` → `"TRUE"/"FALSE"` | CAUGHT — *the toggle wording* |
| 9 | `data-role="row-range"` renamed at the **painting site only** | CAUGHT — *the painted range span* |

**A 34-mutation sample across every pin group: 34/34 CAUGHT.** `controlIdOf`
field, absent-value guard, scalar round/trunc split, continuous precision, both
F-33 label reads, `?`-marker, asset locator, identity/summary arm, range guard,
`RANGE_SEPARATOR`, `UNAVAILABLE_MARK`, `READ_ONLY_MARK`, the read-only
discriminator, the hint run's space join, `hintLabel`'s `move` rule, the colon
pairing, the leave-action surface check, the dedup, the empty-hint rule, the
panel `data-role`, the declared-group re-insertion, the legend read, the head-row
resolution, a `DESIGNED_STRIP_GROUPS` legend, the unit span, the
instrument/capability head, the `key === null` branch, `openSlot`'s initial
value, the empty-group mark, the unavailable-strip rule, the group-title
fallback, the `validActions` read, the null-hint skip.

**All three enforcement behaviours falsified directly, and each names the
offender:**

- *unpinned arm* — added `if (startsWith(id, "patch.macro.")) { return "macro"; }`
  to `stripGroupKey`: CAUGHT, `page.js stripGroupKey line "if (startsWith(id,
  \"patch.macro.\")) {" is transcribed by page_strip_group_key and no pin covers
  it`.
- *duplicate anchor* — put `    if (value.kind === "scalar") {` inside a block
  comment at matching indentation: CAUGHT, `the scalar value discriminator is
  pinned to … which occurs 2 times`. The cycle-1 note is a real assertion now.
- *stale scaffolding* — deleted `var spans = [];` from `hintRun`: CAUGHT, `the
  scaffolding list names 1 lines webview-page/page.js no longer has: ["var spans
  = [];"]`. The list cannot rot into a permit.

**The numbers, run rather than taken.** `cargo test --test
functional_patch_editor` exits 0, emits `CREST_ACCEPTANCE functional_patch_editor
passed` and `page_rules_pinned=69` (63 pins + 6 table entries). `cargo test
--lib` 701 passed / 0 failed / 1 ignored. `cargo test --all-targets` exit 0,
every integration target green, F-12 quiet in my run. `cargo fmt --check` clean,
`cargo clippy --all-targets --all-features` clean. `spec-kitty crest-spec doctor`
OK. `d5f659a` is one file, +316/−75.

**F-65's two corrections both hold.** `page_side_hint_line` now transcribes what
the page does: `hintLabel`'s lowercase + `^(open|move)\s+` + `\s+mode$`, the
space join, and a dedup on the hint/label *pair*. I read it against
`page.js:364-397` and `:1386-1421` line by line; it agrees, including
`String(action.hint)`'s `"undefined"` for a missing hint. The `HINT_SEPARATOR`
pin — a pin standing for a rule this file did not transcribe — is gone.
The assertion is now on the label half (`ends_with(":return")`), and the physical
hint `A / Return` can no longer satisfy it.

## The defect

### 1. The completeness check does not enforce what its docstring says

Its claim is: *"every line of every `page.js` function this file transcribes
whole must either sit inside a pinned fragment or be named below as scaffolding…
A rule added to any of these functions without a pin fails here."*

It does not. The coverage predicate is

```rust
if pins.iter().any(|(_, fragment)| fragment.contains(code)) {
```

`code` keeps its leading indentation, and a **deeper-indented pinned line
contains the same statement at shallower indentation as a substring.** So a new
statement at a function's own body indentation is silently "covered" by a pin
from somewhere else entirely:

| # | mutation | result |
|---|---|---|
| E9 | `    return openSlot;` inserted as the first statement of `stripGroupKey` — it now returns `openSlot` for every identity | **MISSED** |
| E10 | `    return UNAVAILABLE_MARK;` inserted as the first statement of `controlValueText` — every value paints the unavailable mark | **MISSED** |

E9 is covered by *an identity no designed group claims has no key*
(`      return openSlot;\n    }\n    return null;`), E10 by *the malformed
parameter mark* (`      }\n      return UNAVAILABLE_MARK;`). Neither pin has
anything to do with the line it is admitting.

This is not an exotic construction. A bare `return` at body indentation is the
most ordinary edit these two functions can receive, and they are the two the
whole rejection was about. **And the mechanism is F-42 / F-53 / F-55 exactly —
a predicate matching more than the one thing it names — applied to the coverage
check instead of to the pins.** The file requires uniqueness of its anchors and
then admits body lines by unanchored substring.

**The fix is two lines, and I ran it.** Match whole lines:

```rust
            if pins
                .iter()
                .any(|(_, fragment)| fragment.lines().any(|pinned| pinned == code))
            {
```

Exactly one existing pin then fails to cover its line — `controlIdOf`'s, because
it starts mid-statement at the ternary. Widen it to the whole statement:

```rust
("the control identity read", "    return String(\n      control && control.path && control.path.controlId\n        ? control.path.controlId.id\n        : \"\"\n    );"),
```

With those two edits: the target is green, E9 and E10 flip to **CAUGHT**, and
every other mutation above stays CAUGHT (I re-ran all 19 enforcement and cycle-1
mutations against the patched file). That `controlIdOf` was the *only* pin
needing a widen is itself the point — the loose predicate was hiding one real
gap and admitting two fabricated ones.

### 2. Two rules the transcription copies still have no pin (F-52, literally)

The ten walked functions call helpers that are not themselves walked and carry
no pin, and two of those helpers *are* rules the transcription copies:

| # | mutation | result |
|---|---|---|
| E5 | `startsWith(text, prefix)` → `return false;` — four of `stripGroupKey`'s six arms die, grouping collapses entirely | **MISSED** |
| E6 | `designedGroup(key)`'s match → `if (false)` — every group loses its legend and its `designed` flag | **MISSED** |
| E8 | `controlById`'s identity match → `if (true)` — every group head is the first control | **MISSED** |

E5 is the one to look at. With `startsWith` returning `false` the page cannot
group anything at all, and **`cargo test --all-targets` passes in full** — all 30
targets, 0 failed, exit 0. No suite in the repository notices, which is precisely
the state F-55 rejected cycle 1 for.

`page_strip_group_key` writes `id.starts_with("patch.envelope.")`. That copies
the *semantics of the page's `startsWith`*, not just the prefix literal, and
F-52 is explicit: *"If a later mission finds a copied rule no pin covers, the
correct response is to add the pin or delete the transcription — not to trust it
because it was once allowed."* Same for `designedGroup`, whose lookup
`page_strip_groups` copies as its `declared` closure.

`startsWith` is two lines and `designedGroup` is seven; both close with `\n  }\n`
so `page_function_body` reads them unchanged. Adding them to `TRANSCRIBED_WHOLE`
with pins for their bodies closes E5 and E6.

E8 is the weaker case — this file does not transcribe `controlById`, it asserts
the head row through `group.rows[0]`. Either pin it or say in the docstring that
the head-row *call site* is pinned and the *resolution* is not; do not leave it
looking covered.

### 3. The declared table's order is unpinned

| # | mutation | result |
|---|---|---|
| E7 | `DESIGNED_STRIP_GROUPS`: `envelope` and `capability` entries swapped | **MISSED** |

Each of the six entries is asserted to occur exactly once, so membership and
legends are pinned — but the *sequence* is not, and the sequence is a rule this
file copies: the Rust constant is ordered identically, `grouped_strip_shape`
asserts declared order off it, and the page's declared-position re-insertion
walks the JS array by index. Pin the array literal as one fragment instead of six
independent ones and order comes with it for free.

## The residue that is inherent — record it, do not fix it

| # | mutation | result |
|---|---|---|
| E4 | in `stripGroups`, the `if (!control.visible) { continue; }` block **moved** to the end of the loop body, every pinned fragment left contiguous and intact | **MISSED** |

The page now arranges invisible rows — cycle-1 mutation #3's exact defect,
reached by reordering rather than deleting. Both checks are set-based over line
text; neither is order-sensitive. Closing this properly means requiring each
walked body to be a *sequence* of pins and scaffolding rather than a set, which
is a different and much larger control.

I am not asking for it. I am asking that the docstring stop implying it: the
check proves **which statements the page contains**, not **in what order it runs
them**. That is a true and useful thing to have proved, and F-67 is the right
model for saying so.

## One F-65 sibling the sweep missed

`tests/functional_patch_editor.rs:2109` —

```rust
    assert!(
        hint.contains(':'),
        "the hint line pairs each projected hint with its projected label: {hint}"
    );
```

`hint` comes from `page_side_hint_line`, whose every span is built by this file's
own `format!("{hint}:{}", hint_label(…))`. The colon is contributed by the
transcription unconditionally. Preceded by `assert!(!hint.is_empty())` four lines
earlier, this cannot fail — it is satisfied by construction, and it claims to
test the page's colon-pairing rule. Same family as the `contains("Return")` you
fixed, one assertion earlier on the same composed value, and it is why F-65 calls
that shape invisible to every check this mission has built.

Low materiality — the colon rule *is* pinned, and defeating it in `page.js` is
CAUGHT. But assert on the run's structure (each space-separated span splits into
a non-empty hint and a non-empty label) or drop the line; do not leave an
assertion that reads as a check and is a tautology.

Two smaller notes in the same family, neither a defect, both worth a sentence in
the code rather than a finding:

- `grouped_strip_shape` (`:1740`): for `patch.effect.*` rows `expected` is
  `group.key.clone()`, so `expected != group.key` can never fire. The nesting
  claim is genuinely carried by the `rows[0]` occupancy check and by
  `:1809`'s prefix walk. The branch is inert by design; say so.
- `check_ranges_and_units_are_rendered` (`:2566`): `low.parse::<f64>() ==
  minimum` compares `numericRange.minimum` against itself through the
  transcription, so it proves the separator and the arm split, not that the
  painted bound is the projected bound. The precision rule is carried by the pin
  (mutation 7), not by this assertion. This matters for how you word concern 3
  below.

## What to do

Bounded, mechanical, all in `tests/functional_patch_editor.rs`. **Do not touch
`page.js`.** Do not re-run the 67-mutation sweep — I have independently
reproduced 46 of them.

1. Match whole lines in the coverage predicate, and widen *the control identity
   read* to the full `return String(…);` statement. Re-run E9 and E10 above and
   record CAUGHT.
2. Add `startsWith` and `designedGroup` to `TRANSCRIBED_WHOLE` with pins for
   their bodies. Re-run E5 and E6 and record CAUGHT.
3. Pin `DESIGNED_STRIP_GROUPS` as one array-literal fragment so its order is
   pinned with its membership. Re-run E7 and record CAUGHT.
4. Either pin `controlById`'s identity match or state in the docstring that the
   head-row resolution is not pinned.
5. Amend `check_every_line_of_a_transcribed_page_rule_carries_a_pin`'s docstring
   to say the check is over the *set* of statements, not their order, with E4 as
   the named example — the same honesty F-67 applies to the unit half.
6. Replace or delete `:2109`'s `hint.contains(':')`, and add the one-line notes
   at `:1740` and `:2566`.

## Rulings on your four concerns

**1 — Do the pins discharge F-52's condition, or merely bound the cost?**
They bound the cost, and that is the right thing for them to do. F-52 named one
failure mode: the page's text changing without the transcription noticing. The
pins address that and nothing else; they cannot say the Rust computes what the
page computes, and no number of them ever will. WP05's own statement of this is
correct and should stand — F-44's `stripGroupsPainted` is what closes the
computation gap, and this file should shrink when it lands.

But as submitted the pins do not fully bound even the cost they claim: E5 and E7
are two page-text changes the transcription does not notice, and E9/E10 are two
the *completeness check* does not notice. After items 1–3 they will. **Rule: the
transcription stays under F-52, conditionally as before, and the condition is not
yet met.** Cycle 3 meets it.

**2 — F-43's NUL is load-bearing.** Upheld, and sharpened. Verified at
`page.js:1404`: it is the only NUL in the file and it is the dedup-key separator
in `sideRegionHintLine`. WP05's two pins do sit either side, and I confirmed the
repair fires neither — I replaced the NUL with `\x1f` and the target stayed
green.

**I also replaced it with `|`, and the target stayed green.** That is the part
that matters: the pins cannot tell a safe repair from an unsafe one, so the
character F-66 explicitly warns against passes exactly as cleanly as a correct
one. The property — no projected hint or label can contain it — is not testable
here and must be chosen deliberately at merge. Carry it to the accept gate as a
named item, not as a note. `\x1f` (US) or `\x1e` (RS) both satisfy the property
and are greppable; `|` does not.

**3 — `check_ranges_and_units_are_rendered`'s unit half.** Upheld. Declining to
manufacture a rule was right: a pin with no copied rule behind it is precisely
the `HINT_SEPARATOR` defect the same cycle's audit found, and inventing one for
symmetry would have re-committed it one line over. The unit's *painting site* is
pinned and does discriminate — uppercasing `String(control.unit)` is CAUGHT.

One correction to how F-67 words it. It says the unit's text "is not read back
through a transcribed rule the way ranges and values are". The *values* are, in
full. The *ranges* only partly: the range read-back compares
`numericRange.minimum` against itself through the transcription, so it proves
the separator and the endpoint arm split, and the precision rule is carried by
the pin rather than by the assertion. So the honest statement is three tiers —
values read back fully, ranges read back structurally with precision on the pin,
units on the pin alone — not two. Amend F-67 to say that; it makes the asymmetry
smaller than WP05 claimed and describes it correctly.

**4 — The merge, the doctor, the witness.** All three check out.
`kitty/mission-functional-patch-editor-01KZERV9` is merged into lane-e at
`17e716e`/`55dbaf7`; the WP05 deliverable is still exactly `d5f659a`, one file.
`spec-kitty crest-spec doctor` reports OK (7 contexts / 135 resources, 33 project
validations, 20 witnesses). `e78a465`, WP06's F-57 amendment, touches
`contexts/testing.yaml` and `proof/witnesses.yaml` only, and only to swap
`first_patch_audible_edit_delta == 0` for
`audible_edit_isolated_to_second_patch == true`. It does not touch
`validation.functional_patch_editor`, which is still declared in
`project.yaml:182` and passes. No collision.

## For the accept gate and the mission review

- **F-64 must not be written as it stands.** It says the set "is closed under a
  check that runs on every test run". It is not closed today — E4, E5, E6, E7,
  E9, E10. After cycle 3 it is closed against *added statements* and *changed
  statements*, and open against *reordered* ones. The lesson F-64 draws is the
  right one and the most valuable thing this mission produced; it just has to
  state what the control actually covers, or it becomes the next F-53 — a
  recorded belief that is not a control.
- **F-66 needs the `|` result.** The merge step is choosing a character no test
  can police. Name `\x1f` or `\x1e` in the finding rather than leaving "pick
  deliberately" as the instruction, because "pick deliberately" is what F-54
  showed does not work.
- **F-67 is a three-tier asymmetry, not two.** See ruling 3.
- The live witness stays built-and-unrun pending an unlocked console (F-40,
  F-58); nothing in this package changes that, and `functional_patch_editor` is
  the deterministic layer and passes on its own.
