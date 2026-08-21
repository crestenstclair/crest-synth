## 1. Lock Product Authority and Acceptance

- [x] 1.1 Move the durable Phase 7 Sample, subordinate-session, preview-transport, persistence, and real-time decisions from this design into `DESIGN.md` before changing product behavior; preserve the master boundaries and include the exact Figma file/node references used for visual and interaction review.
- [x] 1.2 Audit the shipped Phase 6 evidence, record its closeout in `ROADMAP.md` only when the production tests/live report support it, and add a Phase 7 requirement-to-test/demo acceptance matrix without turning the roadmap into a second architecture source.
- [x] 1.3 Reopen Figma nodes 37:7, 38:60, 39:92, 41:138, 48:173, 48:207, and 49:3 and record reference-date screenshots/notes for shell anatomy, focus, modal, waveform, browser, and command-hint comparison; explicitly label option names, filenames, counts, and values as fixtures.
- [x] 1.4 Add failing production-path acceptance tests for each scenario in the three Phase 7 delta specs, mapping every test name back to its requirement and avoiding direct view-state or callback-engine mutation.

## 2. Generalize Subordinate Interaction State

- [x] 2.1 Add reachability and mutation tests for exactly one active subordinate session, exact semantic origin, no modal-from-modal transition, and no contradictory detail/modal/browser subjects.
- [x] 2.2 Replace the private detail-only invariant with one canonical `PatchSubordinateSession` union for Detail, Choice, and Sample Browser, retaining only the subject/origin/session identities that interaction state owns.
- [x] 2.3 Extend `SurfaceId`, `FocusPath`, `SemanticControlId`, modal/browser IDs, and `ReturnPath` with stable semantic identities and validation; do not store option, folder, or file indices.
- [x] 2.4 Extend the resolver with non-wrapping detail/modal/browser orders, disabled-target skipping, exact return, and deterministic nearest-sibling repair when the recorded origin disappears.
- [x] 2.5 Add reducer transitions for entering/replacing/leaving each subordinate surface and prove every accepted transition ends with one valid focus, one compatible mode, and the correct remembered Patch root.
- [x] 2.6 Extend serialization/schema witnesses only for persistent interaction data that is already part of the saved contract; prove modal, browser, request, and preview sessions remain transient.

## 3. Normalize Phase 7 Input and Choice Semantics

- [x] 3.1 Add keyboard/controller normalization tests for Shift+Up detail/browser entry, Shift+Down single-level return, Edit+Up choice opening versus numeric coarse increment, Edit confirmation, modal Up/Down, and Start press/release.
- [x] 3.2 Add semantic actions/events for opening a generic choice subject, selecting/cancelling a stable option, browser row activation, and preview hold/release/implicit stop, with no physical key or device identity in `AppState`.
- [x] 3.3 Update the keyboard and MIDI/controller adapters to emit the new semantic vocabulary and prove Start remains unavailable outside the Sample Browser.
- [x] 3.4 Add a generic choice-source resolver for installed instruments, `EMPTY` plus installed effects, T00–T0F routes, descriptor choices, and correlated catalog choices; prove unavailable Figma examples never appear.
- [x] 3.5 Route choice confirmation through the owning canonical scalar or structural intent, keep the modal projection mutation-free, and prove cancel changes no session/audio state.

## 4. Complete Descriptor-Driven Detail

- [x] 4.1 Add descriptor conformance tests for ordered detail sections, generic presentation metadata, units/ranges/dependencies, asset rows, structural choices, and non-focusable visualization declarations across multiple installed instruments/effects.
- [x] 4.2 Extend the existing canonical capability descriptor types with the minimum generic detail/visualization metadata needed by projection, reusing `ParameterId`, `ParameterValue`, dependency, asset, and update types rather than creating UI copies.
- [x] 4.3 Complete the shared `PatchDetail` projector for instrument and effect subjects, including occupied effect-slot identity, live canonical values, dependency changes, active/requested status, and non-focusable ADSR/status visualization.
- [x] 4.4 Add reducer/projector tests that open instrument and effect detail on more than one capability, traverse all enabled rows, edit through `AppState::apply`, and return to each exact origin after reprojection.
- [x] 4.5 Extend the no-name-enumeration guard and structural review tests so reducer, resolver, projector, renderer, rack, and live orchestration cannot switch on Sample, SoundFont, Braids, Chorus, or another concrete capability to define fields.

