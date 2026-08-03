---
affected_files: []
cycle_number: 2
mission_slug: crest-component-controls-and-compositions-01KZ25VX
reproduction_command:
reviewed_at: '2026-08-03T01:16:35Z'
reviewer_agent: unknown
review_status: "acknowledged"
verdict: rejected
wp_id: WP03
---

# WP03 review — cycle 1

**Verdict: changes requested.** Narrow scope. The engineering is strong and every hard
constraint holds; what sends this back is one unraised FR-011 divergence in the mixer column's
text band, plus a handoff that was reported as recorded and is not.

## What passed (verified independently, not taken on report)

- **R-02** — no `egui::Slider`, `ProgressBar`, `DragValue`, or any styled egui widget. Every
  appearance is painted through `Painter` over an allocated `Response`. The only matches for
  those names in the four files are doc comments explaining their rejection.
- **FR-003 nine-state** — verified by mutation, not by reading. I stripped `Selected`'s
  non-color evidence (`status::paint_row_fill` early-return + the `StatusMark::Selection` arm),
  leaving only its accent to separate it from `Resting`. All four
  `no_two_applicable_states_look_alike_without_color` assertions failed, naming
  "Resting and Selected are distinguishable only by color". The assertion is real and
  load-bearing. `colorless_evidence()` formats texts + rect geometry + shape kinds with no
  color channel anywhere. Reverted after the experiment.
- **C-001 meter** — verified independently of its self-scan. `meter.rs`'s entire production
  import surface is egui, sibling controls, `SemanticControlViewModel`, and the visual
  vocabulary. No transitive path to `audio`, `real_time`, `rtrb`, `cpal`, or any atomic.
  `MixerTrackId` (reached via `fader.rs`) is a pure identity newtype.
- **FR-010** — both viewports resolve from `ViewportDensityPolicy`. No resolution-specific
  constant, no branch on raw viewport size. The harness viewport comes from
  `density.authored_viewport()`.
- **FR-009 / C-002** — no static, `RefCell`, `Cell`, `OnceLock`, `Mutex`, or cached field.
  All four take an immutable view model and return `ControlIntent`. No `SemanticAction`
  anywhere.
- **NFR-004** — `tests/component_vocabulary.rs` walks `src/shell` recursively, so all four
  files are under the literal guard, and it passes. No hex, no font size, no bare pixel
  constant in any production region.
- **Scope** — exactly the four new files plus `controls/mod.rs` at +9/−4. WP02's four arms
  are untouched at `paints_nothing`, ordering and formatting preserved. No test edited
  (NFR-005). The larger diffstat against the mission branch is WP01's dependency-lane merge
  (`37fe997`), not WP03's work.
- **Validation** — `cargo fmt --all -- --check` clean; `cargo clippy --all-targets -D warnings`
  clean from a forced fresh compile (not a cache hit); 648 lib tests + every integration
  suite green, 0 failures. Count confirmed.
- Tests drive the real production projection (`SemanticGraphicalViewModel::project`) through
  the production dispatch — not synthetic fixtures. No panic or `unwrap` in any paint path.

## Required change 1 — the mixer column's text is aligned against the specimen (FR-011)

This is the blocker. It was not raised as a finding, and the specimen is neither missing nor
ambiguous, so the "raise it, do not approximate" rule applies.

Component set `34:45 Compact Mixer Fader` (page `6:2`), all four variants, column is
`flex-col gap-10 px-8 py-10` inside 82 × 560:

| layer | geometry | alignment |
|---|---|---|
| `Track` | 9,10 64×18 | **centered** |
| `Fader` groove | 34,38 14×380 | centered |
| `Value` | 9,428 64×18 | **centered** |
| `State` | 5,566 72×18 | **centered** |

The screen render (`figma-functional-interpretation/assets/mixer.png`) and the component
render (`mixer-faders.png`) both show `T00`, `7F`, `P C` and `M -- · S --` centered over a
centered groove.

What is painted instead, all three in `paint_column_text` (`src/shell/visual/controls/fader.rs:208-234`):

- `fader.rs:221` — `painter.galley(bands.label.min, label, ...)` puts the track name **flush
  left** at the column edge.
- `fader.rs:226` — `value::paint_value(painter, bands.value.max.x, ...)` is **flush right**;
  `primitives/value.rs:51` computes `right_edge_x_px - size.x`.
- `fader.rs:233` — `status::paint_status_mark(...)` anchors `Align2::LEFT_CENTER`, so the
  status line is **flush left**.

`track_bar` (`fader.rs:186-193`) correctly centers the groove. The result is a column with a
centered groove, a flush-left name, a flush-right value and a flush-left status line — visibly
unbalanced, and wrong against an unambiguous specimen. Because `paint_column_text` is shared,
one fix corrects both the fader and the meter.

