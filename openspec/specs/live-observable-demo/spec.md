# Live Observable Demo

## Purpose

Define the paced, human-observable production demo and its coherent control, projection, physical-audio, coverage, and cleanup evidence.
## Requirements
### Requirement: Dedicated live standalone entry point
The application SHALL provide `make demo-live`, backed by a dedicated interactive CLI mode, that opens the normal Crest Synth window, physical audio output, the two installed SoundFont and Braids capabilities, and existing Corridors of Time MIDI fixture with alternating Patch configs. The live mode SHALL be mutually exclusive with headless, observation, exhaustive-demo, and controlled-negative modes and SHALL NOT substitute a fake window, null device, offline-only renderer, silent engine, or fallback capability.

#### Scenario: User launches the live demo
- **WHEN** the user runs `make demo-live` with the repository fixtures and a usable 48 kHz physical output device
- **THEN** the production window and audio stream open, the fixed fixture begins through the normal input path, alternating SoundFont and Braids Patch identities are visible, and both engines produce audible output

#### Scenario: Required live resource is unavailable
- **WHEN** the SoundFont, pinned Braids build, MIDI fixture, window runtime, or supported physical audio device cannot be opened
- **THEN** live startup fails with a typed visible error and does not silently run a headless, silent, resampled, or single-engine substitute

#### Scenario: Live and headless flags are mixed
- **WHEN** `--demo-live` is combined with any headless, observation, exhaustive-demo, or controlled-negative flag
- **THEN** argument validation rejects the invocation before application startup

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

### Requirement: Exact current editable-surface coverage
Before dispatching its first scene action, the live demo SHALL derive and freeze an ordered unique expected set containing every Patch mixer and ADSR value for every installed Patch, every descriptor-classified Scalar parameter for each matching engine, and every editable global parameter. Every expected parameter instance SHALL receive at least one accepted value change, SHALL remain visibly projected for at least 500 milliseconds, and SHALL receive a generation-correlated audible observation.

#### Scenario: Current parameter surface is exercised
- **WHEN** the scene reaches its coverage-comparison step
- **THEN** exercised identifiers exactly equal the frozen descriptor-derived identifiers, include Model/Timbre/Color once for every Braids Patch and never for SoundFont Patches, and both missing and unexpected sets are empty

#### Scenario: Parameter is omitted or only visited
- **WHEN** an expected mixer, envelope, engine, or global parameter is missing, duplicated, selected without changing, not projected, lacks its dwell, or lacks a qualifying audio observation
- **THEN** the live report is incomplete and identifies the coverage or checkpoint failure

### Requirement: Coherent structured live checkpoints
The live demo SHALL retain its frozen scalar checkpoints and add an ordered SoundFont-to-Braids-to-descriptor-default-SoundFont proof through semantic events, the production threaded worker, structural handoff, renderer, and bounded observation transport. Every checkpoint SHALL correlate canonical event, state, projection, effects, graph status, and required target audio; actual results SHALL NOT define expectations or grant the runner graph ownership.

#### Scenario: Accepted parameter checkpoint completes
- **WHEN** an accepted scalar edit has remained projected for its dwell and audio has consumed that parameter generation
- **THEN** one `CREST_LIVE_CHECKPOINT` record reports matching event, state, projection, effect, generation, and audio-observation data

#### Scenario: Projection or audio generation disagrees
- **WHEN** a checkpoint's state tree, visible projection, parameter snapshot, emitted effects, or audio-observation generation disagrees with its production event record
- **THEN** the checkpoint fails and the live report cannot claim completion

#### Scenario: Live SoundFont changes to Braids
- **WHEN** frozen scalar coverage completes and the first SoundFont Patch requests the adjacent Braids capability
- **THEN** source SoundFont remains active during Preparing, canonical Braids commits before graph publication, Ready uses a newer acknowledged revision, and targeted MIDI produces finite nonzero Braids output on that Patch

#### Scenario: Live Braids changes back to SoundFont
- **WHEN** the acknowledged Braids Patch requests the previous installed capability
- **THEN** the lifecycle completes on another newer revision, targeted MIDI produces finite nonzero SoundFont output, and the Patch finishes Ready with descriptor-default HiDef SoundFont and its required asset

#### Scenario: Live coverage is compared
- **WHEN** the scene reaches final coverage comparison
- **THEN** editable parameters equal the frozen initial production surface, engine transitions equal the two declared ordered identities, all missing and unexpected sets are empty, and a displayed label without acknowledgement and target audio earns no credit

#### Scenario: Deterministic live contract executes production seams
- **WHEN** automated verification runs without a native device and injects mapped input during a pending checkpoint
- **THEN** it exercises the production reducer, worker port, structural coordinator, renderer, observation, input isolation, pacing, cleanup, single report, and one-shot close while separately proving physical composition injects `ThreadedGraphPreparationWorker`

