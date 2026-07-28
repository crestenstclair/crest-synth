## ADDED Requirements

### Requirement: Single fixture advancement owner in live mode
The live demo SHALL use the existing Corridors of Time MIDI fixture and SHALL have exactly one control-side owner advance its elapsed-time input. Each due fixture event SHALL be dispatched exactly once through the production semantic event path, without adding transport state or a second MIDI-file adapter.

#### Scenario: Live window tick advances the fixture
- **WHEN** elapsed live-demo time makes fixture events due
- **THEN** the live orchestration owner advances the existing fixture once and each due event produces one automatic-input event record and routed audio command

#### Scenario: Shell and live runner share a tick
- **WHEN** the live runner owns fixture advancement for a window tick
- **THEN** the standalone shell does not independently poll or dispatch the fixture for that tick