## 5. Build the Sample Asset Boundary

- [x] 5.1 Add canonical `SampleAssetId`, metadata, admission limits, typed validation/capacity errors, and catalog/decoder/preparation ports while retaining `AssetReference`/`AssetAssignment` as the shared capability-level asset vocabulary.
- [x] 5.2 Pin the small WAV decoder selected by the design (prefer `hound`), update `Cargo.lock`, and document/audit its version, license, and feature surface; keep dependency types inside the adapter.
- [x] 5.3 Add deterministic valid mono/stereo PCM16/24/32 and float32 WAV fixtures plus malformed, compressed/renamed, non-finite, oversize-header, over-duration-header, and unsupported-channel/rate fixtures with provenance and bounded repository size.
- [x] 5.4 Implement the production WAV adapter with checked RIFF/chunk arithmetic, exact format admission, finite conversion to `f32`, byte/duration/scalar limits, and typed rejection; pass decoder unit and adversarial fixture tests.
- [x] 5.5 Implement the configured library-root catalog adapter with stable relative IDs, canonicalization, traversal/symlink escape rejection, deterministic folder-first ordering, typed metadata loading, and no native dialog.
- [x] 5.6 Implement off-thread device-rate resampling and a bounded 2,048-pair waveform summary, testing duration/frame preservation, mono/stereo behavior, endpoint coverage, and finite output.
- [x] 5.7 Add deterministic fake catalog/decoder adapters for reducer and demo-controlled failures without creating a silent production fallback.

## 6. Define and Prepare the Sample Capability

- [x] 6.1 Add descriptor tests for the single asset, root note, normalized playback/loop landmarks, `OFF`/`FORWARD`, crossfade, shared ADSR, dependency disabling, defaults, fine/coarse steps, `FixedPerPatch(16)`, and the sixteen-scalar real-time ceiling.
- [x] 6.2 Implement the Sample capability descriptor/config factory and validation using generic capability metadata; keep Sample unregistered in production until its real preparer succeeds.
- [x] 6.3 Add validated control-side Sample playback configuration and conversion from normalized values to inclusive/exclusive prepared frame landmarks, including every range/crossfade invariant and typed failure.
- [x] 6.4 Implement the Sample preparer pipeline—resolve, validate, decode, resample, summarize, validate landmarks, allocate/warm voices—and enforce 28,800,000 scalar samples per asset plus 512 MiB deduplicated PCM per complete graph.
- [x] 6.5 Extend complete-graph construction to deduplicate prepared Sample PCM by stable asset/preparation identity on the worker and reject aggregate overflow before publication.
- [x] 6.6 Extend typed instrument/graph preparation failures so asset unavailable, unsupported format, invalid PCM/landmarks, per-asset overflow, aggregate overflow, cancellation, and allocation failure remain distinguishable through control state.

## 7. Implement the Prepared Sample Engine

- [x] 7.1 Add failing renderer tests for root-note speed, octave ratios, normalized start/end, mono duplication, stereo retention, finite bounded interpolation, note-off release, and Patch-local target isolation.
- [x] 7.2 Implement one prepared Sample instrument with immutable shared PCM and sixteen preallocated voices behind the existing `PreparedInstrument` port; keep all strings, paths, decoders, and dynamic setup off the callback.
- [x] 7.3 Implement deterministic inactive → oldest releasing → oldest active voice allocation and prove the seventeenth overlapping note steals only the specified Patch-local voice.
- [x] 7.4 Implement `OFF` playback and `FORWARD` loop wrapping with the prepared linear crossfade and ADSR release behavior; add boundary, one-frame-neighbor, maximum-crossfade, and no-out-of-range-read tests.
- [x] 7.5 Integrate Sample into the prepared engine rack/builder without capability identity policy in the rack, then register the production descriptor/preparer together so an unpreparable Sample never appears installed.
- [x] 7.6 Extend callback safety/performance audits over Sample dispatch, render, all-notes-off, graph swap, and off-thread retirement, asserting zero callback allocation, locking, blocking, I/O, logging, panic, and destruction.

