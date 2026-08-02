# Phase 0 Research: Crest Component Foundations

**Date**: 2026-08-02

No `[NEEDS CLARIFICATION]` markers were carried out of `/spec-kitty.specify`, so this document records
the technical questions that were genuinely open and how each was resolved by inspection rather than
assumption.

---

## R-01 — Do the authored values in `DESIGN.md` still match the design file?

**Decision**: Yes. `DESIGN.md` is trustworthy as the authored source for every value it lists.

**Rationale**: Read directly from the design file on 2026-08-02. All 11 shared colors, all 8 type styles,
the 6 spacing steps, and the focus halo match exactly — including the halo's 28% opacity, which the design
file publishes as `#65E5FF47` (`0x47` = 71/255 = 0.278).

**Alternatives considered**: Treating `DESIGN.md` as possibly stale and re-deriving everything from the
design file. Rejected once the two were confirmed identical — it would have doubled the source of truth
for no benefit. The design file remains the source for measurements `DESIGN.md` does not carry.

**Consequence**: IC-01 can be built from `DESIGN.md:534-573` without a second lookup per value.

---

## R-02 — Two token-set differences between the design file and `DESIGN.md`

**Decision**: Union both sets. Keep the design file's selected-row background *and* `DESIGN.md`'s
elevated, strong border, patch, and chorus accents.

**Rationale**: The design file publishes 13 color variables; `DESIGN.md` lists 15. Where both define a
value they agree exactly, so this is coverage, not conflict. The selected-row background is needed for
multi-select; the four `DESIGN.md`-only accents are needed for identity and elevation. Trimming either
source to match the other would delete a value some surface requires.

**Alternatives considered**: Treating the design file as exhaustive and dropping the four extras —
rejected, `DESIGN.md` is the product authority for what the product should be. Treating `DESIGN.md` as
exhaustive and dropping the selected-row background — rejected for the same reason in the other direction.

**Consequence**: 13 declared colors in `SemanticVisualToken`, recorded as a durable decision in
`DESIGN.md` via `asset.ProductDesignAuthority`.

---

## R-03 — Can egui render a variable font, or are static weights required?

**Decision**: Static weights are required. Ship four derived static faces, not the upstream variable font.

**Rationale**: This needed two layers checked, and the first answer was misleading. `ab_glyph` 0.2.32
**does** support variation axes — it exposes a `VariableFont` trait with
`set_variation(b"wght", 600.0)` (`ab_glyph-0.2.32/src/variable.rs:9-28`). But egui registers fonts
through `epaint`, and `epaint` 0.32.3's `FontData` carries only the file bytes, a font-face `index`, and
a scale/offset `FontTweak` (`epaint-0.32.3/src/text/fonts.rs:112-121`). There is no axis field, and
`epaint` never calls `set_variation` anywhere in its source. A variable font handed to egui therefore
renders every style at its default instance — one weight for all eight type styles, silently.

**Alternatives considered**:

1. *Ship the variable font and set the axis.* Not reachable through the public egui API without patching
   or forking `epaint`. Rejected — `requirement.selected_egui_stack` forbids introducing an alternate
   GUI runtime, and forking the font layer to save three files is a bad trade.
2. *Ship only Regular and let egui synthesize bold.* Rejected — synthesized weights are not the authored
   faces, and `AuthoredTypeface` forbids a synthesized substitute precisely because it would look
   plausible while being wrong.
3. *Derive four static instances from the variable source.* **Chosen.** Reproducible, exact, and each
   weight is a real authored instance.

**Consequence**: `vendor/azeret-mono/` ships Regular 400, Medium 500, SemiBold 600, Bold 700 plus the
variable source retained byte-exact as the provenance record, with the derivation command recorded so
anyone can reproduce it. Worth re-checking if `epaint` ever exposes an axis.

---

## R-04 — What does the design file give that `DESIGN.md` cannot?

**Decision**: Per-variant measurements. These are the reason the design-file connection was worth setting
up, and they feed IC-03 directly.

**Rationale**: Measured from the Patch Strip frame: patch rows are 52 px tall on a 66 px pitch; content
insets 24 px from the screen edge; the `>` cursor occupies a 9 px column at x=10 with the label starting
at x=19; the label/value hairline runs down the row's vertical middle; utility controls are 380×48 on a
60 px pitch with a 5 px slider bar. None of these appear in `DESIGN.md`, and none can be read off a PNG
export.

