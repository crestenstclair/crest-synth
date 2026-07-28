## Why

The roadmap marks Phase One as the point where Crest Synth must stop presenting its production experience as a single text view and establish the authored PATCH/MIXER application frame. Doing that now gives later interaction and component phases a stable graphical boundary without weakening the reducer, projection, real-time, or live-lifecycle contracts already proven by the executable architecture.

## What Changes

- Add a shallow eframe/egui application shell with five visible regions: context/status line, identity header, PATCH-or-MIXER workspace, persistent Utility/Inspector side region, and footer.
- Add one host-neutral `GraphicalShellProjection`, derived with the retained diagnostic `TextProjection` from the same accepted `AppState` generation.
- **BREAKING** Change `AppWindow` to consume `GraphicalShellProjection` instead of `TextProjection`, and replace the text-only eframe adapter with `EframeGraphicalWindow`.
- Select matching `egui_extras` utilities for layout and image/SVG loading while keeping shell behavior, state, and future component APIs owned by Crest.
- Preserve the existing diagnostic text inside the Phase One workspace so all current controls remain observable while functional Patch/Mixer screens are still deferred.
- Require the same shell hierarchy at 1920×1080 and 1280×800, with every region visible, bounded, ordered, and non-overlapping.
- Add `make demo-live-graphical-shell` and a named headless egui contract test proving both contexts, coherent projections, real input dispatch, responsive geometry, physical audio continuity, and complete teardown.
- Keep Phase Two semantic focus/modes, Phase Three routing/effects, Phase Four reusable components, and later waveform/sample behavior out of this change.

## Capabilities

### New Capabilities

- `graphical-application-shell`: Defines the immutable Phase One shell projection, authored regions, responsive reference layouts, passive egui adapter, and graphical-shell acceptance proof.

### Modified Capabilities

- `one-way-parameter-control`: Replaces the single text window contract with a graphical projection while retaining text as a diagnostic and preserving the semantic input → reducer → projection path.

## Impact

- Control projection/state-tree/AppLoop contracts gain `GraphicalShellProjection`; canonical reducer and audio contracts do not change.
- The shell port, eframe adapter, standalone composition, binary options, and Makefile live target change.
- `Cargo.toml` gains matching `egui_extras`; no alternate GUI runtime or third-party component system is added.
- Existing projection, schema-surface, egui-context, live-demo, and lifecycle tests are updated, plus a new `graphical_application_shell` integration target.
- `DESIGN.md` and evaluated CUE declarations record the selected UI stack and the Phase One architectural boundary.
