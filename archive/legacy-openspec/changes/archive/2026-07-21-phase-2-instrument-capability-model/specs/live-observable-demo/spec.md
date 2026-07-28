## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Responsive physical live window
The physical live demo SHALL use the optimized application binary and SHALL schedule each next idle eframe repaint after 16 ms instead of driving an immediate perpetual repaint loop. Native input and window events MAY wake the event loop sooner.

#### Scenario: Live demo idles between frames
- **WHEN** `make demo-live` launches the physical demo and no native event requests an earlier frame
- **THEN** Cargo uses the release profile and the eframe adapter schedules its next repaint after 16 ms while continuing to advance the canonical control-side tick
