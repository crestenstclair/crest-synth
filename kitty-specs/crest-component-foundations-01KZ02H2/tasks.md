# Tasks: Crest Component Foundations

**Mission**: `crest-component-foundations-01KZ02H2`
**Branch**: `feat/crest-component-foundations` | **Date**: 2026-08-02
**Input**: [spec.md](./spec.md) · [plan.md](./plan.md) · [research.md](./research.md)

Work packages derive from the crest-spec's `assets[]`. Every `owned_files` entry traces to a declared
asset file pattern; nothing here restates implementation intent the crest-spec already carries.

## Asset → work package trace

| Crest-spec asset | File pattern | Work packages |
|---|---|---|
| `asset.ShellContextModules` | `src/*/*` | WP01, WP02, WP03 |
| `asset.AdapterModules` | `src/*/*` | WP04 |
| `asset.TestingContextModules` | `src/*/*` | WP05 |
| `asset.ComponentVocabularyAcceptanceTests` | `tests/*.rs` | WP06 |
| `asset.BuildMakefile` | `Makefile` | WP05 |
| `asset.ProductDesignAuthority` | `DESIGN.md` | WP01 |
| `asset.AzeretMonoTypeface` | `vendor/azeret-mono/*` | already landed — no WP |

## Subtask Index

*Reference table only. Completion is event-sourced — record it with `spec-kitty agent tasks mark-status`.*

| ID | Description | WP | Parallel |
|---|---|---|---|
| T001 | Create `src/shell/visual/` module tree with compiling stubs | WP01 | |
| T002 | Declare the 17 semantic colors with authored values | WP01 | |
| T003 | Declare the 8 type styles, 6 spacing steps, and geometry | WP01 | [P] |
| T004 | Register the vendored typeface with typed failure on absence | WP01 | |
| T005 | Record the three durable decisions in `DESIGN.md` | WP01 | [P] |
| T006 | Prove declared values equal the authored table | WP01 | |
| T007 | Implement the Desktop density policy from measured geometry | WP02 | |
| T008 | Author the Steam Deck density policy | WP02 | |
| T009 | Expose the policy API the adapter will consume | WP02 | |
| T010 | Declare the closed nine-value `ComponentState` | WP02 | [P] |
| T011 | Map Loading and Error onto the structural-edit vocabulary | WP02 | |
| T012 | Prove both policies retain bands, split, and minimum target | WP02 | |
| T013 | Text-role primitives | WP03 | |
| T014 | Hairline and keyline primitives | WP03 | [P] |
| T015 | Focus-frame primitive with the authored halo | WP03 | [P] |
| T016 | Value-display primitive | WP03 | [P] |
| T017 | Status-mark primitive covering Loading and Error | WP03 | [P] |
| T018 | Action-hint primitive | WP03 | [P] |
| T019 | Enforce the component ownership boundary | WP03 | |
| T020 | Delete the seven adapter constants; paint through the vocabulary | WP04 | |
| T021 | Install the typeface at startup and surface its failure | WP04 | |
| T022 | Replace band, split, and side-width constants with the policy | WP04 | |
| T023 | Extend `WindowKey` with `Digit3`–`Digit8`; 21 → 33 descriptors | WP04 | [P] |
| T024 | Make unbound digits produce no semantic action | WP04 | |
| T025 | Confirm `make run` changed and existing shell tests still pass | WP04 | |
| T026 | Gallery scene skeleton and the closed page set | WP05 | |
| T027 | Digit binding, page switch, and unbound-digit retention | WP05 | |
| T028 | Render pages 1–4 | WP05 | [P] |
| T029 | Render pages 5–8 | WP05 | [P] |
| T030 | Show both authored viewports | WP05 | |
| T031 | Emit `ComponentGalleryObservation` after painting | WP05 | |
| T032 | Add the CLI flag and the Makefile target | WP05 | |
| T033 | Acceptance target skeleton and marker | WP06 | |
| T034 | Prove authored-value fidelity through the render path | WP06 | |
| T035 | Literal-absence guard, and prove the guard fails | WP06 | |
| T036 | Prove viewport integrity at both authored sizes | WP06 | [P] |
| T037 | Prove state exhaustiveness, non-color legibility, page totality | WP06 | [P] |
| T038 | Prove the typeface-missing typed failure | WP06 | [P] |

