# live-detail-and-assets-demo Specification

## Purpose
TBD - created by archiving change implement-phase-7-detail-choice-assets. Update Purpose after archive.
## Requirements
### Requirement: Phase 7 has a retained optimized live target
The repository SHALL provide `make demo-live-detail-and-assets` as a bounded autonomous release-mode scene using the production webview shell, production input normalization, `AppState::apply`, production projection, threaded preparation, structural and scalar real-time transports, physical audio output, and the real parsed MIDI fixture. The phase-specific target SHALL remain retained, and `make demo-live` SHALL point to this newest cumulative scene without deleting or renaming earlier phase targets.

#### Scenario: Invoke the Phase 7 target
- **WHEN** `make demo-live-detail-and-assets` runs on a host with the required window, MIDI fixture, Sample fixture, and physical audio device
- **THEN** it opens the production shell and completes the declared Phase 7 checkpoints without direct UI-state mutation or synthetic audio substitution

#### Scenario: Invoke the cumulative alias
- **WHEN** `make demo-live` is invoked after Phase 7 completion
- **THEN** it runs the Phase 7 cumulative scene while every earlier phase-specific live target remains addressable

### Requirement: The scene proves detail and choice focus correlation
The live scene SHALL open instrument detail and effect detail on more than one installed capability, traverse descriptor-projected controls, open engine, effect, route, and another available structural-choice modal, and exercise both commit and cancel. Every subordinate entry and exit SHALL record the origin `FocusPath`, visible focused identity, command, resulting canonical state, and exact return path; at least one controlled origin invalidation SHALL prove deterministic sibling recovery if such invalidation is reachable through production actions.

#### Scenario: Complete a detail and modal journey
- **WHEN** the scene opens a detail, opens or closes an applicable option modal, and returns
- **THEN** its structured checkpoint correlates the rendered focus marker, semantic focus identity, canonical value, active/requested lifecycle, and exact origin

### Requirement: The scene proves Sample browse, preview, error, and commit
The live scene SHALL select the installed Sample capability through semantic choice, open Sample detail and its browser, navigate parent/folder/file rows, hold and release Start on an eligible valid file, exercise cancellation, attempt one intentionally invalid or unavailable asset through the production adapter, and then confirm a valid asset through production preparation and block-boundary activation. It SHALL prove that preview never commits, release stops audition, failure preserves the previous graph, and valid activation updates the correlated Sample row and waveform.

#### Scenario: Preview without assignment
- **WHEN** the scene holds Start on the valid Sample fixture and releases it before pressing Edit
- **THEN** measured preview energy appears only during the correlated hold/stop interval and the active Sample asset remains unchanged

#### Scenario: Recover from an asset error
- **WHEN** the scene requests its declared invalid or unavailable fixture
- **THEN** the typed error becomes visible, the prior graph remains audible and revision-compatible, and the scene continues through semantic recovery rather than fallback

#### Scenario: Commit the valid asset
- **WHEN** the scene confirms the valid asset and its correlated prepared graph activates
- **THEN** the active asset identity, waveform landmarks, graph revision, and subsequent rendered Sample audio all agree on the committed request

### Requirement: The scene audibly proves target isolation and routing
Real MIDI SHALL play concurrently across multiple Patches using at least two installed instrument capabilities, including the committed Sample Patch. The scene SHALL measure pre- and post-action audio by Patch and Mixer track while changing the Sample origin route and exercising mute, solo, level, pan, and at least one send/return already supported by the production Mixer. It SHALL prove that browser preview and committed Sample notes affect only their origin Patch route and expected downstream buses while unrelated Patch stems remain within declared tolerances.

#### Scenario: Isolate the Sample target
- **WHEN** the scene previews or plays the committed Sample while unrelated Patches continue receiving parsed MIDI
- **THEN** structured measurements show the expected Sample-origin and routed-track energy change while non-target stems remain unchanged within the declared tolerance

#### Scenario: Route the committed Sample
- **WHEN** a route modal moves the Sample Patch to another stable Mixer track and the compatible state activates
- **THEN** subsequent Sample energy moves to the selected track and its sends while the prior track loses only that Patch contribution

### Requirement: The live report is bounded, falsifiable, and teardown-complete
Every checkpoint SHALL use named predicates, bounded deadlines, graph/generation compatibility, non-zero and finite audio thresholds, and a declared controlled negative that exits non-zero when a required Phase 7 semantic journey or audible effect is defeated. The final report SHALL contain the lossless semantic event log and typed preparation/preview evidence. Success SHALL require all MIDI notes released, no active audition voice, no pending Sample request, window and stream closure, returned prepared graphs destroyed off the callback, and zero callback allocations, locks, I/O, logging, panics, or destruction.

#### Scenario: A required action is defeated
- **WHEN** the controlled-negative mode suppresses the declared Sample preview, exact-return, or committed-asset action
- **THEN** at least one named reach or audio predicate fails and the command exits with a non-zero status

#### Scenario: Complete teardown
- **WHEN** all successful scene checkpoints finish
- **THEN** the scene releases notes and preview, closes the production window and audio stream, retires graph ownership off-thread, emits its final report, and exits zero only if every teardown predicate passes
