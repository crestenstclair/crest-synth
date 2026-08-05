---
work_package_id: WP06
title: Acceptance and live proof
dependencies:
- WP05
requirement_refs:
- FR-001
- FR-002
- FR-003
- FR-004
- FR-005
- FR-006
- FR-007
planning_base_branch: feat/webview-shell-foundation
merge_target_branch: feat/webview-shell-foundation
branch_strategy: lane worktree computed by finalize-tasks; merges into feat/webview-shell-foundation
subtasks:
- T022
- T023
- T024
- T025
- T026
history:
- '2026-08-05: authored from plan IC-06'
agent_profile: implementer-ivan
authoritative_surface: tests/
create_intent:
- tests/webview_projection_shell.rs
execution_mode: code_change
owned_files:
- tests/webview_projection_shell.rs
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

`tests/webview_projection_shell.rs` realizes the declared
`webview_projection_shell` validation: five falsifiable proofs, printing
`CREST_ACCEPTANCE webview_projection_shell passed` only when all hold, plus
the live layer (paced demo through the Tauri window) with NFR measurements.
Existing egui-default proofs must stay green — this WP changes no production
code.

## Context

- Plan IC-06; research R-05 (two-layer proof posture); crest-spec validation
  `webview_projection_shell` (`proof/validations.yaml`) and
  `asset.WebviewProjectionShellAcceptanceTests` prompts — the five proofs are
  declared there; this prompt adds sequencing and harness mechanics only.
- Marker convention: see how existing acceptance tests print
  `CREST_ACCEPTANCE <name> passed` (e.g. `tests/component_vocabulary.rs`) —
  one final print after all sections pass, `--nocapture` visible.
- Fixture: build the production fixture state exactly as
  `tests/spike_webview_view_model_dump.rs` does (it is the recorded pattern —
  production registries, two patches, chorus on the first, MIXER context).
- Page determinism harness: WP05 exposed `window.crest.render` /
  `renderObservation`. Headless execution without a browser: drive the page's
  JS with a minimal JS engine? No — keep it honest and simple: the harness
  spawns the REAL page in a hidden Tauri window when the environment allows
  (`CREST_WEBVIEW_TESTS=1`, local/live runs) and asserts via
  `renderObservation`; in headless CI the determinism section asserts the
  serialized-document layer (T022) and marks the DOM layer skipped —
  explicitly, in output, never silently (no-silent-fallback discipline).
  Gate structure: the acceptance marker prints only when every section that
  RAN passed, and the skip list is printed beside it.

## Branch Strategy

Planning base and merge target are both `feat/webview-shell-foundation`.
Execution happens in the lane worktree `finalize-tasks` computes; do not
branch manually.

## Subtasks

### T022 — Prove serialized-schema fidelity

Build the production fixture, select MIXER, project via `StateProjector`,
serialize the semantic model. Assert: byte-identity between what
`projection_channel` emits (call its serialization function directly — it
must be the same code path) and `serde_json::to_string` of the projector's
model; assert across at least three distinct states (initial, after a level
edit, after a focus move) so generation-gating can't hide a fork. Any
webview-only struct in the emit path fails this section by construction —
assert the emitted string round-trips into `serde_json::Value` equal to the
model's.

### T023 — Prove token-table freshness

Read the committed `webview-page/tokens.css`; call
`token_export::committed_tokens_are_fresh`. Also assert the generator's
injectivity guarantee (unique property names) and that the file carries the
GENERATED header. A drift failure message must name the property (WP04's
`TokenDrift` contract).

### T024 — Prove page render determinism at both viewports

Live-gated section (`CREST_WEBVIEW_TESTS=1`): open the hidden window at
1920×1080, inject the recorded document twice, assert
`renderObservation` equal both times and structurally correct — five bands
present, sixteen columns each carrying exactly the five declared structures
in order, focused column identified, level readouts in two-digit uppercase
hex, Inspector sends in declared order. Resize to 1280×800, re-assert with
the compact expectations (all bands, Inspector ≥ 320 px equivalent in the
observation, sixteen columns seated). Headless: print the explicit skip.

### T025 — Prove typed startup failure

Use WP02's documented page-override hook to point the shell at an unloadable
page; assert `WebviewShellError::PageLoadFailed` (typed, matchable) and
nonzero exit with no window — spawn the binary as a subprocess
(`assert_cmd`-style via std::process) and assert exit status + stderr
carries the typed error's display. Assert the eframe shell was NOT started
(no fallback) — absence of its startup marker in output.

### T026 — Prove shutdown parity and run the live layer

Live-gated: run the paced demo path through `--shell webview` (reuse the
existing live-demo runner the way the eframe path invokes it — read
`src/testing/live_demo_runner.rs` composition); assert the demo's completed
report, the same shutdown observation sequence the eframe shell records, and
collect NFR measurements: projection-to-paint p95 ≤ 50 ms (timestamp emit →
page ack via a `crest://painted` echo event), meter cadence ≥ 30 Hz sustained
5 min with bounded pending count (NFR-002 — allow a shortened 60 s soak under
test with the 5 min soak behind an env flag, both printed). Also run the full
existing suite in this WP's verification step to prove egui-default proofs
stay green (C-003).

## Definition of Done

- [ ] `cargo test --test webview_projection_shell -- --nocapture` passes
      headless with explicit skip listing; passes fully with
      `CREST_WEBVIEW_TESTS=1` locally
- [ ] `CREST_ACCEPTANCE webview_projection_shell passed` printed on success
- [ ] All five proofs implemented per the declared validation; no section
      silently skipped
- [ ] Full existing suite green (egui default untouched)
- [ ] NFR-001/002 measurements printed with pass/fail against thresholds
- [ ] `spec-kitty agent tasks mark-status T022 T023 T024 T025 T026 --status done`

## Risks

- Hidden-window flakiness on CI: that is what the explicit live-gate is for;
  the gate must never turn a failure into a skip (skip only when the env var
  is absent, before any window attempt).
- The `crest://painted` echo needs a tiny page addition — coordinate: it is a
  presentation-only ack in page.js WP05 already owns; if WP05 is merged,
  the addition rides in this WP with a one-line out-of-map rationale.

## Reviewer Guidance

Reject if: the marker prints while a RUN section failed; skips are silent or
gate on anything but the env var; T022 compares parsed values instead of
bytes/structural equality both; the live layer mocks the runner instead of
reusing it; production code changed.
