# Prepared Engine Rack

## Purpose

Define capability-neutral prepared instrument ownership, complete off-callback graph preparation, revision-safe structural handoff, and no-drop hard-real-time graph activation.
## Requirements
### Requirement: Exact capability-matched rack preparation
The application SHALL construct a prepared engine rack from the accepted ordered Patch set by matching every Patch capability identity to exactly one installed preparation boundary and producing exactly one prepared instrument with the same Patch identity. Construction SHALL fail atomically with a typed error for missing or duplicate preparation matches, duplicate or mismatched Patch identities, invalid capacities, preparation failure, or more active Patches than the declared fixed bound; no partial rack or graph SHALL be published and no fallback SHALL be selected.

#### Scenario: Different prepared implementations share one rack
- **WHEN** two accepted Patches use two conforming preparation boundaries with different implementation types
- **THEN** rack construction succeeds with one correctly identified prepared instrument in each canonical Patch slot and without an engine-identity branch in the rack

#### Scenario: Preparation match is invalid
- **WHEN** a Patch has zero or multiple matching preparation boundaries, or a preparer returns an instrument for another Patch identity
- **THEN** the entire construction fails before publication and every temporary value is retired outside the audio callback

#### Scenario: Rack capacity is exceeded
- **WHEN** the accepted Patch set is larger than the declared fixed active-Patch capacity
- **THEN** construction fails without truncating, merging, reordering, or substituting any Patch

### Requirement: Bounded heterogeneous Patch routing
The prepared rack SHALL use fixed-capacity ordered storage, route each Patch-targeted audio command and only that Patch's matching fixed scalar/envelope projection to the prepared instrument with the exact `PatchId`, render each active instrument once per block into only its matching caller-owned stereo stem, and visit at most the fixed Patch capacity for a global all-notes-off. An unknown Patch identity or parameter-layout mismatch SHALL produce bounded observable failure without fallback, broadcast, or mutation of another instrument.

#### Scenario: MIDI targets one Patch
- **WHEN** a note or controller command names a non-first Patch in a rack containing SoundFont and Braids implementations
- **THEN** only that Patch's prepared instrument receives the command and matching parameters and every untargeted instrument and stem remains unchanged

#### Scenario: Several Patches render one block
- **WHEN** multiple prepared instruments are active for a bounded render block
- **THEN** each instrument is called once with its exact Patch parameter projection and fills only the stem whose index and `PatchId` match the canonical rack order

#### Scenario: Unknown Patch is targeted
- **WHEN** a command names a Patch not present in the active rack
- **THEN** no prepared instrument receives the command, no fallback or broadcast occurs, and a fixed-size routing failure is observable

### Requirement: Complete off-callback graph preparation
The application SHALL prepare one complete graph outside the audio callback containing the engine rack, Patch stems, mixer, one shared reverb, one shared delay, routing, effect memory, scratch storage, declared sample rate and frame capacity, and a compatible initial parameter snapshot. The graph's rack, stems, mixer routing, and parameters SHALL contain the same Patch identities in the same bounded order, and no incomplete or invalid graph SHALL reach audio ownership.

#### Scenario: Initial graph is valid
- **WHEN** graph preparation succeeds for an accepted Patch set and supported device configuration
- **THEN** every engine, effect, route, stem, voice, and scratch capacity is ready before the renderer or device callback can use the graph

#### Scenario: One graph component fails preparation
- **WHEN** instrument, effect, routing, parameter, sample-rate, or frame-capacity preparation fails
- **THEN** no part of the candidate graph becomes active and the previously active graph, if any, remains unchanged

### Requirement: Dedicated structural ownership handoff
Prepared control-to-audio graph ownership and retired audio-to-control graph ownership SHALL use distinct preallocated bounded transports that are separate from discrete audio commands, latest scalar parameters, and callback observations. Moving a graph through either transport SHALL NOT allocate, block, clone graph-owned state, or destroy that state on the audio callback.

#### Scenario: Replacement graph is published
- **WHEN** control or worker ownership publishes one completely prepared replacement
- **THEN** audio receives that same owned graph through the structural input transport without carrying it in a command or scalar snapshot

#### Scenario: Replaced graph returns
- **WHEN** a replacement becomes active
- **THEN** the prior graph returns through the dedicated retirement transport and is destroyed only after control or worker ownership receives it

### Requirement: Block-boundary no-drop graph activation
The audio renderer SHALL consider structural replacement only at the start of a render block, activate at most one prepared graph in that block, and return the replaced graph without dropping it. If the retirement transport is full, the callback SHALL retain the replaced graph in fixed preallocated storage, retry later, continue rendering the active graph, and accept no further replacement until that storage is clear. Control SHALL allow at most one unacknowledged replacement publication.

#### Scenario: Prepared replacement is available
- **WHEN** a complete graph is waiting at the start of a render block and no retirement is pending
- **THEN** the graph becomes active before command and scalar consumption for that block and the previous graph is returned or retained intact

#### Scenario: Retirement transport is full
- **WHEN** the callback cannot return the replaced graph
- **THEN** it keeps exactly that graph for a later bounded retry, performs no destruction, reports retirement pressure, and does not take another replacement

#### Scenario: A second replacement is requested too early
- **WHEN** one published graph has not yet been acknowledged and retired according to the handoff protocol
- **THEN** control throttles the second request rather than overwriting or queueing an unbounded structural backlog

