## ADDED Requirements

### Requirement: Projection targets one prepared graph revision
Every fixed parameter snapshot and its StateTree parameters projection SHALL contain the same nonzero graph revision supplied by the runtime composition that owns graph preparation. The revision SHALL identify the complete graph whose exact Patch order and capacities the snapshot targets, SHALL NOT create a second mutable copy of synth state, and SHALL be covered by the production-owned serialized leaf schema.

#### Scenario: Initial accepted Patches are projected
- **WHEN** the accepted Patch set is projected for a newly prepared initial graph
- **THEN** its parameter snapshot and StateTree parameters branch carry that graph's exact revision and ordered Patch identities

#### Scenario: Projection revision is absent or stale
- **WHEN** a parameter projection has zero revision or targets another graph
- **THEN** it is rejected from active audio consumption rather than relabeled, partially applied, or treated as a fallback

## MODIFIED Requirements

### Requirement: Ordered one-way state transition
Every supported control input SHALL be normalized into a semantic event, and accepted state SHALL be committed before serialization, text projection, graph-revision-tagged parameter publication, or audio-command effects are produced. Structural graph ownership SHALL remain outside `AppState` and SHALL cross only through its dedicated handoff.

#### Scenario: Accepted parameter edit
- **WHEN** a valid adjustment event targets the selected parameter
- **THEN** the accepted state generation advances first and the resulting serialized state, text projection, parameter snapshot for the active target graph revision, and emitted effects all represent that same generation

#### Scenario: Rejected event
- **WHEN** an event is rejected by a declared state invariant
- **THEN** state and parameter generations remain unchanged, the rejection is recorded, and later valid events can still be processed

### Requirement: Cross-projection equality
Serialized state and text SHALL contain the exact immutable capability registry and generic Patch instrument configs accepted for the same state generation, and serialized state, text, and published real-time parameters SHALL contain the exact current Patch identity, editable Patch parameter, global parameter, and selection values for that generation. The StateTree parameters branch and real-time snapshot SHALL additionally agree on the target graph revision and ordered Patch identities.

#### Scenario: Inspect an accepted Patch installation
- **WHEN** generic Patch configs are installed through the production reducer and their initial graph is prepared
- **THEN** the state tree and text projection contain the same descriptor ids, parameter specs, capability ids, assignments, and asset references in canonical order, and the tree's parameter branch targets the prepared graph revision

#### Scenario: Inspect an accepted edit
- **WHEN** any Patch or global parameter edit is accepted
- **THEN** the state tree, selected text value, and corresponding real-time parameter contain the same exact value and target graph revision while all unrelated values and immutable instrument configs remain unchanged

