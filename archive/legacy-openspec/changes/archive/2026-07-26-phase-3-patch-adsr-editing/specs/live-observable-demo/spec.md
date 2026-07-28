## MODIFIED Requirements

### Requirement: Exact current editable-surface coverage
Before dispatching its first scene action, the live demo SHALL derive and freeze one ordered unique expected set containing every Patch mixer and ADSR value for every installed Patch, every descriptor-classified Scalar parameter for each matching engine, and every editable global parameter. The focused first Patch's Attack, Decay, Sustain, and Release instances SHALL be navigated to and edited through PATCH exactly once for coverage; every other editable instance SHALL use the existing MIXER plan. Every expected parameter instance SHALL receive at least one accepted value change, SHALL remain visibly projected for at least 500 milliseconds, and SHALL receive a generation-correlated audible observation without duplicate credit.

#### Scenario: Focused Patch envelope is exercised through PATCH
- **WHEN** the scene reaches the focused first Patch's four frozen envelope identifiers
- **THEN** semantic PATCH navigation visits Attack, Decay, Sustain, and Release in canonical order, each identifier receives one accepted bounded adjustment and visible/audio checkpoint, exactly one row is selected, and no duplicate MIXER exercise is credited

#### Scenario: Current parameter surface is exercised
- **WHEN** the scene reaches its coverage-comparison step
- **THEN** exercised identifiers exactly equal the frozen descriptor-derived identifiers, include all four focused-Patch ADSR instances once, include Model/Timbre/Color once for every Braids Patch and never for SoundFont Patches, and both missing and unexpected sets are empty

#### Scenario: Parameter is omitted or only visited
- **WHEN** an expected mixer, envelope, engine, or global parameter is missing, duplicated, selected without changing, not projected, lacks its dwell, or lacks a qualifying audio observation
- **THEN** the live report is incomplete and identifies the coverage or checkpoint failure

#### Scenario: PATCH focus or value disagrees
- **WHEN** a focused-Patch ADSR step marks another row, changes another Patch or field, emits a structural effect, or reports a value different from canonical state and the fixed snapshot
- **THEN** the checkpoint earns no coverage and the live report cannot complete

### Requirement: Coherent structured live checkpoints
The live demo SHALL retain its frozen scalar checkpoints, route the focused first Patch's four ADSR checkpoints through PATCH, and retain the ordered SoundFont-to-Braids-to-descriptor-default-SoundFont proof through semantic events, the production threaded worker, structural handoff, renderer, and bounded observation transport. Every scalar checkpoint SHALL be bracketed by a Patch-targeted semantic NoteOn before its accepted edit and the matching NoteOff after exact-generation capture and dwell, using the parameter's owning Patch or the focused first Patch for globals, so sparse fixture timing cannot strand audibility while fixture advancement is frozen. Probe events SHALL traverse `AppLoop` and the discrete audio boundary and SHALL NOT earn editable coverage. Every checkpoint SHALL correlate canonical event, state, PATCH focus when applicable, projection, effects, graph status, and required target audio; actual results SHALL NOT define expectations or grant the runner graph ownership. The live command SHALL announce that it is autonomous, input-isolated, and bounded before physical startup. Device negotiation SHALL accept an already valid preferred-rate default configuration without requiring optional supported-range enumeration and SHALL retain an already valid default if that optional enumeration fails. Once window ticks begin, ten seconds without an autonomous scene, checkpoint, engine-lifecycle, or cleanup milestone or 120 seconds total SHALL produce a typed stage-specific failure, close the disposable window, perform semantic note cleanup, release the stream, shut down structural ownership off callback, exit nonzero, and emit no completed report.

#### Scenario: Accepted parameter checkpoint completes
- **WHEN** an accepted scalar edit has remained projected for its dwell and audio has consumed that parameter generation
- **THEN** one `CREST_LIVE_CHECKPOINT` record reports matching event, state, projection, effect, generation, and audio-observation data

