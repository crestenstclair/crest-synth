## 1. Governance and implementation preflight

- [x] 1.1 Evaluate the complete CUE package locally and stop on any structural or project-intent error before touching implementation files.
- [x] 1.2 Inspect the evaluated CUE diff and confirm it declares the eight additions (`MixObservation`, `AudioObservationSnapshot`, `AudioObservation`, `AtomicAudioObservation`, `LiveDemoScene`, `LiveDemoCheckpoint`, `LiveDemoReport`, and `LiveDemoRunner`) plus only the intended dependent-owner changes and no removals.
- [x] 1.3 Record the existing worktree changes and baseline results for format, clippy, and all-target tests so unrelated user edits and pre-existing failures remain distinguishable from this change.
- [x] 1.4 Reconcile the affected implementation frontier directly against the evaluated CUE architecture, preserving unrelated worktree changes and avoiding duplicate models or competing behavior paths.

## 2. Mixer and real-time observation foundation

- [x] 2.1 Implement and reconcile the fixed-size `MixObservation` value and make `MixEngine` derive reverb-input, delay-input, wet-output, final stereo peak/RMS, clipping, and non-finite measurements from its owned buffers without exposing buffers or changing mixed samples.
- [x] 2.2 Add mixer tests that compare rendering with observation enabled and disabled, verify stage-specific finite measurements, and prove clipped/non-finite samples update only bounded counters.
- [x] 2.3 Implement and reconcile `AudioObservationSnapshot`, the callback-to-control `AudioObservation` port, and the split-handle `AtomicAudioObservation` adapter as a dedicated coherent latest-value transport with generation and monotonic block sequence tags.
- [x] 2.4 Add transport tests for coherent concurrent publication, skipped stale observations, non-tearing floating-point bit patterns, bounded overwrite behavior, and separate callback-writer/control-reader ownership.
- [x] 2.5 Extend `AudioRenderer` to combine mixer measurements with fixed-capacity command and active-note observations, publish once per completed block, and clear the relevant note observation on Patch-targeted or global all-notes-off without treating it as engine state.
- [x] 2.6 Run renderer and real-time instrumentation tests proving observation work is bounded and callback-safe: no allocation, locking, blocking, formatting, logging, I/O, panic, backpressure, or owned-state destruction.

## 3. Live scene model and evidence values

- [x] 3.1 Implement and reconcile `LiveDemoScene` so it freezes an ordered unique editable-parameter universe from `ChannelParameters`, `GlobalParameters`, and installed Patch IDs after fixture installation, with no duplicate string-based field list.
- [x] 3.2 Build the bounded scene steps so every editable parameter instance receives an accepted change, a checkpoint, at least 500 ms visible dwell, and an audible-observation requirement, while navigation, fixture work, a boundary rejection probe, recovery, and cleanup remain semantic events.
- [x] 3.3 Implement and reconcile `LiveDemoCheckpoint` with expectation data frozen before dispatch and actual outcome, state hash/generation, exact projection, parameter snapshot, emitted effects, and newer generation-matched audio observation copied from production values afterward.
- [x] 3.4 Implement and reconcile `LiveDemoReport` so exact bidirectional coverage, lossless event-log evidence, final canonical `StateTree`, accepted/rejected records, checkpoint agreement, semantic cleanup, and zero active notes jointly determine completion and its derived control-side summary.
- [x] 3.5 Add focused tests for expected-versus-actual separation, descriptor-derived exact coverage, duplicate/missing/unexpected parameter detection, minimum dwell, stale or non-finite audio rejection, and stage-specific audible predicates.

## 4. Paced live orchestration

