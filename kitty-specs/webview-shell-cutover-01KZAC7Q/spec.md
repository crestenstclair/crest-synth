# Mission Specification: Webview Shell Cutover

**Mission**: `webview-shell-cutover-01KZAC7Q`
**Created**: 2026-08-06
**Status**: Complete — accepted 2026-08-06 (`a38abf7`) and merged; deterministic acceptance 25/25 pass. The post-merge mission review returned **FAIL** on RISK-1 and RISK-2 (with DRIFT-1/DRIFT-2 as enablers); those four findings were the entire scope of the follow-on `webview-render-fidelity-hardening-01KZCEF8` mission, which is merged.
**Target branch**: `feat/webview-shell-cutover` (merges to `main`, PR-bound)
**Input**: Operator decision (2026-08-05): "absolutely moving all current work over to webview, I want to get rid of egui cruft before we continue implementation." Recorded in `ROADMAP.md` as the "Webview shell cutover gate", which blocks Phase 5.

## Why (crest-spec grounding)

This mission serves `goal.use_graphical_shell` and
`goal.build_from_component_vocabulary` by completing the renderer pivot that
`webview-shell-foundation-01KZ9DN7` proved: one shell, the webview, rendering
every shipped surface from the canonical model.

Declarations this mission renders differently and must not weaken:
`requirement.immutable_graphical_shell_projection`,
`requirement.authored_shell_composition`,
`requirement.responsive_shell_blockout`,
`capability.semantic_graphical_view_model`,
`capability.graphical_application_shell`.

Declarations this mission changes (authored in the `/spec-kitty.crest-spec`
phase that follows this spec, before planning):

- `requirement.selected_egui_stack` — its own text reserves retirement as "a
  deliberate future declaration, not a side effect". This mission is that
  declaration: the webview shell becomes the default production shell and the
  egui stack is retired.
- `adapter.TauriWebviewWindow` — its rule "render this mission's MIXER context
  only, with PATCH and modals arriving in a successor mission" names this
  mission; the rule is replaced by full-surface rendering.
- `adapter.EframeGraphicalWindow` — retired.
- `validation.component_vocabulary` and `validation.component_composition` —
  currently prove the egui render path; each is re-declared against the
  webview path or deliberately retired, never silently dropped.
- New structure not yet declared: the painted-ack → `ShellFrameObservation`
  forwarding owner, webview-hosted demo-scene rendering, and the gallery's
  webview surface.

`DESIGN.md` records the pivot in the same authoring pass. The Phase 4
token/type/spacing/state vocabulary survives as the authored single source
behind the generated token table (`adapter.TauriWebviewWindow` already
declares generation as part of the build).

## User Scenarios & Testing

### User Story 1 - Play the whole instrument through the webview shell (Priority: P1)

The player launches Crest Synth. The only shell is the webview: PATCH and
MIXER both render from the canonical semantic view model in the authored
tokens. Every journey that works today works here — context switch, Patch
selection (Q/E), focus traversal, engine selection, effect-slot occupancy,
ADSR and scalar edits, track level/pan/mute/solo, sends and returns — driven
by the same physical keys, with the same audible consequence, meters moving
while audio plays. Closing the window shuts down through the owned-shutdown
path.

**Why this priority**: this is the cutover — one renderer for everything the
product already does.

**Independent Test**: run the application, drive every currently supported
journey by keyboard in both contexts, hear each audible edit, close cleanly.

**Acceptance Scenarios**:

1. **Given** the application is running, **When** the player switches to PATCH and navigates to any focusable control, **Then** the webview page shows the focused control from the reducer's projection and the same generation/stateHash identity the reducer published.
2. **Given** a focused editable parameter, **When** the player adjusts it, **Then** the change is audible and the on-screen value tracks canonical state — no page-local state.
3. **Given** the player presses Q/E in PATCH Navigate mode, **When** the focused Patch changes, **Then** focus recovers against the destination Patch's descriptor schema, as `SelectPatch` already guarantees.
4. **Given** the window is closed, **When** shutdown runs, **Then** stream release, worker completion, graph ownership collection, and normal exit all occur — same contract as today.

