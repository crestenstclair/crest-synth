# Refactoring Review: main working tree

**Date:** 2026-08-21
**Base branch:** `main` (uncommitted Phase 7 working tree)
**Files reviewed:** 87 modified or untracked paths

## Critical Issues

None remain after the fixes below.

## Refactoring Opportunities

- **Resolved — unbounded historical Sample PCM retention.** `SamplePreparer` used its deduplication map as an application-lifetime cache. It now prunes entries whose cache `Arc` is the sole owner before admitting a new asset. PCM referenced by an active, candidate, queued, retired, or in-construction prepared graph remains shared; cache-only historical PCM does not accumulate. A regression test proves both halves.
- **Resolved — decoder identity trust at the adapter boundary.** The preparer now rejects a decoder result whose canonical `SampleMetadata::asset_id` differs from the requested `SampleAssetId`. This prevents reference identity, graph-budget identity, visualization identity, and PCM content from diverging. A controlled fake proves the refusal.
- **Resolved — one-off visual values in the Phase 7 modal.** The modal shadow and geometry now resolve through the authored color, spacing, keyline, minimum-target, and density split tokens. The additional compact-only breakpoint was removed; the existing two density policies and intrinsic bounds drive reflow.

## Minor Suggestions

- `AppState` and `SemanticGraphicalViewModel` remain large, but splitting the Phase 7 reducer or projector by capability would create the adapter leakage and name-enumerated policy this change is required to prevent. Their new helper methods preserve the one reducer/one projection boundary, so no behavior-preserving extraction with a clearer owner was identified.
- `PositionCapabilityIdentity` is intentionally a fixed-size callback-layout record, not a second canonical capability ID. Construction still accepts only canonical `CapabilityId`/`EffectCapabilityId`, and over-capacity identities fail preparation rather than truncate.

## Boundary Review

- Capability-specific names occur in adapters and tests; the reducer, resolver, production projector, renderer, and live orchestration use descriptor/position identity. The repository no-name guard is the executable proof.
- Filesystem paths, `hound` types, decoding, resampling, allocation, and the Sample preparer's mutex stay on adapter/worker ownership. Callback code sees immutable PCM, fixed voice arrays, numeric landmarks, and fixed-size commands.
- The Sample callback paths contain no lock, I/O, log, panic, `String`, or `Vec`, and the measured allocator/destructor audit covers dispatch, render, audition start/stop, graph swap, and retirement.
- Browser focus, preview lifecycle, requested/active asset state, and exact return live in `AppState`/`InteractionState`; the DOM renders semantic identities and dispatches normalized input without owning row indexes or domain values.
- Descriptor choice and Sample Browser rows both resolve to the declared `ModalOption` component and one shared modal renderer. Detail controls remain the declared row family; waveform/envelope figures are non-focusable descriptor visualizations, not control forks.
- Callback traversal is bounded by fixed patch, effect, return, voice, frame, command-ring, and waveform capacities. Control/worker filesystem and preparation work is admitted by byte, duration, scalar, graph-memory, and waveform-pair limits.

## Summary

The Phase 7 structure preserves the repository's canonical mutation,
descriptor, adapter, and real-time boundaries. Three concrete issues found
during review were corrected with focused regression coverage. The post-review
`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test --all-targets` rerun passed, including the exact no-name guard and
Sample callback audits. The headless webview witness passed its serialized,
token, protocol, late-ack, and typed-startup sections; native-window and
physical-device acceptance remain separate incomplete gates.