- [x] 4.1 Implement and reconcile `LiveDemoRunner` as a control-thread state machine driven by monotonic window ticks; it must never sleep, own mutable canonical state, call `AppState::apply` directly, or mutate projections, parameters, commands, engines, mixers, or buffers.
- [x] 4.2 Route each due navigation, edit, rejection probe, fixture event, and cleanup action through `AppLoop::dispatch_from`, dispatch at most one autonomous event per tick, and make the runner the single live-mode owner of `AutomaticMidiTest::tick` so fixture events are never duplicated.
- [x] 4.3 Correlate every accepted checkpoint to the independently frozen expectation, canonical event record and projections, a rendered frame, the 500 ms dwell, and a newer `AudioObservationSnapshot` whose parameter generation matches the accepted generation.
- [x] 4.4 Implement completion as one Patch-targeted semantic all-notes-off event per installed Patch, followed by a newer zero-active-note observation, exact coverage/loss checks, a single completed report, and permanently inert post-completion runner behavior.
- [x] 4.5 Verify rejection recovery and incomplete paths: a boundary no-op stays nonfatal, a later valid edit succeeds, early close attempts semantic cleanup without a success report, and no outcome fabricates state, coverage, projection, effects, or audio agreement.

## 5. Standalone UI, audio, and command entry

- [x] 5.1 Extend `StandaloneApplication::run_live_demo` with the normal startup order and the existing `EframeTextWindow`, physical `CpalAudioOutput`, HiDef SoundFont engine, Corridors MIDI fixture, `AppLoop`, renderer, and mixer; return typed visible errors instead of any headless, null-device, or silent fallback.
- [x] 5.2 Pre-size the live event journal for the frozen scene and expected fixture traffic, expose dropped-record counts as completion failures, and pass checkpoints and the final report exactly once to injected control-side output callbacks.
- [x] 5.3 Wire the eframe tick callback to advance the runner without blocking, render only `AppLoop::current_text`, keep servicing normal input/audio after completion, and preserve the final canonical projection until the user closes the native window.
- [x] 5.4 Add `--demo-live` parsing and mutual-exclusion validation against smoke, observe, demo-scene, exhaustive/degenerate, no-window, no-device, and auto-close modes, then serialize the declared `CREST_LIVE_CHECKPOINT`, `CREST_LIVE_EVENT_LOG`, `CREST_LIVE_STATE_TREE`, `CREST_LIVE_COVERAGE`, and `CREST_LIVE_SUMMARY` markers only from the control side.
- [x] 5.5 Add the public `make demo-live` target and library exports while preserving the exact command, deterministic behavior, output schema, coverage universe, and acceptance predicates of `make demo`.
- [x] 5.6 Add standalone composition, CLI-validation, `make -n demo-live`, early-close, final-state persistence, and no-auto-close tests around the production seams.

## 6. Deterministic live contract verification

- [x] 6.1 Create `tests/live_demo_scene.rs` using a deterministic monotonic clock, frame-observation harness, production reducer/projections/render path, and real `AtomicAudioObservation`, without opening or conditionally skipping for a native window or physical device.
- [x] 6.2 Assert pacing and one-event-per-tick behavior, exact current-surface coverage, independently expected checkpoint agreement, accepted/rejected recovery, generation-correlated finite audio observations, semantic cleanup, and zero active notes.
- [x] 6.3 Assert the completed report and output contract occur exactly once, the runner is inert after completion, the final projection remains available, and print `CREST_ACCEPTANCE live_demo_scene passed` only after all assertions succeed.
- [x] 6.4 Run the dedicated live-demo integration test and all affected unit/integration validations declared by the CUE-governed Mixer, RealTime, Testing, Shell, eframe, CLI, Makefile, and manifest resources.

## 7. Closure and physical acceptance

- [x] 7.1 Review the complete CUE-declared affected frontier and run its local validation closure, repairing all matched owners together when a failure exposes a dependency mismatch.
- [x] 7.2 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`, then run the existing smoke, exhaustive-demo, schema-surface, eframe-context, behavioral-mutation, and real-time checks without weakening or updating their acceptance predicates to accommodate live mode.
- [x] 7.3 Run `make demo` twice and confirm its headless/no-device/no-window trace remains deterministic and unchanged in meaning, including exact coverage and controlled-negative behavior.
- [x] 7.4 With repository fixtures and a usable physical output device, run `make demo-live`; confirm the production window stays responsive, every current parameter visibly dwells, SoundFont audio is audible, live markers agree, cleanup reaches zero active notes, and the final canonical state remains visible until manual close.
- [x] 7.5 Re-evaluate the complete CUE package, inspect the final scoped diff for architectural consistency and no unintended removals, and validate this OpenSpec change before marking the implementation complete.
