# Mission Specification: Webview Shell Foundation

**Mission**: `webview-shell-foundation-01KZ9DN7`
**Created**: 2026-08-05
**Status**: Draft
**Target branch**: `feat/webview-shell-foundation` (merges to `main`, PR-bound)

## Why (crest-spec grounding)

This mission serves `goal.use_graphical_shell` and
`goal.build_from_component_vocabulary`: the player sees the authored visual
design in the running instrument. It changes **how** the shell is drawn, not
what the shell is. The declarations it renders differently — and must not
weaken — are `requirement.immutable_graphical_shell_projection`,
`requirement.authored_shell_composition`,
`requirement.responsive_shell_blockout`, and
`capability.semantic_graphical_view_model`; the adapter seam it adds a peer
into is the one `adapter.EframeGraphicalWindow` occupies today.

`requirement.selected_egui_stack` remains in force for this mission: egui
stays the running default. Its deliberate retirement belongs to the successor
mission that deletes the egui visual layer. The structure this mission adds —
a webview projection adapter and its transport — is not yet declared;
authoring it is the `/spec-kitty.crest-spec` phase that follows this spec.

**Evidence motivating the pivot**: the hand-painted egui visual layer
(`src/shell/visual/`, 17,033 lines) has cost far more effort than the design
warrants, most of it manual layout arithmetic. The spike at
`spike/webview-mixer` (Op `01KZ9CN8E0YEMQEZQG8Q7VEESM`, commit `75a6f7c`)
rendered the complete authored MIXER screen from the production
`SemanticGraphicalViewModel` JSON in 244 lines of HTML/CSS; the sixteen-column
layout that required a 684-line density policy became one CSS grid rule.

## User Scenarios & Testing

### User Story 1 — Play the MIXER through the webview shell (Priority: P1)

The player launches Crest Synth with the webview shell selected. A native
window opens whose content is a webview rendering the MIXER context from the
canonical semantic view model: context bar, identity header, sixteen track
columns with fader/hex/pan state, focused-column keyline and halo, persistent
Inspector, and both hint rows, all in the authored tokens and typeface. The
player navigates tracks and controls with the same physical keys as today,
adjusts level/pan/mute/solo, and hears the change; the on-screen column and
Inspector update from the reducer's projection. Meters move while audio plays.
Closing the window shuts the application down through the same owned-shutdown
path as the egui shell.

**Why this priority**: this is the mission — everything else supports it.

**Independent Test**: run the paced live demo through the webview shell;
edits are audible, the screen tracks the reducer, shutdown is clean.

### User Story 2 — Startup failure is explicit (Priority: P2)

The webview fails to initialize (missing system webview runtime, page load
failure). The application reports a typed startup failure and exits; it does
not silently fall back to the egui shell or to a blank window. Selecting the
egui shell (the default) is explicit, not a fallback.

**Why this priority**: the no-silent-fallback boundary is a standing project
invariant; violating it here would corrupt the shell-selection contract.

**Independent Test**: point the shell at an unloadable page; assert a typed
error and process exit, not a window.

### Testing posture

The projection side is testable without a window: the view-model JSON the
webview consumes is the same serialized structure the existing projection
tests already exercise, and the page's rendering of a given JSON document is
deterministic. Behavioral proof reuses the production reducer and projection
path per `requirement.graphical_shell_behavioral_proof`; the webview page is
proven against recorded view-model documents, and the full window is proven by
the paced live demo path.

## Domain Language

| Canonical term | Meaning | Avoid |
|---|---|---|
| projection surface | The webview page: renders view-model JSON, emits nothing but rendering | "frontend", "web app" |
| semantic transport | The IPC channel carrying serialized view models out and (in later missions) semantic actions in | "bridge", "API" |
| shell selection | The explicit launch-time choice between egui and webview shells | "fallback", "toggle" |

## Functional Requirements

