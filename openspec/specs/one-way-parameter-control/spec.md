# One-Way Parameter Control

## Purpose

Define the normalized one-way control pipeline, exact keyboard vocabulary, bounded parameter behavior, complete text projection, and replaceable external boundaries.
## Requirements
### Requirement: Ordered one-way state transition
Every supported control input SHALL be normalized into a semantic event, and accepted state SHALL be committed before serialization, text projection, graph-revision-tagged parameter publication, or audio-command effects are produced. Structural graph ownership SHALL remain outside `AppState` and SHALL cross only through its dedicated handoff.

#### Scenario: Accepted parameter edit
- **WHEN** a valid adjustment event targets the selected parameter
- **THEN** the accepted state generation advances first and the resulting serialized state, text projection, parameter snapshot for the active target graph revision, and emitted effects all represent that same generation

#### Scenario: Rejected event
- **WHEN** an event is rejected by a declared state invariant
- **THEN** state and parameter generations remain unchanged, the rejection is recorded, and later valid events can still be processed

### Requirement: Projection targets one prepared graph revision
Every fixed parameter snapshot and its StateTree parameters projection SHALL contain the same nonzero graph revision supplied by the runtime composition that owns graph preparation. The revision SHALL identify the complete graph whose exact Patch order and capacities the snapshot targets, SHALL NOT create a second mutable copy of synth state, and SHALL be covered by the production-owned serialized leaf schema.

#### Scenario: Initial accepted Patches are projected
- **WHEN** the accepted Patch set is projected for a newly prepared initial graph
- **THEN** its parameter snapshot and StateTree parameters branch carry that graph's exact revision and ordered Patch identities

#### Scenario: Projection revision is absent or stale
- **WHEN** a parameter projection has zero revision or targets another graph
- **THEN** it is rejected from active audio consumption rather than relabeled, partially applied, or treated as a fallback

### Requirement: Exact keyboard vocabulary
Digit `1` SHALL directly select MIXER and Digit `2` SHALL directly select PATCH. Bare W and S SHALL navigate parameters, bare A and D SHALL navigate Patch or GLOBAL sections, and holding K with W, S, A, or D SHALL adjust only the selected bounded value while MIXER is active. Digit key-up and every other supported key-up SHALL emit no semantic event, K SHALL act only as a modifier, and no physical key SHALL mutate application state directly.

#### Scenario: Select a context
- **WHEN** the player presses `1` or `2`
- **THEN** the shared input translator emits the corresponding semantic `SelectContext` event without reading AppState or changing window-owned page state

#### Scenario: Navigate without editing
- **WHEN** MIXER is active and the player presses bare W, S, A, or D
- **THEN** selection moves according to the declared wrap and clamp rules without changing a synth parameter

#### Scenario: Edit the selected value
- **WHEN** MIXER is active and the player holds K and presses W, S, A, or D
- **THEN** exactly the selected bounded parameter receives the corresponding fine or coarse adjustment through the semantic event path

#### Scenario: Release a page key
- **WHEN** the player releases `1` or `2`, including while K is held
- **THEN** no semantic event is emitted and the current context remains reducer-owned

### Requirement: Nonfatal parameter boundaries
Every editable value SHALL enforce its typed lower and upper bounds, and an adjustment beyond a reached boundary SHALL be an unchanged nonfatal transition.

#### Scenario: Edit beyond a boundary
- **WHEN** the player adjusts a selected value toward an already reached lower or upper bound
- **THEN** the value remains unchanged, a boundary rejection is recorded, and the application accepts a subsequent valid edit

### Requirement: Single complete text view
The basic user interface SHALL remain one scrollable text shell rendering exactly one reducer-selected projection at a time. MIXER SHALL preserve the complete existing descriptor-derived Patch/global diagnostic body, separators, editable selection, and selected-line marker. PATCH SHALL render the focused immutable PatchPageProjection with Patch identity, MIDI channel, active engine and complete installed choices, canonical ADSR, and active descriptor fields with stable IDs and read-only status. Neither context SHALL contain a hard-coded SoundFont/Braids field list, and the window SHALL own no page state.

#### Scenario: View is projected from current state
- **WHEN** MIXER is active and installed Patches, generic configs, envelope/mixer values, Scalar engine values, global values, or MIXER selection change
- **THEN** the text body contains every exact current value, retains required Patch separators, follows production descriptor order, and places one marker at the projected selected line

#### Scenario: Patch capabilities expose different shapes
- **WHEN** navigation or Patch focus moves between a SoundFont Patch and a Braids Patch
- **THEN** MIXER derives its editable selection count from the active descriptor, skips SoundFont Structural fields, includes Braids Model, Timbre, and Color, and PATCH projects each different descriptor shape without a hard-coded engine field list

#### Scenario: PATCH is projected from current state
- **WHEN** PATCH is active for a focused SoundFont or Braids Patch
- **THEN** the text body is a lossless deterministic rendering of that canonical PatchPageProjection and displays Structural values without making any page row editable

#### Scenario: Context is round-tripped
- **WHEN** the player switches from a discriminating MIXER selection to PATCH and back
- **THEN** the same shell renders each canonical body in turn and returns to the exact retained MIXER selection and diagnostic values

### Requirement: Cross-projection equality
Serialized state and text SHALL contain the exact immutable capability registry and generic Patch instrument configs accepted for the same state generation, and serialized state, text, and published real-time parameters SHALL contain the exact current Patch identity, mixer values, common envelope, descriptor-classified Scalar instrument values, global values, and selection for that generation. The StateTree parameters branch and real-time snapshot SHALL additionally agree on the target graph revision, ordered Patch identities, and fixed Scalar layouts.

