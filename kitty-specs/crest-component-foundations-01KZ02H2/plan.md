# Implementation Plan: Crest Component Foundations

**Branch**: `feat/crest-component-foundations` | **Date**: 2026-08-02 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `kitty-specs/crest-component-foundations-01KZ02H2/spec.md`

## Summary

Replace the seven hand-entered visual constants in `src/adapter/eframe_graphical_window.rs:28-34` with one
declared authored vocabulary, install the vendored typeface, express both authored viewports as declared
density policies, add the reusable primitives, repaint the production shell through all of it, and add a
browsable gallery scene whose digit keys page through every declared component state at both viewport
sizes.

The approach is subtractive at the adapter and additive in `src/shell`. Nothing in the reducer, projection,
transport, or render path changes: the shell already receives an immutable `GraphicalShellProjection` and
already paints the correct structural bands (`eframe_graphical_window.rs:17-25` matches the authored
48/72/896/64 geometry). What is wrong is exclusively *which values* it paints with and *where those values
live*. That containment is why this mission can be measured for zero real-time and control-path impact.

## Technical Context

**Language/Version**: Rust 1.96.0, edition 2021
**Primary Dependencies**: eframe/egui 0.32.3, egui_extras 0.32.3 (image + svg features only)
**Storage**: N/A — the vocabulary is compile-time declared; the typeface is a vendored binary asset loaded once at startup
**Testing**: `cargo test --test component_vocabulary` for deterministic authored-value fidelity, literal absence, viewport integrity, state exhaustiveness, and ownership boundary; `make demo-live-component-library` for the browsable live gallery witness; existing `make test`, `make lint`, `make fmt-check` unchanged
**Target Platform**: macOS and Linux desktop at 1920×1080, and a compact 1280×800 handheld target
**Project Type**: single Rust workspace — hexagonal, one crate, contexts under `src/`
**Performance Goals**: interactive rendering stays event-driven at the existing 16 ms idle cadence; token and policy resolution adds no per-frame allocation beyond what egui already owns; typeface registration happens once before the first painted frame
**Constraints**: audio callback contract unchanged (zero allocation, locking, blocking, I/O, logging on the audio thread); the 512-event control-path acceptance fixture stays within its declared 50 ms ceiling; all five structural bands and the persistent side region retained at both authored viewports; no interactive target below 48 px
**Scale/Scope**: 17 semantic colors, 8 type styles, 6 spacing steps, 4 typeface weights, 2 density policies, 9 component states, 7 primitive families, 8 gallery pages

## Charter Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Charter present (`software-dev-default`, compact mode). Directives evaluated against this mission:

| Directive | Status | Note |
|---|---|---|
| DIRECTIVE_001 Architectural Integrity | **Pass** | The vocabulary is a new leaf resource in the Shell context. Nothing crosses a bounded-context boundary; `contextMap` is unchanged. |
| DIRECTIVE_003 Decision Documentation | **Pass** | The color-set union, the authored-vs-measured compact viewport distinction, and the loading/error reuse are recorded in `asset.ProductDesignAuthority` prompts and land in `DESIGN.md`. |
| DIRECTIVE_010 Specification Fidelity | **Pass** | Every FR traces to a declared crest-spec requirement (see Crest-Spec Derivation). No FR exists without one. |
| DIRECTIVE_024 Locality of Change | **Pass** | Additive in `src/shell` and `src/testing`; subtractive in one adapter file. No reducer, projection, transport, or render change. |
| DIRECTIVE_025 Boy Scout Rule | **Applied** | The `WindowInput` 17-vs-21 drift was corrected in the crest-spec phase because this mission edits that exact resource. Domain-matched, not silently absorbed. |
| DIRECTIVE_035 Bulk Edit | **Not applicable** | No identifier, path, key, or term is renamed across files. Constants are deleted at one site and replaced by a new named resource — new identifiers, not a rename. `change_mode` stays unset. |

No violations. Complexity Tracking is therefore omitted.

## Crest-Spec Derivation

Authored in `/spec-kitty.crest-spec` (commit `d02ad6b`) before this plan existed. `spec-kitty crest-spec
doctor`: **OK** — 7 contexts / 128 resources, 14 goals, 20 capabilities, 97 requirements, 30 validations,
19 witnesses.

