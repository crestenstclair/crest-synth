# Tasks: Crest Component Controls and Compositions

**Mission**: `crest-component-controls-and-compositions-01KZ25VX`
**Branch**: `feat/crest-component-controls-and-compositions` → merges to `main`
**Plan**: [plan.md](plan.md) · **Spec**: [spec.md](spec.md) · **Research**: [research.md](research.md)

52 subtasks across 9 work packages. Every `owned_files` entry traces to a declared crest-spec asset; see plan.md § Crest-Spec Derivation.

**WP09 was added mid-mission**, after implementation proved the declared composition family incomplete (finding F-09). The crest-spec amendment declaring the eighth composition `MixerStripBank` and the `mixerColumn` policy member was authored first, in `d91fbf5`; WP09 implements it.

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
| T047 | Author the mixer-column geometry on `ViewportDensityPolicy` | WP09 | |
| T048 | Retire the fader's surface-local column derivation onto the policy | WP09 | [P] |
| T049 | Build the mixer strip bank as a group of groups | WP09 | [P] |
| T050 | Title and mark unavailable at both levels | WP09 | |
| T051 | Wire `MixerStripBank` into the composition family | WP09 | |
| T052 | Drive the bank through a real render pass and prove sixteen seat | WP09 | |

Rows are ordered by ID. **WP09's subtasks run before WP06's and WP07's** despite their higher numbers — it was added mid-mission and its IDs continue the sequence rather than renumbering the mission (C-006's additive-only rule applied to task identity). The phase sections and the dependency graph below carry execution order.

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

## Phase 3b — The eighth composition (mid-mission amendment)

### WP09 — Mixer strip bank and the mixer-column policy

**Prompt**: [tasks/WP09-mixer-strip-bank.md](tasks/WP09-mixer-strip-bank.md)
**Priority**: P1 · **Depends on**: WP01, WP03, WP05 · **Estimated prompt**: ~470 lines

**Goal**: build `MixerStripBank` — sixteen mixer track columns side by side in the main workspace — and give `ViewportDensityPolicy` the `mixerColumn` geometry that lets it allocate them rather than consume the surface.

**Independent test**: drive the bank through a real `egui` pass at both viewports and confirm sixteen columns seat inside the main-surface content width, none narrower than the authored minimum target, with nothing scrolled, clipped, or elided to achieve it.

**Subtasks**: T047, T048, T049, T050, T051, T052

**Why it exists at all**: `paint_patch_workspace` landed in `Section`; `paint_mixer_workspace` landed nowhere in the closed seven (finding **F-09**). A `Section` at `VerticalStrip` is one track *column*; the bank of sixteen had no composition. The cheap alternative — a layout axis on `Section` — was tested and rejected: `Section`'s entries are typed `&[SemanticControlViewModel]` (controls) while a bank's entries are columns, each itself a titled group. **The bank is a group of groups**, so the gap is nesting, not direction. The crest-spec amendment was authored first (`d91fbf5`); this work package implements it.

**Why it also owns `density.rs`**: `src/shell/visual/density.rs` is a Phase 4a file no work package owned, which is how `MIXER_TRACK_MIN_WIDTH_PX = 176.0` and `WORKSPACE_TITLE_ROW_PX = 42.0` both ended up as adapter-local literals with nothing to resolve them from (F-10 item 10). WP09 takes ownership for the one member the crest-spec declares — `mixerColumn`. **`WORKSPACE_TITLE_ROW_PX` stays with WP06**, ruled on in WP09's prompt: the crest-spec declares no workspace-title member, the 42 is no more authored than the 176, and its consumers are WP05's closed files.

**Parallel opportunities**: T048 and T049 touch disjoint files and both follow T047.

**Risks**: **reproducing the shipped mixer is the risk.** The horizontal `ScrollArea` at `eframe_graphical_window.rs:512` exists only because the invented 176 px column is more than double the authored 82, and it is the divergence rather than the baseline — Figma `42:25` seats all sixteen at width 82 on pitch 86 inside 1452 px of Desktop content, which is what `DESIGN.md:462` requires. An implementer who ports the current behavior forward ships the defect under a new file name. The second risk is subtler: dividing 1452 by sixteen fits, arrives at 90.75, and consumes slack the design authored deliberately.

---

## Phase 4 — Production recomposition

### WP06 — Adapter reduction

**Prompt**: [tasks/WP06-adapter-reduction.md](tasks/WP06-adapter-reduction.md)
**Priority**: P1 · **Depends on**: WP04, WP05, WP09 · **Estimated prompt**: ~470 lines

**Goal**: move every region and control paint out of `src/adapter/eframe_graphical_window.rs` into the compositions and controls, leaving window plumbing, event translation, and the frame-observation emit. End at ≤512 lines from 1,282.

**Independent test**: `make run` renders identically, the full existing suite passes with no test file modified, and `wc -l src/adapter/eframe_graphical_window.rs` reports ≤512.

**Subtasks**: T028, T029, T030, T031, T032, T033

**Why it now depends on WP09**: the adapter cannot shed `paint_mixer_workspace` until a composition exists to receive it. Before WP09, `Section` on MIXER resolved `main_for` → `MixerMain` and painted all sixteen tracks **flat at `ListedRow`** — wiring that into `mainWorkspace` would have regressed the operator from sixteen columns to one long vertical list.

