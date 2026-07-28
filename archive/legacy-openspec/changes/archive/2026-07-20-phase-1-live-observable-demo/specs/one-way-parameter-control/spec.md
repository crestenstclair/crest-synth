## ADDED Requirements

### Requirement: Autonomous demo actions use the shared control path
Autonomous live-demo navigation, adjustment, MIDI, rejection-probe, and cleanup actions SHALL enter as semantic events through the same reducer, event log, state commit, projection, parameter-publication, and audio-command path used by keyboard and fixture inputs. The live runner and window SHALL NOT receive mutable canonical state or mutate projections directly.

#### Scenario: Live adjustment is accepted
- **WHEN** a due live action validly adjusts the selected parameter
- **THEN** the production event record, state tree, visible text projection, parameter snapshot, and emitted effects all represent the same newly accepted generation

#### Scenario: Live adjustment is rejected
- **WHEN** a due live action violates a parameter boundary
- **THEN** the existing event log records the rejection, state and projection generations remain unchanged, no audio effect is emitted, and later valid input remains processable

### Requirement: Live checkpoint expectations precede mutation
The live scene SHALL compute and freeze each expected transition from the prior canonical state and the production-owned parameter descriptor before dispatching the corresponding event. Observed post-dispatch state, projection, effect, or audio values SHALL NOT be reused as the expected values.

#### Scenario: Checkpoint compares expected and actual values
- **WHEN** a live event has been processed
- **THEN** its actual production record and projections are compared with the independently frozen expectation and any mismatch prevents checkpoint completion
