---
work_package_id: WP09
title: Mixer strip bank and the mixer-column policy
dependencies:
- WP01
- WP03
- WP05
requirement_refs:
- C-003
- FR-004
- FR-005
- FR-006
- FR-010
- FR-011
planning_base_branch: feat/crest-component-controls-and-compositions
merge_target_branch: feat/crest-component-controls-and-compositions
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-controls-and-compositions. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-controls-and-compositions unless the human explicitly redirects the landing branch.
subtasks:
- T047
- T048
- T049
- T050
- T051
- T052
phase: Phase 3b - The eighth composition
history:
- at: '2026-08-03T04:07:51Z'
  actor: planner
  action: Prompt authored after the F-09 crest-spec amendment (d91fbf5) declared the eighth composition
agent: claude
agent_profile: implementer-ivan
authoritative_surface: src/shell/visual/compositions/
create_intent:
- src/shell/visual/compositions/mixer_strip_bank.rs
execution_mode: code_change
owned_files:
- src/shell/visual/compositions/mixer_strip_bank.rs
- src/shell/visual/density.rs
role: implementer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP09 – Mixer strip bank and the mixer-column policy

## ⚡ Do This First: Load Agent Profile

Use the `/ad-hoc-profile-load` skill to load the agent profile specified in the frontmatter, and behave according to its guidance before parsing the rest of this prompt.

- **Profile**: `implementer-ivan`
- **Role**: `implementer`
- **Agent/tool**: `claude`

If no profile is specified, run `spec-kitty agent profile list` and select the best match for this work package's `task_type` and `authoritative_surface`.

---

## Markdown Formatting

Wrap HTML/XML tags in backticks: `` `<div>` ``, `` `<script>` ``
Use language identifiers in code blocks: ```rust, ```bash

---

## Objectives & Success Criteria

Build `MixerStripBank`, the eighth composition — sixteen mixer track columns side by side in the main workspace — and give `ViewportDensityPolicy` the mixer-column geometry that lets it allocate them.

Complete when:

- `MixerStripBank` renders sixteen track columns from `SurfaceId::MixerMain`, arranging **groups** where every other composition arranges controls.
- Its geometry resolves through `ViewportDensityPolicy::mixer_column()` at both viewports. **No literal, and no surface-local division of the main surface.**
- Two-level titling and two-level unavailable-marking are both implemented, both asserted.
- **An assertion proves sixteen columns actually seat at both viewports**, through a real render pass.
- `MIXER_TRACK_MIN_WIDTH_PX` has a policy to be replaced by, and the fader's surface-local column derivation delegates to it.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and the full suite are green.

## Why this work package exists, mid-mission

Implementation proved the declared composition family incomplete. `paint_patch_workspace` lands in `Section` — WP05 demonstrated and tested it. **`paint_mixer_workspace` landed nowhere in the closed seven.**

A `Section` asked at `PresentationRole::VerticalStrip` is one track **column**. What had no composition is the **bank** — sixteen columns side by side.

The cheap alternative was tested and rejected. Giving `Section` a layout axis fails because `Section`'s entries are typed `&[SemanticControlViewModel]` — *controls* — while a bank's entries are *columns*, and a column is itself a titled group. **The bank is a group of groups.** An axis flag does not change entry type: a horizontal `Section` handed the flat `MixerMain` list paints all sixteen tracks' controls in one horizontal run with no per-column title and no column boundary. The gap is nesting, not direction.

The full argument and its three further blocks are finding **F-09** in `cross-wp-findings.md`. **Read it before you start.** The crest-spec amendment it calls for is already authored and committed (`d91fbf5`); this work package implements it and does not relitigate it.

## The measurement that must shape everything you do

**`MIXER_TRACK_MIN_WIDTH_PX = 176.0` is not an authored value, and the shipped mixer is wrong because of it.**

The constant's own comment says so — *"sub-band splits the authored vocabulary does not declare… named rather than resolved because there is nothing yet to resolve them from"* (`eframe_graphical_window.rs:28-37`). It is an implementer's floor, not a measurement.

