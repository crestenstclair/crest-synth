# Mission Specification: Crest Component Foundations

**Mission Branch**: `feat/crest-component-foundations` (merges to `main`)
**Created**: 2026-08-02
**Status**: Draft
**Input**: Phase 4 (`ROADMAP.md:170-184`), first of two missions. Scope confirmed with the operator: the shared visual vocabulary, the reusable primitives, the gallery, and the production-shell repaint. Configurable controls and compositions are deferred to the follow-on mission. The gallery ships with its `make demo-live-component-library` launch target — every scene gets a demo target.

## Crest-Spec Grounding

This mission derives from the crest-spec at `.kittify/crest-spec/`. It does not restate declared intent; it cites it.

| Cited declaration | Relationship |
|---|---|
| `goal.use_graphical_shell` | The goal this mission advances — controller-first shell at desktop and compact viewport sizes, with no state or audio behavior in the UI. |
| `capability.graphical_application_shell` | The capability being deepened. Its `production_shell` acceptance already names Phase 2 as rendering *"without inventing the Phase 4 component library"* — this mission is that named successor. |
| `requirement.authored_shell_composition` | Preserved unchanged. The five structural bands and the two-context rule still hold. |
| `requirement.responsive_shell_blockout` | Extended from "renders at both viewports" to "resolves both viewports from declared policies rather than scattered constants". |
| `requirement.selected_egui_stack` | Binding. Its clause *"Crest owns the shell, state, semantic behavior, and later component APIs"* is the contract this mission fulfills. |

**New structure this mission requires that the crest-spec does not yet declare**: a semantic visual vocabulary resource, a viewport density policy resource, a primitive state descriptor, a gallery scene with its page vocabulary, and the extension of the normalized window-key vocabulary in `context.Shell.WindowInput` to carry the additional digit keys that select gallery pages. These are authored in `/spec-kitty.crest-spec` — which runs next, before `/spec-kitty.plan` — not assumed here.

**One pre-existing drift this mission corrects.** `context.Shell.WindowInput` asserts *"surfaceDescriptor contains exactly 17 unique valid values"* (`.kittify/crest-spec/contexts/shell.yaml:53`), but `src/shell/window_input.rs:42` has held 21 since `SelectPatch` added Q and E. `spec-kitty crest-spec doctor` does not catch invariant text of this kind. This mission edits exactly that resource for its own reasons, so the correction lands here rather than being filed away (DIRECTIVE_025, domain-matched).

## Domain Language

Figma variable names are canonical. Code and spec use the Figma name, never a re-invention.

| Canonical | Meaning | Avoid |
|---|---|---|
| `color/bg/canvas`, `color/bg/surface`, `color/bg/panel`, `color/bg/selected` | Background roles | "background", "dark grey", `BACKGROUND` |
| `color/accent/focus`, `color/accent/adjust` | The two interaction-mode accents | "highlight", "cyan", `ACCENT` |
| `color/accent/positive`, `color/accent/warning` | Status accents | "green", "red", "error color" |
| `Display/Screen`, `Heading/Section`, `Heading/Panel`, `Body/Default`, `Body/Compact`, `Label/Control`, `Code/Value`, `Instruction/Hint` | The eight authored type styles | "title font", "big text", ad-hoc point sizes |
| `space/4` … `space/32` | The six spacing steps | arbitrary pixel gaps |
| Desktop viewport / compact viewport | The two authored sizes (1920×1080, 1280×800) | "large screen", "small screen", "mobile" |

`DESIGN.md` names the instrument accent `instrument`; Figma names it `color/accent/instrument/plates`. Both denote `#b894ff`. Figma's name wins in code, per the rule above.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The application looks like the approved design (Priority: P1)

The operator launches Crest Synth and sees the interface the design authorizes: the authored dark palette, cyan focus, amber adjustment, and Azeret Mono throughout. Today they see a differently-toned screen drawn from seven hand-entered values in one adapter file, with no typeface loaded at all.

**Why this priority**: This is the only story that changes what a person sees when they run the product. Everything else in the mission is scaffolding that serves it. Delivered alone, it is already worth shipping.

**Independent Test**: Run `make run`, put the window beside the Figma design, and compare the palette and typeface. A test independently asserts every rendered value equals its authored counterpart.

**Acceptance Scenarios**:

1. **Given** the application is launched, **When** the shell paints its first frame, **Then** every surface, text, border, and accent color equals its authored Figma value exactly, and every text run is set in Azeret Mono at the authored size, weight, line height, and letter spacing for its style.
2. **Given** a control is focused, **When** the frame is painted, **Then** it carries the authored 3 px `color/accent/focus` keyline and its halo (radius 8, spread 1, 28% opacity), and it also carries a non-color focus indication.
3. **Given** a control is under active adjustment, **When** the frame is painted, **Then** it carries the authored 3 px `color/accent/adjust` keyline and a non-color adjustment indication.
4. **Given** the authored value for any color, type style, spacing step, or geometry, **When** it is changed in the single shared source, **Then** every place it appears changes with it and no second definition survives.

---

### User Story 2 - A live gallery scene the operator can page through (Priority: P2)

The operator runs one command and gets a real window showing off the components. Number keys change page — one page per group of pieces — and each page renders every meaningful behavioral state of that group: resting, focused, adjusting, disabled, loading, error, muted, soloed, selected. Both the desktop and compact viewport sizes are shown.

**Why this priority**: Visual fidelity is the point of this phase, and a log cannot carry it. Without one browsable screen, judging correctness means driving the app into each state by hand, which is slow enough that it will not be done. Paging by number key makes it a thing the operator actually browses rather than a static contact sheet.

**This scene is browsable, not autonomous.** The `demo-live-*` witness scenes are deliberately input-isolated so an asynchronous edit cannot replace the generation a checkpoint awaits (`DESIGN.md:634-644`). The gallery scene has the opposite purpose — it exists to be driven by hand — so it accepts input and makes no exact-generation claim. It does not weaken the witness contract because it does not assert one.

**Independent Test**: Run the gallery command, press each number key, and confirm the page changes and that each declared state appears. An exhaustiveness assertion fails the build if a state exists with no specimen on any page.

**Acceptance Scenarios**:

1. **Given** the gallery scene is running, **When** the operator presses a number key bound to a page, **Then** that page is shown, its identity is visible on screen, and no application state, focus, Patch value, or audio behavior changes.
2. **Given** the gallery scene is running, **When** the operator presses a number key with no page bound to it, **Then** the current page is retained and nothing changes.
3. **Given** any page is shown, **When** it renders, **Then** every declared state of every primitive on that page appears with representative content, at both the desktop and compact viewport sizes.
4. **Given** a new state is added to the closed state vocabulary, **When** the project is built, **Then** compilation or the coverage assertion fails until a gallery specimen exists for it.
5. **Given** any specimen is shown, **When** it is inspected, **Then** its state is legible from text or shape alone, without relying on color.
6. **Given** the operator is browsing the gallery, **When** they close the window, **Then** the scene exits normally and releases every resource it owns.

---

### User Story 3 - Later screens do not re-derive the vocabulary (Priority: P3)

A maintainer assembling the real Patch or Mixer screen in a later phase reaches for existing pieces and existing values instead of copying paint, spacing, or focus-visualization logic out of another file.

**Why this priority**: This is the phase-completion condition (`ROADMAP.md:184`), but it is only fully provable once the follow-on mission adds controls and compositions. This mission must not block it.

**Independent Test**: Assemble one representative row from the shared vocabulary alone and confirm no color literal, size literal, or focus-visualization branch is written outside the shared source.

**Acceptance Scenarios**:

1. **Given** a maintainer builds a new surface, **When** they need a color, type style, spacing step, or focus treatment, **Then** it is available from the shared vocabulary and does not need to be re-derived.
2. **Given** any adapter or view file outside the vocabulary module, **When** it is inspected, **Then** it contains no literal color, no literal type size, and no literal spacing constant.

---

### Edge Cases

- **The typeface fails to load.** The failure is typed and visible, consistent with the product's "unavailable means explicit" principle. Silently substituting a fallback font would misrepresent the design as satisfied.
- **The window is smaller than the compact viewport.** The declared minimum is enforced; below it, structural bands and the persistent side region are still retained rather than hidden.
- **A behavioral state exists with no declared appearance.** The closed state vocabulary and its exhaustiveness assertion name every site that must change, the same mechanism that made `SelectPatch` safe to add.
- **A viewport falls between the two authored sizes.** The density policy resolves it by declared rule; no third hard-coded layout is introduced.
- **A color is needed that the vocabulary does not declare.** It is added to the vocabulary and to `DESIGN.md`, never introduced as a local literal.
- **A number key with no gallery page bound to it is pressed.** The current page is retained; no page is invented and no key silently does nothing different from any other unbound key.
- **The gallery scene receives a key already bound elsewhere.** Digit1 and Digit2 select PATCH and MIXER in the application; inside the gallery scene they select pages. The scene's binding is scene-local and never reaches `AppState`.

