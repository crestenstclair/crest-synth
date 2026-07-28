## Why

Common Patch ADSR is conforming only when each production engine applies it independently per note, and that claim needs an exhaustive production-path witness. This completed slice keeps canonical envelope semantics and the deterministic headless proof together so post-stem or ignored-control implementations cannot pass.

## What Changes

- Add bounded canonical Patch-owned ADSR state, serialization, schema, reducer editing, and fixed real-time projection.
- Apply ADSR sample-accurately inside every SoundFont and Braids note voice, including overlapping releases and zero-time stages.
- Expand the deterministic demo to cover every mixed editable instance, supported MIDI kind, exact projection, audio isolation, restoration, and two-run equality.
- Add audible evidence that every engine Scalar and envelope field changes the matching production render.
- Keep demo evidence falsifiable through exact event records, mutation cases, and named acceptance markers.

## Capabilities

### New Capabilities

- `per-voice-envelope`: Canonical bounded ADSR applied independently inside both production engines.

### Modified Capabilities

- `observable-demo-scene`: Exhaustive deterministic mixed-engine state, projection, MIDI, audio, restoration, and mutation proof.

## Impact

This slice affects Patch envelope state, reducer targets, real-time envelope state, both prepared engines, deterministic demo construction/reports, mutation evidence, and the per-voice-envelope and exhaustive-demo acceptance targets. It is one non-overlapping slice of the integrated Phase 2 architecture.
