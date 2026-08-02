# Research: Crest Component Controls and Compositions

**Mission**: `crest-component-controls-and-compositions-01KZ25VX`
**Date**: 2026-08-02
**Status**: all four open questions resolved; none blocks task generation

Four questions were open when planning began. Each is recorded below with what was chosen, why, and what was rejected.

---

## R-01 — What is the presentation-role vocabulary?

**Question**: `SemanticControlKind` has seven values and the product names eight control shapes. A continuous parameter is a parameter row on PATCH and a fader in a mixer strip, so kind alone cannot select a shape. What supplies the rest?

**Decision**: a closed four-value `PresentationRole`, supplied by the requesting composition:

| Role | Where it is asked | Shapes it can select |
|---|---|---|
| `ListedRow` | PATCH main surface, MIXER inspector, Utility panel — vertically stacked labelled rows | ParameterRow, ChoiceRow, Toggle, BrowserRow |
| `VerticalStrip` | MIXER track columns — the sixteen fixed compact columns | Fader, Meter, Toggle |
| `PanelEntry` | Utility/Inspector panel entries that are not full rows | CompactSlider, Toggle, ChoiceRow |
| `ModalEntry` | Focus-trapped option modals and the later Sample Browser | ModalOption, BrowserRow |

Selection is total over kind × role, and every one of the eight controls is reachable by at least one pair. Both properties are asserted, not assumed.

**Rationale**: the roles come from surfaces `DESIGN.md` already names — the PATCH strip and its rows (`DESIGN.md:454`), the sixteen compact mixer columns with hairline separators (`:462`), the persistent Utility/Inspector (`:444`, `:466`), and the nested option modal with trapped focus (`:458`). Deriving roles from declared surfaces rather than inventing a taxonomy means a new surface either fits an existing role or is a deliberate addition, and the closed union makes that addition name every site it touches.

**Alternatives rejected**:
- *Kind alone, with the fader as a distinct kind.* Would require adding a `SemanticControlKind` variant, which is reducer-adjacent vocabulary and violates C-002. The view model is correct as it stands; the gap is presentational.
- *Control infers its shape from the surface identity it is painted on.* Puts a layout decision inside a control, so the same control renders differently depending on context it should not know about. The crest-spec now forbids it explicitly (`ComponentControl` invariant on `presentationRole`).
- *One control per surface, no shared family.* That is the copying Phase 4 exists to eliminate.

---

## R-02 — Build the controls on egui widgets, or paint them?

**Question**: egui ships `Slider`, `ProgressBar`, `SelectableLabel`, `Checkbox`. `requirement.selected_egui_stack` permits third-party utilities underneath the Crest layer. Which controls sit on an egui widget and which are painted directly?

**Decision**: paint directly, using egui for layout, input, and response only — not for control appearance. Concretely, controls use `egui::Ui` allocation and `Response` for hit-testing and interaction reporting, and `Painter` for every visual mark. No egui widget is used for the *appearance* of a Crest control.

**Rationale**: every egui widget carries its own visual language — corner rounding, fill, hover tinting, drag handles — configured through `egui::Style`, which is a second source of visual truth beside `SemanticVisualToken`. NFR-004 requires zero visual literals outside the vocabulary, and a styled egui widget defeats that check without tripping it, because the literal lives in a style struct rather than in a call site. Phase 4a already established the precedent: its primitives paint. C-005 keeps the boundary — Crest owns the appearance, egui owns layout and input.

**Alternatives rejected**:
- *Style egui widgets from the tokens.* Attractive and cheaper, but it makes `egui::Style` a parallel vocabulary and puts appearance decisions somewhere the literal guard cannot see. Rejected on NFR-004.
- *Mixed — widgets where convenient.* Produces exactly the inconsistency the gallery exists to catch, and makes "why does this control look different" an archaeology question.

**Consequence for the plan**: the fader, meter, and compact slider are the most painting-heavy controls and should be sequenced with that in mind; they are not one-line wrappers.

---

## R-03 — How does the adapter reduce without changing behavior?

