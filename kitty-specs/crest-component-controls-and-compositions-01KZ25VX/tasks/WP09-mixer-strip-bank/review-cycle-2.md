---
affected_files: []
cycle_number: 2
mission_slug: crest-component-controls-and-compositions-01KZ25VX
reproduction_command:
reviewed_at: '2026-08-03T18:05:54Z'
reviewer_agent: unknown
verdict: rejected
wp_id: WP09
---

# WP09 review — cycle 1

**Verdict: changes requested.** Not a redesign. The implementation is correct; four declared behaviours are unproven, and I reintroduced the exact defect this work package exists to retire without a single test failing.

Reviewer: paula-patterns. Five architecture lenses plus first-hand verification of every claim below.

---

## Read this first: what is settled and must not be redone

Cycle 2 should touch **only** the test module of `src/shell/visual/compositions/mixer_strip_bank.rs`. Everything else in this work package is verified and approved by this review.

- **The 82/86 measurement is CONFIRMED against the live design file.** I queried Figma directly. `42:20` "Faders" is 1500 × 896; `42:25` "16 Fader Grid" sits at x=24 with width 1452 and holds sixteen `Fader / Txx` instances at width **82**, at x = 0, 86, 172 … 1290 — pitch **86**, gutter **4**. `15 × 86 + 82 = 1372 ≤ 1452`. `DESIGN.md:462` requires all sixteen visible at 1920×1080.
  `MIXER_TRACK_MIN_WIDTH_PX = 176.0` (`src/adapter/eframe_graphical_window.rs:37`) is **2.15× the authored 82**, and `16 × 176 = 2816 > 1500`, which is the only reason `egui::ScrollArea::horizontal` exists at `:512`. The constant's own comment admits it is an implementer's floor, not a measurement. **WP06 can retire the scroll — it was compensating for an invented constant, not carrying load.** The shipped mixer additionally paints each track as an `egui::Frame` with fill and stroke, i.e. a card, against `DESIGN.md:462`'s "hairline separators, not cards". WP09's diagnosis is right and if anything understated.
- **T048 overrun: ACCEPTED.** See the ruling below. Do not revisit.
- **WP03's two assertions pass unmodified** — 14/14 in `fader`, 92/92 across `controls::`, verified against real output. Zero test-body changes anywhere in the diff.
- **Gates, run by me on the whole module with exit codes captured and no pipes:** `cargo fmt --check` 0; `cargo clippy --all-targets -- -D warnings` 0 with zero warnings; `cargo test` 0.
- **Baseline measured by me in an isolated worktree at `f62c658^`, not trusted from any number:** **796 passed / 0 failed → 810 passed / 0 failed, delta +14.** Exactly as claimed. There are **zero** pre-existing Rust failures; the prompt's "1 pre-existing failure" is this WP's own `baseline-tests.json` capture failing in the F-12 pattern (`total:1 / passed:0`, a CLI usage error).
- **All five `MixerStripBank` invariants are satisfied in the code.** Entries-are-groups holds structurally; two-level marking is genuinely **one** mechanism (`section::render_entries` delegates to `mark_unavailable` at `section.rs:185-187`); ownership is clean (no `AppState`, no `SemanticAction`, no audio — `CompositionRenderFn` has no audio parameter to leak through, so the meter acquired nothing); scope is clean (exactly four files, WP04's and WP05's arms byte-identical, adapter and `meter.rs` untouched).
- **The disclosed flatten fix is real.** I re-ran both flatten mutations myself. Both die, by name, on the ordered-and-disjoint loop at `:702-710` — `sixteen_columns_seat_inside_the_main_surface_at_both_viewports` and `every_track_cell_is_painted_inside_that_track_own_column`. That hole is closed.
- **Both named seams are adequately handled.** The fader-label double-naming is recorded for WP06 in the module doc and the completion note, with the column title band allocated above the cells so they stack. The Pan-cell divergence is raised with the selector **not** bypassed — the bank imports nothing from `controls` except `PresentationRole`, enforced by a source scan.

---

## Ruling on the T048 overrun — ACCEPTED

The declared bound said "two function bodies plus two doc paragraphs… touch nothing else in `fader.rs`." WP09 added **eight lines** beyond it (two attributes, six doc lines), removed none.

