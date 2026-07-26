## MODIFIED Requirements

### Requirement: Deterministic production-path demo
The headless demo SHALL initialize the real fixed fixture with alternating SoundFont and Braids configs and drive the production normalized `1`, `2`, W, S, A, D, and K input translator, control loop, context/page projections, real-time boundary, mixed prepared rack, and mixer without a native window, physical audio device, wall clock, or random input.

#### Scenario: Demo runs twice
- **WHEN** the complete mixed-engine two-context scene runs twice from fresh identical services
- **THEN** both runs produce byte-identical event logs, state trees, coverage, checkpoints, Patch-page projections, and report JSON

### Requirement: Exhaustive current behavior surface
The demo and table-driven verification SHALL cover every declared semantic event variant and direction, both top-level contexts, the union of installed descriptor-supported MIDI kinds, every valid normalized GUI input, every editable Patch mixer and ADSR value on every installed Patch, every descriptor-classified Scalar engine parameter on each matching Patch, all seven global parameters, every serialized state and projection property including InteractionState and PatchPageProjection, accepted and rejected outcomes including `ActionUnavailableInContext`, every event source, and every emitted downstream effect.

#### Scenario: Coverage is compared
- **WHEN** the scene finishes exercising the current surface
- **THEN** the observed set exactly equals the production-derived complete expected set, contains 17 normalized inputs, five semantic event variants, both contexts, 11 rejection variants, descriptor-derived Patch rows for both installed engines, and empty missing and unexpected sets

#### Scenario: Read-only PATCH rejection recovers
- **WHEN** the scene selects PATCH, attempts Navigate or Adjust, and then selects MIXER
- **THEN** the typed unchanged rejection and following accepted context event are both recorded with exact generations, hashes, projections, parameter publication effects, and no audio or structural command from the rejection

#### Scenario: All-notes-off paths are exercised
- **WHEN** MIDI coverage reaches all-notes-off behavior on both engines
- **THEN** Patch-scoped MIDI all-notes-off and the separate renderer all-notes-off command each have a unique covered identity and measured consequence

### Requirement: Exact state and projection observation
The state tree SHALL contain the complete two-capability registry, every Patch generic instrument config and asset reference, every Patch identity, mixer value, common ADSR value, encoded engine Scalar value, every global parameter, all InteractionState context/focus/MIXER-selection properties, the complete active PatchPageProjection when PATCH is selected, every context-tagged text projection property, and every graph-revision-tagged parameter snapshot property with exact values from the same accepted generation.

#### Scenario: Context state is compared with projections
- **WHEN** an accepted context selection produces new projections
- **THEN** InteractionState, optional PatchPageProjection, TextProjection, StateTree, EventRecord, and ParameterSnapshot agree on context, stable focus, generation, state hash, exact values, and graph revision while session and audio values remain unchanged

#### Scenario: State tree is compared with projections
- **WHEN** an accepted mixer, envelope, or engine transition produces new projections in MIXER
- **THEN** each required descriptor, config, graph revision, Patch value, and current interaction property exists with its exact expected value and the state tree, text projection, and compatible parameter snapshot agree for that generation

#### Scenario: Capability config is malformed
- **WHEN** verification attempts unknown, duplicate, missing, undeclared, wrong-kind, dependency-invalid, out-of-range, over-scalar-capacity, or Structural-as-Scalar config data
- **THEN** production validation rejects it without partial state change, graph publication, page fallback, or engine fallback and the rejection is asserted before any acceptance marker is printed
