# Tasks: Crest Component Controls and Compositions

**Mission**: `crest-component-controls-and-compositions-01KZ25VX`
**Branch**: `feat/crest-component-controls-and-compositions` → merges to `main`
**Plan**: [plan.md](plan.md) · **Spec**: [spec.md](spec.md) · **Research**: [research.md](research.md)

46 subtasks across 8 work packages. Every `owned_files` entry traces to a declared crest-spec asset; see plan.md § Crest-Spec Derivation.

Subtask rows below are **reference rows, not checkboxes**. Record completion with
`spec-kitty agent tasks mark-status T0xx --status done` — the event log is the authority.

## Subtask Index

| ID | Description | WP | Parallel |
|----|-------------|----|----------|
| T001 | Declare the closed `PresentationRole` union and its exhaustiveness assertion | WP01 | |
| T002 | Declare the closed `ComponentControl` union with per-variant state applicability | WP01 | |
| T003 | Write the failing totality assertion for kind × role selection | WP01 | |
| T004 | Implement the total kind × role selector | WP01 | |
| T005 | Define the control call signature and the typed `ControlIntent` it returns | WP01 | |
| T006 | Declare the closed `ShellComposition` union and its region binding | WP01 | |
| T007 | Wire both module trees into `src/shell/visual/mod.rs` | WP01 | |
| T008 | Build the parameter row against its Figma specimen | WP02 | [P] |
| T009 | Build the choice row against its Figma specimen | WP02 | [P] |
| T010 | Build the toggle against its Figma specimen | WP02 | [P] |
| T011 | Build the browser row against its Figma specimen | WP02 | [P] |
| T012 | Assert every applicable state renders non-color evidence for all four | WP02 | |
| T013 | Build the compact slider against its Figma specimen | WP03 | [P] |
| T014 | Build the fader against its Figma specimen | WP03 | [P] |
| T015 | Build the meter against its Figma specimen | WP03 | [P] |
| T016 | Build the modal option against its Figma specimen | WP03 | [P] |
| T017 | Assert every applicable state renders non-color evidence for all four | WP03 | |
| T018 | Build the application shell composition | WP04 | |
| T019 | Build the context switch composition | WP04 | [P] |
| T020 | Build the identity header composition | WP04 | [P] |
| T021 | Build the footer composition | WP04 | [P] |
| T022 | Assert the frame compositions declare no visual value of their own | WP04 | |
| T023 | Build the section composition | WP05 | [P] |
| T024 | Build the Patch strip row composition | WP05 | [P] |
| T025 | Build the Utility/Inspector panel composition | WP05 | [P] |
| T026 | Implement and assert the omit-or-mark-unavailable rule | WP05 | |
| T027 | Record every designed structure the projection does not drive | WP05 | |
| T028 | Capture the pre-reduction behavioral baseline | WP06 | |
| T029 | Move the five band and workspace paint functions into compositions | WP06 | |
| T030 | Move the side region and control painting into compositions | WP06 | |
| T031 | Keep event translation and the frame observation intact in the adapter | WP06 | |
| T032 | Delete the vacated adapter code and verify the ≤512-line threshold | WP06 | |
| T033 | Verify the full suite passes with no existing test modified | WP06 | |
| T034 | Extend `WindowInput` by four keys and update the descriptor count | WP07 | |
| T035 | Add the seven control and composition gallery pages | WP07 | |
| T036 | Add bidirectional non-wrapping page stepping | WP07 | |
| T037 | Pin the eight pre-existing digit bindings against regression | WP07 | |
| T038 | Emit control, composition, and silence coverage in the observation | WP07 | |
| T039 | Assert the gallery constructs no audio output and no MIDI source | WP07 | |
| T040 | Scaffold `tests/component_composition.rs` and its acceptance marker | WP08 | |
| T041 | Prove kind × role totality and control reachability | WP08 | |
| T042 | Prove every shipped region is a composition and the adapter is empty | WP08 | |
| T043 | Prove the no-placeholder rule and the ownership boundary | WP08 | |
| T044 | Prove viewport integrity survived recomposition | WP08 | |
| T045 | Extend `tests/component_vocabulary.rs` for page reachability | WP08 | |
| T046 | Correct `DESIGN.md` state list and amend the `ROADMAP.md` demo bullet | WP08 | [P] |

---

## Phase 1 — Foundation

### WP01 — Component family scaffold and the total selector

**Prompt**: [tasks/WP01-component-family-scaffold.md](tasks/WP01-component-family-scaffold.md)
**Priority**: P1 · **Depends on**: none · **Estimated prompt**: ~420 lines

**Goal**: establish both closed families, the presentation-role vocabulary, the total kind × role selector, and the shared intent type — the contract every later work package plugs into.