**Alternatives considered**: Eyeballing the existing PNG exports in
`figma-functional-interpretation/assets/`. Rejected — those confirm structure (the slider component is
3 tones × 4 states) but carry no measurable pixel values.

**Consequence**: The desktop density policy is measured. Structural band geometry needs no change:
`eframe_graphical_window.rs:17-25` already matches the authored 48/72/64 bands, 420 px side region, and
1920×1080 / 1280×800 sizes.

---

## R-05 — Where should the Steam Deck density policy come from?

**Decision**: Author it from the desktop frames and the declared minimums, then have the operator review
it visually. Record that it is authored rather than measured.

**Rationale**: The design file contains only 1920×1080 frames. `DESIGN.md:450` requires Steam Deck
verification but supplies no authored small-viewport design, so there is nothing to measure. The operator
explicitly approved authoring it.

**Alternatives considered**: Blocking on new design frames — rejected, the operator declined. Scaling the
desktop policy by a single factor — rejected, `DESIGN.md:450` requires *controlled density* and retained
context, not uniform shrinking, and a 48 px minimum target does not scale.

**Consequence**: IC-03 must reach a viewable state early so the review can happen before the gallery is
finished. The authored-vs-measured distinction is declared in
`valueObject.Shell.ViewportDensityPolicy`.

---

## R-06 — What do loading and error states look like?

**Decision**: Reuse the vocabulary `DESIGN.md` already declares for structural edits. Loading is the
adjustment accent with `Preparing` / `Activating` text; error is the warning accent with typed short text.

**Rationale**: The design file's component sets cover default, focused, editing, and disabled only —
`ROADMAP.md:174` additionally requires loading and error. `DESIGN.md:454` already specifies that a
structural row displays its active and requested value plus `Preparing`, `Activating`, or a typed failure.
Reusing that is consistent with an existing product decision instead of inventing a second visual
language for the same idea.

**Alternatives considered**: Designing new loading and error treatments — rejected as unnecessary
invention when the product already answers the question. A spinner or animation — rejected; `DESIGN.md:575`
requires text or shape, and an animated indicator adds a per-frame repaint the 16 ms idle cadence does
not want.

**Consequence**: Declared as an invariant on `valueObject.Shell.ComponentState`.

---

## R-07 — How is "no literals outside the vocabulary" actually enforced?

**Decision**: A guard that is itself proven to fail. Not review, and not a guard taken on trust.

**Rationale**: NFR-002 is the requirement most likely to rot, because a literal reintroduced under
deadline pressure looks harmless at the call site. The repository already has precedent for this shape of
check: `no_name_enumerated_identity` is a declared project validation proving no type in Synth, Mixer,
RealTime, or Control is named after a concrete effect or bus. The same pattern applies here.

**Alternatives considered**: A Clippy lint — rejected, no existing lint expresses "no `Color32::from_rgb`
outside one module", and a custom lint driver is a heavier dependency than a scripted guard. Code review
— rejected outright by `C-006`; convention is not evidence.

**Consequence**: IC-08 owns the guard, and the guard must be shown failing on a deliberately reintroduced
literal. A guard that has never failed is indistinguishable from no guard — this is the mission's
highest-risk item and the plan says so.

---

## R-08 — Does a browsable scene threaten the autonomous witness contract?

**Decision**: No, provided it never claims to be one. The gallery accepts input and asserts nothing about
generations.

**Rationale**: `DESIGN.md:634-644` isolates mapped semantic input during `demo-live` specifically so an
asynchronous user edit cannot replace the exact generation a checkpoint awaits. That constraint exists to
protect a *generation-correlated claim*. The gallery makes no such claim: it paints specimens, and its
observation reports what was painted. The risk is not co-existence — it is someone later conflating the
two in either direction.

**Alternatives considered**: Making the gallery autonomous with a scripted page walk — rejected, it
defeats the operator's stated purpose of browsing by hand. Adding input to the existing witness scenes —
rejected, it would break exactly what their isolation protects.

**Consequence**: `C-005` in the spec, an invariant in `proof/invariants.yaml`, and an explicit prompt on
`asset.BuildMakefile` that this target is not a `demo-live` alias.
