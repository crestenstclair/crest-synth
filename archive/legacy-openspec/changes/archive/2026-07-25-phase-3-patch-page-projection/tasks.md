## 1. Canonical context and interaction state

- [x] 1.1 Add canonical `TopLevelContext::{Patch, Mixer}` and `InteractionState { context, mixer_selection, patch_focus }` modules, export their single public types, default startup to MIXER, and preserve compatibility accessors only as thin views over InteractionState.
- [x] 1.2 Extend `AppEvent` and its exhaustive typed descriptor with both `SelectContext` payloads; extend EventRecord input serialization/leaf discovery so context payloads are exact and no raw key enters Control.
- [x] 1.3 Migrate `AppState` to InteractionState, initialize stable PatchId focus from the first accepted installed Patch, implement idempotent direct context selection and no-Patch rejection, and keep every transition transactional through `apply`.
- [x] 1.4 Add `ActionUnavailableInContext`, gate Navigate/Adjust to MIXER, prove PATCH attempts are unchanged nonfatal rejections, and update the exact eleven-variant rejection descriptor/table/scene partition.
- [x] 1.5 Update AppState and event tests for default context, stable focus, MIXER selection retention, direct/repeated selection, pre-install rejection, PATCH action rejection, generation behavior, and later-event recovery.

## 2. Host-neutral Patch projection

- [x] 2.1 Extend the canonical VoiceEnvelope surface descriptor with stable presentation labels and units needed by projection, retaining the existing IDs, bounds, steps, ordering, and DSP contract.
- [x] 2.2 Add `PatchPageProjection` and its owned engine-choice, envelope-row, section, and parameter-row data in one Control module, including stable IDs, typed values/assets, metadata, dependency results, read-only flags, state hash, and typed serialized-leaf descriptor.
- [x] 2.3 Implement one generic projector walk from stable PatchId, CapabilityRegistry, active CapabilityDescriptor/InstrumentConfig, and VoiceEnvelope; include every registry choice and prohibit capability-id or SoundFont/Braids field matching.
- [x] 2.4 Make `TextProjection` context-tagged; preserve the complete MIXER wall/selection semantics with direct-page hints and render PATCH losslessly from `PatchPageProjection` in the same single-scroll text shell.
- [x] 2.5 Extend canonical serialized state and StateTree with InteractionState, context-tagged text, and an optional PatchPageProjection present exactly in PATCH; bump versioned schemas and update round-trip/hash/leaf equality tests.
- [x] 2.6 Extend `StateProjector` and `AppLoop` accessors and generation-only sharing so accepted context and MIDI events produce coherent snapshot/page/text/tree/parameter generations without parsing Crest's JSON or rebuilding unchanged context bodies unnecessarily.

## 3. Physical input and basic window

- [x] 3.1 Add Digit1/Digit2 to `WindowKey` and expand its production-owned descriptor to exactly 17 unique values including key-up; update canonicalization and uniqueness tests.
- [x] 3.2 Map Digit1/Digit2 key-down to MIXER/PATCH semantic events and make both key-up paths inert regardless of K state; retain the existing W/S/A/D/K mappings and focus-loss behavior.
- [x] 3.3 Normalize the two digit keys in `EframeTextWindow`, dispatch them through the shared translator/callback, and keep the adapter free of context, Patch focus, descriptor, and mutable projection state.
- [x] 3.4 Extend headless egui tests to select PATCH and return to MIXER through real RawInput and consecutive production update frames, asserting the exact EventLog record, context, PatchId, page/body, hash, retained MIXER selection, and scroll target.

## 4. Audio-neutral one-way behavior

- [x] 4.1 Ensure an accepted context event follows reduce → commit → snapshot/page/text/tree projection → same-value ParameterSnapshot publication, emits no AudioCommand, and never invokes the structural graph boundary.
- [x] 4.2 Add controlled tests proving context selection preserves every Patch/config/envelope/mixer/global value, Patch ordering, Scalar layout, GraphRevision, prepared ownership/acknowledgement state, MIDI routing, and command count while only interaction/generation/view data changes.
- [x] 4.3 Compare before/after context parameter values through the production renderer from identical prepared engine/effect state and require finite sample-identical stereo output with no graph preparation, publication, swap, retirement, or callback allocation/destruction.

## 5. Schema-derived demonstrations and acceptance

- [x] 5.1 Expand DemoScene, DemoSceneReport, exhaustive coverage, binary observations, and typed leaf discovery to exactly 17 normalized inputs, five AppEvent variants, two contexts, eleven rejection variants, and the full InteractionState/PatchPageProjection/TextProjection schema.
- [x] 5.2 Exercise both page keys, exact SoundFont and Braids page projections, PATCH Navigate/Adjust rejection, later MIXER recovery, context-only parameter publication, and exact MIXER baseline restoration through KeyboardInputTranslator and AppLoop without a test-owned engine field list.
- [x] 5.3 Update live-demo, capability-schema, performance, mutation, smoke, serialization, schema-surface, and existing projection fixtures for the versioned context schema while keeping the live editable scene in default MIXER and preserving generation-only MIDI performance/eager equivalence.
- [x] 5.4 Add `tests/patch_page_projection.rs` with ordinary assertion-bearing tests against production providers, reducer/projector/AppLoop, eframe callback path, audio boundary, and renderer; print `CREST_ACCEPTANCE patch_page_projection passed` only after exact page, recovery, no-command/no-graph, and sample-identity assertions pass.

## 6. Full verification

- [x] 6.1 Run the focused unit suites for AppEvent, AppState, interaction, projection, serialization/tree, WindowInput, KeyboardInputTranslator, and EframeTextWindow.
- [x] 6.2 Run `cargo test --test patch_page_projection -- --nocapture`, `cargo test --test schema_surface -- --nocapture`, `cargo test --test eframe_context -- --nocapture`, `cargo test --test exhaustive_demo_scene -- --nocapture`, `cargo test --test live_demo_scene -- --nocapture`, `cargo test --test capability_schema -- --nocapture`, and `cargo test --test control_dispatch_performance -- --nocapture`, requiring every declared marker and exact count.
- [x] 6.3 Run `cargo fmt --all -- --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, `make smoke`, and two fresh `make demo` runs with byte-identical logical evidence.
- [x] 6.4 Evaluate `openspec context --json`, require a healthy `openspec doctor --json`, run `openspec validate phase-3-patch-page-projection --strict` and `openspec validate --all --strict`, then run deterministic OpenSpec change/project acceptance for all 20 declared project checks without weakening prior predicates.