**Independent test**: the selector's totality assertion passes with all eight control variants registered and every declared kind × role pair resolving, before any control has a body.

**Subtasks**: T001, T002, T003, T004, T005, T006, T007

**Why it lands alone and first**: the role vocabulary is the load-bearing decision of the whole mission (research.md R-01). Every control and every composition depends on it, and getting it wrong forces rework across fifteen files. It also removes all `mod.rs` contention — WP01 owns the three module roots, so WP02 through WP05 own only leaf files and never collide.

**Risks**: the selector is a two-dimensional exhaustive match. Rust will not check exhaustiveness across a pair automatically unless it is written as a match on a tuple — write it that way, so an added kind or role is a compile error rather than a runtime assertion.

---

## Phase 2 — Controls

### WP02 — Row and choice controls

**Prompt**: [tasks/WP02-row-and-choice-controls.md](tasks/WP02-row-and-choice-controls.md)
**Priority**: P1 · **Depends on**: WP01 · **Estimated prompt**: ~430 lines

**Goal**: build the four controls that appear as listed rows — parameter row, choice row, toggle, browser row — against their Figma specimens, in every state each declares applicable, at both viewports.

**Independent test**: render each of the four in every applicable state and confirm non-color evidence is present and legible in each.

**Subtasks**: T008, T009, T010, T011, T012

**Parallel opportunities**: T008–T011 are four disjoint files and can be worked simultaneously. T012 depends on all four.

**Risks**: Figma extraction is the slow part. A control whose specimen is missing or ambiguous must be raised, not approximated — an approximated control looks authoritative in the gallery and is worse than a missing one.

---

### WP03 — Continuous and modal controls

**Prompt**: [tasks/WP03-continuous-and-modal-controls.md](tasks/WP03-continuous-and-modal-controls.md)
**Priority**: P1 · **Depends on**: WP01 · **Estimated prompt**: ~450 lines

**Goal**: build the compact slider, fader, meter, and modal option. These are the painting-heavy controls — research.md R-02 forbids using egui widgets for appearance, so each is drawn through `Painter` over an allocated response.

**Independent test**: render each of the four in every applicable state and confirm non-color evidence; confirm the meter presents the level the view data reports and reads nothing from the audio boundary.

**Subtasks**: T013, T014, T015, T016, T017

**Parallel opportunities**: T013–T016 are four disjoint files. Runs fully parallel with WP02.

**Risks**: the meter is the one control with no live signal in this slice (C-001, no audio). It must render from view data at whatever level is reported, including resting, and must not acquire an audio dependency to look convincing.

---

## Phase 3 — Compositions

### WP04 — Frame compositions

**Prompt**: [tasks/WP04-frame-compositions.md](tasks/WP04-frame-compositions.md)
**Priority**: P1 · **Depends on**: WP01 · **Estimated prompt**: ~400 lines

**Goal**: build the four compositions that form the shell frame — application shell, context switch, identity header, footer — each resolving its bands and split from `ViewportDensityPolicy` and declaring no visual value of its own.

**Independent test**: render the frame at both viewports and confirm every band height, split width, and inset came from the density policy, with no literal in any of the four files.

**Subtasks**: T018, T019, T020, T021, T022

**Parallel opportunities**: T019–T021 are disjoint. T018 arranges the others and should follow them.

**Risks**: `ApplicationShell` is the composition the adapter will hand its panels to. Its signature determines how much WP06 can move; get it wrong and the adapter cannot shed its paint order.

---

### WP05 — Content compositions and the no-placeholder rule

**Prompt**: [tasks/WP05-content-compositions.md](tasks/WP05-content-compositions.md)
**Priority**: P1 · **Depends on**: WP01, WP02, WP03 · **Estimated prompt**: ~440 lines

**Goal**: build the section, Patch strip row, and Utility/Inspector panel compositions — the three that arrange controls — and implement the rule that a designed structure with no view data behind it is omitted or marked explicitly unavailable, never invented.

**Independent test**: hand a composition a projection slice missing part of its designed structure and confirm it omits or marks the gap and paints no value that was not in the view data.

**Subtasks**: T023, T024, T025, T026, T027

**Parallel opportunities**: T023–T025 are disjoint files.

**Risks**: this is where the placeholder temptation lives (C-003). T027 exists because the list of designed-but-undriven structures is the real input to Phase 5 — record it as it is found rather than reconstructing it later.

---

## Phase 4 — Production recomposition

### WP06 — Adapter reduction

**Prompt**: [tasks/WP06-adapter-reduction.md](tasks/WP06-adapter-reduction.md)
**Priority**: P1 · **Depends on**: WP04, WP05 · **Estimated prompt**: ~470 lines

