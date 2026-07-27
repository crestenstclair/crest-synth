# Observable Demo Scene

## Purpose

Define deterministic, production-derived observation and falsifiable behavioral evidence for the application's complete control, projection, audio, and GUI surface.
## Requirements
### Requirement: Deterministic production-path demo
The headless demo SHALL initialize the real fixed fixture with alternating SoundFont and Braids configs and drive the production normalized `1`, `2`, W, S, A, D, and K input translator, control loop, context/page projections, real-time boundary, mixed prepared rack, and mixer without a native window, physical audio device, wall clock, or random input.

#### Scenario: Demo runs twice
- **WHEN** the complete mixed-engine two-context scene runs twice from fresh identical services
- **THEN** both runs produce byte-identical event logs, state trees, coverage, checkpoints, Patch-page projections, and report JSON

### Requirement: Stable structured observations
Observation mode SHALL emit exactly one deterministic JSON `CREST_EVENT_LOG`, one `CREST_STATE_TREE`, and one `CREST_OBSERVATION` summary with deliberately versioned schemas, explicit missing and unexpected coverage identifiers, graph-revision-tagged parameter data, the complete PATCH focused-control surface, and no opaque debug representation used as evidence. Adding focused-control or envelope-row leaves SHALL advance the applicable observation schema versions and production-owned serialized-leaf descriptors together.

#### Scenario: Maintainer inspects observation output
- **WHEN** the exhaustive demo completes successfully
- **THEN** each required marker has one schema-valid JSON value whose identifiers, ordering, state generations, graph revisions, hashes, focused control, envelope values, and other values are deterministic

#### Scenario: PATCH observation shape changes without a version update
- **WHEN** a focused-control or envelope-row leaf is added, removed, or renamed while a stale observation schema version or typed leaf descriptor remains
- **THEN** exact schema verification fails and the demo cannot report completion

#### Scenario: PATCH focus state is compared with projections
- **WHEN** an accepted PATCH navigation changes control focus
- **THEN** InteractionState, PatchPageProjection focused control, text marker, selected line, StateTree, EventRecord, and unchanged compatible ParameterSnapshot agree exactly and no audio or structural effect exists

### Requirement: Exhaustive current behavior surface
The deterministic demo SHALL cover the production-derived GUI, semantic event, state, projection, audio, serialized, and PATCH Engine-plus-ADSR focus surfaces plus both successful engine directions and their request, preparation, failure, publication, activation, acknowledgement, and retirement outcomes. It SHALL use deterministic scheduling with real providers, preparers, graph builder, renderer, and reducer, SHALL exercise all four focused-Patch ADSR values through PATCH without duplicate coverage credit, and SHALL require exact restoration or the declared final default SoundFont config.

#### Scenario: Coverage is compared
- **WHEN** the scene finishes exercising the current surface
- **THEN** observed input, event, context, direction, rejection, effect, lifecycle, Patch-control, descriptor-row, editable-parameter, and serialized-leaf sets exactly equal their production-derived expectations, retain 17 normalized inputs, and have empty missing and unexpected sets

#### Scenario: PATCH focus and ADSR controls are exercised
- **WHEN** normalized bare and Edit-modified directional input traverses Engine, Attack, Decay, Sustain, and Release on the focused Patch
- **THEN** all five control identities and all four ADSR edits are observed exactly once, focus-only transitions preserve values and audio, fine/coarse edits change only the selected canonical field, and projections and compatible snapshots agree with every accepted generation

#### Scenario: Read-only PATCH rejection recovers
- **WHEN** PATCH receives horizontal navigation, endpoint navigation, a vertical Engine adjustment, or another unavailable action before or after a valid focus or ADSR event and then returns to MIXER
- **THEN** the unchanged rejection and following accepted events retain exact generations, hashes, projections, effects, MIXER selection, and later-event processing

#### Scenario: ADSR edit coexists with structural work
- **WHEN** one focused PATCH ADSR edit occurs during Preparing and another occurs during Activating
- **THEN** the Preparing edit is source-revision compatible and refreshes into the candidate, the Activating edit targets the replacement revision, the activated engine consumes both latest canonical values, the old source remains finite, and neither edit publishes structural work

#### Scenario: All-notes-off paths are exercised
- **WHEN** MIDI coverage reaches all-notes-off behavior on both engines
- **THEN** Patch-scoped semantic all-notes-off and the separate renderer all-notes-off command each have a unique covered identity and measured consequence

#### Scenario: SoundFont changes to Braids and back
- **WHEN** the first Patch returns focus to Engine and requests the adjacent engine in each direction through normalized keyboard input while the deterministic worker is advanced
- **THEN** source audio remains finite during preparation, each target commits before publication and acknowledges a newer graph, targeted MIDI proves its exact target stem, focus and envelope survive, all untargeted Patches remain exact, and the final config is descriptor-default SoundFont

#### Scenario: Controlled failure, busy, and stale outcomes are observed
- **WHEN** preparation returns the declared typed failure, another Engine request arrives while busy, or a result or acknowledgement is stale or mismatched
- **THEN** config, revision, layout, focus, envelope, projection, and audio preservation are exact, no invalid graph or fallback is published, rejected generations/hashes are unchanged, and later healthy focus, scalar, and engine events succeed

#### Scenario: Engine-selection evidence is falsifiable
- **WHEN** claimed success retains the old engine, uses unrelated output, loses an accepted ADSR value, is silent or non-finite, substitutes a fallback, omits publication or acknowledgement, leaks another Patch config, or destroys callback-owned data
- **THEN** the structured predicates fail and neither the engine-selection marker nor complete demo report is emitted

#### Scenario: Demo runs twice
- **WHEN** the complete scene runs twice from freshly constructed identical services
- **THEN** the full logical EventLog, StateTree, coverage, focus and lifecycle checkpoints, graph revisions, observations, and report JSON are byte-identical with no excluded declared field

#### Scenario: Existing verification gate fails
- **WHEN** PATCH ADSR and engine selection pass but any required format, lint, all-target, runtime, schema, UI, mutation, smoke, rack, engine, envelope, exhaustive, live, or performance gate fails
- **THEN** the increment remains incomplete

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

### Requirement: Exact causal event records
Every scene input SHALL produce one accepted or rejected event record containing its source, tagged payload, outcome or rejection, before and after generations and state hashes, exact emitted effects, parameter generation, projection identity, and selected line as observed from the production transition.

#### Scenario: Scene step is recorded
- **WHEN** a startup, keyboard, automatic-MIDI, demo-scene, or system input is dispatched
- **THEN** its event record matches an oracle fixed before dispatch and its state, projection, command, and audio consequences agree causally

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

### Requirement: Audible mixed-engine parameter proof
Deterministic acceptance SHALL render at least one real SoundFont Patch and one real Braids Patch simultaneously through the production reducer, snapshot, rack, stems, and mixer. It SHALL adjust every Braids engine parameter and every common ADSR parameter and require a controlled finite waveform, energy, or envelope-time difference attributable only to the selected value.

#### Scenario: Every engine and envelope control is exercised
- **WHEN** controlled notes are rendered before and after one accepted parameter change from identical prepared comparison state
- **THEN** the target engine's declared measurement changes, untargeted Patch state and stems remain exact, and a zero renderer or ignored parameter cannot pass

