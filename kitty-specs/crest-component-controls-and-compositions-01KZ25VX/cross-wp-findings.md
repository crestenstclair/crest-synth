# Cross-WP findings — Phase 4b

Findings raised during implementation and review that **no single work package can close**, recorded here so they reach the work package that owns them rather than being rediscovered.

Each entry names its owner. An entry with no owner is for mission review.

---

## F-01 — Three controls have no authored Figma specimen

**Raised by**: WP02 (choice-row adjacency, toggle), WP03 (meter)
**Confirmed by**: WP02's review, via an independent node-type census of the entire Screens page (`6:3`)
**Owner**: design authority — not a work package

The design file defines exactly five component sets: Context Switch (`30:14`), CLI Hint (`30:31`), CLI Browser Line (`32:44`), Compact Parameter Slider (`33:69`), Compact Mixer Fader (`34:45`).

The census found 202 frames, 188 text nodes, 104 rounded rectangles, 39 instances, 6 vectors — **zero ellipses, zero polygons, zero boolean shapes**, and the six vectors are the ADSR curve. Every binary in the file is a text run.

What is therefore absent:

| Missing specimen | What was shipped instead | Flagged in source |
|---|---|---|
| Toggle (no toggle, switch, checkbox, or stepper set exists) | Authored `ON`/`OFF` words per `DESIGN.md:468`, plus one filled/hollow shape channel | Yes |
| Choice-row adjacency affordance (the Compact Parameter Slider's twelve variants add no directional mark in any state; the only two directional glyphs in the file are a `→` row-type marker in the browser Meta column and arrowheads on an interaction-map diagram) | Row rendered without direction marks | Yes |
| Meter (the Mixer screen holds sixteen Fader instances and no level readout, segment ladder, or peak mark) | Read-only twin of the mixer column — same geometry, no cap, since the cap is the grab affordance | Yes |

None of the three was approximated silently. **The decision this leaves open is whether Phase 4 ships them as flagged minimums or the design file is extended first.** That is a product call, not an implementation one.

Also confirmed by the census: **the file contains no Steam Deck or compact-viewport screens at all.** This independently validates every control asking `ViewportDensityPolicy` for compact geometry rather than reading it off a single-viewport frame — there is nothing to read.

---

## F-02 — The mixer mute/solo readout is claimed by two work packages

**Raised by**: WP02's review
**Owner**: WP05, before the mixer strip composes

`control_for` routes `(Toggle, VerticalStrip)` to WP02's `Toggle`. But the authored `State` run that renders mute and solo in the mixer column is a child of the **Compact Mixer Fader** component (`34:45`) — WP03's specimen, not WP02's.

The consequence is a spelling divergence already in the tree:

- WP02's toggle paints `ON`/`OFF` there.
- The design file authors `M -- · S --` at rest, `M ON · S --` when muted, `M -- · S ON` when soloed.

WP02 is not at fault — its own specimen genuinely does not exist (F-01) and it said so. But **the seam needs one owner before the mixer strip composes**, or the mixer column will read differently from the design file.

### Settled by WP05 and confirmed in review

**WP02's `Toggle` owns the mixer column's mute/solo readout. The fader does not.**

The reason is not arbitrary: a strip's mute/solo on-ness is a **value** the control paints from `ParameterValue::Toggle`. Also handing the fader `Muted`/`Soloed` as a **state** would give one fact two representations that can disagree. The Inspector reads both toggles by `MixerControlId::Track { track, Mute|Solo }` out of the projection, never from the state a fader was handed (`controls/mod.rs:153` already said this).

The residual divergence from the authored `M ON · S --` is entirely a **projected label** difference (`T00 Mute` vs `M`), built in `semantic_graphical_view_model.rs` — the same unowned file as F-06. Recorded in T027, not approximated.

### Correction — an earlier version of this note was wrong

This note previously claimed `--` is authored in the "this fact is not present" role. **That is not what the specimen shows**, and the bullets three lines above contradict it: the design authors `M -- · S --` **at rest**, so in the fader specimen `--` means *off*, not *absent*.

As originally written, this note would have justified exactly the collapse WP05 correctly refused. The shipped toggle spells off as `OFF` and reserves `--` for *absent*. **Making "off" and "no data" identical is precisely the T026 failure**, so keeping them distinct is deliberate and correct — even though it means the mixer column does not match the specimen's spelling until the projected label is fixed upstream.

---

## F-03 — Two Phase 4a primitives diverge from the design file

**Raised by**: WP02's review
**Owner**: mission review — outside every Phase 4b work package's owned files

The Compact Parameter Slider's variants render:

- **Focused** as `>` plus a cyan underline, and **Editing** as `*`. The shipped `focus::cursor` primitive paints `>` for both Focused and Adjusting.
- **Disabled** as dimmed with no word. WP01's state vocabulary gives Disabled `NonColorSignal::Word("Locked")`.

These live in `src/shell/visual/primitives/focus.rs` and `src/shell/visual/state.rs` — Phase 4a files that no Phase 4b work package owns. WP02 composed the primitives rather than redrawing them, which is what it was instructed to do. **Do not send these back to a control work package.**

---

## F-04 — The fader specimen paints mute and solo simultaneously

**Raised by**: WP03
**Owner**: a later mission — C-002 forbids closing it here

The Compact Mixer Fader specimen shows `M ON · S --`: mute and solo are independently valued. Single-valued `ComponentState` cannot express two simultaneous states. WP03 raised it rather than adding a state, which C-002 forbids.

---

## F-05 — `SemanticControlViewModel` cannot support T009's boundary criterion

**Raised by**: WP02
**Owner**: a later mission — C-002 forbids extending the projection here

T009 asks that at the first or last choice, the adjacency affordance shows unavailable. The view model carries **no choice set, no neighbour, and no boundary flag**, so the criterion is unimplementable from current view data regardless of visual treatment.

This is the mission's own declared edge case working as intended: *"A control needs a value the view model does not carry. The control declares the gap; the view model is not extended in this mission, and no control invents state to fill it."*

---

## F-06 — Compact-viewport label overflow

**Raised by**: WP03
**Owner**: WP06, and **asserted by WP08 T044**

The projected label `"T00 Level"` overflows a compact-viewport mixer column. It is currently clipped at the column edge rather than bleeding into its neighbour, because everything paints into a painter clipped to the column. Figma's column shows only `T00`.

WP03 assigns the fix to the composition layer. **WP08's T044 asserts no clipped or overlapping text at either viewport**, so this must actually be resolved by then — a note is not enough.

---

## F-07 — Value formatting is duplicated until the adapter is reduced

**Raised by**: WP02
**Owner**: WP06

WP02 reproduced the adapter's private `semantic_value_label` exactly (3 dp, `ON`/`OFF`, locator) rather than inventing a second format. The duplicate retires when `src/adapter/eframe_graphical_window.rs:816` is converted during the adapter reduction.

---

## F-08 — `NON_TRACK_STATES` is hand-written where its sibling derives

**Raised by**: WP01's review
**Owner**: whoever next adds a `ComponentState`

`controls/mod.rs:139` `MIXER_STRIP_STATES` derives from `ALL_COMPONENT_STATES`. `:153` `NON_TRACK_STATES` is a hand-written `[ComponentState; 7]`.

A tenth state would grow the mixer set automatically but silently omit itself from the non-track set, and no current test catches it because `Fader` and `Meter` accept everything. Out of scope for Phase 4b; worth deriving when a state is next added.

---

## Settled — do not relitigate

- **Role shape ranges are not binding.** `research.md` R-01's "Shapes it can select" column is a characteristic-shape summary, not a constraint; the crest-spec states the selection invariant (`shell.yaml:195-198`) with no per-role shape range. WP01's T004 table is authoritative: `Asset→BrowserRow` and `Identity`/`Surface→ParameterRow` are askable in `PanelEntry`. Confirmed by WP01's review against three independent lines of evidence.
- **Three pairs are genuinely un-askable and pinned as data**: `Choice`, `Asset`, and `Surface` in `VerticalStrip`.
- **`(Toggle, ModalEntry)` resolves to `ModalOption`**, so the toggle is reachable in three roles, not four. The WP prompt text saying otherwise is wrong; the pinned table is right.
- **The baseline's "1 pre-existing test failure" is not a Rust test.** It is a malformed CLI invocation in the capture harness (`"<declared-command>"` / `"For more information, try '--help'."`). The Rust suite was green before Phase 4b and every failure since is owned by the work package that caused it.

---

## W-01 — Waiver: `--skip-review-artifact-check` used to approve WP03 cycle 2

**Date**: 2026-08-03
**Gate waived**: the review-artifact governance check on `move-task --to approved`
**Waived by**: the WP03 cycle-2 reviewer, recorded in its approval note
**Class**: process/tooling gate — the charter permits self-service waiver here provided it is committed in-repo with rationale and flagged in the next human-visible report. It is **not** a product or proof gate (acceptance validation, live-demo gate, real-time contract proof, or `crest-spec doctor`), none of which may be waived autonomously.

### What happened

`spec-kitty` refused the approval with `WP03 has a rejected review artifact (review-cycle-2.md)`.

### Why the refusal was wrong

`review-cycle-2.md` is not a second rejection. It is the **implementer's acknowledgement copy of the cycle-1 rejection**, written when it set `review_status: "acknowledged"`. Its YAML frontmatter carries `cycle_number: 2` and a stale `verdict: rejected` inherited from the file it copied.

Verified independently by the orchestrator: stripping the frontmatter, the cycle-2 body is **byte-identical to `review-cycle-1.md`** (14,093 bytes each). Both bodies open `# WP03 review — cycle 1`. No cycle-2 rejection was ever issued — cycle 2 was reviewed and approved.

### The underlying tooling defect

**Review-artifact numbering runs one ahead of the review cycle in this mission.** The acknowledgement of cycle-N feedback is written as `review-cycle-(N+1).md` and inherits `verdict: rejected`, so the next approval attempt is blocked by the WP's own acknowledged feedback. The review prompt compounds it by directing a genuine new rejection to `review-cycle-3.md`.

Expect this to recur on every work package that goes through a rejection cycle. WP04 is in cycle 2 now and will hit it.

### Consequence to watch

The board still reports `WP03 ⚠ review artifact: verdict=rejected`. If the merge gate reads the same signal it will refuse — and at that point the override would be applied to a **merge** gate, a different and more serious class than the per-WP approval gate. Resolve the artifact frontmatter before merge rather than waiving again there.

### Fourth occurrence — WP09 cycle 1, 2026-08-03

Reproduced exactly as predicted, and recorded here **before** it blocks anything so the next reviewer does not rediscover it.

The WP09 cycle-1 rejection was written to `review-cycle-1.md` and `move-task --to planned --review-feedback-file` additionally emitted `review-cycle-2.md`. Verified by the reviewer: **the two bodies are byte-identical** (17,285 bytes each), both opening `# WP09 review — cycle 1`; only `review-cycle-2.md`'s frontmatter differs, carrying `cycle_number: 2` and an inherited `verdict: rejected`.

**There is exactly one WP09 rejection, and it is cycle 1.** When WP09 returns for approval, `spec-kitty` will refuse with `WP09 has a rejected review artifact (review-cycle-2.md)`. That refusal is the tooling defect, not a second rejection — inspect both files, confirm the bodies match, and say so plainly in the approval note.

This is the fourth occurrence and the first recorded proactively rather than after a blocked approval. The defect is now confirmed to fire on the **first** rejection cycle of a work package, not only on later ones.

---

## F-09 — The mixer strip has no composition, and an eighth variant is required

**Raised by**: WP04 (as a scope gap), independently confirmed by WP05 and by two architecture reviews
**Owner**: mission level — must be authored before WP06 can finish
**Status**: ruled on; the cheap alternative was tested and rejected

`paint_patch_workspace` lands in `Section` — demonstrated and tested by WP05. **`paint_mixer_workspace` lands nowhere in the closed family of seven.** A `Section` at `VerticalStrip` is one track **column**; what has no composition is the **strip** — sixteen columns side by side.

### The cheap alternative was considered and does not work

The orchestrator proposed giving `Section` a layout axis, so the strip would be a `Section` of sixteen `VerticalStrip` entries laid out horizontally. Both suspected blockers turned out to be neutral: vertical stacking **is** an unparameterized layout choice, an axis is a *structural* argument rather than a visual value so FR-004 would not forbid it, and the crest-spec's `region()` binding is explicitly many-to-one (`Section` and `PatchStripRow` already share `MainWorkspace`), with no hardcoded `7` anywhere in `.kittify/crest-spec/`.

**It fails for a different reason.** `Section`'s entries are typed `&[SemanticControlViewModel]` — *controls*. A mixer strip's entries are *columns*, and a column is itself a titled group in its own role, i.e. a `Section`. **The strip is a group of groups.** An axis flag does not change entry type: a horizontal `Section` handed the flat `MixerMain` list would paint all sixteen tracks' controls in one horizontal run with no per-column title and no column boundary. The gap is nesting and grouping, not direction.

Three further blocks, each independently sufficient:

1. `Section` paints **one** header band with one title. A strip has titles at two levels — per-column, plus the authored `Mixer Legend` (`42:21`).
2. `render_entries` marks the **group** unavailable on zero entries. A strip needs that per-column **and** per-strip.
3. Decisively: **vertical stacking consumes extent; sixteen columns must divide it.** The adapter carries `MIXER_TRACK_MIN_WIDTH_PX = 176.0` as a local literal, and 16 × 176 = 2816 exceeds both main surfaces (Desktop 1500, SteamDeck 960). `ViewportDensityPolicy` has **no** mixer, column, or strip concept at all. Flipping an axis changes the composition from *consuming* to *allocating* — structural, not a parameter.

It cannot stay in the adapter: the `AppWindow` port invariant (`shell.yaml:377`) says the window "decides no paint, layout, band height, or state visualization", and sixteen-column division is layout. It cannot go to `ApplicationShell` (`WholeFrame`, structural bands only) without putting `MixerTrackId` partitioning into the frame composition.

### To author the change

1. Add one name to `valueObject.Shell.ShellComposition.from` (`shell.yaml:224-226`). An eighth variant bound to `MainWorkspace` is legal — `region()` is many-to-one.
2. Add a mixer-column geometry member to `ViewportDensityPolicy` so the 176 px literal resolves through policy rather than living in the adapter.
3. Update `spec.md` FR-004 (it enumerates the seven by name) and SC-002 ("all seven named compositions").
4. **No gallery-page change needed** — `StripPanelAndFooter` already exists and the coverage invariant is generic.

### Authored — the eighth variant is `MixerStripBank`

Declared in `.kittify/crest-spec/contexts/shell.yaml` before any implementing Rust exists. Doctor after the amendment: 130 resources, 102 requirements, 31/31 completion checks, OK.

- `valueObject.Shell.ShellComposition.from` gains **`MixerStripBank`**, inserted after `PatchStripRow` so the `from` list stays grouped in region order. It binds to `MainWorkspace`, the third composition to do so, and five new invariants carry its rigor: the many-to-one binding, entries-are-groups, two-level titling, two-level unavailable-marking, and allocate-don't-consume.
- **Name**: the crest-spec's own `presentationRole` vocabulary already spends "strip" on *one* column — *"a fader in a mixer strip"* (`shell.yaml:190`), role `VerticalStrip`. So the bank is named for a bank of strips, matching `BusReturnBank`'s established sense of a fixed set of N. F-09's prose above uses "strip" for the whole and "column" for the part; the declaration uses the crest-spec's sense, and both readings name the same structure.
- `valueObject.Shell.ViewportDensityPolicy.state` gains **`mixerColumn`** — authored column width and pitch, the floor a column may not narrow past, and the overflow rule — plus three invariants. The rule is **uniform narrowing, never scrolling and never elision**, floored at the authored minimum interactive target (48 px), with each policy proven by validation to seat sixteen at or above that floor.

### Correction — `MIXER_TRACK_MIN_WIDTH_PX = 176.0` is not an authored value

F-09's arithmetic above (`16 × 176 = 2816` exceeds both surfaces) is right about the number and wrong about its provenance. The constant's own comment says so: *"sub-band splits the authored vocabulary does not declare… named rather than resolved because there is nothing yet to resolve them from"* (`eframe_graphical_window.rs:28-37`). It is an implementer's floor, not a measurement.

**The design file was measured.** Figma `42:25` "16 Fader Grid" (inside `42:20` "Faders", 1500 × 896, inset 24 → 1452 content) holds sixteen `Fader / Txx` instances at **width 82, pitch 86**, x = 0, 86, 172 … 1290. So `15 × 86 + 82 = 1372 ≤ 1452`: **all sixteen seat at the authored width on Desktop with room to spare**, which is what `DESIGN.md:462` requires — *"All sixteen faders remain visible at 1920×1080."*

Two consequences for WP06:

1. **The shipped `egui::ScrollArea::horizontal` at `eframe_graphical_window.rs:512` is the divergence, not the baseline.** It exists only because 176 is more than double the authored 82. `MixerStripBank` retires the scroll along with the constant; it does not reproduce it.
2. **The overflow rule bites at SteamDeck, not Desktop.** `15 × 86 + 82 = 1372 > 928` (960 main − 2 × 16 inset), so the SteamDeck policy narrows width and pitch together. The floor is reachable: sixteen columns at the 48 px minimum target with a 4 px gutter need `15 × 52 + 48 = 828 ≤ 928`.

### Urgent for WP06

`Section::render` on MIXER resolves `main_for` → `MixerMain` and paints all sixteen tracks **flat at `ListedRow`**. Wiring it into `mainWorkspace` as-is regresses the operator from sixteen columns to one long vertical list.

Compounding it: the shipped adapter drives a **live meter today** (`eframe_graphical_window.rs:542-570`, an `egui::ProgressBar` fed from `audio_observation.track(track_id).rms()`). WP06 therefore cannot simply delete `paint_mixer_workspace` — no composition replaces it, and C-001 puts the meter out of scope. See F-10.

---

## F-10 — T027: ten designed structures the projection does not drive

**Raised by**: WP05
**Owner**: Phase 5 — this list is its declared input (`plan.md`, `tasks.md:152`)

Transcribed here verbatim from the event log because `status.events.jsonl` is machine-managed and **has already diverged between checkouts** (16 events in the primary worktree vs 101 in coord). The document `plan.md` calls "the real input to Phase 5" must not survive in one mutable log.

**Marked unavailable** (the structure is designed, the data is absent, so the composition marks it):

1. **PATCH Utility `MASTER VOLUME`** — master gain *is* projected, but as `MixerControlId::Global{MasterGainDb}` on the MIXER Inspector. The PATCH Utility surface has no path to it.
2. **MIDI INPUT** — `Patch` carries `MidiChannel`; `patch_utility_paths` projects only `PatchOutputParameter`.
3. **VOICE LIMIT** — no state, no descriptor, no path anywhere.
4. **The requested value of a row mid-structural-edit** — `SemanticLifecycleStatus` carries only the target graph revision. Marked in the lifecycle band.

**Omitted** (designed structure with nothing behind it, so the composition omits it):

5. **The Patch strip row's authored right-hand per-row action hint** — no per-row hints exist; `validActions` are global to the focus.
6. **The Inspector's three-line help block**, including "SELECT enters multi-select", which the reducer cannot do.

**Recorded defects and gaps:**

7. `MixerControlId::Track` is unreachable from `mixer_inspector_paths` (sends/returns/globals only), so the Inspector's cursor, value, range, mute, and solo are resolved from `MixerMain` by identity.
8. **Global rows project `descriptor.name()` (`masterGainDb`) as their label** — a serialization key rendered on screen. This is user-visible.
9. **The mixer meter is not drivable in this slice.** `AudioObservationSnapshot` has no path to any composition and `MixerTrackParameter::MAIN` has no meter. With C-001 putting audio out of scope, WP03's `Meter` is production-unreachable — `(Identity, VerticalStrip) → Meter` never fires in the shipped app. WP08's T041 asserts *selector* reachability, which genuinely holds; production reachability is a different claim and is currently false.
10. **Sub-band constants** `WORKSPACE_TITLE_ROW_PX` and `MIXER_TRACK_MIN_WIDTH_PX` still have no `ViewportDensityPolicy` accessor.

**Additional gaps found in review, not in WP05's list:**

11. The Utility panel's authored hint line is silently dropped.
12. The `M`/`S` label divergence WP05's note claims to have recorded but did not (see F-02).
13. `numeric_range` and `unit` are projected but never painted.

---

## F-11 — Two derived band heights shrink shipped bands

**Raised by**: WP05 (the section header), and by WP05's review (the panel title — unraised by WP05)
**Owner**: WP06, which is structurally guaranteed to hit both constants

- **The panel title band derives to 34 px against the adapter's shipped `WORKSPACE_TITLE_ROW_PX = 42.0`** — an 8 px visible shrink of the side-panel title row the moment WP06 swaps the adapter. WP05 did not raise this one, and it is larger than the one it did.
- **The section header band**: WP05 reported a 30 px vs authored 42 px gap, but review measured the real divergence at **~2 px, not 12** — `render_group` adds the entry gap *after* the band, so it composes to 44 against an authored 42 on desktop, and to *exactly* 42 at `PanelEntry`. WP05 compared its band-only figure against the authored band-plus-gap figure and **overstated its own drift**.

WP05's three other raised divergences are forced by the vocabulary and are not drift: no 12 px SemiBold face exists, no 20 px spacing step exists, and `HINT_SEPARATOR` is the authored Phase 4a separator.

---

## F-12 — The mechanized test baseline is unreliable

**Raised by**: WP05's review
**Owner**: mission tooling — treat every reported baseline as unverified

Three separate baseline numbers circulated for the same tree (741, 768, 796). **768 is correct**, reproduced twice at mission base `e2ee4a4`; WP05's delta is exactly +28. The **741 figure the orchestrator passed in the WP05 dispatch is unsourced** — it matches no ref reachable from that lane, and the planning base measures 668.

Separately, WP05's mechanized `baseline-tests.json` **never ran the test command** (`total:1 / passed:0`, a CLI usage error) and targets a commit that is *not an ancestor of the mission branch*. This is the same capture defect that produced the mission's bogus "1 pre-existing test failure".

**Every WP must measure its own baseline by stashing, and no dispatch should quote a baseline as authoritative.**

---

## F-13 — `cargo test --release` cannot compile this tree, and that is the root cause of F-12

**Raised by**: WP09
**Confirmed by**: the orchestrator, independently
**Owner**: closed for this mission's artifacts; the underlying source defect is unowned

```
error[E0609]: no field `debug` on type `&mut eframe::egui::Style`
   --> tests/component_vocabulary.rs:625:41
    |
625 |         context.style_mut(|style| style.debug.show_interactive_widgets = true);
```

`egui` gates `Style::debug` behind `#[cfg(debug_assertions)]`, so the release profile does not have the field. **Pre-existing since `589fa01` in the previous mission** — nothing in Phase 4b caused it.

### Why this matters more than it first looks

**Ten commands across five mission artifacts told work packages to measure with `cargo test --release`.** Every one of them returns a compile error and runs **zero tests**.

That is the mechanism behind F-12. The mission's mechanized `baseline-tests.json` captures recorded `total:1 / passed:0` with a CLI-shaped error rather than a test run, and three different test counts circulated for the same tree. Work packages that dutifully ran the command they were given got no measurement at all, and one of them — the mission's own bogus "1 pre-existing test failure" — propagated into four dispatches before being disproved.

**A baseline command that cannot compile fails silently in exactly the way a passing baseline looks.**

### What was corrected

All ten occurrences replaced with the debug-profile form, in `quickstart.md`, `tasks.md`, and the WP06, WP08, and WP09 prompts.

### Acceptance was never at risk

The **declared** validations in `.kittify/crest-spec/proof/validations.yaml` do not use `--release`:

```yaml
command: [cargo, test, --test, component_vocabulary, --, --nocapture]
command: [cargo, test, --test, component_composition, --, --nocapture]
```

So `spec-kitty accept` would have passed regardless. The divergence was between the crest-spec's declared commands and the prose that work packages actually read — which is its own lesson: **the artifacts a human or agent reads drifted from the artifact the tooling executes**, and only the executed one was right.

### Still open

`tests/component_vocabulary.rs:625` remains release-incompatible. No work package in this mission owns that file except WP08, and only for the page-reachability change. Either gate the line behind `#[cfg(debug_assertions)]` or drop it — but that is a decision for whoever next owns the file, not a Phase 4b fix.

---

## F-14 — The shared composition probe is blind to two whole classes of defect

**Raised by**: WP09's review, via a sixteen-mutation sweep against the production suite
**Owner**: mission review, and **WP08** if its vocabulary proof is to mean what it says
**Status**: reproduced first-hand; every claim below was applied to a clean tree and run

`section::probe` is the harness every Phase 4b composition proves itself with. Two blind spots in it are not any one work package's to close, because they weaken WP04's, WP05's, and WP09's assertions identically.

### 1. Non-text shapes are never asserted, so hairlines can vanish silently

`probe::Painted` **does** carry `shapes: usize`, documented as *"How many non-text shapes were emitted"* (`section.rs:377`). **No composition test in the mission reads it.** The controls layer does — `browser_row.rs:313` asserts `shown.shapes < 16` — so the pattern exists and the compositions simply never adopted it.

Measured consequence: making `mixer_strip_bank::paint_separators` a no-op — **no hairline separators at all** — passes the full **810-test suite at exit 0**. Moving each hairline off the gutter midpoint onto the column edge also passes. `DESIGN.md:462` names "compact columns with hairline separators, not cards" as a product requirement.

The same exposure exists at `section.rs:309` and `utility_inspector_panel.rs:421`, which paint `rules::hairline` under the same unasserted conditions. **This is why WP09 was not rejected for the hairline gap alone** — it would have applied a standard two approved work packages were not held to.

### 2. `from_projection_or_vocabulary` matches by substring, so C-003 guards leak

`section.rs:1010` accepts a painted run if any projected text **`contains`** it:

```rust
.any(|text| !text.is_empty() && text.contains(piece))
```

Containment, not equality. So any fabricated numeral that happens to be a substring of a real projected label passes the no-placeholder assertion that every composition relies on.

Measured consequence: painting a literal `"0"` into every empty mixer column passes `a_marked_bank_paints_no_value_the_projection_did_not_carry`, whose own docstring claims it proves *"no level, no zero, and no dash standing in for a reading nobody reported."* `"0"` is a substring of `"T00 Level"`. `"0.0"` passes too. `"-12.5 dB"` is correctly caught, so the helper is not inert — but the **canonical** fabrication walks through it.

C-003 is the mission's most-enforced constraint, and this is the shared mechanism guarding it. A per-WP fix (assert whole-label equality in the composition's own test) is what WP09 was asked for; the helper itself belongs to whoever next owns `section.rs`.