### Requirement: Graph-compatible scalar snapshots
Every prepared graph and fixed scalar parameter snapshot SHALL carry a nonzero monotonic graph revision. The renderer SHALL consume a latest scalar snapshot only when its revision, active Patch count, ordered Patch identities, fixed capacities, and descriptor-ordered instrument scalar layouts are compatible with the active graph; otherwise it SHALL retain the last compatible snapshot. Every newly active graph SHALL contain compatible initial mixer, envelope, and instrument parameters so rendering never depends on the relative arrival order of structural and scalar publications.

#### Scenario: Matching scalar snapshot arrives
- **WHEN** the latest complete parameter snapshot targets the active graph revision, Patch order, and prepared capability layouts
- **THEN** the next rendered block uses that exact accepted mixer, envelope, and instrument-scalar generation

#### Scenario: Snapshot targets another graph
- **WHEN** the latest scalar snapshot is stale, belongs to a pending graph revision, or carries an incompatible instrument scalar layout
- **THEN** the renderer does not consume it and continues with the active graph's last compatible complete snapshot

#### Scenario: Graph activates before scalar publication
- **WHEN** a replacement graph becomes active before its separately published latest scalar snapshot is visible
- **THEN** the first block uses the complete compatible initial snapshot embedded during graph preparation

### Requirement: Hard real-time prepared operations
Prepared-instrument dispatch, rendering, all-notes-off, graph activation, and graph retirement retry SHALL use bounded preallocated work and SHALL perform no allocation, deallocation, collection growth, locking, blocking, I/O, logging, formatting, panic, unwind, or owned-state destruction. Polymorphic dispatch MAY occur once per targeted command and once per active Patch per block but SHALL NOT occur inside an instrument's inner sample loop.

#### Scenario: Audio renders during graph and control traffic
- **WHEN** commands, scalar snapshots, and one prepared replacement are published while bounded audio blocks render
- **THEN** measured callback allocation and destruction counts remain zero, output remains finite, and callback work remains bounded by declared capacities

### Requirement: Prepared-rack Phase 2 boundary
This increment SHALL expose HiDef SoundFont and Braids together as the two production instrument capabilities and preparers. Replacement graphs SHALL retain the accepted PatchId and capability layout, and the application SHALL NOT expose engine selection, a PATCH page, user-triggered structural edits, modulation, layering, per-Patch effects, or fallback.

#### Scenario: Production application starts after the rack migration
- **WHEN** the normal, smoke, headless-demo, or live-demo path is launched
- **THEN** the generic prepared rack hosts alternating SoundFont and Braids Patches with exact isolated stems and no unavailable structural control is presented

### Requirement: Falsifiable prepared-rack acceptance
The repository SHALL provide a named prepared-engine-rack acceptance target that exercises the production rack, renderer, and structural handoff and emits its success marker only after exact routing, stem isolation, atomic failure, swap acknowledgement, one-in-flight throttling, full-retirement-transport retry, compatible snapshots, zero callback allocations, and off-callback destruction are measured.

#### Scenario: Healthy prepared-rack contract runs
- **WHEN** the named acceptance target executes with its deterministic fixtures
- **THEN** all declared structured observations satisfy their predicates before `CREST_ACCEPTANCE prepared_engine_rack passed` is emitted

#### Scenario: Ownership or routing proof is absent
- **WHEN** a graph is dropped, a callback destructor runs, a command reaches another Patch, a stem is contaminated, or any required measurement is missing
- **THEN** the acceptance target fails and does not claim prepared-rack completion

### Requirement: Production structural handoff is injected
The standalone composition root SHALL construct and inject the structural graph boundary used by the production application. The application SHALL split and use only that injected boundary for initial graph status, replacement publication, and retirement collection and SHALL NOT construct a concrete structural adapter internally.

#### Scenario: Replaceable boundary is supplied
- **WHEN** a deterministic composition supplies a conforming structural-boundary fixture through the production constructor
- **THEN** startup splits and uses that exact fixture before rendering and no hidden structural boundary is created

### Requirement: Renderer preserves unknown-Patch routing status
The production renderer SHALL preserve an unknown-Patch dispatch as fixed-size callback-to-control observation data while leaving every prepared instrument, active-note observation, and untargeted stem unchanged. The observation SHALL identify the unknown Patch and advance a saturating failure count without fallback, broadcast, allocation, blocking, logging, or formatting.

#### Scenario: Unknown Patch reaches the renderer
- **WHEN** a bounded audio command names a Patch absent from the active parameter layout or prepared rack
- **THEN** no instrument receives the command and the next coherent audio observation reports exactly one additional routing failure and that Patch identity

#### Scenario: Known Patch follows a routing failure
- **WHEN** a later command names an installed Patch
- **THEN** it is dispatched normally to that Patch only and the prior failure does not change routing or active-note state

### Requirement: Production mixed-engine rack proof
The named prepared-engine-rack acceptance SHALL include real SoundFont and real Braids preparers in one production graph and SHALL prove both implementation types respond to exact targeted MIDI, consume only matching scalar/envelope values, produce distinct nonzero finite stems, and retain zero callback allocation/destruction under bounded load.

#### Scenario: Mixed production graph renders
- **WHEN** one SoundFont Patch and one Braids Patch are prepared, sounded, edited, and rendered together
- **THEN** each engine's isolated stem and parameter consequences are measured before the acceptance marker is emitted

#### Scenario: Several Braids Patches share one rack
- **WHEN** `N` Braids Patches are admitted alongside SoundFont within the rack's fixed Patch capacity
- **THEN** the rack owns `N` separate prepared Braids instruments with `16 × N` total voices, including forty-eight for three Braids Patches, and routes commands, parameters, stealing, rendering, and all-notes-off without a global Braids voice pool or a Braids-specific Patch-count limit
