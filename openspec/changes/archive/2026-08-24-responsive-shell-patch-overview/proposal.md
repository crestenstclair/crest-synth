## Why

The shipped webview is a functional projection blockout whose two fixed density policies and Patch Strip root no longer match the live Figma front-end contract. This change establishes a fluid, grid-based shell and implements Patch Overview as the first production-path screen so later detail, browser, and Mixer work can reuse one responsive composition system.

## What Changes

- Update `DESIGN.md` so the durable front-end contract reflects fluid composition, representative reference frames, content-driven layout modes, and presentation-only reflow.
- **BREAKING (presentation contract):** retire the closed Desktop/SteamDeck geometry model as the authority for shell bands, main/side widths, and control geometry. Replace it with bounded layout tokens and responsive Grid/Flex composition using intrinsic sizing, `minmax()`, `clamp()`, and wrapping.
- **BREAKING (PATCH root composition):** replace the current Patch Strip root view with Patch Overview, containing the active Engine and three ordered Post FX slots plus the persistent Utility region.
- Preserve the existing semantic action path, reducer-owned product state, stable focus identities, return identity, capability-driven content, and immutable view projection. Reflow must not dispatch a semantic action or mutate product state.
- Add production-render-path evidence across wide, standard, compact, and intermediate widths, including 1280×800 and scaled-text conditions. Validate reachability, no overlap or clipping, stable focus, and the 48px interactive-target floor.
- Establish shared responsive primitives that the subsequent Instrument/FX Detail, Sample Detail/Browser, Mixer, and native visual-validation changes can reuse without extending this change's product scope.

## Capabilities

### New Capabilities

- `responsive-shell-composition`: Defines fluid shell regions, content-driven layout modes, bounded scaling, presentation-only reflow, accessibility floors, and representative-viewport acceptance.
- `patch-overview`: Defines the PATCH root composition, Engine and ordered Post FX slot presentation, persistent Utility behavior, semantic focus continuity, and navigation into existing subordinate workflows.

### Modified Capabilities

None. This repository has no existing OpenSpec capability specifications.

## Impact

- Durable product reference: `DESIGN.md` and the live Figma Responsive Front-End Contract, Patch Overview, and Interaction Map nodes.
- Shell policy and token export: `src/shell/density.rs`, `src/shell/tokens.rs`, `src/shell/webview/token_export.rs`, generated `webview-page/tokens.css`, and window/reference-viewport helpers.
- Webview composition: `webview-page/index.html`, `webview-page/page.css`, and `webview-page/page.js`.
- Projection surface: existing `SemanticGraphicalViewModel` and state projector contracts remain authoritative; they may be minimally enriched only when Patch Overview requires semantic data not currently projected.
- Evidence: reducer/projection tests, headless webview render observations, focus/reflow assertions, representative viewport checks, and native-window witnesses.
- Audio, real-time transports, graph preparation, capability adapters, persistence, and device ownership are not changed.