### Why this is recorded rather than assigned

Both are in WP05's approved file. Tightening the helper would change what every existing composition assertion accepts, which is a mission-level decision, not a work-package edit. **The cheap per-WP mitigation is for each composition to assert equality rather than lean on the shared helper, and to count non-text shapes where it paints structure.**

### WP09 cycle 2 closed its own exposure locally — the pattern is worth copying

WP09 did **not** edit `section.rs`. It added a `rects` channel to its own harness and asserted exact equality in its own tests. Both exposures above are now closed for `MixerStripBank` and remain open for `Section` and `UtilityInspectorPanel`.

Two details worth carrying to whoever fixes this mission-wide:

1. **A shape *count* is not sufficient.** `probe::Painted.shapes` would have caught "every hairline deleted" but **not** "every hairline moved onto the column edge" — the count stays at fifteen either way. Verified by mutation during review. `mixer_strip_bank`'s `fifteen_hairlines_sit_on_the_gutter_midpoints_at_both_viewports` asserts *positions* derived from the painted column extents, which catches both. If the shared probe is ever extended, extend it with geometry, not a counter.
2. **Exact ordered equality is the form that works.** `a_marked_bank_paints_exactly_its_structure_names_and_nothing_else` asserts the empty-bank run list equals an expected sequence exactly, rather than asking whether each run is "explainable". That is what catches a fabricated `"0"`; substring explainability does not. The broader cross-context sweep still leans on `from_projection_or_vocabulary`, and that residual reliance is documented in the test rather than hidden.

