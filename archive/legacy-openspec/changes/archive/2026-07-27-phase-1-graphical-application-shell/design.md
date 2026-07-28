## Context

The production shell currently passes `TextProjection` directly to a minimal eframe window. That has been useful as a diagnostic, but it cannot represent the structural frame required by roadmap Phase One: context/status, identity, main workspace, persistent Utility/Inspector, and footer. The rest of the application is already deliberately one-way and must remain so: physical input becomes a semantic event, `AppState::apply` is the only mutation point, immutable projections are derived after commit, and audio work crosses separate bounded transports.

`DESIGN.md` is authoritative for the visual hierarchy. Its authored desktop composition is 1920×1080 with 48 px context, 72 px identity, 896 px workspace, a 1500/420 main-to-side split, and a 64 px footer. The same hierarchy must remain usable at 1280×800. The current text projection and all existing controls must remain observable until later roadmap phases replace the workspace blockout with functional graphical surfaces.

## Goals / Non-Goals

**Goals:**

- Establish one production graphical window boundary and one immutable shell projection.
- Render all five authored regions for PATCH and MIXER at both reference viewports.
- Preserve the reducer, input translator, Patch focus, diagnostic projection, audio projections, and hard-real-time rules unchanged.
- Make region geometry, projection coherence, live visibility, and teardown falsifiable through named tests and a retained physical live scene.
- Select eframe/egui plus matching `egui_extras` as the UI stack while keeping Crest's behavioral contracts independent of those types.

**Non-Goals:**

- Phase Two semantic focus paths, interaction modes, valid-action models, or return paths.
- Phase Three effect-slot and bus topology expansion.
- Phase Four public tokens, reusable controls, component gallery, or final styling system.
- Functional Patch editor, Mixer, meters, faders, waveform rendering, Sample Browser, or sample playback behavior.
- Any new UI-owned domain, navigation, graph, audio, or lifecycle state.

## Decisions

### 1. Add a host-neutral shell projection above the retained diagnostic

`StateProjector` will derive `GraphicalShellProjection` from the same accepted snapshot used for `PatchPageProjection`, `TextProjection`, `StateTree`, and `ParameterSnapshot`:

```text
physical input → semantic AppEvent → AppState::apply
                                      │
                                      └─ accepted StateSnapshot
                                           ├─ PatchPageProjection
                                           ├─ TextProjection (diagnostic)
                                           ├─ GraphicalShellProjection → AppWindow
                                           ├─ StateTree
                                           └─ ParameterSnapshot → audio transport
```

The shell value carries generation, state hash, top-level context, context/status labels, identity labels, main/side region identities, footer labels/hints, and the retained `TextProjection`. It carries no egui values, rectangles, widget IDs, callbacks, mutable state, or runtime ownership. The nested diagnostic must share context, generation source, and state hash with its parent.

This keeps structural labels out of the adapter and prevents separate PATCH/MIXER UI state. Passing mutable state or letting the adapter derive product meaning was rejected because either would create a second behavioral authority. Keeping `TextProjection` as the window contract was rejected because it cannot express or verify the Phase One hierarchy.

### 2. Break the window port once, at the projection boundary

`AppWindow::run` will request `GraphicalShellProjection`, emit an immutable post-paint `ShellFrameObservation` through an injected control-side callback, and otherwise retain its existing input/tick lifecycle; `AppLoop` will expose `currentGraphicalShell`; and the production adapter becomes `EframeGraphicalWindow`. `currentText` remains available to diagnostics and verification but is no longer callable by the production window port. The frame observation carries only viewport, projection identity, and named painted rectangles/labels; it is evidence, not canonical application state.

The adapter remains passive: it normalizes egui key/focus input through `KeyboardInputTranslator`, emits only `AppEvent`, advances the injected control-side tick, requests the latest immutable shell projection, and paints it. It must not retain context, Patch focus, selection, runtime status, or live-demo state. A native immediate-mode implementation was chosen because the project already uses eframe/egui and its frame model fits immutable projections. An additional retained-mode GUI or component framework was rejected because it would duplicate runtime and ownership concepts before Crest's Phase Four component boundary exists.

### 3. Compose the shell with adapter-local responsive geometry

The graphical adapter will use egui top/bottom panels for the context line, identity band, and footer; a persistent right side region for Utility or Inspector; and the remaining central area for the active workspace. `egui_extras` layout utilities may divide bands and host future image/SVG assets, but no third-party component API becomes part of Crest's contracts.

