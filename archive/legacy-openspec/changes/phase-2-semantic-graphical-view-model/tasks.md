## 1. Canonical semantic identities and resolver

- [x] 1.1 Add and export `SurfaceId`, `InteractionMode`, `SemanticAction`, `MixerControlId`, `SemanticControlId`, `FocusPath`, `ReturnPath`, and `ValidAction` as closed typed values with exhaustive descriptors, serialization, equality, and invariant tests.
- [x] 1.2 Add canonical `PatchEditableTarget` identities for mixer, envelope, and instrument values, then refactor MIXER navigation, adjustment, demos, and coverage to consume one descriptor-derived target resolver without positional identity.
- [x] 1.3 Implement one pure semantic resolver for ordered PATCH/MIXER controls, path resolution, focus eligibility, exact accepted actions, and context-compatible surface entry; prove ordered duplicate-free results at direction, value, dependency, and lifecycle boundaries.
- [x] 1.4 Replace positional `InteractionState` fields with one active path, remembered PatchMain/MixerMain paths, Navigate/Adjust mode, and optional return path; update initialization and Patch installation to establish valid stable paths.
- [x] 1.5 Implement shared active/remembered/return path validation and next-before-previous recovery over canonical descriptor order, retaining stable targets when they survive and rejecting projector/widget repair.

## 2. Reducer and one-way action path

- [x] 2.1 Extend `AppEvent` and its surface descriptor with typed focus navigation, interaction-mode, surface-entry, and Return events while retaining startup, MIDI, worker, graph, and system event entry unchanged.
- [x] 2.2 Add `AppLoop::dispatchAction` to map each `SemanticAction` to exactly one `AppEvent` before `AppState::apply`, preserving event records, commit-before-project/publication ordering, rejection semantics, and direct system dispatch.
- [x] 2.3 Implement reducer-owned context restoration, Navigate/Adjust interpretation, PatchMain-to-Utility and both side-surface round trips, mode reset on Return, and audio-neutral interaction transitions.
- [x] 2.4 Migrate PATCH Engine, ADSR, preset, effect, and MIXER scalar adjustment logic to resolve the active stable path through the shared resolver without changing structural lifecycle or scalar publication behavior.
- [x] 2.5 Apply deterministic path repair atomically after committed descriptor/dependency changes, including engine replacement, and prove rejected/stale/busy lifecycle events leave valid later semantic actions processable.

## 3. Semantic projection and exact schemas

- [x] 3.1 Add `SemanticControlViewModel`, `SemanticSurfaceViewModel`, and `SemanticGraphicalViewModel` with generation/hash, context/surface/focus/mode/return, valid actions, typed status/errors, canonical summaries, and exhaustive serialized leaf descriptors.
- [x] 3.2 Extend `StateProjector` to derive PATCH controls from `PatchControlId` plus instrument/effect descriptors and MIXER controls from stable Patch/global descriptors, including SoundFont, Braids, Chorus, lifecycle, dependency, and explicit healthy-empty-error cases without concrete capability branches.
- [x] 3.3 Project PatchUtility and MixerInspector as persistent read-only summaries with one focusable `SurfaceRoot`, leaving their later functional controls and Modal/MultiSelect entry unavailable.
- [x] 3.4 Embed the semantic model in `GraphicalShellProjection` and `StateTree`; derive shell context, status, focus treatment, error presentation, and footer hints only from the embedded model while retaining `PatchPageProjection` and `TextProjection` as read-only compatibility diagnostics.
- [x] 3.5 Advance affected event/state/projection schema versions and exact discovery fixtures, then prove missing, unexpected, duplicate, stale-generation, mismatched-hash, invalid-path, and inconsistent valid-action leaves fail bidirectional verification.
- [x] 3.6 Update `AppLoop` projection storage/accessors and all construction, acceptance, rejection, and structural lifecycle paths so semantic, shell, text, tree, and parameter projections remain generation-coherent and unchanged immutable data can still be shared.

## 4. Passive eframe input and rendering