**The design file was measured, and re-verified against the live file while this prompt was written.** Figma `42:25` "16 Fader Grid" sits inside `42:20` "Faders" (1500 × 896, inset 24 → **1452** content) and holds sixteen `Fader / Txx` instances at **width 82, pitch 86**, at x = 0, 86, 172 … 1290:

```
15 × 86 + 82 = 1372 ≤ 1452
```

**All sixteen seat at the authored width on Desktop with room to spare** — which is exactly what `DESIGN.md:462` requires: *"All sixteen faders remain visible at 1920×1080."*

Two consequences you must carry:

1. **The shipped `egui::ScrollArea::horizontal` at `eframe_graphical_window.rs:512` is the divergence, not the baseline.** It exists only because 176 is more than double the authored 82. `MixerStripBank` retires it. **An implementer who reproduces the shipped behavior reproduces the defect.** There is no `ScrollArea` in anything you write.
2. **The overflow rule bites at SteamDeck, not Desktop.** SteamDeck main is 960 with a 16 px inset → **928** content, and `1372 > 928`, so that policy narrows width and pitch together. The floor is reachable: sixteen columns at the 48 px minimum interactive target with the authored 4 px gutter need `15 × 52 + 48 = 828 ≤ 928`.

Uniform narrowing. **Never scrolling, never elision, never a third hard-coded layout.**

## Context you need

- **`cross-wp-findings.md` F-09** — the specification for this work package, including the correction that establishes 82/86 as authored and 176 as invented. Also **F-10 item 10** (both sub-band constants lack a policy accessor) and **F-11** (see the ruling below).
- **`.kittify/crest-spec/contexts/shell.yaml`**:
  - `valueObject.Shell.ShellComposition` — `MixerStripBank` is declared eighth in `from`, after `PatchStripRow`, with **five** invariants: the many-to-one region binding, entries-are-groups, two-level titling, two-level unavailable-marking, and allocate-don't-consume. Those five are your acceptance criteria; read them in the file, not paraphrased here.
  - `valueObject.Shell.ViewportDensityPolicy` — `state.mixerColumn` and its three invariants, including *"mixerColumn resolves through this policy and nowhere else"* and *"where they already fit it keeps the measured values rather than stretching them."*
- **`src/shell/visual/compositions/section.rs`** (WP05) — `render_entries`, `mark_unavailable`, `entry_gap_px`, `inset_scope`, and the `probe` test harness. These are the mechanisms you compose with.
- **`src/shell/visual/compositions/patch_strip_row.rs`** (WP05) — `render_row` and `component_state`. `render_row` is the path that goes through the WP01 selector and derives `ComponentState` from the projection; it is what a column's cells go through.
- **`src/shell/visual/controls/fader.rs`** (WP03) — `column_pitch_px`, `column_width_px`, `STRIP_COLUMN_COUNT`, `bands`. This is where the surface-local column derivation lives today.
- **`src/shell/visual/density.rs`** — the file you are extending. Read `split`, `rhythm`, `utility_control`, and `PolicyProvenance` before adding anything: the shape of what you add must match what is there.
- **`src/adapter/eframe_graphical_window.rs`** — `paint_mixer_workspace:488` and the two constants at `:28-37`. Read it to know what you are replacing. **Do not modify it; it is WP06's.**
- **`DESIGN.md:462-467`** — sixteen faders visible at 1920×1080; compact columns with hairline separators, not cards; **empty tracks remain visible and configurable**; Up/Down moves Volume → Pan → Mute → Solo.

## The seams you will hit, and the rule for each

These are known. They are named here so they get raised rather than approximated.

**1. The fader already paints a label at the top of the column.** `fader.rs` paints `view.label()` in its own label band — the projected label of the *Level* control, which is `"T00 Level"` and which **F-06** records as overflowing a compact column where Figma shows only `T00`.

The bank's two levels are the **legend** and the **column's track identity**. The fader's control label is neither: it is F-06's clipping defect, owned by **WP06** and asserted by **WP08 T044**. Place the bank's column title band so the two do not collide, **do not edit the fader to drop its label**, and record the resulting double-naming in your completion notes for WP06.

