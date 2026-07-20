# Refactoring Review: codex/regenerate-with-crest-spec

**Date:** 2026-07-19
**Base branch:** working-tree `HEAD`, scoped to `spec/`
**Files reviewed:** 7

## Critical Issues

None.

## Refactoring Opportunities

None remaining. The review found and resolved two ownership issues before this report:

- Live checkpoint serialization now crosses the shell through the canonical `LiveDemoCheckpoint` value object; `LiveDemoRunner` owns the data and the composition-root callbacks own stdout I/O.
- Mixer-private send and wet buffers now produce a fixed-size `MixObservation`; `AudioRenderer` consumes that value instead of reaching into `MixEngine` internals to assemble callback observations.

The live runner, audio observation transport, shell composition, and verification assets each retain a single reason to change. Autonomous actions depend on `AppLoop` and the observation port rather than concrete reducer, engine, mixer, window, or device implementations.

## Minor Suggestions

- Keep the inline tagged step shape inside `LiveDemoScene` until another owner needs it. Extracting a separate public `LiveDemoStep` now would be speculative generality.
- The physical window/device acceptance remains intentionally human-run through `make demo-live`; the deterministic integration target verifies orchestration without introducing a fake product path.

## Summary

The Phase 1 CUE diff has clear domain ownership, no duplicate public concept, no direct UI/audio-state mutation path, and no design-pattern abstraction without a current use. No further behavior-preserving structural changes are recommended before crest-spec generation.
