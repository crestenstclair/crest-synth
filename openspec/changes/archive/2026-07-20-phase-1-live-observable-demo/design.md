## Context

Crest Synth already has a deterministic exhaustive scene that drives production input translation, `AppLoop`, projections, the real-time boundary, SoundFont rendering, and mixing without opening a native window or physical device. That path is strong machine evidence but does not demonstrate the same system to a person in the real standalone application.

Phase 1 adds a separate live mode to the current SoundFont/text-view slice. The evaluated CUE architecture introduces eight canonical resources across Mixer, RealTime, and Testing and extends the existing Shell composition. `DESIGN.md` remains authoritative: `AppState::apply` is the only mutation path; views render immutable projections; callback work is fixed-capacity and cannot allocate, lock, block, log, perform I/O, panic, or destroy state; meters and health data travel through a transport separate from commands and parameters.

The implementation must be reconciled directly against the evaluated CUE declarations, with OpenSpec sequencing and verifying the work. Legacy crest-spec tooling is not part of this change's implementation workflow.

## Goals / Non-Goals

**Goals:**

- Launch one `make demo-live` command that opens the production eframe window and physical CPAL stream with the existing SoundFont and MIDI fixture.
- Exercise every current editable parameter instance through semantic events, the production reducer, canonical projections, and normal audio publication with human-observable pacing.
- Correlate each declared step with its precomputed expectation, production event record, state generation, projected value, emitted effects, and a newer audio observation for that generation.
- Publish callback observations through a dedicated coherent latest-value transport without weakening hard real-time constraints.
- Emit complete control-side live evidence, stop active notes semantically, and leave the final canonical UI visible until user close.
- Provide deterministic automated proof of the live orchestration contract and preserve every existing headless and project gate.

**Non-Goals:**

- Replacing or weakening the exhaustive headless demo, its coverage universe, controlled negatives, or output schema.
- Adding a second reducer, UI-owned state, direct engine/mixer mutation, offline-only live substitute, null physical-device fallback, or callback logging.
- Adding live hardware MIDI, transport controls, sequencing, persistence, a new engine/effect, Patch-page behavior, ADSR, preset browsing, Plaits, modulation, dynamic graphs, or the Figma replacement UI.
- Claiming deterministic physical-device audio comparisons. Exact causal DSP proof remains in the existing offline/headless path; the live path provides generation-correlated measurements and human observation.

## Decisions

### 1. Add a distinct interactive mode while reusing the normal composition

`--demo-live` is mutually exclusive with `--smoke`, `--observe`, `--demo-scene`, and verification-only degenerate flags. It calls `StandaloneApplication::run_live_demo` using the same `EframeTextWindow`, `CpalAudioOutput`, `CorridorsMidiEventSource`, `AppLoop`, `AudioRenderer`, SoundFont engine, and mixer used by normal interactive startup.

This avoids turning the deterministic demo into an environment-dependent test and prevents a parallel product path. An alternative was to add native-window/device options to `make demo`; that would make its current repeatability and falsification contract conditional, so it is rejected.

### 2. Model live behavior as a control-side state machine

`LiveDemoScene` is a bounded plan constructed after fixture Patches are installed. `LiveDemoRunner` advances it incrementally from monotonic window ticks, dispatches at most one due autonomous `AppEvent` per tick through `AppLoop::dispatch_from`, and returns optional `LiveDemoCheckpoint` values to the shell. It does not sleep, block the UI thread, own `AppState`, or call the reducer, projector, engine, mixer, parameter publisher, or command queue directly.

In live mode the runner is the single owner of `AutomaticMidiTest::tick`; the shell must not advance the fixture separately. This prevents duplicate due events without introducing transport state. A background thread was considered but rejected because it would add another event producer and synchronization boundary for no benefit.

### 3. Derive the scene and coverage from production parameter descriptors

The scene freezes an ordered unique expected set from the `ChannelParameters` and `GlobalParameters` descriptors plus installed `PatchId`s before dispatch. Each parameter instance receives a valid accepted adjustment, a checkpoint, at least 500 ms dwell, and an audible-observation requirement. Navigation is also expressed as semantic events. A boundary adjustment supplies the required rejection, followed by a valid edit to prove recovery.

The expected set never comes from the observed report, UI text, or post-dispatch state. Missing, unexpected, duplicated, unchanged, unprojected, or inaudible parameter identifiers make completion false. A hand-maintained parameter-name list was rejected because it would drift independently from the production surface.

### 4. Keep expected and actual checkpoint data separate

Before dispatch, the runner computes and freezes the expected outcome, generation transition, selected value, projection value, parameter generation, and effect descriptors from the prior canonical state and the owning descriptor. After dispatch it obtains actual data from the new `EventRecord`, `StateTree`, `TextProjection`, `ParameterSnapshot`, and `AudioObservationSnapshot`.

`LiveDemoCheckpoint` is the one canonical correlation value. `LiveDemoReport` owns the checkpoint list, existing `EventLog`, final existing `StateTree`, bidirectional coverage sets, completion result, and derived human-readable summary. The runner returns data; callbacks injected by the standalone composition root own stdout markers. This keeps I/O out of application and audio services.

