## MODIFIED Requirements

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