### Resources this mission adds

| Canonical ID | Kind |
|---|---|
| `goal.build_from_component_vocabulary` | goal |
| `capability.component_vocabulary` | capability, 2 acceptance journeys |
| `requirement.semantic_visual_vocabulary` | functional |
| `requirement.authored_typeface_installation` | functional |
| `requirement.viewport_density_policy` | functional |
| `requirement.reusable_shell_primitives` | functional |
| `requirement.explicit_state_rendering` | functional |
| `requirement.component_state_ownership_boundary` | nonfunctional |
| `requirement.browsable_component_gallery` | functional |
| `requirement.component_vocabulary_behavioral_proof` | nonfunctional |
| `valueObject.Shell.SemanticVisualToken` | value object |
| `valueObject.Shell.AuthoredTypeface` | value object |
| `valueObject.Shell.ViewportDensityPolicy` | value object |
| `valueObject.Shell.ComponentState` | value object |
| `valueObject.Shell.ComponentGalleryPage` | value object |
| `valueObject.Shell.ComponentGalleryObservation` | value object |
| `assetKind.vendored-typeface` | asset kind |
| `asset.AzeretMonoTypeface` | asset |
| `asset.ComponentVocabularyAcceptanceTests` | asset |

### Resources this mission changes

| Canonical ID | Change |
|---|---|
| `valueObject.Shell.WindowInput` | Key vocabulary gains `Digit3`–`Digit8`; declared descriptor count corrected 17 → 33 and bound to the constructed descriptor by assertion. |
| `asset.ShellContextModules` | Gains targets and prompts for the vocabulary, typeface, policies, and primitives. |
| `asset.TestingContextModules` | Gains targets and prompts for the gallery scene. |
| `asset.BuildMakefile` | Declares `demo-live-component-library` as browsable, explicitly not an autonomous witness and not a `demo-live` alias. |
| `asset.ProductDesignAuthority` | Gains the color-union, compact-viewport-authored, and loading/error-reuse decisions. |
| `project.nonGoals.elaborate_ui`, `project.nonGoals.later_roadmap_phases`, `project.meta.avoid` | Narrowed deliberately from "no Phase 4 component library" to "no Phase 4 configurable controls or compositions"; extended to forbid visual literals outside the vocabulary. |
| `project.completion` | `build_from_component_vocabulary` added to `requiredGoals`; `component_vocabulary` added to `projectChecks`. |

### Resources this mission retires

None.

### Assets → files

| Asset | Produces |
|---|---|
| `asset.ShellContextModules` | `src/shell/*` — vocabulary, typeface installation, density policies, primitives |
| `asset.TestingContextModules` | `src/testing/*` — the browsable gallery scene |
| `asset.ComponentVocabularyAcceptanceTests` | `tests/component_vocabulary.rs` |
| `asset.BuildMakefile` | `Makefile` — the `demo-live-component-library` target |
| `asset.AzeretMonoTypeface` | `vendor/azeret-mono/*` — **already landed**, hash manifest and provenance present |
| `asset.ProductDesignAuthority` | `DESIGN.md` — the three recorded decisions |

The production repaint touches `src/adapter/eframe_graphical_window.rs`, covered by `asset.AdapterModules`.

### Proof covering the change

| Declared proof | Covers |
|---|---|
| `validation.component_vocabulary` | Authored-value fidelity, literal absence, viewport integrity, state exhaustiveness, page-binding totality, typed typeface failure |
| `witness.component_gallery` | 15 measured predicates from `make demo-live-component-library`, including `app_state_generation_delta = 0` |
| `evidence.component_vocabulary_contract` | Binds the validation and the witness to `capability.component_vocabulary` |
| 6 new entries in `proof/invariants.yaml` | Literal absence, no font substitution, no raw-viewport branching, closed state set, passive components, scene-local paging |

**No `data-model.md` and no `contracts/` are produced.** A crest-spec exists, so those artifacts are
forbidden — they would fork `valueObjects` and go stale. Value-object structure lives at the canonical IDs
above.

## Project Structure

### Documentation (this mission)

