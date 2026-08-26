## Why

Instrument Detail and FX Detail already exist as reducer-owned, descriptor-driven subordinate surfaces, but their shipped webview composition remains a functional blockout and does not yet realize the hierarchy, state vocabulary, or responsive grammar defined by Figma. This slice completes the next front-end vertical by bringing both Detail subjects into the responsive Patch shell without introducing another state model, navigation system, or capability-specific renderer.

## What Changes

- Replace the Instrument Detail and FX Detail blockout presentation with one shared, responsive Detail grammar derived from the live Figma Instrument Detail, FX Detail, Interaction Map, Patch Overview, and Responsive Front-End Contract nodes.
- Render the active Patch, exact Detail subject, ordered descriptor sections, canonical controls, shared envelope controls when supplied, scalar ranges/positions, structural lifecycle, typed failures, focus/adjustment/disabled treatments, and projected action hints from the immutable Rust projection.
- Preserve reducer-owned entry, valid-action resolution, singular semantic focus, exact Engine/effect-slot return identity, nearest-enabled-sibling return repair, occupied-slot gating, and persistent PATCH Utility.
- Keep layout presentation-only across wide, standard, intermediate, compact, 1280×800, and enlarged-text conditions, with the existing 48px target floor, bounded scrolling, and no resize-derived semantic action or product mutation.
- Extend production-path reducer, projector, serialization, DOM, native-input, and responsive observation evidence for multiple differently shaped instrument and effect descriptors, structural lifecycle/failure states, deterministic rerendering, and Mixer non-regression.
- Update `DESIGN.md` during implementation only with durable as-built results and verified remaining gaps; do not claim broader Figma parity.

## Capabilities

### New Capabilities

- `instrument-fx-detail`: Defines the shared production-path Instrument Detail and FX Detail contract, including canonical subjects, descriptor-ordered sections and controls, lifecycle/state treatments, navigation and return behavior, persistent Utility, responsive reachability, and measured acceptance.

### Modified Capabilities

None. The existing `patch-overview` and `responsive-shell-composition` requirements remain in force and are consumed by this slice without changing their contracts.

## Impact

- Durable authority and references: `DESIGN.md` plus Figma nodes `98:2`, `37:7`, `38:60`, `49:3`, and `95:202`.
- Canonical control/projection seams: `AppState`, semantic focus/resolution, `SemanticGraphicalViewModel`, state projection, and Patch page projection may receive only the smallest generic descriptor-driven additions required by the authored Detail presentation.
- Webview composition and tokens: `webview-page/page.js`, `webview-page/page.css`, generated/shared tokens, and the existing render-observation surface.
- Evidence: semantic focus/projection suites, Patch projection tests, webview reducer-to-DOM and responsive/native witnesses, target/overflow/overlap measurements, deterministic rerender checks, and the AppKit `FlagsChanged` Shift regression.
- Audio callback behavior, real-time transports, graph preparation/activation/retirement, installed capability ownership, persistence, asset/device ports, and Mixer product semantics are not changed.
