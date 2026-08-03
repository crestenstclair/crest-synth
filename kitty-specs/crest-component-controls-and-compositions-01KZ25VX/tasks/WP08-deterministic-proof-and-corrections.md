---
work_package_id: WP08
title: Deterministic proof and the DESIGN/ROADMAP corrections
dependencies:
- WP06
- WP07
requirement_refs:
- C-003
- FR-001
- FR-005
- FR-006
- FR-009
- FR-010
- FR-013
- FR-014
- NFR-004
planning_base_branch: feat/crest-component-controls-and-compositions
merge_target_branch: feat/crest-component-controls-and-compositions
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-controls-and-compositions. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-controls-and-compositions unless the human explicitly redirects the landing branch.
subtasks:
- T040
- T041
- T042
- T043
- T044
- T045
- T046
phase: Phase 6 - Proof and corrections
history:
- at: '2026-08-02T21:46:28Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent: claude
agent_profile: reviewer-renata
authoritative_surface: tests/
create_intent:
- tests/component_composition.rs
execution_mode: code_change
owned_files:
- tests/component_composition.rs
- tests/component_vocabulary.rs
- DESIGN.md
- ROADMAP.md
role: reviewer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP08 – Deterministic proof and the DESIGN/ROADMAP corrections

## ⚡ Do This First: Load Agent Profile

Use the `/ad-hoc-profile-load` skill to load the agent profile specified in the frontmatter, and behave according to its guidance before parsing the rest of this prompt.

- **Profile**: `reviewer-renata`
- **Role**: `reviewer`
- **Agent/tool**: `claude`

If no profile is specified, run `spec-kitty agent profile list` and select the best match for this work package's `task_type` and `authoritative_surface`.

---

## Markdown Formatting

Wrap HTML/XML tags in backticks: `` `<div>` ``, `` `<script>` ``
Use language identifiers in code blocks: ```rust, ```bash

---

## Objectives & Success Criteria

Deliver `tests/component_composition.rs` — the declared project completion check — extend `tests/component_vocabulary.rs` for the new page reachability rule, and land the two document corrections.

Complete when:

- `cargo test --test component_composition` exits 0 and emits `CREST_ACCEPTANCE component_composition passed`.
- `cargo test --test component_vocabulary` still passes with the reachability change.
- `DESIGN.md` names all nine non-color-signalled states.
- `ROADMAP.md`'s component-library demo bullet is amended in place to state what was delivered.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and the full suite are green.

## The bound on this work package

**C-007.** This work package proves the library exists and is used. It does **not** build a layer that verifies the verification.

This repository has already paid for that mistake: an entire session went to a mission to verify a demo, then a mission to verify that verification, then an analysis blocking on verifying the verification. If a check here starts checking another check, delete it.

## Context you need

- `.kittify/crest-spec/proof/validations.yaml` — `validation.component_composition`. Its `description` enumerates exactly what this test must prove; treat it as the checklist.
- `.kittify/crest-spec/assets.yaml` — `asset.ComponentCompositionAcceptanceTests`. Its `prompts` are the authoring instructions and its `failurePolicy` names what must fail.
- `tests/component_vocabulary.rs` — Phase 4a's acceptance target. Match its structure and its `CREST_ACCEPTANCE` marker convention.
- `src/shell/visual/controls/`, `compositions/`, `src/adapter/eframe_graphical_window.rs` — what you are proving.
- `DESIGN.md:576` and `ROADMAP.md:182` — the two corrections.

## The non-vacuity rule

Every assertion drives the **production render path** with real projected view data. A test that constructs a control and checks it is not null proves nothing. The asset's `failurePolicy` names six specific failures; each must actually fail the test if introduced. Verify that by introducing each one locally and watching it fail.

---

## Subtasks

### T040 — Scaffold the acceptance target

**Purpose**: create the file and its marker so the declared validation has something to run.

**Steps**:

1. Create `tests/component_composition.rs` with ordinary assertion-bearing tests.
2. Emit `CREST_ACCEPTANCE component_composition passed` **only after every declared check holds** — never at the top, never unconditionally.
3. Match `tests/component_vocabulary.rs`'s structure so the two read as siblings.
4. Confirm `spec-kitty crest-spec doctor` still reports the validation resolved, and that `component_composition` is in `project.yaml` `completion.projectChecks`.

**Files**: `tests/component_composition.rs`

**Validation**:
- `cargo test --test component_composition` runs and the marker appears only on success.

---

### T041 — Prove kind × role totality and control reachability

