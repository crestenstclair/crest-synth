# Implementation Plan: Crest Component Controls and Compositions

**Branch**: `feat/crest-component-controls-and-compositions` | **Date**: 2026-08-02 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `kitty-specs/crest-component-controls-and-compositions-01KZ25VX/spec.md`

## Summary

Phase 4b finishes the component library. Phase 4a gave the project a closed visual vocabulary, a vendored typeface, two density policies, a nine-value state vocabulary, seven primitives, and a browsable gallery. What it did not give is anything a screen is actually assembled from: today `src/adapter/eframe_graphical_window.rs` paints all seven shell regions and every control as private free functions in one 1,282-line adapter, and `paint_semantic_control` (`:816`) renders all seven `SemanticControlKind` values as the same label-and-value row.

This mission adds two closed families — `ComponentControl` (eight variants) and `ShellComposition` (seven variants) — authored against the Figma reference, moves the adapter's painting into them, extends the gallery from eight pages to fifteen so every control and composition is visible, and adds one deterministic acceptance target proving the shipped shell actually uses them. It is a re-composition: no `SemanticAction` variant, no focus target, no reducer behavior, and no MIDI or audio anywhere.

## Technical Context

**Language/Version**: Rust 1.75+ (2021 edition), as the existing workspace
**Primary Dependencies**: eframe/egui + egui_extras (the declared GUI stack, `requirement.selected_egui_stack`); no new runtime dependency is added by this mission
**Storage**: N/A — components hold no state and persist nothing
**Testing**: Cargo integration tests driving the production render path. Two named targets carry this mission's proof: the existing `tests/component_vocabulary.rs` (extended for page reachability) and the new `tests/component_composition.rs`. Plus the `make demo-live-component-library` gallery witness. Black-box through the public surface (DIRECTIVE_036); test-first (DIRECTIVE_034)
**Target Platform**: macOS and Linux desktop at 1920×1080, and the Steam Deck compact viewport at 1280×800
**Project Type**: single Rust library plus one binary composition root
**Performance Goals**: 60 fps interactive paint at both viewports; gallery first paint under 3 s; page change under 100 ms (NFR-001, NFR-002)
**Constraints**: No MIDI, no audio, no audible behavior anywhere in this slice (C-001). No new semantic vocabulary (C-002). No placeholder values in the production shell (C-003). Closed unions stay exhaustively matched (C-004). `src/adapter/eframe_graphical_window.rs` ends at ≤ 40% of its current 1,282 lines, i.e. ≤ 512 lines (NFR-003). Zero visual literals outside `src/shell/visual/` (NFR-004). Existing suite passes unmodified (NFR-005)
**Scale/Scope**: 8 controls × up to 9 states × 2 viewports; 7 compositions; 15 gallery pages; ~1,000 lines relocated out of the render adapter

## Charter Check

*GATE: passed before Phase 0. Re-checked after design — still passing.*

Charter present (`.kittify/charter/`), template set `software-dev-default`, languages java/rust. Governing directives for this mission and how the plan satisfies them:

| Directive | Application here | Status |
|---|---|---|
| DIRECTIVE_001 Architectural Integrity | Controls and compositions are separately replaceable; each owns paint only, and the adapter keeps only plumbing | PASS |
| DIRECTIVE_010 Specification Fidelity | Every deliverable traces to a crest-spec asset; the plan's Crest-Spec Derivation section is the index | PASS |
| DIRECTIVE_024 Locality of Change | Changes concentrate in `src/shell/visual/` and `src/adapter/eframe_graphical_window.rs`; no context outside Shell is touched | PASS |
| DIRECTIVE_025 Boy Scout Rule | The `DESIGN.md` six-vs-nine state contradiction (Phase 4a finding A10) is domain-matched and folded in as FR-013 rather than filed away | PASS |
| DIRECTIVE_030 Test and Typecheck Gate | `cargo fmt`, `cargo clippy`, and the full suite gate every work package | PASS |
| DIRECTIVE_031 Context-Aware Design | All new resources belong to `context.Shell`; nothing crosses into Control, Synth, Mixer, or RealTime | PASS |
| DIRECTIVE_034 Test-First | Each control and composition work package writes its assertion before its rendering | PASS |
| DIRECTIVE_036 Black-Box Integration Testing | Proof drives the production render path and asserts on emitted observations, not on internal structure | PASS |
| DIRECTIVE_035 Bulk Edit Classification | Not applicable — this mission adds new identifiers and relocates code; it renames no existing string across files. `change_mode` stays unset | N/A |

