# Tasks: Webview Shell Cutover

**Mission**: `webview-shell-cutover-01KZAC7Q`
**Branch**: `feat/webview-shell-cutover` (PR-bound to `main`)
**Input**: plan.md (IC-01…IC-07), spec.md (FR-001…008, NFR-001…004, C-001…007), crest-spec `307873e`

## Overview

Seven work packages in three waves. Wave 1 (WP01, WP02) builds PATCH rendering
and the frame-observation forwarding in parallel. Wave 2 (WP03, WP04, WP05)
hosts the scenes, rebuilds the gallery, and retargets the test suite — all
while egui still exists and the tree stays green. Wave 3 (WP06 evidence, then
WP07 deletion) enforces C-007: hardware evidence commits land before the egui
deletion commit. WP07 is the only WP allowed to delete.

## Subtask Index

| ID | Description | WP | Parallel |
|----|-------------|----|----------|
| T001 | PATCH workspace layout in page CSS at both viewports | WP01 | [P] |
| T002 | PATCH render sections + focus/state treatments in page JS | WP01 | [P] |
| T003 | Paint-acknowledgment emission from the page | WP01 | |
| T004 | Extend webview_projection_shell fidelity proof to PATCH + ack | WP01 | |
| T005 | Painted-ack → ShellFrameObservation forwarding (owner) | WP02 | [P] |
| T006 | Qualifying-frame stream for live-report crediting; fix stale WP06 comment | WP02 | |
| T007 | Restrictive CSP + cfg(debug_assertions) on CREST_WEBVIEW_PAGE | WP02 | [P] |
| T008 | Typed page render-exception surfacing + close retry-or-surface | WP02 | |
| T009 | Forwarding/typed-error test coverage | WP02 | |
| T010 | Host all four live scenes on TauriWebviewWindow (egui still default) | WP03 | |
| T011 | Scenes block on qualifying forwarded frames, never sleeps | WP03 | |
| T012 | Makefile demo-live-* targets run the webview shell | WP03 | |
| T013 | Deterministic twins green with add-only frozen identities | WP03 | |
| T014 | Gallery page documents/layouts through generated tokens | WP04 | [P] |
| T015 | Gallery scene re-hosted on the webview surface | WP04 | |
| T016 | Gallery coverage assertions + demo-live-component-library target | WP04 | |
| T017 | tests/shell_event_dispatch.rs replaces the egui-context contract | WP05 | [P] |
| T018 | Retarget tests/graphical_application_shell.rs to the webview path | WP05 | [P] |
| T019 | Retarget tests/semantic_graphical_view_model.rs render half | WP05 | [P] |
| T020 | Retarget component_vocabulary/component_composition tests | WP05 | [P] |
| T021 | Automated key-injection witness through the NSEvent path | WP05 | |
| T022 | RT A/B same-workload measurement while both shells exist | WP06 | |
| T023 | Four scene hardware runs; logs committed as evidence | WP06 | |
| T024 | 300 s soak recorded | WP06 | |
| T025 | Evidence summary + hardware add-only identity comparison | WP06 | |
| T026 | Relocate vocabulary declarations out of src/shell/visual/ | WP07 | |
| T027 | Fader px specimens into the authored vocabulary + regenerated tokens | WP07 | |
| T028 | Composition root webview-only; egui selection removed | WP07 | |
| T029 | Delete visual layer, eframe adapter, eframe_context test, egui deps | WP07 | |
| T030 | DESIGN.md pivot record, ROADMAP notes, probe retention decision | WP07 | |
| T031 | Full suite/clippy/fmt + zero-reference sweep + line-count record | WP07 | |

## Work Packages

### WP01 — PATCH surface and acknowledgment in the page

- **Goal**: the page renders the full PATCH context from the canonical serialized model and acknowledges every painted document.
- **Priority**: P1 (US1) · **Prompt**: `tasks/WP01-patch-page-and-ack.md` (~380 lines)
- **Subtasks**: T001 T002 T003 T004
- **Independent test**: `cargo test --test webview_projection_shell` proves PATCH documents render deterministically and every paint acks; visual check via `CREST_WEBVIEW_PAGE` seam.
- **Dependencies**: none. **Parallel with**: WP02.
- **Risks**: PATCH state treatments must match declared ComponentState set exactly; no page-invented field.

T001 PATCH workspace layout in page CSS at both viewports (WP01)
T002 PATCH render sections + focus/state treatments in page JS (WP01)
T003 Paint-acknowledgment emission from the page (WP01)
T004 Extend webview_projection_shell fidelity proof to PATCH + ack (WP01)

### WP02 — Frame observation forwarding and shell hardening

- **Goal**: painted-ack → `ShellFrameObservation` forwarding has an owner; CSP, release gating, typed exception/close handling land.
- **Priority**: P1 (US2 enabler) · **Prompt**: `tasks/WP02-forwarding-and-hardening.md` (~360 lines)
- **Subtasks**: T005 T006 T007 T008 T009
- **Independent test**: forwarding unit/integration tests; a paint produces exactly one observation copying semantic identity; forced page failure yields typed error.
- **Dependencies**: none. **Parallel with**: WP01.
- **Risks**: observation must be constructible only post-paint (crest-spec invariant); CSP must not break `crest://` assets.

T005 Painted-ack → ShellFrameObservation forwarding (WP02)
T006 Qualifying-frame stream for live-report crediting (WP02)
T007 Restrictive CSP + release gating of CREST_WEBVIEW_PAGE (WP02)
T008 Typed render-exception surfacing + close retry-or-surface (WP02)
T009 Forwarding/typed-error test coverage (WP02)

### WP03 — Live scenes hosted on the webview shell