---

## WP01 — Authored visual vocabulary and typeface

**Prompt**: [`tasks/WP01-authored-visual-vocabulary.md`](./tasks/WP01-authored-visual-vocabulary.md)
**Priority**: P1 · **Depends on**: none · **Estimated prompt size**: ~430 lines

**Goal**: Declare every semantic color, type style, spacing step, and geometry value once, with the
authored values exactly, and make the vendored typeface renderable.

**Independent test**: `cargo test` proves each declared value equals its authored counterpart; deleting
`vendor/azeret-mono/` produces a typed error rather than a fallback face.

**Included subtasks**: T001, T002, T003, T004, T005, T006

**Implementation sketch**: Create the module tree with stubs so everything compiles from the first
commit → declare colors → declare type styles, spacing, geometry → implement typeface registration →
record the decisions in `DESIGN.md` → assert values against the authored table.

**Parallel opportunities**: T003 and T005 are independent of T002.

**Risks**: The values are already measured and confirmed; the risk is in the *shape* of the declaration.
Everything downstream imports from here, so a token that is awkward to consume gets worked around rather
than fixed.

---

## WP02 — Viewport density policies and component state

**Prompt**: [`tasks/WP02-density-policies-and-state.md`](./tasks/WP02-density-policies-and-state.md)
**Priority**: P1 · **Depends on**: WP01 · **Estimated prompt size**: ~420 lines

**Goal**: Express both authored viewports as declared policies, and close the behavioral state set.

**Independent test**: A test resolves both policies and asserts every structural band, the persistent
side region, and the 48 px minimum target survive at 1920×1080 and 1280×800.

**Included subtasks**: T007, T008, T009, T010, T011, T012

**Implementation sketch**: Desktop policy from measured geometry → Steam Deck policy authored from it →
consumer-facing API → closed `ComponentState` → Loading/Error mapping → viewport assertions.

**Parallel opportunities**: T010 is independent of T007–T009.

**Risks**: The Steam Deck policy is authored, not measured — it needs the operator's eye and should
become viewable before the mission ends, not at the end. The desktop policy must reproduce today's
geometry exactly; a silent shift there would be a regression disguised as a refactor.

---

## WP03 — Reusable primitives

**Prompt**: [`tasks/WP03-reusable-primitives.md`](./tasks/WP03-reusable-primitives.md)
**Priority**: P1 · **Depends on**: WP01, WP02 · **Estimated prompt size**: ~480 lines

**Goal**: Provide the seven primitive families as passive functions over immutable data plus explicit
state.

**Independent test**: Each primitive renders every applicable `ComponentState` with text or shape beyond
color, and a check proves no primitive reads application state.

**Included subtasks**: T013, T014, T015, T016, T017, T018, T019

**Implementation sketch**: Text roles first (everything else composes them) → hairline and keyline →
focus frame → value display → status mark → action hint → ownership boundary check.

**Parallel opportunities**: T014–T018 are mutually independent once T013 lands.

**Risks**: The ownership boundary is the one to watch. A primitive that reaches for focus state once
becomes the template for the next six.

---

## WP04 — Production shell repaint and key vocabulary

**Prompt**: [`tasks/WP04-production-shell-repaint.md`](./tasks/WP04-production-shell-repaint.md)
**Priority**: P1 · **Depends on**: WP01, WP02, WP03 · **Estimated prompt size**: ~450 lines

**Goal**: Make `make run` show the authored design, and normalize the digit keys the gallery will bind.

**Independent test**: Launch `make run` and compare against the design reference; the seven constants at
`src/adapter/eframe_graphical_window.rs:28-34` no longer exist.

