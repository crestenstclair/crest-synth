# Work Packages: Webview Shell Foundation

**Mission**: `webview-shell-foundation-01KZ9DN7`
**Branch contract**: planned on `feat/webview-shell-foundation`; merges into
`feat/webview-shell-foundation`; lands on `main` via PR.
**Derived from**: [plan.md](plan.md) (IC-01…IC-06), which derives from the
crest-spec assets `WebviewShellModules`, `WebviewProjectionPage`,
`WebviewProjectionShellAcceptanceTests`, `CargoManifest`.

## Subtask Index

| ID | Description | WP | Parallel |
|----|-------------|-----|----------|
| T001 | Add tauri 2.x dependency to Cargo.toml | WP01 | |
| T002 | Build the disposable input-capture probe window | WP01 | |
| T003 | Wire the NSEvent local-monitor fallback if tao events do not surface | WP01 | |
| T004 | Run the cpal production-fixture stream inside the probe | WP01 | |
| T005 | Record the probe verdict in research/input-capture-probe.md | WP01 | |
| T006 | Declare shell selection and the typed webview startup error | WP02 | |
| T007 | Compose the Tauri window satisfying the AppWindow contract | WP02 | |
| T008 | Add the launch-time selection seam to StandaloneApplication and the bin | WP02 | |
| T009 | Drive window close through the owned shutdown path | WP02 | |
| T010 | Prove typed init failure ends the process with no fallback | WP02 | |
| T011 | Push serialized view models on accepted projections | WP03 | [P] |
| T012 | Coalesce meters at 30 Hz latest-value | WP03 | [P] |
| T013 | Assert the transports add nothing to the RT callback | WP03 | |
| T014 | Generate tokens.css from the authored Rust vocabulary | WP04 | [P] |
| T015 | Wire generation into the Makefile and commit the generated table | WP04 | |
| T016 | Expose the freshness check the acceptance test consumes | WP04 | |
| T017 | Author index.html — the MIXER projection surface skeleton | WP05 | |
| T018 | Author page.js — the pure document→DOM render function | WP05 | |
| T019 | Lay out the authored bank and column anatomy from generated tokens | WP05 | |
| T020 | Render the Inspector, hint rows, and focus/state emphasis | WP05 | |
| T021 | Hold both authored viewports | WP05 | |
| T022 | Prove serialized-schema fidelity | WP06 | |
| T023 | Prove token-table freshness | WP06 | |
| T024 | Prove page render determinism at both viewports | WP06 | |
| T025 | Prove typed startup failure | WP06 | |
| T026 | Prove shutdown parity and run the live layer with NFR measurements | WP06 | |

## Work Packages

### WP01 — Input capture and coexistence probe

**Goal**: kill the mission's only unsettled mechanism (plan IC-01, research
R-02) before dependent work: Rust-side key capture under a focused WKWebView,
and tauri + cpal coexistence in one process.
**Priority**: P1 — strictly first.
**Independent test**: every key in the MIXER vocabulary observed Rust-side
with press/release fidelity while the webview has focus and the production
fixture audibly plays; verdict recorded.
**Subtasks**: T001 T002 T003 T004 T005
**Dependencies**: none.
**Risk**: both capture paths fail → STOP the mission and return to
`/spec-kitty.crest-spec` (declared in plan IC-01).
**Prompt**: [tasks/WP01-input-capture-probe.md](tasks/WP01-input-capture-probe.md) (~340 lines)

T001 Add tauri 2.x dependency to Cargo.toml (WP01)
T002 Build the disposable input-capture probe window (WP01)
T003 Wire the NSEvent local-monitor fallback if tao events do not surface (WP01)
T004 Run the cpal production-fixture stream inside the probe (WP01)
T005 Record the probe verdict in research/input-capture-probe.md (WP01)

### WP02 — Shell selection and window composition

**Goal**: the Tauri window as an explicitly selected peer behind the
`AppWindow` contract, with typed startup failure and owned shutdown (plan
IC-02; FR-001, FR-006, FR-007).
**Priority**: P1.
**Independent test**: `crest-synth --shell webview` opens the Tauri window and
close exits cleanly through the same shutdown observations as the egui shell;
an unloadable page yields the typed error and process exit.
**Subtasks**: T006 T007 T008 T009 T010
**Dependencies**: WP01.
**Prompt**: [tasks/WP02-shell-selection-and-window.md](tasks/WP02-shell-selection-and-window.md) (~380 lines)