## Requirements *(mandatory)*

### Functional Requirements

| ID | Title | User Story | Priority | Status |
|----|-------|------------|----------|--------|
| FR-001 | Single semantic visual vocabulary | As a maintainer, I want one source declaring every semantic color, type style, spacing step, radius, keyline width, and minimum interactive target, so that no visual value is defined twice. | High | Open |
| FR-002 | Azeret Mono installed and mapped | As a player, I want the interface set in the authored typeface at all four authored weights, so that the product reads as designed rather than as a default system font. | High | Open |
| FR-003 | Declared viewport density policies | As a player on either device, I want the desktop and compact viewport sizes resolved from declared policies, so that no surface carries resolution-specific constants. | High | Open |
| FR-004 | Reusable primitives | As a maintainer, I want text roles, hairlines, keylines, focus frames, value displays, status marks, and action hints available as shared pieces, so that later screens compose rather than repaint. | High | Open |
| FR-005 | Explicit state rendering | As a player, I want focus, adjustment, disabled, loading, error, mute, solo, and selection each rendered with text or shape in addition to color, so that state is never conveyed by color alone. | High | Open |
| FR-006 | Production shell renders through the vocabulary | As a player, I want the real application — not only a demo — painted through the shared vocabulary, so that launching the product shows the approved design. | High | Open |
| FR-007 | Live gallery demo scene and launch target | As an operator, I want one command that opens a real window showing off the components in every meaningful state at both authored sizes, so that visual correctness can be judged by looking. | High | Open |
| FR-008 | Number-key page selection in the gallery | As an operator, I want number keys to change which gallery page is shown, so that I can browse the component groups by hand instead of scrolling one long sheet. | High | Open |
| FR-009 | Components own no application state | As a maintainer, I want every component to accept immutable view data and return typed semantic intent, so that no component owns Patch values, focus, navigation, reducer state, or audio state. | High | Open |
| FR-010 | Typed failure when the typeface is unavailable | As a player, I want a visible typed error if the authored typeface cannot load, so that a silent fallback never misrepresents the design as satisfied. | Medium | Open |

### Non-Functional Requirements

| ID | Title | Requirement | Category | Priority | Status |
|----|-------|-------------|----------|----------|--------|
| NFR-001 | Exact authored-value fidelity | Every semantic color equals its authored value exactly across all 17 declared colors, and every one of the 8 type styles matches its authored family, weight, size, line height, and letter spacing exactly. Zero tolerated deviations, asserted by test. | Correctness | High | Open |
| NFR-002 | No visual literals outside the vocabulary | Zero literal color constructions, literal type sizes, or literal spacing constants exist in any adapter, view, or scene file outside the vocabulary module, enforced by an automated check rather than review. | Maintainability | High | Open |
| NFR-003 | Both authored viewports render intact | At 1920×1080 and 1280×800, all five structural bands and the persistent side region remain visible, with zero clipped or overlapping text runs and every interactive target at or above the authored 48 px minimum. | Compatibility | High | Open |
| NFR-004 | No real-time or control-path regression | The audio callback contract is unchanged: zero allocation, locking, blocking, I/O, or logging on the audio thread. Interactive rendering remains event-driven at the 16 ms idle cadence, and the 512-event control-path acceptance fixture stays within its declared 50 ms ceiling. | Performance | High | Open |
| NFR-005 | Complete gallery state coverage | Every state in the closed state vocabulary has at least one gallery specimen at both authored sizes, and every declared gallery page is reachable by its bound number key. A state or page added without a specimen or binding fails the build or the coverage assertion. | Testability | High | Open |
| NFR-006 | Vendored typeface provenance | The typeface is vendored with its verbatim license, upstream revision, a byte-exact hash manifest, and a reproducible derivation procedure for any weight not shipped upstream. | Compliance | Medium | Open |

### Constraints

