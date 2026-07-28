## MODIFIED Requirements

### Requirement: Production-derived coverage schema
The expected coverage universe SHALL be derived from production-owned input, installed capability and capability-parameter descriptors, other typed semantic and parameter descriptors, and discovered serialized leaves; a separately hand-maintained string list SHALL NOT define a passing universe.

#### Scenario: Required serialized leaf is removed
- **WHEN** a schema test removes one required capability, config, or other discovered leaf
- **THEN** exact schema equality fails and reports that leaf as missing

#### Scenario: Unexpected serialized leaf is inserted
- **WHEN** a schema test inserts one undeclared leaf
- **THEN** exact schema equality fails and reports that leaf as unexpected

#### Scenario: Capability descriptor differs from coverage expectation
- **WHEN** an installed descriptor adds, removes, renames, duplicates, or reorders a parameter
- **THEN** the production descriptor defines the expected surface and any stale duplicate expectation fails verification

### Requirement: Exact state and projection observation
The state tree SHALL contain the complete installed capability registry, every Patch generic instrument config and asset reference, every Patch identity and editable parameter, every global parameter, all selection properties, every text projection property, and every parameter snapshot property with exact values from the same accepted generation.

#### Scenario: State tree is compared with projections
- **WHEN** an accepted transition produces new projections
- **THEN** each required descriptor, config, and current control property exists with its exact expected value and the state tree, text projection, and parameter snapshot agree for that generation

#### Scenario: Capability config is malformed
- **WHEN** verification attempts unknown, duplicate, missing, undeclared, wrong-kind, dependency-invalid, or out-of-range config data
- **THEN** production installation rejects it without partial state change or fallback and the rejection is asserted before any acceptance marker is printed

### Requirement: Headless demo remains an independent deterministic proof
Adding the Phase 2 capability model SHALL preserve the `make demo` command, its headless/no-device/no-window execution, deterministic timing, two-run equality, structured output markers, mutation cases, and behavioral acceptance predicates. Its production-derived schema universe SHALL deliberately expand to include the installed capability descriptors and generic Patch configs.

#### Scenario: Headless demo runs after capability polymorphism is added
- **WHEN** the maintainer runs `make demo`
- **THEN** the exhaustive scene produces a deterministic event log, state tree, observation, exact descriptor-derived coverage, and controlled-negative behavior without opening a native window or physical device

#### Scenario: Capability schema changes
- **WHEN** the canonical registry or Patch config serialization changes
- **THEN** both runs agree byte-for-byte on the new schema and exact production-derived coverage fails if any declared or discovered field is absent or unexpected

### Requirement: Existing verification gates remain required
Phase 2 increment 1 completion SHALL require the named capability-schema test in addition to the exhaustive-demo, schema-surface, live-demo, GUI-context, mutation, real-time, smoke, format, lint, and all-target checks.

#### Scenario: Capability behavior passes but another gate fails
- **WHEN** the capability-schema test passes and any existing required behavioral or project check fails
- **THEN** the Phase 2 increment is incomplete and cannot be accepted

#### Scenario: Existing behavior passes but capability schema fails
- **WHEN** existing Phase 1 gates pass but exact descriptor/config or no-fallback assertions fail
- **THEN** the Phase 2 increment is incomplete and cannot be accepted