- **Goal**: all four retained `make demo-live-<scene>` targets run through `TauriWebviewWindow`, blocking on qualifying frames; deterministic twins stay green with add-only identities. egui remains the interactive default until WP07.
- **Priority**: P1 (US2) · **Prompt**: `tasks/WP03-scene-hosting.md` (~420 lines)
- **Subtasks**: T010 T011 T012 T013
- **Independent test**: each twin passes; `make demo-live-effects-and-buses` opens the webview window locally and completes teardown.
- **Dependencies**: WP01, WP02.
- **Risks**: frozen baselines byte-identical (C-004); crest_synth.rs arg plumbing is WP07-owned — record any touch as out-of-map with rationale.

T010 Host all four live scenes on TauriWebviewWindow (WP03)
T011 Scenes block on qualifying forwarded frames (WP03)
T012 Makefile demo-live-* targets run the webview shell (WP03)
T013 Deterministic twins green, add-only identities (WP03)

### WP04 — Component gallery through the webview

- **Goal**: the 15-page gallery renders through the webview at both densities; browsing input stays Rust-side; observation emitted post-paint.
- **Priority**: P2 (US3) · **Prompt**: `tasks/WP04-gallery-webview.md` (~300 lines)
- **Subtasks**: T014 T015 T016
- **Independent test**: `make demo-live-component-library` browses all pages; coverage assertion fails on a missing specimen.
- **Dependencies**: WP01.
- **Risks**: gallery keeps its declared no-audio/no-MIDI scope; no second styling source (C-006, NFR-004).

T014 Gallery page documents/layouts through generated tokens (WP04)
T015 Gallery scene re-hosted on the webview surface (WP04)
T016 Gallery coverage assertions + make target (WP04)

### WP05 — Headless acceptance retargeting and input witness

- **Goal**: the five egui-path test contracts are re-proven against the webview path while both shells exist: new `tests/shell_event_dispatch.rs`, retargeted shell/view-model/component tests, and the automated key-injection witness.
- **Priority**: P1 (SC-005) · **Prompt**: `tasks/WP05-test-retargeting.md` (~460 lines)
- **Subtasks**: T017 T018 T019 T020 T021
- **Independent test**: each named target emits its `CREST_ACCEPTANCE <name> passed` marker; witness drives the full WindowKey vocabulary through the production translator.
- **Dependencies**: WP01, WP02.
- **Risks**: `tests/eframe_context.rs` stays in place (deleted by WP07); the renamed validation `shell_event_dispatch` only goes green when T017 lands — sequence acceptance accordingly.

T017 tests/shell_event_dispatch.rs headless contract (WP05)
T018 Retarget tests/graphical_application_shell.rs (WP05)
T019 Retarget tests/semantic_graphical_view_model.rs (WP05)
T020 Retarget component vocabulary/composition tests (WP05)
T021 Automated key-injection witness (WP05)

### WP06 — Hardware evidence: scenes, RT A/B, soak

- **Goal**: the C-007 evidence wall — four scene runs on the physical rig through the webview shell, the same-workload RT A/B while both shells exist, the 300 s soak, all committed under `kitty-specs/webview-shell-cutover-01KZAC7Q/evidence/`.
- **Priority**: P1 (US2, gate) · **Prompt**: `tasks/WP06-hardware-evidence.md` (~360 lines)
- **Subtasks**: T022 T023 T024 T025
- **Independent test**: every evidence log shows exit 0, clean teardown, zero `audio_uninterrupted=false`; identity comparison shows 0 modified / 0 removed vs frozen baselines.
- **Dependencies**: WP03 (and transitively WP01, WP02).
- **Risks**: run on this machine with the real window and physical audio — no headless substitute counts; deterministic twins must be green before burning rig time.

T022 RT A/B same-workload measurement (WP06)
T023 Four scene hardware runs, evidence committed (WP06)
T024 300 s soak recorded (WP06)
T025 Evidence summary + identity comparison (WP06)

### WP07 — Sole-shell flip, egui deletion, records

- **Goal**: the cutover lands — vocabulary declarations relocate out of `src/shell/visual/`, fader px values join the authored tokens, the composition root goes webview-only, the egui layer/adapter/deps/test are deleted, and the pivot is recorded in DESIGN.md.
- **Priority**: P1 (US1/US4 completion) · **Prompt**: `tasks/WP07-flip-and-deletion.md` (~480 lines)
- **Subtasks**: T026 T027 T028 T029 T030 T031
- **Independent test**: forced webview init failure exits typed with no alternate window; zero egui/eframe references outside history; full suite + clippy + fmt green; net line-count recorded.
- **Dependencies**: WP04, WP05, WP06 (hard gate: evidence commits precede the deletion commit — C-007).
- **Risks**: the no-literal guard and token-freshness proof must survive the relocation; this WP is where a silent fallback would sneak in — the typed-failure test is the tripwire.

T026 Relocate vocabulary declarations out of visual/ (WP07)
T027 Fader px specimens into authored vocabulary (WP07)
T028 Composition root webview-only (WP07)
T029 Delete visual layer, eframe adapter, egui deps (WP07)
T030 DESIGN.md/ROADMAP records + probe decision (WP07)
T031 Full-suite sweep + line-count record (WP07)

## Dependency Graph

```
WP01 ──┬─→ WP03 ──→ WP06 ──┐
WP02 ──┤                   ├─→ WP07
       ├─→ WP05 ───────────┤
WP01 ──┴─→ WP04 ───────────┘
```

## MVP Scope

WP01 + WP02 + WP03: the whole instrument playable and scene-provable through
the webview shell locally — everything after is proof, parity, and deletion.