---

## F-15 — DEFERRED: the gallery and shell need an event-injection path, not synthetic keystrokes

**Raised by**: the operator, 2026-08-03
**Owner**: the next Spec Kitty pass — **explicitly deferred, do not act on it in Phase 4b**

To verify WP06's recomposition by eye, the orchestrator drove the running app with **synthesized OS keyboard events** (`osascript … keystroke "2"` via System Events) to change context, plus System Events calls to move and resize the window.

**That is a crutch and it should not be necessary.** The architecture already has the right seam — physical input → semantic action/event → `AppState::apply` → view projection — so a verification pass should be able to **emit the semantic event directly** and observe the resulting frame. Driving the real OS input stack to reach an internal state transition tests the window manager as much as the shell, is fragile against focus and window ordering, and cannot run headless.

What is wanted: a supported way to inject the event (or a scripted sequence of them) into a running or headless scene and read back the painted frame, without the operating system in the loop.

Note the existing gallery scene already accepts input by design and makes no exact-generation claim, so this does not touch the `demo-live-*` witness contract.

---

## F-16 — FUTURE GOAL: pop-over modals

**Raised by**: the operator, 2026-08-03
**Owner**: a later phase — **not Phase 4b scope**

The product will have **pop-over modals**, and the component vocabulary should cover them.