**Included subtasks**: T020, T021, T022, T023, T024, T025

**Implementation sketch**: Delete the constants and paint through tokens → install the typeface at
startup → swap band/split/side-width constants for the policy → extend the key vocabulary → make unbound
digits inert → confirm nothing regressed.

**Parallel opportunities**: T023 touches a different file from T020–T022.

**Risks**: This WP delivers the mission's only user-visible P1 outcome — if it slips, the gallery is a
side project no one's screen benefits from. The existing exhaustiveness assertion on the key vocabulary
must be kept honest, not widened to pass.

---

## WP05 — Browsable gallery scene

**Prompt**: [`tasks/WP05-browsable-gallery-scene.md`](./tasks/WP05-browsable-gallery-scene.md)
**Priority**: P2 · **Depends on**: WP03, WP04 · **Estimated prompt size**: ~500 lines

**Goal**: One command opens a real window whose digit keys page through every declared state at both
authored viewport sizes.

**Independent test**: `make demo-live-component-library`, press 1–8, confirm each page appears and that
an unbound digit changes nothing.

**Included subtasks**: T026, T027, T028, T029, T030, T031, T032

**Implementation sketch**: Scene skeleton and closed page set → digit binding → pages 1–4 → pages 5–8 →
both viewports → observation after paint → CLI flag and Makefile target.

**Parallel opportunities**: T028 and T029 are independent once T026 and T027 land.

**Risks**: Every other `demo-live-*` scene is autonomous and input-isolated; this one is deliberately the
opposite. The danger runs both ways — giving this scene the witness contract breaks paging; copying its
input handling into a witness breaks generation correlation.

---

## WP06 — Measured proof

**Prompt**: [`tasks/WP06-measured-proof.md`](./tasks/WP06-measured-proof.md)
**Priority**: P1 · **Depends on**: WP05 · **Estimated prompt size**: ~470 lines

**Goal**: Prove every claim by measurement through the production render path, and prove each guard can
fail.

**Independent test**: `cargo test --test component_vocabulary` prints
`CREST_ACCEPTANCE component_vocabulary passed` only when every declared check holds.

**Included subtasks**: T033, T034, T035, T036, T037, T038

**Implementation sketch**: Skeleton and marker → authored-value fidelity → literal-absence guard plus its
own failure proof → viewport integrity → state and page coverage → typeface-missing failure.

**Parallel opportunities**: T036, T037, T038 are mutually independent once T033 lands.

**Risks**: The dominant risk in this mission. A test asserting the token *names* exist while never
comparing a rendered value would pass forever and prove nothing. Every check compares values through the
production render path, and the literal-absence guard and the coverage assertion must each be
demonstrated failing on a deliberate break.

---

## Dependency graph

```
WP01 ──► WP02 ──► WP03 ──► WP04 ──► WP05 ──► WP06
   └──────────────────────────┘
```

WP04 depends on WP01, WP02, and WP03; WP05 depends on WP03 and WP04; WP06 depends on WP05.

**This mission is mostly serial, and the plan does not pretend otherwise.** Every layer consumes the one
below it — primitives cannot exist before the vocabulary, the repaint cannot precede the primitives, and
the gallery renders what the repaint proves. The parallelism that exists is *within* work packages
(marked `[P]` above), not across them. Expect roughly one lane.

## MVP scope

**WP01 → WP04.** That sequence alone delivers User Story 1: `make run` shows the authored design. The
gallery (WP05) and the measured proof (WP06) complete the mission, but the P1 user-visible outcome lands
at the end of WP04.

## Requirement coverage

| WP | Requirements |
|---|---|
| WP01 | FR-001, FR-002, FR-010, NFR-006 |
| WP02 | FR-003, FR-005 |
| WP03 | FR-004, FR-005, FR-009 |
| WP04 | FR-006, NFR-004 |
| WP05 | FR-007, FR-008, NFR-005 |
| WP06 | NFR-001, NFR-002, NFR-003 |
