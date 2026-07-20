## ADDED Requirements

### Requirement: Deterministic production-path demo
The headless demo SHALL initialize the real fixed fixture and drive the production normalized W, S, A, D, and K input translator, control loop, projections, real-time boundary, SoundFont path, and mixer without a native window, physical audio device, wall clock, or random input.

#### Scenario: Demo runs twice
- **WHEN** the complete scene runs twice from fresh identical services
- **THEN** both runs produce byte-identical event logs, state trees, coverage, checkpoints, and report JSON

### Requirement: Stable structured observations
Observation mode SHALL emit exactly one deterministic JSON `CREST_EVENT_LOG`, one `CREST_STATE_TREE`, and one `CREST_OBSERVATION` summary with stable schema versions, explicit missing and unexpected coverage identifiers, and no opaque debug representation used as evidence.

#### Scenario: Maintainer inspects observation output
- **WHEN** the exhaustive demo completes successfully
- **THEN** each required marker has one schema-valid JSON value whose identifiers, ordering, generations, hashes, and values are deterministic

### Requirement: Exhaustive current behavior surface
The demo and table-driven verification SHALL cover every declared semantic event variant and direction, supported MIDI message kind, valid normalized GUI input, editable Patch parameter on every installed Patch, all seven global parameters, serialized state and projection property, accepted and rejected outcome, event source, and emitted downstream effect.

#### Scenario: Coverage is compared
- **WHEN** the scene finishes exercising the current surface
- **THEN** the observed set exactly equals the complete expected set and both missing and unexpected sets are empty

#### Scenario: All-notes-off paths are exercised
- **WHEN** MIDI coverage reaches all-notes-off behavior
- **THEN** Patch-scoped MIDI all-notes-off and the separate renderer all-notes-off command each have a unique covered identity and measured consequence

### Requirement: Production-derived coverage schema
The expected coverage universe SHALL be derived from production-owned input and typed semantic and parameter descriptors plus discovered serialized leaves; a separately hand-maintained string list SHALL NOT define a passing universe.

#### Scenario: Required serialized leaf is removed
- **WHEN** a schema test removes one required discovered leaf
- **THEN** exact schema equality fails and reports that leaf as missing

#### Scenario: Unexpected serialized leaf is inserted
- **WHEN** a schema test inserts one undeclared leaf
- **THEN** exact schema equality fails and reports that leaf as unexpected

### Requirement: Exact causal event records
Every scene input SHALL produce one accepted or rejected event record containing its source, tagged payload, outcome or rejection, before and after generations and state hashes, exact emitted effects, parameter generation, projection identity, and selected line as observed from the production transition.

#### Scenario: Scene step is recorded
- **WHEN** a startup, keyboard, automatic-MIDI, demo-scene, or system input is dispatched
- **THEN** its event record matches an oracle fixed before dispatch and its state, projection, command, and audio consequences agree causally

### Requirement: Exact state and projection observation
The state tree SHALL contain every current Patch identity, instrument and parameter, every global parameter, all selection properties, every text projection property, and every parameter snapshot property with exact values from the same accepted generation.

#### Scenario: State tree is compared with projections
- **WHEN** an accepted transition produces new projections
- **THEN** each required property exists with its exact expected value and the state tree, text projection, and parameter snapshot agree for that generation

### Requirement: Faithful audio evidence and restoration
Audio evidence SHALL use only supplied Patch stems and effect-send inputs, SHALL establish discriminating nonzero signals before comparisons, SHALL begin paired comparisons from identical engine and effect state, and SHALL restore every reversible parameter, send, selection, and projection exactly to baseline.

#### Scenario: Patch isolation is measured
- **WHEN** one non-first Patch parameter is changed in a discriminating multi-Patch scene
- **THEN** the target contribution changes, every untargeted Patch contribution remains sample-identical, and the final edit is restored to baseline

#### Scenario: Shared wet parameters are measured
- **WHEN** global reverb or delay values are compared
- **THEN** corresponding supplied send energy is nonzero, the comparison starts from identical effect state, no dry-derived excitation is used, and all values return exactly to baseline

### Requirement: Real GUI-context verification
A headless GUI context SHALL process real GUI key and focus events through the production application update callback and SHALL prove their next-frame control and projection effects without opening a native window.

#### Scenario: GUI adjustment is dispatched
- **WHEN** a supported adjustment is delivered as a real GUI event with the required focus state
- **THEN** the next frame, event log, accepted state, exact selected text value, and scroll target all reflect the same adjustment

### Requirement: Falsifiable production seams
Verification SHALL provide six isolated controlled-negative cases—dropped adjustment, cross-Patch parameter leak, Patch identity misroute, omitted state-tree leaf, dry-to-wet bypass, and zeroed renderer output—and each case MUST alter only its named production seam.

#### Scenario: Healthy mutation case runs
- **WHEN** any mutation-harness case runs with its mutant disabled
- **THEN** it emits one schema-valid typed observation, satisfies every healthy predicate, and exits successfully

#### Scenario: Controlled mutant runs
- **WHEN** any one of the six cases runs with only its matching mutant enabled
- **THEN** the same typed observation falsifies the causal predicate for that seam and the process exits with status 1 without editing coverage or completed report fields

### Requirement: Evidence-backed completion
The demo SHALL NOT report acceptance solely because components were constructed, code paths were called, marker text was printed, or a master buffer changed; completion MUST be supported by the declared structured observations and predicates.

#### Scenario: Acceptance marker is present without required measurements
- **WHEN** output contains a success marker but a required typed observation or predicate is absent or false
- **THEN** the capability is considered unproven and verification fails

