# Change Summary

## Outcome

- **Problem:** Roadmap Phase One requires an authored graphical frame, but production currently exposes a single text-only window contract.
- **Result:** PATCH and MIXER render in one immutable five-region shell at desktop and Steam Deck sizes while reducer, diagnostic, audio, and teardown behavior remain intact.

## Change Outline

- **Adds:** `GraphicalShellProjection`, post-paint `ShellFrameObservation`, responsive shell composition, headless graphical acceptance, and retained `make demo-live-graphical-shell`.
- **Changes:** `AppWindow` consumes the graphical projection and reports painted-frame evidence; `EframeGraphicalWindow` replaces the text adapter; `make demo-live` aliases the new cumulative scene.
- **Removes:** The production “single complete text view” requirement; its complete text remains nested read-only diagnostic content.

## System Impact

- **Capabilities:** Adds `graphical-application-shell`; modifies `one-way-parameter-control`, `schema-driven-patch-page`, `observable-demo-scene`, and `live-observable-demo`.
- **Architecture:** Adds `goal.use_graphical_shell`, `valueObject.Control.GraphicalShellProjection`, and `valueObject.Shell.ShellFrameObservation`; changes `port.Shell.AppWindow`, `adapter.EframeGraphicalWindow`, `applicationService.Control.AppLoop`, and `applicationService.Testing.LiveDemoRunner` relationships.
- **Interfaces/data:** Advances exact projection/state-tree schemas; adds matching `egui_extras`; leaves `AppState`, semantic events, real-time transports, prepared graphs, and audio callback contracts unchanged.

## Delivery

- **Implementation:** Build canonical projection/schema first, migrate the passive window and responsive layout, wire standalone/live evidence, then update behavioral tests.
- **Validation:** Require strict schema/projection tests, real egui frames at 1920×1080 and 1280×800, all existing gates, deterministic demo regression, and an actual physical `make demo-live-graphical-shell` run with complete teardown and zero exit.

## Risks and Decisions

- **Key decisions:** Pixel geometry stays adapter-boundary evidence; both view projections derive from one snapshot; teardown success is measured before the marker and parent completion is proven by the command exit code.
- **Risks:** Transitional diagnostic content may resemble final UI, responsive paint assertions may become brittle, and physical acceptance is environment-dependent; the design constrains styling, tests semantic rectangles rather than primitive counts, and forbids skips or substitutes.
