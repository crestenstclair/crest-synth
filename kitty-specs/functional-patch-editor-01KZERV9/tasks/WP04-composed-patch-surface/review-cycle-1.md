# WP04 review — cycle 1: REJECTED

The reasoning in this package is the best in the mission so far. Both disputed
rulings are upheld — you were right about the transport and right about
`pathLabel`, and I confirmed both by execution rather than by reading. The
projection change was the correct call and I am not asking you to revert it.

The rejection is entirely about the half you could not run. I have a display
that seats 1920×1080, so I ran `CREST_WEBVIEW_TESTS=1` against the shipped
WebKit. Three things came back that the Chrome substitution could not see, and
one of them is a regression of an assertion that passed before this package.

---

## What I upheld

### The transport premise — confirmed, by execution

I built the production fixture, projected it, and pushed it through the real
`ProjectionChannel::push`, asserting the emitted payload equals
`serde_json::to_value(projection.semantic_model())`. The document the page
receives has exactly these top-level keys:

```
["activeSurface", "context", "errors", "focusPath", "generation",
 "interactionMode", "returnPath", "stateHash", "status", "surfaces",
 "validActions"]
```

`patchPage`: **absent**. `graphicalShell`: **absent**. `pathLabel`: **absent**.
The same run confirms all three are present in the StateTree.

So WP03's B2 resolution rested on a false claim, and I propagated it. The fact
did not "already reach the screen"; it reached a different document that no
shipped surface paints. You were right to say so.

### `editable` really does discriminate nothing

Measured on both detail subjects:

| subject | row | `editable` | `patchInteraction` |
|---|---|---|---|
| SoundFont | Preset | `false` | `structuralChoice` |
| SoundFont | SoundFont File | `false` | `readOnly` |
| Chorus | Amount | `false` | `scalarEdit` |
| Chorus | Depth | `false` | `scalarEdit` |

I also checked every other leaf on those rows for a usable discriminator —
`kind` (`choice`/`asset`), `enabled` (all true), `focusable` (all true),
`validActions` (same shape). There was none. T028 was literally unimplementable
without the field.

**Ruling: adding `patch_interaction` was correct, not a WP03 cycle 3.** WP03 is
approved and closed; the consumer is yours; the producer is the single existing
`ParameterSpec::patch_interaction` rather than a re-derivation; schema 15 is
unshipped so it costs no extra bump. Splitting it across a closed package would
have bought nothing — the same reasoning that moved F-18 onto you.

One consequence to record, not to fix here: `contexts/control.yaml`'s
`SemanticControlViewModel.state` does not name `patchInteraction`. That block
already omitted `numericRange` and `focusable` (F-31); it now omits three.
`crest-spec doctor` passes because the block has no code binding. The crest-spec
is in no package's map, so this is a mission-level action before accept — but
the mission should close it deliberately rather than let the code keep drifting
ahead of the declaration.

### F-25 — you are right, and it is better than either of us said

`pathLabel` lives on `GraphicalShellProjection`, is emitted only into the
StateTree, and is absent from the page's document (executed, above). The lift I
gave you was meaningless and you were right to keep the footer on
`control.label`.

Refinement: WP03 already fixed the *composition*. `footer_path_label` now reads
`semantic.focused_control().label()`, and the live tree emits
`"PATCH / Engine"` — an authored label, not `"PATCH / patch.voiceLimit"`. So
F-25's real status is not "unfixed": the key leak is closed, and what remains is
a correctly-composed projection field with no reader, plus a second independent
breadcrumb composer in `page.js`. That is a duplication finding for the mission
review, not a defect you inherited.

### F-18 — complete, on both sides, and alive by rendering

Zero snake_case leaves remain anywhere in the shipped document (walked every
leaf path of the serialized model and asserted the set is empty). No
`track_id` / `patch_name` / `capability_id` reader remains in `src`, `tests`, or
`webview-page`. And the reader half is proven by rendering, not by compiling —
falsified by mutation in the shipped WebKit. Reverting the three
`summary.patchName` reads in `page.js` back to `summary.patch_name` produces:

```
T024 desktop 1920x1080 patch-navigate: the strip header names the focused
Patch from the projection
  left:  Some("undefined")
 right:  Some("Semantic 1")
```

The blank surface F-18 warned about is real, and there is a live guard that
names it. (The worktree was restored; this was a review-only mutation.)

### The rest of the reviewer checklist

- **Group ordering**: no table can drift. Groups are created in first-appearance
  order keyed on the control's serialized identity; `DESIGNED_STRIP_GROUPS`
  supplies legends and inserts only *empty* groups, which carry no rows and
  therefore cannot reorder a painted row. Live-asserted both ways: painted row
  order equals projected order, and the concatenation of the groups' rows equals
  the flat painted order.
- **`detailShellHtml` contains no `summary.subject` reference** — only three
  comment mentions. The title resolves through `ownerGroupKey` →
  `groupHeadControlId` → the owning strip row's projected value. Both subject
  kinds rendered and asserted live.
- **`src/bin/crest_synth.rs`**: one JSON pointer, forced by the casing change,
  minimal. Same shape as F-24; WP06 will be told.
- **`#inspector` keeps `overflow: hidden`** — measured in the shipped engine:
  `overflowY: "hidden"`, `scrollHeight == clientHeight` (863/863 desktop,
  613/613 compact). Five rows seat at both viewports. T027 holds.
- **Component vocabulary**: clean, and it derives from the crest-spec, which
  does declare `PatchStrip` and `CapabilityDetailShell`.
- **Utility ordering table**: `DESIGNED_UTILITY_ENTRIES` exists but the live
  assertion compares painted rows to the projected list *in projected order*, so
  a divergence fails rather than paints. Acceptable.

---

## Why this is rejected

### B1 — the position rail collapses. A measured regression of a passing assertion.

`CREST_WEBVIEW_TESTS=1`, shipped WebKit, desktop 1920×1080:

```
T011 desktop 1920x1080 PATCH: row patch.envelope.attackMilliseconds
position rail must have width (got 4.28px)
```

`rail > 5.0` is **not your assertion** — it pre-exists this package. I ran the
same live section at your base (`a380235`, with only the superseded Utility
assertion neutralized so the run could reach T011) and it passes there:

> `T011 painted-geometry fidelity: PASS (measured .fader-fill/.prow-position-fill
> geometry proportional to document values … at 1920x1080 and 1280x800 …)`

So this package turned it red. Measured rail widths, same engine, same window:

| document | row | base | WP04 |
|---|---|---|---|
| patch-navigate | attackMilliseconds | 1264.97 px | 366.73 px |
| patch-navigate | chorus.amount | 1296.88 px | 319.09 px |
| patch-braids | releaseMilliseconds | 1256.38 px | 326.34 px |
| T011 geometry | attackMilliseconds (focused) | (passing) | **4.28 px** |

The mechanism: `.prow` gained `flex-wrap: wrap`, and `.prow-position` is
`flex: 1` — i.e. `flex-basis: 0`. A zero-basis item always "fits", so the line
never wraps on its account; every item crams onto line one and the rail gets
whatever the hint run leaves. On the focused row, whose `validActions` list is
longest, that is 4 px. Your CSS comment anticipates exactly this and says the
row "wraps it onto its own line rather than clipping it" — the wrap does not
happen for the rail.

This is the primary visual readout of a numeric parameter losing ~70% of its
width on ordinary rows and disappearing on the focused one. It is not a test
artifact.

### B2 — T025's own new assertion cannot pass as written

```
T024 desktop 1920x1080 patch-navigate: the focused row's hints are the
model-level hint run, exactly
  left:  ["2:patch1:mixerE:next", "patchS:downD:rightrelease", …]
  right: ["2:patch", "1:mixer", "E:next", "patch", "S:down", …]
```

`hintRun` emits adjacent `<span>`s with no separating text, so `textContent`
concatenates them; the harness builds its expectation by `join(" ")` and both
sides are then `split_whitespace()`. That can never agree whenever a row has
more than one hint.

