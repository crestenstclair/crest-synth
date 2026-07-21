## MODIFIED Requirements

### Requirement: Stable structured observations
Observation mode SHALL emit exactly one deterministic JSON `CREST_EVENT_LOG`, one `CREST_STATE_TREE`, and one `CREST_OBSERVATION` summary with deliberately versioned schemas, explicit missing and unexpected coverage identifiers, graph-revision-tagged parameter data, and no opaque debug representation used as evidence.

#### Scenario: Maintainer inspects observation output
- **WHEN** the exhaustive demo completes successfully
- **THEN** each required marker has one schema-valid JSON value whose identifiers, ordering, state generations, graph revisions, hashes, and values are deterministic

### Requirement: Production-derived coverage schema
The expected coverage universe SHALL be derived from production-owned input, installed capability and capability-parameter descriptors, other typed semantic and parameter descriptors, and discovered serialized leaves including graph revision; a separately hand-maintained string list SHALL NOT define a passing universe.

#### Scenario: Required serialized leaf is removed
- **WHEN** a schema test removes one required capability, config, graph-revision, or other discovered leaf
- **THEN** exact schema equality fails and reports that leaf as missing

#### Scenario: Unexpected serialized leaf is inserted
- **WHEN** a schema test inserts one undeclared leaf
- **THEN** exact schema equality fails and reports that leaf as unexpected

#### Scenario: Capability descriptor differs from coverage expectation
- **WHEN** an installed descriptor adds, removes, renames, duplicates, or reorders a parameter
- **THEN** the production descriptor defines the expected surface and any stale duplicate expectation fails verification

### Requirement: Exact state and projection observation
The state tree SHALL contain the complete installed capability registry, every Patch generic instrument config and asset reference, every Patch identity and editable parameter, every global parameter, all selection properties, every text projection property, and every graph-revision-tagged parameter snapshot property with exact values from the same accepted generation.

#### Scenario: State tree is compared with projections
- **WHEN** an accepted transition produces new projections
- **THEN** each required descriptor, config, graph revision, and current control property exists with its exact expected value and the state tree, text projection, and compatible parameter snapshot agree for that generation

#### Scenario: Capability config is malformed
- **WHEN** verification attempts unknown, duplicate, missing, undeclared, wrong-kind, dependency-invalid, or out-of-range config data
- **THEN** production installation rejects it without partial state change, graph publication, or fallback and the rejection is asserted before any acceptance marker is printed

### Requirement: Headless demo remains an independent deterministic proof
Adding the Phase 2 prepared engine rack SHALL preserve the `make demo` command, its headless/no-device/no-window execution, deterministic timing, two-run equality, structured output markers, mutation cases, and behavioral acceptance predicates. Its production-derived schema universe SHALL include the installed capability descriptors, generic Patch configs, and graph-revision-tagged parameter projection.

#### Scenario: Headless demo runs after the prepared-rack migration
- **WHEN** the maintainer runs `make demo`
- **THEN** the exhaustive scene produces a deterministic event log, state tree, observation, exact descriptor-derived coverage, graph-compatible parameter evidence, and controlled-negative behavior without opening a native window or physical device

#### Scenario: Capability or graph schema changes
- **WHEN** the canonical registry, Patch config, or parameter graph-revision serialization changes
- **THEN** both runs agree byte-for-byte on the new schema and exact production-derived coverage fails if any declared or discovered field is absent or unexpected

### Requirement: Existing verification gates remain required
Phase 2 increment 2 completion SHALL require the named prepared-engine-rack contract in addition to the capability-schema, exhaustive-demo, schema-surface, live-demo, GUI-context, mutation, real-time, smoke, control-dispatch-performance, format, lint, and all-target checks.

#### Scenario: Prepared-rack behavior passes but another gate fails
- **WHEN** the prepared-engine-rack contract passes and any existing required behavioral or project check fails
- **THEN** the Phase 2 increment is incomplete and cannot be accepted

#### Scenario: Existing behavior passes but prepared-rack proof fails
- **WHEN** existing Phase 1 and capability-model gates pass but exact heterogeneous routing, structural handoff, retirement-pressure, or callback lifecycle assertions fail
- **THEN** the Phase 2 increment is incomplete and cannot be accepted