**The overrun was forced, and the bound as written was unsatisfiable.** Stripping the attributes fails the declared gate: `error: constant STRIP_COLUMN_COUNT is never used`, exit 101 under `cargo clippy --all-targets -- -D warnings`. Deleting the constant instead would have required editing WP03's `sixteen_columns_and_their_hairlines_fit_the_main_surface`, which reads it three times at `fader.rs:673,674,677` — NFR-005 forbids exactly that. Every invariant the bound actually named is intact: both signatures, `pub(super)`, the constant itself, no test touched. That is a bound that failed to anticipate the lint gate, not an implementer expanding its remit. **Asking for the ruling rather than deciding unilaterally was the right instinct and is noted approvingly.**

**One correction for the record, not grounds to reject:** `#[cfg(test)]` was not the minimum. `#[allow(dead_code)]` on the constant alone passes the identical gate at exit 0 in **one** line, leaving the module's non-test item set unchanged. The second `#[cfg(test)]`, on the `MixerTrackId` import, was a cascade WP09's own remedy created — the delegation did not force it, because the import is still consumed by the constant in the lib target. The disclosure attributes it to the delegation. Optional to simplify in cycle 2; harmless if left.

---

## Ruling on SteamDeck 52/56 — choice SOUND, doc justification PARTLY WRONG

The **choice is sound and I affirm it**: `15 × 56 + 52 = 892 ≤ 928`; `52 ≥ 48`; gutter 4 = `SpacingStep::S4` at both policies; and 56 is genuinely the pitch this policy already authors — I verified both `rhythm().row_pitch_px == 56` and `utility_control().pitch_px == 56` at SteamDeck. Rhythm-consistency over tightest-fit is defensible: the crest-spec's own "where they already fit it keeps the measured values rather than stretching them" establishes the overflow rule is not occupancy-maximizing, and a tightest-fit value would make `PolicyProvenance::AuthoredFromDesktopFrames` a misstatement.

**But the parenthetical that disposes of the alternative is arithmetically wrong**, at `density.rs:324-326`:

> "the tightest pitch that would fit (`(928 − 15 × 4) ÷ 16` is 54.25, giving a fractional column that consumes all but 4 px of the surface)"

- **54.25 is the tightest WIDTH, not the tightest pitch.** Solving `16w + 15×4 ≤ 928` gives `w ≤ 54.25`; the tightest *pitch* is 58.25.
- **"consumes all but 4 px" is false.** At 54.25/58.25 the bank is `15 × 58.25 + 54.25 = 928.0` exactly — zero slack, not 4.
- Consequently **the real nearest competitor is never disposed of**: pitch 58 / width 54 is integral, above the floor, gutter 4, and leaves 4 px slack. Your *primary* argument does dispose of it (58 is not a pitch this policy authors anywhere), but the broken parenthetical does not.

**Fix the paragraph.** Also `density.rs:328-330` claims SteamDeck's slack is "the same kind of slack the desktop grid was drawn with" — it is not. Desktop's 80 px is *measured* slack that exists in the design file; SteamDeck's 36 px is residue from a rhythm-aligned pitch. That blurs measured against authored in the one place the crest-spec says they must stay distinguishable.

---

## MUST FIX — four unproven behaviours, all reproduced first-hand

Each was applied to a clean checkout of `f62c658` in an isolated worktree and run against the real suite. **The implementation is correct in every case — what is missing is the assertion.**

### 1. BLOCKING — the proof cannot tell the authored grid from an invented one

I replaced `column_rect`'s policy-derived placement with the bank dividing the content rect sixteen ways itself:

```rust
let invented_pitch = body.width() / MixerTrackId::COUNT as f32;   // Desktop: 1452/16 = 90.75
```

**All 729 lib tests passed. Exit 0.**

That is *exactly* the 90.75 px pitch the prompt calls "a reject even though it fits", and exactly the surface-local division T048 was written to remove from `fader.rs` — reintroduced in `mixer_strip_bank.rs` with nothing to stop it. The crest-spec invariant "mixerColumn resolves through this policy **and nowhere else**" is guarded at the policy (`the_desktop_mixer_column_reproduces_the_measured_design_grid`) and **not at the composition**, which is the side that paints.

