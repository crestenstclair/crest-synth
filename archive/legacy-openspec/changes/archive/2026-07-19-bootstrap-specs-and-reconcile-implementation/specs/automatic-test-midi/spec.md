## ADDED Requirements

### Requirement: Fixed automatic MIDI fixture
The application SHALL use `./midi/Corridors of Time - Chrono Trigger.mid` as its automatic MIDI input and SHALL begin emitting its events without requiring transport input.

#### Scenario: Application opens with the fixture
- **WHEN** the application opens with the fixed MIDI file available
- **THEN** the fixture is prepared and begins emitting MIDI events automatically

#### Scenario: Fixed fixture is unavailable
- **WHEN** the fixed MIDI file is missing or invalid
- **THEN** fixture initialization fails clearly instead of silently substituting another file

### Requirement: Distinct instrument Patch assignment
The automatic input SHALL create exactly one Patch for each discovered instrument identity, assign part N to a stable unique channel N, and fail on channel exhaustion rather than reuse a channel.

#### Scenario: Several instrument parts are discovered
- **WHEN** the fixture contains several bank, program, or percussion identities
- **THEN** each identity has one Patch and a distinct stable MIDI channel

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

