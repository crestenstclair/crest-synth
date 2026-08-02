# Mission Specification: Crest Component Controls and Compositions

**Mission Branch**: `feat/crest-component-controls-and-compositions` (merges to `main`)
**Created**: 2026-08-02
**Status**: Draft
**Input**: Phase 4 (`ROADMAP.md:170-184`), second and final mission. Phase 4a (`crest-component-foundations-01KZ02H2`, merged) delivered the vocabulary, primitives, gallery, and shell repaint, and deferred configurable controls and reusable compositions to this mission (`C-002`, that mission's `spec.md:137`). This mission delivers exactly those, extends the gallery to show them, and makes the production shell compose from them.

**Operator scope directive (2026-08-02)**: this slice is a working component library plus a demo scene. No MIDI. No audio. Anywhere.

## Crest-Spec Grounding

This mission derives from the crest-spec at `.kittify/crest-spec/`. It cites declared intent rather than restating it.

| Cited declaration | Relationship |
|---|---|
| `goal.use_graphical_shell` | The goal advanced — controller-first shell at both authored viewport sizes, with no state or audio behavior in the UI. |
| `capability.graphical_application_shell` | The capability completed. Phase 4a supplied its vocabulary and primitives; this mission supplies the controls and compositions that vocabulary exists to serve. |
| `requirement.authored_shell_composition` | Preserved. The five structural bands and the two-context rule still hold; this mission changes what assembles them, not what they are. |
| `requirement.responsive_shell_blockout` | Preserved. Controls and compositions resolve both viewports from the Phase 4a density policies; they introduce no new resolution constants. |
| `requirement.selected_egui_stack` | Binding. Its clause *"Crest owns the shell, state, semantic behavior, and later component APIs"* is what this mission completes: Crest owns the control and composition API; egui/egui_extras remain underneath it. |

**New structure this mission requires that the crest-spec does not yet declare**: a configurable-control resource family keyed to the existing `SemanticControlKind` union, a composition resource family for the shell regions, and the gallery page vocabulary's extension beyond eight pages (which also extends the normalized window-key vocabulary in `context.Shell.WindowInput` past `Digit8`). These are authored in `/spec-kitty.crest-spec`, which runs next and before `/spec-kitty.plan` — not assumed here.

**One deliberate roadmap amendment.** `ROADMAP.md:182` describes `make demo-live-component-library` as a scene that *"plays the real MIDI fixture"* and *"exercises representative controls through semantic actions"*. The operator has scoped MIDI and audio out of this slice entirely. The component-library demo is therefore the hand-browsable gallery scene Phase 4a shipped, extended to cover the controls and compositions this mission adds. This is recorded as an amendment, not a silent omission: `ROADMAP.md` is updated to match, and no measured audio-bearing witness is claimed for Phase 4.

**One pre-existing drift this mission corrects.** `DESIGN.md:576` reads *"Focus, mute, solo, loading, error, and selection always have text or shape in addition to color"* — six states. `ComponentState` (`src/shell/visual/state.rs:27`) has held nine since Phase 4a, adding `Resting`, `Adjusting`, and `Disabled`, each with an authored `NonColorSignal`. This mission builds every control against that nine-state vocabulary, so the product authority is corrected here rather than filed away (DIRECTIVE_025, domain-matched). This is finding A10 of the Phase 4a analysis report.

## Domain Language

Phase 4a's canonical terms carry forward unchanged (Figma variable names win over re-inventions). This mission adds:

| Canonical | Meaning | Avoid |
|---|---|---|
| Control | A configurable component that presents one `SemanticControlViewModel` and returns typed intent | "widget", "input", "field" |
| Composition | A reusable arrangement of primitives and controls filling a shell region | "layout", "container", "panel component" |
| Specimen | One rendered instance of a component in one state on a gallery page | "example", "demo", "sample" |
| `ComponentState` | The closed nine-value state vocabulary: Resting, Focused, Adjusting, Disabled, Loading, Error, Muted, Soloed, Selected | "status", "mode", ad-hoc booleans |
| `SemanticControlKind` | The closed seven-value control-kind union: Continuous, Stepped, Choice, Toggle, Asset, Identity, Surface | "control type", "parameter type" |
| Gallery page | One digit-key-addressable screen of specimens | "tab", "section", "view" |

The roadmap names controls informally as "parameter rows, choice rows, toggles, compact sliders, faders, meters, browser rows, and modal options" (`ROADMAP.md:177`). Those are the product's names for shapes; `SemanticControlKind` is the code's closed union that selects among them. Both are canonical in their own register and must not be conflated.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Every control the product needs exists as a shared piece (Priority: P1)

A maintainer building any screen reaches for a parameter row, choice row, toggle, compact slider, fader, meter, browser row, or modal option and finds one, already correct in all nine states at both viewport sizes. Today the only control rendering in the product is `paint_semantic_control` (`src/adapter/eframe_graphical_window.rs:816`), a private free function in the render adapter that draws every control kind as the same label-and-value row.

**Why this priority**: this is the mission's deliverable. Without it, Phase 4's completion condition (`ROADMAP.md:184`) cannot be met by any later screen.

**Independent Test**: assemble a representative row from the shared controls alone and confirm no color literal, size literal, spacing literal, or state-visualization branch is written outside the shared source.

**Acceptance Scenarios**:

1. **Given** any declared pairing of a `SemanticControlKind` value with a presentation role, **When** a maintainer needs to render it, **Then** exactly one control resolves for that pair, accepts a `SemanticControlViewModel`, and returns typed semantic intent — and every control in the family is reachable by at least one declared pair. A kind alone does not select a shape: the same kind reads as a parameter row in one region and a fader in another, so selection is total over the pair, not over the kind.
2. **Given** any control and any of the nine `ComponentState` values, **When** it is rendered in that state, **Then** it carries that state's authored color treatment and its authored `NonColorSignal`, and the state is legible from text or shape alone.
3. **Given** a control is rendered, **When** its source is inspected, **Then** it holds no Patch value, no focus, no navigation, no reducer state, and no audio state, and it reads every value it paints from immutable view data passed in.
4. **Given** a control renders at the desktop viewport and at the compact viewport, **When** both frames are compared, **Then** every size difference resolves from a declared density policy and no control carries a resolution-specific constant.
5. **Given** a new value is added to `SemanticControlKind`, to the presentation-role vocabulary, or to `ComponentState`, **When** the project is built, **Then** compilation or an exhaustiveness assertion fails until every control names the new value.

---

### User Story 2 - Every shell region is a reusable composition (Priority: P1)

A maintainer assembling PATCH or MIXER composes the screen from an application shell, context switch, identity header, section, Patch strip row, Utility/Inspector panel, and footer, rather than re-deriving band heights and paint order. Today those regions are private free functions in one 1,282-line render adapter (`paint_context_line:357`, `paint_identity_header:418`, `paint_main_workspace:440`, `paint_patch_workspace:468`, `paint_mixer_workspace:488`, `paint_side_region:627`, `paint_footer:733`).

**Why this priority**: the controls alone do not satisfy `ROADMAP.md:184` — "layout" is named alongside "paint" in the completion condition. Compositions are how layout stops being copied.

**Independent Test**: the production render adapter contains no band-height constant, no paint-order decision, and no state-visualization branch of its own; every region it shows comes from a composition.

**Acceptance Scenarios**:

1. **Given** each of the seven named regions, **When** a maintainer needs it, **Then** a composition exists that accepts immutable view data and returns typed semantic intent.
2. **Given** a composition renders, **When** its source is inspected, **Then** it composes primitives and controls and defines no color, type size, spacing, or geometry value of its own.
3. **Given** the production shell is launched, **When** it paints a frame, **Then** every region on screen is produced by a composition, and the render adapter contributes window plumbing and event translation only.
4. **Given** the production shell before and after this mission, **When** the existing shell, projection, and focus tests are run, **Then** they pass unchanged — this is a re-composition, not a behavior change.
5. **Given** the compact viewport, **When** the shell paints, **Then** the header/footer bands, the visible Utility/Inspector, and the minimum interactive targets are all retained, as `DESIGN.md:450` requires.

---

### User Story 3 - One command shows the whole library (Priority: P1)

The operator runs `make demo-live-component-library`, gets a real window, and pages through it by number key. Every control appears in every applicable state, every composition appears filled with representative content, and both viewport sizes are shown. Judging whether the library is right is a matter of looking at it.

**Why this priority**: visual fidelity is the point of this phase and a log cannot carry it. The demo scene is how the work is judged, so it is not deferrable to a later slice.

**This scene is browsable and silent.** It accepts input by design, makes no exact-generation claim, and does not weaken the `demo-live-*` witness contract (`DESIGN.md:634-644`) because it asserts no witness. It opens no audio device, loads no MIDI fixture, and sounds nothing.

**Independent Test**: run the command, press each bound digit key, confirm the page changes and every declared specimen is present; an exhaustiveness assertion fails the build if any control kind or state has no specimen on any page.

**Acceptance Scenarios**:

1. **Given** the gallery scene is running, **When** the operator presses a bound number key, **Then** that page is shown, its identity is visible on screen, and no application state, focus, Patch value, or audio behavior changes.
2. **Given** the gallery scene is running, **When** the operator presses an unbound number key, **Then** the current page is retained and nothing changes.
3. **Given** any page renders, **When** it is inspected, **Then** every specimen on it appears with representative content at both the desktop and compact viewport sizes.
4. **Given** a control kind or component state exists, **When** the project is built, **Then** the coverage assertion fails unless a specimen for it exists on some page.
5. **Given** the scene runs, **When** its construction is inspected, **Then** no audio stream is opened and no MIDI event source is constructed.
6. **Given** the operator closes the window, **When** the scene exits, **Then** it exits normally and releases every resource it owns.
7. **Given** Phase 4a's eight pages, **When** this mission adds pages, **Then** the existing eight page identities and their digit bindings are unchanged and the additions are pure insertions.

---

### User Story 4 - Later screens do not re-derive anything (Priority: P2)

A maintainer starting the Phase 5 Patch editor writes screen-specific assembly only. No paint, no layout, no focus visualization, no state visualization.

**Why this priority**: this is the phase-completion condition (`ROADMAP.md:184`). It is provable only once Stories 1–3 land, and it is what closes Phase 4.

**Independent Test**: a guard over every view and adapter file outside the visual module finds no literal color, type size, spacing constant, band height, or state-visualization branch.

**Acceptance Scenarios**:

1. **Given** any file outside `src/shell/visual/`, **When** it is inspected, **Then** it contains no literal color, no literal type size, no literal spacing constant, and no literal band height.
2. **Given** a maintainer needs a control, composition, state treatment, or density decision, **When** they look for it, **Then** it exists in the shared vocabulary and needs no re-derivation.

---

### Edge Cases

- **A control kind has no distinct designed shape.** `Identity` and `Surface` are read-only presentational kinds; they render as declared read-only forms rather than being omitted, so the union stays exhaustively covered.
- **A state does not apply to a control.** `Muted` and `Soloed` apply to mixer-track controls only. Non-applicability is declared per control, not silently skipped, and the coverage assertion checks declared applicability rather than the full cross product.
- **A composition has no view data for part of its designed structure.** It omits that part or marks it explicitly unavailable. It never paints an invented or placeholder value in the production shell. Gallery specimens use representative content, which is what a gallery is for.
- **The gallery needs more than nine pages.** Digit keys are finite. If page count exceeds the available digits, the page vocabulary declares its own paging rule; no page becomes unreachable.
- **A digit key already means something in the application.** `Digit1` and `Digit2` select PATCH and MIXER. Inside the gallery scene digit keys select pages; the binding is scene-local and never reaches `AppState`. This is Phase 4a's established rule and is unchanged.
- **A control needs a value the view model does not carry.** The control declares the gap; the view model is not extended in this mission, and no control invents state to fill it.
- **The compact viewport cannot fit a composition at authored density.** The density policy resolves it; no composition is hidden and no third hard-coded layout is introduced.

## Requirements *(mandatory)*

### Functional Requirements

| ID | Title | User Story | Priority | Status |
|----|-------|------------|----------|--------|
| FR-001 | Configurable control family | As a maintainer, I want every declared pairing of a `SemanticControlKind` with a presentation role to resolve to exactly one shared control, with every control reachable by at least one pair, so that no screen re-derives how a parameter, choice, toggle, or asset is drawn. | High | Open |
| FR-002 | Product control shapes | As a player, I want parameter rows, choice rows, toggles, compact sliders, faders, meters, browser rows, and modal options to look like their designed shapes, so that the interface reads as designed rather than as uniform label-and-value rows. | High | Open |
| FR-003 | Nine-state rendering per control | As a player, I want every control to render all of its applicable states from the closed `ComponentState` vocabulary with both color and a non-color signal, so that state is never conveyed by color alone. | High | Open |
| FR-004 | Reusable composition family | As a maintainer, I want the application shell, context switch, identity header, section, Patch strip row, Utility/Inspector panel, and footer available as shared compositions, so that later screens assemble rather than re-lay-out. | High | Open |
| FR-005 | Production shell composes from the library | As a player, I want the real application painted through the shared controls and compositions, so that the library is proven by the shipped product and not only by a gallery. | High | Open |
| FR-006 | Render adapter holds no visual decisions | As a maintainer, I want the render adapter reduced to window plumbing and event translation, so that no paint, layout, or state-visualization logic lives outside the visual module. | High | Open |
| FR-007 | Gallery covers controls and compositions | As an operator, I want gallery pages showing every control in every applicable state and every composition with representative content, so that visual correctness can be judged by looking. | High | Open |
| FR-008 | Coverage assertion over the closed unions | As a maintainer, I want the build to fail when a control kind or component state has no gallery specimen, so that the library cannot silently drift out of coverage. | High | Open |
| FR-009 | Components own no application state | As a maintainer, I want every control and composition to accept immutable view data and return typed semantic intent, so that none owns Patch values, focus, navigation, reducer state, or audio state. | High | Open |
| FR-010 | Both viewports from declared policies | As a player on either device, I want every control and composition to resolve its desktop and compact sizing from the Phase 4a density policies, so that no new resolution-specific constant is introduced. | High | Open |
| FR-011 | Figma-authored appearance | As a player, I want each control and composition's geometry, spacing, and state treatment taken from the design file, so that the library is faithful rather than approximated. | High | Open |
| FR-012 | Additive gallery page vocabulary | As an operator, I want Phase 4a's eight pages and their digit bindings preserved exactly, so that the pages I already know keep working. | Medium | Open |
| FR-013 | DESIGN.md state list corrected | As a maintainer, I want the product authority to name all nine non-color-signalled states, so that it stops contradicting the shipped vocabulary. | Medium | Open |
| FR-014 | ROADMAP amendment recorded | As a maintainer, I want the MIDI-bearing component-library demo bullet amended in place, so that the roadmap states what Phase 4 actually delivered. | Medium | Open |

### Non-Functional Requirements

| ID | Title | Requirement | Category | Priority | Status |
|----|-------|-------------|----------|----------|--------|
| NFR-001 | Gallery opens promptly | The gallery window presents its first painted page promptly enough that the operator running `make demo-live-component-library` does not wait on it — under 3 seconds on the machine they run it from. **Operator-judged in this slice, not machine-enforced** (see below). | Performance | Medium | Open |
| NFR-002 | Page changes feel immediate | A bound digit or stepping key changes the shown page with no perceptible lag — under 100 ms. **Operator-judged in this slice, not machine-enforced** (see below). | Performance | Medium | Open |

**Why NFR-001 and NFR-002 are operator-judged rather than measured.** Both describe a live window the operator already looks at, and instrumenting them would mean adding duration fields to `valueObject.Shell.ComponentGalleryObservation`, which is a structural crest-spec change made after crest-spec authoring closed, to serve two numbers on a demo scene. C-007 bounds this mission to the library and the scene, with no acceptance tooling of its own. So these two are stated as the standard the scene is judged against when it is run, and no subtask asserts them. If either is ever missed, it becomes a measured requirement in the mission that fixes it — not a retrofit here. Every other NFR in this table is machine-enforced.
| NFR-003 | Render adapter size reduction | `src/adapter/eframe_graphical_window.rs` ends at no more than 40% of its current 1,282 lines, with the removed content relocated into compositions rather than deleted. | Maintainability | High | Open |
| NFR-004 | No visual literals outside the module | A repository guard reports zero literal colors, type sizes, spacing constants, and band heights in any file outside `src/shell/visual/`. | Maintainability | High | Open |
| NFR-005 | Existing suite unbroken | The full test suite passes with zero failures, and no existing shell, projection, or focus test is modified to accommodate this mission. | Reliability | High | Open |
| NFR-006 | Silence is verifiable | An automated check confirms the gallery scene constructs no audio output and no MIDI event source. | Correctness | High | Open |

### Constraints

| ID | Title | Constraint | Category | Priority | Status |
|----|-------|------------|----------|----------|--------|
| C-001 | No MIDI, no audio | This mission introduces no MIDI fixture, no audio device, no note events, and no audible behavior in any scene, test, or demo target it adds. Operator directive, 2026-08-02. | Scope | High | Open |
| C-002 | No semantic vocabulary changes | This mission adds no `SemanticAction` variant, no focus target, and no reducer behavior. It changes how state is rendered, never what state exists. Phase 5 owns functional changes. | Technical | High | Open |
| C-003 | No invented values in the production shell | Where a composition's designed structure has no view data behind it, it omits that structure or marks it explicitly unavailable. It never paints a placeholder or representative value in the shipped product. | Technical | High | Open |
| C-004 | Closed unions stay closed and exhaustive | Control kinds, component states, and gallery pages remain closed unions with exhaustiveness assertions, so adding a value names every site that must change. | Technical | High | Open |
| C-005 | Crest owns the component API | Third-party egui utilities may be used underneath, but Crest owns the stable control and composition API, behavior, tokens, and visual contract. | Technical | High | Open |
| C-006 | Phase 4a artifacts are additive-only | Existing gallery page identities, digit bindings, tokens, and primitives are extended, never renumbered or redefined. | Technical | High | Open |
| C-007 | No mission-artifact proof work | This mission's deliverable is a component library and a demo scene a person can see. It adds no proof-about-proof layer and no acceptance tooling of its own. | Scope | High | Open |

### Key Entities

- **Control**: a component presenting one `SemanticControlViewModel`, selected by `SemanticControlKind`, rendered in one `ComponentState`, returning typed semantic intent.
- **Composition**: a component filling one named shell region by arranging primitives and controls; owns no values.
- **ComponentState**: the closed nine-value state vocabulary from Phase 4a (`src/shell/visual/state.rs:27`) with its authored `NonColorSignal` per state.
- **Gallery page**: a digit-key-addressable screen of specimens; a closed union with an exhaustiveness assertion.
- **Specimen**: one control or composition rendered in one state with representative content at one viewport size.

## Assumptions

- The view model already carries what the controls need. `SemanticControlViewModel` supplies kind, value, numeric range, lifecycle status, and error; the `PatchMain`, `PatchUtility`, `MixerMain`, and `MixerInspector` surfaces already exist, and all sixteen mixer tracks expose level, pan, mute, solo, and sends. The gap this mission closes is presentational.
- Phase 4a's tokens, typeface, density policies, primitives, and state vocabulary are correct and need no revision; this mission consumes them.
- The Figma file linked from `DESIGN.md` is reachable and authoritative for control and composition geometry, as it was for Phase 4a's tokens.
- Meters render from whatever the view model reports. With audio out of scope they show a resting reading in the shell and representative readings in the gallery; the meter component itself is complete either way.

## Dependencies

- Phase 4a (`crest-component-foundations-01KZ02H2`), merged and accepted — supplies every token, primitive, density policy, and state treatment this mission builds on.
- The Figma design file referenced by `DESIGN.md`, for control and composition fidelity.
- `.kittify/crest-spec/`, which must declare the control, composition, and extended page-vocabulary resources before planning.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every one of the seven control kinds and every one of the nine component states has at least one gallery specimen, enforced by an assertion that fails the build otherwise.
- **SC-002**: The operator can see all eight named product control shapes and all seven named compositions by running one command and pressing number keys, with no other setup.
- **SC-003**: The shipped application renders every on-screen region through a composition; no region is painted by the render adapter.
- **SC-004**: No file outside the visual module contains a literal color, type size, spacing constant, or band height.
- **SC-005**: Assembling a new representative screen requires writing screen-specific arrangement only — no paint, layout, focus-visualization, or state-visualization logic.
- **SC-006**: The full test suite passes with zero failures and no existing test was modified to accommodate this mission.
- **SC-007**: The gallery scene opens no audio device and constructs no MIDI event source, verified automatically.
- **SC-008**: `DESIGN.md` names all nine non-color-signalled states, and `ROADMAP.md:182` states what Phase 4 actually delivered.