This is the residual form of the self-consistency hole you found and disclosed. Your disjointness loop closed *"all columns identical."* It does not close *"all columns mutually consistent but collectively wrong."* Your headline test says it asserts "against the painted result, not against the policy the paint was supposed to use" — that is the right instinct against re-deriving, but taken this far it cannot detect that the paint **ignored** the policy.

**Fix:** in `sixteen_columns_seat_inside_the_main_surface_at_both_viewports`, additionally assert the *painted* pitch and width equal `density.mixer_column().pitch_px` / `.width_px` — i.e. `columns[i].min.x - columns[i-1].min.x == pitch_px` and `columns[i].width() == width_px`, at both viewports. This is measuring the paint **and** binding it to the declaration; the two are not in tension.

### 2. BLOCKING — the no-placeholder assertion is vacuous for the case it names

`a_marked_bank_paints_no_value_the_projection_did_not_carry` docstring: *"no level, no zero, and no dash standing in for a reading nobody reported."*

I painted a literal `"0"` into every empty column. **All 10 tests passed.**

Root cause: `section.rs:1010` accepts a run if any projected text **`contains`** it as a substring, not equals it. `"0"` is a substring of `"T00 Level"`. So is `"0.0"`, and so is any truncated fragment of a real label. C-003 is the mission's most-enforced constraint and this is its guard.

**Fix inside your own test — do not touch WP05's helper.** Require whole-label equality for the runs your bank emits, or assert the empty-bank run set equals exactly the declared structure names plus the authored mark plus the legend. (`"-12.5 dB"` *is* caught, so the test is not wholly inert — but the canonical fabrication is not.)

### 3. BLOCKING — the hairline separators have no assertion at all

Making `paint_separators` a no-op — **no hairlines anywhere** — passes the full **810-test suite, exit 0**. Moving each hairline off the gutter midpoint onto the column edge also passes.

`DESIGN.md:462` names "compact columns with hairline separators, not cards" as a product requirement and T049 step 7 explicitly required them. The code is right (15 of them, `skip(1)`, authored `rules::hairline`, exactly at the gutter midpoint — I verified the arithmetic). It is the one declared WP09 behaviour with no proof behind it.

Mechanism: your local `Seen` harness (`:429`) drops the non-text shape count, and `collect` (`:538-554`) discards every non-text shape via `other => { let _ = other; }`. **WP05's shared `probe::Painted` already carries `shapes: usize`, documented as "How many non-text shapes were emitted"** (`section.rs:377`) — the mechanism you needed exists in the harness you were told to compose with, and the geometry-reading harness you added is strictly less capable in this one dimension.

**Fix:** count non-text shapes in `Seen`, or assert the 15 hairline x-positions.

*Context, so this is weighted fairly:* no composition in the mission asserts shape counts, and `section.rs:309` and `utility_inspector_panel.rs:421` paint the same unasserted hairlines in approved WP04/WP05 work. I am raising it as **yours** only because it is a behaviour your own prompt named explicitly; the mission-wide version goes to cross-WP findings, below.

### 4. BLOCKING — there is no vertical assertion anywhere in the module

I collapsed every cell of every column onto the same `y` (`let top = body.min.y;`) — a completely broken column. **All 10 tests passed.**

Every geometric claim in the module is x-axis only. `render_column`'s band arithmetic at `:236-241` has zero coverage. A bank whose columns are perfectly placed and whose contents are piled on one line satisfies your entire proof.

**Fix:** assert the cells within a column are vertically ordered and disjoint — the y-axis mirror of the loop you already added at `:702-710`.

---

## SHOULD FIX

### 5. "The bank scrolls nothing" is a string grep, not a behavioural proof

The needle list at `:920-934` knows only `"ScrollArea"`. A real `ui.scroll_with_delta(...)` — the exact defect this composition retires — passes untouched, as do `scroll_to_rect` and `scroll_to_cursor`. Add the sibling needles at minimum. The scan's plumbing is otherwise sound: the `#[cfg(test)]` split is correct, a moved file panics loudly rather than passing vacuously, and the `!source.is_empty()` guard is present.

### 6. `TOLERANCE_PX` hides a real sub-pixel overlap

