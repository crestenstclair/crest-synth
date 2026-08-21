## Why

Phase 6 established the sixteen-track Mixer, but PATCH still lacks the controller-native subordinate workflows needed to inspect instruments and effects, make structural choices, and assign real sample assets without leaving the semantic focus model. Phase 7 closes that gap now and resolves the deliberately deferred Sample capability contract before implementation can harden controller behavior in Phase 8.

## What Changes

- Complete the shared, descriptor-driven Patch detail surface for instrument and effect capabilities, including Sample-specific asset, waveform, loop, and envelope presentation.
- Add trapped-focus choice modals for engine, effect, route, and other descriptor-declared structural choices, with exact semantic focus return on commit or cancel.
- Define a bounded Sample capability and production preparation pipeline covering admitted assets, playback and loop behavior, polyphony, root pitch, preparation limits, typed loading/error/cancellation state, and no-silent-fallback failure behavior.
- Add a controller-native Sample Browser with semantic row focus, metadata and waveform landmarks, hold-Start preview, release/focus-change stop, assignment, cancellation, and exact-origin return.
- Render the Phase 7 workflows with the repository component library at reference and compact densities, using the linked Crest Synth Figma file for composition, navigation, focus, and visual language while treating its example engines, effects, patches, assets, and values as fixtures rather than an exhaustive product contract.
- Add `make demo-live-detail-and-assets`, driven by real MIDI and production reducers/render/audio paths, to exercise detail, choice, asset preview and commit, cancel/error recovery, focus return, and audible target isolation; make it the cumulative `make demo-live` scene.

## Capabilities

### New Capabilities

- `patch-detail-and-choice-workflows`: Descriptor-driven instrument/effect detail and structural choice workflows, including semantic modal focus, exact return, responsive projection, and accessible state communication.
- `sample-capability-and-browser`: The bounded Sample engine contract, asset preparation lifecycle, Sample detail projection, controller-native browser, preview/assignment semantics, and typed failure/cancellation behavior.
- `live-detail-and-assets-demo`: A deterministic real-MIDI acceptance scene that proves Phase 7 through the production reducer, render path, preparation handoff, and audio engine.

### Modified Capabilities

None. This repository has no established OpenSpec capability baseline; Phase 7 capabilities are introduced as new delta specifications and remain subordinate to `DESIGN.md`.

## Impact

The change affects canonical app state and semantic actions, capability descriptors and ports, semantic focus/modal state, asset catalog and persistence adapters, off-thread sample decoding/preparation and bounded real-time graph handoff, Patch projection and Tauri webview components, production-path tests, demo MIDI fixtures and Make targets, and the Phase 7 durable decisions in `DESIGN.md`/roadmap evidence. No SoundFont or Sample fallback path is introduced, and the existing physical-input-to-reducer boundary and real-time callback constraints remain mandatory.
