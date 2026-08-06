# Research: Webview Shell Cutover

**Mission**: `webview-shell-cutover-01KZAC7Q` · **Date**: 2026-08-06

No open unknowns: the foundation mission (`webview-shell-foundation-01KZ9DN7`)
and its committed research already resolved the risky questions on hardware.
This file consolidates the decisions this plan inherits or makes, with
rationale and alternatives.

## D-01 — Renderer: Tauri v2 / wry system webview (inherited, proven)

- **Decision**: keep the foundation's stack; no runtime change in scope.
- **Rationale**: proven on hardware at merge — production MIXER at authored
  parity from 244 lines of HTML/CSS vs a 684-line density policy;
  `validation.webview_projection_shell` green at HEAD.
- **Alternatives considered**: staying on egui (rejected by operator decision
  2026-08-05 — manual layout arithmetic cost, 17k-line visual layer); other
  webview hosts (rejected: nothing wrong with the proven one).

## D-02 — Input capture: NSEvent local monitor, Rust-side (inherited, proven)

- **Decision**: keep `src/shell/webview/input_capture.rs`; the tao/tauri
  event path is structurally impossible for keyboard (no keyboard variant in
  `tauri::WindowEvent`, confirmed in vendored source by the foundation
  reviewer).
- **Rationale**: committed probe verdict `webview-shell-foundation-01KZ9DN7/research/input-capture-probe.md`;
  bijectivity test guards the keycode map.
- **Alternatives considered**: DOM key handlers (forbidden — page owns no
  input; asserted by test); tao path (proven impossible).

## D-03 — Cutover ordering: parity → evidence → deletion

- **Decision**: migrate scenes and refresh hardware evidence while both
  shells exist; take the same-workload RT A/B in that window; the deletion
  commit is the last structural change (C-007).
- **Rationale**: the A/B needs both shells alive; retained evidence must
  never have a gap where neither shell has proven the scenes; ROADMAP gate
  terms ("retained evidence survives the cutover or the cutover does not
  complete").
- **Alternatives considered**: delete-then-reprove (rejected: evidence gap,
  no A/B baseline); shell-selection flag kept permanently (rejected:
  `selected_webview_stack` declares one runtime, and a permanent flag is a
  standing silent-fallback risk).

## D-04 — PATCH rendering: extend the one page, no second document

- **Decision**: PATCH renders from the same serialized
  `SemanticGraphicalViewModel` document and the same `tokens.css`; the page
  gains PATCH layout rules, not a PATCH-specific schema.
- **Rationale**: `requirement.serialized_projection_transport` — one schema,
  no webview variant; the projector already carries full PATCH content
  (`PatchPageProjection` → view model), which the egui shell renders today.
- **Alternatives considered**: separate PATCH document/route (rejected:
  schema fork, exactly what the byte-identity proof forbids).

## D-05 — Vocabulary relocation before deletion

- **Decision**: move the surviving declarations (`token.rs`, `typeface.rs`,
  `density.rs`, `state.rs` and control/composition/structure enums) out of
  `src/shell/visual/` up to `src/shell/`, then delete `visual/`'s painting
  modules (`primitives/`, `controls/`, `compositions/`).
- **Rationale**: the crest-spec keeps `ShellContextModules` as the authored
  single source the token export generates from; the no-literal guard and
  token-freshness validation must survive the move.
- **Alternatives considered**: deleting the whole tree and re-declaring
  tokens page-side (rejected: breaks the authored-Rust-vocabulary single
  source and the generation-freshness proof).

## D-06 — Gallery: webview-hosted, same declared scope

- **Decision**: rebuild the 15 pages on the webview surface (operator,
  2026-08-06); keep the gallery's declared no-audio/no-MIDI scope and
  scene-local Rust-side digit/stepping input.
- **Rationale**: Phase 4's browsable-evidence surface survives; C-006.
- **Alternatives considered**: retire the gallery (offered, rejected);
  defer to plan (moot — decided at specify).

## D-07 — Scene hosting: scenes unchanged, shell swapped underneath

- **Decision**: the four retained scenes keep their checkpoint protocols and
  frozen identity baselines; only the AppWindow implementation under them
  changes. New webview-era checkpoints are pure insertions (C-004).
- **Rationale**: "migration" would imply rewriting scenes — the roadmap gate
  and Domain Language forbid that reading; add-only identity is the
  established contract (`FROZEN_TOPOLOGY_IDENTITY_BASELINE` precedent).
- **Risk noted**: scenes must block on qualifying forwarded frames (IC-02),
  not sleeps — the foundation review flagged exactly this seam.

## D-08 — Hardening scope

- **Decision**: restrictive CSP over `crest://` embedded assets;
  `CREST_WEBVIEW_PAGE` compiled out of release; key-injection witness drives
  the full `WindowKey` vocabulary through the NSEvent path into the running
  shell; six fader px specimens move into the generated table.
- **Rationale**: foundation review open items 5 and the FR-003 PARTIAL
  (no automated key-injection witness) recorded for this successor.
- **Alternatives considered**: leaving CSP null (rejected: free
  defense-in-depth, review-recommended).