---

### User Story 2 - Retained live scenes replay through the webview (Priority: P1)

The maintainer runs each retained `make demo-live-<scene>` target
(`effects-and-buses`, `sixteen-track-mixer-routing`, `semantic-view-model`,
`graphical-shell`). Each scene runs through the webview shell on the physical
rig — real window, physical audio, real MIDI fixture — and completes its full
teardown contract. Evidence is refreshed and committed before the egui window
is deleted.

**Why this priority**: retained evidence survives the cutover or the cutover
does not complete (ROADMAP gate term). These scenes are the product's
regression proof.

**Independent Test**: each target exits 0 on hardware with its committed log
showing the scene's declared checkpoints, zero `audio_uninterrupted=false`,
and clean teardown.

**Acceptance Scenarios**:

1. **Given** a retained scene, **When** it runs under the webview shell, **Then** every frozen checkpoint identity is preserved byte-identically and in order; new identities are pure insertions (add-only contract).
2. **Given** the live report, **When** it credits rendered frames, **Then** it counts qualifying webview `ShellFrameObservation`s — the painted-ack forwarding has an owner and the observation is emitted only after painting the supplied projection.
3. **Given** all scenes pass, **When** the egui layer is deleted, **Then** the scenes still pass — the deletion commit is after the evidence commit.

---

### User Story 3 - Browse the component gallery in the webview (Priority: P2)

The maintainer runs the gallery target. The pages covering the vocabulary,
controls, compositions, and component states render through the webview at
both authored viewport densities, browsable by the same keys as today
(digits, `[`/`]`), and closing the window finishes.

**Why this priority**: operator decision (2026-08-06) — the gallery is rebuilt
in the webview, not retired; Phase 4's browsable-evidence surface survives.

**Independent Test**: open the gallery, step through every page at both
densities, close cleanly.

**Acceptance Scenarios**:

1. **Given** the gallery is open, **When** the maintainer steps through pages, **Then** every vocabulary, control, composition, and state page renders through the webview from the same authored token source as the product shell.

---

### User Story 4 - Startup failure stays explicit with no fallback left (Priority: P2)

The webview fails to initialize (missing system webview runtime, page load
failure). The application reports a typed startup failure and exits. With the
egui shell deleted there is nothing to fall back to — the mission proves no
silent fallback path was introduced by the deletion.

**Why this priority**: the no-silent-fallback boundary is a project
invariant; deletion is where a hidden fallback would sneak in.

**Independent Test**: force a page-load failure; observe the typed error and
nonzero exit; grep the tree for any egui code path.

**Acceptance Scenarios**:

1. **Given** a forced webview initialization failure, **When** the application starts, **Then** it exits with the typed startup error — no blank window, no alternate renderer.

### Edge Cases

- Page render exception after a successful load: surfaced as a typed error, not a frozen window (foundation review open item).
- Meter-frame decimation under load: lost frames degrade display only; scalar and structural channels are unaffected.
- Focus recovery when a schema change alters visible placement mid-scene: existing deterministic focus-recovery contract must hold in the webview shell.
- `window.close()` failure during teardown: retry-or-surface, never ignore (foundation review RISK-2).
- A demo scene that outpaces painted acks: checkpoints correlating visible projection must block on qualifying frames, not on wall-clock sleeps.

## Domain Language

- **Webview shell** — the Tauri/wry projection surface behind the `AppWindow` port. Not "the web app", not "the frontend": it is a projection surface that captures no input and holds no state.
- **Cutover** — the webview shell becoming the only shell. Distinct from the foundation mission, which added it as a peer.
- **Retirement** — the deliberate crest-spec declaration removing the egui stack. Never a side effect of deletion.
- Avoid "migration" for the demo scenes: scenes are not rewritten; the shell under them changes.

