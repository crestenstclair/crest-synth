# Mission Specification: Mixer Track Column

**Mission Branch**: `feat/mixer-track-column` (merges to `main`)
**Created**: 2026-08-04
**Status**: Draft
**Input**: Operator inspection of the shipped MIXER context, 2026-08-04. Phase 4b (`crest-component-controls-and-compositions-01KZ25VX`, merged to `main` in PR #1) delivered `MixerStripBank` and the sixteen-column seating, and shipped each column as a generic group of every control the projection reports for that track. The design file authors a different column. This mission replaces the generic group with the authored composite.

**Operator scope directive (2026-08-04)**: presentation only. Pan, mute, and solo stay focusable and adjustable exactly as they are today — what changes is how a column is drawn, not what can be addressed in it.

## Crest-Spec Grounding

This mission derives from the crest-spec at `.kittify/crest-spec/`. It cites declared intent rather than restating it.

| Cited declaration | Relationship |
|---|---|
| `goal.build_from_component_vocabulary` | The goal advanced — the bank's column becomes a declared shape later surfaces can reuse rather than an incidental division of available height. |
| `capability.component_vocabulary` | Extended. The vocabulary gains the column anatomy the design file authors; nothing in the existing eight compositions or eight controls is retired. |
| `requirement.reusable_shell_compositions` | Binding and amended. Its clause *"the mixer strip bank arranges sixteen track columns rather than controls, so it is a group of groups"* still holds; what changes is what one column **is**. |
| `requirement.shell_composed_from_components` | Preserved. Every region stays produced by a declared composition and every control by a declared `ComponentControl`; the render adapter gains nothing. |
| `requirement.no_placeholder_values_in_production` | Binding. The authored column has five structures; any the projection does not drive is marked unavailable, never invented. |
| `requirement.responsive_shell_blockout` | Preserved. The column resolves both viewports from `valueObject.Shell.ViewportDensityPolicy`; its `mixerColumn` member and the uniform-narrowing rule are unchanged. |
| `requirement.canonical_sixteen_track_bank` | Untouched. Sixteen tracks, each owning Level, Pan, Mute, Solo and eight indexed sends. This mission changes no track state. |
| `requirement.passive_semantic_action_boundary` | Binding. No `SemanticAction` variant, focus target, or reducer behavior is added or changed. |
| `valueObject.Mixer.MixerTrackParameter` | Cited. Its invariant *"MixerMain orders Level, Pan, Mute, and Solo; MixerInspector orders the eight sends by ascending BusId"* is what makes the sends' absence from the column correct rather than a gap. |

**New structure this mission requires that the crest-spec does not yet declare.** The current `valueObject.Shell.ShellComposition` invariant states that under `MixerStripBank` *"each column is a titled group of controls asked in the vertical-strip role."* That is precisely the declaration this mission retires: the authored column is not a group of equal cells but a fixed composite of five named structures at authored proportions, in which the level fader is dominant and pan and state are compact readouts. The replacement anatomy, and the value-presentation form the column's readout uses, are authored in `/spec-kitty.crest-spec`, which runs next and before `/spec-kitty.plan` — not assumed here.

## Domain Language

Phase 4 and 4b's canonical terms carry forward unchanged. This mission adds:

| Canonical | Meaning | Avoid |
|---|---|---|
| Track column | One of the bank's sixteen fixed columns, a composite of five named structures | "strip", "channel", "cell stack" |
| Column anatomy | The declared five-structure arrangement and its authored proportions | "column layout", "column template" |
| Readout | A structure that presents a value without being the control that adjusts it | "display", "label", "text field" |
| Hex readout | A value presented as two-digit hexadecimal over the MIDI domain (`00`–`7F`) | "hex label", "raw value" |
| State line | The single line presenting a track's mute and solo condition together | "status row", "flags", "toggles" |

The design file names the five structures `Track`, `Fader`, `Value`, `Pan`, and `State` (`42:26`). Those are the product's names for the structures; `MixerTrackParameter` remains the code's closed union of what a track owns. Both are canonical in their own register and must not be conflated — `Pan` the authored structure is a readout, while `MixerTrackParameter::Pan` remains an adjustable field.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The mixer reads as a mixing desk (Priority: P1)

A player opens MIXER and sees sixteen channel strips: a track name, a tall fader carrying most of the column, its level as a value, and a compact indication of pan and of mute/solo condition. Today they see sixteen columns of four equal stacked bands, each a labelled control with its own caption and its own numeric line — `T00 Level` above a short fader above `0.000`, then `T00 Pan` above a second full fader above `0.000`, then `T00 Mute` / `OFF`, then `T00 Solo` / `OFF`. Eight rows where the design authors five structures.

**Why this priority**: this is the mission's deliverable and the reason it exists. The mixer is the most-looked-at surface in the product and it does not resemble its design.

**Independent Test**: render the bank at the desktop viewport and confirm each column presents exactly the five declared structures, in authored order, at authored proportions, with no per-control caption.

**Acceptance Scenarios**:

1. **Given** the MIXER context at either authored viewport, **When** the bank paints, **Then** each of the sixteen columns presents exactly five structures — track header, level fader, level readout, pan readout, state line — in that order, and presents nothing else.
2. **Given** a column renders, **When** its proportions are measured, **Then** the level fader is the dominant structure, and every extent resolves from the density policy rather than from dividing available height by the number of controls the projection reported.
3. **Given** a column renders, **When** its text is read, **Then** no per-control caption appears: the track header is the only name in the column, and pan and state are identified by their authored compact form rather than by a repeated `T00 Pan` label.
4. **Given** the compact viewport, **When** the bank paints, **Then** all sixteen columns still seat under the declared uniform-narrowing rule and the column anatomy is the same one, not a second layout.

---

### User Story 2 - Nothing became unadjustable (Priority: P1)

A player who could focus and adjust a track's level, pan, mute, and solo before this mission can still focus and adjust all four after it. The controls look different; they behave identically.

**Why this priority**: the redesign is presentation-only by operator directive. A silent loss of addressability would be a functional regression disguised as a visual improvement, and it is exactly the failure mode a redrawing invites.

**Independent Test**: drive the same focus and adjustment sequence against the shipped shell before and after, and confirm the same fields are reachable, in the same order, producing the same intent.

**Acceptance Scenarios**:

1. **Given** any mixer track, **When** the player navigates within it, **Then** Level, Pan, Mute, and Solo are each reachable in the order `MixerTrackParameter` declares, unchanged from before this mission.
2. **Given** a focused field in a column, **When** it is adjusted, **Then** it returns the same typed intent it returned before, and no new `SemanticAction` variant, focus target, or reducer behavior exists.
3. **Given** a field is focused, **When** the column renders, **Then** the focus is visible on the structure that presents that field, including on the compact pan readout and the state line.
4. **Given** any of the nine `ComponentState` values applies to a structure in the column, **When** it renders in that state, **Then** it carries both the authored color treatment and its authored `NonColorSignal`, as every other control does.
5. **Given** the existing shell, projection, and focus tests, **When** they run against the recomposed column, **Then** they pass unchanged — this is a redrawing, not a behavior change.

---

### User Story 3 - A value reads the way the design writes it (Priority: P2)

A player reads a track's level as the design file writes it — two hexadecimal digits over the MIDI domain — rather than as a three-decimal float.

**Why this priority**: it is the most visible remaining difference once the structure is right, and it is confined to one readout. It is P2 because the column is legible without it and it can be judged separately.

**This change is scoped to the mixer track column only.** Every other value in the product keeps its current presentation. Extending hexadecimal presentation to every MIDI-ranged value elsewhere is deliberately out of scope (C-004).

**Independent Test**: read the column's level readout against the design file's value for the same reading, and confirm no value outside the column changed form.

**Acceptance Scenarios**:

1. **Given** a track's level, **When** the column's readout renders, **Then** it presents the value as two hexadecimal digits over the MIDI domain, matching the form the design file uses.
2. **Given** any value outside the mixer track column, **When** it renders, **Then** its presentation is unchanged by this mission.
3. **Given** a level the projection reports, **When** the readout renders, **Then** it presents that reported value and derives nothing the projection did not supply.

---

### User Story 4 - The undriven structure is marked, not invented (Priority: P2)

A maintainer inspecting the shipped column can tell which of the five authored structures the projection actually drives and which it does not.

**Why this priority**: `requirement.no_placeholder_values_in_production` is a standing constraint, and the column is where the temptation is highest — a plausible-looking pan or state value costs nothing to fabricate and misrepresents absent state as present.

**Independent Test**: hand the column a projection slice missing part of its designed structure and confirm it omits or marks the gap and paints no value that was not in the view data.

**Acceptance Scenarios**:

1. **Given** an authored structure with no view data behind it, **When** the column renders in the production shell, **Then** that structure is omitted or marked explicitly unavailable and no representative value is painted.
2. **Given** the gallery, **When** the column appears as a specimen, **Then** it may use representative content, which is what a gallery is for.

---

### Edge Cases

- **A track's level is at its minimum.** The fader presents the reported value; a floor reading is a real reading and is not confused with an unavailable one.
- **Pan is centered.** The design file's authored pan readout is the pair `P` `C` — a label and a centered condition. A centered pan is a value, not an absence, and is presented as such.
- **Both mute and solo are engaged.** The single state line presents both conditions together rather than being replaced by whichever was set last.
- **Neither mute nor solo is engaged.** The state line presents the resting condition rather than collapsing to empty, because an empty line and an unavailable line must not look alike.
- **The compact viewport narrows the column past what the readouts need.** The declared uniform-narrowing rule governs; the column never scrolls, never elides, and never swaps to a second anatomy.
- **The focused field is one the column presents compactly.** Focus visualization is carried by the compact structure; a field is never made unfocusable because its presentation is small.
- **A track carries sends.** Sends are not part of the column. They remain on `MixerInspector`, where `MixerTrackParameter`'s invariant already places them.

## Requirements *(mandatory)*

### Functional Requirements

| ID | Title | User Story | Priority | Status |
|----|-------|------------|----------|--------|
| FR-001 | Declared column anatomy | As a maintainer, I want the mixer track column declared as a fixed composite of five named structures in authored order, so that a column is a designed shape rather than available height divided by however many controls the projection reported. | High | Open |
| FR-002 | Authored column proportions | As a player, I want each structure to occupy the proportion the design file authors — the level fader dominant, the readouts compact — so that the mixer reads as a channel strip. | High | Open |
| FR-003 | One name per column | As a player, I want the track header to be the only name in a column, so that sixteen columns are not eight rows of repeated captions. | High | Open |
| FR-004 | Compact pan readout | As a player, I want pan presented in the authored compact form rather than as a second full-height fader, so the column has one fader and one meaning for its dominant structure. | High | Open |
| FR-005 | Single state line | As a player, I want mute and solo presented together on one state line, so a track's condition is read at a glance instead of as two labelled rows. | High | Open |
| FR-006 | Addressability preserved exactly | As a player, I want Level, Pan, Mute, and Solo to remain focusable and adjustable in the declared order with the same typed intent, so that no field became unreachable when the column was redrawn. | High | Open |
| FR-007 | Focus and state visible on every structure | As a player, I want focus and every applicable `ComponentState` visible on whichever structure presents the field, including the compact ones, so that small presentation never means invisible state. | High | Open |
| FR-008 | Hex readout in the column | As a player, I want the column's level readout presented as two hexadecimal digits over the MIDI domain, so it matches the design file. | Medium | Open |
| FR-009 | Undriven structure marked | As a maintainer, I want any authored structure the projection does not drive to be omitted or marked unavailable in the shipped shell, so absent state is never shown as present. | Medium | Open |
| FR-010 | Bank and gallery follow the column | As an operator, I want the bank and the gallery specimen to present the new anatomy, so the library and the shipped product cannot disagree about what a column is. | Medium | Open |

### Non-Functional Requirements

| ID | Title | Requirement | Category | Priority | Status |
|----|-------|-------------|----------|----------|--------|
| NFR-001 | Both viewports seat sixteen | Sixteen columns seat inside the main-surface content width at both authored viewports with nothing scrolled, clipped, or elided, and no column narrower than the authored minimum interactive target. Enforced by the validation that already proves this, re-run against the new anatomy. | Correctness | High | Open |
| NFR-002 | Existing suite unbroken | The full test suite passes with zero failures, and no existing shell, projection, or focus test is modified to accommodate this mission. | Reliability | High | Open |
| NFR-003 | No visual literals outside the module | The repository guard continues to report zero literal colors, type sizes, spacing constants, and band heights outside `src/shell/visual/`, with the column's authored proportions resolving through the density policy. | Maintainability | High | Open |
| NFR-004 | No text unreachable | No glyph run in the bank leaves its column with the column as its own container. Trimming inside a narrower inner container remains bounded by the recorded ceiling the deterministic proof already carries and may not grow. | Correctness | High | Open |

### Constraints

| ID | Title | Constraint | Category | Priority | Status |
|----|-------|------------|----------|----------|--------|
| C-001 | Presentation only | This mission adds no `SemanticAction` variant, no focus target, no reducer behavior, and no projection field. It changes how a track is drawn, never what state exists or what can be addressed. Operator directive, 2026-08-04. | Scope | High | Open |
| C-002 | No MIDI, no audio | This mission introduces no MIDI fixture, no audio device, and no audible behavior in any scene, test, or demo target it adds. The meter's absence of a live signal is unchanged and is not this mission's to close. | Scope | High | Open |
| C-003 | Sends stay in the Inspector | The column presents no send. `MixerInspector` remains where the eight indexed sends are presented, as `MixerTrackParameter` declares. | Technical | High | Open |
| C-004 | Hexadecimal is scoped to the column | The hexadecimal readout applies to the mixer track column's level and nowhere else. No other value's presentation changes, and no shared formatter is retargeted product-wide. | Scope | High | Open |
| C-005 | Closed unions stay closed and exhaustive | The column anatomy is declared as a closed vocabulary with an exhaustiveness assertion, so adding a structure names every site that must change. | Technical | High | Open |
| C-006 | Phase 4 artifacts are additive-only | Existing tokens, primitives, controls, compositions, density policies, and gallery page identities are extended, never renumbered or redefined. The eight compositions remain eight. | Technical | High | Open |
| C-007 | No proof-about-proof work | This mission's deliverable is a column a person can look at. It adds no layer that verifies the verification and no acceptance tooling of its own; it re-runs the validations Phase 4 declared. | Scope | High | Open |

### Key Entities

- **Track column**: one of `MixerStripBank`'s sixteen fixed columns; a composite of five named structures at authored proportions, presenting one `MixerTrackId`'s fields and returning typed intent.
- **Column anatomy**: the closed, ordered vocabulary of the five structures a column contains, with an exhaustiveness assertion.
- **Readout**: a structure presenting a value it does not own and does not adjust.
- **State line**: the single structure presenting mute and solo condition together.

## Assumptions

- The projection already carries what the column needs. `MixerMain` supplies Level, Pan, Mute, and Solo per track with lifecycle status and error, and the sixteen-track bank is fixed. The gap this mission closes is presentational.
- Phase 4's tokens, typeface, density policies, primitives, state vocabulary, and the eight controls are correct and need no revision; this mission consumes them. The fader control in particular is reused rather than replaced.
- The design file linked from `DESIGN.md` is reachable and authoritative for the column's anatomy and proportions, as it was for Phase 4's tokens. Frame `42:25` is the authored bank and `42:26` the authored column.
- `ViewportDensityPolicy::mixer_column()` already resolves column width and pitch; the column's *internal* proportions are the part this mission adds, and they resolve through the same policy rather than through a new constant.

## Dependencies

- Phase 4b (`crest-component-controls-and-compositions-01KZ25VX`), merged to `main` — supplies `MixerStripBank`, the fader and meter controls, the density policy's `mixerColumn` member, and the validations this mission re-runs.
- The design file referenced by `DESIGN.md`, for column anatomy and proportions.
- `.kittify/crest-spec/`, which must declare the column anatomy and retire the generic-group invariant before planning.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Each of the sixteen columns presents exactly the five authored structures in authored order, and presents nothing else.
- **SC-002**: The level fader is the dominant structure in every column at both authored viewports, and every extent resolves from the density policy.
- **SC-003**: A column contains exactly one name — its track header.
- **SC-004**: Level, Pan, Mute, and Solo remain focusable and adjustable in the declared order, returning the intent they returned before, verified by the existing focus and projection tests passing unmodified.
- **SC-005**: The column's level readout presents two hexadecimal digits over the MIDI domain, and no value outside the column changed presentation.
- **SC-006**: Sixteen columns seat at both authored viewports with nothing scrolled, clipped, or elided, and no column below the authored minimum interactive target.
- **SC-007**: The full test suite passes with zero failures and no existing shell, projection, or focus test was modified to accommodate this mission.
- **SC-008**: The operator can see the new column in the shipped application and as a gallery specimen, and the two agree.