No violations. Complexity Tracking is therefore omitted.

## Crest-Spec Derivation

`crest_spec_impact: structural`. Authored in `/spec-kitty.crest-spec` before this plan, committed as `7562526` and corrected in `b8191b4`. Doctor: 130 resources, 102 requirements, 31/31 completion checks, OK.

### Resources this mission adds

| Canonical ID | What it declares |
|---|---|
| `valueObject.Shell.ComponentControl` | The eight-variant closed control family, its presentation-role selector, its declared per-control state applicability, and its ownership boundary |
| `valueObject.Shell.ShellComposition` | The seven-variant closed composition family, its region binding, and the no-placeholder rule |
| `requirement.configurable_control_family` | Selection total over kind × role |
| `requirement.reusable_shell_compositions` | Compositions declare no visual value of their own |
| `requirement.shell_composed_from_components` | The adapter holds no visual decision |
| `requirement.no_placeholder_values_in_production` | Omit or mark unavailable; never invent |
| `requirement.silent_component_gallery` | Silence measured, not claimed |
| `asset.ComponentCompositionAcceptanceTests` | `tests/component_composition.rs` |
| `validation.component_composition` | Project completion check |

### Resources this mission changes

| Canonical ID | Change |
|---|---|
| `valueObject.Shell.ComponentGalleryPage` | 8 → 15 variants; reachability replaces one-digit-per-page; the eight existing bindings are pinned |
| `valueObject.Shell.WindowInput` | +`Digit9`, `Digit0`, `BracketLeft`, `BracketRight`; surfaceDescriptor 33 → 41 |
| `valueObject.Shell.ComponentGalleryObservation` | +controls, compositions, and `audioOrMidiConstructed` fields |
| `port.Shell.AppWindow` | New invariant: every region painted by a composition, every control by a control |
| `capability.component_vocabulary` | +`every_control_shape_exists`, +`every_region_is_a_composition`; silence measured in `every_state_is_judgable` |
| `requirement.browsable_component_gallery` | Widened to controls, compositions, and stepping |
| `requirement.component_vocabulary_behavioral_proof` | Widened to control/composition coverage and silence |
| `goal.build_from_component_vocabulary` | +5 requirement links |
| `witness.component_gallery` | Predicates: 15 pages, 10 digit-reachable, 15 step-reachable, 8 controls, 7 compositions, silence false |

### Resources this mission retires

None.

### Assets → files

| Asset | Files it produces |
|---|---|
| `asset.ShellContextModules` | `src/shell/visual/controls/*`, `src/shell/visual/compositions/*`, `src/shell/visual/mod.rs`, `src/shell/window_input.rs` |
| `asset.AdapterModules` | `src/adapter/eframe_graphical_window.rs` (reduced) |
| `asset.TestingContextModules` | `src/testing/component_gallery_scene.rs` |
| `asset.ComponentCompositionAcceptanceTests` | `tests/component_composition.rs` |
| `asset.ComponentVocabularyAcceptanceTests` | `tests/component_vocabulary.rs` (page-reachability update) |
| `asset.ProductDesignAuthority` | `DESIGN.md` (FR-013) |
| `asset.DeliveryRoadmap` | `ROADMAP.md` (FR-014) |

### Proof covering the change

- `validation.component_vocabulary` — extended for page reachability by binding or stepping.
- `validation.component_composition` — new; the deterministic proof of kind×role totality, per-control state applicability, region-by-composition coverage, adapter emptiness, the no-placeholder rule, ownership, and viewport integrity.
- `witness.component_gallery` — the live browsable proof, now covering 15 pages, 8 controls, 7 compositions, and measured silence.
- `evidence.component_vocabulary_contract` — binds both validations and the witness.

