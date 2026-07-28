## MODIFIED Requirements

### Requirement: Paced production-path scene
The live demo SHALL advance incrementally from control-side window ticks and SHALL express every autonomous navigation, adjustment, fixture, rejection-probe, and note-cleanup action as a semantic event through the same reducer and publication path used by other inputs. While the autonomous scene is active, semantic window input SHALL NOT dispatch an application event or mutate canonical state, projections, audio publications, or the event log. The live demo SHALL NOT directly mutate UI, canonical state, projections, engine state, mixer state, audio parameters, commands, or output buffers.

#### Scenario: Autonomous action becomes due
- **WHEN** the live scene dwell expires for its next planned action
- **THEN** at most one due semantic event is dispatched through the production control path and the next visible/audio projections derive from its accepted canonical generation

#### Scenario: Event is rejected at a parameter boundary
- **WHEN** the scene adjusts a parameter beyond an already reached bound
- **THEN** the existing event log records the rejection, state and projections remain unchanged, the window stays open, and the following valid adjustment can be accepted

#### Scenario: User presses a mapped key during an autonomous checkpoint
- **WHEN** the live window produces a semantic input after a scene edit is dispatched and before its audio checkpoint is captured
- **THEN** the input is ignored without an EventLog record, generation advance, projection change, parameter publication, checkpoint retry, or live-demo failure

### Requirement: Deterministic live-orchestration verification
Automated verification SHALL exercise the live runner with a deterministic monotonic clock, the production reducer and projections, the production render path, the real bounded observation transport, an interleaved semantic window input, and a frame-observation harness without requiring a native CI window or physical audio device. It SHALL prove both the runner contract and the standalone window lifetime.

#### Scenario: Live contract test passes
- **WHEN** the deterministic live scene runs to completion while the window injects a mapped adjustment during a pending checkpoint
- **THEN** input isolation, pacing, exact coverage, accepted/rejected scene records, checkpoint agreement, audio-generation correlation, semantic cleanup, zero active notes, single report completion, one successful close request, and absence of post-completion ticks are all asserted before the acceptance marker is printed

#### Scenario: Test environment lacks a native device
- **WHEN** the automated live contract test runs in CI without a native window or physical output
- **THEN** it still executes the production control, render, semantic-input, and window-lifetime seams and does not skip or claim physical-device acceptance

### Requirement: Live device health remains visible
The physical live demo SHALL monitor the bounded runtime device-status path from stream start until successful report completion or an earlier failure. A post-start device failure SHALL stop the live runtime from presenting a healthy window, preserve the typed failure outside the callback, and return it as an application-visible error without emitting successful completion or selecting a silent, headless, or alternate-device fallback.

#### Scenario: Physical stream fails during the live scene
- **WHEN** the stream starts successfully and later reports a device failure before live completion
- **THEN** control observes the exact typed failure, ends the unhealthy window lifetime, suppresses a successful live report, and returns the error

#### Scenario: Physical stream remains healthy
- **WHEN** no runtime failure is reported before the complete live report is emitted
- **THEN** device-status polling does not mutate the scene, projections, audio, or checkpoints, and successful completion closes the window and stream through normal ownership

## ADDED Requirements

### Requirement: Bounded live completion and cleanup
After all parameter checkpoints, the live demo SHALL dispatch one Patch-targeted semantic all-notes-off event for every installed Patch, wait for a newer observation reporting zero active notes, retain the complete final EventLog in its typed `LiveDemoReport`, and emit exactly one compact event-log summary, state tree, coverage result, and human-readable summary from the control side. Immediately after emitting those four records, it SHALL request window close without another autonomous, input, projection, or window tick, then release the physical stream through normal ownership and return exit code zero. Interactive output SHALL NOT dump every retained performance MIDI record.

#### Scenario: Scene completes successfully
- **WHEN** all checkpoints agree, coverage is exact, no event records were dropped, cleanup events are accepted, and zero active notes are observed
- **THEN** the application emits `CREST_LIVE_EVENT_LOG_SUMMARY`, `CREST_LIVE_STATE_TREE`, `CREST_LIVE_COVERAGE`, and `CREST_LIVE_SUMMARY` exactly once, requests window close in the same control tick, releases the audio stream, and returns success

#### Scenario: Complete event evidence is verified
- **WHEN** deterministic verification inspects the completed `LiveDemoReport`
- **THEN** the complete retained EventLog remains available in memory and agrees with the compact summary and final StateTree endpoint before ownership teardown

#### Scenario: User closes before completion
- **WHEN** the user closes the window before the final report is complete
- **THEN** the application attempts semantic note cleanup, does not emit a successful final report, and returns a typed incomplete-live-demo result

#### Scenario: Successful command is left unattended
- **WHEN** the user launches `make demo-live` and supplies no window input or close action
- **THEN** the command completes the scene and returns success without remaining resident after its final report

## REMOVED Requirements

### Requirement: Live completion, cleanup, and persistent final state

After all parameter checkpoints, the live demo SHALL dispatch one Patch-targeted semantic all-notes-off event for every installed Patch, wait for a newer observation reporting zero active notes, retain the complete final EventLog in its typed `LiveDemoReport`, emit exactly one compact event-log summary, state tree, coverage result, and human-readable summary from the control side, become inert, and continue showing the final canonical projection until the user closes the window. Interactive output SHALL NOT dump every retained performance MIDI record.

#### Scenario: Scene completes successfully
- **WHEN** all checkpoints agree, coverage is exact, no event records were dropped, cleanup events are accepted, and zero active notes are observed
- **THEN** the application emits `CREST_LIVE_EVENT_LOG_SUMMARY`, `CREST_LIVE_STATE_TREE`, `CREST_LIVE_COVERAGE`, and `CREST_LIVE_SUMMARY` exactly once, the compact summary reports lossless counts and canonical chain endpoints, and the final UI remains visible without further autonomous actions

#### Scenario: Complete event evidence is verified
- **WHEN** deterministic verification inspects the completed `LiveDemoReport`
- **THEN** the complete retained EventLog remains available in memory and agrees with the compact summary and final StateTree endpoint

#### Scenario: User closes before completion
- **WHEN** the user closes the window before the final report is complete
- **THEN** the application attempts semantic note cleanup, does not emit a successful final report, and returns a typed incomplete-live-demo result

#### Scenario: User closes after completion
- **WHEN** the final report has been emitted and the user closes the window
- **THEN** the physical stream and window shut down through their normal ownership path without additional live-scene mutations

**Reason**: Retaining the window and physical stream after the final report makes the command remain resident indefinitely and permits unrelated input to invalidate exact-generation proof before completion.

**Migration**: Inspect the emitted final StateTree, coverage, event-log summary, and human summary for demo evidence; use normal interactive mode rather than `demo-live` for an open-ended controllable window.
