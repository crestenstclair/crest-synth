# Implementation Plan: Shell Hygiene Sweep

**Branch**: `feat/shell-hygiene` | **Date**: 2026-08-06 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `kitty-specs/shell-hygiene-01KZD0KR/spec.md`

## Summary

Discharge six deferred findings from two post-merge mission reviews, changing no product behavior. (1) **RISK-3**: `close_window_once_with_retry` (`src/shell/webview/window.rs`) records a typed `WindowClose` error when both close attempts fail and then returns normally, but nothing else drives termination — the recorded fatal error never reaches the operator because `run_return` never yields. Add the missing exit edge; leave the retry, the typed error, and the first-error latch untouched. (2) **RISK-4**: `forward_painted_ack` (`src/shell/webview/projection_channel.rs:388-405`) cannot validate a superseded-late ack because the document was already dropped from the `MAX_IN_FLIGHT_DOCUMENTS = 8` `VecDeque`; retain retired identities in a bounded companion store and apply the same `ACK_IDENTITY_FIELDS` comparison. (3) **RISK-5**: delete four dead items whose caller search returns nothing — `await_qualifying`/`FrameAwaitError`, the `step_index()` accessor (not the field), the `ControlIntent`/`ControlRequest`/`CompositionIntent` family, and `CURSOR_GLYPH`. (4) **OBS-1**: narrate the gallery's policy-free serving as deliberate; the gallery is retained by operator decision. (5) **SMELL-1**: extend the purity-needle scan to every page source it means to bind. (6) **DRIFT-3**: discharge documentation residue.

## Technical Context

**Language/Version**: Rust (edition per workspace `Cargo.toml`, toolchain pinned by `rust-toolchain`/CI); vanilla ES5-style JavaScript for the page and gallery assets (no build step, no framework)
**Primary Dependencies**: Tauri v2 (WKWebView on macOS) — window lifecycle, `crest://` custom protocol, event transport; serde/serde_json for the serialized projection
**Storage**: N/A — no persistence surface in this mission
**Testing**: Cargo integration-test targets invoked per crest-spec validations — `tests/webview_projection_shell.rs`, `tests/component_vocabulary.rs`, `tests/component_composition.rs`; unit tests in the touched modules; live check via `make demo-live`
**Target Platform**: macOS desktop (WKWebView); authored viewports 1920x1080 and 1280x800
**Project Type**: Single Rust crate with embedded webview page assets (`webview-page/`)
**Performance Goals**: Unchanged — reducer state change visible within 50 ms p95 under the paced live demo workload (`requirement.serialized_projection_transport`); this mission must not move it
**Constraints**: No reducer, real-time, projection-schema, or product-surface change (C-001); every retirement authored in the crest-spec before deletion (C-002); the gallery is retained, not deleted or converted (C-003); no Phase 5 product work (C-004)
**Scale/Scope**: 6 findings across 6 disjoint surfaces, ~8 files, net code reduction expected

## Charter Check

*GATE: passed.* Compact charter (software-dev-default template set; git + spec-kitty tooling). Relevant directives honored: DIRECTIVE_001 (architectural integrity — every change stays inside the shell/testing/test surfaces the declared assets already own; no new public API), DIRECTIVE_003 (decisions recorded in `research.md` D1–D7), RECONCILE_CHANGE_SCOPE_TENSIONS (the gallery retirement was drafted, surfaced, and deliberately reversed by the operator; the reversal is recorded in the decision ledger and pinned by C-003 rather than silently dropped). No conflicts found; no Complexity Tracking entries needed.

## Crest-Spec Derivation

Authored in the `/spec-kitty.crest-spec` phase (commit `7c7f1cf`), `crest_spec_impact: structural` — one retirement, no additions:

- **Changed declarations**:
  - `requirement.component_state_ownership_boundary` — the Rust-side typed-intent return retires; the passivity boundary it protected (no ownership, no caching, no reach into `AppState`, no input-to-action conversion) is now held directly by this requirement.
  - `proof/invariants.yaml` component-passivity invariant — same retirement, same surviving boundary, rationale records why.
  - `requirement.configurable_control_family` — loses the "returning typed semantic intent" clause; controls present rather than act.
  - `validation.webview_projection_shell` — description deepened to name the two error-path proofs this mission adds (recorded typed failure still surfaces when both closes fail; superseded-late acks are identity-validated).
- **Retired resources**: none by canonical ID — the control-intent family was declared in prose, not as named resources (see `research.md` D4). No resource is added.
- **Assets → files**:
  - `WebviewShellModules` → `src/shell/webview/window.rs` (RISK-3 exit edge), `src/shell/webview/projection_channel.rs` (RISK-4 retired-identity validation), `src/shell/webview/frame_stream.rs` (dead `await_qualifying`/`FrameAwaitError`), `src/shell/webview/mod.rs` (stale doc reference).
  - `ShellContextModules` → `src/shell/component_vocabulary.rs` (control-intent family, `CURSOR_GLYPH`).
  - `TestingContextModules` → `src/testing/live_demo_runner.rs` (dead accessor), `src/testing/component_gallery_scene.rs` (OBS-1 narration).
  - `WebviewProjectionShellAcceptanceTests` → `tests/webview_projection_shell.rs` (RISK-3 and RISK-4 falsifiable proofs).
  - `ComponentCompositionAcceptanceTests` → `tests/component_composition.rs` (purity-needle scan extension; corrected assertion message).
- **Validations/witnesses covering the change**: `validation.webview_projection_shell` (deepened assertions, unchanged command surface), `validation.component_composition`, `validation.component_vocabulary`, `validation.graphical_application_shell` — rolled up by `evidence.graphical_application_shell_contract` and `evidence.component_vocabulary_contract`.
- `data-model.md` / `contracts/`: **not produced** (forbidden — a crest-spec exists).

## Project Structure

### Documentation (this mission)

```
kitty-specs/shell-hygiene-01KZD0KR/
├── plan.md              # This file
├── research.md          # Phase 0 output — decisions D1–D7 with rationale
├── quickstart.md        # Phase 1 output — how to run the affected proofs
└── spec.md              # Mission specification
```

### Source Code (repository root)

```
src/shell/webview/
├── window.rs                # close_window_once_with_retry: add the missing exit
│                            #   edge so a recorded typed error still surfaces when
│                            #   both closes fail (RISK-3)
├── projection_channel.rs    # forward_painted_ack l.388-405: bounded retired-identity
│                            #   store + ACK_IDENTITY_FIELDS validation in the
│                            #   superseded-late window (RISK-4)
├── frame_stream.rs          # delete await_qualifying + FrameAwaitError (RISK-5)
└── mod.rs                   # drop the doc reference to the deleted error
src/shell/
└── component_vocabulary.rs  # delete ControlIntent/ControlRequest/CompositionIntent
                             #   (l.348-585) and CURSOR_GLYPH (l.812) (RISK-5)
src/testing/
├── live_demo_runner.rs      # delete the step_index() accessor at l.1753 — the
│                            #   field stays, it is used throughout the runner
└── component_gallery_scene.rs  # narrate the policy-free protocol handler (OBS-1)
tests/
├── webview_projection_shell.rs  # forced double-close-failure proof; corrupted
│                                #   superseded-late ack proof; negative controls
└── component_composition.rs     # purity-needle scan extension (SMELL-1); correct
                                 #   the assertion message naming ControlIntent
```

**Structure Decision**: Single-crate layout unchanged. No new modules, no new test targets, no new public API — the crest-spec validation command surfaces stay identical and only their assertions deepen. Every touched path is inside a surface the declared assets already own.

## Implementation Concern Map

> Implementation concerns are NOT work packages. `/spec-kitty.tasks` translates these into executable WPs.

### IC-01 — Window close-failure exit edge (RISK-3)

