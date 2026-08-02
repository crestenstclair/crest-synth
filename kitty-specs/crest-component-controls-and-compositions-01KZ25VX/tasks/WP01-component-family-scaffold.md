---
work_package_id: WP01
title: Component family scaffold and the total selector
dependencies: []
requirement_refs:
- FR-001
- FR-009
planning_base_branch: feat/crest-component-controls-and-compositions
merge_target_branch: feat/crest-component-controls-and-compositions
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-controls-and-compositions. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-controls-and-compositions unless the human explicitly redirects the landing branch.
subtasks:
- T001
- T002
- T003
- T004
- T005
- T006
- T007
phase: Phase 1 - Foundation
history:
- at: '2026-08-02T21:46:28Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent_profile: architect-alphonso
authoritative_surface: src/shell/visual/controls/
create_intent:
- src/shell/visual/controls/mod.rs
- src/shell/visual/compositions/mod.rs
execution_mode: code_change
owned_files:
- src/shell/visual/controls/mod.rs
- src/shell/visual/compositions/mod.rs
- src/shell/visual/mod.rs
role: architect
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP01 – Component family scaffold and the total selector

## ⚡ Do This First: Load Agent Profile

Use the `/ad-hoc-profile-load` skill to load the agent profile specified in the frontmatter, and behave according to its guidance before parsing the rest of this prompt.

- **Profile**: `architect-alphonso`
- **Role**: `architect`
- **Agent/tool**: `claude`

If no profile is specified, run `spec-kitty agent profile list` and select the best match for this work package's `task_type` and `authoritative_surface`.

---

## Markdown Formatting

Wrap HTML/XML tags in backticks: `` `<div>` ``, `` `<script>` ``
Use language identifiers in code blocks: ```rust, ```bash

---

## Objectives & Success Criteria

Establish the two closed component families and the selector every later work package plugs into. **This work package writes no control body and no composition body** — it writes the contract they satisfy.

Complete when:

- `PresentationRole` and `ComponentControl` are closed unions with exhaustiveness assertions.
- Selection over `(SemanticControlKind, PresentationRole)` is **total** and written so an added kind or role is a **compile error**, not a runtime assertion failure.
- Every one of the eight `ComponentControl` variants is reachable by at least one pair.
- `ShellComposition` is a closed seven-variant union bound to its shell regions.
- Both module trees are wired into `src/shell/visual/mod.rs`, and the crate builds with stub bodies.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and the full suite are green.

## Why this lands alone and first

The presentation-role vocabulary is the load-bearing decision of the whole mission (`research.md` R-01). Fifteen files depend on it. Getting it wrong after the controls are built means rework everywhere.

It also removes all module contention: **this WP owns the three module roots**, so WP02 through WP05 own only leaf files and can never collide on a `mod.rs`. Do not leave a `mod.rs` for a later work package to append to.

## Context you need

Read before starting:

- `.kittify/crest-spec/contexts/shell.yaml` — `valueObject.Shell.ComponentControl` and `valueObject.Shell.ShellComposition` are the authoritative declarations. Their invariants are the acceptance criteria, not suggestions.
- `research.md` R-01 — the four-role vocabulary and why it was chosen over the alternatives.
- `src/shell/visual/state.rs` — `ComponentState`, the closed nine-value union you will declare applicability against. Note how `ALL_COMPONENT_STATES` and the exhaustive match are written; follow that shape.
- `src/shell/visual/primitives/mod.rs` — Phase 4a's primitive family. Your families should read like siblings of it, not like a different codebase.
- `src/control/semantic_graphical_view_model.rs:22` — `SemanticControlKind`, the seven-value union you are selecting over. **Do not modify it** (C-002).

---

## Subtasks

### T001 — Declare the closed `PresentationRole` union

**Purpose**: name the four roles a composition can ask a control in, so kind alone does not have to select a shape.

**Steps**:

1. In `src/shell/visual/controls/mod.rs`, declare:

```rust
/// The role a requesting composition asks a control in. The same control kind
/// is a parameter row on a listed surface and a fader in a mixer strip, so kind
/// alone cannot select a shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PresentationRole {
    /// PATCH main surface, MIXER inspector, Utility panel — stacked labelled rows.
    ListedRow,
    /// MIXER track columns — the sixteen fixed compact columns.
    VerticalStrip,
    /// Utility/Inspector panel entries that are not full rows.
    PanelEntry,
    /// Focus-trapped option modals and the later Sample Browser.
    ModalEntry,
}
```

2. Add `ALL_PRESENTATION_ROLES` as a `const` slice, mirroring `ALL_COMPONENT_STATES` in `state.rs`.
3. Add the declared-count assertion: the slice length equals the variant count, asserted so an added variant that is not added to the slice fails.

