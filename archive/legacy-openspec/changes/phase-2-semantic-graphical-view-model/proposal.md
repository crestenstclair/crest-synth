## Why

The Phase 1 shell can place PATCH, MIXER, Utility, Inspector, and footer regions, but it still exposes transitional labels and positional navigation rather than one semantic graphical contract that multiple layouts can render safely. Phase 2 is the required bridge between canonical application state and later reusable components/pages: focus, mode, return, valid actions, status, errors, and descriptor-derived content must become explicit before UI composition grows.

## What Changes

- Add one immutable, host-neutral `SemanticGraphicalViewModel` for PATCH and MIXER, including four stable surfaces, one semantic `FocusPath`, explicit interaction mode and return path, exact valid actions, lifecycle status, typed errors, and descriptor-derived control content.
- Replace index-based MIXER selection and parallel PATCH focus fields with reducer-owned stable focus/control identities plus remembered root paths and deterministic schema recovery.
- Add a closed `SemanticAction` boundary between physical/passive UI input and `AppEvent`; future components remain passive and `AppState::apply` remains the only mutation path.
- Make Edit press/release project `Adjust`/`Navigate`, support exact Utility/Inspector entry and return, and derive footer hints solely from actions valid at the current focus.
- Evolve `GraphicalShellProjection` to embed the semantic model while retaining the read-only diagnostic projection and the existing eframe/egui + `egui_extras` stack.
- Prove layout-independent focus at desktop and Steam Deck viewports and reducer-owned next-before-previous recovery when a committed descriptor/dependency change removes a focused target.
- Add deterministic status/error/recovery coverage and cumulative `make demo-live-semantic-view-model`; retain `make demo-live-graphical-shell`, and point `make demo-live` to the new physical scene.
- **BREAKING**: change the keyboard/window input sink from `AppEvent` to `SemanticAction`, replace positional `InteractionState` focus fields, and change the serialized semantic/shell/state-tree schemas.

## Capabilities

### New Capabilities

- `semantic-graphical-view-model`: Defines canonical semantic actions, focus/mode/return state, descriptor-driven multi-layout view data, valid actions, status/errors, deterministic recovery, and its headless plus physical evidence.

### Modified Capabilities

- `one-way-parameter-control`: Inserts the explicit `SemanticAction` stage, reducer-owned mode/surface transitions, and semantic projection into the existing commit-before-project/publish path.
- `graphical-application-shell`: Makes the passive window consume a shell embedding the canonical semantic model and emit only semantic actions.
- `schema-driven-patch-page`: Moves PATCH focus into `FocusPath`, adds the specified PATCH Main-to-Utility transition/return, and makes schema-change recovery deterministic.
- `live-observable-demo`: Adds the cumulative physical semantic-model traversal and evidence while retaining every scalar, structural, audio, and teardown obligation.

## Impact

- Control/application: `InteractionState`, `AppEvent`, action dispatch, reducers, focus resolvers, `StateProjector`, `AppLoop`, serialization, and schema descriptors.
- UI/shell: `KeyboardInputTranslator`, `AppWindow`, `EframeGraphicalWindow`, `GraphicalShellProjection`, frame observations, footer/status rendering, and responsive tests.
- Verification: exhaustive/demo/live scene schemas and counts, a new `semantic_graphical_view_model` integration target, new CLI/Make live target, and cumulative physical evidence.
- Existing instrument/effect descriptors, prepared graphs, discrete/scalar/structural transports, render path, eframe/egui dependencies, and hard-real-time callback contract remain unchanged.
