## MODIFIED Requirements

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