At 1920×1080 the frame uses the authored 48/72/896/64 vertical composition and 1500/420 workspace split. At 1280×800 it retains all bands and assigns the side region at least 320 px; the main region receives the remainder. Width is otherwise proportional and clamped, labels truncate or wrap within their own region, and no required region is hidden or overlaid. Pixel geometry is not canonical state. After paint, the renderer emits one adapter-boundary `ShellFrameObservation` of named rectangles and visible labels; headless tests inspect it directly and the live runner correlates it with the canonical projection and bounded production-path audio observation without gaining window ownership. Physical composition sources that observation from the device callback, while the deterministic harness uses the same renderer and transport without claiming physical-device acceptance.

A single scrollable diagnostic in the central workspace preserves current visibility and selected-line behavior. Rich page widgets were rejected for this phase because they depend on the Phase Two semantic view model and Phase Four components.

### 4. Keep Phase One styling deliberately private and shallow

The adapter may use the dark canvas/surface/panel colors, mono typography, hairlines, and context accents already established by `DESIGN.md`, but these remain private shell paint constants. It will not publish reusable parameter rows, focus frames, tokens, or component traits. This allows the structural blockout to look intentional without prematurely freezing the Phase Four API.

### 5. Extend exact schema and headless egui evidence

`GraphicalShellProjection` and its stable leaf descriptor join the exact bidirectional schema surface. Existing context/input tests will use a real egui `RawInput` frame wired to `AppLoop.dispatch`, then compare the next frame, `EventRecord`, `StateTree`, shell projection, nested diagnostic, and parameter snapshot to the same generation.

A new `graphical_application_shell` integration target renders 1920×1080 and 1280×800 frames through the production update path. It asserts the identity, order, containment, visibility, minimum bounds, and non-overlap of all five regions; PATCH→Utility and MIXER→Inspector mapping; audio-neutral context switching; and absence of adapter-owned context/focus state. Inspecting only a layout helper or supplied mock projection is insufficient.

### 6. Retain a separate physical graphical-shell witness

`make demo-live-graphical-shell` runs the release binary with the normal eframe window, CPAL stream, real SoundFont/Braids/Chorus composition, and Corridors fixture. The control-side scene visibly holds PATCH and MIXER long enough for rendered-frame observations to credit every region and both contexts, while preserving the existing scalar and structural proof.

After semantic all-notes-off and zero active notes, the owner closes the window, releases the stream, shuts down the worker, and drains graph ownership off callback. Only then does the binary emit one `CREST_GRAPHICAL_SHELL_LIVE_OBSERVATION` containing measured region/context, physical-audio, cleanup, close, stream, and graph fields. The witness command's actual zero exit independently proves normal parent completion. `make demo-live` aliases this newest cumulative scene; prior phase-specific targets remain stable.

## Risks / Trade-offs

- **[Transitional text can look like the final product]** → Label and style it as diagnostic workspace content, and keep functional screen work explicitly assigned to later phases.
- **[Two projections can drift]** → Derive both in one projector call from one snapshot, store the shell in `StateTree`, compare generation/hash and all leaves exactly, and reject adapter-side derivation.
- **[Immediate-mode layout tests can become paint-order fragile]** → Assert named region observations plus essential tessellated labels and geometry, not incidental primitive counts.
- **[1280×800 can squeeze content]** → Clamp the side region at 320 px, retain every structural band, and allow only local text truncation/wrapping or diagnostic scrolling.
- **[A private Phase One palette may be replaced in Phase Four]** → Keep it adapter-local and expose no stable token/component API yet.
- **[Physical UI/audio evidence is environment-dependent]** → Keep deterministic headless behavior mandatory, and require the separate physical target to be run by the implementer as the final phase gate; never skip or substitute it.

## Migration Plan

1. Add `GraphicalShellProjection`, its projector logic, schema descriptor, `StateTree` field, and `AppLoop` accessor while retaining all text APIs.
2. Change `AppWindow` and the eframe adapter to the new projection, then update standalone composition and the binary adapter name.
3. Add responsive shell rendering and migrate existing diagnostic/selected-line behavior into the central workspace.
4. Update schema, egui, demo, and lifecycle tests; add the new headless acceptance target and live command.
5. Run all existing gates, then run `make demo-live-graphical-shell` on a supported physical system and retain its structured result.

Rollback is the inverse adapter/port migration: restore `TextProjection` as the window callback and remove only the new shell projection, dependency, target, and test. No persisted domain or audio data requires migration because this change adds presentation projection only.
