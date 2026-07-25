# Automatic Test MIDI

## Purpose

Define the fixed automatic MIDI fixture, instrument-to-Patch discovery, and production-path routing used to exercise the application without introducing domain-level transport state.

## Requirements

### Requirement: Fixed automatic MIDI fixture
The application SHALL use `./midi/Corridors of Time - Chrono Trigger.mid` as its automatic MIDI input, SHALL prepare its discovered identities and install their Patches without emitting events, and SHALL begin emitting automatically only after the complete initial audio graph for those accepted Patches has been prepared and installed. No transport input SHALL be required.

#### Scenario: Application opens with the fixture
- **WHEN** the application opens with the fixed MIDI file and required SoundFont available
- **THEN** fixture discovery installs its Patches through the production reducer, the matching complete graph is prepared, and only then does the fixture begin emitting MIDI events

#### Scenario: Initial graph preparation fails
- **WHEN** accepted fixture Patches cannot be prepared into one complete graph
- **THEN** startup fails before fixture emission or physical audio begins and no partial graph or fallback is used

#### Scenario: Fixed fixture is unavailable
- **WHEN** the fixed MIDI file is missing or invalid
- **THEN** fixture preparation fails clearly instead of silently substituting another file or beginning graph/audio startup

### Requirement: Distinct instrument Patch assignment
The automatic input SHALL create exactly one Patch for each discovered instrument identity, translate each identity into the stable generic assignments declared by the installed capability descriptor, validate them through the installed instrument capability provider, assign part N to a stable unique channel N, and fail on conversion or channel exhaustion rather than substitute a config, reuse a channel, or merge identities.

#### Scenario: Several instrument parts are discovered
- **WHEN** the fixture contains several bank, program, or percussion identities
- **THEN** each identity has one Patch with an exact `instrument.soundfont.hidef` config and a distinct stable MIDI channel

#### Scenario: Instrument conversion fails
- **WHEN** the provider cannot represent a discovered identity or its descriptor disagrees with the installed registry
- **THEN** fixture initialization fails atomically before Patch installation and no preset, config, provider, or engine is substituted

#### Scenario: Instrument parts exceed channel capacity
- **WHEN** discovery requires more distinct channels than the supported bounded capacity
- **THEN** initialization fails without reusing a channel or merging instrument identities

### Requirement: MIDI timing remains adapter-private
The system SHALL keep MIDI-file timing outside the domain model and SHALL NOT expose sequencing, transport, timeline, song, clip, pattern, recording, editing, or playback-control state.

#### Scenario: Automatic playback advances
- **WHEN** elapsed time causes fixture MIDI events to become due
- **THEN** the events are emitted as input without creating or mutating any transport or sequencer state

### Requirement: Automatic events use the production control path
Every emitted fixture event SHALL pass through the same semantic event, accepted-state, command-publication, and audio-routing path used by other production inputs.

#### Scenario: Due fixture events are dispatched
- **WHEN** an automatic tick emits MIDI events
- **THEN** each event is recorded with its automatic-input source and produces its exact routed audio command through the production control loop

### Requirement: Single fixture advancement owner in live mode
The live demo SHALL use the existing Corridors of Time MIDI fixture and SHALL have exactly one control-side owner advance its elapsed-time input. Each due fixture event SHALL be dispatched exactly once through the production semantic event path, without adding transport state or a second MIDI-file adapter.

#### Scenario: Live window tick advances the fixture
- **WHEN** elapsed live-demo time makes fixture events due
- **THEN** the live orchestration owner advances the existing fixture once and each due event produces one automatic-input event record and routed audio command

#### Scenario: Shell and live runner share a tick
- **WHEN** the live runner owns fixture advancement for a window tick
- **THEN** the standalone shell does not independently poll or dispatch the fixture for that tick

### Requirement: Bounded fixture backlog draining
Automatic MIDI polling SHALL append due events in source order only up to the caller-owned fixed batch's available capacity, retain every remaining overdue event for later polls, and never fail merely because delayed control-side projection work makes more events due than one batch can hold.

#### Scenario: One delayed live-demo tick spans a dense MIDI interval
- **WHEN** more fixture events are overdue than fit in one `FixedEventBatch`
- **THEN** the source returns one full batch without error and subsequent polls drain the retained events in exact source order without loss or duplication

#### Scenario: One full overdue batch reaches the control loop
- **WHEN** the source returns all 256 entries of one bounded batch against the fifteen-Patch live fixture
- **THEN** the production generation-only projection path drains it without a visible long-running serialization loop and retains canonical per-event reducer, journal, parameter, and command evidence