Centering the name and the status line is local to `fader.rs`. The value is painted through
the WP01 primitive `value::paint_value`, which only offers a right edge — center it in
`fader.rs` by passing a right edge of `bands.value.center().x + galley_width / 2.0`, or paint
the galley directly as the label path already does. Do not modify `primitives/value.rs`; it is
outside `owned_files`.

## Required change 2 — the compact-viewport label overflow has no owner

The behaviour is confirmed: at `SteamDeck` the projected label `"T00 Level"` lays out at
**79.59 px** in a **54 px** column and is cut at the column edge (clip `[8.00..62.00]`). It does
**not** bleed into the neighbour — `ui.painter_at(column)` (`fader.rs:273`) clips correctly, and
that part of the reasoning is sound. At Desktop it fits (79.59 in 86.75).

The problem is the routing. The handoff to WP06 is recorded nowhere WP06 would find it — not in
the task file, not in `status.events.jsonl`, not in code. The only trace is `fader.rs:64-66`,
which describes the clipping as intended behaviour without flagging it as a defect or naming
WP06 or T044. And WP06 could not act on it anyway:

- WP06 `owned_files` is `src/adapter/eframe_graphical_window.rs` only — it cannot change a
  fader label.
- The label text is built in `src/control/semantic_graphical_view_model.rs`, which **no work
  package in this mission owns**.
- WP08 owns `tests/`, `DESIGN.md`, `ROADMAP.md` — it can assert T044 but cannot fix it.

So as it stands WP08's T044 ("no clipped or overlapping text at either viewport") fails with no
owner able to repair it. Note the specimen labels the column **`T00`**, not `"T00 Level"` — the
architecturally correct fix is a shorter projected label upstream, not truncation in the control,
so the deferral instinct was right and only the destination was wrong.

Pick one and make it real:

- **(a)** Constrain the label to the column inside `fader.rs` (egui layout-job truncation).
  If you do, `it_paints_only_what_the_view_data_carries` (`fader.rs:544`) must move from exact
  equality to a prefix check, or it will fail on the ellipsized run.
- **(b)** Keep the control faithful and record the handoff where the receiving agent will read
  it, naming the real owner rather than WP06, and stating that T044 blocks on it.

## Required change 3 — correct one overstated claim in the meter's docs

`src/shell/visual/controls/meter.rs:7-9` says frame `SCREEN · Mixer · 1920×1080` (`42:3`)
"holds a `16 Fader Grid` of sixteen `Fader / T00…T0F` instances and nothing else." `42:3` also
holds `42:4 Context Line`, `42:14 Mixer Header`, `42:21 Mixer Legend`, `42:186 Adjustment Line`,
`42:191 Inspector` and `42:200 Controls`. Only the *grid* (`42:25`) holds solely the sixteen
instances. The conclusion drawn from it is correct and independently verified — the framing just
overstates what was swept, and a future reader would be misled about the search's breadth.

## Findings — my verdicts (all five factually verified against the design file)

**1. No Meter specimen — CONFIRMED. Resolution accepted.** A case-insensitive sweep for meter,
VU, level, peak, segment, ladder, dB, signal, LED and bar across every reachable page returns
zero meter nodes. Page `6:2` holds exactly five component sets — Context Switch, CLI Hint, CLI
Browser Line, Compact Parameter Slider, Compact Mixer Fader — and no Meter. The `Level` nodes
(`34:8/18/28/38`) are the fader's 8 × 286 fill, not a readout. No meter asset exists in the
repo's Figma export either.

Stopping short was **not** available: `.kittify/crest-spec/contexts/shell.yaml:181` declares
`Meter` a member of the closed `ComponentControl` family, with an explicit invariant at :210,
and the gallery proof at :282 requires every declared control to appear with visible evidence.
Given that, building the read-only twin of the column — same authored geometry, no cap because
the cap is the grab affordance, and deliberately no segmentation or peak because both would be
invented — is the faithful minimum. Correct call, honestly recorded.

**2. Fader paints mute and solo simultaneously — CONFIRMED. Resolution correct; escalate.**
The `State` layer is literally `M -- · S --` (Default, Focused), `M ON · S --` (Muted),
`M -- · S ON` (Solo) — one line carrying both readouts, in every variant, including a two-slot
readout at rest that the implementation does not paint at all. Single-valued `ComponentState`
cannot express it, and C-002 rightly forbids adding a state.

Record this as the resolution: `DESIGN.md:420` already settles precedence — **"Mute always
wins"** — so a composition handed a track that is both hands down `Muted`. The always-present
two-slot readout is a *status band* concern belonging to the mixer column composition, not a
`ComponentState` concern. No new state is needed, and no state should be added. This is a real
vocabulary gap worth carrying upward. (Note for whoever picks it up: the `State` layer sits at
y=566 inside a 560-tall `overflow-clip` grid, so it is clipped out of the 1920×1080 screen
render entirely — on the screen, mute and solo surface only through the inspector.)

