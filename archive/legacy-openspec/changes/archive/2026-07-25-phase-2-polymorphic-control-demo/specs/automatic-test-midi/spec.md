## MODIFIED Requirements

### Requirement: Fixed automatic MIDI fixture
The application SHALL use `./midi/Corridors of Time - Chrono Trigger.mid` as its automatic MIDI input, SHALL prepare its discovered identities and install their Patches without emitting events, and SHALL begin emitting automatically only after the complete initial mixed-engine audio graph for those accepted Patches has been prepared and installed. No transport input SHALL be required.

#### Scenario: Application opens with the fixture
- **WHEN** the application opens with the fixed MIDI file, required SoundFont, and installed Braids adapter available
- **THEN** fixture discovery installs alternating SoundFont and Braids Patches through the production reducer, the matching complete graph is prepared, and only then does the fixture begin emitting MIDI events

#### Scenario: Initial graph preparation fails
- **WHEN** any accepted fixture Patch cannot be prepared by its exact capability into one complete graph
- **THEN** startup fails before fixture emission or physical audio begins and no partial graph or fallback is used

#### Scenario: Fixed fixture is unavailable
- **WHEN** the fixed MIDI file is missing or invalid
- **THEN** fixture preparation fails clearly instead of silently substituting another file or beginning graph/audio startup

### Requirement: Distinct instrument Patch assignment
The automatic input SHALL create exactly one Patch for each discovered instrument identity, assign zero-based even parts to an exact `instrument.soundfont.hidef` config and odd parts to a schema-valid default `instrument.braids` config, validate each through its installed provider, assign part N to a stable unique channel N, and fail on conversion or channel exhaustion rather than substitute a config, reuse a channel, merge identities, or layer engines.

#### Scenario: Several instrument parts are discovered
- **WHEN** the fixture contains several bank, program, or percussion identities
- **THEN** each identity has one stable Patch and channel, Patch capability ids alternate SoundFont then Braids in discovery order, and SoundFont Patches alone retain the exact discovered preset identity

#### Scenario: Instrument conversion fails
- **WHEN** the selected provider cannot construct a valid config or its descriptor disagrees with the installed registry
- **THEN** fixture initialization fails atomically before Patch installation and no preset, config, provider, capability, or engine is substituted

#### Scenario: Instrument parts exceed channel capacity
- **WHEN** discovery requires more distinct channels than the supported bounded capacity
- **THEN** initialization fails without reusing a channel, merging instrument identities, or changing the alternation policy
