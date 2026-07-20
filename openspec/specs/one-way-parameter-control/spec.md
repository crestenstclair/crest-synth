# One-Way Parameter Control

## Purpose

Define the normalized one-way control pipeline, exact keyboard vocabulary, bounded parameter behavior, complete text projection, and replaceable external boundaries.

## Requirements

### Requirement: Ordered one-way state transition
Every supported control input SHALL be normalized into a semantic event, and accepted state SHALL be committed before serialization, text projection, parameter publication, or audio-command effects are produced.

#### Scenario: Accepted parameter edit
- **WHEN** a valid adjustment event targets the selected parameter
- **THEN** the accepted state generation advances first and the resulting serialized state, text projection, parameter snapshot, and emitted effects all represent that same generation

#### Scenario: Rejected event
- **WHEN** an event is rejected by a declared state invariant
- **THEN** state and parameter generations remain unchanged, the rejection is recorded, and later valid events can still be processed

### Requirement: Exact keyboard vocabulary
Bare W and S SHALL navigate parameters, bare A and D SHALL navigate Patch or GLOBAL sections, and holding K with W, S, A, or D SHALL adjust only the selected bounded value. K SHALL act only as a modifier, and no physical key SHALL mutate application state directly.

#### Scenario: Navigate without editing
- **WHEN** the player presses bare W, S, A, or D
- **THEN** selection moves according to the declared wrap and clamp rules without changing a synth parameter

#### Scenario: Edit the selected value
- **WHEN** the player holds K and presses W, S, A, or D
- **THEN** exactly the selected bounded parameter receives the corresponding fine or coarse adjustment through the semantic event path

### Requirement: Nonfatal parameter boundaries
Every editable value SHALL enforce its typed lower and upper bounds, and an adjustment beyond a reached boundary SHALL be an unchanged nonfatal transition.

#### Scenario: Edit beyond a boundary
- **WHEN** the player adjusts a selected value toward an already reached lower or upper bound
- **THEN** the value remains unchanged, a boundary rejection is recorded, and the application accepts a subsequent valid edit

### Requirement: Single complete text view
The user interface SHALL be one scrollable text view listing every current Patch parameter and all global parameters, with Patch sections separated by `------------------------------------------------------------` and the selected line visibly identified.

#### Scenario: View is projected from current state
- **WHEN** installed Patches, parameters, or selection change
- **THEN** the one text body contains every exact current value, the required Patch separators, and a selection marker at the projected selected line

### Requirement: Cross-projection equality
Serialized state, text, and published real-time parameters SHALL contain the exact values accepted for the same state generation, including every Patch identity, instrument, editable Patch parameter, global parameter, and selection property.

#### Scenario: Inspect an accepted edit
- **WHEN** any Patch or global parameter edit is accepted
- **THEN** the state tree, selected text value, and corresponding real-time parameter contain the same exact value and all unrelated values remain unchanged

### Requirement: Production GUI input uses the shared path
Real GUI key and focus events SHALL pass through the same normalized input translator and one-way control loop used by deterministic headless verification.

#### Scenario: Headless GUI frame processes an edit
- **WHEN** a headless GUI context sends a supported key or focus event through the production update callback
- **THEN** the next frame, event record, accepted state, exact text projection, and scroll target all reflect that event without opening a native window

### Requirement: Replaceable external boundaries
Sound generation, MIDI input, audio output, text rendering, and the real-time handoff SHALL be expressed through stable behavioral boundaries so one conforming adapter can be replaced without changing domain behavior.

#### Scenario: Production and headless adapters share behavior
- **WHEN** a production adapter is replaced by a conforming headless test adapter
- **THEN** the same normalized inputs and semantic events produce equivalent accepted state and projections

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
