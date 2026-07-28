## MODIFIED Requirements

### Requirement: Exact state and projection observation
The state tree SHALL contain the complete two-capability registry, every Patch generic instrument config and asset reference, every Patch identity, output track, trim, common ADSR value, and encoded engine Scalar value, exactly sixteen persistent mixer-track parameter sets, every global parameter, all InteractionState context/surface/focus properties, the complete active PatchPageProjection when PATCH is selected, every semantic graphical and text projection property, and every graph-revision-tagged parameter snapshot property with exact values from the same accepted generation. The separate audio observation SHALL correlate that generation with exactly sixteen track-identified numeric meters and SHALL NOT become writable canonical or view-owned state.

#### Scenario: Context state is compared with projections
- **WHEN** an accepted context or surface selection produces new projections
- **THEN** InteractionState, SemanticGraphicalViewModel, optional PatchPageProjection, TextProjection, StateTree, EventRecord, and ParameterSnapshot agree on context, stable Patch-or-track focus, generation, state hash, exact values, and graph revision while session and audio values remain unchanged

#### Scenario: State tree is compared with projections
- **WHEN** an accepted Patch-output, envelope, engine, effect, track, or global transition produces new projections
- **THEN** canonical Patch outputs and sixteen-track state, semantic and text projections, active PATCH projection when applicable, EventRecord, compatible ParameterSnapshot, generation, state hash, and graph revision agree exactly while audio meters retain separate observation provenance

#### Scenario: State tree is compared with Patch projections
- **WHEN** an accepted Patch-output, envelope, engine, or effect transition produces new projections in PATCH
- **THEN** each required descriptor, config, Patch output, graph revision, Patch value, and interaction property exists with its exact expected value and the state tree, page, text, semantic, and compatible parameter projections agree for that generation

#### Scenario: State tree is compared with mixer projections
- **WHEN** an accepted track or global transition produces new projections in MIXER
- **THEN** all sixteen track identities and parameter sets remain present, the targeted track and control value agree across canonical state, focus, semantic/text projections, and the compatible parameter snapshot, every unrelated track is exact, and the matching meter retains its separate audio-observation provenance

#### Scenario: Shared-track behavior is compared
- **WHEN** two discriminating Patch stems share one destination and that track is adjusted
- **THEN** the expected contribution of both Patches changes after their exact Patch-local output values, no untargeted track changes, and a cross-track parameter-leak mutant falsifies the same production-path assertions

#### Scenario: Capability config or Patch route is malformed
- **WHEN** verification attempts unknown, duplicate, missing, undeclared, wrong-kind, dependency-invalid, non-finite, out-of-range, over-scalar-capacity, Structural-as-Scalar config data, or an invalid mixer-track destination
- **THEN** production validation rejects it without partial state change, graph publication, route fallback, page fallback, or engine fallback and the rejection is asserted before any acceptance marker is printed

#### Scenario: Capability config is malformed
- **WHEN** verification attempts unknown, duplicate, missing, undeclared, wrong-kind, dependency-invalid, non-finite, out-of-range, over-scalar-capacity, or Structural-as-Scalar config data
- **THEN** production validation rejects it without partial state change, graph publication, page fallback, or engine fallback and the rejection is asserted before any acceptance marker is printed