### 5. Measure mixer-owned stages without leaking their buffers

`MixEngine` produces a fixed-size `MixObservation` from its own reverb-input, delay-input, wet-return, and final-output buffers while processing the normal block. The value contains finite numeric peaks/RMS values plus non-finite and clipping counts and cannot affect the mixed output.

`AudioRenderer` combines that value with its bounded command and prepared active-note observations into `AudioObservationSnapshot`, tagged with the exact consumed `ParameterSnapshot` generation and monotonically increasing block sequence. Having the renderer inspect mixer scratch buffers was rejected because it would violate ownership; synthesizing measurements from the final output alone was rejected because sends and wet-input behavior would be ambiguous.

### 6. Use a separate latest-value callback-to-control transport

`AudioObservation` is distinct from the ordered `AudioCommand` ring and control-to-audio `ParameterSnapshot` publication. `AtomicAudioObservation` exposes separate callback-write and control-read handles and publishes a complete fixed-size snapshot using a bounded coherent atomic scheme. Floating-point measurements are stored as bit patterns. The callback overwrites stale observations rather than queueing or backpressuring.

The implementation must prove that the control reader cannot combine fields from different publications and that callback publication performs no allocation, locking, blocking, logging, formatting, I/O, or destruction. Reusing the command ring or parameter triple buffer was rejected because those transports have different direction, ownership, and delivery semantics.

### 7. Make completion explicit and non-closing

When all parameter checkpoints complete, the runner dispatches one Patch-targeted semantic all-notes-off MIDI event per installed Patch through `AppLoop`. It waits for a newer observation reporting zero active notes, captures the final event log and state tree, verifies exact coverage and no dropped records, and becomes inert. The shell invokes completion output exactly once and continues rendering `AppLoop::current_text` until user close.

If the user closes early, the shell attempts semantic note cleanup while the control loop is available, does not emit a successful final report, and returns a typed incomplete result. Auto-close and silent success are rejected because they would contradict the human-observable completion contract.

### 8. Separate deterministic contract proof from the physical acceptance run

`tests/live_demo_scene.rs` drives the public runner with a deterministic monotonic clock, production reducer/projections/renderer, an actual `AtomicAudioObservation`, and a frame-observation harness without opening a native CI window or device. It proves pacing, exact coverage, checkpoint equality, rejection recovery, semantic cleanup, zero active notes, one completed report, and inert post-completion behavior.

The physical `make demo-live` run separately proves device negotiation, visible pacing, and audible playback. It must fail clearly on missing assets or device/window errors; there is no silent or headless fallback. The existing headless demo remains the exact causal/offline audio proof.

## Risks / Trade-offs

- **[Physical fixture audio changes over wall time, so raw before/after energy is not deterministic]** → Tag observations with accepted parameter generation and block sequence, record fixture position, use stage-appropriate metrics, and retain paired deterministic headless DSP proof as the causal gate.
- **[A long fixture/live scene can overflow the interactive event journal]** → Freeze the scene first, pre-size a bounded journal capacity for scene plus expected fixture traffic, surface dropped-record counts, and fail report completion on any drop.
- **[An incoherent atomic read could associate measurements from different blocks]** → Use one proven coherent publication protocol, split audio/control handles, and add concurrency plus allocation instrumentation tests.
- **[Live work could stall the eframe thread]** → Advance incrementally, dispatch at most one due action per tick, never sleep in the runner, and request normal repaint/projection flow.
- **[Active-note observation could become a second engine state]** → Keep it as a bounded note-lifecycle observer updated from commands already dispatched by the renderer; it never drives SoundFont behavior and global/Patch all-notes-off clears it with bounded work.
- **[New renderer observation work could regress the existing callback]** → Keep calculations single-pass and bounded, run the allocator/real-time contract tests, and require all existing smoke, demo, schema, mutation, and project validations.
- **[The implementation overlaps an already dirty worktree]** → Capture the baseline, preserve unrelated user changes, edit only the CUE-declared dependency frontier, and review scoped diffs before closure verification.

## Migration Plan

1. Re-evaluate the changed CUE package and inspect its scoped diff; require the eight declared new resources and only the intended dependent owners.
2. Implement the evaluated CUE architecture in dependency order: mixer observation; real-time snapshot/port/adapter; renderer integration; live scene/checkpoint/report/runner; shell/window/CLI/Makefile/test assets.
3. Run affected local validation after each dependency group and resolve only attributable failures while preserving the canonical architecture and existing user changes.
4. Run the dedicated live-demo integration target, then every pre-existing format, clippy, all-target, smoke, exhaustive-demo, schema-surface, egui-context, mutation, and real-time check.
5. Run `make demo-live` with the repository assets and a physical output, confirm visible/audible completion and final-state persistence, then close the window manually.

Rollback restores the Phase 1 CUE declarations and their matching scoped implementation changes together. There is no persisted-data migration. The ordinary interactive, smoke, and headless-demo entry points remain available throughout.