**Goal**: move every region and control paint out of `src/adapter/eframe_graphical_window.rs` into the compositions and controls, leaving window plumbing, event translation, and the frame-observation emit. End at ≤512 lines from 1,282.

**Independent test**: `make run` renders identically, the full existing suite passes with no test file modified, and `wc -l src/adapter/eframe_graphical_window.rs` reports ≤512.

**Subtasks**: T028, T029, T030, T031, T032, T033

**Risks**: **the highest-risk work package.** NFR-005 forbids editing any existing shell, projection, or focus test to accommodate the move — a failure there means the recomposition changed behavior and the recomposition is what gets fixed. The `ShellFrameObservation` construction must survive intact, because it is exactly what those tests assert on (research.md R-03).

---

## Phase 5 — Gallery

### WP07 — Gallery pages, stepping, and measured silence

**Prompt**: [tasks/WP07-gallery-pages-and-silence.md](tasks/WP07-gallery-pages-and-silence.md)
**Priority**: P1 · **Depends on**: WP02, WP03, WP04, WP05 · **Estimated prompt**: ~460 lines

**Goal**: grow the gallery from eight pages to fifteen, add the four new window keys and bidirectional stepping so every page is reachable, and emit control, composition, and silence coverage in the observation.

**Independent test**: run `make demo-live-component-library`, press every digit and both bracket keys, and confirm all fifteen pages appear with the original eight bindings unmoved.

**Subtasks**: T034, T035, T036, T037, T038, T039

**Risks**: FR-012 pins the eight existing digit bindings — an operator who knows `Digit4` is InteractionStates must keep finding it there. The `WindowInput` descriptor count invariant (33 → 41) is asserted, so the key extension and the count update must land in the same change or the build breaks.

---

## Phase 6 — Proof and document corrections

### WP08 — Deterministic proof and the DESIGN/ROADMAP corrections

**Prompt**: [tasks/WP08-deterministic-proof-and-corrections.md](tasks/WP08-deterministic-proof-and-corrections.md)
**Priority**: P1 · **Depends on**: WP06, WP07 · **Estimated prompt**: ~490 lines

**Goal**: deliver `tests/component_composition.rs` as the declared project completion check, extend `tests/component_vocabulary.rs` for page reachability, and land the two document corrections.

**Independent test**: `cargo test --release --test component_composition` emits `CREST_ACCEPTANCE component_composition passed` and exits 0.

**Subtasks**: T040, T041, T042, T043, T044, T045, T046

**Parallel opportunities**: T046 depends on nothing in this WP and can land at any time.

**Risks**: C-007 bounds this work package. It proves the library exists and is used; it does not build a layer that verifies the verification. If a check here starts checking another check, it is out of scope and should be deleted.

---

## Dependency Graph

```
WP01 ──┬──> WP02 ──┐
       ├──> WP03 ──┤
       └──> WP04 ──┼──> WP05 ──> WP06 ──┐
                   │                     ├──> WP08
                   └─────────────────────┴──> WP07 ──┘
```

## Parallelization

- **After WP01**: WP02, WP03, and WP04 run concurrently — three lanes, fully disjoint file sets.
- **Within WP02/WP03/WP04/WP05**: the per-control and per-composition subtasks are marked `[P]` and touch one file each.
- **Serialized**: WP06 must follow both composition packages; WP08 must follow WP06 and WP07.

## MVP Scope

**WP01 + WP02 + WP04** delivers a coherent, demonstrable slice: the family contract, the four listed-row controls, and the shell frame assembled from compositions. It is worth shipping alone and unblocks everything else.

## Ownership Map

| WP | authoritative_surface | owned_files |
|----|----------------------|-------------|
| WP01 | `src/shell/visual/controls/` | `controls/mod.rs`, `compositions/mod.rs`, `visual/mod.rs` |
| WP02 | `src/shell/visual/controls/` | `parameter_row.rs`, `choice_row.rs`, `toggle.rs`, `browser_row.rs` |
| WP03 | `src/shell/visual/controls/` | `compact_slider.rs`, `fader.rs`, `meter.rs`, `modal_option.rs` |
| WP04 | `src/shell/visual/compositions/` | `application_shell.rs`, `context_switch.rs`, `identity_header.rs`, `footer.rs` |
| WP05 | `src/shell/visual/compositions/` | `section.rs`, `patch_strip_row.rs`, `utility_inspector_panel.rs` |
| WP06 | `src/adapter/` | `eframe_graphical_window.rs` |
| WP07 | `src/testing/` | `component_gallery_scene.rs`, `src/shell/window_input.rs` |
| WP08 | `tests/` | `component_composition.rs`, `component_vocabulary.rs`, `DESIGN.md`, `ROADMAP.md` |

No two work packages share an owned file.
