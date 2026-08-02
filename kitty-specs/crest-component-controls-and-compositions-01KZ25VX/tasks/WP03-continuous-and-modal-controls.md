---
work_package_id: WP03
title: Continuous and modal controls
dependencies:
- WP01
requirement_refs:
- FR-002
- FR-003
- FR-011
planning_base_branch: feat/crest-component-controls-and-compositions
merge_target_branch: feat/crest-component-controls-and-compositions
branch_strategy: Planning artifacts for this mission were generated on feat/crest-component-controls-and-compositions. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/crest-component-controls-and-compositions unless the human explicitly redirects the landing branch.
subtasks:
- T013
- T014
- T015
- T016
- T017
phase: Phase 2 - Controls
history:
- at: '2026-08-02T21:46:28Z'
  actor: system
  action: Prompt generated via /spec-kitty.tasks
agent_profile: designer-dagmar
authoritative_surface: src/shell/visual/controls/
create_intent:
- src/shell/visual/controls/compact_slider.rs
- src/shell/visual/controls/fader.rs
- src/shell/visual/controls/meter.rs
- src/shell/visual/controls/modal_option.rs
execution_mode: code_change
owned_files:
- src/shell/visual/controls/compact_slider.rs
- src/shell/visual/controls/fader.rs
- src/shell/visual/controls/meter.rs
- src/shell/visual/controls/modal_option.rs
role: designer
tags: []
task_type: implement
tracker_refs: []
---

# Work Package Prompt: WP03 – Continuous and modal controls

## ⚡ Do This First: Load Agent Profile

Use the `/ad-hoc-profile-load` skill to load the agent profile specified in the frontmatter, and behave according to its guidance before parsing the rest of this prompt.

- **Profile**: `designer-dagmar`
- **Role**: `designer`
- **Agent/tool**: `claude`

If no profile is specified, run `spec-kitty agent profile list` and select the best match for this work package's `task_type` and `authoritative_surface`.

---

## Markdown Formatting

Wrap HTML/XML tags in backticks: `` `<div>` ``, `` `<script>` ``
Use language identifiers in code blocks: ```rust, ```bash

---

## Objectives & Success Criteria

Build the four painting-heavy controls: compact slider, fader, meter, modal option. These carry continuous geometry rather than text layout, so they are the ones where "paint, do not use an egui widget" costs the most and matters the most.

Complete when:

- All four render from their Figma specimens in every applicable state at both viewports.
- Each carries text or shape in addition to color for every state.
- The meter renders whatever level the view data reports — including resting — and reads nothing from the audio boundary.
- Zero literals; everything through `SemanticVisualToken` and `ViewportDensityPolicy`.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and the full suite are green.

## Context you need

- `.kittify/crest-spec/contexts/shell.yaml` — `valueObject.Shell.ComponentControl`, including the invariant that `Meter` presents what the view data reports and invents no signal.
- `src/shell/visual/controls/mod.rs` (WP01) — signature, roles, applicability, `ControlIntent`.
- `src/shell/visual/primitives/` — compose these; do not redraw them.
- `research.md` R-02 — **paint through `Painter`; use egui for layout, input, and `Response` only.** No `egui::Slider`, no `egui::ProgressBar`. A styled egui widget puts a visual literal inside `egui::Style`, where NFR-004's guard cannot see it.
- `DESIGN.md:462` — the sixteen mixer track columns are compact, hairline-separated, and all visible at 1920×1080. That constrains fader and meter width hard.
- `DESIGN.md:458` — option modals trap focus until choose or cancel.
- `DESIGN.md:420` — meters observe post-level/pan, **pre** mute/solo gate, so a muted track stays diagnosable. That is why `Meter` declares `Muted` and `Soloed` applicable.

## The Figma rule

Geometry, spacing, and state treatment come from the Figma file linked in `DESIGN.md`.

**If a specimen is missing or ambiguous, raise it. Do not approximate.** Record the source frame for each control.

## The audio rule

C-001: no MIDI, no audio, anywhere in this slice. The meter is the one control that would normally have a live signal. It does not here — and it must not acquire one to look convincing. It renders the level in the view data, whatever that is.

---

## Subtasks

### T013 — Build the compact slider `[P]`

**Purpose**: a continuous control for panel entries where a full parameter row does not fit.

**Steps**:

1. Create `src/shell/visual/controls/compact_slider.rs`.
2. Layout from the Figma specimen. Track, fill, and position indicator geometry, plus radius and keyline widths, all from the token vocabulary.
3. Drive fill and indicator position from `SemanticNumericRange` and the current value. **If the range is absent, render the track with the value explicitly unpositioned** — do not assume 0..1. An invented range is an invented value (C-003).
4. Minimum interactive target is 48 px (`DESIGN.md:572`) and the density policy carries it. A compact slider is compact in one axis only; it does not shrink below the target in the axis it is grabbed on.
5. All applicable states with non-color evidence, per the vocabulary Phase 4a declared.
6. Return `ControlIntent` describing the adjustment asked for. It does not clamp, and it does not decide whether the edit is legal.

**Files**: `src/shell/visual/controls/compact_slider.rs` (~200 lines)

**Validation**:
- With no range, nothing is positioned and nothing is invented.
- The interactive target is never below 48 px.
- No `egui::Slider`.

---

### T014 — Build the fader `[P]`

**Purpose**: the vertical strip control for the sixteen mixer tracks.

**Steps**:

1. Create `src/shell/visual/controls/fader.rs`.
2. This is a `VerticalStrip` control. Sixteen must fit at 1920×1080 (`DESIGN.md:462`) — take the column width from the density policy, and verify sixteen at the desktop width before considering it done.
3. Layout from Figma: track, cap, scale marks if shown, and the label. Vertical orientation.
4. Drive position from the value and its range, same absent-range rule as T013.
5. **`Muted` and `Soloed` are applicable here.** Both are mixer-track facts and both need non-color evidence — a muted fader and a soloed fader must be distinguishable from each other and from resting without color. Figma decides the marks; if it does not say, raise it.
6. A track can be both muted and soloed. `ComponentState` is a single value, so the composition decides which state it hands down — but check the Figma specimen for whether both must be visible at once. If they must, that is a finding to raise, not something to solve by adding a state.
7. All applicable states with non-color evidence.

**Files**: `src/shell/visual/controls/fader.rs` (~210 lines)

**Validation**:
- Sixteen faders fit at 1920×1080 with hairline separators.
- Muted, soloed, and resting are distinguishable without color.
- No `egui::Slider`.

---

### T015 — Build the meter `[P]`

**Purpose**: the level display beside each mixer track.

**Steps**:

1. Create `src/shell/visual/controls/meter.rs`.
2. Layout from Figma: segment or continuous bar, scale, peak indication if shown.
3. **Render the level the view data reports.** Do not read `AudioObservation`, do not read any atomic, do not construct anything from the audio boundary. The control receives a value; it paints it.
4. **A resting meter is a correct meter in this slice.** With audio out of scope, the reported level will typically be resting. That is not a bug and must not be worked around.
5. `Muted` and `Soloed` are applicable. `DESIGN.md:420` — meters observe pre-gate so a muted track stays diagnosable, which means a muted meter still shows level. Its muted state is signalled separately from its level, not by zeroing it.
6. Non-color evidence for every applicable state.
7. The meter is presentational. It returns no intent beyond "none" unless the Figma specimen shows an interactive element.

**Files**: `src/shell/visual/controls/meter.rs` (~190 lines)

**Validation**:
- `grep` the file for `AudioObservation`, `atomic`, `Meter`-snapshot types — no hit.
- A muted meter still displays its level and signals muted separately.
- No `egui::ProgressBar`.

---

### T016 — Build the modal option `[P]`

**Purpose**: one selectable entry inside a focus-trapped option modal.

**Steps**:

1. Create `src/shell/visual/controls/modal_option.rs`.
2. Layout from Figma. This is the row type in a nested modal (`DESIGN.md:458`), so it reads denser and more selection-oriented than a parameter row.
3. `Selected` is the state that matters most here — the currently chosen option among installed choices. Its non-color evidence must be unmistakable, since a modal is a list of near-identical rows.
4. **The modal option does not trap focus and does not manage the modal.** Focus trapping and return path are reducer and composition concerns. This control paints one entry and returns intent.
5. All applicable states with non-color evidence.

**Files**: `src/shell/visual/controls/modal_option.rs` (~180 lines)

**Validation**:
- `Selected` is distinguishable from `Focused` without color — in a modal these two co-occur and must not look the same.
- The control contains no focus-trap logic and no modal lifecycle.

---

### T017 — Assert every applicable state renders non-color evidence

**Purpose**: make FR-003 checkable for these four.

**Steps**:

1. Add `#[cfg(test)]` assertions inside each of the four files — not a shared file, which would collide with WP02.
2. Per control, per applicable state: render and assert non-color evidence is present.
3. Assert each control paints no value absent from its `SemanticControlViewModel`.
4. **Assert the meter specifically**: it constructs nothing audio-related and its output is a pure function of the view data handed in. Render it twice with the same input and assert identical output.
5. Assert no control constructs a `SemanticAction`.

**Files**: all four control files

**Validation**:
- Removing a state's non-color treatment fails the assertion.
- The meter's twice-rendered output is identical.

---

## Branch Strategy

- **Planning base branch**: `feat/crest-component-controls-and-compositions`
- **Final merge target**: `feat/crest-component-controls-and-compositions`, and from there to `main`
- Execution worktrees are allocated per computed lane from `lanes.json`.

## Definition of Done

- All five subtasks complete; `mark-status` recorded.
- Four controls from Figma specimens, source frame recorded for each.
- Sixteen faders verified to fit at 1920×1080.
- Meter proven free of any audio dependency.
- Zero literals; the guard passes.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, full suite green.
- No file outside `owned_files` modified.

## Risks

- **The meter acquiring an audio dependency.** It is the most natural mistake in the mission — a meter with nothing to show feels broken, and wiring it to a real signal would fix that feeling and violate C-001. A resting meter is correct here.
- **Reaching for `egui::Slider` or `ProgressBar`.** Rejected in `research.md` R-02 on NFR-004 grounds.
- **Sixteen faders not fitting.** Discover this at T014, not at WP06 when the mixer workspace is recomposed.
- **Approximating a missing Figma specimen.** Raise it.

## Reviewer Guidance

1. `grep` all four files for hex literals, numeric font sizes, bare pixel constants — any hit is a reject.
2. `grep` for `egui::Slider`, `ProgressBar`, `DragValue` — any hit is a reject.
3. `grep` the meter for anything audio-adjacent. It must be inert.
4. Render sixteen faders at 1920×1080. Do they fit with separators?
5. In the modal option, are `Selected` and `Focused` distinguishable without color? They co-occur.
6. With an absent numeric range, does the slider or fader invent a position? It must not.