**3. `CURRENT` versus `NonColorSignal::Shape` — the vocabulary wins here. Escalate the fix.**
The specimen genuinely marks the current row with the literal word `CURRENT` (node `48:184`,
10px semibold, `text-muted`, right-aligned) — confirmed, not inferred.

**Which authority wins:** for *how a state signals*, the closed vocabulary wins; for *geometry,
spacing and appearance*, Figma wins. `state.appearance().signal` is the single source the T017
assertions are written against, and `state.rs` belongs to WP01 — painting `CURRENT` while the
vocabulary declares `Shape` would put the code in disagreement with the vocabulary the crest-spec
mandates. Deferring was right and the painted shape (row fill plus selection mark) is
unmistakable against `Focused`, which is what T016 asks for.

But the durable answer runs the other way, and it is cheap: `NonColorSignal` already has a
`Word(&'static str)` variant, so `Selected → Word("CURRENT")` would satisfy the vocabulary *and*
the specimen with no new state and no new enum variant, keeping the shape as reinforcement. WP03
cannot make that edit — `state.rs` is outside `owned_files`. Escalate it; do not act on it here.

**4. Derived 90.75 px pitch against the specimen's 86 px — not drift, but escalate.** Every
number checks out. Specimen pitch is **exactly 86** (82 width + 4 gutter, sixteen instances at
x = 0, 86, 172 … 1290). Grid slack is **exactly 80** (1452 − (16×82 + 15×4) = 1452 − 1372).
The derivation: desktop 1500 − 48 = 1452, /16 = 90.75, so 86.75 × 16 + 4 × 15 = **1448 ≤ 1452**;
compact 960 − 32 = 928, /16 = 58, so 54 × 16 + 4 × 15 = **924 ≤ 928**. Sixteen fit at both, and
columns come out wider than the specimen, never narrower, so `DESIGN.md:462` holds.

Held to the stated standard, this is **not silent** — `fader.rs:25-40` names the 90.75-against-86
deviation, the cause, and the direction of the error. That is the opposite of drift. But it is a
+5.5% deviation on the mixer's most visible element, and the right fix already exists as a
pattern in this codebase: `ViewportDensityPolicy` declares `utility_control()` as measured
380 × 48 × 60 with `MeasuredFromAuthoredDesign` provenance. A `mixer_strip()` accessor declaring
the authored 82 width and 4 gutter would satisfy FR-011 *and* FR-010/NFR-004 together, with no
literal in the control. WP03 cannot add it — `density.rs` is outside `owned_files`, and no work
package in this mission owns it. Escalate; leave the derivation in place for now.

**5. Compact label overflow — clipping confirmed, handoff not.** See Required change 2.

## Not blocking, worth a look while you are in here

- `modal_option.rs:155-175` paints a value readout and a dot-leader rule between the name and
  the value. Neither appears in `48:173` / `48:207`, whose rows carry only the option name and,
  on the current row, `CURRENT`. Defensible for a control generic over kinds, but it is an
  addition to the specimen and was not raised.
- `modal_option.rs:72-77` reserves `focus::LABEL_START_X_PX` (18) for every row. The specimen
  puts the name at x=18 only on the current row — the other five sit at x=10, because the cursor
  glyph is a zero-width space there, so the design actually shifts labels by 8 px. Reserving the
  column is the better behaviour and the doc says why; it is inherited from a WP01 constant, so
  no action. Noted so it is not mistaken for an oversight later.
- `modal_option.rs:61-66` takes `.min(MIN_INTERACTIVE_TARGET_PX)`, which is a ceiling at 48, not
  a floor. Both current policies land on 48 and
  `a_row_sits_on_the_authored_interactive_minimum_at_both_viewports` guards it, so there is no
  live defect — but the name and doc say "denser of", and a future policy below 48 would silently
  produce a sub-minimum target.
- `value_text` renders `{scalar:.3}` → `0.000` where the specimen shows hex `7F`. This matches
  the pre-existing `semantic_value_label` in the adapter (verified on the mission branch), so it
  is consistent rather than new drift — but the specimen's value format is a real open question
  for WP06.

## Coordination

`src/shell/visual/controls/mod.rs` is edited by both WP02 and WP03. Each touched only its own
four arms and each inserts a `pub mod` block at the same anchor, so the two hunks are disjoint
in content but adjacent in line range — expect a trivial textual conflict at merge, resolved by
taking the union of the `pub mod` lines and each WP's own arms. Nothing to fix; flagged so it is
not mistaken for a regression.
