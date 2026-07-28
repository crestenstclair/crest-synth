## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Bounded fixture backlog draining
Automatic MIDI polling SHALL append due events in source order only up to the caller-owned fixed batch's available capacity, retain every remaining overdue event for later polls, and never fail merely because delayed control-side projection work makes more events due than one batch can hold.

#### Scenario: One delayed live-demo tick spans a dense MIDI interval
- **WHEN** more fixture events are overdue than fit in one `FixedEventBatch`
- **THEN** the source returns one full batch without error and subsequent polls drain the retained events in exact source order without loss or duplication

#### Scenario: One full overdue batch reaches the control loop
- **WHEN** the source returns all 256 entries of one bounded batch against the fifteen-Patch live fixture
- **THEN** the production generation-only projection path drains it without a visible long-running serialization loop and retains canonical per-event reducer, journal, parameter, and command evidence