#### Scenario: Sparse fixture passage cannot strand a scalar checkpoint
- **WHEN** the Corridors fixture has no currently sounding part capable of satisfying the next parameter's audio predicate
- **THEN** the scene dispatches one bounded semantic NoteOn to the owning Patch before the edit, captures the exact edit generation while that probe is rendered, dispatches its matching NoteOff after dwell, resumes fixture advancement, and grants coverage only to the accepted parameter edit

#### Scenario: Accepted PATCH ADSR checkpoint completes
- **WHEN** a focused-Patch ADSR edit has remained selected and projected for its dwell and audio has consumed that parameter generation
- **THEN** the checkpoint additionally reports the exact PATCH control identity, canonical envelope value, empty discrete/structural effects, and the same frozen editable identifier used by coverage

#### Scenario: Projection or audio generation disagrees
- **WHEN** a checkpoint's state tree, visible projection, focused control, parameter snapshot, emitted effects, or audio-observation generation disagrees with its production event record
- **THEN** the checkpoint fails and the live report cannot claim completion

#### Scenario: Live SoundFont changes to Braids
- **WHEN** frozen scalar coverage completes, the scene returns PATCH focus to Engine, and the first SoundFont Patch requests the adjacent Braids capability
- **THEN** source SoundFont remains active during Preparing, canonical Braids commits before graph publication, Ready uses a newer acknowledged revision, focus and envelope remain exact, and targeted MIDI produces finite nonzero Braids output on that Patch

#### Scenario: Live Braids changes back to SoundFont
- **WHEN** the acknowledged Braids Patch with Engine focused requests the previous installed capability
- **THEN** the lifecycle completes on another newer revision, targeted MIDI produces finite nonzero SoundFont output, and the Patch finishes Ready with descriptor-default HiDef SoundFont, its required asset, and its canonical envelope intact

#### Scenario: Live coverage is compared
- **WHEN** the scene reaches final coverage comparison
- **THEN** editable parameters equal the frozen initial production surface, the focused Patch's four ADSR route identities are PATCH, engine transitions equal the two declared ordered identities, all missing and unexpected sets are empty, and a displayed label without acknowledgement and target audio earns no credit

#### Scenario: Deterministic live contract executes production seams
- **WHEN** automated verification runs without a native device and injects mapped input during a pending checkpoint
- **THEN** it exercises the production reducer, PATCH focus and editing, worker port, structural coordinator, renderer, observation, input isolation, pacing, cleanup, single report, and one-shot close while separately proving physical composition injects `ThreadedGraphPreparationWorker`

#### Scenario: Live structural work does not finish
- **WHEN** worker completion, publication, activation, retirement collection, target audio, or device health fails correlation
- **THEN** the scene remains pending or returns a typed incomplete/runtime result and fabricates no transition, fallback, final report, or successful close

#### Scenario: Scene completes and tears down
- **WHEN** all scalar and engine checkpoints agree, both coverage sets are exact, the final config is default SoundFont, semantic all-notes-off reaches zero active notes, and callback allocation/destruction counts are zero
- **THEN** the application emits each compact live record once, requests close in the same tick, releases the stream before worker shutdown and graph draining, and exits successfully

#### Scenario: Optional device ranges fail after a valid default
- **WHEN** the default output reports a valid 48 kHz PCM configuration and optional supported-range enumeration is unavailable or faulty
- **THEN** startup uses that exact validated default without substituting a device or format and prepares the graph before starting the stream

#### Scenario: Live audio or engine progress stalls
- **WHEN** a pending exact-generation audio observation, engine lifecycle milestone, targeted engine output, or cleanup observation makes no qualifying progress for ten seconds
- **THEN** the runner returns a typed error naming the stalled stage and duration, the owner closes and tears down normally, and no success report or coverage is fabricated

#### Scenario: Whole live scene exceeds its bound
- **WHEN** incremental milestones continue but the autonomous scene reaches 120 seconds without completion
- **THEN** the owner reports a typed whole-run timeout and follows the same cleanup, close, stream-release, structural-shutdown, and nonzero-exit path
