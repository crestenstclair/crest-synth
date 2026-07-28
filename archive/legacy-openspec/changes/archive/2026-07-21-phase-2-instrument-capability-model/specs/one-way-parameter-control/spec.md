## MODIFIED Requirements

### Requirement: Single complete text view
The user interface SHALL remain one scrollable text view listing the installed capability identity and descriptor-driven instrument config for every Patch, every current editable Patch parameter, and all global parameters, with Patch sections separated by `------------------------------------------------------------` and the selected line visibly identified.

#### Scenario: View is projected from current state
- **WHEN** installed Patches, generic instrument configs, parameters, or selection change
- **THEN** the one text body contains every exact current value, walks instrument fields in production descriptor order, includes the required Patch separators, and places a selection marker at the projected selected line

### Requirement: Cross-projection equality
Serialized state and text SHALL contain the exact immutable capability registry and generic Patch instrument configs accepted for the same state generation, and serialized state, text, and published real-time parameters SHALL contain the exact current Patch identity, editable Patch parameter, global parameter, and selection values for that generation.

#### Scenario: Inspect an accepted Patch installation
- **WHEN** generic Patch configs are installed through the production reducer
- **THEN** the state tree and text projection contain the same descriptor ids, parameter specs, capability ids, assignments, and asset references in canonical order

#### Scenario: Inspect an accepted edit
- **WHEN** any Patch or global parameter edit is accepted
- **THEN** the state tree, selected text value, and corresponding real-time parameter contain the same exact value and all unrelated values and immutable instrument configs remain unchanged

### Requirement: Replaceable external boundaries
Instrument capability metadata, sound generation, MIDI input, audio output, text rendering, and the real-time handoff SHALL be expressed through stable behavioral boundaries so one conforming adapter can be replaced without changing domain behavior.

#### Scenario: Production and headless adapters share behavior
- **WHEN** a production adapter is replaced by a conforming headless test adapter
- **THEN** the same capability descriptors, configs, normalized inputs, and semantic events produce equivalent accepted state and projections

#### Scenario: Capability provider is unavailable
- **WHEN** the declared instrument provider is missing or does not match the installed registry
- **THEN** startup fails visibly without selecting a fallback capability or sound generator

## ADDED Requirements

### Requirement: Responsive sustained MIDI projection
Accepted MIDI SHALL continue through `AppState::apply`, canonical generation advancement, state/text/parameter/tree projection, EventLog recording, and audio-command publication without deep-cloning immutable capability/Patch state, parsing Crest's own serialized JSON, rebuilding an unchanged text body, or eagerly materializing large JSON documents that no observer requested. A fifteen-Patch production-path acceptance fixture SHALL dispatch 512 MIDI events within 50 milliseconds in Cargo's unoptimized test profile.

#### Scenario: A dense fifteen-Patch MIDI batch is dispatched
- **WHEN** 512 normalized MIDI events target installed Patches through the production `AppLoop`
- **THEN** every event has one accepted generation, coherent logical projections, journal record, parameter publication, and audio command, no records are dropped, and the batch completes within 50 milliseconds

#### Scenario: Deferred projection JSON is inspected
- **WHEN** a generation-only StateSnapshot or StateTree is materialized after MIDI dispatch
- **THEN** its JSON, hash, text selection, parameter generation, and complete nested values exactly equal an eager canonical projection from the same accepted `AppState`
