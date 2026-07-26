## 1. Canonical envelope state

- [x] 1.1 Add bounded Patch ADSR to canonical state, serialization, event records, text, StateTree, and schema-derived editing.
- [x] 1.2 Project ADSR into fixed destructor-free real-time Patch parameters.

## 2. Per-note engine behavior

- [x] 2.1 Implement independent sample-accurate envelope state for Braids voices with bounded stealing and zero-time stages.
- [x] 2.2 Route common ADSR through the engine-managed SoundFont per-note seam without multiplying synthesizer instances or applying a post-stem envelope.

## 3. Deterministic proof

- [x] 3.1 Expand headless coverage to every mixed editable instance and descriptor-supported MIDI kind with exact state/projection equality and restoration.
- [x] 3.2 Measure audible effects for every engine and envelope control and retain falsifying mutation cases.

## 4. Verification

- [x] 4.1 Add the named per-voice-envelope and exhaustive-demo acceptance targets with concrete overlapping-note assertions.
- [x] 4.2 Format, compile, lint, and run the declared deterministic acceptance targets.