**Question**: NFR-003 requires `src/adapter/eframe_graphical_window.rs` to end at ≤ 512 lines, and NFR-005 forbids modifying any existing shell, projection, or focus test to accommodate the change. What has to survive untouched?

**Decision**: three things stay in the adapter and everything else moves:

1. **Window plumbing** — `eframe::App::update`, panel construction (`TopBottomPanel`, `CentralPanel`), close requests, and the tick callback.
2. **Event translation** — the egui key → `WindowKey` normalization at `:1287`, which is boundary work, not paint.
3. **The observation emit** — `ShellFrameObservation` is constructed after painting from the rectangles the panels actually produced. It keeps its current shape and its current invariants (`context.Shell.ShellFrameObservation`).

Everything the seven `paint_*` free functions currently do moves into compositions. The compositions receive the `egui::Ui` and the projection slice; they return typed intent and the geometry the observation needs.

**Rationale**: the observation is the thing existing tests assert on. If it is constructed from real painted rectangles before and after, the existing shell tests cannot tell the difference — which is precisely the check NFR-005 encodes. Moving the observation construction into `ApplicationShell` was considered and rejected: the rectangles come from egui panel responses the adapter owns, so constructing it there would mean threading panel responses back out of the composition for no gain.

**Alternatives rejected**:
- *Delete the adapter's paint functions and rewrite compositions from Figma.* Loses the behavior the existing tests pin, and turns a recomposition into a rewrite. The move is mechanical first, then Figma fidelity is applied inside the composition where the tests can catch a regression.
- *Keep a thin `paint_*` shim per region.* Leaves seven functions in the adapter that decide paint order, which is exactly what FR-006 forbids.

**Risk recorded**: ~1,000 lines must relocate for NFR-003 to hold. If it lands at, say, 600 lines, that is a signal something visual stayed behind, not that the threshold was too strict.

---

## R-04 — How are fifteen pages reachable with ten digits?

**Question**: the gallery grows from 8 to 15 pages. Digit keys give ten bindings. FR-012 pins the eight existing bindings exactly.

**Decision**: ten digit bindings in declared order — `Digit1`–`Digit9` then `Digit0` — plus bidirectional stepping on `BracketLeft` / `BracketRight` over the full fifteen-page order, non-wrapping. Pages 1–8 keep the exact pages they have today. Pages 9 and 10 (`ParameterAndChoiceRows`, `TogglesAndSliders`) gain the two new digits. Pages 11–15 are reached by stepping.

Reachability, not binding, is the asserted invariant: every page must be reachable by its digit *or* by stepping, and stepping must reach all fifteen. `pages_reachable_by_digit == 10` and `pages_reachable_by_step == 15` are both witness predicates, so neither route can quietly regress.

**Rationale**: FR-012 is an operator-facing promise — someone who knows `Digit4` is InteractionStates must keep finding it there. Appending new pages after the existing eight satisfies that by construction. Stepping is the smallest addition that keeps the page set open-ended without renumbering anything, and non-wrapping matches the nonwrapping focus movement the product already uses everywhere else (`DESIGN.md:309`).

**Alternatives rejected**:
- *Cram controls and compositions onto two pages to stay within ten digits.* Eight control shapes across nine states at two viewports on one page is unreadable, which defeats the only purpose the gallery has.
- *A modifier plus digit for pages 11–20.* Adds a chord vocabulary to a scene whose whole virtue is that it is trivially browsable.
- *Renumber so the most useful pages get digits.* Breaks FR-012 and moves bindings an operator already knows.

---

## Standing note — Figma extraction

Per-control geometry, spacing, and state treatment are pulled from the Figma file linked in `DESIGN.md` during each control's work package, the same way Phase 4a authored its tokens. This is deliberately not front-loaded here: extracting all eight controls before building any of them would produce a specification document that goes stale against the design file, which is the failure mode the crest-spec exists to avoid.

**One rule for the work packages**: if a control's Figma specimen is missing or ambiguous, raise it rather than approximate. An approximated control that looks plausible is worse than a missing one, because the gallery will show it as authoritative.