Two things to note. First, this is **not** a Blink-vs-WebKit difference —
`textContent` is identical in both, so running your Chrome harness against this
assertion would have failed there too. It was never exercised anywhere. Second,
decide deliberately which side is wrong: either the painted hint run should carry
a separator in text (which also affects how it reads to a screen reader and to
anyone selecting the text), or the assertion should compare on the terms the DOM
actually presents. The footer uses the same `hintRun`, so the page is
self-consistent — T025's real requirement ("neither was special-cased") does
hold. Only the assertion is wrong about the shape.

### B3 — the desktop viewport claim in the commit message is false in the shipped window

The commit says:

> "…and the desktop strip seating without scroll exactly as it did before the
> grouping."

Measured, shipped WebKit, window at `LogicalSize(1920, 1080)`:

| document | base `#strip` sh/ch | WP04 `#strip` sh/ch |
|---|---|---|
| patch-navigate | 754 / 754 (seats) | **801 / 754 (overflows 47 px)** |
| patch-adjust | 754 / 754 (seats) | **811 / 754 (overflows 57 px)** |
| patch-braids | 754 / 754 (seats) | 754 / 754 (seats) |

The root cause of the measurement disagreement is worth more than the number:
**the shipped window gives the page `innerHeight` 1018, not 1080** (and 768, not
800, at compact). Your Chrome harness used an exactly-sized 1920×1080 iframe, so
every seating conclusion was drawn with 62 px of vertical room the product does
not have. That is the one methodological flaw in an otherwise careful
substitution, and it is the reason the desktop claim inverted.

Two honesty notes on my own measurement. Part of that 62 px is my display: a
1920×1080 window on a 1920×1080 screen is clamped by the menu bar, so on a larger
display `innerHeight` would be nearer 1052 and the strip's client height nearer
788. The regression survives that: 801 and 811 still overflow 788, and the base's
754 still seats. And base and head were measured in the *same* window on the
*same* machine minutes apart, so the A/B itself is exact regardless of chrome.

**On NFR-003 and "inherited".** Compact, same measurements:

| document | base | WP04 |
|---|---|---|
| patch-navigate | 708 / 504 | 846 / 504 |
| patch-adjust | 708 / 504 | 894 / 504 |
| patch-braids | 648 / 504 | 784 / 504 |

So: compact overflow *is* inherited in kind — it scrolled before you. But it grew
from 204 px to 342–390 px, and desktop went from seating to scrolling, which is
not inherited at all. NFR-003 as written is still satisfiable (scrolling is not
clipping; every band, the side region, the 420/320 floors and the 48 px minimum
all hold, and I confirmed the five Utility rows seat with `overflow: hidden`
intact). But "a degree rather than a kind, and inherited" is not an accurate
report of what this package did, and the DoD line it is graded against says both
authored viewports seat the surface.

Fix B1 first — the rail and the hint run are competing for the same line, and a
layout that gives the rail its own line is likely to relieve the height pressure
in B3 as well, or make it worse in a way you can then measure honestly. Re-run
the live gate, do not re-reason about it.

---

## Not yours, recorded

- The choice row painting a choice id is a **projection gap, not a page gap** —
  confirmed by execution: the Preset row's projected value is
  `{"kind":"choice","value":"sf2.bank-0.program-40"}`, so the page has no label
  to paint. Worth telling WP05: the label guard and T027's own check both test
  *labels* (`!row_label.contains('.')`). This key reaches the screen as a
  **value**, which nothing guards.
- **The capability-sections restraint was right.** All five production
  capabilities declare exactly one `CapabilitySection`, and the semantic
  projection flattens `descriptor.parameters()`, so extending it would change
  nothing visible — whereas `patchInteraction` was blocking. Extending the
  projection only where the subtask is otherwise impossible is the right line,
  and you drew it and recorded the disagreement instead of quietly widening
  scope. Keep doing that.
- `T026`/NFR-001 could not be exercised on my machine: the screen was locked, so
  `requestAnimationFrame` is suspended and no paint acks return (0 of 150). This
  reproduces identically at your base, so it is environmental, not yours.