| ID | Title | Constraint | Category | Priority | Status |
|----|-------|------------|----------|----------|--------|
| C-001 | eframe/egui stack only | The UI remains eframe/egui with egui_extras. No alternate GUI runtime and no third-party component system is introduced; Crest owns the component API, behavior, and visual contract. Binds `requirement.selected_egui_stack`. | Technical | High | Open |
| C-002 | Bounded scope | Configurable controls and reusable compositions are out of scope and belong to the follow-on Phase 4 mission. This mission ships the vocabulary, the primitives, the gallery, the production repaint, and the gallery's `make demo-live-component-library` launch target. Every scene in this project ships with a demo target; that is never deferred. | Scope | High | Open |
| C-003 | Crest-spec authored before planning | `/spec-kitty.crest-spec` declares the new vocabulary, density policy, state descriptor, and gallery resources before `/spec-kitty.plan` derives from them, and `spec-kitty crest-spec doctor` stays green. | Process | High | Open |
| C-004 | Two top-level contexts preserved | PATCH and MIXER remain the only top-level contexts. The gallery is a scene, never a third context. Its page selection is scene-local, never becomes a `SemanticAction`, and never enters canonical application state. | Technical | High | Open |
| C-005 | Gallery input isolation is one-way | The gallery scene accepts input by design and therefore makes no exact-generation claim. It must not weaken the autonomous `demo-live-*` witness contract (`DESIGN.md:634-644`), which stays input-isolated. | Technical | High | Open |
| C-006 | Deterministic proof discipline | Visual claims are proven by measured comparison against authored values through the production render path. Construction-only tests, success-token logs, and pre-render layout plans are not evidence. | Technical | High | Open |
| C-007 | Typeface licensing | Azeret Mono ships under SIL Open Font License 1.1 with its license text retained verbatim. | Regulatory | Medium | Open |

### Key Entities

- **Semantic visual vocabulary**: the closed set of named colors, type styles, spacing steps, radii, keyline widths, and interactive minimums. Names are the canonical Figma names. Raw values stay private to it.
- **Viewport density policy**: the declared resolution of the vocabulary onto one authored viewport — band heights, workspace split, side-region width, row pitch, and inset. Two exist: desktop and compact.
- **Primitive state**: the closed set of behavioral states a primitive can be handed — resting, focused, adjusting, disabled, loading, error, muted, soloed, selected. Closed so that adding one names every site that must change.
- **Gallery page**: one named group of specimens, selected by a bound number key. The page vocabulary is closed, so a page without a binding — or a binding without a page — is a build failure rather than a dead key.
- **Gallery specimen**: one primitive rendered in one state at one viewport with representative content, used only for visual judgment and coverage assertion.

## Assumptions

- **Figma and `DESIGN.md` agree** on all shared values. Verified directly against the Figma file on 2026-08-02: all 11 shared colors, all 8 type styles, the focus halo, and the six spacing steps match exactly.
- **Two token-set differences are resolved by union.** Figma declares `color/bg/selected` (`#2a3745`) which `DESIGN.md` omits; `DESIGN.md` declares `elevated`, `border/strong`, `patch`, and `chorus` which Figma does not publish as variables. Both sets are kept, and `DESIGN.md` is updated to record the union as a durable decision.
- **Compact-viewport frames are not authored in Figma.** Only the 1920×1080 screens exist. The compact density policy is authored during this mission from the desktop frames and the declared minimums, then reviewed visually by the operator, per their explicit instruction.
- **Loading and error appearances are not authored in Figma.** They reuse the vocabulary already declared in `DESIGN.md` for structural edits — an adjustment-accent treatment with `Preparing`/`Activating` text for loading, and a warning-accent treatment with typed short text for error — so no new visual language is invented.
- **Row geometry comes from measurement, not estimate.** Patch rows are 52 px tall on a 66 px pitch with a 24 px content inset; utility controls are 380×48 on a 60 px pitch with a 5 px slider bar. Read directly from the Figma file rather than eyeballed from an export.

## Dependencies

- The vendored Azeret Mono typeface, its license, and its provenance record must be present before FR-002 can be satisfied. **Already landed** at `vendor/azeret-mono/` in all four authored weights.
- Read access to the Figma design file for per-variant measurement. **Already established** and verified.
- `/spec-kitty.crest-spec` must author the new resources before `/spec-kitty.plan` runs (C-003).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All 17 authored colors and all 8 authored text styles render exactly as authored, with zero deviations, measured through the production render path.
- **SC-002**: A person launching the application recognizes the approved design from the screen alone, without consulting a log, a test report, or a design file.
- **SC-003**: One command opens a real, browsable window; every declared gallery page is reachable by its number key; and every declared behavioral state of every primitive appears at both authored sizes with no declared state missing.
- **SC-004**: Zero visual values remain defined outside the single shared source; changing any authored value in one place changes every appearance of it.
- **SC-005**: Every behavioral state is distinguishable without color, verified specimen by specimen in the gallery.
- **SC-006**: Both authored viewports render every structural band and the persistent side region with no clipped or overlapping text and no interactive target below the authored minimum.
- **SC-007**: The instrument sounds and responds exactly as it did before this mission — no added latency, no dropout, no change to what is heard while the interface is repainted.
