---
work_package_id: WP07
title: Gallery pages, stepping, and measured silence
dependencies:
- WP02
- WP03
- WP04
- WP05
- WP09
requirement_refs:
- FR-007
- FR-008
- FR-012
planning_base_branch: feat/crest-component-controls-and-compositions
merge_target_branch: feat/crest-component-controls-and-compositions
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-controls-and-compositions. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-controls-and-compositions unless the human explicitly redirects the landing branch.
subtasks:
- T034
- T035
- T036
- T037
- T038
- T039
phase: Phase 5 - Gallery
history:
- at: '2026-08-02T21:46:28Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent: claude
agent_profile: implementer-ivan
authoritative_surface: src/testing/
create_intent: []
execution_mode: code_change
owned_files:
- src/testing/component_gallery_scene.rs
- src/shell/window_input.rs
role: implementer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP07 – Gallery pages, stepping, and measured silence

## ⚡ Do This First: Load Agent Profile

Use the `/ad-hoc-profile-load` skill to load the agent profile specified in the frontmatter, and behave according to its guidance before parsing the rest of this prompt.

- **Profile**: `implementer-ivan`
- **Role**: `implementer`
- **Agent/tool**: `claude`

If no profile is specified, run `spec-kitty agent profile list` and select the best match for this work package's `task_type` and `authoritative_surface`.

---

## Markdown Formatting

Wrap HTML/XML tags in backticks: `` `<div>` ``, `` `<script>` ``
Use language identifiers in code blocks: ```rust, ```bash

---

## Objectives & Success Criteria

Grow the gallery from eight pages to fifteen so every control and composition is visible, make all fifteen reachable across ten digits, and make the scene's silence measured rather than claimed.

Complete when:

- `WindowInput` carries `Digit9`, `Digit0`, `BracketLeft`, `BracketRight`; the descriptor count is **41**.
- Seven new pages exist; **the eight pre-existing pages keep their exact digit bindings**.
- Bidirectional non-wrapping stepping reaches all fifteen.
- The observation carries control, composition, and audio/MIDI-construction coverage.
- `make demo-live-component-library` opens, pages, and closes cleanly, sounding nothing.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and the full suite are green.

## Context you need

- `.kittify/crest-spec/contexts/shell.yaml` — `ComponentGalleryPage` (fifteen variants, the reachability and pinned-binding invariants), `ComponentGalleryObservation` (the new fields), `WindowInput` (twenty keys, 41 descriptors).
- `.kittify/crest-spec/proof/witnesses.yaml` — `witness.component_gallery`. Its predicates are what this scene must satisfy: 15 pages declared and painted, 10 digit-reachable, 15 step-reachable, 9 states, 8 controls, 7 compositions, `audio_or_midi_constructed == false`, `app_state_generation_delta == 0`.
- `src/testing/component_gallery_scene.rs` — the existing scene. `ComponentGalleryPage:107`, the page list at `:128`, digit bindings at `:146`, `PageSelection:240`, key mapping at `:1287`.
- `src/shell/window_input.rs` — the normalized key vocabulary and its asserted descriptor count.
- `research.md` R-04 — the reachability decision and why renumbering was rejected.

## Two rules

1. **FR-012 pins the existing bindings.** An operator who knows `Digit4` is `InteractionStates` must keep finding it there. New pages append; nothing renumbers.
2. **C-001: no audio, no MIDI.** The scene must construct neither, and T039 makes that falsifiable rather than a claim in a comment.

---

## Subtasks

### T034 — Extend `WindowInput` by four keys and update the descriptor count

**Purpose**: normalize the keys the gallery needs, without giving them application meaning.

**Steps**:

1. In `src/shell/window_input.rs`, add `Digit9`, `Digit0`, `BracketLeft`, `BracketRight` to the normalized key set. Twenty keys total.
2. Update the descriptor construction and the **asserted count from 33 to 41** (20 keys × KeyDown/KeyUp + FocusLost). The crest-spec invariant requires the declared count and the constructed descriptor to be asserted equal, so these must change together or the build breaks — which is the point.
3. **None of the four carries an application binding.** The existing invariant covers `Digit3`–`Digit8`; these extend it. An unbound key reaching the translator produces **no** `SemanticAction`, not a substitute one.
4. Do not touch `KeyboardInputTranslator` beyond what normalization requires. `Digit1` and `Digit2` keep their `SelectContext` meaning; the new keys map to nothing.
5. Confirm the Phase 4a name-enumeration guard still passes.

**Files**: `src/shell/window_input.rs`

**Validation**:
- Descriptor count asserts 41 and passes.
- Each new key produces no `SemanticAction` through the translator.
- Adding a key without updating the count fails the assertion.

---

### T035 — Add the seven control and composition pages

**Purpose**: make every control and composition visible.

**Steps**:

1. Extend `ComponentGalleryPage` with seven variants, **appended after the existing eight in declared order**:
   - `ParameterAndChoiceRows` — parameter row, choice row
   - `TogglesAndSliders` — toggle, compact slider
   - `FadersAndMeters` — fader, meter
   - `BrowserAndModalOptions` — browser row, modal option
   - `ShellAndContextSwitch` — application shell, context switch
   - `HeadersAndSections` — identity header, section
   - `StripPanelAndFooter` — Patch strip row, Utility/Inspector panel, footer
2. Each control page renders its controls **in every state that control declares applicable** — the applicability declaration from WP01 is what the page iterates, so a control that declares nine states shows nine and one that declares seven shows seven.
3. Each composition page renders its compositions **with representative content**. This is where representative content belongs; the production shell is where it does not (C-003).
4. Every page renders at both the desktop and Steam Deck viewports.
5. Each page shows its own identity on screen, as the existing eight do.
6. Extend the coverage assertion: every `ComponentControl` and every `ShellComposition` must appear on some page, so an added variant with no specimen fails the build.

**Files**: `src/testing/component_gallery_scene.rs`

**Validation**:
- 15 pages declared; the first eight byte-identical in identity and order.
- Every control and composition has a specimen; removing one fails the assertion.
- Both viewports render on every page.

---

### T036 — Add bidirectional non-wrapping page stepping

**Purpose**: reach the five pages past the ten digits.

**Steps**:

1. Bind `BracketLeft` to previous page and `BracketRight` to next, over the full fifteen-page declared order.
2. **Non-wrapping**: at the first page, previous retains the first; at the last, next retains the last. This matches the nonwrapping movement the product uses everywhere else (`DESIGN.md:309`).
3. Stepping is scene-local, exactly like digit selection — it never becomes a `SemanticAction`, never reaches `AppState`, and changes no focus, Patch value, graph revision, or audio behavior.
4. Add the reachability assertion: every declared page is reachable by digit **or** by stepping, and stepping alone reaches all fifteen.

**Files**: `src/testing/component_gallery_scene.rs`

**Validation**:
- Stepping from page 1 to page 15 and back visits all fifteen.
- Stepping past either end retains the end page.
- `app_state_generation_delta` stays 0 across a full traversal.

---

### T037 — Pin the eight pre-existing digit bindings

**Purpose**: make FR-012 a regression gate rather than a promise.

**Steps**:

1. Add a frozen baseline in the scene — the eight `(page identity, digit)` pairs as they exist today — and assert the current bindings match it exactly and in order.
2. Model it on `FROZEN_TOPOLOGY_IDENTITY_BASELINE` (`tests/effects_and_buses.rs:59`), which the project already uses for exactly this kind of add-only contract.
3. Assign `Digit9` → `ParameterAndChoiceRows` and `Digit0` → `TogglesAndSliders`, the ninth and tenth in declared order.
4. Assert `pages_reachable_by_digit == 10` and that the ten bindings are unique — no digit bound twice.
5. Assert an unbound digit retains the current page (the existing behavior, now with more digits bound).

**Files**: `src/testing/component_gallery_scene.rs`

**Validation**:
- Reordering the page list fails the frozen-baseline assertion.
- All ten digit bindings are unique.

