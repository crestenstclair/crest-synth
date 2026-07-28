## MODIFIED Requirements

### Requirement: Exhaustive current behavior surface
The deterministic demo SHALL cover the production-derived GUI, semantic event, state, projection, audio, and serialized surfaces plus both successful engine directions and their request, preparation, failure, publication, activation, acknowledgement, and retirement outcomes. It SHALL use deterministic scheduling with real providers, preparers, graph builder, renderer, and reducer, and SHALL require exact restoration or the declared final default SoundFont config.

#### Scenario: Coverage is compared
- **WHEN** the scene finishes exercising the current surface
- **THEN** observed input, event, context, direction, rejection, effect, lifecycle, descriptor-row, and serialized-leaf sets exactly equal their production-derived expectations, retain 17 normalized inputs, and have empty missing and unexpected sets

#### Scenario: Read-only PATCH rejection recovers
- **WHEN** PATCH receives an unavailable non-engine action before or after a valid engine request and then returns to MIXER
- **THEN** the unchanged rejection and following accepted events retain exact generations, hashes, projections, effects, MIXER selection, and later-event processing

#### Scenario: All-notes-off paths are exercised
- **WHEN** MIDI coverage reaches all-notes-off behavior on both engines
- **THEN** Patch-scoped semantic all-notes-off and the separate renderer all-notes-off command each have a unique covered identity and measured consequence

#### Scenario: SoundFont changes to Braids and back
- **WHEN** the first Patch requests the adjacent engine in each direction through normalized keyboard input and the deterministic worker is advanced
- **THEN** source audio remains finite during preparation, each target commits before publication and acknowledges a newer graph, targeted MIDI proves its exact target stem, all untargeted Patches remain exact, and the final config is descriptor-default SoundFont

#### Scenario: Controlled failure, busy, and stale outcomes are observed
- **WHEN** preparation returns the declared typed failure, another request arrives while busy, or a result or acknowledgement is stale or mismatched
- **THEN** config, revision, layout, projection, and audio preservation are exact, no invalid graph or fallback is published, rejected generations/hashes are unchanged, and a later healthy request succeeds

#### Scenario: Engine-selection evidence is falsifiable
- **WHEN** claimed success retains the old engine, uses unrelated output, is silent or non-finite, substitutes a fallback, omits publication or acknowledgement, leaks another Patch config, or destroys callback-owned data
- **THEN** the structured predicates fail and neither the engine-selection marker nor complete demo report is emitted

#### Scenario: Demo runs twice
- **WHEN** the complete scene runs twice from freshly constructed identical services
- **THEN** the full logical EventLog, StateTree, coverage, lifecycle checkpoints, graph revisions, observations, and report JSON are byte-identical with no excluded declared field

#### Scenario: Existing verification gate fails
- **WHEN** engine selection passes but any required format, lint, all-target, runtime, schema, UI, mutation, smoke, rack, engine, envelope, exhaustive, live, or performance gate fails
- **THEN** the increment remains incomplete
