# Implementation Plan: Webview Shell Cutover

**Branch**: `feat/webview-shell-cutover` | **Date**: 2026-08-06 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `kitty-specs/webview-shell-cutover-01KZAC7Q/spec.md`

## Summary

Make the Tauri webview the sole shell: render PATCH and every shipped surface
from the canonical serialized `SemanticGraphicalViewModel`, give the painted-ack
→ `ShellFrameObservation` forwarding an owner, re-run all four retained live
scenes through the webview on hardware, rebuild the gallery in the webview,
then — and only then — delete `src/shell/visual/` (~17k lines), the eframe
window adapter, and the `eframe`/`egui_extras` dependencies. The crest-spec
retirement is already authored (commit `307873e`); this plan derives from it.

## Technical Context

**Language/Version**: Rust (stable, 2021 edition) for the application; vanilla HTML/CSS/JS (no framework, no bundler) for the embedded webview page
**Primary Dependencies**: tauri v2 (wry system webview, default macOS bundle features only), objc2/objc2-app-kit NSEvent local monitor for key capture, serde/serde_json (projection transport), cpal, rustysynth, rtrb, triple_buffer; **removed at cutover**: eframe 0.32.3, egui_extras 0.32.3
**Storage**: N/A (fixed file fixtures: `sf2/HiDef.sf2`, `midi/Corridors of Time - Chrono Trigger.mid`)
**Testing**: cargo integration targets behind the 32 declared project validations (`spec-kitty accept`); deterministic twins plus operator-run hardware live scenes (`make demo-live-*`)
**Target Platform**: macOS desktop (physical rig); authored viewports 1920x1080 and 1280x800 (Steam Deck layout)
**Project Type**: single Rust crate + embedded `webview-page/` assets served over `crest://`
**Performance Goals**: reducer state change visible in the webview ≤50 ms p95 under the paced live workload; meter channel sustains 30 Hz for 5 minutes without queue growth; RT callback measured bounds unchanged from the recorded pre-cutover baseline (`requirement.serialized_projection_transport`)
**Constraints**: hard RT callback (no allocation/locking/blocking/destruction); one-way loop with input Rust-side only; add-only frozen checkpoint identities in re-hosted scenes (C-004); evidence commits land before the deletion commit (C-007); no `data-model.md`/`contracts/` (C-005)
**Scale/Scope**: ~17k lines deleted, 4 retained live scenes re-hosted, 15 gallery pages rebuilt, 8 FRs / 4 NFRs / 7 constraints, net reduction target ≥10k lines (SC-004)

## Charter Check

*GATE: evaluated against `.kittify/charter/charter.md` (2026-07-31 posture).*

- **Full mission rigor**: this mission runs specify → crest-spec → plan → tasks → implement → review → accept; crest-spec authored first at `307873e`. PASS.
- **Silent design drift is the costliest failure**: the renderer pivot is recorded in ROADMAP (gate, `d526b0f`), the crest-spec (`selected_webview_stack` carries the deliberate retirement), and DESIGN.md gets the pivot record as mission work (FR-007). PASS.
- **Retained live-demo scenes as phase gates**: preserved and central — every retained scene re-runs through the webview with refreshed hardware evidence before deletion. PASS.
- **Proof-enforced invariants over prose**: the no-literal guard, token generation freshness, add-only identity baselines, and the page-registers-no-input assertion all stay executable. PASS.
- **Exceptions self-service but never silent**: none anticipated; any waiver commits its rationale in-repo. PASS.

No violations → Complexity Tracking not required.

## Crest-Spec Derivation

Authored in `/spec-kitty.crest-spec` at commit `307873e`; doctor clean (132 resources, 32 project validations, 3787 relationships).