The relevant asymmetry today: WP03 built the **`ModalOption` control** against Figma `48:173` / `48:207`, and `PresentationRole::ModalEntry` exists and is reachable — `(Toggle, ModalEntry)` resolves to `ModalOption`. So the *row inside* a modal is built. What does not exist is the **modal surface itself**: no `ShellComposition` variant presents a pop-over, and `ShellRegion` has no name for a layer above the five structural bands.

A modal is not a sixth band. It is a layer over the frame, which means whatever composition owns it has to answer things the current family never had to: what it dims or blocks beneath, how it is dismissed, where focus goes while it is open and where it returns, and whether it participates in `ShellFrameObservation` at all.

Worth noting for whoever scopes it: adding it is the same shape of change as `MixerStripBank` (F-09) — a crest-spec amendment to `valueObject.Shell.ShellComposition`, authored first, then a work package deriving from it.

---

## F-17 — T044's residual exposure, and the helper it will be built on is partly blind

**Raised by**: WP06 cycle 2
**Owner**: WP08, before T044 is written

WP06's caption fix closes the clipping the operator saw by eye in the Inspector route line and the `Locked` label. **It does not close T044.** Runs that do not pass through `section::caption` still clip in the production frame:

| Run | Where it comes from | Measured |
|---|---|---|
| `ROUTED PATCHES · 01` | a projected surface title, `src/control/state_projector.rs:596`, painted by the section *header* rather than the caption | overruns to x=2048 at 1920 |
| `READY`, `NAVIGATE` | context line | overrun the viewport |
| 64 mixer cell labels | mixer columns | clip at 1280 |