## 8. Correlate Assignment and Activation

- [x] 8.1 Add reducer/coordinator tests for `Loading → Validating → Preparing → Activating → Ready`, every typed terminal, active/requested distinction, stale completion, cancellation, and prior-graph preservation.
- [x] 8.2 Extend `StructuralEditIntent`, preparation requests/results, request IDs, source revision checks, and coordinator state for Sample asset assignment without creating a Sample-only bypass around complete graph construction.
- [x] 8.3 Commit the active Sample reference only after the correlated prepared graph activates at a block boundary; prove failure, cancellation, and incompatible generation leave session and audio graph unchanged.
- [x] 8.4 Project active/requested asset and all typed lifecycle states from canonical state using explicit status text/shape as well as color, including `Cancelled — unchanged` on browser cancellation.
- [x] 8.5 Add saved-state versioning/migration tests for stable relative Sample IDs and normalized playback config; prove older sessions migrate, missing assets restore as `Unavailable`, and no decoded/transient state serializes.

## 9. Add Prepared Browser Audition

- [x] 9.1 Add state-machine tests for Start press/hold/release, release-before-ready, focus/navigation/assign/cancel implicit stop, repeated press, stale preparation, and Start rejection outside the browser.
- [x] 9.2 Add a single correlated `PreparedSampleAudition` slot to complete graph preparation without changing the persisted or active Sample assignment; retain and destroy candidate PCM only through worker/graph ownership.
- [x] 9.3 Add fixed-size `PreviewStart`/`PreviewStop` discrete audio events and route them through the existing bounded event transport after `AppState::apply`; prove no preview command is emitted before compatible audition activation.
- [x] 9.4 Mix the one preallocated audition voice into the origin Patch stem before post effects/trim so track level, pan, mute, solo, sends, returns, and meters apply; add target/routing isolation tests.
- [x] 9.5 Implement original-pitch, file-start, no-loop audition plus the prepared 5 ms de-click stop, and prove preview never mutates the committed Sample config or persisted session.
- [x] 9.6 Extend latest audio observation snapshots with revision-compatible Sample playhead/preview state and tests for stale suppression, finite values, and zero application mutation during repaint.
- [x] 9.7 Run callback audits for audition activation, start, render, implicit stop, graph replacement, and retirement with the same hard-real-time prohibitions as instrument rendering.

## 10. Project Detail, Choice, Browser, and Waveform

- [x] 10.1 Add typed view-model nodes for detail sections, current/requested option entries, browser row kinds, metadata, lifecycle/status, waveform pairs/landmarks/playhead, and command hints with stable semantic focus identities.
- [x] 10.2 Implement the generic option-modal projector with origin label, active/current marker, focused marker, non-wrapping ordered choices, empty/unavailable handling, and trapped modal mode.
- [x] 10.3 Implement the Sample Browser projector for parent/folder/file/cancel rows, listing/metadata loading and errors, exact row focus, preview status, route/mute explanation, and non-focusable Preview region.
- [x] 10.4 Extend `PatchDetail` projection for Sample asset, root/playback/loop/ADSR controls and the bounded waveform landmarks, sourcing playhead only from a compatible latest observation.
- [x] 10.5 Add JSON/schema round-trip and production frame-stream tests proving webview messages contain no decoded PCM, absolute library path, widget index authority, or UI-owned domain value.
- [x] 10.6 Add projection tests for density reflow, catalog refresh, schema dependency changes, missing origin repair, stale observations, and explicit text/shape treatment of focus/edit/current/loading/unavailable/invalid/cancelled/playing states.

## 11. Compose the Production Webview Surfaces