#### Scenario: Inspect an accepted Patch installation
- **WHEN** generic SoundFont and Braids configs are installed through the production reducer and their initial graph is prepared
- **THEN** the state tree and text projection contain the same descriptor ids, parameter specs, capability ids, assignments, asset references, and envelopes in canonical order, and the tree's parameter branch targets the prepared graph revision

#### Scenario: Inspect an accepted edit
- **WHEN** any editable Patch or global parameter is adjusted
- **THEN** the state tree, selected text value, and corresponding real-time parameter contain the same exact value and target graph revision while every unrelated Patch/config/envelope/value remains unchanged

### Requirement: Production GUI input uses the shared path
Real GUI key and focus events SHALL pass through the same normalized input translator and one-way control loop used by deterministic headless verification.

#### Scenario: Headless GUI frame processes an edit
- **WHEN** a headless GUI context sends a supported key or focus event through the production update callback
- **THEN** the next frame, event record, accepted state, exact text projection, and scroll target all reflect that event without opening a native window

### Requirement: Replaceable external boundaries
Instrument capability metadata, sound generation, MIDI input, audio output, text rendering, and the real-time handoff SHALL be expressed through stable behavioral boundaries so one conforming adapter can be replaced without changing domain behavior.

#### Scenario: Production and headless adapters share behavior
- **WHEN** a production adapter is replaced by a conforming headless test adapter
- **THEN** the same capability descriptors, configs, normalized inputs, and semantic events produce equivalent accepted state and projections

#### Scenario: Capability provider is unavailable
- **WHEN** the declared instrument provider is missing or does not match the installed registry
- **THEN** startup fails visibly without selecting a fallback capability or sound generator

### Requirement: Autonomous demo actions use the shared control path
Autonomous live-demo navigation, adjustment, MIDI, rejection-probe, and cleanup actions SHALL enter as semantic events through the same reducer, event log, state commit, projection, parameter-publication, and audio-command path used by keyboard and fixture inputs. The live runner and window SHALL NOT receive mutable canonical state or mutate projections directly.

#### Scenario: Live adjustment is accepted
- **WHEN** a due live action validly adjusts the selected parameter
- **THEN** the production event record, state tree, visible text projection, parameter snapshot, and emitted effects all represent the same newly accepted generation

#### Scenario: Live adjustment is rejected
- **WHEN** a due live action violates a parameter boundary
- **THEN** the existing event log records the rejection, state and projection generations remain unchanged, no audio effect is emitted, and later valid input remains processable

### Requirement: Live checkpoint expectations precede mutation
The live scene SHALL compute and freeze each expected transition from the prior canonical state and the production-owned parameter descriptor before dispatching the corresponding event. Observed post-dispatch state, projection, effect, or audio values SHALL NOT be reused as the expected values.

#### Scenario: Checkpoint compares expected and actual values
- **WHEN** a live event has been processed
- **THEN** its actual production record and projections are compared with the independently frozen expectation and any mismatch prevents checkpoint completion

### Requirement: Responsive sustained MIDI projection
Accepted MIDI SHALL continue through `AppState::apply`, canonical generation advancement, state/text/parameter/tree projection, EventLog recording, and audio-command publication without deep-cloning immutable capability/Patch state, parsing Crest's own serialized JSON, rebuilding an unchanged text body, or eagerly materializing large JSON documents that no observer requested. A fifteen-Patch production-path acceptance fixture SHALL dispatch 512 MIDI events within 50 milliseconds in Cargo's unoptimized test profile.

#### Scenario: A dense fifteen-Patch MIDI batch is dispatched
- **WHEN** 512 normalized MIDI events target installed Patches through the production `AppLoop`
- **THEN** every event has one accepted generation, coherent logical projections, journal record, parameter publication, and audio command, no records are dropped, and the batch completes within 50 milliseconds

#### Scenario: Deferred projection JSON is inspected
- **WHEN** a generation-only StateSnapshot or StateTree is materialized after MIDI dispatch
- **THEN** its JSON, hash, text selection, parameter generation, and complete nested values exactly equal an eager canonical projection from the same accepted `AppState`

### Requirement: Descriptor-derived Patch editable surface
The ordered editable surface for each Patch SHALL be derived from one production-owned resolver containing the four Patch mixer values, four common ADSR values, and every active descriptor parameter classified Scalar in descriptor order. Reducer navigation, adjustment, text selection, deterministic demo coverage, and live-demo coverage SHALL use this same surface.

#### Scenario: SoundFont Patch surface is derived
- **WHEN** a SoundFont Patch is selected
- **THEN** its mixer and ADSR values are editable while bank, program, percussion, and file remain visible Structural values outside the live selection cycle

#### Scenario: Braids Patch surface is derived
- **WHEN** a Braids Patch is selected
- **THEN** its mixer, ADSR, Model, Timbre, and Color values are each reachable exactly once in the declared order

### Requirement: Fixed engine-scalar audio projection
Each real-time Patch parameter value SHALL carry its canonical ADSR plus at most sixteen destructor-free engine Scalar values encoded in descriptor order. The projection SHALL contain no string, vector, capability-specific union, asset, or engine object, and choice values SHALL use only the index fixed by the active graph's immutable descriptor revision.

#### Scenario: Braids scalar snapshot is published
- **WHEN** a Braids value is accepted
- **THEN** the newest complete snapshot carries its matching finite encoded value in the graph-compatible Braids layout and no SoundFont Patch receives that slot

#### Scenario: Scalar capacity is exceeded
- **WHEN** an installed descriptor declares more than sixteen Scalar values
- **THEN** descriptor/registry construction fails before Patch installation or audio publication