### The part that matters more than the list

**`check_no_text_clips_or_overlaps` (`tests/component_vocabulary.rs:1642`) is blind in two ways, and T044 would inherit both.**

1. **It asserts clipping only for `ContextLine` and `IdentityHeader`.** Everything in `PersistentSideRegion` and `MainWorkspace` is outside its reach — which is exactly why WP06 cycle 1 shipped both the meter displacement and the caption clipping with the whole suite green at 862/0. Reverting the wrap fix *still* passes 862/0 today.
2. **It compares clip-rect identity against the band rect**, so a `ContextLine` run that escapes the viewport is counted as *scrolled* rather than flagged. `READY` overruns and the helper does not notice.

So the helper does not merely under-cover: on its own two declared bands it can miss the failure it exists to catch.

**T044 must not be layered on it unexamined.** Whatever WP08 writes needs its own clip determination, over every band, with a denominator — and per F-14 and WP04's reachability finding, absence and clipping are different failures: a culled run and a run never composed are the same absence in a shape stream, so a reachability check needs a supplied-count comparison rather than a shape scan.

### Not WP06's to fix

`state_projector.rs` is unowned by any work package in this mission, and `tests/component_vocabulary.rs` is WP08's only for the page-reachability change. WP06 raised this rather than widening its own scope, which is correct.
