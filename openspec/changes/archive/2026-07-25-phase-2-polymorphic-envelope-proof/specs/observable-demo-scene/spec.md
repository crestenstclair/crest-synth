## MODIFIED Requirements

### Requirement: Deterministic production-path demo
The headless demo SHALL initialize the real fixed fixture with alternating SoundFont and Braids configs and drive the production normalized W, S, A, D, and K input translator, control loop, projections, real-time boundary, mixed prepared rack, and mixer without a native window, physical audio device, wall clock, or random input.

#### Scenario: Demo runs twice
- **WHEN** the complete mixed-engine scene runs twice from fresh identical services
- **THEN** both runs produce byte-identical event logs, state trees, coverage, checkpoints, and report JSON

### Requirement: Exhaustive current behavior surface
The demo and table-driven verification SHALL cover every declared semantic event variant and direction, the union of installed descriptor-supported MIDI kinds, every valid normalized GUI input, every editable Patch mixer and ADSR value on every installed Patch, every descriptor-classified Scalar engine parameter on each matching Patch, all seven global parameters, every serialized state and projection property, accepted and rejected outcomes, every event source, and every emitted downstream effect.

#### Scenario: Coverage is compared
- **WHEN** the scene finishes exercising the current surface
- **THEN** the observed set exactly equals the production-derived complete expected set, includes Model/Timbre/Color only for Braids Patches, and both missing and unexpected sets are empty

#### Scenario: All-notes-off paths are exercised
- **WHEN** MIDI coverage reaches all-notes-off behavior on both engines
- **THEN** Patch-scoped MIDI all-notes-off and the separate renderer all-notes-off command each have a unique covered identity and measured consequence

### Requirement: Exact state and projection observation
The state tree SHALL contain the complete two-capability registry, every Patch generic instrument config and asset reference, every Patch identity, mixer value, common ADSR value, encoded engine Scalar value, every global parameter, all selection properties, every text projection property, and every graph-revision-tagged parameter snapshot property with exact values from the same accepted generation.

#### Scenario: State tree is compared with projections
- **WHEN** an accepted mixer, envelope, or engine transition produces new projections
- **THEN** each required descriptor, config, graph revision, Patch value, and current control property exists with its exact expected value and the state tree, text projection, and compatible parameter snapshot agree for that generation

#### Scenario: Capability config is malformed
- **WHEN** verification attempts unknown, duplicate, missing, undeclared, wrong-kind, dependency-invalid, out-of-range, over-scalar-capacity, or Structural-as-Scalar config data
- **THEN** production validation rejects it without partial state change, graph publication, or fallback and the rejection is asserted before any acceptance marker is printed

### Requirement: Headless demo remains an independent deterministic proof
Adding the production Braids engine and common per-voice envelope SHALL preserve the `make demo` command, its headless/no-device/no-window execution, deterministic timing, two-run equality, structured output markers, mutation cases, and behavioral acceptance predicates. Its production-derived schema universe SHALL include both installed descriptors, mixed generic Patch configs, ADSR, capability scalars, and graph-revision-tagged parameter projection.

#### Scenario: Headless demo runs after the prepared-rack migration
- **WHEN** the maintainer runs `make demo`
- **THEN** the exhaustive mixed scene produces a deterministic event log, state tree, observation, exact descriptor-derived coverage, graph-compatible parameter evidence, audible engine/envelope evidence, and controlled-negative behavior without opening a native window or physical device

#### Scenario: Capability or graph schema changes
- **WHEN** the canonical registry, Patch config/envelope, Scalar layout, or graph-revision serialization changes
- **THEN** both runs agree byte-for-byte on the new schema and exact production-derived coverage fails if any declared or discovered field is absent or unexpected

### Requirement: Existing verification gates remain required
Phase 2 Braids increment completion SHALL require the named Braids-engine and per-voice-envelope contracts in addition to prepared-engine-rack, capability-schema, exhaustive-demo, schema-surface, live-demo, GUI-context, mutation, real-time, smoke, control-dispatch-performance, format, lint, and all-target checks.

#### Scenario: Prepared-rack behavior passes but another gate fails
- **WHEN** the Braids and mixed-rack contracts pass and any existing required behavioral or project check fails
- **THEN** the Phase 2 increment is incomplete and cannot be accepted

#### Scenario: Existing behavior passes but prepared-rack proof fails
- **WHEN** existing gates pass but source provenance, independent sixteen-voice ownership for every admitted Braids Patch, engine-managed single-instance SoundFont polyphony, per-note ADSR, exact mixed routing, parameter isolation, FFI lifecycle, or timing assertions fail
- **THEN** the Phase 2 increment is incomplete and cannot be accepted

## ADDED Requirements

### Requirement: Audible mixed-engine parameter proof
Deterministic acceptance SHALL render at least one real SoundFont Patch and one real Braids Patch simultaneously through the production reducer, snapshot, rack, stems, and mixer. It SHALL adjust every Braids engine parameter and every common ADSR parameter and require a controlled finite waveform, energy, or envelope-time difference attributable only to the selected value.

#### Scenario: Every engine and envelope control is exercised
- **WHEN** controlled notes are rendered before and after one accepted parameter change from identical prepared comparison state
- **THEN** the target engine's declared measurement changes, untargeted Patch state and stems remain exact, and a zero renderer or ignored parameter cannot pass