```
kitty-specs/crest-component-foundations-01KZ02H2/
├── spec.md              # Committed
├── plan.md              # This file
├── research.md          # Phase 0 output
├── quickstart.md        # Phase 1 output
├── checklists/
│   └── requirements.md  # Committed, all items pass
└── tasks/               # /spec-kitty.tasks output — NOT created here
```

### Source Code (repository root)

```
src/
├── shell/                          # ADDED THIS MISSION
│   ├── visual_token.rs             # valueObject.Shell.SemanticVisualToken
│   ├── authored_typeface.rs        # valueObject.Shell.AuthoredTypeface
│   ├── density_policy.rs           # valueObject.Shell.ViewportDensityPolicy
│   ├── component_state.rs          # valueObject.Shell.ComponentState
│   ├── primitives/                 # text roles, hairline, keyline, focus frame,
│   │                               #   value display, status mark, action hint
│   ├── window_input.rs             # CHANGED — Digit3..Digit8, 21 → 33 descriptors
│   └── (existing shell modules unchanged)
├── testing/
│   └── component_gallery_scene.rs  # ADDED — pages, digit binding, observation
├── adapter/
│   └── eframe_graphical_window.rs  # CHANGED — constants deleted, paints via vocabulary
└── (kernel, synth, mixer, real_time, control unchanged)

tests/
└── component_vocabulary.rs         # ADDED

vendor/azeret-mono/                 # ALREADY LANDED
Makefile                            # CHANGED — demo-live-component-library
DESIGN.md                           # CHANGED — three recorded decisions
```

**Structure Decision**: The vocabulary lives in `src/shell` rather than a new context. `context.Shell` is
already declared as the graphical-window boundary and already owns `ShellFrameObservation` and
`WindowInput`; the six new value objects were authored into it for that reason. Introducing a seventh
bounded context for presentation values would add a `contextMap` edge and a layer rule for no ownership
benefit. `src/adapter/eframe_graphical_window.rs` remains the only eframe-facing file and becomes a
consumer of the vocabulary rather than a definer of values.

## Implementation Concern Map

> **Note**: Implementation concerns are NOT work packages and are NOT executable units.
> `/spec-kitty.tasks` translates these into executable WPs — one concern may become
> multiple WPs; multiple small concerns may merge into one WP.

### IC-01 — Authored visual vocabulary

- **Purpose**: Declare every semantic color, type style, spacing step, and geometry value once, with the authored values exactly, so nothing downstream re-derives them.
- **Relevant requirements**: FR-001, NFR-001, NFR-002
- **Crest-spec**: `valueObject.Shell.SemanticVisualToken`, `requirement.semantic_visual_vocabulary`
- **Affected surfaces**: `src/shell/visual_token.rs`
- **Sequencing/depends-on**: none — this is the root; every other concern consumes it
- **Risks**: The authored values are already measured and recorded in `DESIGN.md:534-573` and confirmed against the design file. The real risk is the *guard*, not the values: proving literal absence requires a check that fails when a literal is reintroduced, and a guard that never fails is indistinguishable from no guard. It must be mutation-tested.

### IC-02 — Typeface installation

- **Purpose**: Register the vendored family in four weights before the first painted frame and bind each authored style to its weight, failing typed rather than substituting.
- **Relevant requirements**: FR-002, FR-010, NFR-006
- **Crest-spec**: `valueObject.Shell.AuthoredTypeface`, `requirement.authored_typeface_installation`, `asset.AzeretMonoTypeface`
- **Affected surfaces**: `src/shell/authored_typeface.rs`, `src/adapter/eframe_graphical_window.rs`
- **Sequencing/depends-on**: IC-01 (type styles are declared there)
- **Risks**: egui's default font stack silently absorbs a missing family — the failure mode is a screen that looks plausible and is wrong. The typed failure has to be proven by removing the asset, not by reading the registration code.

### IC-03 — Viewport density policies

