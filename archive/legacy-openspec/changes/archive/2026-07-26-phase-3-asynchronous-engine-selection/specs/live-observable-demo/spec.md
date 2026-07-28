## MODIFIED Requirements

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
