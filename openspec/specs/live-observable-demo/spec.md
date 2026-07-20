# Live Observable Demo

## Purpose

Define the paced, human-observable production demo and its coherent control, projection, physical-audio, coverage, and cleanup evidence.

## Requirements

### Requirement: Dedicated live standalone entry point
The application SHALL provide `make demo-live`, backed by a dedicated interactive CLI mode, that opens the normal Crest Synth window, physical audio output, fixed SoundFont, and existing Corridors of Time MIDI fixture. The live mode SHALL be mutually exclusive with headless, observation, exhaustive-demo, and controlled-negative modes and SHALL NOT substitute a fake window, null device, offline-only renderer, or silent fallback.

#### Scenario: User launches the live demo
- **WHEN** the user runs `make demo-live` with the repository fixtures and a usable physical output device
- **THEN** the production window and audio stream open, the fixed fixture begins through the normal input path, and audible SoundFont output is produced

#### Scenario: Required live resource is unavailable
- **WHEN** the SoundFont, MIDI fixture, window runtime, or physical audio device cannot be opened
- **THEN** live startup fails with a typed visible error and does not silently run a headless or silent substitute

#### Scenario: Live and headless flags are mixed
- **WHEN** `--demo-live` is combined with any headless, observation, exhaustive-demo, or controlled-negative flag
- **THEN** argument validation rejects the invocation before application startup

### Requirement: Paced production-path scene
The live demo SHALL advance incrementally from control-side window ticks and SHALL express every autonomous navigation, adjustment, fixture, rejection-probe, and note-cleanup action as a semantic event through the same reducer and publication path used by other inputs. It SHALL NOT directly mutate UI, canonical state, projections, engine state, mixer state, audio parameters, commands, or output buffers.

#### Scenario: Autonomous action becomes due
- **WHEN** the live scene dwell expires for its next planned action
- **THEN** at most one due semantic event is dispatched through the production control path and the next visible/audio projections derive from its accepted canonical generation

#### Scenario: Event is rejected at a parameter boundary
- **WHEN** the scene adjusts a parameter beyond an already reached bound
- **THEN** the existing event log records the rejection, state and projections remain unchanged, the window stays open, and the following valid adjustment can be accepted

### Requirement: Exact current editable-surface coverage
Before dispatching its first scene action, the live demo SHALL derive and freeze an ordered unique expected set containing every editable Patch parameter for every installed Patch and every editable global parameter. Every expected parameter instance SHALL receive at least one accepted value change, SHALL remain visibly projected for at least 500 milliseconds, and SHALL receive a generation-correlated audible observation.

#### Scenario: Current parameter surface is exercised
- **WHEN** the scene reaches its coverage-comparison step
- **THEN** exercised identifiers exactly equal the frozen expected identifiers and both missing and unexpected sets are empty

#### Scenario: Parameter is omitted or only visited
- **WHEN** an expected parameter is missing, duplicated, selected without changing, not projected, lacks its dwell, or lacks a qualifying audio observation
- **THEN** the live report is incomplete and identifies the coverage or checkpoint failure

### Requirement: Coherent structured live checkpoints
For each declared scene checkpoint, the live demo SHALL expose a stable structured record containing the input, expectation fixed before dispatch, accepted or rejected outcome, canonical state generation and hash, exact projected value, parameter generation, emitted effects, a newer audio observation, and the parameter-specific measured predicate. Actual post-dispatch values SHALL NOT define their own expectations.

#### Scenario: Accepted parameter checkpoint completes
- **WHEN** an accepted edit has remained projected for its dwell and audio has consumed that parameter generation
- **THEN** one `CREST_LIVE_CHECKPOINT` record reports matching event, state, projection, effect, generation, and audio-observation data

#### Scenario: Projection or audio generation disagrees
- **WHEN** a checkpoint's state tree, visible projection, parameter snapshot, emitted effects, or audio-observation generation does not agree with its production event record
- **THEN** the checkpoint fails and the live report cannot claim completion

### Requirement: Bounded measured audio consequence
Each accepted parameter checkpoint SHALL use finite measurements from the actual physical render path and SHALL require an audio observation whose sequence advanced after dispatch and whose parameter generation equals the accepted generation. Gain and master edits SHALL observe output level, pan SHALL observe stereo balance, sends SHALL observe their corresponding effect input, and shared effect controls SHALL observe wet output.

#### Scenario: Audible observation follows an edit
- **WHEN** the callback renders nonzero fixture audio using the accepted parameter generation
- **THEN** the checkpoint records the relevant finite mixer/output measurement and its declared parameter-specific predicate result

#### Scenario: Audio observation is stale or non-finite
- **WHEN** the latest observation predates the edit, carries another parameter generation, has no required nonzero signal, or reports non-finite output
- **THEN** the checkpoint remains pending or fails rather than crediting the parameter as audibly exercised

### Requirement: Live completion, cleanup, and persistent final state
After all parameter checkpoints, the live demo SHALL dispatch one Patch-targeted semantic all-notes-off event for every installed Patch, wait for a newer observation reporting zero active notes, emit exactly one final event log, state tree, coverage result, and human-readable summary from the control side, become inert, and continue showing the final canonical projection until the user closes the window.

#### Scenario: Scene completes successfully
- **WHEN** all checkpoints agree, coverage is exact, no event records were dropped, cleanup events are accepted, and zero active notes are observed
- **THEN** the application emits `CREST_LIVE_EVENT_LOG`, `CREST_LIVE_STATE_TREE`, `CREST_LIVE_COVERAGE`, and `CREST_LIVE_SUMMARY` exactly once and leaves the final UI visible without further autonomous actions

#### Scenario: User closes before completion
- **WHEN** the user closes the window before the final report is complete
- **THEN** the application attempts semantic note cleanup, does not emit a successful final report, and returns a typed incomplete-live-demo result

#### Scenario: User closes after completion
- **WHEN** the final report has been emitted and the user closes the window
- **THEN** the physical stream and window shut down through their normal ownership path without additional live-scene mutations

### Requirement: Deterministic live-orchestration verification
Automated verification SHALL exercise the live runner with a deterministic monotonic clock, the production reducer and projections, the production render path, the real bounded observation transport, and a frame-observation harness without requiring a native CI window or physical audio device.

#### Scenario: Live contract test passes
- **WHEN** the deterministic live scene runs to completion
- **THEN** pacing, exact coverage, accepted/rejected records, checkpoint agreement, audio-generation correlation, semantic cleanup, zero active notes, single report completion, and inert post-completion behavior are all asserted before the acceptance marker is printed

#### Scenario: Test environment lacks a native device
- **WHEN** the automated live contract test runs in CI without a native window or physical output
- **THEN** it still executes the production control and render seams and does not skip or claim physical-device acceptance
