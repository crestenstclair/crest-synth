---
work_package_id: WP07
title: Sole-shell flip, egui deletion, records
dependencies:
- WP04
- WP05
- WP06
requirement_refs:
- FR-002
- FR-006
- FR-007
- NFR-004
planning_base_branch: feat/webview-shell-cutover
merge_target_branch: feat/webview-shell-cutover
branch_strategy: Planning artifacts for this mission were generated on feat/webview-shell-cutover. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into feat/webview-shell-cutover unless the human explicitly redirects the landing branch.
subtasks:
- T026
- T027
- T028
- T029
- T030
- T031
history:
- '2026-08-06: authored from plan IC-05 (flip/deletion) + IC-06 (token migration) + IC-07 (records); C-007 gate consumer'
agent_profile: implementer-ivan
authoritative_surface: src/shell/visual/
create_intent:
- src/shell/tokens.rs
- src/shell/typeface.rs
- src/shell/density.rs
- src/shell/component_state.rs
execution_mode: code_change
owned_files:
- src/shell/visual/**
- src/shell/mod.rs
- src/shell/tokens.rs
- src/shell/typeface.rs
- src/shell/density.rs
- src/shell/component_state.rs
- src/adapter/**
- src/bin/crest_synth.rs
- src/bin/webview_input_probe.rs
- Cargo.toml
- Cargo.lock
- tests/eframe_context.rs
- DESIGN.md
- ROADMAP.md
role: implementer
tags: []
tracker_refs: []
---

## ⚡ Do This First: Load Agent Profile

Before reading anything else in this prompt, load your assigned profile:

```
/ad-hoc-profile-load implementer-ivan
```

Adopt its identity, boundaries, and governance scope for the duration of this
work package.

## Objective

Land the cutover itself. HARD GATE (C-007): before writing a line, verify
WP06's evidence commits exist on your lane's history
(`kitty-specs/webview-shell-cutover-01KZAC7Q/evidence/README.md` with the
four scene logs, A/B, and soak). If absent, STOP and report — this WP does
not start.

Then: relocate the surviving vocabulary declarations out of
`src/shell/visual/`, move the six fader px specimens into the authored
vocabulary, flip the composition root to webview-only, delete the egui
painting layer + eframe adapter + `tests/eframe_context.rs` + both egui
dependencies, and record the pivot in `DESIGN.md`. The deletion commit is the
LAST structural change in the mission (spec C-007) — structure this WP's
commits so evidence → relocation → flip → deletion are separately visible in
history.

Authorities: crest-spec `selected_webview_stack` (sole runtime; the
retirement record), `ShellContextModules` (declarations survive, painting
dies), `AdapterModules` (window adapter = plumbing/transport/translation
only), `CargoManifest` (dependency policy without eframe/egui_extras), spec
FR-002/FR-006/FR-007/NFR-004, SC-003/SC-004.

## Context

- `src/shell/visual/` today: `token.rs`, `typeface.rs`, `density.rs`,
  `state.rs` (declarations — SURVIVE, relocated), `primitives/`,
  `controls/`, `compositions/`, `mod.rs` (egui painting — DELETED).
  Control/composition/structure ENUM declarations that live inside the
  painting modules must be extracted into the relocated declaration files
  before their hosts die — `token_export.rs` and the serialized model
  reference them.
- `tokens.css` generation (`src/shell/webview/token_export.rs`, WP02-owned
  file) reads the authored vocabulary — your relocation must keep the
  generation building; small import-path edits in `token_export.rs` are
  expected: record them as out-of-map with the one-line rationale.
- `webview-page/page.css` still carries six fader specimen px values
  (foundation review housekeeping); WP01 owned that file — your T027 edit
  there is out-of-map: record it.
- The typed-startup-failure proof (`webview_projection_shell` T-series) and
  the `component_composition` transport-only-adapter guard are your
  tripwires: they must be green at every commit of this WP.

## Subtasks

### T026 — Relocate vocabulary declarations

**Purpose**: the authored single source survives the deletion of its old
host directory.

**Steps**:
1. Move `token.rs` → `src/shell/tokens.rs`, `typeface.rs` →
   `src/shell/typeface.rs`, `density.rs` → `src/shell/density.rs`,
   `state.rs` → `src/shell/component_state.rs`. Extract `ComponentControl`,
   `ShellComposition`, `MixerTrackColumnStructure`, `ValuePresentationForm`,
   `ComponentGalleryPage` enum/vocabulary declarations from the painting
   modules into the relocated files (or a `src/shell/component_vocabulary.rs`
   if cleaner — one file per concept family, match repo idiom).
2. Update `src/shell/mod.rs` exports; fix `token_export.rs` imports
   (out-of-map, record).
3. The no-literal guard and token-freshness validation must pass unchanged —
   they assert about the vocabulary, wherever it lives. If a guard
   hardcodes the old path, fix the path in the guard (tests/ edits here are
   out-of-map for the two component test files WP05 owns — record, or fold
   into T029's coordinated commit).

**Validation**: `cargo build` + `component_vocabulary` marker green with
`src/shell/visual/` still present (relocation is a pure move commit).

### T027 — Fader px specimens into the authored vocabulary

**Purpose**: NFR-004 — zero hand-copied styling values anywhere.

**Steps**:
1. Add the six fader specimen values (see foundation review housekeeping
   item; they sit in `webview-page/page.css`) to the authored vocabulary in
   the relocated `tokens.rs` under their canonical authored names, taken
   from the design reference.
2. Regenerate `tokens.css` via the build step; replace the six literals in
   `page.css` with `var(--…)` references (out-of-map on `page.css`,
   record).
3. Token-freshness proof green: a hand-edited table now fails.

### T028 — Composition root webview-only

**Purpose**: FR-002 — the webview shell is the only shell.

**Steps**:
1. In `src/bin/crest_synth.rs` and `src/shell/standalone_application.rs`
   composition (the latter is WP03-owned but merged — you edit crest_synth
   and any residual selection seam): remove the egui/`--shell` selection
   entirely; every mode (interactive, all live modes) constructs
   `TauriWebviewWindow`. Selection code that only ever chose webview
   collapses to direct construction — a launch flag choosing a renderer no
   longer exists.
2. Webview init failure: typed startup error, nonzero exit, no alternate
   window — re-run the typed-failure test and the US4 forced-failure check
   (`CREST_WEBVIEW_PAGE` to an unloadable path in a debug build is the
   cheap forcing seam).
3. `make run`, `make demo-live`, `make demo-live-component-library` all
   webview, all clean.

### T029 — Delete the egui layer

**Purpose**: FR-006 — the cruft leaves the tree.

**Steps**:
1. Delete `src/shell/visual/primitives/`, `controls/`, `compositions/`, the
   painting `mod.rs` remnants, and then the emptied `src/shell/visual/`.
2. Delete the eframe window adapter (the `EframeGraphicalWindow` module
   under `src/adapter/` or `src/shell/` — locate by its type name) and every
   `use eframe`/`use egui` site.
3. Delete `tests/eframe_context.rs` (its contract lives on in
   `tests/shell_event_dispatch.rs`, green since WP05).
4. Remove `eframe` and `egui_extras` from `Cargo.toml`; `cargo update
   --workspace` regenerates `Cargo.lock`; confirm the tauri/wry/objc2 set
   still matches the crest-spec dependency policy.
5. Full suite green at this commit; this is THE deletion commit — keep it
   free of unrelated edits.

### T030 — Records

**Purpose**: FR-007 and the probe decision — the pivot is recorded where
products are defined, not only where missions live.

**Steps**:
1. `DESIGN.md`: record the webview projection shell as the product's
   rendering approach — brief, factual, replacing/annotating whatever
   describes the egui stack today; point at the crest-spec
   (`selected_webview_stack`) as the declaration of record. Do not rewrite
   unrelated design prose.
2. `ROADMAP.md`: under the cutover gate, add the closure evidence pointers
   (WP06 README, deletion commit hash) — add-only, the gate text stays.
3. Probe binary decision: `src/bin/webview_input_probe.rs` (484-line
   foundation evidence artifact). Recommended disposition: delete the
   `[[bin]]` — its evidence is the committed probe verdict, and the
   key-injection witness (WP05) now covers the living contract. Record the
   decision and rationale in the ROADMAP gate note either way.

### T031 — Sweep and record the numbers

**Purpose**: SC-003 / SC-004 made checkable.

**Steps**:
1. `grep -ri "egui\|eframe" src/ tests/ webview-page/ Cargo.toml` → zero
   hits (comments included; historical mission/evidence docs exempt).
2. Full `cargo test` (all targets), `cargo clippy --all-targets`,
   `cargo fmt --check` — green/clean.
3. Record in the ROADMAP gate note: net line delta of the mission's diff
   (`git diff --stat` against the pre-mission base), which must show ≥10k
   net reduction (SC-004), and the zero-reference grep result (SC-003).

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-cutover`.
Execution worktrees are allocated per computed lane from `lanes.json`; enter
the lane workspace `spec-kitty agent action implement WP07 --agent claude`
gives you. Commit order within the lane: relocation → tokens → flip →
deletion → records/sweep.

## Definition of Done

- Evidence-precedes-deletion verified and linked (C-007).
- Vocabulary relocated with guards green; fader tokens authored and
  consumed; `tokens.css` regenerated.
- One shell: forced init failure exits typed with no alternate window.
- Zero egui/eframe references in production tree and manifests; deps gone;
  `tests/eframe_context.rs` gone; suite/clippy/fmt green.
- DESIGN.md pivot recorded; ROADMAP gate closure notes with numbers; probe
  decision recorded.

## Reviewer Guidance

- Verify commit order: the deletion commit must postdate the WP06 evidence
  commits AND contain no non-deletion changes.
- Run the forced-failure scenario yourself — a blank or alternate window is
  an instant reject (US4).
- grep the final tree for egui/eframe including comments and Cargo.lock.
- Diff DESIGN.md: the pivot record must not quietly rewrite unrelated
  product decisions.
- Check SC-004 arithmetic against `git diff --stat` yourself.