No `data-model.md` and no `contracts/` are produced: the crest-spec's `valueObjects` and `ports[].contract` are canonical and forking them is rejected at acceptance.

## Project Structure

### Documentation (this mission)

```
kitty-specs/crest-component-controls-and-compositions-01KZ25VX/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── quickstart.md        # Phase 1 output
├── spec.md              # From /spec-kitty.specify
├── checklists/
│   └── requirements.md
└── tasks.md             # Created by /spec-kitty.tasks — NOT by this command
```

`data-model.md` and `contracts/` are deliberately absent — forbidden when a crest-spec exists.

### Source Code (repository root)

```
src/shell/visual/
├── mod.rs                    # re-exports; adds controls and compositions
├── token.rs                  # unchanged (Phase 4a)
├── typeface.rs               # unchanged (Phase 4a)
├── density.rs                # unchanged (Phase 4a)
├── state.rs                  # unchanged (Phase 4a) — the nine ComponentStates
├── primitives/               # unchanged (Phase 4a) — text, rules, focus, value, status, hint
├── controls/                 # NEW — valueObject.Shell.ComponentControl
│   ├── mod.rs                #   the family, PresentationRole, and the total kind×role selector
│   ├── parameter_row.rs
│   ├── choice_row.rs
│   ├── toggle.rs
│   ├── compact_slider.rs
│   ├── fader.rs
│   ├── meter.rs
│   ├── browser_row.rs
│   └── modal_option.rs
└── compositions/             # NEW — valueObject.Shell.ShellComposition
    ├── mod.rs                #   the family and its region binding
    ├── application_shell.rs
    ├── context_switch.rs
    ├── identity_header.rs
    ├── section.rs
    ├── patch_strip_row.rs
    ├── utility_inspector_panel.rs
    └── footer.rs

src/adapter/
└── eframe_graphical_window.rs   # REDUCED to ≤ 512 lines: window plumbing and event translation

src/shell/
└── window_input.rs              # +Digit9, Digit0, BracketLeft, BracketRight

src/testing/
└── component_gallery_scene.rs   # +7 pages, stepping, control/composition/silence observation

tests/
├── component_vocabulary.rs      # extended: page reachability
└── component_composition.rs     # NEW
```

**Structure Decision**: `controls/` and `compositions/` sit beside the existing `primitives/` inside `src/shell/visual/`, because NFR-004 draws the literal-free boundary at that directory and a control placed anywhere else would have to import visual values across it. One file per variant keeps each control's Figma-authored geometry reviewable on its own and lets work packages own disjoint file sets, which is what makes parallel lanes possible.

## Implementation Concern Map

> Implementation concerns are NOT work packages. `/spec-kitty.tasks` translates these into WPs; one concern may become several, and small ones may merge.

### IC-01 — Control family scaffold and the total kind × role selector

- **Purpose**: establish the `ComponentControl` family, the `PresentationRole` vocabulary, and the exhaustive selector every later control plugs into, so the eight controls can then be built independently.
- **Relevant requirements**: FR-001, FR-009, C-004
- **Affected surfaces**: `src/shell/visual/controls/mod.rs`, `src/shell/visual/mod.rs`
- **Sequencing/depends-on**: none — this is the first thing built
- **Risks**: the role vocabulary is the load-bearing decision. Too coarse and a fader and a parameter row collapse into one shape; too fine and every composition invents a role. `research.md` fixes it at four roles from the Figma surfaces. Getting this wrong forces rework across all eight controls, so it lands alone and first.

### IC-02 — The eight controls, authored against Figma