**Purpose**: FR-001, the mission's central claim.

**Steps**:

1. Drive every declared `(SemanticControlKind, PresentationRole)` pair through the production selector and assert each resolves to exactly one `ComponentControl`.
2. Assert every one of the eight controls is reachable by at least one pair — a control nothing can ask for is dead code that passes every other check.
3. Assert the mapping is **exhaustive rather than defaulted**: no pair resolves to a generic fallback. The strongest form is a source-level check that the selector contains no `_` arm, since a defaulted mapping is behaviorally indistinguishable from a real one at runtime.
4. For each pair, render through the production path with a real `SemanticControlViewModel` and assert the resulting control paints. Constructing without rendering is vacuous.
5. Assert each control renders **every state it declares applicable** with text or shape beyond color, and that a state declared non-applicable is declared rather than merely absent.

**Files**: `tests/component_composition.rs`

**Validation**:
- Removing a pair from the selector fails the test.
- Adding a control unreachable by any pair fails the test.

---

### T042 — Prove every shipped region is a composition and the adapter is empty

**Purpose**: FR-005 and FR-006.

**Steps**:

1. Render the production shell through its real path and assert **every region** in the emitted `ShellFrameObservation` was produced by a declared `ShellComposition`.
2. Assert every control within those regions was produced by a declared `ComponentControl` — not by adapter-local painting.
3. Assert the render adapter holds no paint, layout, band-height, or state-visualization decision. A source-level guard over `src/adapter/eframe_graphical_window.rs` is the practical form: no color literal, no type size, no spacing constant, no band height.
   **Then widen that same guard to every source file outside `src/shell/visual/`, which is the scope NFR-004 and SC-004 actually state.** The adapter is the loudest case, not the only one — `src/testing/component_gallery_scene.rs`, the other adapters, and every view file are inside the requirement and must be inside the guard. Walk the source tree, exclude `src/shell/visual/` and its descendants, and report every hit with its file and line. A guard scoped to one file passes while the requirement it claims to enforce is being violated two directories away.
4. **Prove the guard fails when a decision is reintroduced.** Add one locally, watch the guard fail, remove it. A guard never observed failing is not known to work.
5. Assert the adapter is at or below the declared line threshold, so the reduction cannot silently regress.

**Files**: `tests/component_composition.rs`

**Validation**:
- Reintroducing a literal into the adapter fails the test.
- Reintroducing a literal into any other file outside `src/shell/visual/` — the gallery scene is the convenient one to try — also fails the test. If only the adapter case fails, the guard is still scoped to one file and NFR-004 is unproven.
- A region painted outside a composition fails the test.

---

### T043 — Prove the no-placeholder rule and the ownership boundary

**Purpose**: C-003 and FR-009.

**Steps**:

1. Hand a composition a projection slice with **no data** for part of its designed structure. Assert it omits that structure or marks it explicitly unavailable, and paints no value that was not in the slice.
2. Assert the marker is the declared unavailable treatment, not an empty string or a zero — an unlabelled blank is indistinguishable from a real empty value.
3. Assert no control and no composition owns, caches, or derives a Patch value, focus, navigation, reducer state, or audio state.
4. Assert none dispatches a `SemanticAction`. `ControlIntent` is returned; it is not converted inside the visual module.
5. Assert no control or composition reads a raw viewport size — every size difference resolves through `ViewportDensityPolicy`.

**Files**: `tests/component_composition.rs`

**Validation**:
- Changing a composition to paint a default value fails the test.
- A control that caches a value between renders fails the test.

---

### T044 — Prove viewport integrity survived recomposition

**Purpose**: FR-010, and the check that WP06 did not quietly lose the compact viewport.

**Steps**:

1. Render the production shell at both authored viewports and assert every structural band, the persistent side region, and the 48 px minimum interactive target are retained at each.
2. Assert **no clipped or overlapping text** at either. The gallery witness asserts zero; the deterministic target should catch it first, since it runs on every build and the witness needs a window.
3. Assert both viewports resolve from `ViewportDensityPolicy` rather than a branch on size.
4. Assert a viewport between or below the two authored sizes resolves through the declared rule and introduces no third layout.

**Files**: `tests/component_composition.rs`

**Validation**:
- Hiding a band at the compact viewport fails the test.
- Text overflow at 1280×800 fails the test.

---

### T045 — Extend the vocabulary target for page reachability

**Purpose**: the reachability rule replaced "exactly one digit binding per page", and Phase 4a's target still asserts the old rule.

