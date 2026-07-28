## ADDED Requirements

### Requirement: Effect scalar adjustment uses the canonical reducer path
Every Patch-effect parameter adjustment SHALL originate as semantic input, pass through `AppState::apply`, commit the matching `PostEffectConfig` assignment, and only then derive EventRecord, StateTree, PatchPageProjection, TextProjection, and the complete graph-compatible `ParameterSnapshot`. Adapters, views, native processors, demos, and audio code SHALL NOT mutate canonical effect values directly or maintain another writable copy.

#### Scenario: Chorus Amount is adjusted from keyboard input
- **WHEN** the focused Amount row receives Edit+Right
- **THEN** the translator emits one semantic Adjust, `AppState::apply` accepts exactly a `+0.01` change, the committed generation precedes every projection/publication, and the prepared effect later consumes the matching scalar

#### Scenario: Adjustment is rejected
- **WHEN** an effect scalar is already at its bound or its slot/parameter identity is stale or invalid
- **THEN** state, generation, projections, parameter publication, audio-command count, and graph queues remain unchanged and a later valid event is still accepted

#### Scenario: Versioned effect leaves are serialized
- **WHEN** a state containing a configured effect is projected after this change
- **THEN** the advanced StateTree/page/text/parameter schemas expose exact registry, slot, assignment, focus, and fixed scalar leaves from the same accepted generation and schema equality rejects a missing, duplicated, renamed, or unexpected leaf

