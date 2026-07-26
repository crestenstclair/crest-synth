# Refactoring Review: repair-production-runtime-contracts

**Date:** 2026-07-25
**Base:** working-tree `HEAD`, scoped to the production runtime-contract repair
**Files reviewed:** 24 implementation, test, script, CUE, and OpenSpec artifacts

## Critical Issues

None.

## Refactoring Opportunities

None remaining. The repair keeps each ownership boundary explicit:

- `StandaloneApplication` depends on injected provider, preparer, structural, observation, MIDI, window, and audio-output ports; concrete adapter choice stays in `src/bin/crest_synth.rs`.
- Audio-output negotiation and stream start are separate phases, with one validated `AudioDeviceConfig` feeding preparation before callback ownership begins.
- Runtime device failures and routing failures cross callback ownership through fixed-size atomic observations and are interpreted only on the control side.
- Oversized device buffers are rendered as bounded graph-capacity chunks, retaining the existing prepared graph and renderer rather than introducing a second rendering path.

The generic standalone type and explicit constructor are intentionally verbose because they expose the replaceable production ports required by the architecture. Bundling them into an opaque dependency container would reduce surface syntax while weakening the composition witness.

## Minor Suggestions

- If additional runtime-status kinds are introduced later, consider a common fixed-size status envelope. The current device-error and audio-observation transports have different semantics, so merging them now would be premature.
- Keep the exact-selector validation script limited to one-test witnesses; multi-test validation should receive a separate structured-count contract instead of relaxing this guard.

## Summary

The reviewed repair has no duplicate canonical concept, concrete application-service dependency, callback-side allocation/logging/formatting path, or silent rendering fallback. No further behavior-preserving refactor is recommended before acceptance.