- **Purpose**: A recorded typed fatal error must reach the operator even when the window refuses to close twice.
- **Relevant requirements**: FR-001, FR-002; NFR-001; `requirement.webview_projection_shell`
- **Affected surfaces**: `src/shell/webview/window.rs` (`close_window_once_with_retry` and its callers), `tests/webview_projection_shell.rs`
- **Sequencing/depends-on**: none
- **Risks**: The fix must not alter the single-close-failure retry path, the `get_or_insert` latch (a prior `PageRenderFailed` must still win over the later `WindowClose`), or the ordinary `Destroyed` teardown. The forced-double-failure proof needs a deterministic way to make both closes fail without weakening the production path — prefer a test seam that compiles out of release, mirroring the existing `cfg(debug_assertions)` override precedent, over a runtime flag.

### IC-02 — Superseded-late ack identity validation (RISK-4)

- **Purpose**: Make "verbatim or typed-rejected" true in every window, including after the document is dropped.
- **Relevant requirements**: FR-003; `requirement.serialized_projection_transport`
- **Affected surfaces**: `src/shell/webview/projection_channel.rs` (the `in_flight` eviction at l.343-346, the ack-consumption `drain` at l.456, and the superseded-late branch at l.388-405), `tests/webview_projection_shell.rs`
- **Sequencing/depends-on**: none
- **Risks**: Both retirement paths (capacity eviction and ack consumption) must feed the retained store or the validation is silently partial. A well-formed superseded-late ack must still be accepted — the negative control is a full live run recording zero rejections. An ack older than the retained window keeps today's lost-frame behavior and must not become a false rejection. The store is shell-side only; it must not appear anywhere near the real-time callback.

### IC-03 — Dead code removal (RISK-5)

- **Purpose**: No public item without a caller; no declaration without an implementation.
- **Relevant requirements**: FR-004, FR-005; C-002
- **Affected surfaces**: `src/shell/webview/frame_stream.rs`, `src/shell/webview/mod.rs`, `src/shell/component_vocabulary.rs`, `src/testing/live_demo_runner.rs`, `tests/component_composition.rs` (assertion message only)
- **Sequencing/depends-on**: the crest-spec retirement (already committed at `7c7f1cf`) precedes every deletion, per C-002
- **Risks**: `step_index` is a field AND an accessor — delete only the accessor. Deleting the control-intent family must not disturb the surviving component vocabulary (tokens, states, controls, compositions) that both the production page and the gallery prove. `component_vocabulary.rs`'s module doc references the retiring types and must be corrected with them.

### IC-04 — Gallery serving narration (OBS-1)

- **Purpose**: Record the gallery's policy-free serving as deliberate so it is never mistaken for the drift that hid a shipped defect.
- **Relevant requirements**: FR-006; C-003
- **Affected surfaces**: `src/testing/component_gallery_scene.rs` (the `register_uri_scheme_protocol` handler)
- **Sequencing/depends-on**: none
- **Risks**: Narration only — no gallery source, page asset, CLI option, or make target may be deleted, converted, or reduced (C-003). The narration must name the production asset table as the reason the shape is unreachable, so a future reader can tell when that stops being true.

### IC-05 — Purity-needle scan coverage (SMELL-1)

- **Purpose**: Every page source the scan means to bind is inside its scanned set.
- **Relevant requirements**: FR-007
- **Affected surfaces**: `tests/component_composition.rs` (the purity-needle loop at l.1790-1805)
- **Sequencing/depends-on**: none
- **Risks**: A legitimate gallery construct may fire a needle; the fix is at the source or a declared, narrated exemption mirroring the existing precedent — never a silent carve-out. Falsifiability must be demonstrated per newly covered source.

### IC-06 — Documentation residue (DRIFT-3)

- **Purpose**: The record says what happened.
- **Relevant requirements**: FR-008
- **Affected surfaces**: the affected missions' planning documents under `kitty-specs/`
- **Sequencing/depends-on**: none
- **Risks**: Amend completed missions' records honestly — status fields and terminology only. Never rewrite a closed gate's evidence or verdict to read better in hindsight.