`TOLERANCE_PX = KEYLINE_RESTING_PX` (1.0) is a hairline *thickness* reused as a float epsilon, and it is applied on the permissive side of every inequality — `column.width() + TOLERANCE_PX >= floor` accepts 47 px against a hard 48 px minimum, eroding a quarter of SteamDeck's real 4 px margin.

For the column grid it hides nothing (all coordinates are exactly integral: Desktop `24 + 86i` width 82, SteamDeck `16 + 56i` width 52). But at `:799-802` it **does** hide a real defect: at SteamDeck the `ROWS` run ends at x=218.59 and the `--` mark begins at x=217.88 — a genuine **0.72 px overlap** inside the 52 px column, absorbed by the tolerance. The comment above it says non-collision is "a property worth holding rather than assuming"; as written the test does not hold it.

Note when you re-probe this: setting the tolerance to `0.0` produces a **strict-`<` artifact** on exactly-equal values ("starts its bank at 24 rather than at the authored inset 24") and is not a usable probe. Use `0.001`.

### 7. Fix the two doc-arithmetic errors in `density.rs` per the SteamDeck ruling above.

### 8. Optional: `fader.rs:155`

`column_width_px` computes `column_pitch_px(density) - density.mixer_column().gutter_px()` — a round trip returning its own input, i.e. `pitch − (pitch − width)`. It is exact at both policies and cannot diverge at a 4 px gutter, so this is cosmetic, but `density.mixer_column().width_px` reads as what it is, and the doc directly above it argues against a control computing what the policy already answers.

---

## NOT YOURS — recorded so they are not lost

- **Legend duplication.** `render_legend` (`:272-319`) is a faithful but verbatim ~45-line fork of the private `section::render_header` (`section.rs:215-259`). It is **not** a second header shape — same shared band height, same anatomy, shared `FOCUS_ANNOTATION_LABEL` and separator, and it correctly inherits F-11's ~2 px drift rather than adding a new one. But four style decisions now live in two places and no test compares them. Closes with one keyword (`pub(super) fn render_header`) in `section.rs` — **WP06's**, which is already opening that file for F-11. Your stated reason (privacy) describes a work-package boundary as if it were a language barrier; the scope call was right, the wording understates it.
- **Hairline assertions are missing mission-wide** (`section.rs:309`, `utility_inspector_panel.rs:421`) while the controls layer does assert shape counts (`browser_row.rs:313`). For cross-WP findings / WP08.
- **`from_projection_or_vocabulary`'s substring laxity** (`section.rs:1010`) weakens every composition's no-placeholder assertion, not just yours. WP05's approved file; for cross-WP findings.
- **MERGE WATCH.** `d91fbf5`, the crest-spec amendment declaring `MixerStripBank`, is reachable **only** from `feat/crest-component-controls-and-compositions` — not from the mission branch or any lane. Code declares eight compositions; bedrock reachable from this lane declares seven. Correct authoring order (spec Aug 2, code Aug 3) and normal for this mission's topology, but `spec-kitty accept` must run where both are reachable. Not yours to fix.
- **The meter is now a deletion, not a port.** `CompositionRenderFn` has no audio parameter, so when WP06 swaps the adapter the live meter does not move — it disappears. Correct under C-001, but it is a Phase 5 signature change rather than a wiring change, and should be a known cost rather than a WP06 surprise.
- **Completion-note wording.** The SteamDeck seating figure reports column 15's *left* edge (856.0) where Desktop reports the *right* edge (1396.0). The actual SteamDeck last-right is **908.0 ≤ 944.0**. Geometry correct, note misworded.

---

## Why this is a reject rather than an approval with findings

The implementation is genuinely good and the disclosure was honest and complete — you found and fixed a self-consistency hole that two earlier work packages died on, and you asked for the one ruling you should have asked for.

But the single thing this work package exists to make true — *mixer-column geometry resolves through the policy and nowhere else* — is the thing its proof does not establish. I reintroduced the retired 90.75 derivation and 729 tests stayed green. Alongside a C-003 guard that a literal `"0"` walks through, a product-required hairline that can vanish entirely, and a column layout that can collapse to one line, that is four declared behaviours resting on inspection in the mission's most scrutinised work package.

Every fix is inside `mixer_strip_bank.rs`'s own test module plus one doc paragraph in `density.rs`. No production logic changes, no other work package's files, no crest-spec amendment. This should be a short cycle.