- [x] 4.1 Change `KeyboardInputTranslator` to emit `SemanticAction`, including exact 1/2/W/S/A/D behavior, K down/up mode actions, focus-loss Navigate cleanup, PATCH Right-to-Utility, and Utility Left/Return, without reading application state.
- [x] 4.2 Change `AppWindow`, `EframeGraphicalApplication`, standalone composition, and test callbacks to accept an injected semantic-action sink wired to `AppLoop::dispatchAction`; keep live-mode mapped input isolated and every adapter free of mutable interaction state.
- [x] 4.3 Render all semantic surfaces, the sole focused path, Navigate/Adjust treatment, typed status/errors, read-only side summaries, and exact valid-action footer hints from the embedded model while retaining the Phase 1 diagnostic workspace and private styling.
- [x] 4.4 Extend post-paint frame observations to correlate semantic context, surface, focus, mode, return path, valid actions, generation, and hash with the painted frame, without promoting geometry or scroll position into canonical state.
- [x] 4.5 Prove the same immutable model at 1920×1080 and 1280×800 preserves all semantic paths and actions while the existing five regions remain visible, ordered, bounded, non-overlapping, and responsive.

## 5. Deterministic demos and live composition

- [x] 5.1 Migrate deterministic `DemoScene`, exhaustive coverage, checkpoints, reports, and action/event surface counts to the semantic action/path schemas while retaining exact current scalar, structural, audio, rejection, and mutation evidence.
- [x] 5.2 Extend `LiveDemoScene` and `LiveDemoRunner` with bounded production-path traversal of two contexts, four surfaces, Navigate/Adjust, both return round trips, exact valid actions, responsive focus invariance, healthy empty errors, and one genuine descriptor-commit focus recovery.
- [x] 5.3 Extend live checkpoints/report serialization with semantic path, mode, return, valid-action, status/error, recovery, and rendered-frame correlations; require measured physical audio continuity and retain every existing scalar/structural lifecycle obligation.
- [x] 5.4 Add deterministic controlled cases for typed Failed projection and successful recovery without fallback, while keeping the physical live scene healthy and free of injected failures.
- [x] 5.5 Add exclusive `--demo-live-semantic-view-model`, retained `--demo-live-graphical-shell`, and compatibility `--demo-live` parsing/composition; add `make demo-live-semantic-view-model`, retain the Phase 1 target, and point `make demo-live` to the new cumulative scene.
- [x] 5.6 Emit `CREST_SEMANTIC_VIEW_MODEL_LIVE_OBSERVATION` only after semantic note cleanup, zero active notes, window return, stream release, worker shutdown, and graph draining; suppress completion on timeout, early close, stale evidence, silence, or nonzero parent exit.

## 6. Behavioral verification

- [x] 6.1 Add `tests/semantic_graphical_view_model.rs` using public production seams to assert exact action/event mapping, stable paths, descriptor-polymorphic content, four surfaces, two reachable modes, two returns, exact actions, status/errors, focus recovery, responsive invariance, generation coherence, and audio neutrality before its acceptance marker.
- [x] 6.2 Update `patch_page_projection`, `eframe_context`, `graphical_application_shell`, `schema_surface`, and control-path tests for semantic action dispatch, exact new schemas, real egui input/application updates, and absence of adapter-owned focus or mode.
- [x] 6.3 Update capability, engine-selection, preset, effect, envelope, performance, exhaustive/live-demo, production-runtime, and mutation fixtures for the new canonical interaction types while preserving all prior falsifiable assertions and callback constraints.
- [x] 6.4 Add negative assertions for positional/path drift, duplicate or impossible valid actions, invalid context/surface combinations, unreachable Modal/MultiSelect, adapter focus repair, stale return origins, missed K release/focus loss, and layout-driven state mutation.
- [x] 6.5 Keep hard-real-time callback measurements at zero allocation, locking, blocking, I/O, logging, panic, and destruction, and prove semantic-only transitions publish no discrete, scalar, or structural audio change.

## 7. Completion gates

- [x] 7.1 Run `cargo fmt --all -- --check`, `cargo check --all-targets`, strict Clippy, and `cargo test --all-targets`; fix every regression without ignored tests, environment-dependent skips, or weakened exact-schema assertions.
- [x] 7.2 Run `make demo` plus the named semantic-view-model, graphical-shell, patch-page, schema, egui, exhaustive, live-scene, runtime, and mutation acceptance targets; confirm their exact markers occur only after assertions.
- [x] 7.3 On a supported physical system, run `make demo-live-semantic-view-model` to normal completion and verify visible semantic traversal, finite nonzero physical audio, exact structured evidence, zero active notes, closed window, released stream, drained worker/graphs, and zero command exit.
- [x] 7.4 Re-run the retained `make demo-live-graphical-shell` target to confirm its Phase 1 entry point and obligations remain stable after the cumulative alias moves forward.