- **Purpose**: Express both authored viewports as declared policies carrying bands, split, inset, row height, row pitch, and control geometry.
- **Relevant requirements**: FR-003, NFR-003
- **Crest-spec**: `valueObject.Shell.ViewportDensityPolicy`, `requirement.viewport_density_policy`
- **Affected surfaces**: `src/shell/density_policy.rs`, `src/adapter/eframe_graphical_window.rs`
- **Sequencing/depends-on**: IC-01
- **Risks**: Only the desktop viewport is authored in the design file. The compact policy is derived and needs the operator's eye, so it should reach a viewable state early rather than at the end. The existing adapter already proportionally scales the side region (`desired_side_width`, `eframe_graphical_window.rs:707`); that ad-hoc rule is what the policy replaces, and deleting it must not silently change desktop geometry.

### IC-04 — Component state vocabulary

- **Purpose**: Close the behavioral state set so adding a state names every rendering site, and guarantee every state carries text or shape beyond color.
- **Relevant requirements**: FR-005
- **Crest-spec**: `valueObject.Shell.ComponentState`, `requirement.explicit_state_rendering`
- **Affected surfaces**: `src/shell/component_state.rs`
- **Sequencing/depends-on**: IC-01
- **Risks**: Loading and Error have no authored appearance in the design file. They reuse the structural-edit vocabulary `DESIGN.md` already declares — this is a deliberate decision recorded in the spec, not an invention, and it should not drift into a second visual language.

### IC-05 — Reusable primitives

- **Purpose**: Provide text roles, hairlines, keylines, focus frames, value displays, status marks, and action hints as passive functions over immutable data plus explicit state.
- **Relevant requirements**: FR-004, FR-009
- **Crest-spec**: `requirement.reusable_shell_primitives`, `requirement.component_state_ownership_boundary`
- **Affected surfaces**: `src/shell/primitives/`
- **Sequencing/depends-on**: IC-01, IC-04
- **Risks**: The ownership boundary is easy to state and easy to violate under convenience pressure — a primitive that reaches for focus or a Patch value once becomes the pattern. The boundary needs a check, not a convention.

### IC-06 — Production shell repaint

- **Purpose**: Make `make run` show the authored design by deleting the adapter's constants and painting through the vocabulary and policies.
- **Relevant requirements**: FR-006, NFR-002, NFR-004
- **Crest-spec**: `asset.AdapterModules`, `asset.ShellContextModules`
- **Affected surfaces**: `src/adapter/eframe_graphical_window.rs`
- **Sequencing/depends-on**: IC-01, IC-02, IC-03, IC-05
- **Risks**: This is the concern that delivers the mission's only user-visible P1 outcome; if it slips, the gallery becomes a side project nobody's screen benefits from. The adapter also carries the normalized key vocabulary and its existing tests — the `Digit3`–`Digit8` extension lands here and must keep the exhaustiveness assertion honest rather than widening it to pass.

### IC-07 — Browsable gallery scene

- **Purpose**: One command opens a real window whose digit keys page through every declared state at both viewport sizes.
- **Relevant requirements**: FR-007, FR-008, NFR-005
- **Crest-spec**: `valueObject.Shell.ComponentGalleryPage`, `valueObject.Shell.ComponentGalleryObservation`, `requirement.browsable_component_gallery`, `witness.component_gallery`
- **Affected surfaces**: `src/testing/component_gallery_scene.rs`, `Makefile`
- **Sequencing/depends-on**: IC-05, IC-06
- **Risks**: Every other `demo-live-*` scene is autonomous and input-isolated; this one is deliberately the opposite. The risk runs both ways — giving this scene the witness contract would break paging, and copying this scene's input handling back into a witness would break the generation correlation those scenes depend on. `C-005` exists for exactly this.

### IC-08 — Measured proof

- **Purpose**: Prove authored-value fidelity, literal absence, viewport integrity, state coverage, page reachability, and typeface failure by measurement through the production render path.
- **Relevant requirements**: NFR-001, NFR-002, NFR-003, NFR-005, C-006
- **Crest-spec**: `validation.component_vocabulary`, `witness.component_gallery`, `evidence.component_vocabulary_contract`, `asset.ComponentVocabularyAcceptanceTests`
- **Affected surfaces**: `tests/component_vocabulary.rs`
- **Sequencing/depends-on**: IC-01 through IC-07
- **Risks**: The dominant risk in this mission. A test that asserts the token *names* exist while never comparing a rendered value is vacuous and would pass forever. Every check here compares values through the production render path, and the literal-absence guard and the state-coverage assertion must each be shown to fail when deliberately broken.