**2. Pan selects a second `Fader`.** `control_for(Continuous, VerticalStrip)` resolves to `Fader`, so a column's Pan cell renders as a fader while the specimen shows a 19 × 100 label-and-value frame (`42:26` → `0:8`). **The bank supplies the role; it does not get to pick a different control for one cell** — that is the `ComponentControl` invariant, and bypassing the selector is what WP08's totality proof exists to catch.

What the bank *does* own is the extent each cell is allocated. `fader::bands` already degrades honestly when a column is short (*"A column too short to hold all four leaves the track with no height, and the paint below skips it"*), so allocating the Pan cell its authored extent is legitimate. Choosing a different control for it is not. **Raise the divergence; do not bypass the selector.**

**3. `render_group` double-insets.** `section::render_group` opens `inset_scope`, which applies the authored content inset. Inside a column you have already inset, that insets twice. Compose from `section::render_entries` and `section::mark_unavailable` directly, and own the column's title band yourself.

**4. `Section`'s own doc cites `42:21` as its specimen.** On MIXER the section header *is* the Mixer Legend. Once the bank fills `MainWorkspace` on MIXER, the legend moves to the bank. **It moves; it is not duplicated.** Do not paint a second, differently-shaped header band.

## The ruling on `WORKSPACE_TITLE_ROW_PX` — it stays with WP06

F-11 raises the panel title band deriving to 34 px against the adapter's shipped `WORKSPACE_TITLE_ROW_PX = 42.0`, an 8 px visible shrink, and asks whether that belongs here alongside `mixerColumn`. **It does not.** Three reasons, in order of weight:

1. **The crest-spec declares no workspace-title member.** `ViewportDensityPolicy.state` gained exactly one member in the F-09 amendment: `mixerColumn`. Adding a second, undeclared policy accessor would be code reaching ahead of the declaration it derives from — the precise inversion the project forbids. A workspace-title accessor needs a crest-spec amendment first.
2. **The 42 is not authored either, and 34 is nearer the design than 42 is.** Measured from the live file: the Inspector (`42:191`, 420 × 896) opens with `I Label` at y=20, 12 px tall, and its next element `I Selection` at y=46 — a **26 px** band. The design file authors no 42 px title row anywhere on that screen. So the "shrink" moves *toward* the design, not away from it. It is the same class of unauthored floor as `MIXER_TRACK_MIN_WIDTH_PX` and it retires the same way.
3. **Neither WP09 nor WP06 owns the consumers.** The derived band lives in `section.rs::header_height_px` and `utility_inspector_panel.rs` — WP05's files, approved and closed. An accessor authored here would ship dead.

So: **WP06 keeps it**, as F-11 assigned it, and deletes the constant with the rest of the adapter's paint — recording the 42 → 34 change as the intended retirement of an unauthored floor rather than a regression to compensate for. If a reviewer decides the authored 26 px band should be reproduced exactly, that is a new policy member and therefore a crest-spec amendment plus a follow-on work package. **It is not a literal restored in either place.** This ruling is recorded so the constant is not left unowned a third time.

## Two standing cautions

**Do not quote a test baseline as authoritative.** Three numbers have circulated for one tree (741 / 768 / 796), the 741 an earlier dispatch used was unsourced, and the mechanized `baseline-tests.json` capture is broken mission-wide (**F-12**). **Measure your own baseline by stashing**, before you write anything:

```bash
git stash push --include-untracked
cargo test --release > /tmp/wp09-baseline.log 2>&1; echo "exit=$?"
git stash pop
```

Never pipe test output through `head` or `tail` — the pipe reports the pager's exit code, and a "green" recorded that way is a lie.

**If a specimen is missing or ambiguous, raise it. Do not approximate.** Every work package in this mission that approximated an unauthored value instead of raising it was rejected. An approximated value looks authoritative and is worse than a missing one, because nobody will know to re-check it.

---

## Subtasks

### T047 — Author the mixer-column geometry on `ViewportDensityPolicy`

**Purpose**: give the sixteen-way division of the main surface one home, so it stops being derived at a call site.

**Steps**:

1. Add a `MixerColumnGeometry` value to `src/shell/visual/density.rs` carrying the column's **width**, its **pitch**, and the **floor** it may not narrow past — shaped like the `ControlGeometry` and `ContentRhythm` values already there, with the same doc-comment register.
2. The floor is `token::MIN_INTERACTIVE_TARGET_PX`. **Read it; do not restate it.** The column count is `MixerTrackId::COUNT`, read the same way — `fader.rs::STRIP_COLUMN_COUNT` already sets that precedent and gives the reason.
3. Add `pub const fn mixer_column(self) -> MixerColumnGeometry`, exhaustive over the two policies with no wildcard.
   - **Desktop returns the measured values unstretched: width 82, pitch 86.** `15 × 86 + 82 = 1372 ≤ 1452`. The crest-spec is explicit that a policy where sixteen already fit *"keeps the measured values rather than stretching them"* — the design file leaves 80 px of slack at the right of the grid and that slack is authored, not spare room to consume.
   - **SteamDeck narrows width and pitch together** until sixteen seat in 928 px, holding the authored `SpacingStep::S4` gutter (`pitch − width == 4`, which is exactly what the Desktop measurement shows: `86 − 82`). Show your arithmetic in the doc comment the way the existing accessors do.
4. Record provenance the way `density.rs` already does: Desktop is measured from the authored design, SteamDeck is authored from it. Do not blur them.
5. Write the assertions, in `density.rs`'s existing test module and in its existing style:
   - Sixteen columns and their gutters fit the main-surface content width at **both** policies.
   - Column width is at or above the floor at **both** policies.
   - Desktop reproduces the Figma measurement exactly — this is the test that fails if someone later "optimizes" the column to consume the authored slack.
   - The gutter is the authored spacing step at both policies, not a number.

**Files**: `src/shell/visual/density.rs` (~90 lines added)

**Validation**:
- Every existing `density.rs` test passes unmodified.
- No new literal outside the two authored policy tuples.
- The accessor is `const fn` and exhaustive, matching its siblings.

---

### T048 — Retire the fader's surface-local column derivation onto the policy `[P]`

**Purpose**: satisfy the crest-spec invariant that mixer-column geometry *"resolves through this policy and nowhere else."*

**Steps**:

1. `fader.rs` currently derives its own column geometry:

   ```rust
   pub(super) fn column_pitch_px(density: &ViewportDensityPolicy) -> f32 {
       (density.split().main_px - density.rhythm().inset_px * 2.0) / STRIP_COLUMN_COUNT as f32
   }
   ```

   That is a surface-local division of the main surface, and the amended crest-spec rejects it *"on the same terms as any other"* resolution constant outside the policy. It lands at **90.75 px pitch on Desktop against the specimen's 86** — it stretches the columns into the authored slack.

2. **This is a declared narrow edit into WP03's owned file.** Change **only the two function bodies** to delegate to `density.mixer_column()`, and **only the doc comments that the delegation makes false**. Keep both signatures, keep `pub(super)`, keep `STRIP_COLUMN_COUNT`. Touch nothing else in `fader.rs`, reorder nothing, and do not enter any other WP03 file.
3. The module doc's *"No accessor on `ViewportDensityPolicy` carries mixer-strip geometry"* and the *"lands at 90.75 px pitch"* paragraph both become false statements the moment you land T047. Correct them; leave the rest of that doc alone.
4. **`meter.rs` needs no edit.** It imports `column_width_px` from `fader`, so one delegation corrects both controls. Confirm that by reading, then leave it alone.
5. **WP03's two assertions must pass unmodified.** They will, and here is the arithmetic so you can check before you run:
   - `sixteen_columns_and_their_hairlines_fit_the_main_surface`: Desktop `16 × 82 + 15 × 4 = 1372 ≤ 1452`; SteamDeck at the narrowed values must likewise land at or under 928.
   - `a_column_is_never_narrower_than_the_authored_interactive_target`: Desktop `82 ≥ 48`; SteamDeck likewise.

   If either fails, **your policy values are wrong — fix the policy, not the test.** Editing a WP03 assertion to accommodate this work package is the NFR-005 failure in a different file, and it is a reject.

**Files**: `src/shell/visual/controls/fader.rs` (two function bodies and two doc paragraphs — declared narrow edit, not owned)

**Validation**:
- `git diff src/shell/visual/controls/fader.rs` shows two bodies and their docs, nothing else.
- No WP03 test modified.
- `meter.rs` unmodified.