- **Retires**: `adapter.EframeGraphicalWindow`; `requirement.selected_egui_stack`; `validation.egui_context`, `validation.adapter.eframe_context`, `validation.asset.acceptance_eframe_context`; eframe/egui_extras in the CargoManifest dependency policy.
- **Changes**: `adapter.TauriWebviewWindow` (sole AppWindow implementer; full-surface PATCH+MIXER; painted-ack → `ShellFrameObservation` forwarding; transferred meter-pairing/viewport/live-tick/close rules); `requirement.webview_projection_shell` (every shipped surface); `requirement.passive_graphical_window`, `requirement.graphical_shell_behavioral_proof`, `requirement.semantic_view_model_behavioral_proof`, `requirement.deterministic_demo_scene`, `requirement.separate_live_demo` (webview end-state); `capability.graphical_application_shell` acceptance; `context.Shell` token/input invariants; 11 witness participant lists; 3 evidence descriptions.
- **Adds/renames**: `requirement.selected_webview_stack`; `requirement.headless_shell_event_verification`; `validation.shell_event_dispatch` + `validation.adapter.shell_event_dispatch` + `validation.asset.acceptance_shell_event_dispatch` (test target `tests/shell_event_dispatch.rs`); `validation.adapter.tauri_webview_window`.
- **Assets → files**:
  - `WebviewShellModules` → `src/shell/webview/` (sole-shell composition, projection/meter channels, painted-ack forwarding, render-exception surfacing, CSP, release-gated `CREST_WEBVIEW_PAGE`, token export)
  - `WebviewProjectionPage` → `webview-page/` (PATCH composition, gallery pages, paint acknowledgment, MIXER as shipped, `tokens.css` generated)
  - `ShellContextModules` → `src/shell/` (vocabulary/policy/state/control/composition declarations the page renders; `src/shell/visual/` painting layer deleted)
  - `AdapterModules` → window adapter reduced to plumbing/transport/translation
  - `CargoManifest` → `Cargo.toml` dependency swap
  - `GraphicalShellAcceptanceTests`, `BehavioralAcceptanceTests`, `SemanticGraphicalViewModelAcceptanceTests`, `WebviewProjectionShellAcceptanceTests`, `ComponentVocabularyAcceptanceTests`, `ComponentCompositionAcceptanceTests` → the retargeted test files incl. `tests/shell_event_dispatch.rs`
  - `TestingContextModules` → gallery scene through the webview surface; scene hosting unchanged Rust-side
- **Validations/witnesses covering the change**: `validation.webview_projection_shell` (fidelity, token freshness, determinism, typed failure, shutdown); `validation.shell_event_dispatch` (headless event → document coherence); `validation.component_vocabulary` / `validation.component_composition` (re-declared against the webview path); `validation.graphical_application_shell`, `validation.semantic_graphical_view_model`; live-demo witnesses with `TauriWebviewWindow` as participant; `requirement.serialized_projection_transport` carries the RT-baseline and 5-minute-sustain proof obligations.

## Project Structure

### Documentation (this mission)

```
kitty-specs/webview-shell-cutover-01KZAC7Q/
├── plan.md              # This file
├── research.md          # Phase 0 output — decisions consolidated, no open unknowns
├── quickstart.md        # Phase 1 output — run/verify the webview shell and scenes
└── tasks.md             # /spec-kitty.tasks output (NOT created here)
```

No `data-model.md`, no `contracts/` — crest-spec project (C-005).

### Source Code (repository root)

```
src/shell/
├── mod.rs, app_window.rs, keyboard_input_translator.rs, window_input.rs,
│   shell_frame_observation.rs, standalone_application.rs   # unchanged boundaries
├── visual/                    # DELETED at cutover (token.rs/typeface.rs/density.rs/state.rs
│                              #   declarations move up to src/shell/ before deletion)
└── webview/
    ├── window.rs              # sole-shell composition, CSP, release-gated override, close retry
    ├── projection_channel.rs  # serialized document push + painted-ack forwarding (new owner)
    ├── meter_channel.rs       # decimated latest-value meters (unchanged contract)
    ├── input_capture.rs       # NSEvent monitor (unchanged) + key-injection witness seam
    └── token_export.rs        # build-step token table generation (absorbs fader px values)

webview-page/
├── index.html, page.js        # + paint acknowledgment
├── page.css                   # + PATCH composition, gallery pages; fader px values → tokens
└── tokens.css                 # generated

src/testing/                   # scenes unchanged in shape; component_gallery_scene.rs
                               #   re-hosted on the webview surface
src/bin/crest_synth.rs         # composition root: webview-only, egui path removed
tests/                         # shell_event_dispatch.rs replaces eframe_context.rs;
                               #   graphical_application_shell.rs, semantic_graphical_view_model.rs,
                               #   component_vocabulary.rs, component_composition.rs retargeted
```

**Structure Decision**: single crate; the webview modules and page already exist from the foundation mission — this mission widens them to full surface and removes the parallel renderer.

## Implementation Concern Map

### IC-01 — PATCH surface through the webview page

- **Purpose**: the page renders the full PATCH context (strip, identity, envelope, engine/effect-slot rows, Utility, footer, hints) from the same serialized model; MIXER already ships.
- **Relevant requirements**: FR-001, C-002, NFR-004
- **Affected surfaces**: `webview-page/page.css`, `webview-page/page.js`, `src/shell/webview/token_export.rs`
- **Sequencing/depends-on**: none
- **Risks**: PATCH focus/edit visual states must match the declared ComponentState treatments exactly; the deterministic `webview_projection_shell` fidelity proof extends to PATCH documents.