- **Purpose**: build each control's geometry, spacing, and state treatment from the design reference, in every state it declares applicable, at both viewports.
- **Relevant requirements**: FR-002, FR-003, FR-010, FR-011
- **Affected surfaces**: `src/shell/visual/controls/{parameter_row,choice_row,toggle,compact_slider,fader,meter,browser_row,modal_option}.rs`
- **Sequencing/depends-on**: IC-01
- **Risks**: eight disjoint files over one shared `mod.rs` — parallelizable, but the shared selector registration is a contention point and should be registered by IC-01 up front rather than appended per control. Figma extraction is the slow part; controls whose Figma specimen is missing must raise it rather than approximate silently.

### IC-03 — The seven compositions

- **Purpose**: build each shell region as a composition that arranges primitives and controls and declares no visual value of its own, including the omit-or-mark rule where view data is absent.
- **Relevant requirements**: FR-004, FR-009, FR-010, FR-011, C-003
- **Affected surfaces**: `src/shell/visual/compositions/*`
- **Sequencing/depends-on**: IC-01, and each composition needs the controls it arranges from IC-02
- **Risks**: compositions are where the placeholder temptation lives. The Figma layout shows structure the projection may not drive; C-003 requires omitting or marking, never inventing. Every such gap should be recorded as it is found, because the list of them is the real input to Phase 5.

### IC-04 — Production shell recomposition and adapter reduction

- **Purpose**: move the adapter's region and control painting into the compositions and controls, leaving window plumbing and event translation, and prove the existing behavior is unchanged.
- **Relevant requirements**: FR-005, FR-006, NFR-003, NFR-004, NFR-005, C-002
- **Affected surfaces**: `src/adapter/eframe_graphical_window.rs`, `src/shell/visual/compositions/application_shell.rs`
- **Sequencing/depends-on**: IC-03
- **Risks**: the highest-risk concern. It touches the file every existing shell test observes, and NFR-005 forbids editing those tests to accommodate it — a failure there means the recomposition changed behavior and must be fixed, not accommodated. The `ShellFrameObservation` contract must survive intact.

### IC-05 — Gallery extension: pages, stepping, and measured silence

- **Purpose**: add the seven control and composition pages, add bidirectional stepping so all fifteen pages are reachable past the ten digits, and emit control, composition, and silence coverage in the observation.
- **Relevant requirements**: FR-007, FR-008, FR-012, NFR-001, NFR-002, NFR-006, C-006
- **Affected surfaces**: `src/testing/component_gallery_scene.rs`, `src/shell/window_input.rs`
- **Sequencing/depends-on**: IC-02 and IC-03 for the specimens; the window-input key extension can land earlier
- **Risks**: FR-012 pins the eight existing digit bindings — a page-order change that silently moves one is a regression an operator would feel immediately. The `WindowInput` descriptor count invariant (33 → 41) is asserted, so the key extension and the count must land together.

### IC-06 — Deterministic proof and the document corrections

- **Purpose**: deliver `tests/component_composition.rs`, extend `tests/component_vocabulary.rs` for page reachability, and land the `DESIGN.md` and `ROADMAP.md` corrections.
- **Relevant requirements**: FR-013, FR-014, SC-001, SC-003, SC-004, SC-006, SC-007
- **Affected surfaces**: `tests/component_composition.rs`, `tests/component_vocabulary.rs`, `DESIGN.md`, `ROADMAP.md`
- **Sequencing/depends-on**: assertions are written test-first alongside IC-02 through IC-05; the target passes only once they land. The document corrections depend on nothing and can go early.
- **Risks**: C-007 bounds this concern — it delivers proof of the library, not another proof layer about the proof. The failure mode this repository has already paid for is meta-work; if a check here starts verifying a verification, it is out of scope.

## Phase 0 — Research

See [research.md](research.md). Four questions were open at plan time; all four are resolved there and none blocks task generation.

## Phase 1 — Design

Design lives in the crest-spec, not in a parallel planning artifact. [quickstart.md](quickstart.md) records how to run and judge the result.

## Branch Contract

Current branch at plan start: `feat/crest-component-controls-and-compositions`. Planning/base branch: `feat/crest-component-controls-and-compositions`. Final merge target for completed changes: `feat/crest-component-controls-and-compositions`, and from there to `main`. `branch_matches_target` is true.