T006 Declare shell selection and the typed webview startup error (WP02)
T007 Compose the Tauri window satisfying the AppWindow contract (WP02)
T008 Add the launch-time selection seam to StandaloneApplication and the bin (WP02)
T009 Drive window close through the owned shutdown path (WP02)
T010 Prove typed init failure ends the process with no fallback (WP02)

### WP03 — Projection and meter transports

**Goal**: serialized view-model push and 30 Hz latest-value meter channel,
nothing touching the RT callback (plan IC-03; FR-004, FR-005, NFR-002,
NFR-003).
**Priority**: P1.
**Independent test**: a reducer edit reaches the page as the exact projector
serialization; meters stream at 30 Hz for 5 minutes with no queue growth.
**Subtasks**: T011 T012 T013
**Dependencies**: WP02.
**Prompt**: [tasks/WP03-projection-and-meter-transports.md](tasks/WP03-projection-and-meter-transports.md) (~300 lines)

T011 Push serialized view models on accepted projections (WP03)
T012 Coalesce meters at 30 Hz latest-value (WP03)
T013 Assert the transports add nothing to the RT callback (WP03)

### WP04 — Token generation

**Goal**: `webview-page/tokens.css` generated from the authored Rust
vocabulary with a freshness guarantee (plan IC-04; FR-002).
**Priority**: P2 — parallel with WP03.
**Independent test**: regenerating tokens.css produces a byte-identical file;
mutating a token value makes the freshness check fail.
**Subtasks**: T014 T015 T016
**Dependencies**: WP02.
**Prompt**: [tasks/WP04-token-generation.md](tasks/WP04-token-generation.md) (~260 lines)

T014 Generate tokens.css from the authored Rust vocabulary (WP04)
T015 Wire generation into the Makefile and commit the generated table (WP04)
T016 Expose the freshness check the acceptance test consumes (WP04)

### WP05 — MIXER projection page

**Goal**: the authored MIXER composition as a pure render over the serialized
document, from generated tokens, at both viewports (plan IC-05; FR-001,
FR-002, NFR-004).
**Priority**: P1.
**Independent test**: the page renders the recorded production fixture
document to the authored composition at 1920×1080 and 1280×800; same document
twice → same DOM.
**Subtasks**: T017 T018 T019 T020 T021
**Dependencies**: WP03, WP04.
**Prompt**: [tasks/WP05-mixer-projection-page.md](tasks/WP05-mixer-projection-page.md) (~420 lines)

T017 Author index.html — the MIXER projection surface skeleton (WP05)
T018 Author page.js — the pure document→DOM render function (WP05)
T019 Lay out the authored bank and column anatomy from generated tokens (WP05)
T020 Render the Inspector, hint rows, and focus/state emphasis (WP05)
T021 Hold both authored viewports (WP05)

### WP06 — Acceptance and live proof

**Goal**: `tests/webview_projection_shell.rs` realizing the declared
validation's five proofs, plus the live layer with NFR measurements (plan
IC-06; the `webview_projection_shell` validation).
**Priority**: P1 — last.
**Independent test**: `cargo test --test webview_projection_shell` passes and
prints `CREST_ACCEPTANCE webview_projection_shell passed`; the paced live demo
completes through the Tauri window; existing egui-default proofs stay green.
**Subtasks**: T022 T023 T024 T025 T026
**Dependencies**: WP05.
**Prompt**: [tasks/WP06-acceptance-and-live-proof.md](tasks/WP06-acceptance-and-live-proof.md) (~400 lines)

T022 Prove serialized-schema fidelity (WP06)
T023 Prove token-table freshness (WP06)
T024 Prove page render determinism at both viewports (WP06)
T025 Prove typed startup failure (WP06)
T026 Prove shutdown parity and run the live layer with NFR measurements (WP06)

## Execution Notes

- **Ordering**: WP01 → WP02 → {WP03 ∥ WP04} → WP05 → WP06.
- **Parallelization**: WP03 and WP04 have disjoint owned files and can run in
  separate lanes after WP02 merges.
- **MVP scope**: WP01+WP02 alone answer "does the pivot hold" — window opens,
  keys captured Rust-side, audio plays, clean shutdown.
- **Completion tracking**: `spec-kitty agent tasks mark-status Txxx --status done`
  (event-sourced; no checkboxes).
