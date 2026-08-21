# Refactoring Review: assemble-sixteen-track-mixer

**Date:** 2026-08-20
**Base branch:** working-tree `HEAD` on `main`
**Files reviewed:** 20 implementation, test, design, and OpenSpec artifacts

## Critical Issues

None.

## Refactoring Opportunities

Three findings were resolved during review and acceptance:

- The WebView had a second object-valued semantic-control lookup beside
  `controlById`. The Mixer Inspector now uses the shared lookup and the same
  stable serialized identity as every other projected control.
- The live Inspector evidence paired buses and controls with `zip`, which
  could accept a truncated prefix. It now requires at least the canonical
  eight indexed send rows before validating their identity, order, range, and
  selected track.
- The Inspector summary projected the active side-row identity while the
  correlation header requires the remembered Mixer-main origin. Both summary
  fields now derive from the same remembered main `FocusPath`, and the
  serialized-leaf declaration reflects that Track-only invariant.

No additional behavior-preserving refactor is warranted. The canonical Mixer
state and focus remain in the reducer and semantic projection; the WebView
only composes and observes projected controls. Meter updates remain a passive,
revision-compatible transport projection. The dedicated demo reuses the
existing production scene, runner, MIDI, reducer, renderer, and physical audio
path rather than creating a parallel Mixer implementation.

## Minor Suggestions

- If another object-valued control identity consumer appears, consider naming
  the serialization operation itself. With the duplicate removed, extracting
  an abstraction now would be premature.
- Keep `LiveMixerSceneEvidence` explicit while its fields are acceptance
  predicates. A generic property bag would shorten the type but weaken typed
  completeness and emitted-schema review.

## Summary

The reviewed change introduces no duplicate canonical track, bus, focus, or
meter concept and does not move application decisions into the page. The two
concrete structural/evidence issues found during review were corrected; no
blocking refactoring finding remains.
