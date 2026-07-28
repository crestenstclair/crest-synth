## 1. Canonical shell projection

- [x] 1.1 Add `GraphicalShellProjection` with typed context/status, identity, workspace/side-region, footer, nested diagnostic, generation/hash accessors, serialization, stable leaf descriptors, and unit tests for PATCH and MIXER.
- [x] 1.2 Extend `StateProjector` to derive Patch page, retained text, graphical shell, state tree, and parameter snapshot from the same accepted snapshot without capability-specific or adapter-specific branches.
- [x] 1.3 Add the graphical shell to `StateTree` and exact schema discovery, advance affected schema versions, and assert missing, unexpected, stale-generation, and mismatched-context shell leaves fail.
- [x] 1.4 Store and publish `currentGraphicalShell` through every `AppLoop` construction, accepted/rejected dispatch, and structural lifecycle path while retaining `currentText` for diagnostic and verification consumers.

## 2. Passive graphical window

- [x] 2.1 Add matching `egui_extras` configuration, replace the text-window module/export with `EframeGraphicalWindow` and `EframeGraphicalApplication`, and keep alternate GUI/component runtimes out of the dependency graph.
- [x] 2.2 Change `AppWindow` to request immutable `GraphicalShellProjection` and emit post-paint `ShellFrameObservation` values through an injected callback; update its contract tests to reject mutable or stale projection behavior.
- [x] 2.3 Render the context line, identity header, active main workspace, persistent Utility/Inspector, and footer with private Phase One styling, retaining the complete scrollable diagnostic and selected-line behavior in the workspace.
- [x] 2.4 Implement the authored 1920×1080 bands and 1500/420 split plus the 1280×800 constrained layout with a side region of at least 320 px; emit finite named region rectangles/visible-label evidence only after the production frame paints.
- [x] 2.5 Preserve shared key/focus normalization, semantic event emission, tick ordering, 16 ms idle repaint scheduling, nonfatal rejection behavior, and one-shot viewport close without adding adapter-owned context or focus state.

## 3. Standalone and live composition

- [x] 3.1 Migrate normal, smoke, deterministic demo, and live standalone composition to the graphical projection/frame callbacks while preserving the existing reducer, worker, structural handoff, audio transports, device order, and teardown ownership.
- [x] 3.2 Extend live scene coverage, checkpoints, runner, and report serialization to correlate qualifying rendered PATCH/MIXER frames and all five regions with the exact shell generation and finite nonzero bounded production-path audio observation; source it from the device callback in physical composition and the same renderer/transport in deterministic verification, and reject planned, stale, overlapping, hidden, or silent frame evidence.
- [x] 3.3 Add the exclusive `--demo-live-graphical-shell` option, keep `--demo-live` as its compatibility alias, add the retained Make target, and point `make demo-live` at the newest cumulative scene without changing `make demo`.
- [x] 3.4 Emit one measured `CREST_GRAPHICAL_SHELL_LIVE_OBSERVATION` only after semantic note cleanup, window return, stream release, worker shutdown, and graph draining; rely on the command's real exit status for parent completion and suppress success evidence on every incomplete path.

## 4. Behavioral verification

- [x] 4.1 Migrate existing eframe-context, Patch-page, capability, envelope, control-performance, exhaustive-scene, live-scene, and standalone fixtures to the graphical window callback while continuing to assert the retained diagnostic values.
- [x] 4.2 Extend `schema_surface` and related projector/state-tree tests to require the exact bidirectional `GraphicalShellProjection` surface alongside all prior event, state, text, and parameter leaves; verify `ShellFrameObservation` geometry and projection identity in the dedicated adapter tests rather than treating pixel data as canonical state.
- [x] 4.3 Add `tests/graphical_application_shell.rs` using real egui `RawInput` and the production update callback to assert both contexts, both reference viewports, every region's identity/order/bounds/non-overlap, coherent generation/hash, real semantic dispatch, selected diagnostic, and audio-neutral context switching before printing its marker.
- [x] 4.4 Extend deterministic live tests with qualifying PATCH/MIXER frame observations plus controlled stale, helper-only, missing-region, overlap, silent-audio, early-close, timeout, and post-completion cases that cannot fabricate shell coverage or teardown success.
- [x] 4.5 Keep production runtime, real-time allocation/destruction, mutation, DSP, physical-device health, and existing live lifecycle gates passing after the window-port migration.

## 5. Completion gates

- [x] 5.1 Run formatting, `cargo check --all-targets`, strict Clippy, and `cargo test --all-targets`; fix every regression without weakening existing assertions or adding environment-dependent skips.
- [x] 5.2 Run the existing deterministic `make demo` and all named shell/schema/live acceptance targets, confirming exact success markers and unchanged headless behavior.
- [x] 5.3 On a supported physical system, run `make demo-live-graphical-shell` to completion and verify visible PATCH/MIXER regions, audible fixture output, exact structured shell evidence, zero active notes, window close, stream release, worker/graph cleanup, and zero command exit.