- [x] 11.1 Reuse/extend shared component-library variants for detail group headings, option entries, browser rows, waveform landmarks, status lines, and CLI hints; do not add one-off Sample-only focus or layout primitives.
- [x] 11.2 Render instrument/effect/Sample detail through one DOM composition and bind events only to semantic action dispatch, verifying Utility remains persistent and informative visualizations remain outside tab/focus order.
- [x] 11.3 Render the shared 920×680 authored option modal and its compact policy with trapped focus, origin/current markers, explicit empty/error states, and valid live command hints.
- [x] 11.4 Render the controller-native Sample Browser and Preview region with deterministic row reconciliation, hold/release feedback, metadata/waveform/status, and no browser or native file APIs in JavaScript.
- [ ] 11.5 Compare 1920×1080 output against the captured Figma detail/modal/browser nodes for bands, 1500/420 workspace split, hairlines, typography, spacing, focus/edit/disabled cues, accent semantics, and footer correlation; document intentional behavior-owned differences.
- [ ] 11.6 Verify 1280×800 composition preserves the same focused semantic IDs, exact return, Utility/Preview context, minimum targets, hierarchy, and readable explicit states without dispatching a density event.
- [ ] 11.7 Extend webview input-capture and projection-shell witnesses to prove keyboard/controller normalization, Start key-up delivery, modal capture, browser exact return, and clean window teardown through production ports.

## 12. Integrate and Harden Production Paths

- [x] 12.1 Add end-to-end reducer → snapshot/request → worker → prepared graph → callback → observation → projector tests for valid assignment, invalid assignment, cancellation, stale completion, and preview hold/release.
- [x] 12.2 Add production-path routing tests with concurrent SoundFont/Braids/Sample Patches proving Sample notes and preview affect only the origin Patch stem, selected Mixer track, and configured sends/returns.
- [x] 12.3 Add persistence/restore integration tests for ready, unavailable, invalid, and over-budget Sample sessions, proving restore commits only after preparation and never falls back.
- [ ] 12.4 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, the exact validation script, no-name guard, callback audits, and webview projection witnesses; resolve failures without weakening assertions.
- [x] 12.5 Perform a post-implementation structural review for duplicate concepts, capability-identity policy, callback hazards, adapter leakage, UI-owned state, unbounded work, and one-off component forks; refactor and rerun the full gate.

## 13. Build the Cumulative Live Evidence

- [x] 13.1 Add typed Phase 7 live checkpoint/report structures for focus origins/returns, active/requested choices, asset lifecycle, preview hold intervals, graph/generation compatibility, per-Patch/track/bus energy, controlled negatives, and teardown.
- [x] 13.2 Build the cumulative scene on the production webview/device/threaded-worker composition and real parsed MIDI fixture, with concurrent MIDI on multiple installed instrument capabilities including Sample.
- [x] 13.3 Script semantic instrument/effect detail journeys plus engine, effect, route, and descriptor-choice modal commit/cancel journeys, asserting visible/canonical focus and exact return at each bounded checkpoint.
- [x] 13.4 Script library navigation, valid hold preview/release, cancellation, one production-adapter invalid/unavailable attempt, valid assignment, preparation/activation, and Sample detail waveform correlation without direct state or engine injection.
- [x] 13.5 Add measured audible predicates for preview interval, release stop, committed Sample playback, route movement, mute/solo/level/pan, send/return behavior, and non-target stem isolation with finite thresholds and revision matching.
- [x] 13.6 Add a declared controlled-negative flag that suppresses one required semantic preview/return/commit action and prove the optimized command exits non-zero on the named reach or audio predicate.
- [x] 13.7 Add the `--demo-live-detail-and-assets` CLI mode and retained `make demo-live-detail-and-assets` release target, preserve every earlier target, and repoint `make demo-live` to the new cumulative scene.
- [x] 13.8 Add deterministic headless coverage for the scene orchestration/report and require zero active MIDI notes, zero audition voice, zero pending request, window/stream closure, graph retirement/destruction off-callback, and a complete lossless event log.
- [ ] 13.9 Run `make demo-live-detail-and-assets` and `make demo-live` with the production window and physical audio device, capture the structured passing reports and Figma comparison evidence, and record any typed environmental skip as incomplete rather than acceptance.
- [ ] 13.10 Update `DESIGN.md` and `ROADMAP.md` with measured Phase 7 completion evidence, exact retained targets, dependency/version pins, known bounded limitations, and the Phase 8 handoff only after every deterministic and required physical gate passes.