| ID | Requirement | Status |
|---|---|---|
| FR-001 | The application can launch with a Tauri v2 window whose webview renders the MIXER context of the canonical `SemanticGraphicalViewModel`, selected explicitly at launch; egui remains the default shell. | Proposed |
| FR-002 | The webview page renders the authored MIXER composition — context bar, identity header, sixteen-column strip bank, focused-column emphasis, Inspector, hint rows — resolving every color, type style, spacing step, and geometry value from the authored token vocabulary (single source: the Rust token module; the page's token table is generated from it, not hand-copied). | Proposed |
| FR-003 | Physical keyboard input is captured on the Rust side by the existing input translators; the webview captures no input and owns no navigation, focus, or value state. The full existing MIXER key vocabulary (navigate, fine/coarse adjust, toggle, context switch) works unchanged. | Proposed |
| FR-004 | Every projection the reducer publishes reaches the webview as the same serialized view model the projector emits today — one schema, no webview-specific variant, no field the page invents. | Proposed |
| FR-005 | Decimated meter observations reach the webview and animate the MIXER meters while audio renders; meter traffic is latest-value, polled outside the real-time callback, and its loss degrades display only, never state. | Proposed |
| FR-006 | Window close through the webview shell drives the same owned shutdown as the egui shell: stream release, worker completion, graph ownership collection, normal exit. | Proposed |
| FR-007 | Webview initialization failure is a typed, reported startup error ending the process; no silent fallback to another shell or degraded window. | Proposed |

## Non-Functional Requirements

| ID | Requirement | Status |
|---|---|---|
| NFR-001 | Projection-to-paint latency: a reducer state change is visible in the webview within 50 ms at p95 under the paced live demo workload. | Proposed |
| NFR-002 | Meter animation sustains 30 Hz update rate without accumulating queue depth over a 5-minute render. | Proposed |
| NFR-003 | The real-time audio callback's measured bounds (existing RT health counters) are unchanged from the egui shell baseline under the same workload — the webview adds zero work to the callback. | Proposed |
| NFR-004 | The webview MIXER screen passes the same layout verification viewports as the egui shell: 1920×1080 and the compact 1280×800 blockout, preserving bands, minimum targets, and hierarchy per `requirement.responsive_shell_blockout`. | Proposed |

## Constraints

| ID | Constraint | Status |
|---|---|---|
| C-001 | The boundaries in `CLAUDE.md`/`DESIGN.md` hold untouched: physical input → semantic action → `AppState::apply` → projections; RT callback with bounded preallocated work; separate transports for events, snapshots, and structural changes. | Accepted |
| C-002 | No UI-owned copy of audio or application state in the page; the page is stateless between view-model documents apart from presentation-only concerns (e.g. animation interpolation). | Accepted |
| C-003 | `requirement.selected_egui_stack` stays satisfied: the egui shell remains the default and its proofs keep passing throughout this mission. | Accepted |
| C-004 | Scope excludes: PATCH context, modals, sample browser, action IPC from page to reducer beyond what meters/shutdown need, and any egui code deletion. | Accepted |
| C-005 | New structure (webview adapter, semantic transport, page asset) must be declared in the crest-spec before planning; no `data-model.md`/`contracts/` artifacts. | Accepted |

## Success Criteria

1. A player at the keyboard cannot distinguish MIXER behavior between the two shells: same keys, same navigation, same audible result, same displayed values.
2. Side-by-side capture of the webview MIXER and the authored design shows the authored composition, tokens, and typeface (the spike's comparison, now from the live window).
3. The paced live demo completes through the webview shell: edits audible, meters moving, clean shutdown, normal exit.
4. All existing proofs pass unchanged with the egui shell still default.

## Key Entities

- **Serialized semantic view model** — the JSON form of `SemanticGraphicalViewModel`; already exists, already `Serialize`, is the entire page-facing contract.
- **Webview shell adapter** — the Tauri window + transport composition, a peer of the eframe window adapter behind the same projection seam.
- **Projection page asset** — the HTML/CSS/JS document rendering MIXER; its token table is generated from the Rust vocabulary.

## Assumptions

- Tauri v2 is compatible with the existing cpal stream and worker ownership model in one process (the spike did not test this; plan phase must burn it down first).
- The system webview (WKWebView on macOS) renders the Azeret Mono faces and CSS grid identically enough to Chrome that the spike's parity holds; NFR-004's viewport checks verify.
- Meter decimation at 30 Hz over Tauri IPC is well within budget (spike measured nothing; NFR-002 makes it falsifiable).

## Deferred decisions

None. The three discovery decisions (scope, stack, input ownership) are
resolved in the mission decision ledger.