---

### T049 — Build the mixer strip bank as a group of groups `[P]`

**Purpose**: the eighth composition. The one that arranges groups where every other composition arranges controls.

**Steps**:

1. Create `src/shell/visual/compositions/mixer_strip_bank.rs` with the family signature: `fn render(&mut Ui, &GraphicalShellProjection, &ViewportDensityPolicy) -> CompositionIntent`.
2. Resolve `SurfaceId::MixerMain` from the semantic model. Its controls arrive as one **flat** list — sixteen tracks × `MixerTrackParameter::MAIN` (Level, Pan, Mute, Solo).
3. **Iterate `MixerTrackId::ALL`, not the projection's groups.** Sixteen columns exist because the mixer has sixteen tracks, always. `DESIGN.md:462` is explicit: *"empty tracks remain visible and configurable."* A track with no view data is a **visible column carrying a mark**, never an absent one. Driving the loop off the projection would make an empty track disappear, which is the same misrepresentation C-003 forbids.
4. Partition the flat list by the track identity in each control's `path()` — `SemanticControlId::Mixer(MixerControlId::Track { track_id, .. })` — **not by index arithmetic**. Chunking by four assumes a projection shape the bank does not own; reading the projected identity is the honest read and survives a projection that reorders. Match exhaustively, with no wildcard.
5. **Allocate; do not consume.** Compute the content rect once (main surface less the authored inset on both sides), then place column *i* at `content.min.x + i * pitch` with width `width_px`, both from `density.mixer_column()`. Run each column's contents inside a `UiBuilder::new().max_rect(column)` scope so a column paints inside its own bounds.
6. Each column's cells go through `patch_strip_row::render_row` at `PresentationRole::VerticalStrip` — via `section::render_entries`, which already handles the `visible()` filter, the entry gap, and the empty case. That path goes through the WP01 selector and derives `ComponentState` from the projection. **No direct control call bypasses it.**
7. **Paint the hairline separators.** `DESIGN.md:462` — compact columns with hairline separators, not cards. `fader.rs` says where they belong: *"What is painted in it is a hairline, which belongs to the composition that stacks the columns, not to a column."* They go in the gutter, through the authored rule primitive.
8. **No `ScrollArea`, in either axis.** The shipped horizontal one is the defect this composition retires; a vertical one would hide a column's lower cells and violates the same invariant.
9. Aggregate every column's intent into one `CompositionIntent`.

**Files**: `src/shell/visual/compositions/mixer_strip_bank.rs` (~230 lines)

**Validation**:
- Sixteen columns are painted whatever the projection carries.
- Every extent traces to `density.mixer_column()` or an authored token. Zero literals.
- No `ScrollArea`, no `SemanticAction`, no wildcard match arm.
- Every cell reaches its control through the selector.

---

### T050 — Title and mark unavailable at both levels

**Purpose**: two of the five declared invariants, and two of the three reasons `Section` could not host this.

**Steps**:

1. **Titling, level one — the bank's legend.** The authored `Mixer Legend` is `42:21`: x=24, y=20, 1452 × 36, holding a left run (`42:22`, 179 wide), an unfilled spacer (`42:23`), and a right run (`42:24`, 165 wide, right-aligned at x=1287). That is the same header anatomy `Section` was built against, on the same surface. **Compose the existing vocabulary; do not author a second header shape.** The spacer carries no fill, so nothing is painted between the runs — it is layout, not a leader.
2. **Titling, level two — each column's track identity.** A single header band can name the mixer or name a track, but not both, and an operator reading a column needs to know which track it is. Mind seam 1 above: the fader already paints a control label in that neighbourhood, and it is not yours to remove.
3. **Marking, level one — the bank.** A bank with no track view data at all marks the **bank** unavailable, through `section::mark_unavailable`, which is WP05's shared C-003 mechanism. Do not invent a second marker; a second spelling of "absent" is a second thing to keep in step.
4. **Marking, level two — each column.** A track whose designed row has no view data behind it marks **that row, inside its own column**. The crest-spec gives the reason and it is worth holding onto: *"a bank that could mark only itself would report one empty track as an empty mixer, and a column that could mark only itself would leave the bank silent about which tracks it could not draw."*
5. Neither level invents a value. Not a resting level, not a zero, not a dash that could be mistaken for a real reading. `--` already means *off* in the fader specimen (F-02's correction), so it is not available to mean *absent*.

**Files**: `src/shell/visual/compositions/mixer_strip_bank.rs`

**Validation**:
- Hand the bank a projection with one track's data missing: that column marks, the other fifteen paint, the bank does not mark.
- Hand it a projection with no track data: the bank marks.
- Neither case paints a value that was not in the projection.

---

### T051 — Wire `MixerStripBank` into the composition family

**Purpose**: make the eighth variant real, without disturbing the seven.

**Steps**:

`compositions/mod.rs` is **WP01's owned file, shared with WP04 and WP05 under the operator-approved narrow-edit convention**. Adding an eighth variant genuinely requires more than one arm, so here is the exact and complete list. Make these edits and no others; reorder nothing; do not touch WP04's or WP05's arms.

1. `pub mod mixer_strip_bank;` — inserted in the existing alphabetical order of the `pub mod` lines.
2. The `ShellComposition::MixerStripBank` variant, **after `PatchStripRow`**, matching the order the crest-spec declares in `from`.
3. `ALL_SHELL_COMPOSITIONS` — the variant, in the same position.
4. `SHELL_COMPOSITION_COUNT` — 7 → 8.
5. `canonical_name` — one arm.
6. `region` — join the `MainWorkspace` arm. `Section`, `PatchStripRow`, and `MixerStripBank` all fill it. Extend the doc comment that explains the many-to-one binding to name the third; the crest-spec's own wording is the model.
7. `renderer` — one arm, to `mixer_strip_bank::render`.
8. The module doc says the family is *"closed at seven"*. Correct the count.
9. Two tests name the old count: `the_composition_family_holds_exactly_seven_compositions` (rename and update its assertion) and `iteration_yields_every_declared_composition` (add the arm to its literal match). **These two are the exception to "add only your own arm"** — they are structurally required by an eighth variant, and this prompt authorizes exactly these two and nothing further.

**Files**: `src/shell/visual/compositions/mod.rs` (declared narrow edit, not owned)

**Validation**:
- `git diff src/shell/visual/compositions/mod.rs` shows only the nine items above.
- `every_observed_region_is_filled_by_a_composition` and `only_the_application_shell_fills_the_whole_frame` pass unmodified.
- The family's wildcard and `SemanticAction` scans still pass over the new file.

---

### T052 — Drive the bank through a real render pass and prove sixteen seat

**Purpose**: **constructing a composition without rendering it is vacuous.** Every prior work package in this mission was held to that standard and two were rejected for falling short.

**Steps**:

1. Use `section::probe` — the shared harness WP05 built for exactly this. It paints through a real `egui::Context` with the authored faces installed, against a projection the **production `StateProjector`** builds from a real `AppState`, then reads back the text runs and shape count that actually reached the output. `probe::paint`, `probe::paint_with`, and `probe::projection(TopLevelContext::Mixer)` are reachable from your module. **Do not fabricate a view model** — its fields are private precisely so no surface can invent one, and a test that assembled its input would not be testing the contract the bank receives.
2. **The headline assertion: sixteen columns actually seat, at both viewports.** Read the emitted shapes, derive the painted column extents, and assert:
   - sixteen distinct columns reached the output;
   - the leftmost starts at the content inset and the rightmost ends **at or before** the content edge;
   - no column is narrower than the floor;
   - **nothing is scrolled, clipped, or elided to achieve it.**

   This is the requirement the shipped code silently violates, so assert it against the painted result, not against the policy the paint was supposed to use. A test that re-derives the policy proves the policy consistent with itself.
3. Assert the two-level titling reached the output: the legend's runs, and per-column track identity.
4. Assert both marking levels, per T050's two cases, by reading emitted text.
5. **Make each assertion falsifiable and say how.** The house standard in this mission is that a reviewer can break the code and watch a *named* assertion fail. Check yours by hand: narrow a column past the floor, drop a column, drop the legend, remove a column's mark — each must fail an assertion you can name.
6. Assert the bank builds no `SemanticAction`, reads no `AppState`, and takes no raw viewport size. The family-wide scans in `mod.rs` cover the first; the other two are yours.

**Files**: `src/shell/visual/compositions/mixer_strip_bank.rs` (test module)

**Validation**:
- Every assertion drives a real paint pass. No assertion passes on a composition that was constructed and not rendered.
- Sixteen columns seat at **both** `ALL_DENSITY_POLICIES` entries.
- Each assertion has been falsified by hand at least once.

---

## Branch Strategy

- **Planning base branch**: `feat/crest-component-controls-and-compositions`
- **Final merge target**: `feat/crest-component-controls-and-compositions`, and from there to `main`
- Execution worktrees are allocated per computed lane from `lanes.json`.

## Definition of Done

- All six subtasks complete; `mark-status` recorded.
- `MixerStripBank` renders sixteen columns as groups of controls, through the WP01 selector.
- `ViewportDensityPolicy::mixer_column()` exists, is exhaustive, and returns the measured Desktop values unstretched.
- The fader's surface-local derivation delegates to it, with every WP03 assertion passing unmodified.
- Two-level titling and two-level marking implemented and asserted.
- Sixteen columns proven to seat at both viewports through a real render pass.
- No `ScrollArea` anywhere in what you wrote.
- Zero literals; the guard passes.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, full suite green against **your own stashed baseline**.
- Outside `owned_files`, only the two declared narrow edits — `compositions/mod.rs` (nine listed items) and `fader.rs` (two bodies and their docs). Nothing else.
- Completion notes record: the fader-label double-naming for WP06, the Pan-cell selector divergence, and any specimen ambiguity you raised.

## Risks

- **Reproducing the shipped mixer.** The single risk this work package exists to manage. The shipped `ScrollArea` and the 176 px column look like the baseline and are the defect. An implementer who ports the current behavior forward ships the bug with a new file name on it.
- **Consuming the authored slack.** Dividing 1452 by sixteen is easy, arrives at 90.75, fits, and is wrong — the design authors 82 on 86 and leaves the remainder deliberately. The crest-spec says keep the measured values where they fit; a policy that stretches them fails T047's Desktop assertion.
- **Widening the fader edit.** T048 is two function bodies in an approved work package's file. Anything more is a scope breach, and "while I was in there" is how the review that catches it will describe it.
- **Flattening the bank.** Handing the flat `MixerMain` list to something that arranges controls produces one horizontal run of sixty-four controls that *looks* plausible in a screenshot and has no columns in it. That is the exact failure F-09 ruled out; entries-are-groups is a declared invariant, not a style preference.
- **Driving the loop off the projection.** Sixteen columns come from `MixerTrackId::ALL`. Iterating what the projection supplied makes an empty track vanish, and an operator cannot see that a track they cannot see is empty.
- **A vacuous test.** Constructing the bank and asserting on the return value proves nothing about what was painted. Read the emitted shapes.

## Reviewer Guidance

1. `grep` `mixer_strip_bank.rs` and `density.rs` for hex literals, numeric font sizes, and bare pixel constants outside the two authored policy tuples. Any hit is a reject.
2. `grep` for `ScrollArea` in anything this work package wrote. Any hit is a reject.
3. Check the Desktop policy against the design file: width **82**, pitch **86**. A derived 90.75 is a reject even though it fits — it stretches into authored slack and contradicts the declared overflow rule.
4. `git diff src/shell/visual/controls/fader.rs` — two function bodies and their doc comments, nothing more. Any WP03 test modified is a reject.
5. `git diff src/shell/visual/compositions/mod.rs` — the nine items T051 lists, nothing more, nothing reordered.
6. Does the bank iterate `MixerTrackId::ALL`? Hand it a projection missing one track and confirm the column is **present and marked**, not absent.
7. Do the assertions drive a real paint pass through `section::probe`, and do they read emitted shapes? An assertion on a constructed-but-unrendered composition is a reject.
8. Break something and watch a named assertion fail: narrow a column below the floor, drop the legend, drop a column's mark. If nothing fails by name, the proof is decorative.
9. Are the two recorded findings in the completion notes — the fader-label double-naming and the Pan-cell selector divergence? Silence on a seam this prompt named is a reject.
