## Why

The current exhaustive demo proves Crest Synth headlessly, but a user cannot launch one command and observe the same production state and audio path operating through the real standalone window and physical output. Phase 1 adds that human-observable proof now, while the current SoundFont/text-interface slice is still small enough to exercise exhaustively and before later roadmap capabilities broaden the surface.

## What Changes

- Add a separate `make demo-live` / `--demo-live` mode that opens the normal eframe window, physical CPAL output, fixed SoundFont, and existing Corridors of Time MIDI fixture.
- Add a paced control-side live scene derived from the current typed parameter descriptors. It navigates and changes every installed Patch parameter and every global parameter through `AppLoop`, includes an accepted/rejected recovery case, and leaves each accepted edit visible and audible for the declared dwell.
- Add canonical live checkpoint and report values correlating the planned input, expected transition, production event record, accepted generation, projected value, emitted effects, and generation-tagged audio observation.
- Add fixed-size mixer and audio observation values plus a dedicated latest-value callback-to-control port and atomic adapter. Callback publication remains bounded, lock-free, allocation-free, nonblocking, and free of logging, formatting, I/O, and destruction.
- Finish the live scene by dispatching Patch-scoped semantic all-notes-off events, waiting for a newer zero-active-note observation, emitting the final event log/state tree/coverage/summary on the control side, and leaving the final canonical UI visible until the user closes the window.
- Add deterministic-clock integration coverage for live pacing, exact current-surface coverage, checkpoint agreement, observation correlation, rejection recovery, cleanup, and inert completion without requiring a CI window or physical device.
- Preserve the existing `make demo` command, deterministic trace, schema universe, controlled mutants, and all current project gates unchanged.
- Explicitly exclude Patch-page, ADSR, preset-selection, Plaits, per-Patch effects, modulation, dynamic-graph, and replacement-interface work from this change.

## Capabilities

### New Capabilities

- `live-observable-demo`: Covers the real-window/physical-audio live mode, paced production-path scene, typed checkpoints and final report, exact editable-parameter coverage, semantic note cleanup, and persistent final UI state.

### Modified Capabilities

- `realtime-execution`: Adds bounded callback-local mix measurements and a coherent latest-value audio-observation transport from the callback to the control side.
- `one-way-parameter-control`: Extends the shared semantic event/reducer/projection contract to autonomous live-demo actions and their exact generation-correlated checkpoints.
- `automatic-test-midi`: Requires live mode to use the existing fixture and advance it through exactly one owner so each due event is dispatched once.
- `observable-demo-scene`: Requires the new interactive mode to coexist without changing the current deterministic headless command, coverage, outputs, or falsification gates.

## Impact

- New canonical resources: `valueObject.Mixer.MixObservation`, `valueObject.RealTime.AudioObservationSnapshot`, `port.RealTime.AudioObservation`, `adapter.AtomicAudioObservation`, `valueObject.Testing.LiveDemoScene`, `valueObject.Testing.LiveDemoCheckpoint`, `valueObject.Testing.LiveDemoReport`, and `applicationService.Testing.LiveDemoRunner`.
- Changed owners: `domainService.Mixer.MixEngine`, `applicationService.RealTime.AudioRenderer`, `port.Shell.AppWindow`, `applicationService.Shell.StandaloneApplication`, `adapter.EframeTextWindow`, `adapter.CpalAudioOutput`, the standalone CLI, library exports, Makefile, and integration-test assets.
- New public entry point: `make demo-live`, backed by mutually exclusive `--demo-live` CLI parsing.
- New test target: `tests/live_demo_scene.rs`; current format, clippy, all-target, smoke, headless-demo, schema, egui-context, and mutation gates remain required.
- No new third-party dependency, synthesis engine, effect processor, product transport, persistent state, or UI architecture is introduced.