### IC-02 — Painted-ack → ShellFrameObservation forwarding

- **Purpose**: give the acknowledgment forwarding an explicit owner so live reports credit real painted webview frames (foundation DRIFT-4/RISK-1).
- **Relevant requirements**: FR-004, C-003
- **Affected surfaces**: `src/shell/webview/projection_channel.rs`, `src/shell/webview/window.rs` (stale WP06 comment), `src/shell/shell_frame_observation.rs` consumers
- **Sequencing/depends-on**: none
- **Risks**: scenes must block on qualifying frames, not wall-clock sleeps; observation must be constructible only post-paint (crest-spec invariant).

### IC-03 — Retained scene re-hosting + hardware evidence

- **Purpose**: all four `make demo-live-<scene>` targets run through the webview shell; RT A/B taken while both shells exist; 300 s soak recorded; evidence committed.
- **Relevant requirements**: FR-003, NFR-001, NFR-002, C-004, C-007
- **Affected surfaces**: `src/testing/live_*_scene.rs` (shell hosting only), `src/bin/crest_synth.rs`, `Makefile`, scene evidence dirs under `kitty-specs/`
- **Sequencing/depends-on**: IC-01, IC-02
- **Risks**: frozen identity baselines must stay byte-identical (add-only); hardware runs are operator-executed — deterministic twins must pass first so rig time is not wasted.

### IC-04 — Gallery through the webview

- **Purpose**: rebuild the 15-page component gallery on the webview surface (operator decision: rebuild, not retire).
- **Relevant requirements**: FR-005, C-006
- **Affected surfaces**: `src/testing/component_gallery_scene.rs`, `webview-page/`, `Makefile` (`demo-live-component-library`)
- **Sequencing/depends-on**: IC-01
- **Risks**: gallery keeps its no-audio/no-MIDI declared scope; digit/stepping input stays Rust-side scene-local.

### IC-05 — Sole-shell flip, egui deletion, test retargeting

- **Purpose**: composition root goes webview-only; delete `src/shell/visual/`, the eframe adapter, and both dependencies; retarget the five egui-path test files; move surviving vocabulary declarations out of `visual/` first.
- **Relevant requirements**: FR-002, FR-006, C-001, C-003; SC-003, SC-004
- **Affected surfaces**: `src/bin/crest_synth.rs`, `src/shell/visual/` (deleted), `src/adapter/`, `Cargo.toml`/`Cargo.lock`, `tests/eframe_context.rs` → `tests/shell_event_dispatch.rs`, `tests/graphical_application_shell.rs`, `tests/semantic_graphical_view_model.rs`, `tests/component_*.rs`
- **Sequencing/depends-on**: IC-03 (evidence first — C-007), IC-04
- **Risks**: this is where a silent fallback could sneak in — the typed-startup-failure test must pass with no alternate window; the no-literal guard must survive the vocabulary relocation.

### IC-06 — Hardening and witness

- **Purpose**: restrictive CSP; `CREST_WEBVIEW_PAGE` behind `cfg(debug_assertions)`; automated key-injection witness through the production translator; six fader px values into the generated tokens.
- **Relevant requirements**: FR-008, NFR-003, NFR-004
- **Affected surfaces**: `src/shell/webview/window.rs`, `src/shell/webview/input_capture.rs`, `webview-page/page.css`, `tests/` (witness)
- **Sequencing/depends-on**: none (CSP/gating early; witness once IC-02 lands)
- **Risks**: CSP must not break `crest://` embedded assets; witness must drive the real NSEvent path, not a synthetic shortcut around the translator.

### IC-07 — Records: DESIGN.md pivot + closure

- **Purpose**: DESIGN.md records the webview shell as the product's rendering approach; probe-binary retention decision recorded; ROADMAP gate closure notes at accept.
- **Relevant requirements**: FR-007
- **Affected surfaces**: `DESIGN.md`, `ROADMAP.md`, `src/bin/webview_input_probe.rs` (retention decision)
- **Sequencing/depends-on**: text can land any time; closure notes after IC-03/IC-05 evidence
- **Risks**: none — records only.

## Phase Sequencing (derived, for /spec-kitty.tasks)

1. IC-01 + IC-02 + IC-06(CSP/gating) in parallel.
2. IC-03 deterministic twins → operator hardware runs (scenes, RT A/B, soak) → evidence commits. IC-04 alongside once IC-01 lands.
3. IC-05 deletion only after IC-03 evidence is committed (C-007), then IC-06 witness at HEAD, IC-07 closure.