## Requirements

### Functional Requirements

| ID | Title | User Story | Priority | Status |
|----|-------|------------|----------|--------|
| FR-001 | PATCH context renders through the webview: every shipped PATCH surface (strip, identity/header, envelope, engine and effect-slot rows, Utility region, footer, hint rows) renders from the canonical serialized view model | US1 | High | Accepted |
| FR-002 | The webview shell is the default and only shell; launch-time egui selection is removed with the adapter | US1, US4 | High | Accepted |
| FR-003 | Every retained `make demo-live-<scene>` target runs through the webview shell with refreshed committed hardware evidence, before egui deletion lands | US2 | High | Accepted |
| FR-004 | Painted-ack → `ShellFrameObservation` forwarding is owned and live reports credit only qualifying webview frames | US2 | High | Accepted |
| FR-005 | The component gallery renders through the webview: all pages, both authored densities, keyboard-browsable | US3 | Medium | Accepted |
| FR-006 | The egui visual layer (`src/shell/visual/`), the eframe window adapter, and the `eframe`/`egui_extras` dependencies are deleted | US1, US4 | High | Accepted |
| FR-007 | The crest-spec authors the retirement first (requirement, adapters, validations) and `DESIGN.md` records the pivot | — | High | Accepted |
| FR-008 | An automated key-injection witness drives the full key vocabulary through the production translator into the running webview shell (foundation FR-003 successor item) | US1 | Medium | Accepted |

### Non-Functional Requirements

| ID | Title | Requirement | Category | Priority | Status |
|----|-------|-------------|----------|----------|--------|
| NFR-001 | Real-time neutrality | Same-workload RT A/B measurement (egui vs webview, taken before deletion) shows the webview shell adds no real-time callback work; re-hosted scenes record zero `audio_uninterrupted=false` checkpoints | Performance | High | Accepted |
| NFR-002 | Sustained stability | One recorded 300 s soak of the webview shell under the live workload completes with no leak growth trend, no dropped-event records, and clean teardown. Leak bound (quantified after acceptance, discharging analysis finding A1 — it was not a pre-declared bar at the accept gate): the measured field is process RSS sampled across the soak, and the bound is no monotonic growth across sampling windows. **The whole recorded series** (`evidence/soak-300s-rss.samples.log` — all 33 samples, 10 s apart, 07:13:09–07:18:30Z): it opens at 507840 KiB, rises once by +99552 KiB to its peak of **607392 KiB** at sample 2 (07:13:19Z), holds ≈607 MiB for ~60 s (samples 2–7), then drops to 155840 KiB at sample 8 (07:14:19Z) and declines to a 103–107 MiB plateau, ending at 94160 KiB (minimum 93568 KiB at sample 32) — net −413680 KiB first sample to last, the peak being 6.45× the final sample. **Stated plainly: the bound as declared is not met across the full run — the first sampling window grows.** It is met from the peak (sample 2) onward. That opening step is the only material rise in the series and is never resumed: every later positive delta is ≤592 KiB (≤0.6%), and the longest run of consecutive rising windows anywhere is two windows totalling +32 KiB (0.03%), which is sampling jitter and not a trend. The shape is therefore a large early allocation followed by decline to a plateau, not accumulation. Correction of record: this row previously reported "the recorded run declined 107728 → 103904 KiB to a plateau", which was samples 13–31 of 33 — a post-warm-up window reported as the whole run, disclosing neither the exclusion nor the 6× peak (mission shell-hygiene-01KZD0KR post-merge review, RISK-1). Cadence and teardown are unchanged: 29.43 Hz sustained, max gap 44.3 ms, 0 lost records, page-side 8830/8830 lossless (`evidence/soak-300s.log`) | Reliability | High | Accepted |
| NFR-003 | Page hardening | A restrictive CSP is set; `CREST_WEBVIEW_PAGE` is compiled out of release builds (`cfg(debug_assertions)`) | Security | Medium | Accepted |
| NFR-004 | Token single source | Zero hand-copied style values in the page: every color, type style, spacing step, and geometry value resolves from the token table generated from the authored Rust vocabulary as part of the build; includes the six fader-specimen px values noted by the foundation review | Maintainability | Medium | Accepted |

