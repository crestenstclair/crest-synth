# Implementation Plan: Webview Shell Foundation

**Mission**: `webview-shell-foundation-01KZ9DN7`
**Branch contract**: planned on `feat/webview-shell-foundation`; completed
changes merge into `feat/webview-shell-foundation`, which lands on `main` via
PR (mission is `pr_bound`).
**Spec**: [spec.md](spec.md) · **Research**: [research.md](research.md)

## Summary

Add a Tauri v2 webview shell as an explicitly selected peer of the eframe
window behind the same `AppWindow` contract. The webview renders the MIXER
context from the serde serialization of the canonical
`SemanticGraphicalViewModel` (one schema, no fork), with meters on a separate
30 Hz latest-value channel, input captured Rust-side through the existing
`KeyboardInputTranslator`, a token table generated from the authored Rust
vocabulary, typed startup failure, and the same owned shutdown as the eframe
shell. egui remains the default; nothing egui is deleted.

## Technical Context

**Language/Version**: Rust 2021 (existing toolchain) + page-side ES2022
JavaScript, no framework, no bundler
**Primary Dependencies**: `tauri` 2.x (wry/tao) added alongside existing
`eframe` 0.32 / `cpal` 0.18 / `serde_json`; page consumes vendored Azeret
Mono and generated `tokens.css`
**Storage**: N/A
**Testing**: `cargo test` integration targets; new declared validation
`webview_projection_shell` (tests/webview_projection_shell.rs); paced live
demo through the Tauri window for the live layer
**Target Platform**: macOS desktop first (WKWebView); shell-selection design
keeps Linux/Windows webviews reachable later
**Project Type**: single Rust binary, two selectable shells, static page
assets under `webview-page/`
**Performance Goals**: NFR-001 projection-to-paint ≤ 50 ms p95; NFR-002
meters 30 Hz sustained, no queue growth over 5 min; NFR-003 RT callback
bounds unchanged
**Constraints**: C-001–C-005 (spec) — boundaries untouched, page stateless,
egui default with proofs passing, MIXER-only scope, crest-spec-declared
structure only
**Scale/Scope**: one screen (MIXER), 16 columns + inspector; ~6 Rust modules
(`src/shell/webview/`), one page directory, one test target

## Charter Check

Compact charter context loaded. Directives honored in this plan:
DIRECTIVE_035 (no bulk edit here — additive mission, `change_mode` default),
DIRECTIVE_043 (constraints enforced structurally: schema-fork guard and token
freshness are declared validations, not discipline), DIRECTIVE_044 (canonical
sources: one view-model schema, one token vocabulary, generated artifacts
asserted fresh). No conflicts found.

## Crest-Spec Derivation

- **Adds**: `adapter.TauriWebviewWindow` (peer implementer of
  `port.AppWindow`); `requirement.webview_projection_shell`;
  `requirement.serialized_projection_transport`.
- **Changes**: `requirement.selected_egui_stack` — amended in the crest-spec
  phase: webview is the one declared alternate runtime, egui stays default.
- **Retires**: none (egui retirement is the successor mission's declaration).
- **Assets → files**:
  - `asset.WebviewShellModules` → `src/shell/webview/**` (window
    composition, transports, shell selection, token generation)
  - `asset.WebviewProjectionPage` → `webview-page/**` (MIXER page +
    generated `tokens.css`)
  - `asset.WebviewProjectionShellAcceptanceTests` →
    `tests/webview_projection_shell.rs`
  - `asset.CargoManifest` → `Cargo.toml` (tauri dependency)
- **Proof covering the change**: validation `webview_projection_shell`
  (schema fidelity, token freshness, page determinism, typed startup failure,
  owned shutdown); existing `component_vocabulary`, `graphical_application_shell`,
  and demo validations must remain green (C-003).

## Project Structure

### Documentation (this mission)

```
kitty-specs/webview-shell-foundation-01KZ9DN7/
├── spec.md
├── plan.md
├── research.md
└── checklists/requirements.md
```

No `data-model.md`, no `contracts/` — crest-spec is canonical.

### Source Code (repository root)

```
src/shell/webview/
├── mod.rs                  # shell selection + typed startup error
├── window.rs               # tauri::Builder composition, AppWindow impl
├── projection_channel.rs   # serialized view-model push (Emitter)
├── meter_channel.rs        # 30 Hz latest-value AudioObservationSnapshot
├── input_capture.rs        # native key capture -> KeyboardInputTranslator
└── token_export.rs         # tokens.css generation from the vocabulary
webview-page/
├── index.html              # MIXER projection surface
├── page.js                 # document -> DOM render function (pure)
└── tokens.css              # GENERATED — never hand-edited
tests/
└── webview_projection_shell.rs
```

## Complexity Tracking

Carrying two GUI stacks in one binary until the successor mission retires
egui — accepted deliberately (R-01); cost is build weight, not runtime
coupling. No other charter-relevant complexity added.

## Implementation Concern Map

### IC-01 — Native input capture probe (riskiest first)

The one mechanism research could not settle (R-02): whether tao surfaces
window-level key events under a focused WKWebView. Build a disposable probe:
Tauri window + `on_window_event` logging; if keys don't surface, wire the
`NSEvent` local monitor. Exit criterion: every key in the MIXER vocabulary
observed Rust-side with press/release fidelity while the webview has focus.
If both paths fail: STOP, return to `/spec-kitty.crest-spec` (the page-side
conduit would need declaring). Also proves tauri + cpal stream coexistence by
playing the production fixture during the probe.

### IC-02 — Shell selection and window composition

`src/shell/webview/mod.rs` + `window.rs`: launch-time selection (explicit
flag), `AppWindow` contract satisfied by the Tauri composition, typed
startup error path (FR-007), owned shutdown parity (FR-006). The eframe path
is untouched; `StandaloneApplication` gains the selection seam only.

### IC-03 — Projection and meter transports

`projection_channel.rs` + `meter_channel.rs`: push serialized view model on
accepted projection; coalesce meters at 30 Hz latest-value; nothing blocks,
nothing queues unboundedly, nothing touches the RT callback (FR-004, FR-005,
NFR-002, NFR-003).

### IC-04 — Token generation

`token_export.rs` emits `webview-page/tokens.css` from the authored
vocabulary; freshness assertion in the acceptance test (FR-002, R-04).

### IC-05 — MIXER projection page

`webview-page/`: pure render function over the serialized document; authored
MIXER composition per the declared `MixerTrackColumnStructure` anatomy; both
authored viewports; hex level readout binding; focus/state visible beyond
color (FR-001, FR-002, NFR-004). Seeded from the spike
(`spike/webview-mixer/`), rebuilt against generated tokens.

### IC-06 — Acceptance and live proof

`tests/webview_projection_shell.rs` realizing the declared validation's five
proofs (schema fidelity, token freshness, page determinism, typed startup
failure, shutdown parity), plus the paced live demo run through the Tauri
window; existing egui-default proofs stay green (C-003, R-05).

## Phase Ordering

IC-01 strictly first (kill risk before dependent work). IC-02 next, then
IC-03/IC-04 in parallel, then IC-05, then IC-06. NFR-001/NFR-002 measured
inside IC-06's live layer.