**Files**: `src/shell/visual/controls/mod.rs`

**Validation**:
- Adding a fifth variant without adding it to the slice fails the assertion.
- The role is supplied by callers; nothing in this module derives one.

---

### T002 — Declare the closed `ComponentControl` union with state applicability

**Purpose**: name the eight control shapes and declare, per shape, which of the nine `ComponentState` values it can be handed.

**Steps**:

1. Declare the eight-variant union in `src/shell/visual/controls/mod.rs`: `ParameterRow`, `ChoiceRow`, `Toggle`, `CompactSlider`, `Fader`, `Meter`, `BrowserRow`, `ModalOption`. Add `ALL_COMPONENT_CONTROLS` with the same declared-count assertion.

2. Declare applicability as a method with an exhaustive match:

```rust
impl ComponentControl {
    /// The states this control can be handed. A control that can never be muted
    /// or soloed declares that here rather than silently omitting a specimen.
    pub const fn applicable_states(self) -> &'static [ComponentState] { … }
}
```

3. Suggested applicability, subject to the Figma specimens the later WPs read — **record any change you make and why**:
   - `Fader` and `Meter`: all nine, including `Muted` and `Soloed` (they live in mixer strips).
   - `ParameterRow`, `ChoiceRow`, `Toggle`, `CompactSlider`, `BrowserRow`, `ModalOption`: the seven excluding `Muted` and `Soloed` — mute and solo are mixer-track concepts and these controls are never handed them.

4. Every control must declare at least `Resting`, `Focused`, and `Disabled`. Assert that.

**Files**: `src/shell/visual/controls/mod.rs`

**Validation**:
- Every declared state on every control appears in `ALL_COMPONENT_STATES`.
- The union of all declared applicable states covers all nine — no state is applicable to nothing.
- Adding a `ComponentControl` variant fails compilation at `applicable_states`.

---

### T003 — Write the failing totality assertion first

**Purpose**: test-first (DIRECTIVE_034). The assertion exists and fails before the selector is written.

**Steps**:

1. In `src/shell/visual/controls/mod.rs`, add a `#[cfg(test)]` module asserting:
   - **Totality**: for every `(SemanticControlKind, PresentationRole)` pair the model declares valid, selection returns a `ComponentControl`.
   - **Reachability**: every one of the eight `ComponentControl` variants is returned by at least one pair. This catches a control that exists but nothing can ask for.
2. Run it and confirm it fails for the right reason (no selector yet), not a compile error in the test itself.

**Files**: `src/shell/visual/controls/mod.rs`

**Validation**:
- The test fails before T004 and passes after, with no change to the test.

---

### T004 — Implement the total kind × role selector

**Purpose**: make selection a compile-time-checked total function.

**Steps**:

1. Write it as a **match on a tuple**, not nested matches with a catch-all:

```rust
pub const fn control_for(
    kind: SemanticControlKind,
    role: PresentationRole,
) -> ControlSelection { … }
```

Rust checks tuple-match exhaustiveness, so an added kind or role becomes a compile error. Nested matches with a `_ =>` arm do not, and would silently swallow exactly the drift C-004 exists to catch.

2. **No `_ =>` arm anywhere in this function.** If a pair is genuinely not askable, return an explicit typed "not askable in this role" value — do not fall through to a generic row. That distinction is the whole point of FR-001.

3. Suggested mapping, subject to what the Figma specimens show — record deviations:

| Kind | ListedRow | VerticalStrip | PanelEntry | ModalEntry |
|---|---|---|---|---|
| `Continuous` | ParameterRow | Fader | CompactSlider | ModalOption |
| `Stepped` | ParameterRow | Fader | CompactSlider | ModalOption |
| `Choice` | ChoiceRow | *not askable* | ChoiceRow | ModalOption |
| `Toggle` | Toggle | Toggle | Toggle | ModalOption |
| `Asset` | BrowserRow | *not askable* | BrowserRow | BrowserRow |
| `Identity` | ParameterRow | Meter | ParameterRow | ModalOption |
| `Surface` | ParameterRow | *not askable* | ParameterRow | ModalOption |

Check reachability against this table before committing to it: every one of the eight controls must appear at least once.

**Files**: `src/shell/visual/controls/mod.rs`

**Validation**:
- T003's assertion passes.
- Adding a `PresentationRole` variant produces a compile error naming this function.
- `grep` finds no `_ =>` arm in the selector.

---

### T005 — Define the control call signature and `ControlIntent`

**Purpose**: fix the shape every control implements, so eight parallel work streams produce compatible functions.

**Steps**:

1. Declare the signature every control satisfies:

```rust
pub fn render(
    ui: &mut egui::Ui,
    view: &SemanticControlViewModel,
    state: ComponentState,
    role: PresentationRole,
    density: &ViewportDensityPolicy,
) -> ControlIntent;
```

Pass the density policy explicitly rather than letting a control read a viewport size — `ViewportDensityPolicy`'s invariant requires it.

2. Declare `ControlIntent` as a closed union of what a control can *ask for*, never what it does. It must carry no `SemanticAction` and must not be convertible into one inside this module — mapping intent to action is the caller's job, and C-002 forbids adding action variants here.

3. Document on the trait or module: a control paints, reads only what it is handed, and returns intent. It owns nothing, caches nothing, dispatches nothing, and never reaches `AppState`.

4. Add a stub body per control variant returning "no intent", so the crate compiles before WP02 and WP03 fill them in.

**Files**: `src/shell/visual/controls/mod.rs`

**Validation**:
- The crate compiles with stubs.
- `ControlIntent` has no path to `SemanticAction` from this module.
- No control signature takes `&AppState`, a viewport size, or a mutable projection.

---

### T006 — Declare the closed `ShellComposition` union

**Purpose**: name the seven regions and bind each to the shell region it fills.

**Steps**:

1. In `src/shell/visual/compositions/mod.rs`, declare the seven variants: `ApplicationShell`, `ContextSwitch`, `IdentityHeader`, `Section`, `PatchStripRow`, `UtilityInspectorPanel`, `Footer`. Add `ALL_SHELL_COMPOSITIONS` with the declared-count assertion.
2. Bind each to its region, matching the region names already in `ShellFrameObservation` (`contextLine`, `identityHeader`, `mainWorkspace`, `persistentSideRegion`, `footer`) — WP06 depends on that correspondence surviving.
3. Declare the composition call signature: it takes `&mut egui::Ui`, an immutable projection slice, and the density policy, and returns typed intent aggregated from the controls it arranges.
4. Add stub bodies so the crate compiles.

**Files**: `src/shell/visual/compositions/mod.rs`

**Validation**:
- Adding a variant fails compilation at the region binding.
- Every `ShellFrameObservation` region name maps to at least one composition.
- No composition signature takes `&AppState` or a mutable projection.

---

### T007 — Wire both module trees into `src/shell/visual/mod.rs`

**Purpose**: make the families reachable and pin the module boundary NFR-004 is measured against.

**Steps**:

1. Add `pub mod controls;` and `pub mod compositions;` to `src/shell/visual/mod.rs`, and re-export the family types beside the existing primitive re-exports.
2. Keep the re-export surface **narrow**: the families, the roles, the states, the intents. Do not re-export a control's internal helpers — a wide surface is how a literal escapes the module.
3. Run the Phase 4a literal guard and confirm it still passes with the new files present.

**Files**: `src/shell/visual/mod.rs`

**Validation**:
- `cargo build` and the full suite are green.
- `cargo clippy --all-targets -- -D warnings` is clean.
- The literal guard passes.
- No file outside `src/shell/visual/` changed.

---

## Branch Strategy

- **Planning base branch**: `feat/crest-component-controls-and-compositions`
- **Final merge target**: `feat/crest-component-controls-and-compositions`, and from there to `main`
- Execution worktrees are allocated per computed lane from `lanes.json`; enter the workspace the lane resolves to rather than assuming a path.

## Definition of Done

- All seven subtasks complete; `mark-status` recorded for each.
- Both families closed, exhaustive, and compile-checked.
- Selection total over kind × role with no `_ =>` arm; every control reachable.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, full suite green.
- No file outside `owned_files` modified.
- Any deviation from the suggested applicability or selector tables is recorded with its reason.

## Risks

- **Nested matches instead of a tuple match.** The single most likely mistake. It compiles, it passes the tests, and it silently defeats C-004. Review for it explicitly.
- **A too-fine role vocabulary.** If a composition in WP04/WP05 needs a fifth role, that is a signal to re-examine the four — not to add roles per surface.
- **Stub bodies that do something.** Stubs must return no intent and paint nothing. A stub that paints a placeholder row will be forgotten and ship.

## Reviewer Guidance

Verify by inspection, not by trusting the tests:

1. Open the selector. Is it one match on a tuple? Is there any `_` arm? Any `if let`?
2. Delete a `PresentationRole` variant locally and build. Does the compiler name the selector? Restore.
3. Does any control or composition signature accept `&AppState`, a mutable projection, or a raw viewport size? Any of those is a reject.
4. Is every one of the eight controls returned by at least one pair? A control nothing can ask for is dead code that will pass every other check.
5. Does `ControlIntent` have any path to `SemanticAction` from this module? It must not.