**Steps**:

1. In `tests/component_vocabulary.rs`, find the assertion that every declared gallery page has exactly one digit binding. It is now false by design — fifteen pages, ten digits.
2. Replace it with the reachability rule: every declared page is reachable by its digit binding **or** by stepping, and stepping alone reaches all fifteen.
3. Assert the ten digit bindings are unique.
4. **This is the one existing test file this mission may modify**, because the rule it encodes was deliberately changed in the crest-spec. Note that in the change. It is not a licence to edit other tests — NFR-005 stands.
5. Confirm `CREST_ACCEPTANCE component_vocabulary passed` still emits.

**Files**: `tests/component_vocabulary.rs`

**Validation**:
- The target passes with fifteen pages and ten digits.
- Making a page unreachable by both routes fails it.

---

### T046 — Correct `DESIGN.md` and amend `ROADMAP.md` `[P]`

**Purpose**: FR-013 and FR-014. Two documents currently say things the code contradicts.

**Steps**:

1. **`DESIGN.md:576`** reads *"Focus, mute, solo, loading, error, and selection always have text or shape in addition to color"* — six states. `ComponentState` has held nine since Phase 4a, adding resting, adjustment, and disabled. Correct the sentence to name all nine, and record the widening as a durable decision — it was an authorial choice made during the vocabulary's authoring, not a drift.
2. While there, name the control and composition families as the pieces later surfaces assemble from, in the product's own register: parameter rows, choice rows, toggles, compact sliders, faders, meters, browser rows, modal options, and the seven shell regions. Keep it terse; `DESIGN.md` states what the product should be and does not restate the declared model.
3. **`ROADMAP.md:182`** describes `make demo-live-component-library` as a scene that *"plays the real MIDI fixture"* and exercises controls through semantic actions. The delivered demo is browsable and silent by deliberate operator scope decision (C-001). **Amend the bullet in place** to state what was delivered and that MIDI was scoped out — record it as an amendment, not a deletion. A gate is closed by recorded evidence, never by quietly removing the requirement.
4. Add the Phase 4 completion note: what closes it, and what carries forward.
5. **Do not rewrite either document from the crest-spec.** Both are hand-authored authorities; edits are deliberate authorial acts.

**Files**: `DESIGN.md`, `ROADMAP.md`

**Validation**:
- `DESIGN.md` names nine states and no longer contradicts `src/shell/visual/state.rs:27`.
- `ROADMAP.md` states what Phase 4 delivered, with the MIDI de-scope recorded rather than erased.
- Neither document was regenerated.

---

## Branch Strategy

- **Planning base branch**: `feat/crest-component-controls-and-compositions`
- **Final merge target**: `feat/crest-component-controls-and-compositions`, and from there to `main`
- Execution worktrees are allocated per computed lane from `lanes.json`.

## Definition of Done

- All seven subtasks complete; `mark-status` recorded.
- `component_composition` passes and emits its marker only on success.
- Each of the asset's six named failures verified to actually fail the test.
- `component_vocabulary` passes with the reachability rule.
- Both documents corrected without regeneration.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, full suite green.
- No file outside `owned_files` modified.

## Risks

- **Vacuous assertions.** A test that constructs and checks non-null passes forever and proves nothing. Every assertion drives the production render path with real view data.
- **A marker emitted unconditionally.** Then the validation is a no-op that reports success. It emits only after every check holds.
- **Never watching the guards fail.** A guard not observed failing is not known to work. Introduce each of the six named failures locally and confirm.
- **Scope creep into meta-proof (C-007).** If you find yourself writing a check that verifies another check, stop and delete it.
- **Treating T045 as permission to edit other tests.** It is not. That one file's rule changed in the crest-spec; NFR-005 protects every other.

## Reviewer Guidance

1. Read every assertion in `component_composition.rs`. For each, ask: what change to the source would make this fail? If the answer is "none", it is vacuous — reject.
2. Confirm the `CREST_ACCEPTANCE` marker is emitted only after the checks, not at the top.
3. Introduce a literal into the adapter and run the test. Does it fail? If not, T042's guard is decorative.
4. Change a composition to paint a default value and run. Does T043 fail?
5. `git diff --name-only` over `tests/`. Only `component_composition.rs` (new) and `component_vocabulary.rs` (the deliberate reachability change) may appear.
6. Read the `ROADMAP.md` amendment. Does it state what was delivered and record the de-scope, or does it just delete the inconvenient bullet? Deletion is a reject.