---

### T038 — Emit control, composition, and silence coverage

**Purpose**: make the witness predicates satisfiable from measured output.

**Steps**:

1. Extend `ComponentGalleryObservation` emission with the declared fields: `controlsRendered`, `compositionsRendered`, `audioOrMidiConstructed`, and the counts the witness predicates read — `pages_declared`, `pages_painted`, `pages_reachable_by_digit`, `pages_reachable_by_step`, `controls_declared`, `controls_painted`, `kind_role_pairs_unmapped`, `controls_unreachable_by_any_pair`, `compositions_declared`, `compositions_painted`.
2. **Measure, do not declare.** Every count comes from what was actually painted, with visible-label evidence — the crest-spec invariant states that a pre-render plan or a constructed specimen list must not be able to satisfy the observation. Counting a list you built before rendering is exactly the vacuity that invariant forbids.
3. Emit under the existing `CREST_COMPONENT_GALLERY_OBSERVATION ` marker, keeping every existing field.
4. Keep the scene's deliberate non-claim: it is browsable and makes **no** exact-generation claim, so it does not acquire the autonomous witness contract and does not weaken it in the scenes that hold it.

**Files**: `src/testing/component_gallery_scene.rs`

**Validation**:
- Every witness predicate in `.kittify/crest-spec/proof/witnesses.yaml` has a field to read.
- Removing a specimen changes the painted count — proving it is measured, not declared.

---

### T039 — Assert the gallery constructs no audio output and no MIDI source

**Purpose**: NFR-006 and C-001, made falsifiable.

**Steps**:

1. Determine `audioOrMidiConstructed` from what the scene actually constructed, not from a hard-coded `false`. A literal `false` satisfies the predicate and proves nothing.
2. Add an assertion that the gallery scene's construction path touches no audio output port and no MIDI event source. Options in order of strength: a construction counter the scene owns; a test that builds the scene with a panicking audio/MIDI factory injected; a static check that the module imports neither.
3. Assert the scene opens no stream and dispatches no note event across a full page traversal.
4. Run `make demo-live-component-library` and confirm silence by listening, not only by reading the flag.

**Files**: `src/testing/component_gallery_scene.rs`

**Validation**:
- `audio_or_midi_constructed` is derived, not literal — introducing an audio construction flips it to `true`.
- The scene is audibly silent on a real run.

---

## Branch Strategy

- **Planning base branch**: `feat/crest-component-controls-and-compositions`
- **Final merge target**: `feat/crest-component-controls-and-compositions`, and from there to `main`
- Execution worktrees are allocated per computed lane from `lanes.json`.

## Definition of Done

- All six subtasks complete; `mark-status` recorded.
- 15 pages; the original eight bindings frozen and asserted.
- Stepping reaches all fifteen, non-wrapping.
- Every witness predicate has a measured field behind it.
- Silence derived and confirmed by a real run.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, full suite green.
- No file outside `owned_files` modified.

## Risks

- **Moving an existing digit binding.** FR-012 exists because an operator feels this immediately. T037's frozen baseline is the guard; do not weaken it to make a reordering pass.
- **A declared observation instead of a measured one.** Counting a pre-built specimen list is the exact vacuity the crest-spec invariant names. If removing a specimen does not change the count, the observation is worthless.
- **A hard-coded silence flag.** Same failure, smaller. It must be derived.
- **The descriptor count drifting from the key list.** They are asserted equal, so they must change together.

## Reviewer Guidance

1. Diff the first eight page identities and digit bindings against the pre-change file. Any difference is a reject.
2. Delete a specimen locally and re-run. Does `controls_painted` drop? If not, the observation is declared, not measured — reject.
3. `grep` the scene for a literal `false` assigned to the silence field. Reject.
4. `grep` the scene module for audio or MIDI imports. Any is a reject.
5. Confirm the descriptor count assertion says 41 and passes.
6. Run `make demo-live-component-library`. Press every digit, both brackets. All fifteen pages? Silent? Clean exit?
