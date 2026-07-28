## MODIFIED Requirements

### Requirement: Exact keyboard vocabulary
Digit 1 SHALL select MIXER and Digit 2 SHALL select PATCH. Existing bare and K-modified W/S/A/D behavior SHALL remain exact in MIXER. On PATCH's sole engine control, K+A/D SHALL request the adjacent installed engine through the same semantic Adjust event and `AppState::apply`; all other PATCH navigation/editing SHALL remain unavailable. Worker outcomes and acknowledgements SHALL also enter through correlated semantic events before projection or publication.

#### Scenario: Select a context
- **WHEN** the player presses `1` or `2`
- **THEN** the shared translator emits the matching semantic `SelectContext` without reading AppState or owning page state

#### Scenario: Navigate without editing
- **WHEN** MIXER is active and the player presses bare W, S, A, or D
- **THEN** selection follows the declared wrap and clamp rules without changing a synth parameter

#### Scenario: Edit the selected value
- **WHEN** MIXER is active and the player holds K and presses W, S, A, or D
- **THEN** exactly the selected bounded parameter receives the matching fine or coarse adjustment through the semantic event path

#### Scenario: Release a page key
- **WHEN** the player releases `1` or `2`, including while K is held
- **THEN** no semantic event is emitted and context remains reducer-owned

#### Scenario: Select an engine in PATCH
- **WHEN** PATCH is active on the engine row and the player holds K and presses A or D
- **THEN** one normalized Adjust requests the adjacent nonwrapping registry choice without the translator inspecting a capability, worker, or graph

#### Scenario: Accepted engine request preserves the source
- **WHEN** the reducer accepts an adjacent choice
- **THEN** it commits Preparing and emits one correlated preparation request before projection while accepted config, graph revision, parameter values, and active audio remain unchanged

#### Scenario: Prepared result commits before graph publication
- **WHEN** a complete matching candidate arrives through the worker outcome event
- **THEN** `AppState::apply` commits the target config and Activating state before target-revision state, text, parameters, structural effects, or graph ownership are published

#### Scenario: Failed, stale, or early lifecycle input arrives
- **WHEN** a worker failure, stale or mismatched result, busy request, or early acknowledgement is reduced
- **THEN** it records visible source-preserving failure or rejects unchanged, emits no invalid structural effect, and leaves later MIDI, context, and valid MIXER scalar events processable

#### Scenario: Text view follows canonical lifecycle
- **WHEN** PATCH renders Ready, Preparing, Activating, or Failed state and the player later returns to MIXER
- **THEN** the shell displays only the canonical immutable projection, marks only a ready/failed engine row editable, owns no lifecycle or graph state, and restores the exact retained MIXER selection