#### Scenario: Live structural work does not finish
- **WHEN** worker completion, publication, activation, retirement collection, target audio, or device health fails correlation
- **THEN** the scene remains pending or returns a typed incomplete/runtime result and fabricates no transition, fallback, final report, or successful close

#### Scenario: Scene completes and tears down
- **WHEN** all scalar and engine checkpoints agree, both coverage sets are exact, the final config is default SoundFont, semantic all-notes-off reaches zero active notes, and callback allocation/destruction counts are zero
- **THEN** the application emits each compact live record once, requests close in the same tick, releases the stream before worker shutdown and graph draining, and exits successfully

### Requirement: Bounded measured audio consequence
Each accepted parameter checkpoint SHALL use finite measurements from the actual physical mixed-engine render path and SHALL require an audio observation whose sequence advanced after dispatch and whose parameter generation equals the accepted generation. Gain/master edits SHALL observe output level, pan SHALL observe stereo balance, sends SHALL observe their corresponding effect input, shared effect controls SHALL observe wet output, and ADSR/Braids controls SHALL observe nonzero output plus their deterministic offline audible-difference proof.

#### Scenario: Audible observation follows an edit
- **WHEN** the callback renders nonzero SoundFont or Braids fixture audio using the accepted parameter generation
- **THEN** the checkpoint records the relevant finite mixer/output measurement and its declared parameter-specific predicate result

#### Scenario: Audio observation is stale or non-finite
- **WHEN** the latest observation predates the edit, carries another parameter generation, has no required nonzero signal, or reports non-finite output
- **THEN** the checkpoint remains pending or fails rather than crediting the parameter as audibly exercised

### Requirement: Deterministic live-orchestration verification
Automated verification SHALL exercise the live runner with a deterministic monotonic clock, the production reducer and projections, the production render path, the real bounded observation transport, an interleaved semantic window input, and a frame-observation harness without requiring a native CI window or physical audio device. It SHALL prove both the runner contract and the standalone window lifetime.

#### Scenario: Live contract test passes
- **WHEN** the deterministic live scene runs to completion while the window injects a mapped adjustment during a pending checkpoint
- **THEN** input isolation, pacing, exact coverage, accepted/rejected scene records, checkpoint agreement, audio-generation correlation, semantic cleanup, zero active notes, single report completion, one successful close request, and absence of post-completion ticks are all asserted before the acceptance marker is printed

#### Scenario: Test environment lacks a native device
- **WHEN** the automated live contract test runs in CI without a native window or physical output
- **THEN** it still executes the production control, render, semantic-input, and window-lifetime seams and does not skip or claim physical-device acceptance

### Requirement: Responsive physical live window
The physical live demo SHALL use the optimized application binary and SHALL schedule each next idle eframe repaint after 16 ms instead of driving an immediate perpetual repaint loop. Native input and window events MAY wake the event loop sooner.

#### Scenario: Live demo idles between frames
- **WHEN** `make demo-live` launches the physical demo and no native event requests an earlier frame
- **THEN** Cargo uses the release profile and the eframe adapter schedules its next repaint after 16 ms while continuing to advance the canonical control-side tick

### Requirement: Live device health remains visible
The physical live demo SHALL monitor the bounded runtime device-status path from stream start until successful report completion or an earlier failure. A post-start device failure SHALL stop the live runtime from presenting a healthy window, preserve the typed failure outside the callback, and return it as an application-visible error without emitting successful completion or selecting a silent, headless, or alternate-device fallback.

#### Scenario: Physical stream fails during the live scene
- **WHEN** the stream starts successfully and later reports a device failure before live completion
- **THEN** control observes the exact typed failure, ends the unhealthy window lifetime, suppresses a successful live report, and returns the error

#### Scenario: Physical stream remains healthy
- **WHEN** no runtime failure is reported before the complete live report is emitted
- **THEN** device-status polling does not mutate the scene, projections, audio, or checkpoints, and successful completion closes the window and stream through normal ownership

### Requirement: Alternating engine identity remains observable
The live final StateTree, text projection, coverage, and summary SHALL demonstrate that discovery-order Patches alternate between `instrument.soundfont.hidef` and `instrument.braids`, that at least one Patch of each type sounded, and that no Patch was layered, silently replaced, or routed to another capability.

#### Scenario: Live report completes
- **WHEN** the alternating scene reaches successful cleanup
- **THEN** final structured evidence contains both capability identities, exact alternating Patch assignments, nonzero mixed-engine render evidence, complete editable coverage, and zero active notes

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