**Risks**: **the highest-risk work package.** NFR-005 forbids editing any existing shell, projection, or focus test to accommodate the move — a failure there means the recomposition changed behavior and the recomposition is what gets fixed. The `ShellFrameObservation` construction must survive intact, because it is exactly what those tests assert on (research.md R-03). Two adapter-local values need deliberate handling rather than preservation: the horizontal `ScrollArea` at `:512` is **retired, not reproduced** (it exists only because the invented `MIXER_TRACK_MIN_WIDTH_PX = 176.0` is more than double the authored 82), and `WORKSPACE_TITLE_ROW_PX = 42.0` is deleted with the rest of the paint rather than compensated for — see the ruling in WP09's prompt and finding F-11.

---

## Phase 5 — Gallery

### WP07 — Gallery pages, stepping, and measured silence

**Prompt**: [tasks/WP07-gallery-pages-and-silence.md](tasks/WP07-gallery-pages-and-silence.md)
**Priority**: P1 · **Depends on**: WP02, WP03, WP04, WP05, WP09 · **Estimated prompt**: ~460 lines

**Goal**: grow the gallery from eight pages to fifteen, add the four new window keys and bidirectional stepping so every page is reachable, and emit control, composition, and silence coverage in the observation.

**Independent test**: run `make demo-live-component-library`, press every digit and both bracket keys, and confirm all fifteen pages appear with the original eight bindings unmoved.

**Subtasks**: T034, T035, T036, T037, T038, T039

**Why it now depends on WP09**: the gallery's coverage assertion requires *every declared `ShellComposition`* to appear on a page with representative content, and the family is now eight. Without WP09 in its base, WP07's lane would see `SHELL_COMPOSITION_COUNT == 7`, cover seven, pass, and break on merge. No new page is needed — `StripPanelAndFooter` hosts the bank and the coverage invariant is generic.

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

The previous ASCII drawing here was wrong — it showed WP05 flowing from WP04 and WP07 hanging off the WP06/WP08 line, neither of which the frontmatter or `lanes.json` declares (analysis-report finding **A5**). It is replaced by an edge table, because a table transcribes the same field `lanes.json` computes from and cannot drift into a different shape.

| WP | Direct dependencies | Depth |
|----|---------------------|-------|
| WP01 | — | 0 |
| WP02 | WP01 | 1 |
| WP03 | WP01 | 1 |
| WP04 | WP01 | 1 |
| WP05 | WP01, WP02, WP03 | 2 |
| WP09 | WP01, WP03, WP05 | 3 |
| WP06 | WP04, WP05, WP09 | 4 |
| WP07 | WP02, WP03, WP04, WP05, WP09 | 4 |
| WP08 | WP06, WP07 | 5 |

Depth is the longest path from WP01, so work packages sharing a depth can run concurrently once their dependencies are met:

```
depth 0   WP01
depth 1   WP02   WP03   WP04
depth 2   WP05
depth 3   WP09
depth 4   WP06   WP07
depth 5   WP08
```

## Parallelization

- **After WP01**: WP02, WP03, and WP04 run concurrently — three lanes, fully disjoint file sets.
- **Within WP02/WP03/WP04/WP05/WP09**: the per-control, per-composition, and per-file subtasks are marked `[P]` and touch one file each.
- **After WP09**: WP06 and WP07 run concurrently. They were already independent of each other; WP09 pushes both out one depth without serializing them against each other.
- **Serialized**: WP09 must follow WP05, because a mixer column *is* a `Section` and `mark_unavailable` is WP05's shared C-003 mechanism. WP06 and WP07 must both follow WP09. WP08 must follow WP06 and WP07.

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
| WP09 | `src/shell/visual/compositions/` | `mixer_strip_bank.rs`, `src/shell/visual/density.rs` |
| WP06 | `src/adapter/` | `eframe_graphical_window.rs` |
| WP07 | `src/testing/` | `component_gallery_scene.rs`, `src/shell/window_input.rs` |
| WP08 | `tests/` | `component_composition.rs`, `component_vocabulary.rs`, `DESIGN.md`, `ROADMAP.md` |

No two work packages share an owned file.

**Declared narrow edits.** Two files are edited by work packages that do not own them, under the operator-approved narrow-edit convention: the editing work package adds only its own declared items, reorders nothing, and touches nothing else. Each is enumerated in the editing prompt so a reviewer can diff against a closed list.

| File | Owner | Narrow editors | Scope of the edit |
|---|---|---|---|
| `src/shell/visual/compositions/mod.rs` | WP01 | WP04, WP05, WP09 | One `pub mod` line and one `renderer` arm each. WP09 additionally bumps the family count 7 → 8 and updates the two tests that name the old count — structurally required by an eighth variant, enumerated as nine items in WP09 T051 |
| `src/shell/visual/controls/fader.rs` | WP03 | WP09 | Two function bodies (`column_pitch_px`, `column_width_px`) delegating to `ViewportDensityPolicy::mixer_column()`, plus the two doc paragraphs the delegation falsifies. Every WP03 assertion must pass unmodified (WP09 T048) |