### Constraints

| ID | Title | Constraint | Category | Priority | Status |
|----|-------|------------|----------|----------|--------|
| C-001 | Input stays Rust-side | All key press/release/focus-loss normalization goes through the production `KeyboardInputTranslator`; the webview DOM registers no input handlers | Technical | High | Accepted |
| C-002 | One schema | The page consumes the serde serialization of `SemanticGraphicalViewModel` and decimated latest-value meter frames; no webview-specific variant of the model exists | Technical | High | Accepted |
| C-003 | Boundaries unchanged | One-way loop, immutable projections, owned shutdown, hard real-time callback contract, and no-silent-fallback are preserved exactly | Technical | High | Accepted |
| C-004 | Add-only scene identity | Frozen checkpoint baselines (e.g. `FROZEN_TOPOLOGY_IDENTITY_BASELINE`) stay byte-identical and in order; webview-era identities are pure insertions | Proof | High | Accepted |
| C-005 | Derivation discipline | Crest-spec authored before plan; no `data-model.md`, no `contracts/` | Process | High | Accepted |
| C-006 | Gallery disposition | Operator decision 2026-08-06: rebuild the gallery in the webview (not retired, not deferred) | Product | Medium | Accepted |
| C-007 | Ordering | Evidence-refresh commits land before the egui deletion commit; the deletion is the last structural change, so every proof re-run brackets it | Process | High | Accepted |

### Key Entities

- **SemanticGraphicalViewModel**: the one canonical serialized projection the page renders; unchanged by this mission.
- **ShellFrameObservation**: the adapter-boundary frame observation; this mission gives its webview forwarding an owner so live reports credit real painted frames.
- **Token table**: build-generated projection of the authored Rust vocabulary into page styling; the single styling source after cutover.

## Success Criteria

### Measurable Outcomes

- **SC-001**: Every currently supported player journey completes in the webview shell in both contexts with audible consequence — zero journeys lost relative to the egui shell.
- **SC-002**: 4/4 retained live scenes pass on hardware through the webview shell with clean teardown, zero audio interruptions, and all frozen checkpoint identities preserved add-only.
- **SC-003**: Zero references to the retired UI stack remain in production code and dependency manifests after deletion.
- **SC-004**: Net shell code reduction of at least 10,000 lines against the ~17k-line hand-painted visual layer.
- **SC-005**: All declared deterministic acceptance checks are green at mission end, including the re-declared (or deliberately retired) component proofs — no proof silently dropped.
- **SC-006**: The maintainer can browse the complete component gallery at both densities through the webview shell.

## Assumptions

- Hardware runs (scene evidence, RT A/B, soak) are performed by the operator on the physical rig, as for every prior gate; deterministic twins carry the machine-checkable half.
- The Tauri v2 / wry stack vendored by the foundation mission is the webview runtime; no runtime change is in scope.
- The ten designed-but-undriven structures, the two unprojected control kinds, and the mixer-meter bank path recorded at Phase 4 completion remain Phase 5 inputs — this mission changes the renderer, not the projection's data coverage.
- Display-fidelity cleanup items from Phase 4 (label trims, inset overrun, band heights) transfer to the webview vocabulary but their completion is not gated here unless the cutover itself resolves them.

## Out of Scope

- Any new product surface (Phase 5 Patch editor functionality, modals, choice surfaces).
- Closing LIMIT-1's remaining half (the multi-instrument `demo-live-patch-editor` scene) — that stays Phase 5's entry condition.
- Modulation, plugin hosting, preset/library management (roadmap deferrals).
- Changes to audio engines, effects, routing, or the reducer's semantic vocabulary.
